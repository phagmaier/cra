//! Nonplastic continuous actor (M1-07, B3).
//!
//! Spec: 3.4 (only `Observation` crosses into the agent; only `MotorOutput`
//! leaves), 5.5–5.8 (public `K + 6` features carry cue/go/outcome/previous
//! action; actions do not perturb the exogenous schedule), 6.1/6.4 (actor
//! transition plus fixed motor readout), 9 (feedback is consumed before the
//! current neural transition; `advance` never applies feedback a second
//! time), 10.4 (birth `h = a = q = 0`), 13.1 (B3: same inherited actor with
//! plasticity disabled — a negative control, not a B7 activity-only
//! optimum).
//!
//! Scope: continuously evolving actor state plus fixed motor filters with
//! **no weight learning**. Public reward arrives only as ordinary sensory
//! channels (`outcome-present`, `outcome-value`) inside `advance`; the
//! `apply_feedback` path only deduplicates event identity and never touches
//! weights, activity, or filters. Inherited `W0` is stored once and never
//! mutated. Plastic offsets, eligibility, gates, and search arrive in later
//! milestones; this module must not grow them.
//!
//! Seed ownership (spec 5.8, 20.5): inherited sampling uses the outer-seed
//! `init` tuple `(root, namespace, outer, lifetime 0, "init")` so every
//! lifetime under one outer seed shares the mask and weights, pairing B3
//! with future B4/B5/B6 conditions by construction. Per-lifetime draws use
//! only `actor_noise` (one perturbation per neuron per tick, every phase)
//! and `tie_break` (exact commitment ties only). The environment never
//! draws either stream, so agent stepping cannot shift cue/change/noise/
//! timing schedules.

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::agent::actor::{ActorError, ActorState};
use crate::agent::health::{HealthError, check_state};
use crate::agent::motor::{MotorError, MotorState, decide_action};
use crate::agent::topology::DEFAULT_MAX_STRUCTURAL_ATTEMPTS;
use crate::agent::weights::{InheritedParams, ParamsError, sample_inherited};
use crate::config::{Actor, Config};
use crate::environment::{Feedback, MotorOutput, SimError, feature_dim};
use crate::rng::{RngState, SeedTuple, rng_for};

/// Nonplastic actor construction/advance failures. Duplicate feedback uses
/// [`SimError::DuplicateFeedback`] through the `Agent` trait (state
/// unchanged); every variant here is an explicit error, never a silent
/// default.
#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum NoLearningError {
    #[error("actor section missing: a no-learning profile must set [actor]")]
    MissingActor,
    #[error("invalid no-learning configuration: {0}")]
    InvalidConfig(String),
    #[error("inherited parameters error: {0}")]
    Params(#[from] ParamsError),
    #[error("actor transition error: {0}")]
    Actor(#[from] ActorError),
    #[error("motor readout error: {0}")]
    Motor(#[from] MotorError),
    #[error("bad seed tuple: {0}")]
    BadSeed(String),
}

fn sim_from_config(message: String) -> SimError {
    SimError::InvalidConfiguration(message)
}

fn sim_from_actor(err: ActorError, tick: u64) -> SimError {
    match err {
        ActorError::NonFiniteState(component) => SimError::NonFiniteState {
            tick,
            component: format!("no-learning actor {component}"),
        },
        other => sim_from_config(format!("no-learning actor step: {other}")),
    }
}

fn sim_from_motor(err: MotorError) -> SimError {
    sim_from_config(format!("no-learning motor step: {err}"))
}

/// B3: the same inherited actor with lifetime plasticity disabled.
///
/// Birth state is `h = a = q = 0` (spec 10.4); `W0` is immutable for the
/// lifetime. `last_output` holds the most recent continuously available
/// motor readout; commitment reads it through [`select_action`](Self::select_action).
#[derive(Clone, Debug)]
pub struct NoLearningActor {
    actor_cfg: Actor,
    params: InheritedParams,
    state: ActorState,
    motor: MotorState,
    noise_rng: ChaCha8Rng,
    tie_rng: ChaCha8Rng,
    last_output: MotorOutput,
    last_feedback: Option<u64>,
    cue_count: usize,
    ticks_advanced: u64,
    initialization: Option<InitializationRecord>,
}

/// Structural sampling history; no performance-based selection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitializationRecord {
    pub accepted_attempt: u32,
    pub rejected: Vec<crate::agent::topology::AttemptRecord>,
}

/// Serializable dynamic actor snapshot for lifetime checkpoints (M1-09).
/// Live membranes/adaptation/filters plus RNG positions; inherited
/// weights travel in the checkpoint body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentSnapshot {
    pub h: Vec<f64>,
    pub last_perturbations: Vec<f64>,
    #[serde(deserialize_with = "crate::checkpoint::required_option")]
    pub initialization: Option<InitializationRecord>,
    pub a: Vec<f64>,
    pub q: [f64; 2],
    pub last_output: MotorOutput,
    #[serde(deserialize_with = "crate::checkpoint::required_option")]
    pub last_feedback: Option<u64>,
    pub ticks_advanced: u64,
    pub noise_rng: RngState,
    pub tie_rng: RngState,
}

impl NoLearningActor {
    /// Build a birth-state actor for one lifetime.
    ///
    /// Inherited weights come from the outer-seed `init` tuple with
    /// `lifetime_index = 0` (shared across lifetimes under one outer seed);
    /// noise/tie streams use this lifetime's index. Rejects configs without
    /// an `[actor]` section or with neural/search execution the M1 runner
    /// does not implement (enabled learning/evolution, non-fixed gates).
    pub fn new(
        cfg: &Config,
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<Self, NoLearningError> {
        crate::config::validate_actor_no_learning_execution(cfg)
            .map_err(|e| NoLearningError::InvalidConfig(e.to_string()))?;
        let actor_cfg = cfg.actor.clone().ok_or(NoLearningError::MissingActor)?;
        let cue_count = cfg.environment.cue_count;
        let input_dim = feature_dim(cue_count);
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
        let mut actor = Self::from_sampled(
            actor_cfg,
            sampled.params,
            cue_count,
            root_seed,
            namespace,
            outer_seed,
            lifetime_index,
        )?;
        actor.initialization = Some(InitializationRecord {
            accepted_attempt: sampled.accepted_attempt,
            rejected: sampled.rejected,
        });
        Ok(actor)
    }

    /// Explicit-parts constructor for fixtures: skips sampling and takes
    /// caller-owned inherited parameters plus per-lifetime RNGs. Validates
    /// shapes (`W0` square, `B` width matches `K + 6`, pools from the
    /// topology) without reading hidden state.
    pub fn from_parts(
        actor_cfg: Actor,
        params: InheritedParams,
        cue_count: usize,
        noise_rng: ChaCha8Rng,
        tie_rng: ChaCha8Rng,
    ) -> Result<Self, NoLearningError> {
        let n = actor_cfg.neuron_count;
        params.validate(&actor_cfg, feature_dim(cue_count))?;
        if params.topology.neuron_count != n {
            return Err(NoLearningError::InvalidConfig(format!(
                "topology has {} neurons but actor config has {n}",
                params.topology.neuron_count
            )));
        }
        let input_dim = feature_dim(cue_count);
        if params.input_dim() != input_dim {
            return Err(NoLearningError::InvalidConfig(format!(
                "sensory width {} must match K + 6 = {input_dim} for {cue_count} cues",
                params.input_dim()
            )));
        }
        Ok(Self {
            actor_cfg,
            params,
            state: ActorState::new(n).map_err(NoLearningError::Actor)?,
            motor: MotorState::new(),
            noise_rng,
            tie_rng,
            last_output: MotorOutput {
                action_0: 0.0,
                action_1: 0.0,
            },
            last_feedback: None,
            cue_count,
            ticks_advanced: 0,
            initialization: None,
        })
    }

    fn from_sampled(
        actor_cfg: Actor,
        params: InheritedParams,
        cue_count: usize,
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<Self, NoLearningError> {
        let noise_rng = rng_for(&SeedTuple::new(
            root_seed,
            namespace,
            outer_seed,
            lifetime_index,
            "actor_noise",
        ))
        .map_err(|e| NoLearningError::BadSeed(e.to_string()))?;
        let tie_rng = rng_for(&SeedTuple::new(
            root_seed,
            namespace,
            outer_seed,
            lifetime_index,
            "tie_break",
        ))
        .map_err(|e| NoLearningError::BadSeed(e.to_string()))?;
        Self::from_parts(actor_cfg, params, cue_count, noise_rng, tie_rng)
    }

    pub fn initialization(&self) -> Option<&InitializationRecord> {
        self.initialization.as_ref()
    }

    /// Inherited actor configuration (scalar time constants, scales).
    pub fn actor_config(&self) -> &Actor {
        &self.actor_cfg
    }

    /// Inherited topology plus `W0`/`B`/biases. `W0` is never mutated by
    /// `apply_feedback` or `advance`; tests compare it before/after runs.
    pub fn inherited(&self) -> &InheritedParams {
        &self.params
    }

    /// Live actor state (`h`/`a`/`r` plus the last perturbation vector).
    pub fn actor_state(&self) -> &ActorState {
        &self.state
    }

    /// Live motor filter state (`q[0]`, `q[1]`).
    pub fn motor_state(&self) -> &MotorState {
        &self.motor
    }

    /// Most recent continuously available readout (birth `[0, 0]`).
    pub fn last_output(&self) -> MotorOutput {
        self.last_output
    }

    /// Number of `advance` calls since birth (one per tick, all phases).
    pub fn ticks_advanced(&self) -> u64 {
        self.ticks_advanced
    }

    /// Highest consumed feedback id, if any (deduplication bookkeeping).
    pub fn last_feedback(&self) -> Option<u64> {
        self.last_feedback
    }

    /// Read-only numerical health of the current state: finiteness plus
    /// the conservative finite watchdog from [`crate::agent::health`].
    /// Borrows immutably, draws no randomness, mutates nothing — diagnostic
    /// reads never perturb the trajectory (M1-08 logging invariance).
    pub fn health_check(&self) -> Result<(), HealthError> {
        check_state(
            self.ticks_advanced,
            self.state.h(),
            self.state.a(),
            self.state.r(),
            self.motor.q(),
        )
    }

    /// Stream names owned by this actor (spec 5.8): perturbations plus
    /// commitment ties. The environment never draws either stream.
    const NOISE_STREAM: &'static str = "actor_noise";
    const TIE_STREAM: &'static str = "tie_break";

    /// Snapshot the dynamic actor state for a lifetime checkpoint (M1-09):
    /// membranes, adaptation, filters, readout, feedback bookkeeping, tick
    /// count, and both RNG positions. Inherited weights travel in the
    /// checkpoint body, not here.
    pub(crate) fn snapshot(
        &self,
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<AgentSnapshot, NoLearningError> {
        let bad = |e: crate::rng::SeedError| NoLearningError::BadSeed(e.to_string());
        let noise = RngState::capture(
            &self.noise_rng,
            &SeedTuple::new(
                root_seed,
                namespace,
                outer_seed,
                lifetime_index,
                Self::NOISE_STREAM,
            ),
        )
        .map_err(bad)?;
        let tie = RngState::capture(
            &self.tie_rng,
            &SeedTuple::new(
                root_seed,
                namespace,
                outer_seed,
                lifetime_index,
                Self::TIE_STREAM,
            ),
        )
        .map_err(bad)?;
        Ok(AgentSnapshot {
            h: self.state.h().to_vec(),
            last_perturbations: self.state.last_perturbations().to_vec(),
            initialization: self.initialization.clone(),
            a: self.state.a().to_vec(),
            q: self.motor.q(),
            last_output: self.last_output,
            last_feedback: self.last_feedback,
            ticks_advanced: self.ticks_advanced,
            noise_rng: noise,
            tie_rng: tie,
        })
    }

    /// Restore dynamic actor state onto already-validated inherited
    /// parameters, validating shapes, finiteness, and RNG seed identity
    /// instead of trusting stored bytes. Missing state is an explicit
    /// error, never a silent default.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn restore(
        snapshot: AgentSnapshot,
        actor_cfg: Actor,
        params: InheritedParams,
        cue_count: usize,
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<Self, NoLearningError> {
        let invalid = |reason: String| NoLearningError::InvalidConfig(reason);
        let n = actor_cfg.neuron_count;
        params.validate(&actor_cfg, feature_dim(cue_count))?;
        if params.topology.neuron_count != n {
            return Err(invalid(format!(
                "topology has {} neurons but actor config has {n}",
                params.topology.neuron_count
            )));
        }
        if snapshot.h.len() != n || snapshot.a.len() != n {
            return Err(invalid(format!(
                "actor state len ({}/{}) must match {n} neurons",
                snapshot.h.len(),
                snapshot.a.len()
            )));
        }
        if !snapshot.h.iter().all(|v| v.is_finite()) || !snapshot.a.iter().all(|v| v.is_finite()) {
            return Err(invalid("restored actor state must be finite".to_owned()));
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
            (Self::NOISE_STREAM, &snapshot.noise_rng),
            (Self::TIE_STREAM, &snapshot.tie_rng),
        ] {
            let tuple = SeedTuple::new(root_seed, namespace, outer_seed, lifetime_index, name);
            let expected = crate::rng::derive_seed_bytes(&tuple)
                .map_err(|e| NoLearningError::BadSeed(e.to_string()))?;
            if state.seed_bytes != expected || state.word_pos >= (1_u128 << 68) {
                return Err(invalid(format!(
                    "rng stream '{name}' seed does not match the checkpoint seed identity"
                )));
            }
        }
        let input_dim = feature_dim(cue_count);
        if params.input_dim() != input_dim {
            return Err(invalid(format!(
                "sensory width {} must match K + 6 = {input_dim} for {cue_count} cues",
                params.input_dim()
            )));
        }
        Ok(Self {
            state: ActorState::from_snapshot(snapshot.h, snapshot.a, snapshot.last_perturbations)
                .map_err(NoLearningError::Actor)?,
            initialization: snapshot.initialization,
            motor: MotorState::from_q(snapshot.q).map_err(NoLearningError::Motor)?,
            last_output: snapshot.last_output,
            last_feedback: snapshot.last_feedback,
            ticks_advanced: snapshot.ticks_advanced,
            noise_rng: snapshot.noise_rng.restore(),
            tie_rng: snapshot.tie_rng.restore(),
            actor_cfg,
            params,
            cue_count,
        })
    }

    /// Choose the committed action from the newest readout. Strict winners
    /// draw nothing; exact ties draw one fair coin from `tie_break`.
    pub fn select_action(&mut self) -> u8 {
        decide_action(self.last_output, &mut self.tie_rng)
    }

    fn check_feedback(&self, event: Feedback) -> Result<(), SimError> {
        if self.last_feedback.is_some_and(|id| event.event_id <= id) {
            return Err(SimError::DuplicateFeedback(event.event_id));
        }
        if event.reward != 0.0 && event.reward != 1.0 {
            return Err(SimError::InvalidConfiguration(
                "feedback reward must be 0 or 1".to_owned(),
            ));
        }
        if !event.reward.is_finite() {
            return Err(SimError::InvalidConfiguration(
                "feedback reward must be finite".to_owned(),
            ));
        }
        Ok(())
    }
}

impl crate::environment::Agent for NoLearningActor {
    /// Deduplicate one feedback event. Performs no weight, activity, or
    /// filter change: the public reward reaches the network only as sensory
    /// channels on the following `advance` calls.
    fn apply_feedback(&mut self, event: Feedback) -> Result<(), SimError> {
        self.check_feedback(event)?;
        self.last_feedback = Some(event.event_id);
        Ok(())
    }

    /// One neural transition on ordinary features plus the fixed motor
    /// filter. Advances on every phase (quiet/cue/gap/response/delay/
    /// feedback); never resets state and never applies feedback twice.
    fn advance(&mut self, features: &[f64]) -> Result<MotorOutput, SimError> {
        let want = feature_dim(self.cue_count);
        if features.len() != want {
            return Err(sim_from_config(format!(
                "feature len {} must match K + 6 = {want} for {} cues",
                features.len(),
                self.cue_count
            )));
        }
        let tick = self.ticks_advanced;
        self.state
            .step(&self.actor_cfg, &self.params, features, &mut self.noise_rng)
            .map_err(|e| sim_from_actor(e, tick))?;
        let out = self
            .motor
            .update(
                self.actor_cfg.motor_filter_tau,
                self.state.r(),
                &self.params.topology.motor0,
                &self.params.topology.motor1,
            )
            .map_err(sim_from_motor)?;
        check_state(
            tick,
            self.state.h(),
            self.state.a(),
            self.state.r(),
            self.motor.q(),
        )
        .map_err(|e| e.to_sim_error())?;
        self.last_output = out;
        self.ticks_advanced += 1;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_actor_section_is_an_explicit_error() {
        let cfg: Config =
            toml::from_str(&std::fs::read_to_string("configs/env_smoke.toml").unwrap()).unwrap();
        assert!(cfg.actor.is_none());
        assert!(matches!(
            NoLearningActor::new(&cfg, 1, "development", 1, 0),
            Err(NoLearningError::InvalidConfig(_) | NoLearningError::MissingActor)
        ));
    }
}
