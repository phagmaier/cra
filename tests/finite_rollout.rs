//! M2-04 restricted diagnostic contracts; M2-05 owns recurrent Monte Carlo.

use cra::agent::actor::{ActorState, leak_alpha};
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::Actor;
use cra::experiments::finite_rollout::{FiniteRollout, MODE, RolloutError, RolloutPhase};
use cra::rng::{SeedTuple, rng_for};

fn fixture() -> (Actor, InheritedParams) {
    let actor = Actor {
        neuron_count: 2,
        motor_neurons_per_action: 1,
        edge_probability: 1.0,
        self_edges: false,
        recurrent_gain: 0.8,
        input_scale: 0.3,
        tau_h: 1.0 / std::f64::consts::LN_2,
        tau_a: 100.0,
        adaptation_strength: 0.0,
        noise_sigma: 0.1,
        motor_filter_tau: 3.0,
    };
    let params = InheritedParams {
        topology: topology_from_mask(
            2,
            1,
            1.0,
            false,
            vec![vec![false, true], vec![true, false]],
            "M2-04".to_owned(),
        )
        .unwrap(),
        weights: Weights {
            w0: vec![vec![0.0; 2]; 2],
            input_weights: vec![vec![0.0]; 2],
            bias: vec![0.0; 2],
        },
    };
    (actor, params)
}

fn new_run(horizon: u64, baseline: f64) -> FiniteRollout {
    let (actor, params) = fixture();
    FiniteRollout::new(actor, params, horizon, baseline).unwrap()
}

fn golden_perturbations() -> [[f64; 2]; 3] {
    [
        [0.2_f64.atanh() / 0.1, -0.4_f64.atanh() / 0.1],
        [-0.2, 0.4],
        [0.0, 0.0],
    ]
}

fn complete_golden(run: &mut FiniteRollout) {
    for xi in golden_perturbations() {
        run.step_with_perturbations(&[0.0], &xi).unwrap();
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}

#[test]
fn golden_sum_has_no_decay_and_only_one_terminal_update() {
    let mut run = new_run(3, 0.25);
    let frozen = run.parameters().clone();
    let xi = golden_perturbations();
    run.step_with_perturbations(&[0.0], &xi[0]).unwrap();
    assert_eq!(run.score_sums(), vec![vec![0.0; 2]; 2]); // old activity was zero
    close(run.state().r()[0], 0.2);
    close(run.state().r()[1], -0.4);
    run.step_with_perturbations(&[0.0], &xi[1]).unwrap();
    close(run.score_sums()[1][0], 0.4);
    close(run.score_sums()[0][1], 0.4);
    let saved_sum = run.score_sums().to_vec();
    run.step_with_perturbations(&[0.0], &xi[2]).unwrap();
    assert_eq!(run.score_sums(), saved_sum); // zero increment retains EXACT sum
    assert_eq!(run.parameters(), &frozen);
    assert_eq!(run.baseline(), 0.25);

    let terminal_state = run.state().clone();
    let result = run.finish(1.0, Some(10.0)).unwrap();
    assert_eq!(result.mode, MODE);
    assert_eq!(result.steps, 3);
    for (j, i) in [(0, 1), (1, 0)] {
        close(result.reward_weighted_scores[j][i], 0.3);
        close(result.terminal_weights.as_ref().unwrap()[j][i], 3.0); // no main plastic-bound clamp
    }
    for j in 0..2 {
        assert_eq!(result.score_sums[j][j], 0.0);
        assert_eq!(result.reward_weighted_scores[j][j], 0.0);
        assert_eq!(result.terminal_weights.as_ref().unwrap()[j][j], 0.0);
    }
    assert_eq!(run.parameters(), &frozen); // terminal copy never overwrites W0
    assert_eq!(run.baseline(), 0.25);
    assert_eq!(run.state(), &terminal_state);
    assert_eq!(run.phase(), RolloutPhase::Finished);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    if let Some(path) = std::env::var_os("CRA_M2_ROLLOUT_EVIDENCE") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &serde_json::json!({
            "configuration": {"actor": fixture().0, "parameters": frozen, "horizon": 3, "baseline": 0.25, "terminal_reward": 1.0, "terminal_eta": 10.0},
            "perturbations": xi, "initial_state": "zero independent of weights", "result": result,
            "rng": "none: deterministic injected perturbations", "reset": "only between completed rollouts", "decay": "none"
        })).unwrap();
    }
}

#[test]
fn lifecycle_rejects_online_updates_extra_ticks_and_duplicate_finish() {
    let mut run = new_run(3, 0.25);
    for xi in golden_perturbations() {
        assert!(matches!(
            run.finish(1.0, Some(0.1)),
            Err(RolloutError::InvalidPhase(_))
        ));
        assert!(run.reset_between_rollouts().is_err());
        assert_eq!(run.baseline(), 0.25);
        assert_eq!(run.parameters().weights.w0, vec![vec![0.0; 2]; 2]);
        run.step_with_perturbations(&[0.0], &xi).unwrap();
    }
    let seed = SeedTuple::new(1, "development", 204, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    let saved_rng = rng.clone();
    assert!(run.step(&[0.0], &mut rng).is_err());
    assert_eq!(rng, saved_rng);
    let result = run.finish(1.0, Some(10.0)).unwrap();
    assert!(run.finish(0.0, Some(10.0)).is_err());
    assert!(run.step(&[0.0], &mut rng).is_err());
    assert_eq!(rng, saved_rng);
    run.reset_between_rollouts().unwrap();
    assert_eq!(run.state(), &ActorState::new(2).unwrap());
    assert_eq!(run.score_sums(), vec![vec![0.0; 2]; 2]);
    assert_eq!(run.steps(), 0);
    assert_eq!(run.parameters().weights.w0, vec![vec![0.0; 2]; 2]);
    complete_golden(&mut run);
    let repeated = run.finish(1.0, Some(10.0)).unwrap();
    assert_eq!(result.score_sums, repeated.score_sums);
    assert_eq!(result.terminal_weights, repeated.terminal_weights);
    assert_eq!(repeated.baseline, 0.25);
}

#[test]
fn no_update_and_zero_delta_modes_leave_weights_unchanged() {
    let mut run = new_run(3, 0.25);
    complete_golden(&mut run);
    let result = run.finish(0.25, None).unwrap();
    assert!(result.terminal_weights.is_none());
    assert_eq!(result.reward_weighted_scores, vec![vec![0.0; 2]; 2]);
    run.reset_between_rollouts().unwrap();
    complete_golden(&mut run);
    let result = run.finish(0.25, Some(10.0)).unwrap();
    assert_eq!(
        result.terminal_weights.unwrap(),
        run.parameters().weights.w0
    );
}

#[test]
fn stochastic_rollout_matches_frozen_actor_and_draw_schedule() {
    let (mut actor, mut params) = fixture();
    actor.adaptation_strength = 0.1;
    params.weights.w0 = vec![vec![0.0, -0.4], vec![0.7, 0.0]];
    params.weights.input_weights = vec![vec![0.2], vec![-0.3]];
    let mut run = FiniteRollout::new(actor.clone(), params.clone(), 16, 0.37).unwrap();
    let mut reference = ActorState::new(2).unwrap();
    let seed = SeedTuple::new(1, "development", 204, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    let mut reference_rng = rng.clone();
    let mut sum = vec![vec![0.0; 2]; 2];
    for tick in 0..16 {
        let input = [if tick % 2 == 0 { 0.7 } else { -0.2 }];
        let old_r = reference.r().to_vec();
        reference
            .step(&actor, &params, &input, &mut reference_rng)
            .unwrap();
        run.step(&input, &mut rng).unwrap();
        for &(j, i) in &params.topology.edges {
            sum[j][i] += leak_alpha(actor.tau_h) * old_r[i] * reference.last_perturbations()[j]
                / actor.noise_sigma;
            close(run.score_sums()[j][i], sum[j][i]);
        }
        assert_eq!(run.state(), &reference);
        assert_eq!(rng, reference_rng);
        assert_eq!(run.baseline(), 0.37);
        assert_eq!(run.parameters(), &params);
    }
    let result = run.finish(0.0, None).unwrap();
    for &(j, i) in &params.topology.edges {
        close(result.reward_weighted_scores[j][i], -0.37 * sum[j][i]);
    }
}

#[test]
fn initial_state_is_independent_of_weights_and_finite_membranes_are_not_clipped() {
    let (actor, mut params) = fixture();
    params.weights.w0[1][0] = 50.0;
    params.weights.bias[0] = 1e6;
    let mut run = FiniteRollout::new(actor, params, 1, 0.5).unwrap();
    assert_eq!(run.state(), new_run(1, 0.5).state());
    run.step_with_perturbations(&[0.0], &[0.0, 0.0]).unwrap();
    close(run.state().h()[0], 5e5); // exceeds production watchdog, no clipping in this diagnostic
    assert_eq!(run.state().r()[0], 1.0);
    run.finish(1.0, None).unwrap();
}

#[test]
fn invalid_configuration_and_inputs_are_explicit_errors() {
    let (actor, params) = fixture();
    for (horizon, baseline) in [(0, 0.5), (1, f64::NAN), (1, f64::INFINITY)] {
        assert!(FiniteRollout::new(actor.clone(), params.clone(), horizon, baseline).is_err());
    }
    for sigma in [0.0, -0.1, f64::NAN, f64::INFINITY] {
        let mut invalid = actor.clone();
        invalid.noise_sigma = sigma;
        assert!(FiniteRollout::new(invalid, params.clone(), 1, 0.5).is_err());
    }
    let mut malformed = params.clone();
    malformed.weights.w0[0].clear();
    assert!(FiniteRollout::new(actor.clone(), malformed, 1, 0.5).is_err());
    let mut nonfinite = params;
    nonfinite.weights.w0[1][0] = f64::NAN;
    assert!(FiniteRollout::new(actor, nonfinite, 1, 0.5).is_err());

    let mut run = new_run(1, 0.25);
    let seed = SeedTuple::new(1, "development", 204, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    let saved_rng = rng.clone();
    assert!(run.step(&[], &mut rng).is_err());
    assert!(run.step(&[f64::NAN], &mut rng).is_err());
    assert!(run.step_with_perturbations(&[0.0], &[1.0]).is_err());
    assert!(
        run.step_with_perturbations(&[0.0], &[f64::INFINITY, 0.0])
            .is_err()
    );
    assert_eq!(rng, saved_rng);
    assert_eq!(run.state(), &ActorState::new(2).unwrap());
    assert_eq!(run.steps(), 0);
    run.step_with_perturbations(&[0.0], &[0.0, 0.0]).unwrap();
    for (reward, eta) in [
        (f64::NAN, None),
        (1.0, Some(-0.1)),
        (1.0, Some(f64::INFINITY)),
    ] {
        assert!(run.finish(reward, eta).is_err());
        assert_eq!(run.phase(), RolloutPhase::Collecting);
    }
    run.finish(1.0, None).unwrap();
}

fn assert_failed(run: &mut FiniteRollout) {
    assert_eq!(run.phase(), RolloutPhase::Failed);
    assert!(run.finish(0.0, None).is_err());
    assert!(run.step_with_perturbations(&[0.0], &[0.0, 0.0]).is_err());
    assert!(run.reset_between_rollouts().is_err());
}

#[test]
fn nonfinite_transitions_and_scores_cannot_be_finalized_or_retried() {
    let (actor, mut params) = fixture();
    params.weights.input_weights[0][0] = f64::MAX;
    let mut run = FiniteRollout::new(actor, params, 2, 0.5).unwrap();
    assert!(matches!(
        run.step_with_perturbations(&[2.0], &[0.0, 0.0]),
        Err(RolloutError::Actor(_))
    ));
    assert_failed(&mut run);
    let (mut actor, mut params) = fixture();
    actor.noise_sigma = 1e-308;
    params.weights.bias.fill(1000.0);
    let mut run = FiniteRollout::new(actor, params, 2, 0.5).unwrap();
    run.step_with_perturbations(&[0.0], &[0.0, 0.0]).unwrap();
    assert!(matches!(
        run.step_with_perturbations(&[0.0], &[10.0, 10.0]),
        Err(RolloutError::Score(_))
    ));
    assert_failed(&mut run);
}

#[test]
fn accumulation_and_terminal_overflow_are_explicit_failures() {
    let (mut actor, mut params) = fixture();
    actor.noise_sigma = 1e-308;
    params.weights.bias.fill(1000.0);
    let mut run = FiniteRollout::new(actor, params, 6, 0.5).unwrap();
    run.step_with_perturbations(&[0.0], &[0.0, 0.0]).unwrap();
    for _ in 0..3 {
        run.step_with_perturbations(&[0.0], &[1.0, 1.0]).unwrap();
    }
    assert!(matches!(
        run.step_with_perturbations(&[0.0], &[1.0, 1.0]),
        Err(RolloutError::NonFinite("score sum"))
    ));
    assert_failed(&mut run);

    let mut run = new_run(3, 0.25);
    complete_golden(&mut run);
    assert!(matches!(
        run.finish(f64::MAX, Some(f64::MAX)),
        Err(RolloutError::NonFinite("terminal weight"))
    ));
    assert_failed(&mut run);
    let mut run = new_run(1, -f64::MAX);
    run.step_with_perturbations(&[0.0], &[0.0, 0.0]).unwrap();
    assert!(matches!(
        run.finish(f64::MAX, None),
        Err(RolloutError::NonFinite("reward minus baseline"))
    ));
    assert_failed(&mut run);

    let mut run = new_run(2, 0.5);
    run.step_with_perturbations(&[0.0], &golden_perturbations()[0])
        .unwrap();
    run.step_with_perturbations(&[0.0], &[-2.0, 4.0]).unwrap();
    assert!(matches!(
        run.finish(f64::MAX, None),
        Err(RolloutError::NonFinite("reward-weighted score"))
    ));
    assert_failed(&mut run);
}
