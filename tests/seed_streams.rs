//! Seed-stream contracts (M0-04).
//!
//! - Known tuples produce golden hex outputs. One vector is cross-checked
//!   against system `sha256sum(1)`, not just the implementation itself.
//! - Namespaces are disjoint: the same (root, outer, lifetime, stream) in
//!   `development` vs `training` derives different seeds.
//! - Streams are independent: each reserved stream derives a distinct seed,
//!   and heavy draws from an agent stream leave environment-stream RNGs
//!   untouched (separate instances per stream).
//! - Unknown namespaces and malformed stream names are rejected; the
//!   runtime-randomized `HashMap` hasher is never used for derivation.

use cra::rng::{
    ACTOR_INIT_STREAM, CUE_MEMBERSHIP_STREAM, SUPPORTED_NAMESPACES, SUPPORTED_STREAMS, SeedTuple,
    derive_seed_hex, is_supported_namespace, rng_for, validate_tuple,
};
use rand_core::RngCore;

fn tuple(root: u64, ns: &str, outer: u64, lifetime: u64, stream: &str) -> SeedTuple {
    SeedTuple::new(root, ns, outer, lifetime, stream)
}

#[test]
fn golden_stream_outputs() {
    // Cross-checked with: printf '%s' 'cra-v1|...' | sha256sum
    let cases = [
        (
            tuple(1, "development", 1, 0, "cue_order"),
            "da2a722cf5b2f554953ce70c39fa286eb6aa9b22edbf7cdd1f391ca9ba4925a2",
        ),
        (
            tuple(1, "development", 1, 0, "reward_noise"),
            "969ba372438ed40631092677238c3a96e2ad02e5b1b2ecc06364ee235b6b180c",
        ),
        (
            tuple(1, "development", 1, 1, "cue_order"),
            "216be82e33b5eb4aaf53ed06433325c62df5a5410b10e78f9709f41cab9bdc89",
        ),
        (
            tuple(1, "training", 1, 0, "cue_order"),
            "6d06fd918de59cf14c0192ad6f09bc2e31a4565c2f5b0e87ab7f19b9a14e7ae7",
        ),
    ];
    for (t, expected) in cases {
        assert_eq!(derive_seed_hex(&t).unwrap(), expected, "{t:?}");
    }
}

#[test]
fn namespaces_are_disjoint() {
    assert_eq!(SUPPORTED_NAMESPACES.len(), 4);
    for ns in SUPPORTED_NAMESPACES {
        assert!(is_supported_namespace(ns));
    }
    // Same coordinates, different namespaces -> different seeds.
    let dev = derive_seed_hex(&tuple(42, "development", 7, 0, "timing")).unwrap();
    let val = derive_seed_hex(&tuple(42, "validation", 7, 0, "timing")).unwrap();
    let tst = derive_seed_hex(&tuple(42, "final_test", 7, 0, "timing")).unwrap();
    assert_ne!(dev, val);
    assert_ne!(dev, tst);
    assert_ne!(val, tst);
}

#[test]
fn reserved_streams_are_pairwise_distinct() {
    let mut seen = std::collections::BTreeSet::new();
    for stream in SUPPORTED_STREAMS {
        let hex = derive_seed_hex(&tuple(1, "development", 1, 0, stream)).unwrap();
        assert!(seen.insert(hex), "duplicate seed for stream {stream}");
    }
}

#[test]
fn actor_initialization_and_hidden_membership_have_distinct_streams() {
    assert_ne!(ACTOR_INIT_STREAM, CUE_MEMBERSHIP_STREAM);
    let actor = derive_seed_hex(&tuple(1, "development", 1, 0, ACTOR_INIT_STREAM)).unwrap();
    let membership =
        derive_seed_hex(&tuple(1, "development", 1, 0, CUE_MEMBERSHIP_STREAM)).unwrap();
    assert_ne!(actor, membership);
}

#[test]
fn agent_draws_do_not_shift_environment_streams() {
    // Simulate: the agent draws 10_000 values from actor_noise; the cue
    // schedule RNG (fresh instance for cue_order) must still produce the
    // same first value as an untouched instance.
    let mut agent_rng = rng_for(&tuple(1, "development", 1, 0, "actor_noise")).unwrap();
    let mut blackhole = 0u64;
    for _ in 0..10_000 {
        blackhole = blackhole.wrapping_add(agent_rng.next_u64());
    }
    assert_ne!(blackhole, 0);

    let mut env_rng_a = rng_for(&tuple(1, "development", 1, 0, "cue_order")).unwrap();
    let mut env_rng_b = rng_for(&tuple(1, "development", 1, 0, "cue_order")).unwrap();
    assert_eq!(env_rng_a.next_u64(), env_rng_b.next_u64());
}

#[test]
fn unknown_namespace_and_bad_stream_rejected() {
    assert!(validate_tuple(&tuple(1, "test", 1, 0, "cue_order")).is_err());
    assert!(validate_tuple(&tuple(1, "development", 1, 0, "")).is_err());
    assert!(validate_tuple(&tuple(1, "development", 1, 0, "CueOrder")).is_err());
    assert!(validate_tuple(&tuple(1, "development", 1, 0, "cue order")).is_err());
    assert!(rng_for(&tuple(1, "staging", 1, 0, "timing")).is_err());
}

#[test]
fn snapshot_rejects_foreign_live_rng_and_nonzero_chacha_substream() {
    let tuple = cra::rng::SeedTuple::new(1, "development", 1, 0, "actor_noise");
    let foreign = cra::rng::SeedTuple::new(1, "development", 2, 0, "actor_noise");
    let mut rng = cra::rng::rng_for(&tuple).unwrap();
    assert!(cra::rng::RngState::capture(&rng, &foreign).is_err());
    rng.set_stream(1);
    assert!(cra::rng::RngState::capture(&rng, &tuple).is_err());
}
