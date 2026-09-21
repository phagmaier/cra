//! Plastic offsets, eligibility, and masks (M3-01; spec 7.3, 10.3-10.4, 17.2).
//!
//! - Birth `P = E = 0` with `W_effective = W0` for both masks and both
//!   trace policies; `W0` is never mutated.
//! - `motor_afferent_only` keeps only existing edges into motor neurons;
//!   `all_recurrent_edges` keeps every existing edge; missing edges are
//!   never plastic.
//! - Eligibility uses the receiver's `xi` on all its incoming plastic
//!   edges, one `alpha_h` factor, actual `sigma`, no activation
//!   derivative; zero presynaptic activity gives zero score; zero noise is
//!   rejected even then.
//! - `persistent` decays by `exp(-1 / tau_e)` while `no_decay_diagnostic`
//!   sums exactly (`lambda = 1.0`); the two policies are distinct
//!   configurations, not a large-`tau_e` approximation.
//! - The effective cache is refreshed in one tested location and stays
//!   `W0 + P` (zero on missing); nonzero `P`/`E` on nonplastic edges is
//!   rejected at construction, advance, refresh, and snapshot restore.
//! - Snapshots round-trip through JSON files with versioned validation;
//!   unknown/missing/corrupt/incompatible snapshots fail explicitly.
//! - The actor's effective-weight path shares the `W0` arithmetic core:
//!   with `P = 0` it reproduces `step` bitwise, and with validated `P` it
//!   applies `W0 + P` without touching `B`/biases.

use cra::agent::actor::{ActorState, leak_alpha};
use cra::agent::plasticity::{PLASTIC_SNAPSHOT_SCHEMA_VERSION, PlasticState, PlasticityError};
use cra::agent::topology::{Topology, topology_from_mask};
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::{Actor, Learning};
use cra::rng::{SeedTuple, rng_for};

const PLASTIC_BOUND: f64 = 0.5;

fn small_topology() -> Topology {
    // N = 4, m = 1: M0 = [2], M1 = [3], non-motor [0, 1].
    let mask = vec![
        vec![false, true, false, false],
        vec![true, false, false, false],
        vec![true, false, false, true],
        vec![false, true, true, false],
    ];
    topology_from_mask(4, 1, 0.25, false, mask, "test".to_owned()).expect("mask")
}

fn small_w0(topology: &Topology) -> Vec<Vec<f64>> {
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

fn actor_cfg() -> Actor {
    Actor {
        neuron_count: 4,
        motor_neurons_per_action: 1,
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

fn inherited(topology: Topology, w0: Vec<Vec<f64>>) -> InheritedParams {
    InheritedParams {
        topology,
        weights: Weights {
            w0,
            input_weights: vec![vec![0.0; 1]; 4],
            bias: vec![0.0; 4],
        },
    }
}

fn close(got: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (got - expected).abs() <= tol,
        "{what}: got {got}, expected {expected}"
    );
}

#[test]
fn birth_zero_with_effective_equal_to_w0() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    for mask in ["all_recurrent_edges", "motor_afferent_only"] {
        for trace in ["persistent", "no_decay_diagnostic"] {
            let state =
                PlasticState::new(&topology, &w0, mask, trace, 32.0, PLASTIC_BOUND).expect("birth");
            assert!(state.p().iter().flatten().all(|&v| v == 0.0));
            assert!(state.e().iter().flatten().all(|&v| v == 0.0));
            assert_eq!(state.effective_weights(), &w0);
            assert_eq!(state.neuron_count(), 4);
        }
    }
}

#[test]
fn masks_are_subsets_with_motor_restriction() {
    let topology = small_topology();
    let all = PlasticState::new(
        &topology,
        &small_w0(&topology),
        "all_recurrent_edges",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .expect("all")
    .plastic_mask()
    .to_vec();
    assert_eq!(all, topology.mask);
    let motor = PlasticState::new(
        &topology,
        &small_w0(&topology),
        "motor_afferent_only",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .expect("motor")
    .plastic_mask()
    .to_vec();
    for (j, (topo_row, motor_row)) in topology.mask.iter().zip(motor.iter()).enumerate() {
        for (i, (&structural, &plastic)) in topo_row.iter().zip(motor_row.iter()).enumerate() {
            if !structural {
                assert!(!plastic, "missing ({j},{i}) never plastic");
            } else if j == 2 || j == 3 {
                assert!(plastic, "motor receiver {j} keeps ({j},{i})");
            } else {
                assert!(!plastic, "non-motor receiver {j} not plastic");
            }
        }
    }
    // Stable receiver-grouped order on the plastic edge list.
    let state = PlasticState::new(
        &topology,
        &small_w0(&topology),
        "motor_afferent_only",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let mut sorted = state.plastic_edges().to_vec();
    sorted.sort();
    assert_eq!(state.plastic_edges(), &sorted);
}

#[test]
fn unknown_names_and_bad_tau_are_rejected() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    assert!(matches!(
        PlasticState::new(&topology, &w0, "dense", "persistent", 32.0, PLASTIC_BOUND),
        Err(PlasticityError::UnknownMask(_))
    ));
    assert!(matches!(
        PlasticState::new(
            &topology,
            &w0,
            "all_recurrent_edges",
            "fancy",
            32.0,
            PLASTIC_BOUND,
        ),
        Err(PlasticityError::UnknownTracePolicy(_))
    ));
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            PlasticState::new(
                &topology,
                &w0,
                "all_recurrent_edges",
                "persistent",
                bad,
                PLASTIC_BOUND,
            )
            .is_err(),
            "tau_e {bad}"
        );
        assert!(
            PlasticState::new(
                &topology,
                &w0,
                "all_recurrent_edges",
                "no_decay_diagnostic",
                bad,
                PLASTIC_BOUND,
            )
            .is_err(),
            "diagnostic tau_e {bad}"
        );
    }
    for bad_bound in [0.0, -0.5, f64::NAN, f64::INFINITY] {
        assert!(
            PlasticState::new(
                &topology,
                &w0,
                "all_recurrent_edges",
                "persistent",
                32.0,
                bad_bound,
            )
            .is_err(),
            "plastic_bound {bad_bound}"
        );
    }
    // w0 nonzero on a missing edge is a dimension error, never silent.
    let mut bad_w0 = w0.clone();
    bad_w0[0][0] = 0.5;
    assert!(!topology.mask[0][0]);
    assert!(matches!(
        PlasticState::new(
            &topology,
            &bad_w0,
            "all_recurrent_edges",
            "persistent",
            32.0,
            PLASTIC_BOUND,
        ),
        Err(PlasticityError::DimensionMismatch(_))
    ));
}

#[test]
fn eligibility_shares_receiver_perturbation() {
    // Receiver 2 has two plastic incoming edges (2<-0, 2<-3 in the small
    // mask). One alpha factor, receiver xi, actual sigma, no derivative.
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "no_decay_diagnostic",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let alpha = leak_alpha(5.0);
    let sigma = 0.05;
    let r_old = vec![0.2, -0.1, 0.0, 0.5];
    let xi = vec![0.0, 0.0, 0.4, 0.0];
    state
        .advance_eligibility(&r_old, &xi, alpha, sigma)
        .unwrap();
    let e = state.e();
    // Score = alpha * r_old[i] * xi[j] / sigma on plastic edges only.
    close(e[2][0], alpha * 0.2 * 0.4 / sigma, 1e-15, "E[2,0]");
    close(e[2][3], alpha * 0.5 * 0.4 / sigma, 1e-15, "E[2,3]");
    // Same receiver perturbation scales both edges by their own r_old.
    close(
        e[2][0] / e[2][3],
        0.2 / 0.5,
        1e-12,
        "shared receiver perturbation",
    );
    // Other receivers saw xi = 0, so their scores are zero.
    for (j, i) in [(0, 1), (1, 0), (3, 1), (3, 2)] {
        assert_eq!(e[j][i], 0.0, "E[{j},{i}] with xi=0");
    }
    // Missing edges stay zero.
    assert_eq!(e[0][0], 0.0);
    assert_eq!(e[0][2], 0.0);
}

#[test]
fn zero_presynaptic_gives_zero_score_with_decay() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    // Persistent with known lambda: first accrue, then advance with
    // r_old = 0 so new scores are zero and only decay remains.
    let tau = 16.0;
    let lambda = (-1.0_f64 / tau).exp();
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        tau,
        PLASTIC_BOUND,
    )
    .unwrap();
    let alpha = leak_alpha(5.0);
    state
        .advance_eligibility(&[1.0, 1.0, 1.0, 1.0], &[0.3, -0.2, 0.5, 0.1], alpha, 0.05)
        .unwrap();
    let before = state.e().to_vec();
    state
        .advance_eligibility(&[0.0, 0.0, 0.0, 0.0], &[9.0, 9.0, 9.0, 9.0], alpha, 0.05)
        .unwrap();
    for (j, (e_row, before_row)) in state.e().iter().zip(before.iter()).enumerate() {
        for (i, (&got, &prior)) in e_row.iter().zip(before_row.iter()).enumerate() {
            close(got, lambda * prior, 1e-12, &format!("E[{j},{i}]"));
        }
    }
}

#[test]
fn invalid_score_inputs_are_rejected() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let zeros = vec![0.0; 4];
    // Zero noise is rejected even when activity and perturbation are zero
    // (the score contract is active).
    for bad_sigma in [0.0, -0.05, f64::NAN, f64::INFINITY] {
        assert!(
            state
                .advance_eligibility(&zeros, &zeros, leak_alpha(5.0), bad_sigma)
                .is_err(),
            "sigma {bad_sigma}"
        );
    }
    for bad_alpha in [0.0, -0.1, 1.5, f64::NAN, f64::INFINITY] {
        assert!(
            state
                .advance_eligibility(&zeros, &zeros, bad_alpha, 0.05)
                .is_err(),
            "alpha {bad_alpha}"
        );
    }
    // Shape mismatches fail without touching E.
    let before = state.e().to_vec();
    assert!(
        state
            .advance_eligibility(&[0.0; 3], &zeros, 0.2, 0.05)
            .is_err()
    );
    assert!(
        state
            .advance_eligibility(&zeros, &[0.0; 5], 0.2, 0.05)
            .is_err()
    );
    assert_eq!(state.e(), &before);
}

#[test]
fn missing_and_nonplastic_edges_never_accrue() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let alpha = leak_alpha(5.0);
    for _ in 0..8 {
        state
            .advance_eligibility(&[0.7, -0.4, 0.9, 0.2], &[0.5, -0.6, 0.7, -0.8], alpha, 0.05)
            .unwrap();
    }
    for (j, (((mask_row, topo_row), p_row), e_row)) in state
        .plastic_mask()
        .iter()
        .zip(topology.mask.iter())
        .zip(state.p().iter())
        .zip(state.e().iter())
        .enumerate()
    {
        let eff_row = &state.effective_weights()[j];
        for (i, (((&plastic, &structural), &pv), &ev)) in mask_row
            .iter()
            .zip(topo_row.iter())
            .zip(p_row.iter())
            .zip(e_row.iter())
            .enumerate()
        {
            if !plastic {
                assert_eq!(ev, 0.0, "E[{j},{i}] nonplastic stays zero");
                assert_eq!(pv, 0.0, "P[{j},{i}] nonplastic stays zero");
                if !structural {
                    assert_eq!(eff_row[i], 0.0, "effective[{j},{i}] missing stays zero");
                } else {
                    assert_eq!(
                        eff_row[i], w0[j][i],
                        "effective[{j},{i}] nonplastic equals W0"
                    );
                }
            }
        }
    }
    // Plastic motor-afferent edges did accrue.
    assert!(state.e()[2][0] != 0.0);
    assert!(state.e()[3][2] != 0.0);
}

#[test]
fn persistent_and_no_decay_are_distinct_policies() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    // Spec 17.3 eligibility half: score 0.4 from alpha 0.5, r 0.2, xi 0.4,
    // sigma 0.1; old E 0.3. Persistent with lambda 0.9 gives 0.67;
    // no-decay gives 0.70. Lambda 0.9 is exp(-1/tau) for tau = -1/ln(0.9).
    let tau_09 = -1.0 / 0.9_f64.ln();
    let mut persistent = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        tau_09,
        PLASTIC_BOUND,
    )
    .unwrap();
    let mut diagnostic = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "no_decay_diagnostic",
        tau_09,
        PLASTIC_BOUND,
    )
    .unwrap();
    assert!((persistent.trace_policy().decay_factor() - 0.9).abs() < 1e-12);
    assert_eq!(diagnostic.trace_policy().decay_factor(), 1.0);
    // Seed E[2,0] = 0.3 via snapshot restore on a plastic edge.
    for state in [&mut persistent, &mut diagnostic] {
        let mut snap = state.snapshot();
        snap.p[2][0] = 0.0;
        snap.e[2][0] = 0.3;
        *state = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    }
    // Advance one transition that scores 0.4 exactly on (2 <- 0):
    // r_old[0] = 0.2, xi[2] = 0.4, alpha 0.5, sigma 0.1. Other receivers
    // see xi = 0; other senders see r_old = 0, so only (2,0) accrues.
    let r_old = vec![0.2, 0.0, 0.0, 0.0];
    let xi = vec![0.0, 0.0, 0.4, 0.0];
    persistent
        .advance_eligibility(&r_old, &xi, 0.5, 0.1)
        .unwrap();
    diagnostic
        .advance_eligibility(&r_old, &xi, 0.5, 0.1)
        .unwrap();
    close(persistent.e()[2][0], 0.9 * 0.3 + 0.4, 1e-12, "persistent E");
    close(persistent.e()[2][0], 0.67, 1e-12, "persistent golden 0.67");
    close(diagnostic.e()[2][0], 0.3 + 0.4, 1e-15, "diagnostic E");
    assert_ne!(persistent.e()[2][0], diagnostic.e()[2][0]);
}

#[test]
fn w0_is_never_mutated_by_advance_or_refresh() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let frozen = w0.clone();
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    state
        .advance_eligibility(&[0.5; 4], &[0.5; 4], leak_alpha(5.0), 0.05)
        .unwrap();
    state.refresh_effective(&w0).unwrap();
    assert_eq!(w0, frozen, "caller W0 slice unchanged");
    assert_eq!(
        state.effective_weights(),
        &frozen,
        "P = 0 so effective == W0"
    );
}

#[test]
fn refresh_is_the_single_cache_location_with_plastic_offsets() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    // Install nonzero P on plastic edges only via validated restore.
    let mut snap = state.snapshot();
    snap.p[2][0] = 0.05;
    snap.p[3][2] = -0.03;
    snap.e[2][0] = 0.1;
    state = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    assert_eq!(state.effective_weights()[2][0], w0[2][0] + 0.05);
    assert_eq!(state.effective_weights()[3][2], w0[3][2] - 0.03);
    assert_eq!(state.effective_weights()[0][1], w0[0][1]);
    // Missing entries stay exactly zero.
    assert_eq!(state.effective_weights()[0][0], 0.0);
    // Nonzero P on a nonplastic edge is rejected (motor-only view).
    let mut motor = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let mut bad = motor.snapshot();
    bad.p[0][1] = 0.01;
    assert!(!motor.plastic_mask()[0][1]);
    assert!(PlasticState::restore(bad, &topology, &w0, 0.5).is_err());
    let _ = &mut motor;
}

#[test]
fn snapshot_round_trips_through_a_file() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let mut state = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        24.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    state
        .advance_eligibility(
            &[0.4, 0.1, -0.2, 0.3],
            &[0.2, 0.1, -0.4, 0.6],
            leak_alpha(5.0),
            0.05,
        )
        .unwrap();
    let snapshot = state.snapshot();
    assert_eq!(snapshot.schema_version, PLASTIC_SNAPSHOT_SCHEMA_VERSION);
    let path = std::env::temp_dir().join(format!("cra-plastic-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec_pretty(&snapshot).unwrap()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let loaded: cra::agent::plasticity::PlasticSnapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(loaded, snapshot);
    let restored = PlasticState::restore(loaded, &topology, &w0, 0.5).unwrap();
    assert_eq!(restored, state);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn snapshot_rejections_are_explicit() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let state = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let good = state.snapshot();

    // Wrong schema version.
    let mut bad = good.clone();
    bad.schema_version = 999;
    assert!(matches!(
        PlasticState::restore(bad, &topology, &w0, 0.5),
        Err(PlasticityError::Incompatible(_))
    ));
    // Dimension mismatch.
    let mut bad = good.clone();
    bad.p.pop();
    assert!(PlasticState::restore(bad, &topology, &w0, 0.5).is_err());
    // Mask-name mismatch: snapshot claims motor-only but carries P on a
    // non-motor edge valid under all-recurrent.
    let mut bad = good.clone();
    bad.p[0][1] = 0.02;
    bad.plastic_mask = "motor_afferent_only".to_owned();
    assert!(!topology.mask[0][1] || bad.p[0][1] != 0.0);
    assert!(PlasticState::restore(bad, &topology, &w0, 0.5).is_err());
    // Unknown trace policy and bad tau.
    let mut bad = good.clone();
    bad.trace_policy = "fancy".to_owned();
    assert!(PlasticState::restore(bad, &topology, &w0, 0.5).is_err());
    let mut bad = good.clone();
    bad.tau_e = 0.0;
    assert!(PlasticState::restore(bad, &topology, &w0, 0.5).is_err());
    // Unknown JSON fields are rejected, never defaulted.
    let mut value = serde_json::to_value(&good).unwrap();
    value["future_gates"] = serde_json::json!([1.0]);
    assert!(serde_json::from_value::<cra::agent::plasticity::PlasticSnapshot>(value).is_err());
    // Missing required field is rejected.
    let mut value = serde_json::to_value(&good).unwrap();
    value.as_object_mut().unwrap().remove("p");
    assert!(serde_json::from_value::<cra::agent::plasticity::PlasticSnapshot>(value).is_err());
    // A wire-format schema-2 snapshot lacks the schema-3 bound and cannot be
    // silently upgraded with a default.
    let mut value = serde_json::to_value(&good).unwrap();
    value["schema_version"] = serde_json::json!(2);
    value.as_object_mut().unwrap().remove("plastic_bound");
    assert!(serde_json::from_value::<cra::agent::plasticity::PlasticSnapshot>(value).is_err());
    // Nonfinite P/E is corrupt, not a successful snapshot.
    let mut bad = good.clone();
    bad.e[2][0] = f64::NAN;
    assert!(matches!(
        PlasticState::restore(bad, &topology, &w0, 0.5),
        Err(PlasticityError::Corrupt(_))
    ));
    // The snapshot's declared bound must match the resolved configuration,
    // and every restored offset must already satisfy it.
    let mut bad = good.clone();
    bad.plastic_bound = 1.0;
    assert!(matches!(
        PlasticState::restore(bad, &topology, &w0, 0.5),
        Err(PlasticityError::Incompatible(_))
    ));
    let mut bad = good;
    bad.p[2][0] = 0.500_000_000_1;
    assert!(matches!(
        PlasticState::restore(bad, &topology, &w0, 0.5),
        Err(PlasticityError::Corrupt(_))
    ));
}

#[test]
fn effective_path_matches_w0_path_when_p_is_zero() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let params = inherited(topology.clone(), w0.clone());
    let actor = actor_cfg();
    let plastic = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let seed = SeedTuple::new(1, "development", 7, 0, "actor_noise");
    let mut rng_a = rng_for(&seed).unwrap();
    let mut rng_b = rng_for(&seed).unwrap();
    let mut a = ActorState::new(4).unwrap();
    let mut b = ActorState::new(4).unwrap();
    let input = vec![0.5];
    for _ in 0..6 {
        a.step(&actor, &params, &input, &mut rng_a).unwrap();
        b.step_with_effective_weights(
            &actor,
            &params,
            plastic.effective_weights(),
            &input,
            &mut rng_b,
        )
        .unwrap();
        assert_eq!(a.h(), b.h(), "h must match bitwise when P = 0");
        assert_eq!(a.a(), b.a());
        assert_eq!(a.r(), b.r());
        assert_eq!(a.last_perturbations(), b.last_perturbations());
    }
    assert_eq!(params.weights.w0, w0, "W0 unchanged by either path");
}

#[test]
fn effective_path_applies_validated_offsets_without_touching_inputs() {
    // One plastic edge 0 -> 1 in a 2-neuron network; P = 0.1 there must
    // shift the receiver drive by exactly P * r_old through the shared
    // core, while B/bias contributions stay identical.
    let topology = topology_from_mask(
        2,
        1,
        0.5,
        false,
        vec![vec![false, false], vec![true, false]],
        "t".into(),
    )
    .unwrap();
    let w0 = vec![vec![0.0, 0.0], vec![0.6, 0.0]];
    let params = InheritedParams {
        topology: topology.clone(),
        weights: Weights {
            w0: w0.clone(),
            input_weights: vec![vec![0.4], vec![-0.2]],
            bias: vec![0.05, -0.07],
        },
    };
    let actor = Actor {
        neuron_count: 2,
        motor_neurons_per_action: 1,
        edge_probability: 0.5,
        self_edges: false,
        recurrent_gain: 0.8,
        input_scale: 0.3,
        tau_h: 5.0,
        tau_a: 100.0,
        adaptation_strength: 0.0,
        noise_sigma: 0.05,
        motor_filter_tau: 3.0,
    };
    let mut plastic = PlasticState::new(
        &topology,
        &w0,
        "all_recurrent_edges",
        "persistent",
        32.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    let mut snap = plastic.snapshot();
    snap.p[1][0] = 0.1;
    plastic = PlasticState::restore(snap, &topology, &w0, 0.5).unwrap();
    let mut base = ActorState::from_state(vec![0.5, -0.3], vec![0.0, 0.0]).unwrap();
    let mut shifted = ActorState::from_state(vec![0.5, -0.3], vec![0.0, 0.0]).unwrap();
    let input = vec![1.0];
    let xi = vec![0.0, 0.0];
    base.step_with_perturbations(&actor, &params, &input, &xi)
        .unwrap();
    shifted
        .step_with_effective_and_perturbations(
            &actor,
            &params,
            plastic.effective_weights(),
            &input,
            &xi,
        )
        .unwrap();
    let alpha = leak_alpha(5.0);
    let r0 = 0.5_f64.tanh();
    // Only receiver 1 sees the +0.1 offset through r_old[0].
    close(
        shifted.h()[1] - base.h()[1],
        alpha * 0.1 * r0,
        1e-12,
        "offset drive",
    );
    close(shifted.h()[0], base.h()[0], 1e-15, "sender unchanged");
}

#[test]
fn effective_path_validates_missing_and_ragged_weights() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let params = inherited(topology, w0.clone());
    let actor = actor_cfg();
    let mut state = ActorState::new(4).unwrap();
    let seed = SeedTuple::new(1, "development", 3, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    // Nonzero on a missing edge is rejected.
    let mut bad = w0.clone();
    bad[0][0] = 0.25;
    assert!(
        state
            .step_with_effective_weights(&actor, &params, &bad, &[0.0], &mut rng)
            .is_err()
    );
    // Ragged and nonfinite matrices are rejected.
    let mut ragged = w0.clone();
    ragged[0].pop();
    assert!(
        state
            .step_with_effective_weights(&actor, &params, &ragged, &[0.0], &mut rng)
            .is_err()
    );
    let mut nonfinite = w0.clone();
    nonfinite[2][0] = f64::INFINITY;
    assert!(
        state
            .step_with_effective_weights(&actor, &params, &nonfinite, &[0.0], &mut rng)
            .is_err()
    );
}

#[test]
fn from_learning_config_uses_toml_strings() {
    let topology = small_topology();
    let w0 = small_w0(&topology);
    let learning: Learning = toml::from_str(
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
    let via_config = PlasticState::from_learning_config(&topology, &w0, &learning).unwrap();
    let direct = PlasticState::new(
        &topology,
        &w0,
        "motor_afferent_only",
        "persistent",
        24.0,
        PLASTIC_BOUND,
    )
    .unwrap();
    assert_eq!(via_config, direct);
    // The shipped debug profile carries the main mask/trace pair.
    let debug: cra::config::Config =
        toml::from_str(&std::fs::read_to_string("configs/debug_stationary.toml").unwrap()).unwrap();
    let learning = debug.learning.as_ref().unwrap();
    assert_eq!(learning.plastic_mask, "all_recurrent_edges");
    assert_eq!(learning.trace_policy, "persistent");
    let _ = PlasticState::from_learning_config(&topology, &w0, learning).unwrap();
}
