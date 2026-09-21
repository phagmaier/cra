//! Separated event logging: ordinary records vs evaluator annotations.
//!
//! Ordinary event data (what the agent could observe) and hidden
//! annotations (evaluator truth) serialize to separate JSONL streams,
//! joined offline only by explicit keys (`event_id`, `choice_index`).
//! Raw output is immutable once written; derived analysis goes elsewhere.
//! Files are written atomically (temporary file + rename).
//!
//! Schema discipline: `events.jsonl` records carry `schema_version = 1`
//! and only M0-available fields. Later-stage quantities (motor margins,
//! eligibility norms, gate statistics) arrive with their milestones under
//! new schema versions — never fabricated here (M0-14).

pub mod events;
