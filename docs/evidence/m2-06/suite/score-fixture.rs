//! M2-01 arithmetic contracts only. Derivative and Monte Carlo diagnostics
//! are separate later M2 tasks; these fixtures make no learning claim.

use cra::agent::actor::{ActorState, leak_alpha};
use cra::agent::score::{ScoreError, conditional_score};

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-14,
        "score {actual} != {expected}"
    );
}

#[test]
fn spec_golden_score_has_one_leak_factor_and_actual_sigma() {
    // Spec 17.3: no trace, reward, gate or update is involved here.
    close(conditional_score(0.5, 0.2, 0.4, 0.1).unwrap(), 0.4);
    close(conditional_score(0.25, 0.2, 0.4, 0.1).unwrap(), 0.2);
    close(conditional_score(0.5, 0.2, 0.4, 0.2).unwrap(), 0.2);
    close(conditional_score(1.0, 0.2, 0.4, 0.1).unwrap(), 0.8);
}

#[test]
fn incoming_edges_share_receiver_noise_and_parameters() {
    // Deliberately heterogeneous receivers and signed senders distinguish
    // transposed indexing, sender noise, variance and extra leak factors.
    let r_old = [0.2, -0.4, 0.8];
    let alpha_h = [0.5, 0.25, 0.125];
    let xi = [0.4, -0.8, 1.6];
    let sigma = [0.1, 0.2, 0.4];
    let expected = [[0.4, -0.8, 1.6], [-0.2, 0.4, -0.8], [0.1, -0.2, 0.4]];
    for j in 0..3 {
        for (i, &activity) in r_old.iter().enumerate() {
            close(
                conditional_score(alpha_h[j], activity, xi[j], sigma[j]).unwrap(),
                expected[j][i],
            );
        }
    }
}

#[test]
fn saturated_activity_has_no_tanh_derivative() {
    let state = ActorState::from_state(vec![30.0, -30.0], vec![0.0; 2]).unwrap();
    assert_eq!(state.r(), &[1.0, -1.0]);
    // tanh' is exactly zero here, but the conditional score is nonzero.
    close(
        conditional_score(0.25, state.r()[0], 0.8, 0.2).unwrap(),
        1.0,
    );
    close(
        conditional_score(0.25, state.r()[1], 0.8, 0.2).unwrap(),
        -1.0,
    );
}

#[test]
fn zero_activity_or_perturbation_yields_zero_score() {
    for activity in [0.0, -0.0] {
        for xi in [-2.0, 0.0, 3.0] {
            assert_eq!(conditional_score(0.5, activity, xi, 0.1), Ok(0.0));
        }
    }
    assert_eq!(conditional_score(0.5, -0.8, 0.0, 0.1), Ok(0.0));
}

#[test]
fn score_uses_actor_leak_without_an_extra_factor() {
    // tau = 1/ln(2) gives alpha = 1/2 independently of the helper formula.
    let alpha = leak_alpha(1.0 / std::f64::consts::LN_2);
    close(conditional_score(alpha, 0.2, 0.4, 0.1).unwrap(), 0.4);
}

#[test]
fn invalid_noise_is_rejected_even_for_zero_numerator() {
    for sigma in [0.0, -0.0, -0.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for (activity, xi) in [(0.2, 0.4), (0.0, 0.4), (0.2, 0.0)] {
            assert!(matches!(
                conditional_score(0.5, activity, xi, sigma),
                Err(ScoreError::InvalidInput(_))
            ));
        }
    }
}

#[test]
fn nonfinite_inputs_and_invalid_leaks_are_rejected() {
    for alpha in [0.0, -0.1, 1.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            conditional_score(alpha, 0.2, 0.4, 0.1),
            Err(ScoreError::InvalidInput(_))
        ));
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            conditional_score(0.5, value, 0.4, 0.1),
            Err(ScoreError::InvalidInput(_))
        ));
        assert!(matches!(
            conditional_score(0.5, 0.2, value, 0.1),
            Err(ScoreError::InvalidInput(_))
        ));
    }
}

#[test]
fn positive_sigma_is_not_floored_and_overflow_is_explicit() {
    assert_eq!(conditional_score(0.5, 0.25, 1.0, 1e-200), Ok(1.25e199));
    assert_eq!(
        conditional_score(0.5, 1.0, 4.0, f64::MIN_POSITIVE / 4.0),
        Err(ScoreError::NonFiniteScore)
    );
}
