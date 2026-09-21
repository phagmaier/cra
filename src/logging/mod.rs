//! Separated event logging: ordinary records vs evaluator annotations.
//!
//! Ordinary event data (what the agent could observe) and hidden
//! annotations (evaluator truth) serialize to separate JSONL streams,
//! joined offline within contiguous lifetime blocks by (`event_id`,
//! `choice_index`). IDs restart per lifetime; ordinary rows carry lifetime
//! identity and hidden rows follow the same block order.
//! Raw output is immutable once written; derived analysis goes elsewhere.
//! Files are written atomically (temporary file + rename).
//!
//! Schema discipline: `events.jsonl` records carry `schema_version = 1`
//! and only M0-available fields. Later-stage quantities (motor margins,
//! eligibility norms, gate statistics) arrive with their milestones under
//! new schema versions — never fabricated here (M0-14).

pub mod events;
