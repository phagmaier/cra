//! Run-directory and provenance scaffolding (M0-03 / M0-05).
//!
//! Every run gets a unique directory holding the fully resolved
//! configuration, a manifest with code/seed identity, and the derived seed
//! stream table. Raw event output stays immutable once written; analysis
//! writes derived outputs elsewhere. Large generated data and build output
//! stay out of ordinary source commits (see README and `.gitignore`).
//!
//! Full provenance (hidden-stream separation, failure records, hardware
//! details) arrives with the logging milestone (M0-13). This module provides
//! the directory convention and the manifest fields available at M0.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::config::{Config, resolved_toml};
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
    pub schema_version: u32,
    pub created_unix_secs: u64,
    pub code_version: String,
    pub git_revision: String,
    pub git_dirty: Option<bool>,
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
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(m) => write!(f, "run I/O error: {m}"),
            Self::Seed(e) => write!(f, "run seed error: {e}"),
            Self::Config(e) => write!(f, "run config error: {e}"),
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
    base: &Path,
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
    let mut dir = base.join(&stem);
    for retry in 0..100 {
        if retry > 0 {
            dir = base.join(format!("{stem}-retry{retry}"));
        }
        match std::fs::create_dir_all(&dir) {
            Ok(()) => break,
            Err(e) => {
                if retry == 99 {
                    return Err(RunError::Io(e.to_string()));
                }
            }
        }
        // create_dir_all succeeds even if the dir existed; require our
        // marker to be absent so two runs never share a directory.
        if !dir.join("manifest.json").exists() {
            break;
        }
        if retry == 99 {
            return Err(RunError::Io(
                "cannot allocate a unique run directory".to_owned(),
            ));
        }
    }

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
        schema_version: cfg.schema_version,
        created_unix_secs: ts,
        code_version: env!("CARGO_PKG_VERSION").to_owned(),
        git_revision,
        git_dirty,
        seed_policy: "(root_seed, namespace, outer_seed, lifetime_index, stream_name) -> SHA-256 -> ChaCha8Rng; namespaces development|training|validation|final_test are disjoint (spec 20.5)".to_owned(),
        seeds: seeds.clone(),
        rng_policy: "sha2::Sha256 derivation (see Cargo.lock) + rand_chacha::ChaCha8Rng; one RNG instance per stream; no runtime-randomized hashes".to_owned(),
        note: "M0 scaffold: validation + provenance only. Full environment stepping arrives in M0-07+; neural/search code in later milestones.".to_owned(),
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
}
