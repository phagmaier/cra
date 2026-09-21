//! Shared builders and tick drivers for environment integration tests.
//!
//! The drivers assert structural invariants on every tick (no feedback
//! before commitment, no second commitment while unresolved), so contract
//! violations fail fast with the offending tick attached.

use cra::config::{Config, Environment, Logging, Seeds, Simulation};
use cra::environment::{Lifetime, TickOutput};

/// Minimal valid config: warmup 4, quiet [4,4], cue 8, gap [0,0],
/// response 4, delay [1,1], 6 outcomes. Tests mutate copies per case.
pub fn base_config() -> Config {
    Config {
        schema_version: 1,
        profile_name: "test".to_owned(),
        simulation: Simulation {
            dt: 1.0,
            precision: "f64".to_owned(),
            warmup_ticks: 4,
            outcomes_per_lifetime: 6,
            reset_policy: "birth_only".to_owned(),
            feedback_order: "before_neural_transition".to_owned(),
        },
        environment: Environment {
            kind: "stationary_clean".to_owned(),
            cue_count: 2,
            cue_encoding: "one_hot".to_owned(),
            stable_fraction: 1.0,
            feedback_noise_values: vec![0.0],
            volatile_hazard_values: vec![0.0],
            hazard_clock: "cue_exposure".to_owned(),
            quiet_ticks: [4, 4],
            cue_ticks: 8,
            memory_gap_ticks: [0, 0],
            response_ticks: 4,
            reward_delay_ticks: [1, 1],
            feedback_ticks: 1,
            max_pending_choices: 1,
        },
        actor: None,
        learning: None,
        modulator: None,
        evolution: None,
        logging: Logging {
            event_log: true,
            trace_every_ticks: 10,
            full_trace_lifetimes: 1,
            record_raw_and_applied_updates: true,
        },
        seeds: Seeds {
            root_seed: 1,
            namespace: "development".to_owned(),
            outer_seed: 1,
        },
    }
}

pub fn birth(cfg: &Config) -> Lifetime {
    cra::config::validate(cfg).expect("test config validates");
    Lifetime::new(cfg, 1, "development", 1, 0).expect("birth succeeds")
}

/// Step until the commitment point, asserting no feedback arrives first.
/// Returns the final response tick output.
pub fn advance_until_commitment(lt: &mut Lifetime) -> TickOutput {
    for _ in 0..100_000 {
        let out = lt.advance().expect("advance");
        assert!(
            out.observation.feedback.is_none(),
            "no feedback before commitment (tick {})",
            out.tick
        );
        assert!(out.annotation.is_none());
        if out.commitment_due {
            return out;
        }
    }
    panic!("never reached a commitment point");
}

/// Step until the next feedback tick, asserting no second commitment is
/// requested while the choice is unresolved.
pub fn advance_until_feedback(lt: &mut Lifetime) -> TickOutput {
    for _ in 0..100_000 {
        let out = lt.advance().expect("advance");
        assert!(
            !out.commitment_due,
            "no second commitment while unresolved (tick {})",
            out.tick
        );
        if out.observation.feedback.is_some() {
            assert!(out.annotation.is_some());
            return out;
        }
    }
    panic!("never reached a feedback tick");
}

/// One full choice: the cue shown this cycle, commit tick, feedback tick,
/// delivered reward, event id, and latent correctness (evaluator side).
#[derive(Debug)]
pub struct CycleRecord {
    pub cue: usize,
    pub commit_tick: u64,
    pub feedback_tick: u64,
    pub reward: f64,
    pub event_id: u64,
    pub correct: bool,
}

/// Drive one cycle with `commit_fn` choosing the commit path. The closure
/// receives the cue presented this cycle (cues vanish outside presentation,
/// so the driver records it while visible).
pub fn run_cycle_with(
    lt: &mut Lifetime,
    commit_fn: impl FnOnce(&mut Lifetime, usize),
) -> CycleRecord {
    let mut cue = None;
    let commit_out = loop {
        let out = lt.advance().expect("advance");
        assert!(
            out.observation.feedback.is_none(),
            "no feedback before commitment (tick {})",
            out.tick
        );
        if out.cue.is_some() {
            cue = out.cue;
        }
        if out.commitment_due {
            break out;
        }
    };
    let commit_tick = commit_out.tick;
    let cue = cue.expect("a cue was presented this cycle");
    commit_fn(lt, cue);
    let fb_out = advance_until_feedback(lt);
    let feedback = fb_out.observation.feedback.expect("feedback present");
    let annotation = fb_out.annotation.expect("annotation present");
    assert_eq!(feedback.event_id, annotation.event_id);
    CycleRecord {
        cue,
        commit_tick,
        feedback_tick: fb_out.tick,
        reward: feedback.reward,
        event_id: feedback.event_id,
        correct: annotation.latent_correctness,
    }
}

/// Drive one cycle committing `action` through the production path.
pub fn run_cycle(lt: &mut Lifetime, action: u8) -> CycleRecord {
    run_cycle_with(lt, |lt, _cue| {
        lt.commit(action).expect("commit");
    })
}
