//! Failure-isolation diagnostics for the episodic learner (M3-09).
//!
//! Spec: M3 "If it fails: reduce to a single noisy motor unit with a
//! constant input and known preferred action. Inspect update sign before
//! increasing network size or task complexity"; 21.1 (debug in the
//! smallest possible system); 21.2 (separate learning failure from
//! representation failure).
//!
//! Reduction ladder (apply in order; stop at the first step that fails):
//!
//! ```text
//! 1. Sign: one plastic edge, hand-set E, +/-delta -> P moves as
//!    eta * delta * E with raw == eta * delta * E_old exactly.
//!    (tests/m3_reduction.rs single_edge_update_sign)
//! 2. Order: the applied update equals eta * delta * E_old read BEFORE
//!    the feedback transition, never post-feedback scores.
//!    (checked per outcome inside run_single_motor_diagnostic)
//! 3. Minimal closed loop: single noisy motor unit, constant input, known
//!    preferred action, synthetic reward -> offsets drift toward the
//!    preferred action while every update keeps the step-2 identity.
//!    (run_single_motor_diagnostic)
//! 4. Representation: matched B3/B4 with real P movement but locked
//!    behavior (outer-1 pattern) is a representation/birth-lock failure,
//!    not a plasticity failure. Do not "fix" it with evolution or hidden
//!    resets.
//! 5. Sensitivity: receiver-permuted perturbations must change the
//!    updates (run_episodic_permuted_lifetime). Exact equivalence with
//!    the correct assignment means the score wiring is broken and must
//!    be investigated, not tuned around.
//! ```
//!
//! Scope: diagnostic-only helpers. The single-motor runner uses synthetic
//! constant inputs and synthetic rewards (a known preferred action), so it
//! is a mechanism check, never a task result and never a comparison
//! against environment-driven runners. The permuted runner mirrors
//! [`run_episodic_lifetime`](crate::experiments::episodic::run_episodic_lifetime)
//! tick for tick except for the perturbation assignment, so any
//! behavioral difference can only come from that assignment. Neither
//! runner writes M1 checkpoints; learned-offset replay is M3-10 work.
//!
//! Status in M3: the troubleshooting path was NOT needed to rescue
//! acquisition (M3-07/M3-08 pass). It is validated here as working
//! tooling, and step 4 is demonstrated on the documented outer-1
//! birth-locked case. See docs/evidence/m3-09/summary.md for the record.

use rand_chacha::ChaCha8Rng;

use crate::agent::health::HealthSummary;
use crate::agent::plasticity::FeedbackOutcome;
use crate::agent::weights::InheritedParams;
use crate::config::{Actor, Config, Learning};
use crate::environment::{Feedback, HiddenAnnotation, Lifetime};
use crate::experiments::episodic::{
    EpisodicChoice, EpisodicError, EpisodicInitRecord, EpisodicLearner, EpisodicSummary,
    PROTOCOL_OBSERVED, lifetime_agent_rngs, sample_matched_inheritance,
};

/// Condition id for the receiver-permuted diagnostic run. Distinct from
/// [`CONDITION_B4`] so analysis can never mistake a deliberately
/// mis-wired score for the verified learner.
pub const CONDITION_PERMUTED_XI: &str = "B4-permuted-xi";

fn invalid(message: String) -> EpisodicError {
    EpisodicError::InvalidConfig(message)
}

/// One synthetic outcome of the single-motor diagnostic: the chosen
/// action, the synthetic reward, the teaching signal, and the proof that
/// the applied update used the pre-feedback trace (ladder step 2).
#[derive(Clone, Debug)]
pub struct SingleMotorChoice {
    pub outcome_index: u64,
    pub action: u8,
    pub reward: f64,
    pub delta: f64,
    pub baseline_old: f64,
    /// Maximum |raw - eta * delta * E_old| over plastic edges. Zero (up
    /// to floating-point round-trip) proves the update used the
    /// pre-feedback trace with the fixed gate.
    pub max_raw_identity_error: f64,
    /// Whether the `max_update` clamp changed any edge this outcome.
    pub clipped: bool,
    /// The single plastic edge value after the update.
    pub p_edge_after: f64,
    pub update: FeedbackOutcome,
}

/// Complete single-motor diagnostic result.
#[derive(Clone, Debug)]
pub struct SingleMotorSummary {
    pub preferred_action: u8,
    pub ticks_per_outcome: u64,
    pub outcomes: u64,
    pub plastic_edge: (usize, usize),
    pub choices: Vec<SingleMotorChoice>,
    pub final_p: Vec<Vec<f64>>,
    pub final_baseline: f64,
}

impl SingleMotorSummary {
    /// Fraction of outcomes choosing the preferred action over a
    /// half-open range. Empty ranges yield `0.0`.
    pub fn preferred_rate(&self, range: std::ops::Range<usize>) -> f64 {
        let n = range.len();
        if n == 0 {
            return 0.0;
        }
        self.choices[range]
            .iter()
            .filter(|c| c.action == self.preferred_action)
            .count() as f64
            / n as f64
    }
}

/// Minimal closed-loop diagnostic (M3-09 ladder step 3; spec M3 "If it
/// fails" recipe): one plastic edge, constant features every tick, and a
/// synthetic reward for a known preferred action.
///
/// The learner is built from agent-only inputs exactly like the episodic
/// runner; the only non-runner inputs are the constant feature vector
/// and the preferred action. Each outcome runs `ticks_per_outcome`
/// noisy transitions, commits from the newest readout, applies the
/// synthetic reward through the verified exactly-once path, and resets
/// state/traces at the rollout boundary (preserving `P`/baseline/dedup).
/// Requires exactly one plastic edge; anything else is an explicit
/// error, never a silent subset.
#[allow(clippy::too_many_arguments)]
pub fn run_single_motor_diagnostic(
    actor_cfg: Actor,
    learning: Learning,
    inherited: InheritedParams,
    cue_count: usize,
    noise_rng: ChaCha8Rng,
    tie_rng: ChaCha8Rng,
    features: Vec<f64>,
    ticks_per_outcome: u64,
    outcomes: u64,
    preferred_action: u8,
) -> Result<SingleMotorSummary, EpisodicError> {
    if preferred_action > 1 {
        return Err(invalid(format!(
            "preferred_action must be 0 or 1; found {preferred_action}"
        )));
    }
    if ticks_per_outcome < 1 || outcomes < 1 {
        return Err(invalid(format!(
            "ticks_per_outcome ({ticks_per_outcome}) and outcomes ({outcomes}) must each be >= 1"
        )));
    }
    let eta = learning.eta;
    let mut learner = EpisodicLearner::from_agent_parts(
        actor_cfg, learning, inherited, cue_count, noise_rng, tie_rng,
    )?;
    let edges = learner.plastic().plastic_edges().to_vec();
    if edges.len() != 1 {
        return Err(invalid(format!(
            "single-motor diagnostic requires exactly one plastic edge; found {}",
            edges.len()
        )));
    }
    let (rj, si) = edges[0];
    let mut choices = Vec::with_capacity(outcomes as usize);
    for k in 0..outcomes {
        for _ in 0..ticks_per_outcome {
            learner.advance(&features)?;
        }
        let action = learner.select_action();
        let reward = f64::from(action == preferred_action);
        // Pre-feedback trace for the step-2 identity check.
        let e_old = learner.plastic().e().to_vec();
        let baseline_old = learner.reward_baseline();
        let update = learner.apply_feedback(Feedback {
            event_id: k,
            reward,
        })?;
        let delta = reward - baseline_old;
        let mut max_err = 0.0f64;
        let mut clipped = false;
        for (j, (raw_row, e_row)) in update.raw_updates.iter().zip(e_old.iter()).enumerate() {
            for (i, (&raw, &e)) in raw_row.iter().zip(e_row.iter()).enumerate() {
                let expected = eta * delta * e;
                max_err = max_err.max((raw - expected).abs());
                if update.limited_updates[j][i] != raw {
                    clipped = true;
                }
            }
        }
        choices.push(SingleMotorChoice {
            outcome_index: k,
            action,
            reward,
            delta,
            baseline_old,
            max_raw_identity_error: max_err,
            clipped,
            p_edge_after: learner.plastic().p()[rj][si],
            update,
        });
        learner.reset_between_rollouts()?;
    }
    Ok(SingleMotorSummary {
        preferred_action,
        ticks_per_outcome,
        outcomes,
        plastic_edge: (rj, si),
        choices,
        final_p: learner.plastic().p().to_vec(),
        final_baseline: learner.reward_baseline(),
    })
}

/// Episodic lifetime with receiver-permuted perturbations (M3-09 ladder
/// step 5): identical inheritance, schedule, RNG streams, resets, and
/// update arithmetic to
/// [`run_episodic_lifetime`](crate::experiments::episodic::run_episodic_lifetime),
/// except every transition advances eligibility with `xi[perm[j]]` on
/// receiver `j`'s incoming edges via
/// [`EpisodicLearner::advance_with_receiver_permutation`].
/// The identity permutation must reproduce the verified runner exactly
/// (tested); any other permutation deliberately violates the spec 7.2
/// receiver-`xi` contract. Unexpected equivalence with the correct
/// assignment is a wiring bug to investigate, never a tuned-away
/// non-result.
pub fn run_episodic_permuted_lifetime(
    cfg: &Config,
    root_seed: u64,
    namespace: &str,
    outer_seed: u64,
    lifetime_index: u64,
    policy_name: &'static str,
    perm: &[usize],
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
    learner.set_initialization(EpisodicInitRecord {
        accepted_attempt: init_record.accepted_attempt,
        rejected: init_record.rejected.clone(),
    });

    let mut lifetime = Lifetime::new(cfg, root_seed, namespace, outer_seed, lifetime_index)?;
    let mut resets = vec![0u64];
    let mut choices = Vec::new();
    let mut annotations: Vec<HiddenAnnotation> = Vec::new();
    let mut health = HealthSummary::new();

    while !lifetime.is_complete() {
        // Spec 9 step 1: observable input + due feedback (no clock yet).
        let out = lifetime.observe()?;
        let rollout_index = (resets.len() - 1) as u64;
        let rollout_start_tick = resets[resets.len() - 1];
        if let Some(feedback) = out.observation.feedback {
            let update = learner.apply_feedback(feedback)?;
            lifetime.note_feedback_consumed(feedback.event_id)?;
            let annotation = out.annotation.clone().ok_or_else(|| {
                EpisodicError::Sim(crate::environment::SimError::InconsistentCounts(
                    "feedback without annotation".to_owned(),
                ))
            })?;
            let action = lifetime.last_action().ok_or_else(|| {
                EpisodicError::Sim(crate::environment::SimError::InconsistentCounts(
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
        learner.advance_with_receiver_permutation(&out.observation.features, perm)?;
        health.observe(
            lifetime.tick(),
            learner.actor_state().h(),
            learner.actor_state().a(),
            learner.actor_state().r(),
            learner.motor_state().q(),
        )?;
        // Finish before commit: `finish_tick` moves the final response tick
        // into the transient Committed phase, so `commit` still observes
        // `commit_tick = tick - 1` and golden delay accounting is unchanged
        // (M4-01 equivalence note in docs/decisions.md).
        lifetime.finish_tick()?;
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
        condition_id: CONDITION_PERMUTED_XI,
        learning_enabled: true,
        reward_protocol: PROTOCOL_OBSERVED,
        profile_name: cfg.profile_name.clone(),
        mode: crate::experiments::episodic::EPISODIC_MODE,
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
        health,
    })
}
