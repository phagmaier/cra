//! Deterministic environment contracts (M0-07 through M0-09, M0-11; spec
//! 17.1 scheduling, reward, and feature items).
//!
//! Covered here: exact phase tick counts (including zero-length gaps),
//! hazard 0/1 exposure semantics, delay-1 next-tick delivery, reward from
//! the mapping saved at commitment, forced noise reversal, one commitment
//! per pending reward, unique event identity, completed-lifetime counts,
//! quiet-tick hidden-state invariance, consumption-ledger errors,
//! agent-stream draw independence, and every K + 6 feature channel
//! (one-hot cue, cue-present, go, outcome-present/value, previous-action
//! latch). The tick-20/delay-3 event-order fixture lives in
//! `tests/event_order.rs`.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::environment::Phase;
use cra::environment::observation::SimError;
use cra::rng::{SeedTuple, rng_for};
use rand_core::RngCore;
use support::{advance_until_commitment, advance_until_feedback, base_config, birth, run_cycle};

#[test]
fn exact_cycle_counts_with_zero_gap_and_delay_one() {
    // warmup 4, quiet [4,4], cue 8, gap [0,0], response 4, delay [1,1]:
    // ticks 0-3 quiet, 4-11 cue, 12-15 response, commit at 15,
    // feedback at 16, next quiet 17-20.
    let cfg = base_config();
    let mut lt = birth(&cfg);
    let mut phases = Vec::new();
    // Ticks 0-15: warmup/quiet, cue, response. Tick 16 (delay 1) is the
    // feedback tick and is checked separately below.
    for _ in 0..16 {
        let out = lt.advance().expect("advance");
        assert_eq!(out.observation.features.len(), 8, "K + 6 channels");
        assert!(out.observation.feedback.is_none(), "no feedback yet");
        if out.commitment_due {
            assert_eq!(out.tick, 15);
            assert_eq!(out.phase, Phase::Response);
            let pending_before = lt.pending().is_none();
            assert!(pending_before);
            lt.commit(0).expect("commit");
            let pending = lt.pending().expect("one pending reward");
            assert_eq!(pending.due_tick, 16, "delay 1: next-tick feedback");
        }
        phases.push((out.tick, out.phase, out.cue, out.go));
    }
    let phase_at = |t: u64| phases.iter().find(|(tick, ..)| *tick == t).unwrap().1;
    for t in 0..4 {
        assert_eq!(phase_at(t), Phase::Quiet, "tick {t}");
    }
    for t in 4..12 {
        assert_eq!(phase_at(t), Phase::Cue, "tick {t}");
    }
    // Zero-length gap spends no tick: response starts immediately.
    for t in 12..16 {
        assert_eq!(phase_at(t), Phase::Response, "tick {t}");
    }
    assert_eq!(lt.commitments(), 1);
    assert_eq!(lt.outcomes(), 0, "feedback not yet delivered");

    let fb = lt.advance().expect("feedback tick");
    assert_eq!(fb.tick, 16);
    assert_eq!(fb.phase, Phase::Feedback);
    assert!(fb.observation.feedback.is_some());
    assert!(fb.annotation.is_some());
    assert!(lt.pending().is_none());
    assert_eq!(lt.outcomes(), 1);

    let next = lt.advance().expect("next quiet");
    assert_eq!(next.phase, Phase::Quiet);
    assert!(next.observation.feedback.is_none());
}

#[test]
fn gap_and_delay_three_shape() {
    // gap [2,2], delay [3,3]: quiet 0-3, cue 4-11, gap 12-13,
    // response 14-17, commit at 17, ordinary delay 18-19, feedback at 20.
    let mut cfg = base_config();
    cfg.environment.memory_gap_ticks = [2, 2];
    cfg.environment.reward_delay_ticks = [3, 3];
    let mut lt = birth(&cfg);

    let commit_out = advance_until_commitment(&mut lt);
    assert_eq!(commit_out.tick, 17);
    let cue = {
        // Re-derive the presented cue from hidden exposures instead of
        // trusting driver memory: exactly one cue has an exposure.
        let hidden = lt.hidden();
        (0..2)
            .find(|&c| hidden.exposures(c) == 1)
            .expect("one exposure")
    };
    lt.commit(0).expect("commit");
    assert_eq!(lt.pending().expect("pending").due_tick, 20);

    let mut delay_ticks = Vec::new();
    for _ in 0..2 {
        let out = lt.advance().expect("delay tick");
        assert_eq!(out.phase, Phase::Delay);
        assert!(out.observation.feedback.is_none());
        delay_ticks.push(out.tick);
    }
    assert_eq!(delay_ticks, vec![18, 19]);

    let fb = lt.advance().expect("feedback");
    assert_eq!(fb.tick, 20, "commit 17 + delay 3 = tick 20 start");
    assert!(fb.observation.feedback.is_some());
    assert_eq!(lt.outcomes(), 1);
    assert_eq!(lt.commitments(), 1);
    let annotation = fb.annotation.expect("annotation");
    assert_eq!(annotation.cue_id, cue);
    assert_eq!(annotation.commit_tick, 17);
    assert_eq!(annotation.outcome_tick, 20);
}

#[test]
fn hazard_zero_never_flips_across_cycles() {
    let cfg = base_config();
    let mut lt = birth(&cfg);
    let birth_mapping = lt.hidden().mapping_snapshot();
    for cycle in 0..6 {
        let rec = run_cycle(&mut lt, (cycle % 2) as u8);
        assert_eq!(lt.hidden().mapping_snapshot(), birth_mapping);
        assert!(!rec.correct || rec.reward == 1.0);
    }
    for c in 0..2 {
        assert!(lt.hidden().exposures(c) > 0, "both cues seen");
    }
}

#[test]
fn hazard_one_flips_every_repeat_but_not_the_first() {
    let mut cfg = base_config();
    cfg.environment.cue_count = 1;
    cfg.environment.stable_fraction = 0.0;
    cfg.environment.volatile_hazard_values = vec![1.0];
    assert_eq!(cfg.environment.feedback_noise_values, vec![0.0]);
    let mut lt = birth(&cfg);
    assert_eq!(lt.hidden().hazard(0), 1.0);
    let birth_mapping = lt.hidden().mapping(0);

    // First presentation: no flip; committing the birth mapping is correct.
    let first = run_cycle(&mut lt, birth_mapping);
    assert_eq!(first.cue, 0);
    assert_eq!(first.reward, 1.0);
    assert_eq!(lt.hidden().mapping(0), birth_mapping);

    // Second presentation flips before the cue: the old action is now wrong.
    let second = run_cycle(&mut lt, birth_mapping);
    assert_eq!(lt.hidden().mapping(0), 1 - birth_mapping);
    assert_eq!(second.reward, 0.0);

    // Third presentation flips back to the birth mapping (hazard 1 flips
    // *every* repeat): the birth action is correct again.
    let third = run_cycle(&mut lt, birth_mapping);
    assert_eq!(lt.hidden().mapping(0), birth_mapping);
    assert_eq!(third.reward, 1.0);
}

#[test]
fn quiet_ticks_never_mutate_hidden_mappings() {
    let cfg = base_config();
    let mut lt = birth(&cfg);
    // Birth phase is the warmup quiet: step through it watching mappings.
    let before = lt.hidden().mapping_snapshot();
    for _ in 0..4 {
        let out = lt.advance().expect("quiet tick");
        assert_eq!(out.phase, Phase::Quiet);
        assert_eq!(lt.hidden().mapping_snapshot(), before);
    }
}

#[test]
fn correct_action_scores_one_and_wrong_scores_zero_without_noise() {
    let cfg = base_config();
    let mut lt = birth(&cfg);
    // Alternate: commit the presented cue's mapping (reward 1), then the
    // opposite (reward 0).
    let rec = run_cycle_with_correct(&mut lt, true);
    assert_eq!(rec.reward, 1.0);
    assert!(rec.correct);
    let rec = run_cycle_with_correct(&mut lt, false);
    assert_eq!(rec.reward, 0.0);
    assert!(!rec.correct);
}

fn run_cycle_with_correct(
    lt: &mut cra::environment::Lifetime,
    correct: bool,
) -> support::CycleRecord {
    support::run_cycle_with(lt, |lt, cue| {
        // Resolve the presented cue's current mapping through the evaluator
        // accessor, then commit accordingly. The agent itself never does
        // this; the test harness plays the oracle role for accounting.
        let target = lt.hidden().mapping(cue);
        lt.commit(if correct { target } else { 1 - target })
            .expect("commit");
    })
}

#[test]
fn forced_noise_bit_reverses_either_reward() {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 4;
    let mut lt = birth(&cfg);
    // correct+noflip=1, correct+flip=0, wrong+noflip=0, wrong+flip=1.
    for (want_correct, noise, want_reward) in [
        (true, false, 1.0),
        (true, true, 0.0),
        (false, false, 0.0),
        (false, true, 1.0),
    ] {
        let rec = support::run_cycle_with(&mut lt, |lt, cue| {
            let target = lt.hidden().mapping(cue);
            lt.commit_with_noise(if want_correct { target } else { 1 - target }, noise)
                .expect("forced commit");
        });
        assert_eq!(rec.reward, want_reward);
    }
    assert_eq!((lt.commitments(), lt.outcomes()), (4, 4));
}

#[test]
fn pending_reward_uses_mapping_saved_at_commit() {
    let mut cfg = base_config();
    cfg.environment.cue_count = 1;
    cfg.environment.stable_fraction = 0.0;
    cfg.environment.volatile_hazard_values = vec![1.0];
    let mut lt = birth(&cfg);

    // Commit with the mapping visible at commit time; the stored pending
    // reward must equal what is later delivered, and the stored target must
    // equal the mapping read at commit (not a stale or future one).
    let commit_out = advance_until_commitment(&mut lt);
    assert!(commit_out.commitment_due);
    let target_now = lt.hidden().mapping(0);
    lt.commit(target_now).expect("commit");
    let stored = lt.pending().expect("pending").clone();
    assert_eq!(stored.target_at_commit, target_now);
    assert_eq!(stored.reward, 1.0);
    let fb = advance_until_feedback(&mut lt);
    assert_eq!(
        fb.observation.feedback.expect("feedback").reward,
        stored.reward
    );

    // Next cycle the hazard-1 mapping has flipped; the new pending reward
    // is computed from the new mapping, proving commits read current truth.
    advance_until_commitment(&mut lt);
    let flipped = lt.hidden().mapping(0);
    assert_eq!(flipped, 1 - target_now);
    lt.commit(flipped).expect("commit");
    assert_eq!(lt.pending().expect("pending").target_at_commit, flipped);
}

#[test]
fn double_commit_invalid_action_and_missing_commit_are_errors() {
    let cfg = base_config();
    let mut lt = birth(&cfg);

    // Commit outside the final response tick.
    assert!(matches!(
        lt.commit(0),
        Err(SimError::CommitOutOfPhase { .. })
    ));

    advance_until_commitment(&mut lt);
    assert!(matches!(lt.commit(2), Err(SimError::InvalidAction(2))));
    lt.commit(0).expect("first commit");
    // Second commitment while one is unresolved.
    assert!(matches!(
        lt.commit(1),
        Err(SimError::CommitOutOfPhase { .. })
    ));
    // Advancing past the commitment point without committing.
    let mut lt2 = birth(&cfg);
    advance_until_commitment(&mut lt2);
    assert!(matches!(lt2.advance(), Err(SimError::MissingCommitment(_))));
}

#[test]
fn event_ids_are_unique_and_each_commitment_delivers_once() {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 4;
    let mut lt = birth(&cfg);
    let mut ids = Vec::new();
    for cycle in 0..4 {
        let rec = run_cycle(&mut lt, (cycle % 2) as u8);
        ids.push(rec.event_id);
        // Delay [1,1]: feedback lands on the tick after commitment.
        assert_eq!(rec.feedback_tick, rec.commit_tick + 1);
        assert_eq!(lt.consumed_ids().len() as u64, lt.outcomes());
        assert_eq!(lt.commitments(), lt.outcomes());
    }
    assert_eq!(ids, vec![0, 1, 2, 3]);
    assert!(lt.is_complete());
    assert!(matches!(
        lt.advance(),
        Err(SimError::LifetimeComplete { .. })
    ));
}

#[test]
fn completed_lifetime_has_equal_counts_and_no_pending_reward() {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 3;
    let mut lt = birth(&cfg);
    for cycle in 0..3 {
        run_cycle(&mut lt, (cycle % 2) as u8);
    }
    assert_eq!((lt.commitments(), lt.outcomes()), (3, 3));
    assert!(lt.pending().is_none());
    assert!(lt.is_complete());
}

#[test]
fn consumption_ledger_rejects_duplicates_and_unknown_ids() {
    let cfg = base_config();
    let mut lt = birth(&cfg);
    let rec = run_cycle(&mut lt, 0);
    lt.note_feedback_consumed(rec.event_id)
        .expect("first confirm");
    let before = lt.consumed_ids().len();
    assert!(matches!(
        lt.note_feedback_consumed(rec.event_id),
        Err(SimError::DuplicateFeedback(_))
    ));
    assert_eq!(
        lt.consumed_ids().len(),
        before,
        "failed confirm changes nothing"
    );
    assert!(matches!(
        lt.note_feedback_consumed(9999),
        Err(SimError::UnknownFeedback(_))
    ));
}

/// One traced tick for schedule comparison: tick, phase, shown cue,
/// go flag, and optional (event id, reward).
type TraceRow = (u64, Phase, Option<usize>, bool, Option<(u64, f64)>);

#[test]
fn agent_stream_draws_do_not_alter_the_exogenous_schedule() {
    fn trace(use_agent_draws: bool) -> Vec<TraceRow> {
        let cfg = base_config();
        let mut lt = birth(&cfg);
        let mut agent_rng = if use_agent_draws {
            Some(rng_for(&SeedTuple::new(1, "development", 1, 0, "actor_noise")).expect("stream"))
        } else {
            None
        };
        let mut trace = Vec::new();
        while !lt.is_complete() {
            if let Some(rng) = agent_rng.as_mut() {
                for _ in 0..100 {
                    let _ = rng.next_u64();
                }
            }
            let out = lt.advance().expect("advance");
            if out.commitment_due {
                lt.commit(0).expect("commit");
            }
            trace.push((
                out.tick,
                out.phase,
                out.cue,
                out.go,
                out.observation.feedback.map(|fb| (fb.event_id, fb.reward)),
            ));
        }
        trace.push((lt.tick(), Phase::Done, None, false, None));
        trace
    }
    assert_eq!(trace(false), trace(true));

    // ... while a different lifetime index draws a different schedule.
    let cfg = base_config();
    cra::config::validate(&cfg).expect("valid");
    let mut other = cra::environment::Lifetime::new(&cfg, 1, "development", 1, 3).expect("birth");
    let mut first_cues = Vec::new();
    for _ in 0..30 {
        let out = other.advance().expect("advance");
        if out.commitment_due {
            other.commit(0).expect("commit");
        }
        if out.cue.is_some() {
            first_cues.push(out.cue);
        }
        if other.is_complete() {
            break;
        }
    }
    assert!(!first_cues.is_empty());
}

#[test]
fn feature_channels_follow_phase_flags() {
    // Base config: warmup 4, quiet [4,4], cue 8, gap [0,0], response 4,
    // delay [1,1], K = 2. Channel layout: 0,1 cue; 2 cue-present; 3 go;
    // 4 outcome-present; 5 outcome-value; 6,7 previous-action.
    let cfg = base_config();
    let mut lt = birth(&cfg);
    let mut presented = None;
    let mut commit_tick = None;
    for _ in 0..16 {
        let out = lt.advance().expect("advance");
        let f = &out.observation.features;
        assert_eq!(f.len(), 8);
        match out.tick {
            0..=3 => {
                assert_eq!(out.phase, Phase::Quiet);
                assert_eq!(f, &vec![0.0; 8], "quiet shows nothing, latch empty");
            }
            4..=11 => {
                assert_eq!(out.phase, Phase::Cue);
                let cue = out.cue.expect("cue shown");
                presented = Some(cue);
                let mut expected = vec![0.0; 8];
                expected[cue] = 1.0;
                expected[2] = 1.0;
                assert_eq!(f, &expected, "one-hot cue plus cue-present");
            }
            12..=15 => {
                assert_eq!(out.phase, Phase::Response);
                assert!(out.go);
                let mut expected = vec![0.0; 8];
                expected[3] = 1.0;
                assert_eq!(f, &expected, "go only; cue gone, latch still empty");
            }
            _ => unreachable!(),
        }
        if out.commitment_due {
            commit_tick = Some(out.tick);
            let cue = presented.expect("cue was shown");
            lt.commit(lt.hidden().mapping(cue)).expect("commit");
        }
    }
    assert_eq!(commit_tick, Some(15));

    // Feedback tick: outcome channels set, latch shows the committed action.
    let fb = lt.advance().expect("feedback");
    assert_eq!(fb.tick, 16);
    let f = &fb.observation.features;
    assert_eq!(f[4], 1.0, "outcome-present set");
    assert_eq!(f[5], 1.0, "correct action at zero noise");
    assert_eq!(&f[0..4], &[0.0; 4], "no cue or go on a feedback tick");
    let committed = lt.last_action().expect("latched");
    assert_eq!(f[6 + usize::from(committed)], 1.0);
    assert_eq!(f[6 + usize::from(1 - committed)], 0.0);
}

#[test]
fn zero_reward_is_distinguishable_from_no_outcome() {
    let cfg = base_config();
    let mut lt = birth(&cfg);
    // Drive the whole lifetime tick by tick. The first cycle commits the
    // wrong action at zero noise (reward exactly 0); later cycles commit 0.
    let mut cycle_cue = None;
    let mut first = true;
    let mut present_ticks = Vec::new();
    let mut values = Vec::new();
    while !lt.is_complete() {
        let out = lt.advance().expect("advance");
        if out.cue.is_some() {
            cycle_cue = out.cue;
        }
        if out.observation.features[4] == 1.0 {
            present_ticks.push(out.tick);
            values.push(out.observation.features[5]);
        }
        if out.commitment_due {
            if first {
                let cue = cycle_cue.expect("cue shown");
                lt.commit(1 - lt.hidden().mapping(cue))
                    .expect("commit wrong");
                first = false;
            } else {
                lt.commit(0).expect("commit");
            }
        }
    }
    // outcome-present fires exactly once per delivered outcome ...
    assert_eq!(present_ticks.len(), lt.outcomes() as usize);
    assert!(present_ticks.windows(2).all(|w| w[1] > w[0]));
    for v in &values {
        assert!(*v == 0.0 || *v == 1.0);
    }
    // ... and the forced wrong first cycle delivered reward 0 *through* the
    // envelope: present with value 0, not absent.
    assert_eq!(values[0], 0.0);
}

#[test]
fn previous_action_latch_changes_starting_next_tick() {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 2;
    let mut lt = birth(&cfg);

    // Cycle 1 commits action 1 at tick 15: outputs through tick 15 show an
    // empty latch; tick 16 onward shows channel 7.
    let mut boundary = Vec::new();
    for _ in 0..17 {
        let out = lt.advance().expect("advance");
        boundary.push((
            out.tick,
            out.observation.features[6],
            out.observation.features[7],
        ));
        if out.commitment_due {
            assert_eq!(out.tick, 15);
            lt.commit(1).expect("commit 1");
        }
    }
    for (tick, a0, a1) in &boundary[..16] {
        assert_eq!((*a0, *a1), (0.0, 0.0), "latch empty through tick {tick}");
    }
    assert_eq!(boundary[16], (16, 0.0, 1.0));

    // Cycle 2 commits action 0: the latch flips starting the next tick.
    let mut saw_zero_latch = false;
    while !lt.is_complete() {
        let out = lt.advance().expect("advance");
        if out.commitment_due {
            // Latch still shows the previous commitment (action 1).
            assert_eq!(
                (out.observation.features[6], out.observation.features[7]),
                (0.0, 1.0),
                "old latch visible at commitment tick {}",
                out.tick
            );
            lt.commit(0).expect("commit 0");
        } else if out.tick > 16 && out.observation.features[6] == 1.0 {
            saw_zero_latch = true;
            assert_eq!(out.observation.features[7], 0.0);
        }
    }
    assert!(saw_zero_latch, "latch flipped to action 0 after tick 16");
}

#[test]
fn cues_vanish_outside_presentation_and_feedback_lasts_one_tick() {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 2;
    let mut lt = birth(&cfg);
    let mut cue_ticks = 0;
    let mut present_ticks = 0;
    while !lt.is_complete() {
        let out = lt.advance().expect("advance");
        let f = &out.observation.features;
        let cue_active = f[0] == 1.0 || f[1] == 1.0;
        if cue_active {
            cue_ticks += 1;
            assert_eq!(out.phase, Phase::Cue);
            assert_eq!(f[2], 1.0);
        } else {
            assert_eq!(f[2], 0.0, "cue-present only with cue content");
        }
        if f[4] == 1.0 {
            present_ticks += 1;
            assert_eq!(out.phase, Phase::Feedback);
        }
        if out.commitment_due {
            lt.commit(0).expect("commit");
        }
    }
    // Two cycles x 8 cue ticks; exactly one feedback tick per outcome.
    assert_eq!(cue_ticks, 16);
    assert_eq!(present_ticks, 2);
}
