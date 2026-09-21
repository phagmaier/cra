//! Versioned TOML configuration parsing and validation (M0-05).
//!
//! Schema source: spec Section 19 (starting debug profile 19.2, validation
//! rules 19.4, condition construction 19.5). `schema_version = 1` is the only
//! accepted version.
//!
//! Scope notes:
//!
//! - Required sections: `simulation`, `environment`, `logging`, `seeds`.
//!   These are enough for the M0 environment-only smoke profile.
//! - Optional sections: `actor`, `learning`, `modulator`, `evolution`. They
//!   are parsed and validated when present (so `debug_stationary.toml` checks
//!   dimension, score-noise, reset-policy, and evolution rules from M0), but
//!   no neural/search code executes until its milestone.
//! - All structs use `deny_unknown_fields`: unknown or unsupported modes are
//!   rejected at load time, never silently ignored.
//! - Seed namespaces come from [`crate::rng::SUPPORTED_NAMESPACES`]; a
//!   missing or unknown namespace is an error, never a silent test-seed
//!   fallback.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The only schema version accepted by this implementation.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// Environment kinds named in spec 5.9 / 19.x.
pub const SUPPORTED_ENVIRONMENT_KINDS: &[&str] = &[
    "stationary_clean",
    "stationary_noisy",
    "isolated_reversal",
    "mixed_continual",
    "long_life",
    "uninformative_reward",
];

/// Reset policies. `birth_only` is the primary continuous condition;
/// explicitly named diagnostics are allowed but never silently substituted.
pub const SUPPORTED_RESET_POLICIES: &[&str] = &[
    "birth_only",
    "episodic_diagnostic",
    "event_reset_diagnostic",
];

/// Trace policies. `no_decay_diagnostic` belongs to the explicit episodic
/// score diagnostic (M2/M3) and is rejected alongside `birth_only`.
pub const SUPPORTED_TRACE_POLICIES: &[&str] = &["persistent", "no_decay_diagnostic"];

/// Plastic masks supported by the first implementation.
pub const SUPPORTED_PLASTIC_MASKS: &[&str] = &["all_recurrent_edges", "motor_afferent_only"];

/// Gate modes (heads themselves arrive in M6; the mode label is validated
/// from M0 so mismatched configs fail early).
pub const SUPPORTED_GATE_MODES: &[&str] = &["fixed", "global", "targeted"];

/// Search spaces for the later evolutionary engine.
pub const SUPPORTED_SEARCH_SPACES: &[&str] = &["gate_projection_only", "modulator_and_gate"];

/// Configuration load/validation errors.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ConfigError {
    #[error("unsupported execution: {0}")]
    UnsupportedExecution(String),
    #[error("cannot read config '{path}': {message}")]
    Io { path: String, message: String },
    #[error("cannot parse config '{path}': {message}")]
    Parse { path: String, message: String },
    #[error("unsupported schema_version {found}; this implementation accepts schema_version = 1")]
    UnsupportedSchemaVersion { found: u32 },
    #[error("invalid profile_name: must be non-empty")]
    InvalidProfileName,
    #[error("invalid simulation.dt {found}: this implementation requires dt = 1.0 (spec 4.3)")]
    InvalidDt { found: f64 },
    #[error("unsupported simulation.precision '{found}': this implementation requires \"f64\"")]
    UnsupportedPrecision { found: String },
    #[error("unknown simulation.reset_policy '{found}'")]
    UnknownResetPolicy { found: String },
    #[error("unknown simulation.feedback_order '{found}': expected \"before_neural_transition\"")]
    UnknownFeedbackOrder { found: String },
    #[error("invalid simulation.outcomes_per_lifetime {found}: must be >= 1")]
    InvalidLifetimeLength { found: u64 },
    #[error("unknown environment.kind '{found}'")]
    UnknownEnvironmentKind { found: String },
    #[error("invalid environment.cue_count {found}: must be >= 1")]
    InvalidCueCount { found: usize },
    #[error("unsupported environment.cue_encoding '{found}': expected \"one_hot\"")]
    UnsupportedCueEncoding { found: String },
    #[error("invalid environment.stable_fraction {found}: must be in [0, 1]")]
    InvalidStableFraction { found: f64 },
    #[error("invalid environment.feedback_noise_values: {reason}")]
    InvalidFeedbackNoise { reason: String },
    #[error("invalid environment.volatile_hazard_values: {reason}")]
    InvalidHazard { reason: String },
    #[error("unsupported environment.hazard_clock '{found}': expected \"cue_exposure\"")]
    UnsupportedHazardClock { found: String },
    #[error("invalid environment timing: {reason}")]
    InvalidTiming { reason: String },
    #[error("invalid environment.max_pending_choices {found}: the first protocol allows exactly 1")]
    InvalidPendingChoices { found: u64 },
    #[error(
        "unknown seeds.namespace '{found}'; expected one of development|training|validation|final_test"
    )]
    UnknownSeedNamespace { found: String },
    #[error("invalid actor dimensions: {reason}")]
    InvalidActorDimensions { reason: String },
    #[error("invalid actor time constants or scales: {reason}")]
    InvalidActorParams { reason: String },
    #[error("invalid learning section: {reason}")]
    InvalidLearning { reason: String },
    #[error("invalid modulator section: {reason}")]
    InvalidModulator { reason: String },
    #[error("invalid evolution section: {reason}")]
    InvalidEvolution { reason: String },
    #[error("invalid logging section: {reason}")]
    InvalidLogging { reason: String },
    #[error("inconsistent reset/trace policy: {reason}")]
    InconsistentResetPolicy { reason: String },
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub profile_name: String,
    pub simulation: Simulation,
    pub environment: Environment,
    #[serde(default)]
    pub actor: Option<Actor>,
    #[serde(default)]
    pub learning: Option<Learning>,
    #[serde(default)]
    pub modulator: Option<Modulator>,
    #[serde(default)]
    pub evolution: Option<Evolution>,
    pub logging: Logging,
    pub seeds: Seeds,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Simulation {
    pub dt: f64,
    pub precision: String,
    pub warmup_ticks: u64,
    pub outcomes_per_lifetime: u64,
    pub reset_policy: String,
    pub feedback_order: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub kind: String,
    pub cue_count: usize,
    pub cue_encoding: String,
    pub stable_fraction: f64,
    pub feedback_noise_values: Vec<f64>,
    pub volatile_hazard_values: Vec<f64>,
    pub hazard_clock: String,
    pub quiet_ticks: [u64; 2],
    pub cue_ticks: u64,
    pub memory_gap_ticks: [u64; 2],
    pub response_ticks: u64,
    pub reward_delay_ticks: [u64; 2],
    pub feedback_ticks: u64,
    pub max_pending_choices: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub neuron_count: usize,
    pub motor_neurons_per_action: usize,
    pub edge_probability: f64,
    pub self_edges: bool,
    pub recurrent_gain: f64,
    pub input_scale: f64,
    pub tau_h: f64,
    pub tau_a: f64,
    pub adaptation_strength: f64,
    pub noise_sigma: f64,
    pub motor_filter_tau: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Learning {
    pub enabled: bool,
    pub rule: String,
    pub plastic_mask: String,
    pub trace_policy: String,
    pub tau_e: f64,
    pub eta: f64,
    pub max_update: f64,
    pub plastic_bound: f64,
    pub plastic_decay: f64,
    pub reward_baseline_initial: f64,
    pub reward_baseline_beta: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Modulator {
    pub mode: String,
    pub neuron_count: usize,
    pub tau_m: f64,
    pub ordinary_feedback_to_actor: bool,
    pub gate_timing: String,
    pub gate_bias_initial: f64,
    pub projection_init_std: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Evolution {
    pub enabled: bool,
    pub population_size: u64,
    pub elite_count: u64,
    pub mutation_std: f64,
    pub search_space: String,
    #[serde(default)]
    pub decoded_weight_limit: Option<f64>,
    #[serde(default)]
    pub lifetimes_per_candidate: Option<u64>,
    #[serde(default)]
    pub generations: Option<u64>,
    #[serde(default)]
    pub validation_every_generations: Option<u64>,
    #[serde(default)]
    pub validation_lifetimes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Logging {
    pub event_log: bool,
    pub trace_every_ticks: u64,
    pub full_trace_lifetimes: u64,
    pub record_raw_and_applied_updates: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Seeds {
    pub root_seed: u64,
    pub namespace: String,
    pub outer_seed: u64,
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Read, parse, and validate a TOML config file.
pub fn load_and_validate(path: &Path) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    let cfg: Config = toml::from_str(&text).map_err(|e| ConfigError::Parse {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    validate(&cfg)?;
    Ok(cfg)
}

/// Serialize the fully resolved configuration for provenance records.
/// A small user override file is not sufficient provenance if defaults later
/// change, so every run manifest must embed this output.
pub fn resolved_toml(cfg: &Config) -> Result<String, ConfigError> {
    toml::to_string(cfg).map_err(|e| ConfigError::Parse {
        path: "<resolved>".to_owned(),
        message: e.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Validation (spec 19.4 plus forward-compatible neural/search checks)
// ---------------------------------------------------------------------------

fn in_01_closed(x: f64) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}

fn range_ok([lo, hi]: [u64; 2]) -> bool {
    lo <= hi
}

/// Validate every section of an already-parsed config.
pub fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(ConfigError::UnsupportedSchemaVersion {
            found: cfg.schema_version,
        });
    }
    if cfg.profile_name.trim().is_empty() {
        return Err(ConfigError::InvalidProfileName);
    }

    // --- simulation ---
    if cfg.simulation.dt != 1.0 || !cfg.simulation.dt.is_finite() {
        return Err(ConfigError::InvalidDt {
            found: cfg.simulation.dt,
        });
    }
    if cfg.simulation.precision != "f64" {
        return Err(ConfigError::UnsupportedPrecision {
            found: cfg.simulation.precision.clone(),
        });
    }
    if !SUPPORTED_RESET_POLICIES.contains(&cfg.simulation.reset_policy.as_str()) {
        return Err(ConfigError::UnknownResetPolicy {
            found: cfg.simulation.reset_policy.clone(),
        });
    }
    if cfg.simulation.feedback_order != "before_neural_transition" {
        return Err(ConfigError::UnknownFeedbackOrder {
            found: cfg.simulation.feedback_order.clone(),
        });
    }
    if cfg.simulation.outcomes_per_lifetime < 1 {
        return Err(ConfigError::InvalidLifetimeLength {
            found: cfg.simulation.outcomes_per_lifetime,
        });
    }

    validate_environment(&cfg.environment)?;
    // Bound the entire lifetime, including the clock increment after its
    // final feedback. Reject overflow before allocating or drawing RNG.
    let env = &cfg.environment;
    let cycle = env
        .cue_ticks
        .checked_add(env.memory_gap_ticks[1])
        .and_then(|n| n.checked_add(env.response_ticks))
        .and_then(|n| n.checked_add(env.reward_delay_ticks[1]));
    let max_ticks = cycle
        .and_then(|n| n.checked_add(env.quiet_ticks[1]))
        .and_then(|n| n.checked_mul(cfg.simulation.outcomes_per_lifetime - 1))
        .and_then(|n| n.checked_add(cycle?))
        .and_then(|n| n.checked_add(cfg.simulation.warmup_ticks));
    if max_ticks.is_none() {
        return Err(ConfigError::InvalidTiming {
            reason: "maximum lifetime tick count overflows u64".to_owned(),
        });
    }
    validate_seeds(&cfg.seeds)?;

    if let Some(actor) = &cfg.actor {
        validate_actor(actor)?;
    }
    if let Some(learning) = &cfg.learning {
        validate_learning(learning, cfg.actor.as_ref(), &cfg.simulation.reset_policy)?;
    }
    if let Some(modulator) = &cfg.modulator {
        validate_modulator(modulator)?;
    }
    if let Some(evolution) = &cfg.evolution {
        validate_evolution(evolution)?;
    }
    validate_logging(&cfg.logging)?;

    Ok(())
}

/// Validate the environment modes actually implemented by the tick driver.
/// Parsing a future profile does not authorize silently substituting a mode.
pub fn validate_environment_execution(cfg: &Config) -> Result<(), ConfigError> {
    validate(cfg)?;
    if matches!(
        cfg.environment.kind.as_str(),
        "isolated_reversal" | "long_life"
    ) {
        return Err(ConfigError::UnsupportedExecution(format!(
            "environment '{}' is reserved for a later milestone",
            cfg.environment.kind
        )));
    }
    if cfg.simulation.reset_policy != "birth_only" {
        return Err(ConfigError::UnsupportedExecution(
            "M0 supports only birth_only lifetimes".to_owned(),
        ));
    }
    Ok(())
}

/// Baseline simulation must not pretend to execute neural/search sections.
pub fn validate_baseline_execution(cfg: &Config) -> Result<(), ConfigError> {
    validate_environment_execution(cfg)?;
    if cfg.actor.is_some()
        || cfg.learning.is_some()
        || cfg.modulator.is_some()
        || cfg.evolution.is_some()
    {
        return Err(ConfigError::UnsupportedExecution(
            "M0 baseline simulation requires an environment-only config (use configs/env_smoke.toml); neural/search sections are not executed".to_owned(),
        ));
    }
    Ok(())
}

fn validate_environment(env: &Environment) -> Result<(), ConfigError> {
    if !SUPPORTED_ENVIRONMENT_KINDS.contains(&env.kind.as_str()) {
        return Err(ConfigError::UnknownEnvironmentKind {
            found: env.kind.clone(),
        });
    }
    if env.cue_count < 1 || env.cue_count.checked_add(6).is_none() {
        return Err(ConfigError::InvalidCueCount {
            found: env.cue_count,
        });
    }
    if env.cue_encoding != "one_hot" {
        return Err(ConfigError::UnsupportedCueEncoding {
            found: env.cue_encoding.clone(),
        });
    }
    if !in_01_closed(env.stable_fraction) {
        return Err(ConfigError::InvalidStableFraction {
            found: env.stable_fraction,
        });
    }
    if env.feedback_noise_values.is_empty() {
        return Err(ConfigError::InvalidFeedbackNoise {
            reason: "feedback_noise_values must list at least one rate".to_owned(),
        });
    }
    for &e in &env.feedback_noise_values {
        if !(e.is_finite() && (0.0..=0.5).contains(&e)) {
            return Err(ConfigError::InvalidFeedbackNoise {
                reason: format!("each rate must be in [0, 0.5]; found {e}"),
            });
        }
    }
    if env.volatile_hazard_values.is_empty() {
        return Err(ConfigError::InvalidHazard {
            reason: "volatile_hazard_values must list at least one rate".to_owned(),
        });
    }
    for &h in &env.volatile_hazard_values {
        if !(h.is_finite() && (0.0..=1.0).contains(&h)) {
            return Err(ConfigError::InvalidHazard {
                reason: format!("each hazard must be in [0, 1]; found {h}"),
            });
        }
    }
    if env.hazard_clock != "cue_exposure" {
        return Err(ConfigError::UnsupportedHazardClock {
            found: env.hazard_clock.clone(),
        });
    }
    if !range_ok(env.quiet_ticks) {
        return Err(ConfigError::InvalidTiming {
            reason: format!(
                "quiet_ticks min must be <= max; found {:?}",
                env.quiet_ticks
            ),
        });
    }
    if env.cue_ticks < 1 {
        return Err(ConfigError::InvalidTiming {
            reason: format!("cue_ticks must be >= 1; found {}", env.cue_ticks),
        });
    }
    if !range_ok(env.memory_gap_ticks) {
        return Err(ConfigError::InvalidTiming {
            reason: format!(
                "memory_gap_ticks min must be <= max; found {:?}",
                env.memory_gap_ticks
            ),
        });
    }
    if env.response_ticks < 1 {
        return Err(ConfigError::InvalidTiming {
            reason: format!("response_ticks must be >= 1; found {}", env.response_ticks),
        });
    }
    if !range_ok(env.reward_delay_ticks)
        || env.reward_delay_ticks[0] < 1
        || env.reward_delay_ticks[1] < 1
    {
        return Err(ConfigError::InvalidTiming {
            reason: format!(
                "reward_delay_ticks must satisfy 1 <= min <= max; found {:?}",
                env.reward_delay_ticks
            ),
        });
    }
    if env.feedback_ticks != 1 {
        return Err(ConfigError::InvalidTiming {
            reason: format!(
                "feedback_ticks must be exactly 1; found {}",
                env.feedback_ticks
            ),
        });
    }
    if env.max_pending_choices != 1 {
        return Err(ConfigError::InvalidPendingChoices {
            found: env.max_pending_choices,
        });
    }
    Ok(())
}

fn validate_seeds(seeds: &Seeds) -> Result<(), ConfigError> {
    if !crate::rng::is_supported_namespace(&seeds.namespace) {
        return Err(ConfigError::UnknownSeedNamespace {
            found: seeds.namespace.clone(),
        });
    }
    Ok(())
}

fn validate_actor(actor: &Actor) -> Result<(), ConfigError> {
    if actor.motor_neurons_per_action < 1 {
        return Err(ConfigError::InvalidActorDimensions {
            reason: format!(
                "motor_neurons_per_action must be >= 1; found {}",
                actor.motor_neurons_per_action
            ),
        });
    }
    if actor.motor_neurons_per_action > actor.neuron_count / 2 {
        return Err(ConfigError::InvalidActorDimensions {
            reason: format!(
                "neuron_count ({}) must hold two disjoint motor pools of {} each",
                actor.neuron_count, actor.motor_neurons_per_action
            ),
        });
    }
    if !(actor.edge_probability.is_finite() && (0.0..=1.0).contains(&actor.edge_probability)) {
        return Err(ConfigError::InvalidActorDimensions {
            reason: format!(
                "edge_probability must be in [0, 1]; found {}",
                actor.edge_probability
            ),
        });
    }
    if !(actor.tau_h.is_finite() && actor.tau_h > 0.0) {
        return Err(ConfigError::InvalidActorParams {
            reason: format!("tau_h must be > 0; found {}", actor.tau_h),
        });
    }
    if !(actor.tau_a.is_finite() && actor.tau_a > 0.0) {
        return Err(ConfigError::InvalidActorParams {
            reason: format!("tau_a must be > 0; found {}", actor.tau_a),
        });
    }
    if !(actor.motor_filter_tau.is_finite() && actor.motor_filter_tau > 0.0) {
        return Err(ConfigError::InvalidActorParams {
            reason: format!(
                "motor_filter_tau must be > 0; found {}",
                actor.motor_filter_tau
            ),
        });
    }
    if !(actor.noise_sigma.is_finite() && actor.noise_sigma > 0.0) {
        return Err(ConfigError::InvalidActorParams {
            reason: format!(
                "noise_sigma must be > 0 while the stochastic score is active; found {}",
                actor.noise_sigma
            ),
        });
    }
    for (name, v) in [
        ("recurrent_gain", actor.recurrent_gain),
        ("input_scale", actor.input_scale),
        ("adaptation_strength", actor.adaptation_strength),
    ] {
        if !v.is_finite() {
            return Err(ConfigError::InvalidActorParams {
                reason: format!("{name} must be finite; found {v}"),
            });
        }
    }
    Ok(())
}

fn validate_learning(
    learning: &Learning,
    actor: Option<&Actor>,
    reset_policy: &str,
) -> Result<(), ConfigError> {
    if learning.rule != "gaussian_transition_score" {
        return Err(ConfigError::InvalidLearning {
            reason: format!(
                "unknown rule '{}'; expected \"gaussian_transition_score\"",
                learning.rule
            ),
        });
    }
    if !SUPPORTED_PLASTIC_MASKS.contains(&learning.plastic_mask.as_str()) {
        return Err(ConfigError::InvalidLearning {
            reason: format!("unknown plastic_mask '{}'", learning.plastic_mask),
        });
    }
    if !SUPPORTED_TRACE_POLICIES.contains(&learning.trace_policy.as_str()) {
        return Err(ConfigError::InvalidLearning {
            reason: format!("unknown trace_policy '{}'", learning.trace_policy),
        });
    }
    if reset_policy == "birth_only" && learning.trace_policy != "persistent" {
        return Err(ConfigError::InconsistentResetPolicy {
            reason: format!(
                "reset_policy is birth_only but trace_policy is '{}'; \
                 diagnostic trace resets require an explicitly named diagnostic reset_policy",
                learning.trace_policy
            ),
        });
    }
    if !(learning.tau_e.is_finite() && learning.tau_e > 0.0) {
        return Err(ConfigError::InvalidLearning {
            reason: format!("tau_e must be > 0; found {}", learning.tau_e),
        });
    }
    if !(learning.eta.is_finite() && learning.eta >= 0.0) {
        return Err(ConfigError::InvalidLearning {
            reason: format!("eta must be >= 0 and finite; found {}", learning.eta),
        });
    }
    if !(learning.max_update.is_finite() && learning.max_update > 0.0) {
        return Err(ConfigError::InvalidLearning {
            reason: format!("max_update must be > 0; found {}", learning.max_update),
        });
    }
    if !(learning.plastic_bound.is_finite() && learning.plastic_bound > 0.0) {
        return Err(ConfigError::InvalidLearning {
            reason: format!(
                "plastic_bound must be > 0; found {}",
                learning.plastic_bound
            ),
        });
    }
    if learning.plastic_decay != 0.0 || !learning.plastic_decay.is_finite() {
        return Err(ConfigError::InvalidLearning {
            reason: format!(
                "plastic_decay must be exactly 0.0 in the first implementation (no hidden forgetting); found {}",
                learning.plastic_decay
            ),
        });
    }
    if !in_01_closed(learning.reward_baseline_initial) {
        return Err(ConfigError::InvalidLearning {
            reason: format!(
                "reward_baseline_initial must be in [0, 1]; found {}",
                learning.reward_baseline_initial
            ),
        });
    }
    if !(learning.reward_baseline_beta.is_finite()
        && (0.0..=1.0).contains(&learning.reward_baseline_beta))
    {
        return Err(ConfigError::InvalidLearning {
            reason: format!(
                "reward_baseline_beta must be in [0, 1]; found {}",
                learning.reward_baseline_beta
            ),
        });
    }
    if learning.enabled
        && learning.eta > 0.0
        && let Some(actor) = actor
        && !(actor.noise_sigma.is_finite() && actor.noise_sigma > 0.0)
    {
        return Err(ConfigError::InvalidLearning {
            reason: "score-noise contract: eta > 0 requires actor.noise_sigma > 0".to_owned(),
        });
    }
    Ok(())
}

fn validate_modulator(modulator: &Modulator) -> Result<(), ConfigError> {
    if !SUPPORTED_GATE_MODES.contains(&modulator.mode.as_str()) {
        return Err(ConfigError::InvalidModulator {
            reason: format!("unknown mode '{}'", modulator.mode),
        });
    }
    if !(modulator.tau_m.is_finite() && modulator.tau_m > 0.0) {
        return Err(ConfigError::InvalidModulator {
            reason: format!("tau_m must be > 0; found {}", modulator.tau_m),
        });
    }
    if modulator.ordinary_feedback_to_actor {
        return Err(ConfigError::InvalidModulator {
            reason:
                "ordinary_feedback_to_actor must be false in the initial architecture (spec 3.3/8)"
                    .to_owned(),
        });
    }
    if modulator.gate_timing != "pre_outcome" {
        return Err(ConfigError::InvalidModulator {
            reason: format!(
                "unknown gate_timing '{}'; expected \"pre_outcome\"",
                modulator.gate_timing
            ),
        });
    }
    for (name, v) in [
        ("gate_bias_initial", modulator.gate_bias_initial),
        ("projection_init_std", modulator.projection_init_std),
    ] {
        if !v.is_finite() {
            return Err(ConfigError::InvalidModulator {
                reason: format!("{name} must be finite; found {v}"),
            });
        }
    }
    if modulator.projection_init_std < 0.0 {
        return Err(ConfigError::InvalidModulator {
            reason: format!(
                "projection_init_std must be >= 0; found {}",
                modulator.projection_init_std
            ),
        });
    }
    Ok(())
}

fn validate_evolution(evolution: &Evolution) -> Result<(), ConfigError> {
    if evolution.population_size < 2 {
        return Err(ConfigError::InvalidEvolution {
            reason: format!(
                "population_size must be >= 2; found {}",
                evolution.population_size
            ),
        });
    }
    if evolution.elite_count == 0 || evolution.elite_count >= evolution.population_size {
        return Err(ConfigError::InvalidEvolution {
            reason: format!(
                "elite_count must be strictly between 0 and population_size ({}); found {}",
                evolution.population_size, evolution.elite_count
            ),
        });
    }
    if !(evolution.mutation_std.is_finite() && evolution.mutation_std > 0.0) {
        return Err(ConfigError::InvalidEvolution {
            reason: format!("mutation_std must be > 0; found {}", evolution.mutation_std),
        });
    }
    if !SUPPORTED_SEARCH_SPACES.contains(&evolution.search_space.as_str()) {
        return Err(ConfigError::InvalidEvolution {
            reason: format!("unknown search_space '{}'", evolution.search_space),
        });
    }
    if let Some(v) = evolution.decoded_weight_limit
        && !(v.is_finite() && v > 0.0)
    {
        return Err(ConfigError::InvalidEvolution {
            reason: format!("decoded_weight_limit must be > 0; found {v}"),
        });
    }
    for (name, v) in [
        ("lifetimes_per_candidate", evolution.lifetimes_per_candidate),
        ("generations", evolution.generations),
        (
            "validation_every_generations",
            evolution.validation_every_generations,
        ),
        ("validation_lifetimes", evolution.validation_lifetimes),
    ] {
        if let Some(n) = v
            && n < 1
        {
            return Err(ConfigError::InvalidEvolution {
                reason: format!("{name} must be >= 1 when set; found {n}"),
            });
        }
    }
    Ok(())
}

fn validate_logging(logging: &Logging) -> Result<(), ConfigError> {
    if logging.trace_every_ticks < 1 {
        return Err(ConfigError::InvalidLogging {
            reason: format!(
                "trace_every_ticks must be >= 1; found {}",
                logging.trace_every_ticks
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smoke_toml() -> String {
        r#"
schema_version = 1
profile_name = "test_smoke"

[simulation]
dt = 1.0
precision = "f64"
warmup_ticks = 32
outcomes_per_lifetime = 64
reset_policy = "birth_only"
feedback_order = "before_neural_transition"

[environment]
kind = "stationary_clean"
cue_count = 2
cue_encoding = "one_hot"
stable_fraction = 1.0
feedback_noise_values = [0.0]
volatile_hazard_values = [0.0]
hazard_clock = "cue_exposure"
quiet_ticks = [4, 4]
cue_ticks = 8
memory_gap_ticks = [0, 0]
response_ticks = 4
reward_delay_ticks = [1, 1]
feedback_ticks = 1
max_pending_choices = 1

[logging]
event_log = true
trace_every_ticks = 10
full_trace_lifetimes = 1
record_raw_and_applied_updates = true

[seeds]
root_seed = 1
namespace = "development"
outer_seed = 1
"#
        .to_owned()
    }

    #[test]
    fn minimal_smoke_profile_validates() {
        let cfg: Config = toml::from_str(&smoke_toml()).expect("smoke parses");
        validate(&cfg).expect("smoke validates");
    }
}
