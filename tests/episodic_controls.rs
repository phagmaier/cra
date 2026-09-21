//! Matched no-update and shuffled-reward controls (M3-05; spec 13.1, 16/M3,
//! 17.8).
//!
//! - B3, B4, and shuffled share one profile, one task schedule, and one
//!   inherited actor within a seed: same `W0`, same initialization record,
//!   same cue order, same reset ticks. They differ only in the declared
//!   mechanism (`condition_id`, `learning_enabled`, `reward_protocol`).
//! - B3 and B4 take identical first-rollout actions (both start with
//!   `P = 0` and paired noise); later divergence can only come from
//!   acquired offsets.
//! - Behavior and actual `P` changes are checked together: B4 moves `P`
//!   while B3 owns no plastic state by construction.
//! - The shuffled control replaces each teaching signal with an independent
//!   fair coin from a dedicated public-seed stream. The test re-derives the
//!   exact draw sequence without touching hidden state, proving the protocol
//!   is as declared and privilege-free; observed rewards stay separately
//!   recorded.

use cra::config::Config;
use cra::experiments::episodic::{
    CONDITION_B3, CONDITION_B4, CONDITION_SHUFFLED, PROTOCOL_OBSERVED, SHUFFLE_STREAM,
    ShuffleProtocol, run_episodic_conditions, run_episodic_no_learning, run_episodic_shuffled,
};
use cra::rng::{SeedTuple, rng_for};

fn episodic_config() -> Config {
    let text =
        std::fs::read_to_string("configs/episodic_stationary.toml").expect("episodic profile");
    toml::from_str(&text).expect("episodic parses")
}

fn small_config(outcomes: u64) -> Config {
    let mut cfg = episodic_config();
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cra::config::validate(&cfg).expect("small config validates");
    cra::config::validate_episodic_execution(&cfg).expect("small config executes");
    cfg
}

fn l1(matrix: &[Vec<f64>]) -> f64 {
    matrix.iter().flatten().map(|v| v.abs()).sum()
}

#[test]
fn conditions_share_inheritance_schedule_and_resets() {
    let cfg = small_config(4);
    let set = run_episodic_conditions(&cfg, 1, "development", 1, 0).expect("conditions");
    // Same inherited weights and sampling history across the family.
    assert_eq!(set.b3.w0, set.b4.w0, "B3/B4 must share W0");
    assert_eq!(set.b4.w0, set.shuffled.w0, "B4/shuffled must share W0");
    assert_eq!(set.b3.initialization, set.b4.initialization);
    assert_eq!(set.b4.initialization, set.shuffled.initialization);
    // Same exogenous schedule: identical cue order and reset ticks.
    let b3_cues: Vec<usize> = set.b3.choices.iter().map(|c| c.cue).collect();
    let b4_cues: Vec<usize> = set.b4.choices.iter().map(|c| c.cue).collect();
    let sh_cues: Vec<usize> = set.shuffled.choices.iter().map(|c| c.cue).collect();
    assert_eq!(b3_cues, b4_cues, "cue order must match across B3/B4");
    assert_eq!(b4_cues, sh_cues, "cue order must match across B4/shuffled");
    assert_eq!(set.b3.resets, set.b4.resets, "reset ticks must match");
    assert_eq!(set.b4.resets, set.shuffled.resets);
    assert_eq!(set.b3.outcomes, 4);
    assert_eq!(set.b4.outcomes, 4);
    assert_eq!(set.shuffled.outcomes, 4);
    // Identifiers name the mechanism; everything else agrees.
    assert_eq!(set.b3.condition_id, CONDITION_B3);
    assert_eq!(set.b4.condition_id, CONDITION_B4);
    assert_eq!(set.shuffled.condition_id, CONDITION_SHUFFLED);
    assert!(!set.b3.learning_enabled);
    assert!(set.b4.learning_enabled);
    assert!(set.shuffled.learning_enabled);
    assert_eq!(set.b3.reward_protocol, PROTOCOL_OBSERVED);
    assert_eq!(set.b4.reward_protocol, PROTOCOL_OBSERVED);
    assert_eq!(
        set.shuffled.reward_protocol,
        ShuffleProtocol::IndependentFairCoin.name()
    );
    assert_eq!(set.b3.profile_name, set.b4.profile_name);
    assert_eq!(set.b4.profile_name, set.shuffled.profile_name);
    assert_eq!(set.b3.mode, set.b4.mode);
    // B3 owns no plastic state: no baseline, no offsets, no update reports.
    // Its choices carry no `update` field by construction (see type).
    assert_eq!(set.b3.final_last_feedback, Some(3));
}

#[test]
fn first_rollout_actions_match_before_learning_can_act() {
    // Both agents start with P = 0 and paired noise/tie draws, so the first
    // commitment — made before any update exists — must agree. Later
    // divergence is then attributable to acquired offsets alone.
    for lifetime in 0..3 {
        let cfg = small_config(4);
        let set = run_episodic_conditions(&cfg, 1, "development", 1, lifetime).expect("conditions");
        assert_eq!(
            set.b3.choices[0].action, set.b4.choices[0].action,
            "lifetime {lifetime}: first actions must match"
        );
        assert_eq!(
            set.b3.choices[0].action, set.shuffled.choices[0].action,
            "lifetime {lifetime}: shuffled first action must match"
        );
    }
}

#[test]
fn behavior_and_p_changes_are_checked_together() {
    let cfg = small_config(4);
    let set = run_episodic_conditions(&cfg, 1, "development", 1, 0).expect("conditions");
    // B4's updates moved real offsets: raw reports are consumed per event
    // and the final P norm is nonzero.
    assert!(
        l1(&set.b4.final_p) > 0.0,
        "B4 must show actual P movement, not just reward"
    );
    for choice in &set.b4.choices {
        assert_eq!(choice.applied_reward, choice.reward);
        assert!((choice.update.delta - (choice.reward - choice.update.baseline_old)).abs() < 1e-12);
    }
    // Effective weights stay W0 + P exactly.
    for (j, (w_row, eff_row)) in set
        .b4
        .w0
        .iter()
        .zip(set.b4.final_effective.iter())
        .enumerate()
    {
        for (i, (&w, &eff)) in w_row.iter().zip(eff_row.iter()).enumerate() {
            assert!(
                (eff - (w + set.b4.final_p[j][i])).abs() <= 1e-12,
                "B4 effective must equal W0 + P"
            );
        }
    }
    // The shuffled control also moves P (updates are real), but from a
    // corrupted signal — weight movement without task information.
    assert!(
        l1(&set.shuffled.final_p) > 0.0,
        "shuffled control must apply real (corrupted) updates"
    );
    // B3 behavior is fully explained without offsets: same schedule, same
    // start, and every reward is finite.
    assert!(set.b3.mean_reward.is_finite());
    assert_eq!(set.b3.choices.len(), 4);
}

#[test]
fn shuffled_protocol_is_exactly_as_declared_and_privilege_free() {
    let cfg = small_config(6);
    let shuffled = run_episodic_shuffled(
        &cfg,
        1,
        "development",
        1,
        0,
        "B4-shuffled",
        ShuffleProtocol::IndependentFairCoin,
    )
    .expect("shuffled");
    // Re-derive the draw sequence from public seeds only: no hidden mapping,
    // correctness, noise bit, or hazard enters. Equality with the recorded
    // applied signals proves protocol fidelity and the information boundary.
    let mut rng = rng_for(&SeedTuple::new(1, "development", 1, 0, SHUFFLE_STREAM)).expect("rng");
    let protocol = ShuffleProtocol::IndependentFairCoin;
    for choice in &shuffled.choices {
        let expected = protocol.draw_applied(&mut rng);
        assert_eq!(
            choice.applied_reward, expected,
            "applied signal must equal the re-derived public-coin draw"
        );
        assert!(
            choice.applied_reward == 0.0 || choice.applied_reward == 1.0,
            "applied signal must be binary"
        );
        // Observed environment reward is recorded unchanged, distinct from
        // the corrupted teaching signal whenever the coin disagrees.
        assert!(choice.reward == 0.0 || choice.reward == 1.0);
        assert_eq!(choice.update.reward, choice.applied_reward);
    }
    assert_eq!(shuffled.choices.len(), 6);
    assert!(
        shuffled
            .choices
            .iter()
            .any(|c| c.applied_reward != c.reward),
        "a fair coin must disagree with the observed reward at least once in six draws for this seed"
    );
    // The update's delta uses the applied signal against the running
    // baseline, never the observed reward directly.
    for choice in &shuffled.choices {
        assert!(
            (choice.update.delta - (choice.applied_reward - choice.update.baseline_old)).abs()
                < 1e-12
        );
    }
}

#[test]
fn b3_runner_is_deterministic_and_rejects_non_episodic_profiles() {
    let cfg = small_config(4);
    let first = run_episodic_no_learning(&cfg, 1, "development", 1, 0, "B3").expect("b3");
    let second = run_episodic_no_learning(&cfg, 1, "development", 1, 0, "B3").expect("b3 rerun");
    assert_eq!(
        first.choices.iter().map(|c| c.action).collect::<Vec<_>>(),
        second.choices.iter().map(|c| c.action).collect::<Vec<_>>()
    );
    assert_eq!(first.resets, second.resets);
    assert_eq!(first.w0, second.w0);
    // A birth_only actor profile is not an episodic control config.
    let debug: Config =
        toml::from_str(&std::fs::read_to_string("configs/debug_stationary.toml").unwrap()).unwrap();
    assert!(
        run_episodic_no_learning(&debug, 1, "development", 1, 0, "B3").is_err(),
        "B3 control requires the episodic diagnostic profile"
    );
}
