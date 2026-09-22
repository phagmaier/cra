//! Fully persistent plastic learner for the main continuous condition
//! (M4-02).
//!
//! Spec: 7.3 (persistent eligibility `E <- lambda_e * E + S` with
//! `lambda_e = exp(-1 / tau_e)` every tick, never a `(1 - lambda_e)`
//! score factor), 7.4 (birth baseline from config, one `beta_R` update
//! per consumed outcome), 7.5 (bounded `P` on plastic actor edges only),
//! 7.7 (traces never zeroed at choice boundaries or feedback events),
//! 7.8 (post-commitment scores stay in the live trace; the
//! `commit_snapshot_credit` alternative is a different named rule, not
//! this one), 9 (feedback consumed before the current neural transition;
//! exactly once), 10.4 (birth `h = a = q = P = E = 0`).
//!
//! Scope: fixed-gate (`gate = 1`) plasticity on the persistent trace
//! policy, driven tick-by-tick through the M4-01 split order
//! (observe → apply → advance → finish → commit) by
//! [`run_continuous_lifetime`], which performs no mid-lifetime resets
//! and audits that fact with a birth-only reset log. The separate
//! [`run_event_reset_lifetime`] diagnostic explicitly clears only `E`
//! through a named method and records every clear; the main runner never
//! calls it. Continuity profiles arrived in M4-04. M4-05 records the live
//! pre-feedback trace magnitude in each choice summary for declared
//! timing/`tau_e` sensitivity measurements. Gate heads belong to M6;
//! continuous checkpoints belong to M4-06.
//!
//! Information boundary: the learner sees only `Observation.features`
//! and observed `Feedback.reward` plus its own state. Agent-construction
//! boundary: built from agent-only inputs (`Actor` + `Learning`
//! sections, inherited parameters, cue count, dedicated noise/tie
//! RNGs), never from the full environment `Config` or master seeds.

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::agent::actor::{ActorError, ActorState, leak_alpha};
use crate::agent::health::{HealthError, HealthSummary, check_state};
use crate::agent::motor::{MotorError, MotorState, decide_action};
use crate::agent::plasticity::{
    FeedbackOutcome, FeedbackUpdateParams, PlasticSnapshot, PlasticState, PlasticityError,
};
use crate::agent::weights::{InheritedParams, ParamsError};
use crate::config::{Actor, Config, Learning};
use crate::environment::{
    Feedback, HiddenAnnotation, Lifetime, MotorOutput, SimError, feature_dim,
};
use crate::rng::{RngState, SeedTuple, rng_for};

/// Continuous learner/runner failures. Every variant is an explicit error,
/// never a silent default. Duplicate feedback leaves all state unchanged
/// (via the plastic exactly-once path).
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ContinuousError {
    #[error("invalid continuous configuration: {0}")]
    InvalidConfig(String),
    #[error("inherited parameters error: {0}")]
    Params(#[from] ParamsError),
    #[error("actor transition error: {0}")]
    Actor(#[from] ActorError),
    #[error("motor readout error: {0}")]
    Motor(#[from] MotorError),
    #[error("plasticity error: {0}")]
    Plastic(#[from] PlasticityError),
    #[error("numerical health error: {0}")]
    Health(#[from] HealthError),
    #[error("simulation error: {0}")]
    Sim(#[from] SimError),
}

fn invalid(message: String) -> ContinuousError {
    ContinuousError::InvalidConfig(message)
}

/// Fixed-gate fully persistent plastic learner.
///
/// Birth state is `h = a = q = P = E = 0`, baseline from the `Learning`
/// section, `last_output = [0, 0]` (spec 10.4). `W0` lives in `inherited`
/// and is never mutated; `P`/`E`/baseline/dedup live in `plastic`.
/// Noise/tie RNGs are dedicated per-lifetime streams the environment never
/// draws.
#[derive(Clone, Debug)]
pub struct ContinuousLearner {
    actor_cfg: Actor,
    update: FeedbackUpdateParams,
    inherited: InheritedParams,
    state: ActorState,
    motor: MotorState,
    plastic: PlasticState,
    noise_rng: ChaCha8Rng,
    tie_rng: ChaCha8Rng,
    cue_count: usize,
    ticks_advanced: u64,
    last_output: MotorOutput,
}

impl ContinuousLearner {
    /// Agent-only constructor for fixtures and the future continuous
    /// driver.
    ///
    /// Takes only the `[actor]` + `[learning]` sections, already-sampled
    /// inherited parameters, the cue count (for the `K + 6` width check),
    /// and the two dedicated RNGs. Reads no hidden state, no schedules,
    /// no master seeds. Requires `learning.enabled = true`, the
    /// `gaussian_transition_score` rule, `trace_policy = "persistent"`
    /// (the diagnostic `no_decay_diagnostic` policy is an explicit error
    /// here, mirroring the episodic learner's rejection in the other
    /// direction), and `plastic_decay = 0.0`.
    pub fn from_agent_parts(
        actor_cfg: Actor,
        learning: Learning,
        inherited: InheritedParams,
        cue_count: usize,
        noise_rng: ChaCha8Rng,
        tie_rng: ChaCha8Rng,
    ) -> Result<Self, ContinuousError> {
        if !learning.enabled {
            return Err(invalid(
                "continuous learner requires learning.enabled = true".to_owned(),
            ));
        }
        if learning.rule != "gaussian_transition_score" {
            return Err(invalid(format!(
                "unknown learning rule '{}'; expected \"gaussian_transition_score\"",
                learning.rule
            )));
        }
        if learning.trace_policy != "persistent" {
            return Err(invalid(format!(
                "continuous learner requires trace_policy 'persistent'; found '{}'",
                learning.trace_policy
            )));
        }
        if learning.plastic_decay != 0.0 {
            return Err(invalid(format!(
                "plastic_decay must be exactly 0.0; found {}",
                learning.plastic_decay
            )));
        }
        let n = actor_cfg.neuron_count;
        inherited.validate(&actor_cfg, feature_dim(cue_count))?;
        if inherited.topology.neuron_count != n {
            return Err(invalid(format!(
                "topology has {} neurons but actor config has {n}",
                inherited.topology.neuron_count
            )));
        }
        if learning.enabled
            && learning.eta > 0.0
            && !(actor_cfg.noise_sigma.is_finite() && actor_cfg.noise_sigma > 0.0)
        {
            return Err(invalid(
                "score-noise contract: eta > 0 requires actor.noise_sigma > 0".to_owned(),
            ));
        }
        let plastic = PlasticState::from_learning_config(
            &inherited.topology,
            &inherited.weights.w0,
            &learning,
        )?;
        let update = FeedbackUpdateParams::from_learning_config(&learning);
        Ok(Self {
            actor_cfg,
            update,
            inherited,
            state: ActorState::new(n)?,
            motor: MotorState::new(),
            plastic,
            noise_rng,
            tie_rng,
            cue_count,
            ticks_advanced: 0,
            last_output: MotorOutput {
                action_0: 0.0,
                action_1: 0.0,
            },
        })
    }

    pub fn actor_config(&self) -> &Actor {
        &self.actor_cfg
    }

    /// Inherited topology plus immutable `W0`/`B`/biases. Never mutated by
    /// feedback or advance; tests compare it before/after runs.
    pub fn inherited(&self) -> &InheritedParams {
        &self.inherited
    }

    pub fn actor_state(&self) -> &ActorState {
        &self.state
    }

    pub fn motor_state(&self) -> &MotorState {
        &self.motor
    }

    pub fn plastic(&self) -> &PlasticState {
        &self.plastic
    }

    pub fn last_output(&self) -> MotorOutput {
        self.last_output
    }

    pub fn ticks_advanced(&self) -> u64 {
        self.ticks_advanced
    }

    pub fn reward_baseline(&self) -> f64 {
        self.plastic.reward_baseline()
    }

    pub fn last_feedback(&self) -> Option<u64> {
        self.plastic.last_feedback()
    }

    /// Fixed-gate (`gate = 1`) exactly-once update at feedback arrival.
    ///
    /// Must be called before [`Self::advance`] on the feedback tick (spec
    /// 9): it reads the live pre-feedback `E` — including scores accrued
    /// after commitment during the delay — with the fixed gate and the old
    /// baseline, applies one bounded update, updates the baseline once,
    /// marks the event consumed, and refreshes the single effective
    /// cache. `E` is read, never reset, here and nowhere else. Returns
    /// the full raw/limited/actual report; the driver must consume
    /// (store/log) it, not drop it.
    pub fn apply_feedback(&mut self, event: Feedback) -> Result<FeedbackOutcome, ContinuousError> {
        let n = self.actor_cfg.neuron_count;
        let gates = vec![1.0; n];
        let w0 = self.inherited.weights.w0.clone();
        Ok(self.plastic.apply_feedback_once(
            event.event_id,
            event.reward,
            &gates,
            self.update,
            &w0,
        )?)
    }

    /// One neural transition on ordinary features plus exactly one coupled
    /// eligibility update (spec 7.3, 9 steps 4/6/7).
    ///
    /// Saves `r_old`, steps the actor with the current effective weights
    /// (`W0 + P`), advances live persistent eligibility from `r_old` and
    /// this transition's receiver perturbations
    /// (`E <- lambda_e * E + S` with `lambda_e = exp(-1 / tau_e)`, no
    /// `(1 - lambda_e)` factor), updates the fixed motor filter, then
    /// health-checks the new state. Never applies feedback a second time.
    /// Advances on every tick kind — quiet, cue, gap, response, delay,
    /// and feedback transitions alike — and never resets state.
    pub fn advance(&mut self, features: &[f64]) -> Result<MotorOutput, ContinuousError> {
        let want = feature_dim(self.cue_count);
        if features.len() != want {
            return Err(invalid(format!(
                "feature len {} must match K + 6 = {want} for {} cues",
                features.len(),
                self.cue_count
            )));
        }
        let r_old = self.state.r().to_vec();
        let effective = self.plastic.effective_weights().to_vec();
        let tick = self.ticks_advanced;
        // Spec 9 step 4: actor transition on post-feedback weights.
        self.state.step_with_effective_weights(
            &self.actor_cfg,
            &self.inherited,
            &effective,
            features,
            &mut self.noise_rng,
        )?;
        // Spec 9 step 6: persistent live eligibility from `r_old` and this
        // transition's receiver perturbations.
        let xi = self.state.last_perturbations().to_vec();
        let alpha_h = leak_alpha(self.actor_cfg.tau_h);
        self.plastic
            .advance_eligibility(&r_old, &xi, alpha_h, self.actor_cfg.noise_sigma)?;
        // Spec 9 step 7: motor filters from new activity. Step 8 (future
        // gates) is the fixed gate 1 until M6; step 5 (modulator) absent.
        let out = self.motor.update(
            self.actor_cfg.motor_filter_tau,
            self.state.r(),
            &self.inherited.topology.motor0,
            &self.inherited.topology.motor1,
        )?;
        check_state(
            tick,
            self.state.h(),
            self.state.a(),
            self.state.r(),
            self.motor.q(),
        )?;
        self.last_output = out;
        self.ticks_advanced += 1;
        Ok(out)
    }

    /// Choose the committed action from the newest readout. Strict winners
    /// draw nothing; exact ties draw one fair coin from `tie_break`.
    pub fn select_action(&mut self) -> u8 {
        decide_action(self.last_output, &mut self.tie_rng)
    }

    /// Clear eligibility traces for the explicitly named
    /// `event_reset_diagnostic` condition (M4-04, spec 7.7): set `E = 0`
    /// after a delivered outcome, preserving membranes, adaptation,
    /// motor filters, offsets, baseline, dedup, and RNG positions.
    ///
    /// The fully persistent runner never calls this; its M4-03 audit
    /// (`resets == [0]` plus P-telescoping) proves each primary run kept
    /// every trace. Condition distinction therefore rests on the
    /// driver/config/summary labels, not on type confusion: same
    /// transition core, honestly different trace treatment.
    pub fn reset_traces_event_diagnostic(&mut self) -> Result<(), ContinuousError> {
        Ok(self.plastic.reset_traces_event_diagnostic()?)
    }

    /// Snapshot every live learner value required for exact continuous
    /// pause/resume (M4-06): actor/adaptation state, the most recent
    /// perturbations, motor filters/readout, tick count, both RNG positions,
    /// persistent `P`/`E` plus baseline/dedup, and the derived effective
    /// weight cache. Inherited parameters travel in the checkpoint envelope.
    ///
    /// The cache is stored as an integrity assertion rather than trusted:
    /// restore recomputes `W0 + P` through `PlasticState::restore` and
    /// rejects any disagreement.
    pub(crate) fn snapshot(
        &self,
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<ContinuousAgentSnapshot, ContinuousError> {
        let bad = |e: crate::rng::SeedError| ContinuousError::InvalidConfig(e.to_string());
        let noise_rng = RngState::capture(
            &self.noise_rng,
            &SeedTuple::new(
                root_seed,
                namespace,
                outer_seed,
                lifetime_index,
                NOISE_STREAM,
            ),
        )
        .map_err(bad)?;
        let tie_rng = RngState::capture(
            &self.tie_rng,
            &SeedTuple::new(root_seed, namespace, outer_seed, lifetime_index, TIE_STREAM),
        )
        .map_err(bad)?;
        Ok(ContinuousAgentSnapshot {
            h: self.state.h().to_vec(),
            a: self.state.a().to_vec(),
            last_perturbations: self.state.last_perturbations().to_vec(),
            q: self.motor.q(),
            last_output: self.last_output,
            ticks_advanced: self.ticks_advanced,
            noise_rng,
            tie_rng,
            plastic: self.plastic.snapshot(),
            effective_weights: self.plastic.effective_weights().to_vec(),
        })
    }

    /// Restore a fully persistent learner from a continuous checkpoint.
    /// Every field is required by serde; shapes, finiteness, seed identity,
    /// config identity, plastic bounds/masks, and the derived effective cache
    /// are validated rather than silently repaired or reset.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn restore(
        snapshot: ContinuousAgentSnapshot,
        actor_cfg: Actor,
        learning: Learning,
        inherited: InheritedParams,
        cue_count: usize,
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<Self, ContinuousError> {
        if !learning.enabled {
            return Err(invalid(
                "continuous learner requires learning.enabled = true".to_owned(),
            ));
        }
        if learning.rule != "gaussian_transition_score" {
            return Err(invalid(format!(
                "unknown learning rule '{}'; expected \"gaussian_transition_score\"",
                learning.rule
            )));
        }
        if learning.trace_policy != "persistent" {
            return Err(invalid(format!(
                "continuous learner requires trace_policy 'persistent'; found '{}'",
                learning.trace_policy
            )));
        }
        if learning.plastic_decay != 0.0 {
            return Err(invalid(format!(
                "plastic_decay must be exactly 0.0; found {}",
                learning.plastic_decay
            )));
        }
        let n = actor_cfg.neuron_count;
        inherited.validate(&actor_cfg, feature_dim(cue_count))?;
        if inherited.topology.neuron_count != n {
            return Err(invalid(format!(
                "topology has {} neurons but actor config has {n}",
                inherited.topology.neuron_count
            )));
        }
        if learning.eta > 0.0 && !(actor_cfg.noise_sigma.is_finite() && actor_cfg.noise_sigma > 0.0)
        {
            return Err(invalid(
                "score-noise contract: eta > 0 requires actor.noise_sigma > 0".to_owned(),
            ));
        }
        if snapshot.plastic.plastic_mask != learning.plastic_mask {
            return Err(invalid(format!(
                "snapshot mask '{}' disagrees with resolved '{}'",
                snapshot.plastic.plastic_mask, learning.plastic_mask
            )));
        }
        if snapshot.plastic.trace_policy != learning.trace_policy {
            return Err(invalid(format!(
                "snapshot trace '{}' disagrees with resolved '{}'",
                snapshot.plastic.trace_policy, learning.trace_policy
            )));
        }
        if snapshot.plastic.tau_e != learning.tau_e {
            return Err(invalid(format!(
                "snapshot tau_e {} disagrees with resolved {}",
                snapshot.plastic.tau_e, learning.tau_e
            )));
        }
        if snapshot.h.len() != n || snapshot.a.len() != n || snapshot.last_perturbations.len() != n
        {
            return Err(invalid(format!(
                "agent state len ({}/{}/{}) must match {n} neurons",
                snapshot.h.len(),
                snapshot.a.len(),
                snapshot.last_perturbations.len()
            )));
        }
        if !snapshot.h.iter().all(|v| v.is_finite())
            || !snapshot.a.iter().all(|v| v.is_finite())
            || !snapshot.last_perturbations.iter().all(|v| v.is_finite())
        {
            return Err(invalid("restored agent state must be finite".to_owned()));
        }
        if !snapshot.q.iter().all(|v| v.is_finite()) {
            return Err(invalid("restored motor filters must be finite".to_owned()));
        }
        if !snapshot.last_output.action_0.is_finite() || !snapshot.last_output.action_1.is_finite()
        {
            return Err(invalid("restored motor readout must be finite".to_owned()));
        }
        if snapshot.q != [snapshot.last_output.action_0, snapshot.last_output.action_1] {
            return Err(invalid("motor readout disagrees with filters".to_owned()));
        }
        for (name, state) in [
            (NOISE_STREAM, &snapshot.noise_rng),
            (TIE_STREAM, &snapshot.tie_rng),
        ] {
            let tuple = SeedTuple::new(root_seed, namespace, outer_seed, lifetime_index, name);
            let expected = crate::rng::derive_seed_bytes(&tuple)
                .map_err(|e| ContinuousError::InvalidConfig(e.to_string()))?;
            if state.seed_bytes != expected || state.word_pos >= (1_u128 << 68) {
                return Err(invalid(format!(
                    "rng stream '{name}' seed does not match the checkpoint seed identity"
                )));
            }
        }
        let state = ActorState::from_snapshot(snapshot.h, snapshot.a, snapshot.last_perturbations)?;
        let motor = MotorState::from_q(snapshot.q)?;
        let plastic = PlasticState::restore(
            snapshot.plastic,
            &inherited.topology,
            &inherited.weights.w0,
            learning.plastic_bound,
        )?;
        if snapshot.effective_weights != plastic.effective_weights() {
            return Err(invalid(
                "stored effective-weight cache disagrees with derived W0 + P".to_owned(),
            ));
        }
        check_state(
            snapshot.ticks_advanced,
            state.h(),
            state.a(),
            state.r(),
            motor.q(),
        )?;
        let update = FeedbackUpdateParams::from_learning_config(&learning);
        Ok(Self {
            actor_cfg,
            update,
            inherited,
            state,
            motor,
            plastic,
            noise_rng: snapshot.noise_rng.restore(),
            tie_rng: snapshot.tie_rng.restore(),
            cue_count,
            ticks_advanced: snapshot.ticks_advanced,
            last_output: snapshot.last_output,
        })
    }
}

/// Versioned-envelope payload for the live continuous learner (M4-06).
/// Every optional lifetime value lives in the environment snapshot and uses
/// an explicit required-option deserializer; this agent half has no defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContinuousAgentSnapshot {
    pub h: Vec<f64>,
    pub a: Vec<f64>,
    pub last_perturbations: Vec<f64>,
    pub q: [f64; 2],
    pub last_output: MotorOutput,
    pub ticks_advanced: u64,
    pub noise_rng: RngState,
    pub tie_rng: RngState,
    pub plastic: PlasticSnapshot,
    pub effective_weights: Vec<Vec<f64>>,
}

/// Stream names owned by the continuous learner (spec 5.8): perturbations
/// plus commitment ties. Identical strings to the episodic learner's
/// streams by design, so outer-seed inheritance and per-lifetime agent
/// draws pair across conditions; the environment never draws either
/// stream.
const NOISE_STREAM: &str = "actor_noise";
const TIE_STREAM: &str = "tie_break";

/// Mode label for the never-reset main continuous condition. Distinct
/// from the episodic `"episodic_diagnostic"` mode and from
/// [`EVENT_RESET_MODE`]; carried in summaries and logs, never silently
/// substituted.
pub const CONTINUOUS_MODE: &str = "continuous_persistent";

/// Mode label for the per-outcome trace-clearing interference diagnostic
/// (spec 7.7). Same transition core and activity as the main condition;
/// only `E` is cleared after each delivered outcome. The label plus the
/// resolved `reset_policy` keep it honestly separate from the main
/// model.
pub const EVENT_RESET_MODE: &str = "event_reset_diagnostic";

/// Sample the shared inherited actor for one outer seed.
///
/// Uses the outer-seed `init` tuple with `lifetime_index = 0`, so every
/// lifetime and every present/future control condition under one outer
/// seed shares the same mask, `W0`, `B`, and motor assignment by
/// construction — the same tuple the episodic family uses. Returns the
/// parameters plus their structural sampling history. No
/// performance-based selection: rejections carry structural reasons
/// only.
pub fn sample_matched_inheritance(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
) -> Result<
    (
        InheritedParams,
        crate::experiments::episodic::EpisodicInitRecord,
    ),
    ContinuousError,
> {
    crate::config::validate_continuous_execution(cfg)
        .map_err(|e| ContinuousError::InvalidConfig(e.to_string()))?;
    let actor_cfg = cfg
        .actor
        .clone()
        .ok_or_else(|| invalid("continuous execution requires an [actor] section".to_owned()))?;
    let input_dim = feature_dim(cfg.environment.cue_count);
    let init_tuple = SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        0,
        crate::rng::ACTOR_INIT_STREAM,
    );
    let sampled = crate::agent::weights::sample_inherited(
        &actor_cfg,
        input_dim,
        &init_tuple,
        crate::agent::topology::DEFAULT_MAX_STRUCTURAL_ATTEMPTS,
    )?;
    let record = crate::experiments::episodic::EpisodicInitRecord {
        accepted_attempt: sampled.accepted_attempt,
        rejected: sampled.rejected,
    };
    Ok((sampled.params, record))
}

/// Derive one lifetime's dedicated agent RNGs on the shared
/// `actor_noise`/`tie_break` streams (same tuples as the episodic
/// family, so agent stepping cannot shift exogenous schedules and
/// conditions pair by construction).
fn lifetime_agent_rngs(
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
) -> Result<(rand_chacha::ChaCha8Rng, rand_chacha::ChaCha8Rng), ContinuousError> {
    let bad = |e: crate::rng::SeedError| ContinuousError::InvalidConfig(e.to_string());
    let noise_rng = rng_for(&SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        lifetime_index,
        NOISE_STREAM,
    ))
    .map_err(bad)?;
    let tie_rng = rng_for(&SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        lifetime_index,
        TIE_STREAM,
    ))
    .map_err(bad)?;
    Ok((noise_rng, tie_rng))
}

/// One evaluated continuous choice with its feedback update, joining
/// ordinary and hidden records by explicit keys (evaluator-side summary;
/// the learner never sees the hidden half).
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousChoice {
    pub choice_index: u64,
    pub event_id: u64,
    pub cue: usize,
    pub action: u8,
    pub reward: f64,
    pub correct: bool,
    pub noise_bit: bool,
    pub commit_tick: u64,
    pub feedback_tick: u64,
    /// L1 magnitude of the live eligibility matrix read at feedback,
    /// before applying this choice's update or advancing the feedback tick.
    pub eligibility_l1_before_update: f64,
    pub update: FeedbackOutcome,
}

/// Complete-lifetime continuous-family result (M4-03 main condition,
/// M4-04 event-reset diagnostic).
///
/// `resets` is the reset audit: `[0]` (birth) only for the never-reset
/// main condition; `[0]` plus one post-feedback tick per non-final
/// outcome for the event-reset diagnostic. Any unlogged mid-lifetime
/// reset would fail the tripwires — the main runner contains no reset
/// call, and the diagnostic runner logs every `E` clear. `updates` are
/// the per-feedback raw/limited/actual reports in event order; dropping
/// them is a driver bug, not an optimization.
#[derive(Clone, Debug)]
pub struct ContinuousSummary {
    pub policy: &'static str,
    /// Condition mode label: [`CONTINUOUS_MODE`] for the never-reset main
    /// condition, [`EVENT_RESET_MODE`] for the per-outcome trace-clearing
    /// diagnostic. Together with `reset_policy`/`trace_policy` (resolved
    /// config strings) no summary can be mistaken across conditions.
    pub mode: &'static str,
    pub profile_name: String,
    pub reset_policy: String,
    pub trace_policy: String,
    pub plastic_mask: String,
    pub lifetime_index: u64,
    pub ticks: u64,
    pub commitments: u64,
    pub outcomes: u64,
    pub mean_reward: f64,
    pub resets: Vec<u64>,
    pub choices: Vec<ContinuousChoice>,
    pub annotations: Vec<HiddenAnnotation>,
    pub initialization: Option<crate::experiments::episodic::EpisodicInitRecord>,
    pub w0: Vec<Vec<f64>>,
    pub final_p: Vec<Vec<f64>>,
    pub final_e: Vec<Vec<f64>>,
    pub final_effective: Vec<Vec<f64>>,
    pub final_baseline: f64,
    pub final_last_feedback: Option<u64>,
    /// Read-only numerical diagnostics accumulated on every neural tick.
    pub health: HealthSummary,
}

impl ContinuousSummary {
    pub fn latent_accuracy(&self) -> f64 {
        if self.choices.is_empty() {
            return 0.0;
        }
        self.choices.iter().filter(|c| c.correct).count() as f64 / self.choices.len() as f64
    }

    pub fn mean_reward(&self) -> f64 {
        self.mean_reward
    }
}

/// Run one fully persistent continuous lifetime (M4-03).
///
/// The driver owns the full config and seeds: it validates the
/// continuous profile, samples shared inheritance from the outer-seed
/// `init` tuple (pairing masks across present/future conditions by
/// construction), derives the per-lifetime `actor_noise`/`tie_break`
/// RNGs, builds the agent-only learner, then drives the shared
/// `Lifetime` tick machine in the M4-01 split order (observe → apply →
/// advance → finish → commit) with no resets of any kind. Hidden
/// annotations are recorded evaluator-side only. The returned `resets`
/// audit is `[0]` (birth); the M4-03 tripwire asserts this.
pub fn run_continuous_lifetime(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
) -> Result<ContinuousSummary, ContinuousError> {
    crate::config::validate_continuous_execution(cfg)
        .map_err(|e| ContinuousError::InvalidConfig(e.to_string()))?;
    let actor_cfg = cfg
        .actor
        .clone()
        .ok_or_else(|| invalid("continuous execution requires an [actor] section".to_owned()))?;
    let learning = cfg
        .learning
        .clone()
        .ok_or_else(|| invalid("continuous execution requires a [learning] section".to_owned()))?;
    let cue_count = cfg.environment.cue_count;
    let (params, init_record) = sample_matched_inheritance(cfg, root_seed, namespace, outer_seed)?;
    let (noise_rng, tie_rng) =
        lifetime_agent_rngs(root_seed, namespace, outer_seed, lifetime_index)?;

    let mut learner = ContinuousLearner::from_agent_parts(
        actor_cfg.clone(),
        learning.clone(),
        params,
        cue_count,
        noise_rng,
        tie_rng,
    )?;

    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    // Birth tick 0 is the only reset this lifetime will ever see.
    let resets = vec![0u64];
    let mut choices = Vec::new();
    let mut annotations = Vec::new();
    let mut health = HealthSummary::new();

    while !lifetime.is_complete() {
        // Spec 9 step 1: observable input + due feedback (no clock yet).
        let out = lifetime.observe()?;
        if let Some(feedback) = out.observation.feedback {
            let eligibility_l1_before_update = learner
                .plastic()
                .e()
                .iter()
                .flatten()
                .map(|v| v.abs())
                .sum();
            // Spec 9 step 2: live pre-tick E, fixed gate 1, old baseline.
            let update = learner.apply_feedback(feedback)?;
            lifetime.note_feedback_consumed(feedback.event_id)?;
            let annotation = out.annotation.clone().ok_or_else(|| {
                ContinuousError::Sim(crate::environment::SimError::InconsistentCounts(
                    "feedback without annotation".to_owned(),
                ))
            })?;
            let action = lifetime.last_action().ok_or_else(|| {
                ContinuousError::Sim(crate::environment::SimError::InconsistentCounts(
                    "feedback without commitment".to_owned(),
                ))
            })?;
            choices.push(ContinuousChoice {
                choice_index: annotation.choice_index,
                event_id: feedback.event_id,
                cue: annotation.cue_id,
                action,
                reward: feedback.reward,
                correct: annotation.latent_correctness,
                noise_bit: annotation.noise_bit,
                commit_tick: annotation.commit_tick,
                feedback_tick: annotation.outcome_tick,
                eligibility_l1_before_update,
                update,
            });
            annotations.push(annotation);
        }
        // Spec 9 steps 4/6/7/8 (fixed gate 1; no modulator until M6).
        learner.advance(&out.observation.features)?;
        health.observe(
            lifetime.tick(),
            learner.actor_state().h(),
            learner.actor_state().a(),
            learner.actor_state().r(),
            learner.motor_state().q(),
        )?;
        // Finish before commit: the transient Committed phase exists only
        // between finish and commit (`commit_tick = tick - 1`).
        lifetime.finish_tick()?;
        if out.commitment_due {
            // Spec 9 step 9: commit from the new motor output.
            let action = learner.select_action();
            lifetime.commit(action)?;
        }
        // No rollout/reset boundary here: h/a/q/P/E/baseline persist
        // across every choice and feedback by construction (the learner
        // exposes no reset method). The `resets` audit stays `[0]`.
    }

    let outcomes = choices.len() as u64;
    let mean_reward = if choices.is_empty() {
        0.0
    } else {
        choices.iter().map(|c| c.reward).sum::<f64>() / choices.len() as f64
    };
    let w0 = learner.inherited().weights.w0.clone();
    let final_p = learner.plastic().p().to_vec();
    let final_e = learner.plastic().e().to_vec();
    let final_effective = learner.plastic().effective_weights().to_vec();
    let final_baseline = learner.reward_baseline();
    let final_last_feedback = learner.last_feedback();
    Ok(ContinuousSummary {
        policy: policy_name,
        mode: CONTINUOUS_MODE,
        profile_name: cfg.profile_name.clone(),
        reset_policy: cfg.simulation.reset_policy.clone(),
        trace_policy: learning.trace_policy.clone(),
        plastic_mask: learning.plastic_mask.clone(),
        lifetime_index,
        ticks: lifetime.tick(),
        commitments: lifetime.commitments(),
        outcomes,
        mean_reward,
        resets,
        choices,
        annotations,
        initialization: Some(init_record),
        w0,
        final_p,
        final_e,
        final_effective,
        final_last_feedback,
        final_baseline,
        health,
    })
}

/// Run one event-reset diagnostic lifetime (M4-04, spec 7.7): persistent
/// activity with traces cleared after each delivered outcome.
///
/// Same split-order loop, inheritance, RNG streams, and update arithmetic
/// as [`run_continuous_lifetime`], except that after each non-final
/// feedback tick is fully processed (transition + eligibility for the
/// feedback input are in `E`), only `E` is cleared via
/// [`ContinuousLearner::reset_traces_event_diagnostic`]. Membranes,
/// adaptation, motor filters, offsets, baseline, dedup, and RNG positions
/// persist across the boundary; the cleared tick is appended to `resets`
/// so the summary audit names every clear. No reset follows the final
/// feedback: the lifetime is complete. The returned `mode` is
/// [`EVENT_RESET_MODE`] with the resolved `reset_policy`
/// `"event_reset_diagnostic"`, honestly distinct from the main model.
pub fn run_event_reset_lifetime(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
) -> Result<ContinuousSummary, ContinuousError> {
    crate::config::validate_event_reset_execution(cfg)
        .map_err(|e| ContinuousError::InvalidConfig(e.to_string()))?;
    let actor_cfg = cfg
        .actor
        .clone()
        .ok_or_else(|| invalid("event-reset execution requires an [actor] section".to_owned()))?;
    let learning = cfg
        .learning
        .clone()
        .ok_or_else(|| invalid("event-reset execution requires a [learning] section".to_owned()))?;
    let cue_count = cfg.environment.cue_count;
    // Same outer-seed init tuple as the main condition: shared
    // inheritance by construction. The sampling helper validates the
    // continuous guard, so sample directly under this runner's guard.
    let input_dim = feature_dim(cue_count);
    let init_tuple = SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        0,
        crate::rng::ACTOR_INIT_STREAM,
    );
    let sampled = crate::agent::weights::sample_inherited(
        &actor_cfg,
        input_dim,
        &init_tuple,
        crate::agent::topology::DEFAULT_MAX_STRUCTURAL_ATTEMPTS,
    )?;
    let init_record = crate::experiments::episodic::EpisodicInitRecord {
        accepted_attempt: sampled.accepted_attempt,
        rejected: sampled.rejected,
    };
    let (noise_rng, tie_rng) =
        lifetime_agent_rngs(root_seed, namespace, outer_seed, lifetime_index)?;

    let mut learner = ContinuousLearner::from_agent_parts(
        actor_cfg.clone(),
        learning.clone(),
        sampled.params,
        cue_count,
        noise_rng,
        tie_rng,
    )?;

    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    // Birth tick 0, then one entry per non-final outcome clear.
    let mut resets = vec![0u64];
    let mut choices = Vec::new();
    let mut annotations = Vec::new();
    let mut health = HealthSummary::new();

    while !lifetime.is_complete() {
        // Spec 9 step 1: observable input + due feedback (no clock yet).
        let out = lifetime.observe()?;
        if let Some(feedback) = out.observation.feedback {
            let eligibility_l1_before_update = learner
                .plastic()
                .e()
                .iter()
                .flatten()
                .map(|v| v.abs())
                .sum();
            // Spec 9 step 2: live pre-tick E, fixed gate 1, old baseline.
            let update = learner.apply_feedback(feedback)?;
            lifetime.note_feedback_consumed(feedback.event_id)?;
            let annotation = out.annotation.clone().ok_or_else(|| {
                ContinuousError::Sim(crate::environment::SimError::InconsistentCounts(
                    "feedback without annotation".to_owned(),
                ))
            })?;
            let action = lifetime.last_action().ok_or_else(|| {
                ContinuousError::Sim(crate::environment::SimError::InconsistentCounts(
                    "feedback without commitment".to_owned(),
                ))
            })?;
            choices.push(ContinuousChoice {
                choice_index: annotation.choice_index,
                event_id: feedback.event_id,
                cue: annotation.cue_id,
                action,
                reward: feedback.reward,
                correct: annotation.latent_correctness,
                noise_bit: annotation.noise_bit,
                commit_tick: annotation.commit_tick,
                feedback_tick: annotation.outcome_tick,
                eligibility_l1_before_update,
                update,
            });
            annotations.push(annotation);
        }
        // Spec 9 steps 4/6/7/8 (fixed gate 1; no modulator until M6).
        learner.advance(&out.observation.features)?;
        health.observe(
            lifetime.tick(),
            learner.actor_state().h(),
            learner.actor_state().a(),
            learner.actor_state().r(),
            learner.motor_state().q(),
        )?;
        // Finish before commit: the transient Committed phase exists only
        // between finish and commit (`commit_tick = tick - 1`).
        lifetime.finish_tick()?;
        if out.commitment_due {
            // Spec 9 step 9: commit from the new motor output.
            let action = learner.select_action();
            lifetime.commit(action)?;
        }
        // Event-reset boundary (spec 7.7): after the feedback tick is
        // fully processed, clear only E. Post-outcome scores for the
        // feedback input are discarded with the rest of the trace, so the
        // next choice starts from E = 0 while all other state persists.
        // No reset after the final feedback: the lifetime is complete.
        if out.observation.feedback.is_some() && !lifetime.is_complete() {
            learner.reset_traces_event_diagnostic()?;
            resets.push(lifetime.tick());
        }
    }

    let outcomes = choices.len() as u64;
    let mean_reward = if choices.is_empty() {
        0.0
    } else {
        choices.iter().map(|c| c.reward).sum::<f64>() / choices.len() as f64
    };
    let w0 = learner.inherited().weights.w0.clone();
    let final_p = learner.plastic().p().to_vec();
    let final_e = learner.plastic().e().to_vec();
    let final_effective = learner.plastic().effective_weights().to_vec();
    let final_baseline = learner.reward_baseline();
    let final_last_feedback = learner.last_feedback();
    Ok(ContinuousSummary {
        policy: policy_name,
        mode: EVENT_RESET_MODE,
        profile_name: cfg.profile_name.clone(),
        reset_policy: cfg.simulation.reset_policy.clone(),
        trace_policy: learning.trace_policy.clone(),
        plastic_mask: learning.plastic_mask.clone(),
        lifetime_index,
        ticks: lifetime.tick(),
        commitments: lifetime.commitments(),
        outcomes,
        mean_reward,
        resets,
        choices,
        annotations,
        initialization: Some(init_record),
        w0,
        final_p,
        final_e,
        final_effective,
        final_last_feedback,
        final_baseline,
        health,
    })
}
