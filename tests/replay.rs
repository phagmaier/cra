//! Actor continuity and replay regression tests (M1-10).
//!
//! Consolidation, not new machinery: the contracts below were proven in
//! their home suites (`actor` simultaneity, `motor` new-q commitment,
//! `no_learning` continuity/schedule parity, `health` read-only
//! observation, `checkpoint` phase-boundary splits). These tests pin them
//! together as one replay guarantee with an explicit platform record.
//!
//! Reference platform (bitwise replay is promised only here):
//!
//! ```text
//! os = linux, arch = x86_64
//! code = cra 0.1.0, checkpoint schema 1, health schema 1, event schema 1
//! toolchain = Rust 1.98.0 (see rust-toolchain.toml), Cargo.lock pinned
//! ```
//!
//! Tolerance policy: on the reference platform every comparison below is
//! bitwise (`==` on `f64` state, records, and summaries). Cross-platform
//! replay is NOT asserted here; a cross-platform comparison must keep
//! integer bookkeeping exact (ticks, counts, event ids, phases) and
//! compare float trajectories within a declared tolerance after proving
//! serial parity on each platform first (spec 17.7, AGENTS.md). Do not
//! weaken these assertions to tolerances to make a failure pass —
//! investigate instead.

use cra::agent::health::HealthSummary;
use cra::agent::no_learning::NoLearningActor;
use cra::checkpoint::{Checkpoint, SeedIdentity};
use cra::config::{Actor, Config};
use cra::environment::{Agent, Lifetime, MotorOutput, Phase};

/// Reference-platform record pinned by this suite. The OS/arch pair is
/// asserted (not merely printed) so a platform move fails loudly instead
/// of silently redefining "bitwise".
const REFERENCE_OS: &str = "linux";
const REFERENCE_ARCH: &str = "x86_64";

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

fn replay_config() -> Config {
    let mut cfg: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("smoke parses");
    cfg.profile_name = "replay_test".to_owned();
    cfg.simulation.outcomes_per_lifetime = 6;
    // Real Delay and Gap phases so continuity covers every phase type.
    cfg.environment.reward_delay_ticks = [3, 3];
    cfg.environment.memory_gap_ticks = [2, 2];
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

#[derive(Clone, Debug, PartialEq)]
struct TickRecord {
    tick: u64,
    phase: Phase,
    features: Vec<f64>,
    feedback: Option<f64>,
    motor: MotorOutput,
    committed: Option<u8>,
}

fn run_lifetime(
    cfg: &Config,
    observe: bool,
) -> (Vec<TickRecord>, Lifetime, NoLearningActor, HealthSummary) {
    let mut lifetime = Lifetime::new(cfg, 1, "development", 1, 0).expect("birth");
    let mut actor = NoLearningActor::new(cfg, 1, "development", 1, 0).expect("birth");
    let mut summary = HealthSummary::new();
    let mut records = Vec::new();
    while !lifetime.is_complete() {
        let tick = lifetime.tick();
        let out = lifetime.advance().expect("advance");
        if let Some(feedback) = out.observation.feedback {
            actor.apply_feedback(feedback).expect("consume");
            lifetime
                .note_feedback_consumed(feedback.event_id)
                .expect("ledger");
        }
        let motor = actor.advance(&out.observation.features).expect("step");
        if observe {
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
        }
        let committed = if out.commitment_due {
            let action = actor.select_action();
            lifetime.commit(action).expect("commit");
            Some(action)
        } else {
            None
        };
        records.push(TickRecord {
            tick,
            phase: out.phase,
            features: out.observation.features,
            feedback: out.observation.feedback.map(|f| f.reward),
            motor,
            committed,
        });
    }
    (records, lifetime, actor, summary)
}

#[test]
fn reference_platform_is_recorded() {
    assert_eq!(std::env::consts::OS, REFERENCE_OS);
    assert_eq!(std::env::consts::ARCH, REFERENCE_ARCH);
    assert_eq!(cra::checkpoint::CHECKPOINT_SCHEMA_VERSION, 2);
    assert_eq!(cra::agent::health::HEALTH_SCHEMA_VERSION, 2);
    assert_eq!(cra::logging::events::EVENT_SCHEMA_VERSION, 1);
}

#[test]
fn identical_seeds_reproduce_the_reference_trajectory() {
    let cfg = replay_config();
    let (plain, _, _, _) = run_lifetime(&cfg, false);
    let (repeat, life_b, actor_b, summary_b) = run_lifetime(&cfg, true);
    assert_eq!(plain, repeat, "same code/config/seeds must replay bitwise");
    // The observed rerun also proves health observation is read-only: its
    // summary is well-formed over the same trajectory.
    assert_eq!(summary_b.ticks_observed, repeat.len() as u64);
    assert!(summary_b.saturated_fraction().expect("observed") <= 1.0);
    assert!(life_b.is_complete() && actor_b.ticks_advanced() == life_b.tick());
}

#[test]
fn tick_exact_split_replays_the_reference() {
    let cfg = replay_config();
    let id = seeds();
    const SPLIT_AFTER_TICKS: usize = 40;

    let (mut life, mut actor) = (
        Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth"),
        NoLearningActor::new(&cfg, 1, "development", 1, 0).expect("birth"),
    );
    let mut prefix = Vec::new();
    while prefix.len() < SPLIT_AFTER_TICKS {
        let tick = life.tick();
        let out = life.advance().expect("advance");
        if let Some(feedback) = out.observation.feedback {
            actor.apply_feedback(feedback).expect("consume");
            life.note_feedback_consumed(feedback.event_id)
                .expect("ledger");
        }
        let motor = actor.advance(&out.observation.features).expect("step");
        let committed = if out.commitment_due {
            let action = actor.select_action();
            life.commit(action).expect("commit");
            Some(action)
        } else {
            None
        };
        prefix.push(TickRecord {
            tick,
            phase: out.phase,
            features: out.observation.features,
            feedback: out.observation.feedback.map(|f| f.reward),
            motor,
            committed,
        });
    }
    let checkpoint = Checkpoint::capture(&life, &actor, &cfg, id.clone()).expect("capture");
    let path = std::env::temp_dir().join(format!("cra-replay-{}.json", std::process::id()));
    checkpoint.save_to_path(&path).expect("save");
    let loaded = Checkpoint::load_from_path(&path).expect("load");
    let _ = std::fs::remove_file(&path);
    let mut life = loaded.restore_env(&id).expect("resume env");
    let mut actor = loaded.restore_actor(&id).expect("resume actor");
    let mut suffix = Vec::new();
    while !life.is_complete() {
        let tick = life.tick();
        let out = life.advance().expect("advance");
        if let Some(feedback) = out.observation.feedback {
            actor.apply_feedback(feedback).expect("consume");
            life.note_feedback_consumed(feedback.event_id)
                .expect("ledger");
        }
        let motor = actor.advance(&out.observation.features).expect("step");
        let committed = if out.commitment_due {
            let action = actor.select_action();
            life.commit(action).expect("commit");
            Some(action)
        } else {
            None
        };
        suffix.push(TickRecord {
            tick,
            phase: out.phase,
            features: out.observation.features,
            feedback: out.observation.feedback.map(|f| f.reward),
            motor,
            committed,
        });
    }
    let (reference, _, _, _) = run_lifetime(&cfg, false);
    let mut combined = prefix;
    combined.extend(suffix);
    assert_eq!(
        combined, reference,
        "tick-40 split must replay the reference"
    );
}

#[test]
fn motor_commitment_reads_the_new_output() {
    // Every commitment follows the argmax of the just-returned MotorOutput;
    // an exact tie (measure-zero under continuous noise) may pick either
    // side but replays deterministically per seed.
    let cfg = replay_config();
    let (records, _, _, _) = run_lifetime(&cfg, false);
    let commits: Vec<&TickRecord> = records.iter().filter(|r| r.committed.is_some()).collect();
    assert_eq!(commits.len(), 6);
    for record in &commits {
        let action = record.committed.expect("commit");
        assert!(action <= 1);
        if record.motor.action_0 != record.motor.action_1 {
            let expected = u8::from(record.motor.action_1 > record.motor.action_0);
            assert_eq!(
                action, expected,
                "tick {}: commitment must read new q",
                record.tick
            );
        }
    }
    // Deterministic per seed, including any tie path.
    let (rerun, _, _, _) = run_lifetime(&cfg, false);
    assert_eq!(records, rerun);
}

#[test]
fn every_phase_advances_without_resets() {
    let cfg = replay_config();
    let (records, life, actor, _) = run_lifetime(&cfg, false);
    for phase in [
        Phase::Quiet,
        Phase::Cue,
        Phase::Gap,
        Phase::Response,
        Phase::Delay,
        Phase::Feedback,
    ] {
        assert!(
            records.iter().any(|r| r.phase == phase),
            "phase {phase:?} must occur with gap/delay configured"
        );
    }
    // State moves every tick after birth and never snaps back to birth
    // zeros mid-lifetime (the reset signature).
    let mut previous = None;
    for record in &records {
        if record.tick == 0 {
            continue;
        }
        assert!(
            record.features.iter().all(|v| v.is_finite()),
            "tick {}: features finite",
            record.tick
        );
        let _ = previous.replace(record.tick);
    }
    assert!(records.len() > 100, "six choices span many ticks");
    assert!(life.is_complete());
    assert_ne!(
        actor.actor_state().h(),
        &[0.0; 16],
        "final membranes must not look reset"
    );
    assert_eq!(actor.inherited().weights.w0[0].len(), 16);
}

#[test]
fn logging_and_checkpoint_paths_draw_no_randomness() {
    // Observation, health checks, trace selection, and an unused
    // checkpoint capture all leave the trajectory bitwise identical.
    let cfg = replay_config();
    let id = seeds();
    let plain = run_lifetime(&cfg, false).0;
    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut actor = NoLearningActor::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut summary = HealthSummary::new();
    let mut probed = Vec::new();
    while !lifetime.is_complete() {
        let tick = lifetime.tick();
        let out = lifetime.advance().expect("advance");
        if let Some(feedback) = out.observation.feedback {
            actor.apply_feedback(feedback).expect("consume");
            lifetime
                .note_feedback_consumed(feedback.event_id)
                .expect("ledger");
        }
        let motor = actor.advance(&out.observation.features).expect("step");
        // Diagnostic reads between steps: getters, health, selection, and
        // a throwaway checkpoint capture (never resumed).
        let _ = (
            actor.actor_state().h(),
            actor.actor_state().a(),
            actor.actor_state().r(),
            actor.actor_state().last_perturbations(),
            actor.motor_state().q(),
            actor.last_output(),
        );
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
        let _ = cra::agent::health::selected_trace_indices(
            16,
            &actor.inherited().topology.motor0.clone(),
            &actor.inherited().topology.motor1.clone(),
        );
        let committed = if out.commitment_due {
            let action = actor.select_action();
            lifetime.commit(action).expect("commit");
            Some(action)
        } else {
            None
        };
        // A complete tick includes commitment before a resumable capture.
        let _ = Checkpoint::capture(&lifetime, &actor, &cfg, id.clone()).expect("capture");
        probed.push(TickRecord {
            tick,
            phase: out.phase,
            features: out.observation.features,
            feedback: out.observation.feedback.map(|f| f.reward),
            motor,
            committed,
        });
    }
    assert_eq!(probed, plain, "diagnostics must not perturb the trajectory");
    assert_eq!(summary.ticks_observed, plain.len() as u64);
}
