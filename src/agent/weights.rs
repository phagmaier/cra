//! Inherited weights and neuron-parameter validation (M1-02).
//!
//! Spec: 6.3 (starting constants), 10.2 (inherited actor weights), 20.5
//! (deterministic seed derivation and recorded distribution implementation).
//!
//! Scope: the inherited recurrent matrix `W0` (row-scaled by in-degree),
//! the dense sensory projection `B`, zero actor biases, and validation of
//! the `[actor]` constants. No dynamics (M1-03) and no plastic offsets `P`
//! (M3): `W0` is stored alone, and the lifetime effective weight `W0 + P`
//! is constructed only once plasticity arrives.
//!
//! Sampling contract (recorded in `docs/decisions.md`, 2026-09-21 UTC
//! M1-02): one `init` RNG per initialization seed, drawn in the fixed order
//! mask attempts, then `W0` on existing edges in receiver-grouped edge
//! order, then dense `B` row-major (`j` outer, `d` inner). Standard normals
//! come from a Box–Muller transform over the pinned `rand 0.9` uniform
//! output, with the second deviate cached only inside one sampling call.
//! `B` takes its width as an explicit caller argument (the runner passes
//! `K + 6` observable features); no dimension normalization is applied.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::agent::topology::{
    AttemptRecord, Topology, TopologyError, check_init_seed, edge_list, motor_pools,
    sample_topology_with_rng, validate_probability,
};
use crate::config::Actor;
use crate::rng::{SeedTuple, rng_for};

/// Inherited weight errors. Structural failures wrap [`TopologyError`];
/// dimension and parameter failures name the offending field and value.
#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum ParamsError {
    #[error("inherited topology error: {0}")]
    Topology(#[from] TopologyError),
    #[error("invalid inherited dimensions: {0}")]
    InvalidDimensions(String),
    #[error("invalid inherited parameters: {0}")]
    InvalidParams(String),
}

/// One Box–Muller pair of independent standard normals.
///
/// `u1 = 1 - uniform[0, 1)` lies in `(0, 1]`, so `ln(u1)` is always finite;
/// the outputs are finite for every uniform input, including exact 0.0.
pub fn box_muller_pair(rng: &mut ChaCha8Rng) -> (f64, f64) {
    let u1 = 1.0 - rng.random::<f64>();
    let u2 = rng.random::<f64>();
    let radius = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    (radius * theta.cos(), radius * theta.sin())
}

/// Sequential standard-normal stream over one RNG.
///
/// The second Box–Muller deviate is cached only inside this instance, so
/// the output sequence is a pure function of position in the caller's draw
/// order. Never share one stream across parameter groups with different
/// ordering requirements.
pub struct NormalStream<'a> {
    rng: &'a mut ChaCha8Rng,
    spare: Option<f64>,
}

impl<'a> NormalStream<'a> {
    pub fn new(rng: &'a mut ChaCha8Rng) -> Self {
        Self { rng, spare: None }
    }

    /// Next standard-normal output.
    pub fn next_standard(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let (z0, z1) = box_muller_pair(self.rng);
        self.spare = Some(z1);
        z0
    }
}

/// Recurrent row standard deviation (spec 10.2): `gain / sqrt(in_degree)`.
///
/// This is the standard deviation, not the variance. A zero in-degree
/// yields `0.0`: the row stays exactly zero and consumes no normal draws,
/// instead of dividing by zero into silent `NaN`s.
pub fn row_std(recurrent_gain: f64, in_degree: usize) -> f64 {
    if in_degree == 0 {
        0.0
    } else {
        recurrent_gain / (in_degree as f64).sqrt()
    }
}

/// Inherited weight arrays: recurrent `W0`, sensory projection `B`
/// (`input_weights[j][d]`), and the zero actor bias vector.
///
/// `w0` is `N x N` with exact `0.0` on missing edges. Future plastic
/// offsets `P` (M3) are stored separately, never merged into these arrays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Weights {
    pub w0: Vec<Vec<f64>>,
    pub input_weights: Vec<Vec<f64>>,
    pub bias: Vec<f64>,
}

/// Complete inherited parameters for one initialization seed: topology plus
/// weights. Everything evolution may change between lifetimes lives here;
/// everything acquired within a lifetime (`P`, traces, state) lives
/// elsewhere (M3/M7).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InheritedParams {
    pub topology: Topology,
    pub weights: Weights,
}

impl InheritedParams {
    /// Sensory projection width (observable feature count).
    pub fn input_dim(&self) -> usize {
        self.weights.input_weights.first().map_or(0, Vec::len)
    }
}

/// An accepted inherited sample plus its structural rejection history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampledParams {
    pub params: InheritedParams,
    pub rejected: Vec<AttemptRecord>,
    pub accepted_attempt: u32,
}

/// Validate the `[actor]` constants and projection width for direct library
/// callers (TOML loading already enforces the same rules via
/// [`crate::config::validate`]).
pub fn validate_inherited(actor: &Actor, input_dim: usize) -> Result<(), ParamsError> {
    motor_pools(actor.neuron_count, actor.motor_neurons_per_action)?;
    validate_probability(actor.edge_probability)?;
    if input_dim < 1 {
        return Err(ParamsError::InvalidDimensions(format!(
            "input_dim must be >= 1; found {input_dim}"
        )));
    }
    for (name, value, positive) in [
        ("tau_h", actor.tau_h, true),
        ("tau_a", actor.tau_a, true),
        ("motor_filter_tau", actor.motor_filter_tau, true),
        ("noise_sigma", actor.noise_sigma, true),
    ] {
        if !(value.is_finite() && (!positive || value > 0.0)) {
            return Err(ParamsError::InvalidParams(format!(
                "{name} must be finite and > 0; found {value}"
            )));
        }
    }
    for (name, value) in [
        ("recurrent_gain", actor.recurrent_gain),
        ("input_scale", actor.input_scale),
        ("adaptation_strength", actor.adaptation_strength),
    ] {
        if !value.is_finite() {
            return Err(ParamsError::InvalidParams(format!(
                "{name} must be finite; found {value}"
            )));
        }
    }
    Ok(())
}

/// Sample `W0`, `B`, and zero biases for an explicit mask.
///
/// `W0[j, i] = row_std(gain, d_j) * z` on existing edges in
/// receiver-grouped edge order; missing edges stay exactly `0.0`.
/// `B[j, d] = input_scale * z` dense in row-major order. Biases are exactly
/// `0.0` (spec 10.2). Nonfinite products (reachable only with extreme
/// finite gains) are an explicit error, never silent infinities.
pub fn sample_weights_from_mask(
    rng: &mut ChaCha8Rng,
    actor: &Actor,
    mask: &[Vec<bool>],
    input_dim: usize,
) -> Result<Weights, ParamsError> {
    validate_inherited(actor, input_dim)?;
    let n = actor.neuron_count;
    if mask.len() != n || mask.iter().any(|row| row.len() != n) {
        return Err(ParamsError::InvalidDimensions(format!(
            "mask must be {n}x{n}"
        )));
    }
    if !actor.self_edges {
        for (j, row) in mask.iter().enumerate() {
            if row[j] {
                return Err(ParamsError::InvalidDimensions(format!(
                    "self-edge ({j}, {j}) present while self_edges = false"
                )));
            }
        }
    }

    let mut stream = NormalStream::new(rng);
    let in_degrees: Vec<usize> = mask
        .iter()
        .map(|row| row.iter().filter(|&&b| b).count())
        .collect();
    let mut w0 = vec![vec![0.0; n]; n];
    for (j, i) in edge_list(mask) {
        let std = row_std(actor.recurrent_gain, in_degrees[j]);
        w0[j][i] = stream.next_standard() * std;
    }
    let mut input_weights = vec![vec![0.0; input_dim]; n];
    for row in input_weights.iter_mut() {
        for cell in row.iter_mut() {
            *cell = stream.next_standard() * actor.input_scale;
        }
    }
    if !w0.iter().flatten().all(|v| v.is_finite())
        || !input_weights.iter().flatten().all(|v| v.is_finite())
    {
        return Err(ParamsError::InvalidParams(
            "sampled nonfinite weight with finite gains; reduce recurrent_gain/input_scale"
                .to_owned(),
        ));
    }
    Ok(Weights {
        w0,
        input_weights,
        bias: vec![0.0; n],
    })
}

/// Sample complete inherited parameters for one initialization seed.
///
/// One `init` RNG draws mask attempts first (identical leading draws to a
/// standalone [`crate::agent::topology::sample_topology`] call), then the
/// weight draws continue on the same stream. Inputs are the validated
/// `[actor]` section, the projection width, and the seed tuple only: no
/// reward, hidden state, or fitness enters.
pub fn sample_inherited(
    actor: &Actor,
    input_dim: usize,
    seed: &SeedTuple,
    max_attempts: u32,
) -> Result<SampledParams, ParamsError> {
    if max_attempts < 1 {
        return Err(ParamsError::InvalidDimensions(format!(
            "max_attempts must be >= 1; found {max_attempts}"
        )));
    }
    let init_seed_hex = check_init_seed(seed)?;
    let mut rng = rng_for(seed).map_err(|e| {
        ParamsError::Topology(TopologyError::BadSeed(format!("cannot build RNG: {e}")))
    })?;
    validate_inherited(actor, input_dim)?;
    let sampled = sample_topology_with_rng(actor, &mut rng, init_seed_hex, max_attempts)?;
    let weights = sample_weights_from_mask(&mut rng, actor, &sampled.topology.mask, input_dim)?;
    Ok(SampledParams {
        params: InheritedParams {
            topology: sampled.topology,
            weights,
        },
        rejected: sampled.rejected,
        accepted_attempt: sampled.accepted_attempt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SeedTuple;

    fn init_seed(outer: u64) -> SeedTuple {
        SeedTuple::new(
            1,
            "development",
            outer,
            0,
            crate::agent::topology::INIT_STREAM,
        )
    }

    #[test]
    fn row_scale_is_gain_over_sqrt_degree() {
        assert_eq!(row_std(0.8, 4), 0.4);
        assert_eq!(row_std(0.8, 0), 0.0);
        assert!((row_std(0.8, 2) - 0.8 / 2.0_f64.sqrt()).abs() < 1e-15);
    }

    #[test]
    fn normal_stream_is_deterministic_with_unit_moments() {
        // Declared tolerances: n = 20_000 normals, SE(mean) ~= 0.0071,
        // SE(var) ~= 0.01; bounds sit at ~4 SE.
        let seed = init_seed(11);
        let mut rng_a = rng_for(&seed).expect("rng");
        let mut rng_b = rng_for(&seed).expect("rng");
        let (mut sa, mut sb) = (NormalStream::new(&mut rng_a), NormalStream::new(&mut rng_b));
        let n = 20_000;
        let mut mean = 0.0;
        let mut m2 = 0.0;
        for k in 1..=n {
            let a = sa.next_standard();
            let b = sb.next_standard();
            assert_eq!(a, b, "same seed position must give the same output");
            let x = a;
            let delta = x - mean;
            mean += delta / f64::from(k);
            m2 += delta * (x - mean);
        }
        let var = m2 / f64::from(n);
        assert!(mean.abs() < 0.03, "mean {mean} far from 0");
        assert!((var - 1.0).abs() < 0.04, "variance {var} far from 1");
    }
}
