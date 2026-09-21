//! Fixed-weight, no-decay finite-rollout diagnostic (M2-04; spec 7.6).
//!
//! This isolated harness reuses the actor transition and local score. It is
//! not an `Agent`, does not accept feedback, and does not interpret the main
//! config's `birth_only` or finite `tau_e` as diagnostic resets/no decay.
//! State starts at zero independently of the weights. Parameters and baseline
//! are fixed before any draws; only read-only access is exposed during a run.
//! The caller must choose the baseline independently of this rollout's noise
//! and use an external input/reward rule without direct weight dependence.
//! Hand-injected perturbations are for deterministic fixtures, not statistical
//! gradient claims. No membrane/score/update clipping is performed.

use rand_chacha::ChaCha8Rng;
use serde::Serialize;

use crate::agent::actor::{ActorError, ActorState, leak_alpha};
use crate::agent::score::{ScoreError, conditional_score};
use crate::agent::weights::{InheritedParams, ParamsError};
use crate::config::Actor;

pub const MODE: &str = "fixed_weight_no_decay_rollout";

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum RolloutError {
    #[error("invalid finite-rollout configuration/input: {0}")]
    InvalidInput(&'static str),
    #[error("invalid finite-rollout operation: {0}")]
    InvalidPhase(&'static str),
    #[error("finite-rollout parameters: {0}")]
    Params(#[from] ParamsError),
    #[error("finite-rollout actor: {0}")]
    Actor(#[from] ActorError),
    #[error("finite-rollout score: {0}")]
    Score(#[from] ScoreError),
    #[error("nonfinite finite-rollout result: {0}")]
    NonFinite(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RolloutPhase {
    Collecting,
    Finished,
    /// Numerical failure is terminal; do not reuse this object or count it
    /// as a completed sample. The caller must retain/report the original error.
    Failed,
}

/// Terminal diagnostic quantities, not production plastic offsets.
#[derive(Debug, Serialize)]
pub struct RolloutResult {
    pub mode: &'static str,
    pub steps: u64,
    pub baseline: f64,
    pub reward: f64,
    pub score_sums: Vec<Vec<f64>>,
    pub reward_weighted_scores: Vec<Vec<f64>>,
    /// Optional ONE unclipped terminal update of a COPY of W0. This does not
    /// alter the frozen parameters or automatically enter a subsequent run.
    pub terminal_weights: Option<Vec<Vec<f64>>>,
}

/// Explicit diagnostic lifecycle: construct -> step exactly `horizon` times
/// -> finish once -> optionally reset between independent rollouts.
///
/// No live weights, baseline, noise, initial-state or decay setters exist.
/// Returned references are immutable. For example, this must not compile:
///
/// ```compile_fail
/// use cra::experiments::finite_rollout::FiniteRollout;
/// fn change_live_weight(run: &mut FiniteRollout) {
///     run.parameters().weights.w0[0][1] = 3.0;
/// }
/// ```
///
/// Likewise, running-baseline mutation is not part of this interface:
///
/// ```compile_fail
/// use cra::experiments::finite_rollout::FiniteRollout;
/// fn change_live_baseline(run: &mut FiniteRollout) {
///     run.baseline = 0.9;
/// }
/// ```
pub struct FiniteRollout {
    actor: Actor,
    params: InheritedParams,
    horizon: u64,
    baseline: f64,
    state: ActorState,
    old_r: Vec<f64>,
    score_sums: Vec<Vec<f64>>,
    steps: u64,
    phase: RolloutPhase,
}

impl FiniteRollout {
    pub fn new(
        actor: Actor,
        params: InheritedParams,
        horizon: u64,
        baseline: f64,
    ) -> Result<Self, RolloutError> {
        if horizon == 0 || !baseline.is_finite() {
            return Err(RolloutError::InvalidInput(
                "positive horizon and finite baseline required",
            ));
        }
        params.validate(&actor, params.input_dim())?;
        let n = actor.neuron_count;
        Ok(Self {
            actor,
            params,
            horizon,
            baseline,
            state: ActorState::new(n)?,
            old_r: vec![0.0; n],
            score_sums: vec![vec![0.0; n]; n],
            steps: 0,
            phase: RolloutPhase::Collecting,
        })
    }

    pub fn state(&self) -> &ActorState {
        &self.state
    }
    pub fn parameters(&self) -> &InheritedParams {
        &self.params
    }
    pub fn baseline(&self) -> f64 {
        self.baseline
    }
    pub fn score_sums(&self) -> &[Vec<f64>] {
        &self.score_sums
    }
    pub fn steps(&self) -> u64 {
        self.steps
    }
    pub fn phase(&self) -> RolloutPhase {
        self.phase
    }

    fn check_step(&self, input: &[f64]) -> Result<(), RolloutError> {
        if self.phase != RolloutPhase::Collecting || self.steps == self.horizon {
            return Err(RolloutError::InvalidPhase(
                "cannot step outside the declared horizon",
            ));
        }
        if input.len() != self.params.input_dim() || !input.iter().all(|x| x.is_finite()) {
            return Err(RolloutError::InvalidInput("input shape/finiteness"));
        }
        Ok(())
    }

    /// One normal draw per neuron, using the actor's existing per-tick schedule.
    /// The caller owns and records the independent diagnostic RNG stream.
    pub fn step(&mut self, input: &[f64], rng: &mut ChaCha8Rng) -> Result<(), RolloutError> {
        self.check_step(input)?;
        self.old_r.copy_from_slice(self.state.r());
        let transition = self.state.step(&self.actor, &self.params, input, rng);
        self.complete_step(transition)
    }

    pub fn step_with_perturbations(
        &mut self,
        input: &[f64],
        xi: &[f64],
    ) -> Result<(), RolloutError> {
        self.check_step(input)?;
        if xi.len() != self.actor.neuron_count || !xi.iter().all(|x| x.is_finite()) {
            return Err(RolloutError::InvalidInput("perturbation shape/finiteness"));
        }
        self.old_r.copy_from_slice(self.state.r());
        let transition = self
            .state
            .step_with_perturbations(&self.actor, &self.params, input, xi);
        self.complete_step(transition)
    }

    fn complete_step(&mut self, transition: Result<(), ActorError>) -> Result<(), RolloutError> {
        // A partial transition/accumulation must never be retried or finalized
        // as a successful sample. No reset of failed samples is exposed.
        self.phase = RolloutPhase::Failed;
        transition?;
        let alpha = leak_alpha(self.actor.tau_h);
        for &(j, i) in &self.params.topology.edges {
            let score = conditional_score(
                alpha,
                self.old_r[i],
                self.state.last_perturbations()[j],
                self.actor.noise_sigma,
            )?;
            let sum = self.score_sums[j][i] + score; // exact no-decay policy
            if !sum.is_finite() {
                return Err(RolloutError::NonFinite("score sum"));
            }
            self.score_sums[j][i] = sum;
        }
        self.steps += 1;
        self.phase = RolloutPhase::Collecting;
        Ok(())
    }

    /// Read terminal reward only after all transitions. `None` performs no
    /// weight update; `Some(eta)` produces one updated copy, on existing edges
    /// only. No decay, gates, running baseline, bounds or online updates apply.
    pub fn finish(&mut self, reward: f64, eta: Option<f64>) -> Result<RolloutResult, RolloutError> {
        if self.phase != RolloutPhase::Collecting || self.steps != self.horizon {
            return Err(RolloutError::InvalidPhase(
                "finish requires exactly one complete, unfinished rollout",
            ));
        }
        if !reward.is_finite() || eta.is_some_and(|x| !x.is_finite() || x < 0.0) {
            return Err(RolloutError::InvalidInput(
                "finite reward and nonnegative finite eta required",
            ));
        }
        self.phase = RolloutPhase::Failed;
        let delta = reward - self.baseline;
        if !delta.is_finite() {
            return Err(RolloutError::NonFinite("reward minus baseline"));
        }
        let n = self.actor.neuron_count;
        let mut weighted = vec![vec![0.0; n]; n];
        let mut terminal_weights = eta.map(|_| self.params.weights.w0.clone());
        for &(j, i) in &self.params.topology.edges {
            weighted[j][i] = delta * self.score_sums[j][i];
            if !weighted[j][i].is_finite() {
                return Err(RolloutError::NonFinite("reward-weighted score"));
            }
            if let (Some(rate), Some(weights)) = (eta, terminal_weights.as_mut()) {
                weights[j][i] += rate * weighted[j][i];
                if !weights[j][i].is_finite() {
                    return Err(RolloutError::NonFinite("terminal weight"));
                }
            }
        }
        self.phase = RolloutPhase::Finished;
        Ok(RolloutResult {
            mode: MODE,
            steps: self.steps,
            baseline: self.baseline,
            reward,
            score_sums: self.score_sums.clone(),
            reward_weighted_scores: weighted,
            terminal_weights,
        })
    }

    /// Explicit reset after a successfully finalized independent rollout.
    /// Clears actor state and exact score sums; retains the original weights,
    /// parameters and baseline. Does not rewind or draw the caller-owned RNG.
    /// New baseline/weights require a new diagnostic instance.
    pub fn reset_between_rollouts(&mut self) -> Result<(), RolloutError> {
        if self.phase != RolloutPhase::Finished {
            return Err(RolloutError::InvalidPhase(
                "reset allowed only between completed rollouts",
            ));
        }
        self.state = ActorState::new(self.actor.neuron_count)?;
        self.old_r.fill(0.0);
        for row in &mut self.score_sums {
            row.fill(0.0);
        }
        self.steps = 0;
        self.phase = RolloutPhase::Collecting;
        Ok(())
    }
}
