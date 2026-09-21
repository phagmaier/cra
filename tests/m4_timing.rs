//! Gradual timing and delayed-outcome development profiles (M4-05;
//! spec 5.7, 7.3, 7.8-7.9, 16/M4, 17.2).
//!
//! The checked-in stages keep a clean stationary mapping and all non-timing
//! settings fixed while moving from deterministic short timing to moderate
//! variability and then the spec-5.7 main timing endpoints. Table fixtures
//! pin both ends of every declared interval. Within a profile, all `tau_e`
//! conditions reuse identical environment seed tuples; their exogenous cue
//! and timing schedules must remain identical even if learning changes their
//! actions. The ignored bounded diagnostic saves observed trace/update scales
//! for every declared `tau_e`; it deliberately asserts no monotonic winner.

use cra::config::{Config, load_and_validate, resolved_toml, validate_continuous_execution};
use cra::experiments::continuous::{ContinuousSummary, run_continuous_lifetime};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::path::PathBuf;

const PLAN_PATH: &str = "manifests/m4_timing_sensitivity.json";
const POLICY: &str = "B4-fixed";

#[derive(Debug, Deserialize)]
struct TimingPlan {
    namespace: String,
    root_seed: u64,
    outer_seeds: Vec<u64>,
    lifetime_indices: Vec<u64>,
    outcomes_per_lifetime: u64,
    tau_e_values: Vec<f64>,
    profiles: Vec<ProfileStage>,
    budget: Budget,
}

#[derive(Debug, Deserialize)]
struct ProfileStage {
    stage: u64,
    path: PathBuf,
    profile_name: String,
    quiet_ticks: [u64; 2],
    cue_ticks: u64,
    memory_gap_ticks: [u64; 2],
    response_ticks: u64,
    reward_delay_ticks: [u64; 2],
    max_ticks_per_lifetime: u64,
}

#[derive(Debug, Deserialize)]
struct Budget {
    profiles: usize,
    tau_e_values: usize,
    outer_seeds: usize,
    total_lifetimes: usize,
    total_outcomes: u64,
    maximum_total_ticks: u64,
}

fn load_plan() -> TimingPlan {
    let bytes = std::fs::read(PLAN_PATH).expect("timing plan exists");
    serde_json::from_slice(&bytes).expect("timing plan parses")
}

fn load_profile(stage: &ProfileStage) -> Config {
    let cfg = load_and_validate(&stage.path).expect("declared profile validates");
    validate_continuous_execution(&cfg).expect("declared profile executes continuously");
    cfg
}

fn l1(matrix: &[Vec<f64>]) -> f64 {
    matrix.iter().flatten().map(|v| v.abs()).sum()
}

type ScheduleRow = (u64, usize, u64, u64, bool);

fn schedule(summary: &ContinuousSummary) -> Vec<ScheduleRow> {
    summary
        .choices
        .iter()
        .map(|choice| {
            (
                choice.event_id,
                choice.cue,
                choice.commit_tick,
                choice.feedback_tick,
                choice.noise_bit,
            )
        })
        .collect()
}

fn normalize_timing(mut cfg: Config, reference: &Config) -> Config {
    cfg.profile_name = reference.profile_name.clone();
    cfg.environment.quiet_ticks = reference.environment.quiet_ticks;
    cfg.environment.cue_ticks = reference.environment.cue_ticks;
    cfg.environment.memory_gap_ticks = reference.environment.memory_gap_ticks;
    cfg.environment.response_ticks = reference.environment.response_ticks;
    cfg.environment.reward_delay_ticks = reference.environment.reward_delay_ticks;
    cfg
}

#[test]
fn profiles_follow_the_declared_gradual_timing_table() {
    let plan = load_plan();
    assert_eq!(plan.profiles.len(), 3);
    assert_eq!(plan.budget.profiles, plan.profiles.len());
    assert_eq!(plan.budget.tau_e_values, plan.tau_e_values.len());
    assert_eq!(plan.budget.outer_seeds, plan.outer_seeds.len());
    assert_eq!(plan.lifetime_indices, vec![0]);
    assert_eq!(plan.tau_e_values, vec![16.0, 32.0, 64.0]);
    assert_eq!(plan.budget.total_lifetimes, 27);
    assert_eq!(plan.budget.total_outcomes, 6_912);
    assert_eq!(plan.budget.maximum_total_ticks, 270_180);

    let base = load_profile(&plan.profiles[0]);
    for (expected_stage, stage) in plan.profiles.iter().enumerate() {
        assert_eq!(stage.stage, expected_stage as u64);
        let cfg = load_profile(stage);
        assert_eq!(cfg.profile_name, stage.profile_name);
        assert_eq!(cfg.environment.quiet_ticks, stage.quiet_ticks);
        assert_eq!(cfg.environment.cue_ticks, stage.cue_ticks);
        assert_eq!(cfg.environment.memory_gap_ticks, stage.memory_gap_ticks);
        assert_eq!(cfg.environment.response_ticks, stage.response_ticks);
        assert_eq!(cfg.environment.reward_delay_ticks, stage.reward_delay_ticks);
        assert_eq!(cfg.environment.kind, "stationary_clean");
        assert_eq!(cfg.environment.stable_fraction, 1.0);
        assert_eq!(cfg.environment.feedback_noise_values, vec![0.0]);
        assert_eq!(cfg.environment.volatile_hazard_values, vec![0.0]);
        assert_eq!(cfg.environment.max_pending_choices, 1);
        assert_eq!(cfg.simulation.reset_policy, "birth_only");
        assert_eq!(cfg.simulation.outcomes_per_lifetime, 2_000);
        assert_eq!(
            normalize_timing(cfg, &base),
            base,
            "stage {expected_stage} may differ only in profile name and declared timing"
        );
    }

    assert_eq!(plan.profiles[0].quiet_ticks, [4, 4]);
    assert_eq!(plan.profiles[0].memory_gap_ticks, [0, 0]);
    assert_eq!(plan.profiles[0].reward_delay_ticks, [1, 1]);
    assert_eq!(plan.profiles[1].quiet_ticks, [4, 8]);
    assert_eq!(plan.profiles[1].memory_gap_ticks, [0, 4]);
    assert_eq!(plan.profiles[1].reward_delay_ticks, [1, 4]);
    assert_eq!(plan.profiles[2].quiet_ticks, [8, 16]);
    assert_eq!(plan.profiles[2].memory_gap_ticks, [0, 8]);
    assert_eq!(plan.profiles[2].reward_delay_ticks, [8, 24]);
}

#[test]
fn every_declared_timing_endpoint_has_exact_cycle_boundaries() {
    let plan = load_plan();
    for stage in &plan.profiles {
        for endpoint in 0..=1 {
            let mut cfg = load_profile(stage);
            let quiet = stage.quiet_ticks[endpoint];
            let gap = stage.memory_gap_ticks[endpoint];
            let delay = stage.reward_delay_ticks[endpoint];
            cfg.simulation.outcomes_per_lifetime = 2;
            cfg.environment.quiet_ticks = [quiet, quiet];
            cfg.environment.memory_gap_ticks = [gap, gap];
            cfg.environment.reward_delay_ticks = [delay, delay];
            let summary = run_continuous_lifetime(
                &cfg,
                plan.root_seed,
                &plan.namespace,
                plan.outer_seeds[0],
                plan.lifetime_indices[0],
                POLICY,
            )
            .expect("endpoint lifetime runs");
            assert_eq!(summary.choices.len(), 2);

            let first_commit =
                cfg.simulation.warmup_ticks + stage.cue_ticks + gap + stage.response_ticks - 1;
            let first_feedback = first_commit + delay;
            let second_commit =
                first_feedback + quiet + stage.cue_ticks + gap + stage.response_ticks;
            let second_feedback = second_commit + delay;
            assert_eq!(summary.choices[0].commit_tick, first_commit);
            assert_eq!(summary.choices[0].feedback_tick, first_feedback);
            assert_eq!(summary.choices[1].commit_tick, second_commit);
            assert_eq!(summary.choices[1].feedback_tick, second_feedback);
            assert_eq!(summary.ticks, second_feedback + 1);
            assert_eq!(summary.commitments, 2);
            assert_eq!(summary.outcomes, 2);
            assert_eq!(summary.resets, vec![0]);
        }
    }
}

#[test]
fn paired_tau_e_conditions_keep_identical_exogenous_schedules() {
    let plan = load_plan();
    for stage in &plan.profiles {
        let mut expected_schedule = None;
        let mut expected_w0 = None;
        for &tau_e in &plan.tau_e_values {
            let mut cfg = load_profile(stage);
            cfg.simulation.outcomes_per_lifetime = 24;
            cfg.learning.as_mut().expect("learning").tau_e = tau_e;
            let summary = run_continuous_lifetime(
                &cfg,
                plan.root_seed,
                &plan.namespace,
                plan.outer_seeds[0],
                plan.lifetime_indices[0],
                POLICY,
            )
            .expect("paired lifetime runs");
            let observed = schedule(&summary);
            if let Some(expected) = &expected_schedule {
                assert_eq!(&observed, expected, "{} tau_e {tau_e}", stage.profile_name);
                assert_eq!(Some(&summary.w0), expected_w0.as_ref());
            } else {
                expected_schedule = Some(observed);
                expected_w0 = Some(summary.w0);
            }
        }
    }
}

#[test]
fn feedback_records_live_trace_and_corresponding_update_scale() {
    let plan = load_plan();
    let stage = &plan.profiles[1];
    let mut cfg = load_profile(stage);
    cfg.simulation.outcomes_per_lifetime = 24;
    let eta = cfg.learning.as_ref().expect("learning").eta;
    let summary = run_continuous_lifetime(
        &cfg,
        plan.root_seed,
        &plan.namespace,
        plan.outer_seeds[0],
        plan.lifetime_indices[0],
        POLICY,
    )
    .expect("measurement lifetime runs");

    assert!(
        summary
            .choices
            .iter()
            .any(|c| c.eligibility_l1_before_update > 0.0)
    );
    for choice in &summary.choices {
        assert!(choice.eligibility_l1_before_update.is_finite());
        assert!(choice.eligibility_l1_before_update >= 0.0);
        let raw_l1 = l1(&choice.update.raw_updates);
        let expected = eta * choice.update.delta.abs() * choice.eligibility_l1_before_update;
        let tolerance = 1e-12 * expected.abs().max(1.0);
        assert!(
            (raw_l1 - expected).abs() <= tolerance,
            "event {} raw {raw_l1} expected {expected}",
            choice.event_id
        );
        let delay = choice.feedback_tick - choice.commit_tick;
        assert!(stage.reward_delay_ticks[0] <= delay && delay <= stage.reward_delay_ticks[1]);
        assert!(l1(&choice.update.limited_updates).is_finite());
        assert!(l1(&choice.update.actual_updates).is_finite());
    }
}

#[test]
#[ignore = "bounded M4-05 timing sensitivity diagnostic (3 profiles x 3 tau_e x 3 outers x 256 outcomes); invoke explicitly in release with CRA_M4_TIMING_DIR"]
fn m4_timing_sensitivity_diagnostic() {
    let out_dir = std::env::var_os("CRA_M4_TIMING_DIR")
        .map(PathBuf::from)
        .expect("set CRA_M4_TIMING_DIR to a fresh path");
    std::fs::create_dir(&out_dir).expect("evidence directory must be fresh");
    let plan = load_plan();
    let plan_bytes = std::fs::read(PLAN_PATH).expect("plan readable");
    let plan_sha256 = format!("{:x}", Sha256::digest(&plan_bytes));
    let records_path = out_dir.join("records.jsonl");
    let mut records = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&records_path)
        .expect("records file must be fresh");

    let mut measured_lifetimes = 0usize;
    let mut measured_outcomes = 0u64;
    let mut measured_ticks = 0u64;
    for stage in &plan.profiles {
        for &outer_seed in &plan.outer_seeds {
            let mut expected_schedule = None;
            for &tau_e in &plan.tau_e_values {
                let mut cfg = load_profile(stage);
                cfg.simulation.outcomes_per_lifetime = plan.outcomes_per_lifetime;
                cfg.learning.as_mut().expect("learning").tau_e = tau_e;
                validate_continuous_execution(&cfg).expect("diagnostic point executes");
                let config_toml = resolved_toml(&cfg).expect("resolved config");
                let config_sha256 = format!("{:x}", Sha256::digest(config_toml.as_bytes()));
                let summary = run_continuous_lifetime(
                    &cfg,
                    plan.root_seed,
                    &plan.namespace,
                    outer_seed,
                    plan.lifetime_indices[0],
                    POLICY,
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "{} outer {outer_seed} tau_e {tau_e}: {e}",
                        stage.profile_name
                    )
                });

                assert_eq!(summary.outcomes, plan.outcomes_per_lifetime);
                assert_eq!(summary.commitments, plan.outcomes_per_lifetime);
                assert_eq!(summary.resets, vec![0]);
                assert!(summary.ticks <= stage.max_ticks_per_lifetime);
                assert_eq!(
                    summary.final_last_feedback,
                    Some(plan.outcomes_per_lifetime - 1)
                );
                assert!(summary.final_p.iter().flatten().all(|v| v.is_finite()));
                assert!(summary.final_e.iter().flatten().all(|v| v.is_finite()));
                let observed_schedule = schedule(&summary);
                if let Some(expected) = &expected_schedule {
                    assert_eq!(
                        &observed_schedule, expected,
                        "paired schedule changed for {} outer {outer_seed} tau_e {tau_e}",
                        stage.profile_name
                    );
                } else {
                    expected_schedule = Some(observed_schedule.clone());
                }
                let schedule_json =
                    serde_json::to_vec(&observed_schedule).expect("schedule serializes");
                let schedule_sha256 = format!("{:x}", Sha256::digest(&schedule_json));

                let traces: Vec<_> = summary
                    .choices
                    .iter()
                    .map(|c| c.eligibility_l1_before_update)
                    .collect();
                let raw: Vec<_> = summary
                    .choices
                    .iter()
                    .map(|c| l1(&c.update.raw_updates))
                    .collect();
                let limited: Vec<_> = summary
                    .choices
                    .iter()
                    .map(|c| l1(&c.update.limited_updates))
                    .collect();
                let actual: Vec<_> = summary
                    .choices
                    .iter()
                    .map(|c| l1(&c.update.actual_updates))
                    .collect();
                let (clipped_updates, nonzero_updates) = clipping_counts(&summary);
                let plastic_bound = cfg.learning.as_ref().expect("learning").plastic_bound;
                let total_entries = summary.final_p.iter().flatten().count();
                let at_bound = summary
                    .final_p
                    .iter()
                    .flatten()
                    .filter(|value| value.abs() == plastic_bound)
                    .count();
                let record = serde_json::json!({
                    "task": "M4-05",
                    "stage": stage.stage,
                    "profile": stage.profile_name,
                    "profile_path": stage.path,
                    "timing": {
                        "quiet_ticks": stage.quiet_ticks,
                        "cue_ticks": stage.cue_ticks,
                        "memory_gap_ticks": stage.memory_gap_ticks,
                        "response_ticks": stage.response_ticks,
                        "reward_delay_ticks": stage.reward_delay_ticks,
                    },
                    "tau_e": tau_e,
                    "delay_retained_factor": {
                        "minimum_delay": (-((stage.reward_delay_ticks[0] as f64) / tau_e)).exp(),
                        "maximum_delay": (-((stage.reward_delay_ticks[1] as f64) / tau_e)).exp(),
                    },
                    "root_seed": plan.root_seed,
                    "namespace": plan.namespace,
                    "outer_seed": outer_seed,
                    "lifetime_index": plan.lifetime_indices[0],
                    "outcomes": summary.outcomes,
                    "ticks": summary.ticks,
                    "schedule_sha256": schedule_sha256,
                    "config_sha256": config_sha256,
                    "pre_feedback_eligibility_l1": stats(&traces),
                    "raw_update_l1": stats(&raw),
                    "limited_update_l1": stats(&limited),
                    "actual_update_l1": stats(&actual),
                    "clipped_update_fraction": if nonzero_updates == 0 { 0.0 } else { clipped_updates as f64 / nonzero_updates as f64 },
                    "bound_occupancy": at_bound as f64 / total_entries as f64,
                    "final_p_l1": l1(&summary.final_p),
                    "final_e_l1": l1(&summary.final_e),
                    "final_baseline": summary.final_baseline,
                    "mean_reward": summary.mean_reward(),
                    "latent_accuracy": summary.latent_accuracy(),
                });
                writeln!(records, "{record}").expect("record append");
                measured_lifetimes += 1;
                measured_outcomes += summary.outcomes;
                measured_ticks += summary.ticks;
            }
        }
    }
    assert_eq!(measured_lifetimes, plan.budget.total_lifetimes);
    assert_eq!(measured_outcomes, plan.budget.total_outcomes);
    assert!(measured_ticks <= plan.budget.maximum_total_ticks);

    let result = serde_json::json!({
        "schema_version": 1,
        "task": "M4-05",
        "plan": PLAN_PATH,
        "plan_sha256": plan_sha256,
        "records": "records.jsonl",
        "measured_budget": {
            "lifetimes": measured_lifetimes,
            "outcomes": measured_outcomes,
            "ticks": measured_ticks,
            "maximum_declared_ticks": plan.budget.maximum_total_ticks,
        },
        "schedule_pairing": "passed for every profile/outer across all tau_e values",
        "interpretation": "descriptive sensitivity only; no monotonic tau_e ordering was asserted",
        "provenance": {
            "revision": command_output("git", &["rev-parse", "HEAD"]),
            "dirty": !command_output("git", &["status", "--porcelain"]).is_empty(),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "profile": "release",
        },
    });
    std::fs::write(
        out_dir.join("result.json"),
        serde_json::to_string_pretty(&result).expect("result serializes"),
    )
    .expect("result write");
    eprintln!("timing sensitivity result: {result}");
}

fn stats(values: &[f64]) -> serde_json::Value {
    assert!(!values.is_empty());
    assert!(values.iter().all(|v| v.is_finite() && *v >= 0.0));
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let p95_index = ((ordered.len() * 95).div_ceil(100)).saturating_sub(1);
    serde_json::json!({
        "mean": ordered.iter().sum::<f64>() / ordered.len() as f64,
        "p95": ordered[p95_index],
        "max": ordered[ordered.len() - 1],
    })
}

fn clipping_counts(summary: &ContinuousSummary) -> (u64, u64) {
    let mut clipped = 0u64;
    let mut nonzero = 0u64;
    for choice in &summary.choices {
        for (raw_row, limited_row) in choice
            .update
            .raw_updates
            .iter()
            .zip(choice.update.limited_updates.iter())
        {
            for (&raw, &limited) in raw_row.iter().zip(limited_row.iter()) {
                if raw != 0.0 {
                    nonzero += 1;
                    if raw != limited {
                        clipped += 1;
                    }
                }
            }
        }
    }
    (clipped, nonzero)
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
