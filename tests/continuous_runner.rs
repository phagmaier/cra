//! Birth-only resets and warmup semantics (M4-03; spec 9-10, 17.2).
//!
//! - The continuous runner's reset audit is exactly `[0]` (birth) over
//!   full multi-choice lifetimes: no reset fires at cue changes,
//!   rewards, hidden reversals, or post-warmup boundaries. (There is no
//!   log-rotation path in the library runners — `run.rs` writes
//!   per-lifetime files — so no hook exists to trip there; the audit
//!   covers every in-library boundary.)
//! - Acquired state provably persists: final `P` telescopes exactly to
//!   the sum of per-choice actual updates (a mid-lifetime reset would
//!   break the sum), and `E` is nonzero at every feedback (cross-choice
//!   carryover, never cleared).
//! - Warmup ticks run with live traces: `E` is nonzero entering the
//!   first cue, with no post-warmup reset.
//! - Birth clears exactly: fresh learners start at `P = E = 0` with the
//!   configured baseline; distinct lifetime indices draw independent
//!   agent streams; identical coordinates reproduce bit-identically.
//! - Finishing includes the final feedback transition: the lifetime ends
//!   one tick after the last outcome with counts reconciled.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::config::{Actor, Learning};
use cra::environment::Lifetime;
use cra::experiments::continuous::{ContinuousLearner, run_continuous_lifetime};
use cra::rng::{SeedTuple, rng_for};
use support::base_config;

fn actor_cfg() -> Actor {
    Actor {
        neuron_count: 4,
        motor_neurons_per_action: 1,
        edge_probability: 0.25,
        self_edges: false,
        recurrent_gain: 0.8,
        input_scale: 0.3,
        tau_h: 5.0,
        tau_a: 100.0,
        adaptation_strength: 0.0,
        noise_sigma: 0.05,
        motor_filter_tau: 3.0,
    }
}

fn learning_cfg() -> Learning {
    Learning {
        enabled: true,
        rule: "gaussian_transition_score".to_owned(),
        plastic_mask: "motor_afferent_only".to_owned(),
        trace_policy: "persistent".to_owned(),
        tau_e: 64.0,
        eta: 0.001,
        max_update: 0.01,
        plastic_bound: 0.5,
        plastic_decay: 0.0,
        reward_baseline_initial: 0.5,
        reward_baseline_beta: 0.02,
    }
}

fn continuous_config(outcomes: u64) -> cra::config::Config {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cfg.actor = Some(actor_cfg());
    cfg.learning = Some(learning_cfg());
    cra::config::validate(&cfg).expect("config validates");
    cra::config::validate_continuous_execution(&cfg).expect("continuous executes");
    cfg
}

fn l1(m: &[Vec<f64>]) -> f64 {
    m.iter().flatten().map(|v| v.abs()).sum()
}

#[test]
fn continuous_guard_rejects_non_birth_only_conditions() {
    let mut cfg = continuous_config(2);
    cfg.simulation.reset_policy = "episodic_diagnostic".to_owned();
    assert!(cra::config::validate_continuous_execution(&cfg).is_err());

    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 2;
    cfg.actor = Some(actor_cfg());
    let mut learning = learning_cfg();
    learning.enabled = false;
    cfg.learning = Some(learning);
    assert!(cra::config::validate_continuous_execution(&cfg).is_err());

    let mut cfg = continuous_config(2);
    cfg.modulator = Some(cra::config::Modulator {
        mode: "global".to_owned(),
        neuron_count: 2,
        tau_m: 20.0,
        ordinary_feedback_to_actor: false,
        gate_timing: "pre_outcome".to_owned(),
        gate_bias_initial: 0.0,
        projection_init_std: 0.0,
    });
    assert!(cra::config::validate_continuous_execution(&cfg).is_err());

    let mut cfg = continuous_config(2);
    cfg.evolution = Some(cra::config::Evolution {
        enabled: true,
        population_size: 4,
        elite_count: 1,
        mutation_std: 0.1,
        search_space: "gate_projection_only".to_owned(),
        decoded_weight_limit: None,
        lifetimes_per_candidate: None,
        generations: None,
        validation_every_generations: None,
        validation_lifetimes: None,
    });
    assert!(cra::config::validate_continuous_execution(&cfg).is_err());
}

#[test]
fn no_resets_across_full_lifetime_with_telescoping_offsets() {
    let cfg = continuous_config(8);
    let summary = run_continuous_lifetime(&cfg, 1, "development", 1, 0, "continuous")
        .expect("continuous run");
    // Reset audit: birth tick only.
    assert_eq!(summary.resets, vec![0]);
    assert_eq!(summary.reset_policy, "birth_only");
    assert_eq!(summary.trace_policy, "persistent");
    // Counts reconcile over 8 choices.
    assert_eq!(summary.outcomes, 8);
    assert_eq!(summary.commitments, 8);
    assert_eq!(summary.choices.len(), 8);
    assert_eq!(summary.annotations.len(), 8);
    for (k, choice) in summary.choices.iter().enumerate() {
        assert_eq!(choice.choice_index, k as u64);
        assert_eq!(choice.event_id, k as u64);
        assert!(choice.feedback_tick > choice.commit_tick);
    }
    assert_eq!(summary.final_last_feedback, Some(7));
    // Acquired offsets telescope: final P equals the summed per-choice
    // actual updates. Any mid-lifetime P/E reset would break the sum.
    let mut acc = vec![vec![0.0; 4]; 4];
    for choice in &summary.choices {
        for (a_row, u_row) in acc.iter_mut().zip(choice.update.actual_updates.iter()) {
            for (a, &u) in a_row.iter_mut().zip(u_row.iter()) {
                *a += u;
            }
        }
    }
    for (j, (got_row, acc_row)) in summary.final_p.iter().zip(acc.iter()).enumerate() {
        for (i, (&got, &want)) in got_row.iter().zip(acc_row.iter()).enumerate() {
            assert!(
                (got - want).abs() <= 1e-12,
                "final P[{j},{i}] {got} != summed actuals {want}"
            );
        }
    }
    assert!(l1(&summary.final_p) > 0.0, "offsets moved");
    // Traces persist across choices: nonzero at the end, and the running
    // baseline moved exactly once per outcome.
    assert!(l1(&summary.final_e) > 0.0, "traces persist");
    assert_ne!(summary.final_baseline, 0.5);
}

#[test]
fn hidden_reversal_fires_no_reset_and_lifetime_completes() {
    // Volatile hazard 1.0 guarantees mapping flips on repeat exposures;
    // the runner must ride through them with resets == [0].
    let mut cfg = continuous_config(24);
    cfg.environment.stable_fraction = 0.5;
    cfg.environment.feedback_noise_values = vec![0.0];
    cfg.environment.volatile_hazard_values = vec![1.0];
    cra::config::validate(&cfg).expect("reversal config validates");
    cra::config::validate_continuous_execution(&cfg).expect("continuous executes");
    let summary = run_continuous_lifetime(&cfg, 1, "development", 1, 0, "continuous")
        .expect("continuous run");
    let changes = summary
        .annotations
        .iter()
        .filter(|a| a.hidden_change_before_presentation)
        .count();
    assert!(changes > 0, "at least one hidden reversal must occur");
    assert_eq!(summary.resets, vec![0], "no reset at reversals or rewards");
    assert_eq!((summary.commitments, summary.outcomes), (24, 24));
    assert_eq!(summary.final_last_feedback, Some(23));
    // Offsets still telescope across the reversals.
    let mut acc = vec![vec![0.0; 4]; 4];
    for choice in &summary.choices {
        for (a_row, u_row) in acc.iter_mut().zip(choice.update.actual_updates.iter()) {
            for (a, &u) in a_row.iter_mut().zip(u_row.iter()) {
                *a += u;
            }
        }
    }
    for (got_row, acc_row) in summary.final_p.iter().zip(acc.iter()) {
        for (&got, &want) in got_row.iter().zip(acc_row.iter()) {
            assert!((got - want).abs() <= 1e-12);
        }
    }
}

#[test]
fn warmup_runs_with_live_traces_and_no_post_warmup_reset() {
    // Manual split-order drive over the first two choices: warmup ticks
    // (leading quiet, 4 ticks here) must already accrue eligibility, and
    // the warmup→cue boundary must neither freeze nor clear it.
    let cfg = continuous_config(2);
    let mut lifetime = Lifetime::new(&cfg, 1, "development", 1, 0).expect("birth");
    let noise = rng_for(&SeedTuple::new(1, "development", 1, 0, "actor_noise")).expect("rng");
    let tie = rng_for(&SeedTuple::new(1, "development", 1, 0, "tie_break")).expect("rng");
    let actor = actor_cfg();
    let learning = learning_cfg();
    let inherited = {
        let tuple = SeedTuple::new(1, "development", 1, 0, cra::rng::ACTOR_INIT_STREAM);
        cra::agent::weights::sample_inherited(
            &actor,
            cra::environment::feature_dim(2),
            &tuple,
            cra::agent::topology::DEFAULT_MAX_STRUCTURAL_ATTEMPTS,
        )
        .expect("inheritance")
        .params
    };
    let mut learner =
        ContinuousLearner::from_agent_parts(actor, learning, inherited, 2, noise, tie)
            .expect("learner");
    let mut advances = 0u64;
    let mut saw_cue = false;
    let mut e_before_first_cue = 0.0;
    let mut e_after_first_cue = 0.0;
    while !lifetime.is_complete() {
        let out = lifetime.observe().expect("observe");
        if !saw_cue && out.cue.is_some() {
            saw_cue = true;
            // Traces accrued during warmup must already be live.
            e_before_first_cue = l1(learner.plastic().e());
            assert!(
                e_before_first_cue > 0.0,
                "warmup ticks must accrue eligibility"
            );
        }
        learner.advance(&out.observation.features).expect("advance");
        advances += 1;
        if saw_cue && e_after_first_cue == 0.0 {
            e_after_first_cue = l1(learner.plastic().e());
        }
        lifetime.finish_tick().expect("finish");
        if out.commitment_due {
            lifetime.commit(learner.select_action()).expect("commit");
        }
    }
    assert!(saw_cue);
    assert!(
        e_after_first_cue > 0.0 && e_after_first_cue != e_before_first_cue,
        "warmup→cue boundary must evolve traces, not freeze or clear them"
    );
    assert_eq!(learner.ticks_advanced(), advances);
    assert_eq!(
        advances,
        lifetime.tick(),
        "every tick advanced exactly once"
    );
    assert_eq!((lifetime.commitments(), lifetime.outcomes()), (2, 2));
}

#[test]
fn birth_clears_exactly_with_independent_lifetime_streams() {
    // Same coordinates reproduce identically; distinct lifetime indices
    // draw independent agent streams; every birth is P = E = 0.
    let cfg = continuous_config(4);
    let run = |lifetime_index: u64| {
        run_continuous_lifetime(&cfg, 1, "development", 1, lifetime_index, "continuous")
            .expect("run")
    };
    let a = run(0);
    let b = run(0);
    assert_eq!(a.final_p, b.final_p, "same coordinates reproduce");
    assert_eq!(a.final_e, b.final_e);
    assert_eq!(a.choices, b.choices);
    let c = run(1);
    assert_eq!(c.resets, vec![0]);
    assert!(
        c.final_p != a.final_p || c.final_e != a.final_e,
        "distinct lifetime indices must draw independent agent streams"
    );
}

#[test]
fn finishing_includes_the_final_feedback_transition() {
    let cfg = continuous_config(4);
    let summary = run_continuous_lifetime(&cfg, 1, "development", 1, 0, "continuous").expect("run");
    let last = summary.choices.last().expect("choices");
    // Delay [1,1]: feedback arrives one tick after commitment, and the
    // lifetime closes on the tick after the final feedback.
    assert_eq!(last.feedback_tick, last.commit_tick + 1);
    assert_eq!(summary.ticks, last.feedback_tick + 1);
    assert_eq!(summary.final_last_feedback, Some(last.event_id));
    assert_eq!(last.event_id, 3);
}
