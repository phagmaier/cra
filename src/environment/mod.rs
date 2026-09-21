//! Continuous environment lifetimes: birth, phase scheduling, observations,
//! commitments, and exactly-once pending rewards (M0-07 through M0-09).
//!
//! The environment owns hidden mappings, schedules, reward delivery, and
//! public observations. It never exposes hidden truth to ordinary agents:
//! [`observation`] carries agent-visible data, [`hidden_state`] carries
//! evaluator-only truth, and the two serialize to separate streams.
//!
//! Tick driver (M0; the agent-interleaved split of spec 9.3 arrives with the
//! neural loop in M1):
//!
//! ```text
//! loop {
//!     let tick = lifetime.advance()?;      // one tick, feedback attached
//!     if tick.commitment_due {
//!         lifetime.commit(action)?;        // final response tick only
//!     }
//!     if lifetime.is_complete() { break; }
//! }
//! ```
//!
//! RNG ownership: the lifetime draws only from `init` (membership shuffle),
//! `mapping_init` (birth mappings), `mapping_change` (hazard flips),
//! `cue_order` (cue choice), `timing` (quiet/gap/delay lengths), and
//! `reward_noise` (one bit per commitment). It never touches `actor_noise`,
//! `tie_break`, or `evolution`, so agent-side draws cannot perturb the
//! exogenous schedule (spec 5.8).

pub mod hidden_state;
pub mod observation;
pub mod schedule;

use rand::distr::{Bernoulli, Distribution};
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::rng::{SeedTuple, rng_for};
pub use hidden_state::{CueRole, HiddenAnnotation, HiddenState};
pub use observation::{Agent, Feedback, MotorOutput, Observation, SimError, feature_dim};
pub use schedule::Phase;
use schedule::{PhaseState, sample_cue, sample_len};

/// Stored pending outcome: sampled once at commitment, delivered once at
/// `due_tick`. Later mapping changes cannot alter it (spec 5.1:
/// correctness is evaluated against the mapping saved at commitment).
/// Evaluator-side data (contains hidden truth); the agent only ever sees
/// [`Feedback`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingReward {
    pub event_id: u64,
    pub choice_index: u64,
    pub cue: usize,
    pub action: u8,
    pub target_at_commit: u8,
    pub correct: bool,
    pub noise_bit: bool,
    pub epsilon: f64,
    pub cue_hazard: f64,
    pub cue_exposure_index: u64,
    pub hidden_change_before_presentation: bool,
    pub role: CueRole,
    pub reward: f64,
    pub commit_tick: u64,
    pub due_tick: u64,
}

/// Minimal driver-visible receipt for a commitment. Timing (`due_tick`) is
/// deliberately absent: the agent must not receive a countdown to the
/// reward (spec 5.6). Tests observe the due tick behaviorally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitReceipt {
    pub event_id: u64,
    pub choice_index: u64,
}

/// One delivered tick: public observation plus driver routing flags. The
/// hidden annotation accompanies feedback ticks for the evaluator/logger
/// only.
#[derive(Clone, Debug, PartialEq)]
pub struct TickOutput {
    pub tick: u64,
    pub phase: Phase,
    pub cue: Option<usize>,
    pub go: bool,
    pub commitment_due: bool,
    pub observation: Observation,
    pub annotation: Option<HiddenAnnotation>,
}

/// A single continuous lifetime: birth-to-completion environment state.
pub struct Lifetime {
    cue_count: usize,
    cue_ticks: u64,
    response_ticks: u64,
    quiet_range: [u64; 2],
    gap_range: [u64; 2],
    delay_range: [u64; 2],
    outcomes_target: u64,
    hidden: HiddenState,
    cue_rng: ChaCha8Rng,
    timing_rng: ChaCha8Rng,
    noise_rng: ChaCha8Rng,
    change_rng: ChaCha8Rng,
    tick: u64,
    phase: PhaseState,
    current_cue: Option<usize>,
    cycle_gap: u64,
    cycle_exposure_index: u64,
    cycle_changed: bool,
    pending: Option<PendingReward>,
    next_event_id: u64,
    commitments: u64,
    outcomes: u64,
    consumed: Vec<u64>,
    confirmed: Vec<u64>,
    last_action: Option<u8>,
}

impl Lifetime {
    /// Begin a fresh lifetime: sample membership, birth mappings, and the
    /// first quiet interval. The first quiet lasts `warmup_ticks` (warmup is
    /// the leading quiet before the first cue, part of the lifetime).
    pub fn new(
        cfg: &Config,
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
    ) -> Result<Self, SimError> {
        crate::config::validate_environment_execution(cfg)
            .map_err(|e| SimError::InvalidConfiguration(e.to_string()))?;
        let env = &cfg.environment;
        let cue_count = env.cue_count;
        let stream = |name: &str| {
            rng_for(&SeedTuple::new(
                root_seed,
                namespace,
                outer_seed,
                lifetime_index,
                name,
            ))
            .map_err(|e| SimError::InvalidConfiguration(format!("bad seed tuple: {e}")))
        };
        let mut init_rng = stream("init")?;
        let mut mapping_init_rng = stream("mapping_init")?;
        let change_rng = stream("mapping_change")?;
        let cue_rng = stream("cue_order")?;
        let timing_rng = stream("timing")?;
        let noise_rng = stream("reward_noise")?;

        // Stable/volatile membership: shuffle cue ids on the init stream.
        let n_stable = ((cue_count as f64 * env.stable_fraction).round() as usize).min(cue_count);
        let mut membership: Vec<usize> = (0..cue_count).collect();
        membership.shuffle(&mut init_rng);
        let hidden = HiddenState::at_birth(
            cue_count,
            &membership,
            n_stable,
            &env.feedback_noise_values,
            &env.volatile_hazard_values,
            &mut mapping_init_rng,
        )?;

        let mut lifetime = Self {
            cue_count,
            cue_ticks: env.cue_ticks,
            response_ticks: env.response_ticks,
            quiet_range: env.quiet_ticks,
            gap_range: env.memory_gap_ticks,
            delay_range: env.reward_delay_ticks,
            outcomes_target: cfg.simulation.outcomes_per_lifetime,
            hidden,
            cue_rng,
            timing_rng,
            noise_rng,
            change_rng,
            tick: 0,
            phase: PhaseState::Quiet { remaining: 1 },
            current_cue: None,
            cycle_gap: 0,
            cycle_exposure_index: 0,
            cycle_changed: false,
            pending: None,
            next_event_id: 0,
            commitments: 0,
            outcomes: 0,
            consumed: Vec::new(),
            confirmed: Vec::new(),
            last_action: None,
        };
        // First quiet is the configured warmup; sample the first cue and gap
        // in the same fixed draw order as every later cycle. A zero-length
        // warmup skips straight to the first cue presentation.
        let first_cue = sample_cue(&mut lifetime.cue_rng, cue_count)?;
        let first_gap = sample_len(&mut lifetime.timing_rng, env.memory_gap_ticks, "memory_gap")?;
        lifetime.current_cue = Some(first_cue);
        lifetime.cycle_gap = first_gap;
        let warmup = cfg.simulation.warmup_ticks;
        lifetime.phase = if warmup == 0 {
            lifetime.enter_cue()?
        } else {
            PhaseState::Quiet { remaining: warmup }
        };
        Ok(lifetime)
    }

    // -- read-only accessors (evaluator/driver) ---------------------------

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn phase(&self) -> Phase {
        self.phase.snapshot()
    }

    pub fn cue_count(&self) -> usize {
        self.cue_count
    }

    /// Evaluator-only hidden state. Agent code paths must never call this.
    pub fn hidden(&self) -> &HiddenState {
        &self.hidden
    }

    pub fn pending(&self) -> Option<&PendingReward> {
        self.pending.as_ref()
    }

    pub fn consumed_ids(&self) -> &[u64] {
        &self.consumed
    }

    pub fn commitments(&self) -> u64 {
        self.commitments
    }

    pub fn outcomes(&self) -> u64 {
        self.outcomes
    }

    pub fn last_action(&self) -> Option<u8> {
        self.last_action
    }

    /// Complete when every committed choice has exactly one delivered
    /// outcome and nothing is pending.
    pub fn is_complete(&self) -> bool {
        self.pending.is_none() && self.outcomes == self.outcomes_target
    }

    // -- tick driver --------------------------------------------------------

    /// Deliver one tick: build the observation (attaching any due feedback
    /// exactly once), then advance the phase machine. Commitment, when due,
    /// is made via [`commit`](Self::commit) before the next `advance`.
    pub fn advance(&mut self) -> Result<TickOutput, SimError> {
        match &self.phase {
            PhaseState::Done => {
                return Err(SimError::LifetimeComplete {
                    outcomes: self.outcomes,
                });
            }
            PhaseState::Committed => {
                return Err(SimError::MissingCommitment(self.tick));
            }
            _ => {}
        }
        let tick = self.tick;
        let (cue, go, commitment_due) = match &self.phase {
            PhaseState::Cue { cue, .. } => (Some(*cue), false, false),
            PhaseState::Response { remaining } => (None, true, *remaining == 1),
            _ => (None, false, false),
        };

        let (feedback, annotation) = if matches!(self.phase, PhaseState::Feedback) {
            let pending = self.pending.take().ok_or_else(|| {
                SimError::InconsistentCounts(format!("feedback tick {tick} with no pending reward"))
            })?;
            let feedback = Feedback {
                event_id: pending.event_id,
                reward: pending.reward,
            };
            let annotation = HiddenAnnotation {
                event_id: pending.event_id,
                choice_index: pending.choice_index,
                cue_id: pending.cue,
                target_at_commit: pending.target_at_commit,
                latent_correctness: pending.correct,
                noise_bit: pending.noise_bit,
                epsilon: pending.epsilon,
                cue_hazard: pending.cue_hazard,
                cue_exposure_index: pending.cue_exposure_index,
                hidden_change_before_presentation: pending.hidden_change_before_presentation,
                stable_or_volatile: pending.role,
                commit_tick: pending.commit_tick,
                outcome_tick: tick,
            };
            self.outcomes += 1;
            self.consumed.push(pending.event_id);
            (Some(feedback), Some(annotation))
        } else {
            (None, None)
        };

        // Observable features, K + 6 channels (spec 5.5, M0-09): K one-hot
        // cue channels (cue ticks only), cue-present, go (response ticks),
        // outcome-present + outcome-value (feedback tick only, so a zero
        // reward stays distinguishable from no outcome), and the
        // previous-action latch (the committed action, visible starting on
        // the tick after commitment: this output is built before any commit
        // for the current tick, so `last_action` always lags correctly).
        // Sensory phase flags only — never evaluator annotations.
        let k = self.cue_count;
        let mut features = vec![0.0; feature_dim(k)];
        if let Some(cue) = cue {
            features[cue] = 1.0;
            features[k] = 1.0;
        }
        if go {
            features[k + 1] = 1.0;
        }
        if let Some(fb) = feedback {
            features[k + 2] = 1.0;
            features[k + 3] = fb.reward;
        }
        if let Some(action) = self.last_action {
            features[k + 4 + usize::from(action)] = 1.0;
        }
        debug_assert!(features.iter().all(|v| v.is_finite()));
        let observation = Observation::new(features, feedback);
        let output = TickOutput {
            tick,
            phase: self.phase.snapshot(),
            cue,
            go,
            commitment_due,
            observation,
            annotation,
        };
        self.finish_tick()?;
        Ok(output)
    }

    fn finish_tick(&mut self) -> Result<(), SimError> {
        let next = match self.phase.clone() {
            PhaseState::Quiet { remaining } => {
                if remaining > 1 {
                    PhaseState::Quiet {
                        remaining: remaining - 1,
                    }
                } else {
                    self.enter_cue()?
                }
            }
            PhaseState::Cue { remaining, .. } => {
                if remaining > 1 {
                    PhaseState::Cue {
                        cue: self.current_cue.ok_or_else(|| {
                            SimError::InconsistentCounts("cue tick with no current cue".to_owned())
                        })?,
                        remaining: remaining - 1,
                    }
                } else if self.cycle_gap > 0 {
                    PhaseState::Gap {
                        remaining: self.cycle_gap,
                    }
                } else {
                    PhaseState::Response {
                        remaining: self.response_ticks,
                    }
                }
            }
            PhaseState::Gap { remaining } => {
                if remaining > 1 {
                    PhaseState::Gap {
                        remaining: remaining - 1,
                    }
                } else {
                    PhaseState::Response {
                        remaining: self.response_ticks,
                    }
                }
            }
            PhaseState::Response { remaining } => {
                if remaining > 1 {
                    PhaseState::Response {
                        remaining: remaining - 1,
                    }
                } else {
                    PhaseState::Committed
                }
            }
            PhaseState::Delay { remaining } => {
                if remaining > 1 {
                    PhaseState::Delay {
                        remaining: remaining - 1,
                    }
                } else {
                    PhaseState::Feedback
                }
            }
            PhaseState::Feedback => {
                if self.outcomes >= self.outcomes_target {
                    PhaseState::Done
                } else {
                    self.enter_quiet()?
                }
            }
            PhaseState::Committed | PhaseState::Done => {
                return Err(SimError::InconsistentCounts(
                    "finish_tick reached a non-steppable phase".to_owned(),
                ));
            }
        };
        self.phase = next;
        self.tick += 1;
        Ok(())
    }

    fn enter_quiet(&mut self) -> Result<PhaseState, SimError> {
        let quiet_len = sample_len(&mut self.timing_rng, self.quiet_range, "quiet")?;
        let next_cue = sample_cue(&mut self.cue_rng, self.cue_count)?;
        let gap = sample_len(&mut self.timing_rng, self.gap_range, "memory_gap")?;
        self.current_cue = Some(next_cue);
        self.cycle_gap = gap;
        // A zero-length quiet goes straight to the cue presentation: every
        // interval lasts exactly its declared number of ticks.
        if quiet_len == 0 {
            self.enter_cue()
        } else {
            Ok(PhaseState::Quiet {
                remaining: quiet_len,
            })
        }
    }

    fn enter_cue(&mut self) -> Result<PhaseState, SimError> {
        let cue = self.current_cue.ok_or_else(|| {
            SimError::InconsistentCounts("entering cue phase with no current cue".to_owned())
        })?;
        let (exposure_index, changed) = self.hidden.present(cue, &mut self.change_rng)?;
        self.cycle_exposure_index = exposure_index;
        self.cycle_changed = changed;
        Ok(PhaseState::Cue {
            cue,
            remaining: self.cue_ticks,
        })
    }

    // -- commitments and pending rewards (M0-08) ----------------------------

    /// Commit an action at the final response tick. Stores the action and
    /// the then-current mapping, samples/stores the reward noise once, and
    /// schedules exactly one feedback at the start of
    /// `commit_tick + delay`. Rejects a second commitment while one is
    /// unresolved (the phase machine only admits `Committed` with nothing
    /// pending) and any action outside {0, 1}.
    pub fn commit(&mut self, action: u8) -> Result<CommitReceipt, SimError> {
        self.validate_commit(action)?;
        let epsilon = self.epsilon_for_current_cue()?;
        let noise_bit = Bernoulli::new(epsilon)
            .map(|d| d.sample(&mut self.noise_rng))
            .map_err(|_| SimError::InvalidConfiguration(format!("invalid noise rate {epsilon}")))?;
        self.commit_with_noise(action, noise_bit)
    }

    /// Diagnostic forcing path for fixtures (M0-11): identical to
    /// [`commit`](Self::commit) except the noise bit is supplied instead of
    /// sampled. Production lifetimes must use `commit`.
    pub fn commit_with_noise(
        &mut self,
        action: u8,
        noise_bit: bool,
    ) -> Result<CommitReceipt, SimError> {
        self.validate_commit(action)?;
        self.store_commit(action, noise_bit)
    }

    /// Check all caller errors before consuming any simulation randomness.
    fn validate_commit(&self, action: u8) -> Result<(), SimError> {
        if !matches!(self.phase, PhaseState::Committed) {
            return Err(SimError::CommitOutOfPhase {
                tick: self.tick,
                phase: self.phase.snapshot().name(),
            });
        }
        if action > 1 {
            return Err(SimError::InvalidAction(action));
        }
        if self.pending.is_some() {
            return Err(SimError::InconsistentCounts(
                "commit with an unresolved pending reward".to_owned(),
            ));
        }
        Ok(())
    }

    fn store_commit(&mut self, action: u8, noise_bit: bool) -> Result<CommitReceipt, SimError> {
        let cue = self
            .current_cue
            .ok_or_else(|| SimError::InconsistentCounts("commit with no current cue".to_owned()))?;
        let commit_tick = self.tick - 1;
        let target = self.hidden.mapping(cue);
        let correct = action == target;
        // Reward uses the mapping saved at commitment and one stored noise
        // bit (spec 5.1); later hazard flips cannot alter it.
        let reward = f64::from(correct ^ noise_bit);
        let delay = sample_len(&mut self.timing_rng, self.delay_range, "reward_delay")?;
        if delay < 1 {
            return Err(SimError::InconsistentCounts(format!(
                "reward delay {delay} below one tick"
            )));
        }
        let event_id = self.next_event_id;
        self.next_event_id += 1;
        let choice_index = self.commitments;
        self.pending = Some(PendingReward {
            event_id,
            choice_index,
            cue,
            action,
            target_at_commit: target,
            correct,
            noise_bit,
            epsilon: self.hidden.epsilon(cue),
            cue_hazard: self.hidden.hazard(cue),
            cue_exposure_index: self.cycle_exposure_index,
            hidden_change_before_presentation: self.cycle_changed,
            role: self.hidden.role(cue),
            reward,
            commit_tick,
            due_tick: commit_tick + delay,
        });
        self.commitments += 1;
        self.last_action = Some(action);
        self.phase = if delay == 1 {
            PhaseState::Feedback
        } else {
            PhaseState::Delay {
                remaining: delay - 1,
            }
        };
        Ok(CommitReceipt {
            event_id,
            choice_index,
        })
    }

    fn epsilon_for_current_cue(&self) -> Result<f64, SimError> {
        self.current_cue
            .map(|cue| self.hidden.epsilon(cue))
            .ok_or_else(|| SimError::InconsistentCounts("commit with no current cue".to_owned()))
    }

    /// Backstop consumption ledger for the future agent `apply_feedback`
    /// path (spec 9.2): the first confirmation of a delivered id succeeds;
    /// confirming it again fails as [`SimError::DuplicateFeedback`] without
    /// changing state; confirming an id this lifetime never delivered fails
    /// as [`SimError::UnknownFeedback`].
    pub fn note_feedback_consumed(&mut self, event_id: u64) -> Result<(), SimError> {
        if self.confirmed.contains(&event_id) {
            return Err(SimError::DuplicateFeedback(event_id));
        }
        if !self.consumed.contains(&event_id) {
            return Err(SimError::UnknownFeedback(event_id));
        }
        self.confirmed.push(event_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smoke_cfg() -> Config {
        let text = std::fs::read_to_string("configs/env_smoke.toml").expect("smoke config");
        toml::from_str(&text).expect("smoke parses")
    }

    #[test]
    fn birth_samples_mappings_and_first_quiet_is_warmup() {
        let cfg = smoke_cfg();
        let lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
        assert_eq!(lifetime.tick(), 0);
        assert_eq!(lifetime.phase(), Phase::Quiet);
        assert_eq!(lifetime.commitments(), 0);
        assert_eq!(lifetime.outcomes(), 0);
        assert!(lifetime.pending().is_none());
        assert!(lifetime.current_cue.is_some());
        // Warmup is part of the lifetime: first quiet lasts warmup_ticks.
        if let PhaseState::Quiet { remaining } = lifetime.phase {
            assert_eq!(remaining, cfg.simulation.warmup_ticks);
        } else {
            panic!("birth phase must be quiet");
        }
    }

    #[test]
    fn bad_namespace_rejected_at_birth() {
        let cfg = smoke_cfg();
        assert!(Lifetime::new(&cfg, 1, "staging", 1, 0).is_err());
    }
}
