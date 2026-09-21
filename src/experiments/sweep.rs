//! M3-07 acquisition analysis over the frozen M3-06 grid.
//!
//! Spec: 16/M3 (exit needs learning over matched no-update and
//! shuffled-reward controls across several seeds, with valid numerics).
//!
//! Scope: pure analysis over [`EpisodicSummary`]s. Windowed latent accuracy
//! and observed reward, per-event clipped-update fractions, plastic-bound
//! occupancy, offset/trace norms, per-seed margin checks against the frozen
//! [`DevelopmentGrid`] criterion, and deterministic winner selection by the
//! manifest tiebreak order. No simulation here; the explicitly invoked sweep
//! test owns execution. Learning-health quantities (clipping, bound
//! occupancy) stand in for activity-saturation traces, which the episodic
//! summaries do not record; that limitation is stated, not hidden.

use serde::{Deserialize, Serialize};

use crate::experiments::episodic::{EpisodicNoLearningSummary, EpisodicSummary};
use crate::experiments::grid::DevelopmentGrid;

/// Windowed behavior for one lifetime: latent accuracy and observed reward
/// over the first (`early`) and final (`late`) declared windows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowStats {
    pub early_accuracy: f64,
    pub early_reward: f64,
    pub late_accuracy: f64,
    pub late_reward: f64,
    pub full_accuracy: f64,
    pub full_reward: f64,
}

/// Learning-health diagnostics for one plastic lifetime: clipped-update
/// fraction (per-edge events where `limited != raw` over events with
/// nonzero `raw`), plastic-bound occupancy (fraction of `N x N` entries
/// with `|P| == bound`), and offset/trace norms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HealthStats {
    pub clipped_update_fraction: f64,
    pub clipped_events: u64,
    pub nonzero_update_events: u64,
    pub bound_occupancy: f64,
    pub final_p_l1: f64,
    pub final_p_l2: f64,
    pub final_e_l1: f64,
    pub final_e_l2: f64,
    pub final_baseline: f64,
}

/// Per-seed paired outcome for one grid point: windowed behavior for all
/// three conditions plus the two declared margins on late accuracy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeedOutcome {
    pub grid_index: usize,
    pub outer_seed: u64,
    pub b3: WindowStats,
    pub b4: WindowStats,
    pub shuffled: WindowStats,
    pub b4_health: HealthStats,
    pub shuffled_health: HealthStats,
    pub margin_b4_minus_b3: f64,
    pub margin_b4_minus_shuffled: f64,
}

/// Verdict for one grid point under the frozen criterion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointVerdict {
    pub grid_index: usize,
    pub eta: f64,
    pub input_scale: f64,
    pub recurrent_gain: f64,
    pub noise_sigma: f64,
    pub seeds_passing: usize,
    pub seeds_total: usize,
    pub min_margin_b4_b3: f64,
    pub min_margin_b4_shuffled: f64,
    pub passes: bool,
    pub failure_reasons: Vec<String>,
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Windowed accuracy/reward from parallel `correct`/`reward` slices.
/// `early` covers `[0, early_len)`; `late` covers the final `late_len`.
/// Empty windows yield `0.0` (lengths are validated positive by the grid).
pub fn window_stats(
    correct: &[bool],
    reward: &[f64],
    early_len: usize,
    late_len: usize,
) -> WindowStats {
    assert_eq!(correct.len(), reward.len());
    let n = correct.len();
    let acc = |range: std::ops::Range<usize>| {
        if range.is_empty() {
            0.0
        } else {
            range.clone().filter(|&k| correct[k]).count() as f64 / range.len() as f64
        }
    };
    let rew = |range: std::ops::Range<usize>| {
        if range.is_empty() {
            0.0
        } else {
            mean(&reward[range])
        }
    };
    WindowStats {
        early_accuracy: acc(0..early_len.min(n)),
        early_reward: rew(0..early_len.min(n)),
        late_accuracy: acc(n.saturating_sub(late_len)..n),
        late_reward: rew(n.saturating_sub(late_len)..n),
        full_accuracy: acc(0..n),
        full_reward: rew(0..n),
    }
}

/// Learning-health diagnostics from one plastic summary. Bound occupancy
/// counts exact `|P| == bound` entries (the clamp writes exact bounds);
/// nonplastic/missing entries hold `0.0` and never count.
pub fn health_stats(summary: &EpisodicSummary, plastic_bound: f64) -> HealthStats {
    let mut clipped = 0u64;
    let mut nonzero = 0u64;
    for choice in &summary.choices {
        for (raw_row, lim_row) in choice
            .update
            .raw_updates
            .iter()
            .zip(choice.update.limited_updates.iter())
        {
            for (&raw, &limited) in raw_row.iter().zip(lim_row.iter()) {
                if raw != 0.0 {
                    nonzero += 1;
                    if limited != raw {
                        clipped += 1;
                    }
                }
            }
        }
    }
    let total_entries = (summary.final_p.len() * summary.final_p.len()) as f64;
    let at_bound = summary
        .final_p
        .iter()
        .flatten()
        .filter(|v| v.abs() == plastic_bound)
        .count() as f64;
    let l1 = |m: &[Vec<f64>]| m.iter().flatten().map(|v| v.abs()).sum::<f64>();
    let l2 = |m: &[Vec<f64>]| m.iter().flatten().map(|v| v * v).sum::<f64>().sqrt();
    HealthStats {
        clipped_update_fraction: if nonzero == 0 {
            0.0
        } else {
            clipped as f64 / nonzero as f64
        },
        clipped_events: clipped,
        nonzero_update_events: nonzero,
        bound_occupancy: if total_entries == 0.0 {
            0.0
        } else {
            at_bound / total_entries
        },
        final_p_l1: l1(&summary.final_p),
        final_p_l2: l2(&summary.final_p),
        final_e_l1: l1(&summary.final_e),
        final_e_l2: l2(&summary.final_e),
        final_baseline: summary.final_baseline,
    }
}

/// The three matched summaries for one seed, built by
/// [`run_episodic_conditions`](crate::experiments::episodic::run_episodic_conditions).
pub struct MatchedConditions<'a> {
    pub b3: &'a EpisodicNoLearningSummary,
    pub b4: &'a EpisodicSummary,
    pub shuffled: &'a EpisodicSummary,
}

/// Build one seed outcome from the three matched summaries. Margins use
/// late-window latent accuracy exactly as the manifest declares.
pub fn seed_outcome(
    grid_index: usize,
    outer_seed: u64,
    early_len: usize,
    late_len: usize,
    plastic_bound: f64,
    conditions: MatchedConditions<'_>,
) -> SeedOutcome {
    let take =
        |choices: &[bool], rewards: &[f64]| window_stats(choices, rewards, early_len, late_len);
    let (b3, b4, shuffled) = (conditions.b3, conditions.b4, conditions.shuffled);
    let b3_stats = take(
        &b3.choices.iter().map(|c| c.correct).collect::<Vec<_>>(),
        &b3.choices.iter().map(|c| c.reward).collect::<Vec<_>>(),
    );
    let b4_stats = take(
        &b4.choices.iter().map(|c| c.correct).collect::<Vec<_>>(),
        &b4.choices.iter().map(|c| c.reward).collect::<Vec<_>>(),
    );
    let sh_stats = take(
        &shuffled
            .choices
            .iter()
            .map(|c| c.correct)
            .collect::<Vec<_>>(),
        &shuffled
            .choices
            .iter()
            .map(|c| c.reward)
            .collect::<Vec<_>>(),
    );
    let margin_b4_minus_b3 = b4_stats.late_accuracy - b3_stats.late_accuracy;
    let margin_b4_minus_shuffled = b4_stats.late_accuracy - sh_stats.late_accuracy;
    SeedOutcome {
        grid_index,
        outer_seed,
        b3: b3_stats,
        b4: b4_stats,
        shuffled: sh_stats,
        b4_health: health_stats(b4, plastic_bound),
        shuffled_health: health_stats(shuffled, plastic_bound),
        margin_b4_minus_b3,
        margin_b4_minus_shuffled,
    }
}

/// Judge one grid point: every seed must clear both margins; guardrails
/// (zero failures is enforced by construction — a failed lifetime never
/// reaches this function — plus clipping and `P` movement) apply per seed.
/// Returns the verdict with machine-checkable failure reasons.
pub fn judge_point(
    grid: &DevelopmentGrid,
    grid_index: usize,
    eta: f64,
    input_scale: f64,
    recurrent_gain: f64,
    noise_sigma: f64,
    seeds: &[SeedOutcome],
) -> PointVerdict {
    let c = &grid.criterion;
    let mut passing = 0usize;
    let mut reasons = Vec::new();
    let mut min_b3 = f64::INFINITY;
    let mut min_sh = f64::INFINITY;
    for s in seeds {
        min_b3 = min_b3.min(s.margin_b4_minus_b3);
        min_sh = min_sh.min(s.margin_b4_minus_shuffled);
        let mut seed_ok = true;
        if s.margin_b4_minus_b3 < c.margin_b4_minus_b3 {
            seed_ok = false;
        }
        if s.margin_b4_minus_shuffled < c.margin_b4_minus_shuffled {
            seed_ok = false;
        }
        if s.b4_health.clipped_update_fraction > c.max_clipped_update_fraction {
            seed_ok = false;
        }
        if c.require_b4_final_p_movement && s.b4_health.final_p_l1 == 0.0 {
            seed_ok = false;
        }
        if seed_ok {
            passing += 1;
        } else {
            reasons.push(format!(
                "outer {}: margin_b4_b3 {:.4} margin_b4_sh {:.4} clipped {:.4} p_l1 {:.6}",
                s.outer_seed,
                s.margin_b4_minus_b3,
                s.margin_b4_minus_shuffled,
                s.b4_health.clipped_update_fraction,
                s.b4_health.final_p_l1
            ));
        }
    }
    PointVerdict {
        grid_index,
        eta,
        input_scale,
        recurrent_gain,
        noise_sigma,
        seeds_passing: passing,
        seeds_total: seeds.len(),
        min_margin_b4_b3: if min_b3.is_infinite() { 0.0 } else { min_b3 },
        min_margin_b4_shuffled: if min_sh.is_infinite() { 0.0 } else { min_sh },
        passes: passing >= c.min_outer_seeds_passing,
        failure_reasons: reasons,
    }
}

/// Select the winning grid point by the manifest tiebreak order:
/// larger minimum margin first, then smaller hyperparameters, then smaller
/// grid index. Returns `None` when no point passes (M3-09 path, gate open).
/// The comparison key is exact (no tolerance): ties on one key fall
/// through to the next, and the trailing grid index makes it total.
pub fn select_winner(verdicts: &[PointVerdict]) -> Option<usize> {
    verdicts
        .iter()
        .filter(|v| v.passes)
        .min_by(|a, b| {
            b.min_margin_b4_b3
                .total_cmp(&a.min_margin_b4_b3)
                .then_with(|| a.eta.total_cmp(&b.eta))
                .then_with(|| a.input_scale.total_cmp(&b.input_scale))
                .then_with(|| a.recurrent_gain.total_cmp(&b.recurrent_gain))
                .then_with(|| a.noise_sigma.total_cmp(&b.noise_sigma))
                .then_with(|| a.grid_index.cmp(&b.grid_index))
        })
        .map(|v| v.grid_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_split_early_and_late() {
        // 10 choices: first 3 correct, last 3 correct, middle wrong.
        let correct = vec![
            true, true, true, false, false, false, false, true, true, true,
        ];
        let reward: Vec<f64> = correct.iter().map(|&c| f64::from(c)).collect();
        let w = window_stats(&correct, &reward, 3, 3);
        assert_eq!(w.early_accuracy, 1.0);
        assert_eq!(w.late_accuracy, 1.0);
        assert_eq!(w.full_accuracy, 0.6);
        assert_eq!(w.full_reward, 0.6);
    }

    #[test]
    fn judge_and_select_follow_the_manifest_order() {
        let text = std::fs::read_to_string("manifests/m3_development_grid.json").unwrap();
        let grid: DevelopmentGrid = serde_json::from_str(&text).unwrap();
        let mk = |outer: u64, m_b3: f64, m_sh: f64| SeedOutcome {
            grid_index: 0,
            outer_seed: outer,
            b3: WindowStats {
                early_accuracy: 0.5,
                early_reward: 0.5,
                late_accuracy: 0.5,
                late_reward: 0.5,
                full_accuracy: 0.5,
                full_reward: 0.5,
            },
            b4: WindowStats {
                early_accuracy: 0.5,
                early_reward: 0.5,
                late_accuracy: 0.5 + m_b3,
                late_reward: 0.5 + m_b3,
                full_accuracy: 0.6,
                full_reward: 0.6,
            },
            shuffled: WindowStats {
                early_accuracy: 0.5,
                early_reward: 0.5,
                late_accuracy: 0.5 + m_b3 - m_sh,
                late_reward: 0.5,
                full_accuracy: 0.5,
                full_reward: 0.5,
            },
            b4_health: HealthStats {
                clipped_update_fraction: 0.0,
                clipped_events: 0,
                nonzero_update_events: 100,
                bound_occupancy: 0.0,
                final_p_l1: 1.0,
                final_p_l2: 0.5,
                final_e_l1: 2.0,
                final_e_l2: 1.0,
                final_baseline: 0.6,
            },
            shuffled_health: HealthStats {
                clipped_update_fraction: 0.0,
                clipped_events: 0,
                nonzero_update_events: 100,
                bound_occupancy: 0.0,
                final_p_l1: 0.5,
                final_p_l2: 0.25,
                final_e_l1: 2.0,
                final_e_l2: 1.0,
                final_baseline: 0.5,
            },
            margin_b4_minus_b3: m_b3,
            margin_b4_minus_shuffled: m_sh,
        };
        // Two passing seeds, one failing: 2/3 clears the bar.
        let v = judge_point(
            &grid,
            7,
            0.001,
            0.3,
            0.8,
            0.05,
            &[mk(1, 0.2, 0.15), mk(2, 0.3, 0.2), mk(3, 0.0, 0.0)],
        );
        assert!(v.passes);
        assert_eq!(v.seeds_passing, 2);
        assert_eq!(v.failure_reasons.len(), 1);
        // Larger min margin wins regardless of hyperparameters.
        let lo = PointVerdict {
            grid_index: 1,
            eta: 0.0003,
            input_scale: 0.2,
            recurrent_gain: 0.5,
            noise_sigma: 0.03,
            seeds_passing: 3,
            seeds_total: 3,
            min_margin_b4_b3: 0.2,
            min_margin_b4_shuffled: 0.15,
            passes: true,
            failure_reasons: vec![],
        };
        let hi = PointVerdict {
            grid_index: 9,
            eta: 0.003,
            input_scale: 0.3,
            recurrent_gain: 0.8,
            noise_sigma: 0.05,
            seeds_passing: 3,
            seeds_total: 3,
            min_margin_b4_b3: 0.3,
            min_margin_b4_shuffled: 0.2,
            passes: true,
            failure_reasons: vec![],
        };
        assert_eq!(select_winner(&[lo.clone(), hi.clone()]), Some(9));
        // Exact tie on margin falls to smaller eta.
        let hi2 = PointVerdict {
            min_margin_b4_b3: 0.2,
            ..hi
        };
        assert_eq!(select_winner(&[hi2.clone(), lo.clone()]), Some(1));
        // No passer selects nothing.
        let fail = PointVerdict {
            passes: false,
            ..lo.clone()
        };
        assert_eq!(select_winner(&[fail]), None);
    }
}
