//! Public agent boundary: `Feedback`, `Observation`, `MotorOutput`,
//! `SimError`, and the `Agent` trait (M0-06, spec 18.3).
//!
//! Everything in this module is ordinary (agent-visible) data. Hidden truth
//! — preferred actions, correctness, noise bits/rates, hazards,
//! stable/volatile membership, change flags, future schedules, lifetime
//! countdowns, split identity — lives in [`super::hidden_state`] and never
//! appears here. `Feedback.event_id` is deduplication infrastructure, never
//! a neural feature: feature vectors always have exactly `K + 6` channels
//! (spec 5.5) and are built without access to the event id.

use serde::{Deserialize, Serialize};

/// Observed outcome delivered on exactly one feedback tick.
///
/// `event_id` is infrastructure for exactly-once delivery. It is not a
/// sensory feature and must never enter the feature vector.
/// `reward` is the observed scalar outcome (0 or 1), not latent correctness.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feedback {
    pub event_id: u64,
    pub reward: f64,
}

/// One tick of agent-visible input.
///
/// `features` has exactly [`feature_dim`] entries: `K` one-hot cue channels
/// plus cue-present, go, outcome-present, outcome-value, and two
/// previous-action channels. `feedback` is `Some` on exactly one tick per
/// committed choice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub features: Vec<f64>,
    pub feedback: Option<Feedback>,
}

impl Observation {
    /// Build an observation from an explicit feature vector and optional
    /// feedback. This constructor bundles the supplied values without
    /// validating them; the environment tick driver builds contractual features.
    pub fn new(features: Vec<f64>, feedback: Option<Feedback>) -> Self {
        Self { features, feedback }
    }
}

/// Number of observable feature channels for `K` one-hot cues: cue content
/// (`K`), cue-present, go, outcome-present, outcome-value, and two
/// previous-action channels (spec 5.5).
pub fn feature_dim(cue_count: usize) -> usize {
    cue_count + 6
}

/// Continuously available motor readout. The environment latches an action
/// from these values only at the final response tick (M1); the no-learning
/// baselines and oracles use the same commitment rule through this type.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotorOutput {
    pub action_0: f64,
    pub action_1: f64,
}

/// Explicit simulation errors. Nonfinite values, duplicate feedback, invalid
/// configurations, corrupt checkpoints, and inconsistent counts are errors,
/// never silent defaults or successful-looking output.
#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum SimError {
    /// Rejected configuration (message names the offending field/value).
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),
    /// A feedback event was delivered or confirmed twice; state unchanged.
    #[error("duplicate feedback event {0}")]
    DuplicateFeedback(u64),
    /// A feedback id was never delivered by this lifetime.
    #[error("unknown feedback event {0}")]
    UnknownFeedback(u64),
    /// A nonfinite value reached a simulated quantity.
    #[error("nonfinite state at tick {tick} in {component}")]
    NonFiniteState { tick: u64, component: String },
    /// A checkpoint is corrupt, partial, or incompatible (M1-09).
    #[error("inconsistent checkpoint: {0}")]
    InconsistentCheckpoint(String),
    /// `commit` was called outside the final response tick.
    #[error("commit at tick {tick} outside the final response tick (phase {phase})")]
    CommitOutOfPhase { tick: u64, phase: &'static str },
    /// `commit` was called with an action outside {0, 1}.
    #[error("invalid action {0}; the first environment supports actions 0 and 1")]
    InvalidAction(u8),
    /// The driver advanced past a commitment point without committing.
    #[error("missing commitment for the choice ending at tick {0}")]
    MissingCommitment(u64),
    /// Internal counts disagree (e.g. feedback tick with no pending reward).
    #[error("inconsistent counts: {0}")]
    InconsistentCounts(String),
    /// The lifetime already delivered all its outcomes.
    #[error("lifetime complete: {outcomes} outcomes delivered")]
    LifetimeComplete { outcomes: u64 },
}

/// Ordinary agent interface (spec 9.3). The agent sees only an `Observation`
/// per tick and returns motor output; it never receives hidden state,
/// future schedules, or evaluation labels.
pub trait Agent {
    /// Called at most once per delivered feedback event, before `advance`.
    /// A second call with the same event id must fail without changing
    /// state.
    fn apply_feedback(&mut self, event: Feedback) -> Result<(), SimError>;

    /// One neural transition on ordinary features. Must not apply feedback
    /// a second time.
    fn advance(&mut self, features: &[f64]) -> Result<MotorOutput, SimError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_dim_is_k_plus_six() {
        assert_eq!(feature_dim(2), 8);
        assert_eq!(feature_dim(8), 14);
    }

    #[test]
    fn boundary_types_round_trip_through_json() {
        let obs = Observation::new(
            vec![0.0; feature_dim(2)],
            Some(Feedback {
                event_id: 7,
                reward: 1.0,
            }),
        );
        let json = serde_json::to_string(&obs).expect("serializes");
        let back: Observation = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(obs, back);
    }
}
