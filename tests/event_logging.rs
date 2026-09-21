//! Event-logging round-trips (M0-13).
//!
//! A multi-lifetime simulation run writes ordinary + hidden streams,
//! condition identity, and a completion record; reading them back
//! reconstructs reward counts and schedule identity exactly. Disabling
//! logging changes nothing about the simulated behavior (only the event
//! files disappear). Tampered logs fail validation instead of looking
//! successful.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::logging::events::{read_json, validate_stream};
use cra::run::{BaselineSel, EffectiveSeeds, read_completion, read_run_streams, run_simulation};
use support::base_config;

fn temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("cra-{}-{}", name, std::process::id()))
}

fn two_lifetime_config() -> cra::config::Config {
    base_config()
}

#[test]
fn round_trip_reconstructs_counts_and_schedule() {
    let cfg = two_lifetime_config();
    cra::config::validate(&cfg).expect("valid");
    let seeds = EffectiveSeeds::from_config(&cfg);
    let base = temp_dir("roundtrip");
    let report = run_simulation(&cfg, &seeds, BaselineSel::Random, 2, &base).expect("run");
    assert_eq!((report.commitments, report.outcomes), (12, 12));

    let (events, hidden) = read_run_streams(&report.dir).expect("read back");
    assert_eq!(events.len(), 12);
    // Event ids restart per lifetime in M0, so each lifetime block
    // validates as its own stream (same rule the runner enforces). Hidden
    // records are stored in the same per-lifetime order.
    let mut epos = 0usize;
    let mut hpos = 0usize;
    while epos < events.len() {
        let lifetime = events[epos].lifetime_index;
        let start = epos;
        while epos < events.len() && events[epos].lifetime_index == lifetime {
            epos += 1;
        }
        let n = epos - start;
        validate_stream(&events[start..epos], &hidden[hpos..hpos + n], 2)
            .expect("valid block stream");
        hpos += n;
    }
    assert_eq!(hpos, hidden.len());
    // Per-lifetime blocks each hold 6 contiguous choices.
    for lifetime in 0..2 {
        let block: Vec<_> = events
            .iter()
            .filter(|e| e.lifetime_index == lifetime)
            .collect();
        assert_eq!(block.len(), 6);
        for (i, event) in block.iter().enumerate() {
            assert_eq!(event.choice_index, i as u64);
        }
    }
    // Reward counts reconstruct exactly from the ordinary stream.
    let reward_sum: f64 = events.iter().map(|e| e.reward).sum();
    assert_eq!(reward_sum, report.mean_reward * report.outcomes as f64);

    let completion = read_completion(&report.dir).expect("completion");
    assert_eq!((completion.commitments, completion.outcomes), (12, 12));

    let condition: cra::logging::events::ConditionInfo =
        read_json(&report.dir, "condition.json").expect("condition");
    assert_eq!(condition.condition_id, "B0");
    assert_eq!(condition.policy, "random");

    let manifest: serde_json::Value = read_json(&report.dir, "manifest.json").expect("manifest");
    assert_eq!(manifest["condition_id"], "B0");
    assert!(manifest.get("platform").is_some());
    assert!(manifest.get("rustc_version").is_some());
}

#[test]
fn logging_off_yields_identical_behavior() {
    let mut on_cfg = two_lifetime_config();
    on_cfg.logging.event_log = true;
    let mut off_cfg = two_lifetime_config();
    off_cfg.logging.event_log = false;
    for cfg in [&on_cfg, &off_cfg] {
        cra::config::validate(cfg).expect("valid");
    }
    let seeds = EffectiveSeeds::from_config(&on_cfg);
    let on = run_simulation(
        &on_cfg,
        &seeds,
        BaselineSel::Constant(1),
        2,
        &temp_dir("logon"),
    )
    .expect("run");
    let off = run_simulation(
        &off_cfg,
        &seeds,
        BaselineSel::Constant(1),
        2,
        &temp_dir("logoff"),
    )
    .expect("run");
    assert_eq!(
        (on.commitments, on.outcomes, on.mean_reward),
        (off.commitments, off.outcomes, off.mean_reward),
        "logging must not draw randomness or affect behavior"
    );
    assert!(on.dir.join("events.jsonl").exists());
    assert!(!off.dir.join("events.jsonl").exists());
    assert!(!off.dir.join("hidden.jsonl").exists());
    // Completion accounting still works with logging off.
    let completion = read_completion(&off.dir).expect("completion");
    assert_eq!((completion.commitments, completion.outcomes), (12, 12));
}

#[test]
fn tampered_logs_fail_validation() {
    let cfg = two_lifetime_config();
    cra::config::validate(&cfg).expect("valid");
    let seeds = EffectiveSeeds::from_config(&cfg);
    let report =
        run_simulation(&cfg, &seeds, BaselineSel::Random, 1, &temp_dir("tamper")).expect("run");
    // Append a duplicate of the first line: the stream no longer validates.
    let path = report.dir.join("events.jsonl");
    let text = std::fs::read_to_string(&path).expect("read");
    let first = text.lines().next().expect("nonempty").to_owned();
    std::fs::write(&path, format!("{text}{first}\n")).expect("tamper");
    let (events, hidden) = read_run_streams(&report.dir).expect("read back");
    assert_eq!(events.len(), 7);
    assert!(validate_stream(&events, &hidden, 2).is_err());
}
