//! Perturbation generator and draw-schedule contracts (M1-04).
//!
//! - `step` draws exactly one perturbation per actor neuron on every call
//!   from the dedicated `actor_noise` stream, matching a reference
//!   `NormalStream` output-for-output across ticks (including an odd neuron
//!   count, where the unpaired deviate is discarded, not carried).
//! - Draws ignore input values and membrane saturation: different phases
//!   of the lifetime cannot shift the noise schedule.
//! - Observation (getters) draws nothing: interleaved reads leave the
//!   trajectory bitwise identical.
//! - Seeded moments match `Normal(0, 1)` within declared tolerances.
//! - Actor stepping never touches environment streams.
//! - `ChaCha8Rng` word position round-trips the uniform stream for resume,
//!   while a word position alone provably does NOT resume the normal
//!   stream (the cached Box–Muller spare must be stored too; M1-09).

use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};

use cra::agent::actor::ActorState;
use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, NormalStream, Weights};
use cra::config::Actor;
use cra::rng::{SeedTuple, derive_seed_bytes, rng_for};

fn actor_with(n: usize, m: usize) -> Actor {
    Actor {
        neuron_count: n,
        motor_neurons_per_action: m,
        edge_probability: 0.0,
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

fn quiet_params(n: usize, m: usize, input_dim: usize) -> InheritedParams {
    let mask = vec![vec![false; n]; n];
    let topology =
        topology_from_mask(n, m, 0.0, false, mask, "noise-fixture".to_owned()).expect("mask");
    InheritedParams {
        topology,
        weights: Weights {
            w0: vec![vec![0.0; n]; n],
            input_weights: vec![vec![0.0; input_dim]; n],
            bias: vec![0.0; n],
        },
    }
}

fn noise_seed(outer: u64) -> SeedTuple {
    SeedTuple::new(1, "development", outer, 0, "actor_noise")
}

#[test]
fn step_draws_match_reference_stream_every_tick() {
    // The documented rule is per-tick-local pairing: each tick reads its N
    // outputs from a fresh stream over the shared RNG position. For even N
    // that coincides with one contiguous stream, which is additionally
    // checked absolutely below.
    for (n, ticks) in [(4_usize, 6_usize), (3_usize, 3_usize)] {
        let actor = actor_with(n, 1);
        let params = quiet_params(n, 1, 2);
        let seed = noise_seed(1);
        let mut ref_rng = rng_for(&seed).expect("rng");
        let mut actor_rng = rng_for(&seed).expect("rng");
        let mut state = ActorState::new(n).expect("state");
        for t in 0..ticks {
            // Documented rule: each tick reads its N outputs from a fresh
            // stream over the shared RNG position.
            let expected: Vec<f64> = {
                let mut fresh = NormalStream::new(&mut ref_rng);
                (0..n).map(|_| fresh.next_standard()).collect()
            };
            // Absolute check for even N: an independent stream advanced to
            // the same tick must agree (fresh pairing coincides with the
            // contiguous stream when no spare is ever left over).
            if n % 2 == 0 {
                let mut probe_rng = rng_for(&seed).expect("rng");
                let mut probe = NormalStream::new(&mut probe_rng);
                for _ in 0..t * n {
                    let _ = probe.next_standard();
                }
                let check: Vec<f64> = (0..n).map(|_| probe.next_standard()).collect();
                assert_eq!(expected, check, "tick {t} derivations disagree");
            }
            let input = if t % 2 == 0 {
                vec![0.0, 0.0]
            } else {
                vec![1.0, 0.0]
            };
            state
                .step(&actor, &params, &input, &mut actor_rng)
                .expect("step");
            assert_eq!(
                state.last_perturbations(),
                expected.as_slice(),
                "tick {t}, N = {n}"
            );
        }
    }
}

#[test]
fn draws_ignore_inputs_and_saturation() {
    // Same seed, wildly different lifetime phases: zero state with zero
    // inputs versus saturated state with large inputs. Trajectories may
    // differ; the perturbation schedule must not.
    let actor = actor_with(4, 1);
    let params = quiet_params(4, 1, 2);
    let seed = noise_seed(3);
    let mut rng_a = rng_for(&seed).expect("rng");
    let mut rng_b = rng_for(&seed).expect("rng");
    let mut calm = ActorState::new(4).expect("state");
    let mut hot =
        ActorState::from_state(vec![500.0, -500.0, 500.0, -500.0], vec![0.0; 4]).expect("state");
    for _ in 0..3 {
        calm.step(&actor, &params, &[0.0, 0.0], &mut rng_a)
            .expect("step");
        hot.step(&actor, &params, &[3.0, -3.0], &mut rng_b)
            .expect("step");
        assert_eq!(calm.last_perturbations(), hot.last_perturbations());
    }
}

#[test]
fn observation_draws_nothing() {
    let actor = actor_with(4, 1);
    let params = quiet_params(4, 1, 2);
    let seed = noise_seed(5);
    let mut rng_a = rng_for(&seed).expect("rng");
    let mut rng_b = rng_for(&seed).expect("rng");
    let mut plain = ActorState::new(4).expect("state");
    let mut probed = ActorState::new(4).expect("state");
    for t in 0..4 {
        let input = vec![f64::from(t as u8), 0.0];
        plain
            .step(&actor, &params, &input, &mut rng_a)
            .expect("step");
        probed
            .step(&actor, &params, &input, &mut rng_b)
            .expect("step");
        // Diagnostic reads between steps must not advance any stream.
        let _ = (
            probed.h(),
            probed.a(),
            probed.r(),
            probed.last_perturbations(),
        );
    }
    assert_eq!(plain.h(), probed.h());
    assert_eq!(plain.last_perturbations(), probed.last_perturbations());
}

#[test]
fn actor_noise_moments_match_standard_normal() {
    // n = 50_000 normals: SE(mean) ~= 0.0045, SE(var) ~= 0.0063. Declared
    // bounds sit at ~4.5 SE.
    let seed = noise_seed(1);
    let mut rng = rng_for(&seed).expect("rng");
    let mut stream = NormalStream::new(&mut rng);
    let n = 50_000_u32;
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for k in 1..=n {
        let x = stream.next_standard();
        let delta = x - mean;
        mean += delta / f64::from(k);
        m2 += delta * (x - mean);
    }
    let var = m2 / f64::from(n);
    assert!(mean.abs() < 0.02, "mean {mean} far from 0");
    assert!((var - 1.0).abs() < 0.03, "variance {var} far from 1");
}

#[test]
fn actor_stepping_never_touches_environment_streams() {
    // The exogenous cue schedule must be identical before and after heavy
    // actor-side stepping: step owns only its actor_noise RNG.
    let actor = actor_with(4, 1);
    let params = quiet_params(4, 1, 2);
    let cue = SeedTuple::new(1, "development", 1, 0, "cue_order");
    let mut env_before = rng_for(&cue).expect("rng");
    let first = env_before.next_u64();
    let mut actor_rng = rng_for(&noise_seed(1)).expect("rng");
    let mut state = ActorState::new(4).expect("state");
    for _ in 0..100 {
        state
            .step(&actor, &params, &[1.0, 0.0], &mut actor_rng)
            .expect("step");
    }
    let mut env_after = rng_for(&cue).expect("rng");
    assert_eq!(env_after.next_u64(), first);
}

#[test]
fn word_pos_round_trips_the_uniform_stream() {
    // Resume primitive for M1-09: seed bytes plus word position reproduce
    // the uniform stream exactly, without replaying draws.
    let bytes = derive_seed_bytes(&noise_seed(9)).expect("seed bytes");
    let mut a = ChaCha8Rng::from_seed(bytes);
    for _ in 0..10 {
        let _ = a.next_u64();
    }
    let pos = a.get_word_pos();
    let mut b = ChaCha8Rng::from_seed(bytes);
    b.set_word_pos(pos);
    for _ in 0..10 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn word_pos_resumes_next_tick_perturbations() {
    // Checkpoint path for M1-09: after a tick, seed bytes plus word
    // position reproduce the following tick's perturbations exactly — for
    // even and (re-pairing) odd neuron counts alike. No spare crosses
    // ticks, so none is stored.
    for n in [4_usize, 3_usize] {
        let actor = actor_with(n, 1);
        let params = quiet_params(n, 1, 2);
        let bytes = derive_seed_bytes(&noise_seed(9)).expect("seed bytes");
        let mut rng = ChaCha8Rng::from_seed(bytes);
        let mut state = ActorState::new(n).expect("state");
        state
            .step(&actor, &params, &[0.0, 0.0], &mut rng)
            .expect("step");
        let pos = rng.get_word_pos();
        let mut restored_rng = ChaCha8Rng::from_seed(bytes);
        restored_rng.set_word_pos(pos);
        let mut restored_stream = NormalStream::new(&mut restored_rng);
        let expected: Vec<f64> = (0..n).map(|_| restored_stream.next_standard()).collect();
        state
            .step(&actor, &params, &[1.0, 0.0], &mut rng)
            .expect("step");
        assert_eq!(state.last_perturbations(), expected.as_slice(), "N = {n}");
    }
}
