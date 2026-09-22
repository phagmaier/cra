//! First full lifetime checkpoint: exact pause/resume (M1-09).
//!
//! Spec: 10.7 (checkpoint carries neural state, adaptation, motor filters,
//! environment phase plus pending reward/action latch, consumed-feedback
//! bookkeeping, RNG states/counters, inherited parameters, and resolved
//! configuration identifiers — a genome alone is not a checkpoint), 10.6
//! (no silent clipping; failures are explicit), 20 (deterministic streams
//! resume exactly).
//!
//! Scope: the nonplastic M1 actor plus the continuous environment for
//! [`Checkpoint`] (schema 2); M3-10 adds [`LearningCheckpoint`] (schema
//! 3) for the episodic plastic learner, and M4-06 adds
//! [`ContinuousCheckpoint`] (schema 4) for the fully persistent learner.
//! Both plastic envelopes carry `P`/`E`/baseline/dedup inside the versioned
//! [`PlasticSnapshot`](crate::agent::plasticity::PlasticSnapshot); schema 4
//! additionally pins and validates the derived `W0 + P` cache.
//! [`Checkpoint`] rejects files that claim plasticity (unknown fields)
//! instead of silently defaulting them, and none of the three loaders reads
//! another envelope's schema. Health summaries and traces (M1-08) are
//! diagnostics, not lifetime state, and are intentionally absent: resume
//! reproduces the trajectory, and observers re-derive identical summaries.
//!
//! Format: one JSON file holding a versioned payload plus a SHA-256
//! checksum over the canonical payload bytes. Writes go to a unique temp
//! file in the destination directory followed by an atomic rename, so a
//! crash never leaves a half-written checkpoint at the target path.
//! Bitwise replay is promised only on the recorded reference platform
//! (same code revision, same config, same seeds); cross-platform
//! comparisons need declared tolerances (see AGENTS.md).
//!
//! Information boundary: this module joins ordinary agent state with
//! evaluator-side hidden state inside one file. Agent code paths
//! (`agent::*`) never read the hidden half: the environment and agent
//! snapshots are built and restored through their own `pub(crate)`
//! entry points, and only this module holds both at once.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::agent::no_learning::{NoLearningActor, NoLearningError};
use crate::agent::weights::InheritedParams;
use crate::config::{Config, resolved_toml};
use crate::environment::{Lifetime, SimError};
use crate::experiments::continuous::{ContinuousError, ContinuousLearner};
use crate::experiments::episodic::{EpisodicError, EpisodicLearner};

/// Schema version for M1 checkpoint files. Bumped only with a documented
/// format change; older files are rejected, never silently migrated.
pub const CHECKPOINT_SCHEMA_VERSION: u32 = 2;

// Option fields in live snapshots must be present, even when null.
pub(crate) fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// Seed identity bound to every RNG position in the file. Restore
/// re-derives each stream's seed bytes from this tuple and rejects a
/// mismatch instead of reseeding silently.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedIdentity {
    pub root_seed: u64,
    pub namespace: String,
    pub outer_seed: u64,
    pub lifetime_index: u64,
}

/// Versioned checkpoint payload (everything the checksum covers).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointPayload {
    schema_version: u32,
    code_version: String,
    reference_os: String,
    reference_arch: String,
    config: Config,
    config_hash_sha256: String,
    seeds: SeedIdentity,
    env: crate::environment::LifetimeSnapshot,
    agent: crate::agent::no_learning::AgentSnapshot,
    inherited: InheritedParams,
}

/// Checkpoint file: payload plus integrity checksum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointFile {
    payload: CheckpointPayload,
    checksum_sha256: String,
}

/// Explicit checkpoint failures: schema, integrity, compatibility, and
/// I/O errors are distinct variants, never successful-looking output.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum CheckpointError {
    #[error("checkpoint I/O error for '{path}': {message}")]
    Io { path: String, message: String },
    #[error("checkpoint parse error for '{path}': {message}")]
    Parse { path: String, message: String },
    #[error("unsupported checkpoint schema_version {found}")]
    SchemaVersion { found: u32 },
    #[error("checkpoint checksum mismatch: file is corrupt or tampered")]
    ChecksumMismatch,
    #[error("incompatible checkpoint: {0}")]
    Incompatible(String),
    #[error("corrupt checkpoint: {0}")]
    Corrupt(String),
}

impl From<SimError> for CheckpointError {
    fn from(error: SimError) -> Self {
        match error {
            SimError::InconsistentCheckpoint(reason) => Self::Incompatible(reason),
            SimError::InvalidConfiguration(reason) => Self::Incompatible(reason),
            other => Self::Corrupt(other.to_string()),
        }
    }
}

impl From<NoLearningError> for CheckpointError {
    fn from(error: NoLearningError) -> Self {
        match error {
            NoLearningError::InvalidConfig(reason) | NoLearningError::BadSeed(reason) => {
                Self::Incompatible(reason)
            }
            other => Self::Corrupt(other.to_string()),
        }
    }
}

impl From<EpisodicError> for CheckpointError {
    fn from(error: EpisodicError) -> Self {
        match error {
            EpisodicError::InvalidConfig(reason) | EpisodicError::BadSeed(reason) => {
                Self::Incompatible(reason)
            }
            other => Self::Corrupt(other.to_string()),
        }
    }
}

impl From<ContinuousError> for CheckpointError {
    fn from(error: ContinuousError) -> Self {
        match error {
            ContinuousError::InvalidConfig(reason) => Self::Incompatible(reason),
            other => Self::Corrupt(other.to_string()),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0xf) as usize] as char);
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// A captured lifetime checkpoint: payload plus its integrity checksum.
///
/// Build with [`Checkpoint::capture`], persist with [`save_to_path`](Checkpoint::save_to_path),
/// read back with [`load_from_path`](Checkpoint::load_from_path), and
/// resume with [`restore_env`](Checkpoint::restore_env) plus
/// [`restore_actor`](Checkpoint::restore_actor).
#[derive(Clone, Debug, PartialEq)]
pub struct Checkpoint {
    payload: CheckpointPayload,
    checksum_sha256: String,
}

impl Checkpoint {
    /// Capture the full pause state at a tick boundary: environment
    /// driver, actor dynamics, inherited parameters, resolved config plus
    /// its hash, and the seed identity every RNG position is bound to.
    pub fn capture(
        lifetime: &Lifetime,
        actor: &NoLearningActor,
        cfg: &Config,
        seeds: SeedIdentity,
    ) -> Result<Self, CheckpointError> {
        crate::config::validate_actor_no_learning_execution(cfg).map_err(|e| {
            CheckpointError::Incompatible(format!("checkpoint config not executable: {e}"))
        })?;
        if cfg.actor.as_ref() != Some(actor.actor_config()) {
            return Err(CheckpointError::Incompatible(
                "live actor configuration differs from capture configuration".to_owned(),
            ));
        }
        let resolved = resolved_toml(cfg).map_err(|e| {
            CheckpointError::Corrupt(format!("cannot resolve checkpoint config: {e}"))
        })?;
        let payload = CheckpointPayload {
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            code_version: env!("CARGO_PKG_VERSION").to_owned(),
            reference_os: std::env::consts::OS.to_owned(),
            reference_arch: std::env::consts::ARCH.to_owned(),
            config: cfg.clone(),
            config_hash_sha256: sha256_hex(resolved.as_bytes()),
            env: lifetime.snapshot(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                seeds.lifetime_index,
            )?,
            agent: actor.snapshot(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                seeds.lifetime_index,
            )?,
            inherited: actor.inherited().clone(),
            seeds,
        };
        let checksum_sha256 = sha256_hex(
            &serde_json::to_vec(&payload)
                .map_err(|e| CheckpointError::Corrupt(format!("cannot encode payload: {e}")))?,
        );
        let checkpoint = Self {
            payload,
            checksum_sha256,
        };
        checkpoint.check_compatibility()?;
        Ok(checkpoint)
    }

    /// Seed identity recorded in the file.
    pub fn seeds(&self) -> &SeedIdentity {
        &self.payload.seeds
    }

    /// Resolved configuration recorded in the file.
    pub fn config(&self) -> &Config {
        &self.payload.config
    }

    /// Environment tick recorded in the file.
    pub fn tick(&self) -> u64 {
        self.payload.env.tick
    }

    /// Atomically persist the checkpoint: serialize, checksum, write a
    /// unique temp file in the destination directory, then rename over the
    /// target. A crash leaves the target untouched (or fully replaced),
    /// never half-written.
    pub fn save_to_path(&self, path: &Path) -> Result<(), CheckpointError> {
        let io = |message: String| CheckpointError::Io {
            path: path.display().to_string(),
            message,
        };
        let parent = path
            .parent()
            .ok_or_else(|| io("no parent directory".to_owned()))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| io("no file name".to_owned()))?
            .to_string_lossy()
            .into_owned();
        let file = CheckpointFile {
            payload: self.payload.clone(),
            checksum_sha256: self.checksum_sha256.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|e| CheckpointError::Corrupt(format!("cannot encode file: {e}")))?;
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        let (tmp, mut output) = loop {
            let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let tmp = parent.join(format!(".{file_name}.tmp.{}.{nonce}", std::process::id()));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
            {
                Ok(file) => break (tmp, file),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io(e.to_string())),
            }
        };
        if let Err(e) = output.write_all(&bytes).and_then(|_| output.sync_all()) {
            let _ = std::fs::remove_file(&tmp);
            return Err(io(e.to_string()));
        }
        drop(output);
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            io(e.to_string())
        })?;
        Ok(())
    }

    /// Read back a checkpoint file, rejecting unknown fields, schema
    /// mismatches, and checksum failures before any state is trusted.
    pub fn load_from_path(path: &Path) -> Result<Self, CheckpointError> {
        let io = |message: String| CheckpointError::Io {
            path: path.display().to_string(),
            message,
        };
        let parse = |message: String| CheckpointError::Parse {
            path: path.display().to_string(),
            message,
        };
        let bytes = std::fs::read(path).map_err(|e| io(e.to_string()))?;
        let file: CheckpointFile =
            serde_json::from_slice(&bytes).map_err(|e| parse(e.to_string()))?;
        if file.payload.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::SchemaVersion {
                found: file.payload.schema_version,
            });
        }
        let recomputed = sha256_hex(
            &serde_json::to_vec(&file.payload)
                .map_err(|e| CheckpointError::Corrupt(format!("cannot re-encode payload: {e}")))?,
        );
        if recomputed != file.checksum_sha256 {
            return Err(CheckpointError::ChecksumMismatch);
        }
        let checkpoint = Self {
            payload: file.payload,
            checksum_sha256: file.checksum_sha256,
        };
        checkpoint.check_compatibility()?;
        Ok(checkpoint)
    }

    /// Compatibility rules shared by both resume halves: executable
    /// config, matching config hash, matching inherited parameters, and
    /// agent/environment tick agreement (both advance once per tick, so a
    /// tick-boundary capture always agrees).
    fn check_seed_identity(&self, expected: &SeedIdentity) -> Result<(), CheckpointError> {
        if self.payload.seeds != *expected {
            return Err(CheckpointError::Incompatible(format!(
                "checkpoint recorded for lifetime {:?} but resume expects {:?}",
                self.payload.seeds, expected
            )));
        }
        Ok(())
    }

    fn check_compatibility(&self) -> Result<(), CheckpointError> {
        if self.payload.code_version != env!("CARGO_PKG_VERSION")
            || self.payload.reference_os != std::env::consts::OS
            || self.payload.reference_arch != std::env::consts::ARCH
        {
            return Err(CheckpointError::Incompatible(
                "checkpoint code version or reference platform differs".to_owned(),
            ));
        }
        crate::config::validate_actor_no_learning_execution(&self.payload.config).map_err(|e| {
            CheckpointError::Incompatible(format!("checkpoint config not executable: {e}"))
        })?;
        let resolved = resolved_toml(&self.payload.config).map_err(|e| {
            CheckpointError::Corrupt(format!("cannot resolve checkpoint config: {e}"))
        })?;
        if sha256_hex(resolved.as_bytes()) != self.payload.config_hash_sha256 {
            return Err(CheckpointError::Corrupt(
                "config hash does not match the stored configuration".to_owned(),
            ));
        }
        if self.payload.agent.ticks_advanced != self.payload.env.tick {
            return Err(CheckpointError::Incompatible(format!(
                "agent ticks {} disagree with environment tick {}",
                self.payload.agent.ticks_advanced, self.payload.env.tick
            )));
        }
        let p = &self.payload;
        let cfg = &p.config;
        let e = &p.env;
        let bad = |s: &str| CheckpointError::Incompatible(s.to_owned());
        let actor_cfg = cfg
            .actor
            .as_ref()
            .ok_or_else(|| bad("missing actor config"))?;
        p.inherited
            .validate(
                actor_cfg,
                crate::environment::feature_dim(cfg.environment.cue_count),
            )
            .map_err(|err| bad(&err.to_string()))?;
        if e.cue_count != cfg.environment.cue_count
            || e.cue_ticks != cfg.environment.cue_ticks
            || e.response_ticks != cfg.environment.response_ticks
            || e.quiet_range != cfg.environment.quiet_ticks
            || e.gap_range != cfg.environment.memory_gap_ticks
            || e.delay_range != cfg.environment.reward_delay_ticks
            || e.outcomes_target != cfg.simulation.outcomes_per_lifetime
            || e.warmup_ticks != cfg.simulation.warmup_ticks
        {
            return Err(bad(
                "stored environment differs from resolved configuration",
            ));
        }
        let id = &p.seeds;
        // Validate both halves even when the caller asks to restore only one.
        let env = Lifetime::restore(
            e.clone(),
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        let actor = NoLearningActor::restore(
            p.agent.clone(),
            actor_cfg.clone(),
            p.inherited.clone(),
            e.cue_count,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        actor.health_check().map_err(|err| bad(&err.to_string()))?;
        if e.confirmed != e.consumed || p.agent.last_feedback != e.consumed.last().copied() {
            return Err(bad(
                "agent feedback bookkeeping disagrees with delivered/confirmed events",
            ));
        }
        if matches!(e.phase, crate::environment::schedule::PhaseState::Committed) {
            return Err(bad("commitment must finish before capture"));
        }
        // Bind hidden static assignments to the recorded config/seed. Birth
        // draws are local here and never touch the live simulation streams.
        let birth = Lifetime::new(
            cfg,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        for cue in 0..e.cue_count {
            if env.hidden().epsilon(cue) != birth.hidden().epsilon(cue)
                || env.hidden().hazard(cue) != birth.hidden().hazard(cue)
                || env.hidden().role(cue) != birth.hidden().role(cue)
            {
                return Err(bad(
                    "hidden noise/hazard assignment differs from config and seed",
                ));
            }
        }
        let init = crate::rng::SeedTuple::new(
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            0,
            crate::rng::ACTOR_INIT_STREAM,
        );
        let init_hex = crate::rng::derive_seed_hex(&init).map_err(|err| bad(&err.to_string()))?;
        if p.inherited.topology.init_seed_hex != init_hex {
            return Err(bad(
                "inherited initialization seed differs from checkpoint identity",
            ));
        }
        Ok(())
    }

    /// Resume the environment driver. The caller declares which lifetime
    /// it is resuming via `expected`: a file recorded for another
    /// lifetime is rejected instead of silently continuing a foreign
    /// trajectory. Corrupt/incompatible state is rejected; missing state
    /// is never filled with silent defaults.
    pub fn restore_env(&self, expected: &SeedIdentity) -> Result<Lifetime, CheckpointError> {
        self.check_seed_identity(expected)?;
        self.check_compatibility()?;
        let seeds = &self.payload.seeds;
        let lifetime = Lifetime::restore(
            self.payload.env.clone(),
            seeds.root_seed,
            &seeds.namespace,
            seeds.outer_seed,
            seeds.lifetime_index,
        )?;
        Ok(lifetime)
    }

    /// Resume the actor. The actor config and cue count come from the
    /// checkpoint's own resolved config so a hand-swapped file cannot mix
    /// mismatched halves; `expected` must match the recorded seed identity.
    pub fn restore_actor(
        &self,
        expected: &SeedIdentity,
    ) -> Result<NoLearningActor, CheckpointError> {
        self.check_seed_identity(expected)?;
        self.check_compatibility()?;
        let seeds = &self.payload.seeds;
        let actor_cfg = self.payload.config.actor.clone().ok_or_else(|| {
            CheckpointError::Incompatible("checkpoint config has no [actor] section".to_owned())
        })?;
        let cue_count = self.payload.config.environment.cue_count;
        if self.payload.inherited.topology.neuron_count != actor_cfg.neuron_count {
            return Err(CheckpointError::Incompatible(
                "inherited topology does not match the checkpoint actor config".to_owned(),
            ));
        }
        let actor = NoLearningActor::restore(
            self.payload.agent.clone(),
            actor_cfg,
            self.payload.inherited.clone(),
            cue_count,
            seeds.root_seed,
            &seeds.namespace,
            seeds.outer_seed,
            seeds.lifetime_index,
        )?;
        Ok(actor)
    }
}

/// Schema version for learned-offset (plastic-learner) checkpoint files
/// (M3-10). Distinct from the M1 nonplastic schema 2: neither loader
/// reads the other's files (unknown fields fail to parse), so a
/// schema-2 file can never resume as a learning lifetime or vice versa.
pub const LEARNING_CHECKPOINT_SCHEMA_VERSION: u32 = 3;

/// Versioned learned-offset checkpoint payload (everything the checksum
/// covers): the M1 envelope shape with the episodic plastic-learner
/// snapshot (live `h`/`a`/`xi`/`q`, RNG positions, sampling record, and
/// the versioned `PlasticSnapshot` carrying `P`/`E`/baseline/dedup) in
/// place of the nonplastic agent snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LearningCheckpointPayload {
    schema_version: u32,
    code_version: String,
    reference_os: String,
    reference_arch: String,
    config: Config,
    config_hash_sha256: String,
    seeds: SeedIdentity,
    env: crate::environment::LifetimeSnapshot,
    agent: crate::experiments::episodic::EpisodicAgentSnapshot,
    inherited: InheritedParams,
}

/// Learned-offset checkpoint file: payload plus integrity checksum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LearningCheckpointFile {
    payload: LearningCheckpointPayload,
    checksum_sha256: String,
}

/// A captured learning-lifetime checkpoint: payload plus its integrity
/// checksum.
///
/// Build with [`LearningCheckpoint::capture`], persist with
/// [`save_to_path`](LearningCheckpoint::save_to_path), read back with
/// [`load_from_path`](LearningCheckpoint::load_from_path), and resume
/// with [`restore_env`](LearningCheckpoint::restore_env) plus
/// [`restore_learner`](LearningCheckpoint::restore_learner). Split runs
/// with nonzero `P`/`E` reproduce uninterrupted learning on the
/// recorded reference platform (M3-10); the M1 [`Checkpoint`] still
/// rejects these files instead of misreading them.
#[derive(Clone, Debug, PartialEq)]
pub struct LearningCheckpoint {
    payload: LearningCheckpointPayload,
    checksum_sha256: String,
}

impl LearningCheckpoint {
    /// Capture the full pause state at a tick boundary: environment
    /// driver, plastic learner dynamics plus offsets/traces/baseline/
    /// dedup, inherited parameters, resolved config plus its hash, and
    /// the seed identity every RNG position is bound to. Capture only
    /// at points the driver has fully processed (after any due commit);
    /// a mid-tick capture with an unfinished commitment is rejected.
    pub fn capture(
        lifetime: &Lifetime,
        learner: &EpisodicLearner,
        cfg: &Config,
        seeds: SeedIdentity,
    ) -> Result<Self, CheckpointError> {
        crate::config::validate_episodic_execution(cfg).map_err(|e| {
            CheckpointError::Incompatible(format!("checkpoint config not executable: {e}"))
        })?;
        if cfg.actor.as_ref() != Some(learner.actor_config()) {
            return Err(CheckpointError::Incompatible(
                "live actor configuration differs from capture configuration".to_owned(),
            ));
        }
        let learning = cfg.learning.clone().ok_or_else(|| {
            CheckpointError::Incompatible("checkpoint config has no [learning] section".to_owned())
        })?;
        // The stored learning identity must match the live plastic state;
        // a hand-swapped config cannot silently capture foreign plasticity.
        let bad = |s: &str| CheckpointError::Incompatible(s.to_owned());
        if learner.plastic().mask_kind().name() != learning.plastic_mask {
            return Err(bad("live plastic mask differs from capture configuration"));
        }
        if learner.plastic().trace_policy().name() != learning.trace_policy {
            return Err(bad("live trace policy differs from capture configuration"));
        }
        if learner.plastic().tau_e_config() != learning.tau_e {
            return Err(bad("live tau_e differs from capture configuration"));
        }
        if learner.plastic().plastic_bound() != learning.plastic_bound {
            return Err(bad("live plastic bound differs from capture configuration"));
        }
        let resolved = resolved_toml(cfg).map_err(|e| {
            CheckpointError::Corrupt(format!("cannot resolve checkpoint config: {e}"))
        })?;
        let payload = LearningCheckpointPayload {
            schema_version: LEARNING_CHECKPOINT_SCHEMA_VERSION,
            code_version: env!("CARGO_PKG_VERSION").to_owned(),
            reference_os: std::env::consts::OS.to_owned(),
            reference_arch: std::env::consts::ARCH.to_owned(),
            config: cfg.clone(),
            config_hash_sha256: sha256_hex(resolved.as_bytes()),
            env: lifetime.snapshot(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                seeds.lifetime_index,
            )?,
            agent: learner.snapshot(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                seeds.lifetime_index,
            )?,
            inherited: learner.inherited().clone(),
            seeds,
        };
        let checksum_sha256 = sha256_hex(
            &serde_json::to_vec(&payload)
                .map_err(|e| CheckpointError::Corrupt(format!("cannot encode payload: {e}")))?,
        );
        let checkpoint = Self {
            payload,
            checksum_sha256,
        };
        checkpoint.check_compatibility()?;
        Ok(checkpoint)
    }

    /// Seed identity recorded in the file.
    pub fn seeds(&self) -> &SeedIdentity {
        &self.payload.seeds
    }

    /// Resolved configuration recorded in the file.
    pub fn config(&self) -> &Config {
        &self.payload.config
    }

    /// Environment tick recorded in the file.
    pub fn tick(&self) -> u64 {
        self.payload.env.tick
    }

    /// Atomically persist the checkpoint: serialize, checksum, write a
    /// unique temp file in the destination directory, then rename over the
    /// target. A crash leaves the target untouched (or fully replaced),
    /// never half-written.
    pub fn save_to_path(&self, path: &Path) -> Result<(), CheckpointError> {
        let io = |message: String| CheckpointError::Io {
            path: path.display().to_string(),
            message,
        };
        let parent = path
            .parent()
            .ok_or_else(|| io("no parent directory".to_owned()))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| io("no file name".to_owned()))?
            .to_string_lossy()
            .into_owned();
        let file = LearningCheckpointFile {
            payload: self.payload.clone(),
            checksum_sha256: self.checksum_sha256.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|e| CheckpointError::Corrupt(format!("cannot encode file: {e}")))?;
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        let (tmp, mut output) = loop {
            let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let tmp = parent.join(format!(".{file_name}.tmp.{}.{nonce}", std::process::id()));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
            {
                Ok(file) => break (tmp, file),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io(e.to_string())),
            }
        };
        if let Err(e) = output.write_all(&bytes).and_then(|_| output.sync_all()) {
            let _ = std::fs::remove_file(&tmp);
            return Err(io(e.to_string()));
        }
        drop(output);
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            io(e.to_string())
        })?;
        Ok(())
    }

    /// Read back a learned-offset checkpoint file, rejecting unknown
    /// fields, schema mismatches, and checksum failures before any state
    /// is trusted. M1 schema-2 files fail here (missing plastic state),
    /// exactly as these files fail the M1 loader.
    pub fn load_from_path(path: &Path) -> Result<Self, CheckpointError> {
        let io = |message: String| CheckpointError::Io {
            path: path.display().to_string(),
            message,
        };
        let parse = |message: String| CheckpointError::Parse {
            path: path.display().to_string(),
            message,
        };
        let bytes = std::fs::read(path).map_err(|e| io(e.to_string()))?;
        let file: LearningCheckpointFile =
            serde_json::from_slice(&bytes).map_err(|e| parse(e.to_string()))?;
        if file.payload.schema_version != LEARNING_CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::SchemaVersion {
                found: file.payload.schema_version,
            });
        }
        let recomputed = sha256_hex(
            &serde_json::to_vec(&file.payload)
                .map_err(|e| CheckpointError::Corrupt(format!("cannot re-encode payload: {e}")))?,
        );
        if recomputed != file.checksum_sha256 {
            return Err(CheckpointError::ChecksumMismatch);
        }
        let checkpoint = Self {
            payload: file.payload,
            checksum_sha256: file.checksum_sha256,
        };
        checkpoint.check_compatibility()?;
        Ok(checkpoint)
    }

    /// Compatibility rules shared by both resume halves: executable
    /// config, matching config hash, matching inherited parameters, and
    /// agent/environment tick agreement (both advance once per tick, so a
    /// tick-boundary capture always agrees).
    fn check_seed_identity(&self, expected: &SeedIdentity) -> Result<(), CheckpointError> {
        if self.payload.seeds != *expected {
            return Err(CheckpointError::Incompatible(format!(
                "checkpoint recorded for lifetime {:?} but resume expects {:?}",
                self.payload.seeds, expected
            )));
        }
        Ok(())
    }

    fn check_compatibility(&self) -> Result<(), CheckpointError> {
        if self.payload.code_version != env!("CARGO_PKG_VERSION")
            || self.payload.reference_os != std::env::consts::OS
            || self.payload.reference_arch != std::env::consts::ARCH
        {
            return Err(CheckpointError::Incompatible(
                "checkpoint code version or reference platform differs".to_owned(),
            ));
        }
        crate::config::validate_episodic_execution(&self.payload.config).map_err(|e| {
            CheckpointError::Incompatible(format!("checkpoint config not executable: {e}"))
        })?;
        let resolved = resolved_toml(&self.payload.config).map_err(|e| {
            CheckpointError::Corrupt(format!("cannot resolve checkpoint config: {e}"))
        })?;
        if sha256_hex(resolved.as_bytes()) != self.payload.config_hash_sha256 {
            return Err(CheckpointError::Corrupt(
                "config hash does not match the stored configuration".to_owned(),
            ));
        }
        if self.payload.agent.ticks_advanced != self.payload.env.tick {
            return Err(CheckpointError::Incompatible(format!(
                "agent ticks {} disagree with environment tick {}",
                self.payload.agent.ticks_advanced, self.payload.env.tick
            )));
        }
        let p = &self.payload;
        let cfg = &p.config;
        let e = &p.env;
        let bad = |s: &str| CheckpointError::Incompatible(s.to_owned());
        let actor_cfg = cfg
            .actor
            .as_ref()
            .ok_or_else(|| bad("missing actor config"))?;
        let learning = cfg
            .learning
            .as_ref()
            .ok_or_else(|| bad("missing learning config"))?;
        p.inherited
            .validate(
                actor_cfg,
                crate::environment::feature_dim(cfg.environment.cue_count),
            )
            .map_err(|err| bad(&err.to_string()))?;
        if e.cue_count != cfg.environment.cue_count
            || e.cue_ticks != cfg.environment.cue_ticks
            || e.response_ticks != cfg.environment.response_ticks
            || e.quiet_range != cfg.environment.quiet_ticks
            || e.gap_range != cfg.environment.memory_gap_ticks
            || e.delay_range != cfg.environment.reward_delay_ticks
            || e.outcomes_target != cfg.simulation.outcomes_per_lifetime
            || e.warmup_ticks != cfg.simulation.warmup_ticks
        {
            return Err(bad(
                "stored environment differs from resolved configuration",
            ));
        }
        let id = &p.seeds;
        // Validate both halves even when the caller asks to restore only one.
        let env = Lifetime::restore(
            e.clone(),
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        let learner = EpisodicLearner::restore(
            p.agent.clone(),
            actor_cfg.clone(),
            learning.clone(),
            p.inherited.clone(),
            e.cue_count,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        // The resumed offsets must still satisfy the resolved bound; the
        // restore above already enforces mask/trace/bound agreement.
        if learner.plastic().plastic_bound() != learning.plastic_bound {
            return Err(bad("restored plastic bound disagrees with configuration"));
        }
        if e.confirmed != e.consumed || p.agent.plastic.last_feedback != e.consumed.last().copied()
        {
            return Err(bad(
                "learner feedback bookkeeping disagrees with delivered/confirmed events",
            ));
        }
        if matches!(e.phase, crate::environment::schedule::PhaseState::Committed) {
            return Err(bad("commitment must finish before capture"));
        }
        // Bind hidden static assignments to the recorded config/seed. Birth
        // draws are local here and never touch the live simulation streams.
        let birth = Lifetime::new(
            cfg,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        for cue in 0..e.cue_count {
            if env.hidden().epsilon(cue) != birth.hidden().epsilon(cue)
                || env.hidden().hazard(cue) != birth.hidden().hazard(cue)
                || env.hidden().role(cue) != birth.hidden().role(cue)
            {
                return Err(bad(
                    "hidden noise/hazard assignment differs from config and seed",
                ));
            }
        }
        let init = crate::rng::SeedTuple::new(
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            0,
            crate::rng::ACTOR_INIT_STREAM,
        );
        let init_hex = crate::rng::derive_seed_hex(&init).map_err(|err| bad(&err.to_string()))?;
        if p.inherited.topology.init_seed_hex != init_hex {
            return Err(bad(
                "inherited initialization seed differs from checkpoint identity",
            ));
        }
        Ok(())
    }

    /// Resume the environment driver. The caller declares which lifetime
    /// it is resuming via `expected`: a file recorded for another
    /// lifetime is rejected instead of silently continuing a foreign
    /// trajectory. Corrupt/incompatible state is rejected; missing state
    /// is never filled with silent defaults.
    pub fn restore_env(&self, expected: &SeedIdentity) -> Result<Lifetime, CheckpointError> {
        self.check_seed_identity(expected)?;
        self.check_compatibility()?;
        let seeds = &self.payload.seeds;
        let lifetime = Lifetime::restore(
            self.payload.env.clone(),
            seeds.root_seed,
            &seeds.namespace,
            seeds.outer_seed,
            seeds.lifetime_index,
        )?;
        Ok(lifetime)
    }

    /// Resume the plastic learner. The actor/learning config and cue
    /// count come from the checkpoint's own resolved config so a
    /// hand-swapped file cannot mix mismatched halves; `expected` must
    /// match the recorded seed identity.
    pub fn restore_learner(
        &self,
        expected: &SeedIdentity,
    ) -> Result<EpisodicLearner, CheckpointError> {
        self.check_seed_identity(expected)?;
        self.check_compatibility()?;
        let seeds = &self.payload.seeds;
        let actor_cfg = self.payload.config.actor.clone().ok_or_else(|| {
            CheckpointError::Incompatible("checkpoint config has no [actor] section".to_owned())
        })?;
        let learning = self.payload.config.learning.clone().ok_or_else(|| {
            CheckpointError::Incompatible("checkpoint config has no [learning] section".to_owned())
        })?;
        let cue_count = self.payload.config.environment.cue_count;
        if self.payload.inherited.topology.neuron_count != actor_cfg.neuron_count {
            return Err(CheckpointError::Incompatible(
                "inherited topology does not match the checkpoint actor config".to_owned(),
            ));
        }
        let learner = EpisodicLearner::restore(
            self.payload.agent.clone(),
            actor_cfg,
            learning,
            self.payload.inherited.clone(),
            cue_count,
            seeds.root_seed,
            &seeds.namespace,
            seeds.outer_seed,
            seeds.lifetime_index,
        )?;
        Ok(learner)
    }
}

/// Schema version for fully persistent continuous-learning checkpoints
/// (M4-06). This is a separate envelope from M1 schema 2 and episodic
/// schema 3; no loader silently migrates between the three contracts.
pub const CONTINUOUS_CHECKPOINT_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuousCheckpointPayload {
    schema_version: u32,
    code_version: String,
    reference_os: String,
    reference_arch: String,
    config: Config,
    config_hash_sha256: String,
    seeds: SeedIdentity,
    env: crate::environment::LifetimeSnapshot,
    agent: crate::experiments::continuous::ContinuousAgentSnapshot,
    inherited: InheritedParams,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuousCheckpointFile {
    payload: ContinuousCheckpointPayload,
    checksum_sha256: String,
}

/// Exact pause/resume envelope for the fully persistent continuous learner.
///
/// The environment half carries phase/countdowns, pending feedback,
/// previous-action latch, hidden schedule state, delivery/confirmation
/// ledgers, and all live environment RNGs. The learner half carries live
/// neural/adaptation/motor state, both agent RNGs, `P`, `E`, baseline,
/// feedback deduplication, and an asserted effective-weight cache. Inherited
/// parameters and the fully resolved config travel in this envelope.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousCheckpoint {
    payload: ContinuousCheckpointPayload,
    checksum_sha256: String,
}

impl ContinuousCheckpoint {
    /// Capture only at a fully processed tick boundary. A due feedback may
    /// still be pending (the immediately-before-feedback boundary), but a
    /// delivered feedback must already be applied and confirmed. The
    /// transient commitment phase is rejected.
    pub fn capture(
        lifetime: &Lifetime,
        learner: &ContinuousLearner,
        cfg: &Config,
        seeds: SeedIdentity,
    ) -> Result<Self, CheckpointError> {
        crate::config::validate_continuous_execution(cfg).map_err(|e| {
            CheckpointError::Incompatible(format!("checkpoint config not executable: {e}"))
        })?;
        if cfg.actor.as_ref() != Some(learner.actor_config()) {
            return Err(CheckpointError::Incompatible(
                "live actor configuration differs from capture configuration".to_owned(),
            ));
        }
        let learning = cfg.learning.as_ref().ok_or_else(|| {
            CheckpointError::Incompatible("checkpoint config has no [learning] section".to_owned())
        })?;
        let bad = |s: &str| CheckpointError::Incompatible(s.to_owned());
        if learner.plastic().mask_kind().name() != learning.plastic_mask {
            return Err(bad("live plastic mask differs from capture configuration"));
        }
        if learner.plastic().trace_policy().name() != learning.trace_policy {
            return Err(bad("live trace policy differs from capture configuration"));
        }
        if learner.plastic().tau_e_config() != learning.tau_e {
            return Err(bad("live tau_e differs from capture configuration"));
        }
        if learner.plastic().plastic_bound() != learning.plastic_bound {
            return Err(bad("live plastic bound differs from capture configuration"));
        }
        let resolved = resolved_toml(cfg).map_err(|e| {
            CheckpointError::Corrupt(format!("cannot resolve checkpoint config: {e}"))
        })?;
        let payload = ContinuousCheckpointPayload {
            schema_version: CONTINUOUS_CHECKPOINT_SCHEMA_VERSION,
            code_version: env!("CARGO_PKG_VERSION").to_owned(),
            reference_os: std::env::consts::OS.to_owned(),
            reference_arch: std::env::consts::ARCH.to_owned(),
            config: cfg.clone(),
            config_hash_sha256: sha256_hex(resolved.as_bytes()),
            env: lifetime.snapshot(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                seeds.lifetime_index,
            )?,
            agent: learner.snapshot(
                seeds.root_seed,
                &seeds.namespace,
                seeds.outer_seed,
                seeds.lifetime_index,
            )?,
            inherited: learner.inherited().clone(),
            seeds,
        };
        let checksum_sha256 = sha256_hex(
            &serde_json::to_vec(&payload)
                .map_err(|e| CheckpointError::Corrupt(format!("cannot encode payload: {e}")))?,
        );
        let checkpoint = Self {
            payload,
            checksum_sha256,
        };
        checkpoint.check_compatibility()?;
        Ok(checkpoint)
    }

    pub fn seeds(&self) -> &SeedIdentity {
        &self.payload.seeds
    }

    pub fn config(&self) -> &Config {
        &self.payload.config
    }

    pub fn tick(&self) -> u64 {
        self.payload.env.tick
    }

    /// Persist with the established checksum + unique-temp + fsync + atomic
    /// rename pattern used by schemas 2 and 3.
    pub fn save_to_path(&self, path: &Path) -> Result<(), CheckpointError> {
        let io = |message: String| CheckpointError::Io {
            path: path.display().to_string(),
            message,
        };
        let parent = path
            .parent()
            .ok_or_else(|| io("no parent directory".to_owned()))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| io("no file name".to_owned()))?
            .to_string_lossy()
            .into_owned();
        let file = ContinuousCheckpointFile {
            payload: self.payload.clone(),
            checksum_sha256: self.checksum_sha256.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|e| CheckpointError::Corrupt(format!("cannot encode file: {e}")))?;
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        let (tmp, mut output) = loop {
            let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let tmp = parent.join(format!(".{file_name}.tmp.{}.{nonce}", std::process::id()));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
            {
                Ok(file) => break (tmp, file),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io(e.to_string())),
            }
        };
        if let Err(e) = output.write_all(&bytes).and_then(|_| output.sync_all()) {
            let _ = std::fs::remove_file(&tmp);
            return Err(io(e.to_string()));
        }
        drop(output);
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            io(e.to_string())
        })?;
        Ok(())
    }

    /// Load only complete schema-4 continuous checkpoints. Missing or
    /// unknown fields, old schemas, bad checksums, and incompatible state
    /// are explicit failures.
    pub fn load_from_path(path: &Path) -> Result<Self, CheckpointError> {
        let io = |message: String| CheckpointError::Io {
            path: path.display().to_string(),
            message,
        };
        let parse = |message: String| CheckpointError::Parse {
            path: path.display().to_string(),
            message,
        };
        let bytes = std::fs::read(path).map_err(|e| io(e.to_string()))?;
        let file: ContinuousCheckpointFile =
            serde_json::from_slice(&bytes).map_err(|e| parse(e.to_string()))?;
        if file.payload.schema_version != CONTINUOUS_CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::SchemaVersion {
                found: file.payload.schema_version,
            });
        }
        let recomputed = sha256_hex(
            &serde_json::to_vec(&file.payload)
                .map_err(|e| CheckpointError::Corrupt(format!("cannot re-encode payload: {e}")))?,
        );
        if recomputed != file.checksum_sha256 {
            return Err(CheckpointError::ChecksumMismatch);
        }
        let checkpoint = Self {
            payload: file.payload,
            checksum_sha256: file.checksum_sha256,
        };
        checkpoint.check_compatibility()?;
        Ok(checkpoint)
    }

    fn check_seed_identity(&self, expected: &SeedIdentity) -> Result<(), CheckpointError> {
        if self.payload.seeds != *expected {
            return Err(CheckpointError::Incompatible(format!(
                "checkpoint recorded for lifetime {:?} but resume expects {:?}",
                self.payload.seeds, expected
            )));
        }
        Ok(())
    }

    fn check_compatibility(&self) -> Result<(), CheckpointError> {
        if self.payload.code_version != env!("CARGO_PKG_VERSION")
            || self.payload.reference_os != std::env::consts::OS
            || self.payload.reference_arch != std::env::consts::ARCH
        {
            return Err(CheckpointError::Incompatible(
                "checkpoint code version or reference platform differs".to_owned(),
            ));
        }
        crate::config::validate_continuous_execution(&self.payload.config).map_err(|e| {
            CheckpointError::Incompatible(format!("checkpoint config not executable: {e}"))
        })?;
        let resolved = resolved_toml(&self.payload.config).map_err(|e| {
            CheckpointError::Corrupt(format!("cannot resolve checkpoint config: {e}"))
        })?;
        if sha256_hex(resolved.as_bytes()) != self.payload.config_hash_sha256 {
            return Err(CheckpointError::Corrupt(
                "config hash does not match the stored configuration".to_owned(),
            ));
        }
        if self.payload.agent.ticks_advanced != self.payload.env.tick {
            return Err(CheckpointError::Incompatible(format!(
                "agent ticks {} disagree with environment tick {}",
                self.payload.agent.ticks_advanced, self.payload.env.tick
            )));
        }
        let p = &self.payload;
        let cfg = &p.config;
        let e = &p.env;
        let bad = |s: &str| CheckpointError::Incompatible(s.to_owned());
        let actor_cfg = cfg
            .actor
            .as_ref()
            .ok_or_else(|| bad("missing actor config"))?;
        let learning = cfg
            .learning
            .as_ref()
            .ok_or_else(|| bad("missing learning config"))?;
        p.inherited
            .validate(
                actor_cfg,
                crate::environment::feature_dim(cfg.environment.cue_count),
            )
            .map_err(|err| bad(&err.to_string()))?;
        if e.cue_count != cfg.environment.cue_count
            || e.cue_ticks != cfg.environment.cue_ticks
            || e.response_ticks != cfg.environment.response_ticks
            || e.quiet_range != cfg.environment.quiet_ticks
            || e.gap_range != cfg.environment.memory_gap_ticks
            || e.delay_range != cfg.environment.reward_delay_ticks
            || e.outcomes_target != cfg.simulation.outcomes_per_lifetime
            || e.warmup_ticks != cfg.simulation.warmup_ticks
        {
            return Err(bad(
                "stored environment differs from resolved configuration",
            ));
        }
        let id = &p.seeds;
        let env = Lifetime::restore(
            e.clone(),
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        let learner = ContinuousLearner::restore(
            p.agent.clone(),
            actor_cfg.clone(),
            learning.clone(),
            p.inherited.clone(),
            e.cue_count,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        if learner.plastic().plastic_bound() != learning.plastic_bound {
            return Err(bad("restored plastic bound disagrees with configuration"));
        }
        if learner.plastic().effective_weights() != p.agent.effective_weights {
            return Err(bad(
                "restored effective-weight cache disagrees with checkpoint assertion",
            ));
        }
        if e.confirmed != e.consumed || p.agent.plastic.last_feedback != e.consumed.last().copied()
        {
            return Err(bad(
                "learner feedback bookkeeping disagrees with delivered/confirmed events",
            ));
        }
        if matches!(e.phase, crate::environment::schedule::PhaseState::Committed) {
            return Err(bad("commitment must finish before capture"));
        }
        let birth = Lifetime::new(
            cfg,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )?;
        for cue in 0..e.cue_count {
            if env.hidden().epsilon(cue) != birth.hidden().epsilon(cue)
                || env.hidden().hazard(cue) != birth.hidden().hazard(cue)
                || env.hidden().role(cue) != birth.hidden().role(cue)
            {
                return Err(bad(
                    "hidden noise/hazard assignment differs from config and seed",
                ));
            }
        }
        let init = crate::rng::SeedTuple::new(
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            0,
            crate::rng::ACTOR_INIT_STREAM,
        );
        let init_hex = crate::rng::derive_seed_hex(&init).map_err(|err| bad(&err.to_string()))?;
        if p.inherited.topology.init_seed_hex != init_hex {
            return Err(bad(
                "inherited initialization seed differs from checkpoint identity",
            ));
        }
        Ok(())
    }

    pub fn restore_env(&self, expected: &SeedIdentity) -> Result<Lifetime, CheckpointError> {
        self.check_seed_identity(expected)?;
        self.check_compatibility()?;
        let seeds = &self.payload.seeds;
        Ok(Lifetime::restore(
            self.payload.env.clone(),
            seeds.root_seed,
            &seeds.namespace,
            seeds.outer_seed,
            seeds.lifetime_index,
        )?)
    }

    pub fn restore_learner(
        &self,
        expected: &SeedIdentity,
    ) -> Result<ContinuousLearner, CheckpointError> {
        self.check_seed_identity(expected)?;
        self.check_compatibility()?;
        let seeds = &self.payload.seeds;
        let actor_cfg = self.payload.config.actor.clone().ok_or_else(|| {
            CheckpointError::Incompatible("checkpoint config has no [actor] section".to_owned())
        })?;
        let learning = self.payload.config.learning.clone().ok_or_else(|| {
            CheckpointError::Incompatible("checkpoint config has no [learning] section".to_owned())
        })?;
        let cue_count = self.payload.config.environment.cue_count;
        if self.payload.inherited.topology.neuron_count != actor_cfg.neuron_count {
            return Err(CheckpointError::Incompatible(
                "inherited topology does not match the checkpoint actor config".to_owned(),
            ));
        }
        Ok(ContinuousLearner::restore(
            self.payload.agent.clone(),
            actor_cfg,
            learning,
            self.payload.inherited.clone(),
            cue_count,
            seeds.root_seed,
            &seeds.namespace,
            seeds.outer_seed,
            seeds.lifetime_index,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_is_pinned() {
        assert_eq!(CHECKPOINT_SCHEMA_VERSION, 2);
    }

    #[test]
    fn learning_schema_version_is_pinned() {
        // The plastic-learner envelope is a separate version line: M1
        // schema 2 files never gain plastic fields by migration.
        assert_eq!(LEARNING_CHECKPOINT_SCHEMA_VERSION, 3);
    }

    #[test]
    fn continuous_schema_version_is_pinned() {
        assert_eq!(CONTINUOUS_CHECKPOINT_SCHEMA_VERSION, 4);
    }

    fn test_capture() -> (Checkpoint, SeedIdentity) {
        let mut cfg: Config =
            toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").unwrap()).unwrap();
        cfg.profile_name = "checkpoint_test".to_owned();
        cfg.simulation.outcomes_per_lifetime = 6;
        cfg.environment.reward_delay_ticks = [3, 3];
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
        let id = SeedIdentity {
            root_seed: 1,
            namespace: "development".to_owned(),
            outer_seed: 1,
            lifetime_index: 0,
        };
        let life = Lifetime::new(&cfg, 1, "development", 1, 0).unwrap();
        let actor = NoLearningActor::new(&cfg, 1, "development", 1, 0).unwrap();
        let checkpoint = Checkpoint::capture(&life, &actor, &cfg, id.clone()).unwrap();
        (checkpoint, id)
    }

    fn test_continuous_capture() -> (ContinuousCheckpoint, SeedIdentity) {
        let mut cfg: Config =
            toml::from_str(&std::fs::read_to_string("configs/continuous_stationary.toml").unwrap())
                .unwrap();
        cfg.simulation.outcomes_per_lifetime = 4;
        let id = SeedIdentity {
            root_seed: 1,
            namespace: "development".to_owned(),
            outer_seed: 2,
            lifetime_index: 0,
        };
        let (inherited, _) = crate::experiments::continuous::sample_matched_inheritance(
            &cfg,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
        )
        .unwrap();
        let noise = crate::rng::rng_for(&crate::rng::SeedTuple::new(
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
            "actor_noise",
        ))
        .unwrap();
        let tie = crate::rng::rng_for(&crate::rng::SeedTuple::new(
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
            "tie_break",
        ))
        .unwrap();
        let learner = ContinuousLearner::from_agent_parts(
            cfg.actor.clone().unwrap(),
            cfg.learning.clone().unwrap(),
            inherited,
            cfg.environment.cue_count,
            noise,
            tie,
        )
        .unwrap();
        let lifetime = Lifetime::new(
            &cfg,
            id.root_seed,
            &id.namespace,
            id.outer_seed,
            id.lifetime_index,
        )
        .unwrap();
        let checkpoint =
            ContinuousCheckpoint::capture(&lifetime, &learner, &cfg, id.clone()).unwrap();
        (checkpoint, id)
    }

    #[test]
    fn continuous_derived_cache_disagreement_is_incompatible() {
        let (mut checkpoint, id) = test_continuous_capture();
        checkpoint.payload.agent.effective_weights[0][0] += 1.0;
        assert!(matches!(
            checkpoint.restore_env(&id),
            Err(CheckpointError::Incompatible(_))
        ));
        assert!(matches!(
            checkpoint.restore_learner(&id),
            Err(CheckpointError::Incompatible(_))
        ));
    }

    #[test]
    fn continuous_resolved_learning_mismatch_is_incompatible() {
        let (mut checkpoint, id) = test_continuous_capture();
        checkpoint.payload.config.learning.as_mut().unwrap().tau_e += 1.0;
        let resolved = resolved_toml(&checkpoint.payload.config).unwrap();
        checkpoint.payload.config_hash_sha256 = sha256_hex(resolved.as_bytes());
        assert!(matches!(
            checkpoint.restore_env(&id),
            Err(CheckpointError::Incompatible(_))
        ));
        assert!(matches!(
            checkpoint.restore_learner(&id),
            Err(CheckpointError::Incompatible(_))
        ));
    }

    #[test]
    fn continuous_loader_rejects_schema_2_checkpoint() {
        let (checkpoint, _) = test_capture();
        let path = std::env::temp_dir().join(format!(
            "cra-schema2-continuous-cross-load-{}.json",
            std::process::id()
        ));
        checkpoint.save_to_path(&path).unwrap();
        assert!(ContinuousCheckpoint::load_from_path(&path).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn tick_disagreement_is_incompatible() {
        let (mut checkpoint, id) = test_capture();
        // Tick-boundary captures always agree (both halves advance once
        // per tick); skewing either side is corruption, not a resume.
        checkpoint.payload.agent.ticks_advanced += 5;
        assert!(matches!(
            checkpoint.restore_env(&id),
            Err(CheckpointError::Incompatible(_))
        ));
        assert!(matches!(
            checkpoint.restore_actor(&id),
            Err(CheckpointError::Incompatible(_))
        ));
    }

    #[test]
    fn config_dimension_drift_is_incompatible() {
        let (mut checkpoint, id) = test_capture();
        // The recorded config (K = 5 here) no longer fits the stored
        // sensory projection (width K + 6 = 8 for K = 2). The config hash
        // is repaired so the failure comes from the dimension check
        // itself, not the tamper evidence.
        checkpoint.payload.config.environment.cue_count = 5;
        let resolved = resolved_toml(&checkpoint.payload.config).unwrap();
        checkpoint.payload.config_hash_sha256 = sha256_hex(resolved.as_bytes());
        assert!(matches!(
            checkpoint.restore_actor(&id),
            Err(CheckpointError::Incompatible(_))
        ));
    }

    #[test]
    fn pending_phase_mismatch_is_incompatible() {
        let (mut checkpoint, id) = test_capture();
        // Birth phase is Quiet with no pending reward; claiming Feedback
        // without one is corruption, not a resume point.
        checkpoint.payload.env.phase = crate::environment::schedule::PhaseState::Feedback;
        assert!(matches!(
            checkpoint.restore_env(&id),
            Err(CheckpointError::Incompatible(_))
        ));
    }
    #[test]
    fn corrupt_state_is_rejected_by_both_restore_halves() {
        let (original, id) = test_capture();
        let mutations: Vec<(&str, Checkpoint)> = (0..9)
            .map(|case| {
                let mut cp = original.clone();
                let name = match case {
                    0 => {
                        cp.payload.inherited.weights.w0[0].pop();
                        "ragged W0"
                    }
                    1 => {
                        cp.payload.inherited.weights.w0[0][0] = 1.0;
                        "missing edge weight"
                    }
                    2 => {
                        cp.payload.agent.last_output.action_0 = 0.4;
                        "stale readout"
                    }
                    3 => {
                        cp.payload.agent.last_feedback = Some(9);
                        "foreign feedback"
                    }
                    4 => {
                        cp.payload.env.commitments = 5;
                        cp.payload.env.next_event_id = 5;
                        "lost pending choice"
                    }
                    5 => {
                        cp.payload.env.cue_ticks += 1;
                        "config drift"
                    }
                    6 => {
                        cp.payload.agent.noise_rng.word_pos = 1_u128 << 68;
                        "wrapped RNG position"
                    }
                    7 => {
                        cp.payload.reference_arch = "foreign".into();
                        "platform mismatch"
                    }
                    _ => {
                        cp.payload.inherited.topology.motor0[0] = 999;
                        "invalid motor pool"
                    }
                };
                (name, cp)
            })
            .collect();
        for (name, cp) in mutations {
            assert!(cp.restore_actor(&id).is_err(), "{name}: actor");
            assert!(cp.restore_env(&id).is_err(), "{name}: env");
        }
    }
}
