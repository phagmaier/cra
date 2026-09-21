//! M2-03 / spec 17.5. Bounded, explicitly invoked Monte Carlo diagnostic.
//! Plan: docs/evidence/m2-03/plan.md. This is not an online learning loop.

use cra::agent::score::conditional_score;
use cra::agent::weights::NormalStream;
use cra::rng::{SeedTuple, canonical_string, derive_seed_hex, rng_for};
use serde::Serialize;
use sha2::{Digest, Sha256};

const SAMPLES: u64 = 1_000_000;
const ALPHA: f64 = 0.2;
const INPUT: f64 = 0.7;
const WEIGHT: f64 = 0.3;
const SIGMA: f64 = 0.4;
const BASELINE: f64 = 0.5;
const SE_MULTIPLIER: f64 = 5.0;
const NUMERICAL_TOLERANCE: f64 = 1e-12;

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
    analytical: f64,
    absolute_error: f64,
    tolerance: f64,
    passed: bool,
}

impl Moments {
    fn add(&mut self, value: f64) {
        assert!(value.is_finite(), "nonfinite diagnostic term");
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        self.m2 += delta * (value - self.mean);
    }

    fn estimate(&self, analytical: f64) -> Estimate {
        assert!(
            self.count >= 2,
            "sample variance requires at least two terms"
        );
        let sample_variance = self.m2 / (self.count - 1) as f64;
        assert!(sample_variance.is_finite() && sample_variance >= 0.0);
        let standard_error = (sample_variance / self.count as f64).sqrt();
        let absolute_error = (self.mean - analytical).abs();
        let tolerance = SE_MULTIPLIER * standard_error + NUMERICAL_TOLERANCE;
        Estimate {
            count: self.count,
            mean: self.mean,
            sample_variance,
            standard_error,
            analytical,
            absolute_error,
            tolerance,
            passed: absolute_error <= tolerance,
        }
    }
}

fn analytical_derivative() -> f64 {
    let mu = ALPHA * WEIGHT * INPUT;
    let z = mu / SIGMA;
    (ALPHA * INPUT / SIGMA) * (-0.5 * z * z).exp() / std::f64::consts::TAU.sqrt()
}

fn sample_terms(xi: f64) -> (f64, f64, bool) {
    let mu = ALPHA * WEIGHT * INPUT;
    let h = mu + SIGMA * xi;
    assert!(h.is_finite(), "nonfinite diagnostic membrane");
    let rewarded = h > 0.0;
    let reward = if rewarded { 1.0 } else { 0.0 };
    let opposite_reward = 1.0 - reward;
    let score = conditional_score(ALPHA, INPUT, xi, SIGMA).expect("valid score");
    (
        (reward - BASELINE) * score,
        (opposite_reward - BASELINE) * score,
        rewarded,
    )
}

fn command_output(command: &str, args: &[&str]) -> String {
    let output = std::process::Command::new(command)
        .args(args)
        .output()
        .expect("provenance command available");
    assert!(output.status.success(), "provenance command failed");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn moments_match_hand_calculated_sample_variance_and_standard_error() {
    let mut moments = Moments::default();
    for value in [1.0, 2.0, 3.0, 4.0] {
        moments.add(value);
    }
    let estimate = moments.estimate(2.5);
    assert_eq!(estimate.count, 4);
    assert_eq!(estimate.mean, 2.5);
    assert!((estimate.sample_variance - 5.0 / 3.0).abs() < 1e-14);
    assert!((estimate.standard_error - (5.0_f64 / 12.0).sqrt()).abs() < 1e-14);
    assert!(estimate.passed);
    // Constant negative terms have exactly zero uncertainty, not NaN.
    let mut constant = Moments::default();
    for _ in 0..4 {
        constant.add(-0.25);
    }
    let estimate = constant.estimate(-0.25);
    assert_eq!(estimate.sample_variance, 0.0);
    assert_eq!(estimate.standard_error, 0.0);
    assert_eq!(estimate.tolerance, NUMERICAL_TOLERANCE);
    assert!(estimate.passed);
    assert!(!constant.estimate(0.25).passed);
}

#[test]
fn forced_samples_cover_both_rewards_and_opposite_target() {
    for (xi, expected, reward) in [
        (-1.0, 0.175, false),
        (0.0, 0.0, true),
        (1.0, 0.175, true),
        (-0.05, -0.00875, true),
    ] {
        let (positive, opposite, rewarded) = sample_terms(xi);
        assert!((positive - expected).abs() < 1e-14);
        assert!((opposite + expected).abs() < 1e-14);
        assert_eq!(rewarded, reward);
    }
}

#[test]
fn analytical_value_matches_specification() {
    assert!((ALPHA * WEIGHT * INPUT - 0.042).abs() < 1e-14);
    assert!((analytical_derivative() - 0.138862).abs() < 5e-7);
}

#[test]
#[ignore = "bounded million-sample M2-03 diagnostic; invoke explicitly per README"]
fn one_neuron_learning_direction() {
    // Reserve immutable output before sampling; failure assertions happen only
    // after the result has been printed and saved. Never overwrite an old run.
    let evidence = std::env::var_os("CRA_M2_DIRECTION_EVIDENCE").map(|path| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .expect("evidence parent must exist and file must be fresh")
    });
    let revision = command_output("git", &["rev-parse", "HEAD"]);
    let status = command_output("git", &["status", "--porcelain"]);
    let seed = SeedTuple::new(1, "development", 203, 0, "actor_noise");
    let mut rng = rng_for(&seed).unwrap();
    let mut normal = NormalStream::new(&mut rng);
    let mut positive = Moments::default();
    let mut opposite = Moments::default();
    let mut positive_rewards = 0_u64;
    for _ in 0..SAMPLES {
        let (positive_term, opposite_term, rewarded) = sample_terms(normal.next_standard());
        positive.add(positive_term);
        opposite.add(opposite_term);
        positive_rewards += u64::from(rewarded);
    }
    let expected = analytical_derivative();
    let positive = positive.estimate(expected);
    let opposite = opposite.estimate(-expected);
    let sign_reversal = positive.mean > 0.0
        && opposite.mean < 0.0
        && (positive.mean + opposite.mean).abs() <= NUMERICAL_TOLERANCE
        && (positive.standard_error - opposite.standard_error).abs() <= NUMERICAL_TOLERANCE;
    let passed = positive.passed && opposite.passed && sign_reversal;
    let report = serde_json::json!({
        "schema_version": 1, "task": "M2-03", "passed": passed,
        "configuration": {
            "samples": SAMPLES, "alpha": ALPHA, "input": INPUT, "weight": WEIGHT,
            "sigma": SIGMA, "baseline": BASELINE, "mu": ALPHA * WEIGHT * INPUT,
            "se_multiplier": SE_MULTIPLIER, "numerical_tolerance": NUMERICAL_TOLERANCE,
            "positive_reward": "h > 0", "opposite_reward": "1 - positive_reward",
            "paired_targets": true, "online_updates": 0
        },
        "seed": { "canonical": canonical_string(&seed), "derived_hex": derive_seed_hex(&seed).unwrap() },
        "rng": "ChaCha8Rng / existing Box-Muller NormalStream; spare retained across independent samples; versions in Cargo.lock",
        "positive": positive, "opposite": opposite, "sign_reversal": sign_reversal,
        "positive_reward_count": positive_rewards, "opposite_reward_count": SAMPLES - positive_rewards,
        "resource_counts": { "normal_draws": SAMPLES, "transitions": SAMPLES, "workers": 1 },
        "provenance": {
            "revision": revision, "dirty": !status.is_empty(), "git_status": status,
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            "rustc": command_output("rustc", &["--version"]),
            "cargo": command_output("cargo", &["--version"]),
            "source_sha256": {
                "tests/score_learning_direction.rs": sha256(include_bytes!("score_learning_direction.rs")),
                "src/agent/score.rs": sha256(include_bytes!("../src/agent/score.rs")),
                "src/agent/weights.rs": sha256(include_bytes!("../src/agent/weights.rs")),
                "src/rng.rs": sha256(include_bytes!("../src/rng.rs")),
                "Cargo.lock": sha256(include_bytes!("../Cargo.lock"))
            }
        },
        "claim_limit": "One-transition direction diagnostic only; no online learning or convergence claim."
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if let Some(file) = evidence {
        serde_json::to_writer_pretty(&file, &report).unwrap();
        file.sync_all().unwrap();
    }
    assert!(
        passed,
        "direction check failed; inspect saved evidence without reseeding"
    );
}
