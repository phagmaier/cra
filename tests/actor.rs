//! Actor transition contracts (M1-03).
//!
//! - A one-edge fixture proves `W[receiver, sender]` orientation: the
//!   receiver gains the drive term while the sender's update shows no
//!   reverse contribution, even with large activity on the other neuron.
//! - A two-neuron bidirectional fixture proves simultaneous updates: the
//!   expected values use old activity only, so in-place updates would
//!   visibly mismatch.
//! - Forced perturbations at two leak rates prove noise is added after
//!   leaky integration (`sigma * xi`, not scaled by `alpha_h`).
//! - A saturated-state fixture proves no hidden clipping of `h`.
//! - Dimension, parameter, and nonfinite violations are explicit errors.

use cra::agent::actor::{ActorError, ActorState, leak_alpha};
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::Actor;
use cra::rng::{SeedTuple, rng_for};

fn actor_with(tau_h: f64, strength: f64, sigma: f64) -> Actor {
    Actor {
        neuron_count: 2,
        motor_neurons_per_action: 1,
        edge_probability: 0.5,
        self_edges: false,
        recurrent_gain: 0.8,
        input_scale: 0.3,
        tau_h,
        tau_a: 100.0,
        adaptation_strength: strength,
        noise_sigma: sigma,
        motor_filter_tau: 3.0,
    }
}

fn params_from(mask: Vec<Vec<bool>>, w0: Vec<Vec<f64>>) -> InheritedParams {
    let topology = topology_from_mask(2, 1, 0.5, false, mask, "fixture".to_owned()).expect("mask");
    InheritedParams {
        topology,
        weights: Weights {
            w0,
            input_weights: vec![vec![0.0; 1]; 2],
            bias: vec![0.0; 2],
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
fn one_edge_drives_receiver_not_sender() {
    // Single edge 0 -> 1 with weight 0.6; neuron 1 carries large activity
    // that must NOT leak backwards into neuron 0.
    let params = params_from(
        vec![vec![false, false], vec![true, false]],
        vec![vec![0.0, 0.0], vec![0.6, 0.0]],
    );
    let actor = actor_with(5.0, 0.5, 0.05);
    let mut state = ActorState::from_state(vec![0.5, 0.7], vec![0.1, -0.2]).expect("state");
    state
        .step_with_perturbations(&actor, &params, &[0.0], &[0.0, 0.0])
        .expect("step");

    let (r0, r1) = (0.5_f64.tanh(), 0.7_f64.tanh());
    let alpha_h = leak_alpha(5.0);
    let alpha_a = leak_alpha(100.0);
    // drive[0] = -strength * a0 (no incoming edges); drive[1] adds 0.6 * r0.
    let drive0 = -0.5 * 0.1;
    let drive1 = -0.5 * -0.2 + 0.6 * r0;
    close(
        state.h()[0],
        (1.0 - alpha_h) * 0.5 + alpha_h * drive0,
        1e-12,
        "h0",
    );
    close(
        state.h()[1],
        (1.0 - alpha_h) * 0.7 + alpha_h * drive1,
        1e-12,
        "h1",
    );
    close(
        state.a()[0],
        (1.0 - alpha_a) * 0.1 + alpha_a * r0,
        1e-12,
        "a0",
    );
    close(
        state.a()[1],
        (1.0 - alpha_a) * -0.2 + alpha_a * r1,
        1e-12,
        "a1",
    );
    close(state.r()[0], state.h()[0].tanh(), 1e-15, "r0");
    close(state.r()[1], state.h()[1].tanh(), 1e-15, "r1");
}

#[test]
fn reciprocal_edges_use_old_activity_only() {
    // Edges both ways: with in-place updates, neuron 1 would see neuron
    // 0's already-updated activity. Hand values use old r throughout.
    let params = params_from(
        vec![vec![false, true], vec![true, false]],
        vec![vec![0.0, -0.4], vec![0.6, 0.0]],
    );
    let actor = actor_with(5.0, 0.0, 0.05);
    let mut state = ActorState::from_state(vec![0.3, -0.4], vec![0.0, 0.0]).expect("state");
    state
        .step_with_perturbations(&actor, &params, &[0.0], &[0.0, 0.0])
        .expect("step");

    let (r0, r1) = (0.3_f64.tanh(), (-0.4_f64).tanh());
    let alpha_h = leak_alpha(5.0);
    close(
        state.h()[0],
        (1.0 - alpha_h) * 0.3 + alpha_h * (-0.4 * r1),
        1e-12,
        "h0",
    );
    close(
        state.h()[1],
        (1.0 - alpha_h) * -0.4 + alpha_h * (0.6 * r0),
        1e-12,
        "h1",
    );
    // Adaptation still tracks old activity with zero strength drive.
    let alpha_a = leak_alpha(100.0);
    close(state.a()[0], alpha_a * r0, 1e-12, "a0");
    close(state.a()[1], alpha_a * r1, 1e-12, "a1");
}

#[test]
fn noise_is_added_after_leaky_integration() {
    // Empty graph isolates the leak/noise interplay. At tau_h = 100 the
    // correct sigma*xi term is ~100x the alpha_h*sigma*xi mistake.
    let params = params_from(
        vec![vec![false, false], vec![false, false]],
        vec![vec![0.0, 0.0], vec![0.0, 0.0]],
    );
    let mut state = ActorState::from_state(vec![0.25, -0.1], vec![0.0, 0.0]).expect("state");
    state
        .step_with_perturbations(&actor_with(100.0, 0.0, 0.05), &params, &[0.0], &[2.0, 0.0])
        .expect("step");
    let alpha_slow = leak_alpha(100.0);
    close(
        state.h()[0],
        (1.0 - alpha_slow) * 0.25 + 0.05 * 2.0,
        1e-12,
        "slow h0",
    );
    close(state.h()[1], (1.0 - alpha_slow) * -0.1, 1e-12, "slow h1");

    let mut state = ActorState::from_state(vec![0.25, -0.1], vec![0.0, 0.0]).expect("state");
    state
        .step_with_perturbations(&actor_with(5.0, 0.0, 0.05), &params, &[0.0], &[0.0, -1.5])
        .expect("step");
    let alpha_fast = leak_alpha(5.0);
    close(state.h()[0], (1.0 - alpha_fast) * 0.25, 1e-12, "fast h0");
    close(
        state.h()[1],
        (1.0 - alpha_fast) * -0.1 + 0.05 * -1.5,
        1e-12,
        "fast h1",
    );
}

#[test]
fn saturated_membrane_is_never_clipped() {
    let params = params_from(
        vec![vec![false, false], vec![false, false]],
        vec![vec![0.0, 0.0], vec![0.0, 0.0]],
    );
    let mut state = ActorState::from_state(vec![500.0, -500.0], vec![0.0, 0.0]).expect("state");
    state
        .step_with_perturbations(&actor_with(5.0, 0.0, 0.05), &params, &[0.0], &[0.0, 0.0])
        .expect("step");
    let alpha_h = leak_alpha(5.0);
    let expected = (1.0 - alpha_h) * 500.0;
    assert!(
        (state.h()[0] - expected).abs() / expected < 1e-12,
        "large h must leak exactly, got {}",
        state.h()[0]
    );
    assert_eq!(state.r()[0], 1.0);
    assert_eq!(state.r()[1], -1.0);
}

#[test]
fn stochastic_step_shares_the_fixture_core() {
    let params = params_from(
        vec![vec![false, true], vec![true, false]],
        vec![vec![0.0, -0.4], vec![0.6, 0.0]],
    );
    let actor = actor_with(5.0, 0.0, 0.05);
    let seed = SeedTuple::new(1, "development", 1, 0, "actor_noise");
    let mut rng = rng_for(&seed).expect("rng");
    let mut state = ActorState::from_state(vec![0.3, -0.4], vec![0.0, 0.0]).expect("state");
    state.step(&actor, &params, &[0.0], &mut rng).expect("step");
    // Same invariants as the fixture path: fresh tanh activity, moved state.
    close(state.r()[0], state.h()[0].tanh(), 1e-15, "r0");
    close(state.r()[1], state.h()[1].tanh(), 1e-15, "r1");
    assert_ne!(state.h(), &[0.3, -0.4]);
}

#[test]
fn birth_state_is_zero_and_constructors_validate() {
    let zero = ActorState::new(2).expect("birth");
    assert_eq!(zero.h(), &[0.0, 0.0]);
    assert_eq!(zero.a(), &[0.0, 0.0]);
    assert_eq!(zero.r(), &[0.0, 0.0]);
    assert!(matches!(
        ActorState::new(0),
        Err(ActorError::DimensionMismatch(_))
    ));
    assert!(matches!(
        ActorState::from_state(vec![0.0], vec![0.0, 0.0]),
        Err(ActorError::DimensionMismatch(_))
    ));
    assert!(matches!(
        ActorState::from_state(vec![f64::INFINITY], vec![0.0]),
        Err(ActorError::NonFiniteState(_))
    ));
}

#[test]
fn step_rejects_bad_dims_params_and_nonfinite_values() {
    let params = params_from(
        vec![vec![false, false], vec![false, false]],
        vec![vec![0.0, 0.0], vec![0.0, 0.0]],
    );
    let actor = actor_with(5.0, 0.0, 0.05);

    let mut state = ActorState::new(2).expect("state");
    assert!(matches!(
        state.step_with_perturbations(&actor, &params, &[0.0, 0.0], &[0.0, 0.0]),
        Err(ActorError::DimensionMismatch(_))
    ));
    assert!(matches!(
        state.step_with_perturbations(&actor, &params, &[0.0], &[0.0]),
        Err(ActorError::DimensionMismatch(_))
    ));
    assert!(matches!(
        state.step_with_perturbations(&actor, &params, &[0.0], &[f64::INFINITY, 0.0]),
        Err(ActorError::NonFiniteState(_))
    ));
    assert!(matches!(
        state.step_with_perturbations(&actor, &params, &[f64::NAN], &[0.0, 0.0]),
        Err(ActorError::NonFiniteState(_))
    ));

    let bad_tau = actor_with(0.0, 0.0, 0.05);
    assert!(matches!(
        state.step_with_perturbations(&bad_tau, &params, &[0.0], &[0.0, 0.0]),
        Err(ActorError::InvalidParams(_))
    ));

    let mut wide = ActorState::new(3).expect("state");
    assert!(matches!(
        wide.step_with_perturbations(&actor, &params, &[0.0], &[0.0, 0.0, 0.0]),
        Err(ActorError::DimensionMismatch(_))
    ));
}
