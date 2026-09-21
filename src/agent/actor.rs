//! Double-buffered `f64` actor transition (M1-03).
//!
//! Spec: 4.1 (`W[receiver, sender]` orientation), 6.1 (actor update), 6.2
//! (adaptation as a slow signed average), 18.4–18.5 (preallocated working
//! buffers, old-state right-hand sides, dense reference).
//!
//! Scope: membrane `h`, adaptation `a`, and activity `r = tanh(h)` only.
//! Motor filters and commitment arrive in M1-06; the perturbation schedule
//! audit in M1-04; numerical health summaries in M1-08.
//!
//! M3-01 adds [`ActorState::step_with_effective_weights`] (and its
//! injected-perturbation variant), which reads the plastic effective
//! recurrent matrix `W0 + P` from the single
//! [`crate::agent::plasticity::PlasticState`] refresh location. It shares
//! the same compute core and still reads `B`/bias from inherited
//! parameters, so `P = 0` reproduces the `W0` path bitwise.
//!
//! Draw schedule (spec 18.4, verified in M1-04): `step` draws exactly one
//! perturbation per actor neuron on every call from the dedicated
//! `actor_noise` stream, unconditionally — input values, saturation, and
//! (future) gate settings never change the count or order. Pairing is
//! per-tick-local (one `NormalStream` per call): even neuron counts read a
//! contiguous slice, odd counts deterministically re-pair each tick.
//! Observation (`h`, `a`, `r`, `last_perturbations`) draws nothing.
//!
//! Transition (spec 6.1), all right-hand sides read old arrays:
//!
//! ```text
//! alpha_h = 1 - exp(-1 / tau_h)          (via -expm1, see leak_alpha)
//! drive[j] = sum_i W[j,i] * r_old[i] + sum_d B[j,d] * input[d]
//!            + bias[j] - adaptation_strength * a_old[j]
//! mu[j] = (1 - alpha_h) * h_old[j] + alpha_h * drive[j]
//! h_new[j] = mu[j] + sigma * xi[j]       (noise AFTER leaky integration)
//! a_new[j] = (1 - alpha_a) * a_old[j] + alpha_a * r_old[j]
//! r_new[j] = tanh(h_new[j])
//! ```
//!
//! There is no clipping of `h`: a nonfinite `h_new`/`a_new` is an explicit
//! error. Scalar `[actor]` constants are broadcast to all neurons (see
//! `docs/decisions.md`, M1-03 entry).

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::agent::weights::{InheritedParams, NormalStream, Weights};
use crate::config::Actor;

/// Actor transition errors. Dimension and parameter failures name the
/// offending value; nonfinite state names the component, never a silent
/// clamp or a successful-looking output.
#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum ActorError {
    #[error("actor dimension mismatch: {0}")]
    DimensionMismatch(String),
    #[error("invalid actor parameters: {0}")]
    InvalidParams(String),
    #[error("nonfinite actor state in {0}")]
    NonFiniteState(String),
}

/// Leaky-integration factor `alpha = 1 - exp(-1 / tau)`.
///
/// Computed as `-expm1(-1 / tau)` for accuracy at large time constants
/// (e.g. `tau_a = 100`). Precondition: `tau` finite and `> 0`, checked by
/// the step entry points, not here.
pub fn leak_alpha(tau: f64) -> f64 {
    -(-1.0 / tau).exp_m1()
}

/// Continuously evolving actor state with preallocated working buffers.
///
/// Live arrays are `h`/`a`/`r`; `*_next`, `drive`, and `xi_buf` are scratch
/// reused every tick so the inner loop never allocates. `r` is always
/// `tanh(h)` elementwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActorState {
    n: usize,
    h: Vec<f64>,
    a: Vec<f64>,
    r: Vec<f64>,
    h_next: Vec<f64>,
    a_next: Vec<f64>,
    r_next: Vec<f64>,
    drive: Vec<f64>,
    xi_buf: Vec<f64>,
}

impl ActorState {
    /// Birth state (spec 10.4): all zeros.
    pub fn new(neuron_count: usize) -> Result<Self, ActorError> {
        if neuron_count == 0 {
            return Err(ActorError::DimensionMismatch(
                "neuron_count must be >= 1".to_owned(),
            ));
        }
        Ok(Self {
            n: neuron_count,
            h: vec![0.0; neuron_count],
            a: vec![0.0; neuron_count],
            r: vec![0.0; neuron_count],
            h_next: vec![0.0; neuron_count],
            a_next: vec![0.0; neuron_count],
            r_next: vec![0.0; neuron_count],
            drive: vec![0.0; neuron_count],
            xi_buf: vec![0.0; neuron_count],
        })
    }

    /// Explicit `(h, a)` state with `r = tanh(h)`. Serves exact fixtures
    /// now and checkpoint restore in M1-09; shapes and finiteness are
    /// validated rather than trusted.
    pub fn from_state(h: Vec<f64>, a: Vec<f64>) -> Result<Self, ActorError> {
        if h.is_empty() || h.len() != a.len() {
            return Err(ActorError::DimensionMismatch(format!(
                "h (len {}) and a (len {}) must be non-empty and equal",
                h.len(),
                a.len()
            )));
        }
        if !h.iter().all(|v| v.is_finite()) || !a.iter().all(|v| v.is_finite()) {
            return Err(ActorError::NonFiniteState("from_state input".to_owned()));
        }
        let r: Vec<f64> = h.iter().map(|v| v.tanh()).collect();
        let n = h.len();
        Ok(Self {
            n,
            h,
            a,
            r,
            h_next: vec![0.0; n],
            a_next: vec![0.0; n],
            r_next: vec![0.0; n],
            drive: vec![0.0; n],
            xi_buf: vec![0.0; n],
        })
    }

    pub(crate) fn from_snapshot(
        h: Vec<f64>,
        a: Vec<f64>,
        xi: Vec<f64>,
    ) -> Result<Self, ActorError> {
        let mut state = Self::from_state(h, a)?;
        if xi.len() != state.n || !xi.iter().all(|v| v.is_finite()) {
            return Err(ActorError::InvalidParams(
                "invalid saved perturbations".to_owned(),
            ));
        }
        state.xi_buf = xi;
        Ok(state)
    }

    pub fn neuron_count(&self) -> usize {
        self.n
    }

    pub fn h(&self) -> &[f64] {
        &self.h
    }

    pub fn a(&self) -> &[f64] {
        &self.a
    }

    pub fn r(&self) -> &[f64] {
        &self.r
    }

    /// Perturbations used by the most recent transition, one per neuron.
    ///
    /// Both entry points leave the applied vector here: `step` draws it,
    /// `step_with_perturbations` copies the injected slice. Read-only
    /// observation never advances simulation randomness. The later
    /// eligibility trace (M2) reads the receiving neuron's perturbation
    /// from this buffer.
    pub fn last_perturbations(&self) -> &[f64] {
        &self.xi_buf
    }

    /// One transition on ordinary features with caller-supplied
    /// perturbations (deterministic fixture path; the M1-04 schedule audit
    /// uses this entry point).
    pub fn step_with_perturbations(
        &mut self,
        actor: &Actor,
        params: &InheritedParams,
        input: &[f64],
        xi: &[f64],
    ) -> Result<(), ActorError> {
        check_step_dims(self.n, actor, params, input.len(), xi.len())?;
        let Self {
            h,
            a,
            r,
            h_next,
            a_next,
            r_next,
            drive,
            xi_buf,
            ..
        } = self;
        xi_buf.copy_from_slice(xi);
        advance_new(
            actor,
            &params.weights,
            input,
            xi,
            h,
            a,
            r,
            h_next,
            a_next,
            drive,
        )?;
        swap_and_refresh(h, a, r, h_next, a_next, r_next);
        Ok(())
    }

    /// One transition drawing fresh perturbations from `rng` into the
    /// preallocated buffer (production path; same core as the fixture
    /// path above).
    pub fn step(
        &mut self,
        actor: &Actor,
        params: &InheritedParams,
        input: &[f64],
        rng: &mut ChaCha8Rng,
    ) -> Result<(), ActorError> {
        check_step_dims(self.n, actor, params, input.len(), self.n)?;
        let Self {
            h,
            a,
            r,
            h_next,
            a_next,
            r_next,
            drive,
            xi_buf,
            ..
        } = self;
        {
            let mut stream = NormalStream::new(rng);
            for z in xi_buf.iter_mut() {
                *z = stream.next_standard();
            }
        }
        advance_new(
            actor,
            &params.weights,
            input,
            xi_buf,
            h,
            a,
            r,
            h_next,
            a_next,
            drive,
        )?;
        swap_and_refresh(h, a, r, h_next, a_next, r_next);
        Ok(())
    }

    /// One transition using plastic effective recurrent weights (M3-01).
    ///
    /// Recurrent drive reads `effective[j,i]` (`W0 + P` from the single
    /// [`crate::agent::plasticity::PlasticState`] refresh location);
    /// sensory projection, bias, adaptation, leak, and noise handling are
    /// the shared core below, so `effective == W0` reproduces [`Self::step`]
    /// bitwise. `effective` must be `N x N`, finite, and exactly `0.0` on
    /// missing structural edges (missing edges stay absent). Existing-edge
    /// values may differ from `W0` only through validated `P`.
    pub fn step_with_effective_weights(
        &mut self,
        actor: &Actor,
        params: &InheritedParams,
        effective: &[Vec<f64>],
        input: &[f64],
        rng: &mut ChaCha8Rng,
    ) -> Result<(), ActorError> {
        check_step_dims(self.n, actor, params, input.len(), self.n)?;
        check_effective_dims(self.n, params, effective)?;
        let Self {
            h,
            a,
            r,
            h_next,
            a_next,
            r_next,
            drive,
            xi_buf,
            ..
        } = self;
        {
            let mut stream = NormalStream::new(rng);
            for z in xi_buf.iter_mut() {
                *z = stream.next_standard();
            }
        }
        advance_new_with_recurrence(
            actor,
            effective,
            &params.weights.input_weights,
            &params.weights.bias,
            input,
            xi_buf,
            h,
            a,
            r,
            h_next,
            a_next,
            drive,
        )?;
        swap_and_refresh(h, a, r, h_next, a_next, r_next);
        Ok(())
    }

    /// Deterministic-perturbation variant of
    /// [`Self::step_with_effective_weights`] for fixtures.
    pub fn step_with_effective_and_perturbations(
        &mut self,
        actor: &Actor,
        params: &InheritedParams,
        effective: &[Vec<f64>],
        input: &[f64],
        xi: &[f64],
    ) -> Result<(), ActorError> {
        check_step_dims(self.n, actor, params, input.len(), xi.len())?;
        check_effective_dims(self.n, params, effective)?;
        let Self {
            h,
            a,
            r,
            h_next,
            a_next,
            r_next,
            drive,
            xi_buf,
            ..
        } = self;
        xi_buf.copy_from_slice(xi);
        advance_new_with_recurrence(
            actor,
            effective,
            &params.weights.input_weights,
            &params.weights.bias,
            input,
            xi,
            h,
            a,
            r,
            h_next,
            a_next,
            drive,
        )?;
        swap_and_refresh(h, a, r, h_next, a_next, r_next);
        Ok(())
    }
}

/// Validate everything the transition reads: state/weight/input/xi shapes
/// and the scalar constants it broadcasts.
fn check_step_dims(
    state_n: usize,
    actor: &Actor,
    params: &InheritedParams,
    input_len: usize,
    xi_len: usize,
) -> Result<(), ActorError> {
    let n = params.topology.neuron_count;
    if n != state_n {
        return Err(ActorError::DimensionMismatch(format!(
            "inherited topology has {n} neurons but state has {state_n}"
        )));
    }
    let w = &params.weights;
    if w.w0.len() != n || w.w0.iter().any(|row| row.len() != n) {
        return Err(ActorError::DimensionMismatch(format!("W0 must be {n}x{n}")));
    }
    if w.bias.len() != n {
        return Err(ActorError::DimensionMismatch(format!(
            "bias len {} must match {n} neurons",
            w.bias.len()
        )));
    }
    let d = w.input_weights.first().map_or(0, Vec::len);
    if w.input_weights.len() != n || w.input_weights.iter().any(|row| row.len() != d) {
        return Err(ActorError::DimensionMismatch(format!(
            "input projection must be {n}x{d}"
        )));
    }
    if input_len != d {
        return Err(ActorError::DimensionMismatch(format!(
            "input len {input_len} must match projection width {d}"
        )));
    }
    if xi_len != n {
        return Err(ActorError::DimensionMismatch(format!(
            "perturbation len {xi_len} must match {n} neurons"
        )));
    }
    for (name, value) in [
        ("tau_h", actor.tau_h),
        ("tau_a", actor.tau_a),
        ("noise_sigma", actor.noise_sigma),
        ("adaptation_strength", actor.adaptation_strength),
    ] {
        if !value.is_finite() {
            return Err(ActorError::InvalidParams(format!(
                "{name} must be finite; found {value}"
            )));
        }
    }
    for (name, value) in [("tau_h", actor.tau_h), ("tau_a", actor.tau_a)] {
        if value <= 0.0 {
            return Err(ActorError::InvalidParams(format!(
                "{name} must be > 0; found {value}"
            )));
        }
    }
    Ok(())
}

/// Effective-weight validation for the plastic path: square, finite, and
/// exactly `0.0` on missing structural edges. Existing edges may carry any
/// finite value (validated `P` offsets enter only through the single
/// plasticity refresh location).
fn check_effective_dims(
    state_n: usize,
    params: &InheritedParams,
    effective: &[Vec<f64>],
) -> Result<(), ActorError> {
    let n = params.topology.neuron_count;
    if n != state_n {
        return Err(ActorError::DimensionMismatch(format!(
            "inherited topology has {n} neurons but state has {state_n}"
        )));
    }
    if effective.len() != n || effective.iter().any(|row| row.len() != n) {
        return Err(ActorError::DimensionMismatch(format!(
            "effective weights must be {n}x{n}"
        )));
    }
    for (j, row) in effective.iter().enumerate() {
        for (i, &w) in row.iter().enumerate() {
            if !w.is_finite() {
                return Err(ActorError::NonFiniteState("effective".to_owned()));
            }
            if !params.topology.mask[j][i] && w != 0.0 {
                return Err(ActorError::DimensionMismatch(format!(
                    "effective[{j},{i}] must be 0.0 on missing edge"
                )));
            }
        }
    }
    Ok(())
}

/// Shared compute core: dense drive accumulation from old arrays only,
/// leak, post-integration noise, and the adaptation update into the
/// scratch buffers. No allocation, no clipping. Buffer swaps and the fresh
/// `tanh` activity stay with the callers so swaps happen on sized `Vec`s.
#[allow(clippy::too_many_arguments)]
fn advance_new(
    actor: &Actor,
    weights: &Weights,
    input: &[f64],
    xi: &[f64],
    h: &[f64],
    a: &[f64],
    r: &[f64],
    h_next: &mut [f64],
    a_next: &mut [f64],
    drive: &mut [f64],
) -> Result<(), ActorError> {
    advance_new_with_recurrence(
        actor,
        &weights.w0,
        &weights.input_weights,
        &weights.bias,
        input,
        xi,
        h,
        a,
        r,
        h_next,
        a_next,
        drive,
    )
}

/// Recurrence-parameterized core shared by the `W0` and plastic effective
/// paths. `w_rec` is the dense recurrent matrix; sensory projection and
/// bias always come from inherited parameters (never plastic in M3).
#[allow(clippy::too_many_arguments)]
fn advance_new_with_recurrence(
    actor: &Actor,
    w_rec: &[Vec<f64>],
    input_weights: &[Vec<f64>],
    bias: &[f64],
    input: &[f64],
    xi: &[f64],
    h: &[f64],
    a: &[f64],
    r: &[f64],
    h_next: &mut [f64],
    a_next: &mut [f64],
    drive: &mut [f64],
) -> Result<(), ActorError> {
    let alpha_h = leak_alpha(actor.tau_h);
    let alpha_a = leak_alpha(actor.tau_a);

    // Dense recurrent + sensory drive from OLD activity/adaptation only.
    // Missing edges contribute their exact 0.0 through the dense storage.
    for (j, dj) in drive.iter_mut().enumerate() {
        let mut d = bias[j] - actor.adaptation_strength * a[j];
        for (w, r_old) in w_rec[j].iter().zip(r.iter()) {
            d += w * r_old;
        }
        for (w, x) in input_weights[j].iter().zip(input.iter()) {
            d += w * x;
        }
        *dj = d;
    }

    for (j, hn) in h_next.iter_mut().enumerate() {
        let mu = (1.0 - alpha_h) * h[j] + alpha_h * drive[j];
        *hn = mu + actor.noise_sigma * xi[j];
    }
    for (j, an) in a_next.iter_mut().enumerate() {
        *an = (1.0 - alpha_a) * a[j] + alpha_a * r[j];
    }
    if !h_next.iter().all(|v| v.is_finite()) || !a_next.iter().all(|v| v.is_finite()) {
        return Err(ActorError::NonFiniteState("h_new/a_new".to_owned()));
    }
    Ok(())
}

/// Swap the computed scratch buffers live and refresh activity as
/// `tanh` of the new membrane state.
fn swap_and_refresh(
    h: &mut Vec<f64>,
    a: &mut Vec<f64>,
    r: &mut Vec<f64>,
    h_next: &mut Vec<f64>,
    a_next: &mut Vec<f64>,
    r_next: &mut Vec<f64>,
) {
    std::mem::swap(h, h_next);
    std::mem::swap(a, a_next);
    for (rn, hh) in r_next.iter_mut().zip(h.iter()) {
        *rn = hh.tanh();
    }
    std::mem::swap(r, r_next);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leak_alpha_matches_one_minus_exp() {
        // The two formulations agree to ~1 ulp; the bound below is far
        // tighter than any dynamics tolerance yet loose enough for that.
        for tau in [0.5_f64, 3.0, 5.0, 100.0] {
            let expected = 1.0 - (-1.0 / tau).exp();
            let got = leak_alpha(tau);
            let diff = (got - expected).abs() / expected;
            assert!(diff < 1e-14, "tau {tau}: {got} vs {expected}");
        }
    }

    #[test]
    fn leak_alpha_stays_accurate_at_large_tau() {
        // alpha(1e9) ~= 1e-9 within 1%: the -expm1 motivation.
        let got = leak_alpha(1e9);
        assert!((got - 1e-9).abs() / 1e-9 < 0.01, "got {got}");
        assert!((leak_alpha(100.0) - 0.009_950_166_250_831_893).abs() < 1e-15);
    }
}
