//! Phase state machine and timing draws (M0-07).
//!
//! Cycle order per choice (spec 5.7): quiet -> cue presentation ->
//! optional blank gap -> response (`go = 1`) -> commit at the final
//! response tick -> reward delay -> exactly one feedback tick -> next quiet.
//! Every interval lasts exactly its declared number of ticks; a zero-length
//! gap skips the gap phase rather than spending a tick in it.
//!
//! Delay accounting (spec 9.4, AGENTS.md): commitment at the end of tick
//! `t_commit` with delay `d >= 1` delivers feedback at the *start* of tick
//! `t_commit + d`. The `d - 1` intermediate ticks are ordinary delay ticks;
//! for `d = 1` the next tick is the feedback tick. The delay includes the
//! feedback endpoint — no extra tick is added.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use super::observation::SimError;

/// Public phase snapshot for logs and tests. The driver enum
/// `PhaseState` additionally carries countdowns and is internal to the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Quiet,
    Cue,
    Gap,
    Response,
    Delay,
    Feedback,
    /// Final response tick has been delivered but `commit` not yet called.
    /// No tick is ever spent here; advancing without committing is an error.
    Committed,
    Done,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Cue => "cue",
            Self::Gap => "gap",
            Self::Response => "response",
            Self::Delay => "delay",
            Self::Feedback => "feedback",
            Self::Committed => "committed",
            Self::Done => "done",
        }
    }
}

/// Driver state: the current phase plus its remaining ticks (remaining is
/// always >= 1 while the phase is active). Serialized verbatim into
/// lifetime checkpoints (M1-09); countdowns are validated on restore
/// rather than trusted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum PhaseState {
    Quiet { remaining: u64 },
    Cue { cue: usize, remaining: u64 },
    Gap { remaining: u64 },
    Response { remaining: u64 },
    Committed,
    Delay { remaining: u64 },
    Feedback,
    Done,
}

impl PhaseState {
    pub(super) fn snapshot(&self) -> Phase {
        match self {
            Self::Quiet { .. } => Phase::Quiet,
            Self::Cue { .. } => Phase::Cue,
            Self::Gap { .. } => Phase::Gap,
            Self::Response { .. } => Phase::Response,
            Self::Committed => Phase::Committed,
            Self::Delay { .. } => Phase::Delay,
            Self::Feedback => Phase::Feedback,
            Self::Done => Phase::Done,
        }
    }
}

/// Sample an inclusive uniform integer in `[lo, hi]` from the timing stream.
/// Endpoints are exact: `[4, 4]` always yields 4 (table-driven tests rely on
/// this rather than on loop-bound conventions).
pub fn sample_len(
    timing_rng: &mut ChaCha8Rng,
    range: [u64; 2],
    what: &str,
) -> Result<u64, SimError> {
    let [lo, hi] = range;
    if lo > hi {
        return Err(SimError::InvalidConfiguration(format!(
            "{what} range min {lo} exceeds max {hi}"
        )));
    }
    Ok(timing_rng.random_range(lo..=hi))
}

/// Sample a cue index uniformly from `[0, cue_count)`.
pub fn sample_cue(cue_rng: &mut ChaCha8Rng, cue_count: usize) -> Result<usize, SimError> {
    if cue_count == 0 {
        return Err(SimError::InvalidConfiguration(
            "cue_count must be >= 1".to_owned(),
        ));
    }
    Ok(cue_rng.random_range(0..cue_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{SeedTuple, rng_for};

    fn timing_rng() -> ChaCha8Rng {
        rng_for(&SeedTuple::new(1, "development", 1, 0, "timing")).expect("valid")
    }

    #[test]
    fn fixed_range_is_exact() {
        let mut rng = timing_rng();
        for _ in 0..10 {
            assert_eq!(sample_len(&mut rng, [4, 4], "quiet").expect("sample"), 4);
        }
    }

    #[test]
    fn sampled_lengths_stay_in_range() {
        let mut rng = timing_rng();
        for _ in 0..200 {
            let q = sample_len(&mut rng, [8, 16], "quiet").expect("sample");
            assert!((8..=16).contains(&q));
            let c = sample_cue(&mut rng, 8).expect("cue");
            assert!(c < 8);
        }
    }

    #[test]
    fn inverted_range_is_an_explicit_error() {
        let mut rng = timing_rng();
        assert!(sample_len(&mut rng, [8, 4], "quiet").is_err());
    }
}
