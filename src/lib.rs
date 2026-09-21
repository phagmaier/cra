//! Learning When to Learn — authoritative simulator crate (M0 scaffold).
//!
//! M0 covers contracts only: deterministic RNG streams ([`rng`]), versioned
//! configuration parsing and validation ([`config`]), and run provenance
//! scaffolding ([`run`]). Neural dynamics, plasticity, search, and the full
//! environment state machine arrive in later milestones and must not be
//! pretended to exist here.

pub mod config;
pub mod rng;
pub mod run;
