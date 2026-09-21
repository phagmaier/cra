//! M2-05 / spec 17.6. Plan fixed in docs/evidence/m2-05/plan.md.
//! Common Gaussian draws couple separately evolving, fixed-weight rollouts.

use cra::agent::topology::topology_from_mask;
use cra::agent::weights::{InheritedParams, Weights};
use cra::config::Actor;
use cra::experiments::finite_rollout::{FiniteRollout, MODE};
use cra::rng::{SeedTuple, canonical_string, derive_seed_hex, rng_for};
use rand_chacha::ChaCha8Rng;
use serde::Serialize;
use sha2::{Digest, Sha256};

const SAMPLES: u64 = 2_000_000;
const HORIZON: u64 = 6;
const EPSILONS: [f64; 3] = [0.04, 0.02, 0.01];
const SE_MULTIPLIER: f64 = 5.0;
const BIAS_ALLOWANCE: f64 = 0.0005;
const MAX_HALF_WIDTH: f64 = 0.02;

#[derive(Default)]
struct Moments {
    count: u64,
    mean: f64,
    m2: f64,
}

#[derive(Debug, Serialize)]
struct Estimate {
    count: u64,
    mean: f64,
    sample_variance: f64,
    standard_error: f64,
    five_se_half_width: f64,
}

impl Moments {
    fn add(&mut self, x: f64) {
        assert!(x.is_finite());
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        self.m2 += delta * (x - self.mean);
    }

    fn estimate(&self) -> Option<Estimate> {
        if self.count < 2 {
            return None;
        }
        let sample_variance = self.m2 / (self.count - 1) as f64;
        assert!(sample_variance.is_finite() && sample_variance >= 0.0);
        let standard_error = (sample_variance / self.count as f64).sqrt();
        Some(Estimate {
            count: self.count,
            mean: self.mean,
            sample_variance,
            standard_error,
            five_se_half_width: SE_MULTIPLIER * standard_error,
        })
    }
}

#[derive(Debug, Serialize)]
struct Comparison {
    discrepancy_tolerance: f64,
    agreement: bool,
    precision_resolved: bool,
    positive_signal: bool,
    passed: bool,
}

fn compare(score: &Estimate, fd: &Estimate, difference: &Estimate) -> Comparison {
    let discrepancy_tolerance = difference.five_se_half_width + BIAS_ALLOWANCE;
    let agreement = difference.mean.abs() <= discrepancy_tolerance;
    let precision_resolved = [score, fd, difference]
        .iter()
        .all(|x| x.five_se_half_width <= MAX_HALF_WIDTH);
    let positive_signal = score.mean > score.five_se_half_width && fd.mean > fd.five_se_half_width;
    Comparison {
        discrepancy_tolerance,
        agreement,
        precision_resolved,
        positive_signal,
        passed: agreement && precision_resolved && positive_signal,
    }
}

fn fixture(weight_shift: f64) -> (Actor, InheritedParams) {
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
        noise_sigma: 0.4,
        motor_filter_tau: 3.0,
    };
    let params = InheritedParams {
        topology: topology_from_mask(
            2,
            1,
            1.0,
            false,
            vec![vec![false, true], vec![true, false]],
            "M2-05".to_owned(),
        )
        .unwrap(),
        weights: Weights {
            w0: vec![vec![0.0, -0.4], vec![0.3 + weight_shift, 0.0]],
            input_weights: vec![vec![0.7], vec![-0.2]],
            bias: vec![0.0; 2],
        },
    };
    (actor, params)
}

fn rollouts() -> Vec<FiniteRollout> {
    std::iter::once(0.0)
        .chain(EPSILONS.into_iter().flat_map(|eps| [eps, -eps]))
        .map(|shift| {
            let (actor, params) = fixture(shift);
            FiniteRollout::new(actor, params, HORIZON, 0.0).unwrap()
        })
        .collect()
}

#[derive(Default, Serialize)]
struct Resources {
    completed_transitions: u64,
    completed_rollouts: u64,
    completed_paired_samples: u64,
    normal_draws: u64,
}

struct Sample {
    rewards: [f64; 7],
    score: f64,
}

fn sample(
    runs: &mut [FiniteRollout],
    rng: &mut ChaCha8Rng,
    counts: &mut Resources,
) -> Result<Sample, String> {
    for (setting, run) in runs.iter_mut().enumerate() {
        if run.steps() != 0 {
            run.reset_between_rollouts()
                .map_err(|e| format!("setting {setting}, reset: {e}"))?;
        }
    }
    for tick in 0..HORIZON {
        // The base uses the real actor RNG path. For N=2 exactly two normals
        // are drawn and there is no discarded Box-Muller spare at tick end.
        counts.normal_draws += 2;
        runs[0]
            .step(&[1.0], rng)
            .map_err(|e| format!("setting 0, tick {tick}: {e}"))?;
        counts.completed_transitions += 1;
        let xi = [
            runs[0].state().last_perturbations()[0],
            runs[0].state().last_perturbations()[1],
        ];
        for (setting, run) in runs.iter_mut().enumerate().skip(1) {
            run.step_with_perturbations(&[1.0], &xi)
                .map_err(|e| format!("setting {setting}, tick {tick}: {e}"))?;
            counts.completed_transitions += 1;
        }
    }
    let mut result = Sample {
        rewards: [0.0; 7],
        score: 0.0,
    };
    for (setting, run) in runs.iter_mut().enumerate() {
        let reward = f64::from(run.state().h()[1] > 0.0);
        let terminal = run
            .finish(reward, None)
            .map_err(|e| format!("setting {setting}, finish: {e}"))?;
        result.rewards[setting] = reward;
        if setting == 0 {
            result.score = terminal.reward_weighted_scores[1][0];
        }
        counts.completed_rollouts += 1;
    }
    counts.completed_paired_samples += 1;
    Ok(result)
}

#[test]
fn paired_uncertainty_uses_sample_differences() {
    let mut fd = Moments::default();
    let mut score = Moments::default();
    let mut difference = Moments::default();
    for x in [1.0, 2.0, 3.0, 4.0] {
        fd.add(x);
        score.add(x - 0.25);
        difference.add(x - (x - 0.25));
    }
    let fd = fd.estimate().unwrap();
    let score = score.estimate().unwrap();
    let difference = difference.estimate().unwrap();
    assert_eq!(fd.count, 4);
    assert_eq!(fd.mean, 2.5);
    assert!((fd.sample_variance - 5.0 / 3.0).abs() < 1e-14);
    assert!((fd.standard_error - (5.0_f64 / 12.0).sqrt()).abs() < 1e-14);
    assert_eq!(score.sample_variance, fd.sample_variance);
    assert_eq!(difference.mean, 0.25);
    assert_eq!(difference.standard_error, 0.0);
    assert!(Moments::default().estimate().is_none());
}

#[test]
fn broad_intervals_and_precise_disagreement_cannot_pass() {
    let estimate = |mean, half_width| Estimate {
        count: SAMPLES,
        mean,
        sample_variance: 0.0,
        standard_error: half_width / SE_MULTIPLIER,
        five_se_half_width: half_width,
    };
    let score = estimate(0.3, 0.001);
    let fd = estimate(0.3, 0.002);
    assert!(compare(&score, &fd, &estimate(0.0, 0.002)).passed);
    let broad = compare(&score, &fd, &estimate(0.1, 0.5));
    assert!(broad.agreement);
    assert!(!broad.precision_resolved && !broad.passed);
    assert!(!compare(&score, &fd, &estimate(0.01, 0.001)).passed);
    assert!(!compare(&estimate(0.0, 0.001), &fd, &estimate(0.0, 0.001)).passed);
}

#[test]
fn forced_trajectory_preserves_weights_and_exposes_recurrent_effect() {
    let mut runs = rollouts();
    let frozen: Vec<_> = runs.iter().map(|r| r.parameters().clone()).collect();
    for run in &mut runs {
        assert_eq!(run.state().h(), &[0.0, 0.0]);
        run.step_with_perturbations(&[1.0], &[0.0, 0.0]).unwrap();
        assert_eq!(run.state().h(), &[0.35, -0.1]);
        assert_eq!(run.score_sums()[1][0], 0.0);
        run.step_with_perturbations(&[1.0], &[0.0, 1.0]).unwrap();
        let expected = 0.5 * 0.35_f64.tanh() / 0.4;
        assert!((run.score_sums()[1][0] - expected).abs() < 1e-14);
    }
    for (k, eps) in EPSILONS.iter().enumerate() {
        let plus = runs[1 + 2 * k].state().h()[1];
        let minus = runs[2 + 2 * k].state().h()[1];
        assert!(((plus - minus) / (2.0 * eps) - 0.5 * 0.35_f64.tanh()).abs() < 1e-14);
    }
    for (run, params) in runs.iter_mut().zip(frozen) {
        for _ in 2..HORIZON {
            run.step_with_perturbations(&[1.0], &[0.1, -0.2]).unwrap();
        }
        let reward = f64::from(run.state().h()[1] > 0.0);
        let result = run.finish(reward, None).unwrap();
        assert!(result.terminal_weights.is_none());
        assert_eq!(
            result.reward_weighted_scores[1][0],
            reward * result.score_sums[1][0]
        );
        assert_eq!(run.parameters(), &params);
        assert_eq!(run.baseline(), 0.0);
        run.reset_between_rollouts().unwrap();
        assert_eq!(run.state().h(), &[0.0, 0.0]);
        assert_eq!(run.score_sums(), &[vec![0.0; 2], vec![0.0; 2]]);
        assert_eq!(run.parameters(), &params);
    }
}

#[test]
fn paired_sampler_resets_and_replays_exactly() {
    let seed = SeedTuple::new(1, "development", 205, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    let mut runs = rollouts();
    let mut counts = Resources::default();
    let first = sample(&mut runs, &mut rng, &mut counts).unwrap();
    let mut replay_rng = rng_for(&seed).unwrap();
    let second = sample(&mut runs, &mut replay_rng, &mut counts).unwrap();
    assert_eq!(first.rewards, second.rewards);
    assert_eq!(first.score.to_bits(), second.score.to_bits());
    assert_eq!(counts.completed_paired_samples, 2);
    assert_eq!(counts.completed_rollouts, 14);
    assert_eq!(counts.completed_transitions, 84);
    assert_eq!(counts.normal_draws, 24);
    // All paired settings consumed exactly the same per-neuron draws.
    for run in &runs[1..] {
        assert_eq!(
            run.state().last_perturbations(),
            runs[0].state().last_perturbations()
        );
    }
}

#[test]
fn failed_paired_sample_is_reported_without_counting_it_as_complete() {
    let mut runs = rollouts();
    let (actor, mut params) = fixture(EPSILONS[0]);
    params.weights.input_weights[0][0] = f64::MAX;
    params.weights.bias[0] = f64::MAX; // finite inputs whose drive sum overflows
    runs[1] = FiniteRollout::new(actor, params, HORIZON, 0.0).unwrap();
    let seed = SeedTuple::new(1, "development", 205, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    let mut counts = Resources::default();
    let error = sample(&mut runs, &mut rng, &mut counts).err().unwrap();
    assert!(error.contains("setting 1, tick 0"), "{error}");
    assert_eq!(counts.completed_paired_samples, 0);
    assert_eq!(counts.completed_rollouts, 0);
    assert_eq!(counts.completed_transitions, 1);
    assert_eq!(counts.normal_draws, 2);
}

fn command_output(command: &str, args: &[&str]) -> String {
    let output = std::process::Command::new(command)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
#[ignore = "bounded two-million-trajectory M2-05 diagnostic; invoke explicitly per README"]
fn two_neuron_recurrent_finite_difference() {
    let evidence = std::env::var_os("CRA_M2_RECURRENT_EVIDENCE").map(|path| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .expect("evidence parent must exist and file must be fresh")
    });
    let started = std::time::Instant::now();
    let revision = command_output("git", &["rev-parse", "HEAD"]);
    let status = command_output("git", &["status", "--porcelain"]);
    let seed = SeedTuple::new(1, "development", 205, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    let mut runs = rollouts();
    let mut counts = Resources::default();
    let mut score = Moments::default();
    let mut rewards: [Moments; 7] = std::array::from_fn(|_| Moments::default());
    let mut fd: [Moments; 3] = std::array::from_fn(|_| Moments::default());
    let mut differences: [Moments; 3] = std::array::from_fn(|_| Moments::default());
    let mut failures = Vec::new();
    for index in 0..SAMPLES {
        let value = match sample(&mut runs, &mut rng, &mut counts) {
            Ok(value) => value,
            Err(error) => {
                failures.push(serde_json::json!({"sample_index": index, "error": error}));
                break;
            }
        };
        score.add(value.score);
        for (moments, reward) in rewards.iter_mut().zip(value.rewards) {
            moments.add(reward);
        }
        for (k, eps) in EPSILONS.iter().enumerate() {
            let derivative = (value.rewards[1 + 2 * k] - value.rewards[2 + 2 * k]) / (2.0 * eps);
            fd[k].add(derivative);
            differences[k].add(derivative - value.score);
        }
    }
    let score = score.estimate();
    let mut comparisons = Vec::new();
    let mut passed = failures.is_empty() && counts.completed_paired_samples == SAMPLES;
    for (k, eps) in EPSILONS.iter().enumerate() {
        let fd = fd[k].estimate();
        let difference = differences[k].estimate();
        let comparison = score
            .as_ref()
            .zip(fd.as_ref())
            .zip(difference.as_ref())
            .map(|((s, d), error)| compare(s, d, error));
        passed &= comparison.as_ref().is_some_and(|c| c.passed);
        comparisons.push(serde_json::json!({
            "epsilon": eps, "plus_reward": rewards[1 + 2*k].estimate(),
            "minus_reward": rewards[2 + 2*k].estimate(),
            "finite_difference": fd, "paired_difference_fd_minus_score": difference,
            "assessment": comparison,
        }));
    }
    let (actor, params) = fixture(0.0);
    let report = serde_json::json!({
        "schema_version": 1, "task": "M2-05", "passed": passed,
        "mode": MODE, "failures": failures,
        "configuration": {
            "samples": SAMPLES, "horizon": HORIZON, "actor": actor, "parameters": params,
            "initial_state": "zero, independent of weights; explicit between-rollout reset",
            "input": [1.0], "terminal_reward": "1[h[1] > 0]", "baseline": 0.0,
            "perturbed_edge": [1, 0], "epsilons": EPSILONS, "online_updates": 0,
            "common_random_numbers": true, "score_decay": "none (exact sum)",
            "se_multiplier": SE_MULTIPLIER, "finite_epsilon_bias_allowance": BIAS_ALLOWANCE,
            "max_five_se_half_width": MAX_HALF_WIDTH,
            "requires_positive_five_se_lower_bounds": true,
        },
        "seed": {"canonical": canonical_string(&seed), "derived_hex": derive_seed_hex(&seed).unwrap()},
        "rng": "ChaCha8 / existing actor per-tick Box-Muller NormalStream; two draws/tick, shared across settings only; versions in Cargo.lock",
        "score": score, "base_reward": rewards[0].estimate(), "comparisons": comparisons,
        "resource_counts": counts, "workers": 1, "elapsed_seconds": started.elapsed().as_secs_f64(),
        "provenance": {
            "revision": revision, "dirty": !status.is_empty(), "git_status": status,
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            "rustc": command_output("rustc", &["--version"]),
            "cargo": command_output("cargo", &["--version"]),
            "source_sha256": {
                "tests/score_recurrent.rs": sha256(include_bytes!("score_recurrent.rs")),
                "docs/evidence/m2-05/plan.md": sha256(include_bytes!("../docs/evidence/m2-05/plan.md")),
                "src/experiments/finite_rollout.rs": sha256(include_bytes!("../src/experiments/finite_rollout.rs")),
                "src/agent/actor.rs": sha256(include_bytes!("../src/agent/actor.rs")),
                "src/agent/score.rs": sha256(include_bytes!("../src/agent/score.rs")),
                "src/agent/weights.rs": sha256(include_bytes!("../src/agent/weights.rs")),
                "src/agent/topology.rs": sha256(include_bytes!("../src/agent/topology.rs")),
                "src/config.rs": sha256(include_bytes!("../src/config.rs")),
                "src/rng.rs": sha256(include_bytes!("../src/rng.rs")),
                "Cargo.lock": sha256(include_bytes!("../Cargo.lock")),
            },
        },
        "claim_limit": "Restricted finite-horizon fixed-weight diagnostic only; not online learning, convergence or unbiased online updates. Five-SE bands are Monte Carlo uncertainty estimates, not certified bounds."
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if let Some(file) = evidence {
        serde_json::to_writer_pretty(&file, &report).unwrap();
        file.sync_all().unwrap();
    }
    assert!(
        passed,
        "recurrent diagnostic failed or uncertainty unresolved; retain evidence and investigate without reseeding"
    );
}
