# AGENTS.md

## Project purpose and source of truth

This repository implements **Learning When to Learn**, specified in [`spec.md`](spec.md), version 0.1. The research question is whether internally generated learning gates improve a continuously running recurrent agent's adaptation to real changes without unnecessarily damaging stable associations under misleading feedback.

Read this as durable project guidance for coding agents. Current milestone status, ownership, and next eligible work live in `to-do.md`. Inspect the repository and evidence before assuming a feature exists or a task is unimplemented.

- **`spec.md`** defines the scientific model, information boundaries, equations, experimental controls, and claim limits. Section 9 is authoritative for tick ordering. Required contracts, proposed defaults, and hypotheses are different things.
- **`to-do.md`** defines the ordered work queue, milestone gates, ownership, and completion evidence. Keep it current as work is verified.
- **`AGENTS.md`** defines execution conventions. Repository workflow details introduced here, such as evidence records and quality commands, are engineering choices rather than new scientific requirements.
- **[`docs/handoff.md`](docs/handoff.md)** is the current continuation guide: implementation map, next-task entry points, and integration limits. Update it when those facts change; it does not override the spec or tracker.
- **[`README.md`](README.md)** owns runnable command examples. **[`docs/decisions.md`](docs/decisions.md)** and **[`docs/experiments.md`](docs/experiments.md)** preserve dated decisions and results. Read later corrections before applying an older entry.

Do not silently revise a scientific contract to make code or a result look successful. Record an ambiguity or proposed deviation in `docs/decisions.md`, including the affected spec sections, alternatives, and consequences. An unresolved scientific conflict blocks dependent implementation; continue independent work or report the blocker. Do not overwrite `spec.md` without an explicit request to change it.

## Start every work session

1. Read this file, the status/ownership block in `to-do.md`, and `docs/handoff.md`. Inspect `git status --short` and recent commits before claiming work. A handoff revision is a snapshot; check for newer changes.
2. Read the spec sections linked by the next eligible task. Before main-agent work, read Sections 1-10. Revisit Sections 5, 8, 9, and 17 whenever changing simulation, feedback, or learning behavior.
3. Inspect the relevant implementation, tests, configuration, and recent decision/experiment records. A checked box without usable evidence is not proof that a gate passes.
4. Claim one small task or an explicitly bounded related group. Preserve other agents' and the user's work. Do not perform unrelated refactors, destructive Git operations, or broad dependency upgrades.
5. State the task ID, intended change, and verification scope. Work from the smallest relevant test to the milestone smoke run.

If no task has been completed, start at **M0-01**. Otherwise use the tracker's next eligible task. A user-requested review or documentation task can be claimed separately without advancing implementation milestones. Do not jump to evolution, dashboards, or the nominal main search ahead of the milestone gates.

## Task selection, completion, and handoff

Within a milestone, follow numerical task order unless the tracker identifies work as independent and its dependencies are complete. Later milestones require the preceding `M*-GATE`. Optional/conditional work is not automatically unlocked when the core queue is complete.

Use `[ ]` for incomplete work and `[x]` only for verified work. Put `in progress`, `blocked`, or `deferred` in the status/ownership record; do not invent checkbox states. Do not check a milestone gate merely because its code compiles.

Before checking a task, implement its behavior, add or update the relevant tests, run its stated checks, and record evidence in the completion ledger in `to-do.md`. Include task IDs, commands, outcomes, and artifact paths; include configuration/seed/code identifiers for simulations. If a check cannot run, record the exact reason and leave the affected task unchecked.

Empirical gates need measured evidence. In particular, M3 and M4 require demonstrated learning, not just correct-looking equations. A null modulation result can be a valid research outcome; a broken environment or a learner that has not met its prerequisite is not a completed comparison. Never weaken assertions, change seeds repeatedly, or add hidden resets to manufacture a pass.

At handoff, update the current milestone, claimed task, last verified task, blockers, and next eligible task. Summarize changed files, commands actually run, results, deviations, and remaining risks. Do not claim tests or runs that were not executed.

Keep current summaries in `README.md` and `docs/handoff.md` aligned with the tracker. Preserve historical evidence, raw runs, and dated review reports; append corrections and link them rather than rewriting past results. Distinguish checks run this session from prior recorded checks. For documentation-only work, check links, commands, and factual consistency; do not claim a fresh simulator test pass from old evidence.

### Multiple agents

Use the ownership table in `to-do.md` to record owner, task IDs, and files before parallel work. Agree on shared interfaces first. One integrator owns shared API changes and the tracker merge. Avoid concurrent edits to the same files. Shared-file changes require coordination; all integration tests must pass after merging, even if individual branches passed. Do not use worker completion order to define scientific ordering.

Parallel sessions drift: re-read the status/ownership block and `git status` at the start of every session, since another agent may have committed, claimed tasks, or left uncommitted work. The evidence ledger in `to-do.md` is append-only — add entries, never rewrite or delete another agent's. Coordinate through the integrator before touching shared interfaces (tick ordering, RNG policy, checkpoint schema, event schemas).

## Language and architecture decisions

### Library documentation lookup

For library-, framework-, SDK-, API-, CLI-, or cloud-specific questions,
fetch current documentation with Context7. Start with `resolve-library-id`
using the library name and lookup purpose unless an exact `/org/project`
ID is supplied. Choose the relevant authoritative match, then use
`query-docs` scoped to one concept per query. Prefer Context7 over web search
for library documentation. This requirement does not apply to ordinary
code review, refactoring, business-logic debugging, or general programming
concepts. Report missing documentation access if it prevents verification.

### Implementation responsibilities

**Rust is the authoritative simulator and runner.** Implement environment scheduling, actor/modulator transitions, eligibility, learning, motor commitment, baselines, evolution, replay, checkpoints, and intervention branching in Rust. Use `f64` for the first reference implementation.

**Python is the offline analysis layer.** Use it for log audits, aggregation, uncertainty estimates, and figures. Start with ordinary scripts; NumPy and Matplotlib are suitable analysis dependencies. Add a dataframe library only when it simplifies actual analysis. A notebook may explore results but must not be the only executable record.

Use TOML for human-edited configuration and JSON/JSONL for manifests and initial portable output streams. Treat these as file formats, not additional implementation languages. Keep the first language boundary file-based: Rust produces versioned data; Python reads it. Do not create a second production simulator in Python or introduce FFI before a demonstrated need. Tiny independent numerical fixtures are fine.

Pin the Rust toolchain, Python interpreter version, and dependency resolutions when bootstrapping. Keep `Cargo.lock`. The Python layer is currently standard-library-only (see `analysis/requirements.txt`; interpreter pinned in `.python-version`); introduce a pinned Python dependency lock together with the first third-party analysis dependency. Record actual versions; do not invent version numbers or assume that a floating `stable` channel is a reproducibility pin. Use ordinary configuration/serialization/RNG libraries. A deep-learning framework, autograd engine, GPU backend, and browser frontend are not needed for the initial model.

Maintain four responsibilities:

| Component | Owns | Must not do |
| --- | --- | --- |
| Environment | Hidden mappings, schedules, reward delivery, public observations | Expose hidden truth to ordinary agents |
| Agent | Dynamics, local traces, plastic offsets, gates, motor output | Access evaluation labels or future schedules |
| Search | Genomes, complete-lifetime fitness, selection | Carry acquired lifetime state into offspring |
| Evaluator | Hidden-truth metrics and explicit interventions | Feed privileged information into a normal running agent |

Keep the CLI thin and the simulation code testable without spawning a process. Prefer one crate with a library and binary initially. Follow the target layout in spec Section 18; add modules only when their milestone needs them. The README's layout section describes the modules as actually implemented. `docs/decisions.md`, `docs/experiments.md`, and the tracker evidence ledger are workflow additions.

## Non-negotiable scientific contracts

### Information boundary and environment

The ordinary agent sees only `Observation.features`, permitted observed reward feedback, and its own state. Feedback event IDs are infrastructure for deduplication, never neural features. Keep hidden annotations in a separate type/module and output stream.

For `K` one-hot cues, there are `K + 6` features: cue content, cue-present, go, outcome-present, outcome-value, and two previous-action channels. A zero reward is distinguishable from no outcome. The committed action becomes visible through the previous-action latch starting on the next tick.

Do not expose target action, latent correctness, noise bit/rate, hazard, stable/volatile membership, hidden change flags, future cues, lifetime countdown, split identity, or task-identifying hidden event metadata. Privileged oracles and evaluator interventions must be explicitly isolated and labeled.

Hazard is per exposure of that cue, applied before repeat presentations, never at its first presentation. Reward uses the mapping saved at commitment and one sampled noise bit. Agent actions must not alter the exogenous cue/change/noise/timing schedule in the first environment. Use one unresolved choice at a time.

### Exact tick order

For the primary model, preserve spec Section 9:

1. Obtain current observable input and any due feedback. Route hidden annotations only to the evaluator/logger.
2. At feedback arrival, check event identity; read old eligibility, old gates, old offsets, and old baseline; apply one bounded update; update the baseline once; mark the event consumed. Do not reset state.
3. Snapshot old actor, adaptation, modulator, and motor arrays.
4. Advance the actor using old activity and the effective weights after the feedback update. Draw one independent Gaussian perturbation per actor neuron.
5. Advance the modulator using old actor/modulator activity and current ordinary input.
6. Advance live eligibility from old presynaptic activity and this transition's receiving-neuron perturbation. It cannot explain feedback already consumed this tick.
7. Advance motor filters using new actor activity.
8. Compute new gates for future feedback.
9. At the final response tick, commit from the new motor output. Schedule one reward and make the new previous-action latch visible next tick.
10. Log observationally and finish the tick.

`advance_neural_tick` must not apply feedback a second time. Feedback for commitment at `t` and delay `d >= 1` arrives at the **start** of `t + d`. For `d = 1`, it arrives next tick, not at commitment. The delay includes the feedback endpoint: do not add an extra feedback tick to cycle accounting. Finish a lifetime only after the final feedback tick has completed, with no unresolved action.

### Dynamics and local learning

`W[j, i]` means sender `i` to receiver `j`. Rows receive, columns send. All transition right-hand sides use old arrays; compute new arrays and swap. Noise is added after leaky integration, not multiplied by the leak coefficient.

The primary local score and trace are:

```text
alpha_h[j] = -expm1(-1 / tau_h[j])
S[j,i] = alpha_h[j] * r_old[i] * xi[j] / sigma[j]
E_new[j,i] = exp(-1 / tau_e) * E_old[j,i] + S[j,i]
```

Use the receiving neuron's `xi[j]` on all its incoming edges. Include `alpha_h` exactly once; divide by actual positive `sigma`; do not multiply by a tanh derivative or by `1 - lambda_e`. Reject zero noise when this score is active. Do not silently clip membrane states; stop and record numerical failures instead.

At feedback, using the already-existing trace and gate:

```text
delta = reward - reward_baseline_old
raw = eta * delta * gate[receiver] * eligibility_old
limited = clamp(raw, -max_update, +max_update)
P_new = clamp(P_old + limited, -plastic_bound, +plastic_bound)
reward_baseline_new = reward_baseline_old + beta_R * delta
W_effective = W0 + P_new
```

Keep `W0` immutable during a lifetime and store it separately from `P`. Apply updates only to plastic existing actor edges. Log raw, limited, and actual applied changes; bounds can make these different. Biases, sensory weights, and modulator weights are not lifetime-plastic in the first main experiment. No default weight decay, global normalization, learned readout, homeostasis, or other unreported stabilizer.

The exact conditional score is not a guarantee that the clipped, decaying, gated, changing-weight online learner is an unbiased lifetime gradient. Keep the fixed-weight, no-decay, constant-baseline finite-rollout diagnostic separate from the main algorithm.

### Gates and state persistence

Fixed mode uses gate 1. Global mode shares a sigmoid scalar. Targeted mode produces one sigmoid gate per receiving neuron, shared by its incoming plastic edges. Gates scale magnitude, not sign. Main gates are sampled **before** current feedback is integrated.

The modulator has no ordinary activation-feedback path into the actor in the initial architecture. With plasticity disabled and actor noise paired, changing modulator parameters must not change actor behavior.

Main lifetimes reset only at birth: activity, adaptation, modulation, motor filters, eligibility, offsets, baseline, and bookkeeping persist across choices and feedback. Warmup is part of the lifetime; traces evolve through it. Explicitly named diagnostic reset modes are allowed, never silently substituted for the continuous condition. Initialize all birth state as specified in Section 10, including `P = E = 0` and baseline 0.5.

Evaluation freezes evolution, not ordinary lifetime learning. A fresh evaluation lifetime starts with `P = 0`. A genome is not a resumable lifetime checkpoint.

## Determinism, logging, and failure behavior

Derive RNG streams from documented stable seed tuples. Separate cue selection, mapping changes, reward noise, timing, neural perturbations, tie-breaking, initialization, and evolution. Do not use a process-randomized language hash or a shared mutable global RNG. Pin and record the RNG algorithm, distribution implementation, stream mapping, and versions.

Draw every actor neuron's perturbation on every tick, independent of gate values and logging. Logging must not draw simulation randomness or affect behavior. Keep stable neuron/edge ordering and deterministic reductions. Parallelize independent lifetimes/candidates only after serial parity tests; bound the worker pool.

A complete checkpoint must include current agent state, derived state needed for exact continuation, environment phase/pending reward/action latch, consumed-feedback bookkeeping, RNG state/counters, genome, and resolved configuration identifiers. Include schema/version/hash/checksum information and write atomically. Reject incompatible or incomplete checkpoints. Extend checkpoint tests whenever adding state. Promise bitwise replay only on the recorded reference platform; cross-platform comparisons need declared tolerances.

Every run records the fully resolved configuration, code revision and dirty-tree status, condition, seed namespace, software/platform identity, failures, and resource counts. Keep ordinary event data separate from hidden annotations. Keep raw output immutable; write derived analysis elsewhere. Detailed traces are for selected replays, not every search candidate.

Nonfinite values, duplicate feedback, invalid configurations, corrupt checkpoints, and inconsistent counts are explicit errors. Failed candidates receive the predeclared finite worst fitness plus a failure code. Missing/interrupted results are not successful zero-score runs. Do not omit failures from reports or rerun until a favorable seed appears.

## Search, evaluation, and claim discipline

Do not enable search before M6 passes. Start with the small projection-only search. Keep the initial actor, masks, inputs, and noise process paired across B4/B5/B6. Tune the fixed-rate baseline on a declared development budget; do not compare thousands of searched gates with one arbitrary fixed learning rate.

Evaluate all candidates on the same fresh stratified training batch, with fresh births. Reevaluate preserved elites on each new batch. Use observed reward over delivered outcomes as initial fitness, including acquisition; no hidden-correctness objective or gate-shape bonus. Acquired `P` never enters offspring genomes. Selection ties and result aggregation must be deterministic.

Keep development, training, validation, and final-test seed namespaces disjoint. Select checkpoints using the declared validation metric. Lock the study plan and analysis before final-test inspection; subsequent tuning starts a new exploratory cycle. Do not inspect final-test results to choose thresholds, examples, permutations, or gate shifts.

An outer search replicate, not a tick/reward/lifetime from one genome, is the primary independent unit for evolved-model comparisons. Report paired outer-seed effects, uncertainty, all failures, tuning/search/validation budgets, and observed reward alongside latent accuracy/regret. Account for censored recovery events and overlapping reversals.

B3 (the same actor with learning disabled) is not B7 (a separately optimized activity-only recurrent agent). Without the matched expanded search and B7, restrict claims to the tested locally plastic actor family. Correlations between gates and changes do not establish mechanism. Use branch interventions and the raw-update-magnitude control; equal mean gates do not imply equal update norms.

## Verification and commands

`README.md` is the runnable-command source of truth; the contracts below
are targets to implement progressively. A missing command or environment
is a blocker to report, not a reason to pretend it passed.

Implemented and covered by tests (M0-GATE passed):

```bash
cargo run --release --locked -- validate-config configs/debug_stationary.toml
cargo run --release --locked -- simulate --config configs/env_smoke.toml --baseline oracle --lifetimes 2 --seed 1
python3 analysis/validate_logs.py runs/<run-id>
python3 analysis/test_validate_logs.py
```

Baselines for `simulate --baseline`: `random` (B0), `constant-0` /
`constant-1` (B1), `oracle` (O1, privileged reference). Still planned:
`benchmark`, `evolve`, `evaluate`, `intervene`, and `aggregate.py`.

`debug_stationary.toml` currently validates as a reference profile but cannot
be simulated by the M0 baseline runner. Use `env_smoke.toml` for executable
baselines. In the audit command, replace `runs/<run-id>` with the printed
directory; `analysis/fixtures/valid` is the committed portable alternative.

After a Rust task, use the relevant focused tests and these checks once bootstrapped:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

Keep expensive Monte Carlo and empirical experiments in explicitly invoked, bounded suites. Pin statistical-test seeds and tolerances before running. Record ignored or unavailable tests and invoke relevant slow diagnostics separately when a milestone requires them.

Search, evaluation, intervention, and Python analysis command contracts are listed in `to-do.md` and spec Section 18. The resolved seed namespace must come from a documented CLI/config/manifest source; never silently use test seeds. Run analysis tests when changing analysis code, and audit logs before aggregating them.

Add regression tests for every scientific-contract bug. Preserve deterministic golden fixtures. Never fix a mismatch by updating expected values without explaining why the old expectation was wrong.

## Compute and scope boundaries

Default to the smallest deterministic fixture, then a bounded development smoke run. A checked configuration is not permission to launch its nominal 100-generation search. Before scaling, benchmark representative lifetimes with logging on/off and record candidate-lifetimes, ticks, validation work, expected changes, measured throughput, and a finite run budget. Obtain a declared owner-approved budget for substantial search; do not launch an unbounded job.

Keep the initial tick loop single-threaded, buffers preallocated, and the dense reference readable. Optimize only after profiling and parity tests; introduce a sparse kernel without removing the reference implementation.

Do not begin 3D/embodied environments, spiking models, evolving topology, multiple pending choices, post-feedback gates, learned teaching signals, explicit-perturbation removal, or a dashboard before the relevant core gates and a separately approved scope. These are extensions, not fixes to hide a failed baseline.

The first meaningful success is a small agent that learns unknown associations from delayed outcomes and still learns without within-lifetime neural resets. Support every larger claim with its specified controls.
