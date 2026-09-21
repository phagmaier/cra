//! Experiment harnesses: baseline runs and isolated numerical diagnostics.
//!
//! Later milestones add suites, evolution evaluation, and interventions
//! here. The baseline harness covers B0/B1/O1 plus the M1 nonplastic actor
//! B3; M2-04 adds the restricted fixed-weight finite-rollout diagnostic.

pub mod baseline;
/// Restricted fixed-weight score diagnostic; never a continuous agent mode.
pub mod finite_rollout;
