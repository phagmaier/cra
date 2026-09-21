//! Exact event-order fixture: commitment at tick 20, delay 3 (M0-11;
//! spec 9.4, 17.1).
//!
//! Pinned golden schedule: warmup 4, quiet [4,4], cue 8, gap [5,5],
//! response 4, delay [3,3], K = 2, zero noise, zero hazard, seeds
//! (root 1, development, outer 1, lifetime 0). Birth mappings are [1, 1]
//! (both stable); the first presented cue is 1.
//!
//! Expected timeline: quiet 0-3, cue 4-11, gap 12-16, response 17-20 with
//! the only commitment point at tick 20, ordinary delay ticks 21-22,
//! exactly one feedback at the start of tick 23, next quiet from 24.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::environment::Phase;
use support::{base_config, birth};

fn fixture_config() -> cra::config::Config {
    let mut cfg = base_config();
    cfg.environment.memory_gap_ticks = [5, 5];
    cfg.environment.reward_delay_ticks = [3, 3];
    cfg.simulation.outcomes_per_lifetime = 4;
    cfg
}

#[test]
fn birth_goldens() {
    let cfg = fixture_config();
    cra::config::validate(&cfg).expect("valid");
    let lt = birth(&cfg);
    assert_eq!(lt.hidden().mapping_snapshot(), vec![1, 1]);
    for c in 0..2 {
        assert_eq!(lt.hidden().epsilon(c), 0.0);
        assert_eq!(lt.hidden().hazard(c), 0.0);
        assert_eq!(lt.hidden().role(c), cra::environment::CueRole::Stable);
        assert_eq!(lt.hidden().exposures(c), 0);
    }
}

#[test]
fn commit_at_20_delay_3_delivers_exactly_at_23() {
    let cfg = fixture_config();
    let mut lt = birth(&cfg);

    // Ticks 0-20: no feedback anywhere before commitment.
    for _ in 0..21 {
        let out = lt.advance().expect("advance");
        assert!(
            out.observation.feedback.is_none(),
            "no feedback before commitment (tick {})",
            out.tick
        );
        assert!(out.annotation.is_none());
        let f = &out.observation.features;
        assert_eq!(f.len(), 8);
        match out.tick {
            0..=3 => {
                assert_eq!(out.phase, Phase::Quiet);
                assert_eq!(f, &vec![0.0; 8]);
            }
            4..=11 => {
                assert_eq!(out.phase, Phase::Cue);
                assert_eq!(out.cue, Some(1), "golden first cue");
                assert_eq!(f, &vec![0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
            }
            12..=16 => {
                assert_eq!(out.phase, Phase::Gap);
                assert!(!out.go);
                assert_eq!(f, &vec![0.0; 8], "gap shows nothing");
            }
            17..=19 => {
                assert_eq!(out.phase, Phase::Response);
                assert!(out.go);
                assert!(!out.commitment_due);
                assert_eq!(f, &vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
            }
            20 => {
                assert_eq!(out.phase, Phase::Response);
                assert!(out.commitment_due, "sole commitment point");
                assert_eq!(f, &vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
            }
            _ => unreachable!(),
        }
    }

    // Commit the correct action (target 1) through the production path.
    let receipt = lt.commit(1).expect("commit at tick 20");
    assert_eq!(receipt.event_id, 0);
    assert_eq!(receipt.choice_index, 0);
    assert_eq!(lt.pending().expect("pending").due_tick, 23);

    // Ticks 21-22: ordinary delay transitions, latch now shows action 1.
    for tick in [21, 22] {
        let out = lt.advance().expect("delay tick");
        assert_eq!((out.tick, out.phase), (tick, Phase::Delay));
        assert!(out.observation.feedback.is_none());
        assert!(out.annotation.is_none());
        assert_eq!(
            out.observation.features,
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]
        );
    }

    // Tick 23 start: exactly one outcome using traces/gates through tick 22
    // (no learning yet — the "trace" is the stored pending reward).
    let fb = lt.advance().expect("feedback tick");
    assert_eq!((fb.tick, fb.phase), (23, Phase::Feedback));
    let feedback = fb.observation.feedback.expect("exactly one outcome");
    assert_eq!((feedback.event_id, feedback.reward), (0, 1.0));
    assert_eq!(
        fb.observation.features,
        vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0],
        "outcome-present + value, latch persists"
    );
    let annotation = fb.annotation.expect("annotation");
    assert_eq!(annotation.cue_id, 1);
    assert_eq!(annotation.target_at_commit, 1);
    assert!(annotation.latent_correctness);
    assert!(!annotation.noise_bit);
    assert_eq!(annotation.cue_exposure_index, 1);
    assert!(!annotation.hidden_change_before_presentation);
    assert_eq!(annotation.commit_tick, 20);
    assert_eq!(annotation.outcome_tick, 23);
    assert_eq!((lt.commitments(), lt.outcomes()), (1, 1));

    // Next quiet starts a fresh cycle with no leftover feedback.
    let next = lt.advance().expect("next quiet");
    assert_eq!((next.tick, next.phase), (24, Phase::Quiet));
    assert!(next.observation.feedback.is_none());
}

#[test]
fn forced_wrong_action_at_20_delivers_zero_through_the_envelope() {
    let cfg = fixture_config();
    let mut lt = birth(&cfg);
    for _ in 0..21 {
        let out = lt.advance().expect("advance");
        if out.commitment_due {
            // Wrong action, forced no-flip: reward exactly 0.
            lt.commit_with_noise(0, false).expect("forced commit");
        }
    }
    assert!(
        lt.advance()
            .expect("tick 21")
            .observation
            .feedback
            .is_none()
    );
    assert!(
        lt.advance()
            .expect("tick 22")
            .observation
            .feedback
            .is_none()
    );
    let fb = lt.advance().expect("tick 23");
    let feedback = fb.observation.feedback.expect("outcome");
    assert_eq!(feedback.reward, 0.0);
    // Zero reward is present-with-value-0, not absent.
    assert_eq!(fb.observation.features[4], 1.0);
    assert_eq!(fb.observation.features[5], 0.0);
}
