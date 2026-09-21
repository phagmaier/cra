# Agent continuation guide

Updated 2026-09-21 UTC after M1-GATE, from code revision `4d6aec1`
(`m1-08 done`) with M1-09 through M1-GATE changes uncommitted. This is
a working handoff. Check the tracker and Git state for newer work
before claiming a task.

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

**M0-GATE passed and was re-verified. M1-GATE passed. Next task: M2-01.**
There is no outstanding M1 blocker. The claim track remains `family_only`.
No reserved final-test outcomes have been inspected.

M1 built a continuous nonplastic actor (M1-01 topology → M1-09
checkpoints, M1-10 replay consolidation, M1-11 observability smokes,
M1-12 B3 demo command) and exits as a verified dynamical-system
foundation — explicitly not learning. Fresh gate battery: 173 Rust
tests passing (1 ignored probe), 15 Python tests, clean fmt/clippy,
three validated profiles, O1 + B3 smoke runs audited OK; see the
M1-GATE tracker ledger entry. Reference platform for bitwise replay:
linux/x86_64. All prior results unchanged.

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

## Next task: M2-01

**Deliver:** a pure conditional score function `S[j,i] =
alpha_h[j] * r_old[i] * xi[j] / sigma[j]` — independently callable by
tests and reusable by the later trace implementation. Do not build
plasticity, eligibility decay, or learning updates yet.

Read spec Sections 7.1–7.2 and 7.6 alongside the
[M2 task queue](../to-do.md#m2---verify-the-stochastic-score-independently).
Inspect `src/agent/{actor,weights}.rs` (receiving-`xi` buffer, `alpha_h`
helper, noise-scale contract) and `src/agent/health.rs` (read-only
observation precedent) first. The score reads the receiving neuron's
perturbation on all its incoming edges, carries exactly one `alpha_h`
factor, divides by the actual positive `sigma`, and never multiplies a
tanh derivative or `1 - lambda_e`.

- Keep the actor transition untouched; the score is a pure function
  beside it (later M2 tasks test it against finite differences and
  closed-form derivatives).
- Verify with the stated checks, then run the applicable full quality
  checks and append tracker evidence.

M2 validates score arithmetic under restricted diagnostics only — it
says nothing about continual-learning performance. Do not start M3
plasticity on the strength of implemented code alone.

## Implementation map

| Existing component | Entry points | Coverage to preserve |
| --- | --- | --- |
| Config schema and execution guards | `src/config.rs`, `configs/` | `tests/config_validation.rs`; schema validation is distinct from execution |
| Seed derivation and stream names | `src/rng.rs` | `tests/seed_streams.rs`; existing golden seeds and environment schedules |
| Environment phases and commitments | `src/environment/{mod,schedule}.rs` | `tests/environment_contract.rs`, `tests/event_order.rs` |
| Public inputs versus hidden truth | `src/environment/{observation,hidden_state}.rs` | `tests/leakage.rs`; only ordinary data reaches `Agent` |
| B0/B1/B3 and isolated O1 runner | `src/experiments/baseline.rs` | `tests/baselines.rs`, `tests/no_learning.rs`, `tests/randomized_env.rs` |
| Run ownership and provenance | `src/run.rs` | atomic allocation unit tests, `tests/event_logging.rs` |
| Inherited topology (M1-01) | `src/agent/topology.rs` | `tests/topology.rs`; mask/motor/order fixtures, rejection logging |
| Inherited weights (M1-02) | `src/agent/weights.rs` | `tests/weights.rs`; row-scale/B/bias fixtures, golden draw sequence |
| Actor transition (M1-03) | `src/agent/actor.rs` | `tests/actor.rs`; orientation/simultaneity/leak/no-clip fixtures |
| Perturbation schedule (M1-04) | `src/agent/actor.rs` + `weights.rs` | `tests/actor_noise.rs`; per-tick draws, moments, resume primitive |
| Adaptation at strength 0 (M1-05) | `src/agent/actor.rs` (verified) | `tests/adaptation.rs`; inertness, sign, persistence, config pin |
| Motor readout (M1-06) | `src/agent/motor.rs` | `tests/motor.rs`; golden filters, new-q commitment, tie-only draws |
| Nonplastic actor (M1-07, B3) | `src/agent/no_learning.rs` | `tests/no_learning.rs`; continuity, W0 invariance, schedule parity, guard separation |
| Numerical health (M1-08) | `src/agent/health.rs` | `tests/health.rs`; watchdog, summary, stable traces, file round-trip |
| Lifetime checkpoint (M1-09) | `src/checkpoint.rs` (+ snapshots in `rng`/`environment`/`agent`) | `tests/checkpoint.rs`; 3-boundary bitwise resume, rejection paths |
| Replay consolidation (M1-10) | — (no production change) | `tests/replay.rs`; platform record, reference/split replay, commitment, continuity |
| Observability smoke (M1-11) | — (no production change) | `tests/observability.rs`; cue response, bivalence, long-run finiteness |
| B3 demo command (M1-12) | `src/run.rs` (`BaselineSel::Actor`), `src/main.rs`, `configs/actor_no_learning.toml` | run unit tests, `tests/config_validation.rs`, Python B3 audit test; README demo |
| Ordinary/hidden event serialization | `src/logging/events.rs` | `tests/event_logging.rs`, `analysis/test_validate_logs.py` |
| CLI dispatch | `src/main.rs` | `validate-config` and baseline-only `simulate` (actor CLI waits for M1-12) |

## Carry-forward integration constraints

| Topic | Current implementation | Next responsibility |
| --- | --- | --- |
| Tick API | `Lifetime::advance` builds a tick and advances the environment before returning. `commit` follows a final response output. `run_actor_ordinary` preserves this loop with the actor advanced every tick. | M1-09 introduces checkpoint splits; preserve tick-20/delay-3 and final-feedback-transition fixtures. The earlier observe/finish-split note is satisfied by the ordered `apply_feedback`-before-`advance` runner, not a new `Lifetime` API. |
| Agent feedback | Ordinary runners call `apply_feedback` once before `advance(features)`; B3 dedups without learning; selection reads policy state only. | Keep this ordering and information boundary for plasticity (M3) and gates (M6). `TickOutput` belongs to the driver/evaluator. |
| Warmup | Replaces the first quiet interval; zero starts directly at cue presentation. | Preserve the documented M0 convention; use measured ticks for budgets. An additive-warmup change needs an explicit decision and new evidence. |
| Noise/hazard assignment | Stable membership shuffled at birth; noise rates cycle by cue index. | M5-02 owns factorial counterbalancing; current assignment is not a completed training-distribution implementation. |
| Event identity | IDs/choice indices restart per lifetime. Ordinary records carry lifetime identity; hidden rows align within contiguous lifetime blocks. | Checkpoint resume preserves exactly-once delivery (pending plus consumed/confirmed ledgers round-trip); M1-10 records the tolerance policy. A bare event ID is not a cross-lifetime join key. |
| Logging | `event_log=false` omits event streams; audit coverage then stops at provenance/completion. Actor/health reads draw nothing (M1-07/M1-08 logging invariance). | Extend Rust validation, Python audit, fixtures, and versioning together when adding quantities (health JSON uses `HEALTH_SCHEMA_VERSION = 1`; checkpoint schema arrives in M1-09). |
| Execution guards | M0 still rejects neural/search sections for baselines; M1-07 adds `validate_actor_no_learning_execution` (actor required, learning disabled/absent, modulator absent/fixed, evolution disabled/absent). | Enable plasticity (M3), gates (M6), and search (M7) with their implementations/tests; never bypass guards to make a future config appear runnable. |
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
