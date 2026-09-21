//! Learning When to Learn — authoritative simulator crate.
//!
//! M0 covers the environment contracts: deterministic RNG streams ([`rng`]),
//! versioned configuration parsing and validation ([`config`]), run
//! provenance and baseline execution ([`run`]), and the continuous environment itself
//! ([`environment`]: observation boundary, hidden state, phase scheduling,
//! commitments, pending rewards). Separated event logging and the offline
//! Python log audit are implemented. Neural dynamics, plasticity, search,
//! and comparative analysis remain later milestones.

pub mod config;
pub mod environment;
pub mod experiments;
pub mod logging;
pub mod rng;
pub mod run;
