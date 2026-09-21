//! Evaluator-only hidden state and annotations (M0-06).
//!
//! This module owns everything the ordinary agent must never see: preferred
//! actions `y[c]`, per-cue noise rates and hazards, stable/volatile
//! membership, exposure counters, change flags, and the per-outcome
//! annotation stream. It is imported by the environment (which generates
//! observations and consequences) and by the evaluator/logger — never by
//! agent code. Ordinary and hidden streams serialize to separate records
//! (joined only offline by explicit keys such as `event_id`).

use rand::Rng;
use rand::distr::{Bernoulli, Distribution};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use super::observation::SimError;

/// Stable versus volatile cue membership. Membership is assigned at birth by
/// shuffling cue ids; it is never encoded in cue magnitude, position, or
/// frequency (spec 5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CueRole {
    Stable,
    Volatile,
}

/// Hidden per-cue state: preferred action, noise rate, hazard, role, and the
/// number of times the cue has been presented so far.
#[derive(Clone, Debug, PartialEq)]
pub struct HiddenState {
    /// Preferred action `y[c]` in {0, 1}, sampled independently at birth.
    mapping: Vec<u8>,
    /// Feedback noise rate `epsilon[c]` per cue (spec 5.1).
    epsilon: Vec<f64>,
    /// Per-exposure flip hazard `hazard[c]` per cue (spec 5.2).
    hazard: Vec<f64>,
    /// Stable/volatile membership per cue.
    role: Vec<CueRole>,
    /// Presentations so far per cue. Hazard applies only when this is
    /// already positive at presentation time (never on first exposure).
    exposures: Vec<u64>,
    /// Whether the mapping flipped immediately before the most recent
    /// presentation of each cue (for the annotation stream).
    changed_before_presentation: Vec<bool>,
}

impl HiddenState {
    /// Sample birth state.
    ///
    /// - `mapping_init_rng`: draws each `y[c]` independently with p = 1/2.
    /// - `membership`: pre-shuffled cue ids; the first `n_stable` are stable.
    /// - `n_stable`: `round(K * stable_fraction)` from the config.
    /// - `noise_values` / `hazard_values`: config lists, cycled
    ///   deterministically (in cue-index order for noise, in shuffled order
    ///   for hazards). Full factorial counterbalancing arrives in M5-02;
    ///   until then the assignment rule is this documented cycling.
    pub fn at_birth(
        cue_count: usize,
        membership: &[usize],
        n_stable: usize,
        noise_values: &[f64],
        hazard_values: &[f64],
        mapping_init_rng: &mut ChaCha8Rng,
    ) -> Result<Self, SimError> {
        if cue_count == 0
            || n_stable > cue_count
            || membership.len() != cue_count
            || noise_values.is_empty()
            || hazard_values.is_empty()
        {
            return Err(SimError::InvalidConfiguration(
                "hidden-state birth needs full membership and nonempty noise/hazard lists"
                    .to_owned(),
            ));
        }
        let mut seen = vec![false; cue_count];
        for &cue in membership {
            if cue >= cue_count || seen[cue] {
                return Err(SimError::InvalidConfiguration(
                    "membership must be a permutation of all cue ids".to_owned(),
                ));
            }
            seen[cue] = true;
        }
        if noise_values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=0.5).contains(v))
            || hazard_values
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(SimError::InvalidConfiguration(
                "hidden-state noise must be in [0, 0.5] and hazard in [0, 1]".to_owned(),
            ));
        }
        let mapping: Vec<u8> = (0..cue_count)
            .map(|_| u8::from(mapping_init_rng.random_bool(0.5)))
            .collect();
        let epsilon: Vec<f64> = (0..cue_count)
            .map(|c| noise_values[c % noise_values.len()])
            .collect();
        let mut hazard = vec![0.0; cue_count];
        let mut role = vec![CueRole::Stable; cue_count];
        let mut volatile_rank = 0usize;
        for (rank, &cue) in membership.iter().enumerate() {
            if rank < n_stable {
                role[cue] = CueRole::Stable;
                hazard[cue] = 0.0;
            } else {
                role[cue] = CueRole::Volatile;
                hazard[cue] = hazard_values[volatile_rank % hazard_values.len()];
                volatile_rank += 1;
            }
        }
        Ok(Self {
            mapping,
            epsilon,
            hazard,
            role,
            exposures: vec![0; cue_count],
            changed_before_presentation: vec![false; cue_count],
        })
    }

    pub fn cue_count(&self) -> usize {
        self.mapping.len()
    }

    /// Preferred action `y[c]` at this moment (evaluator only).
    pub fn mapping(&self, cue: usize) -> u8 {
        self.mapping[cue]
    }

    /// Full mapping snapshot (evaluator only; used to prove quiet ticks and
    /// unrelated streams never mutate hidden state).
    pub fn mapping_snapshot(&self) -> Vec<u8> {
        self.mapping.clone()
    }

    pub fn epsilon(&self, cue: usize) -> f64 {
        self.epsilon[cue]
    }

    pub fn hazard(&self, cue: usize) -> f64 {
        self.hazard[cue]
    }

    pub fn role(&self, cue: usize) -> CueRole {
        self.role[cue]
    }

    pub fn exposures(&self, cue: usize) -> u64 {
        self.exposures[cue]
    }

    pub fn changed_before_presentation(&self, cue: usize) -> bool {
        self.changed_before_presentation[cue]
    }

    /// Present cue `c`: apply the per-exposure hazard flip before the first
    /// sensory tick, then count the exposure. Returns `(exposure_index,
    /// changed)`, where the index starts at 1 for the first presentation.
    ///
    /// Hazard is applied here and nowhere else: not on quiet ticks, not on
    /// global decisions, never at first presentation (spec 5.2, AGENTS.md).
    pub fn present(
        &mut self,
        cue: usize,
        mapping_change_rng: &mut ChaCha8Rng,
    ) -> Result<(u64, bool), SimError> {
        if cue >= self.mapping.len() {
            return Err(SimError::InvalidConfiguration(format!(
                "cue {cue} out of range for {} cues",
                self.mapping.len()
            )));
        }
        let mut changed = false;
        if self.exposures[cue] > 0 {
            let h = self.hazard[cue];
            let flip = Bernoulli::new(h)
                .map(|d| d.sample(mapping_change_rng))
                .map_err(|_| {
                    SimError::InvalidConfiguration(format!("invalid hazard {h} for cue {cue}"))
                })?;
            if flip {
                self.mapping[cue] = 1 - self.mapping[cue];
                changed = true;
            }
        }
        self.exposures[cue] += 1;
        self.changed_before_presentation[cue] = changed;
        Ok((self.exposures[cue], changed))
    }

    /// Snapshot the full hidden mapping state for a lifetime checkpoint
    /// (M1-09). Evaluator-side data; agent code must never call this.
    pub(crate) fn snapshot(&self) -> HiddenSnapshot {
        HiddenSnapshot {
            mapping: self.mapping.clone(),
            epsilon: self.epsilon.clone(),
            hazard: self.hazard.clone(),
            role: self.role.clone(),
            exposures: self.exposures.clone(),
            changed_before_presentation: self.changed_before_presentation.clone(),
        }
    }

    /// Restore hidden state from a checkpoint snapshot, validating shapes
    /// and value ranges instead of trusting stored bytes.
    pub(crate) fn restore(snapshot: HiddenSnapshot) -> Result<Self, SimError> {
        let inconsistent = |reason: String| SimError::InconsistentCheckpoint(reason);
        let n = snapshot.mapping.len();
        if n == 0 {
            return Err(inconsistent("hidden mapping must be non-empty".to_owned()));
        }
        for name in [
            "epsilon",
            "hazard",
            "role",
            "exposures",
            "changed_before_presentation",
        ] {
            let len = match name {
                "epsilon" => snapshot.epsilon.len(),
                "hazard" => snapshot.hazard.len(),
                "role" => snapshot.role.len(),
                "exposures" => snapshot.exposures.len(),
                _ => snapshot.changed_before_presentation.len(),
            };
            if len != n {
                return Err(inconsistent(format!(
                    "hidden {name} len {len} must match mapping len {n}"
                )));
            }
        }
        if snapshot.mapping.iter().any(|&y| y > 1) {
            return Err(inconsistent(
                "hidden mapping must hold actions 0/1".to_owned(),
            ));
        }
        if snapshot
            .epsilon
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=0.5).contains(v))
        {
            return Err(inconsistent(
                "hidden epsilon must be finite in [0, 0.5]".to_owned(),
            ));
        }
        if snapshot
            .hazard
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(inconsistent(
                "hidden hazard must be finite in [0, 1]".to_owned(),
            ));
        }
        Ok(Self {
            mapping: snapshot.mapping,
            epsilon: snapshot.epsilon,
            hazard: snapshot.hazard,
            role: snapshot.role,
            exposures: snapshot.exposures,
            changed_before_presentation: snapshot.changed_before_presentation,
        })
    }
}

/// Serializable hidden-state snapshot for lifetime checkpoints (M1-09).
/// Stored inside the checkpoint file only; never delivered to an agent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct HiddenSnapshot {
    pub mapping: Vec<u8>,
    pub epsilon: Vec<f64>,
    pub hazard: Vec<f64>,
    pub role: Vec<CueRole>,
    pub exposures: Vec<u64>,
    pub changed_before_presentation: Vec<bool>,
}

/// Evaluator-only per-outcome annotation (spec 20.2 hidden stream). Joined
/// offline to the ordinary event record by `event_id` / `choice_index`;
/// never delivered to a running agent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HiddenAnnotation {
    pub event_id: u64,
    pub choice_index: u64,
    pub cue_id: usize,
    pub target_at_commit: u8,
    pub latent_correctness: bool,
    pub noise_bit: bool,
    pub epsilon: f64,
    pub cue_hazard: f64,
    pub cue_exposure_index: u64,
    pub hidden_change_before_presentation: bool,
    pub stable_or_volatile: CueRole,
    pub commit_tick: u64,
    pub outcome_tick: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{SeedTuple, rng_for};

    fn birth_rng() -> ChaCha8Rng {
        rng_for(&SeedTuple::new(1, "development", 1, 0, "mapping_init")).expect("valid")
    }

    fn change_rng() -> ChaCha8Rng {
        rng_for(&SeedTuple::new(1, "development", 1, 0, "mapping_change")).expect("valid")
    }

    #[test]
    fn hazard_zero_never_flips_and_first_presentation_is_clean() {
        let mut hidden =
            HiddenState::at_birth(2, &[0, 1], 2, &[0.0], &[0.0], &mut birth_rng()).expect("birth");
        let before = hidden.mapping_snapshot();
        for _ in 0..5 {
            let (index, changed) = hidden.present(0, &mut change_rng()).expect("present");
            assert!(!changed);
            assert_eq!(index, hidden.exposures(0));
        }
        assert_eq!(hidden.mapping_snapshot(), before);
        assert_eq!(hidden.exposures(0), 5);
    }

    #[test]
    fn hazard_one_flips_every_repeat_but_not_the_first() {
        let mut hidden =
            HiddenState::at_birth(1, &[0], 0, &[0.0], &[1.0], &mut birth_rng()).expect("birth");
        assert_eq!(hidden.hazard(0), 1.0);
        let birth_mapping = hidden.mapping(0);
        let (first_index, first_changed) = hidden.present(0, &mut change_rng()).expect("present");
        assert_eq!((first_index, first_changed), (1, false));
        assert_eq!(hidden.mapping(0), birth_mapping);
        for exposure in 2..=6u64 {
            let (index, changed) = hidden.present(0, &mut change_rng()).expect("present");
            assert_eq!(index, exposure);
            assert!(changed, "hazard 1 must flip every repeat");
        }
        // Six presentations, five flips: back to the opposite of birth.
        assert_eq!(hidden.mapping(0), 1 - birth_mapping);
    }

    #[test]
    fn annotation_serializes_separately_from_observation() {
        let annotation = HiddenAnnotation {
            event_id: 3,
            choice_index: 3,
            cue_id: 1,
            target_at_commit: 0,
            latent_correctness: true,
            noise_bit: false,
            epsilon: 0.1,
            cue_hazard: 0.02,
            cue_exposure_index: 4,
            hidden_change_before_presentation: false,
            stable_or_volatile: CueRole::Volatile,
            commit_tick: 20,
            outcome_tick: 23,
        };
        let value = serde_json::to_value(&annotation).expect("serializes");
        for key in [
            "target_at_commit",
            "latent_correctness",
            "noise_bit",
            "cue_hazard",
            "stable_or_volatile",
            "hidden_change_before_presentation",
        ] {
            assert!(value.get(key).is_some(), "hidden stream keeps {key}");
        }
        assert!(value.get("features").is_none());
    }
}
