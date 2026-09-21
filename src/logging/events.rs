//! Ordinary event records, condition/completion provenance, and stream
//! validation (M0-13).
//!
//! `OrdinaryEvent` holds one delivered outcome with only agent-observable
//! content (the shown cue index is public one-hot input, not hidden truth).
//! [`crate::environment::HiddenAnnotation`] is the separate hidden stream.
//! `validate_stream` enforces the same accounting the offline audit checks:
//! contiguous choices, unique increasing event ids, ordered ticks, finite
//! 0/1 rewards, and an exact hidden join — as explicit errors, never
//! successful-looking output.

use std::path::Path;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::environment::HiddenAnnotation;

/// Schema version for M0 event records. Bumped only with a documented
/// record-format change.
pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// One delivered outcome, ordinary (agent-observable) content only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrdinaryEvent {
    pub schema_version: u32,
    pub run_id: String,
    pub condition_id: String,
    pub namespace: String,
    pub outer_seed: u64,
    pub lifetime_index: u64,
    pub choice_index: u64,
    pub event_id: u64,
    /// Shown cue position (public one-hot input). The preferred action,
    /// correctness, and noise/hazard labels are hidden-stream only.
    pub cue_index: usize,
    pub commit_tick: u64,
    pub outcome_tick: u64,
    pub action: u8,
    pub reward: f64,
}

/// Condition identity: which rung ran under which profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditionInfo {
    pub condition_id: String,
    pub policy: String,
    pub profile_name: String,
    pub lifetimes: u64,
    pub outcomes_per_lifetime: u64,
    pub event_log: bool,
}

/// Terminal run state. Missing or non-completed records fail the audit;
/// they are never treated as successful zero-score runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompletionInfo {
    pub run_id: String,
    #[serde(flatten)]
    pub status: RunStatus,
    pub lifetimes_completed: u64,
    pub commitments: u64,
    pub outcomes: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RunStatus {
    Completed,
    Interrupted { reason: String },
}

/// Explicit log failures.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum LogError {
    #[error("log I/O error for '{path}': {message}")]
    Io { path: String, message: String },
    #[error("cannot parse {path} line {line}: {message}")]
    Parse {
        path: String,
        line: usize,
        message: String,
    },
    #[error("unsupported event schema_version {found}; this audit accepts {EVENT_SCHEMA_VERSION}")]
    SchemaVersion { found: u32 },
    #[error("choice_index not contiguous at position {position}: found {found}")]
    NonContiguousChoice { position: usize, found: u64 },
    #[error("duplicate event_id {0}")]
    DuplicateEventId(u64),
    #[error("event ids not strictly increasing at position {position}")]
    UnorderedEventId { position: usize },
    #[error("outcome ticks not strictly increasing at position {position}")]
    UnorderedTicks { position: usize },
    #[error("invalid field in event {event_id}: {reason}")]
    InvalidField { event_id: u64, reason: String },
    #[error("hidden stream mismatch: {reason}")]
    HiddenMismatch { reason: String },
    #[error("missing required file '{0}'")]
    MissingFile(String),
}

/// Field-level validation: finite 0/1 reward, binary action, in-range cue,
/// commit strictly before outcome. Cross-record order is checked by
/// [`validate_stream`].
pub fn validate_event(event: &OrdinaryEvent, cue_count: usize) -> Result<(), LogError> {
    if event.schema_version != EVENT_SCHEMA_VERSION {
        return Err(LogError::SchemaVersion {
            found: event.schema_version,
        });
    }
    let bad = |reason: &str| LogError::InvalidField {
        event_id: event.event_id,
        reason: reason.to_owned(),
    };
    if !event.reward.is_finite() || (event.reward != 0.0 && event.reward != 1.0) {
        return Err(bad(&format!(
            "reward must be 0 or 1; found {}",
            event.reward
        )));
    }
    if event.action > 1 {
        return Err(bad(&format!(
            "action must be 0 or 1; found {}",
            event.action
        )));
    }
    if event.cue_index >= cue_count {
        return Err(bad(&format!(
            "cue_index {} out of range for {cue_count} cues",
            event.cue_index
        )));
    }
    if event.commit_tick >= event.outcome_tick {
        return Err(bad(&format!(
            "commit_tick {} must precede outcome_tick {}",
            event.commit_tick, event.outcome_tick
        )));
    }
    Ok(())
}

/// Stream-level validation: contiguous choices from 0, unique strictly
/// increasing event ids, strictly increasing outcome ticks, and an exact
/// hidden-stream join on (`event_id`, `choice_index`).
pub fn validate_stream(
    events: &[OrdinaryEvent],
    hidden: &[HiddenAnnotation],
    cue_count: usize,
) -> Result<(), LogError> {
    for (position, event) in events.iter().enumerate() {
        validate_event(event, cue_count)?;
        if event.choice_index != position as u64 {
            return Err(LogError::NonContiguousChoice {
                position,
                found: event.choice_index,
            });
        }
        if position > 0 {
            let prev = &events[position - 1];
            if event.event_id == prev.event_id {
                return Err(LogError::DuplicateEventId(event.event_id));
            }
            if event.event_id < prev.event_id {
                return Err(LogError::UnorderedEventId { position });
            }
            if event.outcome_tick <= prev.outcome_tick {
                return Err(LogError::UnorderedTicks { position });
            }
        }
    }
    if hidden.len() != events.len() {
        return Err(LogError::HiddenMismatch {
            reason: format!(
                "{} hidden records for {} events",
                hidden.len(),
                events.len()
            ),
        });
    }
    for (event, annotation) in events.iter().zip(hidden.iter()) {
        if annotation.event_id != event.event_id || annotation.choice_index != event.choice_index {
            return Err(LogError::HiddenMismatch {
                reason: format!(
                    "join mismatch at choice {}: event ({}, {}) vs hidden ({}, {})",
                    event.choice_index,
                    event.event_id,
                    event.choice_index,
                    annotation.event_id,
                    annotation.choice_index
                ),
            });
        }
    }
    Ok(())
}

/// Write JSONL atomically (temporary file + rename) so partial files are
/// never mistaken for complete logs.
pub fn write_jsonl<T: Serialize>(dir: &Path, name: &str, records: &[T]) -> Result<(), LogError> {
    let path = dir.join(name);
    let tmp = dir.join(format!("{name}.tmp"));
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record).map_err(|e| LogError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?);
        text.push('\n');
    }
    std::fs::write(&tmp, text).map_err(|e| LogError::Io {
        path: tmp.display().to_string(),
        message: e.to_string(),
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| LogError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    Ok(())
}

/// Write any serializable provenance record atomically.
pub fn write_json<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<(), LogError> {
    let path = dir.join(name);
    let tmp = dir.join(format!("{name}.tmp"));
    let text = serde_json::to_string_pretty(value).map_err(|e| LogError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    std::fs::write(&tmp, text).map_err(|e| LogError::Io {
        path: tmp.display().to_string(),
        message: e.to_string(),
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| LogError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    Ok(())
}

/// Read back one JSONL stream, reporting the offending line on failure.
pub fn read_jsonl<T: DeserializeOwned>(dir: &Path, name: &str) -> Result<Vec<T>, LogError> {
    let path = dir.join(name);
    let text =
        std::fs::read_to_string(&path).map_err(|_| LogError::MissingFile(name.to_owned()))?;
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        records.push(serde_json::from_str(line).map_err(|e| LogError::Parse {
            path: name.to_owned(),
            line: index + 1,
            message: e.to_string(),
        })?);
    }
    Ok(records)
}

/// Read back one JSON provenance record.
pub fn read_json<T: DeserializeOwned>(dir: &Path, name: &str) -> Result<T, LogError> {
    let path = dir.join(name);
    let text =
        std::fs::read_to_string(&path).map_err(|_| LogError::MissingFile(name.to_owned()))?;
    serde_json::from_str(&text).map_err(|e| LogError::Parse {
        path: name.to_owned(),
        line: 0,
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: u64) -> OrdinaryEvent {
        OrdinaryEvent {
            schema_version: EVENT_SCHEMA_VERSION,
            run_id: "r".to_owned(),
            condition_id: "B0".to_owned(),
            namespace: "development".to_owned(),
            outer_seed: 1,
            lifetime_index: 0,
            choice_index: id,
            event_id: id,
            cue_index: 0,
            commit_tick: 10 * id,
            outcome_tick: 10 * id + 1,
            action: 0,
            reward: 1.0,
        }
    }

    fn annotation(id: u64) -> HiddenAnnotation {
        HiddenAnnotation {
            event_id: id,
            choice_index: id,
            cue_id: 0,
            target_at_commit: 0,
            latent_correctness: true,
            noise_bit: false,
            epsilon: 0.0,
            cue_hazard: 0.0,
            cue_exposure_index: id + 1,
            hidden_change_before_presentation: false,
            stable_or_volatile: crate::environment::CueRole::Stable,
            commit_tick: 10 * id,
            outcome_tick: 10 * id + 1,
        }
    }

    #[test]
    fn valid_stream_passes() {
        let events: Vec<_> = (0..3).map(event).collect();
        let hidden: Vec<_> = (0..3).map(annotation).collect();
        validate_stream(&events, &hidden, 2).expect("valid");
    }

    #[test]
    fn field_and_order_violations_are_explicit() {
        let mut bad_reward = event(0);
        bad_reward.reward = f64::NAN;
        assert!(validate_event(&bad_reward, 2).is_err());

        let mut bad_action = event(0);
        bad_action.action = 2;
        assert!(validate_event(&bad_action, 2).is_err());

        let mut bad_schema = event(0);
        bad_schema.schema_version = 99;
        assert!(validate_event(&bad_schema, 2).is_err());

        let events = vec![event(0), event(0)];
        let hidden = vec![annotation(0), annotation(0)];
        assert!(matches!(
            validate_stream(&events, &hidden, 2),
            Err(LogError::NonContiguousChoice { .. }) | Err(LogError::DuplicateEventId(_))
        ));

        let mut swapped = vec![event(0), event(1)];
        swapped[1].event_id = 0;
        swapped[1].choice_index = 1;
        let hidden = vec![annotation(0), annotation(1)];
        assert!(validate_stream(&swapped, &hidden, 2).is_err());

        let events = vec![event(0)];
        let hidden: Vec<HiddenAnnotation> = vec![];
        assert!(matches!(
            validate_stream(&events, &hidden, 2),
            Err(LogError::HiddenMismatch { .. })
        ));
    }
}
