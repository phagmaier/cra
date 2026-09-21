//! Learned-offset checkpoints and exact replay through learning events
//! (M3-10; spec 10.7).
//!
//! - A faithful manual driver reproduces `run_episodic_lifetime`
//!   bit-for-bit (proving the split tests resume the same scientific
//!   object, not a reimplementation).
//! - Checkpoint splits at three learning-sensitive points — a
//!   rollout boundary with nonzero `P`, a pre-feedback tick with a
//!   delivered-but-unconsumed reward, and a post-feedback tick with
//!   fresh `P` and post-outcome scores in `E` — each resume to a
//!   continuation identical to the uninterrupted run.
//! - Rejection paths: tampered bytes, foreign seed identity, garbage
//!   input, and cross-schema loads (M1 schema 2 files are not learning
//!   lifetimes and vice versa).
//! - Archive pins: `docs/evidence/m3-10/working_config.toml` validates
//!   as the winner configuration, and the archived learned-offset
//!   checkpoint reloads with nonzero offsets.
//! - Bounded archival capture (ignored; invoke explicitly in release):
//!   a 2,000-outcome winner run saves boundary/final checkpoint files
//!   plus distilled update summaries to a fresh directory for archiving
//!   under `docs/evidence/m3-10/checkpoints/`.

use cra::checkpoint::{Checkpoint, CheckpointError, LearningCheckpoint, SeedIdentity};
use cra::config::Config;
use cra::environment::{Lifetime, TickOutput, schedule::Phase};
use cra::experiments::episodic::{
    EpisodicLearner, run_episodic_lifetime, sample_matched_inheritance,
};
use cra::rng::{SeedTuple, rng_for};
use std::path::{Path, PathBuf};

fn winner_full_mask_config(outcomes: u64) -> Config {
    let text =
        std::fs::read_to_string("configs/episodic_stationary.toml").expect("episodic profile");
    let mut cfg: Config = toml::from_str(&text).expect("episodic parses");
    // Frozen M3-07 winner (grid index 11); literals here so this suite
    // cannot silently re-tune (the grid manifest owns them).
    let actor = cfg.actor.as_mut().expect("base has [actor]");
    actor.input_scale = 0.2;
    actor.recurrent_gain = 0.8;
    actor.noise_sigma = 0.05;
    let learning = cfg.learning.as_mut().expect("base has [learning]");
    learning.eta = 0.001;
    learning.plastic_mask = "all_recurrent_edges".to_owned();
    cfg.simulation.outcomes_per_lifetime = outcomes;
    cra::config::validate(&cfg).expect("config validates");
    cra::config::validate_episodic_execution(&cfg).expect("config executes");
    cfg
}

/// Distilled per-outcome update report (the full `FeedbackOutcome`
/// matrices stay in the run; the archive keeps norms plus identity).
#[derive(Clone, Debug)]
struct UpdateDigest {
    event_id: u64,
    reward: f64,
    delta: f64,
    baseline_old: f64,
    baseline_new: f64,
    raw_l1: f64,
    limited_l1: f64,
    actual_l1: f64,
    clipped: bool,
    p_l1_after: f64,
}

fn l1(matrix: &[Vec<f64>]) -> f64 {
    matrix.iter().flatten().map(|v| v.abs()).sum()
}

/// Final learning state for continuation-equality checks:
/// (`P`, `E`, baseline, last feedback, env tick).
type Finals = (Vec<Vec<f64>>, Vec<Vec<f64>>, f64, Option<u64>, u64);

/// Manual episodic driver mirroring `run_episodic_lifetime` tick for
/// tick, with explicit per-phase methods so tests can capture
/// checkpoints between the driver steps.
struct ManualDriver {
    cfg: Config,
    lifetime: Lifetime,
    learner: EpisodicLearner,
    root_seed: u64,
    namespace: String,
    outer_seed: u64,
    lifetime_index: u64,
    actions: Vec<u8>,
    rewards: Vec<f64>,
    correct: Vec<bool>,
    resets: Vec<u64>,
    digests: Vec<UpdateDigest>,
}

impl ManualDriver {
    fn fresh(cfg: &Config, root: u64, ns: &str, outer: u64, li: u64) -> Self {
        let (params, init) = sample_matched_inheritance(cfg, root, ns, outer).expect("inheritance");
        let noise = rng_for(&SeedTuple::new(root, ns, outer, li, "actor_noise")).expect("rng");
        let tie = rng_for(&SeedTuple::new(root, ns, outer, li, "tie_break")).expect("rng");
        let mut learner = EpisodicLearner::from_agent_parts(
            cfg.actor.clone().expect("actor"),
            cfg.learning.clone().expect("learning"),
            params,
            cfg.environment.cue_count,
            noise,
            tie,
        )
        .expect("learner builds");
        learner.set_initialization(init);
        let lifetime = Lifetime::new(cfg, root, ns, outer, li).expect("lifetime builds");
        Self {
            cfg: cfg.clone(),
            lifetime,
            learner,
            root_seed: root,
            namespace: ns.to_owned(),
            outer_seed: outer,
            lifetime_index: li,
            actions: Vec::new(),
            rewards: Vec::new(),
            correct: Vec::new(),
            resets: vec![0],
            digests: Vec::new(),
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

    fn tick(&mut self) -> TickOutput {
        self.lifetime.advance().expect("tick advances")
    }

    fn handle_feedback(&mut self, out: &TickOutput) {
        if let Some(feedback) = out.observation.feedback {
            let update = self
                .learner
                .apply_feedback(feedback)
                .expect("feedback applies");
            self.lifetime
                .note_feedback_consumed(feedback.event_id)
                .expect("consumption noted");
            let annotation = out.annotation.clone().expect("annotation present");
            let action = self.lifetime.last_action().expect("commitment present");
            self.actions.push(action);
            self.rewards.push(feedback.reward);
            self.correct.push(annotation.latent_correctness);
            let p_l1_after = l1(self.learner.plastic().p());
            let mut clipped = false;
            for (raw_row, lim_row) in update.raw_updates.iter().zip(update.limited_updates.iter()) {
                for (&raw, &lim) in raw_row.iter().zip(lim_row.iter()) {
                    if raw != lim {
                        clipped = true;
                    }
                }
            }
            self.digests.push(UpdateDigest {
                event_id: feedback.event_id,
                reward: feedback.reward,
                delta: update.delta,
                baseline_old: update.baseline_old,
                baseline_new: update.baseline_new,
                raw_l1: l1(&update.raw_updates),
                limited_l1: l1(&update.limited_updates),
                actual_l1: l1(&update.actual_updates),
                clipped,
                p_l1_after,
            });
        }
    }

    fn handle_advance(&mut self, out: &TickOutput) {
        self.learner
            .advance(&out.observation.features)
            .expect("learner advances");
    }

    fn handle_commit(&mut self, out: &TickOutput) {
        if out.commitment_due {
            let action = self.learner.select_action();
            self.lifetime.commit(action).expect("commit accepted");
        }
    }

    fn handle_reset(&mut self, out: &TickOutput) {
        if out.observation.feedback.is_some() && !self.lifetime.is_complete() {
            self.learner
                .reset_between_rollouts()
                .expect("diagnostic reset");
            self.resets.push(self.lifetime.tick());
        }
    }

    /// One full outcome: ticks until a feedback is fully processed.
    fn step_outcome(&mut self) {
        loop {
            let out = self.tick();
            let had_feedback = out.observation.feedback.is_some();
            self.handle_feedback(&out);
            self.handle_advance(&out);
            self.handle_commit(&out);
            self.handle_reset(&out);
            if had_feedback {
                break;
            }
        }
    }

    /// Tick (with advance/commit, no reset possible yet) until a tick
    /// delivering feedback arrives; returns that tick unprocessed.
    fn run_to_next_feedback(&mut self) -> TickOutput {
        loop {
            let out = self.tick();
            if out.observation.feedback.is_some() {
                return out;
            }
            self.handle_advance(&out);
            self.handle_commit(&out);
        }
    }

    /// Tick until a feedback delivery, capturing each fully-processed
    /// tick to `slot`; returns the delivering tick unprocessed. On
    /// return, `slot` holds the just-before-feedback state: aligned
    /// ticks, the rollout's full eligibility, prior offsets, and no
    /// pending delivery (a delivered-but-unconsumed reward is driver-
    /// held state the checkpoint must not pretend to carry).
    fn run_to_just_before_feedback(&mut self, slot: &Path) -> TickOutput {
        loop {
            let out = self.tick();
            if out.observation.feedback.is_some() {
                return out;
            }
            self.handle_advance(&out);
            self.handle_commit(&out);
            self.capture(slot);
        }
    }

    fn run_outcomes(&mut self, n: u64) {
        for _ in 0..n {
            self.step_outcome();
        }
    }

    fn finals(&self) -> Finals {
        (
            self.learner.plastic().p().to_vec(),
            self.learner.plastic().e().to_vec(),
            self.learner.reward_baseline(),
            self.learner.last_feedback(),
            self.lifetime.tick(),
        )
    }

    fn capture(&self, path: &Path) {
        LearningCheckpoint::capture(&self.lifetime, &self.learner, &self.cfg, self.seeds())
            .expect("capture succeeds")
            .save_to_path(path)
            .expect("checkpoint saves");
    }

    fn resume(cfg: &Config, path: &Path, seeds: SeedIdentity, carry: ManualDriver) -> Self {
        let loaded = LearningCheckpoint::load_from_path(path).expect("checkpoint loads");
        let lifetime = loaded.restore_env(&seeds).expect("env resumes");
        let learner = loaded.restore_learner(&seeds).expect("learner resumes");
        Self {
            cfg: cfg.clone(),
            lifetime,
            learner,
            root_seed: carry.root_seed,
            namespace: carry.namespace,
            outer_seed: carry.outer_seed,
            lifetime_index: carry.lifetime_index,
            actions: carry.actions,
            rewards: carry.rewards,
            correct: carry.correct,
            resets: carry.resets,
            digests: carry.digests,
        }
    }
}

fn tmp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("cra-learning-{name}-{}.json", std::process::id()))
}

fn remove_quiet(path: &Path) {
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// Driver faithfulness and split replay through learning events.
// ---------------------------------------------------------------------------

#[test]
fn manual_driver_reproduces_the_verified_runner() {
    // The split tests resume this driver, so it must first be proven
    // bit-identical to run_episodic_lifetime on the same seeds.
    let cfg = winner_full_mask_config(60);
    let mut manual = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    manual.run_outcomes(60);
    assert!(manual.lifetime.is_complete());
    let reference = run_episodic_lifetime(&cfg, 1, "development", 2, 0, "B4").expect("B4 runs");
    let ref_actions: Vec<u8> = reference.choices.iter().map(|c| c.action).collect();
    let ref_rewards: Vec<f64> = reference.choices.iter().map(|c| c.reward).collect();
    let ref_correct: Vec<bool> = reference.choices.iter().map(|c| c.correct).collect();
    assert_eq!(manual.actions, ref_actions);
    assert_eq!(manual.rewards, ref_rewards);
    assert_eq!(manual.correct, ref_correct);
    assert_eq!(manual.resets, reference.resets);
    assert_eq!(manual.learner.plastic().p().to_vec(), reference.final_p);
    assert_eq!(manual.learner.plastic().e().to_vec(), reference.final_e);
    assert!((manual.learner.reward_baseline() - reference.final_baseline).abs() < 1e-15);
    assert_eq!(
        manual.learner.last_feedback(),
        reference.final_last_feedback
    );
}

#[test]
fn boundary_split_with_nonzero_offsets_resumes_exactly() {
    // Split at the rollout-20 boundary: 20 updates applied (P != 0,
    // baseline moved, dedup through event 19), traces freshly reset.
    let cfg = winner_full_mask_config(60);
    let path = tmp_path("boundary");
    let mut split = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    split.run_outcomes(20);
    let p_l1: f64 = l1(split.learner.plastic().p());
    assert!(p_l1 > 0.0, "split point must carry acquired offsets");
    assert_eq!(split.learner.last_feedback(), Some(19));
    split.capture(&path);
    let seeds = split.seeds();
    let mut resumed = ManualDriver::resume(&cfg, &path, seeds, split);
    resumed.run_outcomes(40);
    remove_quiet(&path);

    let mut reference = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    reference.run_outcomes(60);
    assert_eq!(resumed.actions, reference.actions);
    assert_eq!(resumed.rewards, reference.rewards);
    assert_eq!(resumed.correct, reference.correct);
    assert_eq!(resumed.resets, reference.resets);
    assert_eq!(resumed.finals(), reference.finals());
}

#[test]
fn pre_feedback_split_resumes_exactly() {
    // Split at the just-before-feedback state: the rollout's full
    // eligibility and 20 outcomes of offsets, aligned ticks, and no
    // pending delivery. The resumed run integrates the delivered reward
    // exactly as the uninterrupted run did.
    let cfg = winner_full_mask_config(60);
    let slot = tmp_path("justbefore");
    let mut split = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    split.run_outcomes(20);
    let out = split.run_to_just_before_feedback(&slot);
    assert!(
        matches!(out.phase, Phase::Feedback),
        "resumed tick must be the delivery tick"
    );
    // The slot state carries nonzero P and full-rollout E.
    let probe = LearningCheckpoint::load_from_path(&slot).expect("slot loads");
    let probe_learner = probe
        .restore_learner(probe.seeds())
        .expect("slot learner restores");
    assert!(l1(probe_learner.plastic().p()) > 0.0);
    assert!(l1(probe_learner.plastic().e()) > 0.0);
    assert_eq!(probe_learner.last_feedback(), Some(19));
    let seeds = split.seeds();
    // Resume re-delivers the same tick from its own restored state
    // (the held-over output is driver memory, not checkpoint state);
    // deterministic re-delivery must match bit-for-bit.
    let mut resumed = ManualDriver::resume(&cfg, &slot, seeds, split);
    let redelivered = resumed.tick();
    assert_eq!(
        redelivered.observation.feedback, out.observation.feedback,
        "resumed delivery must match the pre-split tick"
    );
    resumed.handle_feedback(&redelivered);
    resumed.handle_advance(&redelivered);
    resumed.handle_commit(&redelivered);
    resumed.handle_reset(&redelivered);
    resumed.run_outcomes(39);
    remove_quiet(&slot);

    let mut reference = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    reference.run_outcomes(60);
    assert_eq!(resumed.actions, reference.actions);
    assert_eq!(resumed.rewards, reference.rewards);
    assert_eq!(resumed.correct, reference.correct);
    assert_eq!(resumed.resets, reference.resets);
    assert_eq!(resumed.finals(), reference.finals());
}

#[test]
fn post_feedback_split_resumes_exactly() {
    // Split after outcome 20's update is applied and its feedback tick
    // fully advanced, but before the rollout reset: fresh P plus
    // post-outcome scores in E must round-trip, and the resumed reset
    // must not double-apply the reward.
    let cfg = winner_full_mask_config(60);
    let path = tmp_path("postfeedback");
    let mut split = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    split.run_outcomes(20);
    let out = split.run_to_next_feedback();
    split.handle_feedback(&out);
    split.handle_advance(&out);
    split.handle_commit(&out);
    let p_l1: f64 = l1(split.learner.plastic().p());
    let e_l1: f64 = l1(split.learner.plastic().e());
    assert!(p_l1 > 0.0 && e_l1 > 0.0, "fresh P and live E required");
    assert_eq!(split.learner.last_feedback(), Some(20));
    split.capture(&path);
    let seeds = split.seeds();
    let mut resumed = ManualDriver::resume(&cfg, &path, seeds, split);
    resumed.handle_reset(&out);
    resumed.run_outcomes(39);
    remove_quiet(&path);

    let mut reference = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    reference.run_outcomes(60);
    assert_eq!(resumed.actions, reference.actions);
    assert_eq!(resumed.rewards, reference.rewards);
    assert_eq!(resumed.correct, reference.correct);
    assert_eq!(resumed.resets, reference.resets);
    assert_eq!(resumed.finals(), reference.finals());
    // Exactly-once: the resumed run consumed each event once.
    assert_eq!(resumed.learner.last_feedback(), Some(59));
}

// ---------------------------------------------------------------------------
// Rejection paths.
// ---------------------------------------------------------------------------

#[test]
fn learning_checkpoint_rejections_are_explicit() {
    let cfg = winner_full_mask_config(12);
    let path = tmp_path("reject");
    let mut driver = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    driver.run_outcomes(12);
    driver.capture(&path);
    let seeds = driver.seeds();

    // Tampered bytes fail the checksum (flip one hex digit inside the
    // stored checksum so the JSON stays valid).
    let mut text = std::fs::read_to_string(&path).expect("checkpoint readable");
    let key = "\"checksum_sha256\": \"";
    let at = text.find(key).expect("checksum present") + key.len();
    let digit = text.as_bytes()[at] as char;
    text.replace_range(at..at + 1, if digit == 'a' { "b" } else { "a" });
    let tampered = tmp_path("tampered");
    std::fs::write(&tampered, text).expect("tampered write");
    assert!(matches!(
        LearningCheckpoint::load_from_path(&tampered),
        Err(CheckpointError::ChecksumMismatch)
    ));
    remove_quiet(&tampered);

    // Foreign seed identity is rejected, never silently continued.
    let foreign = SeedIdentity {
        lifetime_index: 7,
        ..seeds.clone()
    };
    let loaded = LearningCheckpoint::load_from_path(&path).expect("checkpoint loads");
    assert!(matches!(
        loaded.restore_env(&foreign),
        Err(CheckpointError::Incompatible(_))
    ));
    assert!(matches!(
        loaded.restore_learner(&foreign),
        Err(CheckpointError::Incompatible(_))
    ));

    // Garbage input fails to parse.
    let garbage = tmp_path("garbage");
    std::fs::write(&garbage, b"not a checkpoint").expect("garbage write");
    assert!(matches!(
        LearningCheckpoint::load_from_path(&garbage),
        Err(CheckpointError::Parse { .. })
    ));
    remove_quiet(&garbage);

    // Cross-schema loads are rejected: the M1 schema-2 loader must not
    // misread a plastic file as a nonplastic lifetime.
    assert!(
        Checkpoint::load_from_path(&path).is_err(),
        "M1 loader must reject schema-3 plastic files"
    );

    // Missing files are I/O errors, not successful empty runs.
    let missing = std::env::temp_dir().join(format!("cra-absent-{}.json", std::process::id()));
    assert!(matches!(
        LearningCheckpoint::load_from_path(&missing),
        Err(CheckpointError::Io { .. })
    ));
    remove_quiet(&path);
}

// ---------------------------------------------------------------------------
// Archive pins (M3-10 evidence bundle).
// ---------------------------------------------------------------------------

#[test]
fn working_config_archive_matches_the_winner() {
    // The archived working configuration is the frozen grid-index-11
    // winner with the full mask; this test pins it against drift.
    let text = std::fs::read_to_string("docs/evidence/m3-10/working_config.toml")
        .expect("working config archived");
    let cfg: Config = toml::from_str(&text).expect("working config parses");
    cra::config::validate(&cfg).expect("working config validates");
    cra::config::validate_episodic_execution(&cfg).expect("working config executes");
    let actor = cfg.actor.as_ref().expect("actor");
    let learning = cfg.learning.as_ref().expect("learning");
    assert_eq!(cfg.profile_name, "episodic_stationary");
    assert_eq!(learning.plastic_mask, "all_recurrent_edges");
    assert!((learning.eta - 0.001).abs() < 1e-15);
    assert!((actor.input_scale - 0.2).abs() < 1e-15);
    assert!((actor.recurrent_gain - 0.8).abs() < 1e-15);
    assert!((actor.noise_sigma - 0.05).abs() < 1e-15);
    assert_eq!(cfg.simulation.outcomes_per_lifetime, 2000);
    assert_eq!(cfg.simulation.reset_policy, "episodic_diagnostic");
    assert_eq!(learning.trace_policy, "no_decay_diagnostic");
}

#[test]
fn archived_final_checkpoint_reloads_with_offsets() {
    // The archived learned-offset checkpoint is a real capture from the
    // winner run (outer 2, 2,000 outcomes); it must reload on the
    // reference platform with acquired offsets intact.
    let path = PathBuf::from("docs/evidence/m3-10/checkpoints/final.json");
    let loaded = LearningCheckpoint::load_from_path(&path).expect("archived checkpoint loads");
    let seeds = SeedIdentity {
        root_seed: 1,
        namespace: "development".to_owned(),
        outer_seed: 2,
        lifetime_index: 0,
    };
    assert_eq!(loaded.seeds(), &seeds);
    assert_eq!(loaded.config().simulation.outcomes_per_lifetime, 2000);
    let learner = loaded
        .restore_learner(&seeds)
        .expect("archived learner resumes");
    let p_l1 = l1(learner.plastic().p());
    assert!(
        p_l1 > 0.0,
        "archived checkpoint must carry acquired offsets"
    );
    assert_eq!(learner.last_feedback(), Some(1999));
    let lifetime = loaded.restore_env(&seeds).expect("archived env resumes");
    assert!(lifetime.is_complete());
}

#[test]
fn failure_case_archive_is_the_documented_bound() {
    // The archived failure case is the winner config on the
    // birth-locked outer seed: locked behavior under every condition
    // with real offset movement (representation failure, M3-09 step 4).
    let text = std::fs::read_to_string("docs/evidence/m3-10/failure_case.json")
        .expect("failure case archived");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("failure case parses");
    let record = doc.get("record").expect("record present");
    assert_eq!(record.get("outer_seed").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        record.get("plastic_mask").and_then(|v| v.as_str()),
        Some("all_recurrent_edges")
    );
    for condition in ["b3", "b4", "shuffled"] {
        let late = record
            .get(condition)
            .and_then(|c| c.get("late_accuracy"))
            .and_then(|v| v.as_f64())
            .expect("late accuracy present");
        assert_eq!(late, 0.0, "{condition} behavior stays locked");
    }
    let p_l1 = record
        .get("b4_health")
        .and_then(|h| h.get("final_p_l1"))
        .and_then(|v| v.as_f64())
        .expect("B4 offset norm present");
    assert!(p_l1 > 0.0, "offsets move while behavior is locked");
}

#[test]
#[ignore = "bounded M3-10 learning-evidence capture (2000-outcome winner run with boundary/final checkpoints plus update digests); invoke explicitly in release per evidence"]
fn m3_learning_evidence_capture() {
    let out_dir = std::env::var_os("CRA_M3_10_DIR").map(PathBuf::from).expect(
        "set CRA_M3_10_DIR to a fresh directory path (parent must exist); existing directories are rejected",
    );
    std::fs::create_dir(&out_dir).expect("evidence directory must be fresh");
    let cfg = winner_full_mask_config(2000);
    let mut driver = ManualDriver::fresh(&cfg, 1, "development", 2, 0);
    driver.run_outcomes(1000);
    let boundary_p_l1 = l1(driver.learner.plastic().p());
    assert!(boundary_p_l1 > 0.0, "mid-run offsets must exist");
    let mut boundary_path = out_dir.clone();
    boundary_path.push("boundary_r1000.json");
    LearningCheckpoint::capture(&driver.lifetime, &driver.learner, &cfg, driver.seeds())
        .expect("boundary captures")
        .save_to_path(&boundary_path)
        .expect("boundary saves");
    driver.run_outcomes(1000);
    assert!(driver.lifetime.is_complete());
    assert_eq!(driver.actions.len(), 2000);
    assert!(driver.rewards.iter().all(|r| r.is_finite()));
    let mut final_path = out_dir.clone();
    final_path.push("final.json");
    LearningCheckpoint::capture(&driver.lifetime, &driver.learner, &cfg, driver.seeds())
        .expect("final captures")
        .save_to_path(&final_path)
        .expect("final saves");
    // Distilled update summaries: first and last learning events with
    // separated raw/limited/actual norms (the full matrices stay in the
    // run; the archive keeps the auditable digest).
    let digest_json = |d: &UpdateDigest| {
        serde_json::json!({
            "event_id": d.event_id,
            "reward": d.reward,
            "delta": d.delta,
            "baseline_old": d.baseline_old,
            "baseline_new": d.baseline_new,
            "raw_l1": d.raw_l1,
            "limited_l1": d.limited_l1,
            "actual_l1": d.actual_l1,
            "clipped": d.clipped,
            "p_l1_after": d.p_l1_after,
        })
    };
    let doc = serde_json::json!({
        "schema_version": 1,
        "task": "M3-10",
        "profile": "episodic_stationary",
        "plastic_mask": "all_recurrent_edges",
        "hyperparameters": {"eta": 0.001, "input_scale": 0.2, "recurrent_gain": 0.8, "noise_sigma": 0.05},
        "seeds": {"root_seed": 1, "namespace": "development", "outer_seed": 2, "lifetime_index": 0},
        "outcomes": 2000,
        "ticks": driver.lifetime.tick(),
        "final_accuracy": driver.correct.iter().filter(|&&c| c).count() as f64 / 2000.0,
        "final_baseline": driver.learner.reward_baseline(),
        "final_p_l1": l1(driver.learner.plastic().p()),
        "first_update": digest_json(&driver.digests[0]),
        "last_update": digest_json(&driver.digests[1999]),
        "clipped_outcomes": driver.digests.iter().filter(|d| d.clipped).count(),
    });
    let mut digest_path = out_dir.clone();
    digest_path.push("update_summary_example.json");
    std::fs::write(&digest_path, serde_json::to_string_pretty(&doc).unwrap())
        .expect("digest writes");
    let revision = command_output("git", &["rev-parse", "HEAD"]);
    let status = command_output("git", &["status", "--porcelain"]);
    let meta = serde_json::json!({
        "schema_version": 1,
        "task": "M3-10",
        "artifacts": ["boundary_r1000.json", "final.json", "update_summary_example.json", "meta.json"],
        "provenance": {
            "revision": revision, "dirty": !status.is_empty(), "git_status": status,
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            "profile": "release",
        },
    });
    let mut meta_path = out_dir.clone();
    meta_path.push("meta.json");
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).expect("meta writes");
    eprintln!("m3-10 capture: {meta}");
    // Execution integrity: the run is the showcase trajectory with real
    // offset movement and no failures.
    assert!(
        driver
            .learner
            .plastic()
            .p()
            .iter()
            .flatten()
            .all(|v| v.is_finite())
    );
}

fn command_output(cmd: &str, args: &[&str]) -> String {
    String::from_utf8_lossy(
        &std::process::Command::new(cmd)
            .args(args)
            .output()
            .expect("provenance command runs")
            .stdout,
    )
    .trim()
    .to_owned()
}
