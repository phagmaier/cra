# M3-03 hand-calculated golden update fixture

Executed 2026-09-21 UTC on linux/x86_64, base `3fdc541` with uncommitted
M3-02/M3-03 work. Scope is the Section 17.3 arithmetic chain through the
public entry points, independent of ordinary configured time constants
(explicit `alpha_h = 0.5`, `lambda_e = 0.9` via `tau_e = -1 / ln(0.9)`).
No production code changes; new fixture file only. The episodic runner is
M3-04.

## What changed

- `tests/golden_updates.rs` (new, 4 tests): `conditional_score(0.5, 0.2,
  0.4, 0.1) = 0.4`; `advance_eligibility` from old `E = 0.3` gives `0.67`;
  `apply_feedback_once` with reward `1.0`, old baseline `0.6`,
  `beta = 0.1`, `eta = 0.01`, gate `0.25` gives `delta = 0.4`,
  raw `0.00067`, new `P = 0.10067`, new baseline `0.64` with wide bounds;
  separate per-edge-clamp and bound-clamp cases stay distinct from the
  unclipped golden. All comparisons at `1e-12`–`1e-15`, documented below.

## Contracts checked

Spec 17.3 (exact values above, old baseline produces `delta`, baseline
updates once after), 17.2 (clipped vs unclipped separated, `W0`
immutable, non-carrying edges stay `0.0`).

## Numerical note

`actual = (P_old + limited) - P_old` round-trips through one
addition/subtraction, so the unclipped `actual == limited` comparison
allows `1e-15` rather than bitwise equality (observed 1-ulp difference,
`3.8e-18`). `limited == raw` under no per-edge clipping is an exact
`clamp` identity and stays `assert_eq!`. `lambda_e = 0.9` arrives as
`exp(-1 / tau)` with `tau = -1 / ln(0.9)`, matching the M3-01 precedent.

## Commands actually executed

```bash
cargo test --locked --test golden_updates
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
```

Results: `tests/golden_updates` 4/4 passed; full fast suite **247
passed, 0 failed, 3 default ignores**; doc tests 2/2 compile-fail checks
passed; formatting and Clippy clean. The three default ignores are the
two separately invoked Monte Carlo diagnostics (M2-03, M2-05) and the
historical weight-printing probe. No CLI smoke run or Python analysis
test was rerun: fixture-only task with no production execution change.

## SHA-256 identities

| File | SHA-256 |
| --- | --- |
| tests/golden_updates.rs | 7b81be19c550c7cfd3be107487afb5b015a6b6555a1a956300aae698b519ce0d |
| src/agent/plasticity.rs | e9975e72aeae341899a446dde89679a279c653ef31b7c34876f0acb4fdb463b7 |
| tests/feedback_updates.rs | 4bf26bfccb8ee986301c04d93baac5a5eda8eddb6b5ae0ddfea44f89b12b61ff |

## Claim limits

This task proves the arithmetic chain on one fixture; it does not
demonstrate acquisition. No runner, controls, or multi-seed comparison is
claimed. M3-04 builds the episodic stationary runner next.
