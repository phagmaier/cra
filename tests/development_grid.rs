//! Declared development sweep for the episodic learner (M3-06; spec 16/M3).
//!
//! - The manifest is frozen pre-results: grid axes, development-only seeds,
//!   sample lengths, non-overlapping early/late windows, matched conditions,
//!   margins, tiebreak order, no-pass rule, and a self-consistent budget.
//! - Every grid point expands into a config that passes
//!   `validate_episodic_execution`. Expansion validates only: no lifetime is
//!   executed and no outcome observed here, so this declaration cannot leak
//!   sweep results into the criterion it sets.
//! - The spec's suggested final-200 accuracy above 0.8 stays a recorded
//!   debugging target, never a pass/fail threshold.

use cra::config::Config;
use cra::experiments::grid::{ALLOWED_TIEBREAK_TOKENS, DevelopmentGrid, load};
use std::path::PathBuf;

fn manifest_path() -> PathBuf {
    PathBuf::from("manifests/m3_development_grid.json")
}

fn grid() -> DevelopmentGrid {
    load(&manifest_path()).expect("grid manifest loads and validates")
}

fn base_profile() -> Config {
    let text =
        std::fs::read_to_string("configs/episodic_stationary.toml").expect("episodic profile");
    toml::from_str(&text).expect("episodic parses")
}

#[test]
fn manifest_is_frozen_and_self_consistent() {
    let g = grid();
    assert_eq!(g.task, "M3-06");
    assert_eq!(g.namespace, "development");
    assert_eq!(g.root_seed, 1);
    assert_eq!(g.outer_seeds, vec![1, 2, 3]);
    assert_eq!(g.lifetime_indices, vec![0]);
    assert_eq!(g.outcomes_per_lifetime, 2000);
    assert_eq!(g.early_window_choices, 200);
    assert_eq!(g.late_window_choices, 200);
    assert_eq!(g.grid_points(), 24);
    assert_eq!(g.grid.eta, vec![0.0003, 0.001, 0.003]);
    assert_eq!(g.grid.input_scale, vec![0.2, 0.3]);
    assert_eq!(g.grid.recurrent_gain, vec![0.5, 0.8]);
    assert_eq!(g.grid.noise_sigma, vec![0.03, 0.05]);
    assert_eq!(g.conditions, vec!["B3", "B4", "B4-shuffled"]);
    assert_eq!(g.metric, "late_window_latent_accuracy");
    assert_eq!(g.budget.total_lifetimes, 216);
    // Windows must not overlap: early (first 200) + late (final 200) fit in
    // 2000 choices with a 1600-choice unmeasured middle.
    assert!(g.early_window_choices + g.late_window_choices <= g.outcomes_per_lifetime);
    // Tiebreak tokens come from the fixed vocabulary, in manifest order.
    assert!(!g.selection_tiebreak.is_empty());
    for token in &g.selection_tiebreak {
        assert!(
            ALLOWED_TIEBREAK_TOKENS.contains(&token.as_str()),
            "tiebreak token '{token}' must be declared vocabulary"
        );
    }
}

#[test]
fn combination_order_is_fixed_and_total() {
    let g = grid();
    assert_eq!(g.grid_points(), 24);
    let first = g.combination(0).expect("index 0");
    assert_eq!(
        (
            first.eta,
            first.input_scale,
            first.recurrent_gain,
            first.noise_sigma
        ),
        (0.0003, 0.2, 0.5, 0.03)
    );
    let last = g.combination(23).expect("index 23");
    assert_eq!(
        (
            last.eta,
            last.input_scale,
            last.recurrent_gain,
            last.noise_sigma
        ),
        (0.003, 0.3, 0.8, 0.05)
    );
    // Sigma is the minor axis: consecutive indices vary sigma first.
    let next = g.combination(1).expect("index 1");
    assert_eq!(next.eta, first.eta);
    assert_eq!(next.noise_sigma, 0.05);
    // Every index yields a distinct combination; out-of-range is None.
    let mut seen = std::collections::BTreeSet::new();
    for index in 0..24 {
        let p = g.combination(index).expect("in range");
        assert_eq!(p.index, index);
        assert!(seen.insert((
            p.eta.to_bits(),
            p.input_scale.to_bits(),
            p.recurrent_gain.to_bits(),
            p.noise_sigma.to_bits()
        )));
    }
    assert!(g.combination(24).is_none());
}

#[test]
fn every_grid_point_instantiates_to_an_executable_config_without_running() {
    let g = grid();
    let base = base_profile();
    // Expansion validates only: it returns candidate configs, executes no
    // lifetime, and observes no outcome.
    let expanded = g.instantiate(&base).expect("all points executable");
    assert_eq!(expanded.len(), 24);
    for (point, cfg) in &expanded {
        // Only the four swept values plus the declared lifetime length may
        // differ from the base profile; mask, trace/reset policy, gates,
        // baseline, and bounds stay exactly as declared.
        let actor = cfg.actor.as_ref().expect("actor");
        let base_actor = base.actor.as_ref().expect("base actor");
        assert_eq!(actor.input_scale, point.input_scale);
        assert_eq!(actor.recurrent_gain, point.recurrent_gain);
        assert_eq!(actor.noise_sigma, point.noise_sigma);
        assert_eq!(actor.neuron_count, base_actor.neuron_count);
        assert_eq!(actor.edge_probability, base_actor.edge_probability);
        let learning = cfg.learning.as_ref().expect("learning");
        let base_learning = base.learning.as_ref().expect("base learning");
        assert_eq!(learning.eta, point.eta);
        assert_eq!(learning.plastic_mask, base_learning.plastic_mask);
        assert_eq!(learning.trace_policy, base_learning.trace_policy);
        assert_eq!(learning.plastic_bound, base_learning.plastic_bound);
        assert_eq!(cfg.simulation.outcomes_per_lifetime, 2000);
        assert_eq!(cfg.simulation.reset_policy, "episodic_diagnostic");
        cra::config::validate_episodic_execution(cfg).expect("point executes");
    }
}

#[test]
fn budget_estimate_is_derived_not_invented() {
    let g = grid();
    let base = base_profile();
    // Nominal: 216 lifetimes x 2000 outcomes x 17 ticks per max cycle
    // (quiet 4 + cue 8 + gap 0 + response 4 + feedback 1).
    assert_eq!(g.nominal_ticks(&base).expect("nominal"), 7_344_000);
    g.check_budget_estimate(&base).expect("estimate matches");
    assert_eq!(g.budget.estimated_ticks, 7_344_000);
}

#[test]
fn spec_target_stays_a_debugging_target_not_a_threshold() {
    let g = grid();
    assert!(g.spec_target_note.contains("0.8"));
    assert!(g.spec_target_note.contains("debugging target"));
    assert!(g.spec_target_note.contains("not"));
    // The actual thresholds are the declared margins, in [0, 1].
    assert_eq!(g.criterion.margin_b4_minus_b3, 0.15);
    assert_eq!(g.criterion.margin_b4_minus_shuffled, 0.1);
    assert_eq!(g.criterion.min_outer_seeds_passing, 2);
    assert_eq!(g.criterion.max_failed_lifetimes, 0);
    assert!(!g.on_no_pass.trim().is_empty());
}

#[test]
fn invalid_declarations_are_rejected() {
    let mut g = grid();
    // Non-development namespace.
    let mut bad = g.clone();
    bad.namespace = "final_test".to_owned();
    assert!(bad.validate().is_err());
    // Empty axis.
    g = grid();
    bad = g.clone();
    bad.grid.eta.clear();
    assert!(bad.validate().is_err());
    // Non-positive noise breaks the score contract.
    bad = g.clone();
    bad.grid.noise_sigma = vec![0.0];
    assert!(bad.validate().is_err());
    // Overlapping windows.
    bad = g.clone();
    bad.early_window_choices = 1900;
    assert!(bad.validate().is_err());
    // Swapped conditions.
    bad = g.clone();
    bad.conditions = vec!["B4".to_owned(), "B3".to_owned(), "B4-shuffled".to_owned()];
    assert!(bad.validate().is_err());
    // Inconsistent budget.
    bad = g.clone();
    bad.budget.total_lifetimes = 215;
    assert!(bad.validate().is_err());
    // Unknown tiebreak token.
    bad = g.clone();
    bad.selection_tiebreak = vec!["bigger_is_better".to_owned()];
    assert!(bad.validate().is_err());
    // Outer seed outside the development reservation.
    bad = g.clone();
    bad.outer_seeds = vec![1, 2, 10001];
    assert!(bad.validate().is_err());
    // Duplicate outer seeds.
    bad = g.clone();
    bad.outer_seeds = vec![1, 1, 2];
    assert!(bad.validate().is_err());
    // Wrong base profile.
    bad = g;
    let debug: Config =
        toml::from_str(&std::fs::read_to_string("configs/debug_stationary.toml").unwrap()).unwrap();
    assert!(bad.instantiate(&debug).is_err());
}
