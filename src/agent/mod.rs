//! Inherited agent parameters and actor dynamics (M1-01 topology, M1-02
//! weights, M1-03 transition).
//!
//! The recurrent mask, motor assignment, and structural checks live in
//! [`topology`]; inherited `W0`, sensory projection `B`, and zero biases
//! live in [`weights`]; the double-buffered `f64` transition lives in
//! [`actor`]. Plasticity, motor commitment, and search arrive in later
//! milestones; this module must not grow that code ahead of them.

pub mod actor;
pub mod topology;
pub mod weights;
