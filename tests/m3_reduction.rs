//! Failure-isolation diagnostics for the episodic learner (M3-09;
//! spec M3 "If it fails", 21.1-21.2).
//!
//! Ladder (apply in order; stop at the first failing step):
//! 1. Sign: one plastic edge, hand-set trace, +/-delta moves `P` as
//!    `eta * delta * E` with separated raw/limited/actual reports.
//! 2. Order: every applied update equals `eta * delta * E_old` read
//!    before the feedback transition (checked per outcome inside the
//!    single-motor runner, asserted here).
//! 3. Minimal closed loop: single noisy motor unit, constant input, known
//!    preferred action, synthetic reward. Offsets must drift toward the
//!    preferred action while step 2 holds on every outcome.
//! 4. Representation: real `P` movement with locked behavior (the outer-1
//!    pattern) is a representation/birth-lock failure, not plasticity.
//! 5. Sensitivity: receiver-permuted perturbations must change the
//!    updates; exact equivalence with the correct assignment is a wiring
//!    bug to investigate, never a tuned-away non-result. The identity
//!    permutation reproduces the verified runner exactly.
//!
//! Status in M3: the path was NOT needed to rescue acquisition
//! (M3-07/M3-08 pass). It is validated here as working tooling, and step
//! 4 is demonstrated on the documented outer-1 case.

use cra::agent::plasticity::{FeedbackUpdateParams, PlasticState};
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::{Actor, Config, Learning};
use cra::environment::feature_dim;
use cra::experiments::episodic::{EpisodicLearner, run_episodic_conditions, run_episodic_lifetime};
use cra::experiments::reduction::{run_episodic_permuted_lifetime, run_single_motor_diagnostic};
use cra::rng::{SeedTuple, rng_for};

// ---------------------------------------------------------------------------
// Tiny single-edge constructions (ladder steps 1-3).
// ---------------------------------------------------------------------------

/// N = 2, one motor neuron per action (M0 = [0], M1 = [1]), single
/// existing edge 1 -> 0. Under `motor_afferent_only` the only plastic
/// edge is (receiver 0, sender 1).
fn single_edge_topology() -> cra::agent::topology::Topology {
    topology_from_mask(
        2,
        1,
        0.25,
        false,
        vec![vec![false, true], vec![false, false]],
        "test".to_owned(),
    )
    .expect("single-edge mask builds")
}

fn tiny_actor_cfg() -> Actor {
    Actor {
        neuron_count: 2,
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

fn tiny_learning(eta: f64, max_update: f64) -> Learning {
    Learning {
        enabled: true,
        rule: "gaussian_transition_score".to_owned(),
        plastic_mask: "motor_afferent_only".to_owned(),
        trace_policy: "no_decay_diagnostic".to_owned(),
        tau_e: 32.0,
        eta,
        max_update,
        plastic_bound: 0.5,
        plastic_decay: 0.0,
        reward_baseline_initial: 0.5,
        reward_baseline_beta: 0.02,
    }
}

/// Hand-built inheritance for the single edge: neutral `W0` (the
/// diagnostic must create any bias through learning) and a symmetric
/// sensory drive so both motor pools start contested on constant input.
fn tiny_inherited(drive: f64) -> InheritedParams {
    let topology = single_edge_topology();
    let w0 = vec![vec![0.0; 2], vec![0.0; 2]];
    let mut b0 = vec![0.0; feature_dim(2)];
    let mut b1 = vec![0.0; feature_dim(2)];
    b0[0] = drive;
    b1[0] = drive;
    InheritedParams {
        topology,
        weights: Weights {
            w0,
            input_weights: vec![b0, b1],
            bias: vec![0.0; 2],
        },
    }
}

fn tiny_rngs() -> (rand_chacha::ChaCha8Rng, rand_chacha::ChaCha8Rng) {
    let noise = rng_for(&SeedTuple::new(1, "development", 1, 0, "actor_noise")).expect("rng");
    let tie = rng_for(&SeedTuple::new(1, "development", 1, 0, "tie_break")).expect("rng");
    (noise, tie)
}

fn tiny_learner(eta: f64, max_update: f64, drive: f64) -> EpisodicLearner {
    let (noise, tie) = tiny_rngs();
    EpisodicLearner::from_agent_parts(
        tiny_actor_cfg(),
        tiny_learning(eta, max_update),
        tiny_inherited(drive),
        2,
        noise,
        tie,
    )
    .expect("tiny learner builds")
}

// ---------------------------------------------------------------------------
// Step 1: sign on one weight with a hand-set trace.
// ---------------------------------------------------------------------------

#[test]
fn single_edge_update_sign_and_order() {
    let topology = single_edge_topology();
    let w0 = vec![vec![0.0; 2], vec![0.0; 2]];
    let params = FeedbackUpdateParams {
        eta: 0.01,
        max_update: 0.01,
        baseline_beta: 0.02,
    };
    // Exactly one plastic edge: (receiver 0, sender 1).
    let probe = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "no_decay_diagnostic",
        32.0,
        0.5,
    )
    .expect("plastic state builds");
    assert_eq!(probe.plastic_edges(), &[(0usize, 1usize)]);
    // Hand-set trace: sender 1 active, receiver-0 perturbation 0.5.
    // E[0][1] = alpha * r * xi / sigma = 0.2 * 1.0 * 0.5 / 0.4 = 0.25.
    let mut up = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "no_decay_diagnostic",
        32.0,
        0.5,
    )
    .expect("plastic state builds");
    up.advance_eligibility(&[0.0, 1.0], &[0.5, 0.0], 0.2, 0.4)
        .expect("eligibility advances");
    assert!((up.e()[0][1] - 0.25).abs() < 1e-15);
    assert_eq!(up.e()[0][0], 0.0);
    assert_eq!(up.e()[1][0], 0.0);
    assert_eq!(up.e()[1][1], 0.0);
    // Reward 1 with baseline 0.5: delta = +0.5, raw = 0.01 * 0.5 * 0.25.
    let out = up
        .apply_feedback_once(0, 1.0, &[1.0, 1.0], params, &w0)
        .expect("feedback applies");
    assert!((out.delta - 0.5).abs() < 1e-15);
    assert!((out.raw_updates[0][1] - 0.00125).abs() < 1e-15);
    assert_eq!(out.limited_updates[0][1], out.raw_updates[0][1]);
    assert!((out.actual_updates[0][1] - 0.00125).abs() < 1e-15);
    assert!((up.p()[0][1] - 0.00125).abs() < 1e-15);
    assert!((up.reward_baseline() - 0.51).abs() < 1e-15);
    for (j, (raw_row, lim_row)) in out
        .raw_updates
        .iter()
        .zip(out.limited_updates.iter())
        .enumerate()
    {
        for (i, (&raw, &lim)) in raw_row.iter().zip(lim_row.iter()).enumerate() {
            if (j, i) != (0, 1) {
                assert_eq!(raw, 0.0, "nonplastic edge ({j},{i}) raw must be zero");
                assert_eq!(lim, 0.0, "nonplastic edge ({j},{i}) limited must be zero");
                assert_eq!(
                    out.actual_updates[j][i], 0.0,
                    "nonplastic edge ({j},{i}) actual must be zero"
                );
            }
        }
    }
    // Duplicate delivery changes nothing.
    let p_before = up.p().to_vec();
    assert!(
        up.apply_feedback_once(0, 1.0, &[1.0, 1.0], params, &w0)
            .is_err()
    );
    assert_eq!(up.p(), &p_before);
    // Reward 0 on a fresh state flips the sign with equal magnitude.
    let mut down = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "no_decay_diagnostic",
        32.0,
        0.5,
    )
    .expect("plastic state builds");
    down.advance_eligibility(&[0.0, 1.0], &[0.5, 0.0], 0.2, 0.4)
        .expect("eligibility advances");
    let out0 = down
        .apply_feedback_once(0, 0.0, &[1.0, 1.0], params, &w0)
        .expect("feedback applies");
    assert!((out0.delta + 0.5).abs() < 1e-15);
    assert!((out0.raw_updates[0][1] + 0.00125).abs() < 1e-15);
    assert!((down.p()[0][1] + 0.00125).abs() < 1e-15);
}

// ---------------------------------------------------------------------------
// Steps 2-3: minimal closed loop with constant input + preferred action.
// ---------------------------------------------------------------------------

#[test]
fn single_noisy_motor_closed_loop() {
    // Minimal closed loop (ladder step 3): one noisy motor unit,
    // constant input, known preferred action 0, synthetic reward.
    // Fixed seed tuple; diagnostic constants (drive 1.0, 50
    // ticks/outcome, 30 outcomes, eta 0.001, max_update 0.05) chosen so
    // the contest starts near 50/50 and updates stay unclipped.
    let (noise, tie) = tiny_rngs();
    let mut features = vec![0.0; feature_dim(2)];
    features[0] = 1.0;
    let summary = run_single_motor_diagnostic(
        tiny_actor_cfg(),
        tiny_learning(0.001, 0.05),
        tiny_inherited(1.0),
        2,
        noise,
        tie,
        features,
        50,
        30,
        0,
    )
    .expect("diagnostic runs");
    assert_eq!(summary.outcomes, 30);
    assert_eq!(summary.choices.len(), 30);
    assert_eq!(summary.plastic_edge, (0, 1));
    // Step-2 identity on every outcome: the update used pre-feedback E.
    for choice in &summary.choices {
        assert!(
            choice.max_raw_identity_error < 1e-12,
            "outcome {}: raw != eta*delta*E_old ({})",
            choice.outcome_index,
            choice.max_raw_identity_error
        );
        assert_eq!(choice.update.event_id, choice.outcome_index);
    }
    // Only the single plastic edge ever moves.
    for (j, row) in summary.final_p.iter().enumerate() {
        for (i, &p) in row.iter().enumerate() {
            if (j, i) != (0, 1) {
                assert_eq!(p, 0.0, "nonplastic P[{j},{i}] must stay zero");
            }
        }
    }
    let early = summary.preferred_rate(0..10);
    let late = summary.preferred_rate(20..30);
    let clipped = summary.choices.iter().filter(|c| c.clipped).count();
    eprintln!(
        "single-motor: early preferred rate {early:.3}, late {late:.3}, \
         final P[0][1] {:.6}, clipped {clipped}/30, baseline {:.4}",
        summary.final_p[0][1], summary.final_baseline
    );
    // Direction: the single offset drifts toward the preferred action
    // (positive P on edge (0,1) with positive sender activity raises
    // motor-0 drive) and the preferred-action rate improves.
    assert!(
        summary.final_p[0][1] > 0.0,
        "P[0][1] must drift positive for preferred action 0"
    );
    assert!(
        late > early,
        "preferred rate must improve (early {early:.3}, late {late:.3})"
    );
    assert_eq!(clipped, 0, "tiny-system updates must stay unclipped");
}

// ---------------------------------------------------------------------------
// Winner-config helpers and steps 4-5.
// ---------------------------------------------------------------------------

fn winner_full_mask_config(outcomes: u64) -> Config {
    let text =
        std::fs::read_to_string("configs/episodic_stationary.toml").expect("episodic profile");
    let mut cfg: Config = toml::from_str(&text).expect("episodic parses");
    // Frozen M3-07 winner (grid index 11); read here as literals so this
    // diagnostic cannot silently re-tune (the grid manifest owns them).
    let actor = cfg.actor.as_mut().expect("base has [actor]");
    actor.input_scale = 0.2;
    actor.recurrent_gain = 0.8;
    actor.noise_sigma = 0.05;
    let learning = cfg.learning.as_mut().expect("base has [learning]");
    learning.eta = 0.001;
    learning.plastic_mask = "all_recurrent_edges".to_owned();
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cra::config::validate(&cfg).expect("config validates");
    cra::config::validate_episodic_execution(&cfg).expect("config executes");
    cfg
}

#[test]
fn birth_lock_is_representation_failure_not_plasticity() {
    // Step 4 on the documented outer-1 case: plasticity is alive (P
    // moves through the verified update path) while behavior and both
    // controls stay locked at 0.0. No rescue by evolution or hidden
    // resets is attempted; the verdict names the family bound.
    let cfg = winner_full_mask_config(60);
    let set = run_episodic_conditions(&cfg, 1, "development", 1, 0).expect("conditions");
    let p_l1: f64 = set.b4.final_p.iter().flatten().map(|v| v.abs()).sum();
    assert!(
        p_l1 > 0.0,
        "outer-1 B4 must move real offsets (plasticity alive)"
    );
    assert_eq!(set.b4.final_last_feedback, Some(59));
    let late =
        |correct: &[bool]| correct.iter().rev().take(20).filter(|&&c| c).count() as f64 / 20.0;
    let b4_correct: Vec<bool> = set.b4.choices.iter().map(|c| c.correct).collect();
    let b3_correct: Vec<bool> = set.b3.choices.iter().map(|c| c.correct).collect();
    assert_eq!(late(&b4_correct), 0.0, "outer-1 B4 behavior stays locked");
    assert_eq!(late(&b3_correct), 0.0, "outer-1 B3 behavior stays locked");
    assert!(
        (set.b4.latent_accuracy() - set.b3.latent_accuracy()).abs() < 1e-15,
        "no learning margin where representation is locked"
    );
}

#[test]
fn permutation_hook_rejects_bad_permutations() {
    let mut learner = tiny_learner(0.001, 0.05, 1.0);
    let mut features = vec![0.0; feature_dim(2)];
    features[0] = 1.0;
    // Wrong length, out of range, duplicated.
    assert!(
        learner
            .advance_with_receiver_permutation(&features, &[0])
            .is_err()
    );
    assert!(
        learner
            .advance_with_receiver_permutation(&features, &[0, 2])
            .is_err()
    );
    assert!(
        learner
            .advance_with_receiver_permutation(&features, &[1, 1])
            .is_err()
    );
    // Valid reversal steps the transition normally.
    let out = learner
        .advance_with_receiver_permutation(&features, &[1, 0])
        .expect("valid permutation advances");
    assert!(out.action_0.is_finite() && out.action_1.is_finite());
}

#[test]
fn permuted_perturbations_change_updates() {
    // Step 5: reversed receiver perturbations must change the updates
    // (no unexpected equivalence), while the identity permutation
    // reproduces the verified runner exactly (hook fidelity). 600
    // outcomes keep the run fast while letting the correct assignment
    // reach visible acquisition for an informative (unasserted) report.
    let cfg = winner_full_mask_config(600);
    let correct = run_episodic_lifetime(&cfg, 1, "development", 2, 0, "B4").expect("B4 runs");
    assert_eq!(correct.outcomes, 600);
    let n = cfg.actor.as_ref().expect("actor").neuron_count;
    let reversed: Vec<usize> = (0..n).rev().collect();
    let permuted =
        run_episodic_permuted_lifetime(&cfg, 1, "development", 2, 0, "B4-permuted-xi", &reversed)
            .expect("permuted runs");
    assert_eq!(permuted.outcomes, 600);
    assert_eq!(permuted.condition_id, "B4-permuted-xi");
    assert_eq!(permuted.resets, correct.resets, "schedule must match");
    assert!(permuted.final_p.iter().flatten().all(|v| v.is_finite()));
    assert_ne!(
        permuted.final_p, correct.final_p,
        "permuted updates must differ from the correct assignment"
    );
    // Hook fidelity: identity permutation is bitwise the verified runner.
    let identity: Vec<usize> = (0..n).collect();
    let replay =
        run_episodic_permuted_lifetime(&cfg, 1, "development", 2, 0, "B4-identity", &identity)
            .expect("identity runs");
    assert_eq!(replay.final_p, correct.final_p);
    assert_eq!(replay.final_e, correct.final_e);
    assert!((replay.final_baseline - correct.final_baseline).abs() < 1e-15);
    assert_eq!(
        replay.choices.iter().map(|c| c.action).collect::<Vec<_>>(),
        correct.choices.iter().map(|c| c.action).collect::<Vec<_>>()
    );
    // Reported, not asserted: how the wrong assignment behaved. No
    // particular failure magnitude is demanded; mechanism-level
    // sensitivity is already proven by the P-difference above.
    let late = |choices: &[cra::experiments::episodic::EpisodicChoice]| {
        choices.iter().rev().take(20).filter(|c| c.correct).count() as f64 / 20.0
    };
    let l1 = |m: &[Vec<f64>]| m.iter().flatten().map(|v| v.abs()).sum::<f64>();
    eprintln!(
        "PERMUTATION outer-2 (600 outcomes): correct late {:.3} P-L1 {:.3}; reversed late {:.3} P-L1 {:.3}",
        late(&correct.choices),
        l1(&correct.final_p),
        late(&permuted.choices),
        l1(&permuted.final_p)
    );
}
