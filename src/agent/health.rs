//! Numerical health and selected actor traces (M1-08).
//!
//! Spec: 10.6 (`f64` reference, never clip `h`, conservative watchdog with
//! explicit failures) and 6.5 (observability needs distinguishable,
//! non-saturated dynamics — health reports saturation and motor margins so
//! M1-11 can judge usability).
//!
//! Scope: read-only observation of live actor/motor state. Nothing here
//! mutates `h`/`a`/`r`/`q`, draws simulation randomness, or changes the
//! tick loop: `HealthSummary::observe` and `TraceRecorder::maybe_record`
//! borrow state immutably, and [`selected_trace_indices`] is a pure
//! function of topology. The actor transition (`actor.rs`) already fails
//! on nonfinite `h_new`/`a_new` instead of clipping; this module builds on
//! that failure path with a conservative finite watchdog and file-ready
//! summaries, rather than replacing it.
//!
//! Conservative bounds (normal operation stays `O(1)`: recurrent drive
//! `O(1)` from row-scaled `W0`, sensory currents `O(0.5)` from `B` at scale
//! 0.3 over few active channels, leak `alpha_h ~= 0.18` at `tau_h = 5`,
//! noise `0.05`, adaptation a signed average of `tanh` activity, motor `q`
//! a filtered activity mean): the `1e4` watchdog sits ~1000x above any
//! healthy magnitude, so it catches blow-ups without false alarms on the
//! debug (`N = 16`) or main (`N = 60`) profiles. Saturation at `|r| > 0.9`
//! (`|h| > ~1.47`) marks the tanh shoulder where distinct cues stop
//! producing distinguishable activity.

use serde::{Deserialize, Serialize};

use crate::environment::SimError;

/// Version for health/trace JSON payloads. Bumped only with a documented
/// format change.
pub const HEALTH_SCHEMA_VERSION: u32 = 2;

/// Conservative finite-state watchdog: no healthy `|h|` may reach this.
/// See the module docs for the `O(1)`-vs-`1e4` rationale.
pub const WATCHDOG_H_ABS_MAX: f64 = 1e4;

/// Conservative finite-state watchdog for the slow adaptation average,
/// which tracks `tanh` activity and is likewise `O(1)` when healthy.
pub const WATCHDOG_A_ABS_MAX: f64 = 1e4;

/// Conservative finite-state watchdog for filtered motor outputs, which
/// average activity means and are `O(1)` when healthy.
pub const WATCHDOG_Q_ABS_MAX: f64 = 1e4;

/// Activity shoulder: `|r| > 0.9` counts as saturated for observability
/// summaries (`r = tanh(h)`, so this is `|h| > ~1.47`).
pub const SATURATION_R_ABS: f64 = 0.9;

/// Upper bound on the stable trace selection (two non-motor probes plus
/// one probe per motor pool; deduplicated and sorted).
pub const TRACE_SELECTION_BUDGET: usize = 4;

/// Explicit health failures. Nothing is clipped: a breach returns an
/// error and, for the recording entry points, leaves counters/samples
/// unmodified so a failure never looks like a successful quiet tick.
#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum HealthError {
    #[error("nonfinite state at tick {tick} in {component}")]
    NonFinite { tick: u64, component: String },
    #[error(
        "watchdog tripped at tick {tick} in {component}: magnitude {value} exceeds bound {bound}"
    )]
    WatchdogTripped {
        tick: u64,
        component: String,
        value: f64,
        bound: f64,
    },
    #[error("invalid trace configuration: {0}")]
    InvalidConfig(String),
}

impl HealthError {
    /// Map onto the shared simulation error without inventing a silent
    /// default: nonfinite stays nonfinite, a finite watchdog breach names
    /// its bound explicitly.
    pub fn to_sim_error(&self) -> SimError {
        match self {
            Self::NonFinite { tick, component } => SimError::NonFiniteState {
                tick: *tick,
                component: format!("health {component}"),
            },
            Self::WatchdogTripped {
                tick,
                component,
                value,
                bound,
            } => SimError::InvalidConfiguration(format!(
                "watchdog tripped at tick {tick} in {component}: magnitude {value} exceeds bound {bound}"
            )),
            Self::InvalidConfig(message) => {
                SimError::InvalidConfiguration(format!("health trace config: {message}"))
            }
        }
    }
}

/// Stable trace selection: indices `[0, 1, motor0[0], motor1[0]]`,
/// deduplicated, sorted, and filtered to `< neuron_count`, truncated to
/// [`TRACE_SELECTION_BUDGET`].
///
/// Pure function of topology: no RNG, no hidden state, identical on every
/// call. Covers two non-motor probes (when they exist) plus one probe per
/// motor pool so both actions stay observable. Degenerate `N = 2m`
/// topologies (no non-motor units) still yield both motor probes.
pub fn selected_trace_indices(
    neuron_count: usize,
    motor0: &[usize],
    motor1: &[usize],
) -> Vec<usize> {
    let mut selected = Vec::with_capacity(TRACE_SELECTION_BUDGET);
    for candidate in [0_usize, 1] {
        if candidate < neuron_count && !selected.contains(&candidate) {
            selected.push(candidate);
        }
    }
    for pool in [motor0, motor1] {
        if let Some(&probe) = pool.first()
            && probe < neuron_count
            && !selected.contains(&probe)
        {
            selected.push(probe);
        }
    }
    selected.sort_unstable();
    selected.truncate(TRACE_SELECTION_BUDGET);
    selected
}

/// Validate one state snapshot: finiteness first, then the conservative
/// finite watchdog. Draws nothing; mutates nothing.
pub fn check_state(
    tick: u64,
    h: &[f64],
    a: &[f64],
    r: &[f64],
    q: [f64; 2],
) -> Result<(), HealthError> {
    let finite = |name: &str, values: &[f64]| {
        if values.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(HealthError::NonFinite {
                tick,
                component: name.to_owned(),
            })
        }
    };
    finite("h", h)?;
    finite("a", a)?;
    finite("r", r)?;
    if !q.iter().all(|v| v.is_finite()) {
        return Err(HealthError::NonFinite {
            tick,
            component: "q".to_owned(),
        });
    }
    let breach = |name: &str, value: f64, bound: f64| {
        if value <= bound {
            Ok(())
        } else {
            Err(HealthError::WatchdogTripped {
                tick,
                component: name.to_owned(),
                value,
                bound,
            })
        }
    };
    breach(
        "h",
        h.iter().fold(0.0_f64, |m, v| m.max(v.abs())),
        WATCHDOG_H_ABS_MAX,
    )?;
    breach(
        "a",
        a.iter().fold(0.0_f64, |m, v| m.max(v.abs())),
        WATCHDOG_A_ABS_MAX,
    )?;
    breach(
        "q",
        q.iter().fold(0.0_f64, |m, v| m.max(v.abs())),
        WATCHDOG_Q_ABS_MAX,
    )?;
    Ok(())
}

/// Running numerical-health summary over observed ticks.
///
/// Updated only by successful [`observe`](Self::observe) calls: a failed
/// tick returns its [`HealthError`] and leaves every counter/extremum
/// untouched, so failures are explicit records rather than quiet zeros.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HealthSummary {
    pub schema_version: u32,
    pub ticks_observed: u64,
    pub max_abs_h: f64,
    pub max_abs_a: f64,
    pub max_abs_r: f64,
    pub max_abs_q: f64,
    pub saturated_neuron_ticks: u64,
    pub total_neuron_ticks: u64,
    pub min_motor_margin: f64,
    pub max_motor_margin: f64,
}

impl HealthSummary {
    /// Empty, finite and JSON-round-trippable summary. The first observation
    /// initializes the minimum margin; `min_margin` returns None until then.
    pub fn new() -> Self {
        Self {
            schema_version: HEALTH_SCHEMA_VERSION,
            ticks_observed: 0,
            max_abs_h: 0.0,
            max_abs_a: 0.0,
            max_abs_r: 0.0,
            max_abs_q: 0.0,
            saturated_neuron_ticks: 0,
            total_neuron_ticks: 0,
            min_motor_margin: 0.0,
            max_motor_margin: 0.0,
        }
    }

    /// Observe one tick: validate (finiteness + watchdog), then fold
    /// extrema, saturation counts, and motor margins. Fails without
    /// recording on any breach — never clips.
    pub fn observe(
        &mut self,
        tick: u64,
        h: &[f64],
        a: &[f64],
        r: &[f64],
        q: [f64; 2],
    ) -> Result<(), HealthError> {
        check_state(tick, h, a, r, q)?;
        self.max_abs_h = self
            .max_abs_h
            .max(h.iter().fold(0.0_f64, |m, v| m.max(v.abs())));
        self.max_abs_a = self
            .max_abs_a
            .max(a.iter().fold(0.0_f64, |m, v| m.max(v.abs())));
        self.max_abs_r = self
            .max_abs_r
            .max(r.iter().fold(0.0_f64, |m, v| m.max(v.abs())));
        self.max_abs_q = self
            .max_abs_q
            .max(q.iter().fold(0.0_f64, |m, v| m.max(v.abs())));
        let saturated = r.iter().filter(|v| v.abs() > SATURATION_R_ABS).count() as u64;
        self.saturated_neuron_ticks += saturated;
        self.total_neuron_ticks += r.len() as u64;
        let margin = (q[0] - q[1]).abs();
        self.min_motor_margin = if self.ticks_observed == 0 {
            margin
        } else {
            self.min_motor_margin.min(margin)
        };
        self.max_motor_margin = self.max_motor_margin.max(margin);
        self.ticks_observed += 1;
        Ok(())
    }

    /// Fraction of observed neuron-ticks with `|r| > 0.9`. `None` before
    /// the first observation (no silent zero).
    pub fn saturated_fraction(&self) -> Option<f64> {
        if self.total_neuron_ticks == 0 {
            None
        } else {
            Some(self.saturated_neuron_ticks as f64 / self.total_neuron_ticks as f64)
        }
    }

    /// Smallest observed `|q0 - q1|`. `None` before the first observation.
    pub fn min_margin(&self) -> Option<f64> {
        if self.ticks_observed == 0 {
            None
        } else {
            Some(self.min_motor_margin)
        }
    }
}

impl Default for HealthSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// One sampled snapshot of the stable selection plus the motor readout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceSample {
    pub tick: u64,
    pub h: Vec<f64>,
    pub a: Vec<f64>,
    pub r: Vec<f64>,
    pub q: [f64; 2],
}

/// Sampled trace over the stable selection.
///
/// Records at most one sample per `tick % every == 0`; all other ticks
/// return `Ok(false)` without touching `samples` or any RNG. A sampled
/// tick with nonfinite selected state fails instead of recording a
/// clipped placeholder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceRecorder {
    pub schema_version: u32,
    pub selection: Vec<usize>,
    pub every: u64,
    pub samples: Vec<TraceSample>,
}

impl TraceRecorder {
    /// Fix the selection from topology and the sampling period. `every`
    /// must be `>= 1` (mirrors `trace_every_ticks`); an empty selection is
    /// an explicit error, never a silent no-op recorder.
    pub fn new(
        neuron_count: usize,
        motor0: &[usize],
        motor1: &[usize],
        every: u64,
    ) -> Result<Self, HealthError> {
        if every < 1 {
            return Err(HealthError::InvalidConfig(format!(
                "trace_every must be >= 1; found {every}"
            )));
        }
        let selection = selected_trace_indices(neuron_count, motor0, motor1);
        if selection.is_empty() {
            return Err(HealthError::InvalidConfig(
                "trace selection is empty".to_owned(),
            ));
        }
        Ok(Self {
            schema_version: HEALTH_SCHEMA_VERSION,
            selection,
            every,
            samples: Vec::new(),
        })
    }

    /// Current stable selection (borrowed; observing it draws nothing).
    pub fn selection(&self) -> &[usize] {
        &self.selection
    }

    /// All recorded samples in tick order.
    pub fn samples(&self) -> &[TraceSample] {
        &self.samples
    }

    /// Record the selected state when `tick % every == 0`. Returns whether
    /// a sample was appended; non-sampled ticks succeed trivially.
    pub fn maybe_record(
        &mut self,
        tick: u64,
        h: &[f64],
        a: &[f64],
        q: [f64; 2],
    ) -> Result<bool, HealthError> {
        if !tick.is_multiple_of(self.every) {
            return Ok(false);
        }
        for &j in &self.selection {
            let (Some(&hj), Some(&aj)) = (h.get(j), a.get(j)) else {
                return Err(HealthError::InvalidConfig(format!(
                    "trace index {j} outside {}-neuron state",
                    h.len()
                )));
            };
            for (name, value) in [("h", hj), ("a", aj)] {
                if !value.is_finite() {
                    return Err(HealthError::NonFinite {
                        tick,
                        component: format!("trace {name}[{j}]"),
                    });
                }
            }
        }
        if !q.iter().all(|v| v.is_finite()) {
            return Err(HealthError::NonFinite {
                tick,
                component: "trace q".to_owned(),
            });
        }
        self.samples.push(TraceSample {
            tick,
            h: self.selection.iter().map(|&j| h[j]).collect(),
            a: self.selection.iter().map(|&j| a[j]).collect(),
            r: self.selection.iter().map(|&j| h[j].tanh()).collect(),
            q,
        });
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_and_bounds_are_pinned() {
        assert_eq!(HEALTH_SCHEMA_VERSION, 2);
        assert_eq!(WATCHDOG_H_ABS_MAX, 1e4);
        assert_eq!(WATCHDOG_A_ABS_MAX, 1e4);
        assert_eq!(WATCHDOG_Q_ABS_MAX, 1e4);
        assert_eq!(SATURATION_R_ABS, 0.9);
        assert_eq!(HealthSummary::new().schema_version, 2);
    }
}
