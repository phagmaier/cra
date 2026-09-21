//! Hand-calculated golden update (M3-03; spec 17.3).
//!
//! Independent of ordinary configured time constants: the fixture uses
//! explicit `alpha_h = 0.5` and `lambda_e = 0.9` (via
//! `tau_e = -1 / ln(0.9)`), never the actor `tau_h = 5` or a configured
//! `tau_e`. Chain through the public entry points only:
//!
//! ```text
//! score = 0.5 * 0.2 * 0.4 / 0.1 = 0.4
//! new eligibility = 0.9 * 0.3 + 0.4 = 0.67
//! delta = 1.0 - 0.6 = 0.4
//! raw update = 0.01 * 0.4 * 0.25 * 0.67 = 0.00067
//! new P = 0.10067
//! new baseline = 0.6 + 0.1 * 0.4 = 0.64
//! ```
//!
//! Tight tolerances (`1e-12`) throughout. The old baseline produces
//! `delta`; the baseline updates once after. Clipped and unclipped cases
//! are tested separately.

use cra::agent::plasticity::{FeedbackUpdateParams, PlasticState};
use cra::agent::score::conditional_score;
use cra::agent::topology::topology_from_mask;

fn small_topology() -> cra::agent::topology::Topology {
    // N = 4, m = 1: M0 = [2], M1 = [3]. Edge (2 <- 0) exists and is
    // plastic under `all_recurrent_edges`.
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

fn close(got: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (got - expected).abs() <= tol,
        "{what}: got {got}, expected {expected}"
    );
}

/// `lambda_e = 0.9` exactly as a decay factor, without borrowing any
/// configured `tau_e`: `exp(-1 / tau) = 0.9` for `tau = -1 / ln(0.9)`.
fn tau_for_lambda_09() -> f64 {
    -1.0 / 0.9_f64.ln()
}

fn update(max_update: f64) -> FeedbackUpdateParams {
    FeedbackUpdateParams {
        eta: 0.01,
        max_update,
        baseline_beta: 0.1,
    }
}

#[test]
fn golden_score_is_0_4() {
    let score = conditional_score(0.5, 0.2, 0.4, 0.1).unwrap();
    close(score, 0.4, 1e-15, "score");
}

#[test]
fn golden_eligibility_is_0_67() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let tau_09 = tau_for_lambda_09();
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        tau_09,
        0.5,
    )
    .unwrap();
    assert!((state.trace_policy().decay_factor() - 0.9).abs() < 1e-12);
    // Seed old E[2,0] = 0.3 through validated restore.
    let mut snap = state.snapshot();
    snap.e[2][0] = 0.3;
    state = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    // Only edge (2 <- 0) scores: r_old[0] = 0.2, xi[2] = 0.4.
    state
        .advance_eligibility(&[0.2, 0.0, 0.0, 0.0], &[0.0, 0.0, 0.4, 0.0], 0.5, 0.1)
        .unwrap();
    close(state.e()[2][0], 0.67, 1e-12, "new eligibility");
}

#[test]
fn golden_feedback_update_unclipped() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let frozen = w0.clone();
    let tau_09 = tau_for_lambda_09();
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        tau_09,
        10.0,
    )
    .unwrap();
    // Seed old E = 0.3, old P = 0.1, old baseline = 0.6.
    let mut snap = state.snapshot();
    snap.e[2][0] = 0.3;
    snap.p[2][0] = 0.1;
    snap.reward_baseline = 0.6;
    state = PlasticState::restore(snap, &topology, &w0, 10.0).unwrap();
    // Eligibility half of the golden.
    state
        .advance_eligibility(&[0.2, 0.0, 0.0, 0.0], &[0.0, 0.0, 0.4, 0.0], 0.5, 0.1)
        .unwrap();
    close(state.e()[2][0], 0.67, 1e-12, "eligibility");
    // Feedback half: bounds wide enough not to clip.
    let gates = vec![0.25; 4];
    let out = state
        .apply_feedback_once(0, 1.0, &gates, update(1.0), &w0)
        .unwrap();
    // The old baseline produces delta; the new baseline follows after.
    assert_eq!(out.baseline_old, 0.6);
    close(out.delta, 0.4, 1e-15, "delta");
    close(out.baseline_new, 0.64, 1e-12, "new baseline");
    assert_eq!(state.reward_baseline(), out.baseline_new);
    close(out.raw_updates[2][0], 0.00067, 1e-15, "raw update");
    assert_eq!(out.limited_updates[2][0], out.raw_updates[2][0]);
    // `actual = (P_old + limited) - P_old` round-trips through one
    // addition/subtraction, so allow 1 ulp rather than bitwise equality.
    close(
        out.actual_updates[2][0],
        out.limited_updates[2][0],
        1e-15,
        "actual == limited unclipped",
    );
    close(state.p()[2][0], 0.10067, 1e-12, "new P");
    assert_eq!(w0, frozen, "W0 never changes");
    close(
        state.effective_weights()[2][0],
        w0[2][0] + 0.10067,
        1e-12,
        "effective",
    );
    // No other plastic edge moved: only (2, 0) carried eligibility.
    for &(j, i) in state.plastic_edges().to_vec().iter() {
        if (j, i) != (2, 0) {
            assert_eq!(out.raw_updates[j][i], 0.0, "raw[{j},{i}]");
            assert_eq!(state.p()[j][i], 0.0, "P[{j},{i}]");
        }
    }
}

#[test]
fn golden_clipped_cases_stay_distinct() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let tau_09 = tau_for_lambda_09();
    let seed = |p: f64, plastic_bound: f64| {
        let base = PlasticState::new(
            &topology,
            &w0,
            "all_recurrent_edges",
            "persistent",
            tau_09,
            plastic_bound,
        )
        .unwrap();
        let mut snap = base.snapshot();
        snap.e[2][0] = 0.67;
        snap.p[2][0] = p;
        snap.reward_baseline = 0.6;
        PlasticState::restore(snap, &topology, &w0, plastic_bound).unwrap()
    };
    let gates = vec![0.25; 4];
    // Per-edge clamp: raw 0.00067 exceeds max_update 0.0001.
    let mut state = seed(0.1, 10.0);
    let out = state
        .apply_feedback_once(0, 1.0, &gates, update(0.0001), &w0)
        .unwrap();
    close(out.raw_updates[2][0], 0.00067, 1e-15, "raw");
    close(out.limited_updates[2][0], 0.0001, 1e-15, "limited");
    assert!(out.raw_updates[2][0] != out.limited_updates[2][0]);
    close(out.actual_updates[2][0], 0.0001, 1e-15, "actual");
    close(state.p()[2][0], 0.1001, 1e-12, "clamped P");
    close(
        state.reward_baseline(),
        0.64,
        1e-12,
        "baseline still updates",
    );
    // Bound clamp: P_old + limited would exceed the bound.
    let mut state = seed(0.1, 0.1005);
    let out = state
        .apply_feedback_once(0, 1.0, &gates, update(1.0), &w0)
        .unwrap();
    assert_eq!(out.limited_updates[2][0], out.raw_updates[2][0]);
    close(out.actual_updates[2][0], 0.0005, 1e-12, "actual");
    assert!(out.limited_updates[2][0] != out.actual_updates[2][0]);
    close(state.p()[2][0], 0.1005, 1e-15, "P at bound");
}
