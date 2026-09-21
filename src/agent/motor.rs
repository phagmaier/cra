//! Fixed motor pools, leaky filtering, and commitment (M1-06).
//!
//! Spec: 6.3 (`motor filter tau_q: 3 ticks`), 6.4 (pool means, leaky
//! filter, higher-output commitment with a dedicated tie stream), 10.4
//! (birth `q = 0`).
//!
//! Scope: average new activities within the two fixed disjoint motor
//! populations, the leaky filter, and the deterministic-apart-from-ties
//! action choice. No trained decoder, softmax, or epsilon-greedy policy:
//! non-tie decisions consume zero randomness, which is the structural
//! proof no per-call exploration exists.
//!
//! ```text
//! motor_mean[a] = mean(r_new[j] for j in Ma)
//! q_new[a] = (1 - alpha_q) * q_old[a] + alpha_q * motor_mean[a]
//! A = 0 if q_new[0] > q_new[1] else 1 if q_new[1] > q_new[0] else fair_coin()
//! ```
//!
//! Pool identity comes from [`crate::agent::topology::motor_pools`]; this
//! module takes pool slices and revalidates them instead of duplicating
//! the assignment rule.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::agent::actor::leak_alpha;
use crate::environment::MotorOutput;

/// Motor readout errors. Bad pools, time constants, and activity shapes
/// are explicit errors, never silent defaults.
#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum MotorError {
    #[error("motor dimension mismatch: {0}")]
    DimensionMismatch(String),
    #[error("invalid motor parameters: {0}")]
    InvalidParams(String),
}

/// Continuously filtered motor-population outputs `q[0], q[1]`.
///
/// Birth state is `[0, 0]` (spec 10.4). The filter holds no weights and no
/// exploration state; it is a fixed readout, not a learned decoder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotorState {
    q: [f64; 2],
}

impl MotorState {
    /// Birth state: zero filters.
    pub fn new() -> Self {
        Self { q: [0.0, 0.0] }
    }

    /// Explicit filter state (fixtures now, checkpoint restore in M1-09).
    pub fn from_q(q: [f64; 2]) -> Result<Self, MotorError> {
        if !q.iter().all(|v| v.is_finite()) {
            return Err(MotorError::InvalidParams(format!(
                "q must be finite; found {q:?}"
            )));
        }
        Ok(Self { q })
    }

    /// Current filtered outputs.
    pub fn q(&self) -> [f64; 2] {
        self.q
    }

    /// Advance the filter from new actor activity and return the new
    /// readout. Commitment must use the returned values, never stale `q`.
    pub fn update(
        &mut self,
        motor_filter_tau: f64,
        r_new: &[f64],
        motor0: &[usize],
        motor1: &[usize],
    ) -> Result<MotorOutput, MotorError> {
        check_motor_dims(r_new, motor0, motor1)?;
        if !(motor_filter_tau.is_finite() && motor_filter_tau > 0.0) {
            return Err(MotorError::InvalidParams(format!(
                "motor_filter_tau must be finite and > 0; found {motor_filter_tau}"
            )));
        }
        if !r_new.iter().all(|v| v.is_finite()) {
            return Err(MotorError::InvalidParams("r_new must be finite".to_owned()));
        }
        let alpha_q = leak_alpha(motor_filter_tau);
        let means = [pool_mean(r_new, motor0), pool_mean(r_new, motor1)];
        for (a, mean) in means.iter().enumerate() {
            self.q[a] = (1.0 - alpha_q) * self.q[a] + alpha_q * mean;
        }
        Ok(MotorOutput {
            action_0: self.q[0],
            action_1: self.q[1],
        })
    }
}

impl Default for MotorState {
    fn default() -> Self {
        Self::new()
    }
}

/// Mean activity over one fixed motor population.
fn pool_mean(r_new: &[f64], pool: &[usize]) -> f64 {
    pool.iter().map(|&j| r_new[j]).sum::<f64>() / pool.len() as f64
}

/// Validate pool slices against the activity vector: non-empty, in range,
/// and mutually disjoint.
fn check_motor_dims(r_new: &[f64], motor0: &[usize], motor1: &[usize]) -> Result<(), MotorError> {
    if motor0.is_empty() || motor1.is_empty() {
        return Err(MotorError::DimensionMismatch(
            "motor pools must be non-empty".to_owned(),
        ));
    }
    if motor0.len() != motor1.len() {
        return Err(MotorError::DimensionMismatch(
            "motor pools must be equal-sized".to_owned(),
        ));
    }
    for (name, pool) in [("motor0", motor0), ("motor1", motor1)] {
        if pool.iter().enumerate().any(|(i, j)| pool[..i].contains(j)) {
            return Err(MotorError::DimensionMismatch(format!(
                "{name} contains duplicate indices"
            )));
        }
        if pool.iter().any(|&j| j >= r_new.len()) {
            return Err(MotorError::DimensionMismatch(format!(
                "{name} indexes outside {}-neuron activity",
                r_new.len()
            )));
        }
    }
    if motor0.iter().any(|j| motor1.contains(j)) {
        return Err(MotorError::DimensionMismatch(
            "motor pools must be disjoint".to_owned(),
        ));
    }
    Ok(())
}

/// Choose the committed action from new filtered outputs.
///
/// Strict inequality returns the winner and consumes no randomness; an
/// exact tie draws one fair coin from the dedicated `tie_break` stream.
/// There is no epsilon-greedy or softmax draw on any other path.
pub fn decide_action(output: MotorOutput, tie_rng: &mut ChaCha8Rng) -> u8 {
    if output.action_0 > output.action_1 {
        0
    } else if output.action_1 > output.action_0 {
        1
    } else {
        u8::from(tie_rng.random_bool(0.5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birth_filter_is_zero_and_round_trips() {
        assert_eq!(MotorState::new().q(), [0.0, 0.0]);
        assert_eq!(MotorState::default().q(), [0.0, 0.0]);
        assert_eq!(MotorState::from_q([0.2, -0.3]).expect("q").q(), [0.2, -0.3]);
        assert!(MotorState::from_q([f64::NAN, 0.0]).is_err());
    }
}
