# Agent continuation guide

Updated 2026-09-21 UTC after M2-04. Current HEAD is
`72ad075` (`m2-03`), with M2-04 changes uncommitted. The session began at
`9d31b9d`; the earlier M2-02/03 work was committed during this session. Check the tracker
and Git state for newer work before claiming a task.

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

**M0-GATE passed and was re-verified. M1-GATE passed. M2-01–M2-04 complete.
Next task: M2-05.**
There is no outstanding M1 blocker. The claim track remains `family_only`.
No reserved final-test outcomes have been inspected.

M2-01 adds `agent::score::conditional_score`, a pure scalar function with
explicit receiver leak/noise parameters and old sender activity. It rejects
invalid inputs and nonfinite results; callers own edge selection and indexing.
M2-01 recorded eight score tests and **193 Rust tests passed overall**, one
existing ignored weight-printing probe, clean fmt/Clippy. No actor transition,
RNG, config, checkpoint or event schema changed. Python and CLI smoke runs
were not repeated for this isolated arithmetic addition. M2-GATE is open.

M2-02 adds `tests/score_log_probability.rs`: 288 fixed-observation derivative
comparisons against the production actor's conditional mean, Gaussian-density
goldens, and an invalid moving-sample negative control. All pass under the
prespecified tolerance; largest absolute difference is 3.791e-8. M2-02 full
checks: **196 Rust tests passed**, one existing ignored probe, clean
fmt/Clippy. No production changes or new simulation runs. Commands, fixture
settings and source hashes: [M2-02 evidence](evidence/m2-02/summary.md).

M2-03 adds `tests/score_learning_direction.rs`: three fast tests for the
statistics, analytical expectation and forced samples, plus an explicitly
invoked million-sample diagnostic. It passed on first execution: positive
mean 0.1387168, analytical 0.1388622, SE 0.00010645; paired opposite target
negates the mean. Seed: development/root 1/outer 203/lifetime 0/actor_noise.
M2-03 full checks: **199 fast tests passed**, two default ignores (the
separately passed diagnostic and existing weight-printing probe), clean
fmt/Clippy. No production changes. See [M2-03 evidence](evidence/m2-03/summary.md)
for the predeclared plan, command and immutable machine-readable result.

M2-04 adds `experiments::finite_rollout::FiniteRollout`: exact score sums,
fixed weights/baseline/noise, zero initial state, bounded horizon, one terminal
result/update and explicit between-rollout resets. Its terminal weight copy
does not replace inherited parameters. Numerical failure prevents subsequent
steps, finalization or reset. Eight integration tests and two compile-fail
API checks pass; fresh full suite: **207 fast tests passed**, two default
ignores, clean fmt/Clippy. The M2-03 Monte Carlo test was not rerun this session.
Golden output, exact commands and hashes: [M2-04 evidence](evidence/m2-04/summary.md).

M1 implements a continuous nonplastic actor and exits as a verified dynamics
foundation, explicitly not learning. The [M1 corrective review](m1-review.md)
fixed checkpoint seed/config/state validation, concurrent checkpoint writes,
runtime watchdog enforcement, saved diagnostics, and supplied-parameter/pool
validation. Fresh battery: **185 Rust passes, 1 ignored weight-printing probe,
15 Python passes, clean fmt/Clippy**, three validated profiles, eleven audited
corrected runs and two audited original-commit comparisons. Healthy clean/noisy
events match `13a4873` exactly after removing run IDs. Exact commands, hashes,
raw-run paths and measured diagnostics: [review evidence](evidence/m1-review/summary.json).

Checkpoint schema is now **2** (schema 1 rejected); health schema is **2**;
ordinary events remain schema 1. Capture only after the whole tick, including
commitment. Both restored halves are validated together, and the last
perturbations survive immediately after resume. Runtime B3 runs save inherited
weights plus per-lifetime health, selected h/a/r/q traces, initialization
history and explicit failures. Both actions occur across initializations, but
individual actors can be strongly action-biased; no learning capability follows
from M1. Reference platform remains linux/x86_64.

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

## Next task: M2-05

**Deliver:** the short two-neuron recurrent finite-difference diagnostic
(17.6). Compare sampled expected-reward differences under several positive/
negative weight perturbations with terminal reward times summed local scores.
Prespecify development seeds, horizon, samples, perturbation magnitudes and
uncertainty tolerances. Report unresolved uncertainty honestly; do not accept
a very wide interval as strong evidence or reseed after seeing a failure.

Read spec Sections 7.1–7.2, 7.6, 9 and 17.6 alongside the
[M2 task queue](../to-do.md#m2---verify-the-stochastic-score-independently).
Inspect `src/experiments/finite_rollout.rs`, `tests/finite_rollout.rs`,
`src/agent/score.rs` and `src/agent/actor.rs` (receiving-`xi` buffer and
`leak_alpha` helper). The score reads the receiving neuron's
perturbation on all its incoming edges, carries exactly one `alpha_h`
factor, divides by the actual positive `sigma`, and never multiplies a
tanh derivative or `1 - lambda_e`.

- Use a fresh `FiniteRollout::new(actor, params, horizon, baseline)` per weight
  setting; baseline zero yields the terminal-reward-times-score quantity in
  17.6. `step` takes the caller's RNG; `step_with_perturbations` supports paired
  saved perturbations. `finish(reward, None)` returns the estimator without
  updating weights. `reset_between_rollouts` clears state/scores and retains
  the original parameters/baseline. Record failed samples instead of silently
  dropping them. Motor filtering/environment feedback are not in this harness.
- Keep the actor transition untouched. No M3 eligibility decay or online
  plasticity is enabled by the diagnostic.
- Verify with the stated checks, then run the applicable full quality
  checks and append tracker evidence.

The new M2-03 test contains test-local Welford statistics and optional immutable
evidence output that M2-05/06 may reuse or consolidate when needed. Do not run
Monte Carlo implicitly in the fast suite. Keep the saved M2-03 files unchanged;
fresh reruns must use a new result path.

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
| Conditional score (M2-01) | `src/agent/score.rs` | `tests/score.rs`; receiver indexing, golden arithmetic, saturation, zero activity, invalid inputs and overflow |
| Conditional derivative diagnostic (M2-02) | `tests/score_log_probability.rs` | Saved-sample finite differences across edges/leaks/noise scales; moving-sample negative control |
| One-neuron direction diagnostic (M2-03) | `tests/score_learning_direction.rs` | Fast statistics/forced-sample fixtures plus ignored bounded Monte Carlo with immutable evidence export |
| Finite-rollout diagnostic (M2-04) | `src/experiments/finite_rollout.rs` | `tests/finite_rollout.rs` plus compile-fail docs; fixed parameters/baseline, no decay, terminal-only update and explicit reset |
| Adaptation at strength 0 (M1-05) | `src/agent/actor.rs` (verified) | `tests/adaptation.rs`; inertness, sign, persistence, config pin |
| Motor readout (M1-06) | `src/agent/motor.rs` | `tests/motor.rs`; golden filters, new-q commitment, tie-only draws |
| Nonplastic actor (M1-07, B3) | `src/agent/no_learning.rs` | `tests/no_learning.rs`; continuity, W0 invariance, schedule parity, guard separation |
| Numerical health (M1-08/review) | `src/agent/health.rs`, actor transition watchdog, `src/run.rs` observation wrapper | `tests/health.rs`; watchdog, summary, stable traces, file round-trip |
| Lifetime checkpoint (M1-09) | `src/checkpoint.rs` (+ snapshots in `rng`/`environment`/`agent`) | `tests/checkpoint.rs`; 3-boundary bitwise resume, rejection paths |
| Replay consolidation (M1-10) | — (no production change) | `tests/replay.rs`; platform record, reference/split replay, commitment, continuity |
| Observability smoke (M1-11) | — (no production change) | `tests/observability.rs`; cue response, bivalence, long-run finiteness |
| B3 demo command (M1-12) | `src/run.rs` (`BaselineSel::Actor`), `src/main.rs`, `configs/actor_no_learning.toml` | run unit tests, `tests/config_validation.rs`, Python B3 audit test; README demo |
| Ordinary/hidden event serialization | `src/logging/events.rs` | `tests/event_logging.rs`, `analysis/test_validate_logs.py` |
| CLI dispatch | `src/main.rs` | `validate-config` and B0/B1/B3/O1 `simulate` |

## Carry-forward integration constraints

| Topic | Current implementation | Next responsibility |
| --- | --- | --- |
| Tick API | `Lifetime::advance` builds a tick and advances the environment before returning. `commit` follows a final response output. `run_actor_ordinary` preserves this loop with the actor advanced every tick. | Preserve complete-tick checkpoint splits, tick-20/delay-3 and final-feedback-transition fixtures. The earlier observe/finish-split note is satisfied by the ordered `apply_feedback`-before-`advance` runner, not a new `Lifetime` API. |
| Agent feedback | Ordinary runners call `apply_feedback` once before `advance(features)`; B3 dedups without learning; selection reads policy state only. | Keep this ordering and information boundary for plasticity (M3) and gates (M6). `TickOutput` belongs to the driver/evaluator. |
| Warmup | Replaces the first quiet interval; zero starts directly at cue presentation. | Preserve the documented M0 convention; use measured ticks for budgets. An additive-warmup change needs an explicit decision and new evidence. |
| Noise/hazard assignment | Stable membership shuffled at birth; noise rates cycle by cue index. | M5-02 owns factorial counterbalancing; current assignment is not a completed training-distribution implementation. |
| Event identity | IDs/choice indices restart per lifetime. Ordinary records carry lifetime identity; hidden rows align within contiguous lifetime blocks. | Checkpoint resume preserves exactly-once delivery (pending plus consumed/confirmed ledgers round-trip); M1-10 records the tolerance policy. A bare event ID is not a cross-lifetime join key. |
| Logging | `event_log=false` omits event streams; audit coverage then stops at provenance/completion. Actor/health reads draw nothing (M1-07/M1-08 logging invariance). | Extend Rust validation, Python audit, fixtures, and versioning together when adding quantities (health and checkpoint schemas are 2; ordinary events remain 1). |
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
