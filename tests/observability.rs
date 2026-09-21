//! Cue observability and bounded long-run smoke tests (M1-11).
//!
//! Usable-dynamics checks, not learning: fixed inputs stay finite,
//! alternating cues drive distinguishable activity, long quiet periods
//! and long runs remain finite, and both actions are reachable across
//! initializations. Every run is bounded and deterministic (development
//! namespace, fixed outer seeds); diagnostics are health summaries plus
//! JSON-serializable traces. A failure here is a dynamics problem to
//! debug (spec 6.5, 10.6), never evidence of learning.

use cra::agent::health::{HealthSummary, TraceRecorder};
use cra::agent::no_learning::NoLearningActor;
use cra::config::{Actor, Config};
use cra::environment::{Agent, Lifetime};

// Optional immutable evidence export for the bounded milestone suite.
// Assertions run identically with or without recording.
fn save_evidence(name: &str, cfg: &Config, results: serde_json::Value) {
    let Some(dir) = std::env::var_os("CRA_M1_EVIDENCE_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(format!("{name}.json")))
        .expect("evidence path must be fresh");
    serde_json::to_writer_pretty(
        file,
        &serde_json::json!({
            "config": cfg, "root_seed": 1, "namespace": "development", "lifetime_index": 0,
            "reference_os": std::env::consts::OS, "reference_arch": std::env::consts::ARCH,
            "results": results,
        }),
    )
    .unwrap();
}

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

fn observability_config() -> Config {
    let mut cfg: Config =
        toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").expect("smoke"))
            .expect("smoke parses");
    cfg.profile_name = "observability_test".to_owned();
    cfg.simulation.outcomes_per_lifetime = 4;
    cfg.actor = Some(actor_cfg());
    cfg.learning = None;
    cfg.modulator = None;
    cfg.evolution = None;
    cra::config::validate(&cfg).expect("test config validates");
    cfg
}

/// Fixed cue features: one-hot cue `cue` plus cue-present, everything
/// else zero (no go/outcome/latch).
fn cue_features(cue_count: usize, cue: usize) -> Vec<f64> {
    let mut features = vec![0.0; cue_count + 6];
    features[cue] = 1.0;
    features[cue_count] = 1.0;
    features
}

#[test]
fn fixed_inputs_stay_finite_over_long_quiet() {
    // 2000 ticks of zero input (a long quiet): finite membranes,
    // adaptation, readout, and watchdog throughout.
    let cfg = observability_config();
    let dim = cra::environment::feature_dim(cfg.environment.cue_count);
    let mut actor = NoLearningActor::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut summary = HealthSummary::new();
    for t in 0..2000 {
        actor.advance(&vec![0.0; dim]).expect("step");
        actor.health_check().expect("watchdog holds");
        summary
            .observe(
                t,
                actor.actor_state().h(),
                actor.actor_state().a(),
                actor.actor_state().r(),
                actor.motor_state().q(),
            )
            .expect("healthy");
    }
    assert_eq!(summary.ticks_observed, 2000);
    assert!(summary.max_abs_h < cra::agent::health::WATCHDOG_H_ABS_MAX);
    let fraction = summary.saturated_fraction().expect("observed");
    assert!(
        (0.0..=1.0).contains(&fraction),
        "saturation fraction in range"
    );
    // Diagnostics serialize for offline analysis (no notebook needed).
    let json = serde_json::to_string(&summary).expect("summary serializes");
    assert!(json.contains("ticks_observed"));
    save_evidence(
        "fixed-zero-input",
        &cfg,
        serde_json::json!({"outer_seed": 1, "summary": summary}),
    );
}

#[test]
fn alternating_cues_produce_distinguishable_activity() {
    // Alternating 50-tick blocks of cue 0 / cue 1 through the same actor:
    // block-mean activities must differ (the network is cue-driven, not
    // locked into one saturated state).
    let cfg = observability_config();
    let k = cfg.environment.cue_count;
    let mut actor = NoLearningActor::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut means = Vec::new();
    for block in 0..6 {
        let cue = block % 2;
        let mut acc = vec![0.0; 16];
        for _ in 0..50 {
            actor.advance(&cue_features(k, cue)).expect("step");
            for (j, r) in actor.actor_state().r().iter().enumerate() {
                acc[j] += r;
            }
        }
        for v in acc.iter_mut() {
            *v /= 50.0;
        }
        means.push(acc);
    }
    // Same-cue blocks agree more than cue-0 vs cue-1 blocks differ:
    // average within-cue distance must be smaller than across-cue distance.
    let dist = |a: &[f64], b: &[f64]| {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
    };
    let mut within = 0.0;
    let mut across = 0.0;
    let (mut n_within, mut n_across) = (0u32, 0u32);
    for (i, a) in means.iter().enumerate() {
        for (j, b) in means.iter().enumerate() {
            if i >= j {
                continue;
            }
            if i % 2 == j % 2 {
                within += dist(a, b);
                n_within += 1;
            } else {
                across += dist(a, b);
                n_across += 1;
            }
        }
    }
    assert!(n_within > 0 && n_across > 0);
    assert!(
        across / f64::from(n_across) > within / f64::from(n_within),
        "distinct cues must drive distinguishable activity"
    );
    save_evidence(
        "alternating-cues",
        &cfg,
        serde_json::json!({
            "outer_seed": 1, "block_ticks": 50, "cue_sequence": [0,1,0,1,0,1],
            "within_cue_distance": within / f64::from(n_within),
            "across_cue_distance": across / f64::from(n_across), "activity_block_means": means,
        }),
    );
}

#[test]
fn both_actions_reachable_across_initializations() {
    // Eight outer seeds (independent inherited initializations), four
    // outcomes each through the common runner: both actions must occur
    // somewhere, or the family is locked into one motor state (spec 6.5).
    let cfg = observability_config();
    let mut seen = [false, false];
    let mut summaries = Vec::new();
    let mut action_counts = [0_u64; 2];
    let mut diagnostics = Vec::new();
    for outer in 1..=8 {
        let mut lifetime = Lifetime::new(&cfg, 1, "development", outer, 0).expect("birth");
        let mut actor = NoLearningActor::new(&cfg, 1, "development", outer, 0).expect("birth");
        let mut summary = HealthSummary::new();
        let mut recorder = TraceRecorder::new(
            16,
            &actor.inherited().topology.motor0.clone(),
            &actor.inherited().topology.motor1.clone(),
            10,
        )
        .expect("recorder");
        let mut tick = 0u64;
        while !lifetime.is_complete() {
            let out = lifetime.advance().expect("advance");
            if let Some(feedback) = out.observation.feedback {
                actor.apply_feedback(feedback).expect("consume");
                lifetime
                    .note_feedback_consumed(feedback.event_id)
                    .expect("ledger");
            }
            actor.advance(&out.observation.features).expect("step");
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
            recorder
                .maybe_record(
                    tick,
                    actor.actor_state().h(),
                    actor.actor_state().a(),
                    actor.motor_state().q(),
                )
                .expect("sample");
            tick += 1;
            if out.commitment_due {
                let action = actor.select_action();
                seen[usize::from(action)] = true;
                action_counts[usize::from(action)] += 1;
                lifetime.commit(action).expect("commit");
            }
        }
        // Per-initialization diagnostics stay finite with motor margins.
        assert!(summary.max_abs_h < cra::agent::health::WATCHDOG_H_ABS_MAX);
        assert!(summary.min_margin().expect("margins").is_finite());
        assert!(!recorder.samples().is_empty());
        diagnostics
            .push(serde_json::json!({"outer_seed": outer, "summary": summary, "trace": recorder}));
        summaries.push(summary);
    }
    assert!(
        seen[0] && seen[1],
        "both actions must be reachable across initializations"
    );
    assert_eq!(summaries.len(), 8);
    save_evidence(
        "initializations",
        &cfg,
        serde_json::json!({"action_counts": action_counts, "lifetimes": diagnostics}),
    );
}

#[test]
fn long_quiet_lifetimes_remain_finite() {
    // One lifetime with 64-tick quiets (long uninterrupted silences):
    // completes with finite state and bounded health.
    let mut cfg = observability_config();
    cfg.environment.quiet_ticks = [64, 64];
    cra::config::validate(&cfg).expect("long-quiet validates");
    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut actor = NoLearningActor::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut summary = HealthSummary::new();
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
        summary
            .observe(
                ticks,
                actor.actor_state().h(),
                actor.actor_state().a(),
                actor.actor_state().r(),
                actor.motor_state().q(),
            )
            .expect("healthy");
        ticks += 1;
        if out.commitment_due {
            let action = actor.select_action();
            lifetime.commit(action).expect("commit");
        }
        assert!(ticks < 10_000, "bounded smoke must terminate");
    }
    assert_eq!(lifetime.outcomes(), 4);
    // 4 outcomes with 64-tick quiets: warmup 4 + cue 8 + response 4 +
    // feedback 1, then three full 77-tick cycles = 248 ticks total.
    assert_eq!(ticks, 248, "exact warmup + four cycles");
    save_evidence(
        "long-quiet",
        &cfg,
        serde_json::json!({"outer_seed": 1, "outcomes": lifetime.outcomes(), "summary": summary}),
    );
    assert!(summary.max_abs_h < cra::agent::health::WATCHDOG_H_ABS_MAX);
}
