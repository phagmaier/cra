# M4-01 evidence — authoritative main tick order

Date: 2026-09-21 UTC. Base `cfc9092` (worktree dirty: M4-01 changes only).
Task: [M4-01](../to-do.md#m4---remove-artificial-trial-resets) (spec 9, 17.2).

## What changed

- `src/environment/mod.rs`: `Lifetime::advance()` split into public
  `observe()` (build tick + attach due feedback, no clock advance, no RNG
  draws) and public `finish_tick()` (phase machine + clock). `advance()`
  is now exactly `observe` + `finish_tick` (parity pinned). Delivery
  bookkeeping (`pending` taken, `outcomes` inc, `consumed` push) lives in
  `observe`; re-observing a feedback tick without `finish_tick` errors
  instead of double-delivering.
- `src/experiments/{baseline,episodic,reduction}.rs`: all six driver
  loops (ordinary B0/B1/B3, oracle, B4, B3-episodic, shuffled, permuted
  reduction) now run observe → apply → agent-step → finish → commit with
  spec-9 step comments. Finish precedes commit so the transient
  `Committed` phase, `commit_tick = tick - 1`, and golden delay
  accounting are unchanged (equivalence note in `docs/decisions.md`).
- `src/experiments/episodic.rs` `advance_inner`: eligibility (spec 9
  step 6) now runs before the motor update (step 7). Independent state
  (eligibility reads saved `r_old` + `xi`, writes `E`; motor reads new
  `r`, writes `q`), so documentary, not behavioral. Gate stays fixed 1;
  no modulator (M6).
- `tests/main_tick_order.rs` (new, 4 tests): see below.

## Fixture (tick-20/delay-3, real learner, split driver)

Config: warmup 4, quiet [4,4], cue 8, gap [5,5], response 4, delay
[3,3], K = 2, zero noise/hazard, 1 outcome, `episodic_diagnostic` resets
(never fires on a single outcome) + winner-like learning
(eta 0.001, max_update 0.01, bound 0.5, beta 0.02, motor-afferent N = 4).
Seeds root 1 / development / outer 1 / lifetime 0 (birth `[1, 1]`,
first cue 1, as in `tests/event_order.rs`).

- Commitment only at tick 20 (correct action 1 via production `commit`);
  delay ticks 21-22 ordinary with the action latch visible; exactly one
  feedback at tick 23 start (`commit_tick` 20, `outcome_tick` 23).
- `P`/`E`-zero at birth; `P`, baseline, and dedup unchanged by every
  advance through tick 22; the tick-23 update equals
  `eta * delta * 1.0 * E_old` per edge (1e-15), with the ordered
  max_update/plastic_bound clamps and one baseline update verified
  against the stored reports.
- Advancing on the tick-23 features extends `E` (L1 differs), and
  recomputing raw updates with post-feedback `E` changes their L1 —
  feedback-evoked scores cannot explain the consumed outcome.
- Duplicate delivery rejected without state change (exactly-once).
- Companion tests: `advance` on outcome-shaped features never moves
  `P`/baseline/dedup (one feedback path); re-observe without finish
  errors with exactly one delivery; fused `advance` ≡ observe+finish
  over a 2-outcome lifetime (identical `TickOutput`s and end state).

## Verification (this session)

- `cargo test --all-targets --locked`: 293 passed, 0 failed, 6 ignored
  (pre-existing set, unchanged by this task; 289 before + 4 new).
- `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets
  --locked -- -D warnings`: clean.
- `python3 analysis/test_validate_logs.py`: 17 passed.
- Release smoke: `validate-config configs/debug_stationary.toml` OK;
  `simulate --config configs/env_smoke.toml --baseline oracle
  --lifetimes 2 --seed 1` → `runs/env_smoke-root1-outer1-1790027025`
  (16/16, reward 1.0), audit OK; `simulate --config
  configs/actor_no_learning.toml --baseline actor --lifetimes 2 --seed 1`
  → `runs/actor_no_learning-root1-outer1-1790027031` (16/16, reward
  0.5), audit OK. Migrated oracle/B3 loops execute and audit clean.

## Limits

Order only: traces still `no_decay_diagnostic`, resets still
episodic-diagnostic, no continuous profile. Persistence (M4-02),
birth-only resets (M4-03), and continuity profiles (M4-04) remain.
Claim track `family_only`; no acquisition or continuity claim.
