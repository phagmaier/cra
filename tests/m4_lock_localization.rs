//! M4-08b lock-localization probes (spec 16/M4 "If it fails", 21.1-21.2).
//!
//! M4-08 isolated a continuity-induced behavioral lock: outer-2 persistent
//! legs answer action 0 on all 2,000 outcomes while the same-schedule
//! episodic leg explores and learns. This suite localizes the lock WITHOUT
//! changing the frozen family, seeds, rules, or tick order. No production
//! file is modified; every probe drives the public agent-only learner API
//! with ordinary synthetic features under a declared plan
//! (`manifests/m4_lock_localization.json`).
//!
//! Reduction order (smallest system first):
//!
//! ```text
//! lock-in validity (reproduce the M4-08 closed-loop lock at 60 outcomes)
//!   -> dose_response (fresh-state sensitivity to drive strength)
//!   -> flip (can ordinary drive overcome the locked state?)
//!   -> converge (recurrent washout vs attractor persistence)
//! ```
//!
//! Pairing: clones share locked history and future noise draws exactly, so
//! flip-leg differences are pure drive effects; locked/fresh converge pairs
//! share perturbation draws, so distances are pure history effects. Fast
//! tests pin the declaration, analyzer goldens, lock validity, determinism,
//! and structural probe properties. The ignored release test exports the
//! full matrix to a fresh directory for `docs/evidence/m4-08b/`; it asserts
//! execution integrity only and records magnitudes.

use cra::config::{Config, load_and_validate, validate_continuous_execution};
use cra::environment::Feedback;
use cra::experiments::continuous::{ContinuousLearner, sample_matched_inheritance};
use cra::rng::{SeedTuple, rng_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const PLAN_PATH: &str = "manifests/m4_lock_localization.json";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockPlan {
    schema_version: u32,
    task: String,
    purpose: String,
    frozen_source: FrozenSource,
    namespace: String,
    root_seed: u64,
    outer_seeds: Vec<u64>,
    base_profile: PathBuf,
    feature_layout: String,
    lock_in: LockIn,
    probes: Vec<ProbeDecl>,
    diagnostics: Vec<String>,
    integrity_rules: Vec<String>,
    prohibitions: Vec<String>,
    budget: Budget,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenSource {
    m4_08_manifest: String,
    m4_08_evidence: String,
    m4_08_finding: String,
    actor_family: ActorFamily,
    note: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorFamily {
    source: String,
    m3_grid_index: usize,
    eta: f64,
    input_scale: f64,
    recurrent_gain: f64,
    noise_sigma: f64,
    plastic_mask: String,
    tau_e: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockIn {
    protocol: String,
    preferred_action: u8,
    preferred_action_justification: String,
    outcomes: usize,
    ticks_per_outcome: usize,
    validity_gate: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeDecl {
    id: String,
    question: String,
    matrix: String,
    record: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    unit: String,
    lock_in_flip: u64,
    flip_legs: u64,
    lock_in_converge: u64,
    fresh_zero_pairing: u64,
    converge_paths: u64,
    dose_response: u64,
    lock_margins: u64,
    total_declared_transitions: u64,
    maximum_total_transitions: u64,
    execution: String,
}

fn load_plan() -> LockPlan {
    let bytes = std::fs::read(PLAN_PATH).expect("M4-08b lock plan exists");
    serde_json::from_slice(&bytes).expect("M4-08b lock plan parses")
}

/// Frozen M4-07/M4-08 family config rebuilt from the declaration.
fn family_config(plan: &LockPlan) -> Config {
    let mut cfg = load_and_validate(&plan.base_profile).expect("base profile validates");
    let actor = cfg.actor.as_mut().expect("base actor");
    actor.input_scale = plan.frozen_source.actor_family.input_scale;
    actor.recurrent_gain = plan.frozen_source.actor_family.recurrent_gain;
    actor.noise_sigma = plan.frozen_source.actor_family.noise_sigma;
    let learning = cfg.learning.as_mut().expect("base learning");
    learning.eta = plan.frozen_source.actor_family.eta;
    learning.plastic_mask = plan.frozen_source.actor_family.plastic_mask.clone();
    learning.tau_e = plan.frozen_source.actor_family.tau_e;
    cfg.profile_name = "m4_08b_lock_probe".to_owned();
    cfg.simulation.reset_policy = "birth_only".to_owned();
    cfg.learning.as_mut().expect("learning").trace_policy = "persistent".to_owned();
    validate_continuous_execution(&cfg).expect("probe config executes");
    cfg
}

/// Fresh outer-seed family learner on dedicated probe streams.
fn probe_learner(cfg: &Config, plan: &LockPlan, outer: u64, tag: &str) -> ContinuousLearner {
    let (inherited, _) = sample_matched_inheritance(cfg, plan.root_seed, &plan.namespace, outer)
        .expect("matched inheritance");
    let actor = cfg.actor.clone().expect("actor");
    let learning = cfg.learning.clone().expect("learning");
    let cue_count = cfg.environment.cue_count;
    let noise = rng_for(&SeedTuple::new(
        plan.root_seed,
        &plan.namespace,
        outer,
        0,
        &format!("m4_08b_{tag}_noise"),
    ))
    .expect("probe noise rng");
    let tie = rng_for(&SeedTuple::new(
        plan.root_seed,
        &plan.namespace,
        outer,
        0,
        &format!("m4_08b_{tag}_tie"),
    ))
    .expect("probe tie rng");
    ContinuousLearner::from_agent_parts(actor, learning, inherited, cue_count, noise, tie)
        .expect("probe learner builds")
}

/// Ordinary cue features: one-hot cue plus cue-present; `latch_a0` selects
/// the locked previous-action latch versus the fresh zero latch.
fn cue_features(cue: usize, cue_count: usize, latch_a0: bool) -> Vec<f64> {
    let mut features = vec![0.0; cue_count + 6];
    features[cue] = 1.0;
    features[cue_count] = 1.0;
    if latch_a0 {
        features[cue_count + 4] = 1.0;
    }
    features
}

fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// Readout margin: positive favors action 0, negative favors action 1.
fn margin(q: [f64; 2]) -> f64 {
    q[0] - q[1]
}

fn close(got: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (got - expected).abs() <= tol,
        "{what}: got {got}, expected {expected}"
    );
}

/// Lock-in: the M4-08 closed-loop protocol at small scale. Constant cue-0
/// features with the locked latch, 17 ticks per outcome, synthetic reward
/// for the preferred action. Returns the locked learner, its per-outcome
/// actions, and the transition count.
fn lock_in(
    cfg: &Config,
    plan: &LockPlan,
    outer: u64,
    tag: &str,
) -> (ContinuousLearner, Vec<u8>, u64) {
    let mut learner = probe_learner(cfg, plan, outer, tag);
    let features = cue_features(0, cfg.environment.cue_count, true);
    let lock = &plan.lock_in;
    let mut actions = Vec::with_capacity(lock.outcomes);
    let mut transitions = 0u64;
    for k in 0..lock.outcomes {
        for _ in 0..lock.ticks_per_outcome {
            learner.advance(&features).expect("lock advance");
            transitions += 1;
        }
        let action = learner.select_action();
        let reward = if action == lock.preferred_action {
            1.0
        } else {
            0.0
        };
        learner
            .apply_feedback(Feedback {
                event_id: k as u64 + 1,
                reward,
            })
            .expect("lock feedback applies");
        actions.push(action);
    }
    assert!(
        actions.iter().all(|a| *a <= 1),
        "lock actions are valid actions"
    );
    (learner, actions, transitions)
}

#[derive(Clone, Debug, Serialize)]
struct DosePoint {
    cue: usize,
    scale: f64,
    displacement: f64,
    margin: f64,
}

/// Fresh-state dose response: scaled cue drive for 8 ticks from a fresh
/// learner; displacement against the zero-drive endpoint plus readout
/// margin. Same noise tag per (outer) run, so scale differences are pure
/// drive effects.
fn dose_response(
    cfg: &Config,
    plan: &LockPlan,
    outer: u64,
    scales: &[f64],
) -> (Vec<DosePoint>, u64) {
    let cue_count = cfg.environment.cue_count;
    let mut points = Vec::new();
    let mut transitions = 0u64;
    for cue in 0..cue_count {
        let mut endpoints = Vec::new();
        for &scale in scales {
            let mut learner = probe_learner(cfg, plan, outer, "dose");
            let mut features = cue_features(cue, cue_count, false);
            for v in features.iter_mut() {
                *v *= scale;
            }
            for _ in 0..8 {
                learner.advance(&features).expect("dose advance");
                transitions += 1;
            }
            let r = learner.actor_state().r().to_vec();
            assert!(r.iter().all(|x| x.is_finite()));
            endpoints.push((scale, r, margin(learner.motor_state().q())));
        }
        let zero = endpoints
            .iter()
            .find(|(s, _, _)| *s == 0.0)
            .expect("zero-drive reference present");
        for (scale, r, m) in &endpoints {
            points.push(DosePoint {
                cue,
                scale: *scale,
                displacement: euclidean(r, &zero.1),
                margin: *m,
            });
        }
    }
    (points, transitions)
}

#[derive(Clone, Debug, Serialize)]
struct FlipLeg {
    cue: usize,
    scale: f64,
    seed: usize,
    ticks_to_first_one: Option<usize>,
    margin_at_40: f64,
    lock_margin: f64,
}

/// Flip probe: from a locked-state clone, switch to scaled cue drive and
/// poll the readout every tick up to `cap`. Clones share locked history and
/// future noise draws exactly, so leg differences are pure drive effects.
fn flip_matrix(
    cfg: &Config,
    plan: &LockPlan,
    outer: u64,
    scales: &[f64],
    seeds: usize,
    cap: usize,
) -> (Vec<FlipLeg>, u64) {
    let cue_count = cfg.environment.cue_count;
    let mut legs = Vec::new();
    let mut transitions = 0u64;
    for seed in 0..seeds {
        let tag = format!("flip{seed}");
        let (locked, actions, lock_transitions) = lock_in(cfg, plan, outer, &tag);
        transitions += lock_transitions;
        assert!(
            actions[actions.len() - 10..].iter().all(|&a| a == 0),
            "lock-in validity gate: last 10 actions are 0"
        );
        let lock_margin = margin(locked.motor_state().q());
        for cue in 0..cue_count {
            for &scale in scales {
                let mut learner = locked.clone();
                let mut features = if scale == 0.0 {
                    vec![0.0; cue_count + 6]
                } else {
                    cue_features(cue, cue_count, true)
                };
                for v in features.iter_mut() {
                    *v *= scale;
                }
                let mut first_one = None;
                let mut margin_40 = f64::NAN;
                for t in 0..cap {
                    learner.advance(&features).expect("flip advance");
                    transitions += 1;
                    let q = learner.motor_state().q();
                    assert!(q.iter().all(|x| x.is_finite()));
                    if t == 39 {
                        margin_40 = margin(q);
                    }
                    if first_one.is_none() && learner.select_action() == 1 {
                        first_one = Some(t + 1);
                    }
                }
                if cap < 40 {
                    margin_40 = margin(learner.motor_state().q());
                }
                legs.push(FlipLeg {
                    cue,
                    scale,
                    seed,
                    ticks_to_first_one: first_one,
                    margin_at_40: margin_40,
                    lock_margin,
                });
            }
        }
    }
    (legs, transitions)
}

#[derive(Clone, Debug, Serialize)]
struct ConvergePoint {
    cue: usize,
    seed: usize,
    distance_8: f64,
    distance_16: f64,
    distance_32: f64,
    distance_64: f64,
}

/// Converge probe: locked-then-cue-A versus fresh-then-cue-A with paired
/// noise streams (locked lock-in advances and fresh zero-drive advances
/// consume identical perturbation draws). Distances are pure history
/// effects: decay toward zero means drive washes history out; a sustained
/// offset means a recurrent attractor holds it.
fn converge_matrix(
    cfg: &Config,
    plan: &LockPlan,
    outer: u64,
    seeds: usize,
) -> (Vec<ConvergePoint>, u64) {
    let cue_count = cfg.environment.cue_count;
    let mut points = Vec::new();
    let mut transitions = 0u64;
    for seed in 0..seeds {
        let tag = format!("conv{seed}");
        let (locked, actions, lock_transitions) = lock_in(cfg, plan, outer, &tag);
        transitions += lock_transitions;
        assert!(
            actions[actions.len() - 10..].iter().all(|&a| a == 0),
            "lock-in validity gate: last 10 actions are 0"
        );
        // Fresh path: same advance count on zero drive so perturbation
        // draws stay paired with the lock-in path.
        let mut fresh = probe_learner(cfg, plan, outer, &tag);
        let zero = vec![0.0; cue_count + 6];
        let lock_advances = plan.lock_in.outcomes * plan.lock_in.ticks_per_outcome;
        for _ in 0..lock_advances {
            fresh.advance(&zero).expect("fresh zero advance");
            transitions += 1;
        }
        for cue in 0..cue_count {
            let mut path_locked = locked.clone();
            let mut path_fresh = fresh.clone();
            let features = cue_features(cue, cue_count, true);
            let mut distances = Vec::new();
            for t in 0..64 {
                path_locked.advance(&features).expect("locked path");
                path_fresh.advance(&features).expect("fresh path");
                transitions += 2;
                if [8, 16, 32, 64].contains(&(t + 1)) {
                    let d = euclidean(path_locked.actor_state().r(), path_fresh.actor_state().r());
                    assert!(d.is_finite());
                    distances.push(d);
                }
            }
            points.push(ConvergePoint {
                cue,
                seed,
                distance_8: distances[0],
                distance_16: distances[1],
                distance_32: distances[2],
                distance_64: distances[3],
            });
        }
    }
    (points, transitions)
}

const DOSE_SCALES: [f64; 5] = [0.0, 0.5, 1.0, 2.0, 4.0];
const FLIP_SCALES: [f64; 4] = [0.0, 1.0, 2.0, 4.0];
const FLIP_SEEDS: usize = 8;
const FLIP_CAP: usize = 200;
const CONVERGE_SEEDS: usize = 4;

#[derive(Clone, Debug, Serialize)]
struct OuterProbes {
    outer_seed: u64,
    dose: Vec<DosePoint>,
    flip: Vec<FlipLeg>,
    converge: Vec<ConvergePoint>,
    lock_margins: Vec<f64>,
    transitions: u64,
}

fn run_outer_probes(cfg: &Config, plan: &LockPlan, outer: u64) -> OuterProbes {
    let (dose, dose_t) = dose_response(cfg, plan, outer, &DOSE_SCALES);
    let (flip, flip_t) = flip_matrix(cfg, plan, outer, &FLIP_SCALES, FLIP_SEEDS, FLIP_CAP);
    let (converge, conv_t) = converge_matrix(cfg, plan, outer, CONVERGE_SEEDS);
    let mut margin_t = 0u64;
    let lock_margins: Vec<f64> = (0..FLIP_SEEDS)
        .map(|seed| {
            let (locked, _, t) = lock_in(cfg, plan, outer, &format!("flip{seed}"));
            margin_t += t;
            margin(locked.motor_state().q())
        })
        .collect();
    OuterProbes {
        outer_seed: outer,
        dose,
        flip,
        converge,
        lock_margins,
        transitions: dose_t + flip_t + conv_t + margin_t,
    }
}

#[test]
fn lock_plan_declares_frozen_family_and_probe_matrix() {
    let plan = load_plan();
    assert_eq!(plan.schema_version, 1);
    assert_eq!(plan.task, "M4-08b");
    assert!(!plan.purpose.trim().is_empty());
    assert_eq!(plan.namespace, "development");
    assert_eq!(plan.root_seed, 1);
    assert_eq!(plan.outer_seeds, vec![2, 3]);
    assert!(
        plan.base_profile
            .to_str()
            .expect("path")
            .ends_with("continuous_stationary.toml")
    );
    assert!(!plan.feature_layout.trim().is_empty());
    let family = &plan.frozen_source.actor_family;
    assert!(family.source.contains("M3-08"));
    assert_eq!(family.m3_grid_index, 11);
    assert_eq!(family.eta, 0.001);
    assert_eq!(family.input_scale, 0.2);
    assert_eq!(family.recurrent_gain, 0.8);
    assert_eq!(family.noise_sigma, 0.05);
    assert_eq!(family.plastic_mask, "all_recurrent_edges");
    assert_eq!(family.tau_e, 32.0);
    assert_eq!(
        plan.frozen_source.m4_08_manifest,
        "manifests/m4_continuity_audit.json"
    );
    assert_eq!(
        plan.frozen_source.m4_08_evidence,
        "docs/evidence/m4-08/summary.md"
    );
    assert!(plan.frozen_source.m4_08_finding.contains("600/600"));
    assert!(!plan.frozen_source.note.trim().is_empty());
    assert!(plan.lock_in.protocol.contains("closed-loop"));
    assert_eq!(plan.lock_in.preferred_action, 1);
    assert_eq!(plan.lock_in.outcomes, 60);
    assert_eq!(plan.lock_in.ticks_per_outcome, 17);
    assert!(
        !plan
            .lock_in
            .preferred_action_justification
            .trim()
            .is_empty()
    );
    assert!(!plan.lock_in.validity_gate.trim().is_empty());
    let ids: Vec<_> = plan.probes.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["dose_response", "flip", "converge"]);
    for probe in &plan.probes {
        assert!(!probe.question.trim().is_empty());
        assert!(!probe.matrix.trim().is_empty());
        assert!(!probe.record.trim().is_empty());
    }
    // Declared transition budget matches the matrix arithmetic exactly.
    let outers = plan.outer_seeds.len() as u64;
    let lock_advances = (plan.lock_in.outcomes * plan.lock_in.ticks_per_outcome) as u64;
    assert_eq!(
        plan.budget.lock_in_flip,
        outers * FLIP_SEEDS as u64 * lock_advances
    );
    assert_eq!(
        plan.budget.lock_margins,
        outers * FLIP_SEEDS as u64 * lock_advances
    );
    assert_eq!(
        plan.budget.flip_legs,
        outers * FLIP_SEEDS as u64 * 2 * FLIP_SCALES.len() as u64 * FLIP_CAP as u64
    );
    assert_eq!(
        plan.budget.lock_in_converge,
        outers * CONVERGE_SEEDS as u64 * lock_advances
    );
    assert_eq!(
        plan.budget.fresh_zero_pairing,
        outers * CONVERGE_SEEDS as u64 * lock_advances
    );
    assert_eq!(
        plan.budget.converge_paths,
        outers * 2 * CONVERGE_SEEDS as u64 * 2 * 64
    );
    assert_eq!(
        plan.budget.dose_response,
        outers * 2 * DOSE_SCALES.len() as u64 * 8
    );
    assert_eq!(
        plan.budget.lock_in_flip
            + plan.budget.flip_legs
            + plan.budget.lock_in_converge
            + plan.budget.fresh_zero_pairing
            + plan.budget.converge_paths
            + plan.budget.dose_response
            + plan.budget.lock_margins,
        plan.budget.total_declared_transitions
    );
    assert_eq!(plan.budget.total_declared_transitions, 76768);
    assert!(plan.budget.maximum_total_transitions >= 76768);
    assert!(plan.budget.unit.contains("transition"));
    assert!(plan.budget.execution.contains("serial"));
    assert!(!plan.diagnostics.is_empty());
    assert!(!plan.integrity_rules.is_empty());
    assert!(!plan.prohibitions.is_empty());
    let cfg = family_config(&plan);
    assert_eq!(cfg.environment.cue_count, 2);
}

#[test]
fn lock_analyzers_match_hand_computed_values() {
    close(
        euclidean(&[0.0, 0.0], &[3.0, 4.0]),
        5.0,
        1e-15,
        "3-4-5 triangle",
    );
    close(euclidean(&[1.0, 2.0], &[1.0, 2.0]), 0.0, 0.0, "identical");
    close(
        margin([0.3, 0.1]),
        0.2,
        1e-15,
        "positive margin favors action 0",
    );
    close(
        margin([0.1, 0.4]),
        -0.3,
        1e-15,
        "negative margin favors action 1",
    );
    // Feature layout: [cue0, cue1, cue_present, go, outcome, value, prev0, prev1].
    let fresh = cue_features(1, 2, false);
    assert_eq!(fresh, vec![0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let locked = cue_features(0, 2, true);
    assert_eq!(locked, vec![1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    assert_eq!(vec![0.0; 8], vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn lock_in_reproduces_the_closed_loop_lock_deterministically() {
    let plan = load_plan();
    let cfg = family_config(&plan);
    let (locked, actions, transitions) = lock_in(&cfg, &plan, 2, "fastlock");
    assert_eq!(transitions, 60 * 17);
    assert_eq!(actions.len(), 60);
    // Validity gate at small scale: the M4-08 600/600 lock reproduces.
    assert!(
        actions[50..].iter().all(|&a| a == 0),
        "lock-in ends fully locked on action 0"
    );
    assert!(margin(locked.motor_state().q()).is_finite());
    // Determinism: an identical rebuild reproduces actions and state bitwise.
    let (locked2, actions2, _) = lock_in(&cfg, &plan, 2, "fastlock");
    assert_eq!(actions, actions2);
    assert_eq!(
        locked.actor_state().r().to_vec(),
        locked2.actor_state().r().to_vec()
    );
    assert_eq!(locked.motor_state().q(), locked2.motor_state().q());
}

#[test]
fn probe_matrix_runs_structurally_and_reproduces_exactly() {
    let plan = load_plan();
    let cfg = family_config(&plan);
    let first = run_outer_probes(&cfg, &plan, 2);
    assert_eq!(first.dose.len(), 2 * DOSE_SCALES.len());
    assert_eq!(first.flip.len(), FLIP_SEEDS * 2 * FLIP_SCALES.len());
    assert_eq!(first.converge.len(), 2 * CONVERGE_SEEDS);
    assert_eq!(first.lock_margins.len(), FLIP_SEEDS);
    for point in &first.dose {
        assert!(point.displacement.is_finite() && point.margin.is_finite());
    }
    // Zero drive is the displacement reference by construction.
    for point in first.dose.iter().filter(|p| p.scale == 0.0) {
        assert_eq!(point.displacement, 0.0);
    }
    for leg in &first.flip {
        assert!(leg.margin_at_40.is_finite() && leg.lock_margin.is_finite());
        if let Some(t) = leg.ticks_to_first_one {
            assert!((1..=FLIP_CAP).contains(&t));
        }
    }
    for point in &first.converge {
        for d in [
            point.distance_8,
            point.distance_16,
            point.distance_32,
            point.distance_64,
        ] {
            assert!(d.is_finite() && d >= 0.0);
        }
    }
    assert!(first.transitions > 0);
    assert!(first.transitions <= plan.budget.maximum_total_transitions);
    // Exact reproduction of the whole matrix.
    let second = run_outer_probes(&cfg, &plan, 2);
    assert_eq!(
        serde_json::to_string(&first).expect("serializes"),
        serde_json::to_string(&second).expect("serializes")
    );
}

fn command_output(command: &str, args: &[&str]) -> String {
    String::from_utf8_lossy(
        &std::process::Command::new(command)
            .args(args)
            .output()
            .expect("provenance command runs")
            .stdout,
    )
    .trim()
    .to_owned()
}

fn config_sha256(cfg: &Config) -> String {
    let resolved = cra::config::resolved_toml(cfg).expect("resolved config serializes");
    format!("{:x}", Sha256::digest(resolved.as_bytes()))
}

#[test]
#[ignore = "bounded M4-08b lock-localization export (2 outers, dose/flip/converge matrix); invoke explicitly in release with CRA_M4_LOCK_DIR"]
fn m4_lock_localization_export() {
    let out_dir = std::env::var_os("CRA_M4_LOCK_DIR")
        .map(PathBuf::from)
        .expect("set CRA_M4_LOCK_DIR to a fresh path");
    std::fs::create_dir(&out_dir).expect("evidence directory must be fresh");
    let plan = load_plan();
    let plan_bytes = std::fs::read(PLAN_PATH).expect("plan readable");
    let plan_sha256 = format!("{:x}", Sha256::digest(&plan_bytes));
    let cfg = family_config(&plan);
    let mut outers = Vec::new();
    let mut total = 0u64;
    for &outer in &plan.outer_seeds {
        let result = run_outer_probes(&cfg, &plan, outer);
        // Same-tag lock-ins rebuild bitwise-identical locked states: the
        // margins re-measurement must equal every flip leg's lock margin.
        for seed in 0..FLIP_SEEDS {
            let leg_margin = result
                .flip
                .iter()
                .find(|leg| leg.seed == seed)
                .map(|leg| leg.lock_margin)
                .expect("flip legs cover every seed");
            assert_eq!(leg_margin, result.lock_margins[seed]);
        }
        total += result.transitions;
        outers.push(result);
    }
    assert_eq!(
        total, plan.budget.total_declared_transitions,
        "transition counters account for every probe advance"
    );
    assert!(
        total <= plan.budget.maximum_total_transitions,
        "transition budget holds: {total}"
    );
    let export = serde_json::json!({
        "schema_version": 1,
        "task": "M4-08b",
        "plan": PLAN_PATH,
        "plan_sha256": plan_sha256,
        "config_sha256": config_sha256(&cfg),
        "dose_scales": DOSE_SCALES,
        "flip_scales": FLIP_SCALES,
        "flip_seeds": FLIP_SEEDS,
        "flip_cap_ticks": FLIP_CAP,
        "converge_seeds": CONVERGE_SEEDS,
        "outers": outers,
        "measured_budget": {
            "transitions": total,
            "maximum_declared_transitions": plan.budget.maximum_total_transitions,
        },
        "pairing": "flip clones share locked history and future noise draws; converge pairs share perturbation draws; dose legs share fresh noise draws",
        "passes": null,
        "provenance": {
            "revision": command_output("git", &["rev-parse", "HEAD"]),
            "dirty": !command_output("git", &["status", "--porcelain"]).is_empty(),
            "git_status": command_output("git", &["status", "--porcelain"]),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "profile": "release",
        },
    });
    std::fs::write(
        out_dir.join("probes.json"),
        serde_json::to_string_pretty(&export).expect("export serializes"),
    )
    .expect("probes write");
    eprintln!("M4-08b transitions: {total}");
}
