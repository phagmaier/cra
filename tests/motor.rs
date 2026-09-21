//! Motor readout contracts (M1-06).
//!
//! - Golden filter recurrences use an independent `1 - exp` formulation
//!   for the leak factor, so the implementation cannot agree with itself
//!   by sharing one helper.
//! - Pool means read the assigned populations: swapped pools would flip
//!   the golden values.
//! - Commitment reads the new `q`: a stale filter state pointing the
//!   other way must not decide.
//! - Exact ties draw exactly one fair coin from the `tie_break` stream;
//!   strict decisions consume nothing — the structural proof that no
//!   epsilon-greedy or softmax draw happens per call.
//! - Empty, out-of-range, and overlapping pools plus bad constants fail
//!   explicitly.

use rand::Rng;

use cra::agent::motor::{MotorError, MotorState, decide_action};
use cra::agent::topology::motor_pools;
use cra::environment::MotorOutput;
use cra::rng::{SeedTuple, rng_for};

/// Leak factor from the independent formulation (not the shared helper).
fn alpha_independent(tau: f64) -> f64 {
    1.0 - (-1.0 / tau).exp()
}

fn tie_seed() -> SeedTuple {
    SeedTuple::new(1, "development", 1, 0, "tie_break")
}

#[test]
fn golden_filter_recurrence_from_zero() {
    // tau_q = 3, q_old = 0, means [0.5, -0.25]: q_new = alpha * mean.
    let mut motor = MotorState::new();
    let out = motor
        .update(3.0, &[0.5, 0.5, -0.25, -0.25], &[0, 1], &[2, 3])
        .expect("update");
    let alpha = alpha_independent(3.0);
    assert!((out.action_0 - alpha * 0.5).abs() < 1e-12);
    assert!((out.action_1 - alpha * -0.25).abs() < 1e-12);
    assert_eq!(motor.q(), [out.action_0, out.action_1]);
}

#[test]
fn golden_filter_trajectory_over_three_ticks() {
    // Fixed means [0.5, -0.25] for three ticks, hand-rolled recurrence.
    let mut motor = MotorState::new();
    let alpha = alpha_independent(3.0);
    let mut expected = [0.0, 0.0];
    for _ in 0..3 {
        expected[0] = (1.0 - alpha) * expected[0] + alpha * 0.5;
        expected[1] = (1.0 - alpha) * expected[1] + alpha * -0.25;
        let out = motor
            .update(3.0, &[0.5, 0.5, -0.25, -0.25], &[0, 1], &[2, 3])
            .expect("update");
        assert!((out.action_0 - expected[0]).abs() < 1e-12);
        assert!((out.action_1 - expected[1]).abs() < 1e-12);
    }
}

#[test]
fn pool_means_read_assigned_populations() {
    // r = [0.1, 0.2, 0.3, 0.4]: M0 mean 0.15, M1 mean 0.35. Swapped pools
    // would exchange the outputs.
    let mut motor = MotorState::new();
    let out = motor
        .update(3.0, &[0.1, 0.2, 0.3, 0.4], &[0, 1], &[2, 3])
        .expect("update");
    let alpha = alpha_independent(3.0);
    assert!((out.action_0 - alpha * 0.15).abs() < 1e-12);
    assert!((out.action_1 - alpha * 0.35).abs() < 1e-12);
}

#[test]
fn commitment_reads_new_q_not_stale_q() {
    // Stale q favors action 0, but fresh activity (means [-1, 1]) flips the
    // new readout to action 1: the decision must follow the new values.
    let mut motor = MotorState::from_q([0.1, 0.0]).expect("q");
    let out = motor
        .update(3.0, &[-1.0, -1.0, 1.0, 1.0], &[0, 1], &[2, 3])
        .expect("update");
    assert!(out.action_1 > out.action_0);
    let mut rng = rng_for(&tie_seed()).expect("rng");
    assert_eq!(decide_action(out, &mut rng), 1);
}

#[test]
fn strict_decisions_draw_nothing_and_ties_draw_one_coin() {
    let mut rng = rng_for(&tie_seed()).expect("rng");
    let pos = rng.get_word_pos();
    assert_eq!(
        decide_action(
            MotorOutput {
                action_0: 0.5,
                action_1: 0.1
            },
            &mut rng
        ),
        0
    );
    assert_eq!(
        decide_action(
            MotorOutput {
                action_0: 0.1,
                action_1: 0.5
            },
            &mut rng
        ),
        1
    );
    // Two strict decisions must not advance the tie stream at all: this is
    // the structural proof of no per-call exploration draw.
    assert_eq!(rng.get_word_pos(), pos);

    // An exact tie draws exactly one fair coin from the tie stream.
    let mut rng_a = rng_for(&tie_seed()).expect("rng");
    let mut rng_b = rng_for(&tie_seed()).expect("rng");
    let tied = MotorOutput {
        action_0: 0.3,
        action_1: 0.3,
    };
    let action = decide_action(tied, &mut rng_a);
    let expected = u8::from(rng_b.random_bool(0.5));
    assert_eq!(action, expected);
    assert!(rng_a.get_word_pos() > pos);
    // Same seed, same tie: same action (deterministic apart from ties,
    // and ties resolve deterministically per seed).
    let mut rng_c = rng_for(&tie_seed()).expect("rng");
    assert_eq!(decide_action(tied, &mut rng_c), action);
}

#[test]
fn invalid_pools_constants_and_activity_rejected() {
    let mut motor = MotorState::new();
    let r = [0.0, 0.0, 0.0, 0.0];
    assert!(matches!(
        motor.update(3.0, &r, &[], &[2, 3]),
        Err(MotorError::DimensionMismatch(_))
    ));
    assert!(matches!(
        motor.update(3.0, &r, &[0, 9], &[2, 3]),
        Err(MotorError::DimensionMismatch(_))
    ));
    assert!(matches!(
        motor.update(3.0, &r, &[0, 2], &[2, 3]),
        Err(MotorError::DimensionMismatch(_))
    ));
    assert!(matches!(
        motor.update(0.0, &r, &[0, 1], &[2, 3]),
        Err(MotorError::InvalidParams(_))
    ));
    assert!(matches!(
        motor.update(f64::NAN, &r, &[0, 1], &[2, 3]),
        Err(MotorError::InvalidParams(_))
    ));
    assert!(matches!(
        motor.update(3.0, &[0.0, f64::INFINITY, 0.0, 0.0], &[0, 1], &[2, 3]),
        Err(MotorError::InvalidParams(_))
    ));
}

#[test]
fn debug_profile_pools_match_topology_assignment() {
    // The readout must use the same last-index assignment the topology
    // sampler fixes: N = 16, m = 2 -> [12, 13] / [14, 15].
    let text = std::fs::read_to_string("configs/debug_stationary.toml").expect("debug profile");
    let cfg: cra::config::Config = toml::from_str(&text).expect("debug parses");
    let actor = cfg.actor.expect("debug has an actor section");
    let (m0, m1) = motor_pools(actor.neuron_count, actor.motor_neurons_per_action).expect("pools");
    assert_eq!(m0, vec![12, 13]);
    assert_eq!(m1, vec![14, 15]);
    let mut motor = MotorState::new();
    let r = [0.1; 16];
    let out = motor
        .update(actor.motor_filter_tau, &r, &m0, &m1)
        .expect("update");
    assert!(out.action_0.is_finite() && out.action_1.is_finite());
}

#[test]
fn motor_pools_are_equal_sized_sets() {
    let mut state = MotorState::new();
    for (m0, m1) in [(vec![0, 0], vec![2, 3]), (vec![0], vec![2, 3])] {
        assert!(state.update(3.0, &[0.2; 4], &m0, &m1).is_err());
        assert_eq!(state.q(), [0.0, 0.0]);
    }
}
