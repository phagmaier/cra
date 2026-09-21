//! Inherited topology contracts (M1-01).
//!
//! - The same `init` seed reproduces the same mask, motor pools, and edge
//!   order; gate mode is not a sampling input, so paired gate conditions
//!   share topology by construction.
//! - Motor pools are fixed, disjoint, and cover the last `2m` indices.
//! - Missing edges stay absent; the edge list is receiver-grouped and
//!   stable; self-edges obey the configured flag.
//! - Hand-built small graphs separate acceptable structure (cycle plus a
//!   path to each motor pool) from each rejection reason.
//! - Selection is structural only: the sampler takes no reward, hidden
//!   state, or fitness input, and rejections carry explicit logged reasons.

use cra::agent::topology::{
    INIT_STREAM, RejectionReason, TopologyError, check_structure, edge_list, has_cycle,
    motor_pools, pool_reachable_from_sources, sample_mask, sample_topology, topology_from_mask,
    validate_topology,
};
use cra::config::Actor;
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

/// N = 4, m = 1 fixture: motor0 = [2], motor1 = [3], sources = [0, 1].
fn mask_from_edges(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<bool>> {
    let mut mask = vec![vec![false; n]; n];
    for &(j, i) in edges {
        mask[j][i] = true;
    }
    mask
}

#[test]
fn same_init_reproduces_same_mask_and_order() {
    let actor = debug_actor();
    let seed = init_seed(1, "development", 1, 0);
    let first = sample_topology(&actor, &seed, 100).expect("debug mask accepted");
    let second = sample_topology(&actor, &seed, 100).expect("resample accepted");
    assert_eq!(first.topology.mask, second.topology.mask);
    assert_eq!(first.topology.edges, second.topology.edges);
    assert_eq!(first.topology.motor0, second.topology.motor0);
    assert_eq!(first.topology.motor1, second.topology.motor1);
    assert_eq!(first.topology.init_seed_hex, second.topology.init_seed_hex);
    assert_eq!(first.accepted_attempt, second.accepted_attempt);
    // Stable receiver-grouped order.
    let mut sorted = second.topology.edges.clone();
    sorted.sort();
    assert_eq!(second.topology.edges, sorted);
    assert_eq!(second.topology.edges, edge_list(&second.topology.mask));
}

#[test]
fn paired_gate_conditions_share_topology_by_construction() {
    // Gate mode is not a sampling input: the fixed-condition mask and the
    // targeted-condition mask for one outer seed are two calls with the
    // same init tuple, so they must agree exactly.
    let actor = debug_actor();
    let seed = init_seed(1, "development", 7, 0);
    let fixed = sample_topology(&actor, &seed, 100).expect("fixed mask");
    let targeted = sample_topology(&actor, &seed, 100).expect("targeted mask");
    assert_eq!(fixed.topology.mask, targeted.topology.mask);
    assert_eq!(fixed.topology.edges, targeted.topology.edges);
}

#[test]
fn independent_outer_seeds_vary_the_mask() {
    let actor = debug_actor();
    let first =
        sample_topology(&actor, &init_seed(1, "development", 1, 0), 100).expect("outer 1 accepted");
    let second =
        sample_topology(&actor, &init_seed(1, "development", 2, 0), 100).expect("outer 2 accepted");
    assert_ne!(
        first.topology.mask, second.topology.mask,
        "independent outer seeds must not share one lucky reservoir"
    );
}

#[test]
fn motor_pools_are_fixed_disjoint_and_cover_last_indices() {
    let (m0, m1) = motor_pools(16, 2).expect("debug pools");
    assert_eq!(m0, vec![12, 13]);
    assert_eq!(m1, vec![14, 15]);
    let (m0, m1) = motor_pools(60, 4).expect("main pools");
    assert_eq!(m0, vec![52, 53, 54, 55]);
    assert_eq!(m1, vec![56, 57, 58, 59]);
    // Disjointness across sizes.
    for (n, m) in [(4, 1), (16, 2), (60, 4)] {
        let (m0, m1) = motor_pools(n, m).expect("pools");
        assert_eq!(m0.len(), m);
        assert_eq!(m1.len(), m);
        let mut all = m0.clone();
        all.extend(m1.clone());
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 2 * m, "pools must be disjoint");
    }
}

#[test]
fn invalid_motor_dimensions_rejected() {
    assert!(motor_pools(3, 2).is_err());
    assert!(motor_pools(16, 0).is_err());
    assert!(motor_pools(0, 1).is_err());
}

#[test]
fn missing_edges_stay_absent_and_counts_agree() {
    let actor = debug_actor();
    let sampled =
        sample_topology(&actor, &init_seed(1, "development", 1, 0), 100).expect("accepted");
    let topo = &sampled.topology;
    let mask_count: usize = topo.mask.iter().flatten().filter(|&&b| b).count();
    assert_eq!(topo.edge_count(), mask_count);
    assert_eq!(topo.edges.len(), mask_count);
    for (j, row) in topo.mask.iter().enumerate() {
        for (i, &present) in row.iter().enumerate() {
            if present {
                assert!(
                    topo.edges.contains(&(j, i)),
                    "present edge ({j}, {i}) missing from edge list"
                );
                assert!(topo.has_edge(j, i));
            } else {
                assert!(
                    !topo.edges.contains(&(j, i)),
                    "absent edge ({j}, {i}) leaked into edge list"
                );
                assert!(!topo.has_edge(j, i));
            }
        }
        let row_count: usize = topo.mask[j].iter().filter(|&&b| b).count();
        assert_eq!(topo.in_degree(j), row_count);
    }
}

#[test]
fn self_edges_obey_the_configured_flag() {
    // Disabled: diagonal never present, even at p = 1.
    let mut rng = rng_for(&init_seed(9, "development", 9, 0)).expect("rng");
    let mask = sample_mask(&mut rng, 6, 1.0, false).expect("full mask");
    for (j, row) in mask.iter().enumerate() {
        assert!(!row[j], "self-edge ({j}, {j}) must stay absent");
    }
    // Enabled at p = 1: every pair present, including the diagonal.
    let mut rng = rng_for(&init_seed(9, "development", 9, 0)).expect("rng");
    let mask = sample_mask(&mut rng, 4, 1.0, true).expect("full mask");
    assert!(mask.iter().flatten().all(|&b| b));
    // A hand-built diagonal is rejected when the flag is off.
    let mut bad = vec![vec![false; 3]; 3];
    bad[1][1] = true;
    assert!(matches!(
        topology_from_mask(3, 1, 0.5, false, bad, "hex".to_owned()),
        Err(TopologyError::InvalidDimensions(_))
    ));
    // Sampled masks with the flag off never carry a diagonal.
    let actor = debug_actor();
    let sampled =
        sample_topology(&actor, &init_seed(1, "development", 3, 0), 100).expect("accepted");
    for (j, row) in sampled.topology.mask.iter().enumerate() {
        assert!(!row[j]);
    }
}

#[test]
fn probability_boundaries_behave() {
    let mut rng = rng_for(&init_seed(1, "development", 5, 0)).expect("rng");
    let empty = sample_mask(&mut rng, 4, 0.0, false).expect("p=0");
    assert!(empty.iter().flatten().all(|&b| !b));
    assert!(sample_mask(&mut rng, 4, f64::NAN, false).is_err());
    assert!(sample_mask(&mut rng, 4, 1.5, false).is_err());
}

#[test]
fn hand_built_good_graph_is_accepted() {
    // 0 -> 2 -> 0 is a cycle; 0 reaches M0 = [2], 1 reaches M1 = [3].
    let mask = mask_from_edges(4, &[(2, 0), (0, 2), (3, 1)]);
    assert!(has_cycle(&mask));
    assert!(pool_reachable_from_sources(&mask, &[0, 1], &[2]));
    assert!(pool_reachable_from_sources(&mask, &[0, 1], &[3]));
    let topo =
        topology_from_mask(4, 1, 0.5, false, mask, "fixture".to_owned()).expect("well-formed");
    assert_eq!(topo.motor0, vec![2]);
    assert_eq!(topo.motor1, vec![3]);
    validate_topology(&topo).expect("acceptable structure");
}

#[test]
fn hand_built_bad_graphs_yield_exact_reasons() {
    // Acyclic but fully reaching: only the cycle check fires.
    let dag = mask_from_edges(4, &[(2, 0), (3, 1), (3, 0)]);
    assert!(!has_cycle(&dag));
    assert_eq!(
        check_structure(&dag, 4, 1, &[2], &[3]),
        vec![RejectionReason::NoCycle]
    );
    // Cycle present, M1 isolated.
    let no_m1 = mask_from_edges(4, &[(2, 0), (2, 1), (0, 2)]);
    assert!(has_cycle(&no_m1));
    assert_eq!(
        check_structure(&no_m1, 4, 1, &[2], &[3]),
        vec![RejectionReason::Motor1Unreachable]
    );
    // Cycle present, M0 isolated.
    let no_m0 = mask_from_edges(4, &[(3, 0), (3, 1), (0, 3)]);
    assert!(has_cycle(&no_m0));
    assert_eq!(
        check_structure(&no_m0, 4, 1, &[2], &[3]),
        vec![RejectionReason::Motor0Unreachable]
    );
    // Empty: every check fires.
    let empty = vec![vec![false; 4]; 4];
    assert_eq!(
        check_structure(&empty, 4, 1, &[2], &[3]),
        vec![
            RejectionReason::NoCycle,
            RejectionReason::Motor0Unreachable,
            RejectionReason::Motor1Unreachable,
        ]
    );
    // validate_topology surfaces the same reasons as an explicit error.
    let topo =
        topology_from_mask(4, 1, 0.5, false, empty, "fixture".to_owned()).expect("well-formed");
    assert!(matches!(
        validate_topology(&topo),
        Err(TopologyError::StructurallyRejected { .. })
    ));
}

#[test]
fn rejected_samples_carry_logged_reasons_and_exhaustion_is_explicit() {
    // p = 0 can never satisfy any check: every attempt logs all reasons.
    let mut actor = debug_actor();
    actor.neuron_count = 8;
    actor.motor_neurons_per_action = 1;
    actor.edge_probability = 0.0;
    let err = sample_topology(&actor, &init_seed(1, "development", 1, 0), 3)
        .expect_err("empty masks never validate");
    match err {
        TopologyError::Exhausted { attempts, log } => {
            assert_eq!(attempts, 3);
            assert_eq!(log.len(), 3);
            for (k, record) in log.iter().enumerate() {
                assert_eq!(record.attempt, k as u32);
                assert_eq!(record.edge_count, 0);
                assert_eq!(
                    record.reasons,
                    vec![
                        RejectionReason::NoCycle,
                        RejectionReason::Motor0Unreachable,
                        RejectionReason::Motor1Unreachable,
                    ]
                );
            }
        }
        other => panic!("expected exhaustion, got {other:?}"),
    }
}

#[test]
fn non_init_streams_and_bad_attempts_rejected() {
    let actor = debug_actor();
    let wrong = SeedTuple::new(1, "development", 1, 0, "actor_noise");
    assert!(matches!(
        sample_topology(&actor, &wrong, 10),
        Err(TopologyError::BadSeed(_))
    ));
    let bad_ns = SeedTuple::new(1, "staging", 1, 0, INIT_STREAM);
    assert!(matches!(
        sample_topology(&actor, &bad_ns, 10),
        Err(TopologyError::BadSeed(_))
    ));
    assert!(matches!(
        sample_topology(&actor, &init_seed(1, "development", 1, 0), 0),
        Err(TopologyError::InvalidDimensions(_))
    ));
}

#[test]
fn debug_profile_topology_validates_end_to_end() {
    let text = std::fs::read_to_string("configs/debug_stationary.toml").expect("debug profile");
    let cfg: cra::config::Config = toml::from_str(&text).expect("debug parses");
    let actor = cfg.actor.expect("debug has an actor section");
    let seed = SeedTuple::new(
        cfg.seeds.root_seed,
        &cfg.seeds.namespace,
        cfg.seeds.outer_seed,
        0,
        INIT_STREAM,
    );
    let sampled = sample_topology(&actor, &seed, 100).expect("debug mask accepted");
    validate_topology(&sampled.topology).expect("accepted mask validates");
    // No self-edges in the shipped initial profile.
    for (j, row) in sampled.topology.mask.iter().enumerate() {
        assert!(!row[j]);
    }
    assert!(!sampled.topology.init_seed_hex.is_empty());
}
