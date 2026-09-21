//! Persistent eligibility and the running baseline (M4-02; spec 7.3-7.5,
//! 7.7-7.8, 10.4, 17.2).
//!
//! - Persistent recurrence is exactly `E <- lambda_e * E + S` with
//!   `lambda_e = exp(-1 / tau_e)`: closed-form match over scripted ticks
//!   plus a sensitivity check proving no hidden `(1 - lambda_e)` score
//!   factor. Decay compounds once per eligibility advance (quiet, delay,
//!   and feedback transitions alike — the driver calls it every tick per
//!   the M4-01 order).
//! - The live trace at feedback includes post-commitment delay scores
//!   with decay (spec 7.8): the applied update matches the live `E`, and
//!   both a frozen commit-time snapshot and a decay-only (no new scores)
//!   counterfactual give different updates. The `commit_snapshot_credit`
//!   alternative is a different named rule, not this one.
//! - The baseline starts from the configured initial value and updates
//!   exactly once per consumed feedback (closed form over a reward
//!   sequence); advances, duplicate deliveries, and rejections never move
//!   it.
//! - The persistent learner constructs from agent-only inputs on the
//!   `persistent` policy only, is born at `P = E = 0` with
//!   `W_effective = W0`, and wires its second-tick scores exactly.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::agent::actor::leak_alpha;
use cra::agent::plasticity::PlasticState;
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::{Actor, Learning};
use cra::environment::{Phase, feature_dim};
use cra::experiments::continuous::ContinuousLearner;
use cra::rng::{SeedTuple, rng_for};
use support::base_config;

const TAU_E: f64 = 64.0;
const ETA: f64 = 0.001;
const BETA: f64 = 0.02;

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
        trace_policy: "persistent".to_owned(),
        tau_e: TAU_E,
        eta: ETA,
        max_update: 0.01,
        plastic_bound: 0.5,
        plastic_decay: 0.0,
        reward_baseline_initial: 0.5,
        reward_baseline_beta: BETA,
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

fn learner() -> ContinuousLearner {
    let noise = rng_for(&SeedTuple::new(1, "development", 1, 0, "actor_noise")).expect("rng");
    let tie = rng_for(&SeedTuple::new(1, "development", 1, 0, "tie_break")).expect("rng");
    ContinuousLearner::from_agent_parts(
        actor_cfg(),
        learning_cfg(),
        small_inherited(),
        2,
        noise,
        tie,
    )
    .expect("learner")
}

fn close(got: f64, want: f64, tol: f64, what: &str) {
    assert!(
        (got - want).abs() <= tol,
        "{what}: got {got} want {want} (tol {tol})"
    );
}

#[test]
fn persistent_recurrence_matches_closed_form_without_unit_factor() {
    // Scripted scores on the motor-afferent mask (plastic edges enter
    // receivers 2 and 3): E_5 = sum_{t} lambda^(4-t) S_t, no (1-lambda).
    let tau = 16.0;
    let lambda = (-1.0_f64 / tau).exp();
    let alpha = 0.5;
    let sigma = 0.1;
    let inherited = small_inherited();
    let mut w0 = vec![vec![0.0; 4]; 4];
    for (j, row) in inherited.topology.mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                w0[j][i] = 0.3;
            }
        }
    }
    let mut state = PlasticState::new(
        &inherited.topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        tau,
        0.5,
    )
    .expect("birth");
    // (r_old, xi) per tick; r_old varies per sender, xi per receiver.
    let ticks: [([f64; 4], [f64; 4]); 5] = [
        ([0.2, -0.3, 0.1, 0.4], [0.5, -0.2, 0.4, 0.1]),
        ([0.1, 0.2, -0.4, 0.3], [-0.3, 0.6, 0.2, -0.5]),
        ([-0.2, 0.1, 0.3, -0.1], [0.4, 0.1, -0.6, 0.2]),
        ([0.3, -0.1, 0.2, 0.2], [0.1, -0.4, 0.3, 0.4]),
        ([0.0, 0.4, -0.2, 0.1], [-0.2, 0.3, 0.5, -0.3]),
    ];
    for (r_old, xi) in &ticks {
        state
            .advance_eligibility(r_old, xi, alpha, sigma)
            .expect("advance");
    }
    let plastic_edges = [(2, 0), (2, 3), (3, 1), (3, 2)];
    for &(j, i) in &plastic_edges {
        // Independent closed form from the script literals.
        let mut want = 0.0;
        for (t, (r_old, xi)) in ticks.iter().enumerate() {
            let s = alpha * r_old[i] * xi[j] / sigma;
            want += lambda.powi(4 - t as i32) * s;
        }
        close(state.e()[j][i], want, 1e-12, &format!("E[{j},{i}]"));
        // A (1-lambda)-normalized score rule would give a different value;
        // the live value must not match it (sensitivity guard).
        let mut wrong = 0.0;
        for (t, (r_old, xi)) in ticks.iter().enumerate() {
            let s = (1.0 - lambda) * alpha * r_old[i] * xi[j] / sigma;
            wrong = lambda * wrong + s;
            let _ = t;
        }
        assert!(
            (state.e()[j][i] - wrong).abs() > 1e-6,
            "E[{j},{i}] must be distinguishable from a (1-lambda) rule"
        );
    }
    // Nonplastic entries stay exactly zero throughout.
    for j in 0..4 {
        for i in 0..4 {
            if !plastic_edges.contains(&(j, i)) {
                assert_eq!(state.e()[j][i], 0.0, "E[{j},{i}] nonplastic");
            }
        }
    }
}

#[test]
fn decay_compounds_once_per_advance_with_zero_scores() {
    // With r_old = 0 every new score is zero, isolating pure decay:
    // three advances multiply E by exactly lambda^3 (quiet/delay/feedback
    // transitions all run this same call in the M4-01 driver order).
    let tau = 16.0;
    let lambda = (-1.0_f64 / tau).exp();
    let inherited = small_inherited();
    let mut w0 = vec![vec![0.0; 4]; 4];
    for (j, row) in inherited.topology.mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                w0[j][i] = 0.3;
            }
        }
    }
    let mut state = PlasticState::new(
        &inherited.topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        tau,
        0.5,
    )
    .expect("birth");
    state
        .advance_eligibility(&[0.5, -0.5, 0.5, -0.5], &[1.0, 1.0, 1.0, 1.0], 0.5, 0.1)
        .expect("accrue");
    let before = state.e().to_vec();
    assert!(before.iter().flatten().any(|&v| v != 0.0));
    let zero_r = [0.0; 4];
    let zero_xi = [0.0; 4];
    for _ in 0..3 {
        state
            .advance_eligibility(&zero_r, &zero_xi, 0.5, 0.1)
            .expect("decay");
    }
    for (j, (got_row, before_row)) in state.e().iter().zip(before.iter()).enumerate() {
        for (i, (&got, &prior)) in got_row.iter().zip(before_row.iter()).enumerate() {
            close(got, lambda.powi(3) * prior, 1e-12, &format!("E[{j},{i}]"));
        }
    }
}

#[test]
fn persistent_construction_guards_and_birth_wiring() {
    // Wrong policy, disabled learning, and unknown rule are explicit
    // errors; the diagnostic policy is rejected here (mirroring the
    // episodic learner's rejection in the other direction).
    let (noise, tie) = (
        rng_for(&SeedTuple::new(1, "development", 1, 0, "actor_noise")).expect("rng"),
        rng_for(&SeedTuple::new(1, "development", 1, 0, "tie_break")).expect("rng"),
    );
    let mut bad = learning_cfg();
    bad.trace_policy = "no_decay_diagnostic".to_owned();
    assert!(
        ContinuousLearner::from_agent_parts(
            actor_cfg(),
            bad,
            small_inherited(),
            2,
            noise.clone(),
            tie.clone(),
        )
        .is_err()
    );
    let mut off = learning_cfg();
    off.enabled = false;
    assert!(
        ContinuousLearner::from_agent_parts(
            actor_cfg(),
            off,
            small_inherited(),
            2,
            noise.clone(),
            tie.clone(),
        )
        .is_err()
    );
    let mut rule = learning_cfg();
    rule.rule = "hebbian".to_owned();
    assert!(
        ContinuousLearner::from_agent_parts(actor_cfg(), rule, small_inherited(), 2, noise, tie,)
            .is_err()
    );

    // Birth: P = E = 0, configured baseline, effective == W0.
    let mut l = learner();
    assert!(l.plastic().p().iter().flatten().all(|&v| v == 0.0));
    assert!(l.plastic().e().iter().flatten().all(|&v| v == 0.0));
    assert_eq!(l.reward_baseline(), 0.5);
    assert_eq!(l.last_feedback(), None);
    assert_eq!(l.ticks_advanced(), 0);
    assert_eq!(
        l.plastic().effective_weights(),
        &l.inherited().weights.w0,
        "effective == W0 at birth"
    );

    // First advance from zero activity creates no scores (r_old = 0);
    // the second advance wires S = alpha * r_old * xi / sigma exactly on
    // plastic edges (E was 0, so no decay term to confound).
    let input = vec![0.0; feature_dim(2)];
    l.advance(&input).expect("tick 0");
    assert!(l.plastic().e().iter().flatten().all(|&v| v == 0.0));
    let r_old = l.actor_state().r().to_vec();
    assert!(r_old.iter().any(|&v| v != 0.0));
    l.advance(&input).expect("tick 1");
    let xi = l.actor_state().last_perturbations().to_vec();
    let alpha = leak_alpha(actor_cfg().tau_h);
    let sigma = actor_cfg().noise_sigma;
    let plastic = [(2, 0), (2, 3), (3, 1), (3, 2)];
    for &(j, i) in &plastic {
        close(
            l.plastic().e()[j][i],
            alpha * r_old[i] * xi[j] / sigma,
            1e-12,
            &format!("E[{j},{i}]"),
        );
    }
}

#[test]
fn baseline_counts_consumed_feedback_only() {
    // Closed form over rewards [1, 0, 1] with beta = 0.02 from 0.5;
    // advances and duplicate deliveries never move it.
    let mut l = learner();
    let input = vec![0.0; feature_dim(2)];
    for _ in 0..3 {
        l.advance(&input).expect("advance");
    }
    assert_eq!(l.reward_baseline(), 0.5);
    let mut want = 0.5;
    for (id, reward) in [(10u64, 1.0), (11, 0.0), (12, 1.0)] {
        let prev = l.reward_baseline();
        let outcome = l
            .apply_feedback(cra::environment::Feedback {
                event_id: id,
                reward,
            })
            .expect("apply");
        want += BETA * (reward - want);
        close(outcome.baseline_old, prev, 0.0, "baseline_old");
        close(outcome.baseline_new, want, 1e-15, "baseline_new");
        close(l.reward_baseline(), want, 1e-15, "stored baseline");
        assert_eq!(l.last_feedback(), Some(id));
        l.advance(&input).expect("advance between outcomes");
        close(
            l.reward_baseline(),
            want,
            0.0,
            "advance must not move baseline",
        );
    }
    // b3 = 0.5 + .02*.5=0.51; +.02*(0-.51)=0.4998; +.02*(1-.4998)=0.509804.
    close(l.reward_baseline(), 0.509804, 1e-12, "closed form");
    let dup = l.apply_feedback(cra::environment::Feedback {
        event_id: 12,
        reward: 1.0,
    });
    assert!(dup.is_err(), "duplicate rejected");
    close(
        l.reward_baseline(),
        0.509804,
        0.0,
        "duplicate moves nothing",
    );
}

#[test]
fn live_trace_includes_post_commitment_scores_no_snapshot() {
    // End-to-end persistent choice on the tick-20/delay-3 schedule with
    // birth_only resets: the tick-23 update uses live E through tick 22
    // (decayed commit-time trace plus delay scores), not a frozen
    // commit-time snapshot and not decay without new scores.
    let mut cfg = base_config();
    cfg.environment.memory_gap_ticks = [5, 5];
    cfg.environment.reward_delay_ticks = [3, 3];
    cfg.simulation.outcomes_per_lifetime = 1;
    cfg.actor = Some(actor_cfg());
    cfg.learning = Some(learning_cfg());
    cra::config::validate(&cfg).expect("fixture config validates");
    let mut lifetime =
        cra::environment::Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    assert_eq!(lifetime.hidden().mapping_snapshot(), vec![1, 1]);
    let mut learner = learner();
    let lambda = (-1.0_f64 / TAU_E).exp();

    for tick in 0..=20u64 {
        let out = lifetime.observe().expect("observe");
        assert!(out.observation.feedback.is_none());
        learner.advance(&out.observation.features).expect("advance");
        lifetime.finish_tick().expect("finish");
        if out.commitment_due {
            assert_eq!(tick, 20);
            lifetime.commit(1).expect("commit correct");
        }
    }
    // E after the tick-20 advance: the commit-time trace (includes
    // tick-20 scores; later delay scores not yet accrued).
    let e_commit = learner.plastic().e().to_vec();
    assert!(e_commit.iter().flatten().any(|&v| v != 0.0));
    for tick in [21, 22] {
        let out = lifetime.observe().expect("delay");
        assert_eq!((out.tick, out.phase), (tick, Phase::Delay));
        learner.advance(&out.observation.features).expect("advance");
        lifetime.finish_tick().expect("finish");
    }
    let e_live = learner.plastic().e().to_vec();
    assert_ne!(e_live, e_commit, "delay ticks must change the live trace");

    let out = lifetime.observe().expect("feedback");
    assert_eq!((out.tick, out.phase), (23, Phase::Feedback));
    let feedback = out.observation.feedback.expect("outcome");
    assert_eq!((feedback.event_id, feedback.reward), (0, 1.0));
    let baseline_old = learner.reward_baseline();
    let delta = feedback.reward - baseline_old;
    let outcome = learner.apply_feedback(feedback).expect("apply");
    lifetime
        .note_feedback_consumed(feedback.event_id)
        .expect("ledger");
    // Live trace: raw equals eta * delta * E_live per edge.
    for (j, (raw_row, e_row)) in outcome.raw_updates.iter().zip(e_live.iter()).enumerate() {
        for (i, (&raw, &e)) in raw_row.iter().zip(e_row.iter()).enumerate() {
            close(raw, ETA * delta * 1.0 * e, 1e-12, &format!("raw[{j},{i}]"));
        }
    }
    // Frozen commit-time snapshot would give a different update.
    let frozen_l1: f64 = e_commit
        .iter()
        .flatten()
        .map(|&e| (ETA * delta * e).abs())
        .sum();
    let applied_l1: f64 = outcome.raw_updates.iter().flatten().map(|v| v.abs()).sum();
    assert!(
        (frozen_l1 - applied_l1).abs() > 0.0,
        "frozen commit-time E must give a different update"
    );
    // Decay without new delay scores would also differ, proving delay
    // scores (not just decay) are in the live trace.
    let decay_only_l1: f64 = e_commit
        .iter()
        .flatten()
        .map(|&e| (ETA * delta * lambda.powi(2) * e).abs())
        .sum();
    assert!(
        (decay_only_l1 - applied_l1).abs() > 0.0,
        "decay-only E must give a different update"
    );
    // Ordered clamps and the single baseline update still hold.
    for (j, (raw_row, (limited_row, actual_row))) in outcome
        .raw_updates
        .iter()
        .zip(
            outcome
                .limited_updates
                .iter()
                .zip(outcome.actual_updates.iter()),
        )
        .enumerate()
    {
        for (i, (&raw, (&limited, &actual))) in raw_row
            .iter()
            .zip(limited_row.iter().zip(actual_row.iter()))
            .enumerate()
        {
            close(
                limited,
                raw.clamp(-0.01, 0.01),
                1e-15,
                &format!("limited[{j},{i}]"),
            );
            close(
                actual,
                limited.clamp(-0.5, 0.5),
                1e-15,
                &format!("actual[{j},{i}]"),
            );
        }
    }
    close(
        learner.reward_baseline(),
        baseline_old + BETA * delta,
        1e-15,
        "baseline once",
    );
    lifetime.finish_tick().expect("finish");
    assert!(lifetime.is_complete());
}
