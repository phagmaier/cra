//! Config validation table tests (M0-05).
//!
//! Each case starts from a known-valid profile and mutates one contract
//! dimension: invalid probabilities, bad durations, `dt != 1`,
//! pending-choice limits, schema versions, and missing/unknown seed
//! namespaces — plus dimension, score-noise, reset-policy, and evolution
//! checks that arrive with their modules. Unknown fields are rejected
//! rather than silently ignored.

use cra::config::{Config, ConfigError, validate};

fn env_smoke() -> String {
    std::fs::read_to_string("configs/env_smoke.toml").expect("smoke profile exists")
}

fn debug_stationary() -> String {
    std::fs::read_to_string("configs/debug_stationary.toml").expect("debug profile exists")
}

fn parse_and_validate(text: &str) -> Result<Config, ConfigError> {
    let cfg: Config = toml::from_str(text).map_err(|e| ConfigError::Parse {
        path: "<test>".to_owned(),
        message: e.to_string(),
    })?;
    validate(&cfg)?;
    Ok(cfg)
}

fn expect_err(text: &str) -> ConfigError {
    parse_and_validate(text).expect_err("case must fail validation")
}

fn err_kind(e: &ConfigError) -> &'static str {
    match e {
        ConfigError::Io { .. } => "io",
        ConfigError::Parse { .. } => "parse",
        ConfigError::UnsupportedSchemaVersion { .. } => "schema",
        ConfigError::InvalidProfileName => "profile",
        ConfigError::InvalidDt { .. } => "dt",
        ConfigError::UnsupportedPrecision { .. } => "precision",
        ConfigError::UnknownResetPolicy { .. } => "reset",
        ConfigError::UnknownFeedbackOrder { .. } => "feedback-order",
        ConfigError::InvalidLifetimeLength { .. } => "lifetime",
        ConfigError::UnknownEnvironmentKind { .. } => "env-kind",
        ConfigError::InvalidCueCount { .. } => "cues",
        ConfigError::UnsupportedCueEncoding { .. } => "encoding",
        ConfigError::InvalidStableFraction { .. } => "stable",
        ConfigError::InvalidFeedbackNoise { .. } => "noise",
        ConfigError::InvalidHazard { .. } => "hazard",
        ConfigError::UnsupportedHazardClock { .. } => "hazard-clock",
        ConfigError::InvalidTiming { .. } => "timing",
        ConfigError::InvalidPendingChoices { .. } => "pending",
        ConfigError::UnknownSeedNamespace { .. } => "seed-ns",
        ConfigError::InvalidActorDimensions { .. } => "actor-dims",
        ConfigError::InvalidActorParams { .. } => "actor-params",
        ConfigError::InvalidLearning { .. } => "learning",
        ConfigError::InvalidModulator { .. } => "modulator",
        ConfigError::InvalidEvolution { .. } => "evolution",
        ConfigError::InvalidLogging { .. } => "logging",
        ConfigError::InconsistentResetPolicy { .. } => "reset-trace",
    }
}

#[test]
fn valid_profiles_pass() {
    parse_and_validate(&env_smoke()).expect("env_smoke validates");
    parse_and_validate(&debug_stationary()).expect("debug_stationary validates");
}

#[test]
fn table_invalid_configs() {
    let smoke = env_smoke();
    let debug = debug_stationary();
    let cases: Vec<(&str, String, &str)> = vec![
        ("dt != 1", smoke.replace("dt = 1.0", "dt = 0.5"), "dt"),
        (
            "noise above 0.5",
            smoke.replace(
                "feedback_noise_values = [0.0]",
                "feedback_noise_values = [0.6]",
            ),
            "noise",
        ),
        (
            "negative noise",
            smoke.replace(
                "feedback_noise_values = [0.0]",
                "feedback_noise_values = [-0.1]",
            ),
            "noise",
        ),
        (
            "hazard above 1",
            smoke.replace(
                "volatile_hazard_values = [0.0]",
                "volatile_hazard_values = [1.5]",
            ),
            "hazard",
        ),
        (
            "zero reward delay",
            smoke.replace("reward_delay_ticks = [1, 1]", "reward_delay_ticks = [0, 1]"),
            "timing",
        ),
        (
            "inverted quiet range",
            smoke.replace("quiet_ticks = [4, 4]", "quiet_ticks = [8, 4]"),
            "timing",
        ),
        (
            "feedback ticks != 1",
            smoke.replace("feedback_ticks = 1", "feedback_ticks = 2"),
            "timing",
        ),
        (
            "two pending choices",
            smoke.replace("max_pending_choices = 1", "max_pending_choices = 2"),
            "pending",
        ),
        (
            "unsupported schema",
            smoke.replace("schema_version = 1", "schema_version = 2"),
            "schema",
        ),
        (
            "unknown seed namespace",
            smoke.replace("namespace = \"development\"", "namespace = \"staging\""),
            "seed-ns",
        ),
        (
            "missing seed namespace",
            smoke.replace("namespace = \"development\"\n", ""),
            "parse",
        ),
        (
            "unknown top-level field rejected",
            format!("{smoke}\n[unknown_section]\nfoo = 1\n"),
            "parse",
        ),
        (
            "motor pools exceed neurons",
            debug.replace("neuron_count = 16", "neuron_count = 3"),
            "actor-dims",
        ),
        (
            "zero score noise with active rule",
            debug.replace("noise_sigma = 0.05", "noise_sigma = 0.0"),
            "actor-params",
        ),
        (
            "diagnostic traces under birth_only",
            debug.replace(
                "trace_policy = \"persistent\"",
                "trace_policy = \"no_decay_diagnostic\"",
            ),
            "reset-trace",
        ),
        (
            "elite equals population",
            debug.replace("elite_count = 8", "elite_count = 32"),
            "evolution",
        ),
        (
            "unknown gate mode",
            debug.replace("mode = \"fixed\"", "mode = \"fancy\""),
            "modulator",
        ),
        (
            "forbidden modulator-to-actor pathway",
            debug.replace(
                "ordinary_feedback_to_actor = false",
                "ordinary_feedback_to_actor = true",
            ),
            "modulator",
        ),
        (
            "hidden weight decay",
            debug.replace("plastic_decay = 0.0", "plastic_decay = 0.9"),
            "learning",
        ),
    ];
    assert!(!cases.is_empty());
    for (name, text, want) in cases {
        let err = expect_err(&text);
        assert_eq!(err_kind(&err), want, "case '{name}': got {err}");
    }
}
