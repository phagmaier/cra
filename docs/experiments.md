# Experiments

Append-only experiment log. Newest entries go at the end. Record the
question, code/configuration hashes, what changed, observed result,
interpretation, and next decision. Include null results, failures, and
interrupted runs — missing data is not a zero score.

## 2026-09-21 — M0-01–M0-05 bootstrap (no empirical result)

- Question: none (engineering scaffold, not an experiment).
- Code/config: M0 scaffold (`src/config.rs`, `src/rng.rs`, `src/run.rs`,
  thin CLI); configs `env_smoke.toml`, `debug_stationary.toml`; manifests
  reserve disjoint outer-seed ranges. See the `to-do.md` completion ledger
  for exact commands, outcomes, and artifact paths.
- Result: `cargo test`, `validate-config` on both profiles, and scaffold
  `simulate` provenance runs. No learning claim; no statistical check.
- Next: M0-06 (observation boundary types).

## 2026-09-21 — M0 environment evidence bundle (M0-15, no empirical claim)

- Question: none (milestone exit evidence, not an experiment).
- Code/config: environment core + baselines + logging + audit at base
  10a7ae8 with M0 work uncommitted (`git_dirty=true` in manifests);
  `configs/env_smoke.toml` (warmup 4, quiet [4,4], cue 8, gap [0,0],
  response 4, delay [1,1], K=2, eps 0, hazard 0, 8 outcomes/lifetime);
  seeds (root 1, development, outer 1, lifetimes 0-1).
- Commands and results (all release mode unless noted):
  - `cargo fmt --all -- --check` — clean.
  - `cargo clippy --all-targets --locked -- -D warnings` — clean.
  - `cargo test --all-targets --locked` — 62/62 pass (18 lib + 5
    baselines + 2 config + 17 contract + 3 event_logging + 3 event_order
    + 5 leakage + 4 randomized + 5 seeds).
  - `python3 analysis/test_validate_logs.py` — 6/6 pass.
  - `validate-config configs/env_smoke.toml` — OK.
  - `simulate --baseline random --lifetimes 2` → B0, 16/16 outcomes,
    mean reward 0.2500 (`runs/env_smoke-root1-outer1-1789963074/`).
  - `simulate --baseline constant-0 --lifetimes 2` → B1, 16/16, 0.5000
    (`...-retry1/`).
  - `simulate --baseline constant-1 --lifetimes 2` → B1, 16/16, 0.5000
    (`...-retry2/`).
  - `simulate --baseline oracle --lifetimes 2` → O1, 16/16, 1.0000
    (`...-retry3/`).
  - `python3 analysis/validate_logs.py` on all four run dirs — OK x4.
  - Randomized checks (fixed seeds, declared tolerances): mapping
    balance, cue frequency, B0 chance correctness, oracle eps-0.2 reward
    — all inside tolerance on first execution (no reseeding).
- Failure found and fixed: same-second runs overwrote one directory
  (broken retry break); fixed with regression test, colliding runs
  deleted and re-executed. See `docs/decisions.md`.
- Next: M1-01 (inherited topology). M0 smoke command for the M1 reader:
  `cargo run --release --locked -- simulate --config
  configs/env_smoke.toml --baseline random --seed 1` then
  `python3 analysis/validate_logs.py runs/<run-id>`.

## 2026-09-21 UTC — M0 corrective review and re-verification

- Scope: owner-requested review of completed M0, base `57b6870`, corrections
  in a dirty tree. No new scientific experiment or change to `spec.md`.
- Found and reproduced a false audit pass for a seven-of-eight truncated
  lifetime with matching shortened completion counts. Corrected audit,
  runner feedback/interface, run-directory ownership, execution validation,
  and invalid-input RNG handling; details in `docs/m0-review.md` and the
  appended decision record. Original outputs were preserved.
- Full checks: 73 Rust tests, 14 Python tests, fmt, and clippy pass. Existing
  timing/RNG goldens retained. The mapping-pair diagnostic used 2,048 births
  and a tolerance declared before execution: counts 516/502/520/510 pass.
- All four original smoke runs pass the stricter audit. Seven fresh runs
  (development/root 1/outer 1, two lifetimes each) also pass, including
  noisy variable timing and logging disabled. Every original clean smoke
  event is reproduced apart from run identity. The noisy oracle is 32/32
  latently correct with 27 rewards, five flipped outcomes, four changes,
  and 602 ticks; its logging-disabled counterpart reports the same results.
- Evidence: `docs/evidence/m0-review/commands.json` (exact commands/outputs),
  `summary.json` (code/binary/artifact hashes and counts), two diagnostic
  configs, and raw directories under `runs/m0-review/`. Logged-off audit
  coverage is explicitly limited to provenance/completion.
- Failure during audit development: the original duplicate-event fixture
  initially reported only a count mismatch; independent duplicate checking
  restored the diagnostic and all tests pass. No seeds or golden values
  changed to obtain a pass. The intentionally unsupported neural simulation
  exits 1 as expected and is recorded in the evidence.
- Result: M0-GATE re-verified. These are correctness checks, not evidence of
  learning or relative performance. Next eligible task remains M1-01.
