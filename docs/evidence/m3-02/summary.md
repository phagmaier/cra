# M3-02 exactly-once feedback updates and baseline arithmetic

Executed 2026-09-21 UTC on linux/x86_64, base `3fdc541` (`m3-01 reviewed
and fixed`) with uncommitted M3-02 work. Scope is the unit-level
feedback-gated `P` update plus the running reward baseline: read old
`E`/gates/`P`/baseline, compute `delta` from the old baseline, clamp
per-edge raw updates, clamp the resulting `P`, update the baseline once,
mark the event consumed, and refresh the single `W0 + P` cache. No runner,
no gate heads, and no acquisition claim; the episodic runner is M3-04 and
the golden fixture is M3-03.

## What changed

- `src/agent/plasticity.rs` (extended): `PlasticState` now stores the
  running `reward_baseline` (`0.5` at birth, spec 10.4) and
  `last_feedback` dedup alongside `P`/`E`/cache. New
  `apply_feedback_once(event_id, reward, gates, eta, max_update,
  plastic_bound, baseline_beta, w0)` implements spec 7.4-7.5 and 9 step 2:
  `delta = reward - baseline_old`, `raw = eta * delta * gate[j] *
  E_old`, `limited = clamp(raw, ±max_update)`, `P_new = clamp(P_old +
  limited, ±plastic_bound)`, `baseline_new = baseline_old + beta *
  delta`, then `refresh_effective(w0)`. Fixed mode passes gate `1`;
  per-receiver gates are validated in `[0, 1]` for future M6 heads.
  `E` is read, never reset. Hyperparameters ride per call (validated
  against the same ranges as config validation) so tests state exact
  values; the runner will pass its resolved configuration consistently.
  `from_learning_config` sets the baseline from validated TOML
  `reward_baseline_initial`. The diagnostic fixed-baseline policy is
  unchanged in `experiments::finite_rollout` (frozen baseline, no running
  update); this method always performs the one running update.
- `PlasticSnapshot` bumped to schema 2 with `reward_baseline` and
  `last_feedback` (required even when null, so v1 files fail to parse
  instead of defaulting); restore validates the finite baseline plus all
  prior dimensions/mask/finiteness rules and recomputes the cache.
  `PlasticityError::DuplicateFeedback(u64)` rejects `event_id <=
  last_feedback` with all state unchanged; every other input is validated
  before any mutation, so rejected calls leave `P`/baseline/cache/dedup
  unchanged. New `FeedbackOutcome` reports `delta`, old/new baselines,
  and full `N x N` raw/limited/actual matrices (zero on
  missing/nonplastic) for the clipping-separation tests and future
  `record_raw_and_applied_updates` logging.
- `tests/feedback_updates.rs` (new, 11 tests) covers: old-baseline delta
  with one post-`delta` update and second-outcome chaining; duplicate and
  older-id rejection with bit-identical `P`/`E`/baseline/cache; `eta = 0`,
  all-zero gates, and `delta = 0` giving zero `P` movement; `max_update`
  clipping separating raw from limited on both signs; `plastic_bound`
  clipping separating limited from actual on both bounds; `W0` slice
  invariance with `effective == W0 + P`; motor-only nonplastic zero
  reports; per-receiver gate sharing with doubling; all invalid
  reward/gate/hyperparameter/`w0` inputs rejected without state change;
  snapshot round-trip with nonzero `P`/baseline/dedup plus post-restore
  dedup persistence; `from_learning_config` baseline carriage.

## Contracts checked

Spec 7.4 (baseline after `delta`, init `0.5`, per-outcome `beta`),
7.5 (gated per-edge update, both clamps in order, plastic mask only,
no `P` decay, `B`/bias never plastic), 9 step 2 (exactly-once pre-transition
consumption, no state clearing), 10.4 (birth baseline/dedup), 10.6
(`f64`, explicit nonfinite/duplicate errors, no silent clipping of `h`),
17.2 (missing/nonplastic never accrue; `W0` immutable; zero
`eta`/gate/`delta` gives zero task-dependent change; gate doubling;
second delivery rejected; traces not reset).

## Commands actually executed

```bash
cargo test --locked --test feedback_updates
cargo test --locked --test plasticity
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
```

Results: `tests/feedback_updates` 11/11 passed; `tests/plasticity`
16/16 still passed (snapshot tests use the version constant, so v2
round-trips without modification); full fast suite **243 passed, 0
failed, 3 default ignores**; doc tests 2/2 compile-fail checks passed;
formatting and Clippy clean. The three default ignores are the two
separately invoked Monte Carlo diagnostics (M2-03, M2-05) and the
historical weight-printing probe. No CLI smoke run or Python analysis
test was rerun: this task adds a unit-level update entry point but
enables no new production execution mode (`simulate` guards still reject
enabled learning; no runner calls the new method yet).

## SHA-256 identities

| File | SHA-256 |
| --- | --- |
| src/agent/plasticity.rs | e9975e72aeae341899a446dde89679a279c653ef31b7c34876f0acb4fdb463b7 |
| tests/feedback_updates.rs | 4bf26bfccb8ee986301c04d93baac5a5eda8eddb6b5ae0ddfea44f89b12b61ff |
| tests/plasticity.rs | 53b82f9106befb58df4c91a3f39883eee3619a0b5d1844096efe99ddc9418f08 |

## Claim limits

This task proves update arithmetic and exactly-once bookkeeping on
fixtures; it does not demonstrate learning. No runner wires the new
method, no gate is learned, and no acquisition/control comparison is
claimed. The hand-calculated Section 17.3 golden (`0.4`/`0.67`/`0.4`/
`0.00067`/`0.10067`/`0.64`) is M3-03 work; the episodic stationary
runner is M3-04; checkpoint embedding with nonzero `P`/`E` remains
M3-10/M4-06 (the versioned snapshot is ready but the top-level
checkpoint schema stays 2).
