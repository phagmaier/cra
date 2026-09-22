//! B0 (random action), B1 (constant action), B3 (nonplastic actor), and
//! the O1 hidden-state oracle (M0-10, M1-07, spec 13.1/13.3).
//!
//! Information discipline:
//!
//! - B0/B1/B3 implement the ordinary [`Agent`] trait: they receive only
//!   `Observation`s and their own state. B0/B1 `advance` ignores features
//!   (no motor circuit exists at M0; the harness commits their
//!   `select_action`); B3 advances its inherited actor plus fixed motor
//!   filter on every tick and commits from its own readout. `apply_feedback`
//!   only deduplicates events (B3 performs no weight change). This
//!   type-level conformance is the proof they use only permitted
//!   information.
//! - O1 deliberately does *not* implement `Agent`. It reads [`HiddenState`]
//!   directly through an evaluator-only method. It must never appear on an
//!   ordinary code path; the harness keeps the two runners separate
//!   (`run_ordinary`/`run_actor_ordinary` vs `run_oracle`).
//!
//! Scheduling fairness: every ordinary runner uses the same [`Lifetime`]
//! driver with the same production [`commit`](Lifetime::commit) path. Same
//! seed tuples yield the same exogenous schedule — including the same
//! per-choice noise bits — while each policy's reward follows its own
//! action. No forced or shared rewards cross agents.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::agent::health::HealthSummary;
use crate::agent::no_learning::NoLearningActor;
use crate::config::Config;
use crate::environment::{
    Agent, Feedback, HiddenAnnotation, HiddenState, Lifetime, MotorOutput, SimError,
};
use crate::rng::{SeedTuple, rng_for};

/// Borrowed neural state used only for read-only health accumulation.
pub type HealthView<'a> = (&'a [f64], &'a [f64], &'a [f64], [f64; 2]);

/// Ordinary commitment policy. All input arrives through `Agent`; action
/// selection reads only its own state, never the evaluator's TickOutput.
pub trait OrdinaryPolicy: Agent {
    fn select_action(&mut self) -> u8;

    /// Optional read-only actor state for numerical-health reporting.
    /// Simple baselines have no neural state; B3 exposes its live actor and
    /// motor filters without drawing randomness or changing behavior.
    fn health_view(&self) -> Option<HealthView<'_>> {
        None
    }
}

fn consume_feedback(last: &mut Option<u64>, event: Feedback) -> Result<(), SimError> {
    if last.is_some_and(|id| event.event_id <= id) {
        return Err(SimError::DuplicateFeedback(event.event_id));
    }
    if event.reward != 0.0 && event.reward != 1.0 {
        return Err(SimError::InvalidConfiguration(
            "feedback reward must be 0 or 1".to_owned(),
        ));
    }
    *last = Some(event.event_id);
    Ok(())
}

/// B0: random action. Draws come from the action-selection (`tie_break`)
/// stream of the same seed tuple — an agent-side stream the environment
/// never touches, so B0 draws cannot perturb exogenous schedules.
pub struct RandomBaseline {
    rng: ChaCha8Rng,
    last_feedback: Option<u64>,
}

impl RandomBaseline {
    pub fn new(
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<Self, SimError> {
        let rng = rng_for(&SeedTuple::new(
            root_seed,
            namespace,
            outer_seed,
            lifetime_index,
            "tie_break",
        ))
        .map_err(|e| SimError::InvalidConfiguration(format!("bad seed tuple: {e}")))?;
        Ok(Self {
            rng,
            last_feedback: None,
        })
    }
}

impl Agent for RandomBaseline {
    fn apply_feedback(&mut self, event: Feedback) -> Result<(), SimError> {
        consume_feedback(&mut self.last_feedback, event)
    }

    fn advance(&mut self, _features: &[f64]) -> Result<MotorOutput, SimError> {
        // No motor circuit at M0; the harness commits select_action().
        Ok(MotorOutput {
            action_0: 0.0,
            action_1: 0.0,
        })
    }
}

impl OrdinaryPolicy for RandomBaseline {
    fn select_action(&mut self) -> u8 {
        u8::from(self.rng.random_bool(0.5))
    }
}

/// B1: constant action (always 0 or always 1). Draws no randomness.
pub struct ConstantBaseline {
    action: u8,
    last_feedback: Option<u64>,
}

impl ConstantBaseline {
    pub fn new(action: u8) -> Result<Self, SimError> {
        if action > 1 {
            return Err(SimError::InvalidAction(action));
        }
        Ok(Self {
            action,
            last_feedback: None,
        })
    }
}

impl Agent for ConstantBaseline {
    fn apply_feedback(&mut self, event: Feedback) -> Result<(), SimError> {
        consume_feedback(&mut self.last_feedback, event)
    }

    fn advance(&mut self, _features: &[f64]) -> Result<MotorOutput, SimError> {
        Ok(MotorOutput {
            action_0: 0.0,
            action_1: 0.0,
        })
    }
}

impl OrdinaryPolicy for ConstantBaseline {
    fn select_action(&mut self) -> u8 {
        self.action
    }
}

/// B3 ordinary commitment for the nonplastic actor: reads only the actor's
/// own latest motor readout, with exact ties broken on the dedicated
/// `tie_break` stream. The impl lives here (rather than in `agent`) so the
/// agent crate never depends on the experiment harness.
impl OrdinaryPolicy for NoLearningActor {
    fn select_action(&mut self) -> u8 {
        NoLearningActor::select_action(self)
    }

    fn health_view(&self) -> Option<HealthView<'_>> {
        Some((
            self.actor_state().h(),
            self.actor_state().a(),
            self.actor_state().r(),
            self.motor_state().q(),
        ))
    }
}

/// O1: hidden-state oracle (privileged, researcher-only). Chooses the
/// preferred action at every commitment, so latent correctness is exactly 1
/// and observed reward on cue `c` averages `1 - epsilon[c]` (spec 13.3).
/// This is a scoring reference, not a fair learner.
pub struct Oracle;

impl Oracle {
    pub fn select_action(&self, hidden: &HiddenState, cue: usize) -> u8 {
        hidden.mapping(cue)
    }
}

/// One evaluated choice, joining ordinary and hidden records by explicit
/// keys (evaluator-side summary; agents never see this).
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceRecord {
    pub choice_index: u64,
    pub event_id: u64,
    pub cue: usize,
    pub action: u8,
    pub reward: f64,
    pub correct: bool,
    pub noise_bit: bool,
    pub commit_tick: u64,
    pub feedback_tick: u64,
}

/// Complete-lifetime baseline result (evaluator side: joins ordinary
/// choices with their hidden annotations by explicit keys).
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineSummary {
    pub policy: &'static str,
    pub choices: Vec<ChoiceRecord>,
    pub annotations: Vec<HiddenAnnotation>,
    /// Present for the neural B3 control and absent for B0/B1/O1.
    pub health: Option<HealthSummary>,
}

impl BaselineSummary {
    pub fn latent_accuracy(&self) -> f64 {
        if self.choices.is_empty() {
            return 0.0;
        }
        self.choices.iter().filter(|c| c.correct).count() as f64 / self.choices.len() as f64
    }

    pub fn mean_reward(&self) -> f64 {
        if self.choices.is_empty() {
            return 0.0;
        }
        self.choices.iter().map(|c| c.reward).sum::<f64>() / self.choices.len() as f64
    }
}

fn record_choice(
    feedback: Feedback,
    annotation: &crate::environment::HiddenAnnotation,
    last_action: Option<u8>,
) -> Result<ChoiceRecord, SimError> {
    Ok(ChoiceRecord {
        choice_index: annotation.choice_index,
        event_id: feedback.event_id,
        cue: annotation.cue_id,
        action: last_action.ok_or_else(|| {
            SimError::InconsistentCounts("feedback without commitment".to_owned())
        })?,
        reward: feedback.reward,
        correct: annotation.latent_correctness,
        noise_bit: annotation.noise_bit,
        commit_tick: annotation.commit_tick,
        feedback_tick: annotation.outcome_tick,
    })
}

/// Run an ordinary (non-privileged) baseline through one full lifetime.
/// The policy observes each tick and commits at commitment points via the
/// production path; the harness records joined choice records.
pub fn run_ordinary(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
    policy: &mut dyn OrdinaryPolicy,
) -> Result<BaselineSummary, SimError> {
    crate::config::validate_baseline_execution(cfg)
        .map_err(|e| SimError::InvalidConfiguration(e.to_string()))?;
    run_ordinary_inner(
        cfg,
        root_seed,
        namespace,
        outer_seed,
        lifetime_index,
        policy_name,
        policy,
    )
}

/// Run the nonplastic actor (B3) through one full lifetime.
///
/// Identical tick/commit/record loop to [`run_ordinary`]: the same
/// [`Lifetime`] driver, the same production commit path, feedback consumed
/// before the neural transition, and the actor advanced on every tick
/// (quiet/cue/gap/response/delay/feedback) with no boundary resets. Only
/// the execution guard differs (actor profile instead of env-only).
pub fn run_actor_ordinary(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
    policy: &mut dyn OrdinaryPolicy,
) -> Result<BaselineSummary, SimError> {
    crate::config::validate_actor_no_learning_execution(cfg)
        .map_err(|e| SimError::InvalidConfiguration(e.to_string()))?;
    run_ordinary_inner(
        cfg,
        root_seed,
        namespace,
        outer_seed,
        lifetime_index,
        policy_name,
        policy,
    )
}

fn run_ordinary_inner(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
    policy: &mut dyn OrdinaryPolicy,
) -> Result<BaselineSummary, SimError> {
    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    let mut choices = Vec::new();
    let mut annotations = Vec::new();
    let mut health = HealthSummary::new();
    let mut has_health = false;
    while !lifetime.is_complete() {
        // Spec 9 step 1: observable input + due feedback (no clock yet).
        let out = lifetime.observe()?;
        if let Some(feedback) = out.observation.feedback {
            // Spec 9 step 2: consume with pre-tick state, exactly once.
            policy.apply_feedback(feedback)?;
            lifetime.note_feedback_consumed(feedback.event_id)?;
            let annotation = out.annotation.clone().ok_or_else(|| {
                SimError::InconsistentCounts("feedback without annotation".to_owned())
            })?;
            choices.push(record_choice(
                feedback,
                &annotation,
                lifetime.last_action(),
            )?);
            annotations.push(annotation);
        }
        // Spec 9 steps 4/6/7/8 (fixed gate 1; no modulator until M6).
        policy.advance(&out.observation.features)?;
        if let Some((h, a, r, q)) = policy.health_view() {
            health
                .observe(lifetime.tick(), h, a, r, q)
                .map_err(|e| e.to_sim_error())?;
            has_health = true;
        }
        // Finish before commit: `finish_tick` moves the final response tick
        // into the transient Committed phase, so `commit` still observes
        // `commit_tick = tick - 1` and golden delay accounting is unchanged
        // (M4-01 equivalence note in docs/decisions.md).
        lifetime.finish_tick()?;
        if out.commitment_due {
            // Spec 9 step 9: commit from the new motor output.
            let action = policy.select_action();
            lifetime.commit(action)?;
        }
    }
    Ok(BaselineSummary {
        policy: policy_name,
        choices,
        annotations,
        health: has_health.then_some(health),
    })
}

/// Run the privileged oracle through one full lifetime. The oracle reads
/// the current hidden mapping for the presented cue at each commitment.
/// Separate runner so oracle privileges never enter ordinary code paths.
pub fn run_oracle(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
) -> Result<BaselineSummary, SimError> {
    crate::config::validate_baseline_execution(cfg)
        .map_err(|e| SimError::InvalidConfiguration(e.to_string()))?;
    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    let oracle = Oracle;
    let mut cycle_cue = None;
    let mut choices = Vec::new();
    let mut annotations = Vec::new();
    while !lifetime.is_complete() {
        // Spec 9 step 1: observable input + due feedback (no clock yet).
        let out = lifetime.observe()?;
        if let Some(feedback) = out.observation.feedback {
            let annotation = out.annotation.clone().ok_or_else(|| {
                SimError::InconsistentCounts("feedback without annotation".to_owned())
            })?;
            choices.push(record_choice(
                feedback,
                &annotation,
                lifetime.last_action(),
            )?);
            annotations.push(annotation);
        }
        if out.cue.is_some() {
            cycle_cue = out.cue;
        }
        // Finish before commit: `finish_tick` moves the final response tick
        // into the transient Committed phase, so `commit` still observes
        // `commit_tick = tick - 1` and golden delay accounting is unchanged
        // (M4-01 equivalence note in docs/decisions.md).
        lifetime.finish_tick()?;
        if out.commitment_due {
            // Spec 9 step 9: privileged commit from hidden truth.
            let cue = cycle_cue.ok_or_else(|| {
                SimError::InconsistentCounts("commitment without a presented cue".to_owned())
            })?;
            lifetime.commit(oracle.select_action(lifetime.hidden(), cue))?;
        }
    }
    Ok(BaselineSummary {
        policy: "oracle",
        choices,
        annotations,
        health: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_baseline_rejects_actions_outside_zero_one() {
        assert!(ConstantBaseline::new(0).is_ok());
        assert!(ConstantBaseline::new(1).is_ok());
        assert!(matches!(
            ConstantBaseline::new(2),
            Err(SimError::InvalidAction(2))
        ));
    }
}
