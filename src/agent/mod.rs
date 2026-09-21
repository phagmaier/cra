//! Inherited agent parameters and actor dynamics (M1-01 topology, M1-02
//! weights, M1-03 transition, M1-06 motor readout, M1-07 nonplastic runner,
//! M1-08 numerical health).
//!
//! The recurrent mask, motor assignment, and structural checks live in
//! [`topology`]; inherited `W0`, sensory projection `B`, and zero biases
//! live in [`weights`]; the double-buffered `f64` transition lives in
//! [`actor`]; fixed pool means, leaky filtering, and commitment live in
//! [`motor`]; the continuously running no-learning actor (B3) lives in
//! [`no_learning`]; read-only watchdog/summary/selected-trace observation
//! lives in [`health`]; the pure conditional score (M2-01) lives in
//! [`score`]; plastic offsets/eligibility/masks and the single
//! effective-weight refresh (M3-01) live in [`plasticity`]. Gates and
//! search arrive in later milestones; this module must not grow that code
//! ahead of them.

pub mod actor;
pub mod health;
pub mod motor;
pub mod no_learning;
pub mod plasticity;
pub mod score;
pub mod topology;
pub mod weights;
