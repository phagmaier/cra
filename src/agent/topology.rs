//! Inherited recurrent topology and structural validation (M1-01).
//!
//! Spec: 3.2 (starting sizes), 4.1 (orientation `W[receiver, sender]`),
//! 6.5 (cue-to-motor reachability, recurrent cycles), 10.1 (Bernoulli mask,
//! no self-edges initially, no performance-based selection, paired masks),
//! 18.4-18.5 (stable receiver-grouped edge order, dense reference).
//!
//! Scope: directed mask sampling, fixed disjoint motor pools, a stable edge
//! order, and structural acceptance checks with rejection logging. No
//! weights (`W0`/`B` arrive in M1-02), no dynamics (M1-03), no plasticity.
//!
//! Conventions (recorded in `docs/decisions.md`, 2026-09-21 UTC M1-01):
//!
//! - Motor pools are the last `2 * motor_neurons_per_action` actor indices:
//!   `M0 = [N-2m, N-m)`, `M1 = [N-m, N)`; non-motor are `[0, N-2m)`.
//! - Draw order is receiver-major, sender-inner (`j` outer, `i` inner), one
//!   `Bernoulli(edge_probability)` draw per directed pair. When
//!   `self_edges = false`, diagonal pairs consume no RNG.
//! - Inherited sampling uses stream `"init"` only. The outer-seed-level
//!   convention is `(root, namespace, outer, lifetime_index = 0, "init")`,
//!   so all lifetimes under one outer seed share the mask and independent
//!   outer seeds vary it. Gate mode is not an input, so fixed/global/
//!   targeted conditions pair by construction. A new lifetime is never a
//!   new inherited topology.
//! - Cue-driven proxy: non-motor neurons (sensory projection `B` is planned
//!   dense in M1-02). Acceptance needs a directed path from the non-motor
//!   set to *each* motor pool plus one directed recurrent cycle (length >= 2
//!   while self-edges are off; a self-loop counts only when enabled).
//! - The sampler takes no reward, hidden state, or fitness input, so
//!   selection is structural by construction. Rejected attempts are logged
//!   with edge counts and reasons; exhaustion is an explicit error.

use rand::distr::{Bernoulli, Distribution};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::config::Actor;
use crate::rng::{SeedTuple, derive_seed_hex, rng_for, validate_tuple};

/// The only RNG stream permitted for inherited topology sampling.
pub const INIT_STREAM: &str = "init";

/// Default cap on structural resampling attempts.
pub const DEFAULT_MAX_STRUCTURAL_ATTEMPTS: u32 = 100;

/// Structural rejection reasons. Each names a failed sanity check, never a
/// task-performance judgment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    /// No directed recurrent cycle in the mask.
    NoCycle,
    /// No directed path from the source set to motor pool 0.
    Motor0Unreachable,
    /// No directed path from the source set to motor pool 1.
    Motor1Unreachable,
}

impl std::fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCycle => write!(f, "no_cycle"),
            Self::Motor0Unreachable => write!(f, "m0_unreachable"),
            Self::Motor1Unreachable => write!(f, "m1_unreachable"),
        }
    }
}

/// One rejected structural sample: attempt index, edge count, and reasons.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub attempt: u32,
    pub edge_count: usize,
    pub reasons: Vec<RejectionReason>,
}

/// Inherited topology errors. Structural rejection and sampling exhaustion
/// are explicit errors, never silent fallbacks.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum TopologyError {
    #[error("invalid topology dimensions: {0}")]
    InvalidDimensions(String),
    #[error("invalid edge probability: {0}")]
    InvalidProbability(String),
    #[error("bad initialization seed: {0}")]
    BadSeed(String),
    #[error("structurally rejected: {reasons:?}")]
    StructurallyRejected { reasons: Vec<RejectionReason> },
    #[error("no acceptable mask in {attempts} attempts")]
    Exhausted {
        attempts: u32,
        log: Vec<AttemptRecord>,
    },
}

/// Inherited directed topology: mask, fixed motor pools, and stable edge
/// order. `mask[receiver][sender]` is true exactly for existing edges;
/// `edges` lists `(receiver, sender)` pairs sorted by receiver, then sender.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Topology {
    pub neuron_count: usize,
    pub motor_per_action: usize,
    pub motor0: Vec<usize>,
    pub motor1: Vec<usize>,
    pub edge_probability: f64,
    pub self_edges: bool,
    pub mask: Vec<Vec<bool>>,
    pub edges: Vec<(usize, usize)>,
    /// Lowercase hex of the deriving `init` seed (audit trail).
    pub init_seed_hex: String,
}

/// An accepted sample plus its rejection history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampledTopology {
    pub topology: Topology,
    /// Rejected attempts before acceptance, in order.
    pub rejected: Vec<AttemptRecord>,
    /// Zero-based attempt index that was accepted.
    pub accepted_attempt: u32,
}

impl Topology {
    /// True when the directed edge `sender -> receiver` exists.
    pub fn has_edge(&self, receiver: usize, sender: usize) -> bool {
        self.mask
            .get(receiver)
            .and_then(|row| row.get(sender))
            .copied()
            .unwrap_or(false)
    }

    /// Number of existing directed edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// In-degree (number of incoming existing edges) of `receiver`.
    pub fn in_degree(&self, receiver: usize) -> usize {
        self.mask
            .get(receiver)
            .map_or(0, |row| row.iter().filter(|&&b| b).count())
    }

    /// Non-motor source indices used by the reachability proxy.
    pub fn sources(&self) -> Vec<usize> {
        non_motor_sources(self.neuron_count, self.motor_per_action)
    }
}

/// Fixed disjoint motor pools on the last `2 * m` actor indices.
pub fn motor_pools(
    neuron_count: usize,
    motor_per_action: usize,
) -> Result<(Vec<usize>, Vec<usize>), TopologyError> {
    if motor_per_action < 1 {
        return Err(TopologyError::InvalidDimensions(format!(
            "motor_neurons_per_action must be >= 1; found {motor_per_action}"
        )));
    }
    if neuron_count == 0 || motor_per_action > neuron_count / 2 {
        return Err(TopologyError::InvalidDimensions(format!(
            "neuron_count ({neuron_count}) must hold two disjoint motor pools of {motor_per_action} each"
        )));
    }
    let m0: Vec<usize> =
        ((neuron_count - 2 * motor_per_action)..(neuron_count - motor_per_action)).collect();
    let m1: Vec<usize> = ((neuron_count - motor_per_action)..neuron_count).collect();
    Ok((m0, m1))
}

/// Source set for the cue-to-motor proxy: non-motor neurons when any exist,
/// otherwise every neuron (degenerate `N = 2m`; pools are then trivially
/// reachable and only the cycle check discriminates).
pub fn non_motor_sources(neuron_count: usize, motor_per_action: usize) -> Vec<usize> {
    let non_motor = neuron_count.saturating_sub(2 * motor_per_action);
    if non_motor > 0 {
        (0..non_motor).collect()
    } else {
        (0..neuron_count).collect()
    }
}

fn validate_probability(p: f64) -> Result<(), TopologyError> {
    if !p.is_finite() || !(0.0..=1.0).contains(&p) {
        return Err(TopologyError::InvalidProbability(format!(
            "edge_probability must be in [0, 1]; found {p}"
        )));
    }
    Ok(())
}

/// Sample one directed Bernoulli mask in receiver-major, sender-inner order.
/// Diagonal pairs consume no RNG when `self_edges` is false.
pub fn sample_mask(
    rng: &mut ChaCha8Rng,
    neuron_count: usize,
    edge_probability: f64,
    self_edges: bool,
) -> Result<Vec<Vec<bool>>, TopologyError> {
    validate_probability(edge_probability)?;
    if neuron_count == 0 {
        return Err(TopologyError::InvalidDimensions(
            "neuron_count must be >= 1".to_owned(),
        ));
    }
    // Boundary probabilities are fixed outcomes, independent of draws.
    if edge_probability <= 0.0 {
        return Ok(vec![vec![false; neuron_count]; neuron_count]);
    }
    if edge_probability >= 1.0 {
        let mut mask = vec![vec![true; neuron_count]; neuron_count];
        if !self_edges {
            for (j, row) in mask.iter_mut().enumerate() {
                row[j] = false;
            }
        }
        return Ok(mask);
    }
    let dist = Bernoulli::new(edge_probability)
        .map_err(|e| TopologyError::InvalidProbability(e.to_string()))?;
    let mut mask = vec![vec![false; neuron_count]; neuron_count];
    for (j, row) in mask.iter_mut().enumerate() {
        for (i, cell) in row.iter_mut().enumerate() {
            if i == j && !self_edges {
                continue;
            }
            *cell = dist.sample(rng);
        }
    }
    Ok(mask)
}

/// Stable edge order: `(receiver, sender)` sorted by receiver, then sender.
/// Sampling already produces this order; this constructor enforces it for
/// hand-built masks too.
pub fn edge_list(mask: &[Vec<bool>]) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for (j, row) in mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                edges.push((j, i));
            }
        }
    }
    edges
}

/// Outgoing adjacency: `out[i]` lists receivers `j` with `mask[j][i]`.
fn outgoing(mask: &[Vec<bool>]) -> Vec<Vec<usize>> {
    let n = mask.len();
    let mut out = vec![Vec::new(); n];
    for (j, row) in mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                out[i].push(j);
            }
        }
    }
    out
}

/// True when the directed graph contains a cycle (iterative 3-color DFS).
pub fn has_cycle(mask: &[Vec<bool>]) -> bool {
    let n = mask.len();
    let out = outgoing(mask);
    // 0 = unvisited, 1 = on current stack, 2 = finished.
    let mut state = vec![0u8; n];
    for start in 0..n {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        while let Some((node, child)) = stack.pop() {
            if child < out[node].len() {
                stack.push((node, child + 1));
                let next = out[node][child];
                if state[next] == 1 {
                    return true;
                }
                if state[next] == 0 {
                    state[next] = 1;
                    stack.push((next, 0));
                }
            } else {
                state[node] = 2;
            }
        }
    }
    false
}

/// True when some source reaches some pool member along directed edges.
/// A source inside the pool counts as reachable (length-0 path); this only
/// occurs in the degenerate `N = 2m` case where sources span all neurons.
pub fn pool_reachable_from_sources(mask: &[Vec<bool>], sources: &[usize], pool: &[usize]) -> bool {
    let n = mask.len();
    let in_pool = |v: usize| pool.contains(&v);
    if sources.iter().any(|&s| s < n && in_pool(s)) {
        return true;
    }
    let out = outgoing(mask);
    let mut seen = vec![false; n];
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for &s in sources {
        if s < n && !seen[s] {
            seen[s] = true;
            queue.push_back(s);
        }
    }
    while let Some(v) = queue.pop_front() {
        for &w in &out[v] {
            if in_pool(w) {
                return true;
            }
            if !seen[w] {
                seen[w] = true;
                queue.push_back(w);
            }
        }
    }
    false
}

/// Structural check: empty means acceptable. Reasons name failed checks only.
pub fn check_structure(
    mask: &[Vec<bool>],
    neuron_count: usize,
    motor_per_action: usize,
    motor0: &[usize],
    motor1: &[usize],
) -> Vec<RejectionReason> {
    let mut reasons = Vec::new();
    if !has_cycle(mask) {
        reasons.push(RejectionReason::NoCycle);
    }
    let sources = non_motor_sources(neuron_count, motor_per_action);
    if !pool_reachable_from_sources(mask, &sources, motor0) {
        reasons.push(RejectionReason::Motor0Unreachable);
    }
    if !pool_reachable_from_sources(mask, &sources, motor1) {
        reasons.push(RejectionReason::Motor1Unreachable);
    }
    reasons
}

/// Build a topology from an explicit mask, deriving motor pools and the
/// stable edge order. Rejects dimension mismatches only; structural
/// acceptance is decided by [`validate_topology`].
pub fn topology_from_mask(
    neuron_count: usize,
    motor_per_action: usize,
    edge_probability: f64,
    self_edges: bool,
    mask: Vec<Vec<bool>>,
    init_seed_hex: String,
) -> Result<Topology, TopologyError> {
    validate_probability(edge_probability)?;
    let (motor0, motor1) = motor_pools(neuron_count, motor_per_action)?;
    if mask.len() != neuron_count || mask.iter().any(|row| row.len() != neuron_count) {
        return Err(TopologyError::InvalidDimensions(format!(
            "mask must be {neuron_count}x{neuron_count}"
        )));
    }
    if !self_edges {
        for (j, row) in mask.iter().enumerate() {
            if row[j] {
                return Err(TopologyError::InvalidDimensions(format!(
                    "self-edge ({j}, {j}) present while self_edges = false"
                )));
            }
        }
    }
    let edges = edge_list(&mask);
    Ok(Topology {
        neuron_count,
        motor_per_action,
        motor0,
        motor1,
        edge_probability,
        self_edges,
        mask,
        edges,
        init_seed_hex,
    })
}

/// Structural acceptance for a built topology.
pub fn validate_topology(topology: &Topology) -> Result<(), TopologyError> {
    let reasons = check_structure(
        &topology.mask,
        topology.neuron_count,
        topology.motor_per_action,
        &topology.motor0,
        &topology.motor1,
    );
    if reasons.is_empty() {
        Ok(())
    } else {
        Err(TopologyError::StructurallyRejected { reasons })
    }
}

fn check_init_seed(seed: &SeedTuple) -> Result<String, TopologyError> {
    validate_tuple(seed).map_err(|e| TopologyError::BadSeed(format!("invalid seed tuple: {e}")))?;
    if seed.stream != INIT_STREAM {
        return Err(TopologyError::BadSeed(format!(
            "inherited topology must use stream \"{INIT_STREAM}\"; found \"{}\"",
            seed.stream
        )));
    }
    derive_seed_hex(seed).map_err(|e| TopologyError::BadSeed(format!("cannot derive seed: {e}")))
}

/// Sample inherited topology for one initialization seed.
///
/// Inputs are the validated `[actor]` section and an `init`-stream seed
/// tuple only: no reward, hidden state, or fitness enters, so selection is
/// structural by construction. Attempts draw sequentially from one `init`
/// RNG in the documented fixed order; rejections accumulate in `rejected`.
pub fn sample_topology(
    actor: &Actor,
    seed: &SeedTuple,
    max_attempts: u32,
) -> Result<SampledTopology, TopologyError> {
    if max_attempts < 1 {
        return Err(TopologyError::InvalidDimensions(format!(
            "max_attempts must be >= 1; found {max_attempts}"
        )));
    }
    let init_seed_hex = check_init_seed(seed)?;
    let mut rng =
        rng_for(seed).map_err(|e| TopologyError::BadSeed(format!("cannot build RNG: {e}")))?;
    // Mirror config validation so direct library callers get the same
    // dimension contract without routing through TOML.
    let (motor0, motor1) = motor_pools(actor.neuron_count, actor.motor_neurons_per_action)?;
    validate_probability(actor.edge_probability)?;

    let mut rejected = Vec::new();
    for attempt in 0..max_attempts {
        let mask = sample_mask(
            &mut rng,
            actor.neuron_count,
            actor.edge_probability,
            actor.self_edges,
        )?;
        let edge_count = mask.iter().flatten().filter(|&&b| b).count();
        let reasons = check_structure(
            &mask,
            actor.neuron_count,
            actor.motor_neurons_per_action,
            &motor0,
            &motor1,
        );
        if reasons.is_empty() {
            let edges = edge_list(&mask);
            return Ok(SampledTopology {
                topology: Topology {
                    neuron_count: actor.neuron_count,
                    motor_per_action: actor.motor_neurons_per_action,
                    motor0: motor0.clone(),
                    motor1: motor1.clone(),
                    edge_probability: actor.edge_probability,
                    self_edges: actor.self_edges,
                    mask,
                    edges,
                    init_seed_hex,
                },
                rejected,
                accepted_attempt: attempt,
            });
        }
        rejected.push(AttemptRecord {
            attempt,
            edge_count,
            reasons,
        });
    }
    Err(TopologyError::Exhausted {
        attempts: max_attempts,
        log: rejected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motor_pools_use_last_indices_and_stay_disjoint() {
        let (m0, m1) = motor_pools(16, 2).expect("valid pools");
        assert_eq!(m0, vec![12, 13]);
        assert_eq!(m1, vec![14, 15]);
        let (m0b, m1b) = motor_pools(60, 4).expect("main pools");
        assert_eq!(m0b, vec![52, 53, 54, 55]);
        assert_eq!(m1b, vec![56, 57, 58, 59]);
    }

    #[test]
    fn cycle_detection_handles_self_and_multi_hop_loops() {
        // Empty: no cycle.
        assert!(!has_cycle(&vec![vec![false; 3]; 3]));
        // Self-loop only.
        let mut self_loop = vec![vec![false; 2]; 2];
        self_loop[0][0] = true;
        assert!(has_cycle(&self_loop));
        // Two-hop loop.
        let mut two_hop = vec![vec![false; 3]; 3];
        two_hop[1][0] = true;
        two_hop[0][1] = true;
        assert!(has_cycle(&two_hop));
        // DAG: no cycle.
        let mut dag = vec![vec![false; 3]; 3];
        dag[1][0] = true;
        dag[2][1] = true;
        assert!(!has_cycle(&dag));
    }
}
