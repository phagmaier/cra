//! M4-07 declared continuous acquisition comparison (spec 14.3, 16/M4).
//!
//! The checked-in manifest freezes the actor family, development seeds,
//! per-cue exposure windows, matched-control criterion, diagnostics, and
//! finite compute budget before the ignored empirical test is executed.
//! Fast tests validate the declaration and exercise all five real runners on
//! a small paired fixture. The ignored release test writes immutable-style
//! JSONL records plus a machine-readable verdict to a fresh directory; it
//! records a negative result just as faithfully as a positive one.

use cra::agent::health::HealthSummary;
use cra::agent::no_learning::NoLearningActor;
use cra::agent::plasticity::FeedbackOutcome;
use cra::config::{
    Config, load_and_validate, resolved_toml, validate_actor_no_learning_execution,
    validate_continuous_execution, validate_episodic_execution, validate_event_reset_execution,
};
use cra::experiments::baseline::{BaselineSummary, run_actor_ordinary};
use cra::experiments::continuous::{
    CONTINUOUS_MODE, ContinuousSummary, EVENT_RESET_MODE, run_continuous_lifetime,
    run_event_reset_lifetime,
};
use cra::experiments::episodic::{
    EPISODIC_MODE, EpisodicNoLearningSummary, EpisodicSummary, run_episodic_lifetime,
    run_episodic_no_learning,
};
use cra::experiments::grid::load as load_m3_grid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::path::PathBuf;

const PLAN_PATH: &str = "manifests/m4_continuous_acquisition.json";

// Reuse the frozen M4-07 configuration builders and analyzers verbatim.
// The escape sweep has its own manifest, tests, export, and output path.
#[path = "support/m4_escape_sweep.rs"]
mod escape;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcquisitionPlan {
    schema_version: u32,
    task: String,
    purpose: String,
    base_profile: PathBuf,
    namespace: String,
    root_seed: u64,
    outer_seeds: Vec<u64>,
    lifetime_indices: Vec<u64>,
    outcomes_per_lifetime: u64,
    actor_family: ActorFamily,
    conditions: Vec<ConditionPlan>,
    acquisition_windows: WindowPlan,
    primary_metric: String,
    criterion: Criterion,
    reported_diagnostics: Vec<String>,
    on_no_pass: String,
    budget: Budget,
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
struct ConditionPlan {
    id: String,
    continuity: String,
    learning: String,
    matched_control: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowPlan {
    unit: String,
    early_exposures: usize,
    late_exposures: usize,
    aggregation: String,
    minimum_exposures_per_cue: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Criterion {
    min_continuous_late_accuracy: f64,
    min_continuous_minus_matched_b3: f64,
    min_outer_seeds_passing: usize,
    max_failed_lifetimes: usize,
    max_clipped_update_fraction: f64,
    max_plastic_bound_occupancy: f64,
    require_continuous_p_movement: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    outer_seeds: usize,
    conditions_per_seed: usize,
    total_lifetimes: usize,
    total_outcomes: u64,
    maximum_ticks_per_lifetime: u64,
    maximum_total_ticks: u64,
    execution: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct CueWindowStats {
    cue: usize,
    exposures: usize,
    early_accuracy: f64,
    early_reward: f64,
    late_accuracy: f64,
    late_reward: f64,
    full_accuracy: f64,
    full_reward: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct BehaviorStats {
    early_macro_accuracy: f64,
    early_macro_reward: f64,
    late_macro_accuracy: f64,
    late_macro_reward: f64,
    full_macro_accuracy: f64,
    full_macro_reward: f64,
    per_cue: Vec<CueWindowStats>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct DistributionStats {
    mean: f64,
    p95: f64,
    max: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct UpdateStats {
    raw_l1: DistributionStats,
    limited_l1: DistributionStats,
    actual_l1: DistributionStats,
    clipped_update_fraction: f64,
    clipped_updates: u64,
    nonzero_updates: u64,
    plastic_bound_occupancy: f64,
    final_p_l1: f64,
    final_p_l2: f64,
    final_e_l1: f64,
    final_e_l2: f64,
    final_baseline: f64,
}

fn load_plan() -> AcquisitionPlan {
    let bytes = std::fs::read(PLAN_PATH).expect("M4-07 plan exists");
    serde_json::from_slice(&bytes).expect("M4-07 plan parses")
}

fn configured_family(plan: &AcquisitionPlan) -> Config {
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

fn persistent_config(plan: &AcquisitionPlan) -> Config {
    let mut cfg = configured_family(plan);
    cfg.profile_name = "m4_07_continuous_persistent".to_owned();
    cfg.simulation.reset_policy = "birth_only".to_owned();
    cfg.learning.as_mut().expect("learning").trace_policy = "persistent".to_owned();
    validate_continuous_execution(&cfg).expect("persistent config executes");
    cfg
}

fn event_reset_config(plan: &AcquisitionPlan) -> Config {
    let mut cfg = configured_family(plan);
    cfg.profile_name = "m4_07_event_reset_diagnostic".to_owned();
    cfg.simulation.reset_policy = "event_reset_diagnostic".to_owned();
    cfg.learning.as_mut().expect("learning").trace_policy = "persistent".to_owned();
    validate_event_reset_execution(&cfg).expect("event-reset config executes");
    cfg
}

fn episodic_config(plan: &AcquisitionPlan) -> Config {
    let mut cfg = configured_family(plan);
    cfg.profile_name = "m4_07_episodic_diagnostic".to_owned();
    cfg.simulation.reset_policy = "episodic_diagnostic".to_owned();
    cfg.learning.as_mut().expect("learning").trace_policy = "no_decay_diagnostic".to_owned();
    validate_episodic_execution(&cfg).expect("episodic config executes");
    cfg
}

fn nonplastic_config(plan: &AcquisitionPlan) -> Config {
    let mut cfg = persistent_config(plan);
    cfg.profile_name = "m4_07_continuous_b3".to_owned();
    cfg.learning.as_mut().expect("learning").enabled = false;
    validate_actor_no_learning_execution(&cfg).expect("continuous B3 config executes");
    cfg
}

fn behavior_stats(
    cue_count: usize,
    rows: &[(usize, bool, f64)],
    windows: &WindowPlan,
) -> BehaviorStats {
    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len() as f64;
    let mut per_cue = Vec::with_capacity(cue_count);
    for cue in 0..cue_count {
        let cue_rows: Vec<_> = rows.iter().filter(|row| row.0 == cue).collect();
        assert!(
            cue_rows.len() >= windows.minimum_exposures_per_cue,
            "cue {cue} has {} exposures, needs {}",
            cue_rows.len(),
            windows.minimum_exposures_per_cue
        );
        assert!(
            windows.early_exposures + windows.late_exposures <= cue_rows.len(),
            "cue {cue} early/late windows overlap"
        );
        let summarize = |slice: &[&(usize, bool, f64)]| {
            let accuracy = slice.iter().filter(|row| row.1).count() as f64 / slice.len() as f64;
            let reward = slice.iter().map(|row| row.2).sum::<f64>() / slice.len() as f64;
            (accuracy, reward)
        };
        let (early_accuracy, early_reward) = summarize(&cue_rows[..windows.early_exposures]);
        let (late_accuracy, late_reward) =
            summarize(&cue_rows[cue_rows.len() - windows.late_exposures..]);
        let (full_accuracy, full_reward) = summarize(&cue_rows);
        per_cue.push(CueWindowStats {
            cue,
            exposures: cue_rows.len(),
            early_accuracy,
            early_reward,
            late_accuracy,
            late_reward,
            full_accuracy,
            full_reward,
        });
    }
    BehaviorStats {
        early_macro_accuracy: mean(&per_cue.iter().map(|s| s.early_accuracy).collect::<Vec<_>>()),
        early_macro_reward: mean(&per_cue.iter().map(|s| s.early_reward).collect::<Vec<_>>()),
        late_macro_accuracy: mean(&per_cue.iter().map(|s| s.late_accuracy).collect::<Vec<_>>()),
        late_macro_reward: mean(&per_cue.iter().map(|s| s.late_reward).collect::<Vec<_>>()),
        full_macro_accuracy: mean(&per_cue.iter().map(|s| s.full_accuracy).collect::<Vec<_>>()),
        full_macro_reward: mean(&per_cue.iter().map(|s| s.full_reward).collect::<Vec<_>>()),
        per_cue,
    }
}

fn distribution(mut values: Vec<f64>) -> DistributionStats {
    assert!(!values.is_empty());
    assert!(values.iter().all(|v| v.is_finite() && *v >= 0.0));
    values.sort_by(f64::total_cmp);
    let p95 = ((values.len() * 95).div_ceil(100)).saturating_sub(1);
    DistributionStats {
        mean: values.iter().sum::<f64>() / values.len() as f64,
        p95: values[p95],
        max: values[values.len() - 1],
    }
}

fn l1(matrix: &[Vec<f64>]) -> f64 {
    matrix.iter().flatten().map(|v| v.abs()).sum()
}

fn l2(matrix: &[Vec<f64>]) -> f64 {
    matrix.iter().flatten().map(|v| v * v).sum::<f64>().sqrt()
}

fn update_stats<'a>(
    updates: impl Iterator<Item = &'a FeedbackOutcome>,
    final_p: &[Vec<f64>],
    final_e: &[Vec<f64>],
    final_baseline: f64,
    bound: f64,
) -> UpdateStats {
    let updates: Vec<_> = updates.collect();
    let mut raw = Vec::with_capacity(updates.len());
    let mut limited = Vec::with_capacity(updates.len());
    let mut actual = Vec::with_capacity(updates.len());
    let mut clipped = 0u64;
    let mut nonzero = 0u64;
    for update in updates {
        raw.push(l1(&update.raw_updates));
        limited.push(l1(&update.limited_updates));
        actual.push(l1(&update.actual_updates));
        for (raw_row, limited_row) in update.raw_updates.iter().zip(update.limited_updates.iter()) {
            for (&r, &limited) in raw_row.iter().zip(limited_row.iter()) {
                if r != 0.0 {
                    nonzero += 1;
                    if r != limited {
                        clipped += 1;
                    }
                }
            }
        }
    }
    let entries = final_p.iter().flatten().count();
    let at_bound = final_p
        .iter()
        .flatten()
        .filter(|value| value.abs() == bound)
        .count();
    UpdateStats {
        raw_l1: distribution(raw),
        limited_l1: distribution(limited),
        actual_l1: distribution(actual),
        clipped_update_fraction: if nonzero == 0 {
            0.0
        } else {
            clipped as f64 / nonzero as f64
        },
        clipped_updates: clipped,
        nonzero_updates: nonzero,
        plastic_bound_occupancy: at_bound as f64 / entries as f64,
        final_p_l1: l1(final_p),
        final_p_l2: l2(final_p),
        final_e_l1: l1(final_e),
        final_e_l2: l2(final_e),
        final_baseline,
    }
}

fn continuous_rows(summary: &ContinuousSummary) -> Vec<(usize, bool, f64)> {
    summary
        .choices
        .iter()
        .map(|c| (c.cue, c.correct, c.reward))
        .collect()
}

fn episodic_rows(summary: &EpisodicSummary) -> Vec<(usize, bool, f64)> {
    summary
        .choices
        .iter()
        .map(|c| (c.cue, c.correct, c.reward))
        .collect()
}

fn episodic_b3_rows(summary: &EpisodicNoLearningSummary) -> Vec<(usize, bool, f64)> {
    summary
        .choices
        .iter()
        .map(|c| (c.cue, c.correct, c.reward))
        .collect()
}

fn baseline_rows(summary: &BaselineSummary) -> Vec<(usize, bool, f64)> {
    summary
        .choices
        .iter()
        .map(|c| (c.cue, c.correct, c.reward))
        .collect()
}

fn schedule_from_continuous(summary: &ContinuousSummary) -> Vec<(u64, usize, u64, u64, bool)> {
    summary
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
        .collect()
}

fn schedule_from_episodic(summary: &EpisodicSummary) -> Vec<(u64, usize, u64, u64, bool)> {
    summary
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
        .collect()
}

fn schedule_from_episodic_b3(
    summary: &EpisodicNoLearningSummary,
) -> Vec<(u64, usize, u64, u64, bool)> {
    summary
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
        .collect()
}

fn schedule_from_baseline(summary: &BaselineSummary) -> Vec<(u64, usize, u64, u64, bool)> {
    summary
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
        .collect()
}

fn health_json(health: &HealthSummary) -> serde_json::Value {
    serde_json::json!({
        "ticks_observed": health.ticks_observed,
        "max_abs_h": health.max_abs_h,
        "max_abs_a": health.max_abs_a,
        "max_abs_r": health.max_abs_r,
        "actor_saturated_neuron_tick_fraction": health.saturated_fraction(),
        "max_abs_motor_filter": health.max_abs_q,
        "min_motor_margin": health.min_margin(),
        "max_motor_margin": health.max_motor_margin,
    })
}

fn config_sha256(cfg: &Config) -> String {
    let resolved = resolved_toml(cfg).expect("resolved config serializes");
    format!("{:x}", Sha256::digest(resolved.as_bytes()))
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

#[test]
fn declaration_is_self_consistent_and_pins_the_m3_actor_family() {
    let plan = load_plan();
    assert_eq!(plan.schema_version, 1);
    assert_eq!(plan.task, "M4-07");
    assert!(
        plan.purpose
            .contains("before any M4-07 acquisition outcome")
    );
    assert_eq!(plan.namespace, "development");
    assert_eq!(plan.outer_seeds, vec![1, 2, 3]);
    assert_eq!(plan.lifetime_indices, vec![0]);
    assert_eq!(plan.outcomes_per_lifetime, 2_000);
    assert!(!plan.on_no_pass.trim().is_empty());
    assert_eq!(plan.primary_metric, "late_per_cue_macro_latent_accuracy");
    assert_eq!(plan.acquisition_windows.unit, "exposures_per_cue");
    assert_eq!(
        plan.acquisition_windows.aggregation,
        "equal-weight macro mean across cues"
    );
    assert_eq!(plan.acquisition_windows.early_exposures, 100);
    assert_eq!(plan.acquisition_windows.late_exposures, 100);
    assert_eq!(plan.acquisition_windows.minimum_exposures_per_cue, 200);
    assert_eq!(plan.reported_diagnostics.len(), 10);

    let ids: Vec<_> = plan.conditions.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "episodic_b4",
            "episodic_b3",
            "event_reset_b4",
            "continuous_b4",
            "continuous_b3"
        ]
    );
    assert_eq!(plan.conditions[0].continuity, "episodic_diagnostic");
    assert_eq!(plan.conditions[0].learning, "fixed_gate_plastic");
    assert_eq!(
        plan.conditions[0].matched_control.as_deref(),
        Some("episodic_b3")
    );
    assert_eq!(
        plan.conditions[2].matched_control.as_deref(),
        Some("continuous_b3")
    );
    assert_eq!(
        plan.conditions[3].matched_control.as_deref(),
        Some("continuous_b3")
    );
    assert_eq!(plan.conditions[4].learning, "disabled");
    assert_eq!(plan.conditions[4].matched_control, None);

    let m3 = load_m3_grid(std::path::Path::new("manifests/m3_development_grid.json"))
        .expect("M3 grid loads");
    let winner = m3
        .combination(plan.actor_family.m3_grid_index)
        .expect("declared M3 winner exists");
    assert!(plan.actor_family.source.contains("M3-08"));
    assert_eq!(winner.eta, plan.actor_family.eta);
    assert_eq!(winner.input_scale, plan.actor_family.input_scale);
    assert_eq!(winner.recurrent_gain, plan.actor_family.recurrent_gain);
    assert_eq!(winner.noise_sigma, plan.actor_family.noise_sigma);
    assert_eq!(plan.actor_family.plastic_mask, "all_recurrent_edges");
    assert_eq!(plan.actor_family.tau_e, 32.0);

    let persistent = persistent_config(&plan);
    let event = event_reset_config(&plan);
    let episodic = episodic_config(&plan);
    let b3 = nonplastic_config(&plan);
    assert_eq!(persistent.environment.kind, "stationary_clean");
    assert_eq!(persistent.environment.feedback_noise_values, vec![0.0]);
    assert_eq!(persistent.environment.volatile_hazard_values, vec![0.0]);
    assert_eq!(persistent.environment.max_pending_choices, 1);
    assert_eq!(persistent.simulation.reset_policy, "birth_only");
    assert_eq!(event.simulation.reset_policy, "event_reset_diagnostic");
    assert_eq!(episodic.simulation.reset_policy, "episodic_diagnostic");
    assert_eq!(b3.simulation.reset_policy, "birth_only");
    assert!(!b3.learning.as_ref().expect("learning").enabled);

    let cycle_max = persistent.environment.quiet_ticks[1]
        + persistent.environment.cue_ticks
        + persistent.environment.memory_gap_ticks[1]
        + persistent.environment.response_ticks
        + persistent.environment.reward_delay_ticks[1];
    let first_cycle = persistent.simulation.warmup_ticks
        + persistent.environment.cue_ticks
        + persistent.environment.memory_gap_ticks[1]
        + persistent.environment.response_ticks
        + persistent.environment.reward_delay_ticks[1];
    let max_ticks = first_cycle + (plan.outcomes_per_lifetime - 1) * cycle_max;
    assert_eq!(max_ticks, plan.budget.maximum_ticks_per_lifetime);
    assert_eq!(plan.budget.outer_seeds, plan.outer_seeds.len());
    assert_eq!(plan.budget.conditions_per_seed, plan.conditions.len());
    assert_eq!(
        plan.budget.total_lifetimes,
        plan.outer_seeds.len() * plan.conditions.len()
    );
    assert_eq!(
        plan.budget.total_outcomes,
        plan.outcomes_per_lifetime * plan.budget.total_lifetimes as u64
    );
    assert_eq!(
        plan.budget.maximum_total_ticks,
        plan.budget.maximum_ticks_per_lifetime * plan.budget.total_lifetimes as u64
    );
    assert!(plan.budget.execution.contains("serial release-mode"));
    assert_eq!(plan.criterion.min_continuous_late_accuracy, 0.70);
    assert_eq!(plan.criterion.min_continuous_minus_matched_b3, 0.15);
    assert_eq!(plan.criterion.min_outer_seeds_passing, 2);
}

#[test]
fn all_five_conditions_run_paired_and_report_required_diagnostics() {
    let mut plan = load_plan();
    plan.outcomes_per_lifetime = 40;
    plan.acquisition_windows.early_exposures = 2;
    plan.acquisition_windows.late_exposures = 2;
    plan.acquisition_windows.minimum_exposures_per_cue = 4;
    let persistent_cfg = persistent_config(&plan);
    let event_cfg = event_reset_config(&plan);
    let episodic_cfg = episodic_config(&plan);
    let b3_cfg = nonplastic_config(&plan);
    let root = plan.root_seed;
    let ns = &plan.namespace;
    let outer = plan.outer_seeds[0];
    let lifetime = plan.lifetime_indices[0];

    let episodic = run_episodic_lifetime(&episodic_cfg, root, ns, outer, lifetime, "episodic_b4")
        .expect("episodic B4");
    let episodic_b3 =
        run_episodic_no_learning(&episodic_cfg, root, ns, outer, lifetime, "episodic_b3")
            .expect("episodic B3");
    let event = run_event_reset_lifetime(&event_cfg, root, ns, outer, lifetime, "event_reset_b4")
        .expect("event-reset B4");
    let continuous =
        run_continuous_lifetime(&persistent_cfg, root, ns, outer, lifetime, "continuous_b4")
            .expect("continuous B4");
    let mut b3 = NoLearningActor::new(&b3_cfg, root, ns, outer, lifetime).expect("continuous B3");
    let b3_w0 = b3.inherited().weights.w0.clone();
    let continuous_b3 =
        run_actor_ordinary(&b3_cfg, root, ns, outer, lifetime, "continuous_b3", &mut b3)
            .expect("continuous B3 run");

    assert_eq!(episodic.mode, EPISODIC_MODE);
    assert_eq!(event.mode, EVENT_RESET_MODE);
    assert_eq!(continuous.mode, CONTINUOUS_MODE);
    assert_eq!(episodic.resets.len(), 40);
    assert_eq!(episodic_b3.resets, episodic.resets);
    assert_eq!(event.resets.len(), 40);
    assert_eq!(continuous.resets, vec![0]);
    assert_eq!(episodic.w0, event.w0);
    assert_eq!(event.w0, continuous.w0);
    assert_eq!(continuous.w0, b3_w0);

    let expected = schedule_from_continuous(&continuous);
    assert_eq!(schedule_from_episodic(&episodic), expected);
    assert_eq!(schedule_from_episodic_b3(&episodic_b3), expected);
    assert_eq!(schedule_from_continuous(&event), expected);
    assert_eq!(schedule_from_baseline(&continuous_b3), expected);
    assert_eq!(episodic.choices[0].action, continuous.choices[0].action);
    assert_eq!(
        continuous.choices[0].action,
        continuous_b3.choices[0].action
    );

    let cue_count = persistent_cfg.environment.cue_count;
    for stats in [
        behavior_stats(
            cue_count,
            &episodic_rows(&episodic),
            &plan.acquisition_windows,
        ),
        behavior_stats(
            cue_count,
            &episodic_b3_rows(&episodic_b3),
            &plan.acquisition_windows,
        ),
        behavior_stats(
            cue_count,
            &continuous_rows(&event),
            &plan.acquisition_windows,
        ),
        behavior_stats(
            cue_count,
            &continuous_rows(&continuous),
            &plan.acquisition_windows,
        ),
        behavior_stats(
            cue_count,
            &baseline_rows(&continuous_b3),
            &plan.acquisition_windows,
        ),
    ] {
        assert_eq!(stats.per_cue.len(), cue_count);
        assert!((0.0..=1.0).contains(&stats.late_macro_accuracy));
    }

    let bound = persistent_cfg
        .learning
        .as_ref()
        .expect("learning")
        .plastic_bound;
    for (updates, health) in [
        (
            update_stats(
                episodic.choices.iter().map(|c| &c.update),
                &episodic.final_p,
                &episodic.final_e,
                episodic.final_baseline,
                bound,
            ),
            &episodic.health,
        ),
        (
            update_stats(
                event.choices.iter().map(|c| &c.update),
                &event.final_p,
                &event.final_e,
                event.final_baseline,
                bound,
            ),
            &event.health,
        ),
        (
            update_stats(
                continuous.choices.iter().map(|c| &c.update),
                &continuous.final_p,
                &continuous.final_e,
                continuous.final_baseline,
                bound,
            ),
            &continuous.health,
        ),
    ] {
        assert!(updates.raw_l1.mean.is_finite());
        assert!(updates.actual_l1.max.is_finite());
        assert!((0.0..=1.0).contains(&updates.clipped_update_fraction));
        assert!((0.0..=1.0).contains(&updates.plastic_bound_occupancy));
        assert_eq!(health.ticks_observed, continuous.ticks);
        assert!(health.max_abs_q.is_finite());
        assert!(health.saturated_fraction().is_some());
    }
    let b3_health = continuous_b3.health.as_ref().expect("B3 health present");
    assert_eq!(b3_health.ticks_observed, continuous.ticks);
    assert!(health_json(b3_health)["max_abs_motor_filter"].is_number());
}

#[test]
#[ignore = "one representative 2,000-outcome release lifetime for the predeclared M4-07 throughput check; inspect timing only, not behavior"]
fn m4_continuous_acquisition_benchmark() {
    let plan = load_plan();
    let cfg = persistent_config(&plan);
    let start = std::time::Instant::now();
    let summary = run_continuous_lifetime(
        &cfg,
        plan.root_seed,
        &plan.namespace,
        plan.outer_seeds[0],
        plan.lifetime_indices[0],
        "continuous_b4_benchmark",
    )
    .expect("representative lifetime completes");
    let elapsed = start.elapsed();
    assert_eq!(summary.outcomes, plan.outcomes_per_lifetime);
    assert!(summary.ticks <= plan.budget.maximum_ticks_per_lifetime);
    eprintln!(
        "M4-07 benchmark: outcomes={} ticks={} elapsed_seconds={:.6}",
        summary.outcomes,
        summary.ticks,
        elapsed.as_secs_f64()
    );
}

#[test]
#[ignore = "bounded M4-07 comparison (3 outers x 5 continuity/control conditions x 2,000 outcomes); invoke explicitly in release with CRA_M4_ACQUISITION_DIR"]
fn m4_continuous_acquisition_comparison() {
    let out_dir = std::env::var_os("CRA_M4_ACQUISITION_DIR")
        .map(PathBuf::from)
        .expect("set CRA_M4_ACQUISITION_DIR to a fresh path");
    std::fs::create_dir(&out_dir).expect("evidence directory must be fresh");
    let plan = load_plan();
    let plan_bytes = std::fs::read(PLAN_PATH).expect("plan readable");
    let plan_sha256 = format!("{:x}", Sha256::digest(&plan_bytes));
    let persistent_cfg = persistent_config(&plan);
    let event_cfg = event_reset_config(&plan);
    let episodic_cfg = episodic_config(&plan);
    let b3_cfg = nonplastic_config(&plan);
    let bound = persistent_cfg
        .learning
        .as_ref()
        .expect("learning")
        .plastic_bound;
    let config_hashes = serde_json::json!({
        "episodic_b4_and_b3": config_sha256(&episodic_cfg),
        "event_reset_b4": config_sha256(&event_cfg),
        "continuous_b4": config_sha256(&persistent_cfg),
        "continuous_b3": config_sha256(&b3_cfg),
    });
    let records_path = out_dir.join("records.jsonl");
    let mut records = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&records_path)
        .expect("records file must be fresh");

    let mut measured_lifetimes = 0usize;
    let mut measured_outcomes = 0u64;
    let mut measured_ticks = 0u64;
    let mut failures = Vec::new();
    let mut seed_verdicts = Vec::new();

    for &outer in &plan.outer_seeds {
        let lifetime = plan.lifetime_indices[0];
        let episodic = run_episodic_lifetime(
            &episodic_cfg,
            plan.root_seed,
            &plan.namespace,
            outer,
            lifetime,
            "episodic_b4",
        )
        .map_err(|e| e.to_string());
        let episodic_b3 = run_episodic_no_learning(
            &episodic_cfg,
            plan.root_seed,
            &plan.namespace,
            outer,
            lifetime,
            "episodic_b3",
        )
        .map_err(|e| e.to_string());
        let event = run_event_reset_lifetime(
            &event_cfg,
            plan.root_seed,
            &plan.namespace,
            outer,
            lifetime,
            "event_reset_b4",
        )
        .map_err(|e| e.to_string());
        let continuous = run_continuous_lifetime(
            &persistent_cfg,
            plan.root_seed,
            &plan.namespace,
            outer,
            lifetime,
            "continuous_b4",
        )
        .map_err(|e| e.to_string());
        let continuous_b3 = (|| -> Result<(BaselineSummary, Vec<Vec<f64>>), String> {
            let mut actor =
                NoLearningActor::new(&b3_cfg, plan.root_seed, &plan.namespace, outer, lifetime)
                    .map_err(|e| e.to_string())?;
            let w0 = actor.inherited().weights.w0.clone();
            let summary = run_actor_ordinary(
                &b3_cfg,
                plan.root_seed,
                &plan.namespace,
                outer,
                lifetime,
                "continuous_b3",
                &mut actor,
            )
            .map_err(|e| e.to_string())?;
            Ok((summary, w0))
        })();

        for (condition, failure) in [
            ("episodic_b4", episodic.as_ref().err()),
            ("episodic_b3", episodic_b3.as_ref().err()),
            ("event_reset_b4", event.as_ref().err()),
            ("continuous_b4", continuous.as_ref().err()),
            ("continuous_b3", continuous_b3.as_ref().err()),
        ] {
            if let Some(message) = failure {
                let failure = serde_json::json!({
                    "task": "M4-07", "outer_seed": outer, "condition": condition,
                    "status": "failed", "failure": message,
                });
                writeln!(records, "{failure}").expect("failure record append");
                failures.push(failure);
            }
        }
        let (Ok(episodic), Ok(episodic_b3), Ok(event), Ok(continuous), Ok((continuous_b3, b3_w0))) =
            (episodic, episodic_b3, event, continuous, continuous_b3)
        else {
            continue;
        };

        let expected_schedule = schedule_from_continuous(&continuous);
        assert_eq!(schedule_from_episodic(&episodic), expected_schedule);
        assert_eq!(schedule_from_episodic_b3(&episodic_b3), expected_schedule);
        assert_eq!(schedule_from_continuous(&event), expected_schedule);
        assert_eq!(schedule_from_baseline(&continuous_b3), expected_schedule);
        assert_eq!(episodic.w0, event.w0);
        assert_eq!(event.w0, continuous.w0);
        assert_eq!(continuous.w0, b3_w0);
        assert_eq!(episodic.resets.len(), plan.outcomes_per_lifetime as usize);
        assert_eq!(episodic_b3.resets, episodic.resets);
        assert_eq!(event.resets.len(), plan.outcomes_per_lifetime as usize);
        assert_eq!(continuous.resets, vec![0]);

        let cue_count = persistent_cfg.environment.cue_count;
        let ep_behavior = behavior_stats(
            cue_count,
            &episodic_rows(&episodic),
            &plan.acquisition_windows,
        );
        let ep_b3_behavior = behavior_stats(
            cue_count,
            &episodic_b3_rows(&episodic_b3),
            &plan.acquisition_windows,
        );
        let event_behavior = behavior_stats(
            cue_count,
            &continuous_rows(&event),
            &plan.acquisition_windows,
        );
        let continuous_behavior = behavior_stats(
            cue_count,
            &continuous_rows(&continuous),
            &plan.acquisition_windows,
        );
        let b3_behavior = behavior_stats(
            cue_count,
            &baseline_rows(&continuous_b3),
            &plan.acquisition_windows,
        );
        let ep_updates = update_stats(
            episodic.choices.iter().map(|c| &c.update),
            &episodic.final_p,
            &episodic.final_e,
            episodic.final_baseline,
            bound,
        );
        let event_updates = update_stats(
            event.choices.iter().map(|c| &c.update),
            &event.final_p,
            &event.final_e,
            event.final_baseline,
            bound,
        );
        let continuous_updates = update_stats(
            continuous.choices.iter().map(|c| &c.update),
            &continuous.final_p,
            &continuous.final_e,
            continuous.final_baseline,
            bound,
        );
        let b3_health = continuous_b3.health.as_ref().expect("B3 health recorded");

        let write_record = |file: &mut std::fs::File,
                            condition: &str,
                            matched_control: Option<&str>,
                            ticks: u64,
                            resets: serde_json::Value,
                            behavior: &BehaviorStats,
                            updates: Option<&UpdateStats>,
                            health: &HealthSummary| {
            let record = serde_json::json!({
                "task": "M4-07",
                "status": "complete",
                "root_seed": plan.root_seed,
                "namespace": plan.namespace,
                "outer_seed": outer,
                "lifetime_index": lifetime,
                "condition": condition,
                "matched_control": matched_control,
                "outcomes": plan.outcomes_per_lifetime,
                "ticks": ticks,
                "resets": resets,
                "behavior": behavior,
                "updates": updates,
                "health": health_json(health),
            });
            writeln!(file, "{record}").expect("condition record append");
        };
        write_record(
            &mut records,
            "episodic_b4",
            Some("episodic_b3"),
            episodic.ticks,
            serde_json::json!(episodic.resets),
            &ep_behavior,
            Some(&ep_updates),
            &episodic.health,
        );
        write_record(
            &mut records,
            "episodic_b3",
            None,
            episodic_b3.ticks,
            serde_json::json!(episodic_b3.resets),
            &ep_b3_behavior,
            None,
            &episodic_b3.health,
        );
        write_record(
            &mut records,
            "event_reset_b4",
            Some("continuous_b3"),
            event.ticks,
            serde_json::json!(event.resets),
            &event_behavior,
            Some(&event_updates),
            &event.health,
        );
        write_record(
            &mut records,
            "continuous_b4",
            Some("continuous_b3"),
            continuous.ticks,
            serde_json::json!(continuous.resets),
            &continuous_behavior,
            Some(&continuous_updates),
            &continuous.health,
        );
        write_record(
            &mut records,
            "continuous_b3",
            None,
            continuous_b3
                .choices
                .last()
                .map_or(0, |c| c.feedback_tick + 1),
            serde_json::json!([0]),
            &b3_behavior,
            None,
            b3_health,
        );

        measured_lifetimes += 5;
        measured_outcomes += plan.outcomes_per_lifetime * 5;
        measured_ticks += episodic.ticks
            + episodic_b3.ticks
            + event.ticks
            + continuous.ticks
            + b3_health.ticks_observed;
        let margin = continuous_behavior.late_macro_accuracy - b3_behavior.late_macro_accuracy;
        let passes = continuous_behavior.late_macro_accuracy
            >= plan.criterion.min_continuous_late_accuracy
            && margin >= plan.criterion.min_continuous_minus_matched_b3
            && continuous_updates.clipped_update_fraction
                <= plan.criterion.max_clipped_update_fraction
            && continuous_updates.plastic_bound_occupancy
                <= plan.criterion.max_plastic_bound_occupancy
            && (!plan.criterion.require_continuous_p_movement
                || continuous_updates.final_p_l1 > 0.0);
        seed_verdicts.push(serde_json::json!({
            "outer_seed": outer,
            "continuous_late_macro_accuracy": continuous_behavior.late_macro_accuracy,
            "continuous_b3_late_macro_accuracy": b3_behavior.late_macro_accuracy,
            "continuous_minus_b3_margin": margin,
            "continuous_clipped_update_fraction": continuous_updates.clipped_update_fraction,
            "continuous_bound_occupancy": continuous_updates.plastic_bound_occupancy,
            "continuous_final_p_l1": continuous_updates.final_p_l1,
            "passes": passes,
        }));
    }

    let seeds_passing = seed_verdicts
        .iter()
        .filter(|value| value["passes"] == true)
        .count();
    let passes = failures.len() <= plan.criterion.max_failed_lifetimes
        && measured_lifetimes == plan.budget.total_lifetimes
        && measured_outcomes == plan.budget.total_outcomes
        && measured_ticks <= plan.budget.maximum_total_ticks
        && seeds_passing >= plan.criterion.min_outer_seeds_passing;
    let verdict = serde_json::json!({
        "schema_version": 1,
        "task": "M4-07",
        "plan": PLAN_PATH,
        "plan_sha256": plan_sha256,
        "config_sha256": config_hashes,
        "primary_metric": plan.primary_metric,
        "criterion": plan.criterion,
        "seed_verdicts": seed_verdicts,
        "seeds_passing": seeds_passing,
        "seeds_total": plan.outer_seeds.len(),
        "failures": failures,
        "measured_budget": {
            "lifetimes": measured_lifetimes,
            "outcomes": measured_outcomes,
            "ticks": measured_ticks,
            "maximum_declared_ticks": plan.budget.maximum_total_ticks,
        },
        "pairing": "identical W0 and exogenous event/cue/commit/feedback/noise schedule across all five conditions within each outer seed",
        "passes": passes,
        "on_no_pass": plan.on_no_pass,
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
        out_dir.join("verdict.json"),
        serde_json::to_string_pretty(&verdict).expect("verdict serializes"),
    )
    .expect("verdict write");
    eprintln!("M4-07 verdict: {verdict}");
    // Execution integrity is asserted; the empirical verdict is recorded
    // and interpreted in the tracker rather than forced to pass here.
    assert!(measured_ticks <= plan.budget.maximum_total_ticks);
}
