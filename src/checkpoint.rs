//! First full lifetime checkpoint: exact pause/resume (M1-09).
//!
//! Spec: 10.7 (checkpoint carries neural state, adaptation, motor filters,
//! environment phase plus pending reward/action latch, consumed-feedback
//! bookkeeping, RNG states/counters, inherited parameters, and resolved
//! configuration identifiers — a genome alone is not a checkpoint), 10.6
//! (no silent clipping; failures are explicit), 20 (deterministic streams
//! resume exactly).
//!
//! Scope: the nonplastic M1 actor plus the continuous environment. Plastic
//! offsets, eligibility, modulator state, and gates do not exist yet;
//! [`Checkpoint`] rejects files that claim them (unknown fields) instead
//! of silently defaulting them. Health summaries and traces (M1-08) are
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
    #[error(
        "unsupported checkpoint schema_version {found}; this implementation reads {CHECKPOINT_SCHEMA_VERSION}"
    )]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_is_pinned() {
        assert_eq!(CHECKPOINT_SCHEMA_VERSION, 2);
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
