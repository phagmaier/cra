//! Inherited weight contracts (M1-02).
//!
//! - The same seed reproduces the same `W0`/`B`/bias; gate mode is not a
//!   sampling input, so paired gate conditions share weights by
//!   construction; independent outer seeds vary them.
//! - `W0` uses standard deviation `gain / sqrt(in-degree)` on existing
//!   edges (a full-mask fixture distinguishes this from the variance by
//!   ~300x), zero-in-degree rows stay exactly zero, missing edges stay
//!   exactly `0.0`, biases are exactly `0.0`.
//! - `B` is dense with the configured input scale; extreme gains fail
//!   explicitly instead of silently producing infinities.
//! - Validation rejects bad dimensions, constants, streams, and attempts.

use cra::agent::topology::{INIT_STREAM, TopologyError, validate_topology};
use cra::agent::weights::{ParamsError, row_std, sample_inherited, sample_weights_from_mask};
use cra::config::Actor;
use cra::environment::feature_dim;
use cra::rng::{SeedTuple, rng_for};

fn debug_actor() -> Actor {
    Actor {
        neuron_count: 16,
        motor_neurons_per_action: 2,
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

fn init_seed(root: u64, namespace: &str, outer: u64, lifetime: u64) -> SeedTuple {
    SeedTuple::new(root, namespace, outer, lifetime, INIT_STREAM)
}

fn mean_and_pop_var(xs: &[f64]) -> (f64, f64) {
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    (mean, var)
}

// Filled from the ignored probe below; see the golden test.
const PROBE_EDGES: usize = 52;
const PROBE_W0: f64 = -3.928_806_634_372_872;
const PROBE_B: f64 = -1.192_705_711_609_023_4;

#[test]
fn same_seed_reproduces_weights() {
    let actor = debug_actor();
    let seed = init_seed(1, "development", 1, 0);
    let first = sample_inherited(&actor, 8, &seed, 100).expect("accepted");
    let second = sample_inherited(&actor, 8, &seed, 100).expect("resampled");
    assert_eq!(first.params, second.params);
    assert_eq!(first.rejected, second.rejected);
    assert_eq!(first.accepted_attempt, second.accepted_attempt);
}

#[test]
fn paired_conditions_share_weights_by_construction() {
    // Gate mode is not a sampling input: the fixed-condition weights and
    // the targeted-condition weights for one outer seed are two calls with
    // the same init tuple, so they must agree exactly.
    let actor = debug_actor();
    let seed = init_seed(1, "development", 7, 0);
    let fixed = sample_inherited(&actor, 8, &seed, 100).expect("fixed weights");
    let targeted = sample_inherited(&actor, 8, &seed, 100).expect("targeted weights");
    assert_eq!(fixed.params.weights, targeted.params.weights);
    assert_eq!(fixed.params.topology.mask, targeted.params.topology.mask);
}

#[test]
fn independent_outer_seeds_vary_weights() {
    let actor = debug_actor();
    let first =
        sample_inherited(&actor, 8, &init_seed(1, "development", 1, 0), 100).expect("outer 1");
    let second =
        sample_inherited(&actor, 8, &init_seed(1, "development", 2, 0), 100).expect("outer 2");
    assert_ne!(first.params.weights.w0, second.params.weights.w0);
    assert_ne!(
        first.params.weights.input_weights,
        second.params.weights.input_weights
    );
}

#[test]
fn row_scale_matches_std_not_variance_on_full_mask() {
    // N = 200 full mask (minus diagonal): every row has in-degree 199, so
    // all 39_600 W0 entries share std = 0.8 / sqrt(199). A variance-as-std
    // mistake would give sample variance ~1e-5 instead of ~3.2e-3 (~300x).
    // Declared tolerances: SE(mean) ~= 2.9e-4, SE(var) ~= 2.3e-5; bounds
    // sit at ~7 SE.
    assert_eq!(row_std(0.8, 4), 0.4);
    let mut actor = debug_actor();
    actor.neuron_count = 200;
    actor.motor_neurons_per_action = 4;
    actor.edge_probability = 1.0;
    let sampled = sample_inherited(&actor, 14, &init_seed(1, "development", 21, 0), 5)
        .expect("full mask accepted");
    assert_eq!(sampled.accepted_attempt, 0);
    let edges: Vec<f64> = sampled
        .params
        .topology
        .edges
        .iter()
        .map(|&(j, i)| sampled.params.weights.w0[j][i])
        .collect();
    assert_eq!(edges.len(), 200 * 199);
    let expected_var = 0.8_f64.powi(2) / 199.0;
    let (mean, var) = mean_and_pop_var(&edges);
    assert!(mean.abs() < 0.002, "W0 mean {mean} far from 0");
    assert!(
        (var - expected_var).abs() < 0.05 * expected_var,
        "W0 variance {var} far from std-based {expected_var}"
    );
    // B carries the configured input scale: n = 2800, SE(mean) ~= 0.0057,
    // SE(var) ~= 0.0024; bounds sit at ~4-5 SE.
    let flat_b: Vec<f64> = sampled
        .params
        .weights
        .input_weights
        .iter()
        .flatten()
        .copied()
        .collect();
    assert_eq!(flat_b.len(), 200 * 14);
    let (b_mean, b_var) = mean_and_pop_var(&flat_b);
    assert!(b_mean.abs() < 0.025, "B mean {b_mean} far from 0");
    assert!(
        (b_var - 0.09).abs() < 0.012,
        "B variance {b_var} far from 0.09"
    );
}

#[test]
fn zero_in_degree_rows_stay_zero_and_draws_stay_deterministic() {
    // Row 0 has no incoming edges; rows 1-3 do. Direct mask call skips
    // structural acceptance on purpose: the point is the weight rule.
    let mask = vec![
        vec![false, false, false, false],
        vec![false, false, true, false],
        vec![false, true, false, false],
        vec![true, true, false, false],
    ];
    let actor = Actor {
        neuron_count: 4,
        motor_neurons_per_action: 1,
        ..debug_actor()
    };
    let seed = init_seed(1, "development", 4, 0);
    let mut rng_a = rng_for(&seed).expect("rng");
    let mut rng_b = rng_for(&seed).expect("rng");
    let a = sample_weights_from_mask(&mut rng_a, &actor, &mask, 3).expect("weights");
    let b = sample_weights_from_mask(&mut rng_b, &actor, &mask, 3).expect("weights");
    assert_eq!(a, b, "skipped rows must not shift later draws");
    assert!(a.w0[0].iter().all(|&v| v == 0.0));
    assert!(a.w0.iter().flatten().all(|v| v.is_finite()));
    assert_eq!(a.bias, vec![0.0; 4]);
    // Present edges are exactly the mask positions.
    let mut present = 0;
    for (j, row) in mask.iter().enumerate() {
        for (i, &m) in row.iter().enumerate() {
            assert_eq!(a.w0[j][i] == 0.0, !m, "edge ({j}, {i})");
            present += usize::from(m);
        }
    }
    assert_eq!(present, 4);
}

#[test]
fn missing_edges_are_exactly_zero_and_biases_are_zero() {
    let actor = debug_actor();
    let sampled =
        sample_inherited(&actor, 8, &init_seed(1, "development", 1, 0), 100).expect("accepted");
    let topo = &sampled.params.topology;
    let w = &sampled.params.weights;
    assert_eq!(w.bias, vec![0.0; 16]);
    assert_eq!(w.input_weights.len(), 16);
    for row in &w.input_weights {
        assert_eq!(row.len(), 8);
        assert!(row.iter().all(|v| v.is_finite()));
    }
    for (j, row) in topo.mask.iter().enumerate() {
        for (i, &m) in row.iter().enumerate() {
            if m {
                assert!(w.w0[j][i].is_finite());
            } else {
                assert_eq!(w.w0[j][i], 0.0, "missing edge ({j}, {i}) must stay zero");
            }
        }
    }
}

#[test]
fn extreme_gains_fail_explicitly_not_silently() {
    let mut actor = debug_actor();
    actor.recurrent_gain = 1e308;
    actor.input_scale = 1e308;
    assert!(matches!(
        sample_inherited(&actor, 8, &init_seed(1, "development", 1, 0), 100),
        Err(ParamsError::InvalidParams(_))
    ));
}

#[test]
fn validation_rejects_bad_dimensions_params_and_streams() {
    let base = debug_actor();
    let seed = init_seed(1, "development", 1, 0);
    let mut too_wide = base.clone();
    too_wide.motor_neurons_per_action = 9;
    assert!(matches!(
        sample_inherited(&too_wide, 8, &seed, 10),
        Err(ParamsError::Topology(TopologyError::InvalidDimensions(_)))
    ));
    for (label, actor) in [
        (
            "tau_h",
            Actor {
                tau_h: 0.0,
                ..base.clone()
            },
        ),
        (
            "tau_a",
            Actor {
                tau_a: f64::NAN,
                ..base.clone()
            },
        ),
        (
            "motor_tau",
            Actor {
                motor_filter_tau: -1.0,
                ..base.clone()
            },
        ),
        (
            "sigma",
            Actor {
                noise_sigma: 0.0,
                ..base.clone()
            },
        ),
        (
            "gain",
            Actor {
                recurrent_gain: f64::INFINITY,
                ..base.clone()
            },
        ),
        (
            "scale",
            Actor {
                input_scale: f64::NAN,
                ..base.clone()
            },
        ),
    ] {
        assert!(
            matches!(
                sample_inherited(&actor, 8, &seed, 10),
                Err(ParamsError::InvalidParams(_))
            ),
            "{label} must be rejected"
        );
    }
    assert!(matches!(
        sample_inherited(&base, 0, &seed, 10),
        Err(ParamsError::InvalidDimensions(_))
    ));
    assert!(matches!(
        sample_inherited(&base, 8, &seed, 0),
        Err(ParamsError::InvalidDimensions(_))
    ));
    let mut bad_p = base.clone();
    bad_p.edge_probability = 1.5;
    assert!(matches!(
        sample_inherited(&bad_p, 8, &seed, 10),
        Err(ParamsError::Topology(TopologyError::InvalidProbability(_)))
    ));
    let wrong_stream = SeedTuple::new(1, "development", 1, 0, "actor_noise");
    assert!(matches!(
        sample_inherited(&base, 8, &wrong_stream, 10),
        Err(ParamsError::Topology(TopologyError::BadSeed(_)))
    ));
}

#[test]
fn debug_profile_weights_validate_end_to_end() {
    let text = std::fs::read_to_string("configs/debug_stationary.toml").expect("debug profile");
    let cfg: cra::config::Config = toml::from_str(&text).expect("debug parses");
    let actor = cfg.actor.expect("debug has an actor section");
    let input_dim = feature_dim(cfg.environment.cue_count);
    assert_eq!(input_dim, 8);
    let seed = SeedTuple::new(
        cfg.seeds.root_seed,
        &cfg.seeds.namespace,
        cfg.seeds.outer_seed,
        0,
        INIT_STREAM,
    );
    let sampled = sample_inherited(&actor, input_dim, &seed, 100).expect("accepted");
    validate_topology(&sampled.params.topology).expect("mask validates");
    assert_eq!(sampled.params.input_dim(), 8);
    assert_eq!(sampled.params.weights.bias, vec![0.0; 16]);
    assert!(
        sampled
            .params
            .weights
            .w0
            .iter()
            .flatten()
            .all(|v| v.is_finite())
            && sampled
                .params
                .weights
                .input_weights
                .iter()
                .flatten()
                .all(|v| v.is_finite())
    );
    for (j, row) in sampled.params.topology.mask.iter().enumerate() {
        assert!(!row[j]);
    }
}

#[test]
fn golden_inherited_values_pin_the_draw_sequence() {
    // Change-detector, not a derivation proof: any RNG, ordering, or scale
    // change moves these sums. Update only with a documented derivation
    // change (spec 20.5); never to force a pass.
    let sampled = sample_inherited(&debug_actor(), 8, &init_seed(1, "development", 1, 0), 100)
        .expect("accepted");
    assert_eq!(sampled.params.topology.edge_count(), PROBE_EDGES);
    let w0_sum: f64 = sampled.params.weights.w0.iter().flatten().sum();
    let b_sum: f64 = sampled.params.weights.input_weights.iter().flatten().sum();
    assert!((w0_sum - PROBE_W0).abs() < 1e-12, "w0 sum {w0_sum}");
    assert!((b_sum - PROBE_B).abs() < 1e-12, "B sum {b_sum}");
}

#[test]
#[ignore]
fn probe_golden_values() {
    let sampled = sample_inherited(&debug_actor(), 8, &init_seed(1, "development", 1, 0), 100)
        .expect("accepted");
    let w0_sum: f64 = sampled.params.weights.w0.iter().flatten().sum();
    let b_sum: f64 = sampled.params.weights.input_weights.iter().flatten().sum();
    println!(
        "PROBE edges={} accepted={}",
        sampled.params.topology.edge_count(),
        sampled.accepted_attempt
    );
    println!("PROBE w0_sum={w0_sum:.17e} b_sum={b_sum:.17e}");
}
