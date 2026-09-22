//! M4-08 continuity-failure audit (spec 16/M4 "If it fails", Section 21).
//!
//! The frozen M4-07 comparison executed correctly but its scientific
//! criterion failed (0/3 seeds): the same inherited actor that acquires
//! episodically (outer 2 late 0.97) stays at 0.00 under both event-reset
//! and fully persistent continuity. This suite audits that failure WITHOUT
//! changing the frozen manifest, seeds, windows, criterion, tick order,
//! reset policy, or learning rule. No production file is modified; every
//! diagnostic drives the public runners or the public learner API.
//!
//! Reduction path (spec 21.1, smallest system first):
//!
//! ```text
//! hand-set two-step trace leak (interference mechanism, exact)
//!   -> synthetic two-cue drive through the outer-2 family learner
//!      (representation/carryover probe, Section 21.2)
//!   -> constant-input synthetic-reward loop with no resets
//!      (persistent closed-loop probe)
//!   -> full-lifetime audit series on paired schedules
//!      (baseline drift, update alignment, trace scale, timescale,
//!       per-cue behavior, saturation)
//! ```
//!
//! Fast tests pin the declaration, the mechanism algebra, the analyzers,
//! and paired execution on a small fixture. The ignored release test
//! exports the bounded 8-lifetime audit (outer-2 trio + matched B3,
//! outer-3 contrast, outer-1 control, outer-2 tau_e 16/64 legs) to a fresh
//! directory; it asserts execution integrity only and records measurements
//! for `docs/evidence/m4-08/`. It sets no acquisition pass/fail criterion.

use cra::agent::health::HealthSummary;
use cra::agent::no_learning::NoLearningActor;
use cra::agent::plasticity::{FeedbackUpdateParams, PlasticState};
use cra::agent::topology::topology_from_mask;
use cra::config::{
    Config, load_and_validate, resolved_toml, validate_actor_no_learning_execution,
    validate_continuous_execution, validate_episodic_execution, validate_event_reset_execution,
};
use cra::environment::Feedback;
use cra::experiments::baseline::{BaselineSummary, run_actor_ordinary};
use cra::experiments::continuous::{
    CONTINUOUS_MODE, ContinuousLearner, ContinuousSummary, EVENT_RESET_MODE,
    run_continuous_lifetime, run_event_reset_lifetime, sample_matched_inheritance,
};
use cra::experiments::episodic::{EPISODIC_MODE, EpisodicSummary, run_episodic_lifetime};
use cra::rng::{SeedTuple, rng_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::path::PathBuf;

const AUDIT_PLAN_PATH: &str = "manifests/m4_continuity_audit.json";
const M4_07_PLAN_PATH: &str = "manifests/m4_continuous_acquisition.json";
const M4_07_RECORDS_PATH: &str = "docs/evidence/m4-07/run/records.jsonl";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditPlan {
    schema_version: u32,
    task: String,
    purpose: String,
    frozen_source: FrozenSource,
    base_profile: PathBuf,
    namespace: String,
    root_seed: u64,
    lifetime_indices: Vec<u64>,
    outcomes_per_lifetime: u64,
    actor_family: ActorFamily,
    acquisition_windows: WindowPlan,
    audit_runs: Vec<AuditRun>,
    new_development_ranges: Vec<RangeChange>,
    diagnostics: Vec<String>,
    integrity_rules: Vec<String>,
    prohibitions: Vec<String>,
    budget: Budget,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenSource {
    m4_07_manifest: String,
    m4_07_manifest_sha256: String,
    m4_07_evidence: String,
    m4_07_verdict: String,
    note: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorFamily {
    source: String,
    m3_grid_index: usize,
    eta: f64,
    input_scale: f64,
    recurrent_gain: f64,
    noise_sigma: f64,
    plastic_mask: String,
    tau_e: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowPlan {
    unit: String,
    early_exposures: usize,
    late_exposures: usize,
    aggregation: String,
    minimum_exposures_per_cue: usize,
    note: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditRun {
    id: String,
    outer_seed: u64,
    continuity: String,
    learning: String,
    role: String,
    tau_e_override: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RangeChange {
    parameter: String,
    baseline: f64,
    audit_values: Vec<f64>,
    reason: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    lifetimes: usize,
    outcomes: u64,
    maximum_ticks_per_lifetime: u64,
    maximum_total_ticks: u64,
    execution: String,
}

fn load_audit_plan() -> AuditPlan {
    let bytes = std::fs::read(AUDIT_PLAN_PATH).expect("M4-08 audit plan exists");
    serde_json::from_slice(&bytes).expect("M4-08 audit plan parses")
}

fn sha256_file(path: &str) -> String {
    let bytes = std::fs::read(path).expect("hashable file readable");
    format!("{:x}", Sha256::digest(&bytes))
}

/// M4-07 family config rebuilt from the audit declaration (same overrides).
fn family_config(plan: &AuditPlan) -> Config {
    let mut cfg = load_and_validate(&plan.base_profile).expect("base profile validates");
    cfg.simulation.outcomes_per_lifetime = plan.outcomes_per_lifetime;
    let actor = cfg.actor.as_mut().expect("base actor");
    actor.input_scale = plan.actor_family.input_scale;
    actor.recurrent_gain = plan.actor_family.recurrent_gain;
    actor.noise_sigma = plan.actor_family.noise_sigma;
    let learning = cfg.learning.as_mut().expect("base learning");
    learning.eta = plan.actor_family.eta;
    learning.plastic_mask = plan.actor_family.plastic_mask.clone();
    learning.tau_e = plan.actor_family.tau_e;
    cfg
}

fn persistent_config(plan: &AuditPlan) -> Config {
    let mut cfg = family_config(plan);
    cfg.profile_name = "m4_08_continuous_persistent".to_owned();
    cfg.simulation.reset_policy = "birth_only".to_owned();
    cfg.learning.as_mut().expect("learning").trace_policy = "persistent".to_owned();
    validate_continuous_execution(&cfg).expect("persistent config executes");
    cfg
}

fn timescale_config(plan: &AuditPlan, tau_e: f64) -> Config {
    let mut cfg = persistent_config(plan);
    cfg.profile_name = format!("m4_08_continuous_persistent_tau{tau_e}");
    cfg.learning.as_mut().expect("learning").tau_e = tau_e;
    validate_continuous_execution(&cfg).expect("timescale config executes");
    cfg
}

fn event_reset_config(plan: &AuditPlan) -> Config {
    let mut cfg = family_config(plan);
    cfg.profile_name = "m4_08_event_reset_diagnostic".to_owned();
    cfg.simulation.reset_policy = "event_reset_diagnostic".to_owned();
    cfg.learning.as_mut().expect("learning").trace_policy = "persistent".to_owned();
    validate_event_reset_execution(&cfg).expect("event-reset config executes");
    cfg
}

fn episodic_config(plan: &AuditPlan) -> Config {
    let mut cfg = family_config(plan);
    cfg.profile_name = "m4_08_episodic_diagnostic".to_owned();
    cfg.simulation.reset_policy = "episodic_diagnostic".to_owned();
    cfg.learning.as_mut().expect("learning").trace_policy = "no_decay_diagnostic".to_owned();
    validate_episodic_execution(&cfg).expect("episodic config executes");
    cfg
}

fn nonplastic_config(plan: &AuditPlan) -> Config {
    let mut cfg = persistent_config(plan);
    cfg.profile_name = "m4_08_continuous_b3".to_owned();
    cfg.learning.as_mut().expect("learning").enabled = false;
    validate_actor_no_learning_execution(&cfg).expect("continuous B3 config executes");
    cfg
}

fn config_sha256(cfg: &Config) -> String {
    let resolved = resolved_toml(cfg).expect("resolved config serializes");
    format!("{:x}", Sha256::digest(resolved.as_bytes()))
}

fn l1(matrix: &[Vec<f64>]) -> f64 {
    matrix.iter().flatten().map(|v| v.abs()).sum()
}

/// Cosine of two flattened matrices; 0.0 when either side has zero norm
/// (a silent outcome carries no direction to align with).
fn cosine(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    let (mut dot, mut na, mut nb) = (0.0, 0.0, 0.0);
    for (ra, rb) in a.iter().zip(b.iter()) {
        for (&x, &y) in ra.iter().zip(rb.iter()) {
            dot += x * y;
            na += x * x;
            nb += y * y;
        }
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// Implied retained fraction of a trace increment across `gap` ticks at the
/// given `tau_e` (spec 7.9: `exp(-D/tau_e)`).
fn retained_fraction(gap_ticks: u64, tau_e: f64) -> f64 {
    (-(gap_ticks as f64) / tau_e).exp()
}

/// One audited outcome with the per-event series the M4-07 aggregates drop.
#[derive(Clone, Debug, PartialEq, Serialize)]
struct OutcomeRow {
    outcome_index: usize,
    cue: usize,
    action: u8,
    reward: f64,
    correct: bool,
    delta: f64,
    baseline_old: f64,
    baseline_new: f64,
    eligibility_l1_before_update: Option<f64>,
    raw_l1: f64,
    actual_l1: f64,
    commit_tick: u64,
    feedback_tick: u64,
}

fn continuous_rows(summary: &ContinuousSummary) -> Vec<OutcomeRow> {
    summary
        .choices
        .iter()
        .enumerate()
        .map(|(k, c)| OutcomeRow {
            outcome_index: k,
            cue: c.cue,
            action: c.action,
            reward: c.reward,
            correct: c.correct,
            delta: c.update.delta,
            baseline_old: c.update.baseline_old,
            baseline_new: c.update.baseline_new,
            eligibility_l1_before_update: Some(c.eligibility_l1_before_update),
            raw_l1: l1(&c.update.raw_updates),
            actual_l1: l1(&c.update.actual_updates),
            commit_tick: c.commit_tick,
            feedback_tick: c.feedback_tick,
        })
        .collect()
}

fn episodic_rows(summary: &EpisodicSummary) -> Vec<OutcomeRow> {
    summary
        .choices
        .iter()
        .enumerate()
        .map(|(k, c)| OutcomeRow {
            outcome_index: k,
            cue: c.cue,
            action: c.action,
            reward: c.reward,
            correct: c.correct,
            delta: c.update.delta,
            baseline_old: c.update.baseline_old,
            baseline_new: c.update.baseline_new,
            eligibility_l1_before_update: None,
            raw_l1: l1(&c.update.raw_updates),
            actual_l1: l1(&c.update.actual_updates),
            commit_tick: c.commit_tick,
            feedback_tick: c.feedback_tick,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct CueAudit {
    cue: usize,
    exposures: usize,
    early_accuracy: f64,
    late_accuracy: f64,
    early_reward: f64,
    late_reward: f64,
    early_action0_rate: f64,
    late_action0_rate: f64,
}

/// Per-cue early/late accuracy plus action-0 rates over first/final
/// `early`/`late` exposures. The action rates are the behavioral
/// representation-lock signature: a persistently locked actor answers both
/// cues with the same action in both windows.
fn per_cue_audit(rows: &[OutcomeRow], early: usize, late: usize) -> Vec<CueAudit> {
    let mut cues: Vec<usize> = rows.iter().map(|r| r.cue).collect();
    cues.sort_unstable();
    cues.dedup();
    let mut out = Vec::with_capacity(cues.len());
    for cue in cues {
        let cue_rows: Vec<&OutcomeRow> = rows.iter().filter(|r| r.cue == cue).collect();
        assert!(
            cue_rows.len() >= early + late,
            "cue {cue} has {} exposures, needs {}",
            cue_rows.len(),
            early + late
        );
        let rate = |slice: &[&OutcomeRow], f: fn(&OutcomeRow) -> f64| {
            slice.iter().map(|r| f(r)).sum::<f64>() / slice.len() as f64
        };
        let acc = |r: &OutcomeRow| if r.correct { 1.0 } else { 0.0 };
        let act0 = |r: &OutcomeRow| if r.action == 0 { 1.0 } else { 0.0 };
        let rew = |r: &OutcomeRow| r.reward;
        out.push(CueAudit {
            cue,
            exposures: cue_rows.len(),
            early_accuracy: rate(&cue_rows[..early], acc),
            late_accuracy: rate(&cue_rows[cue_rows.len() - late..], acc),
            early_reward: rate(&cue_rows[..early], rew),
            late_reward: rate(&cue_rows[cue_rows.len() - late..], rew),
            early_action0_rate: rate(&cue_rows[..early], act0),
            late_action0_rate: rate(&cue_rows[cue_rows.len() - late..], act0),
        });
    }
    out
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct DriftStats {
    first: f64,
    mid_mean: f64,
    last: f64,
    min: f64,
    max: f64,
}

/// Baseline-drift summary over the per-outcome `baseline_new` series.
fn baseline_drift(series: &[f64]) -> DriftStats {
    assert!(!series.is_empty());
    let mid = &series[series.len() / 4..3 * series.len() / 4];
    DriftStats {
        first: series[0],
        mid_mean: mid.iter().sum::<f64>() / mid.len() as f64,
        last: series[series.len() - 1],
        min: series.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        max: series.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
    }
}

/// Mean consecutive-outcome cosine of actual-update matrices over a row
/// range. Persistent traces share components across neighboring outcomes,
/// so this is the update-alignment interference signature.
fn mean_consecutive_cosine(
    actuals: &[Vec<Vec<f64>>],
    range: std::ops::Range<usize>,
) -> (f64, usize) {
    assert!(range.start <= range.end && range.end <= actuals.len());
    if range.len() < 2 {
        return (0.0, 0);
    }
    let mut sum = 0.0;
    let mut n = 0usize;
    for k in range.start..range.end - 1 {
        sum += cosine(&actuals[k], &actuals[k + 1]);
        n += 1;
    }
    (sum / n as f64, n)
}

fn close(got: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (got - expected).abs() <= tol,
        "{what}: got {got}, expected {expected}"
    );
}

#[test]
fn audit_manifest_freezes_m4_07_and_declares_ranges_with_reasons() {
    let plan = load_audit_plan();
    assert_eq!(plan.schema_version, 1);
    assert_eq!(plan.task, "M4-08");
    assert!(!plan.purpose.trim().is_empty());
    // Frozen coordinates are identical to the M4-07 declaration.
    assert_eq!(plan.namespace, "development");
    assert_eq!(plan.root_seed, 1);
    assert_eq!(plan.lifetime_indices, vec![0]);
    assert_eq!(plan.outcomes_per_lifetime, 2_000);
    assert_eq!(plan.actor_family.eta, 0.001);
    assert_eq!(plan.actor_family.input_scale, 0.2);
    assert_eq!(plan.actor_family.recurrent_gain, 0.8);
    assert_eq!(plan.actor_family.noise_sigma, 0.05);
    assert_eq!(plan.actor_family.plastic_mask, "all_recurrent_edges");
    assert_eq!(plan.actor_family.tau_e, 32.0);
    assert_eq!(plan.acquisition_windows.early_exposures, 100);
    assert_eq!(plan.acquisition_windows.late_exposures, 100);
    assert_eq!(
        plan.acquisition_windows.aggregation,
        "equal-weight macro mean across cues"
    );
    // The referenced M4-07 manifest on disk still hashes to the declared
    // value: the negative record is untouched by this audit.
    assert_eq!(
        plan.frozen_source.m4_07_manifest,
        "manifests/m4_continuous_acquisition.json"
    );
    assert_eq!(
        plan.frozen_source.m4_07_manifest_sha256,
        sha256_file(M4_07_PLAN_PATH)
    );
    assert_eq!(
        plan.frozen_source.m4_07_evidence,
        "docs/evidence/m4-07/summary.md"
    );
    assert!(plan.frozen_source.m4_07_verdict.contains("0/3"));
    assert!(!plan.frozen_source.note.trim().is_empty());
    assert!(plan.actor_family.source.contains("M3-08"));
    assert_eq!(plan.actor_family.m3_grid_index, 11);
    assert_eq!(plan.acquisition_windows.unit, "exposures_per_cue");
    assert_eq!(plan.acquisition_windows.minimum_exposures_per_cue, 200);
    assert!(!plan.acquisition_windows.note.trim().is_empty());
    // The eight declared legs are exactly the audit set.
    let ids: Vec<_> = plan.audit_runs.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "outer2_episodic_b4",
            "outer2_event_reset_b4",
            "outer2_continuous_b4",
            "outer2_continuous_b3",
            "outer3_continuous_b4",
            "outer1_continuous_b4",
            "outer2_continuous_b4_tau16",
            "outer2_continuous_b4_tau64",
        ]
    );
    for run in &plan.audit_runs {
        assert!(!run.role.trim().is_empty());
    }
    let expected_runs = [
        (
            "outer2_episodic_b4",
            2u64,
            "episodic_diagnostic",
            "fixed_gate_plastic",
        ),
        (
            "outer2_event_reset_b4",
            2,
            "event_reset_diagnostic",
            "fixed_gate_plastic",
        ),
        (
            "outer2_continuous_b4",
            2,
            "continuous_persistent",
            "fixed_gate_plastic",
        ),
        (
            "outer2_continuous_b3",
            2,
            "birth_only_nonplastic",
            "disabled",
        ),
        (
            "outer3_continuous_b4",
            3,
            "continuous_persistent",
            "fixed_gate_plastic",
        ),
        (
            "outer1_continuous_b4",
            1,
            "continuous_persistent",
            "fixed_gate_plastic",
        ),
        (
            "outer2_continuous_b4_tau16",
            2,
            "continuous_persistent",
            "fixed_gate_plastic",
        ),
        (
            "outer2_continuous_b4_tau64",
            2,
            "continuous_persistent",
            "fixed_gate_plastic",
        ),
    ];
    for (run, (id, outer, continuity, learning)) in plan.audit_runs.iter().zip(expected_runs.iter())
    {
        assert_eq!(run.id, *id);
        assert_eq!(run.outer_seed, *outer);
        assert_eq!(run.continuity, *continuity);
        assert_eq!(run.learning, *learning);
    }
    // The only new development range is tau_e, with a recorded spec reason.
    assert_eq!(plan.new_development_ranges.len(), 1);
    let range = &plan.new_development_ranges[0];
    assert_eq!(range.parameter, "tau_e");
    assert_eq!(range.baseline, 32.0);
    assert_eq!(range.audit_values, vec![16.0, 64.0]);
    assert!(range.reason.contains("7.9"));
    let tau_legs: Vec<_> = plan
        .audit_runs
        .iter()
        .filter(|r| r.tau_e_override.is_some())
        .collect();
    assert_eq!(tau_legs.len(), 2);
    assert!(
        tau_legs
            .iter()
            .all(|r| range.audit_values.contains(&r.tau_e_override.expect("tau")))
    );
    assert!(
        plan.audit_runs
            .iter()
            .filter(|r| r.tau_e_override.is_none())
            .count()
            == plan.audit_runs.len() - 2
    );
    // Every diagnostic family the task requires is named.
    let diagnostics = plan.diagnostics.join("\n");
    for keyword in [
        "baseline_drift",
        "update_alignment",
        "trace_scale",
        "inter_feedback_decay",
        "per_cue_behavior",
        "health",
        "representation_probe",
        "persistent_closed_loop_probe",
        "archive_cross_check",
    ] {
        assert!(
            diagnostics.contains(keyword),
            "missing diagnostic {keyword}"
        );
    }
    assert!(!plan.integrity_rules.is_empty());
    assert!(!plan.prohibitions.is_empty());
    // Finite declared budget.
    assert_eq!(plan.budget.lifetimes, plan.audit_runs.len());
    assert_eq!(plan.budget.outcomes, plan.outcomes_per_lifetime * 8);
    assert_eq!(
        plan.budget.maximum_total_ticks,
        plan.budget.maximum_ticks_per_lifetime * 8
    );
    assert!(plan.budget.execution.contains("serial release-mode"));
    // All six configs build and validate, including both tau_e legs.
    let persistent = persistent_config(&plan);
    let event = event_reset_config(&plan);
    let episodic = episodic_config(&plan);
    let b3 = nonplastic_config(&plan);
    let tau16 = timescale_config(&plan, 16.0);
    let tau64 = timescale_config(&plan, 64.0);
    assert_eq!(tau16.learning.as_ref().expect("learning").tau_e, 16.0);
    assert_eq!(tau64.learning.as_ref().expect("learning").tau_e, 64.0);
    assert_eq!(persistent.learning.as_ref().expect("learning").tau_e, 32.0);
    assert_eq!(persistent.environment.kind, "stationary_clean");
    assert_eq!(event.simulation.reset_policy, "event_reset_diagnostic");
    assert_eq!(episodic.simulation.reset_policy, "episodic_diagnostic");
    assert_eq!(b3.simulation.reset_policy, "birth_only");
    assert!(!b3.learning.as_ref().expect("learning").enabled);
}

fn small_topology() -> cra::agent::topology::Topology {
    // N = 4, m = 1: M0 = [2], M1 = [3], non-motor [0, 1].
    let mask = vec![
        vec![false, true, false, false],
        vec![true, false, false, false],
        vec![true, false, false, true],
        vec![false, true, true, false],
    ];
    topology_from_mask(4, 1, 0.25, false, mask, "test".to_owned()).expect("mask")
}

fn small_w0(topology: &cra::agent::topology::Topology) -> Vec<Vec<f64>> {
    let mut w0 = vec![vec![0.0; 4]; 4];
    for (j, row) in topology.mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                w0[j][i] = 0.1 * (j as f64 + 1.0) + 0.01 * (i as f64 + 1.0);
            }
        }
    }
    w0
}

fn update_params(eta: f64, max_update: f64, baseline_beta: f64) -> FeedbackUpdateParams {
    FeedbackUpdateParams {
        eta,
        max_update,
        baseline_beta,
    }
}

/// Seed `E` (and optionally baseline) on a hand-built state through the
/// validated snapshot path. Between-feedback trace evolution is set
/// explicitly so the leak under test is exact, not sampled.
fn seeded_state(
    topology: &cra::agent::topology::Topology,
    w0: &[Vec<f64>],
    edge: (usize, usize),
    e_value: f64,
    baseline: f64,
) -> PlasticState {
    let state =
        PlasticState::new(topology, w0, "all_recurrent_edges", "persistent", 32.0, 0.5).unwrap();
    let mut snap = state.snapshot();
    snap.e[edge.0][edge.1] = e_value;
    snap.reward_baseline = baseline;
    PlasticState::restore(snap, topology, w0, 0.5).unwrap()
}

#[test]
fn stale_trace_component_enters_the_next_update_by_construction() {
    // Mechanism (spec 7.7): feedback reads E and never clears it, so scores
    // from a previous choice, decayed across the inter-choice gap, are part
    // of the next outcome's update. Two hand-set steps with eta = 0.01,
    // max_update far above the raw scale, beta = 0.1, gate 1 on edge (2, 0).
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let gates = vec![1.0; 4];
    let params = update_params(0.01, 1.0, 0.1);
    // Step 1: E = 0.8 from the first choice, baseline 0.5, reward 1.0.
    let mut state = seeded_state(&topology, &w0, (2, 0), 0.8, 0.5);
    let out1 = state
        .apply_feedback_once(0, 1.0, &gates, params, &w0)
        .expect("first feedback applies");
    close(out1.delta, 0.5, 1e-15, "step-1 delta");
    close(
        out1.raw_updates[2][0],
        0.01 * 0.5 * 0.8,
        1e-15,
        "step-1 raw",
    );
    close(out1.baseline_new, 0.55, 1e-15, "step-1 baseline");
    // Feedback must not clear the trace it just read.
    close(state.e()[2][0], 0.8, 0.0, "E persists through feedback");
    // Between choices, 17 live ticks pass (stationary cycle length) at
    // tau_e = 32 and the new choice adds fresh scores of 0.5 on the edge.
    let decay = (-17.0f64 / 32.0).exp();
    let mut snap = state.snapshot();
    snap.e[2][0] = 0.5 + 0.8 * decay;
    let mut state = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    let out2 = state
        .apply_feedback_once(1, 1.0, &gates, params, &w0)
        .expect("second feedback applies");
    close(out2.baseline_old, 0.55, 1e-15, "step-2 old baseline");
    close(out2.delta, 0.45, 1e-15, "step-2 delta");
    let expected = 0.01 * 0.45 * (0.5 + 0.8 * decay);
    close(
        out2.raw_updates[2][0],
        expected,
        1e-15,
        "step-2 raw uses full E",
    );
    // The stale share is isolated and nonzero: the previous choice explains
    // part of the current update through the live trace alone.
    let stale_share = 0.01 * 0.45 * 0.8 * decay;
    close(
        out2.raw_updates[2][0] - 0.01 * 0.45 * 0.5,
        stale_share,
        1e-15,
        "stale component isolated",
    );
    assert!(
        stale_share > 0.001,
        "stale share is material: {stale_share}"
    );
}

#[test]
fn audit_analyzers_match_hand_computed_values() {
    // Cosine: orthogonal, identical, zero-norm, and scaled cases.
    close(
        cosine(&[vec![1.0, 0.0]], &[vec![0.0, 1.0]]),
        0.0,
        1e-15,
        "orthogonal",
    );
    close(
        cosine(&[vec![1.0, 1.0]], &[vec![1.0, 1.0]]),
        1.0,
        1e-15,
        "identical",
    );
    close(
        cosine(&[vec![0.0, 0.0]], &[vec![1.0, 2.0]]),
        0.0,
        1e-15,
        "zero norm",
    );
    close(
        cosine(
            &[vec![1.0, 0.0], vec![0.0, 0.0]],
            &[vec![2.0, 0.0], vec![0.0, 0.0]],
        ),
        1.0,
        1e-15,
        "scaled",
    );
    close(cosine(&[vec![1.0]], &[vec![-1.0]]), -1.0, 1e-15, "opposite");
    // Retained fraction: no gap keeps everything; one time constant keeps
    // exactly 1/e; the stationary 17-tick cycle at tau_e = 32 is partial.
    close(retained_fraction(0, 32.0), 1.0, 0.0, "zero gap");
    close(
        retained_fraction(32, 32.0),
        0.36787944117144233,
        1e-15,
        "one time constant",
    );
    let cycle = retained_fraction(17, 32.0);
    assert!(
        (0.5..0.7).contains(&cycle),
        "stationary-cycle retention is partial: {cycle}"
    );
    // Baseline drift on a hand-built rising series.
    let drift = baseline_drift(&[0.5, 0.6, 0.7, 0.8]);
    close(drift.first, 0.5, 0.0, "drift first");
    close(drift.last, 0.8, 0.0, "drift last");
    close(drift.min, 0.5, 0.0, "drift min");
    close(drift.max, 0.8, 0.0, "drift max");
    close(
        drift.mid_mean,
        0.65,
        1e-15,
        "drift mid mean over [0.6, 0.7]",
    );
    // Per-cue audit on synthetic rows: cue 0 correct early then wrong with
    // action 0 throughout; cue 1 wrong early then correct, flipping to
    // action 1 late.
    let row = |cue: usize, action: u8, correct: bool| OutcomeRow {
        outcome_index: 0,
        cue,
        action,
        reward: if correct { 1.0 } else { 0.0 },
        correct,
        delta: 0.0,
        baseline_old: 0.5,
        baseline_new: 0.5,
        eligibility_l1_before_update: None,
        raw_l1: 0.0,
        actual_l1: 0.0,
        commit_tick: 0,
        feedback_tick: 0,
    };
    let rows = vec![
        row(0, 0, true),
        row(1, 0, false),
        row(0, 0, true),
        row(1, 0, false),
        row(0, 0, false),
        row(1, 1, true),
        row(0, 0, false),
        row(1, 1, true),
    ];
    let audit = per_cue_audit(&rows, 2, 2);
    assert_eq!(audit.len(), 2);
    close(audit[0].early_accuracy, 1.0, 0.0, "cue 0 early acc");
    close(audit[0].late_accuracy, 0.0, 0.0, "cue 0 late acc");
    close(audit[0].early_action0_rate, 1.0, 0.0, "cue 0 early act0");
    close(audit[0].late_action0_rate, 1.0, 0.0, "cue 0 late act0");
    close(audit[1].early_accuracy, 0.0, 0.0, "cue 1 early acc");
    close(audit[1].late_accuracy, 1.0, 0.0, "cue 1 late acc");
    close(audit[1].early_action0_rate, 1.0, 0.0, "cue 1 early act0");
    close(audit[1].late_action0_rate, 0.0, 0.0, "cue 1 late act0");
    // Consecutive cosine: [M, M, -M] averages (1 + -1) / 2 = 0 overall and
    // 1 over the first pair alone.
    let m = vec![vec![0.3, -0.1], vec![0.0, 0.2]];
    let neg_m: Vec<Vec<f64>> = m.iter().map(|r| r.iter().map(|v| -v).collect()).collect();
    let (all, n) = mean_consecutive_cosine(&[m.clone(), m.clone(), neg_m], 0..3);
    assert_eq!(n, 2);
    close(all, 0.0, 1e-12, "opposite pair cancels");
    let (first, n1) = mean_consecutive_cosine(&[m.clone(), m.clone(), m], 0..2);
    assert_eq!(n1, 1);
    close(first, 1.0, 1e-12, "identical pair aligns");
}

/// Synthetic cue features for the representation probe: one-hot cue plus
/// cue-present, with go/outcome/previous-action channels at zero. The probe
/// measures raw cue drive into recurrent activity, not task behavior.
fn probe_features(cue: usize, cue_count: usize) -> Vec<f64> {
    let mut features = vec![0.0; cue_count + 6];
    features[cue] = 1.0;
    features[cue_count] = 1.0;
    features
}

/// Build a fresh outer-seed family learner for diagnostic-only synthetic
/// drive. Uses the public agent-only constructor and dedicated probe RNG
/// streams, never hidden state or schedules.
fn probe_learner(cfg: &Config, plan: &AuditPlan, outer: u64, tag: &str) -> ContinuousLearner {
    let (inherited, _) = sample_matched_inheritance(cfg, plan.root_seed, &plan.namespace, outer)
        .expect("matched inheritance");
    let actor = cfg.actor.clone().expect("actor");
    let learning = cfg.learning.clone().expect("learning");
    let cue_count = cfg.environment.cue_count;
    let noise = rng_for(&SeedTuple::new(
        plan.root_seed,
        &plan.namespace,
        outer,
        0,
        &format!("m4_08_{tag}_noise"),
    ))
    .expect("probe noise rng");
    let tie = rng_for(&SeedTuple::new(
        plan.root_seed,
        &plan.namespace,
        outer,
        0,
        &format!("m4_08_{tag}_tie"),
    ))
    .expect("probe tie rng");
    ContinuousLearner::from_agent_parts(actor, learning, inherited, cue_count, noise, tie)
        .expect("probe learner builds")
}

fn drive(learner: &mut ContinuousLearner, features: &[f64], ticks: usize) -> Vec<f64> {
    for _ in 0..ticks {
        learner.advance(features).expect("probe advance");
    }
    learner.actor_state().r().to_vec()
}

fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// Representation/carryover probe (spec 21.2, diagnostic only):
/// fresh across-cue separation plus the shift that eight ticks of the other
/// cue leave on the same subsequent drive. Returns
/// `(fresh_separation, carryover_shift)`.
fn cue_probe_magnitudes(cfg: &Config, plan: &AuditPlan, outer: u64) -> (f64, f64) {
    let cue_count = cfg.environment.cue_count;
    let cue0 = probe_features(0, cue_count);
    let cue1 = probe_features(1, cue_count);
    let r_a = drive(&mut probe_learner(cfg, plan, outer, "probe"), &cue0, 8);
    let r_a2 = drive(&mut probe_learner(cfg, plan, outer, "probe"), &cue0, 8);
    assert_eq!(
        r_a, r_a2,
        "identical synthetic drive must reproduce activity bitwise"
    );
    let mut carried = probe_learner(cfg, plan, outer, "probe");
    drive(&mut carried, &cue1, 8);
    let r_b = drive(&mut carried, &cue0, 8);
    let r_c = drive(&mut probe_learner(cfg, plan, outer, "probe"), &cue1, 8);
    for v in [&r_a, &r_b, &r_c] {
        assert!(v.iter().all(|x| x.is_finite()), "probe activity finite");
    }
    (euclidean(&r_a, &r_c), euclidean(&r_a, &r_b))
}

#[test]
fn representation_probe_separates_cues_and_detects_carryover() {
    let plan = load_audit_plan();
    let cfg = persistent_config(&plan);
    assert_eq!(cfg.environment.cue_count, 2);
    let (fresh_sep, carryover) = cue_probe_magnitudes(&cfg, &plan, 2);
    // Cue drive must reach recurrent activity (M1-11), and the leak
    // (alpha_h < 1) must carry prior-cue information forward: both are
    // mechanism-existence facts, magnitudes are exported by the audit run.
    assert!(
        fresh_sep > 0.0,
        "fresh cues are distinguishable: {fresh_sep}"
    );
    assert!(
        carryover > 0.0,
        "prior cue leaves activity carryover: {carryover}"
    );
}

/// Constant-input synthetic-reward loop with no resets of any kind (spec
/// 21.1 smallest closed loop): does the fixed-gate rule still drift toward
/// the known preferred action under full persistence?
struct ClosedLoop {
    binned_preferred_rate: Vec<f64>,
    outcomes: usize,
    final_p_l1: f64,
    final_baseline: f64,
    update_l1_sum: f64,
    max_raw_identity_error: f64,
}

fn run_persistent_closed_loop(
    cfg: &Config,
    plan: &AuditPlan,
    outer: u64,
    ticks_per_outcome: usize,
    outcomes: usize,
    preferred_action: u8,
) -> ClosedLoop {
    let eta = cfg.learning.as_ref().expect("learning").eta;
    let mut learner = probe_learner(cfg, plan, outer, "loop");
    let features = probe_features(0, cfg.environment.cue_count);
    let mut preferred = 0usize;
    let mut bins = Vec::new();
    let mut bin_preferred = 0usize;
    let mut bin_total = 0usize;
    let mut update_l1_sum = 0.0;
    let mut max_err = 0.0;
    for k in 0..outcomes {
        for _ in 0..ticks_per_outcome {
            learner.advance(&features).expect("loop advance");
        }
        let action = learner.select_action();
        if action == preferred_action {
            preferred += 1;
            bin_preferred += 1;
        }
        bin_total += 1;
        let reward = if action == preferred_action { 1.0 } else { 0.0 };
        let e_l1 = l1(learner.plastic().e());
        let outcome = learner
            .apply_feedback(Feedback {
                event_id: k as u64 + 1,
                reward,
            })
            .expect("loop feedback applies");
        let raw_l1 = l1(&outcome.raw_updates);
        let expected = eta * outcome.delta.abs() * e_l1;
        let err = (raw_l1 - expected).abs();
        if err > max_err {
            max_err = err;
        }
        let tol = 1e-12 * expected.abs().max(1.0);
        assert!(
            err <= tol,
            "outcome {k} raw identity: raw {raw_l1} expected {expected}"
        );
        update_l1_sum += l1(&outcome.actual_updates);
        if bin_total == 50 {
            bins.push(bin_preferred as f64 / bin_total as f64);
            bin_preferred = 0;
            bin_total = 0;
        }
    }
    if bin_total > 0 {
        bins.push(bin_preferred as f64 / bin_total as f64);
    }
    let _ = preferred;
    ClosedLoop {
        binned_preferred_rate: bins,
        outcomes,
        final_p_l1: l1(learner.plastic().p()),
        final_baseline: learner.reward_baseline(),
        update_l1_sum,
        max_raw_identity_error: max_err,
    }
}

#[test]
fn persistent_closed_loop_probe_engages_learning_machinery() {
    let plan = load_audit_plan();
    let cfg = persistent_config(&plan);
    // Small structural run: the loop completes, updates are nonzero, the
    // raw identity holds every outcome, and state stays finite. The
    // directional question (does it drift to the preferred action?) is
    // exported by the bounded audit run, not asserted here.
    let result = run_persistent_closed_loop(&cfg, &plan, 2, 8, 24, 0);
    assert_eq!(result.outcomes, 24);
    assert!(result.update_l1_sum > 0.0, "updates engaged");
    assert!(result.final_p_l1 > 0.0, "offsets moved");
    assert!(
        result.final_baseline.is_finite() && (0.0..=1.0).contains(&result.final_baseline),
        "baseline sane"
    );
    assert!(result.max_raw_identity_error <= 1e-9);
}

fn schedule_key(cue: usize, commit: u64, feedback: u64, noise: bool) -> (usize, u64, u64, bool) {
    (cue, commit, feedback, noise)
}

#[test]
fn all_audit_conditions_run_paired_on_small_fixture() {
    let mut plan = load_audit_plan();
    plan.outcomes_per_lifetime = 48;
    let (early, late) = (2usize, 2usize);
    let persistent_cfg = persistent_config(&plan);
    let event_cfg = event_reset_config(&plan);
    let episodic_cfg = episodic_config(&plan);
    let b3_cfg = nonplastic_config(&plan);
    let tau16_cfg = timescale_config(&plan, 16.0);
    let tau64_cfg = timescale_config(&plan, 64.0);
    let root = plan.root_seed;
    let ns = &plan.namespace;
    let outer = 2u64;
    let lifetime = 0u64;

    let episodic = run_episodic_lifetime(&episodic_cfg, root, ns, outer, lifetime, "episodic_b4")
        .expect("episodic B4");
    let event = run_event_reset_lifetime(&event_cfg, root, ns, outer, lifetime, "event_reset_b4")
        .expect("event-reset B4");
    let continuous =
        run_continuous_lifetime(&persistent_cfg, root, ns, outer, lifetime, "continuous_b4")
            .expect("continuous B4");
    let tau16 =
        run_continuous_lifetime(&tau16_cfg, root, ns, outer, lifetime, "tau16").expect("tau16 B4");
    let tau64 =
        run_continuous_lifetime(&tau64_cfg, root, ns, outer, lifetime, "tau64").expect("tau64 B4");
    let mut b3actor =
        NoLearningActor::new(&b3_cfg, root, ns, outer, lifetime).expect("continuous B3");
    let b3_w0 = b3actor.inherited().weights.w0.clone();
    let continuous_b3 = run_actor_ordinary(
        &b3_cfg,
        root,
        ns,
        outer,
        lifetime,
        "continuous_b3",
        &mut b3actor,
    )
    .expect("continuous B3 run");

    assert_eq!(episodic.mode, EPISODIC_MODE);
    assert_eq!(event.mode, EVENT_RESET_MODE);
    assert_eq!(continuous.mode, CONTINUOUS_MODE);
    assert_eq!(episodic.resets.len(), 48);
    assert_eq!(event.resets.len(), 48);
    assert_eq!(continuous.resets, vec![0]);
    assert_eq!(tau16.resets, vec![0]);
    assert_eq!(tau64.resets, vec![0]);
    // Identical inheritance across all legs, including the tau_e variants
    // (tau_e never enters W0/schedule sampling).
    assert_eq!(episodic.w0, event.w0);
    assert_eq!(event.w0, continuous.w0);
    assert_eq!(continuous.w0, tau16.w0);
    assert_eq!(tau16.w0, tau64.w0);
    assert_eq!(tau64.w0, b3_w0);
    // Identical exogenous schedules across all legs.
    let expected: Vec<_> = continuous
        .choices
        .iter()
        .map(|c| schedule_key(c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
        .collect();
    for choices in [
        event
            .choices
            .iter()
            .map(|c| schedule_key(c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
            .collect::<Vec<_>>(),
        tau16
            .choices
            .iter()
            .map(|c| schedule_key(c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
            .collect::<Vec<_>>(),
        tau64
            .choices
            .iter()
            .map(|c| schedule_key(c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
            .collect::<Vec<_>>(),
    ] {
        assert_eq!(choices, expected);
    }
    assert_eq!(
        episodic
            .choices
            .iter()
            .map(|c| (c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
            .collect::<Vec<_>>(),
        continuous
            .choices
            .iter()
            .map(|c| (c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        continuous_b3
            .choices
            .iter()
            .map(|c| (c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
            .collect::<Vec<_>>(),
        continuous
            .choices
            .iter()
            .map(|c| (c.cue, c.commit_tick, c.feedback_tick, c.noise_bit))
            .collect::<Vec<_>>()
    );
    // Same birth state and first cue: the first committed action is
    // identical before any update or timescale effect can act.
    let first = continuous.choices[0].action;
    for other in [
        episodic.choices[0].action,
        event.choices[0].action,
        tau16.choices[0].action,
        tau64.choices[0].action,
        continuous_b3.choices[0].action,
    ] {
        assert_eq!(other, first);
    }
    // Raw identity holds on every persistent-style outcome (M4-05 scale
    // check, extended to the tau_e legs).
    let eta = persistent_cfg.learning.as_ref().expect("learning").eta;
    for summary in [&continuous, &event, &tau16, &tau64] {
        for choice in &summary.choices {
            let raw_l1 = l1(&choice.update.raw_updates);
            let expected_raw =
                eta * choice.update.delta.abs() * choice.eligibility_l1_before_update;
            let tol = 1e-12 * expected_raw.abs().max(1.0);
            assert!((raw_l1 - expected_raw).abs() <= tol, "raw identity holds");
        }
    }
    // Analyzers run sanely on every leg.
    for rows in [
        episodic_rows(&episodic),
        continuous_rows(&event),
        continuous_rows(&continuous),
        continuous_rows(&tau16),
        continuous_rows(&tau64),
    ] {
        let cues = per_cue_audit(&rows, early, late);
        assert_eq!(cues.len(), 2);
        let drift = baseline_drift(&rows.iter().map(|r| r.baseline_new).collect::<Vec<_>>());
        assert!(drift.min <= drift.max);
        let gaps: Vec<u64> = rows
            .windows(2)
            .map(|w| w[1].feedback_tick - w[0].feedback_tick)
            .collect();
        assert!(gaps.iter().all(|&g| g > 0));
        assert!(
            gaps.iter()
                .all(|&g| retained_fraction(g, 32.0) > 0.0 && retained_fraction(g, 32.0) <= 1.0)
        );
    }
}

fn command_output(command: &str, args: &[&str]) -> String {
    String::from_utf8_lossy(
        &std::process::Command::new(command)
            .args(args)
            .output()
            .expect("provenance command runs")
            .stdout,
    )
    .trim()
    .to_owned()
}

#[derive(Clone, Debug, Serialize)]
struct LegAudit {
    id: String,
    outer_seed: u64,
    continuity: String,
    learning: String,
    tau_e: Option<f64>,
    outcomes: u64,
    ticks: u64,
    late_macro_accuracy: f64,
    late_macro_reward: f64,
    per_cue: Vec<CueAudit>,
    baseline: DriftStats,
    eligibility_l1_early_mean: Option<f64>,
    eligibility_l1_late_mean: Option<f64>,
    raw_l1_mean: Option<f64>,
    actual_l1_mean: Option<f64>,
    raw_identity_max_error: Option<f64>,
    consecutive_cosine_all: Option<f64>,
    consecutive_cosine_first_half: Option<f64>,
    consecutive_cosine_second_half: Option<f64>,
    inter_feedback_gap_mean: f64,
    implied_retained_mean: Option<f64>,
    final_p_l1: Option<f64>,
    final_e_l1: Option<f64>,
    final_baseline: Option<f64>,
    saturated_fraction: Option<f64>,
    max_abs_motor_filter: f64,
    min_motor_margin: Option<f64>,
    max_motor_margin: f64,
    resets: usize,
}

fn mean(values: &[f64]) -> f64 {
    assert!(!values.is_empty());
    values.iter().sum::<f64>() / values.len() as f64
}

#[allow(clippy::too_many_arguments)]
fn audit_plastic_leg(
    id: &str,
    outer: u64,
    continuity: &str,
    learning: &str,
    tau_e: Option<f64>,
    rows: &[OutcomeRow],
    actuals: &[Vec<Vec<f64>>],
    final_p: &[Vec<f64>],
    final_e: &[Vec<f64>],
    final_baseline: f64,
    health: &HealthSummary,
    ticks: u64,
    resets: usize,
    eta: f64,
    early: usize,
    late: usize,
) -> LegAudit {
    let per_cue = per_cue_audit(rows, early, late);
    let late_acc = mean(&per_cue.iter().map(|c| c.late_accuracy).collect::<Vec<_>>());
    let late_rew = mean(&per_cue.iter().map(|c| c.late_reward).collect::<Vec<_>>());
    let baselines: Vec<f64> = rows.iter().map(|r| r.baseline_new).collect();
    let e_l1: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.eligibility_l1_before_update)
        .collect();
    let half = rows.len() / 2;
    let (cos_all, _) = mean_consecutive_cosine(actuals, 0..actuals.len());
    let (cos_first, _) = mean_consecutive_cosine(actuals, 0..half.max(1));
    let (cos_second, _) = mean_consecutive_cosine(actuals, half..actuals.len());
    let gaps: Vec<u64> = rows
        .windows(2)
        .map(|w| w[1].feedback_tick - w[0].feedback_tick)
        .collect();
    let gap_mean = gaps.iter().sum::<u64>() as f64 / gaps.len() as f64;
    let mut raw_err: f64 = 0.0;
    for row in rows {
        if let Some(e) = row.eligibility_l1_before_update {
            let expected = eta * row.delta.abs() * e;
            let err = (row.raw_l1 - expected).abs();
            if err > raw_err {
                raw_err = err;
            }
            let tol = 1e-12 * expected.abs().max(1.0);
            assert!(
                err <= tol,
                "leg {id} outcome {} raw identity",
                row.outcome_index
            );
        }
    }
    LegAudit {
        id: id.to_owned(),
        outer_seed: outer,
        continuity: continuity.to_owned(),
        learning: learning.to_owned(),
        tau_e,
        outcomes: rows.len() as u64,
        ticks,
        late_macro_accuracy: late_acc,
        late_macro_reward: late_rew,
        per_cue,
        baseline: baseline_drift(&baselines),
        eligibility_l1_early_mean: if e_l1.is_empty() {
            None
        } else {
            Some(mean(&e_l1[..half]))
        },
        eligibility_l1_late_mean: if e_l1.is_empty() {
            None
        } else {
            Some(mean(&e_l1[half..]))
        },
        raw_l1_mean: Some(mean(&rows.iter().map(|r| r.raw_l1).collect::<Vec<_>>())),
        actual_l1_mean: Some(mean(&rows.iter().map(|r| r.actual_l1).collect::<Vec<_>>())),
        raw_identity_max_error: if e_l1.is_empty() { None } else { Some(raw_err) },
        consecutive_cosine_all: Some(cos_all),
        consecutive_cosine_first_half: Some(cos_first),
        consecutive_cosine_second_half: Some(cos_second),
        inter_feedback_gap_mean: gap_mean,
        implied_retained_mean: tau_e.map(|t| {
            mean(
                &gaps
                    .iter()
                    .map(|&g| retained_fraction(g, t))
                    .collect::<Vec<_>>(),
            )
        }),
        final_p_l1: Some(l1(final_p)),
        final_e_l1: Some(l1(final_e)),
        final_baseline: Some(final_baseline),
        saturated_fraction: health.saturated_fraction(),
        max_abs_motor_filter: health.max_abs_q,
        min_motor_margin: health.min_margin(),
        max_motor_margin: health.max_motor_margin,
        resets,
    }
}

fn b3_rows(summary: &BaselineSummary) -> Vec<(usize, u8, bool, f64)> {
    summary
        .choices
        .iter()
        .map(|c| (c.cue, c.action, c.correct, c.reward))
        .collect()
}

fn archived_late_accuracy(outer: u64, condition: &str) -> f64 {
    let text = std::fs::read_to_string(M4_07_RECORDS_PATH).expect("M4-07 records readable");
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("record parses");
        if value["outer_seed"] == outer && value["condition"] == condition {
            return value["behavior"]["late_macro_accuracy"]
                .as_f64()
                .expect("late accuracy present");
        }
    }
    panic!("no archived record for outer {outer} condition {condition}");
}

#[test]
#[ignore = "bounded M4-08 audit export (8 legs x 2,000 outcomes plus probes); invoke explicitly in release with CRA_M4_AUDIT_DIR"]
fn m4_continuity_audit_export() {
    let out_dir = std::env::var_os("CRA_M4_AUDIT_DIR")
        .map(PathBuf::from)
        .expect("set CRA_M4_AUDIT_DIR to a fresh path");
    std::fs::create_dir(&out_dir).expect("evidence directory must be fresh");
    let plan = load_audit_plan();
    assert_eq!(
        plan.frozen_source.m4_07_manifest_sha256,
        sha256_file(M4_07_PLAN_PATH),
        "M4-07 manifest must be untouched at audit execution"
    );
    let plan_bytes = std::fs::read(AUDIT_PLAN_PATH).expect("audit plan readable");
    let audit_sha256 = format!("{:x}", Sha256::digest(&plan_bytes));
    let persistent_cfg = persistent_config(&plan);
    let event_cfg = event_reset_config(&plan);
    let episodic_cfg = episodic_config(&plan);
    let b3_cfg = nonplastic_config(&plan);
    let tau16_cfg = timescale_config(&plan, 16.0);
    let tau64_cfg = timescale_config(&plan, 64.0);
    let eta = persistent_cfg.learning.as_ref().expect("learning").eta;
    let (early, late) = (
        plan.acquisition_windows.early_exposures,
        plan.acquisition_windows.late_exposures,
    );
    let root = plan.root_seed;
    let ns = &plan.namespace;
    let lifetime = plan.lifetime_indices[0];

    let series_path = out_dir.join("series.jsonl");
    let mut series = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&series_path)
        .expect("series file must be fresh");
    let mut legs = Vec::new();
    let mut measured_ticks = 0u64;

    // Focus trio plus matched control on outer 2 (identical W0/schedule).
    let outer = 2u64;
    let episodic = run_episodic_lifetime(&episodic_cfg, root, ns, outer, lifetime, "episodic_b4")
        .expect("outer-2 episodic B4");
    let event = run_event_reset_lifetime(&event_cfg, root, ns, outer, lifetime, "event_reset_b4")
        .expect("outer-2 event-reset B4");
    let continuous =
        run_continuous_lifetime(&persistent_cfg, root, ns, outer, lifetime, "continuous_b4")
            .expect("outer-2 continuous B4");
    let tau16 = run_continuous_lifetime(&tau16_cfg, root, ns, outer, lifetime, "tau16")
        .expect("outer-2 tau16 B4");
    let tau64 = run_continuous_lifetime(&tau64_cfg, root, ns, outer, lifetime, "tau64")
        .expect("outer-2 tau64 B4");
    let mut b3actor = NoLearningActor::new(&b3_cfg, root, ns, outer, lifetime).expect("B3");
    let b3_w0 = b3actor.inherited().weights.w0.clone();
    let continuous_b3 = run_actor_ordinary(
        &b3_cfg,
        root,
        ns,
        outer,
        lifetime,
        "continuous_b3",
        &mut b3actor,
    )
    .expect("outer-2 continuous B3");

    assert_eq!(episodic.w0, event.w0);
    assert_eq!(event.w0, continuous.w0);
    assert_eq!(continuous.w0, tau16.w0);
    assert_eq!(tau16.w0, tau64.w0);
    assert_eq!(tau64.w0, b3_w0);
    let expected_schedule: Vec<_> = continuous
        .choices
        .iter()
        .map(|c| {
            (
                c.event_id,
                c.cue,
                c.commit_tick,
                c.feedback_tick,
                c.noise_bit,
            )
        })
        .collect();
    for (name, schedule) in [
        (
            "episodic",
            episodic
                .choices
                .iter()
                .map(|c| {
                    (
                        c.event_id,
                        c.cue,
                        c.commit_tick,
                        c.feedback_tick,
                        c.noise_bit,
                    )
                })
                .collect::<Vec<_>>(),
        ),
        (
            "event",
            event
                .choices
                .iter()
                .map(|c| {
                    (
                        c.event_id,
                        c.cue,
                        c.commit_tick,
                        c.feedback_tick,
                        c.noise_bit,
                    )
                })
                .collect::<Vec<_>>(),
        ),
        (
            "tau16",
            tau16
                .choices
                .iter()
                .map(|c| {
                    (
                        c.event_id,
                        c.cue,
                        c.commit_tick,
                        c.feedback_tick,
                        c.noise_bit,
                    )
                })
                .collect::<Vec<_>>(),
        ),
        (
            "tau64",
            tau64
                .choices
                .iter()
                .map(|c| {
                    (
                        c.event_id,
                        c.cue,
                        c.commit_tick,
                        c.feedback_tick,
                        c.noise_bit,
                    )
                })
                .collect::<Vec<_>>(),
        ),
        (
            "b3",
            continuous_b3
                .choices
                .iter()
                .map(|c| {
                    (
                        c.event_id,
                        c.cue,
                        c.commit_tick,
                        c.feedback_tick,
                        c.noise_bit,
                    )
                })
                .collect::<Vec<_>>(),
        ),
    ] {
        assert_eq!(schedule, expected_schedule, "{name} schedule pairs");
    }
    assert_eq!(episodic.resets.len(), plan.outcomes_per_lifetime as usize);
    assert_eq!(event.resets.len(), plan.outcomes_per_lifetime as usize);
    assert_eq!(continuous.resets, vec![0]);

    fn record_leg(
        legs: &mut Vec<LegAudit>,
        measured_ticks: &mut u64,
        series: &mut std::fs::File,
        leg: LegAudit,
        rows: &[OutcomeRow],
    ) {
        for row in rows {
            let record = serde_json::json!({
                "task": "M4-08",
                "leg": leg.id,
                "outer_seed": leg.outer_seed,
                "row": row,
            });
            writeln!(series, "{record}").expect("series append");
        }
        *measured_ticks += leg.ticks;
        legs.push(leg);
    }

    let ep_rows = episodic_rows(&episodic);
    let ep_actuals: Vec<Vec<Vec<f64>>> = episodic
        .choices
        .iter()
        .map(|c| c.update.actual_updates.clone())
        .collect();
    record_leg(
        &mut legs,
        &mut measured_ticks,
        &mut series,
        audit_plastic_leg(
            "outer2_episodic_b4",
            outer,
            "episodic_diagnostic",
            "fixed_gate_plastic",
            None,
            &ep_rows,
            &ep_actuals,
            &episodic.final_p,
            &episodic.final_e,
            episodic.final_baseline,
            &episodic.health,
            episodic.ticks,
            episodic.resets.len(),
            eta,
            early,
            late,
        ),
        &ep_rows,
    );
    let event_rows = continuous_rows(&event);
    let event_actuals: Vec<Vec<Vec<f64>>> = event
        .choices
        .iter()
        .map(|c| c.update.actual_updates.clone())
        .collect();
    record_leg(
        &mut legs,
        &mut measured_ticks,
        &mut series,
        audit_plastic_leg(
            "outer2_event_reset_b4",
            outer,
            "event_reset_diagnostic",
            "fixed_gate_plastic",
            Some(32.0),
            &event_rows,
            &event_actuals,
            &event.final_p,
            &event.final_e,
            event.final_baseline,
            &event.health,
            event.ticks,
            event.resets.len(),
            eta,
            early,
            late,
        ),
        &event_rows,
    );
    let cont_rows = continuous_rows(&continuous);
    let cont_actuals: Vec<Vec<Vec<f64>>> = continuous
        .choices
        .iter()
        .map(|c| c.update.actual_updates.clone())
        .collect();
    record_leg(
        &mut legs,
        &mut measured_ticks,
        &mut series,
        audit_plastic_leg(
            "outer2_continuous_b4",
            outer,
            "continuous_persistent",
            "fixed_gate_plastic",
            Some(32.0),
            &cont_rows,
            &cont_actuals,
            &continuous.final_p,
            &continuous.final_e,
            continuous.final_baseline,
            &continuous.health,
            continuous.ticks,
            continuous.resets.len(),
            eta,
            early,
            late,
        ),
        &cont_rows,
    );
    for (id, summary, tau) in [
        ("outer2_continuous_b4_tau16", &tau16, 16.0),
        ("outer2_continuous_b4_tau64", &tau64, 64.0),
    ] {
        let rows = continuous_rows(summary);
        let actuals: Vec<Vec<Vec<f64>>> = summary
            .choices
            .iter()
            .map(|c| c.update.actual_updates.clone())
            .collect();
        record_leg(
            &mut legs,
            &mut measured_ticks,
            &mut series,
            audit_plastic_leg(
                id,
                outer,
                "continuous_persistent",
                "fixed_gate_plastic",
                Some(tau),
                &rows,
                &actuals,
                &summary.final_p,
                &summary.final_e,
                summary.final_baseline,
                &summary.health,
                summary.ticks,
                summary.resets.len(),
                eta,
                early,
                late,
            ),
            &rows,
        );
    }
    // Matched B3 control: behavior and health only (no E/P/baseline fields
    // exist on the nonplastic actor by construction).
    {
        let rows = b3_rows(&continuous_b3);
        let pseudo: Vec<OutcomeRow> = rows
            .iter()
            .enumerate()
            .map(|(k, (cue, action, correct, reward))| OutcomeRow {
                outcome_index: k,
                cue: *cue,
                action: *action,
                reward: *reward,
                correct: *correct,
                delta: f64::NAN,
                baseline_old: f64::NAN,
                baseline_new: f64::NAN,
                eligibility_l1_before_update: None,
                raw_l1: f64::NAN,
                actual_l1: f64::NAN,
                commit_tick: continuous_b3.choices[k].commit_tick,
                feedback_tick: continuous_b3.choices[k].feedback_tick,
            })
            .collect();
        let per_cue = per_cue_audit(&pseudo, early, late);
        let b3_health = continuous_b3.health.as_ref().expect("B3 health recorded");
        let b3_ticks = b3_health.ticks_observed;
        for row in &pseudo {
            let record = serde_json::json!({
                "task": "M4-08",
                "leg": "outer2_continuous_b3",
                "outer_seed": outer,
                "row": { "outcome_index": row.outcome_index, "cue": row.cue, "action": row.action, "reward": row.reward, "correct": row.correct, "commit_tick": row.commit_tick, "feedback_tick": row.feedback_tick },
            });
            writeln!(series, "{record}").expect("series append");
        }
        legs.push(LegAudit {
            id: "outer2_continuous_b3".to_owned(),
            outer_seed: outer,
            continuity: "birth_only_nonplastic".to_owned(),
            learning: "disabled".to_owned(),
            tau_e: None,
            outcomes: pseudo.len() as u64,
            ticks: b3_ticks,
            late_macro_accuracy: mean(&per_cue.iter().map(|c| c.late_accuracy).collect::<Vec<_>>()),
            late_macro_reward: mean(&per_cue.iter().map(|c| c.late_reward).collect::<Vec<_>>()),
            per_cue,
            baseline: DriftStats {
                first: f64::NAN,
                mid_mean: f64::NAN,
                last: f64::NAN,
                min: f64::NAN,
                max: f64::NAN,
            },
            eligibility_l1_early_mean: None,
            eligibility_l1_late_mean: None,
            raw_l1_mean: None,
            actual_l1_mean: None,
            raw_identity_max_error: None,
            consecutive_cosine_all: None,
            consecutive_cosine_first_half: None,
            consecutive_cosine_second_half: None,
            inter_feedback_gap_mean: mean(
                &pseudo
                    .windows(2)
                    .map(|w| (w[1].feedback_tick - w[0].feedback_tick) as f64)
                    .collect::<Vec<_>>(),
            ),
            implied_retained_mean: None,
            final_p_l1: None,
            final_e_l1: None,
            final_baseline: None,
            saturated_fraction: b3_health.saturated_fraction(),
            max_abs_motor_filter: b3_health.max_abs_q,
            min_motor_margin: b3_health.min_margin(),
            max_motor_margin: b3_health.max_motor_margin,
            resets: 1,
        });
        measured_ticks += b3_ticks;
    }

    // Contrast (outer 3) and locked control (outer 1), continuous B4 only.
    for outer_seed in [3u64, 1u64] {
        let summary = run_continuous_lifetime(
            &persistent_cfg,
            root,
            ns,
            outer_seed,
            lifetime,
            "continuous_b4",
        )
        .expect("contrast/control lifetime completes");
        assert_eq!(summary.resets, vec![0]);
        let rows = continuous_rows(&summary);
        let actuals: Vec<Vec<Vec<f64>>> = summary
            .choices
            .iter()
            .map(|c| c.update.actual_updates.clone())
            .collect();
        record_leg(
            &mut legs,
            &mut measured_ticks,
            &mut series,
            audit_plastic_leg(
                &format!("outer{outer_seed}_continuous_b4"),
                outer_seed,
                "continuous_persistent",
                "fixed_gate_plastic",
                Some(32.0),
                &rows,
                &actuals,
                &summary.final_p,
                &summary.final_e,
                summary.final_baseline,
                &summary.health,
                summary.ticks,
                summary.resets.len(),
                eta,
                early,
                late,
            ),
            &rows,
        );
    }

    // Archive cross-check: the re-runs must reproduce the frozen M4-07
    // records exactly (same revision, deterministic). Audit leg ids are
    // `outer{seed}_{condition}`: outer2_episodic_b4, outer2_continuous_b4,
    // outer3_continuous_b4, outer1_continuous_b4.
    let mut cross_check = Vec::new();
    for (outer_seed, condition) in [
        (2u64, "episodic_b4"),
        (2u64, "continuous_b4"),
        (3u64, "continuous_b4"),
        (1u64, "continuous_b4"),
    ] {
        let archived = archived_late_accuracy(outer_seed, condition);
        let rerun_leg = legs
            .iter()
            .find(|leg| leg.id == format!("outer{outer_seed}_{condition}"))
            .unwrap_or_else(|| panic!("leg for {outer_seed}/{condition}"));
        assert_eq!(
            rerun_leg.late_macro_accuracy, archived,
            "outer {outer_seed} {condition} reproduces the M4-07 archive exactly"
        );
        cross_check.push(serde_json::json!({
            "outer_seed": outer_seed,
            "condition": condition,
            "archived_late_macro_accuracy": archived,
            "rerun_late_macro_accuracy": rerun_leg.late_macro_accuracy,
            "exact_match": true,
        }));
    }

    // Diagnostic probes, exported with the audit (magnitudes only; the
    // fast tests own the structural assertions).
    let mut probes = Vec::new();
    for outer_seed in [2u64, 3u64, 1u64] {
        let (fresh_sep, carryover) = cue_probe_magnitudes(&persistent_cfg, &plan, outer_seed);
        probes.push(serde_json::json!({
            "probe": "representation_carryover",
            "outer_seed": outer_seed,
            "fresh_separation": fresh_sep,
            "carryover_shift": carryover,
            "carryover_over_fresh": carryover / fresh_sep,
        }));
    }
    // Both reward directions: preferred 0 matches the birth bias on the
    // constant cue-0 input, while preferred 1 asks whether the persistent
    // loop can escape a fully wrong lock at minimal scale.
    for outer_seed in [2u64, 3u64] {
        for preferred_action in [0u8, 1u8] {
            let closed = run_persistent_closed_loop(
                &persistent_cfg,
                &plan,
                outer_seed,
                8,
                600,
                preferred_action,
            );
            probes.push(serde_json::json!({
                "probe": "persistent_closed_loop",
                "outer_seed": outer_seed,
                "preferred_action": preferred_action,
                "ticks_per_outcome": 8,
                "outcomes": closed.outcomes,
                "binned_preferred_rate_50": closed.binned_preferred_rate,
                "final_p_l1": closed.final_p_l1,
                "final_baseline": closed.final_baseline,
                "update_l1_sum": closed.update_l1_sum,
                "max_raw_identity_error": closed.max_raw_identity_error,
            }));
        }
    }

    assert!(measured_ticks <= plan.budget.maximum_total_ticks);
    let audit = serde_json::json!({
        "schema_version": 1,
        "task": "M4-08",
        "audit_plan": AUDIT_PLAN_PATH,
        "audit_plan_sha256": audit_sha256,
        "m4_07_manifest_sha256_at_audit": sha256_file(M4_07_PLAN_PATH),
        "config_sha256": {
            "episodic_b4": config_sha256(&episodic_cfg),
            "event_reset_b4": config_sha256(&event_cfg),
            "continuous_b4": config_sha256(&persistent_cfg),
            "continuous_b3": config_sha256(&b3_cfg),
            "continuous_b4_tau16": config_sha256(&tau16_cfg),
            "continuous_b4_tau64": config_sha256(&tau64_cfg),
        },
        "legs": legs,
        "archive_cross_check": cross_check,
        "probes": probes,
        "measured_budget": {
            "lifetimes": legs.len(),
            "outcomes": legs.iter().map(|leg| leg.outcomes).sum::<u64>(),
            "ticks": measured_ticks,
            "maximum_declared_ticks": plan.budget.maximum_total_ticks,
        },
        "pairing": "identical W0 and exogenous event/cue/commit/feedback/noise schedule across all outer-2 legs including both tau_e variants",
        "passes": null,
        "provenance": {
            "revision": command_output("git", &["rev-parse", "HEAD"]),
            "dirty": !command_output("git", &["status", "--porcelain"]).is_empty(),
            "git_status": command_output("git", &["status", "--porcelain"]),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "profile": "release",
        },
    });
    std::fs::write(
        out_dir.join("audit.json"),
        serde_json::to_string_pretty(&audit).expect("audit serializes"),
    )
    .expect("audit write");
    eprintln!("M4-08 audit legs: {}", legs.len());
}
