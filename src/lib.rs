//! Learning When to Learn — authoritative simulator crate.
//!
//! M0 covers the environment contracts: deterministic RNG streams ([`rng`]),
//! versioned configuration parsing and validation ([`config`]), run
//! provenance and baseline execution ([`run`]), and the continuous environment itself
//! ([`environment`]: observation boundary, hidden state, phase scheduling,
//! commitments, pending rewards). Separated event logging and the offline
//! Python log audit are implemented. M1-01 adds inherited recurrent topology
//! ([`agent::topology`]: Bernoulli mask, fixed motor pools, stable edge
//! order, structural validation), M1-02 inherited weights
//! ([`agent::weights`]: row-scaled `W0`, dense `B`, zero biases), M1-03
//! the double-buffered actor transition ([`agent::actor`]), M1-06 the
//! fixed motor readout ([`agent::motor`]), M1-07 the nonplastic
//! continuous actor ([`agent::no_learning`]: B3 through the common ordinary
//! runner), and M1-08 read-only numerical health plus selected traces
//! ([`agent::health`]: conservative watchdog, summaries, stable sampling).
//! M1-09 adds exact lifetime pause/resume ([`checkpoint`]: versioned file,
//! config hash, checksum, atomic writes). M3-01 adds plastic offsets,
//! eligibility traces, plastic masks, and the single effective-weight
//! refresh ([`agent::plasticity`]: `P`/`E` stored separately from immutable
//! `W0`, `persistent` versus `no_decay_diagnostic` traces, versioned
//! snapshots).
//! Feedback-gated updates, gating, search, and
//! comparative analysis remain later milestones.

pub mod agent;
pub mod checkpoint;
pub mod config;
pub mod environment;
pub mod experiments;
pub mod logging;
pub mod rng;
pub mod run;
