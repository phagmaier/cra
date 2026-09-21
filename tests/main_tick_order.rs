//! Authoritative main tick order (M4-01; spec 9, 17.2).
//!
//! - The tick-20/delay-3 case with a real learning agent: commitment at
//!   tick 20, ordinary delay ticks 21-22, exactly one feedback at the start
//!   of tick 23. The update applied at tick 23 uses eligibility accumulated
//!   through tick 22 with the fixed gate 1 and the old baseline; the
//!   feedback-evoked scores created by advancing on the tick-23 features
//!   cannot contribute to that outcome's update.
//! - Exactly one feedback application path: neural advances (even on
//!   feedback-shaped features) never move `P`, the baseline, or dedup
//!   bookkeeping; one `apply_feedback` moves `P` once; a duplicate
//!   delivery is rejected without state change.
//! - `advance()` is exactly `observe()` + `finish_tick()` (parity), so the
//!   split learning drivers and the fused environment tests share one
//!   tick semantics.
//!
//! The schedule mirrors `tests/event_order.rs` (warmup 4, quiet [4,4],
//! cue 8, gap [5,5], response 4, delay [3,3], K = 2, zero noise/hazard,
//! seeds root 1 / development / outer 1 / lifetime 0): birth mappings
//! `[1, 1]`, first cue 1. The learner is the M3 motor-afferent family
//! member at winner-like hyperparameters, driven here *without* rollout
//! resets over a single outcome so the test pins ordering only —
//! persistence and continuity profiles belong to M4-02 through M4-04.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::{Actor, Learning};
use cra::environment::{Feedback, Lifetime, Phase, feature_dim};
use cra::experiments::episodic::EpisodicLearner;
use cra::rng::{SeedTuple, rng_for};
use support::base_config;

fn tick203_config() -> cra::config::Config {
    let mut cfg = base_config();
    cfg.environment.memory_gap_ticks = [5, 5];
    cfg.environment.reward_delay_ticks = [3, 3];
    cfg.simulation.outcomes_per_lifetime = 1;
    cfg.simulation.reset_policy = "episodic_diagnostic".to_owned();
    cra::config::validate(&cfg).expect("fixture config validates");
    cfg
}

fn actor_cfg() -> Actor {
    Actor {
        neuron_count: 4,
        motor_neurons_per_action: 1,
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

fn learning_cfg() -> Learning {
    Learning {
        enabled: true,
        rule: "gaussian_transition_score".to_owned(),
        plastic_mask: "motor_afferent_only".to_owned(),
        trace_policy: "no_decay_diagnostic".to_owned(),
        tau_e: 32.0,
        eta: 0.001,
        max_update: 0.01,
        plastic_bound: 0.5,
        plastic_decay: 0.0,
        reward_baseline_initial: 0.5,
        reward_baseline_beta: 0.02,
    }
}

fn small_inherited() -> InheritedParams {
    // N = 4, m = 1: M0 = [2], M1 = [3]. Plastic edges enter 2 and 3.
    let mask = vec![
        vec![false, true, false, false],
        vec![true, false, false, false],
        vec![true, false, false, true],
        vec![false, true, true, false],
    ];
    let topology = topology_from_mask(4, 1, 0.25, false, mask, "test".to_owned()).expect("mask");
    let mut w0 = vec![vec![0.0; 4]; 4];
    for (j, row) in topology.mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                w0[j][i] = 0.1 * (j as f64 + 1.0) + 0.01 * (i as f64 + 1.0);
            }
        }
    }
    InheritedParams {
        topology,
        weights: Weights {
            w0,
            input_weights: vec![vec![0.05; feature_dim(2)]; 4],
            bias: vec![0.0; 4],
        },
    }
}

fn learner() -> EpisodicLearner {
    let noise = rng_for(&SeedTuple::new(1, "development", 1, 0, "actor_noise")).expect("rng");
    let tie = rng_for(&SeedTuple::new(1, "development", 1, 0, "tie_break")).expect("rng");
    EpisodicLearner::from_agent_parts(
        actor_cfg(),
        learning_cfg(),
        small_inherited(),
        2,
        noise,
        tie,
    )
    .expect("learner")
}

fn assert_zero_mat(m: &[Vec<f64>], what: &str) {
    assert!(
        m.iter().flatten().all(|&v| v == 0.0),
        "{what} must be exactly zero, found {m:?}"
    );
}

#[test]
fn commit_at_20_delay_3_update_uses_traces_through_22() {
    let cfg = tick203_config();
    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    assert_eq!(lifetime.hidden().mapping_snapshot(), vec![1, 1]);
    let mut learner = learner();
    assert_zero_mat(learner.plastic().p(), "birth P");
    assert_zero_mat(learner.plastic().e(), "birth E");
    assert_eq!(learner.reward_baseline(), 0.5);
    assert_eq!(learner.last_feedback(), None);

    // Ticks 0-20 through the split driver: observe, no feedback, advance,
    // finish, commit at 20. No update path runs before the first outcome.
    for tick in 0..=20u64 {
        let out = lifetime.observe().expect("observe");
        assert_eq!(out.tick, tick);
        assert!(
            out.observation.feedback.is_none(),
            "no feedback before commitment (tick {tick})"
        );
        assert!(out.annotation.is_none());
        if tick < 20 {
            assert!(!out.commitment_due, "sole commitment point is tick 20");
        } else {
            assert_eq!(out.phase, Phase::Response);
            assert!(out.commitment_due, "sole commitment point");
        }
        learner.advance(&out.observation.features).expect("advance");
        // Neural advances never touch offsets, baseline, or dedup.
        assert_zero_mat(learner.plastic().p(), "P before any feedback");
        assert_eq!(learner.reward_baseline(), 0.5);
        assert_eq!(learner.last_feedback(), None);
        // Finish before commit: the transient Committed phase exists only
        // between finish and commit, so `commit_tick = tick - 1` holds.
        lifetime.finish_tick().expect("finish");
        if out.commitment_due {
            // Evaluator privilege in the fixture only: commit the known
            // correct action (target 1) through the production path.
            lifetime.commit(1).expect("commit at tick 20");
        }
    }
    assert_eq!(lifetime.pending().expect("pending").due_tick, 23);

    // Ticks 21-22: ordinary delay transitions; the latch shows action 1.
    for tick in [21, 22] {
        let out = lifetime.observe().expect("delay tick");
        assert_eq!((out.tick, out.phase), (tick, Phase::Delay));
        assert!(out.observation.feedback.is_none());
        assert!(out.annotation.is_none());
        learner.advance(&out.observation.features).expect("advance");
        assert_zero_mat(learner.plastic().p(), "P still zero on delay ticks");
        lifetime.finish_tick().expect("finish");
    }

    // Tick 23 start: exactly one outcome. The update below must use E
    // through tick 22, the fixed gate 1, and the old baseline.
    let out = lifetime.observe().expect("feedback tick");
    assert_eq!((out.tick, out.phase), (23, Phase::Feedback));
    let feedback = out.observation.feedback.expect("exactly one outcome");
    assert_eq!((feedback.event_id, feedback.reward), (0, 1.0));
    let annotation = out.annotation.clone().expect("annotation");
    assert_eq!(annotation.commit_tick, 20);
    assert_eq!(annotation.outcome_tick, 23);

    let e_old = learner.plastic().e().to_vec();
    assert!(
        e_old.iter().flatten().any(|&v| v != 0.0),
        "E through tick 22 must be nonzero or the ordering test is vacuous"
    );
    let baseline_old = learner.reward_baseline();
    let eta = learning_cfg().eta;
    let beta = learning_cfg().reward_baseline_beta;
    let max_update = learning_cfg().max_update;
    let bound = learning_cfg().plastic_bound;

    // Spec 9 step 2: pre-tick E, fixed gate 1, old baseline, exactly once.
    let outcome = learner.apply_feedback(feedback).expect("apply");
    lifetime
        .note_feedback_consumed(feedback.event_id)
        .expect("ledger");
    let delta = feedback.reward - baseline_old;
    assert_eq!(outcome.event_id, 0);
    assert!((outcome.delta - delta).abs() <= 1e-15);
    assert!((outcome.baseline_old - baseline_old).abs() <= 1e-15);
    assert!((outcome.baseline_new - (baseline_old + beta * delta)).abs() <= 1e-15);
    assert!((learner.reward_baseline() - (baseline_old + beta * delta)).abs() <= 1e-15);
    assert_eq!(learner.last_feedback(), Some(0));
    for (j, ((raw_row, e_row), (limited_row, actual_row))) in outcome
        .raw_updates
        .iter()
        .zip(e_old.iter())
        .zip(
            outcome
                .limited_updates
                .iter()
                .zip(outcome.actual_updates.iter()),
        )
        .enumerate()
    {
        for (i, (&raw, (&e, (&limited, &actual)))) in raw_row
            .iter()
            .zip(e_row.iter().zip(limited_row.iter().zip(actual_row.iter())))
            .enumerate()
        {
            // Fixed gate 1: raw is eta * delta * E_old, bit for bit.
            let expected = eta * delta * 1.0 * e;
            assert!(
                (raw - expected).abs() <= 1e-15,
                "edge ({j},{i}): raw {raw} != eta*delta*E_old {expected}"
            );
            let expected_limited = raw.clamp(-max_update, max_update);
            assert!(
                (limited - expected_limited).abs() <= 1e-15,
                "edge ({j},{i}): limited {limited} != clamp(raw) {expected_limited}"
            );
            // No P movement before this event, so actual is the
            // bound-clamped limited step from zero.
            let expected_actual = limited.clamp(-bound, bound);
            assert!(
                (actual - expected_actual).abs() <= 1e-15,
                "edge ({j},{i}): actual {actual} != {expected_actual}"
            );
            assert!(
                (learner.plastic().p()[j][i] - expected_actual).abs() <= 1e-15,
                "edge ({j},{i}): stored P disagrees with its update report"
            );
        }
    }

    // Advancing on the tick-23 features creates feedback-evoked scores,
    // proving the consumed update could not have used them.
    learner
        .advance(&out.observation.features)
        .expect("post-feedback advance");
    let e_new = learner.plastic().e().to_vec();
    assert_ne!(e_old, e_new, "feedback-tick scores must extend E");
    let l1_old: f64 = e_old.iter().flatten().map(|v| v.abs()).sum();
    let l1_new: f64 = e_new.iter().flatten().map(|v| v.abs()).sum();
    assert!(
        (l1_new - l1_old).abs() > 0.0,
        "E magnitudes must differ to make the ordering test sensitive"
    );
    let stale_raw_l1: f64 = e_new
        .iter()
        .flatten()
        .map(|&e| (eta * delta * 1.0 * e).abs())
        .sum();
    let applied_raw_l1: f64 = outcome.raw_updates.iter().flatten().map(|v| v.abs()).sum();
    assert!(
        (stale_raw_l1 - applied_raw_l1).abs() > 0.0,
        "recomputing with post-feedback E would change the update"
    );

    lifetime.finish_tick().expect("finish");
    assert!(lifetime.is_complete());
    assert_eq!((lifetime.commitments(), lifetime.outcomes()), (1, 1));

    // Exactly once: redelivery is rejected without state change.
    let p_after = learner.plastic().p().to_vec();
    let baseline_after = learner.reward_baseline();
    let dup = learner.apply_feedback(Feedback {
        event_id: 0,
        reward: 1.0,
    });
    assert!(dup.is_err(), "duplicate feedback must be rejected");
    assert_eq!(learner.plastic().p(), &p_after);
    assert_eq!(learner.reward_baseline(), baseline_after);
    assert_eq!(learner.last_feedback(), Some(0));
}

#[test]
fn neural_advance_never_applies_feedback_even_on_feedback_shaped_input() {
    // The second half of the one-path guarantee: `advance` owns no update
    // path, so even outcome-shaped features move only E, never P,
    // baseline, or dedup.
    let mut learner = learner();
    let k = feature_dim(2);
    let mut outcome_shaped = vec![0.0; k];
    outcome_shaped[2 + 2] = 1.0; // outcome-present
    outcome_shaped[2 + 3] = 1.0; // outcome-value
    for _ in 0..4 {
        learner.advance(&outcome_shaped).expect("advance");
    }
    assert!(
        learner.plastic().e().iter().flatten().any(|&v| v != 0.0),
        "scores must accumulate so the P-still-zero check is sensitive"
    );
    assert_zero_mat(learner.plastic().p(), "P");
    assert_eq!(learner.reward_baseline(), 0.5);
    assert_eq!(learner.last_feedback(), None);
}

#[test]
fn reobserving_feedback_without_finish_errors_instead_of_double_delivering() {
    let cfg = tick203_config();
    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    for _ in 0..=20u64 {
        let out = lifetime.observe().expect("observe");
        lifetime.finish_tick().expect("finish");
        if out.commitment_due {
            lifetime.commit(1).expect("commit");
        }
    }
    for _ in [21, 22] {
        lifetime.observe().expect("delay");
        lifetime.finish_tick().expect("finish");
    }
    let first = lifetime.observe().expect("feedback tick");
    assert!(first.observation.feedback.is_some());
    assert!(lifetime.pending().is_none(), "delivery takes pending");
    let second = lifetime.observe();
    assert!(
        second.is_err(),
        "re-observing a feedback tick without finish must error, got {second:?}"
    );
    assert_eq!(lifetime.consumed_ids(), &[0], "exactly one delivery");
}

#[test]
fn fused_advance_equals_observe_plus_finish() {
    // Parity pin: the split drivers and the fused environment primitive
    // share one tick semantics.
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 2;
    cra::config::validate(&cfg).expect("config validates");
    let mut fused = Lifetime::new(&cfg, 1, "development", 1, 0).expect("fused birth");
    let mut split = Lifetime::new(&cfg, 1, "development", 1, 0).expect("split birth");
    let mut fused_out = Vec::new();
    while !fused.is_complete() {
        let out = fused.advance().expect("fused tick");
        if out.commitment_due {
            fused.commit(0).expect("fused commit");
        }
        fused_out.push(out);
    }
    let mut split_out = Vec::new();
    while !split.is_complete() {
        let out = split.observe().expect("split observe");
        split.finish_tick().expect("split finish");
        if out.commitment_due {
            split.commit(0).expect("split commit");
        }
        split_out.push(out);
    }
    // Both loops commit post-finish (the transient Committed phase exists
    // only between finish and commit); the outputs are identical because
    // `advance` is exactly `observe` + `finish_tick`.
    assert_eq!(fused_out, split_out);
    assert_eq!(fused.tick(), split.tick());
    assert_eq!(fused.commitments(), split.commitments());
    assert_eq!(fused.outcomes(), split.outcomes());
    assert_eq!(fused.consumed_ids(), split.consumed_ids());
    assert_eq!(fused.last_action(), split.last_action());
    assert!(fused.pending().is_none() && split.pending().is_none());
}
