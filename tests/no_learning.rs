//! Nonplastic actor integration contracts (M1-07, B3).
//!
//! - Birth is zero with paired inheritance: lifetimes under one outer seed
//!   share mask/`W0`/`B`; independent outer seeds vary them.
//! - Every phase advances the network with no boundary resets; `W0` stays
//!   bit-identical across a full lifetime.
//! - Environment scheduling is unchanged from M0: paired actor and
//!   constant-baseline lifetimes share cues/noise/event ids while rewards
//!   follow each policy's own action.
//! - Public reward is sensory input only: `apply_feedback` changes no
//!   weights/state, while different outcome values drive different
//!   trajectories through `B`.
//! - Duplicate/invalid feedback and feature-width mismatches fail without
//!   state change; observation reads draw no randomness (logging
//!   invariance); execution guards separate env-only, no-learning, and
//!   future plastic/gated/search modes.

use cra::agent::no_learning::{NoLearningActor, NoLearningError};
use cra::config::{Actor, Config};
use cra::environment::{Agent, Feedback, Lifetime, SimError, feature_dim};
use cra::experiments::baseline::{ConstantBaseline, run_actor_ordinary, run_ordinary};

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

fn no_learning_config() -> Config {
    let mut cfg: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("smoke parses");
    cfg.profile_name = "actor_no_learning_test".to_owned();
    cfg.simulation.outcomes_per_lifetime = 6;
    cfg.actor = Some(actor_cfg());
    // No [learning]/[modulator]/[evolution]: plasticity, gates, and search
    // arrive in later milestones. The guard also accepts explicitly
    // disabled sections (covered below).
    cfg.learning = None;
    cfg.modulator = None;
    cfg.evolution = None;
    cra::config::validate(&cfg).expect("test config validates");
    cfg
}

fn birth(cfg: &Config, outer: u64, lifetime: u64) -> NoLearningActor {
    NoLearningActor::new(cfg, 1, "development", outer, lifetime).expect("birth")
}

#[test]
fn birth_is_zero_with_paired_inheritance() {
    let cfg = no_learning_config();
    let a0 = birth(&cfg, 1, 0);
    let a1 = birth(&cfg, 1, 1);
    // Birth dynamics state is zero; readout is the continuous [0, 0].
    assert_eq!(a0.actor_state().h(), &[0.0; 16]);
    assert_eq!(a0.actor_state().a(), &[0.0; 16]);
    assert_eq!(a0.actor_state().r(), &[0.0; 16]);
    assert_eq!(a0.motor_state().q(), [0.0, 0.0]);
    assert_eq!(
        a0.last_output(),
        cra::environment::MotorOutput {
            action_0: 0.0,
            action_1: 0.0
        }
    );
    assert_eq!(a0.ticks_advanced(), 0);
    assert_eq!(a0.last_feedback(), None);
    // Paired inheritance: lifetimes under one outer seed share the sampled
    // mask and weights by construction (future B4/B5/B6 pair on this).
    assert_eq!(a0.inherited().topology.mask, a1.inherited().topology.mask);
    assert_eq!(a0.inherited().weights, a1.inherited().weights);
    // Independent outer seeds vary the initialization.
    let b = birth(&cfg, 2, 0);
    assert_ne!(
        a0.inherited().weights.w0,
        b.inherited().weights.w0,
        "outer seeds must vary inherited weights"
    );
}

#[test]
fn every_phase_advances_without_resets_and_weights_stay_identical() {
    let cfg = no_learning_config();
    let mut actor = birth(&cfg, 1, 0);
    let w0_before = actor.inherited().weights.w0.clone();
    let b_before = actor.inherited().weights.input_weights.clone();
    let bias_before = actor.inherited().weights.bias.clone();

    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut ticks = 0u64;
    let mut feedbacks = 0u64;
    let mut saw_nonzero_after_feedback = false;
    while !lifetime.is_complete() {
        let out = lifetime.advance().expect("advance");
        if let Some(feedback) = out.observation.feedback {
            // Feedback is consumed before the transition, then processed as
            // sensory input below — never as a state reset.
            let h_before = actor.actor_state().h().to_vec();
            actor.apply_feedback(feedback).expect("consume");
            assert_eq!(
                actor.actor_state().h(),
                h_before.as_slice(),
                "apply_feedback must not touch activity"
            );
            lifetime
                .note_feedback_consumed(feedback.event_id)
                .expect("ledger");
            feedbacks += 1;
        }
        let h_before = actor.actor_state().h().to_vec();
        let q_before = actor.motor_state().q();
        let motor_out = actor.advance(&out.observation.features).expect("step");
        ticks += 1;
        assert_eq!(actor.ticks_advanced(), ticks);
        assert!(motor_out.action_0.is_finite() && motor_out.action_1.is_finite());
        assert_eq!(actor.last_output(), motor_out);
        // The transition moves state every tick (independent Gaussian draw
        // plus leak); a boundary reset would snap back to birth zeros.
        assert_ne!(
            actor.actor_state().h(),
            h_before.as_slice(),
            "tick {ticks}: network must advance"
        );
        if feedbacks > 0 {
            let h_norm: f64 = actor
                .actor_state()
                .h()
                .iter()
                .map(|v| v * v)
                .sum::<f64>()
                .sqrt();
            let q_norm = (q_before[0] * q_before[0] + q_before[1] * q_before[1]).sqrt();
            // After the first outcome the network has lived through warmup
            // plus a full choice cycle; neither membrane nor filter may be
            // sitting at the birth zero (a reset signature).
            if ticks > 10 {
                assert!(
                    h_norm > 1e-12 || q_norm > 1e-12,
                    "tick {ticks}: state must persist across feedback"
                );
                if h_norm > 1e-12 {
                    saw_nonzero_after_feedback = true;
                }
            }
        }
        if out.commitment_due {
            let action = actor.select_action();
            assert!(action <= 1);
            lifetime.commit(action).expect("commit");
        }
    }
    assert_eq!(feedbacks, 6);
    assert!(ticks > 6, "a lifetime spans many ticks per outcome");
    assert!(
        saw_nonzero_after_feedback,
        "post-feedback activity must be nonzero somewhere"
    );
    // Inherited weights never learn: bit-identical after a full lifetime.
    assert_eq!(actor.inherited().weights.w0, w0_before);
    assert_eq!(actor.inherited().weights.input_weights, b_before);
    assert_eq!(actor.inherited().weights.bias, bias_before);
    assert_eq!(actor.last_feedback(), Some(5));
}

#[test]
fn actor_runner_completes_lifetime_and_keeps_weights_fixed() {
    let cfg = no_learning_config();
    let mut actor = birth(&cfg, 1, 0);
    let w0_before = actor.inherited().weights.w0.clone();
    let summary = run_actor_ordinary(
        &cfg,
        1,
        "development",
        1,
        0,
        "actor-no-learning",
        &mut actor,
    )
    .expect("actor run");
    assert_eq!(summary.choices.len(), 6);
    assert_eq!(summary.annotations.len(), 6);
    for (n, choice) in summary.choices.iter().enumerate() {
        assert_eq!(choice.event_id, n as u64);
        assert_eq!(choice.choice_index, n as u64);
        assert!(choice.action <= 1);
        assert!(choice.reward == 0.0 || choice.reward == 1.0);
        assert_eq!(
            choice.reward,
            f64::from(choice.correct ^ choice.noise_bit),
            "reward follows own action through the production path"
        );
    }
    assert_eq!(actor.inherited().weights.w0, w0_before);
    assert!(actor.ticks_advanced() > 6);
}

#[test]
fn environment_schedule_matches_constant_baseline() {
    // Same seed tuple: identical exogenous cue/noise/event schedule for the
    // actor and B1, while each reward follows its own action.
    let cfg = no_learning_config();
    // B1 needs the env-only profile (same timing, no actor section).
    let mut env_only: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("parses");
    env_only.simulation.outcomes_per_lifetime = 6;
    let mut constant = ConstantBaseline::new(0).expect("constant");
    let baseline = run_ordinary(
        &env_only,
        1,
        "development",
        1,
        0,
        "constant-0",
        &mut constant,
    )
    .expect("baseline run");
    let mut actor = birth(&cfg, 1, 0);
    let actor_summary = run_actor_ordinary(
        &cfg,
        1,
        "development",
        1,
        0,
        "actor-no-learning",
        &mut actor,
    )
    .expect("actor run");
    assert_eq!(actor_summary.choices.len(), baseline.choices.len());
    for (a, b) in actor_summary.choices.iter().zip(baseline.choices.iter()) {
        assert_eq!(a.cue, b.cue, "same cue schedule");
        assert_eq!(a.noise_bit, b.noise_bit, "shared noise bits");
        assert_eq!(a.event_id, b.event_id, "same event identity");
        assert_eq!(a.commit_tick, b.commit_tick, "same tick counts");
        assert_eq!(a.feedback_tick, b.feedback_tick, "same delay");
        assert_eq!(a.reward, f64::from(a.correct ^ a.noise_bit));
        assert_eq!(b.reward, f64::from(b.correct ^ b.noise_bit));
    }
}

#[test]
fn reward_is_sensory_input_only() {
    // Same seed and state, one channel apart: outcome-value 0 vs 1 must
    // drive different trajectories through B, while W0 never moves and
    // apply_feedback itself moves nothing.
    let cfg = no_learning_config();
    let k = cfg.environment.cue_count;
    let dim = feature_dim(k);
    let mut reward_one = vec![0.0; dim];
    reward_one[k + 2] = 1.0;
    reward_one[k + 3] = 1.0;
    let mut reward_zero = vec![0.0; dim];
    reward_zero[k + 2] = 1.0;
    reward_zero[k + 3] = 0.0;

    let mut a = birth(&cfg, 1, 0);
    let mut b = birth(&cfg, 1, 0);
    // Warm both identically so the reward comparison starts off-birth.
    for _ in 0..5 {
        let input = vec![0.0; dim];
        a.advance(&input).expect("step");
        b.advance(&input).expect("step");
    }
    let h_before = a.actor_state().h().to_vec();
    let w0_before = a.inherited().weights.w0.clone();
    a.apply_feedback(Feedback {
        event_id: 0,
        reward: 1.0,
    })
    .expect("consume");
    assert_eq!(
        a.actor_state().h(),
        h_before.as_slice(),
        "deduplication path must not step the network"
    );
    assert_eq!(a.inherited().weights.w0, w0_before);
    a.advance(&reward_one).expect("reward 1");
    b.apply_feedback(Feedback {
        event_id: 0,
        reward: 0.0,
    })
    .expect("consume");
    b.advance(&reward_zero).expect("reward 0");
    assert_ne!(
        a.actor_state().h(),
        b.actor_state().h(),
        "public reward must enter through sensory features"
    );
    assert_eq!(a.inherited().weights.w0, b.inherited().weights.w0);
}

#[test]
fn duplicate_and_invalid_feedback_fail_without_state_change() {
    let cfg = no_learning_config();
    let mut actor = birth(&cfg, 1, 0);
    let dim = feature_dim(cfg.environment.cue_count);
    actor.advance(&vec![0.0; dim]).expect("first transition");
    let h = actor.actor_state().h().to_vec();
    let a_state = actor.actor_state().a().to_vec();
    let q = actor.motor_state().q();
    let w0 = actor.inherited().weights.w0.clone();

    actor
        .apply_feedback(Feedback {
            event_id: 0,
            reward: 1.0,
        })
        .expect("first delivery");
    assert_eq!(actor.last_feedback(), Some(0));
    // Duplicate (equal id) and backwards ids both fail, state untouched.
    for event in [
        Feedback {
            event_id: 0,
            reward: 0.0,
        },
        Feedback {
            event_id: 0,
            reward: 1.0,
        },
    ] {
        assert!(matches!(
            actor.apply_feedback(event),
            Err(SimError::DuplicateFeedback(0))
        ));
    }
    // Non-binary and nonfinite rewards fail before consumption.
    for reward in [2.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            actor
                .apply_feedback(Feedback {
                    event_id: 1,
                    reward
                })
                .is_err(),
            "reward {reward} must be rejected"
        );
    }
    assert_eq!(actor.last_feedback(), Some(0));
    assert_eq!(actor.actor_state().h(), h.as_slice());
    assert_eq!(actor.actor_state().a(), a_state.as_slice());
    assert_eq!(actor.motor_state().q(), q);
    assert_eq!(actor.inherited().weights.w0, w0);
    // The next valid event still succeeds.
    actor
        .apply_feedback(Feedback {
            event_id: 1,
            reward: 0.0,
        })
        .expect("next event");
    assert_eq!(actor.last_feedback(), Some(1));
}

#[test]
fn feature_width_mismatch_fails_without_advancing() {
    let cfg = no_learning_config();
    let mut actor = birth(&cfg, 1, 0);
    let dim = feature_dim(cfg.environment.cue_count);
    actor.advance(&vec![0.0; dim]).expect("valid width steps");
    assert_eq!(actor.ticks_advanced(), 1);
    let h = actor.actor_state().h().to_vec();
    assert!(matches!(
        actor.advance(&vec![0.0; dim + 1]),
        Err(SimError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        actor.advance(&vec![0.0; dim.saturating_sub(1)]),
        Err(SimError::InvalidConfiguration(_))
    ));
    assert_eq!(actor.ticks_advanced(), 1, "rejected input must not tick");
    assert_eq!(actor.actor_state().h(), h.as_slice());
}

#[test]
fn observation_reads_draw_nothing_and_runs_reproduce() {
    // Interleaved diagnostic reads (the logging-invariance proof) leave the
    // trajectory bitwise identical; two identical seeds agree choice by
    // choice.
    let cfg = no_learning_config();
    let run = |probe: bool| {
        let mut actor = birth(&cfg, 1, 0);
        let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
        let mut actions = Vec::new();
        let mut membranes = Vec::new();
        while !lifetime.is_complete() {
            let out = lifetime.advance().expect("advance");
            if let Some(feedback) = out.observation.feedback {
                actor.apply_feedback(feedback).expect("consume");
                lifetime
                    .note_feedback_consumed(feedback.event_id)
                    .expect("ledger");
            }
            let motor = actor.advance(&out.observation.features).expect("step");
            if probe {
                let _ = (
                    actor.actor_state().h(),
                    actor.actor_state().a(),
                    actor.actor_state().r(),
                    actor.actor_state().last_perturbations(),
                    actor.motor_state().q(),
                    actor.last_output(),
                );
            }
            membranes.push(actor.actor_state().h().to_vec());
            if out.commitment_due {
                let action = actor.select_action();
                actions.push(action);
                lifetime.commit(action).expect("commit");
            } else {
                let _ = motor;
            }
        }
        (actions, membranes)
    };
    let (plain_actions, plain_h) = run(false);
    let (probed_actions, probed_h) = run(true);
    assert_eq!(plain_actions, probed_actions);
    assert_eq!(plain_h, probed_h);

    let mut first = birth(&cfg, 1, 0);
    let first_summary = run_actor_ordinary(
        &cfg,
        1,
        "development",
        1,
        0,
        "actor-no-learning",
        &mut first,
    )
    .expect("run");
    let mut second = birth(&cfg, 1, 0);
    let second_summary = run_actor_ordinary(
        &cfg,
        1,
        "development",
        1,
        0,
        "actor-no-learning",
        &mut second,
    )
    .expect("rerun");
    assert_eq!(
        first_summary.choices, second_summary.choices,
        "same seeds reproduce the same choices"
    );
}

#[test]
fn execution_guards_separate_present_and_future_modes() {
    use cra::config::{validate_actor_no_learning_execution, validate_baseline_execution};
    let cfg = no_learning_config();
    validate_actor_no_learning_execution(&cfg).expect("no-learning profile runs");
    // The same profile must not run as an env-only baseline.
    assert!(validate_baseline_execution(&cfg).is_err());

    // Env-only still runs as a baseline but not as an actor.
    let env_only: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("parses");
    validate_baseline_execution(&env_only).expect("env-only baseline runs");
    assert!(validate_actor_no_learning_execution(&env_only).is_err());

    // Future modes stay rejected: enabled plasticity (M3), non-fixed gates
    // (M6), enabled search (M7), diagnostic resets and reserved env kinds.
    let mut plastic = cfg.clone();
    plastic.learning = Some(cra::config::Learning {
        enabled: true,
        rule: "gaussian_transition_score".to_owned(),
        plastic_mask: "all_recurrent_edges".to_owned(),
        trace_policy: "persistent".to_owned(),
        tau_e: 32.0,
        eta: 0.001,
        max_update: 0.01,
        plastic_bound: 0.5,
        plastic_decay: 0.0,
        reward_baseline_initial: 0.5,
        reward_baseline_beta: 0.02,
    });
    assert!(validate_actor_no_learning_execution(&plastic).is_err());

    // Explicitly disabled learning is still a no-learning profile.
    let mut disabled = cfg.clone();
    disabled.learning = Some(cra::config::Learning {
        enabled: false,
        ..plastic.learning.clone().unwrap()
    });
    validate_actor_no_learning_execution(&disabled).expect("disabled learning runs");

    let mut gated = cfg.clone();
    gated.modulator = Some(cra::config::Modulator {
        mode: "targeted".to_owned(),
        neuron_count: 2,
        tau_m: 20.0,
        ordinary_feedback_to_actor: false,
        gate_timing: "pre_outcome".to_owned(),
        gate_bias_initial: 0.0,
        projection_init_std: 0.01,
    });
    assert!(validate_actor_no_learning_execution(&gated).is_err());

    let mut searching = cfg.clone();
    searching.evolution = Some(cra::config::Evolution {
        enabled: true,
        population_size: 8,
        elite_count: 2,
        mutation_std: 0.1,
        search_space: "gate_projection_only".to_owned(),
        decoded_weight_limit: None,
        lifetimes_per_candidate: None,
        generations: None,
        validation_every_generations: None,
        validation_lifetimes: None,
    });
    assert!(validate_actor_no_learning_execution(&searching).is_err());

    for field in ["reset_policy", "kind"] {
        let mut bad = cfg.clone();
        if field == "reset_policy" {
            bad.simulation.reset_policy = "episodic_diagnostic".to_owned();
        } else {
            bad.environment.kind = "isolated_reversal".to_owned();
        }
        assert!(
            validate_actor_no_learning_execution(&bad).is_err(),
            "{field} must stay rejected"
        );
    }
}

#[test]
fn actor_rejects_bad_seeds_and_missing_sections() {
    use cra::agent::topology::TopologyError;
    use cra::agent::weights::ParamsError;
    let cfg = no_learning_config();
    // A non-reserved namespace fails inside inherited sampling (the init
    // stream validates its tuple); the error is explicit, never a silent
    // fallback seed.
    assert!(matches!(
        NoLearningActor::new(&cfg, 1, "staging", 1, 0),
        Err(NoLearningError::Params(ParamsError::Topology(
            TopologyError::BadSeed(_)
        )))
    ));
    let env_only: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("parses");
    assert!(matches!(
        NoLearningActor::new(&env_only, 1, "development", 1, 0),
        Err(NoLearningError::InvalidConfig(_))
    ));
}
