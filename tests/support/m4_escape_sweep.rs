//! M4-08c: bounded escape sweep, sharing the frozen M4-07 analyzers.
use super::*;
use serde_json::{Value, json};
use std::path::Path;

const ESCAPE_PLAN: &str = "manifests/m4_escape_sweep.json";
const ANCHOR: usize = 7;

fn sha(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

fn declaration() -> Value {
    let p: Value = serde_json::from_slice(&std::fs::read(ESCAPE_PLAN).unwrap()).unwrap();
    assert_eq!(p["task"], "M4-08c");
    assert_eq!(p["schema_version"], 1);
    assert_eq!(p["reference_plan"], PLAN_PATH);
    assert_eq!(p["reference_plan_sha256"], sha(PLAN_PATH));
    assert_eq!(
        p["reference_records_sha256"],
        sha(p["reference_records"].as_str().unwrap())
    );
    assert_eq!(p["eta"], json!([0.0001, 0.0003, 0.001, 0.003]));
    assert_eq!(p["tau_e"], json!([16.0, 32.0, 64.0]));
    assert_eq!(p["mechanism"]["late_delta_floor"], 1e-12);
    p
}

fn grid(p: &Value) -> Vec<(f64, f64)> {
    p["eta"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|eta| {
            p["tau_e"]
                .as_array()
                .unwrap()
                .iter()
                .map(move |tau| (eta.as_f64().unwrap(), tau.as_f64().unwrap()))
        })
        .collect()
}

fn point_config(reference: &AcquisitionPlan, point: (f64, f64)) -> Config {
    let mut cfg = persistent_config(reference);
    let learning = cfg.learning.as_mut().unwrap();
    learning.eta = point.0;
    learning.tau_e = point.1;
    validate_continuous_execution(&cfg).unwrap();
    cfg
}

fn seed_passes(
    behavior: &BehaviorStats,
    control: &BehaviorStats,
    updates: &UpdateStats,
    c: &Criterion,
) -> bool {
    behavior.late_macro_accuracy >= c.min_continuous_late_accuracy
        && behavior.late_macro_accuracy - control.late_macro_accuracy
            >= c.min_continuous_minus_matched_b3
        && updates.clipped_update_fraction <= c.max_clipped_update_fraction
        && updates.plastic_bound_occupancy <= c.max_plastic_bound_occupancy
        && (!c.require_continuous_p_movement || updates.final_p_l1 > 0.0)
}

fn late_indices(summary: &ContinuousSummary, cue_count: usize, count: usize) -> Vec<usize> {
    let mut remaining = vec![count; cue_count];
    let mut indices = Vec::new();
    for (i, choice) in summary.choices.iter().enumerate().rev() {
        if remaining[choice.cue] > 0 {
            remaining[choice.cue] -= 1;
            indices.push(i);
        }
    }
    assert!(remaining.iter().all(|v| *v == 0));
    indices.sort_unstable();
    indices
}

fn assert_paired(summary: &ContinuousSummary, control: &BaselineSummary) {
    assert_eq!(
        schedule_from_continuous(summary),
        schedule_from_baseline(control)
    );
    for (plastic, fixed) in summary.choices.iter().zip(&control.choices) {
        // Recover the evaluator-only target at commitment, never feed it to the actor.
        assert_eq!(
            plastic.action ^ u8::from(!plastic.correct),
            fixed.action ^ u8::from(!fixed.correct)
        );
    }
}

fn mechanism(summary: &ContinuousSummary, cfg: &Config, count: usize, floor: f64) -> Value {
    let indices = late_indices(summary, cfg.environment.cue_count, count);
    let mut actions = [0usize; 2];
    let mut late_actions = [0usize; 2];
    let mut delta = 0.0;
    let mut eligibility = 0.0;
    let mut actual = 0.0;
    let mut error: f64 = 0.0;
    for choice in &summary.choices {
        actions[choice.action as usize] += 1;
        let expected = cfg.learning.as_ref().unwrap().eta
            * choice.update.delta.abs()
            * choice.eligibility_l1_before_update;
        error = error.max((l1(&choice.update.raw_updates) - expected).abs());
    }
    assert!(error <= 1e-9, "raw=eta*abs(delta)*E_L1 identity");
    for &i in &indices {
        let choice = &summary.choices[i];
        late_actions[choice.action as usize] += 1;
        delta += choice.update.delta.abs();
        eligibility += choice.eligibility_l1_before_update;
        actual += l1(&choice.update.actual_updates);
    }
    let n = indices.len() as f64;
    json!({"action_counts": actions, "late_action_counts": late_actions,
        "late_rows": indices.len(), "late_mean_abs_delta": delta / n,
        "late_mean_eligibility_l1": eligibility / n, "late_mean_actual_l1": actual / n,
        "raw_identity_max_error": error,
        "screen_passes": actions.iter().all(|v| *v > 0) && delta / n > floor})
}

fn plastic_record(
    plan: &AcquisitionPlan,
    cfg: &Config,
    outer: u64,
    s: &ContinuousSummary,
) -> Value {
    json!({"task": "M4-07", "status": "complete", "root_seed": plan.root_seed,
        "namespace": plan.namespace, "outer_seed": outer, "lifetime_index": plan.lifetime_indices[0],
        "condition": "continuous_b4", "matched_control": "continuous_b3",
        "outcomes": s.outcomes, "ticks": s.ticks, "resets": s.resets,
        "behavior": behavior_stats(cfg.environment.cue_count, &continuous_rows(s), &plan.acquisition_windows),
        "updates": update_stats(s.choices.iter().map(|c| &c.update), &s.final_p, &s.final_e,
            s.final_baseline, cfg.learning.as_ref().unwrap().plastic_bound),
        "health": health_json(&s.health)})
}

fn control_record(plan: &AcquisitionPlan, outer: u64, s: &BaselineSummary) -> Value {
    let cfg = nonplastic_config(plan);
    json!({"task": "M4-07", "status": "complete", "root_seed": plan.root_seed,
        "namespace": plan.namespace, "outer_seed": outer, "lifetime_index": plan.lifetime_indices[0],
        "condition": "continuous_b3", "matched_control": null,
        "outcomes": s.choices.len(), "ticks": s.health.as_ref().unwrap().ticks_observed,
        "resets": [0], "updates": null,
        "behavior": behavior_stats(cfg.environment.cue_count, &baseline_rows(s), &plan.acquisition_windows),
        "health": health_json(s.health.as_ref().unwrap())})
}

fn archived_record(p: &Value, outer: u64, condition: &str) -> Value {
    std::fs::read_to_string(p["reference_records"].as_str().unwrap())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|v| v["outer_seed"] == outer && v["condition"] == condition)
        .unwrap()
}

fn append(file: &mut std::io::BufWriter<std::fs::File>, value: &Value) {
    writeln!(file, "{value}").unwrap();
}

fn write_json(path: impl AsRef<Path>, value: &Value) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(serde_json::to_string_pretty(value).unwrap().as_bytes())
        .unwrap();
}

fn point_verdict(index: usize, point: (f64, f64), seeds: &[Value], integrity: bool) -> Value {
    let passes = seeds.iter().filter(|v| v["passes"] == true).count();
    let screened = seeds
        .iter()
        .filter(|v| v["passes"] == true)
        .all(|v| v["mechanism"]["screen_passes"] == true);
    let minimum_margin = seeds
        .iter()
        .map(|v| v["margin"].as_f64().unwrap())
        .min_by(f64::total_cmp)
        .unwrap();
    json!({"grid_index": index, "eta": point.0, "tau_e": point.1, "seeds": seeds,
        "seeds_passing": passes, "criterion_passes": integrity && seeds.len() == 3 && passes >= 2,
        "eligible": integrity && seeds.len() == 3 && passes >= 2 && screened,
        "minimum_margin": minimum_margin,
        "departure_distance": (point.0 / 0.001).log2().abs() + (point.1 / 32.0).log2().abs()})
}

fn select(points: &[Value]) -> Option<usize> {
    let mut eligible: Vec<_> = points.iter().filter(|p| p["eligible"] == true).collect();
    eligible.sort_by(|a, b| {
        b["seeds_passing"]
            .as_u64()
            .cmp(&a["seeds_passing"].as_u64())
            .then_with(|| {
                b["minimum_margin"]
                    .as_f64()
                    .unwrap()
                    .total_cmp(&a["minimum_margin"].as_f64().unwrap())
            })
            .then_with(|| {
                a["departure_distance"]
                    .as_f64()
                    .unwrap()
                    .total_cmp(&b["departure_distance"].as_f64().unwrap())
            })
            .then_with(|| a["grid_index"].as_u64().cmp(&b["grid_index"].as_u64()))
    });
    eligible
        .first()
        .map(|p| p["grid_index"].as_u64().unwrap() as usize)
}

#[test]
fn escape_declaration_budget_and_grid_are_frozen() {
    let p = declaration();
    let reference = load_plan();
    let points = grid(&p);
    assert_eq!(points.len(), 12);
    assert_eq!(points[ANCHOR], (0.001, 32.0));
    assert_eq!(p["budget"]["grid_points"], points.len());
    assert_eq!(
        p["budget"]["b4_lifetimes"],
        points.len() * reference.outer_seeds.len()
    );
    assert_eq!(p["budget"]["b3_lifetimes"], reference.outer_seeds.len());
    let lifetimes = (points.len() + 1) * reference.outer_seeds.len();
    assert_eq!(p["budget"]["total_lifetimes"], lifetimes);
    assert_eq!(
        p["budget"]["total_outcomes"],
        lifetimes as u64 * reference.outcomes_per_lifetime
    );
    let base = persistent_config(&reference);
    let e = &base.environment;
    let cycle = e.quiet_ticks[1]
        + e.cue_ticks
        + e.memory_gap_ticks[1]
        + e.response_ticks
        + e.reward_delay_ticks[1];
    let ticks =
        cycle * reference.outcomes_per_lifetime + base.simulation.warmup_ticks - e.quiet_ticks[1];
    assert_eq!(p["budget"]["maximum_ticks_per_lifetime"], ticks);
    assert_eq!(p["budget"]["maximum_total_ticks"], ticks * lifetimes as u64);
    for point in points {
        let mut cfg = point_config(&reference, point);
        cfg.learning = base.learning.clone();
        assert_eq!(resolved_toml(&cfg).unwrap(), resolved_toml(&base).unwrap());
    }
    let b3 = archived_record(&p, 3, "continuous_b3");
    assert!(
        1.0 - b3["behavior"]["late_macro_accuracy"].as_f64().unwrap()
            < reference.criterion.min_continuous_minus_matched_b3
    );
}

#[test]
fn escape_pairing_and_control_reuse_across_every_point() {
    let p = declaration();
    let mut plan = load_plan();
    plan.outcomes_per_lifetime = 24;
    for outer in [1, 2, 3] {
        let b3_cfg = nonplastic_config(&plan);
        let mut actor = NoLearningActor::new(&b3_cfg, 1, "development", outer, 0).unwrap();
        let expected_w0 = actor.inherited().weights.w0.clone();
        let control = run_actor_ordinary(
            &b3_cfg,
            1,
            "development",
            outer,
            0,
            "continuous_b3",
            &mut actor,
        )
        .unwrap();
        for point in grid(&p) {
            let cfg = point_config(&plan, point);
            let s =
                run_continuous_lifetime(&cfg, 1, "development", outer, 0, "continuous_b4").unwrap();
            assert_eq!(s.w0, expected_w0);
            assert_eq!(s.resets, vec![0]);
            assert_paired(&s, &control);
            assert_eq!(s.choices[0].action, control.choices[0].action);
            mechanism(&s, &cfg, 2, 1e-12);
            let mut disabled = cfg.clone();
            disabled.learning.as_mut().unwrap().enabled = false;
            let mut actor = NoLearningActor::new(&disabled, 1, "development", outer, 0).unwrap();
            let other = run_actor_ordinary(
                &disabled,
                1,
                "development",
                outer,
                0,
                "continuous_b3",
                &mut actor,
            )
            .unwrap();
            assert_eq!(other, control);
        }
    }
}

#[test]
fn escape_analyzer_preserves_absolute_bar_and_rejects_bad_health() {
    let p = load_plan();
    let b = behavior_stats(1, &[(0, true, 1.0); 200], &p.acquisition_windows);
    let control = behavior_stats(1, &[(0, false, 0.0); 200], &p.acquisition_windows);
    let update = FeedbackOutcome {
        event_id: 1,
        reward: 1.0,
        delta: 0.5,
        baseline_old: 0.5,
        baseline_new: 0.51,
        raw_updates: vec![vec![0.1]],
        limited_updates: vec![vec![0.1]],
        actual_updates: vec![vec![0.1]],
    };
    let mut stats = update_stats(
        std::iter::once(&update),
        &[vec![0.1]],
        &[vec![1.0]],
        0.51,
        0.5,
    );
    assert!(seed_passes(&b, &control, &stats, &p.criterion));
    assert!(!seed_passes(&control, &control, &stats, &p.criterion));
    assert!(!seed_passes(&b, &b, &stats, &p.criterion));
    stats.clipped_update_fraction = 0.11;
    assert!(!seed_passes(&b, &control, &stats, &p.criterion));
    stats.clipped_update_fraction = 0.0;
    stats.plastic_bound_occupancy = 0.06;
    assert!(!seed_passes(&b, &control, &stats, &p.criterion));
    stats.plastic_bound_occupancy = 0.0;
    stats.final_p_l1 = 0.0;
    assert!(!seed_passes(&b, &control, &stats, &p.criterion));
}

#[test]
fn escape_selection_requires_integrity_and_mechanism_before_ranking() {
    let seed = json!({"passes": true, "margin": 0.2, "mechanism": {"screen_passes": true}});
    let mut seeds = vec![seed.clone(), seed.clone(), seed];
    seeds[2]["passes"] = json!(false);
    let near = point_verdict(7, (0.001, 32.0), &seeds, true);
    let far = point_verdict(0, (0.0001, 16.0), &seeds, true);
    assert_eq!(select(&[far.clone(), near.clone()]), Some(7));
    assert_eq!(
        select(&[point_verdict(7, (0.001, 32.0), &seeds, false)]),
        None
    );
    seeds[0]["mechanism"]["screen_passes"] = json!(false);
    assert_eq!(
        select(&[point_verdict(7, (0.001, 32.0), &seeds, true)]),
        None
    );
    seeds[0]["mechanism"]["screen_passes"] = json!(true);
    seeds[2]["passes"] = json!(true);
    let more = point_verdict(0, (0.0001, 16.0), &seeds, true);
    assert_eq!(select(&[near.clone(), more]), Some(0));
    seeds[2]["passes"] = json!(false);
    for s in &mut seeds {
        s["margin"] = json!(0.3);
    }
    let better = point_verdict(0, (0.0001, 16.0), &seeds, true);
    assert_eq!(select(&[near, better]), Some(0));
    seeds[1]["passes"] = json!(false);
    assert_eq!(
        select(&[point_verdict(0, (0.0001, 16.0), &seeds, true)]),
        None
    );
}

fn export(out: &Path) {
    let start = std::time::Instant::now();
    let p = declaration();
    let plan = load_plan();
    let points = grid(&p);
    let source_paths = command_output(
        "git",
        &[
            "ls-files",
            "src",
            "Cargo.lock",
            "Cargo.toml",
            "rust-toolchain.toml",
            "configs/continuous_stationary.toml",
        ],
    );
    let mut hashes = serde_json::Map::new();
    for path in source_paths.lines().chain([
        "tests/m4_continuous_acquisition.rs",
        "tests/support/m4_escape_sweep.rs",
        ESCAPE_PLAN,
        PLAN_PATH,
    ]) {
        hashes.insert(path.to_owned(), json!(sha(path)));
    }
    write_json(
        out.join("provenance.json"),
        &json!({"task": "M4-08c", "plan": p,
        "plan_sha256": sha(ESCAPE_PLAN), "source_sha256": hashes,
        "revision": command_output("git", &["rev-parse", "HEAD"]),
        "git_status": command_output("git", &["status", "--porcelain"]), "dirty": true,
        "rustc": command_output("rustc", &["--version"]), "cargo": command_output("cargo", &["--version"]),
        "platform": command_output("uname", &["-a"]), "profile": "release"}),
    );
    for (index, point) in points.iter().enumerate() {
        std::fs::write(
            out.join(format!("point_{index:02}.toml")),
            resolved_toml(&point_config(&plan, *point)).unwrap(),
        )
        .unwrap();
    }
    std::fs::write(
        out.join("continuous_b3.toml"),
        resolved_toml(&nonplastic_config(&plan)).unwrap(),
    )
    .unwrap();
    let new_stream = |name| {
        std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(out.join(name))
                .unwrap(),
        )
    };
    let mut records = new_stream("records.jsonl");
    let mut series = new_stream("series.jsonl");
    let mut controls = Vec::new();
    let mut ticks = 0u64;
    let mut lifetimes = 0usize;
    let mut rows = 0usize;
    for &outer in &plan.outer_seeds {
        write_json(
            out.join(format!("started_b3_{outer}.json")),
            &json!({"outer": outer, "condition": "continuous_b3"}),
        );
        let cfg = nonplastic_config(&plan);
        let mut actor =
            NoLearningActor::new(&cfg, plan.root_seed, &plan.namespace, outer, 0).unwrap();
        let w0 = actor.inherited().weights.w0.clone();
        let s = run_actor_ordinary(
            &cfg,
            plan.root_seed,
            &plan.namespace,
            outer,
            0,
            "continuous_b3",
            &mut actor,
        )
        .unwrap();
        let record = control_record(&plan, outer, &s);
        assert_eq!(
            record,
            archived_record(&p, outer, "continuous_b3"),
            "B3 archive mismatch: halt"
        );
        ticks += s.health.as_ref().unwrap().ticks_observed;
        lifetimes += 1;
        append(
            &mut records,
            &json!({"grid_index": null, "m4_07_record": record}),
        );
        for c in &s.choices {
            append(
                &mut series,
                &json!({"grid_index": null, "outer_seed": outer, "condition": "continuous_b3",
                "event_id": c.event_id, "cue": c.cue, "action": c.action, "correct": c.correct,
                "reward": c.reward, "noise_bit": c.noise_bit, "commit_tick": c.commit_tick, "feedback_tick": c.feedback_tick}),
            );
            rows += 1;
        }
        records.flush().unwrap();
        series.flush().unwrap();
        controls.push((outer, s, w0));
    }
    let mut seeds_by_point = vec![Vec::new(); points.len()];
    let order = std::iter::once(ANCHOR).chain((0..points.len()).filter(|i| *i != ANCHOR));
    for index in order {
        let cfg = point_config(&plan, points[index]);
        for (outer, control, w0) in &controls {
            write_json(
                out.join(format!("started_point_{index:02}_{outer}.json")),
                &json!({"grid_index": index, "outer": outer}),
            );
            let s = run_continuous_lifetime(
                &cfg,
                plan.root_seed,
                &plan.namespace,
                *outer,
                0,
                "continuous_b4",
            )
            .unwrap();
            let record = plastic_record(&plan, &cfg, *outer, &s);
            if index == ANCHOR {
                assert_eq!(
                    record,
                    archived_record(&p, *outer, "continuous_b4"),
                    "replication anchor mismatch: halt"
                );
            }
            assert_eq!(&s.w0, w0);
            assert_paired(&s, control);
            assert_eq!(s.resets, vec![0]);
            assert_eq!(s.outcomes, plan.outcomes_per_lifetime);
            assert_eq!(s.commitments, s.outcomes);
            assert_eq!(s.choices.len(), s.outcomes as usize);
            assert_eq!(s.health.ticks_observed, s.ticks);
            assert!(s.ticks <= p["budget"]["maximum_ticks_per_lifetime"].as_u64().unwrap());
            let behavior = behavior_stats(
                cfg.environment.cue_count,
                &continuous_rows(&s),
                &plan.acquisition_windows,
            );
            let b3 = behavior_stats(
                cfg.environment.cue_count,
                &baseline_rows(control),
                &plan.acquisition_windows,
            );
            let updates = update_stats(
                s.choices.iter().map(|c| &c.update),
                &s.final_p,
                &s.final_e,
                s.final_baseline,
                cfg.learning.as_ref().unwrap().plastic_bound,
            );
            let mechanism = mechanism(
                &s,
                &cfg,
                plan.acquisition_windows.late_exposures,
                p["mechanism"]["late_delta_floor"].as_f64().unwrap(),
            );
            seeds_by_point[index].push(json!({"outer_seed": outer,
                "late_accuracy": behavior.late_macro_accuracy, "b3_late_accuracy": b3.late_macro_accuracy,
                "margin": behavior.late_macro_accuracy - b3.late_macro_accuracy,
                "passes": seed_passes(&behavior, &b3, &updates, &plan.criterion), "mechanism": mechanism}));
            append(
                &mut records,
                &json!({"grid_index": index, "eta": points[index].0, "tau_e": points[index].1,
                "m4_07_record": record, "mechanism": mechanism, "config_sha256": config_sha256(&cfg),
                "w0_sha256": format!("{:x}", Sha256::digest(serde_json::to_vec(&s.w0).unwrap())),
                "schedule_sha256": format!("{:x}", Sha256::digest(serde_json::to_vec(&schedule_from_continuous(&s)).unwrap()))}),
            );
            for c in &s.choices {
                append(
                    &mut series,
                    &json!({"grid_index": index, "outer_seed": outer, "condition": "continuous_b4",
                    "event_id": c.event_id, "cue": c.cue, "action": c.action, "correct": c.correct, "reward": c.reward,
                    "noise_bit": c.noise_bit, "commit_tick": c.commit_tick, "feedback_tick": c.feedback_tick,
                    "baseline_old": c.update.baseline_old, "baseline_new": c.update.baseline_new, "delta": c.update.delta,
                    "eligibility_l1": c.eligibility_l1_before_update, "raw_l1": l1(&c.update.raw_updates),
                    "limited_l1": l1(&c.update.limited_updates), "actual_l1": l1(&c.update.actual_updates)}),
                );
                rows += 1;
            }
            ticks += s.ticks;
            lifetimes += 1;
            records.flush().unwrap();
            series.flush().unwrap();
        }
    }
    assert_eq!(
        lifetimes,
        p["budget"]["total_lifetimes"].as_u64().unwrap() as usize
    );
    assert_eq!(
        rows,
        p["budget"]["total_outcomes"].as_u64().unwrap() as usize
    );
    assert!(ticks <= p["budget"]["maximum_total_ticks"].as_u64().unwrap());
    let verdicts: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, point)| point_verdict(i, *point, &seeds_by_point[i], true))
        .collect();
    let selected = select(&verdicts);
    let verdict = json!({"schema_version": 1, "task": "M4-08c", "plan_sha256": sha(ESCAPE_PLAN),
        "criterion": plan.criterion, "points": verdicts, "selected_grid_index": selected,
        "integrity_passes": true, "failures": [], "anchor_records_exact": 3, "b3_records_exact": 3,
        "replication_scope": "Every saved M4-07 aggregate record field equals parsed archive exactly; M4-07 did not save full trajectories, so this is not a bitwise trajectory claim.",
        "measured_budget": {"lifetimes": lifetimes, "outcomes": rows, "ticks": ticks},
        "elapsed_seconds": start.elapsed().as_secs_f64(),
        "records_sha256": sha(out.join("records.jsonl")), "series_sha256": sha(out.join("series.jsonl")),
        "claim_limits": p["claim_limits"]});
    write_json(out.join("verdict.json"), &verdict);
    eprintln!(
        "M4-08c: {lifetimes} lifetimes, {rows} outcomes, {ticks} ticks, selected={selected:?}"
    );
}

#[test]
#[ignore = "39 bounded release lifetimes; requires fresh CRA_M4_ESCAPE_DIR"]
fn m4_escape_sweep_export() {
    if cfg!(debug_assertions) {
        panic!("empirical export requires --release");
    }
    let out = PathBuf::from(
        std::env::var_os("CRA_M4_ESCAPE_DIR").expect("fresh CRA_M4_ESCAPE_DIR required"),
    );
    std::fs::create_dir(&out).expect("evidence directory must be fresh");
    let result = std::panic::catch_unwind(|| export(&out));
    if let Err(error) = result {
        let message = error
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "non-string panic".to_owned());
        write_json(
            out.join("FAILED.json"),
            &json!({"status": "failed", "reason": message,
            "interpretation": "Incomplete or failed execution is not a scientific null. Preserve all artifacts; investigate without rerunning."}),
        );
        std::panic::resume_unwind(error);
    }
}
