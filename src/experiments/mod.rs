//! Experiment harnesses: baseline runs and isolated numerical diagnostics.
//!
//! Later milestones add suites, evolution evaluation, and interventions
//! here. The baseline harness covers B0/B1/O1 plus the M1 nonplastic actor
//! B3; M2-04 adds the restricted fixed-weight finite-rollout diagnostic;
//! M3-04 adds the explicitly episodic clean-learning diagnostic runner
//! (fixed-gate plastic learner with logged rollout resets, not the main
//! continuous condition).

pub mod baseline;
/// Explicitly episodic clean-learning diagnostic (M3-04); not continuous.
pub mod episodic;
/// Restricted fixed-weight score diagnostic; never a continuous agent mode.
pub mod finite_rollout;
/// Frozen pre-results development sweep declaration (M3-06); loads and
/// validates but never executes.
pub mod grid;
/// Acquisition analysis over sweep summaries (M3-07); windows, margins,
/// health, criterion, and selection. No simulation here.
pub mod sweep;
