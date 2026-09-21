//! Exactly-once feedback updates and baseline arithmetic (M3-02;
//! spec 7.4-7.5, 9 step 2, 10.4, 10.6, 17.2).
//!
//! - Reads old `E`/gates/`P`/baseline; `delta` uses the old baseline and
//!   the baseline updates once after `delta`. Fixed mode passes gate `1`.
//! - Per-edge `raw = eta * delta * gate * E`, `limited` clamps each raw to
//!   `[-max_update, +max_update]`, `P_new` clamps to `[-plastic_bound,
//!   +plastic_bound]`; the report keeps raw/limited/actual distinct at both
//!   clipping boundaries.
//! - Duplicate (or older) `event_id` is rejected with all state unchanged.
//! - `eta = 0`, all-zero gates, and `delta = 0` give zero task-dependent `P`
//!   changes. `W0` is never mutated; the effective cache stays `W0 + P`.
//! - Missing/nonplastic edges stay exactly `0.0` in `P` and all reports.
//! - `E` is read, never reset, by feedback. New baseline/dedup state
//!   round-trips through the versioned snapshot.

use cra::agent::plasticity::{FeedbackUpdateParams, PlasticState, PlasticityError};
use cra::agent::topology::topology_from_mask;

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

fn ones(n: usize) -> Vec<f64> {
    vec![1.0; n]
}

fn update(eta: f64, max_update: f64, baseline_beta: f64) -> FeedbackUpdateParams {
    FeedbackUpdateParams {
        eta,
        max_update,
        baseline_beta,
    }
}

fn close(got: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (got - expected).abs() <= tol,
        "{what}: got {got}, expected {expected}"
    );
}

/// Seed `E` on one plastic edge and optionally `P`/baseline via a
/// validated snapshot restore. Returns the state ready for feedback.
fn seeded_state(
    topology: &cra::agent::topology::Topology,
    w0: &[Vec<f64>],
    edge: (usize, usize),
    e_value: f64,
    p_value: f64,
    baseline: f64,
) -> PlasticState {
    let state =
        PlasticState::new(topology, w0, "all_recurrent_edges", "persistent", 32.0, 0.5).unwrap();
    let mut snap = state.snapshot();
    snap.e[edge.0][edge.1] = e_value;
    snap.p[edge.0][edge.1] = p_value;
    snap.reward_baseline = baseline;
    PlasticState::restore(snap, topology, w0, 0.5).unwrap()
}

#[test]
fn delta_uses_old_baseline_and_updates_once_after() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    // E[2,0] = 1.0, baseline 0.6, reward 1.0, gate 1, eta 0.01:
    // delta = 0.4, raw = 0.004, no clipping, P 0 -> 0.004,
    // baseline 0.6 -> 0.64 with beta 0.1.
    let mut state = seeded_state(&topology, &w0, (2, 0), 1.0, 0.0, 0.6);
    let out = state
        .apply_feedback_once(0, 1.0, &ones(4), update(0.01, 0.01, 0.1), &w0)
        .unwrap();
    close(out.delta, 0.4, 1e-15, "delta");
    assert_eq!(out.baseline_old, 0.6);
    close(out.baseline_new, 0.64, 1e-15, "baseline_new");
    assert_eq!(state.reward_baseline(), out.baseline_new);
    close(out.raw_updates[2][0], 0.004, 1e-15, "raw");
    assert_eq!(out.limited_updates[2][0], out.raw_updates[2][0]);
    assert_eq!(out.actual_updates[2][0], out.limited_updates[2][0]);
    close(state.p()[2][0], 0.004, 1e-15, "P");
    // A second outcome reads the updated baseline, not the birth value.
    let mut snap = state.snapshot();
    snap.e[2][0] = 1.0;
    let mut state2 = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    let out2 = state2
        .apply_feedback_once(1, 1.0, &ones(4), update(0.01, 0.01, 0.1), &w0)
        .unwrap();
    close(
        out2.delta,
        1.0 - 0.64,
        1e-15,
        "second delta reads new baseline",
    );
    close(
        out2.baseline_new,
        0.64 + 0.1 * (1.0 - 0.64),
        1e-15,
        "second baseline",
    );
}

#[test]
fn duplicate_and_older_events_leave_all_state_unchanged() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = seeded_state(&topology, &w0, (2, 0), 0.8, 0.02, 0.5);
    let before_p = state.p().to_vec();
    let before_e = state.e().to_vec();
    state
        .apply_feedback_once(3, 1.0, &ones(4), update(0.01, 0.01, 0.02), &w0)
        .unwrap();
    assert_eq!(state.last_feedback(), Some(3));
    let after_p = state.p().to_vec();
    let after_e = state.e().to_vec();
    let after_eff = state.effective_weights().to_vec();
    let after_baseline = state.reward_baseline();
    // Same id twice is a duplicate.
    assert!(matches!(
        state.apply_feedback_once(3, 1.0, &ones(4), update(0.01, 0.01, 0.02), &w0),
        Err(PlasticityError::DuplicateFeedback(3))
    ));
    // An older id is also rejected (monotonic dedup, as in B3).
    assert!(matches!(
        state.apply_feedback_once(1, 0.0, &ones(4), update(0.01, 0.01, 0.02), &w0),
        Err(PlasticityError::DuplicateFeedback(1))
    ));
    assert_eq!(state.p(), &after_p, "P unchanged by duplicates");
    assert_eq!(state.e(), &after_e, "E unchanged by duplicates");
    assert_eq!(state.effective_weights(), &after_eff);
    assert_eq!(state.reward_baseline(), after_baseline);
    assert_eq!(state.last_feedback(), Some(3));
    assert_ne!(after_p, before_p, "first update actually moved P");
    assert_eq!(after_e, before_e, "feedback never resets E");
    // A fresh higher id still applies after rejections.
    let out = state
        .apply_feedback_once(4, 0.0, &ones(4), update(0.01, 0.01, 0.02), &w0)
        .unwrap();
    assert_eq!(out.event_id, 4);
    assert_eq!(state.last_feedback(), Some(4));
}

#[test]
fn zero_eta_zero_gate_and_zero_delta_leave_p_unchanged() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    // eta = 0 with nonzero delta: P frozen, baseline still tracks reward.
    let mut state = seeded_state(&topology, &w0, (2, 0), 1.5, 0.03, 0.5);
    let out = state
        .apply_feedback_once(0, 1.0, &ones(4), update(0.0, 0.01, 0.1), &w0)
        .unwrap();
    assert!(out.raw_updates.iter().flatten().all(|&v| v == 0.0));
    assert!(out.limited_updates.iter().flatten().all(|&v| v == 0.0));
    assert!(out.actual_updates.iter().flatten().all(|&v| v == 0.0));
    assert_eq!(state.p()[2][0], 0.03);
    close(
        state.reward_baseline(),
        0.5 + 0.1 * 0.5,
        1e-15,
        "baseline tracks",
    );
    // All-zero gates with eta > 0: P frozen, baseline still tracks.
    let mut state = seeded_state(&topology, &w0, (2, 0), 1.5, 0.03, 0.5);
    let out = state
        .apply_feedback_once(0, 1.0, &[0.0; 4], update(0.01, 0.01, 0.1), &w0)
        .unwrap();
    assert!(out.actual_updates.iter().flatten().all(|&v| v == 0.0));
    assert_eq!(state.p()[2][0], 0.03);
    close(out.delta, 0.5, 1e-15, "delta");
    // delta = 0 (baseline 1.0, reward 1.0): P frozen and baseline frozen.
    let mut state = seeded_state(&topology, &w0, (2, 0), 1.5, 0.03, 1.0);
    let out = state
        .apply_feedback_once(0, 1.0, &ones(4), update(0.01, 0.01, 0.1), &w0)
        .unwrap();
    assert_eq!(out.delta, 0.0);
    assert!(out.raw_updates.iter().flatten().all(|&v| v == 0.0));
    assert_eq!(state.p()[2][0], 0.03);
    assert_eq!(state.reward_baseline(), 1.0);
}

#[test]
fn max_update_clipping_separates_raw_from_limited() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    // raw = 0.1 * 0.5 * 1 * 2.0 = 0.1; max_update 0.01 clamps to 0.01.
    // Bound 0.5 is wide, so actual == limited != raw.
    let mut state = seeded_state(&topology, &w0, (2, 0), 2.0, 0.0, 0.5);
    let out = state
        .apply_feedback_once(0, 1.0, &ones(4), update(0.1, 0.01, 0.0), &w0)
        .unwrap();
    close(out.raw_updates[2][0], 0.1, 1e-12, "raw");
    close(out.limited_updates[2][0], 0.01, 1e-15, "limited");
    close(out.actual_updates[2][0], 0.01, 1e-15, "actual");
    assert!(out.raw_updates[2][0] != out.limited_updates[2][0]);
    close(state.p()[2][0], 0.01, 1e-15, "P");
    // Negative side clamps symmetrically.
    let mut state = seeded_state(&topology, &w0, (2, 0), 2.0, 0.0, 0.5);
    let out = state
        .apply_feedback_once(0, 0.0, &ones(4), update(0.1, 0.01, 0.0), &w0)
        .unwrap();
    close(out.raw_updates[2][0], -0.1, 1e-12, "negative raw");
    close(out.limited_updates[2][0], -0.01, 1e-15, "negative limited");
    close(state.p()[2][0], -0.01, 1e-15, "negative P");
}

#[test]
fn plastic_bound_clipping_separates_limited_from_actual() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    // P_old 0.49 + limited 0.05 would give 0.54; bound 0.5 clamps the
    // realized change to 0.01.
    let mut state = seeded_state(&topology, &w0, (2, 0), 1.0, 0.49, 0.5);
    let out = state
        .apply_feedback_once(0, 1.0, &ones(4), update(0.1, 1.0, 0.0), &w0)
        .unwrap();
    close(out.raw_updates[2][0], 0.05, 1e-12, "raw");
    assert_eq!(out.limited_updates[2][0], out.raw_updates[2][0]);
    close(out.actual_updates[2][0], 0.01, 1e-12, "actual");
    assert!(out.limited_updates[2][0] != out.actual_updates[2][0]);
    close(state.p()[2][0], 0.5, 1e-15, "P at bound");
    // Negative bound side.
    let mut state = seeded_state(&topology, &w0, (2, 0), 1.0, -0.49, 0.5);
    let out = state
        .apply_feedback_once(0, 0.0, &ones(4), update(0.1, 1.0, 0.0), &w0)
        .unwrap();
    close(out.actual_updates[2][0], -0.01, 1e-12, "negative actual");
    close(state.p()[2][0], -0.5, 1e-15, "P at negative bound");
}

#[test]
fn w0_never_changes_and_effective_stays_w0_plus_p() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let frozen = w0.clone();
    let mut state = seeded_state(&topology, &w0, (3, 2), -0.7, 0.05, 0.4);
    let out = state
        .apply_feedback_once(7, 1.0, &ones(4), update(0.02, 0.01, 0.05), &w0)
        .unwrap();
    assert_eq!(w0, frozen, "caller W0 slice unchanged");
    for (j, (eff_row, w_row)) in state
        .effective_weights()
        .iter()
        .zip(frozen.iter())
        .enumerate()
    {
        for (i, (&eff, &w)) in eff_row.iter().zip(w_row.iter()).enumerate() {
            close(
                eff,
                w + state.p()[j][i],
                1e-15,
                &format!("effective[{j},{i}]"),
            );
        }
    }
    // Missing edges stay exactly absent in every report and the cache.
    assert_eq!(state.effective_weights()[0][0], 0.0);
    assert_eq!(out.raw_updates[0][0], 0.0);
    assert_eq!(out.limited_updates[0][0], 0.0);
    assert_eq!(out.actual_updates[0][0], 0.0);
}

#[test]
fn nonplastic_edges_never_acquire_reports_or_offsets() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let state = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        32.0,
        0.5,
    )
    .unwrap();
    // Seed E on a motor-afferent edge through the validated path, then run
    // feedback: only plastic edges may report or move.
    let mut snap = state.snapshot();
    snap.e[2][0] = 1.2;
    snap.e[3][2] = -0.9;
    let mut state = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    let out = state
        .apply_feedback_once(0, 1.0, &ones(4), update(0.01, 0.01, 0.05), &w0)
        .unwrap();
    for (j, (mask_row, topo_row)) in state
        .plastic_mask()
        .iter()
        .zip(topology.mask.iter())
        .enumerate()
    {
        for (i, (&plastic, &structural)) in mask_row.iter().zip(topo_row.iter()).enumerate() {
            if !plastic {
                assert_eq!(state.p()[j][i], 0.0, "P[{j},{i}]");
                assert_eq!(out.raw_updates[j][i], 0.0, "raw[{j},{i}]");
                assert_eq!(out.limited_updates[j][i], 0.0, "limited[{j},{i}]");
                assert_eq!(out.actual_updates[j][i], 0.0, "actual[{j},{i}]");
                if structural {
                    assert_eq!(state.effective_weights()[j][i], w0[j][i]);
                }
            }
        }
    }
    assert_ne!(state.p()[2][0], 0.0, "plastic motor edge moves");
}

#[test]
fn gate_scaling_is_per_receiver_and_doubling_doubles_unclipped_updates() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    // Receiver 2 owns two plastic edges (2<-0, 2<-3 in the small mask).
    // Seed both traces; one gate scales both edges together.
    let base = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        32.0,
        0.5,
    )
    .unwrap();
    let mut snap = base.snapshot();
    snap.e[2][0] = 1.0;
    snap.e[2][3] = 2.0;
    let mut half = PlasticState::restore(snap.clone(), &topology, &w0, 0.5).unwrap();
    let mut full = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    let mut gates_half = ones(4);
    gates_half[2] = 0.5;
    let half_out = half
        .apply_feedback_once(0, 1.0, &gates_half, update(0.01, 1.0, 0.0), &w0)
        .unwrap();
    let full_out = full
        .apply_feedback_once(0, 1.0, &ones(4), update(0.01, 1.0, 0.0), &w0)
        .unwrap();
    close(
        full_out.raw_updates[2][0],
        2.0 * half_out.raw_updates[2][0],
        1e-15,
        "doubling gate doubles raw",
    );
    close(
        full_out.raw_updates[2][3],
        2.0 * half_out.raw_updates[2][3],
        1e-15,
        "doubling gate doubles sibling edge",
    );
    // A gate on another receiver does not leak into receiver 2.
    let mut other = PlasticState::restore(full.snapshot(), &topology, &w0, 0.5).unwrap();
    let mut gates_other = vec![0.0; 4];
    gates_other[1] = 1.0;
    let other_out = other
        .apply_feedback_once(1, 1.0, &gates_other, update(0.01, 1.0, 0.0), &w0)
        .unwrap();
    assert_eq!(other_out.raw_updates[2][0], 0.0);
    assert_eq!(other_out.raw_updates[2][3], 0.0);
}

#[test]
fn invalid_feedback_inputs_are_rejected_without_state_change() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = seeded_state(&topology, &w0, (2, 0), 0.6, 0.04, 0.5);
    let snapshot = state.snapshot();
    let eff = state.effective_weights().to_vec();
    let mut bad_calls = 0;
    // Bad rewards.
    for &reward in &[0.5, 2.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            state
                .apply_feedback_once(10, reward, &ones(4), update(0.01, 0.01, 0.05), &w0)
                .is_err(),
            "reward {reward}"
        );
        bad_calls += 1;
    }
    // Bad gates: wrong length, out of range, nonfinite.
    assert!(
        state
            .apply_feedback_once(10, 1.0, &[1.0; 3], update(0.01, 0.01, 0.05), &w0)
            .is_err()
    );
    bad_calls += 1;
    for gates in [
        vec![1.0, 1.0, 1.0, 1.5],
        vec![1.0, 1.0, -0.1, 1.0],
        vec![1.0, f64::NAN, 1.0, 1.0],
    ] {
        assert!(
            state
                .apply_feedback_once(10, 1.0, &gates, update(0.01, 0.01, 0.05), &w0)
                .is_err(),
            "gates {gates:?}"
        );
        bad_calls += 1;
    }
    // Bad hyperparameters mirror config validation ranges.
    for (eta, max_update, beta) in [
        (-0.01, 0.01, 0.05),
        (f64::NAN, 0.01, 0.05),
        (0.01, 0.0, 0.05),
        (0.01, -0.01, 0.05),
        (0.01, 0.01, -0.1),
        (0.01, 0.01, 1.5),
    ] {
        assert!(
            state
                .apply_feedback_once(10, 1.0, &ones(4), update(eta, max_update, beta), &w0)
                .is_err(),
            "hyperparams {eta}/{max_update}/{beta}"
        );
        bad_calls += 1;
    }
    // Bad W0: ragged and nonzero-on-missing, without touching state.
    let mut ragged = w0.clone();
    ragged[0].pop();
    assert!(
        state
            .apply_feedback_once(10, 1.0, &ones(4), update(0.01, 0.01, 0.05), &ragged)
            .is_err()
    );
    bad_calls += 1;
    let mut bad_w0 = w0.clone();
    bad_w0[0][0] = 0.25;
    assert!(!topology.mask[0][0]);
    assert!(
        state
            .apply_feedback_once(10, 1.0, &ones(4), update(0.01, 0.01, 0.05), &bad_w0)
            .is_err()
    );
    bad_calls += 1;
    assert!(bad_calls > 10);
    assert_eq!(state.snapshot(), snapshot, "rejected calls change nothing");
    assert_eq!(state.effective_weights(), &eff);
    assert_eq!(state.last_feedback(), None);
    // The state still accepts a valid event after all rejections.
    state
        .apply_feedback_once(10, 1.0, &ones(4), update(0.01, 0.01, 0.05), &w0)
        .unwrap();
    assert_eq!(state.last_feedback(), Some(10));
}

#[test]
fn snapshot_preserves_baseline_dedup_and_nonzero_p() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = seeded_state(&topology, &w0, (2, 0), 0.9, 0.0, 0.5);
    state
        .apply_feedback_once(5, 1.0, &ones(4), update(0.02, 0.01, 0.05), &w0)
        .unwrap();
    assert_ne!(state.p()[2][0], 0.0);
    assert_eq!(state.last_feedback(), Some(5));
    let snapshot = state.snapshot();
    let path = std::env::temp_dir().join(format!("cra-feedback-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec_pretty(&snapshot).unwrap()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let loaded: cra::agent::plasticity::PlasticSnapshot = serde_json::from_slice(&bytes).unwrap();
    let restored = PlasticState::restore(loaded, &topology, &w0, 0.5).unwrap();
    assert_eq!(restored, state);
    assert_eq!(restored.reward_baseline(), state.reward_baseline());
    assert_eq!(restored.last_feedback(), Some(5));
    assert_eq!(restored.effective_weights(), state.effective_weights());
    let _ = std::fs::remove_file(&path);
    // Dedup survives the round-trip: the consumed id is still rejected.
    let mut restored = restored;
    assert!(matches!(
        restored.apply_feedback_once(5, 1.0, &ones(4), update(0.02, 0.01, 0.05), &w0),
        Err(PlasticityError::DuplicateFeedback(5))
    ));
    // Nonfinite baseline snapshots are corrupt, not resumable state.
    let mut bad = state.snapshot();
    bad.reward_baseline = f64::NAN;
    assert!(PlasticState::restore(bad, &topology, &w0, 0.5).is_err());
}

#[test]
fn from_learning_config_carries_baseline_initial() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let learning: cra::config::Learning = toml::from_str(
        r#"
enabled = true
rule = "gaussian_transition_score"
plastic_mask = "all_recurrent_edges"
trace_policy = "persistent"
tau_e = 32.0
eta = 0.001
max_update = 0.01
plastic_bound = 0.5
plastic_decay = 0.0
reward_baseline_initial = 0.7
reward_baseline_beta = 0.05
"#,
    )
    .unwrap();
    let state = PlasticState::from_learning_config(&topology, &w0, &learning).unwrap();
    assert_eq!(state.reward_baseline(), 0.7);
    assert_eq!(state.plastic_bound(), 0.5);
    assert_eq!(state.last_feedback(), None);
    assert_eq!(
        FeedbackUpdateParams::from_learning_config(&learning),
        update(0.001, 0.01, 0.05)
    );
    // The M3-01 fixture with baseline 0.5 still matches direct construction.
    let standard: cra::config::Learning = toml::from_str(
        r#"
enabled = true
rule = "gaussian_transition_score"
plastic_mask = "motor_afferent_only"
trace_policy = "persistent"
tau_e = 24.0
eta = 0.001
max_update = 0.01
plastic_bound = 0.5
plastic_decay = 0.0
reward_baseline_initial = 0.5
reward_baseline_beta = 0.02
"#,
    )
    .unwrap();
    let via_config = PlasticState::from_learning_config(&topology, &w0, &standard).unwrap();
    let direct = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        24.0,
        0.5,
    )
    .unwrap();
    assert_eq!(via_config, direct);
}
