//! Run-directory, provenance, and environment simulation runner
//! (M0-03/M0-05 scaffolding, M0-13/M0-14 execution).
//!
//! Every run gets a unique directory holding the fully resolved
//! configuration, a manifest with code/seed identity, the derived seed
//! stream table, condition identity, ordinary event records plus
//! evaluator-only annotations (when logging is enabled), and a terminal
//! completion record. Raw output stays immutable once written; analysis
//! writes derived outputs elsewhere. Large generated data and build output
//! stay out of ordinary source commits (see README and `.gitignore`).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::config::{Config, resolved_toml};
use crate::environment::SimError;
use crate::experiments::baseline::{
    BaselineSummary, ConstantBaseline, RandomBaseline, run_ordinary,
};
use crate::logging::events::{
    CompletionInfo, ConditionInfo, EVENT_SCHEMA_VERSION, LogError, OrdinaryEvent, RunStatus,
    read_json, read_jsonl, validate_stream, write_json, write_jsonl,
};
use crate::rng::{SUPPORTED_STREAMS, SeedTuple, derive_seed_hex};

/// Where a seed value came from. The resolved namespace must come from a
/// documented CLI/config/manifest source, never from silent test seeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedSource {
    Config,
    Cli,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveSeeds {
    pub root_seed: u64,
    pub root_source: SeedSource,
    pub namespace: String,
    pub namespace_source: SeedSource,
    pub outer_seed: u64,
    pub outer_source: SeedSource,
}

impl EffectiveSeeds {
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            root_seed: cfg.seeds.root_seed,
            root_source: SeedSource::Config,
            namespace: cfg.seeds.namespace.clone(),
            namespace_source: SeedSource::Config,
            outer_seed: cfg.seeds.outer_seed,
            outer_source: SeedSource::Config,
        }
    }

    /// Apply explicit CLI overrides, recording which source won per field.
    pub fn with_overrides(mut self, root_seed: Option<u64>, outer_seed: Option<u64>) -> Self {
        if let Some(r) = root_seed {
            self.root_seed = r;
            self.root_source = SeedSource::Cli;
        }
        if let Some(o) = outer_seed {
            self.outer_seed = o;
            self.outer_source = SeedSource::Cli;
        }
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RunManifest {
    pub run_id: String,
    pub profile_name: String,
    pub condition_id: String,
    pub schema_version: u32,
    pub created_unix_secs: u64,
    pub code_version: String,
    pub git_revision: String,
    pub git_dirty: Option<bool>,
    pub platform: String,
    pub rustc_version: String,
    pub seed_policy: String,
    pub seeds: EffectiveSeeds,
    pub rng_policy: String,
    pub note: String,
}

/// Best-effort git identity for provenance. A hash alone is incomplete when
/// the tree is dirty, so dirty status is recorded alongside.
fn git_identity() -> (String, Option<bool>) {
    let rev = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty());
    (rev, dirty)
}

/// Best-effort compiler version for provenance (falls back to "unknown"
/// outside a toolchain that provides it at run time).
fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Build the unique run directory name. Unique per wall-clock second,
/// profile, and effective seeds; collisions across processes in the same
/// second fall back to a `-retryN` suffix at creation time.
pub fn run_dir_name(profile: &str, seeds: &EffectiveSeeds, unix_secs: u64) -> String {
    let safe_profile: String = profile
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "{safe_profile}-root{}-outer{}-{unix_secs}",
        seeds.root_seed, seeds.outer_seed
    )
}

/// Seed-stream audit table: derived hex seeds for every reserved stream at
/// lifetime index 0. Proves namespace/stream separation for the run.
pub fn seed_stream_table(seeds: &EffectiveSeeds) -> Vec<(String, String)> {
    SUPPORTED_STREAMS
        .iter()
        .map(|stream| {
            let t = SeedTuple::new(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                0,
                stream,
            );
            let hex = derive_seed_hex(&t).expect("reserved streams always validate");
            ((*stream).to_owned(), hex)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunError {
    Io(String),
    Seed(crate::rng::SeedError),
    Config(crate::config::ConfigError),
    Sim(SimError),
    Log(LogError),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(m) => write!(f, "run I/O error: {m}"),
            Self::Seed(e) => write!(f, "run seed error: {e}"),
            Self::Config(e) => write!(f, "run config error: {e}"),
            Self::Sim(e) => write!(f, "run simulation error: {e}"),
            Self::Log(e) => write!(f, "run log error: {e}"),
        }
    }
}

impl std::error::Error for RunError {}

/// Create the run directory and write `resolved_config.toml`,
/// `manifest.json`, and `seed_streams.json`.
///
/// The resolved config embeds CLI seed overrides so the saved file is the
/// complete effective configuration, not the small user override.
pub fn create_run_dir(
    cfg: &Config,
    seeds: &EffectiveSeeds,
    condition_id: &str,
    base: &Path,
) -> Result<PathBuf, RunError> {
    create_run_dir_with_note(
        cfg,
        seeds,
        condition_id,
        base,
        "M0 environment run: validation + provenance + ordinary/hidden event streams. Neural/search code in later milestones.",
    )
}

/// [`create_run_dir`] with an explicit manifest note (M1-12: actor runs
/// record their own provenance instead of inheriting the M0 wording).
pub fn create_run_dir_with_note(
    cfg: &Config,
    seeds: &EffectiveSeeds,
    condition_id: &str,
    base: &Path,
    note: &str,
) -> Result<PathBuf, RunError> {
    crate::rng::validate_tuple(&SeedTuple::new(
        seeds.root_seed,
        &seeds.namespace,
        seeds.outer_seed,
        0,
        "cue_order",
    ))
    .map_err(RunError::Seed)?;

    let ts = now_unix_secs();
    let stem = run_dir_name(&cfg.profile_name, seeds, ts);
    let dir = allocate_run_dir(base, &stem)?;

    // Resolved config with effective seeds baked in.
    let mut effective = cfg.clone();
    effective.seeds.root_seed = seeds.root_seed;
    effective.seeds.namespace.clone_from(&seeds.namespace);
    effective.seeds.outer_seed = seeds.outer_seed;
    let resolved = resolved_toml(&effective).map_err(RunError::Config)?;
    std::fs::write(dir.join("resolved_config.toml"), resolved)
        .map_err(|e| RunError::Io(e.to_string()))?;

    let (git_revision, git_dirty) = git_identity();
    let manifest = RunManifest {
        run_id: dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| stem.clone()),
        profile_name: cfg.profile_name.clone(),
        condition_id: condition_id.to_owned(),
        schema_version: cfg.schema_version,
        created_unix_secs: ts,
        code_version: env!("CARGO_PKG_VERSION").to_owned(),
        git_revision,
        git_dirty,
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        rustc_version: rustc_version(),
        seed_policy: "(root_seed, namespace, outer_seed, lifetime_index, stream_name) -> SHA-256 -> ChaCha8Rng; namespaces development|training|validation|final_test are disjoint (spec 20.5)".to_owned(),
        seeds: seeds.clone(),
        rng_policy: "sha2::Sha256 derivation (see Cargo.lock) + rand_chacha::ChaCha8Rng; one RNG instance per stream; no runtime-randomized hashes".to_owned(),
        note: note.to_owned(),
    };
    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|e| RunError::Io(e.to_string()))?;
    std::fs::write(dir.join("manifest.json"), manifest_json)
        .map_err(|e| RunError::Io(e.to_string()))?;

    let table: Vec<serde_json::Value> = seed_stream_table(seeds)
        .into_iter()
        .map(|(stream, seed_hex)| serde_json::json!({"stream": stream, "seed_hex": seed_hex}))
        .collect();
    let streams_json = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": seeds.namespace,
        "root_seed": seeds.root_seed,
        "outer_seed": seeds.outer_seed,
        "lifetime_index": 0,
        "streams": table,
    }))
    .map_err(|e| RunError::Io(e.to_string()))?;
    std::fs::write(dir.join("seed_streams.json"), streams_json)
        .map_err(|e| RunError::Io(e.to_string()))?;

    Ok(dir)
}

fn allocate_run_dir(base: &Path, stem: &str) -> Result<PathBuf, RunError> {
    std::fs::create_dir_all(base).map_err(|e| RunError::Io(e.to_string()))?;
    let mut dir = base.join(stem);
    // Atomic creation claims ownership, even if another process has not
    // written its manifest yet. Existing incomplete runs remain untouched.
    let mut owned = false;
    for retry in 0..100 {
        if retry > 0 {
            dir = base.join(format!("{stem}-retry{retry}"));
        }
        match std::fs::create_dir(&dir) {
            Ok(()) => {
                owned = true;
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(RunError::Io(e.to_string())),
        }
    }
    if !owned {
        return Err(RunError::Io(
            "cannot allocate a unique run directory".to_owned(),
        ));
    }

    Ok(dir)
}

/// Selectable ladder policies with their condition ids (spec 13.1):
/// `--baseline random` (B0), `constant-0`/`constant-1` (B1), `actor` (B3,
/// the same inherited actor with plasticity disabled), `oracle` (O1,
/// privileged).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaselineSel {
    Random,
    Constant(u8),
    Actor,
    Oracle,
}

impl BaselineSel {
    pub fn parse(name: &str) -> Result<Self, RunError> {
        match name {
            "random" => Ok(Self::Random),
            "constant-0" => Ok(Self::Constant(0)),
            "constant-1" => Ok(Self::Constant(1)),
            "actor" => Ok(Self::Actor),
            "oracle" => Ok(Self::Oracle),
            other => Err(RunError::Io(format!(
                "unknown baseline '{other}'; expected random|constant-0|constant-1|actor|oracle"
            ))),
        }
    }

    pub fn condition_id(self) -> &'static str {
        match self {
            Self::Random => "B0",
            Self::Constant(_) => "B1",
            Self::Actor => "B3",
            Self::Oracle => "O1",
        }
    }

    pub fn policy_name(self) -> &'static str {
        match self {
            Self::Random => "random",
            Self::Constant(0) => "constant-0",
            Self::Constant(1) => "constant-1",
            // Constant values outside {0, 1} are rejected at construction;
            // this arm is unreachable through the public API.
            Self::Constant(_) => "constant-invalid",
            Self::Actor => "actor-no-learning",
            Self::Oracle => "oracle",
        }
    }
}

/// Summary of a multi-lifetime simulation run.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationReport {
    pub dir: PathBuf,
    pub run_id: String,
    pub condition_id: String,
    pub lifetimes: u64,
    pub commitments: u64,
    pub outcomes: u64,
    pub mean_reward: f64,
    pub event_log: bool,
}

/// Run `lifetimes` ladder lifetimes (indices 0..lifetimes) into a fresh
/// run directory with full provenance: manifest, resolved config, seed
/// streams, condition identity, ordinary + hidden event streams (when the
/// config enables `event_log`), and a terminal completion record.
///
/// B0/B1/O1 run under the M0 environment-only guard; B3 (`actor`) runs
/// the nonplastic actor through the same tick/commit/record loop under
/// the M1-07 actor guard. Event schemas are identical across rungs, so
/// one offline audit covers every condition.
///
/// Logging on/off changes nothing about the simulated behavior: the same
/// seeds always produce the same choices and rewards; only the event files
/// are skipped when disabled.
pub fn run_simulation(
    cfg: &Config,
    seeds: &EffectiveSeeds,
    baseline: BaselineSel,
    lifetimes: u64,
    base: &Path,
) -> Result<SimulationReport, RunError> {
    if baseline == BaselineSel::Actor {
        crate::config::validate_actor_no_learning_execution(cfg).map_err(RunError::Config)?;
    } else {
        crate::config::validate_baseline_execution(cfg).map_err(RunError::Config)?;
    }
    if let BaselineSel::Constant(action) = baseline {
        ConstantBaseline::new(action).map_err(RunError::Sim)?;
    }
    if lifetimes == 0 {
        return Err(RunError::Io("lifetimes must be >= 1".to_owned()));
    }
    let condition_id = baseline.condition_id();
    let dir = if baseline == BaselineSel::Actor {
        create_run_dir_with_note(
            cfg,
            seeds,
            condition_id,
            base,
            "M1 no-learning actor run (B3): continuous inherited dynamics with plasticity disabled. Ordinary/hidden event streams share the M0 schema.",
        )?
    } else {
        create_run_dir(cfg, seeds, condition_id, base)?
    };
    let run_id = dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "run".to_owned());
    let event_log = cfg.logging.event_log;
    write_json(
        &dir,
        "condition.json",
        &ConditionInfo {
            condition_id: condition_id.to_owned(),
            policy: baseline.policy_name().to_owned(),
            profile_name: cfg.profile_name.clone(),
            lifetimes,
            outcomes_per_lifetime: cfg.simulation.outcomes_per_lifetime,
            event_log,
        },
    )
    .map_err(RunError::Log)?;

    let outcome = run_lifetimes(cfg, seeds, baseline, lifetimes, &dir, &run_id, event_log);
    match outcome {
        Ok(report) => {
            write_json(
                &dir,
                "completion.json",
                &CompletionInfo {
                    run_id: run_id.clone(),
                    status: RunStatus::Completed,
                    lifetimes_completed: report.0,
                    commitments: report.1,
                    outcomes: report.2,
                },
            )
            .map_err(RunError::Log)?;
            Ok(SimulationReport {
                dir,
                run_id,
                condition_id: condition_id.to_owned(),
                lifetimes: report.0,
                commitments: report.1,
                outcomes: report.2,
                mean_reward: report.3,
                event_log,
            })
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = write_json(
                &dir,
                "completion.json",
                &CompletionInfo {
                    run_id: run_id.clone(),
                    status: RunStatus::Interrupted { reason },
                    lifetimes_completed: 0,
                    commitments: 0,
                    outcomes: 0,
                },
            );
            Err(error)
        }
    }
}

/// Drive the lifetimes, accumulating ordinary events + hidden annotations.
/// Returns (lifetimes_completed, commitments, outcomes, mean_reward).
fn run_lifetimes(
    cfg: &Config,
    seeds: &EffectiveSeeds,
    baseline: BaselineSel,
    lifetimes: u64,
    dir: &Path,
    run_id: &str,
    event_log: bool,
) -> Result<(u64, u64, u64, f64), RunError> {
    let mut events = Vec::new();
    let mut hidden = Vec::new();
    let mut commitments = 0u64;
    let mut outcomes = 0u64;
    let mut reward_sum = 0.0;
    for lifetime_index in 0..lifetimes {
        let summary = run_one_lifetime(cfg, seeds, baseline, lifetime_index)?;
        for (choice, annotation) in summary.choices.iter().zip(summary.annotations.iter()) {
            commitments += 1;
            outcomes += 1;
            reward_sum += choice.reward;
            if event_log {
                events.push(OrdinaryEvent {
                    schema_version: EVENT_SCHEMA_VERSION,
                    run_id: run_id.to_owned(),
                    condition_id: baseline.condition_id().to_owned(),
                    namespace: seeds.namespace.clone(),
                    outer_seed: seeds.outer_seed,
                    lifetime_index,
                    choice_index: choice.choice_index,
                    event_id: choice.event_id,
                    cue_index: choice.cue,
                    commit_tick: choice.commit_tick,
                    outcome_tick: choice.feedback_tick,
                    action: choice.action,
                    reward: choice.reward,
                });
                hidden.push(annotation.clone());
            }
        }
    }
    // NOTE: choice_index and event_id restart at 0 per lifetime in M0,
    // so multi-lifetime streams are validated as one contiguous block
    // per lifetime. M1 checkpoint/continuation work must preserve identity
    // and explicitly specify any change to this schema.
    if event_log {
        validate_per_lifetime(&events, &hidden, cfg.environment.cue_count)?;
        write_jsonl(dir, "events.jsonl", &events).map_err(RunError::Log)?;
        write_jsonl(dir, "hidden.jsonl", &hidden).map_err(RunError::Log)?;
    }
    let mean_reward = if outcomes > 0 {
        reward_sum / outcomes as f64
    } else {
        0.0
    };
    Ok((lifetimes, commitments, outcomes, mean_reward))
}

/// Validate each lifetime's contiguous block separately (M0 event ids are
/// lifetime-local; see above).
fn validate_per_lifetime(
    events: &[OrdinaryEvent],
    hidden: &[crate::environment::HiddenAnnotation],
    cue_count: usize,
) -> Result<(), RunError> {
    let mut start = 0usize;
    while start < events.len() {
        let lifetime = events[start].lifetime_index;
        let mut end = start;
        while end < events.len() && events[end].lifetime_index == lifetime {
            end += 1;
        }
        // Rebase choice ids to the block for the contiguity check: M0
        // lifetimes restart choice_index at 0, so each block is checked
        // as its own stream.
        validate_stream(&events[start..end], &hidden[start..end], cue_count)
            .map_err(RunError::Log)?;
        start = end;
    }
    Ok(())
}

fn run_one_lifetime(
    cfg: &Config,
    seeds: &EffectiveSeeds,
    baseline: BaselineSel,
    lifetime_index: u64,
) -> Result<BaselineSummary, RunError> {
    match baseline {
        BaselineSel::Random => {
            let mut policy = RandomBaseline::new(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                lifetime_index,
            )
            .map_err(RunError::Sim)?;
            run_ordinary(
                cfg,
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                lifetime_index,
                "random",
                &mut policy,
            )
            .map_err(RunError::Sim)
        }
        BaselineSel::Constant(action) => {
            let mut policy = ConstantBaseline::new(action).map_err(RunError::Sim)?;
            let name = if action == 0 {
                "constant-0"
            } else {
                "constant-1"
            };
            run_ordinary(
                cfg,
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                lifetime_index,
                name,
                &mut policy,
            )
            .map_err(RunError::Sim)
        }
        BaselineSel::Oracle => crate::experiments::baseline::run_oracle(
            cfg,
            seeds.root_seed,
            &seeds.namespace,
            seeds.outer_seed,
            lifetime_index,
        )
        .map_err(RunError::Sim),
        BaselineSel::Actor => {
            let mut policy = crate::agent::no_learning::NoLearningActor::new(
                cfg,
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                lifetime_index,
            )
            .map_err(|e| {
                RunError::Sim(crate::environment::SimError::InvalidConfiguration(
                    e.to_string(),
                ))
            })?;
            crate::experiments::baseline::run_actor_ordinary(
                cfg,
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                lifetime_index,
                "actor-no-learning",
                &mut policy,
            )
            .map_err(RunError::Sim)
        }
    }
}

/// Read back a run's ordinary + hidden streams with filenames attached to
/// errors (used by tests; the offline audit lives in analysis/).
pub fn read_run_streams(
    dir: &Path,
) -> Result<
    (
        Vec<OrdinaryEvent>,
        Vec<crate::environment::HiddenAnnotation>,
    ),
    RunError,
> {
    let events = read_jsonl(dir, "events.jsonl").map_err(RunError::Log)?;
    let hidden = read_jsonl(dir, "hidden.jsonl").map_err(RunError::Log)?;
    Ok((events, hidden))
}

/// Read back a run's completion record.
pub fn read_completion(dir: &Path) -> Result<CompletionInfo, RunError> {
    read_json(dir, "completion.json").map_err(RunError::Log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_dir_name_is_unique_per_seed_and_time() {
        let a = EffectiveSeeds {
            root_seed: 1,
            root_source: SeedSource::Config,
            namespace: "development".to_owned(),
            namespace_source: SeedSource::Config,
            outer_seed: 1,
            outer_source: SeedSource::Config,
        };
        let b = EffectiveSeeds {
            root_seed: 2,
            ..a.clone()
        };
        assert_ne!(run_dir_name("p", &a, 100), run_dir_name("p", &b, 100));
        assert_ne!(run_dir_name("p", &a, 100), run_dir_name("p", &a, 101));
    }

    fn test_config() -> Config {
        toml::from_str(
            r#"
schema_version = 1
profile_name = "test"
[simulation]
dt = 1.0
precision = "f64"
warmup_ticks = 4
outcomes_per_lifetime = 2
reset_policy = "birth_only"
feedback_order = "before_neural_transition"
[environment]
kind = "stationary_clean"
cue_count = 2
cue_encoding = "one_hot"
stable_fraction = 1.0
feedback_noise_values = [0.0]
volatile_hazard_values = [0.0]
hazard_clock = "cue_exposure"
quiet_ticks = [4, 4]
cue_ticks = 8
memory_gap_ticks = [0, 0]
response_ticks = 4
reward_delay_ticks = [1, 1]
feedback_ticks = 1
max_pending_choices = 1
[logging]
event_log = true
trace_every_ticks = 10
full_trace_lifetimes = 1
record_raw_and_applied_updates = true
[seeds]
root_seed = 1
namespace = "development"
outer_seed = 1
"#,
        )
        .expect("test config parses")
    }

    fn test_seeds() -> EffectiveSeeds {
        EffectiveSeeds::from_config(&test_config())
    }

    #[test]
    fn same_second_runs_never_share_a_directory() {
        let base = std::env::temp_dir().join(format!("cra-rundir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let cfg = test_config();
        let seeds = test_seeds();
        let first = create_run_dir(&cfg, &seeds, "B0", &base).expect("first run");
        let second = create_run_dir(&cfg, &seeds, "B0", &base).expect("second run");
        assert_ne!(first, second, "same-second runs must not collide");
        assert!(first.join("manifest.json").exists());
        assert!(second.join("manifest.json").exists());
        let _ = std::fs::remove_dir_all(&base);
    }
    #[test]
    fn allocation_preserves_incomplete_runs_and_is_atomic() {
        let base = std::env::temp_dir().join(format!("cra-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("fixed")).unwrap();
        std::fs::write(
            base.join("fixed/resolved_config.toml"),
            "incomplete original",
        )
        .unwrap();
        let barrier = std::sync::Barrier::new(8);
        let dirs = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        allocate_run_dir(&base, "fixed").unwrap()
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .collect::<std::collections::BTreeSet<_>>()
        });
        assert_eq!(dirs.len(), 8);
        assert!(!dirs.contains(&base.join("fixed")));
        assert_eq!(
            std::fs::read_to_string(base.join("fixed/resolved_config.toml")).unwrap(),
            "incomplete original"
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    fn actor_test_config() -> Config {
        let mut cfg = test_config();
        cfg.profile_name = "actor_test".to_owned();
        cfg.actor = Some(crate::config::Actor {
            neuron_count: 16,
            motor_neurons_per_action: 2,
            edge_probability: 0.25,
            self_edges: false,
            recurrent_gain: 0.8,
            input_scale: 0.3,
            tau_h: 5.0,
            tau_a: 100.0,
            adaptation_strength: 0.0,
            noise_sigma: 0.05,
            motor_filter_tau: 3.0,
        });
        crate::config::validate(&cfg).expect("actor config validates");
        cfg
    }

    #[test]
    fn baseline_selection_names_the_b3_actor_rung() {
        assert_eq!(
            BaselineSel::parse("actor").expect("actor"),
            BaselineSel::Actor
        );
        assert_eq!(BaselineSel::Actor.condition_id(), "B3");
        assert_eq!(BaselineSel::Actor.policy_name(), "actor-no-learning");
        assert!(BaselineSel::parse("actor-no-learning").is_err());
    }

    #[test]
    fn actor_simulation_writes_b3_provenance_and_streams() {
        let base = std::env::temp_dir().join(format!("cra-actor-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let cfg = actor_test_config();
        let seeds = EffectiveSeeds::from_config(&cfg);
        // An env-only config must not run as the actor rung.
        assert!(run_simulation(&test_config(), &seeds, BaselineSel::Actor, 1, &base).is_err());
        let report = run_simulation(&cfg, &seeds, BaselineSel::Actor, 2, &base).expect("actor run");
        assert_eq!(report.condition_id, "B3");
        assert_eq!(report.lifetimes, 2);
        assert_eq!(report.commitments, 2 * cfg.simulation.outcomes_per_lifetime);
        assert_eq!(report.outcomes, report.commitments);
        assert!(report.mean_reward.is_finite());
        let completion = read_completion(&report.dir).expect("completion");
        assert_eq!(completion.lifetimes_completed, 2);
        let (events, hidden) = read_run_streams(&report.dir).expect("streams");
        assert_eq!(events.len() as u64, report.outcomes);
        assert_eq!(hidden.len(), events.len());
        assert!(events.iter().all(|e| e.action <= 1));
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(report.dir.join("manifest.json")).expect("manifest"),
        )
        .expect("manifest json");
        assert_eq!(manifest["condition_id"], "B3");
        let _ = std::fs::remove_dir_all(&base);
    }
}
