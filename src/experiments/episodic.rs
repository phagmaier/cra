//! Explicitly episodic clean-learning diagnostic runner (M3-04).
//!
//! Spec: 7.6-7.7 (finite-rollout diagnostic with resets between independent
//! rollouts; the main continuous rule never resets mid-lifetime), 9 (feedback
//! consumed before the current neural transition; exactly once), 10.4 (birth
//! `h = a = q = P = E = 0`, baseline `0.5`), 16/M3 (episodic diagnostic with
//! trace decay 1 and one terminal update; not the main continuous result),
//! 19.2 (separate diagnostic mode; do not overload `birth_only`).
//!
//! Scope: a fixed-gate (`gate = 1`) plastic learner on the clean task
//! (`stationary_clean`, 2 cues, zero noise, zero hazard, no gap, delay 1)
//! with `reset_policy = "episodic_diagnostic"` and
//! `trace_policy = "no_decay_diagnostic"` (`lambda_e = 1.0`, exact
//! summation). Each rollout is one choice cycle: it starts with
//! `h = a = q = E = 0` (preserving `P`, baseline, dedup, RNG positions),
//! accumulates eligibility with one update per actor transition, applies
//! exactly one bounded `P` update plus one baseline update at its terminal
//! feedback (pre-transition, via [`PlasticState::apply_feedback_once`]), then
//! resets before the next rollout. `P`/baseline persist across rollouts
//! within one lifetime so learning accumulates; they reset only at the next
//! lifetime birth (`P = E = 0`, baseline from config).
//!
//! This is a diagnostic, not the continuous result (M4 owns `birth_only` +
//! `persistent`). Resolved configs and [`EpisodicSummary`] carry
//! `reset_policy`, `trace_policy`, mode `"episodic_diagnostic"`, and the
//! per-rollout reset ticks so a diagnostic run cannot masquerade as
//! continuous. Checkpoint embedding with nonzero `P`/`E` is M3-10/M4-06;
//! this runner is library-only and does not write M1 checkpoints.
//!
//! Information boundary: the learner sees only `Observation.features` and
//! observed `Feedback.reward` plus its own state. Hidden mappings,
//! correctness, noise bits/rates, hazards, roles, change flags, future
//! schedules, and evaluator annotations never enter the learner; the driver
//! records them evaluator-side only.
//!
//! Agent-construction boundary (handoff constraint): the learner is built
//! from agent-only inputs (`Actor` + `Learning` sections, inherited
//! parameters, cue count, dedicated noise/tie RNGs), never from the full
//! environment `Config` or master seeds. The driver (which owns the full
//! config and seeds) samples inheritance, derives the two RNGs, and calls
//! [`EpisodicLearner::from_agent_parts`].

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::agent::actor::{ActorError, ActorState, leak_alpha};
use crate::agent::health::{HealthError, check_state};
use crate::agent::motor::{MotorError, MotorState, decide_action};
use crate::agent::no_learning::{NoLearningActor, NoLearningError};
use crate::agent::plasticity::{
    FeedbackOutcome, FeedbackUpdateParams, PlasticState, PlasticityError,
};
use crate::agent::topology::{AttemptRecord, DEFAULT_MAX_STRUCTURAL_ATTEMPTS};
use crate::agent::weights::{InheritedParams, ParamsError, sample_inherited};
use crate::config::{Actor, Config, Learning};
use crate::environment::{
    Agent, Feedback, HiddenAnnotation, Lifetime, MotorOutput, SimError, feature_dim,
};
use crate::rng::{SeedTuple, rng_for};

/// Mode label identifying this diagnostic in summaries and logs.
/// Separate from continuous `birth_only`; never silently substituted.
pub const EPISODIC_MODE: &str = "episodic_diagnostic";

/// Condition identifiers for the matched M3-05 control family. All three
/// conditions share one profile, one task schedule, and one inherited actor
/// within a seed; they differ only in the declared mechanism below.
pub const CONDITION_B3: &str = "B3";
pub const CONDITION_B4: &str = "B4";
pub const CONDITION_SHUFFLED: &str = "B4-shuffled";

/// Teaching-signal protocols. `"observed"` applies the environment's reward
/// unchanged; the shuffled variant replaces it per [`ShuffleProtocol`]
/// while the observed reward stays separately recorded.
pub const PROTOCOL_OBSERVED: &str = "observed";
pub const PROTOCOL_SHUFFLED: &str = "independent-fair-shuffle";

/// Dedicated RNG stream for the shuffled-reward corruption draws (M3-05).
/// Control-only: the environment never draws it, and it needs no
/// provenance-table entry because control runs are library-only. Any
/// lowercase name passes tuple validation; the fixed string keeps the
/// protocol re-derivable in tests without hidden state.
pub const SHUFFLE_STREAM: &str = "shuffle_reward";

/// Recorded corruption protocol for the shuffled-reward control (M3-05;
/// spec 17.8).
///
/// `IndependentFairCoin` replaces each outcome's teaching signal with an
/// independent fair coin (`1.0`/`0.0` with probability one half) drawn from
/// [`SHUFFLE_STREAM`]. The draw uses only the public seed tuple — never the
/// hidden mapping, correctness, noise bit, or hazard — so no privileged
/// information reaches the actor. The environment's observed reward is
/// recorded unchanged alongside the applied signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShuffleProtocol {
    IndependentFairCoin,
}

impl ShuffleProtocol {
    /// Draw one corrupted teaching signal. Returns `1.0` or `0.0`.
    pub fn draw_applied(&self, rng: &mut ChaCha8Rng) -> f64 {
        match self {
            Self::IndependentFairCoin => f64::from(rng.random_bool(0.5)),
        }
    }

    /// Canonical protocol name for summaries.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::IndependentFairCoin => PROTOCOL_SHUFFLED,
        }
    }
}

/// Episodic learner/runner failures. Every variant is an explicit error,
/// never a silent default. Duplicate feedback leaves all state unchanged
/// (via the plastic exactly-once path).
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum EpisodicError {
    #[error("invalid episodic configuration: {0}")]
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
    #[error("bad seed tuple: {0}")]
    BadSeed(String),
}

fn invalid(message: String) -> EpisodicError {
    EpisodicError::InvalidConfig(message)
}

impl From<NoLearningError> for EpisodicError {
    /// Map the B3 actor's failures onto the matching episodic variants so
    /// the matched no-learning control shares one error type without
    /// inventing silent defaults.
    fn from(error: NoLearningError) -> Self {
        match error {
            NoLearningError::MissingActor | NoLearningError::InvalidConfig(_) => {
                Self::InvalidConfig(error.to_string())
            }
            NoLearningError::Params(inner) => Self::Params(inner),
            NoLearningError::Actor(inner) => Self::Actor(inner),
            NoLearningError::Motor(inner) => Self::Motor(inner),
            NoLearningError::BadSeed(reason) => Self::BadSeed(reason),
        }
    }
}

/// Structural sampling history for the paired inherited initialization.
/// No performance-based selection; mirrors the B3 record shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodicInitRecord {
    pub accepted_attempt: u32,
    pub rejected: Vec<AttemptRecord>,
}

/// Fixed-gate episodic plastic learner.
///
/// Birth state is `h = a = q = P = E = 0`, baseline from the `Learning`
/// section, `last_output = [0, 0]` (spec 10.4). `W0` lives in `inherited`
/// and is never mutated; `P`/`E`/baseline/dedup live in `plastic`.
/// Noise/tie RNGs are dedicated per-lifetime streams the environment never
/// draws.
#[derive(Clone, Debug)]
pub struct EpisodicLearner {
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
    initialization: Option<EpisodicInitRecord>,
}

impl EpisodicLearner {
    /// Agent-only constructor for fixtures and the episodic driver.
    ///
    /// Takes only the `[actor]` + `[learning]` sections, already-sampled
    /// inherited parameters, the cue count (for the `K + 6` width check),
    /// and the two dedicated RNGs. Reads no hidden state, no schedules, no
    /// master seeds. Requires `learning.enabled = true` and the explicitly
    /// diagnostic `trace_policy = "no_decay_diagnostic"`; any other policy
    /// is an explicit error so this learner cannot silently run the
    /// continuous rule.
    pub fn from_agent_parts(
        actor_cfg: Actor,
        learning: Learning,
        inherited: InheritedParams,
        cue_count: usize,
        noise_rng: ChaCha8Rng,
        tie_rng: ChaCha8Rng,
    ) -> Result<Self, EpisodicError> {
        if !learning.enabled {
            return Err(invalid(
                "episodic learner requires learning.enabled = true".to_owned(),
            ));
        }
        if learning.rule != "gaussian_transition_score" {
            return Err(invalid(format!(
                "unknown learning rule '{}'; expected \"gaussian_transition_score\"",
                learning.rule
            )));
        }
        if learning.trace_policy != "no_decay_diagnostic" {
            return Err(invalid(format!(
                "episodic learner requires trace_policy 'no_decay_diagnostic'; found '{}'",
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
            initialization: None,
        })
    }

    /// Record the structural sampling history (driver-side only).
    pub fn set_initialization(&mut self, record: EpisodicInitRecord) {
        self.initialization = Some(record);
    }

    pub fn initialization(&self) -> Option<&EpisodicInitRecord> {
        self.initialization.as_ref()
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
    /// Must be called before [`Self::advance`] on the feedback tick (spec 9):
    /// it reads pre-feedback `E`/`P`/baseline, applies one bounded update,
    /// updates the baseline once, marks the event consumed, and refreshes
    /// the single effective cache. `E` is read, never reset, here; the
    /// driver resets `E` between rollouts via
    /// [`Self::reset_between_rollouts`]. Returns the full raw/limited/actual
    /// report; the driver must consume (store/log) it, not drop it.
    pub fn apply_feedback(&mut self, event: Feedback) -> Result<FeedbackOutcome, EpisodicError> {
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
    /// eligibility update (spec 7.3, 9 steps 4/6).
    ///
    /// Saves `r_old`, steps the actor with the current effective weights
    /// (`W0 + P`), updates the fixed motor filter, health-checks the new
    /// state, then advances live eligibility from `r_old` and this
    /// transition's receiver perturbations. Never applies feedback a second
    /// time. Advances on every phase; never resets state.
    pub fn advance(&mut self, features: &[f64]) -> Result<MotorOutput, EpisodicError> {
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
        self.state.step_with_effective_weights(
            &self.actor_cfg,
            &self.inherited,
            &effective,
            features,
            &mut self.noise_rng,
        )?;
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
        let xi = self.state.last_perturbations().to_vec();
        let alpha_h = leak_alpha(self.actor_cfg.tau_h);
        self.plastic
            .advance_eligibility(&r_old, &xi, alpha_h, self.actor_cfg.noise_sigma)?;
        self.last_output = out;
        self.ticks_advanced += 1;
        Ok(out)
    }

    /// Choose the committed action from the newest readout. Strict winners
    /// draw nothing; exact ties draw one fair coin from `tie_break`.
    pub fn select_action(&mut self) -> u8 {
        decide_action(self.last_output, &mut self.tie_rng)
    }

    /// Explicitly diagnostic reset at a rollout boundary (M3-04).
    ///
    /// Zeros actor membranes/adaptation (`h = a = 0` with fresh `r`/`xi`),
    /// motor filters (`q = 0`, readout `[0, 0]`), and eligibility traces
    /// (`E = 0` via the diagnostic-only plastic entry point). Preserves
    /// acquired offsets `P`, the running baseline, exactly-once dedup, the
    /// effective cache (unchanged: `P` is unchanged), tick count, and both
    /// RNG positions. The driver logs the reset tick; the next rollout's
    /// eligibility therefore contains only its own scores (`lambda = 1`
    /// summation within the rollout, no cross-choice contamination).
    pub fn reset_between_rollouts(&mut self) -> Result<(), EpisodicError> {
        let n = self.actor_cfg.neuron_count;
        self.state = ActorState::new(n)?;
        self.motor = MotorState::new();
        self.plastic.reset_traces_episodic_diagnostic()?;
        self.last_output = MotorOutput {
            action_0: 0.0,
            action_1: 0.0,
        };
        Ok(())
    }
}

/// One evaluated choice with its terminal update, joining ordinary and
/// hidden records by explicit keys (evaluator-side summary; the learner
/// never sees the hidden half).
#[derive(Clone, Debug, PartialEq)]
pub struct EpisodicChoice {
    pub choice_index: u64,
    pub event_id: u64,
    pub rollout_index: u64,
    pub rollout_start_tick: u64,
    pub cue: usize,
    pub action: u8,
    /// Observed environment reward (always the lifetime's own outcome).
    pub reward: f64,
    /// Teaching signal actually applied to `P`/`baseline`. Equals `reward`
    /// under [`PROTOCOL_OBSERVED`]; follows [`ShuffleProtocol`] under the
    /// shuffled control while `reward` stays separately recorded.
    pub applied_reward: f64,
    pub correct: bool,
    pub noise_bit: bool,
    pub commit_tick: u64,
    pub feedback_tick: u64,
    pub update: FeedbackOutcome,
}

/// Complete-lifetime episodic diagnostic result.
///
/// `resets` holds the tick index starting each rollout (`resets[0] == 0` at
/// birth; each subsequent entry is the tick after the previous feedback).
/// `mode`, `reset_policy`, `trace_policy`, and `profile_name` visibly
/// identify this as the diagnostic, never the continuous condition.
/// `updates` are the per-feedback raw/limited/actual reports in event order;
/// dropping them is a driver bug, not an optimization.
#[derive(Clone, Debug)]
pub struct EpisodicSummary {
    pub policy: &'static str,
    /// Control identity within the matched family: [`CONDITION_B4`] for the
    /// always-on learner, [`CONDITION_SHUFFLED`] for the corrupted-signal
    /// control. Lets analysis tell conditions apart without parsing logs.
    pub condition_id: &'static str,
    /// Whether this run applied lifetime-plastic updates. `false` never
    /// occurs here (the B3 control has its own summary type); it is kept so
    /// a mechanism audit can compare flags across the family.
    pub learning_enabled: bool,
    /// Teaching-signal protocol: [`PROTOCOL_OBSERVED`] or the
    /// [`ShuffleProtocol`] name. The only config-independent mechanism
    /// difference between B4 and the shuffled control.
    pub reward_protocol: &'static str,
    pub profile_name: String,
    pub mode: &'static str,
    pub reset_policy: String,
    pub trace_policy: String,
    pub plastic_mask: String,
    pub lifetime_index: u64,
    pub ticks: u64,
    pub commitments: u64,
    pub outcomes: u64,
    pub mean_reward: f64,
    pub resets: Vec<u64>,
    pub choices: Vec<EpisodicChoice>,
    pub annotations: Vec<HiddenAnnotation>,
    pub initialization: Option<EpisodicInitRecord>,
    /// Final lifetime state for audit: immutable `W0`, acquired `P`,
    /// terminal `E` (includes the final feedback tick's post-outcome scores;
    /// intermediate rollouts reset to zero), effective `W0 + P`, and the
    /// running baseline. Proves `W0` invariance and bound/mask compliance
    /// without a checkpoint file (M3-10 owns replay).
    pub w0: Vec<Vec<f64>>,
    pub final_p: Vec<Vec<f64>>,
    pub final_e: Vec<Vec<f64>>,
    pub final_effective: Vec<Vec<f64>>,
    pub final_baseline: f64,
    pub final_last_feedback: Option<u64>,
}

impl EpisodicSummary {
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

/// Sample the shared inherited actor for one outer seed (M3-05).
///
/// Uses the outer-seed `init` tuple with `lifetime_index = 0`, so every
/// lifetime and every control condition (B3/B4/shuffled) under one outer
/// seed shares the same mask, `W0`, `B`, and motor assignment by
/// construction. Returns the parameters plus their structural sampling
/// history. No performance-based selection: rejections carry structural
/// reasons only.
pub fn sample_matched_inheritance(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
) -> Result<(InheritedParams, EpisodicInitRecord), EpisodicError> {
    crate::config::validate_episodic_execution(cfg)
        .map_err(|e| EpisodicError::InvalidConfig(e.to_string()))?;
    let actor_cfg = cfg
        .actor
        .clone()
        .ok_or_else(|| invalid("episodic execution requires an [actor] section".to_owned()))?;
    let input_dim = feature_dim(cfg.environment.cue_count);
    let init_tuple = SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        0,
        crate::rng::ACTOR_INIT_STREAM,
    );
    let sampled = sample_inherited(
        &actor_cfg,
        input_dim,
        &init_tuple,
        DEFAULT_MAX_STRUCTURAL_ATTEMPTS,
    )?;
    let record = EpisodicInitRecord {
        accepted_attempt: sampled.accepted_attempt,
        rejected: sampled.rejected,
    };
    Ok((sampled.params, record))
}

/// Derive one lifetime's dedicated agent RNGs. The environment never draws
/// either stream, so agent stepping cannot shift cue/change/noise/timing
/// schedules (spec 5.8).
fn lifetime_agent_rngs(
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
) -> Result<(ChaCha8Rng, ChaCha8Rng), EpisodicError> {
    let noise_rng = rng_for(&SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        lifetime_index,
        "actor_noise",
    ))
    .map_err(|e| EpisodicError::BadSeed(e.to_string()))?;
    let tie_rng = rng_for(&SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        lifetime_index,
        "tie_break",
    ))
    .map_err(|e| EpisodicError::BadSeed(e.to_string()))?;
    Ok((noise_rng, tie_rng))
}

/// Run one explicitly episodic diagnostic lifetime.
///
/// The driver owns the full config and seeds: it validates the episodic
/// profile, samples shared inheritance from the outer-seed `init` tuple
/// (pairing masks across future conditions by construction), derives the
/// per-lifetime `actor_noise`/`tie_break` RNGs, builds the agent-only
/// learner, then drives the shared `Lifetime` tick machine with
/// apply-before-advance ordering and logged diagnostic resets. Hidden
/// annotations are recorded evaluator-side only.
pub fn run_episodic_lifetime(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
) -> Result<EpisodicSummary, EpisodicError> {
    crate::config::validate_episodic_execution(cfg)
        .map_err(|e| EpisodicError::InvalidConfig(e.to_string()))?;
    let actor_cfg = cfg
        .actor
        .clone()
        .ok_or_else(|| invalid("episodic execution requires an [actor] section".to_owned()))?;
    let learning = cfg
        .learning
        .clone()
        .ok_or_else(|| invalid("episodic execution requires a [learning] section".to_owned()))?;
    let cue_count = cfg.environment.cue_count;
    let (params, init_record) = sample_matched_inheritance(cfg, root_seed, namespace, outer_seed)?;
    let (noise_rng, tie_rng) =
        lifetime_agent_rngs(root_seed, namespace, outer_seed, lifetime_index)?;

    let mut learner = EpisodicLearner::from_agent_parts(
        actor_cfg.clone(),
        learning.clone(),
        params,
        cue_count,
        noise_rng,
        tie_rng,
    )?;
    learner.set_initialization(init_record.clone());

    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    // Rollout 0 starts at birth (tick 0) with birth-zero state. Each later
    // rollout starts the tick after the previous feedback.
    let mut resets = vec![0u64];
    let mut choices = Vec::new();
    let mut annotations = Vec::new();

    while !lifetime.is_complete() {
        let out = lifetime.advance()?;
        let rollout_index = (resets.len() - 1) as u64;
        let rollout_start_tick = resets[resets.len() - 1];
        if let Some(feedback) = out.observation.feedback {
            // Terminal update for this rollout: pre-transition E only.
            let update = learner.apply_feedback(feedback)?;
            lifetime.note_feedback_consumed(feedback.event_id)?;
            let annotation = out.annotation.clone().ok_or_else(|| {
                EpisodicError::Sim(SimError::InconsistentCounts(
                    "feedback without annotation".to_owned(),
                ))
            })?;
            let action = lifetime.last_action().ok_or_else(|| {
                EpisodicError::Sim(SimError::InconsistentCounts(
                    "feedback without commitment".to_owned(),
                ))
            })?;
            choices.push(EpisodicChoice {
                choice_index: annotation.choice_index,
                event_id: feedback.event_id,
                rollout_index,
                rollout_start_tick,
                cue: annotation.cue_id,
                action,
                reward: feedback.reward,
                applied_reward: feedback.reward,
                correct: annotation.latent_correctness,
                noise_bit: annotation.noise_bit,
                commit_tick: annotation.commit_tick,
                feedback_tick: annotation.outcome_tick,
                update,
            });
            annotations.push(annotation);
        }
        learner.advance(&out.observation.features)?;
        if out.commitment_due {
            let action = learner.select_action();
            lifetime.commit(action)?;
        }
        // Close the rollout after its feedback tick has been fully
        // processed (transition + eligibility for the feedback input are now
        // in E). Discard those post-outcome scores so the next rollout's
        // trace contains only its own scores. No reset after the final
        // feedback: the lifetime is complete.
        if out.observation.feedback.is_some() && !lifetime.is_complete() {
            learner.reset_between_rollouts()?;
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
    Ok(EpisodicSummary {
        policy: policy_name,
        condition_id: CONDITION_B4,
        learning_enabled: true,
        reward_protocol: PROTOCOL_OBSERVED,
        profile_name: cfg.profile_name.clone(),
        mode: EPISODIC_MODE,
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
        final_baseline,
        final_last_feedback,
    })
}

/// One evaluated B3 choice: same rollout bookkeeping as [`EpisodicChoice`]
/// but no update report, because the no-learning control owns no plastic
/// state. The absence of the field is the structural proof, not a dropped
/// log.
#[derive(Clone, Debug, PartialEq)]
pub struct EpisodicNoLearningChoice {
    pub choice_index: u64,
    pub event_id: u64,
    pub rollout_index: u64,
    pub rollout_start_tick: u64,
    pub cue: usize,
    pub action: u8,
    pub reward: f64,
    pub correct: bool,
    pub noise_bit: bool,
    pub commit_tick: u64,
    pub feedback_tick: u64,
}

/// Complete-lifetime B3 episodic control result (M3-05).
///
/// Same task schedule, same inherited actor, same agent RNG streams, and
/// same rollout resets as B4; the only mechanism difference is the absent
/// plasticity (`learning_enabled: false`, no `P`/`E`/baseline fields exist).
#[derive(Clone, Debug)]
pub struct EpisodicNoLearningSummary {
    pub policy: &'static str,
    pub condition_id: &'static str,
    pub learning_enabled: bool,
    pub reward_protocol: &'static str,
    pub profile_name: String,
    pub mode: &'static str,
    pub reset_policy: String,
    pub lifetime_index: u64,
    pub ticks: u64,
    pub commitments: u64,
    pub outcomes: u64,
    pub mean_reward: f64,
    pub resets: Vec<u64>,
    pub choices: Vec<EpisodicNoLearningChoice>,
    pub annotations: Vec<HiddenAnnotation>,
    pub initialization: Option<EpisodicInitRecord>,
    /// Immutable inherited weights (shared with B4 by construction).
    /// Recorded so tests prove `W0` identity across conditions.
    pub w0: Vec<Vec<f64>>,
    pub final_last_feedback: Option<u64>,
}

impl EpisodicNoLearningSummary {
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

/// Run the matched B3 no-update control through the episodic harness.
///
/// Drives the same `Lifetime` schedule with the same inherited parameters
/// and the same per-lifetime agent RNGs as [`run_episodic_lifetime`], but
/// the agent is the plasticity-free [`NoLearningActor`] built agent-only via
/// `from_parts`. Rollout resets zero `h`/`a`/`q`/readout identically;
/// feedback only advances dedup bookkeeping. Compare against B4's
/// `final_p` movement and behavior seed-by-seed: behavior differences can
/// only come from acquired offsets, never from schedules, inheritance, or
/// noise draws.
pub fn run_episodic_no_learning(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
) -> Result<EpisodicNoLearningSummary, EpisodicError> {
    crate::config::validate_episodic_execution(cfg)
        .map_err(|e| EpisodicError::InvalidConfig(e.to_string()))?;
    let actor_cfg = cfg
        .actor
        .clone()
        .ok_or_else(|| invalid("episodic execution requires an [actor] section".to_owned()))?;
    let cue_count = cfg.environment.cue_count;
    let (params, init_record) = sample_matched_inheritance(cfg, root_seed, namespace, outer_seed)?;
    let (noise_rng, tie_rng) =
        lifetime_agent_rngs(root_seed, namespace, outer_seed, lifetime_index)?;

    let mut actor =
        NoLearningActor::from_parts(actor_cfg, params.clone(), cue_count, noise_rng, tie_rng)?;
    let w0 = params.weights.w0.clone();

    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    let mut resets = vec![0u64];
    let mut choices = Vec::new();
    let mut annotations = Vec::new();

    while !lifetime.is_complete() {
        let out = lifetime.advance()?;
        let rollout_index = (resets.len() - 1) as u64;
        let rollout_start_tick = resets[resets.len() - 1];
        if let Some(feedback) = out.observation.feedback {
            actor.apply_feedback(feedback)?;
            lifetime.note_feedback_consumed(feedback.event_id)?;
            let annotation = out.annotation.clone().ok_or_else(|| {
                EpisodicError::Sim(SimError::InconsistentCounts(
                    "feedback without annotation".to_owned(),
                ))
            })?;
            let action = lifetime.last_action().ok_or_else(|| {
                EpisodicError::Sim(SimError::InconsistentCounts(
                    "feedback without commitment".to_owned(),
                ))
            })?;
            choices.push(EpisodicNoLearningChoice {
                choice_index: annotation.choice_index,
                event_id: feedback.event_id,
                rollout_index,
                rollout_start_tick,
                cue: annotation.cue_id,
                action,
                reward: feedback.reward,
                correct: annotation.latent_correctness,
                noise_bit: annotation.noise_bit,
                commit_tick: annotation.commit_tick,
                feedback_tick: annotation.outcome_tick,
            });
            annotations.push(annotation);
        }
        actor.advance(&out.observation.features)?;
        if out.commitment_due {
            let action = actor.select_action();
            lifetime.commit(action)?;
        }
        if out.observation.feedback.is_some() && !lifetime.is_complete() {
            actor.reset_state_episodic_diagnostic()?;
            resets.push(lifetime.tick());
        }
    }

    let outcomes = choices.len() as u64;
    let mean_reward = if choices.is_empty() {
        0.0
    } else {
        choices.iter().map(|c| c.reward).sum::<f64>() / choices.len() as f64
    };
    Ok(EpisodicNoLearningSummary {
        policy: policy_name,
        condition_id: CONDITION_B3,
        learning_enabled: false,
        reward_protocol: PROTOCOL_OBSERVED,
        profile_name: cfg.profile_name.clone(),
        mode: EPISODIC_MODE,
        reset_policy: cfg.simulation.reset_policy.clone(),
        lifetime_index,
        ticks: lifetime.tick(),
        commitments: lifetime.commitments(),
        outcomes,
        mean_reward,
        resets,
        choices,
        annotations,
        initialization: Some(EpisodicInitRecord {
            accepted_attempt: init_record.accepted_attempt,
            rejected: init_record.rejected,
        }),
        w0,
        final_last_feedback: actor.last_feedback(),
    })
}

/// Run the shuffled-reward control: the B4 learner with a corrupted
/// teaching signal (M3-05; spec 17.8).
///
/// Identical inheritance, schedule, RNG streams, resets, and update
/// arithmetic to [`run_episodic_lifetime`], except each feedback's applied
/// reward is replaced per `protocol` (drawn from [`SHUFFLE_STREAM` with the
/// same seed tuple, using only public seeds). The environment's observed
/// reward is recorded unchanged in `choice.reward`; the corrupted signal
/// lands in `choice.applied_reward` and drives both the `P` update and the
/// running baseline. A learner that improves here is exploiting a leak, not
/// the task — the expected finding is weight movement without systematic
/// latent-accuracy gain.
pub fn run_episodic_shuffled(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
    protocol: ShuffleProtocol,
) -> Result<EpisodicSummary, EpisodicError> {
    crate::config::validate_episodic_execution(cfg)
        .map_err(|e| EpisodicError::InvalidConfig(e.to_string()))?;
    let actor_cfg = cfg
        .actor
        .clone()
        .ok_or_else(|| invalid("episodic execution requires an [actor] section".to_owned()))?;
    let learning = cfg
        .learning
        .clone()
        .ok_or_else(|| invalid("episodic execution requires a [learning] section".to_owned()))?;
    let cue_count = cfg.environment.cue_count;
    let (params, init_record) = sample_matched_inheritance(cfg, root_seed, namespace, outer_seed)?;
    let (noise_rng, tie_rng) =
        lifetime_agent_rngs(root_seed, namespace, outer_seed, lifetime_index)?;
    let mut shuffle_rng = rng_for(&SeedTuple::new(
        root_seed,
        namespace,
        outer_seed,
        lifetime_index,
        SHUFFLE_STREAM,
    ))
    .map_err(|e| EpisodicError::BadSeed(e.to_string()))?;

    let mut learner = EpisodicLearner::from_agent_parts(
        actor_cfg.clone(),
        learning.clone(),
        params,
        cue_count,
        noise_rng,
        tie_rng,
    )?;
    learner.set_initialization(init_record.clone());

    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    let mut resets = vec![0u64];
    let mut choices = Vec::new();
    let mut annotations = Vec::new();

    while !lifetime.is_complete() {
        let out = lifetime.advance()?;
        let rollout_index = (resets.len() - 1) as u64;
        let rollout_start_tick = resets[resets.len() - 1];
        if let Some(feedback) = out.observation.feedback {
            let applied_reward = protocol.draw_applied(&mut shuffle_rng);
            let applied = Feedback {
                event_id: feedback.event_id,
                reward: applied_reward,
            };
            let update = learner.apply_feedback(applied)?;
            lifetime.note_feedback_consumed(feedback.event_id)?;
            let annotation = out.annotation.clone().ok_or_else(|| {
                EpisodicError::Sim(SimError::InconsistentCounts(
                    "feedback without annotation".to_owned(),
                ))
            })?;
            let action = lifetime.last_action().ok_or_else(|| {
                EpisodicError::Sim(SimError::InconsistentCounts(
                    "feedback without commitment".to_owned(),
                ))
            })?;
            choices.push(EpisodicChoice {
                choice_index: annotation.choice_index,
                event_id: feedback.event_id,
                rollout_index,
                rollout_start_tick,
                cue: annotation.cue_id,
                action,
                reward: feedback.reward,
                applied_reward,
                correct: annotation.latent_correctness,
                noise_bit: annotation.noise_bit,
                commit_tick: annotation.commit_tick,
                feedback_tick: annotation.outcome_tick,
                update,
            });
            annotations.push(annotation);
        }
        learner.advance(&out.observation.features)?;
        if out.commitment_due {
            let action = learner.select_action();
            lifetime.commit(action)?;
        }
        if out.observation.feedback.is_some() && !lifetime.is_complete() {
            learner.reset_between_rollouts()?;
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
    Ok(EpisodicSummary {
        policy: policy_name,
        condition_id: CONDITION_SHUFFLED,
        learning_enabled: true,
        reward_protocol: protocol.name(),
        profile_name: cfg.profile_name.clone(),
        mode: EPISODIC_MODE,
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
        final_baseline,
        final_last_feedback,
    })
}

/// Matched B3/B4/shuffled control set for one lifetime (M3-05).
///
/// Runs all three conditions with the same config, seeds, inherited actor,
/// schedule, and agent streams. Pairing is by construction (shared `init`
/// tuple), and tests prove it by comparing `w0`, initialization records,
/// cue orders, and reset ticks across the set.
pub struct EpisodicConditionSet {
    pub b3: EpisodicNoLearningSummary,
    pub b4: EpisodicSummary,
    pub shuffled: EpisodicSummary,
}

/// Run the full matched control set for one lifetime development seed.
pub fn run_episodic_conditions(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
) -> Result<EpisodicConditionSet, EpisodicError> {
    Ok(EpisodicConditionSet {
        b3: run_episodic_no_learning(cfg, root_seed, namespace, outer_seed, lifetime_index, "B3")?,
        b4: run_episodic_lifetime(cfg, root_seed, namespace, outer_seed, lifetime_index, "B4")?,
        shuffled: run_episodic_shuffled(
            cfg,
            root_seed,
            namespace,
            outer_seed,
            lifetime_index,
            "B4-shuffled",
            ShuffleProtocol::IndependentFairCoin,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn episodic_config() -> Config {
        let text =
            std::fs::read_to_string("configs/episodic_stationary.toml").expect("episodic profile");
        toml::from_str(&text).expect("episodic parses")
    }

    #[test]
    fn episodic_profile_passes_its_own_guard_only() {
        let cfg = episodic_config();
        crate::config::validate(&cfg).expect("schema validates");
        crate::config::validate_episodic_execution(&cfg).expect("episodic executes");
        assert!(matches!(
            crate::config::validate_baseline_execution(&cfg),
            Err(crate::config::ConfigError::UnsupportedExecution(_))
        ));
        assert!(matches!(
            crate::config::validate_actor_no_learning_execution(&cfg),
            Err(crate::config::ConfigError::UnsupportedExecution(_))
        ));
    }
}
