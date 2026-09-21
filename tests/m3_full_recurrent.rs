//! M3-08 all-recurrent acquisition (spec 7, 16/M3).
//!
//! - Fast coverage (default suite): the sweep analysis runs end to end on
//!   small real full-mask summaries — windows, margins, health, and judging
//!   behave on library output with `plastic_mask = "all_recurrent_edges"`.
//!   The mask change is proven structural (full plastic set strictly
//!   contains the motor-afferent subset on the same inheritance) and
//!   behavioral (non-motor receivers acquire nonzero offsets).
//! - Bounded comparison (ignored; invoke explicitly in release): repeats the
//!   declared M3-06 development comparisons at the frozen winner
//!   hyperparameters (grid index 11: eta 0.001, input_scale 0.2,
//!   recurrent_gain 0.8, noise_sigma 0.05) with the full existing-edge
//!   plastic mask on outers 1-3, 2,000-outcome lifetimes, matched
//!   B3/B4/B4-shuffled. Saves per-seed aggregates plus the criterion
//!   verdict to a fresh directory and asserts execution integrity (counts,
//!   finiteness, no failures, matched schedules). The verdict itself is
//!   measured and recorded, never asserted here: this test is the
//!   instrument, and the tracker evidence states the outcome honestly.
//!   The M3-07 motor-only run is preserved as a diagnostic and is never
//!   overwritten by this task.

use cra::agent::plasticity::PlasticMaskKind;
use cra::config::Config;
use cra::experiments::episodic::{run_episodic_conditions, sample_matched_inheritance};
use cra::experiments::grid::load as load_grid;
use cra::experiments::sweep::{
    MatchedConditions, health_stats, judge_point, seed_outcome, select_winner,
};
use std::path::PathBuf;

fn manifest_path() -> PathBuf {
    PathBuf::from("manifests/m3_development_grid.json")
}

/// Winner hyperparameters from the frozen M3-07 verdict (grid index 11).
/// Read from the manifest so a silent grid edit fails loudly instead of
/// running a re-tuned point.
fn winner_point() -> (usize, f64, f64, f64, f64) {
    let grid = load_grid(&manifest_path()).expect("grid loads");
    let point = grid.combination(11).expect("grid index 11 exists");
    assert!(
        (point.eta - 0.001).abs() < 1e-15,
        "grid index 11 eta must stay 0.001; found {}",
        point.eta
    );
    assert!(
        (point.input_scale - 0.2).abs() < 1e-15,
        "grid index 11 input_scale must stay 0.2; found {}",
        point.input_scale
    );
    assert!(
        (point.recurrent_gain - 0.8).abs() < 1e-15,
        "grid index 11 recurrent_gain must stay 0.8; found {}",
        point.recurrent_gain
    );
    assert!(
        (point.noise_sigma - 0.05).abs() < 1e-15,
        "grid index 11 noise_sigma must stay 0.05; found {}",
        point.noise_sigma
    );
    (
        point.index,
        point.eta,
        point.input_scale,
        point.recurrent_gain,
        point.noise_sigma,
    )
}

fn full_mask_config(outcomes: u64) -> Config {
    let grid = load_grid(&manifest_path()).expect("grid loads");
    let text = std::fs::read_to_string(&grid.base_profile).expect("base profile readable");
    let mut cfg: Config = toml::from_str(&text).expect("base profile parses");
    let (_, eta, input_scale, recurrent_gain, noise_sigma) = winner_point();
    let actor = cfg.actor.as_mut().expect("base has [actor]");
    actor.input_scale = input_scale;
    actor.recurrent_gain = recurrent_gain;
    actor.noise_sigma = noise_sigma;
    let learning = cfg.learning.as_mut().expect("base has [learning]");
    learning.eta = eta;
    learning.plastic_mask = "all_recurrent_edges".to_owned();
    cfg.simulation.outcomes_per_lifetime = outcomes;
    let trace_policy = cfg
        .learning
        .as_ref()
        .expect("learning")
        .trace_policy
        .clone();
    let reset_policy = cfg.simulation.reset_policy.clone();
    cra::config::validate(&cfg).expect("full-mask config validates");
    cra::config::validate_episodic_execution(&cfg).expect("full-mask config executes");
    assert_eq!(
        trace_policy, "no_decay_diagnostic",
        "M3-08 stays on the episodic diagnostic trace policy"
    );
    assert_eq!(
        reset_policy, "episodic_diagnostic",
        "M3-08 stays on the episodic diagnostic reset policy"
    );
    cfg
}

#[test]
fn analysis_runs_on_real_full_mask_summaries() {
    let grid = load_grid(&manifest_path()).expect("grid loads");
    let cfg = full_mask_config(12);
    assert_eq!(
        cfg.learning.as_ref().expect("learning").plastic_mask,
        "all_recurrent_edges"
    );
    let set = run_episodic_conditions(&cfg, 1, "development", 1, 0).expect("conditions");
    assert_eq!(set.b4.plastic_mask, "all_recurrent_edges");
    assert_eq!(set.shuffled.plastic_mask, "all_recurrent_edges");
    // Matched schedules and inheritance across the family.
    assert_eq!(set.b3.w0, set.b4.w0, "B3/B4 must share W0");
    assert_eq!(set.b4.w0, set.shuffled.w0, "B4/shuffled must share W0");
    let b3_cues: Vec<usize> = set.b3.choices.iter().map(|c| c.cue).collect();
    let b4_cues: Vec<usize> = set.b4.choices.iter().map(|c| c.cue).collect();
    let sh_cues: Vec<usize> = set.shuffled.choices.iter().map(|c| c.cue).collect();
    assert_eq!(b3_cues, b4_cues);
    assert_eq!(b4_cues, sh_cues);
    assert_eq!(set.b3.resets, set.b4.resets);
    assert_eq!(set.b4.resets, set.shuffled.resets);
    // Paired noise: first commitments agree before any update exists.
    assert_eq!(set.b3.choices[0].action, set.b4.choices[0].action);
    assert_eq!(set.b3.choices[0].action, set.shuffled.choices[0].action);
    // Structural mask proof on the same inheritance: full strictly
    // contains motor-afferent, and every motor-afferent edge is full.
    let (params, _) =
        sample_matched_inheritance(&cfg, grid.root_seed, &grid.namespace, 1).expect("inheritance");
    let full = PlasticMaskKind::from_name("all_recurrent_edges")
        .expect("full mask")
        .build_mask(&params.topology);
    let motor = PlasticMaskKind::from_name("motor_afferent_only")
        .expect("motor mask")
        .build_mask(&params.topology);
    let count = |m: &[Vec<bool>]| m.iter().flatten().filter(|&&v| v).count();
    let (n_full, n_motor) = (count(&full), count(&motor));
    assert!(n_full > 0, "full mask must hold plastic edges");
    assert!(n_motor > 0, "motor mask must hold plastic edges");
    assert!(
        n_full > n_motor,
        "full mask ({n_full}) must strictly contain motor-only ({n_motor})"
    );
    for (j, (f_row, m_row)) in full.iter().zip(motor.iter()).enumerate() {
        for (i, (&f, &m)) in f_row.iter().zip(m_row.iter()).enumerate() {
            if m {
                assert!(f, "motor edge ({j},{i}) must also be full-mask plastic");
            }
        }
    }
    // Behavioral mask proof: some non-motor receiver acquired an offset.
    let is_motor =
        |j: usize| params.topology.motor0.contains(&j) || params.topology.motor1.contains(&j);
    let mut non_motor_moved = false;
    for (j, row) in set.b4.final_p.iter().enumerate() {
        if is_motor(j) {
            continue;
        }
        for (i, &p) in row.iter().enumerate() {
            if !params.topology.mask[j][i] {
                assert_eq!(p, 0.0, "missing edge ({j},{i}) must stay zero");
                continue;
            }
            if p != 0.0 {
                non_motor_moved = true;
            }
        }
    }
    assert!(
        non_motor_moved,
        "full-mask B4 must move at least one non-motor offset in 12 outcomes"
    );
    // W0 invariance and effective == W0 + P with bound compliance.
    let bound = cfg.learning.as_ref().expect("learning").plastic_bound;
    for (j, (w_row, eff_row)) in set
        .b4
        .w0
        .iter()
        .zip(set.b4.final_effective.iter())
        .enumerate()
    {
        for (i, (&w, &eff)) in w_row.iter().zip(eff_row.iter()).enumerate() {
            let p = set.b4.final_p[j][i];
            assert!((eff - (w + p)).abs() <= 1e-12);
            assert!(p.abs() <= bound + 1e-12);
            if !full[j][i] {
                assert_eq!(p, 0.0, "nonplastic edge ({j},{i}) must stay zero");
            }
        }
    }
    // Analysis plumbing on real output.
    let bound_v = bound;
    let outcome = seed_outcome(
        11,
        1,
        grid.early_window_choices as usize,
        grid.late_window_choices as usize,
        bound_v,
        MatchedConditions {
            b3: &set.b3,
            b4: &set.b4,
            shuffled: &set.shuffled,
        },
    );
    assert_eq!(outcome.grid_index, 11);
    assert!(
        (outcome.margin_b4_minus_b3 - (outcome.b4.late_accuracy - outcome.b3.late_accuracy)).abs()
            < 1e-15
    );
    assert!(outcome.b4_health.final_p_l1 > 0.0);
    assert!(health_stats(&set.b4, bound_v).final_baseline.is_finite());
    let point = grid.combination(11).expect("point 11");
    let verdict = judge_point(
        &grid,
        11,
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
#[ignore = "bounded M3-08 full-recurrent comparison (1 point x 3 outers x 3 conditions, 2000-outcome lifetimes); invoke explicitly in release per evidence"]
fn m3_full_recurrent_comparison() {
    let out_dir = std::env::var_os("CRA_M3_FULL_SWEEP_DIR").map(PathBuf::from).expect(
        "set CRA_M3_FULL_SWEEP_DIR to a fresh directory path (parent must exist); existing directories are rejected",
    );
    std::fs::create_dir(&out_dir).expect("sweep directory must be fresh");
    let grid = load_grid(&manifest_path()).expect("grid loads");
    let (grid_index, eta, input_scale, recurrent_gain, noise_sigma) = winner_point();
    let cfg = full_mask_config(grid.outcomes_per_lifetime);
    // full_mask_config already applies the winner values; re-assert here so
    // the executed config is self-describing in failures.
    assert_eq!(cfg.learning.as_ref().expect("learning").eta, eta);
    assert_eq!(
        cfg.learning.as_ref().expect("learning").plastic_mask,
        "all_recurrent_edges"
    );
    let bound = cfg.learning.as_ref().expect("learning").plastic_bound;
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
    let mut seeds = Vec::new();
    for &outer in &grid.outer_seeds {
        let set = run_episodic_conditions(
            &cfg,
            grid.root_seed,
            &grid.namespace,
            outer,
            grid.lifetime_indices[0],
        )
        .unwrap_or_else(|e| panic!("grid {grid_index} outer {outer} failed: {e}"));
        // Execution integrity: full lifetimes, matched schedules, finite
        // values, consumed dedup. Any violation aborts loudly.
        assert_eq!(set.b4.outcomes, grid.outcomes_per_lifetime);
        assert_eq!(set.b3.outcomes, grid.outcomes_per_lifetime);
        assert_eq!(set.shuffled.outcomes, grid.outcomes_per_lifetime);
        assert_eq!(set.b3.resets, set.b4.resets);
        assert_eq!(set.b4.resets, set.shuffled.resets);
        assert!(set.b4.mean_reward.is_finite());
        assert!(set.b4.final_baseline.is_finite());
        assert!(set.b4.final_p.iter().flatten().all(|v| v.is_finite()));
        assert!(set.b4.final_e.iter().flatten().all(|v| v.is_finite()));
        assert!(set.shuffled.final_p.iter().flatten().all(|v| v.is_finite()));
        assert_eq!(
            set.b4.final_last_feedback,
            Some(grid.outcomes_per_lifetime - 1)
        );
        assert_eq!(set.b4.plastic_mask, "all_recurrent_edges");
        total_ticks += set.b4.ticks + set.b3.ticks + set.shuffled.ticks;
        let outcome = seed_outcome(
            grid_index,
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
            "grid_index": grid_index,
            "eta": eta,
            "input_scale": input_scale,
            "recurrent_gain": recurrent_gain,
            "noise_sigma": noise_sigma,
            "plastic_mask": "all_recurrent_edges",
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
    let verdict = judge_point(
        &grid,
        grid_index,
        eta,
        input_scale,
        recurrent_gain,
        noise_sigma,
        &seeds,
    );
    let revision = command_output("git", &["rev-parse", "HEAD"]);
    let status = command_output("git", &["status", "--porcelain"]);
    let verdict_doc = serde_json::json!({
        "schema_version": 1,
        "task": "M3-08",
        "grid_manifest": manifest_path().display().to_string(),
        "grid_index": grid_index,
        "eta": eta,
        "input_scale": input_scale,
        "recurrent_gain": recurrent_gain,
        "noise_sigma": noise_sigma,
        "plastic_mask": "all_recurrent_edges",
        "motor_only_reference": "docs/evidence/m3-07/verdict.json (preserved diagnostic, not overwritten)",
        "base_profile": grid.base_profile,
        "conditions": grid.conditions,
        "metric": grid.metric,
        "criterion": grid.criterion,
        "selection_tiebreak": grid.selection_tiebreak,
        "verdict": verdict,
        "passes": verdict.passes,
        "measured_budget": {
            "total_lifetimes": grid.outer_seeds.len() * grid.lifetime_indices.len() * grid.conditions.len(),
            "total_ticks": total_ticks,
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
    eprintln!("full-recurrent verdict: {verdict_doc}");
    // The comparison must execute cleanly; the empirical verdict is
    // recorded above and interpreted in the tracker, not asserted here.
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
