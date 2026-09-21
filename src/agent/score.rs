//! Pure conditional Gaussian score (M2-01; spec 7.2).
//!
//! This is per-edge arithmetic, independent of actor state, topology, RNG,
//! rewards and eligibility. Callers select existing plastic edges and supply
//! old sender activity plus the receiver's parameters and perturbation.
//! It does not establish an unbiased gradient for the online learner (7.6).

/// Invalid score inputs or an unrepresentable result are explicit errors.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ScoreError {
    #[error("invalid conditional score input: {0}")]
    InvalidInput(&'static str),
    #[error("nonfinite conditional score")]
    NonFiniteScore,
}

/// Score for the edge `sender i -> receiver j`:
/// `S[j,i] = alpha_h[j] * r_old[i] * xi[j] / sigma[j]`.
///
/// Use [`super::actor::leak_alpha`] on the receiver's time constant to get
/// `alpha_h_receiver`. `xi_receiver` is the standard-normal perturbation
/// used in that transition, NOT the scaled membrane increment `sigma * xi`.
/// All edges entering a receiver use the same `alpha_h`, `xi`, and `sigma`.
/// Save `r_old_sender` before advancing the actor; new activity is not credit
/// for this transition. No activation derivative or trace-decay factor enters.
///
/// Inputs must be finite, `0 < alpha_h_receiver <= 1`, and
/// `sigma_receiver > 0`. Calling this function activates the score contract,
/// so invalid noise is rejected even when activity or perturbation is zero.
/// No allocation occurs on the success path; no state or randomness is read
/// or modified. Trace accumulation and weight updates belong to later tasks.
pub fn conditional_score(
    alpha_h_receiver: f64,
    r_old_sender: f64,
    xi_receiver: f64,
    sigma_receiver: f64,
) -> Result<f64, ScoreError> {
    if !alpha_h_receiver.is_finite() || alpha_h_receiver <= 0.0 || alpha_h_receiver > 1.0 {
        return Err(ScoreError::InvalidInput("alpha_h must be finite in (0, 1]"));
    }
    if !r_old_sender.is_finite() {
        return Err(ScoreError::InvalidInput("r_old must be finite"));
    }
    if !xi_receiver.is_finite() {
        return Err(ScoreError::InvalidInput("xi must be finite"));
    }
    if !sigma_receiver.is_finite() || sigma_receiver <= 0.0 {
        return Err(ScoreError::InvalidInput("sigma must be finite and > 0"));
    }
    let score = alpha_h_receiver * r_old_sender * xi_receiver / sigma_receiver;
    if !score.is_finite() {
        return Err(ScoreError::NonFiniteScore);
    }
    Ok(score)
}
