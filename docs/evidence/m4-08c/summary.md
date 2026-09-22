# M4-08c — persistent fixed-rule escape sweep (negative)

2026-09-22 UTC; base `08e4f82`, dirty worktree preserved. Linux x86_64,
rustc 1.98.0. No production code, base profile, spec, or earlier empirical
archive changed. [Pre-execution rationale](preflight.md) and
[manifest](../../../manifests/m4_escape_sweep.json) precede the run.

**None of 12 settings passes; nothing adopted.** Every point passes 0/3
seeds under the unchanged M4-07 criterion. This is a completed, valid
negative development experiment, not a passed M4 acquisition milestone.

## Declared scope and measured execution

Eta `{0.0001, 0.0003, 0.001, 0.003}` × tau_e `{16, 32, 64}`; frozen
M3-08 actor/input/noise/mask, fixed gate 1, persistent birth-only reset
policy, clean stationary profile. Development root 1, outers 1–3, lifetime
0; 2,000 outcomes each; first/final 100 exposures of each cue.
Late macro accuracy >=0.70 and matched B3 margin >=0.15, clipping <=0.10,
bound occupancy <=0.05, nonzero final P, zero failures, >=2/3 passing seeds.

The export ran **once**: 36 B4 plus 3 shared fresh B3 lifetimes, 78,000
outcomes, exactly 1,327,092/1,327,092 budgeted ticks, zero failures.
Elapsed export-body time 4.408 seconds (release test 4.54 seconds).
Anchor and controls ran before new settings. All 3 anchor and all 3 B3
records exactly match every saved M4-07 aggregate field, including health
and update statistics. This is aggregate-record reproduction; M4-07 did
not save full trajectories, so no bitwise trajectory claim is made.

All within-outer W0, event/cue/commit/feedback/noise schedules and mapping
targets match. Every B4 reset audit is `[0]`. The independent Python audit
recomputes per-cue windows, action counts, late mechanism means, raw-update
identities, margins and verdicts from all 78,000 saved scalar rows. It
also checks file hashes, counts, event identities, pairing and budgets.

## Results (late per-cue macro accuracy)

| eta | tau_e | Outer 1 | Outer 2 | Outer 3 | Passing seeds |
| --- | --- | --- | --- | --- | --- |
| 0.0001 | 16 | 0.000 | 0.000 | 0.875 | 0/3 |
| 0.0001 | 32 | 0.000 | 0.000 | 0.875 | 0/3 |
| 0.0001 | 64 | 0.000 | 0.000 | 0.875 | 0/3 |
| 0.0003 | 16 | 0.000 | 0.000 | 0.895 | 0/3 |
| 0.0003 | 32 | 0.000 | 0.000 | 0.900 | 0/3 |
| 0.0003 | 64 | 0.000 | 0.000 | 0.900 | 0/3 |
| 0.001 | 16 | 0.000 | 0.000 | 0.935 | 0/3 |
| 0.001 | 32 | 0.000 | 0.000 | 0.940 | 0/3 |
| 0.001 | 64 | 0.000 | 0.000 | 0.935 | 0/3 |
| 0.003 | 16 | 0.000 | 0.000 | 0.965 | 0/3 |
| 0.003 | 32 | 0.000 | 0.000 | 0.970 | 0/3 |
| 0.003 | 64 | 0.000 | 0.000 | 0.965 | 0/3 |
| Shared B3 | — | 0.000 | 0.000 | 0.875 | — |

On outers 1 and 2, **all 24 B4 lifetimes choose action 0 on all 2,000
outcomes**. Late mean abs(delta) is approximately 2.08e-17 / 1.98e-17,
despite late eligibility L1 spanning 133–376. Thus the behavioral lock and
teaching-signal starvation persist throughout this grid, independent of
the outer-3 ceiling in the criterion. Delta is tiny, not mathematically zero.

Outer 3 explores and improves by up to 0.095 over B3, but cannot meet the
frozen margin: B3 0.875 allows at most 0.125 improvement. This limitation
was noted before execution and the bar was preserved. No "best of 12"
setting is adopted. All P norms are positive (minimum 0.05556), maximum
clipping fraction 0.053904 <0.10, bound occupancy 0, maximum saturated
neuron-tick fraction 0.00002939, maximum absolute motor filter 0.68347.
Maximum raw-identity error 6.67e-16 (<1e-9 predeclared check).

## Artifacts and verification

- [Verdict](run/verdict.json), [aggregate records](run/records.jsonl),
  [per-outcome series](run/series.jsonl) (31,231,465 bytes).
- [Provenance](run/provenance.json): manifest snapshot, source hashes,
  revision/dirty status, compiler and platform. Resolved `point_00.toml`
  through `point_11.toml` and `continuous_b3.toml` reside in `run/`.
- `started_*.json` records identify each attempted lifetime. A failed
  export would leave `FAILED.json`; none occurred.
- Four new fast tests: declaration/budget/config scope, all-point small
  pairing and B3 reuse on all three outers, inherited absolute-bar/health
  judgments, and integrity/mechanism/selection rules.

Executed:

```bash
cargo test --locked --test m4_continuous_acquisition
CRA_M4_ESCAPE_DIR=docs/evidence/m4-08c/run cargo test --release --locked --test m4_continuous_acquisition escape::m4_escape_sweep_export -- --ignored --exact --nocapture
python3 analysis/audit_m4_escape.py docs/evidence/m4-08c/run
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
python3 analysis/test_validate_logs.py
python3 analysis/validate_logs.py analysis/fixtures/valid
cargo run --release --locked -- validate-config configs/continuous_stationary.toml
git diff --check
```

Focused target: 6 passed, 3 ignored; empirical export: 1 passed. Full
battery: 339 Rust tests passed, 12 explicitly ignored, 2 compile-fail doc
tests passed, 17 Python log tests passed, fixture and scalar-series audits
passed, fmt/Clippy/config/diff clean. The unrelated historical ignored
empirical suites were not rerun. Initial test-harness compile/Clippy issues
were fixed during preflight, before the single successful empirical export.

SHA-256:

```text
ca00ca01b3d6913e0216f7c846a155556b25a1fc1fbaff4094d27620cd816770  manifests/m4_escape_sweep.json
54e7d02e47e946e7da2764389456a7cfb374edc83ce342b35739dadf020fcb22  run/verdict.json
fd3a1c47e5a90b2317b5c8d0f762ca2d90f45d52a46bacf8476d190e35dcfe5d  run/records.jsonl
5046d1aa71f12e2cb8dedb04e0258849b0de20146f84fa78bfea55c08b239e9b  run/series.jsonl
```

## Decision and limits

Follow the declared null route: [negative M4-09 bundle](../m4-09/summary.md)
and design escalation in [decisions](../../decisions.md). M4-07 and M4-GATE
stay open; M5/gates/evolution are not unlocked. These reused development
seeds and 12 settings do not exclude other fixed-rule settings or actor
families. No validation/final-test data was inspected, no post-result rule
was changed, and no follow-up tuning was run.
