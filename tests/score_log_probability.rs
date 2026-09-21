//! M2-02 / spec 17.4: differentiate log density at a FIXED membrane sample.
//! No RNG, learning, trajectory gradient or statistical acceptance is involved.

use cra::agent::actor::{ActorState, leak_alpha};
use cra::agent::score::conditional_score;
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::Actor;

// Declared before execution. Central differences of this quadratic log density
// have only floating-point error; smaller eps amplifies subtraction roundoff.
const EPSILONS: [f64; 4] = [1e-5, 3e-6, 1e-6, 3e-7];
const ABS_TOL: f64 = 2e-8;
const REL_TOL: f64 = 2e-7;
const INPUT: [f64; 2] = [0.7, -0.2];
const XI: [f64; 4] = [0.3, -1.1, 0.0, 2.0];

/// Independent scalar Gaussian log density, including normalization.
/// The other receivers' densities are constant for a single perturbed edge.
fn gaussian_log_probability(sample: f64, mean: f64, sigma: f64) -> f64 {
    assert!(sample.is_finite() && mean.is_finite() && sigma.is_finite() && sigma > 0.0);
    let residual = (sample - mean) / sigma;
    -0.5 * residual * residual - sigma.ln() - 0.5 * std::f64::consts::TAU.ln()
}

fn fixture(tau_h: f64, sigma: f64) -> (Actor, InheritedParams, ActorState) {
    let actor = Actor {
        neuron_count: 4,
        motor_neurons_per_action: 1,
        edge_probability: 1.0,
        self_edges: false,
        recurrent_gain: 0.8,
        input_scale: 0.3,
        tau_h,
        tau_a: 100.0,
        adaptation_strength: 0.2,
        noise_sigma: sigma,
        motor_filter_tau: 3.0,
    };
    let mask = (0..4).map(|j| (0..4).map(|i| i != j).collect()).collect();
    let params = InheritedParams {
        topology: topology_from_mask(4, 1, 1.0, false, mask, "M2-02".to_owned()).unwrap(),
        weights: Weights {
            w0: vec![
                vec![0.0, -0.4, 0.6, 0.2],
                vec![0.3, 0.0, -0.1, 0.5],
                vec![-0.7, 0.2, 0.0, -0.3],
                vec![0.1, -0.5, 0.4, 0.0],
            ],
            input_weights: vec![
                vec![0.2, -0.3],
                vec![0.4, 0.1],
                vec![-0.2, 0.5],
                vec![0.3, 0.2],
            ],
            bias: vec![0.1, -0.2, 0.3, -0.4],
        },
    };
    params.validate(&actor, INPUT.len()).unwrap();
    // Zero, signed and nearly saturated old activities; adaptation and sensory
    // drives are nonzero so this checks the full production transition mean.
    let old =
        ActorState::from_state(vec![0.0, 0.7, -0.4, 12.0], vec![0.1, -0.3, 0.2, 0.4]).unwrap();
    (actor, params, old)
}

/// Evaluate the production conditional mean by injecting zero perturbations.
/// Always clone the SAME old state; these transitions never replace the saved
/// sampled state and do not construct a new noisy observation.
fn conditional_mean(actor: &Actor, params: &InheritedParams, old: &ActorState) -> Vec<f64> {
    let mut mean_state = old.clone();
    mean_state
        .step_with_perturbations(actor, params, &INPUT, &[0.0; 4])
        .unwrap();
    mean_state.h().to_vec()
}

#[test]
fn log_probability_matches_known_gaussian_values() {
    let cases = [
        (0.0, 0.0, 1.0, -0.918_938_533_204_672_7),
        (1.0, 0.0, 1.0, -1.418_938_533_204_672_7),
        (0.7, 0.3, 0.4, -0.502_647_801_330_517_7),
    ];
    for (sample, mean, sigma, expected) in cases {
        assert!((gaussian_log_probability(sample, mean, sigma) - expected).abs() < 1e-14);
    }
}

#[test]
fn fixed_sample_log_probability_derivatives_match_scores() {
    let mut comparisons = 0;
    for tau_h in [0.5, 5.0, 100.0] {
        for sigma in [0.05, 0.4] {
            let (actor, params, old) = fixture(tau_h, sigma);
            let saved_old = old.clone();
            let saved_params = params.clone();
            let mut sampled = old.clone();
            sampled
                .step_with_perturbations(&actor, &params, &INPUT, &XI)
                .unwrap();
            // Exactly one unperturbed-weight sample, held fixed for every eps
            // and edge. Only conditional means change in the +/- evaluations.
            let h_new = sampled.h().to_vec();
            let mut max_error = 0.0_f64;
            for j in 0..4 {
                for i in 0..4 {
                    if !params.topology.mask[j][i] {
                        continue;
                    }
                    let analytical =
                        conditional_score(leak_alpha(tau_h), old.r()[i], XI[j], sigma).unwrap();
                    for eps in EPSILONS {
                        let mut plus = params.clone();
                        let mut minus = params.clone();
                        plus.weights.w0[j][i] += eps;
                        minus.weights.w0[j][i] -= eps;
                        let mu_plus = conditional_mean(&actor, &plus, &old);
                        let mu_minus = conditional_mean(&actor, &minus, &old);
                        let logp_plus = gaussian_log_probability(h_new[j], mu_plus[j], sigma);
                        let logp_minus = gaussian_log_probability(h_new[j], mu_minus[j], sigma);
                        let numerical = (logp_plus - logp_minus) / (2.0 * eps);
                        let error = (numerical - analytical).abs();
                        assert!(
                            error <= ABS_TOL + REL_TOL * analytical.abs(),
                            "tau={tau_h} sigma={sigma} edge={i}->{j} eps={eps}: analytical={analytical}, numerical={numerical}, error={error}"
                        );
                        // Perturbing one incoming edge cannot change another
                        // receiver's conditional mean (old-state semantics).
                        for k in 0..4 {
                            if k != j {
                                assert_eq!(mu_plus[k].to_bits(), mu_minus[k].to_bits());
                            }
                        }
                        max_error = max_error.max(error);
                        comparisons += 1;
                    }
                }
            }
            assert_eq!(old, saved_old);
            assert_eq!(params, saved_params);
            assert_eq!(sampled.h(), h_new);
            println!("tau_h={tau_h} sigma={sigma}: 48 comparisons, max_abs_error={max_error:.12e}");
        }
    }
    assert_eq!(comparisons, 288);
}

#[test]
fn moving_sample_with_perturbed_mean_is_not_the_conditional_derivative() {
    // Deliberately WRONG control: carry the same xi into a fresh sample for
    // each weight. The standardized residual is then constant and logp has
    // zero derivative, unlike the desired fixed-observation score.
    let (alpha, activity, weight, sigma, xi) = (0.2, 0.7, 0.3, 0.4, 1.2);
    let saved_sample = alpha * weight * activity + sigma * xi;
    let analytical = conditional_score(alpha, activity, xi, sigma).unwrap();
    assert!((analytical - 0.42).abs() < 1e-14);
    for eps in EPSILONS {
        let plus_mean = alpha * (weight + eps) * activity;
        let minus_mean = alpha * (weight - eps) * activity;
        let fixed = (gaussian_log_probability(saved_sample, plus_mean, sigma)
            - gaussian_log_probability(saved_sample, minus_mean, sigma))
            / (2.0 * eps);
        let moving = (gaussian_log_probability(plus_mean + sigma * xi, plus_mean, sigma)
            - gaussian_log_probability(minus_mean + sigma * xi, minus_mean, sigma))
            / (2.0 * eps);
        assert!((fixed - analytical).abs() <= ABS_TOL + REL_TOL * analytical.abs());
        assert!(moving.abs() < ABS_TOL);
        assert!((moving - analytical).abs() > 0.4);
    }
}
