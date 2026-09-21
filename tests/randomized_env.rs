//! Seeded randomized environment sanity checks (M0-12).
//!
//! Statistical plan (declared before executing; fixed diagnostic seeds —
//! a failure is investigated, never rerun with fresh seeds until it
//! passes):
//!
//! | Check | Sample | Null SE | Tolerance | ~Sigma |
//! |---|---|---|---|---|
//! | birth-mapping balance per cue | 2048 births (outer 1..=2048) | 0.0111 | 0.04 | 3.6 |
//! | cue presentation frequency | 1024 presentations (16 lifetimes x 64) | 0.0156 | 0.06 | 3.8 |
//! | B0 latent correctness | 2048 choices (32 lifetimes x 64) | 0.0111 | 0.04 | 3.6 |
//! | oracle mean reward, eps 0.2 | 1024 choices (16 lifetimes x 64) | 0.0125 | 0.05 | 4.0 |
//!
//! Two-sided consistency checks only. The oracle expectation is never used
//! as a per-run upper bound (spec 13.3). Per-check false-alarm probability
//! under the null is ~1e-4, so a failure points at the implementation.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::experiments::baseline::{RandomBaseline, run_oracle, run_ordinary};
use support::base_config;

fn birth_config_64() -> cra::config::Config {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 64;
    cfg
}

#[test]
fn birth_mappings_are_balanced() {
    // 2048 births on outer seeds 1..=2048, lifetime 0. Each cue's preferred
    // action is an independent fair coin: expect mean 0.5 +- 0.04.
    let cfg = base_config();
    cra::config::validate(&cfg).expect("valid");
    let n = 2048u64;
    let mut ones = [0u64; 2];
    for outer in 1..=n {
        let lt = cra::environment::Lifetime::new(&cfg, 1, "development", outer, 0).expect("birth");
        for (c, count) in ones.iter_mut().enumerate() {
            *count += u64::from(lt.hidden().mapping(c));
        }
    }
    for (c, count) in ones.iter().enumerate() {
        let mean = *count as f64 / n as f64;
        assert!(
            (mean - 0.5).abs() <= 0.04,
            "cue {c} mapping balance {mean} outside tolerance (ones={count})"
        );
    }
}

#[test]
fn cue_presentations_are_balanced() {
    // 16 lifetimes x 64 outcomes = 1024 presentations of K = 2 cues driven
    // by the cue_order stream: expect cue-0 fraction 0.5 +- 0.06.
    let cfg = birth_config_64();
    cra::config::validate(&cfg).expect("valid");
    let mut total = 0u64;
    let mut cue_zero = 0u64;
    for lifetime in 0..16 {
        let mut policy = RandomBaseline::new(1, "development", 1, lifetime).expect("baseline");
        let summary =
            run_ordinary(&cfg, 1, "development", 1, lifetime, "random", &mut policy).expect("run");
        assert_eq!(summary.choices.len(), 64);
        for choice in &summary.choices {
            total += 1;
            cue_zero += u64::from(choice.cue == 0);
        }
    }
    assert_eq!(total, 1024);
    let frac = cue_zero as f64 / total as f64;
    assert!(
        (frac - 0.5).abs() <= 0.06,
        "cue-0 fraction {frac} outside tolerance"
    );
}

#[test]
fn random_action_correctness_is_chance() {
    // Random actions against independently random mappings: latent
    // correctness 0.5 +- 0.04 over 32 lifetimes x 64 choices. Correctness
    // (not reward) is used so feedback noise cannot shift the expectation.
    let cfg = birth_config_64();
    cra::config::validate(&cfg).expect("valid");
    let mut total = 0u64;
    let mut correct = 0u64;
    for lifetime in 0..32 {
        let mut policy = RandomBaseline::new(1, "development", 1, lifetime).expect("baseline");
        let summary =
            run_ordinary(&cfg, 1, "development", 1, lifetime, "random", &mut policy).expect("run");
        for choice in &summary.choices {
            total += 1;
            correct += u64::from(choice.correct);
        }
    }
    assert_eq!(total, 2048);
    let rate = correct as f64 / total as f64;
    assert!(
        (rate - 0.5).abs() <= 0.04,
        "B0 correctness {rate} outside tolerance"
    );
}

#[test]
fn oracle_reward_matches_its_noise_conditioned_expectation() {
    // eps = 0.2 on every cue: oracle mean reward 0.8 +- 0.05 over 16 x 64
    // choices. Two-sided consistency only — never a per-run upper bound.
    let mut cfg = birth_config_64();
    cfg.environment.kind = "stationary_noisy".to_owned();
    cfg.environment.feedback_noise_values = vec![0.2];
    cra::config::validate(&cfg).expect("valid");
    let mut total = 0u64;
    let mut reward_sum = 0.0;
    for lifetime in 0..16 {
        let summary = run_oracle(&cfg, 1, "development", 1, lifetime).expect("oracle run");
        assert_eq!(summary.latent_accuracy(), 1.0);
        for choice in &summary.choices {
            total += 1;
            reward_sum += choice.reward;
        }
    }
    assert_eq!(total, 1024);
    let mean = reward_sum / total as f64;
    assert!(
        (mean - 0.8).abs() <= 0.05,
        "oracle mean reward {mean} outside tolerance of 0.8"
    );
}
