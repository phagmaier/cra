//! B0 (random action), B1 (constant action), and the O1 hidden-state oracle
//! (M0-10, spec 13.1/13.3).
//!
//! Information discipline:
//!
//! - B0/B1 implement the ordinary [`Agent`] trait: they receive only
//!   `Observation`s and their own state. Their `advance` ignores features
//!   (no motor circuit exists at M0; the harness commits their
//!   `select_action`), and `apply_feedback` keeps no reward history. This
//!   type-level conformance is the proof they use only permitted
//!   information.
//! - O1 deliberately does *not* implement `Agent`. It reads [`HiddenState`]
//!   directly through an evaluator-only method. It must never appear on an
//!   ordinary code path; the harness keeps the two runners separate
//!   (`run_ordinary` vs `run_oracle`).
//!
//! Scheduling fairness: every baseline runs the same [`Lifetime`] driver
//! with the same production [`commit`](Lifetime::commit) path. Same seed
//! tuples yield the same exogenous schedule — including the same per-choice
//! noise bits — while each baseline's reward follows its own action. No
//! forced or shared rewards cross agents.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::Config;
use crate::environment::{
    Agent, Feedback, HiddenAnnotation, HiddenState, Lifetime, MotorOutput, SimError, TickOutput,
};
use crate::rng::{SeedTuple, rng_for};

/// Ordinary (non-privileged) commitment policy: sees the current tick
/// output — including the publicly shown cue — and its own state. Never
/// hidden mappings, correctness, noise, hazards, or future schedules.
pub trait OrdinaryPolicy: Agent {
    fn select_action(&mut self, tick: &TickOutput) -> u8;
}

/// B0: random action. Draws come from the action-selection (`tie_break`)
/// stream of the same seed tuple — an agent-side stream the environment
/// never touches, so B0 draws cannot perturb exogenous schedules.
pub struct RandomBaseline {
    rng: ChaCha8Rng,
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
        Ok(Self { rng })
    }
}

impl Agent for RandomBaseline {
    fn apply_feedback(&mut self, _event: Feedback) -> Result<(), SimError> {
        Ok(())
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
    fn select_action(&mut self, _tick: &TickOutput) -> u8 {
        u8::from(self.rng.random_bool(0.5))
    }
}

/// B1: constant action (always 0 or always 1). Draws no randomness.
pub struct ConstantBaseline(pub u8);

impl ConstantBaseline {
    pub fn new(action: u8) -> Result<Self, SimError> {
        if action > 1 {
            return Err(SimError::InvalidAction(action));
        }
        Ok(Self(action))
    }
}

impl Agent for ConstantBaseline {
    fn apply_feedback(&mut self, _event: Feedback) -> Result<(), SimError> {
        Ok(())
    }

    fn advance(&mut self, _features: &[f64]) -> Result<MotorOutput, SimError> {
        Ok(MotorOutput {
            action_0: 0.0,
            action_1: 0.0,
        })
    }
}

impl OrdinaryPolicy for ConstantBaseline {
    fn select_action(&mut self, _tick: &TickOutput) -> u8 {
        self.0
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
    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    let mut choices = Vec::new();
    let mut annotations = Vec::new();
    while !lifetime.is_complete() {
        let out = lifetime.advance()?;
        policy.advance(&out.observation.features)?;
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
        if out.commitment_due {
            let action = policy.select_action(&out);
            lifetime.commit(action)?;
        }
    }
    Ok(BaselineSummary {
        policy: policy_name,
        choices,
        annotations,
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
    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    let oracle = Oracle;
    let mut cycle_cue = None;
    let mut choices = Vec::new();
    let mut annotations = Vec::new();
    while !lifetime.is_complete() {
        let out = lifetime.advance()?;
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
        if out.commitment_due {
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
