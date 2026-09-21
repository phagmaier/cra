//! Learning When to Learn — authoritative simulator crate.
//!
//! M0 covers the environment contracts: deterministic RNG streams ([`rng`]),
//! versioned configuration parsing and validation ([`config`]), run
//! provenance scaffolding ([`run`]), and the continuous environment itself
//! ([`environment`]: observation boundary, hidden state, phase scheduling,
//! commitments, pending rewards). Neural dynamics, plasticity, search, and
//! Python analysis arrive in later milestones and must not be pretended to
//! exist here.

pub mod config;
pub mod environment;
pub mod experiments;
pub mod logging;
pub mod rng;
pub mod run;
