# Agent continuation guide

Updated 2026-09-21 UTC after M1-01, from code revision `7132433`
(`docs updated`) with M1-01 changes uncommitted. This is a working handoff.
Check the tracker and Git state for newer work before claiming a task.

## Start here

1. Read [AGENTS.md](../AGENTS.md), then the current status and ownership in
   [to-do.md](../to-do.md#current-status---maintain-at-every-handoff).
2. Inspect `git status --short` and recent commits. Preserve existing work.
3. Read spec Sections 1–10 before implementation, then the sections linked
   by the selected task. Section 9 owns scientific tick ordering.
4. Claim the task and affected files in the tracker before editing. State
   the intended change and verification scope.
5. Use the [README commands](../README.md#verification) for verification;
   append evidence and update this handoff when its facts change.

## Current position and evidence

**M0-GATE passed and was re-verified. M1-01 verified. Next task: M1-02.**
There is no outstanding M1-01 blocker. The claim track remains `family_only`.
No reserved final-test outcomes have been inspected.

M1-01 added `src/agent/topology.rs` (directed Bernoulli mask on the `init`
stream, fixed last-index motor pools, receiver-grouped edge order, cycle +
per-pool reachability checks with rejection logging) with 13 integration
tests in `tests/topology.rs` plus 2 unit tests. Full suite is 88 Rust tests
passing with clean fmt/clippy; see the M1-01 tracker ledger entry.

The [M0 review](m0-review.md) records 73 Rust tests and 14 Python tests
passing, clean fmt/clippy, four original runs audited, and seven fresh
bounded runs verified. These are recorded review results, not a claim that
a new session has rerun them. Exact commands and source/artifact hashes
are in [commands.json](evidence/m0-review/commands.json) and
[summary.json](evidence/m0-review/summary.json).

Raw `runs/` data is ignored by Git and may be absent in a fresh checkout.
The committed [valid audit fixture](../analysis/fixtures/valid/manifest.json)
is available for a quick audit. Reproduce missing raw runs using the saved
commands/configs into new directories; preserve historical evidence paths
and distinguish reruns from the original execution.

## Next task: M1-02

**Deliver:** inherited weights and neuron parameters — `W0` row scaling by
in-degree, input projection `B`, zero actor biases, and the starting
constants from spec Sections 6 and 10. Keep `W0` separate from future
plastic offsets.

Read spec Sections 6.3 and 10.2 alongside the
[M1 task queue](../to-do.md#m1---build-a-continuous-actor-with-no-learning).
Inspect `src/agent/topology.rs` (mask/edge order/motor assignment) and
`src/rng.rs` first. Weights also derive from the `init` stream; specify the
draw order after the mask draws (mask first, then `W0` on existing edges,
then `B`) before adding samples, so mask pairing across gate conditions is
preserved.

- Use standard deviation `recurrent_gain / sqrt(in_degree)`, not the
  variance as a standard deviation. Handle zero-in-degree rows explicitly.
- Validate motor capacity, disjoint pools, time constants, finiteness, and
  dimensions; missing edges stay zero.
- Verify with tight fixtures (row-scale statistics, zero-bias, finiteness)
  and keep `W0` immutable during a lifetime. Then run the applicable full
  quality checks and append tracker evidence.

Keep M1-03 dynamics and later learning/search work in their task order.
Completing M1-02 alone does not pass M1-GATE.

## Implementation map

| Existing component | Entry points | Coverage to preserve |
| --- | --- | --- |
| Config schema and execution guards | `src/config.rs`, `configs/` | `tests/config_validation.rs`; schema validation is distinct from execution |
| Seed derivation and stream names | `src/rng.rs` | `tests/seed_streams.rs`; existing golden seeds and environment schedules |
| Environment phases and commitments | `src/environment/{mod,schedule}.rs` | `tests/environment_contract.rs`, `tests/event_order.rs` |
| Public inputs versus hidden truth | `src/environment/{observation,hidden_state}.rs` | `tests/leakage.rs`; only ordinary data reaches `Agent` |
| B0/B1 and isolated O1 runner | `src/experiments/baseline.rs` | `tests/baselines.rs`, `tests/randomized_env.rs` |
| Run ownership and provenance | `src/run.rs` | atomic allocation unit tests, `tests/event_logging.rs` |
| Inherited topology (M1-01) | `src/agent/topology.rs` | `tests/topology.rs`; mask/motor/order fixtures, rejection logging |
| Ordinary/hidden event serialization | `src/logging/events.rs` | `tests/event_logging.rs`, `analysis/test_validate_logs.py` |
| CLI dispatch | `src/main.rs` | `validate-config` and baseline-only `simulate` |

## Carry-forward integration constraints

| Topic | Current implementation | Next responsibility |
| --- | --- | --- |
| Tick API | `Lifetime::advance` builds a tick and advances the environment before returning. `commit` follows a final response output. | M1-07 introduces the observe/finish split before neural integration. Preserve tick-20/delay-3 and final-feedback-transition fixtures. |
| Agent feedback | Ordinary runner calls `apply_feedback` once before `advance(features)`; selection reads policy state only. | Keep this ordering and information boundary when adding the actor. `TickOutput` belongs to the driver/evaluator. |
| Warmup | Replaces the first quiet interval; zero starts directly at cue presentation. | Preserve the documented M0 convention; use measured ticks for budgets. An additive-warmup change needs an explicit decision and new evidence. |
| Noise/hazard assignment | Stable membership shuffled at birth; noise rates cycle by cue index. | M5-02 owns factorial counterbalancing; current assignment is not a completed training-distribution implementation. |
| Event identity | IDs/choice indices restart per lifetime. Ordinary records carry lifetime identity; hidden rows align within contiguous lifetime blocks. | M1-09 must preserve exactly-once continuation and explicitly specify any schema change. A bare event ID is not a cross-lifetime join key. |
| Logging | `event_log=false` omits event streams; audit coverage then stops at provenance/completion. | Extend Rust validation, Python audit, fixtures, and versioning together when adding quantities. |
| Execution guards | M0 rejects neural/search sections, diagnostic resets, isolated reversal, and long life. | Enable each mode with its implementation/tests; never bypass guards to make a future config appear runnable. |
| Performance | Tick observations allocate; run logs are buffered. Simulation is serial. | Measure before scaling. M1 preallocates actor buffers; broader profiling/budget work remains in its queued milestones. |

The [decision log](decisions.md) preserves the rationale and superseding
corrections. The latest M0 review supersedes bootstrap statements that
simulation writes only provenance, observations are placeholders, or a
missing manifest is sufficient to claim a run directory.

## Seeds and artifact discipline

The JSON files in `manifests/` reserve namespace/range policy; they are not
executable suite definitions, consumed-seed ledgers, or inputs enforced by
the M0 CLI. Namespace validation and SHA-256 derivation provide stream
separation. Use the declared development namespace for M1 diagnostics and
record exact root/outer/lifetime coordinates. See the
[manifest guide](../manifests/README.md) before adding a suite.

Keep `spec.md`, archived review evidence, and original raw runs unchanged.
The spec's initial checklists and proposed commands remain design text;
the tracker and README describe implemented status. Put new results in a
new evidence record with the revision, dirty state, commands, and paths.

## End-of-session handoff

Update tracker ownership, verified task, blocker, and next eligible task.
Append completion evidence; link full run details rather than duplicating
large logs. Update this guide if entry points or integration limits changed,
and the README if executable commands changed. State which tests ran,
which were not applicable/unavailable, and what remains unverified.
