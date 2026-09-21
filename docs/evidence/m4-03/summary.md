# M4-03 evidence — birth-only resets and warmup semantics

Date: 2026-09-21 UTC. Base `cfc9092` + M4-01/M4-02 worktree changes.
Task: [M4-03](../to-do.md#m4---remove-artificial-trial-resets) (spec 9-10,
17.2).

## What changed

- `src/config.rs`: `validate_continuous_execution` — the primary
  no-reset condition requires `reset_policy = "birth_only"`, an
  `[actor]` section, `[learning]` with `enabled = true` and
  `trace_policy = "persistent"`, fixed gates (modulator absent or
  `fixed`), and no enabled search. Task kind, cue counts, timing, and
  noise/hazard schedules are deliberately not pinned (M4-05/M5 own
  those profiles through this same guard).
- `src/experiments/continuous.rs`: `run_continuous_lifetime` — drives
  the shared `Lifetime` machine in the M4-01 split order with no reset
  call of any kind, recording per-choice raw/limited/actual reports
  plus evaluator-side annotations. Returns `ContinuousSummary` with a
  `resets` audit holding exactly `[0]` (birth). Inheritance and agent
  RNGs reuse the outer-seed `init` / `actor_noise` / `tie_break` tuples
  shared with the episodic family, pairing conditions by construction.
  `ContinuousError` gains the `Sim` variant.
- `tests/continuous_runner.rs` (new, 6 tests): see below.

## Fixtures

- Guard rejects `episodic_diagnostic` resets, disabled learning,
  non-fixed modulator, and enabled search.
- 8-outcome stationary lifetime: `resets == [0]`, counts reconcile
  (8/8, event ids 0–7), final `P` telescopes to the summed per-choice
  actual updates at 1e-12 (any mid-lifetime reset would break the sum),
  final `E` nonzero (cross-choice carryover), baseline moved.
- 24-outcome volatile lifetime (stable 0.5, hazard 1.0): hidden
  reversals occur, `resets == [0]`, 24/24 complete, `P` still
  telescopes.
- Warmup (4 leading quiet ticks) accrues live eligibility (`E` nonzero
  entering the first cue; boundary evolves, neither freezes nor
  clears); every tick advances exactly once; 2/2 complete.
- Birth: same coordinates reproduce bit-identically; lifetime index 1
  draws independent agent streams; every birth is `P = E = 0`.
- Finish includes the final feedback: delay-1 arithmetic
  (`feedback_tick == commit_tick + 1`), `ticks == last outcome + 1`,
  `final_last_feedback` reconciled.

No log-rotation path exists in the library runners (`run.rs` writes
per-lifetime files), so there is no hook to trip there; the audit
covers every in-library boundary (cue changes, rewards, reversals,
post-warmup).

## Verification (this session)

- `cargo test --all-targets --locked`: 304 passed, 0 failed, 6 ignored
  (pre-existing set, unchanged; 298 before + 6 new).
- `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets
  --locked -- -D warnings`: clean.
- `python3 analysis/test_validate_logs.py`: 17 passed.

## Limits

Primary condition only: honestly named multi-condition profiles are
M4-04, timing variability M4-05, checkpoints M4-06. No acquisition or
continuity-performance claim yet (M4-07). Claim track `family_only`.
