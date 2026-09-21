//! Three honestly distinguished continuity conditions (M4-04; spec
//! 16/M4, 7.7, 17.2).
//!
//! - The episodic diagnostic (`episodic_diagnostic` resets +
//!   `no_decay_diagnostic` traces), the fully persistent main condition
//!   (`birth_only`, never reset), and the event-reset diagnostic
//!   (`event_reset_diagnostic` resets + `persistent` traces) each accept
//!   only their own guard: every runner rejects the other two
//!   conditions' configs, so no diagnostic run can masquerade as the
//!   main model and no trace-clearing run can pass as persistent.
//! - On identical seeds the continuous and event-reset runners share
//!   inheritance and the full exogenous cue schedule but diverge in
//!   acquired offsets/traces; summaries carry distinct mode,
//!   reset-policy, trace-policy, and profile labels.
//! - The checked-in `continuous_stationary` profile (twin of the
//!   `debug_stationary` source) validates and executes through the
//!   library continuous runner.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::config::{Actor, Learning};
use cra::experiments::continuous::{
    CONTINUOUS_MODE, EVENT_RESET_MODE, run_continuous_lifetime, run_event_reset_lifetime,
};
use cra::experiments::episodic::{EPISODIC_MODE, run_episodic_lifetime};
use support::base_config;

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
        tau_e: 64.0,
        eta: 0.001,
        max_update: 0.01,
        plastic_bound: 0.5,
        plastic_decay: 0.0,
        reward_baseline_initial: 0.5,
        reward_baseline_beta: 0.02,
    }
}

fn persistent_config(outcomes: u64, reset_policy: &str, profile: &str) -> cra::config::Config {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cfg.simulation.reset_policy = reset_policy.to_owned();
    cfg.profile_name = profile.to_owned();
    cfg.actor = Some(actor_cfg());
    cfg.learning = Some(learning_cfg());
    cra::config::validate(&cfg).expect("config validates");
    cfg
}

fn continuous_config(outcomes: u64) -> cra::config::Config {
    persistent_config(outcomes, "birth_only", "continuous-test")
}

fn event_reset_config(outcomes: u64) -> cra::config::Config {
    persistent_config(outcomes, "event_reset_diagnostic", "event-reset-test")
}

#[test]
fn three_guards_are_pairwise_disjoint() {
    let episodic = {
        let text = std::fs::read_to_string("configs/episodic_stationary.toml").expect("profile");
        toml::from_str::<cra::config::Config>(&text).expect("parses")
    };
    let continuous = continuous_config(4);
    let event_reset = event_reset_config(4);
    use cra::config::{
        validate_continuous_execution, validate_episodic_execution, validate_event_reset_execution,
    };
    // Each guard accepts exactly its own condition.
    validate_episodic_execution(&episodic).expect("episodic accepts episodic");
    validate_continuous_execution(&continuous).expect("continuous accepts continuous");
    validate_event_reset_execution(&event_reset).expect("event-reset accepts event-reset");
    // Each guard rejects the other two conditions.
    assert!(validate_episodic_execution(&continuous).is_err());
    assert!(validate_episodic_execution(&event_reset).is_err());
    assert!(validate_continuous_execution(&episodic).is_err());
    assert!(validate_continuous_execution(&event_reset).is_err());
    assert!(validate_event_reset_execution(&episodic).is_err());
    assert!(validate_event_reset_execution(&continuous).is_err());
}

#[test]
fn persistent_label_with_trace_clearing_rejected_from_continuous_runner() {
    // The verify clause by name: a run labeled `persistent` that clears
    // traces (the event-reset config) is rejected from the continuous
    // runner at both guard and execution entry points.
    let cfg = event_reset_config(4);
    assert_eq!(
        cfg.learning.as_ref().expect("learning").trace_policy,
        "persistent"
    );
    assert!(cra::config::validate_continuous_execution(&cfg).is_err());
    assert!(run_continuous_lifetime(&cfg, 1, "development", 1, 0, "continuous").is_err());
    // And the continuous config is rejected from the event-reset runner.
    let cfg = continuous_config(4);
    assert!(cra::config::validate_event_reset_execution(&cfg).is_err());
    assert!(run_event_reset_lifetime(&cfg, 1, "development", 1, 0, "event-reset").is_err());
}

#[test]
fn shared_schedule_divergent_trajectories_across_trace_treatments() {
    // Same seeds, same inheritance, same exogenous schedule; the only
    // difference is the declared trace treatment. Labels, audits, and
    // acquired state must all reflect it.
    let continuous_cfg = continuous_config(6);
    let event_reset_cfg = event_reset_config(6);
    let main = run_continuous_lifetime(&continuous_cfg, 1, "development", 1, 0, "continuous")
        .expect("main run");
    let diag = run_event_reset_lifetime(&event_reset_cfg, 1, "development", 1, 0, "event-reset")
        .expect("diagnostic run");
    // Shared by construction.
    assert_eq!(main.w0, diag.w0, "shared inheritance");
    assert_eq!(main.initialization, diag.initialization);
    let main_cues: Vec<usize> = main.annotations.iter().map(|a| a.cue_id).collect();
    let diag_cues: Vec<usize> = diag.annotations.iter().map(|a| a.cue_id).collect();
    assert_eq!(main_cues, diag_cues, "shared exogenous schedule");
    assert_eq!((main.commitments, main.outcomes), (6, 6));
    assert_eq!((diag.commitments, diag.outcomes), (6, 6));
    // Honestly different trace treatment.
    assert_eq!(main.mode, CONTINUOUS_MODE);
    assert_eq!(diag.mode, EVENT_RESET_MODE);
    assert_eq!(main.reset_policy, "birth_only");
    assert_eq!(diag.reset_policy, "event_reset_diagnostic");
    assert_eq!(main.resets, vec![0]);
    assert_eq!(
        diag.resets.len(),
        6,
        "birth plus one clear per non-final outcome"
    );
    assert_eq!(diag.resets[0], 0);
    for (k, &tick) in diag.resets[1..].iter().enumerate() {
        assert_eq!(
            tick,
            diag.choices[k].feedback_tick + 1,
            "reset tick follows its feedback tick"
        );
    }
    assert_ne!(main.final_p, diag.final_p, "E clears change offsets");
    assert_ne!(main.final_e, diag.final_e, "E clears change traces");
}

#[test]
fn summaries_identify_all_three_policies() {
    let main =
        run_continuous_lifetime(&continuous_config(4), 1, "development", 1, 0, "c").expect("main");
    let diag = run_event_reset_lifetime(&event_reset_config(4), 1, "development", 1, 0, "e")
        .expect("diag");
    let mut episodic_cfg = {
        let text = std::fs::read_to_string("configs/episodic_stationary.toml").expect("profile");
        toml::from_str::<cra::config::Config>(&text).expect("parses")
    };
    episodic_cfg.simulation.outcomes_per_lifetime = 4;
    let episodic =
        run_episodic_lifetime(&episodic_cfg, 1, "development", 1, 0, "b4").expect("episodic");
    assert_eq!(
        (
            main.mode,
            main.reset_policy.as_str(),
            main.trace_policy.as_str()
        ),
        (CONTINUOUS_MODE, "birth_only", "persistent")
    );
    assert_eq!(
        (
            diag.mode,
            diag.reset_policy.as_str(),
            diag.trace_policy.as_str()
        ),
        (EVENT_RESET_MODE, "event_reset_diagnostic", "persistent")
    );
    assert_eq!(
        (
            episodic.mode,
            episodic.reset_policy.as_str(),
            episodic.trace_policy.as_str()
        ),
        (EPISODIC_MODE, "episodic_diagnostic", "no_decay_diagnostic")
    );
    assert_ne!(main.profile_name, diag.profile_name);
}

#[test]
fn event_reset_clears_only_traces_at_plastic_level() {
    // Unit complement to the runner audit: the entry point zeroes E
    // while P, baseline, and dedup survive; the wrong policy is refused.
    use cra::agent::plasticity::PlasticState;
    use cra::agent::topology::topology_from_mask;
    use cra::agent::weights::{InheritedParams, Weights};
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
                w0[j][i] = 0.3;
            }
        }
    }
    let inherited = InheritedParams {
        topology,
        weights: Weights {
            w0: w0.clone(),
            input_weights: vec![vec![0.05; cra::environment::feature_dim(2)]; 4],
            bias: vec![0.0; 4],
        },
    };
    let mut learning = learning_cfg();
    let mut state =
        PlasticState::from_learning_config(&inherited.topology, &w0, &learning).expect("birth");
    state
        .advance_eligibility(&[0.5, -0.5, 0.5, -0.5], &[1.0, -1.0, 0.5, 0.5], 0.5, 0.1)
        .expect("accrue");
    assert!(state.e().iter().flatten().any(|&v| v != 0.0));
    let before_p = state.p().to_vec();
    state
        .apply_feedback_once(
            0,
            1.0,
            &[1.0; 4],
            cra::agent::plasticity::FeedbackUpdateParams::from_learning_config(&learning),
            &w0,
        )
        .expect("apply");
    assert!(state.p().iter().flatten().any(|&v| v != 0.0));
    let p_after_update = state.p().to_vec();
    let baseline_after_update = state.reward_baseline();
    state.reset_traces_event_diagnostic().expect("clear");
    assert!(state.e().iter().flatten().all(|&v| v == 0.0));
    assert_eq!(state.p(), &p_after_update, "P survives the clear");
    assert_eq!(state.reward_baseline(), baseline_after_update);
    assert_eq!(state.last_feedback(), Some(0));
    let _ = before_p;
    // Wrong policy refused.
    learning.trace_policy = "no_decay_diagnostic".to_owned();
    let mut diag =
        PlasticState::from_learning_config(&inherited.topology, &w0, &learning).expect("birth");
    assert!(diag.reset_traces_event_diagnostic().is_err());
}

#[test]
fn checked_in_profiles_validate_and_execute() {
    for name in [
        "configs/continuous_stationary.toml",
        "configs/debug_stationary.toml",
    ] {
        let text = std::fs::read_to_string(name).expect("profile");
        let cfg = toml::from_str::<cra::config::Config>(&text).expect("parses");
        cra::config::validate(&cfg).expect("validates");
        cra::config::validate_continuous_execution(&cfg).expect("continuous executes");
    }
    // Executability through the library runner at a small override (the
    // checked-in 2,000-outcome task is study-shaped, not a unit scale).
    let text = std::fs::read_to_string("configs/continuous_stationary.toml").expect("profile");
    let mut cfg = toml::from_str::<cra::config::Config>(&text).expect("parses");
    cfg.simulation.outcomes_per_lifetime = 4;
    let summary = run_continuous_lifetime(&cfg, 1, "development", 1, 0, "continuous")
        .expect("profile executes");
    assert_eq!(summary.mode, CONTINUOUS_MODE);
    assert_eq!(summary.reset_policy, "birth_only");
    assert_eq!(summary.resets, vec![0]);
    assert_eq!((summary.commitments, summary.outcomes), (4, 4));
    assert_eq!(summary.profile_name, "continuous_stationary");
}
