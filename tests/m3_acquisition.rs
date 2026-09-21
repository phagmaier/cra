//! M3-07 motor-afferent acquisition (spec 16/M3).
//!
//! - Fast coverage (default suite): the sweep analysis runs end to end on
//!   small real summaries — windows, margins, health, and judging behave on
//!   library output, not only on synthetic fixtures.
//! - Bounded sweep (ignored; invoke explicitly in release): executes the
//!   frozen M3-06 grid exactly as declared (24 points x outers 1-3 x
//!   B3/B4/B4-shuffled, 2,000-outcome lifetimes), saves per-seed aggregates
//!   plus the criterion verdict to a fresh directory, and asserts execution
//!   integrity (counts, finiteness, no failures, matched schedules).
//!   The verdict itself is measured and recorded, never asserted here: this
//!   test is the instrument, and the tracker evidence states the outcome
//!   honestly whether or not any point passes.

use cra::config::Config;
use cra::experiments::episodic::run_episodic_conditions;
use cra::experiments::grid::load as load_grid;
use cra::experiments::sweep::{
    MatchedConditions, health_stats, judge_point, seed_outcome, select_winner,
};
use std::path::PathBuf;

fn manifest_path() -> PathBuf {
    PathBuf::from("manifests/m3_development_grid.json")
}

fn small_config(outcomes: u64) -> Config {
    let text =
        std::fs::read_to_string("configs/episodic_stationary.toml").expect("episodic profile");
    let mut cfg: Config = toml::from_str(&text).expect("episodic parses");
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cra::config::validate(&cfg).expect("small config validates");
    cra::config::validate_episodic_execution(&cfg).expect("small config executes");
    cfg
}

#[test]
fn analysis_runs_on_real_summaries() {
    let grid = load_grid(&manifest_path()).expect("grid loads");
    let cfg = small_config(12);
    let set = run_episodic_conditions(&cfg, 1, "development", 1, 0).expect("conditions");
    let bound = cfg.learning.as_ref().expect("learning").plastic_bound;
    let outcome = seed_outcome(
        0,
        1,
        grid.early_window_choices as usize,
        grid.late_window_choices as usize,
        bound,
        MatchedConditions {
            b3: &set.b3,
            b4: &set.b4,
            shuffled: &set.shuffled,
        },
    );
    // Structural properties on real output: windows cover the short run,
    // margins are finite differences of late accuracies, health is finite.
    assert_eq!(outcome.grid_index, 0);
    assert_eq!(outcome.outer_seed, 1);
    assert!(
        (outcome.margin_b4_minus_b3 - (outcome.b4.late_accuracy - outcome.b3.late_accuracy)).abs()
            < 1e-15
    );
    assert!(outcome.b4_health.final_p_l1 >= 0.0);
    assert!(outcome.b4_health.clipped_update_fraction >= 0.0);
    assert!(outcome.b4_health.clipped_update_fraction <= 1.0);
    assert!(health_stats(&set.b4, bound).final_baseline.is_finite());
    // Judging runs on real outcomes without claiming a pass.
    let point = grid.combination(0).expect("point 0");
    let verdict = judge_point(
        &grid,
        0,
        point.eta,
        point.input_scale,
        point.recurrent_gain,
        point.noise_sigma,
        std::slice::from_ref(&outcome),
    );
    assert_eq!(verdict.seeds_total, 1);
    assert!(!verdict.passes, "one seed cannot clear a two-seed bar");
    assert_eq!(select_winner(&[verdict]), None);
}

#[test]
#[ignore = "bounded M3-07 acquisition sweep (24 points x 3 outers x 3 conditions, 2000-outcome lifetimes); invoke explicitly in release per evidence"]
fn m3_motor_afferent_sweep() {
    let out_dir = std::env::var_os("CRA_M3_SWEEP_DIR").map(PathBuf::from).expect(
        "set CRA_M3_SWEEP_DIR to a fresh directory path (parent must exist); existing directories are rejected",
    );
    std::fs::create_dir(&out_dir).expect("sweep directory must be fresh");
    let grid = load_grid(&manifest_path()).expect("grid loads");
    let text = std::fs::read_to_string(&grid.base_profile).expect("base profile readable");
    let base: Config = toml::from_str(&text).expect("base profile parses");
    let expanded = grid.instantiate(&base).expect("all points executable");
    assert_eq!(expanded.len(), grid.grid_points());

    let bound = base.learning.as_ref().expect("learning").plastic_bound;
    let early = grid.early_window_choices as usize;
    let late = grid.late_window_choices as usize;
    let mut records_path = out_dir.clone();
    records_path.push("seed_records.jsonl");
    let mut records_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&records_path)
        .expect("records file must be fresh");
    use std::io::Write as _;

    let mut total_ticks = 0u64;
    let mut verdicts = Vec::new();
    for (point, cfg) in &expanded {
        let mut seeds = Vec::new();
        for &outer in &grid.outer_seeds {
            // Seeds come from the manifest tuple only; the instantiated
            // config carries no seed identity of its own.
            let set = run_episodic_conditions(
                cfg,
                grid.root_seed,
                &grid.namespace,
                outer,
                grid.lifetime_indices[0],
            )
            .unwrap_or_else(|e| panic!("grid {} outer {outer} failed: {e}", point.index));
            // Execution integrity: full lifetimes, matched schedules,
            // finite values, consumed dedup. Any violation aborts the
            // sweep loudly instead of entering the aggregates.
            assert_eq!(set.b4.outcomes, grid.outcomes_per_lifetime);
            assert_eq!(set.b3.outcomes, grid.outcomes_per_lifetime);
            assert_eq!(set.shuffled.outcomes, grid.outcomes_per_lifetime);
            assert_eq!(set.b3.resets, set.b4.resets);
            assert_eq!(set.b4.resets, set.shuffled.resets);
            assert!(set.b4.mean_reward.is_finite());
            assert!(set.b4.final_baseline.is_finite());
            assert!(set.b4.final_p.iter().flatten().all(|v| v.is_finite()));
            assert!(set.b4.final_e.iter().flatten().all(|v| v.is_finite()));
            assert_eq!(
                set.b4.final_last_feedback,
                Some(grid.outcomes_per_lifetime - 1)
            );
            total_ticks += set.b4.ticks + set.b3.ticks + set.shuffled.ticks;
            let outcome = seed_outcome(
                point.index,
                outer,
                early,
                late,
                bound,
                MatchedConditions {
                    b3: &set.b3,
                    b4: &set.b4,
                    shuffled: &set.shuffled,
                },
            );
            let record = serde_json::json!({
                "grid_index": point.index,
                "eta": point.eta,
                "input_scale": point.input_scale,
                "recurrent_gain": point.recurrent_gain,
                "noise_sigma": point.noise_sigma,
                "outer_seed": outer,
                "namespace": grid.namespace,
                "root_seed": grid.root_seed,
                "lifetime_index": grid.lifetime_indices[0],
                "outcomes": grid.outcomes_per_lifetime,
                "ticks": {"b3": set.b3.ticks, "b4": set.b4.ticks, "shuffled": set.shuffled.ticks},
                "b3": outcome.b3,
                "b4": outcome.b4,
                "shuffled": outcome.shuffled,
                "b4_health": outcome.b4_health,
                "shuffled_health": outcome.shuffled_health,
                "margin_b4_minus_b3": outcome.margin_b4_minus_b3,
                "margin_b4_minus_shuffled": outcome.margin_b4_minus_shuffled,
            });
            writeln!(records_file, "{record}").expect("records append");
            seeds.push(outcome);
        }
        verdicts.push(judge_point(
            &grid,
            point.index,
            point.eta,
            point.input_scale,
            point.recurrent_gain,
            point.noise_sigma,
            &seeds,
        ));
    }
    let winner = select_winner(&verdicts);
    let revision = command_output("git", &["rev-parse", "HEAD"]);
    let status = command_output("git", &["status", "--porcelain"]);
    let verdict_doc = serde_json::json!({
        "schema_version": 1,
        "task": "M3-07",
        "grid_manifest": manifest_path().display().to_string(),
        "base_profile": grid.base_profile,
        "conditions": grid.conditions,
        "metric": grid.metric,
        "criterion": grid.criterion,
        "selection_tiebreak": grid.selection_tiebreak,
        "verdicts": verdicts,
        "winner_grid_index": winner,
        "measured_budget": {
            "total_lifetimes": grid.budget.total_lifetimes,
            "total_ticks": total_ticks,
            "declared_estimated_ticks": grid.budget.estimated_ticks,
        },
        "provenance": {
            "revision": revision, "dirty": !status.is_empty(), "git_status": status,
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            "profile": "release",
        },
    });
    let mut verdict_path = out_dir.clone();
    verdict_path.push("verdict.json");
    std::fs::write(
        &verdict_path,
        serde_json::to_string_pretty(&verdict_doc).unwrap(),
    )
    .expect("verdict write");
    eprintln!("sweep verdict: {verdict_doc}");
    // The sweep must execute cleanly; the empirical verdict is recorded
    // above and interpreted in the tracker, not asserted here.
}

fn command_output(cmd: &str, args: &[&str]) -> String {
    String::from_utf8_lossy(
        &std::process::Command::new(cmd)
            .args(args)
            .output()
            .expect("provenance command runs")
            .stdout,
    )
    .trim()
    .to_owned()
}
