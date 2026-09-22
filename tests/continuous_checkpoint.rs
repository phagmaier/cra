//! M4-06 exact pause/resume for the fully persistent continuous learner.
//!
//! The manual driver below mirrors the authoritative production tick order
//! and is first pinned against `run_continuous_lifetime`. Replay splits cover
//! ongoing cue activity, the fully processed boundary immediately before due
//! feedback, and the first fully processed boundary after feedback. Every
//! split carries nonzero `P` and `E`; duplicate delivery is rejected without
//! mutation. Schema-2 M1 and schema-3 episodic loaders remain distinct.

use std::path::{Path, PathBuf};

use cra::agent::plasticity::{FeedbackOutcome, PlasticityError};
use cra::checkpoint::{
    Checkpoint, CheckpointError, ContinuousCheckpoint, LearningCheckpoint, SeedIdentity,
};
use cra::config::Config;
use cra::environment::{Feedback, Lifetime, Phase, TickOutput};
use cra::experiments::continuous::{
    ContinuousError, ContinuousLearner, run_continuous_lifetime, sample_matched_inheritance,
};
use cra::rng::{SeedTuple, rng_for};
use serde_json::Value;

fn continuous_config(outcomes: u64) -> Config {
    let text = std::fs::read_to_string("configs/continuous_variable_short.toml")
        .expect("continuous profile readable");
    let mut cfg: Config = toml::from_str(&text).expect("continuous profile parses");
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cfg.simulation.warmup_ticks = 8;
    cra::config::validate(&cfg).expect("config validates");
    cra::config::validate_continuous_execution(&cfg).expect("continuous config executes");
    cfg
}

fn l1(matrix: &[Vec<f64>]) -> f64 {
    matrix.iter().flatten().map(|v| v.abs()).sum()
}

#[derive(Clone, Debug, PartialEq)]
struct ChoiceRecord {
    event_id: u64,
    action: u8,
    reward: f64,
    correct: bool,
    update: FeedbackOutcome,
}

#[derive(Clone, Debug, PartialEq)]
struct FinalState {
    h: Vec<f64>,
    a: Vec<f64>,
    r: Vec<f64>,
    xi: Vec<f64>,
    q: [f64; 2],
    p: Vec<Vec<f64>>,
    e: Vec<Vec<f64>>,
    effective: Vec<Vec<f64>>,
    baseline: f64,
    last_feedback: Option<u64>,
    learner_ticks: u64,
    env_tick: u64,
    phase: Phase,
    pending: Option<cra::environment::PendingReward>,
    last_action: Option<u8>,
    consumed: Vec<u64>,
    outcomes: u64,
    commitments: u64,
}

struct ManualDriver {
    cfg: Config,
    lifetime: Lifetime,
    learner: ContinuousLearner,
    root_seed: u64,
    namespace: String,
    outer_seed: u64,
    lifetime_index: u64,
    choices: Vec<ChoiceRecord>,
}

impl ManualDriver {
    fn fresh(cfg: &Config, root: u64, namespace: &str, outer: u64, lifetime: u64) -> Self {
        let (inherited, _) =
            sample_matched_inheritance(cfg, root, namespace, outer).expect("inheritance samples");
        let noise = rng_for(&SeedTuple::new(
            root,
            namespace,
            outer,
            lifetime,
            "actor_noise",
        ))
        .expect("noise rng");
        let tie = rng_for(&SeedTuple::new(
            root,
            namespace,
            outer,
            lifetime,
            "tie_break",
        ))
        .expect("tie rng");
        let learner = ContinuousLearner::from_agent_parts(
            cfg.actor.clone().expect("actor config"),
            cfg.learning.clone().expect("learning config"),
            inherited,
            cfg.environment.cue_count,
            noise,
            tie,
        )
        .expect("learner builds");
        let lifetime_driver =
            Lifetime::new(cfg, root, namespace, outer, lifetime).expect("lifetime builds");
        Self {
            cfg: cfg.clone(),
            lifetime: lifetime_driver,
            learner,
            root_seed: root,
            namespace: namespace.to_owned(),
            outer_seed: outer,
            lifetime_index: lifetime,
            choices: Vec::new(),
        }
    }

    fn seeds(&self) -> SeedIdentity {
        SeedIdentity {
            root_seed: self.root_seed,
            namespace: self.namespace.clone(),
            outer_seed: self.outer_seed,
            lifetime_index: self.lifetime_index,
        }
    }

    fn process_observed(&mut self, out: TickOutput) -> Option<Feedback> {
        let delivered = out.observation.feedback;
        if let Some(feedback) = delivered {
            let update = self
                .learner
                .apply_feedback(feedback)
                .expect("feedback applies once");
            self.lifetime
                .note_feedback_consumed(feedback.event_id)
                .expect("delivery confirmed once");
            let annotation = out.annotation.as_ref().expect("feedback annotation");
            self.choices.push(ChoiceRecord {
                event_id: feedback.event_id,
                action: self.lifetime.last_action().expect("committed action"),
                reward: feedback.reward,
                correct: annotation.latent_correctness,
                update,
            });
        }
        self.learner
            .advance(&out.observation.features)
            .expect("learner advances");
        self.lifetime.finish_tick().expect("tick finishes");
        if out.commitment_due {
            let action = self.learner.select_action();
            self.lifetime.commit(action).expect("action commits");
        }
        delivered
    }

    fn step_tick(&mut self) -> Option<Feedback> {
        let out = self.lifetime.observe().expect("tick observes");
        self.process_observed(out)
    }

    fn step_outcome(&mut self) {
        loop {
            if self.step_tick().is_some() {
                break;
            }
        }
    }

    fn run_outcomes(&mut self, outcomes: u64) {
        for _ in 0..outcomes {
            self.step_outcome();
        }
    }

    fn run_to_phase(&mut self, phase: Phase) {
        while self.lifetime.phase() != phase {
            assert!(
                !self.lifetime.is_complete(),
                "target phase must be reachable"
            );
            self.step_tick();
        }
    }

    fn run_to_completion(&mut self) {
        while !self.lifetime.is_complete() {
            self.step_tick();
        }
    }

    fn capture(&self, path: &Path) {
        ContinuousCheckpoint::capture(&self.lifetime, &self.learner, &self.cfg, self.seeds())
            .expect("continuous checkpoint captures")
            .save_to_path(path)
            .expect("continuous checkpoint saves");
    }

    fn resume(path: &Path, carry: ManualDriver) -> Self {
        let loaded = ContinuousCheckpoint::load_from_path(path).expect("checkpoint loads");
        let seeds = carry.seeds();
        let lifetime = loaded.restore_env(&seeds).expect("environment restores");
        let learner = loaded
            .restore_learner(&seeds)
            .expect("continuous learner restores");
        Self {
            cfg: carry.cfg,
            lifetime,
            learner,
            root_seed: carry.root_seed,
            namespace: carry.namespace,
            outer_seed: carry.outer_seed,
            lifetime_index: carry.lifetime_index,
            choices: carry.choices,
        }
    }

    fn final_state(&self) -> FinalState {
        FinalState {
            h: self.learner.actor_state().h().to_vec(),
            a: self.learner.actor_state().a().to_vec(),
            r: self.learner.actor_state().r().to_vec(),
            xi: self.learner.actor_state().last_perturbations().to_vec(),
            q: self.learner.motor_state().q(),
            p: self.learner.plastic().p().to_vec(),
            e: self.learner.plastic().e().to_vec(),
            effective: self.learner.plastic().effective_weights().to_vec(),
            baseline: self.learner.reward_baseline(),
            last_feedback: self.learner.last_feedback(),
            learner_ticks: self.learner.ticks_advanced(),
            env_tick: self.lifetime.tick(),
            phase: self.lifetime.phase(),
            pending: self.lifetime.pending().cloned(),
            last_action: self.lifetime.last_action(),
            consumed: self.lifetime.consumed_ids().to_vec(),
            outcomes: self.lifetime.outcomes(),
            commitments: self.lifetime.commitments(),
        }
    }
}

fn tmp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cra-continuous-{name}-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ))
}

fn remove_quiet(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn assert_matches_reference(resumed: &ManualDriver, reference: &ManualDriver) {
    assert_eq!(resumed.choices, reference.choices);
    assert_eq!(resumed.final_state(), reference.final_state());
}

#[test]
fn manual_driver_matches_the_verified_continuous_runner() {
    let cfg = continuous_config(40);
    let mut manual = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    manual.run_to_completion();
    let reference =
        run_continuous_lifetime(&cfg, 1, "development", 2, 0, "B4").expect("runner succeeds");
    let actions: Vec<u8> = manual.choices.iter().map(|c| c.action).collect();
    let rewards: Vec<f64> = manual.choices.iter().map(|c| c.reward).collect();
    let correct: Vec<bool> = manual.choices.iter().map(|c| c.correct).collect();
    assert_eq!(
        actions,
        reference
            .choices
            .iter()
            .map(|c| c.action)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        rewards,
        reference
            .choices
            .iter()
            .map(|c| c.reward)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        correct,
        reference
            .choices
            .iter()
            .map(|c| c.correct)
            .collect::<Vec<_>>()
    );
    assert_eq!(manual.learner.plastic().p(), reference.final_p);
    assert_eq!(manual.learner.plastic().e(), reference.final_e);
    assert_eq!(
        manual.learner.plastic().effective_weights(),
        reference.final_effective
    );
    assert_eq!(manual.learner.reward_baseline(), reference.final_baseline);
    assert_eq!(
        manual.learner.last_feedback(),
        reference.final_last_feedback
    );
    assert_eq!(manual.lifetime.tick(), reference.ticks);
}

#[test]
fn ongoing_activity_split_with_nonzero_p_and_e_resumes_exactly() {
    let cfg = continuous_config(40);
    let path = tmp_path("ongoing");
    let mut split = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    split.run_outcomes(10);
    split.run_to_phase(Phase::Cue);
    split.step_tick();
    assert_eq!(split.lifetime.phase(), Phase::Cue);
    assert!(l1(split.learner.plastic().p()) > 0.0);
    assert!(l1(split.learner.plastic().e()) > 0.0);
    assert!(split.learner.actor_state().h().iter().any(|v| *v != 0.0));
    split.capture(&path);
    let mut resumed = ManualDriver::resume(&path, split);
    resumed.run_to_completion();
    remove_quiet(&path);

    let mut reference = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    reference.run_to_completion();
    assert_matches_reference(&resumed, &reference);
}

#[test]
fn immediately_before_feedback_split_preserves_pending_reward_and_latch() {
    let cfg = continuous_config(40);
    let path = tmp_path("prefeedback");
    let mut split = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    split.run_outcomes(10);
    split.run_to_phase(Phase::Feedback);
    assert!(l1(split.learner.plastic().p()) > 0.0);
    assert!(l1(split.learner.plastic().e()) > 0.0);
    let pending = split.lifetime.pending().cloned().expect("reward pending");
    let previous_action = split.lifetime.last_action().expect("action latch set");
    split.capture(&path);

    let loaded = ContinuousCheckpoint::load_from_path(&path).expect("checkpoint loads");
    let restored_env = loaded
        .restore_env(loaded.seeds())
        .expect("environment restores");
    assert_eq!(restored_env.phase(), Phase::Feedback);
    assert_eq!(restored_env.pending(), Some(&pending));
    assert_eq!(restored_env.last_action(), Some(previous_action));

    let expected_delivery = split.lifetime.observe().expect("original delivery");
    let mut resumed = ManualDriver::resume(&path, split);
    let resumed_delivery = resumed.lifetime.observe().expect("resumed delivery");
    assert_eq!(resumed_delivery, expected_delivery);
    resumed.process_observed(resumed_delivery);
    resumed.run_to_completion();
    remove_quiet(&path);

    let mut reference = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    reference.run_to_completion();
    assert_matches_reference(&resumed, &reference);
}

#[test]
fn post_feedback_split_rejects_duplicate_without_state_change() {
    let cfg = continuous_config(40);
    let path = tmp_path("postfeedback");
    let mut split = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    split.run_outcomes(10);
    split.run_to_phase(Phase::Feedback);
    let delivered = split.step_tick().expect("feedback delivered and applied");
    assert_eq!(split.learner.last_feedback(), Some(delivered.event_id));
    assert!(l1(split.learner.plastic().p()) > 0.0);
    assert!(l1(split.learner.plastic().e()) > 0.0);
    split.capture(&path);
    let mut resumed = ManualDriver::resume(&path, split);

    let before = resumed.final_state();
    assert!(matches!(
        resumed.learner.apply_feedback(delivered),
        Err(ContinuousError::Plastic(PlasticityError::DuplicateFeedback(id)))
            if id == delivered.event_id
    ));
    assert_eq!(
        resumed.final_state(),
        before,
        "duplicate must be mutation-free"
    );
    assert!(matches!(
        resumed.lifetime.note_feedback_consumed(delivered.event_id),
        Err(cra::environment::SimError::DuplicateFeedback(id)) if id == delivered.event_id
    ));
    assert_eq!(
        resumed.final_state(),
        before,
        "ledger duplicate must be mutation-free"
    );

    resumed.run_to_completion();
    remove_quiet(&path);
    let mut reference = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    reference.run_to_completion();
    assert_matches_reference(&resumed, &reference);
}

#[test]
fn missing_or_incompatible_continuous_fields_are_rejected() {
    let cfg = continuous_config(16);
    let original = tmp_path("rejections");
    let mut driver = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    driver.run_outcomes(8);
    driver.run_to_phase(Phase::Feedback);
    driver.capture(&original);
    let source: Value =
        serde_json::from_str(&std::fs::read_to_string(&original).expect("checkpoint readable"))
            .expect("checkpoint JSON");

    let missing_paths: &[&[&str]] = &[
        &["payload", "agent", "effective_weights"],
        &["payload", "agent", "plastic", "reward_baseline"],
        &["payload", "agent", "plastic", "last_feedback"],
        &["payload", "env", "pending"],
        &["payload", "env", "last_action"],
    ];
    for (index, path) in missing_paths.iter().enumerate() {
        let mut value = source.clone();
        let mut parent = &mut value;
        for key in &path[..path.len() - 1] {
            parent = parent.get_mut(*key).expect("path exists");
        }
        parent
            .as_object_mut()
            .expect("parent object")
            .remove(path[path.len() - 1]);
        let slot = tmp_path(&format!("missing-{index}"));
        std::fs::write(
            &slot,
            serde_json::to_vec_pretty(&value).expect("JSON encodes"),
        )
        .expect("fixture writes");
        assert!(matches!(
            ContinuousCheckpoint::load_from_path(&slot),
            Err(CheckpointError::Parse { .. })
        ));
        remove_quiet(&slot);
    }

    let mut bad_cache = source.clone();
    bad_cache["payload"]["agent"]["effective_weights"][0][0] = Value::from(123.0);
    let cache_path = tmp_path("bad-cache");
    std::fs::write(
        &cache_path,
        serde_json::to_vec_pretty(&bad_cache).expect("JSON encodes"),
    )
    .expect("fixture writes");
    let cache_error = ContinuousCheckpoint::load_from_path(&cache_path).expect_err("bad cache");
    assert!(
        matches!(cache_error, CheckpointError::ChecksumMismatch),
        "unexpected cache error: {cache_error:?}"
    );
    remove_quiet(&cache_path);

    // Both established loaders reject schema 4, and schema 4 rejects a real
    // schema-3 episodic archive. The schema-2 loader remains covered by its
    // unchanged checkpoint suite.
    assert!(Checkpoint::load_from_path(&original).is_err());
    assert!(LearningCheckpoint::load_from_path(&original).is_err());
    assert!(
        ContinuousCheckpoint::load_from_path(Path::new(
            "docs/evidence/m3-10/checkpoints/final.json"
        ))
        .is_err()
    );
    remove_quiet(&original);
}
