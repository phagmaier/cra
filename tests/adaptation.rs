//! Adaptation contracts (M1-05).
//!
//! - At strength zero, adaptation state evolves but never feeds back into
//!   the drive: identical membranes stay identical regardless of `a_old`.
//! - Adaptation runs on its own time constant (`tau_a`, not `tau_h`).
//! - At nonzero strength, adaptation opposes sustained activity with the
//!   specified sign, shown against a paired zero-strength control — the
//!   nonzero case is a unit fixture only, not the initial learner.
//! - Adaptation persists across input changes: replaying its recurrence
//!   from recorded activity reproduces it exactly, so no boundary clears
//!   it.
//! - The shipped `debug_stationary` profile keeps strength at zero.

use cra::agent::actor::{ActorState, leak_alpha};
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::Actor;

fn actor_with(strength: f64, tau_a: f64) -> Actor {
    Actor {
        neuron_count: 2,
        motor_neurons_per_action: 1,
        edge_probability: 0.0,
        self_edges: false,
        recurrent_gain: 0.8,
        input_scale: 0.3,
        tau_h: 5.0,
        tau_a,
        adaptation_strength: strength,
        noise_sigma: 0.05,
        motor_filter_tau: 3.0,
    }
}

fn params_with_b(b: Vec<Vec<f64>>) -> InheritedParams {
    let n = 2;
    let mask = vec![vec![false; n]; n];
    let topology =
        topology_from_mask(n, 1, 0.0, false, mask, "adaptation-fixture".to_owned()).expect("mask");
    InheritedParams {
        topology,
        weights: Weights {
            w0: vec![vec![0.0; n]; n],
            input_weights: b,
            bias: vec![0.0; n],
        },
    }
}

#[test]
fn zero_strength_ignores_adaptation_state() {
    // Same membranes, wildly different adaptation: with strength 0 the
    // drive term -0 * a_old vanishes, so h must evolve bit-identically
    // while a itself still tracks activity.
    let actor = actor_with(0.0, 100.0);
    let params = params_with_b(vec![vec![0.0; 1]; 2]);
    let mut loaded = ActorState::from_state(vec![0.4, -0.3], vec![2.5, -3.0]).expect("state");
    let mut cleared = ActorState::from_state(vec![0.4, -0.3], vec![0.0, 0.0]).expect("state");
    loaded
        .step_with_perturbations(&actor, &params, &[0.0], &[0.0, 0.0])
        .expect("step");
    cleared
        .step_with_perturbations(&actor, &params, &[0.0], &[0.0, 0.0])
        .expect("step");
    assert_eq!(loaded.h(), cleared.h(), "strength 0 must hide a_old from h");
    assert_eq!(loaded.r(), cleared.r());
    assert_ne!(loaded.a(), cleared.a(), "a itself still evolves");
}

#[test]
fn adaptation_runs_on_its_own_time_constant() {
    // One tick from a = 0 must use alpha(tau_a), not alpha(tau_h): the two
    // differ ~18x at (5, 100), so a swap cannot hide.
    let actor = actor_with(0.0, 100.0);
    let params = params_with_b(vec![vec![0.0; 1]; 2]);
    let mut state = ActorState::from_state(vec![1.0, 0.5], vec![0.0, 0.0]).expect("state");
    state
        .step_with_perturbations(&actor, &params, &[0.0], &[0.0, 0.0])
        .expect("step");
    let alpha_a = leak_alpha(100.0);
    let alpha_h = leak_alpha(5.0);
    let r_old = [1.0_f64.tanh(), 0.5_f64.tanh()];
    for (j, &r) in r_old.iter().enumerate() {
        let straightforward = alpha_a * r;
        assert!(
            (state.a()[j] - straightforward).abs() < 1e-15,
            "a[{j}] must follow tau_a"
        );
        let swapped = alpha_h * r;
        assert!(
            (state.a()[j] - swapped).abs() > 0.5 * (straightforward - swapped).abs(),
            "a[{j}] must not follow tau_h"
        );
    }
}

#[test]
fn nonzero_strength_opposes_sustained_activity() {
    // Unit fixture only (fast tau_a = 2 for a short run): positive activity
    // builds positive adaptation, whose subtracted drive pulls h below the
    // paired zero-strength control every tick.
    let adapted_cfg = actor_with(0.5, 2.0);
    let control_cfg = actor_with(0.0, 2.0);
    let params = params_with_b(vec![vec![0.0; 1]; 2]);
    let mut adapted = ActorState::from_state(vec![1.0, 1.0], vec![0.0, 0.0]).expect("state");
    let mut control = ActorState::from_state(vec![1.0, 1.0], vec![0.0, 0.0]).expect("state");
    let mut last_h = 1.0;
    let mut first_a = 0.0;
    for t in 0..12 {
        adapted
            .step_with_perturbations(&adapted_cfg, &params, &[0.0], &[0.0, 0.0])
            .expect("step");
        control
            .step_with_perturbations(&control_cfg, &params, &[0.0], &[0.0, 0.0])
            .expect("step");
        // Sign: sustained positive activity builds positive adaptation.
        assert!(
            adapted.a()[0] > 0.0,
            "tick {t}: sustained activity builds +a"
        );
        // Opposition: from the first tick with nonzero a onward, the
        // subtracted drive pulls h below the paired control.
        if t >= 1 {
            assert!(
                adapted.h()[0] < control.h()[0],
                "tick {t}: adaptation opposes h"
            );
        }
        assert!(adapted.h()[0] < last_h, "tick {t}: h keeps falling");
        last_h = adapted.h()[0];
        if t == 0 {
            first_a = adapted.a()[0];
        }
        if t == 1 {
            assert!(adapted.a()[0] > first_a, "adaptation builds up while a < r");
        }
    }
    // Substantial opposition with the observed signature: adaptation drives
    // h below zero while the leak-only control is still clearly positive.
    assert!(
        adapted.h()[0] < 0.0,
        "adapted h overshoots, got {}",
        adapted.h()[0]
    );
    assert!(
        control.h()[0] > 0.05,
        "control still positive, got {}",
        control.h()[0]
    );
}

#[test]
fn adaptation_persists_across_input_changes() {
    // Inputs cycle through four patterns with a live sensory projection.
    // Replaying a_new = (1-alpha_a) a_old + alpha_a r_old from the recorded
    // activity reproduces a exactly, proving no input change clears it.
    let actor = actor_with(0.5, 20.0);
    let params = params_with_b(vec![vec![0.4, 0.0], vec![0.0, 0.4]]);
    let mut state = ActorState::from_state(vec![0.2, -0.1], vec![0.0, 0.0]).expect("state");
    let patterns = [
        vec![1.0, 0.0],
        vec![0.0, 1.0],
        vec![0.0, 0.0],
        vec![1.0, 1.0],
    ];
    let alpha_a = leak_alpha(20.0);
    let mut expected_a = [0.0, 0.0];
    for (t, input) in patterns.iter().cycle().take(10).enumerate() {
        let r_old = state.r().to_vec();
        state
            .step_with_perturbations(&actor, &params, input, &[0.0, 0.0])
            .expect("step");
        for j in 0..2 {
            expected_a[j] = (1.0 - alpha_a) * expected_a[j] + alpha_a * r_old[j];
            assert!(
                (state.a()[j] - expected_a[j]).abs() < 1e-12,
                "tick {t}: a[{j}] must equal the unbroken recurrence"
            );
        }
    }
}

#[test]
fn debug_profile_keeps_adaptation_at_zero() {
    // The initial learner must not be complicated by this mechanism: the
    // shipped debug profile holds strength 0 (spec 6.2, 6.3).
    let text = std::fs::read_to_string("configs/debug_stationary.toml").expect("debug profile");
    let cfg: cra::config::Config = toml::from_str(&text).expect("debug parses");
    let actor = cfg.actor.expect("debug has an actor section");
    assert_eq!(actor.adaptation_strength, 0.0);
}
