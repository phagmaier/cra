//! Experiment harnesses: baseline runs and isolated numerical diagnostics.
//!
//! Later milestones add suites, evolution evaluation, and interventions
//! here. The baseline harness covers B0/B1/O1 plus the M1 nonplastic actor
//! B3; M2-04 adds the restricted fixed-weight finite-rollout diagnostic;
//! M3-04 adds the explicitly episodic clean-learning diagnostic runner
//! (fixed-gate plastic learner with logged rollout resets, not the main
//! continuous condition); M3-09 adds failure-isolation diagnostics
//! (single-motor sign/order checks, receiver-permutation sensitivity).

pub mod baseline;
/// Fully persistent plastic learner plus its reset-audited lifetime runner
/// for the main continuous condition (M4-02/M4-03; spec 7.3-7.5, 7.7-7.8,
/// 9-10): fixed-gate updates on decaying traces that are never reset at
/// choice/feedback boundaries. Profiles and checkpoints arrive in M4-04
/// through M4-06.
pub mod continuous;
/// Explicitly episodic clean-learning diagnostic (M3-04); not continuous.
pub mod episodic;
/// Restricted fixed-weight score diagnostic; never a continuous agent mode.
pub mod finite_rollout;
/// Frozen pre-results development sweep declaration (M3-06); loads and
/// validates but never executes.
pub mod grid;
/// Failure-isolation diagnostics for the episodic learner (M3-09):
/// single-motor sign/order checks and the receiver-permutation
/// sensitivity probe. Diagnostic-only; never a task result.
pub mod reduction;
/// Acquisition analysis over sweep summaries (M3-07); windows, margins,
/// health, criterion, and selection. No simulation here.
pub mod sweep;
