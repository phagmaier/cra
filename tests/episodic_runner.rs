//! Explicitly episodic clean-learning runner (M3-04; spec 16/M3, 7.6-7.7,
//! 9-10, 19.2).
//!
//! - `episodic_stationary` is a separate diagnostic profile (two cues, zero
//!   noise/hazard, no gap, delay 1, `episodic_diagnostic` resets,
//!   `no_decay_diagnostic` traces, one terminal update per rollout). It
//!   validates its own guard and is rejected by baseline/B3 guards, so it
//!   cannot masquerade as the continuous result.
//! - Birth is `h = a = q = P = E = 0` with baseline from config and
//!   `W_effective = W0`; the learner is built from agent-only inputs
//!   (actor/learning sections, inherited params, cue count, dedicated RNGs).
//! - Each rollout starts with zeroed `h/a/q/E` (preserving `P`/baseline/
//!   dedup/RNGs), accumulates with `lambda = 1`, and applies exactly one
//!   pre-transition update; `P` never moves on non-feedback ticks and `W0`
//!   never moves. Reset ticks, mode, and policies are logged in the summary.
//! - Feedback uses pre-feedback eligibility: the applied update equals
//!   `eta * delta * E_old` (fixed gate 1), not post-feedback `E`.
//! - Env/agent RNG streams stay independent: cue order matches the B3 actor
//!   run on the same seeds.

use cra::agent::plasticity::PlasticState;
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::{Actor, Config, Learning};
use cra::environment::{Feedback, Lifetime, feature_dim};
use cra::experiments::episodic::{EPISODIC_MODE, EpisodicLearner, run_episodic_lifetime};
use cra::rng::{SeedTuple, rng_for};

fn episodic_config() -> Config {
    let text =
        std::fs::read_to_string("configs/episodic_stationary.toml").expect("episodic profile");
    toml::from_str(&text).expect("episodic parses")
}

fn small_outcomes_config(outcomes: u64) -> Config {
    let mut cfg = episodic_config();
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cra::config::validate(&cfg).expect("small config validates");
    cra::config::validate_episodic_execution(&cfg).expect("small config executes");
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

fn small_topology() -> cra::agent::topology::Topology {
    // N = 4, m = 1: M0 = [2], M1 = [3].
    let mask = vec![
        vec![false, true, false, false],
        vec![true, false, false, false],
        vec![true, false, false, true],
        vec![false, true, true, false],
    ];
    topology_from_mask(4, 1, 0.25, false, mask, "test".to_owned()).expect("mask")
}

fn small_inherited() -> InheritedParams {
    let topology = small_topology();
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

fn test_rngs() -> (rand_chacha::ChaCha8Rng, rand_chacha::ChaCha8Rng) {
    let noise = rng_for(&SeedTuple::new(1, "development", 1, 0, "actor_noise")).expect("rng");
    let tie = rng_for(&SeedTuple::new(1, "development", 1, 0, "tie_break")).expect("rng");
    (noise, tie)
}

fn learner() -> EpisodicLearner {
    let (noise, tie) = test_rngs();
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

#[test]
fn episodic_profile_is_a_separate_diagnostic_not_continuous() {
    let cfg = episodic_config();
    cra::config::validate(&cfg).expect("schema validates");
    cra::config::validate_episodic_execution(&cfg).expect("episodic guard passes");
    assert_eq!(cfg.profile_name, "episodic_stationary");
    assert_eq!(cfg.simulation.reset_policy, "episodic_diagnostic");
    assert_eq!(
        cfg.learning.as_ref().expect("learning").trace_policy,
        "no_decay_diagnostic"
    );
    assert_eq!(EPISODIC_MODE, "episodic_diagnostic");
    // Clean task: two cues, zero noise/hazard, no gap, delay 1.
    assert_eq!(cfg.environment.kind, "stationary_clean");
    assert_eq!(cfg.environment.cue_count, 2);
    assert_eq!(cfg.environment.feedback_noise_values, vec![0.0]);
    assert_eq!(cfg.environment.volatile_hazard_values, vec![0.0]);
    assert_eq!(cfg.environment.memory_gap_ticks, [0, 0]);
    assert_eq!(cfg.environment.reward_delay_ticks, [1, 1]);
    assert_eq!(cfg.environment.stable_fraction, 1.0);
    // Baselines and B3 still require birth_only: this profile cannot run as
    // a continuous result.
    assert!(matches!(
        cra::config::validate_baseline_execution(&cfg),
        Err(cra::config::ConfigError::UnsupportedExecution(_))
    ));
    assert!(matches!(
        cra::config::validate_actor_no_learning_execution(&cfg),
        Err(cra::config::ConfigError::UnsupportedExecution(_))
    ));
    // The continuous reference stays distinct.
    let debug: Config =
        toml::from_str(&std::fs::read_to_string("configs/debug_stationary.toml").unwrap()).unwrap();
    assert_eq!(debug.simulation.reset_policy, "birth_only");
    assert!(cra::config::validate_episodic_execution(&debug).is_err());
}

#[test]
fn learner_birth_is_zero_with_agent_only_inputs_and_no_decay() {
    let l = learner();
    assert_eq!(l.ticks_advanced(), 0);
    assert_eq!(l.reward_baseline(), 0.5);
    assert_eq!(l.last_feedback(), None);
    assert_eq!(l.last_output().action_0, 0.0);
    assert_eq!(l.last_output().action_1, 0.0);
    assert!(l.plastic().p().iter().flatten().all(|&v| v == 0.0));
    assert!(l.plastic().e().iter().flatten().all(|&v| v == 0.0));
    assert_eq!(l.plastic().effective_weights(), &l.inherited().weights.w0);
    assert_eq!(l.plastic().trace_policy().decay_factor(), 1.0);
    assert_eq!(l.plastic().trace_policy().name(), "no_decay_diagnostic");
    assert!(l.actor_state().h().iter().all(|&v| v == 0.0));
    assert!(l.actor_state().a().iter().all(|&v| v == 0.0));
    assert_eq!(l.motor_state().q(), [0.0, 0.0]);
}

#[test]
fn learner_construction_rejects_non_diagnostic_policies() {
    let (noise, tie) = test_rngs();
    let (noise2, tie2) = test_rngs();
    // Disabled learning is not an episodic learner.
    let mut off = learning_cfg();
    off.enabled = false;
    assert!(
        EpisodicLearner::from_agent_parts(actor_cfg(), off, small_inherited(), 2, noise, tie)
            .is_err()
    );
    // The persistent continuous trace policy cannot use this learner or its
    // diagnostic reset: it belongs to M4 birth_only runs.
    let mut persistent = learning_cfg();
    persistent.trace_policy = "persistent".to_owned();
    assert!(
        EpisodicLearner::from_agent_parts(
            actor_cfg(),
            persistent,
            small_inherited(),
            2,
            noise2,
            tie2
        )
        .is_err()
    );
}

#[test]
fn diagnostic_reset_needs_the_diagnostic_trace_policy() {
    let topology = small_topology();
    let mut w0 = vec![vec![0.0; 4]; 4];
    for (j, row) in topology.mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                w0[j][i] = 0.05;
            }
        }
    }
    let mut diag = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "no_decay_diagnostic",
        32.0,
        0.5,
    )
    .unwrap();
    diag.reset_traces_episodic_diagnostic()
        .expect("diagnostic policy resets");
    let mut cont = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        32.0,
        0.5,
    )
    .unwrap();
    assert!(cont.reset_traces_episodic_diagnostic().is_err());
}

#[test]
fn reset_zeros_state_and_traces_but_preserves_offsets_baseline_and_dedup() {
    let mut l = learner();
    // Accumulate eligibility over several ordinary transitions. The first
    // tick has r_old = 0 (zero scores); later ticks carry activity.
    let input = vec![0.0; feature_dim(2)];
    for _ in 0..6 {
        l.advance(&input).expect("advance");
    }
    assert_eq!(l.ticks_advanced(), 6);
    assert!(
        l.plastic().e().iter().flatten().any(|&v| v != 0.0),
        "eligibility must accumulate before reset"
    );
    // One terminal update creates P, moves the baseline, and marks dedup.
    let before_p = l.plastic().p().to_vec();
    let outcome = l
        .apply_feedback(Feedback {
            event_id: 0,
            reward: 1.0,
        })
        .expect("feedback");
    assert_eq!(outcome.event_id, 0);
    assert!((outcome.delta - (1.0 - 0.5)).abs() < 1e-15);
    assert!((l.reward_baseline() - 0.5 - 0.02 * 0.5).abs() < 1e-15);
    assert_eq!(l.last_feedback(), Some(0));
    let after_p = l.plastic().p().to_vec();
    assert_ne!(before_p, after_p, "P must move at the terminal update");
    // Duplicate feedback is rejected without changing state.
    assert!(
        l.apply_feedback(Feedback {
            event_id: 0,
            reward: 1.0,
        })
        .is_err()
    );
    assert_eq!(l.plastic().p(), &after_p);
    // Diagnostic reset: h/a/q/E and readout return to zero; P, baseline,
    // dedup, tick count, and effective cache persist.
    let effective_before = l.plastic().effective_weights().to_vec();
    l.reset_between_rollouts().expect("reset");
    assert!(l.actor_state().h().iter().all(|&v| v == 0.0));
    assert!(l.actor_state().a().iter().all(|&v| v == 0.0));
    assert_eq!(l.motor_state().q(), [0.0, 0.0]);
    assert_eq!(l.last_output().action_0, 0.0);
    assert_eq!(l.last_output().action_1, 0.0);
    assert!(l.plastic().e().iter().flatten().all(|&v| v == 0.0));
    assert_eq!(l.plastic().p(), &after_p);
    assert_eq!(l.plastic().effective_weights(), &effective_before);
    assert!((l.reward_baseline() - (0.5 + 0.02 * 0.5)).abs() < 1e-15);
    assert_eq!(l.last_feedback(), Some(0));
    assert_eq!(
        l.ticks_advanced(),
        6,
        "tick count is lifetime time, not reset"
    );
}

#[test]
fn p_never_moves_on_non_feedback_ticks() {
    let mut l = learner();
    let input = vec![0.0; feature_dim(2)];
    let p0 = l.plastic().p().to_vec();
    for _ in 0..4 {
        l.advance(&input).expect("advance");
        assert_eq!(
            l.plastic().p(),
            &p0,
            "eligibility-only ticks must not touch P"
        );
    }
}

#[test]
fn feedback_uses_pre_transition_eligibility_not_post_feedback_scores() {
    // Learning-sensitive ordering: the update equals eta * delta * E_old
    // (fixed gate 1), even though advancing the feedback tick afterwards
    // creates new scores in E.
    let mut l = learner();
    let input = vec![0.0; feature_dim(2)];
    for _ in 0..5 {
        l.advance(&input).expect("advance");
    }
    let e_old = l.plastic().e().to_vec();
    assert!(e_old.iter().flatten().any(|&v| v != 0.0));
    let baseline_old = l.reward_baseline();
    let eta = 0.001;
    let outcome = l
        .apply_feedback(Feedback {
            event_id: 3,
            reward: 1.0,
        })
        .expect("feedback");
    let delta = 1.0 - baseline_old;
    // Every plastic edge matches the pre-feedback trace; missing/nonplastic
    // edges stay exactly zero in all three reports.
    for (j, (raw_row, e_row)) in outcome.raw_updates.iter().zip(e_old.iter()).enumerate() {
        for (i, (&raw, &e)) in raw_row.iter().zip(e_row.iter()).enumerate() {
            let expected = eta * delta * 1.0 * e;
            assert!(
                (raw - expected).abs() <= 1e-15,
                "edge ({j},{i}): raw {raw} != eta*delta*E_old {expected}"
            );
        }
    }
    // Advancing the feedback tick creates post-outcome scores, proving the
    // two traces differ and the update could not have used the new one.
    l.advance(&input).expect("post-feedback advance");
    let e_new = l.plastic().e().to_vec();
    assert_ne!(e_old, e_new, "feedback-tick scores must differ from E_old");
    // Recomputing with E_new would give a different raw update.
    let would_be: f64 = e_new.iter().flatten().map(|v| v.abs()).sum();
    let was: f64 = e_old.iter().flatten().map(|v| v.abs()).sum();
    assert!(
        (would_be - was).abs() > 0.0,
        "E magnitudes must differ to make the ordering test sensitive"
    );
}

#[test]
fn run_logs_diagnostic_resets_and_one_terminal_update_per_rollout() {
    let cfg = small_outcomes_config(4);
    let summary = run_episodic_lifetime(&cfg, 1, "development", 1, 0, "episodic-learner")
        .expect("episodic run");
    assert_eq!(summary.mode, "episodic_diagnostic");
    assert_eq!(summary.reset_policy, "episodic_diagnostic");
    assert_eq!(summary.trace_policy, "no_decay_diagnostic");
    assert_eq!(summary.profile_name, "episodic_stationary");
    assert_eq!(summary.plastic_mask, "motor_afferent_only");
    assert_eq!(summary.outcomes, 4);
    assert_eq!(summary.commitments, 4);
    assert_eq!(summary.choices.len(), 4);
    assert_eq!(summary.annotations.len(), 4);
    // One reset per rollout, starting at birth; strictly increasing.
    assert_eq!(summary.resets.len(), 4);
    assert_eq!(summary.resets[0], 0);
    for window in summary.resets.windows(2) {
        assert!(window[0] < window[1], "resets must increase: {window:?}");
    }
    // Every rollout holds exactly one choice/update: event, choice, and
    // rollout indices agree, and update reports are consumed in order.
    for (k, choice) in summary.choices.iter().enumerate() {
        let k = k as u64;
        assert_eq!(choice.event_id, k);
        assert_eq!(choice.choice_index, k);
        assert_eq!(choice.rollout_index, k);
        assert_eq!(choice.rollout_start_tick, summary.resets[k as usize]);
        assert!(choice.feedback_tick > choice.commit_tick);
        assert!(choice.action <= 1);
        assert!(choice.reward == 0.0 || choice.reward == 1.0);
        assert_eq!(choice.update.event_id, k);
        assert!(
            choice
                .update
                .raw_updates
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
        assert!(
            choice
                .update
                .limited_updates
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
        assert!(
            choice
                .update
                .actual_updates
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
        // Zero noise: observed reward equals latent correctness.
        assert_eq!(choice.reward, f64::from(choice.correct));
        assert!(!choice.noise_bit);
    }
    // Hidden annotations join by explicit keys and stay evaluator-side.
    for (choice, annotation) in summary.choices.iter().zip(summary.annotations.iter()) {
        assert_eq!(choice.event_id, annotation.event_id);
        assert_eq!(choice.choice_index, annotation.choice_index);
        assert_eq!(choice.cue, annotation.cue_id);
    }
    assert!(summary.mean_reward.is_finite());
    assert!((0.0..=1.0).contains(&summary.mean_reward));
    assert_eq!(summary.final_last_feedback, Some(3));
    // W0 never changes; effective stays W0 + P; P respects bounds/mask.
    assert_eq!(summary.w0.len(), 16);
    for (j, (w_row, eff_row)) in summary
        .w0
        .iter()
        .zip(summary.final_effective.iter())
        .enumerate()
    {
        for (i, (&w, &eff)) in w_row.iter().zip(eff_row.iter()).enumerate() {
            let p = summary.final_p[j][i];
            assert!((eff - (w + p)).abs() <= 1e-12, "effective must be W0 + P");
            assert!(p.abs() <= 0.5 + 1e-12, "P must respect plastic_bound");
        }
    }
    assert!(summary.final_baseline.is_finite());
}

#[test]
fn full_profile_run_completes_and_stays_finite() {
    // The committed 50-outcome diagnostic completes without numerical
    // failure; this is a runner smoke check, not an acquisition claim
    // (M3-07 owns the learning-vs-control comparison).
    let cfg = episodic_config();
    assert_eq!(cfg.simulation.outcomes_per_lifetime, 50);
    let summary = run_episodic_lifetime(&cfg, 1, "development", 1, 0, "episodic-learner")
        .expect("full episodic run");
    assert_eq!(summary.outcomes, 50);
    assert_eq!(summary.resets.len(), 50);
    assert_eq!(summary.choices.len(), 50);
    assert!(summary.mean_reward.is_finite());
    assert!(summary.final_p.iter().flatten().all(|v| v.is_finite()));
    assert!(summary.final_e.iter().flatten().all(|v| v.is_finite()));
    assert!(summary.final_baseline.is_finite());
    assert!(summary.ticks > 50, "each rollout spans several ticks");
}

#[test]
fn cue_schedule_is_independent_of_learner_noise_draws() {
    // Same exogenous seeds must yield the same cue order whether the
    // lifetime is driven by the noisy plastic learner or by a constant
    // action policy: agent draws never perturb cue/change/noise/timing
    // schedules (spec 5.8). The constant path draws no actor noise, the
    // episodic path draws per-tick perturbations; matching cues prove
    // stream separation.
    let cfg = small_outcomes_config(4);
    let episodic =
        run_episodic_lifetime(&cfg, 1, "development", 1, 0, "episodic-learner").expect("episodic");
    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("lifetime");
    let mut constant_cues = Vec::new();
    while !lifetime.is_complete() {
        let out = lifetime.advance().expect("tick");
        if let Some(feedback) = out.observation.feedback {
            let annotation = out.annotation.clone().expect("annotation");
            constant_cues.push(annotation.cue_id);
            lifetime
                .note_feedback_consumed(feedback.event_id)
                .expect("consume");
        }
        if out.commitment_due {
            lifetime.commit(0).expect("commit");
        }
    }
    let episodic_cues: Vec<usize> = episodic.choices.iter().map(|c| c.cue).collect();
    assert_eq!(
        episodic_cues, constant_cues,
        "cue order must not depend on learner noise draws"
    );
    // Determinism: rerunning the same episodic lifetime reproduces cues,
    // rewards, resets, and final plastic state exactly.
    let rerun =
        run_episodic_lifetime(&cfg, 1, "development", 1, 0, "episodic-learner").expect("rerun");
    assert_eq!(
        episodic.choices.iter().map(|c| c.cue).collect::<Vec<_>>(),
        rerun.choices.iter().map(|c| c.cue).collect::<Vec<_>>()
    );
    assert_eq!(
        episodic
            .choices
            .iter()
            .map(|c| c.reward)
            .collect::<Vec<_>>(),
        rerun.choices.iter().map(|c| c.reward).collect::<Vec<_>>()
    );
    assert_eq!(episodic.resets, rerun.resets);
    assert_eq!(episodic.final_p, rerun.final_p);
    assert_eq!(episodic.final_baseline, rerun.final_baseline);
}
