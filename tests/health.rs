//! Numerical health and selected-trace contracts (M1-08).
//!
//! - A forced nonfinite value fails visibly through `check_state`,
//!   `HealthSummary::observe`, `TraceRecorder::maybe_record`, and the
//!   actor's read-only `health_check` — nothing is clipped and failed
//!   ticks record nothing.
//! - The finite watchdog trips above the conservative `1e4` bound with the
//!   bound named in the error, while boundary (`1e4`) and ordinary `O(1)`
//!   states pass.
//! - Saturation (`|r| > 0.9`) and motor-margin extrema match hand values;
//!   empty summaries report `None`, never silent zeros.
//! - Trace selection is a stable pure function covering both motor pools
//!   with no randomness; health observation leaves actor trajectories
//!   bitwise identical (logging invariance).
//! - A full no-learning lifetime stays finite with a saved JSON diagnostic
//!   round-trip (file-based output, no notebook).

use cra::agent::health::{
    HEALTH_SCHEMA_VERSION, HealthError, HealthSummary, SATURATION_R_ABS, TRACE_SELECTION_BUDGET,
    TraceRecorder, WATCHDOG_A_ABS_MAX, WATCHDOG_H_ABS_MAX, WATCHDOG_Q_ABS_MAX, check_state,
    selected_trace_indices,
};
use cra::agent::no_learning::NoLearningActor;
use cra::config::{Actor, Config};
use cra::environment::{Agent, Lifetime, SimError};

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
    cfg.profile_name = "actor_health_test".to_owned();
    cfg.simulation.outcomes_per_lifetime = 6;
    cfg.actor = Some(actor_cfg());
    cfg.learning = None;
    cfg.modulator = None;
    cfg.evolution = None;
    cra::config::validate(&cfg).expect("test config validates");
    cfg
}

fn birth(cfg: &Config, outer: u64, lifetime: u64) -> NoLearningActor {
    NoLearningActor::new(cfg, 1, "development", outer, lifetime).expect("birth")
}

fn finite_state(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, [f64; 2]) {
    (vec![0.4; n], vec![-0.2; n], vec![0.3; n], [0.25, -0.1])
}

#[test]
fn forced_nonfinite_fails_visibly_and_records_nothing() {
    let (h, a, r, q) = finite_state(4);
    check_state(7, &h, &a, &r, q).expect("finite passes");
    // Each component names itself; every failure leaves the summary
    // untouched (ticks stay 0, extrema stay 0).
    for (label, bad_h, bad_a, bad_r, bad_q) in [
        ("h", vec![f64::NAN; 4], a.clone(), r.clone(), q),
        ("a", h.clone(), vec![f64::INFINITY; 4], r.clone(), q),
        ("r", h.clone(), a.clone(), vec![f64::NAN; 4], q),
        ("q", h.clone(), a.clone(), r.clone(), [0.0, f64::NAN]),
    ] {
        let err = check_state(7, &bad_h, &bad_a, &bad_r, bad_q).expect_err("must fail");
        assert!(
            matches!(err, HealthError::NonFinite { tick: 7, .. }),
            "{label}: {err:?}"
        );
        let mut summary = HealthSummary::new();
        let before = summary.clone();
        assert!(summary.observe(7, &bad_h, &bad_a, &bad_r, bad_q).is_err());
        assert_eq!(summary, before, "{label}: failed ticks record nothing");
    }
    // The actor's read-only check draws nothing and mutates nothing: a
    // healthy birth passes, and repeated checks agree.
    let cfg = no_learning_config();
    let actor = birth(&cfg, 1, 0);
    actor.health_check().expect("birth is finite");
    actor.health_check().expect("checks are repeatable");
}

#[test]
fn finite_watchdog_trips_above_the_conservative_bound() {
    assert_eq!(WATCHDOG_H_ABS_MAX, 1e4);
    assert_eq!(WATCHDOG_A_ABS_MAX, 1e4);
    assert_eq!(WATCHDOG_Q_ABS_MAX, 1e4);
    let (mut h, mut a, r, mut q) = finite_state(3);
    // Boundary magnitudes pass; anything above trips with the bound named.
    h[0] = 1e4;
    check_state(0, &h, &a, &r, q).expect("boundary passes");
    h[0] = 10_001.0;
    match check_state(3, &h, &a, &r, q).expect_err("must trip") {
        HealthError::WatchdogTripped {
            tick: 3,
            component,
            value,
            bound,
        } => {
            assert_eq!(component, "h");
            assert_eq!(value, 10_001.0);
            assert_eq!(bound, 1e4);
        }
        other => panic!("wrong error: {other:?}"),
    }
    a[1] = -20_000.0;
    h[0] = 0.4;
    assert!(matches!(
        check_state(3, &h, &a, &r, q),
        Err(HealthError::WatchdogTripped { .. })
    ));
    q = [0.0, 50_000.0];
    a[1] = -0.2;
    assert!(matches!(
        check_state(3, &h, &a, &r, q),
        Err(HealthError::WatchdogTripped { .. })
    ));
    // Ordinary O(1) dynamics never approach the bound (1e4 sits ~1000x
    // above healthy magnitudes by the module-docs rationale).
    let (h, a, r, q) = finite_state(16);
    check_state(0, &h, &a, &r, q).expect("healthy O(1) passes");
}

#[test]
fn saturation_and_margin_math_matches_hand_values() {
    assert_eq!(SATURATION_R_ABS, 0.9);
    let empty = HealthSummary::new();
    assert_eq!(empty.saturated_fraction(), None);
    assert_eq!(empty.min_margin(), None);

    let mut summary = HealthSummary::new();
    let h = vec![0.0; 2];
    let a = vec![0.0; 2];
    // Tick 0: one of two activities saturated (|0.95| > 0.9), margin 0.2.
    summary
        .observe(0, &h, &a, &[0.95, -0.2], [0.3, 0.1])
        .expect("tick 0");
    // Tick 1: the other activity saturated, margin 1.0.
    summary
        .observe(1, &h, &a, &[0.0, -0.95], [-0.5, 0.5])
        .expect("tick 1");
    assert_eq!(summary.ticks_observed, 2);
    assert_eq!(summary.saturated_neuron_ticks, 2);
    assert_eq!(summary.total_neuron_ticks, 4);
    assert_eq!(summary.saturated_fraction(), Some(0.5));
    let min_margin = summary.min_margin().expect("margin");
    assert!((min_margin - 0.2).abs() < 1e-12, "min margin {min_margin}");
    assert!((summary.max_motor_margin - 1.0).abs() < 1e-12);
    assert!((summary.max_abs_r - 0.95).abs() < 1e-12);
    assert!((summary.max_abs_q - 0.5).abs() < 1e-12);
}

#[test]
fn trace_selection_is_stable_covers_motor_and_draws_nothing() {
    // Debug profile assignment: N = 16, m = 2 -> M0 = [12, 13], M1 = [14, 15].
    let motor0 = vec![12, 13];
    let motor1 = vec![14, 15];
    let first = selected_trace_indices(16, &motor0, &motor1);
    assert_eq!(first, vec![0, 1, 12, 14]);
    assert_eq!(first, selected_trace_indices(16, &motor0, &motor1));
    assert!(first.len() <= TRACE_SELECTION_BUDGET);
    assert!(first.contains(&12) && first.contains(&14));
    // Small and degenerate topologies stay covered without RNG.
    assert_eq!(selected_trace_indices(4, &[2], &[3]), vec![0, 1, 2, 3]);
    assert_eq!(selected_trace_indices(2, &[0], &[1]), vec![0, 1]);

    // Interleaved selection calls leave the actor trajectory identical:
    // selection is a pure function, not a stream draw.
    let cfg = no_learning_config();
    let dim = cra::environment::feature_dim(cfg.environment.cue_count);
    let run = |probe: bool| {
        let mut actor = birth(&cfg, 1, 0);
        let mut membranes = Vec::new();
        for t in 0..6 {
            if probe {
                let _ = selected_trace_indices(
                    16,
                    &actor.inherited().topology.motor0.clone(),
                    &actor.inherited().topology.motor1.clone(),
                );
            }
            actor.advance(&vec![0.0; dim]).expect("step");
            membranes.push(actor.actor_state().h().to_vec());
            let _ = t;
        }
        membranes
    };
    assert_eq!(run(false), run(true));
}

#[test]
fn health_observation_leaves_actor_trajectories_identical() {
    let cfg = no_learning_config();
    let dim = cra::environment::feature_dim(cfg.environment.cue_count);
    let run = |observe: bool| {
        let mut actor = birth(&cfg, 1, 0);
        let mut summary = HealthSummary::new();
        let mut recorder = TraceRecorder::new(
            16,
            &actor.inherited().topology.motor0.clone(),
            &actor.inherited().topology.motor1.clone(),
            2,
        )
        .expect("recorder");
        let mut membranes = Vec::new();
        let mut actions = Vec::new();
        for t in 0..8 {
            actor.advance(&vec![0.0; dim]).expect("step");
            if observe {
                summary
                    .observe(
                        t,
                        actor.actor_state().h(),
                        actor.actor_state().a(),
                        actor.actor_state().r(),
                        actor.motor_state().q(),
                    )
                    .expect("healthy");
                recorder
                    .maybe_record(
                        t,
                        actor.actor_state().h(),
                        actor.actor_state().a(),
                        actor.motor_state().q(),
                    )
                    .expect("record");
                actor.health_check().expect("healthy");
            }
            membranes.push(actor.actor_state().h().to_vec());
            actions.push(actor.select_action());
        }
        (membranes, actions)
    };
    let (plain_h, plain_a) = run(false);
    let (observed_h, observed_a) = run(true);
    assert_eq!(plain_h, observed_h);
    assert_eq!(plain_a, observed_a);
}

#[test]
fn full_lifetime_health_stays_finite_with_samples() {
    let cfg = no_learning_config();
    let mut actor = birth(&cfg, 1, 0);
    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut summary = HealthSummary::new();
    let mut recorder = TraceRecorder::new(
        16,
        &actor.inherited().topology.motor0.clone(),
        &actor.inherited().topology.motor1.clone(),
        10,
    )
    .expect("recorder");
    let mut ticks = 0u64;
    while !lifetime.is_complete() {
        let out = lifetime.advance().expect("advance");
        if let Some(feedback) = out.observation.feedback {
            actor.apply_feedback(feedback).expect("consume");
            lifetime
                .note_feedback_consumed(feedback.event_id)
                .expect("ledger");
        }
        actor.advance(&out.observation.features).expect("step");
        actor.health_check().expect("watchdog holds");
        summary
            .observe(
                ticks,
                actor.actor_state().h(),
                actor.actor_state().a(),
                actor.actor_state().r(),
                actor.motor_state().q(),
            )
            .expect("finite");
        recorder
            .maybe_record(
                ticks,
                actor.actor_state().h(),
                actor.actor_state().a(),
                actor.motor_state().q(),
            )
            .expect("sample");
        ticks += 1;
        if out.commitment_due {
            let action = actor.select_action();
            lifetime.commit(action).expect("commit");
        }
    }
    assert_eq!(summary.ticks_observed, ticks);
    assert!(ticks > 6);
    assert!(summary.max_abs_h < WATCHDOG_H_ABS_MAX);
    assert!(summary.max_abs_a < WATCHDOG_A_ABS_MAX);
    assert!(summary.max_abs_q < WATCHDOG_Q_ABS_MAX);
    let fraction = summary.saturated_fraction().expect("observed");
    assert!((0.0..=1.0).contains(&fraction));
    let margin = summary.min_margin().expect("observed");
    assert!(margin.is_finite() && margin >= 0.0);
    // Sampling every 10 ticks over a >60-tick lifetime yields several
    // ordered samples with the stable selection width.
    assert!(recorder.samples().len() >= 3);
    let mut last = None;
    for sample in recorder.samples() {
        assert_eq!(sample.tick % 10, 0);
        assert_eq!(sample.h.len(), recorder.selection().len());
        assert_eq!(sample.a.len(), recorder.selection().len());
        if let Some(prev) = last {
            assert!(sample.tick > prev, "samples stay in tick order");
        }
        last = Some(sample.tick);
    }
}

#[test]
fn diagnostic_output_round_trips_through_a_json_file() {
    let mut summary = HealthSummary::new();
    summary
        .observe(0, &[0.5, -0.3], &[0.1, 0.0], &[0.95, 0.0], [0.2, 0.1])
        .expect("tick");
    let mut recorder = TraceRecorder::new(2, &[0], &[1], 1).expect("recorder");
    recorder
        .maybe_record(0, &[0.5, -0.3], &[0.1, 0.0], [0.2, 0.1])
        .expect("sample");
    let payload = serde_json::json!({
        "schema_version": HEALTH_SCHEMA_VERSION,
        "summary": summary,
        "selection": recorder.selection(),
        "samples": recorder.samples(),
    });
    let path = std::env::temp_dir().join(format!("cra-health-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_string_pretty(&payload).expect("json"))
        .expect("write diagnostic");
    let back: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    assert_eq!(back["schema_version"], 2);
    let summary_back: HealthSummary =
        serde_json::from_value(back["summary"].clone()).expect("summary");
    assert_eq!(summary_back, summary);
    let samples_back: Vec<cra::agent::health::TraceSample> =
        serde_json::from_value(back["samples"].clone()).expect("samples");
    assert_eq!(samples_back, recorder.samples());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn recorder_rejects_bad_configs_and_short_state() {
    assert!(matches!(
        TraceRecorder::new(4, &[2], &[3], 0),
        Err(HealthError::InvalidConfig(_))
    ));
    assert!(matches!(
        TraceRecorder::new(0, &[], &[], 1),
        Err(HealthError::InvalidConfig(_))
    ));
    let mut recorder = TraceRecorder::new(4, &[2], &[3], 1).expect("recorder");
    assert!(
        recorder
            .maybe_record(0, &[0.0; 4], &[0.0; 4], [0.0, 0.0])
            .expect("ok")
    );
    assert_eq!(recorder.samples().len(), 1);
    // Short state vectors name the offending index instead of panicking.
    assert!(matches!(
        recorder.maybe_record(1, &[0.0; 2], &[0.0; 2], [0.0, 0.0]),
        Err(HealthError::InvalidConfig(_))
    ));
    assert_eq!(recorder.samples().len(), 1, "failed ticks append nothing");
    // Non-sampled ticks succeed trivially without validating or appending.
    let mut sparse = TraceRecorder::new(4, &[2], &[3], 10).expect("sparse");
    assert!(
        !sparse
            .maybe_record(1, &[f64::NAN; 4], &[0.0; 4], [0.0, 0.0])
            .expect("non-sampled")
    );
    assert!(sparse.samples().is_empty());
}

#[test]
fn health_errors_map_onto_sim_errors_explicitly() {
    let nonfinite = HealthError::NonFinite {
        tick: 9,
        component: "h".to_owned(),
    };
    assert!(matches!(
        nonfinite.to_sim_error(),
        SimError::NonFiniteState { tick: 9, .. }
    ));
    let tripped = HealthError::WatchdogTripped {
        tick: 9,
        component: "q".to_owned(),
        value: 20_000.0,
        bound: 1e4,
    };
    match tripped.to_sim_error() {
        SimError::InvalidConfiguration(message) => {
            assert!(message.contains("watchdog") && message.contains("10000"));
        }
        other => panic!("wrong mapping: {other:?}"),
    }
    assert!(matches!(
        HealthError::InvalidConfig("x".to_owned()).to_sim_error(),
        SimError::InvalidConfiguration(_)
    ));
}

#[test]
fn production_actor_enforces_finite_watchdog() {
    let mut cfg = no_learning_config();
    cfg.actor.as_mut().unwrap().input_scale = 1e6;
    let mut actor = birth(&cfg, 1, 0);
    let mut features = vec![0.0; cfg.environment.cue_count + 6];
    features[0] = 1.0;
    let error = actor
        .advance(&features)
        .expect_err("finite blow-up must stop production");
    assert!(error.to_string().contains("watchdog"));
}

#[test]
fn empty_summary_round_trips_for_failure_before_first_tick() {
    let empty = HealthSummary::new();
    let json = serde_json::to_string(&empty).unwrap();
    let restored: HealthSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, empty);
    assert_eq!(restored.min_margin(), None);
}
