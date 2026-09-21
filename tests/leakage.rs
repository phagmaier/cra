//! Information-boundary tests (M0-06; spec 5.6, AGENTS.md).
//!
//! The ordinary agent sees only `Observation.features`, observed reward
//! feedback, and its own state. These tests pin that contract: ordinary
//! types serialize with exactly their public fields, hidden annotations
//! serialize to a separate stream, feedback event ids never enter the
//! feature vector, and a recording `Agent` driven on real observations
//! receives nothing but ordinary data.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::environment::observation::{Agent, Feedback, MotorOutput, Observation, SimError};
use support::base_config;

const HIDDEN_MARKERS: [&str; 8] = [
    "target_at_commit",
    "latent_correctness",
    "noise_bit",
    "cue_hazard",
    "stable_or_volatile",
    "hidden_change_before_presentation",
    "epsilon",
    "cue_exposure_index",
];

#[test]
fn ordinary_types_serialize_with_exactly_their_public_fields() {
    let obs = Observation::new(
        vec![0.0; 8],
        Some(Feedback {
            event_id: 3,
            reward: 0.0,
        }),
    );
    let value = serde_json::to_value(&obs).expect("serializes");
    let keys: Vec<&str> = value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, vec!["features", "feedback"]);
    assert_eq!(
        value["feedback"]
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["event_id", "reward"]
    );

    let motor = MotorOutput {
        action_0: 0.25,
        action_1: -0.5,
    };
    let motor_value = serde_json::to_value(motor).expect("serializes");
    let motor_keys: Vec<&str> = motor_value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(motor_keys, vec!["action_0", "action_1"]);

    // No hidden marker may appear anywhere in the ordinary stream.
    let ordinary_json = serde_json::to_string(&obs).expect("serializes");
    for marker in HIDDEN_MARKERS {
        assert!(
            !ordinary_json.contains(marker),
            "ordinary stream leaks {marker}"
        );
    }
}

#[test]
fn feedback_ids_never_enter_the_feature_vector() {
    let cfg = base_config();
    cra::config::validate(&cfg).expect("valid");
    let mut lt = cra::environment::Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut seen_ids = Vec::new();
    while !lt.is_complete() {
        let out = lt.advance().expect("advance");
        // Exactly K + 6 channels on every tick, all finite.
        assert_eq!(out.observation.features.len(), 8);
        assert!(out.observation.features.iter().all(|v| v.is_finite()));
        if let Some(feedback) = out.observation.feedback {
            seen_ids.push(feedback.event_id);
        }
        if out.commitment_due {
            lt.commit(0).expect("commit");
        }
    }
    assert_eq!(seen_ids, vec![0, 1, 2, 3, 4, 5]);
    // Features are built without access to the event id: the id travels
    // only in the feedback envelope (structural: width is exactly K + 6,
    // and no id-sized side channel exists in the vector).
    assert_eq!(lt.consumed_ids(), seen_ids.as_slice());
}

#[test]
fn hidden_annotations_serialize_to_a_separate_stream() {
    let cfg = base_config();
    cra::config::validate(&cfg).expect("valid");
    let mut lt = cra::environment::Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut ordinary_records = Vec::new();
    let mut hidden_records = Vec::new();
    while !lt.is_complete() {
        let out = lt.advance().expect("advance");
        ordinary_records.push(serde_json::to_value(&out.observation).expect("serializes"));
        if let Some(annotation) = out.annotation {
            hidden_records.push(serde_json::to_value(&annotation).expect("serializes"));
        }
        if out.commitment_due {
            lt.commit(1).expect("commit");
        }
    }
    assert_eq!(hidden_records.len(), 6);
    // Exact tick accounting for the fixed test config: warmup 4, then per
    // cycle quiet 4 + cue 8 + gap 0 + response 4 + 0 ordinary delay ticks
    // (delay [1,1]) + 1 feedback tick = 17 ticks x 6 outcomes = 102 ticks.
    let env = &cfg.environment;
    let per_cycle = env.quiet_ticks[0]
        + env.cue_ticks
        + env.memory_gap_ticks[0]
        + env.response_ticks
        + (env.reward_delay_ticks[0] - 1)
        + env.feedback_ticks;
    assert_eq!(ordinary_records.len(), 6 * per_cycle as usize);
    // Every ordinary record is clean; every hidden record carries truth.
    for record in &ordinary_records {
        let json = serde_json::to_string(record).expect("serializes");
        for marker in HIDDEN_MARKERS {
            assert!(!json.contains(marker), "ordinary record leaks {marker}");
        }
    }
    assert_eq!(hidden_records.len(), 6);
    for record in &hidden_records {
        assert!(record.get("target_at_commit").is_some());
        assert!(record.get("features").is_none(), "streams stay separate");
        // event_id is the documented offline join key, present in both.
        assert!(record.get("event_id").is_some());
    }
}

/// A stub implementing the ordinary `Agent` interface. It can only receive
/// what the environment hands to `advance`/`apply_feedback`.
struct RecordingAgent {
    feature_snapshots: Vec<Vec<f64>>,
    feedback_ids: Vec<u64>,
}

impl RecordingAgent {
    fn new() -> Self {
        Self {
            feature_snapshots: Vec::new(),
            feedback_ids: Vec::new(),
        }
    }
}

impl Agent for RecordingAgent {
    fn apply_feedback(&mut self, event: Feedback) -> Result<(), SimError> {
        self.feedback_ids.push(event.event_id);
        Ok(())
    }

    fn advance(&mut self, features: &[f64]) -> Result<MotorOutput, SimError> {
        self.feature_snapshots.push(features.to_vec());
        Ok(MotorOutput {
            action_0: 0.0,
            action_1: 0.0,
        })
    }
}

#[test]
fn agent_trait_path_carries_only_ordinary_data() {
    let cfg = base_config();
    cra::config::validate(&cfg).expect("valid");
    let mut lt = cra::environment::Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut agent = RecordingAgent::new();
    let mut ticks = 0;
    while !lt.is_complete() {
        let out = lt.advance().expect("advance");
        if let Some(feedback) = out.observation.feedback {
            agent.apply_feedback(feedback).expect("agent feedback");
        }
        agent
            .advance(&out.observation.features)
            .expect("agent step");
        if out.commitment_due {
            lt.commit(0).expect("commit");
        }
        ticks += 1;
    }
    assert_eq!(agent.feature_snapshots.len(), ticks);
    assert_eq!(agent.feedback_ids, vec![0, 1, 2, 3, 4, 5]);
    // Everything the agent ever received, serialized: no hidden markers.
    let seen = serde_json::to_string(&serde_json::json!({
        "features": agent.feature_snapshots,
        "feedback_ids": agent.feedback_ids,
    }))
    .expect("serializes");
    for marker in HIDDEN_MARKERS {
        assert!(!seen.contains(marker), "agent path leaks {marker}");
    }
}

#[test]
fn pinned_fixture_ordinary_log_matches_expected_public_values() {
    // Same pinned fixture as tests/event_order.rs (gap [5,5], delay [3,3],
    // mappings [1, 1], first cue 1): the ordinary JSON log for the feedback
    // tick carries exactly the public values and no hidden truth.
    let mut cfg = base_config();
    cfg.environment.memory_gap_ticks = [5, 5];
    cfg.environment.reward_delay_ticks = [3, 3];
    cfg.simulation.outcomes_per_lifetime = 1;
    cra::config::validate(&cfg).expect("valid");
    let mut lt = cra::environment::Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let mut ordinary_log = Vec::new();
    while !lt.is_complete() {
        let out = lt.advance().expect("advance");
        ordinary_log.push(serde_json::to_value(&out.observation).expect("serializes"));
        if out.commitment_due {
            lt.commit(1).expect("commit correct");
        }
    }
    // quiet 4 + cue 8 + gap 5 + response 4 + 2 ordinary delay ticks +
    // 1 feedback tick = 24 ticks; feedback is index 23.
    assert_eq!(ordinary_log.len(), 24);
    let feedback_record = &ordinary_log[23];
    assert_eq!(
        feedback_record["features"],
        serde_json::json!([0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0])
    );
    assert_eq!(
        feedback_record["feedback"],
        serde_json::json!({"event_id": 0, "reward": 1.0})
    );
    for record in &ordinary_log {
        let json = serde_json::to_string(record).expect("serializes");
        for marker in HIDDEN_MARKERS {
            assert!(!json.contains(marker), "pinned log leaks {marker}");
        }
    }
}
