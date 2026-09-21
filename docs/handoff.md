# Agent continuation guide

Updated 2026-09-21 UTC after M3-08 all-recurrent comparison at base `0d20d45`.
The worktree was clean at session start; check Git and the tracker
for newer work before claiming.

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

**M0-GATE, M1-GATE, M2-GATE passed (re-verified where noted). M3-01
through M3-GATE verified 2026-09-21 UTC — M3 COMPLETE. Next task: M4-01.**
There is no outstanding milestone blocker. The claim track remains `family_only`.
No reserved final-test outcomes have been inspected.

M3-07 executes the frozen grid in release and passes: 216/216 lifetimes,
winner grid index 11 (eta 0.001, input 0.2, gain 0.8, sigma 0.05) with
outer-2 B4 0.035 early to 0.860 late against 0.040 controls and outer-3 at
0.935 against 0.770/0.765; points 17/19 also pass 2/3 seeds with
guardrails green sweep-wide. Birth-locked outer-1 actors documented as a
family bound. Full checks: **274 fast Rust tests passed** (2 unit + 1
analysis), three default ignores, two compile-fail doc checks, 17 Python
audit tests, clean fmt/Clippy. Archived aggregates plus verdict:
[M3-07 evidence](evidence/m3-07/summary.md). Selected config for M3-08:
grid index 11.

M3-06 freezes the pre-results sweep (`manifests/m3_development_grid.json`,
`experiments::grid`): 24 eta/input-scale/gain/sigma combinations on the
episodic base, development outers 1–3, 2,000-outcome lifetimes with
first-200/final-200 windows, matched B3/B4/B4-shuffled per point, declared
margins with guardrails, deterministic selection, and a 216-lifetime
budget. Full checks: **271 fast Rust tests passed** (6 new), three default
ignores, two compile-fail doc checks, 17 Python audit tests, clean
fmt/Clippy. Plan only — nothing executed, no outcome observed. Commands,
hashes, and limits: [M3-06 evidence](evidence/m3-06/summary.md).

M3-05 adds the matched control family (`run_episodic_no_learning`,
`run_episodic_shuffled`, `run_episodic_conditions`): B3/B4/shuffled share
one profile, schedule, inheritance, and streams, differing only in declared
mechanism flags; the shuffle protocol is an independent fair coin on a
dedicated public-seed stream with observed/applied rewards recorded
separately. Full checks: **265 fast Rust tests passed** (5 new),
three default ignores, two compile-fail doc checks, 17 Python audit tests,
clean fmt/Clippy. Machinery only — no grid, criterion, or acquisition
claim. Commands, hashes, and limits:
[M3-05 evidence](evidence/m3-05/summary.md).

M3-04 adds the explicitly episodic clean-learning runner
(`experiments::episodic`, `configs/episodic_stationary.toml`): fixed-gate
learner with agent-only construction, one coupled eligibility update per
transition, pre-transition terminal updates, `no_decay_diagnostic` traces,
and logged per-rollout resets. Full checks: **260 fast Rust tests passed**
(10 new integration + 1 unit guard), three default ignores, two
compile-fail doc checks, 17 Python audit tests, clean fmt/Clippy.
`simulate` still rejects the profile on both rungs. No acquisition or
continuous claim. Commands, hashes, and limits:
[M3-04 evidence](evidence/m3-04/summary.md).

M3-PREFLIGHT separates hidden stable/volatile membership onto the dedicated
`cue_membership` stream while preserving actor `init` and every other stream
identity. It replaces positional feedback hyperparameters with named
`FeedbackUpdateParams`, stores `plastic_bound` as a lifetime invariant, and
bumps the not-yet-embedded `PlasticSnapshot` to schema 3. Restore requires the
resolved bound and rejects mismatches/out-of-bound `P`. Full checks: **249
fast Rust tests passed**, three default ignores, two compile-fail doc checks,
17 Python audit tests, clean fmt/Clippy, and a fresh audited release smoke.
M3-03 golden values are unchanged. The hidden role assignment
for mixed stable/volatile births intentionally migrates; historical artifacts
remain tied to their revisions. Commands, hashes, and limits:
[M3 preflight evidence](evidence/m3-preflight/summary.md).

M3-02 extends `agent::plasticity::PlasticState` with
`apply_feedback_once`: old-`E`/gate/`P`/baseline read, `delta` from the old
baseline, ordered `max_update`/`plastic_bound` clamps with separated
raw/limited/actual reports, one baseline update after `delta`, monotonic
dedup, and the single cache refresh. `E` is never reset; `W0` never
mutates. M3-02 introduced `PlasticSnapshot` schema 2 with baseline plus required
`last_feedback`. This is unit-level arithmetic: no runner, gate heads, or
acquisition is wired. Full checks at M3-02 time: **243 fast Rust tests
passed** (11 new), three default ignores, two compile-fail doc checks,
clean fmt/Clippy. Commands, hashes, and claim limits:
[M3-02 evidence](evidence/m3-02/summary.md).

M3-03 adds fixture-only `tests/golden_updates.rs`: the Section 17.3 chain
(`0.4` / `0.67` / `0.4` / `0.00067` / `0.10067` / `0.64`) through the
public entry points at `1e-12`–`1e-15`, plus separate clipped cases. No
production code changes. Full checks: **247 fast Rust tests passed** (4
new), three default ignores, two compile-fail doc checks, clean
fmt/Clippy. Commands, hashes, and claim limits:
[M3-03 evidence](evidence/m3-03/summary.md).

M3-01 adds `agent::plasticity::PlasticState`: `P`/`E` stored separately from
immutable `W0`, `all_recurrent_edges` and `motor_afferent_only` masks,
`persistent` versus `no_decay_diagnostic` trace policies, and
`refresh_effective` as the single writer of the `W0 + P` cache. The actor
gains `step_with_effective_weights` and `step_with_effective_and_perturbations`
that share the `W0` arithmetic core, so the no-learning path is bitwise
unchanged (`P = 0` reproduces `step`). `PlasticSnapshot` validation is now
schema 3 (M3-PREFLIGHT); the M3-01/M3-02 records describe schemas 1/2 as
recorded then.
Commands, hashes, and claim limits: [M3-01 evidence](evidence/m3-01/summary.md).

M2-01 adds `agent::score::conditional_score`, a pure scalar function with
explicit receiver leak/noise parameters and old sender activity. It rejects
invalid inputs and nonfinite results; callers own edge selection and indexing.
M2-01 recorded eight score tests and **193 Rust tests passed overall**, one
existing ignored weight-printing probe, clean fmt/Clippy. No actor transition,
RNG, config, checkpoint or event schema changed. Python and CLI smoke runs
were not repeated for that isolated arithmetic addition. M2-GATE was then open.

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

M2-05 adds `tests/score_recurrent.rs`: a paired six-tick, two-neuron
finite-difference diagnostic at epsilon 0.04/0.02/0.01. First bounded run:
2,000,000 independent trajectory groups, 14,000,000 fixed-weight rollouts,
84,000,000 transitions, no failures. Score 0.41802543 (SE 0.00069159);
finite differences 0.41825625, 0.42040000, 0.41765000. All prespecified
agreement/precision criteria pass. Development/root 1/outer 205/lifetime 0/
actor_noise; existing actor RNG draws paired across weight settings.
Five new fast checks; fresh full suite **212 fast tests passed**, three
default ignores, two compile-fail docs, clean fmt/Clippy. M2-03's slow test
was not rerun. [M2-05 evidence](evidence/m2-05/summary.md) records the fixed
plan, immutable result, source hashes and exact checks. Production unchanged.

M2-06 adds `scripts/run_score_diagnostics.sh`: one explicit bounded command
with fresh output, exact commands/status/logs/source hashes and JSON exports.
M2-GATE passed after fresh 27 fast diagnostic tests, 2 compile-fail API tests,
and both million/two-million-sample Monte Carlo checks. Results exactly
match their original samples/settings/seeds. Full suite: **212 passed, 3
default ignores**, clean fmt/Clippy; both Monte Carlo ignores explicitly
passed, existing printing probe unrun. Four wrapper controls verify failure,
reuse and zero-test rejection plus usage. [Gate evidence](evidence/m2-06/summary.md)
records all checks, artifacts and claim limits. No production behavior changed.

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

## Next task: M4-01

**Deliver:** the authoritative main tick order (spec Section 9): apply
any due feedback using pre-tick `E`/gates/baseline before advancing
the actor; then advance actor, optional modulator, eligibility, motor
filter, and future gates; commit from the new motor output and finish
the tick. Until M6, the gate is fixed at 1.

Read spec Sections 5, 8, 9, 10, 17 plus the
[M4 task queue](../to-do.md#m4---remove-artificial-trial-resets)
before touching the runner. Section 9 owns scientific tick ordering;
the current `Lifetime::advance` + episodic apply-before-advance
dispatch is explicitly not the literal finish-last API (see carry-
forward constraints below). M4-01 owns the observe/apply/advance/
commit/log/finish split and the exact learning-sensitive tick-20/
delay-3 fixture. There is exactly one feedback-application path —
keep it that way.

- The M3 family (full-recurrent episodic learner at grid-index-11
  hyperparameters) is the starting point; M4 removes its diagnostic
  resets (`episodic_diagnostic` → `birth_only`,
  `no_decay_diagnostic` → `persistent`) step by step with the three
  continuity conditions kept honestly distinct (M4-04).

M3-GATE passed 2026-09-21 UTC on executed evidence, not stored
claims: fresh release re-runs reproduce the archived M3-07 verdict
(winner index 11, all 72 seed records identical) and the M3-08
verdict (passes 2/3) exactly; full battery green (**289 fast Rust
tests**, two compile-fail doc checks, 17 Python audit tests, fixture
audit OK, clean fmt/Clippy). Exit conditions (a)–(d) in the
[tracker ledger](../to-do.md#completion-evidence-ledger---append-do-not-fabricate).
M3 is complete: clean episodic acquisition with exact learning
replay. Continuous acquisition is explicitly unverified — that is
M4's gate. Claim track stays `family_only`.

M3-10 adds learned-offset checkpoints (`LearningCheckpoint`,
schema 3; M1 schema 2 untouched, neither loader reads the other) and
exact replay through learning events (`tests/episodic_checkpoint.rs`):
a faithful manual driver proven bit-identical to the verified runner,
then splits at the rollout boundary (nonzero `P`), just before
feedback (full `E`, deterministic re-delivery asserted), and
post-feedback pre-reset (fresh `P` plus post-scores, no double-apply)
— all resuming identically with explicit rejections. Archives are
pinned by tests: winner working config, verbatim outer-1 failure
case, real boundary/final checkpoint files (final reloads with `P`
L1 4.6972), and first/last update digests cross-matching the M3-08
record exactly. Full checks: **289 fast Rust tests passed** (9 new),
six ignores, two compile-fail doc checks, 17 Python audit tests,
clean fmt/Clippy. [M3-10 evidence](evidence/m3-10/summary.md)
distinguishes arithmetic, episodic acquisition, and still-unproven
continuous acquisition.

M3-09 adds failure-isolation tooling (`experiments::reduction`,
`tests/m3_reduction.rs`): the troubleshooting path was not needed to
rescue acquisition, but all five instruments pass — hand-set
single-edge sign/order, closed-loop preferred drift (0.400 to 0.700
with per-outcome trace-identity proof), outer-1 representation
separation, permutation-argument rejection, and permutation
sensitivity with a bitwise identity control. The diagnostic-only
learner hook provably cannot affect ordinary runners. Full checks:
**280 fast Rust tests passed** (5 new), five ignores, two
compile-fail doc checks, 17 Python audit tests, clean fmt/Clippy.
[M3-09 evidence](evidence/m3-09/summary.md) records the ladder,
commands, and findings.

M3-08 extends that learner to `all_recurrent_edges` at the frozen
winner point (grid index 11: eta 0.001, input 0.2, gain 0.8, sigma
0.05) with no production change and the M3-06 manifest untouched. The
bounded release comparison (1 point x outers 1–3 x B3/B4/B4-shuffled,
2,000-outcome lifetimes, ~0.7 s, 9/9 complete, 305,964 measured ticks)
passes 2/3 seeds under the same criterion: outer-2 B4 0.045 early to
0.975 late against 0.040/0.030 controls, outer-3 0.790 to 0.955
against 0.770/0.775; birth-locked outer-1 scores 0.0 under both masks.
Guardrails green (max clipping 0.0007, bound occupancy 0.0, non-motor
`P` movement proven, zero failures). Full checks: **275 fast Rust
tests passed** (1 new), five ignores (both Monte Carlo, weight probe,
both M3 sweeps), two compile-fail doc checks, 17 Python audit tests,
clean fmt/Clippy. Archived records plus verdict:
[M3-08 evidence](evidence/m3-08/summary.md). The M3-07 motor-only run
is preserved as a diagnostic. Actor family for M4: the full-recurrent
episodic learner at grid index 11.

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
| Score diagnostic package (M2-06) | `scripts/run_score_diagnostics.sh` | Explicit bounded fast/API/Monte Carlo sequence, fresh evidence and failure/reuse/zero-test checks |
| Recurrent finite-difference diagnostic (M2-05) | `tests/score_recurrent.rs` | Five fast contract/statistics checks plus ignored bounded paired Monte Carlo with immutable evidence export |
| Finite-rollout diagnostic (M2-04) | `src/experiments/finite_rollout.rs` | `tests/finite_rollout.rs` plus compile-fail docs; fixed parameters/baseline, no decay, terminal-only update and explicit reset |
| Plastic offsets/eligibility/masks (M3-01) | `src/agent/plasticity.rs` | `tests/plasticity.rs`; `P`/`E` separate from `W0`, both masks, `persistent` vs `no_decay_diagnostic`, single `refresh_effective` writer, versioned `PlasticSnapshot` validation (now schema 3 with bound compatibility) |
| Exactly-once feedback/baseline (M3-02) | `src/agent/plasticity.rs` (`apply_feedback_once`, `FeedbackUpdateParams`, `FeedbackOutcome`) | `tests/feedback_updates.rs`; old-baseline `delta`, ordered clamps with separated reports, monotonic dedup, zero-change cases, `W0` invariance, named update parameters |
| Golden update fixture (M3-03) | `tests/golden_updates.rs` (fixture only) | spec 17.3 chain at `1e-12`–`1e-15` plus separate clipped cases; no production change |
| Episodic diagnostic runner (M3-04) | `src/experiments/episodic.rs`, `configs/episodic_stationary.toml` | `tests/episodic_runner.rs`; agent-only fixed-gate learner, `no_decay_diagnostic` traces, one terminal update per one-choice rollout, logged resets, guard separation, ordering/RNG regression |
| Matched controls (M3-05) | `src/experiments/episodic.rs` (`run_episodic_no_learning`, `run_episodic_shuffled`, `run_episodic_conditions`), `src/agent/no_learning.rs` (diagnostic reset) | `tests/episodic_controls.rs`; shared W0/schedule/resets, first-action parity, P-movement plus behavior, re-derived shuffle protocol, observed/applied separation |
| Development grid (M3-06) | `manifests/m3_development_grid.json`, `src/experiments/grid.rs` | `tests/development_grid.rs`; frozen axes/seeds/windows/criterion/budget, validation-only instantiation of all 24 points, derived tick estimate, invalid-mutation rejection |
| Acquisition sweep (M3-07) | `src/experiments/sweep.rs`, `tests/m3_acquisition.rs` | fast analysis on real summaries plus ignored release sweep; windows/margins/health/judging/selection, 216/216 integrity, archived records plus verdict |
| Full-recurrent comparison (M3-08) | `tests/m3_full_recurrent.rs` (no production change) | fast full-mask analysis (mask superset, non-motor P, W0/effective/bounds) plus ignored release comparison at frozen index 11 (9/9 integrity, archived records plus verdict); motor-only run preserved |
| Failure isolation (M3-09) | `src/experiments/reduction.rs`, `tests/m3_reduction.rs` + diagnostic hook in `src/experiments/episodic.rs` | single-motor sign/order/drift, outer-1 representation separation, permutation sensitivity + bitwise identity control; hook unreachable from ordinary runners |
| Learning checkpoints (M3-10) | `src/checkpoint.rs` (`LearningCheckpoint` schema 3), learner snapshot/restore in `src/experiments/episodic.rs`, `tests/episodic_checkpoint.rs`, `docs/evidence/m3-10/` | faithful driver + 3 split replays with nonzero P/E, explicit rejections, pinned archives (working config, failure case, real checkpoint files, update digests); M1 schema 2 untouched |
| Effective-weight actor path (M3-01) | `src/agent/actor.rs` (`step_with_effective_weights`, `step_with_effective_and_perturbations`) | `tests/plasticity.rs`; shared `W0` core, `P = 0` bitwise parity, missing/nonfinite rejection, `B`/bias read from inherited parameters |
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
| Tick API | `Lifetime::advance` builds a tick and advances the environment before returning. The ordinary and episodic runners dispatch feedback before the agent transition, but this is not the literal Section 9 finish-last API. | Preserve complete-tick checkpoint splits and current causal behavior. M4-01 owns the observe/apply/advance/commit/log/finish split and exact learning-sensitive tick-20/delay-3 fixture. |
| Agent feedback | Ordinary runners call `apply_feedback` once before `advance(features)`; B3 dedups without learning; selection reads policy state only. The M3-04 episodic runner wires `apply_feedback_once` (fixed gate 1) around the same ordering with one coupled eligibility update per transition and logged resets. | M4-01 fixes the authoritative apply-before-advance runner order for the continuous condition. Keep the information boundary for gates (M6). `TickOutput` belongs to the driver/evaluator. |
| Effective weights | `PlasticState::refresh_effective` is the single `W0 + P` cache writer. `step_with_effective_weights` reads it; the episodic runner is its first production caller and `P` changes only through `apply_feedback_once` or validated restore. | M3-10/M4-06 embed `PlasticSnapshot` (schema 3) in checkpoint schema and prove split replay with nonzero `P`/`E`/baseline/dedup/bound. |
| Warmup | Replaces the first quiet interval; zero starts directly at cue presentation. `episodic_stationary` uses warmup 0. | Preserve the documented M0 convention; use measured ticks for budgets. An additive-warmup change needs an explicit decision and new evidence. |
| Noise/hazard assignment | Stable membership is shuffled from dedicated `cue_membership`; noise rates cycle by cue index. | M5-02 owns factorial counterbalancing; current assignment is not a completed training-distribution implementation. |
| Agent construction boundary | Existing B3 construction still receives broad config/master seed coordinates, although it uses only actor-safe values. The M3-04 learner uses agent-only inputs (`Actor`/`Learning`, inherited params, cue count, dedicated RNGs). | Keep the narrow episodic constructor; do not regress it to broad-config construction in M3-05 controls. |
| Event identity | IDs/choice indices restart per lifetime. Ordinary records carry lifetime identity; hidden rows align within contiguous lifetime blocks. Episodic rollouts reuse the same per-lifetime IDs with an added `rollout_index` (one choice per rollout). | Checkpoint resume preserves exactly-once delivery (pending plus consumed/confirmed ledgers round-trip); M1-10 records the tolerance policy. A bare event ID is not a cross-lifetime join key. |
| Logging | `event_log=false` omits event streams; audit coverage then stops at provenance/completion. Actor/health reads draw nothing (M1-07/M1-08 logging invariance). Episodic summaries carry mode/policies/resets plus consumed raw/limited/actual reports. | Extend Rust validation, Python audit, fixtures, and versioning together when adding quantities (health and checkpoint schemas are 2; ordinary events remain 1). |
| Execution guards | M0 still rejects neural/search sections for baselines; M1-07 adds `validate_actor_no_learning_execution` (actor required, learning disabled/absent, modulator absent/fixed, evolution disabled/absent). M3-04 adds `validate_episodic_execution` (clean task, `episodic_diagnostic` + `no_decay_diagnostic`, enabled learning, fixed gates). | Enable remaining plasticity controls (M3-05), gates (M6), and search (M7) with their implementations/tests; never bypass guards to make a future config appear runnable. |
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
