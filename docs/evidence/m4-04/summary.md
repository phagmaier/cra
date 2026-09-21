# M4-04 evidence — three continuity conditions and profiles

Date: 2026-09-21 UTC. Base `cfc9092` + M4-01/M4-02/M4-03 worktree changes.
Task: [M4-04](../to-do.md#m4---remove-artificial-trial-resets) (spec 16/M4,
5.7, 7.7, 8.3, 10, 17.2).

## What changed

- `src/agent/plasticity.rs`: `PlasticState::reset_traces_event_diagnostic`
  — clears only `E` (preserving `P`/baseline/dedup/cache), restricted to
  the `persistent` policy.
- `src/config.rs`: `validate_event_reset_execution` — requires
  `reset_policy = "event_reset_diagnostic"` with `trace_policy =
  "persistent"`, enabled learning, fixed gates, no search; task and
  timing/noise schedules not pinned. Each of the three continuity
  guards now rejects the other two conditions' reset policies.
- `src/experiments/continuous.rs`: `CONTINUOUS_MODE` /
  `EVENT_RESET_MODE` labels; `ContinuousSummary.mode` (set by both
  runners); `ContinuousLearner::reset_traces_event_diagnostic`;
  `run_event_reset_lifetime` — same split-order loop, inheritance, RNG
  streams, and arithmetic as the main runner, clearing only `E` after
  each non-final feedback tick and logging every clear in `resets`.
- `configs/continuous_stationary.toml` (new): executable primary
  profile, section-identical twin of the `debug_stationary` source
  (only `profile_name` differs; verified by diff).
  `configs/debug_stationary.toml`: header refreshed to its source role
  (values untouched — reference tests unaffected).
- `tests/continuity_conditions.rs` (new, 6 tests): see below.

## Distinction proof

- Guard matrix: episodic / continuous / event-reset guards each accept
  exactly their own config and reject the other two (6 rejections).
- The verify clause by name: an `event_reset_diagnostic` config (trace
  policy `persistent`) is rejected from `validate_continuous_execution`
  and `run_continuous_lifetime`; the `birth_only` config is rejected
  from the event-reset entry points.
- Same-seed pairing: continuous vs event-reset share `w0`,
  initialization record, and the full cue schedule, but carry distinct
  mode/reset-policy labels, `[0]` vs per-outcome reset audits (reset
  ticks == feedback ticks + 1), and divergent final `P`/`E`.
- All three summaries identify `(mode, reset_policy, trace_policy)`:
  `episodic_diagnostic` / `continuous_persistent` /
  `event_reset_diagnostic` with their distinct policies.
- Unit complement: the `E`-clear zeroes traces while `P`/baseline/
  dedup survive and refuses the diagnostic policy.
- Both checked-in profiles validate; `continuous_stationary` executes
  through the library runner at a small outcome override (the
  checked-in 2,000-outcome task is study-shaped, not a unit scale).

## Verification (this session)

- `cargo test --all-targets --locked`: 310 passed, 0 failed, 6 ignored
  (pre-existing set, unchanged; 304 before + 6 new).
- `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets
  --locked -- -D warnings`: clean.
- `python3 analysis/test_validate_logs.py`: 17 passed.
- `validate-config configs/continuous_stationary.toml` and
  `validate-config configs/debug_stationary.toml`: both OK (release).

## Limits

Conditions and profiles only: no timing variability (M4-05), no
checkpoints (M4-06), no acquisition comparison (M4-07). Claim track
`family_only`.
