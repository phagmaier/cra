//! Declared development sweep for the episodic learner (M3-06).
//!
//! Spec: 16/M3 ("tune only on development seeds: `eta`, input scale,
//! recurrent gain, and noise amplitude. Use a small explicit grid rather
//! than unconstrained manual changes").
//!
//! Scope: this module loads and validates the frozen pre-results
//! declaration in `manifests/m3_development_grid.json`. It never executes
//! a lifetime and never observes an outcome: `instantiate` expands grid
//! points into candidate configs and proves each one passes
//! `validate_episodic_execution`, so M3-07 can run the sweep without
//! re-deciding it. Grid values, seeds, windows, criterion, tiebreak order,
//! and budget are all read from the manifest, never defaulted in code.
//!
//! Combination order is fixed and documented: `eta` major, then
//! `input_scale`, `recurrent_gain`, `noise_sigma` minor
//! (`index = (((e * I + i) * G + g) * S + s)`). The spec's suggested
//! final-200 accuracy above 0.8 is recorded in the manifest as a debugging
//! target, never as a pass/fail threshold.

use serde::{Deserialize, Serialize};

use crate::config::{Config, validate_episodic_execution};

/// Manifest schema version. Bumped only with a documented format change.
pub const GRID_SCHEMA_VERSION: u32 = 1;

/// Allowed `selection_tiebreak` tokens, in the only vocabulary the
/// validator accepts.
pub const ALLOWED_TIEBREAK_TOKENS: &[&str] = &[
    "larger_min_margin",
    "smaller_eta",
    "smaller_input_scale",
    "smaller_recurrent_gain",
    "smaller_noise_sigma",
    "smaller_grid_index",
];

/// Grid declaration failures. Every variant is an explicit error, never a
/// silent default or an invented value.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum GridError {
    #[error("cannot read grid manifest '{path}': {message}")]
    Io { path: String, message: String },
    #[error("cannot parse grid manifest '{path}': {message}")]
    Parse { path: String, message: String },
    #[error("invalid development grid: {0}")]
    Invalid(String),
}

/// Hyperparameter axes. Each list must be non-empty; every value must be
/// finite and strictly positive (the score contract needs `sigma > 0`
/// wherever `eta > 0`, and all grid etas are positive).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridAxes {
    pub eta: Vec<f64>,
    pub input_scale: Vec<f64>,
    pub recurrent_gain: Vec<f64>,
    pub noise_sigma: Vec<f64>,
}

/// Pass/fail criterion for one grid point, evaluated per outer seed on the
/// late window and aggregated across seeds (M3-07 owns execution).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridCriterion {
    pub margin_b4_minus_b3: f64,
    pub margin_b4_minus_shuffled: f64,
    pub min_outer_seeds_passing: usize,
    pub max_failed_lifetimes: u64,
    pub max_clipped_update_fraction: f64,
    pub require_b4_final_p_movement: bool,
}

/// Declared compute budget. `total_lifetimes` must equal
/// `grid_points x seeds_per_point x conditions_per_seed` exactly;
/// `estimated_ticks` must equal the nominal derivation from the base
/// profile's maximum cycle length, so the estimate is computed, not
/// invented.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridBudget {
    pub grid_points: usize,
    pub seeds_per_point: usize,
    pub conditions_per_seed: usize,
    pub total_lifetimes: usize,
    pub estimated_ticks: u64,
    pub note: String,
}

/// Frozen M3-06 development sweep declaration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentGrid {
    pub schema_version: u32,
    pub task: String,
    pub purpose: String,
    pub base_profile: String,
    pub namespace: String,
    pub root_seed: u64,
    pub outer_seeds: Vec<u64>,
    pub lifetime_indices: Vec<u64>,
    pub outcomes_per_lifetime: u64,
    pub early_window_choices: u64,
    pub late_window_choices: u64,
    pub grid: GridAxes,
    pub conditions: Vec<String>,
    pub metric: String,
    pub criterion: GridCriterion,
    pub selection_tiebreak: Vec<String>,
    pub on_no_pass: String,
    pub budget: GridBudget,
    pub spec_target_note: String,
}

/// One expanded grid point with its fixed combination index.
#[derive(Clone, Debug, PartialEq)]
pub struct GridPoint {
    pub index: usize,
    pub eta: f64,
    pub input_scale: f64,
    pub recurrent_gain: f64,
    pub noise_sigma: f64,
}

/// Read and validate a grid manifest. Returns the frozen declaration.
pub fn load(path: &std::path::Path) -> Result<DevelopmentGrid, GridError> {
    let text = std::fs::read_to_string(path).map_err(|e| GridError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    let grid: DevelopmentGrid = serde_json::from_str(&text).map_err(|e| GridError::Parse {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    grid.validate()?;
    Ok(grid)
}

impl DevelopmentGrid {
    /// Validate every declaration invariant. Called by [`load`] and
    /// directly by tests.
    pub fn validate(&self) -> Result<(), GridError> {
        let bad = |reason: String| GridError::Invalid(reason);
        if self.schema_version != GRID_SCHEMA_VERSION {
            return Err(bad(format!(
                "unsupported schema_version {}; this implementation reads {GRID_SCHEMA_VERSION}",
                self.schema_version
            )));
        }
        if self.task != "M3-06" {
            return Err(bad(format!("task must be 'M3-06'; found '{}'", self.task)));
        }
        if self.namespace != "development" {
            return Err(bad(format!(
                "grid namespace must be 'development'; found '{}'",
                self.namespace
            )));
        }
        if self.outer_seeds.is_empty() || self.lifetime_indices.is_empty() {
            return Err(bad(
                "outer_seeds and lifetime_indices must each list at least one seed".to_owned(),
            ));
        }
        let mut sorted = self.outer_seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != self.outer_seeds.len() {
            return Err(bad("outer_seeds must be distinct".to_owned()));
        }
        // Development reservation is outer 1-9999 (see manifests/README.md).
        if self.outer_seeds.iter().any(|s| *s < 1 || *s > 9999) {
            return Err(bad(format!(
                "outer_seeds must lie in the development reservation 1-9999; found {:?}",
                self.outer_seeds
            )));
        }
        for (name, values) in [
            ("eta", &self.grid.eta),
            ("input_scale", &self.grid.input_scale),
            ("recurrent_gain", &self.grid.recurrent_gain),
            ("noise_sigma", &self.grid.noise_sigma),
        ] {
            if values.is_empty() {
                return Err(bad(format!("grid.{name} must list at least one value")));
            }
            if values.iter().any(|v| !v.is_finite() || *v <= 0.0) {
                return Err(bad(format!(
                    "grid.{name} must be finite and > 0; found {values:?}"
                )));
            }
        }
        if self.outcomes_per_lifetime < 1 {
            return Err(bad(format!(
                "outcomes_per_lifetime must be >= 1; found {}",
                self.outcomes_per_lifetime
            )));
        }
        if self.early_window_choices < 1 || self.late_window_choices < 1 {
            return Err(bad(format!(
                "windows must each hold >= 1 choice; found early {} late {}",
                self.early_window_choices, self.late_window_choices
            )));
        }
        if self.early_window_choices + self.late_window_choices > self.outcomes_per_lifetime {
            return Err(bad(format!(
                "early ({}) + late ({}) windows must not overlap in a {}-choice lifetime",
                self.early_window_choices, self.late_window_choices, self.outcomes_per_lifetime
            )));
        }
        if self.conditions != vec!["B3", "B4", "B4-shuffled"] {
            return Err(bad(format!(
                "conditions must be exactly [B3, B4, B4-shuffled]; found {:?}",
                self.conditions
            )));
        }
        if self.metric != "late_window_latent_accuracy" {
            return Err(bad(format!(
                "metric must be 'late_window_latent_accuracy'; found '{}'",
                self.metric
            )));
        }
        let c = &self.criterion;
        for (name, v) in [
            ("margin_b4_minus_b3", c.margin_b4_minus_b3),
            ("margin_b4_minus_shuffled", c.margin_b4_minus_shuffled),
            ("max_clipped_update_fraction", c.max_clipped_update_fraction),
        ] {
            if !(v.is_finite() && (0.0..=1.0).contains(&v)) {
                return Err(bad(format!(
                    "criterion.{name} must be finite in [0, 1]; found {v}"
                )));
            }
        }
        if c.min_outer_seeds_passing < 1 || c.min_outer_seeds_passing > self.outer_seeds.len() {
            return Err(bad(format!(
                "min_outer_seeds_passing must lie in 1..={}; found {}",
                self.outer_seeds.len(),
                c.min_outer_seeds_passing
            )));
        }
        if self.selection_tiebreak.is_empty() {
            return Err(bad(
                "selection_tiebreak must list at least one token".to_owned()
            ));
        }
        for token in &self.selection_tiebreak {
            if !ALLOWED_TIEBREAK_TOKENS.contains(&token.as_str()) {
                return Err(bad(format!(
                    "unknown tiebreak token '{token}'; allowed: {ALLOWED_TIEBREAK_TOKENS:?}"
                )));
            }
        }
        if self.on_no_pass.trim().is_empty() || self.spec_target_note.trim().is_empty() {
            return Err(bad(
                "on_no_pass and spec_target_note must be non-empty declarations".to_owned(),
            ));
        }
        // Budget must be exactly self-consistent.
        let points = self.grid_points();
        let seeds = self.outer_seeds.len() * self.lifetime_indices.len();
        let budget = &self.budget;
        if budget.grid_points != points
            || budget.seeds_per_point != seeds
            || budget.conditions_per_seed != self.conditions.len()
            || budget.total_lifetimes != points * seeds * self.conditions.len()
        {
            return Err(bad(format!(
                "budget must satisfy grid_points {points} x seeds {seeds} x conditions {} = total {}; found {budget:?}",
                self.conditions.len(),
                points * seeds * self.conditions.len(),
            )));
        }
        Ok(())
    }

    /// Number of hyperparameter combinations in the fixed nesting order.
    #[must_use]
    pub fn grid_points(&self) -> usize {
        self.grid.eta.len()
            * self.grid.input_scale.len()
            * self.grid.recurrent_gain.len()
            * self.grid.noise_sigma.len()
    }

    /// The combination at `index` in the documented nesting order
    /// (`eta` major, `noise_sigma` minor). Returns `None` out of range.
    #[must_use]
    pub fn combination(&self, index: usize) -> Option<GridPoint> {
        let (e_len, i_len, g_len, s_len) = (
            self.grid.eta.len(),
            self.grid.input_scale.len(),
            self.grid.recurrent_gain.len(),
            self.grid.noise_sigma.len(),
        );
        if index >= e_len * i_len * g_len * s_len {
            return None;
        }
        let mut rest = index;
        let s = rest % s_len;
        rest /= s_len;
        let g = rest % g_len;
        rest /= g_len;
        let i = rest % i_len;
        let e = rest / i_len;
        Some(GridPoint {
            index,
            eta: self.grid.eta[e],
            input_scale: self.grid.input_scale[i],
            recurrent_gain: self.grid.recurrent_gain[g],
            noise_sigma: self.grid.noise_sigma[s],
        })
    }

    /// Expand every grid point into a candidate config cloned from `base`
    /// with the four swept values plus the declared lifetime length
    /// applied. Each candidate must pass `validate_episodic_execution`;
    /// a failure names its grid index instead of running anything. No
    /// lifetime is executed and no outcome observed here.
    pub fn instantiate(&self, base: &Config) -> Result<Vec<(GridPoint, Config)>, GridError> {
        if base.profile_name != "episodic_stationary" {
            return Err(GridError::Invalid(format!(
                "grid base must be the episodic_stationary profile; found '{}'",
                base.profile_name
            )));
        }
        let points = self.grid_points();
        let mut out = Vec::with_capacity(points);
        for index in 0..points {
            let point = self.combination(index).expect("index in range");
            let mut cfg = base.clone();
            let actor = cfg.actor.as_mut().ok_or_else(|| {
                GridError::Invalid("base profile requires an [actor] section".to_owned())
            })?;
            actor.input_scale = point.input_scale;
            actor.recurrent_gain = point.recurrent_gain;
            actor.noise_sigma = point.noise_sigma;
            let learning = cfg.learning.as_mut().ok_or_else(|| {
                GridError::Invalid("base profile requires a [learning] section".to_owned())
            })?;
            learning.eta = point.eta;
            cfg.simulation.outcomes_per_lifetime = self.outcomes_per_lifetime;
            validate_episodic_execution(&cfg).map_err(|e| {
                GridError::Invalid(format!("grid point {index} is not executable: {e}"))
            })?;
            out.push((point, cfg));
        }
        Ok(out)
    }

    /// Nominal tick estimate for the whole sweep, derived from the base
    /// profile's maximum cycle length (max quiet + cue + max gap + response
    /// + one feedback tick) times declared lifetimes and outcomes.
    ///
    /// The manifest's `estimated_ticks` must equal this derivation exactly.
    pub fn nominal_ticks(&self, base: &Config) -> Result<u64, GridError> {
        let env = &base.environment;
        let cycle = env.quiet_ticks[1]
            .checked_add(env.cue_ticks)
            .and_then(|n| n.checked_add(env.memory_gap_ticks[1]))
            .and_then(|n| n.checked_add(env.response_ticks))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| GridError::Invalid("tick estimate overflows u64".to_owned()))?;
        let total = self.budget.total_lifetimes as u64;
        total
            .checked_mul(self.outcomes_per_lifetime)
            .and_then(|n| n.checked_mul(cycle))
            .ok_or_else(|| GridError::Invalid("tick estimate overflows u64".to_owned()))
    }

    /// Check the manifest's declared estimate against the derivation.
    /// Call after [`Self::validate`]; `base` is the loaded base profile.
    pub fn check_budget_estimate(&self, base: &Config) -> Result<(), GridError> {
        let nominal = self.nominal_ticks(base)?;
        if self.budget.estimated_ticks != nominal {
            return Err(GridError::Invalid(format!(
                "budget.estimated_ticks {} must equal the nominal derivation {nominal} from the base profile timing",
                self.budget.estimated_ticks
            )));
        }
        Ok(())
    }
}
