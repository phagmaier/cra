//! First full lifetime checkpoint contracts (M1-09).
//!
//! - Resume at quiet, response (just-committed delay), and pending-feedback
//!   boundaries reproduces the uninterrupted reference continuation
//!   bitwise on this platform: per-tick features, feedback, motor outputs,
//!   actions, final neural/environment/hidden state, and continued health
//!   summaries all agree. Save/load round-trips through a real file on
//!   every boundary.
//! - Health observation extends across the split: both halves stay finite
//!   and the continued summary equals the reference summary.
//! - Corrupt files (tampered payload, wrong schema, truncation, unknown
//!   fields, missing path) and incompatible in-memory mutations (seed
//!   identity, config dims, tick agreement) fail explicitly; missing state
//!   is never defaulted.
//!
//! Bitwise identity is asserted because both halves run on this reference
//! platform in one process. Cross-platform comparisons need declared
//! tolerances (see AGENTS.md); these tests do not promise them.

use cra::agent::health::HealthSummary;
use cra::agent::no_learning::NoLearningActor;
use cra::checkpoint::{CHECKPOINT_SCHEMA_VERSION, Checkpoint, CheckpointError, SeedIdentity};
use cra::config::{Actor, Config};
use cra::environment::{Agent, Lifetime, MotorOutput, Phase};

fn actor_cfg() -> Actor {
    Actor {
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
    }
}

fn checkpoint_config() -> Config {
    let mut cfg: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("smoke parses");
    cfg.profile_name = "checkpoint_test".to_owned();
    cfg.simulation.outcomes_per_lifetime = 6;
    // Delay 3 exposes a real Delay phase so the just-committed response
    // boundary and the pending-feedback boundary are distinct ticks.
    cfg.environment.reward_delay_ticks = [3, 3];
    cfg.actor = Some(actor_cfg());
    cfg.learning = None;
    cfg.modulator = None;
    cfg.evolution = None;
    cra::config::validate(&cfg).expect("test config validates");
    cfg
}

fn seeds() -> SeedIdentity {
    SeedIdentity {
        root_seed: 1,
        namespace: "development".to_owned(),
        outer_seed: 1,
        lifetime_index: 0,
    }
}

fn birth_pair(cfg: &Config) -> (Lifetime, NoLearningActor) {
    let lifetime = Lifetime::new(cfg, 1, "development", 1, 0).expect("birth");
    let actor = NoLearningActor::new(cfg, 1, "development", 1, 0).expect("birth");
    (lifetime, actor)
}

/// One tick of observable continuation used to compare halves.
#[derive(Clone, Debug, PartialEq)]
struct TickRecord {
    tick: u64,
    features: Vec<f64>,
    feedback: Option<f64>,
    motor: MotorOutput,
    committed: Option<u8>,
}

fn step_both(
    lifetime: &mut Lifetime,
    actor: &mut NoLearningActor,
    summary: &mut HealthSummary,
) -> TickRecord {
    let tick = lifetime.tick();
    let out = lifetime.advance().expect("advance");
    if let Some(feedback) = out.observation.feedback {
        actor.apply_feedback(feedback).expect("consume");
        lifetime
            .note_feedback_consumed(feedback.event_id)
            .expect("ledger");
    }
    let motor = actor.advance(&out.observation.features).expect("step");
    actor.health_check().expect("finite");
    summary
        .observe(
            tick,
            actor.actor_state().h(),
            actor.actor_state().a(),
            actor.actor_state().r(),
            actor.motor_state().q(),
        )
        .expect("healthy");
    let committed = if out.commitment_due {
        let action = actor.select_action();
        lifetime.commit(action).expect("commit");
        Some(action)
    } else {
        None
    };
    TickRecord {
        tick,
        features: out.observation.features,
        feedback: out.observation.feedback.map(|f| f.reward),
        motor,
        committed,
    }
}

fn tmp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("cra-checkpoint-{name}-{}.json", std::process::id()))
}

fn assert_full_state_equal(
    left_life: &Lifetime,
    left_actor: &NoLearningActor,
    right_life: &Lifetime,
    right_actor: &NoLearningActor,
) {
    assert_eq!(left_actor.actor_state().h(), right_actor.actor_state().h());
    assert_eq!(left_actor.actor_state().a(), right_actor.actor_state().a());
    assert_eq!(left_actor.actor_state().r(), right_actor.actor_state().r());
    assert_eq!(left_actor.motor_state().q(), right_actor.motor_state().q());
    assert_eq!(left_actor.last_output(), right_actor.last_output());
    assert_eq!(left_actor.last_feedback(), right_actor.last_feedback());
    assert_eq!(left_actor.ticks_advanced(), right_actor.ticks_advanced());
    assert_eq!(
        left_actor.inherited().weights,
        right_actor.inherited().weights
    );
    assert_eq!(left_life.tick(), right_life.tick());
    assert_eq!(left_life.phase(), right_life.phase());
    assert_eq!(left_life.commitments(), right_life.commitments());
    assert_eq!(left_life.outcomes(), right_life.outcomes());
    assert_eq!(left_life.consumed_ids(), right_life.consumed_ids());
    assert_eq!(left_life.last_action(), right_life.last_action());
    assert_eq!(left_life.pending(), right_life.pending());
    let k = left_life.cue_count();
    for cue in 0..k {
        assert_eq!(
            left_life.hidden().mapping(cue),
            right_life.hidden().mapping(cue)
        );
        assert_eq!(
            left_life.hidden().epsilon(cue),
            right_life.hidden().epsilon(cue)
        );
        assert_eq!(
            left_life.hidden().hazard(cue),
            right_life.hidden().hazard(cue)
        );
        assert_eq!(left_life.hidden().role(cue), right_life.hidden().role(cue));
        assert_eq!(
            left_life.hidden().exposures(cue),
            right_life.hidden().exposures(cue)
        );
        assert_eq!(
            left_life.hidden().changed_before_presentation(cue),
            right_life.hidden().changed_before_presentation(cue)
        );
    }
}

/// Drive a reference run to completion while capturing one checkpoint at
/// the first tick boundary matching `at_boundary`; resume from the
/// file-loaded checkpoint and prove the suffix plus final state agree.
fn split_resume_case(name: &str, at_boundary: impl Fn(&Lifetime) -> bool) {
    let cfg = checkpoint_config();
    let id = seeds();

    // Reference: straight through, recording every tick and the health
    // summary across the whole lifetime.
    let (mut ref_life, mut ref_actor) = birth_pair(&cfg);
    let mut ref_summary = HealthSummary::new();
    let mut ref_records = Vec::new();
    while !ref_life.is_complete() {
        ref_records.push(step_both(&mut ref_life, &mut ref_actor, &mut ref_summary));
    }

    // Split: drive to the boundary, checkpoint through a real file, resume.
    let (mut life, mut actor) = birth_pair(&cfg);
    let mut prefix = Vec::new();
    let mut summary = HealthSummary::new();
    let split_tick = loop {
        assert!(!life.is_complete(), "boundary never reached");
        if at_boundary(&life) {
            break life.tick();
        }
        prefix.push(step_both(&mut life, &mut actor, &mut summary));
    };
    assert!(
        !prefix.is_empty(),
        "{name}: split must happen mid-lifetime, not at birth"
    );
    let checkpoint = Checkpoint::capture(&life, &actor, &cfg, id.clone()).expect("capture");
    let path = tmp_path(name);
    checkpoint.save_to_path(&path).expect("save");
    let loaded = Checkpoint::load_from_path(&path).expect("load");
    assert_eq!(loaded, checkpoint, "file round-trip preserves the capture");
    let _ = std::fs::remove_file(&path);
    assert_eq!(loaded.tick(), split_tick);
    assert_eq!(loaded.seeds(), &id);
    assert_eq!(loaded.config().profile_name, "checkpoint_test");

    let mut life = loaded.restore_env(&id).expect("resume env");
    let mut actor = loaded.restore_actor(&id).expect("resume actor");
    // Resume starts exactly where the prefix stopped: tick agreement is
    // the first proof the right lifetime was restored.
    assert_eq!(life.tick(), split_tick);
    assert_eq!(actor.ticks_advanced(), split_tick);
    let mut suffix = Vec::new();
    while !life.is_complete() {
        suffix.push(step_both(&mut life, &mut actor, &mut summary));
    }

    let mut combined = prefix;
    let split_len = combined.len();
    combined.extend(suffix);
    assert_eq!(combined, ref_records, "{name}: resumed suffix must match");
    assert_full_state_equal(&ref_life, &ref_actor, &life, &actor);
    assert_eq!(summary, ref_summary, "{name}: continued health must match");
    assert!(
        split_len < ref_records.len(),
        "{name}: split must leave a non-empty suffix"
    );
}

#[test]
fn resume_at_quiet_boundary_matches_uninterrupted_run() {
    split_resume_case("quiet", |life| {
        life.phase() == Phase::Quiet && life.outcomes() >= 1 && life.pending().is_none()
    });
}

#[test]
fn resume_at_response_boundary_matches_uninterrupted_run() {
    // Just-committed: delay phase entered with exactly one unresolved
    // reward (commitments one ahead of outcomes).
    split_resume_case("response", |life| {
        life.phase() == Phase::Delay
            && life.pending().is_some()
            && life.commitments() == life.outcomes() + 1
    });
}

#[test]
fn resume_at_pending_feedback_boundary_matches_uninterrupted_run() {
    split_resume_case("feedback", |life| {
        life.phase() == Phase::Feedback && life.pending().is_some()
    });
}

#[test]
fn capture_rejects_non_executable_configs() {
    let cfg = checkpoint_config();
    let (life, actor) = birth_pair(&cfg);
    let id = seeds();
    Checkpoint::capture(&life, &actor, &cfg, id).expect("executable captures");

    let env_only: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("parses");
    // Same live objects, but an env-only config must not checkpoint them.
    assert!(matches!(
        Checkpoint::capture(&life, &actor, &env_only, seeds()),
        Err(CheckpointError::Incompatible(_))
    ));
}

#[test]
fn corrupt_files_fail_without_state() {
    let cfg = checkpoint_config();
    let (mut life, mut actor) = birth_pair(&cfg);
    let mut summary = HealthSummary::new();
    for _ in 0..12 {
        if life.is_complete() {
            break;
        }
        step_both(&mut life, &mut actor, &mut summary);
    }
    let checkpoint = Checkpoint::capture(&life, &actor, &cfg, seeds()).expect("capture");
    let path = tmp_path("corrupt");
    checkpoint.save_to_path(&path).expect("save");

    // Tampered payload bytes: valid JSON, broken integrity.
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    value["payload"]["env"]["tick"] = serde_json::json!(999_999u64);
    std::fs::write(&path, serde_json::to_string_pretty(&value).expect("json")).expect("write");
    assert!(matches!(
        Checkpoint::load_from_path(&path),
        Err(CheckpointError::ChecksumMismatch)
    ));

    // Wrong schema version is named before any state is trusted.
    checkpoint.save_to_path(&path).expect("rewrite");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    value["payload"]["schema_version"] = serde_json::json!(999u32);
    std::fs::write(&path, serde_json::to_string_pretty(&value).expect("json")).expect("write");
    assert!(matches!(
        Checkpoint::load_from_path(&path),
        Err(CheckpointError::SchemaVersion { found: 999 })
    ));

    // Unknown fields are rejected, never defaulted.
    checkpoint.save_to_path(&path).expect("rewrite");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    value["payload"]["future_plasticity"] = serde_json::json!([1, 2, 3]);
    std::fs::write(&path, serde_json::to_string_pretty(&value).expect("json")).expect("write");
    assert!(matches!(
        Checkpoint::load_from_path(&path),
        Err(CheckpointError::Parse { .. })
    ));

    // Truncation is a parse failure, not a partial resume.
    checkpoint.save_to_path(&path).expect("rewrite");
    let bytes = std::fs::read(&path).expect("read");
    std::fs::write(&path, &bytes[..bytes.len() / 2]).expect("truncate");
    assert!(matches!(
        Checkpoint::load_from_path(&path),
        Err(CheckpointError::Parse { .. })
    ));

    // Missing paths are I/O errors with the path attached.
    let missing = std::env::temp_dir().join(format!("cra-missing-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&missing);
    assert!(matches!(
        Checkpoint::load_from_path(&missing),
        Err(CheckpointError::Io { .. })
    ));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn incompatible_mutations_fail_without_silent_defaults() {
    let cfg = checkpoint_config();
    let (mut life, mut actor) = birth_pair(&cfg);
    let mut summary = HealthSummary::new();
    for _ in 0..12 {
        if life.is_complete() {
            break;
        }
        step_both(&mut life, &mut actor, &mut summary);
    }
    let checkpoint = Checkpoint::capture(&life, &actor, &cfg, seeds()).expect("capture");

    // Seed-identity swap: the file belongs to outer seed 1, so resuming
    // it as outer seed 2 is rejected on both halves before any state is
    // trusted. Tick/config skew inside the payload is covered by unit
    // tests in src/checkpoint.rs (in-crate mutation without file
    // checksum repair).
    let foreign = SeedIdentity {
        outer_seed: 2,
        ..seeds()
    };
    assert!(matches!(
        checkpoint.restore_env(&foreign),
        Err(CheckpointError::Incompatible(_))
    ));
    assert!(matches!(
        checkpoint.restore_actor(&foreign),
        Err(CheckpointError::Incompatible(_))
    ));

    // The reference platform promise is bitwise: the untouched capture
    // still resumes for its own identity after the forgery attempts.
    checkpoint.restore_env(&seeds()).expect("env restores");
    checkpoint.restore_actor(&seeds()).expect("actor restores");
    assert_eq!(CHECKPOINT_SCHEMA_VERSION, 1);
}

#[test]
fn atomic_save_never_leaves_a_partial_target() {
    let cfg = checkpoint_config();
    let (life, actor) = birth_pair(&cfg);
    let checkpoint = Checkpoint::capture(&life, &actor, &cfg, seeds()).expect("capture");
    // A missing parent directory is an I/O error with no file created.
    let bad = std::env::temp_dir().join(format!(
        "cra-no-such-dir-{}/checkpoint.json",
        std::process::id()
    ));
    assert!(!bad.parent().expect("parent").exists());
    assert!(matches!(
        checkpoint.save_to_path(&bad),
        Err(CheckpointError::Io { .. })
    ));
    assert!(
        !bad.exists(),
        "failed saves must not create partial targets"
    );
    // Overwriting the same path twice stays valid (temp + rename).
    let path = tmp_path("atomic");
    checkpoint.save_to_path(&path).expect("first save");
    checkpoint.save_to_path(&path).expect("second save");
    let loaded = Checkpoint::load_from_path(&path).expect("load");
    assert_eq!(loaded, checkpoint);
    let _ = std::fs::remove_file(&path);
}
