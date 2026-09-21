//! Plastic offsets, eligibility traces, and plastic masks (M3-01) plus
//! exactly-once feedback updates and baseline arithmetic (M3-02).
//!
//! Spec: 7.3 (persistent eligibility `E <- lambda_e * E + S`), 7.4 (reward
//! baseline `delta = R - baseline_old`, one baseline update after `delta`),
//! 7.5 (offsets `P` on plastic actor edges only; `W_effective = W0 + P`),
//! 9 (feedback consumed before the current neural transition; exactly once),
//! 10.3 (plastic mask choices), 10.4 (birth `P = E = 0`, baseline `0.5`),
//! 17.2 (missing/nonplastic edges never acquire updates; one
//! effective-weight refresh location).
//!
//! Scope: lifetime state `P`/`E`/baseline/dedup stored separately from
//! immutable `W0`, two plastic-mask kinds (`all_recurrent_edges`,
//! `motor_afferent_only`), and two trace policies (`persistent`,
//! `no_decay_diagnostic`) as distinct configurations. This module owns
//! storage, eligibility accumulation, the single effective-weight refresh,
//! the exactly-once gated `P` update with its raw/limited/actual report,
//! and a versioned snapshot for future checkpoint embedding (M3-10/M4-06).
//! Fixed (ungated) plasticity passes gate `1`; gate heads arrive in M6.
//! The episodic runner arrives in M3-04.
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
//! Checkpoint note: [`PlasticSnapshot`] is versioned (`deny_unknown_fields`,
//! currently schema 3 with the configured plastic bound) and
//! validated on restore (dimensions, finiteness, mask agreement, zero on
//! nonplastic/missing, `tau_e`, finite baseline). The top-level
//! [`crate::checkpoint`] schema stays 2 for the nonplastic M1 actor;
//! embedding this snapshot plus replay with nonzero `P`/`E` is M3-10/M4-06
//! work. This module establishes the serializable state and its
//! compatibility rules now.

use serde::{Deserialize, Serialize};

use crate::agent::score::{ScoreError, conditional_score};
use crate::agent::topology::Topology;
use crate::config::{Learning, SUPPORTED_PLASTIC_MASKS, SUPPORTED_TRACE_POLICIES};

/// Version for [`PlasticSnapshot`]. Bumped only with a documented format
/// change; older snapshots are rejected, never silently migrated.
/// v3 adds the configured plastic bound so restore can reject offsets outside
/// the resolved invariant. v2 added baseline and feedback bookkeeping; older
/// files are incompatible rather than silently acquiring a bound.
pub const PLASTIC_SNAPSHOT_SCHEMA_VERSION: u32 = 3;

/// Plastic-state construction, advance, feedback-update, and snapshot
/// failures. Every variant is an explicit error, never a silent default or
/// a clipped state. Duplicate feedback leaves all state unchanged.
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
    #[error("duplicate feedback event {0}")]
    DuplicateFeedback(u64),
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

/// Lifetime plastic state: offsets `P`, traces `E`, the running reward
/// baseline, exactly-once feedback bookkeeping, and the single
/// effective-weight cache `W0 + P`.
///
/// `p`/`e`/`w_effective` are `N x N`; missing and nonplastic entries stay
/// exactly `0.0` (`w_effective` equals `w0` there, which is `0.0` on
/// missing edges). Fields are private so the cache can only be refreshed
/// through [`Self::refresh_effective`], `P`/baseline/dedup can only change
/// through [`Self::apply_feedback_once`] or validated snapshot restore,
/// and `E` can only change through [`Self::advance_eligibility`] or
/// validated restore. `E` is never reset by feedback (spec 7.7).
#[derive(Clone, Debug, PartialEq)]
pub struct PlasticState {
    n: usize,
    mask_kind: PlasticMaskKind,
    structural_mask: Vec<Vec<bool>>,
    plastic_mask: Vec<Vec<bool>>,
    plastic_edges: Vec<(usize, usize)>,
    trace_policy: TracePolicy,
    tau_e_config: f64,
    plastic_bound: f64,
    p: Vec<Vec<f64>>,
    e: Vec<Vec<f64>>,
    w_effective: Vec<Vec<f64>>,
    reward_baseline: f64,
    last_feedback: Option<u64>,
}

/// Per-feedback update report (spec 7.5): the teaching signal plus the
/// three update stages on every edge. `raw_updates` is the unbounded
/// `eta * delta * gate * E`; `limited_updates` clamps each raw value to
/// `[-max_update, +max_update]`; `actual_updates` is the realized `P`
/// change after the `[-plastic_bound, +plastic_bound]` clamp, so
/// bound saturation makes `actual != limited`. All three are `N x N` with
/// exactly `0.0` on missing/nonplastic edges.
#[derive(Clone, Debug, PartialEq)]
pub struct FeedbackOutcome {
    pub event_id: u64,
    pub reward: f64,
    pub delta: f64,
    pub baseline_old: f64,
    pub baseline_new: f64,
    pub raw_updates: Vec<Vec<f64>>,
    pub limited_updates: Vec<Vec<f64>>,
    pub actual_updates: Vec<Vec<f64>>,
}

/// Named, per-feedback update parameters. The lifetime's `plastic_bound` is
/// stored in [`PlasticState`] because it is a state invariant, not a value
/// that may vary from one feedback event to the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedbackUpdateParams {
    pub eta: f64,
    pub max_update: f64,
    pub baseline_beta: f64,
}

impl FeedbackUpdateParams {
    /// Read the event-level parameters from the resolved learning config.
    #[must_use]
    pub fn from_learning_config(learning: &Learning) -> Self {
        Self {
            eta: learning.eta,
            max_update: learning.max_update,
            baseline_beta: learning.reward_baseline_beta,
        }
    }

    fn validate(self) -> Result<(), PlasticityError> {
        if !(self.eta.is_finite() && self.eta >= 0.0) {
            return Err(PlasticityError::InvalidParams(format!(
                "eta must be finite and >= 0; found {}",
                self.eta
            )));
        }
        if !(self.max_update.is_finite() && self.max_update > 0.0) {
            return Err(PlasticityError::InvalidParams(format!(
                "max_update must be finite and > 0; found {}",
                self.max_update
            )));
        }
        if !(self.baseline_beta.is_finite() && (0.0..=1.0).contains(&self.baseline_beta)) {
            return Err(PlasticityError::InvalidParams(format!(
                "baseline_beta must be finite in [0, 1]; found {}",
                self.baseline_beta
            )));
        }
        Ok(())
    }
}

impl PlasticState {
    /// Birth state (spec 10.4): `P = E = 0`, `W_effective = W0`,
    /// `reward_baseline = 0.5`, no feedback consumed yet.
    ///
    /// Validates topology/`w0` agreement (square, finite, zero on missing),
    /// the mask and trace-policy names, and `tau_e`. The cache is written
    /// once through the single refresh location before returning.
    /// The lifetime-invariant `plastic_bound` is stored and snapshotted.
    /// Event-level parameters (`eta`, `max_update`, baseline `beta`) are
    /// supplied as a named [`FeedbackUpdateParams`] value.
    /// [`Self::from_learning_config`] sets the baseline from the validated
    /// TOML `reward_baseline_initial`.
    pub fn new(
        topology: &Topology,
        w0: &[Vec<f64>],
        plastic_mask_name: &str,
        trace_policy_name: &str,
        tau_e: f64,
        plastic_bound: f64,
    ) -> Result<Self, PlasticityError> {
        let mask_kind = PlasticMaskKind::from_name(plastic_mask_name)?;
        let trace_policy = TracePolicy::from_name(trace_policy_name, tau_e)?;
        validate_plastic_bound(plastic_bound)?;
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
            plastic_bound,
            p: vec![vec![0.0; n]; n],
            e: vec![vec![0.0; n]; n],
            w_effective: vec![vec![0.0; n]; n],
            reward_baseline: 0.5,
            last_feedback: None,
        };
        state.refresh_effective(w0)?;
        Ok(state)
    }

    /// Build from a validated [`Learning`] section, keeping TOML strings as
    /// the source of truth for mask/trace/`tau_e` and the baseline initial
    /// value and binds the configured plastic bound to the lifetime state.
    pub fn from_learning_config(
        topology: &Topology,
        w0: &[Vec<f64>],
        learning: &Learning,
    ) -> Result<Self, PlasticityError> {
        if !(learning.reward_baseline_initial.is_finite()
            && (0.0..=1.0).contains(&learning.reward_baseline_initial))
        {
            return Err(PlasticityError::InvalidParams(format!(
                "reward_baseline_initial must be finite in [0, 1]; found {}",
                learning.reward_baseline_initial
            )));
        }
        let mut state = Self::new(
            topology,
            w0,
            &learning.plastic_mask,
            &learning.trace_policy,
            learning.tau_e,
            learning.plastic_bound,
        )?;
        state.reward_baseline = learning.reward_baseline_initial;
        Ok(state)
    }

    /// Neuron count.
    #[must_use]
    pub fn neuron_count(&self) -> usize {
        self.n
    }

    /// Symmetric bound enforced for every plastic offset in this lifetime.
    #[must_use]
    pub fn plastic_bound(&self) -> f64 {
        self.plastic_bound
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

    /// Running reward baseline (spec 7.4, 10.4): `0.5` at birth, updated
    /// once per consumed feedback after `delta` is computed.
    #[must_use]
    pub fn reward_baseline(&self) -> f64 {
        self.reward_baseline
    }

    /// Highest consumed feedback id, if any (exactly-once bookkeeping).
    #[must_use]
    pub fn last_feedback(&self) -> Option<u64> {
        self.last_feedback
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
    /// future runner, M3-04/M4-01).
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

    /// Apply one bounded plastic update at feedback arrival (spec 7.4-7.5,
    /// 9 step 2, 17.2): read old `E`/gates/`P`/baseline, compute `delta`
    /// from the old baseline, clamp per-edge raw updates, clamp the
    /// resulting `P`, then update the baseline once and mark the event
    /// consumed. `E` is read, never reset.
    ///
    /// ```text
    /// delta = reward - reward_baseline_old
    /// raw[j,i] = eta * delta * gate[j] * E_old[j,i]
    /// limited[j,i] = clamp(raw[j,i], -max_update, +max_update)
    /// P_new[j,i] = clamp(P_old[j,i] + limited[j,i], -plastic_bound, +plastic_bound)
    /// baseline_new = baseline_old + beta * delta
    /// ```
    ///
    /// Fixed (ungated) plasticity passes `gates = [1.0; N]`; gate heads
    /// arrive in M6. `gates` is per receiving neuron, shared by its
    /// incoming plastic edges, finite in `[0, 1]`. Only plastic edges are
    /// written; missing/nonplastic entries stay exactly `0.0` in `P` and
    /// in all three update reports. `w0` is read to refresh the single
    /// effective cache and is never mutated. The diagnostic fixed-baseline
    /// policy lives in [`crate::experiments::finite_rollout`] (frozen
    /// baseline, no running update); this method always performs the one
    /// running-baseline update above.
    ///
    /// Exactly-once: an `event_id` at or below [`Self::last_feedback`] is
    /// rejected as [`PlasticityError::DuplicateFeedback`] with all state
    /// unchanged. All other inputs are validated before any mutation, so a
    /// rejected call also leaves `P`/baseline/cache/dedup unchanged.
    /// `eta = 0`, all-zero gates, or `delta = 0` yield zero task-dependent
    /// `P` changes (the baseline still updates once when `delta != 0`).
    pub fn apply_feedback_once(
        &mut self,
        event_id: u64,
        reward: f64,
        gates: &[f64],
        update: FeedbackUpdateParams,
        w0: &[Vec<f64>],
    ) -> Result<FeedbackOutcome, PlasticityError> {
        if self.last_feedback.is_some_and(|last| event_id <= last) {
            return Err(PlasticityError::DuplicateFeedback(event_id));
        }
        if !(reward.is_finite() && (reward == 0.0 || reward == 1.0)) {
            return Err(PlasticityError::InvalidParams(format!(
                "reward must be 0.0 or 1.0; found {reward}"
            )));
        }
        if gates.len() != self.n {
            return Err(PlasticityError::DimensionMismatch(format!(
                "gates (len {}) must match {} neurons",
                gates.len(),
                self.n
            )));
        }
        if !gates
            .iter()
            .all(|g| g.is_finite() && (0.0..=1.0).contains(g))
        {
            return Err(PlasticityError::InvalidParams(
                "gates must be finite in [0, 1]".to_owned(),
            ));
        }
        update.validate()?;
        if !self.reward_baseline.is_finite() {
            return Err(PlasticityError::NonFiniteState("baseline".to_owned()));
        }
        if !self.p.iter().flatten().all(|v| v.is_finite())
            || !self.e.iter().flatten().all(|v| v.is_finite())
        {
            return Err(PlasticityError::NonFiniteState("P/E".to_owned()));
        }
        check_w0_shape(self.n, w0)?;
        for (j, (w_row, s_row)) in w0.iter().zip(self.structural_mask.iter()).enumerate() {
            for (i, (&w, &structural)) in w_row.iter().zip(s_row.iter()).enumerate() {
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

        let baseline_old = self.reward_baseline;
        let delta = reward - baseline_old;
        if !delta.is_finite() {
            return Err(PlasticityError::NonFiniteState("delta".to_owned()));
        }
        let n = self.n;
        let mut raw = vec![vec![0.0; n]; n];
        let mut limited = vec![vec![0.0; n]; n];
        let mut actual = vec![vec![0.0; n]; n];
        let mut next_p = self.p.clone();
        for &(j, i) in &self.plastic_edges {
            let unbounded = update.eta * delta * gates[j] * self.e[j][i];
            if !unbounded.is_finite() {
                return Err(PlasticityError::NonFiniteState("raw update".to_owned()));
            }
            let bounded = unbounded.clamp(-update.max_update, update.max_update);
            let candidate = self.p[j][i] + bounded;
            if !candidate.is_finite() {
                return Err(PlasticityError::NonFiniteState("P".to_owned()));
            }
            let clamped = candidate.clamp(-self.plastic_bound, self.plastic_bound);
            raw[j][i] = unbounded;
            limited[j][i] = bounded;
            actual[j][i] = clamped - self.p[j][i];
            next_p[j][i] = clamped;
        }
        let baseline_new = baseline_old + update.baseline_beta * delta;
        if !baseline_new.is_finite() {
            return Err(PlasticityError::NonFiniteState("baseline".to_owned()));
        }

        self.p = next_p;
        self.reward_baseline = baseline_new;
        self.last_feedback = Some(event_id);
        self.refresh_effective(w0)?;
        Ok(FeedbackOutcome {
            event_id,
            reward,
            delta,
            baseline_old,
            baseline_new,
            raw_updates: raw,
            limited_updates: limited,
            actual_updates: actual,
        })
    }

    /// Explicitly diagnostic trace reset between episodic rollouts
    /// (M3-04; spec 7.7, 16/M3).
    ///
    /// Zeros every `E` entry, preserving `P`, the running baseline,
    /// exactly-once bookkeeping, the bound, masks, and the effective cache
    /// (the cache is unchanged because `P` is unchanged, so no refresh is
    /// needed). Only the `no_decay_diagnostic` policy may use this entry
    /// point: the persistent continuous rule never resets traces
    /// mid-lifetime, and a call under any other policy is an explicit
    /// error. The episodic runner (M3-04) calls this once per rollout
    /// boundary and logs the reset tick; M4 owns the fully persistent
    /// condition without this call.
    pub fn reset_traces_episodic_diagnostic(&mut self) -> Result<(), PlasticityError> {
        if !matches!(self.trace_policy, TracePolicy::NoDecayDiagnostic) {
            return Err(PlasticityError::InvalidParams(format!(
                "reset_traces_episodic_diagnostic requires trace_policy 'no_decay_diagnostic'; found '{}'",
                self.trace_policy.name()
            )));
        }
        for row in &mut self.e {
            for cell in row.iter_mut() {
                *cell = 0.0;
            }
        }
        Ok(())
    }

    /// Versioned snapshot for checkpoint embedding (M3-10/M4-06). The
    /// effective cache is not stored: restore recomputes it through the
    /// single refresh location from the supplied `w0`. The running baseline
    /// and exactly-once bookkeeping round-trip so a resumed lifetime
    /// continues its teaching signal and dedup without silent resets.
    #[must_use]
    pub fn snapshot(&self) -> PlasticSnapshot {
        PlasticSnapshot {
            schema_version: PLASTIC_SNAPSHOT_SCHEMA_VERSION,
            neuron_count: self.n,
            plastic_mask: self.mask_kind.name().to_owned(),
            trace_policy: self.trace_policy.name().to_owned(),
            tau_e: self.tau_e_config,
            plastic_bound: self.plastic_bound,
            p: self.p.clone(),
            e: self.e.clone(),
            reward_baseline: self.reward_baseline,
            last_feedback: self.last_feedback,
        }
    }

    /// Restore validated state onto a topology plus its immutable `w0`.
    /// Rejects schema, dimension, finiteness, bound/config disagreement,
    /// out-of-bound offsets, mask disagreement, nonzero `P`/`E` on missing
    /// or nonplastic entries, and nonfinite baseline instead of defaulting.
    /// Pre-v3 snapshots lack at least one required lifetime invariant and are
    /// rejected as incompatible.
    pub fn restore(
        snapshot: PlasticSnapshot,
        topology: &Topology,
        w0: &[Vec<f64>],
        expected_plastic_bound: f64,
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
        validate_plastic_bound(expected_plastic_bound)?;
        validate_plastic_bound(snapshot.plastic_bound)?;
        if snapshot.plastic_bound != expected_plastic_bound {
            return Err(PlasticityError::Incompatible(format!(
                "snapshot plastic_bound {} does not match resolved {}",
                snapshot.plastic_bound, expected_plastic_bound
            )));
        }
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
        if snapshot
            .p
            .iter()
            .flatten()
            .any(|value| value.abs() > expected_plastic_bound)
        {
            return Err(PlasticityError::Corrupt(format!(
                "P exceeds plastic_bound {expected_plastic_bound}"
            )));
        }
        if !snapshot.reward_baseline.is_finite() {
            return Err(PlasticityError::Corrupt(
                "reward_baseline must be finite".to_owned(),
            ));
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
            plastic_bound: snapshot.plastic_bound,
            p: snapshot.p,
            e: snapshot.e,
            w_effective: vec![vec![0.0; n]; n],
            reward_baseline: snapshot.reward_baseline,
            last_feedback: snapshot.last_feedback,
        };
        state.refresh_effective(w0)?;
        Ok(state)
    }
}

/// Versioned plastic-state snapshot. `deny_unknown_fields` rejects files
/// claiming future plasticity fields instead of silently defaulting them.
/// `last_feedback` must be present even when null so v1 files without
/// exactly-once bookkeeping fail to parse instead of defaulting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlasticSnapshot {
    pub schema_version: u32,
    pub neuron_count: usize,
    pub plastic_mask: String,
    pub trace_policy: String,
    pub tau_e: f64,
    pub plastic_bound: f64,
    pub p: Vec<Vec<f64>>,
    pub e: Vec<Vec<f64>>,
    pub reward_baseline: f64,
    #[serde(deserialize_with = "crate::checkpoint::required_option")]
    pub last_feedback: Option<u64>,
}

fn validate_plastic_bound(plastic_bound: f64) -> Result<(), PlasticityError> {
    if !(plastic_bound.is_finite() && plastic_bound > 0.0) {
        return Err(PlasticityError::InvalidParams(format!(
            "plastic_bound must be finite and > 0; found {plastic_bound}"
        )));
    }
    Ok(())
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
                let state = PlasticState::new(&topology, &w0, mask, trace, 32.0, 0.5).unwrap();
                assert!(state.p().iter().flatten().all(|&v| v == 0.0));
                assert!(state.e().iter().flatten().all(|&v| v == 0.0));
                assert_eq!(state.effective_weights(), &w0);
            }
        }
    }
}
