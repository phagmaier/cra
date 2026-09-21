//! Plastic offsets, eligibility traces, and plastic masks (M3-01).
//!
//! Spec: 7.3 (persistent eligibility `E <- lambda_e * E + S`), 7.5 (offsets
//! `P` on plastic actor edges only; `W_effective = W0 + P`), 10.3 (plastic
//! mask choices), 10.4 (birth `P = E = 0`), 17.2 (missing/nonplastic edges
//! never acquire updates; one effective-weight refresh location).
//!
//! Scope: lifetime state `P`/`E` stored separately from immutable `W0`,
//! two plastic-mask kinds (`all_recurrent_edges`, `motor_afferent_only`),
//! and two trace policies (`persistent`, `no_decay_diagnostic`) as distinct
//! configurations. Feedback-gated `P` updates, the reward baseline, and the
//! episodic runner arrive in M3-02/M3-04; this module owns storage,
//! eligibility accumulation, the single effective-weight refresh, and a
//! versioned snapshot for future checkpoint embedding (M3-10/M4-06).
//!
//! Information boundary: `P`/`E` cover recurrent actor edges only (`N x N`).
//! Biases, sensory projection `B`, and modulator weights are never plastic
//! here; the actor transition keeps reading `B`/bias from inherited
//! parameters while recurrent drive comes from the refreshed cache.
//!
//! Trace policies (spec 7.3, 7.7): `persistent` uses
//! `lambda_e = exp(-1 / tau_e)` every tick including quiet/delay/feedback
//! transitions; `no_decay_diagnostic` uses exactly `1.0` (true summation,
//! never a large-`tau_e` approximation) and belongs to explicitly named
//! diagnostic reset profiles, never to `birth_only` (enforced in
//! [`crate::config::validate`]). `tau_e` is still validated finite and
//! `> 0` for both policies so configuration checking stays uniform; the
//! diagnostic policy ignores its value when decaying.
//!
//! Effective weights: `w_effective[j,i] = w0[j,i] + p[j,i]`, with `p == 0`
//! on every missing or nonplastic edge. [`PlasticState::refresh_effective`]
//! is the single tested location that writes the cache; all actor reads go
//! through [`PlasticState::effective_weights`]. `W0` slices passed in are
//! never mutated (tested).
//!
//! Checkpoint note: [`PlasticSnapshot`] is versioned (`deny_unknown_fields`)
//! and validated on restore (dimensions, finiteness, mask agreement, zero
//! on nonplastic/missing, `tau_e`). The top-level [`crate::checkpoint`]
//! schema stays 2 for the nonplastic M1 actor; embedding this snapshot
//! plus replay with nonzero `P`/`E` is M3-10/M4-06 work. This module
//! establishes the serializable state and its compatibility rules now.

use serde::{Deserialize, Serialize};

use crate::agent::score::{ScoreError, conditional_score};
use crate::agent::topology::Topology;
use crate::config::{Learning, SUPPORTED_PLASTIC_MASKS, SUPPORTED_TRACE_POLICIES};

/// Version for [`PlasticSnapshot`]. Bumped only with a documented format
/// change; older snapshots are rejected, never silently migrated.
pub const PLASTIC_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Plastic-state construction, advance, and snapshot failures. Every
/// variant is an explicit error, never a silent default or a clipped state.
#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum PlasticityError {
    #[error("plastic dimension mismatch: {0}")]
    DimensionMismatch(String),
    #[error("invalid plastic parameters: {0}")]
    InvalidParams(String),
    #[error("unknown plastic mask '{0}'")]
    UnknownMask(String),
    #[error("unknown trace policy '{0}'")]
    UnknownTracePolicy(String),
    #[error("nonfinite plastic state in {0}")]
    NonFiniteState(String),
    #[error("incompatible plastic snapshot: {0}")]
    Incompatible(String),
    #[error("corrupt plastic snapshot: {0}")]
    Corrupt(String),
}

impl From<ScoreError> for PlasticityError {
    fn from(error: ScoreError) -> Self {
        match error {
            ScoreError::InvalidInput(reason) => {
                Self::InvalidParams(format!("score input: {reason}"))
            }
            ScoreError::NonFiniteScore => Self::NonFiniteState("score".to_owned()),
        }
    }
}

/// Which existing edges may carry `P`/`E` (spec 10.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlasticMaskKind {
    /// Every existing recurrent edge is plastic.
    AllRecurrent,
    /// Only existing edges entering motor neurons (`motor0 ∪ motor1`) are
    /// plastic. Early debugging restriction; M8-07 keeps this diagnostic
    /// separate from the main mask-matched gate comparison.
    MotorAfferentOnly,
}

impl PlasticMaskKind {
    /// Parse a `plastic_mask` config string. Accepts exactly the
    /// [`SUPPORTED_PLASTIC_MASKS`] names.
    pub fn from_name(name: &str) -> Result<Self, PlasticityError> {
        if !SUPPORTED_PLASTIC_MASKS.contains(&name) {
            return Err(PlasticityError::UnknownMask(name.to_owned()));
        }
        match name {
            "all_recurrent_edges" => Ok(Self::AllRecurrent),
            "motor_afferent_only" => Ok(Self::MotorAfferentOnly),
            other => Err(PlasticityError::UnknownMask(other.to_owned())),
        }
    }

    /// Canonical config name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::AllRecurrent => "all_recurrent_edges",
            Self::MotorAfferentOnly => "motor_afferent_only",
        }
    }

    /// Build the `N x N` plastic mask: subset of `topology.mask`.
    /// Missing edges are never plastic; motor-only further requires the
    /// receiver to be a motor neuron.
    #[must_use]
    pub fn build_mask(self, topology: &Topology) -> Vec<Vec<bool>> {
        let n = topology.neuron_count;
        let mut mask = vec![vec![false; n]; n];
        for (j, (mask_row, topo_row)) in mask.iter_mut().zip(topology.mask.iter()).enumerate() {
            let receiver_plastic = match self {
                Self::AllRecurrent => true,
                Self::MotorAfferentOnly => {
                    topology.motor0.contains(&j) || topology.motor1.contains(&j)
                }
            };
            if !receiver_plastic {
                continue;
            }
            for (cell, &present) in mask_row.iter_mut().zip(topo_row.iter()) {
                if present {
                    *cell = true;
                }
            }
        }
        mask
    }
}

/// Eligibility decay policy (spec 7.3, 7.7).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TracePolicy {
    /// `E_new = exp(-1 / tau_e) * E_old + S` (main continuous condition).
    Persistent { tau_e: f64 },
    /// `E_new = E_old + S` (explicit diagnostic accumulation; never a
    /// large finite `tau_e`, never paired with `birth_only`).
    NoDecayDiagnostic,
}

impl TracePolicy {
    /// Parse a `trace_policy` config string with its `tau_e`. `tau_e` must
    /// be finite and `> 0` for both policies (uniform config validation);
    /// the diagnostic policy ignores the value when decaying.
    pub fn from_name(name: &str, tau_e: f64) -> Result<Self, PlasticityError> {
        if !SUPPORTED_TRACE_POLICIES.contains(&name) {
            return Err(PlasticityError::UnknownTracePolicy(name.to_owned()));
        }
        if !(tau_e.is_finite() && tau_e > 0.0) {
            return Err(PlasticityError::InvalidParams(format!(
                "tau_e must be finite and > 0; found {tau_e}"
            )));
        }
        match name {
            "persistent" => Ok(Self::Persistent { tau_e }),
            "no_decay_diagnostic" => Ok(Self::NoDecayDiagnostic),
            other => Err(PlasticityError::UnknownTracePolicy(other.to_owned())),
        }
    }

    /// Canonical config name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Persistent { .. } => "persistent",
            Self::NoDecayDiagnostic => "no_decay_diagnostic",
        }
    }

    /// Decay factor applied to `E_old` before adding the new score.
    #[must_use]
    pub fn decay_factor(self) -> f64 {
        match self {
            Self::Persistent { tau_e } => (-1.0 / tau_e).exp(),
            Self::NoDecayDiagnostic => 1.0,
        }
    }

    /// Configured `tau_e`, when the policy carries one.
    #[must_use]
    pub fn tau_e(self) -> Option<f64> {
        match self {
            Self::Persistent { tau_e } => Some(tau_e),
            Self::NoDecayDiagnostic => None,
        }
    }
}

/// Lifetime plastic state: offsets `P`, traces `E`, and the single
/// effective-weight cache `W0 + P`.
///
/// `p`/`e`/`w_effective` are `N x N`; missing and nonplastic entries stay
/// exactly `0.0` (`w_effective` equals `w0` there, which is `0.0` on
/// missing edges). Fields are private so the cache can only be refreshed
/// through [`Self::refresh_effective`] and `P`/`E` can only change through
/// [`Self::advance_eligibility`] (M3-02 adds the feedback update path) or
/// validated snapshot restore.
#[derive(Clone, Debug, PartialEq)]
pub struct PlasticState {
    n: usize,
    mask_kind: PlasticMaskKind,
    structural_mask: Vec<Vec<bool>>,
    plastic_mask: Vec<Vec<bool>>,
    plastic_edges: Vec<(usize, usize)>,
    trace_policy: TracePolicy,
    tau_e_config: f64,
    p: Vec<Vec<f64>>,
    e: Vec<Vec<f64>>,
    w_effective: Vec<Vec<f64>>,
}

impl PlasticState {
    /// Birth state (spec 10.4): `P = E = 0`, `W_effective = W0`.
    ///
    /// Validates topology/`w0` agreement (square, finite, zero on missing),
    /// the mask and trace-policy names, and `tau_e`. The cache is written
    /// once through the single refresh location before returning.
    pub fn new(
        topology: &Topology,
        w0: &[Vec<f64>],
        plastic_mask_name: &str,
        trace_policy_name: &str,
        tau_e: f64,
    ) -> Result<Self, PlasticityError> {
        let mask_kind = PlasticMaskKind::from_name(plastic_mask_name)?;
        let trace_policy = TracePolicy::from_name(trace_policy_name, tau_e)?;
        check_w0(topology, w0)?;
        let n = topology.neuron_count;
        let plastic_mask = mask_kind.build_mask(topology);
        let plastic_edges = crate::agent::topology::edge_list(&plastic_mask);
        let mut state = Self {
            n,
            mask_kind,
            structural_mask: topology.mask.clone(),
            plastic_mask,
            plastic_edges,
            trace_policy,
            tau_e_config: tau_e,
            p: vec![vec![0.0; n]; n],
            e: vec![vec![0.0; n]; n],
            w_effective: vec![vec![0.0; n]; n],
        };
        state.refresh_effective(w0)?;
        Ok(state)
    }

    /// Build from a validated [`Learning`] section, keeping TOML strings as
    /// the source of truth for mask/trace/`tau_e`.
    pub fn from_learning_config(
        topology: &Topology,
        w0: &[Vec<f64>],
        learning: &Learning,
    ) -> Result<Self, PlasticityError> {
        Self::new(
            topology,
            w0,
            &learning.plastic_mask,
            &learning.trace_policy,
            learning.tau_e,
        )
    }

    /// Neuron count.
    #[must_use]
    pub fn neuron_count(&self) -> usize {
        self.n
    }

    /// Plastic-mask kind.
    #[must_use]
    pub fn mask_kind(&self) -> PlasticMaskKind {
        self.mask_kind
    }

    /// `N x N` plastic mask (subset of the structural mask).
    #[must_use]
    pub fn plastic_mask(&self) -> &[Vec<bool>] {
        &self.plastic_mask
    }

    /// `N x N` structural mask copy (existing edges).
    #[must_use]
    pub fn structural_mask(&self) -> &[Vec<bool>] {
        &self.structural_mask
    }

    /// Plastic edges in stable receiver-then-sender order.
    #[must_use]
    pub fn plastic_edges(&self) -> &[(usize, usize)] {
        &self.plastic_edges
    }

    /// Trace policy.
    #[must_use]
    pub fn trace_policy(&self) -> TracePolicy {
        self.trace_policy
    }

    /// Configured `tau_e` (validated even when the diagnostic policy
    /// ignores it for decay).
    #[must_use]
    pub fn tau_e_config(&self) -> f64 {
        self.tau_e_config
    }

    /// Acquired offsets `P` (`N x N`, zero on missing/nonplastic).
    #[must_use]
    pub fn p(&self) -> &[Vec<f64>] {
        &self.p
    }

    /// Eligibility traces `E` (`N x N`, zero on missing/nonplastic).
    #[must_use]
    pub fn e(&self) -> &[Vec<f64>] {
        &self.e
    }

    /// Cached effective weights `W0 + P`. Read-only: the only writer is
    /// [`Self::refresh_effective`].
    #[must_use]
    pub fn effective_weights(&self) -> &[Vec<f64>] {
        &self.w_effective
    }

    /// Recompute the cache from `w0`: `w_effective = w0 + p`.
    ///
    /// This is the single tested location that writes `w_effective`.
    /// Validates `w0` shape/finiteness/zero-on-missing and the `P`
    /// invariant (zero on missing/nonplastic) before writing, so a stale
    /// or hand-edited cache cannot silently persist.
    pub fn refresh_effective(&mut self, w0: &[Vec<f64>]) -> Result<(), PlasticityError> {
        check_w0_shape(self.n, w0)?;
        for (j, ((((w_row, s_row), m_row), p_row), e_row)) in w0
            .iter()
            .zip(self.structural_mask.iter())
            .zip(self.plastic_mask.iter())
            .zip(self.p.iter())
            .zip(self.e.iter())
            .enumerate()
        {
            for (i, ((((w, structural), plastic), pv), ev)) in w_row
                .iter()
                .zip(s_row.iter())
                .zip(m_row.iter())
                .zip(p_row.iter())
                .zip(e_row.iter())
                .enumerate()
            {
                if !w.is_finite() {
                    return Err(PlasticityError::NonFiniteState("w0".to_owned()));
                }
                if !*structural && *w != 0.0 {
                    return Err(PlasticityError::DimensionMismatch(format!(
                        "w0[{j},{i}] must be 0.0 on missing edge"
                    )));
                }
                if !*plastic && *pv != 0.0 {
                    return Err(PlasticityError::Incompatible(format!(
                        "P[{j},{i}] must stay 0.0 on nonplastic edge"
                    )));
                }
                if !*plastic && *ev != 0.0 {
                    return Err(PlasticityError::Incompatible(format!(
                        "E[{j},{i}] must stay 0.0 on nonplastic edge"
                    )));
                }
                if !pv.is_finite() || !ev.is_finite() {
                    return Err(PlasticityError::NonFiniteState("P/E".to_owned()));
                }
            }
        }
        for ((w_row, p_row), eff_row) in w0
            .iter()
            .zip(self.p.iter())
            .zip(self.w_effective.iter_mut())
        {
            for ((&w, &pv), cell) in w_row.iter().zip(p_row.iter()).zip(eff_row.iter_mut()) {
                let sum = w + pv;
                if !sum.is_finite() {
                    return Err(PlasticityError::NonFiniteState("W_effective".to_owned()));
                }
                *cell = sum;
            }
        }
        // Missing edges stay exactly absent through the dense storage.
        for (s_row, eff_row) in self.structural_mask.iter().zip(self.w_effective.iter()) {
            for (&structural, &eff) in s_row.iter().zip(eff_row.iter()) {
                if !structural {
                    debug_assert_eq!(eff, 0.0);
                    if eff != 0.0 {
                        return Err(PlasticityError::Corrupt(
                            "effective weight on missing edge".to_owned(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Advance live eligibility from old presynaptic activity and this
    /// transition's receiver perturbations (spec 7.3, 9 step 6):
    /// `E_new = lambda * E_old + S`, `S = alpha_h * r_old[i] * xi[j] / sigma`.
    ///
    /// Call after the actor transition with `r_old` saved before it and
    /// `xi` read from that transition. Only plastic edges are written;
    /// missing/nonplastic entries stay exactly `0.0`. New scores never
    /// explain feedback already consumed this tick (ordering owned by the
    /// future runner, M3-02/M4-01).
    pub fn advance_eligibility(
        &mut self,
        r_old: &[f64],
        xi: &[f64],
        alpha_h: f64,
        sigma: f64,
    ) -> Result<(), PlasticityError> {
        if r_old.len() != self.n || xi.len() != self.n {
            return Err(PlasticityError::DimensionMismatch(format!(
                "r_old (len {}) and xi (len {}) must match {} neurons",
                r_old.len(),
                xi.len(),
                self.n
            )));
        }
        if !r_old.iter().all(|v| v.is_finite()) || !xi.iter().all(|v| v.is_finite()) {
            return Err(PlasticityError::NonFiniteState("r_old/xi".to_owned()));
        }
        if !(alpha_h.is_finite() && alpha_h > 0.0 && alpha_h <= 1.0) {
            return Err(PlasticityError::InvalidParams(format!(
                "alpha_h must be finite in (0, 1]; found {alpha_h}"
            )));
        }
        if !(sigma.is_finite() && sigma > 0.0) {
            // Score contract is active, so zero noise is rejected even when
            // activity or perturbation is zero (spec 6.3, 17.2).
            return Err(PlasticityError::InvalidParams(format!(
                "sigma must be finite and > 0; found {sigma}"
            )));
        }
        let lambda = self.trace_policy.decay_factor();
        if !lambda.is_finite() {
            return Err(PlasticityError::NonFiniteState("decay".to_owned()));
        }
        for &(j, i) in &self.plastic_edges {
            let score = conditional_score(alpha_h, r_old[i], xi[j], sigma)?;
            let next = lambda * self.e[j][i] + score;
            if !next.is_finite() {
                return Err(PlasticityError::NonFiniteState("E".to_owned()));
            }
            self.e[j][i] = next;
        }
        // Nonplastic/missing entries are never written; assert the invariant.
        for (j, (m_row, e_row)) in self.plastic_mask.iter().zip(self.e.iter()).enumerate() {
            for (i, (&plastic, &ev)) in m_row.iter().zip(e_row.iter()).enumerate() {
                if !plastic && ev != 0.0 {
                    return Err(PlasticityError::Corrupt(format!(
                        "E[{j},{i}] must stay 0.0 on nonplastic edge"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Versioned snapshot for checkpoint embedding (M3-10/M4-06). The
    /// effective cache is not stored: restore recomputes it through the
    /// single refresh location from the supplied `w0`.
    #[must_use]
    pub fn snapshot(&self) -> PlasticSnapshot {
        PlasticSnapshot {
            schema_version: PLASTIC_SNAPSHOT_SCHEMA_VERSION,
            neuron_count: self.n,
            plastic_mask: self.mask_kind.name().to_owned(),
            trace_policy: self.trace_policy.name().to_owned(),
            tau_e: self.tau_e_config,
            p: self.p.clone(),
            e: self.e.clone(),
        }
    }

    /// Restore validated state onto a topology plus its immutable `w0`.
    /// Rejects schema, dimension, finiteness, mask-agreement, and nonzero
    /// `P`/`E` on missing/nonplastic entries instead of defaulting them.
    pub fn restore(
        snapshot: PlasticSnapshot,
        topology: &Topology,
        w0: &[Vec<f64>],
    ) -> Result<Self, PlasticityError> {
        if snapshot.schema_version != PLASTIC_SNAPSHOT_SCHEMA_VERSION {
            return Err(PlasticityError::Incompatible(format!(
                "unsupported plastic schema_version {}; this implementation reads {PLASTIC_SNAPSHOT_SCHEMA_VERSION}",
                snapshot.schema_version
            )));
        }
        if snapshot.neuron_count != topology.neuron_count {
            return Err(PlasticityError::Incompatible(format!(
                "snapshot has {} neurons but topology has {}",
                snapshot.neuron_count, topology.neuron_count
            )));
        }
        let mask_kind = PlasticMaskKind::from_name(&snapshot.plastic_mask)?;
        let trace_policy = TracePolicy::from_name(&snapshot.trace_policy, snapshot.tau_e)?;
        check_w0(topology, w0)?;
        let n = topology.neuron_count;
        if snapshot.p.len() != n
            || snapshot.p.iter().any(|row| row.len() != n)
            || snapshot.e.len() != n
            || snapshot.e.iter().any(|row| row.len() != n)
        {
            return Err(PlasticityError::Incompatible(format!(
                "P/E must be {n}x{n}"
            )));
        }
        if !snapshot.p.iter().flatten().all(|v| v.is_finite())
            || !snapshot.e.iter().flatten().all(|v| v.is_finite())
        {
            return Err(PlasticityError::Corrupt("P/E must be finite".to_owned()));
        }
        let expected_mask = mask_kind.build_mask(topology);
        for (j, (((exp_row, topo_row), p_row), e_row)) in expected_mask
            .iter()
            .zip(topology.mask.iter())
            .zip(snapshot.p.iter())
            .zip(snapshot.e.iter())
            .enumerate()
        {
            for (i, (((&plastic, &structural), &pv), &ev)) in exp_row
                .iter()
                .zip(topo_row.iter())
                .zip(p_row.iter())
                .zip(e_row.iter())
                .enumerate()
            {
                if !plastic && (pv != 0.0 || ev != 0.0) {
                    return Err(PlasticityError::Incompatible(format!(
                        "snapshot P/E[{j},{i}] must be 0.0 on nonplastic edge"
                    )));
                }
                if !structural && (pv != 0.0 || ev != 0.0) {
                    return Err(PlasticityError::Incompatible(format!(
                        "snapshot P/E[{j},{i}] must be 0.0 on missing edge"
                    )));
                }
            }
        }
        let plastic_edges = crate::agent::topology::edge_list(&expected_mask);
        let mut state = Self {
            n,
            mask_kind,
            structural_mask: topology.mask.clone(),
            plastic_mask: expected_mask,
            plastic_edges,
            trace_policy,
            tau_e_config: snapshot.tau_e,
            p: snapshot.p,
            e: snapshot.e,
            w_effective: vec![vec![0.0; n]; n],
        };
        state.refresh_effective(w0)?;
        Ok(state)
    }
}

/// Versioned plastic-state snapshot. `deny_unknown_fields` rejects files
/// claiming future plasticity fields instead of silently defaulting them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlasticSnapshot {
    pub schema_version: u32,
    pub neuron_count: usize,
    pub plastic_mask: String,
    pub trace_policy: String,
    pub tau_e: f64,
    pub p: Vec<Vec<f64>>,
    pub e: Vec<Vec<f64>>,
}

fn check_w0_shape(n: usize, w0: &[Vec<f64>]) -> Result<(), PlasticityError> {
    if w0.len() != n || w0.iter().any(|row| row.len() != n) {
        return Err(PlasticityError::DimensionMismatch(format!(
            "w0 must be {n}x{n}"
        )));
    }
    Ok(())
}

fn check_w0(topology: &Topology, w0: &[Vec<f64>]) -> Result<(), PlasticityError> {
    let n = topology.neuron_count;
    check_w0_shape(n, w0)?;
    for (j, (w_row, m_row)) in w0.iter().zip(topology.mask.iter()).enumerate() {
        for (i, (&w, &structural)) in w_row.iter().zip(m_row.iter()).enumerate() {
            if !w.is_finite() {
                return Err(PlasticityError::NonFiniteState("w0".to_owned()));
            }
            if !structural && w != 0.0 {
                return Err(PlasticityError::DimensionMismatch(format!(
                    "w0[{j},{i}] must be 0.0 on missing edge"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::agent::topology::topology_from_mask;

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

    #[test]
    fn mask_names_round_trip_and_reject_unknown() {
        assert_eq!(
            PlasticMaskKind::from_name("all_recurrent_edges").unwrap(),
            PlasticMaskKind::AllRecurrent
        );
        assert_eq!(
            PlasticMaskKind::from_name("motor_afferent_only").unwrap(),
            PlasticMaskKind::MotorAfferentOnly
        );
        assert!(matches!(
            PlasticMaskKind::from_name("dense"),
            Err(PlasticityError::UnknownMask(_))
        ));
        assert_eq!(
            SUPPORTED_PLASTIC_MASKS,
            &["all_recurrent_edges", "motor_afferent_only"]
        );
    }

    #[test]
    fn trace_policies_are_distinct_configurations() {
        let persistent = TracePolicy::from_name("persistent", 64.0).unwrap();
        let diagnostic = TracePolicy::from_name("no_decay_diagnostic", 64.0).unwrap();
        assert!((persistent.decay_factor() - (-1.0_f64 / 64.0).exp()).abs() < 1e-15);
        assert_eq!(diagnostic.decay_factor(), 1.0);
        assert_ne!(persistent.decay_factor(), diagnostic.decay_factor());
        assert!(matches!(
            TracePolicy::from_name("fancy", 64.0),
            Err(PlasticityError::UnknownTracePolicy(_))
        ));
        assert!(matches!(
            TracePolicy::from_name("persistent", 0.0),
            Err(PlasticityError::InvalidParams(_))
        ));
        assert!(matches!(
            TracePolicy::from_name("persistent", f64::NAN),
            Err(PlasticityError::InvalidParams(_))
        ));
    }

    #[test]
    fn motor_mask_restricts_to_motor_receivers() {
        let topology = small_topology();
        let all = PlasticMaskKind::AllRecurrent.build_mask(&topology);
        let motor = PlasticMaskKind::MotorAfferentOnly.build_mask(&topology);
        assert_eq!(all, topology.mask);
        for (j, (topo_row, motor_row)) in topology.mask.iter().zip(motor.iter()).enumerate() {
            for (i, (&structural, &plastic)) in topo_row.iter().zip(motor_row.iter()).enumerate() {
                if !structural {
                    assert!(!plastic, "missing edge ({j},{i}) never plastic");
                } else if j == 2 || j == 3 {
                    assert!(plastic, "motor receiver {j} keeps edge ({j},{i})");
                } else {
                    assert!(!plastic, "non-motor receiver {j} not plastic");
                }
            }
        }
    }

    #[test]
    fn birth_state_is_zero_with_effective_equal_to_w0() {
        let topology = small_topology();
        let w0 = small_w0(&topology);
        for mask in ["all_recurrent_edges", "motor_afferent_only"] {
            for trace in ["persistent", "no_decay_diagnostic"] {
                let state = PlasticState::new(&topology, &w0, mask, trace, 32.0).unwrap();
                assert!(state.p().iter().flatten().all(|&v| v == 0.0));
                assert!(state.e().iter().flatten().all(|&v| v == 0.0));
                assert_eq!(state.effective_weights(), &w0);
            }
        }
    }
}
