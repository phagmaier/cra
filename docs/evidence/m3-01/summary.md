# M3-01 plastic offsets, eligibility, and plastic masks

Executed 2026-09-21 UTC on linux/x86_64, base `38c4ffd` (`m2 done`) with
uncommitted M3-01 work. Scope is limited to lifetime plastic state storage,
eligibility accumulation, plastic masks, and the single effective-weight
refresh. Feedback-gated `P` updates, the reward baseline, the episodic
runner, and evolution remain M3-02/M3-04/M7 work.

## What changed

- `src/agent/plasticity.rs` (new): `PlasticState` stores `P` and `E`
  separately from immutable `W0`, builds the plastic mask for
  `all_recurrent_edges` or `motor_afferent_only`, accumulates eligibility
  (`E_new = lambda * E_old + S`) under `persistent` (`lambda =
  exp(-1 / tau_e)`) or `no_decay_diagnostic` (`lambda = 1.0`, never a large
  `tau_e`), and exposes `refresh_effective` as the single writer of the
  `W0 + P` cache. `PlasticSnapshot` is the versioned, `deny_unknown_fields`
  serialization unit; `PlasticState::restore` validates dimensions,
  finiteness, mask agreement, zero on missing/nonplastic, `tau_e`, and
  schema before recomputing the cache.
- `src/agent/actor.rs`: added `step_with_effective_weights` and
  `step_with_effective_and_perturbations`. The dense drive core is now
  parameterized by the recurrent matrix while sensory `B` and bias still
  come from inherited parameters. `step`/`step_with_perturbations`
  delegate through `&weights.w0`, so the no-learning path is unchanged.
- `src/agent/mod.rs`, `src/lib.rs`: module wiring and updated scope docs.

`W0` is never mutated. Biases, sensory weights, and (future) modulator
weights are not lifetime-plastic. Nonzero `P`/`E` on missing or nonplastic
edges is rejected at construction, advance, refresh, and restore.

## Contract checks

`tests/plasticity.rs` (16 tests) covers:

- Birth `P = E = 0`, `W_effective = W0` for both masks and both trace
  policies; `W0` never mutated; cache stays `W0 + P` with missing edges at
  exactly `0.0`.
- Mask construction is a subset of the structural mask; motor-only keeps
  only edges into `motor0 ∪ motor1`; stable receiver-grouped edge order.
- Eligibility uses the receiving neuron's `xi` on all its incoming plastic
  edges, one `alpha_h` factor, actual `sigma`, and no activation
  derivative; zero presynaptic activity yields zero score; zero/invalid
  noise and invalid `alpha_h` are rejected even when activity and
  perturbation are zero.
- The spec 17.3 eligibility arithmetic: `alpha 0.5`, `r_old 0.2`,
  `xi 0.4`, `sigma 0.1`, old `E 0.3`, `lambda 0.9` gives `0.67` for
  `persistent` and `0.70` for `no_decay_diagnostic`; the policies differ.
- Snapshots round-trip through a JSON file and reject wrong schema,
  dimensions, mask disagreement, unknown trace policy, bad `tau_e`,
  unknown/missing fields, and nonfinite `P`/`E`.
- The actor effective path reproduces `step` bitwise when `P = 0`, applies
  a validated `P` offset through the shared core without changing `B` or
  bias, and rejects nonzero missing-edge, ragged, and nonfinite matrices.
- `PlasticState::from_learning_config` matches the direct constructor and
  accepts the shipped `debug_stationary` learning section.

## Commands actually executed

```bash
cargo fmt --all
cargo test --locked --test plasticity
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
```

Results: `tests/plasticity` 16/16 passed; full fast suite **232 passed, 0
failed, 3 default ignores**; doc tests 2/2 compile-fail checks passed;
formatting and Clippy clean. The three default ignores are the two
separately invoked Monte Carlo diagnostics (M2-03, M2-05) and the
historical weight-printing probe. No CLI smoke run or Python analysis test
was rerun: this task adds serializable state and an actor entry point but
enables no new production execution mode (`simulate` guards still reject
enabled learning).

## SHA-256 identities

| File | SHA-256 |
| --- | --- |
| src/agent/plasticity.rs | 2c9909c3fea3ee7b7dbf84b97672d3164cf41c25bd97efb32471ed1aa0c911ef |
| src/agent/actor.rs | 08b9a7b4ec7ff7b075e85f68382f2336b938a1326255a22a76ab44c333c0b7ce |
| tests/plasticity.rs | 53b82f9106befb58df4c91a3f39883eee3619a0b5d1844096efe99ddc9418f08 |

## Claim limits

This task stores and validates plastic state and accumulates eligibility;
it does not demonstrate learning. No `P` is ever updated from reward, no
baseline exists, and no acquisition claim is made. `no_decay_diagnostic`
is an explicitly named diagnostic policy, distinct from the main
`persistent` condition and rejected under `birth_only` by config
validation. The top-level checkpoint schema remains 2; embedding this
snapshot and proving split replay with nonzero `P`/`E` is M3-10/M4-06 work.
