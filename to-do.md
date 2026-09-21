# to-do.md - Learning When to Learn

## Purpose and authority

This is the checkable implementation and research queue for [`spec.md`](spec.md), **version 0.1, prepared September 20, 2026**. Keep this file and [`AGENTS.md`](AGENTS.md) at the repository root beside `spec.md`.

The checklist follows the specification's **M0-M10 milestone sequence**. Scientific equations, information boundaries, and claim limits come from the spec; Section 9 overrides informal descriptions of tick ordering. Task IDs, evidence records, ownership conventions, extra smoke-profile filenames, and the final-study scheduling below are execution choices added for agent coordination, not changes to the scientific model.

Checked tasks below have recorded verification in the completion ledger. Unchecked tasks remain planned. Inspect implementation and evidence before changing status; writing a plan does not complete a task. Historical ledger entries describe the repository at their recorded revision.

Navigation: [current status](#current-status---maintain-at-every-handoff),
[M1 queue](#m1---build-a-continuous-actor-with-no-learning),
[milestone map](#milestone-map), [coverage index](#contract-and-test-coverage-index),
[evidence ledger](#completion-evidence-ledger---append-do-not-fabricate),
and [continuation guide](docs/handoff.md).

### Working rules

1. Read `AGENTS.md`, the current status below, and the linked spec sections. Work on the earliest unclaimed eligible task, normally in numerical order. Do not advance past an unchecked milestone gate.
2. Claim a bounded task and record ownership. Implement it, add tests, run the stated verification, and save evidence. Use `[x]` only after verification; keep blocked/in-progress/deferred work unchecked with a written status.
3. Separate **implementation completion** from **empirical milestone completion**. M3/M4 require actual learning evidence. M7/M9 can produce legitimate null findings when their implementations and controls are trustworthy.
4. Record exact commands and outputs in the evidence ledger. For experiments, include run paths, code/configuration identity, seed namespace, sample counts, failures, and interpretation. Never fabricate a test pass or rerun seeds until a statistical check passes.
5. At handoff, update the status, blockers, last verified task, and next eligible task. Keep stable IDs; add suffixed tasks if a ticket needs splitting rather than renumbering completed work.

**Final-test staging:** M8 builds/rehearses comparisons and M9 builds/rehearses interventions using development/validation data. M10 freezes the complete protocol and executes the untouched test suites. This operational staging protects the spec's train/validation/test separation; it does not change the learning rule or the required comparisons.

**Claim tracks:** the initial `family_only` track studies gates on the tested locally plastic actor family. The conditional B7 track is required before making a stronger comparison against an independently optimized activity-only agent. Leaving B7 deferred limits claims; it is not permission to mislabel B3 as B7.

## Current status - maintain at every handoff

| Field | Current value |
| --- | --- |
| Current milestone | M3 in progress; M2-GATE passed 2026-09-21 UTC; M3-03 verified 2026-09-21 UTC |
| Claim track | family_only for the first study; broader track not authorized |
| Last verified task | M3-03 — hand-calculated golden update fixture |
| Claimed task | None |
| Next eligible task | M3-04 |
| Current blocker | None |
| Final-test status | No reserved final-test results inspected |
| Last evidence record | 2026-09-21 UTC M3-03; docs/evidence/m3-03/summary.md and ledger below |

### Session ownership and handoffs

| Owner/session | Task IDs | Files or interfaces owned | Status / handoff |
| --- | --- | --- | --- |
| opencode 2026-09-21 M3-03 | M3-03 | tests/golden_updates.rs, docs/evidence/m3-03/, README.md, docs/{decisions,evidence/README,handoff}.md, to-do.md | Done; 4 new tests (fixture only) and full checks pass; next M3-04 |
| opencode 2026-09-21 M3-02 | M3-02 | src/agent/plasticity.rs, tests/feedback_updates.rs, docs/evidence/m3-02/, README.md, docs/{decisions,evidence/README,handoff}.md, to-do.md | Done; 11 new tests and full checks pass; next M3-03 |
| opencode 2026-09-21 M3-01 | M3-01 | src/agent/{plasticity,actor,mod}.rs, src/lib.rs, tests/plasticity.rs, docs/evidence/m3-01/, README.md, docs/{decisions,handoff}.md, to-do.md | Done; 16 new tests and full checks pass; next M3-02 |
| Codex 2026-09-21 M2-06 | M2-06, M2-GATE | scripts/run_score_diagnostics.sh, docs/evidence/m2-06/, README.md, docs/{experiments,handoff}.md, docs/evidence/README.md, to-do.md | Done; all diagnostics freshly passed; M2-GATE verified, next M3-01 |
| Codex 2026-09-21 M2-05 | M2-05 | tests/score_recurrent.rs, docs/evidence/m2-05/, README.md, docs/{experiments,handoff}.md, to-do.md | Done; three Monte Carlo comparisons and full checks pass; next M2-06 |
| Codex 2026-09-21 M2-04 | M2-04 | src/experiments/{mod,finite_rollout}.rs, tests/finite_rollout.rs, README.md, docs/{decisions,handoff}.md, docs/evidence/m2-04/, to-do.md | Done; 8 integration and 2 compile-fail checks pass; next M2-05 |
| Codex 2026-09-21 M2-03 | M2-03 | tests/score_learning_direction.rs, docs/evidence/m2-03/, docs/experiments.md, README.md, docs/handoff.md, to-do.md | Done; million-sample diagnostic and full fast checks pass; next M2-04 |
| Codex 2026-09-21 continuation | M2-02 | tests/score_log_probability.rs, docs/evidence/m2-02/, README.md, docs/handoff.md, to-do.md | Done; 288 derivative comparisons and full Rust checks pass; next M2-03 |
| Codex 2026-09-21 | M2-01 | src/agent/{mod,score}.rs, tests/score.rs, README.md, docs/handoff.md, to-do.md | Done; eight score tests and full Rust checks pass; next M2-02 |
| Codex review 2026-09-21 | M1-REVIEW | M1 implementation, regression tests, review evidence, README and handoff | Done; M1-GATE re-verified; next M2-01 |
| Codex documentation | M0-DOCS | Agent guidance, current handoff, README, tracker, manifest notes, source/config comments | Done; no behavior changes; handoff to M1-01 |
| Codex review | M0-REVIEW | M0 Rust/Python implementation, regression tests, evidence and documentation | Done; M0-GATE re-verified, handoff to M1-01 |
| agent 2026-09-21 | M0-01–M0-05 | src/{lib,main,config,rng,run}.rs, Cargo.toml, rust-toolchain.toml, configs/, manifests/, tests/{seed_streams,config_validation}.rs, README, docs/, analysis/ stub | Done, verified; handoff to M0-06 |
| agent 2026-09-21 | M0-06–M0-08 | src/environment/*, tests/{environment_contract,leakage}.rs, tests/support/ | Done, verified; handoff to M0-09 |
| agent 2026-09-21 | M0-09–M0-11 | src/environment/mod.rs (features), src/experiments/*, tests/{environment_contract,event_order,leakage,baselines}.rs | Done, verified; handoff to M0-12 |
| agent 2026-09-21 | M0-12–M0-GATE | src/logging/*, src/run.rs (runner), src/main.rs (CLI), analysis/*, tests/{randomized_env,event_logging}.rs | Done, verified; M0-GATE passed, handoff to M1-01 |
| agent 2026-09-21 | M1-01 | src/agent/{mod,topology}.rs, tests/topology.rs, docs/decisions.md | Done, verified; handoff to M1-02 |
| agent 2026-09-21 | M1-02 | src/agent/{mod,topology,weights}.rs, tests/weights.rs, docs/decisions.md | Done, verified; handoff to M1-03 |
| agent 2026-09-21 | M1-03 | src/agent/{mod,actor}.rs, tests/actor.rs, docs/decisions.md | Done, verified; handoff to M1-04 |
| agent 2026-09-21 | M1-04 | src/agent/actor.rs, tests/actor_noise.rs, docs/decisions.md | Done, verified; handoff to M1-05 |
| agent 2026-09-21 | M1-05 | tests/adaptation.rs | Done, verified; handoff to M1-06 |
| agent 2026-09-21 | M1-06 | src/agent/{mod,motor}.rs, tests/motor.rs, docs/decisions.md | Done, verified; handoff to M1-07 |
| agent 2026-09-21 | M1-07 | src/agent/{mod,no_learning}.rs, src/config.rs, src/experiments/baseline.rs, src/lib.rs, tests/no_learning.rs, docs/decisions.md | Done, verified; handoff to M1-08 |
| agent 2026-09-21 | M1-08 | src/agent/{mod,health,no_learning}.rs, src/lib.rs, tests/health.rs, docs/decisions.md | Done, verified; handoff to M1-09 |
| agent 2026-09-21 | M1-09 | src/checkpoint.rs, src/{lib,rng}.rs, src/agent/no_learning.rs, src/environment/{mod,hidden_state,schedule}.rs, Cargo.toml, tests/checkpoint.rs | Done, verified; handoff to M1-10 |
| agent 2026-09-21 | M1-10 | tests/replay.rs, docs/decisions.md | Done, verified; handoff to M1-11 |
| agent 2026-09-21 | M1-11 | tests/observability.rs, docs/decisions.md | Done, verified; handoff to M1-12 |
| agent 2026-09-21 | M1-12 | configs/actor_no_learning.toml, src/{main,run}.rs, src/run.rs tests, tests/config_validation.rs, analysis/{validate_logs,test_validate_logs}.py, README.md | Done, verified; handoff to M1-GATE |
| agent 2026-09-21 | M1-GATE | to-do.md, docs/{decisions,handoff}.md, README.md (evidence only; no behavior change) | Done, verified; handoff to M2-01 |

Parallel work requires settled interfaces and satisfied dependencies. Do not parallelize successive scientific milestones or let two agents independently redefine feedback ordering, RNG policy, or checkpoint schema. Coordinate changes to this tracker through one integrator.

## Milestone map

| Milestone | Objective | Required predecessor |
| --- | --- | --- |
| M0 | Freeze contracts and build the environment | None. Start here. |
| M1 | Build a continuous actor with no learning | M0-GATE |
| M2 | Verify the stochastic score independently | M1-GATE |
| M3 | Make an ungated local learner learn a clean task | M2-GATE |
| M4 | Remove artificial trial resets | M3-GATE |
| M5 | Establish the adaptation/noise trade-off | M4-GATE |
| M6 | Add gates before evolving them | M5-GATE |
| M7 | Evolve a small learning-control mechanism | M6-GATE |
| M8 | Run the primary comparisons | M7-GATE |
| M9 | Test the mechanism causally | M8-GATE |
| M10 | Package the research result | M9-GATE; additionally B7-GATE if the broader claim track is selected |

## Completion standard

Every milestone leaves **a runnable command, automated tests, and saved example output**, as required by spec Section 16. Proposed command names and filenames below are targets to implement, not existing functionality. Record actual commands in the README and evidence ledger when they work.

For an implementation task, completion means its behavior and error cases exist, focused tests pass, broader affected regression tests pass, and configuration/checkpoint/logging changes are accounted for. For an empirical task, completion also requires the declared runs and controls, honest uncertainty/failure reporting, and saved raw evidence. A result that does not support a hypothesis is still evidence; missing execution is not.

---

## M0 - Freeze contracts and build the environment

**Spec:** Sections 4-5, 9, 16/M0, 17.1, 18-20.  
**Prerequisite:** None. Completed; see current status for the next eligible task.

Prove that observations, hidden mappings, timing, and reward accounting are correct without implementing recurrent neurons or evolution.

- [x] **M0-01 - Inspect the repository and preserve the specification**
  - Deliver: Read AGENTS.md and the supplied spec.md; inspect existing files and changes. Confirm the root filename is spec.md, not an upload suffix. Record source version and the actual initial repository state.
  - Verify: Do not mark existing implementation complete without its tests. Keep the supplied scientific specification unchanged. Set the tracker status and claim the first task.

- [x] **M0-02 - Bootstrap the minimal Rust project and reproducible toolchain**
  - Deliver: Create a small Cargo package with a testable library and thin CLI binary. Pin the installed supported toolchain and dependency resolution; keep Cargo.lock. Establish formatting, linting, and test commands.
  - Verify: A minimal unit test and CLI help run successfully. Record actual toolchain versions. Do not create pretend implementations of future modules or introduce a deep-learning framework.

- [x] **M0-03 - Establish repository records and artifact conventions**
  - Deliver: Create README.md, docs/decisions.md, docs/experiments.md, configs/, manifests/, tests/, and analysis/ as needed. Define unique run directories and a policy keeping large generated data/build output out of ordinary source commits.
  - Verify: Document that decision records, task evidence, and study-plan files are workflow additions. The README distinguishes implemented commands from planned commands; experiment records are append-only, including failures.

- [x] **M0-04 - Define deterministic seed namespaces and independent streams**
  - Deliver: Document and implement a stable derivation from root seed, namespace, outer seed, lifetime index, and stream name. Reserve development, training, validation, and test namespaces; separate environment, initialization, actor noise, action ties, and evolution streams.
  - Verify: Known seed tuples produce golden stream outputs. Namespace separation is auditable. Additional draws in an agent stream do not change cue, change, noise, or timing schedules. Runtime-randomized hashes are not used.

- [x] **M0-05 - Implement versioned configuration parsing and validation**
  - Deliver: Implement a resolved TOML schema from Section 19 and an environment-only smoke profile. Record all defaults and the seed source. Reject unknown/unsupported modes at execution rather than silently ignoring them. Add validation incrementally with each feature.
  - Verify: Table-test invalid probabilities, durations, dt != 1, pending-choice limits, schema versions, and missing seed namespaces. Include dimension, score-noise, reset-policy, and evolution validation as their modules arrive. The environment smoke profile does not pretend to run an unimplemented actor.

- [x] **M0-06 - Define the public observation boundary and private evaluator types**
  - Deliver: Implement Feedback, Observation, MotorOutput, and explicit error types following Section 18. Keep hidden state/annotations in separate environment/evaluator modules. Serialize ordinary and hidden streams separately.
  - Verify: The ordinary agent API has no access to target, correctness, hazard, noise labels, future schedule, or split identity. Feedback IDs are used only for infrastructure; they never enter the feature vector.

- [x] **M0-07 - Implement birth mappings, cue exposure counters, and phase scheduling**
  - Deliver: Build the quiet -> cue -> optional gap -> response -> delayed feedback state machine. Initialize binary mappings independently, apply hazard before repeat exposure only, and sample timing through its own stream.
  - Verify: Table-driven schedules include zero-length memory gaps and exact declared tick counts. Hazard 0 never flips; hazard 1 flips every repeat but not the first exposure. Hazard is not applied on global decisions or quiet ticks.

- [x] **M0-08 - Implement commitments and one pending reward**
  - Deliver: At the final response tick, store action and mapping-at-commit, sample/store the reward noise once, and schedule one feedback at start(t_commit + delay). Maintain unique event identity and complete-lifetime outcome counts.
  - Verify: Delay 1 means next-tick feedback. A later mapping change cannot alter an already pending reward. No second commitment is accepted while unresolved; a completed lifetime has equal commitment/outcome counts and no pending reward.

- [x] **M0-09 - Build every observable feature and latch transition**
  - Deliver: Populate K + 6 channels exactly: one-hot cue, cue-present, go, outcome-present, outcome-value, and two previous-action channels. Keep sensory phase flags distinct from evaluator annotations.
  - Verify: Zero reward sets outcome-present=1 and outcome-value=0. Before first commitment both action channels are zero; afterward the new action appears starting next tick. Cues disappear outside presentation and feedback lasts exactly one tick.

- [x] **M0-10 - Add B0, B1, and the isolated O1 oracle harness**
  - Deliver: Implement random action, both constant-action agents, and a researcher-only hidden-state oracle. Route all through the same environment timing and reward accounting. Keep oracle privileges outside ordinary agent code paths.
  - Verify: The oracle has latent accuracy exactly 1. Random/constant controls use only their permitted information. Reward uses each agent's action and shared noise bit, not a shared forced reward across agents.

- [x] **M0-11 - Write deterministic environment contract fixtures**
  - Deliver: Populate tests/environment_contract.rs, tests/event_order.rs, and tests/leakage.rs with explicit schedules, mappings, forced noise bits, and expected features/rewards. Include commitment at tick 20 and delay 3.
  - Verify: Tick 23 start delivers exactly one outcome; ticks 21-22 are ordinary transitions. Cover every Section 17.1 item, correct/wrong zero-noise actions, forced noise reversal, and no feedback before commitment.

- [x] **M0-12 - Add seeded randomized environment sanity checks**
  - Deliver: Test cue/mapping/noise frequencies across sufficient randomized births and long schedules. Declare sample counts and statistical tolerances before executing; preserve fixed diagnostic seeds.
  - Verify: Random-action correctness is consistent with chance across randomized mappings, and oracle reward is consistent with its noise-conditioned expectation. Do not use expected oracle reward as a hard finite-run upper bound or rerun seeds until a test passes.

- [x] **M0-13 - Create minimal provenance and separated event logging**
  - Deliver: Write run manifest, fully resolved configuration, condition identity, ordinary event records, and evaluator-only annotations. Record code revision, dirty-tree status, platform/tool versions, RNG policy, and interruption/failure state.
  - Verify: Logs can reconstruct reward counts and schedule identity. Hidden truth never returns to the agent. Logging on/off yields identical behavior. Unsupported/nonfinite data and duplicate events produce explicit errors, not successful-looking output.

- [x] **M0-14 - Expose the first runnable CLI and log audit**
  - Deliver: Implement validate-config and environment-only simulate using the smoke profile and documented baseline selection. Add analysis/validate_logs.py using minimal dependencies, with fixtures for valid and deliberately corrupt logs.
  - Verify: A clean checkout can run the smoke command and audit its output. Audit checks order, counts, finite values, duplicate feedback, and missing completion records; unavailable later-stage fields are handled by schema, not fabricated.

- [x] **M0-15 - Run and save the M0 evidence bundle**
  - Deliver: Run the deterministic tests, bounded randomized checks, and random/constant/oracle smoke lifetimes. Save exact commands, configuration, seeds, logs, and results; update the tracker ledger and README.
  - Verify: All Section 16/M0 exit conditions have evidence. No neural code, evolution, or renderer is needed to pass this milestone.

- [x] **M0-GATE - Verify and record milestone exit**
  - All deterministic environment tests pass; oracle latent accuracy is exactly 1; chance controls pass the declared statistical checks; completed lifetimes have one delivered reward per commitment; hidden data and RNG isolation are verified. Save a runnable smoke command and example output before starting M1.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

- [x] **M0-REVIEW - Owner-requested corrective review**
  - Reviewed implementation/results against M0 contracts; corrected runner feedback/boundaries, unsupported execution, invalid-input determinism, atomic run ownership, and audit acceptance.
  - Verified: 73 Rust tests, 14 Python tests, fmt/clippy, four unchanged original runs and seven fresh bounded runs. See `docs/m0-review.md` and the appended evidence ledger entry.

- [x] **M0-DOCS - Prepare agent continuation documentation**
  - Deliver: Current `docs/handoff.md`, clear document authority and navigation, corrected bootstrap descriptions, and seed/evidence guides.
  - Verify: Local links/anchors, TOML/JSON parsing, source/config comment-only assertions, historical-record preservation, fmt, warning-free Rust documentation build, and portable fixture audit. No fresh simulator test run is claimed.

---

## M1 - Build a continuous actor with no learning

**Spec:** Sections 3-6, 10, 16/M1, 17.2, 17.7, 18.3-18.5.  
**Prerequisite:** M0-GATE.

Establish correct, continuously evolving actor dynamics and replay before introducing plasticity.

- [x] **M1-01 - Implement inherited topology and structural validation**
  - Deliver: Sample the directed Bernoulli mask, initially with no self-edges; fix a stable edge order and motor assignment. Check cue-driven-to-motor reachability and recurrent cycles. Log rejected structural samples and reasons.
  - Verify: The same initialization reproduces the same mask. Selection is structural, never based on test performance. Missing edges stay absent. Pair masks and actor inheritance across future gate conditions.

- [x] **M1-02 - Initialize inherited weights and neuron parameters**
  - Deliver: Implement W0 row scaling by in-degree, input projection B, zero actor biases, and the starting constants from Sections 6 and 10. Keep W0 separate from future plastic offsets.
  - Verify: Use standard deviation recurrent_gain/sqrt(in_degree), not the variance as a standard deviation. Handle zero-in-degree rows explicitly. Validate motor capacity, disjoint pools, time constants, finiteness, and dimensions.

- [x] **M1-03 - Implement the double-buffered f64 actor transition**
  - Deliver: Compute alpha with -expm1(-1/tau), read only old h/r/a on right-hand sides, accumulate recurrent and sensory drive, add Gaussian noise after leaky integration, and compute new tanh activity.
  - Verify: One-edge and two-neuron fixtures prove receiver/source orientation and simultaneous updates. Forced perturbations prove the leak factor is not accidentally applied to sigma. No hidden state clipping or inner-loop allocations are introduced.

- [x] **M1-04 - Verify the perturbation generator and draw schedule**
  - Deliver: Support a deterministic injected-noise fixture path for tests and a pinned stochastic generator for production. Draw one perturbation per actor neuron per tick, including quiet periods.
  - Verify: Seeded sample moments match the specified mean/variance within declared tolerances. Extra logging and unused gate changes do not shift draws. Record distribution implementation and RNG state needed for resume.

- [x] **M1-05 - Implement adaptation but keep its initial strength zero**
  - Deliver: Update the signed adaptation average from old activity using its separate time constant. Preserve it across all environmental phase boundaries.
  - Verify: At strength zero adaptation has no influence on actor dynamics. A nonzero-strength unit fixture checks its sign and recurrence, but the initial learner is not simultaneously complicated by enabling this mechanism.

- [x] **M1-06 - Implement fixed motor pools, filtering, and commitment**
  - Deliver: Average new activities within the two fixed disjoint motor populations, apply the leaky motor filter, and choose the higher filtered output at commitment with a dedicated fair tie RNG.
  - Verify: Golden filter recurrences pass. Commitment reads the new q values; exact ties use only the tie stream. No trained decoder, softmax exploration, or epsilon-greedy actor policy is added.

- [x] **M1-07 - Integrate the nonplastic actor through the common runner**
  - Deliver: Expose continuously available motor output and the Section 18 agent boundary. Process public reward as sensory input without weight learning. Use an explicit actor-no-learning profile for B3.
  - Verify: All phases advance the network; no cue/reward/hidden-change boundary resets it. The inherited weights remain identical throughout. Environment scheduling is unchanged from M0.

- [x] **M1-08 - Add numerical health and selected actor traces**
  - Deliver: Record sampled activity/adaptation/motor traces, saturation, margins, and state finiteness. Define a conservative watchdog and explicit failure records rather than clipping h.
  - Verify: A forced nonfinite value fails visibly. Trace selection is stable and does not consume simulation randomness. Diagnostic output is saved without making an interactive notebook mandatory.

- [x] **M1-09 - Implement the first full lifetime checkpoint**
  - Deliver: Serialize all state currently present: actor, adaptation, motor, environment phase, pending reward/action latch, bookkeeping, inherited parameters, resolved config, and full RNG state/counters. Add schema, hashes, checksum, and atomic writes.
  - Verify: Resume at quiet, response, and pending-feedback boundaries reproduces uninterrupted continuation on the recorded reference platform. Reject corrupt/incompatible state; do not restore missing state with silent defaults.

- [x] **M1-10 - Add actor continuity and replay regression tests**
  - Deliver: Populate tests/replay.rs and dynamics tests for saved old arrays, exact motor commitment, phase continuity, logging invariance, and checkpoint splits. Record the reference platform and tolerance policy.
  - Verify: Identical code/config/seeds reproduce the reference trajectory. Test instructions distinguish reference-platform bitwise replay from cross-platform tolerance comparisons.

- [x] **M1-11 - Run cue observability and bounded long-run smoke tests**
  - Deliver: Use fixed inputs, alternating cues, long quiet periods, and multiple initializations. Save activity/motor diagnostics and numerical-health summaries.
  - Verify: Both actions are reachable across initializations; distinct cues produce distinguishable activity; long runs remain finite. These checks establish usable dynamics, not learning or biological realism.

- [x] **M1-12 - Expose and document the no-learning actor command**
  - Deliver: Save configs/actor_no_learning.toml and a runnable simulate example using actual generated run paths. Update manifest coverage and the task ledger.
  - Verify: A reader can reproduce the M1 demo and checkpoint continuation. Any unpassed observability or numerical-health requirement remains a blocker.

- [x] **M1-GATE - Verify and record milestone exit**
  - The actor obeys simultaneous-update and noise contracts, continuously preserves state, has usable cue/motor responses across seeds, remains finite in the declared smoke run, and exactly resumes on the reference platform. This milestone is explicitly a dynamical-system demonstration, not learning.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M2 - Verify the stochastic score independently

**Spec:** Sections 7.1-7.2, 7.6, 16/M2, 17.4-17.6.  
**Prerequisite:** M1-GATE.

Validate the conditional Gaussian score and restricted finite-rollout interpretation without relying on an apparent learning curve.

- [x] **M2-01 - Implement a pure conditional score function**
  - Deliver: Implement S[j,i] = alpha_h[j] * r_old[i] * xi[j] / sigma[j]. Keep it independently callable by tests and reusable by the later trace implementation.
  - Verify: Tests use receiving xi, one alpha_h factor, actual sigma, and no tanh derivative. All incoming edges share their receiver perturbation; zero presynaptic activity yields zero score; active score with sigma <= 0 is rejected.

- [x] **M2-02 - Implement the fixed-sample conditional log-probability check**
  - Deliver: For a saved old state and saved h_new, perturb one weight by +/- eps and recompute Gaussian log probability. Compare its central finite difference with the analytical score.
  - Verify: Test several eps values around 1e-6 and several parameters. The sampled h_new must stay fixed; do not resample it or use the perturbed noise realization as a new observation.

- [x] **M2-03 - Implement the one-neuron analytical learning-direction test**
  - Deliver: Use alpha=0.2, input=0.7, weight=0.3, sigma=0.4; compare the mean of (reward-0.5)*score against the closed-form derivative, approximately 0.138862. Save sample count, seed, mean, and standard error.
  - Verify: Use a tolerance declared before execution, such as five standard errors plus numerical tolerance. Confirm the opposite target reverses the derivative sign. Investigate a failure instead of rerunning until it passes.

- [x] **M2-04 - Build the exact finite-rollout diagnostic mode**
  - Deliver: Use weight-independent initial state, fixed weights during a rollout, fixed nonzero noise, no state clipping, no eligibility decay, and a baseline fixed independently of rollout perturbations. Sum local scores and apply at most one terminal update.
  - Verify: Tests prohibit online updates or running-baseline changes inside this diagnostic. Its resets and no-decay policy are explicit, not hidden meanings of birth_only or a very large finite tau.

- [x] **M2-05 - Run the short two-neuron recurrent finite-difference check**
  - Deliver: Implement the Section 17.6 short-horizon test over several weight perturbation magnitudes. Compare sampled expected-reward finite differences with terminal reward times accumulated score and quantify uncertainty.
  - Verify: Weights stay fixed within each rollout; initial state and baseline obey the diagnostic assumptions. Use adequate samples or report unresolved uncertainty rather than accepting a wide interval as strong evidence.

- [x] **M2-06 - Package score tests as bounded reproducible diagnostics**
  - Deliver: Keep deterministic derivative tests in the fast suite and expensive Monte Carlo checks in explicit bounded commands. Save their configuration, analytic expectations, seeds, samples, and results.
  - Verify: All required diagnostic checks actually run before the milestone gate. Documentation states that these tests do not prove unbiasedness or convergence of the main online learner.

- [x] **M2-GATE - Verify and record milestone exit**
  - Deterministic derivative tests pass and Monte Carlo estimates agree with the specified analytical/recurrent checks within prespecified uncertainty tolerances. Save runnable diagnostic commands and evidence. Do not substitute a plotted reward curve for score validation.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M3 - Make an ungated local learner learn a clean task

**Spec:** Sections 7, 10-11, 16/M3, 17.2-17.5, 19.2.  
**Prerequisite:** M2-GATE.

Demonstrate learning from delayed terminal rewards in a deliberately episodic diagnostic before removing resets.

- [x] **M3-01 - Add plastic offsets, eligibility, and plastic masks**
  - Deliver: Store P and E separately from immutable W0. Support motor-afferent-only and all-existing-recurrent-edge plastic masks. Add explicit no-decay diagnostic accumulation and persistent-decay policy as separate configurations.
  - Verify: Missing/nonplastic edges never acquire updates. Effective weight caches are refreshed in one tested location. All new state participates in checkpoint serialization and compatibility validation.
  - Evidence: `docs/evidence/m3-01/summary.md` (2026-09-21 UTC). `src/agent/plasticity.rs` stores `P`/`E` separately from `W0`, builds both masks, implements `persistent` vs `no_decay_diagnostic`, and exposes `refresh_effective` as the single cache writer; `PlasticSnapshot` (schema 1, `deny_unknown_fields`) is validated on restore. `src/agent/actor.rs` adds the effective-weight transition sharing the `W0` core. 16 new tests in `tests/plasticity.rs`; 232 fast Rust tests pass, 3 default ignores, 2 compile-fail doc checks, clean fmt/Clippy. Checkpoint embedding/replay with nonzero `P`/`E` remains M3-10/M4-06.

- [x] **M3-02 - Implement exactly-once feedback updates and baseline arithmetic**
  - Deliver: Read old E/gates/P/baseline, compute delta, clamp raw updates per edge, clamp resulting P, and update the baseline once after delta. Fixed mode uses gate 1; the diagnostic baseline follows its declared fixed-rollout policy.
  - Verify: Duplicate feedback is rejected without changing state. eta=0, gate=0, and delta=0 give zero task-dependent changes. W0 never changes. Tests separate raw, limited, and actual update values at both clipping boundaries.
  - Evidence: `docs/evidence/m3-02/summary.md` (2026-09-21 UTC). `PlasticState::apply_feedback_once` reads old `E`/per-receiver gates/`P`/baseline, reports `delta` plus full raw/limited/actual matrices, enforces ordered `max_update`/`plastic_bound` clamps, updates the baseline once after `delta`, marks monotonic dedup, and refreshes the single cache; `E` never resets, `W0` never mutates. `PlasticSnapshot` schema 2 carries baseline plus required `last_feedback`. 11 new tests in `tests/feedback_updates.rs`; 243 fast Rust tests pass, 3 default ignores, 2 compile-fail doc checks, clean fmt/Clippy. No runner/gate/acquisition wired; M3-03 golden and M3-04 episodic runner remain.

- [x] **M3-03 - Pass the hand-calculated golden update fixture**
  - Deliver: Implement Section 17.3 independently of ordinary configured time constants: score 0.4, new eligibility 0.67, delta 0.4, raw update 0.00067, new P 0.10067, and new baseline 0.64.
  - Verify: Use tight stated floating-point tolerances. Assert the old baseline is used for delta, and test both clipped and unclipped cases separately. Save the fixture in tests/golden_updates.rs.
  - Evidence: `docs/evidence/m3-03/summary.md` (2026-09-21 UTC). Fixture-only `tests/golden_updates.rs` drives the 17.3 chain through `conditional_score`/`advance_eligibility`/`apply_feedback_once` with explicit `alpha_h = 0.5`, `lambda_e = 0.9`; unclipped golden plus separate per-edge and bound clamps at `1e-12`–`1e-15` (1-ulp allowance on one `actual == limited` comparison). 4 new tests; 247 fast Rust tests pass, 3 default ignores, 2 compile-fail doc checks, clean fmt/Clippy. No production change; M3-04 episodic runner remains.

- [ ] **M3-04 - Create the explicitly episodic clean-learning runner**
  - Deliver: Use two unknown cue-action mappings, zero noise, no reversals or blank gap, short delay, reset state/traces between diagnostic rollouts, no trace decay, and one terminal update. Name this profile episodic_stationary, separate from continuous debug_stationary.
  - Verify: Configuration and logs visibly identify diagnostic resets. Every rollout respects the terminal-credit contract. Do not present this run as the main continuous result.

- [ ] **M3-05 - Add matched no-update and shuffled-reward controls**
  - Deliver: Run B3 and always-on B4 through matched actor initialization, timing, and task schedules. Define a development shuffled-reward control with its corruption protocol recorded and no hidden information fed to the actor.
  - Verify: Controls differ only in the declared mechanism; observed environment reward remains separately recorded where diagnostic teaching signals are modified. Check both behavior and actual P changes rather than inferring learning from reward alone.

- [ ] **M3-06 - Declare the development grid and acquisition criterion**
  - Deliver: Specify a small grid over eta, input scale, recurrent gain, and sigma using development seeds only. Declare number of seeds, sample lengths, acquisition windows, and the learning-vs-control criterion before results.
  - Verify: Treat the spec's example final-200-choice median accuracy >0.8 in a 2,000-choice run as a proposed debugging target, not a guaranteed benchmark or automatically fixed final-study threshold. Log tuning budget and every outcome.

- [ ] **M3-07 - Demonstrate acquisition with motor-afferent plasticity**
  - Deliver: Run the smallest diagnostic first with only incoming motor edges plastic. Save early/late exposure accuracy, reward, offset/trace norms, saturation, and seed-by-seed paired controls.
  - Verify: Several seeds show the declared learning improvement over no-update and shuffled-reward controls without widespread clipping or numerical failure. A single favorable trajectory does not pass.

- [ ] **M3-08 - Extend the verified learner to all recurrent plastic edges**
  - Deliver: Use the same verified score/update machinery with the full existing-edge plastic mask. Repeat the declared development comparisons and preserve the motor-only run as a diagnostic.
  - Verify: Show the required learning evidence for the actor family carried into M4. Do not compare different plastic masks later while attributing every difference to gates.

- [ ] **M3-09 - Add the smallest failure-isolation path and negative checks**
  - Deliver: Provide a one-weight/noisy-motor diagnostic and a documented reduction path for failed acquisition. Add the postsynaptic-perturbation permutation diagnostic and investigate unexpected equivalence without demanding a particular failure magnitude.
  - Verify: A failing learner is reduced to a sign/order/representation test rather than rescued by evolution. Record whether this troubleshooting path was needed and what was found; do not change the scientific rule invisibly.

- [ ] **M3-10 - Save reproducible learning evidence and updated checkpoints**
  - Deliver: Archive working and failure configurations, all development grid results, paired controls, actual update summaries, and learned-offset checkpoints. Extend exact replay tests through learning events.
  - Verify: Checkpoint splitting with nonzero P/E reproduces uninterrupted learning. The evidence distinguishes correct arithmetic, episodic acquisition, and still-unproven continuous acquisition.

- [ ] **M3-GATE - Verify and record milestone exit**
  - The clean episodic learner demonstrably improves over matched no-update and shuffled-reward controls across several development seeds; score/golden-update tests remain valid and numerical behavior is interpretable. If it does not learn, leave this gate open and debug. Do not start M4 or evolution on the strength of implemented code alone.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M4 - Remove artificial trial resets

**Spec:** Sections 5.7, 7.3-7.9, 8.3, 9-10, 16/M4, 17.2, 19.2.  
**Prerequisite:** M3-GATE.

Show that the ungated learner still acquires associations with persistent neural state and eligibility.

- [ ] **M4-01 - Implement the authoritative main tick order**
  - Deliver: Apply any feedback using pre-tick E/gates/baseline before advancing the actor. Then advance actor, optional modulator, eligibility, motor filter, and future gates; commit from new motor output and finish the tick. Until M6, the gate is fixed at 1.
  - Verify: An event-order fixture proves current feedback-evoked activity cannot contribute to the same outcome's update. The tick-20/delay-3 case uses traces through tick 22 at tick 23 start. There is exactly one feedback application path.

- [ ] **M4-02 - Enable persistent eligibility and the running baseline**
  - Deliver: Implement E <- exp(-1/tau_e)*E + score every tick, including quiet/delay/feedback transitions, without feedback resets or an extra (1-lambda) factor. Use baseline 0.5 at birth and beta_R per outcome.
  - Verify: Trace recurrences and baseline event counts pass deterministic fixtures. Scores after commitment remain in the live trace as specified; do not silently replace it with a commitment snapshot.

- [ ] **M4-03 - Enforce birth-only resets and warmup semantics**
  - Deliver: Instrument reset reasons and allow the primary condition to reset only at independent lifetime birth. Preserve h/a/q/P/E and, once implemented, z/gates across all phase boundaries. Run the optional warmup with live traces.
  - Verify: Tests detect any reset at cue changes, rewards, hidden reversals, log rotation, or post-warmup. Birth clears acquired state and bookkeeping exactly. Finishing a lifetime includes the final feedback transition.

- [ ] **M4-04 - Create three explicitly distinguished continuity profiles**
  - Deliver: Keep the episodic diagnostic, persistent activity with event_reset_diagnostic traces, and fully persistent activity/traces as separately named conditions. Create continuous_stationary and the source debug_stationary profile without overloading reset flags.
  - Verify: Resolved configs and logs identify each policy. Validation rejects a run labeled persistent that clears traces. No comparison mistakes diagnostic resets for an equivalent implementation of the main model.

- [ ] **M4-05 - Introduce variable timing and delayed outcomes gradually**
  - Deliver: Keep stationary clean mappings while increasing timing variability and reward delay through declared development profiles. Preserve the single-pending-choice rule and identical exogenous schedules for paired comparisons.
  - Verify: Table tests cover timing endpoints. Measured delay sensitivity is saved together with tau_e, actual trace magnitudes, and update norms; longer traces are not assumed to be strictly better.

- [ ] **M4-06 - Extend checkpoints and replay to continuous learning**
  - Deliver: Checkpoint during nonzero traces/offsets, just before feedback, and after feedback. Include baseline, consumed-event identity, latches, and any derived caches needed for exact continuation.
  - Verify: Split runs match uninterrupted continuous runs on the reference platform. Duplicate resume/delivery does not double-apply a reward; missing new fields are incompatible rather than silently reset.

- [ ] **M4-07 - Run the declared continuous acquisition comparison**
  - Deliver: Compare the three continuity conditions and matched nonplastic controls across development seeds. Use predeclared per-cue exposure windows and report raw/actual updates, bound occupancy, motor saturation, and failures.
  - Verify: Above-chance acquisition remains measurable in the fully persistent condition and exceeds the declared matched control criterion. Episodic success alone is insufficient.

- [ ] **M4-08 - Audit continuity failures before expanding scope**
  - Deliver: Use the Section 21 reduction path to inspect cross-choice interference, ordering, baseline drift, trace timescale, and saturation. Record changes to development ranges and rerun affected controls.
  - Verify: Do not introduce hidden resets, weight decay, membrane clipping, or a trained decoder to pass. If continuous learning remains unverified, leave the milestone blocked and report the negative finding honestly.

- [ ] **M4-09 - Save the M4 continuous-system evidence bundle**
  - Deliver: Save resolved profiles, reset audit, several-seed acquisition/control results, exact replay evidence, sample traces, and the runnable continuous-clean command.
  - Verify: The report labels the main learner a heuristic online local system rather than importing the restricted diagnostic's unbiased-gradient interpretation.

- [ ] **M4-GATE - Verify and record milestone exit**
  - The birth-only-reset learner shows the declared acquisition evidence with persistent traces; no within-lifetime reset hooks fire, replay passes, and trace/update/bound statistics remain interpretable. This is the first core continuous-learning result. An unresolved failure blocks noisy-task and gate-search claims.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M5 - Establish the adaptation/noise trade-off

**Spec:** Sections 5.2-5.3, 5.9, 13.2-13.4, 14.1-14.7, 16/M5, 17.8, 22.  
**Prerequisite:** M4-GATE.

Characterize a tuned always-on learner and simple references before asking evolution to improve plasticity regulation.

- [ ] **M5-01 - Implement the remaining core environment profiles**
  - Deliver: Add stationary_noisy, isolated_reversal, mixed_continual, long_life, and uninformative_reward. Keep isolated scheduled reversals separate from the memoryless main hazard process.
  - Verify: Named profiles do what their resolved configs state. The main process has no unreported minimum dwell time. The noise=0.5 profile is a negative control, not a learnable target.

- [ ] **M5-02 - Counterbalance noise, volatility, and cue identities**
  - Deliver: Randomize stable/volatile membership at birth and stratify the proposed noise/hazard combinations across lifetimes/batches. Implement noisy stable and reliable volatile cues, with mappings independent of cue roles.
  - Verify: No cue magnitude, index convention, or frequency encodes volatility. All training combinations are represented across the declared suite; do not assume eight cues can cover every combination within one lifetime. Save realized change/noise counts.

- [ ] **M5-03 - Implement B2, the observable-cue tabular learner**
  - Deliver: Remember the last observed cue through its delayed outcome, maintain Q[c,action] initialized to 0.5, and implement the specified update with epsilon-greedy exploration. Keep its learning rate/exploration tuning separate and recorded.
  - Verify: Cue memory comes from sensory input, never an evaluator pointer. Tests check delayed credit, initial values, action selection, and updates. Confirm it learns the clean task before using it as a reference.

- [ ] **M5-04 - Implement O2, the known-parameter belief reference**
  - Deliver: Implement the binary-flip prior and noisy-label Bayesian update from Section 13.4, with explicit access to true hazard/noise only inside this privileged comparator.
  - Verify: Hand fixtures verify first-vs-repeat exposure, both actions/rewards, zero-noise cases, and finite probability arithmetic. Impossible dogmatic observations raise checks. Label O2 privileged and distinct from a fair ordinary learner.

- [ ] **M5-05 - Implement primary metrics and per-cue acquisition summaries**
  - Deliver: Compute observed reward, latent accuracy at commitment, and regret=(1-2*epsilon)*wrong from evaluator data. Aggregate by cue exposure and stable/volatile subgroup, with declared early/late windows.
  - Verify: Hand-calculated streams validate every metric. Fitness still uses observed reward only; hidden-truth metrics remain analysis-only. Missing/failing records are distinct from valid zero performance.

- [ ] **M5-06 - Implement isolated-reversal recovery and interference metrics**
  - Deliver: Track changed-cue errors in a declared post-change exposure window, interleaved unchanged-cue performance, and optional recovery thresholds. Record censoring and exclude/censor overlapping events only in the event-aligned analysis.
  - Verify: Tests include never-recovered events, incomplete windows, repeated flips, and uneven cue frequency. Full-lifetime metrics still include the entire stream. Do not drop unrecovered events or count them as zero errors.

- [ ] **M5-07 - Add learning-health diagnostics and negative controls**
  - Deliver: Record eligibility and actual-update L1/L2, raw/limited/actual clipping, plastic-bound occupancy, activity saturation, and motor margins. Run uninformative/shuffled-feedback controls with explicit protocols.
  - Verify: An apparent absence of updates can be distinguished from bound saturation. Feedback independent of correctness does not reliably reveal random mappings across the declared seeds; an anomaly triggers a leakage audit, not a learning claim.

- [ ] **M5-08 - Run the declared eta-by-tau_e development sweep**
  - Deliver: Start from the spec's proposed eta values {1e-4,3e-4,1e-3,3e-3} and tau_e values {16,32,64,128}, or record a reason for a revised range before using it. Tune fixed plasticity jointly, not one parameter in isolation.
  - Verify: Save all configurations, seeds, candidate-lifetime/tick counts, failures, recovery, and stable/noisy performance. No final-test information is used to select the fixed baseline.

- [ ] **M5-09 - Characterize whether a meaningful trade-off actually exists**
  - Deliver: Summarize the fixed-rule performance surface and whether faster recovery is accompanied by noise/interference damage. Include the tuned tabular reference and both clean and noisy stable cues.
  - Verify: Do not cherry-pick a weak fixed eta to create room for gates. If a single fixed setting works everywhere, record that outcome and a transparent proceed/revise decision rather than claiming the intended trade-off was found.

- [ ] **M5-10 - Benchmark representative complete lifetimes**
  - Deliver: Add the benchmark command; measure ticks/s, outcomes/s, and output cost for small representative lifetimes with logging off/on. Record hardware, build profile, actual tick counts, and realized reversals.
  - Verify: Validate debug cycle accounting and the main average-timing arithmetic from Section 22. The 60-neuron/100-generation nominal configuration is not launched merely to obtain a benchmark.

- [ ] **M5-11 - Freeze the development baseline settings for the first gate study**
  - Deliver: Save chosen actor family, plastic mask, eta/tau_e, task mixture, timing, and selection criteria. Record the fixed baseline's tuning budget and any unresolved task weakness.
  - Verify: The same base condition can generate fixed/global/targeted variants with explicit overrides. The M5 record distinguishes empirical evidence from proposed future changes.

- [ ] **M5-GATE - Verify and record milestone exit**
  - The always-on learner has a characterized, reproducible performance surface; tabular and privileged references are verified; metrics and hidden-data separation pass; a trade-off is measured or its absence is explicitly documented with a proceed/revise decision. Do not assert an unobserved trade-off or move directly to a large search.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M6 - Add gates before evolving them

**Spec:** Sections 3.3, 8-9, 16/M6, 17.2, 17.7, 19.5.  
**Prerequisite:** M5-GATE.

Prove that global and receiving-neuron gates do exactly what the intended architecture says, independently of optimization.

- [ ] **M6-01 - Implement the deterministic recurrent modulator**
  - Deliver: Add z, the actor-to-modulator C projection, recurrent H, sensory U, biases, and tau_m. Use old actor activity and old modulator state with current ordinary inputs. Do not create an ordinary modulator-to-actor activation pathway.
  - Verify: One-step golden tests verify old-state use and leaky dynamics. The modulator receives no hidden labels. It advances during every phase and becomes part of birth/reset/checkpoint state.

- [ ] **M6-02 - Implement fixed, global, and targeted gate heads**
  - Deliver: Fixed mode emits 1; global emits one stable-sigmoid scalar; targeted emits one sigmoid per receiver shared by incoming plastic edges. Validate dimensions and initialize gate heads near half-open as specified.
  - Verify: Gates stay finite within [0,1]. Check initialization standard deviations from Section 8.5 rather than confusing variance with standard deviation. Fixed eta and half-open gate eta are not automatically treated as equal effective rates.

- [ ] **M6-03 - Test gate arithmetic through controlled injections**
  - Deliver: In a diagnostic harness, supply all-zero, all-one, all-half, single-receiver, and alternating gate vectors with known E and delta. Keep such injections separate from ordinary learned-gate configurations.
  - Verify: Zero gates block task-dependent P updates; all-one matches fixed mode; half gates halve unclipped updates; one enabled receiver changes only its incoming plastic edges. Clipping exceptions are measured explicitly.

- [ ] **M6-04 - Prove pre-outcome gate timing**
  - Deliver: Use sentinel old gates and deliberately different post-feedback gates to test the exact feedback event. Keep old gate/eligibility snapshots distinct from the values logged after the transition.
  - Verify: Current feedback is gated only by the pre-feedback values. Feedback-tick scores and gates cannot be reused retroactively. Extend the tick-20/delay-3 regression to a nonconstant modulator.

- [ ] **M6-05 - Prove modulation-only causal isolation**
  - Deliver: With P updates disabled and the actor noise schedule paired, vary modulator parameters or turn modulation off while preserving actor inheritance/input.
  - Verify: Actor states, motor values, actions, and rewards are unchanged on the reference platform. A difference reveals a forbidden activation pathway, RNG coupling, or another implementation bug.

- [ ] **M6-06 - Extend gate diagnostics and full-state replay**
  - Deliver: Log event-aligned gate vectors and summaries, near-bound fractions, diversity, and autocorrelation for selected lifetimes. Add z/current gates/caches to exact checkpoint state.
  - Verify: Resume preserves the gate used at the next reward, not merely a newly recomputed approximate value. Logging records which values were applied versus produced for future feedback.

- [ ] **M6-07 - Generate matched gate-condition configurations**
  - Deliver: Create mixed_fixed, mixed_global, and mixed_targeted from one shared base and explicit overrides. Pair actor/mask/motor/input/noise settings; record rate tuning and inherited parameter differences.
  - Verify: A machine-readable comparison identifies every difference. Fixed/global/targeted conditions are not independently edited files that silently diverge. No evolution is enabled until this milestone passes.

- [ ] **M6-08 - Save gate-fixture and regression evidence**
  - Deliver: Run the complete gate arithmetic, timing, architecture-isolation, replay, and earlier score/environment suites. Save controlled examples without claiming their hand-designed gate patterns are evolved results.
  - Verify: The tests establish actual update behavior, not just that gate values can be plotted. The milestone evidence includes all required Section 16/M6 checks.

- [ ] **M6-GATE - Verify and record milestone exit**
  - Zero/one/half/single-receiver/alternating gate checks pass; pre-outcome ordering and modulation-only isolation are proven; full-state replay and paired-condition config audits pass. Only now is evolutionary search eligible.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M7 - Evolve a small learning-control mechanism

**Spec:** Sections 11-12, 16/M7, 17.7, 19.3-19.5, 20, 22.  
**Prerequisite:** M6-GATE.

Test a small, auditable gate-only search before scaling model size or inherited search dimension.

- [ ] **M7-01 - Define immutable genomes and fresh-birth construction**
  - Deliver: Serialize inherited parameters separately from acquired state. Initialize each candidate lifetime from birth with P=E=0 and reset dynamic/bookkeeping state. Pair actor inheritance across gate conditions within outer seed.
  - Verify: Tests show no learned offsets, traces, reward baselines, or neural state leak between evaluation lifetimes or generations. A genome cannot be mistaken for a lifetime checkpoint.

- [ ] **M7-02 - Implement bounded parameter encoding and search spaces**
  - Deliver: Decode searched weights with weight_limit*tanh(theta); encode valid initialized weights with atanh(weight/limit), redrawing out-of-bound initial samples. Start projection-only with C/H/U fixed; support the later modulator-and-gate space explicitly.
  - Verify: Round trips and mutation-to-decoded-weight tests pass. Parameter counts match shape formulas, including 560 targeted and 265 global parameters for the main full-modulator parameterization. Initial eta/tau_e/tau_m/sigma are not silently evolved.

- [ ] **M7-03 - Implement the deterministic elitist evolutionary engine**
  - Deliver: Implement population initialization, fitness sorting with fixed ties, elite preservation, uniform elite-parent choice, and Gaussian genotype mutation. Use a small explicit configuration; the proposed nominal values are not mandatory smoke-run sizes.
  - Verify: Test population/elite counts, deterministic ties, mutation reproducibility, and bounded decoding. Run a trivial numerical objective with a known optimum before attaching the simulator.

- [ ] **M7-04 - Build stratified common training batches**
  - Deliver: Generate one fresh training batch per generation, shared by all candidates. Pair initial mappings, cue sequence, changes, timing, and noise bits while computing reward from each candidate's own action.
  - Verify: A manifest records the exact batch and strata. Altering actor RNG draws leaves exogenous schedules unchanged. The batch contains enough actual changes for the intended fitness signal; a six-lifetime batch is not assumed adequate automatically.

- [ ] **M7-05 - Implement full-lifetime scoring and explicit failure policy**
  - Deliver: Score total observed reward divided by delivered outcomes, counting acquisition onward. Complete the configured outcome count. Assign failed candidates the predeclared finite worst fitness and a reason.
  - Verify: No hidden accuracy or gate-shape bonus enters fitness. Candidate-level failure records are saved even when event logs are off. Early failures cannot look favorable because they completed fewer outcomes; missing data stays separately identifiable.

- [ ] **M7-06 - Reevaluate every candidate, including elites, on the fresh batch**
  - Deliver: Evaluate preserved elites again each generation; aggregate candidate lifetime scores in a deterministic order. Keep candidate IDs independent of scheduling/completion order.
  - Verify: A regression catches stale elite scores compared with fresh child scores. Every candidate in a generation uses exactly the same batch; lifetime count and scoring denominators are audited.

- [ ] **M7-07 - Implement validation selection and untouched-test safeguards**
  - Deliver: Use disjoint validation lifetimes at declared intervals and a prespecified selection metric. Save distinct last-generation, best-training, and best-validation candidates. Enforce namespace manifests and hashes.
  - Verify: Final-test seeds never enter tuning/search/validation. An overlap is a hard audit failure. A visually attractive training trajectory cannot silently replace the validation-selected genome.

- [ ] **M7-08 - Implement atomic resumable search checkpoints**
  - Deliver: Save population/genotypes, generation position, current scores/batch, evaluation bookkeeping, RNG state/counters, validation history, config/code identifiers, and selection state. Use atomic writes and compatibility checks.
  - Verify: A split serial search produces the same resumed population, scores, selection, and validation record as an uninterrupted reference run. Partial files are rejected instead of restarted with default values.

- [ ] **M7-09 - Create the small-search budget and launch profile**
  - Deliver: Start around 16 actor neurons and 2 modulators with gate-projection-only search. Choose bounded candidate-lifetime/tick budgets using measured throughput and realized change counts. Record training, tuning, validation, and failure costs.
  - Verify: The profile has enough lifetime exposure to test the phenomenon. Do not shorten lifetimes until reversals disappear or launch the nominal 60-neuron search without a recorded budget decision.

- [ ] **M7-10 - Run bounded serial search smoke tests for both gate families**
  - Deliver: Exercise global and targeted projection search through the same evaluated actor family and tuned fixed baseline. Save complete candidate summaries and selected validation replays.
  - Verify: The entire search-to-selection-to-fresh-birth evaluation path works with no inherited P or final-test leakage. These smoke runs are integration checks, not proof of a robust advantage.

- [ ] **M7-11 - Run independent small discovery replicates**
  - Deliver: Use several fresh outer seeds/topologies and report each fixed/global/targeted result. Analyze improvement or null results with mutation behavior, gate saturation, ranking noise, and realized environmental changes.
  - Verify: A reproducible advantage or a clearly documented valid null result may complete this task. A stalled/broken engine, ineffective base learner, or insufficient fitness signal is diagnosed rather than interpreted as impossibility.

- [ ] **M7-12 - Add bounded parallel evaluation after serial correctness**
  - Deliver: Parallelize independent candidate/lifetime jobs only; keep each small tick loop single-threaded. Preserve explicit stream ownership and deterministic reduction/selection order.
  - Verify: Serial and parallel candidate scores/selection match on the reference platform, including deliberately reordered worker completion. Avoid nested oversubscription; record worker count.

- [ ] **M7-13 - Profile and decide whether a sparse kernel is justified**
  - Deliver: Measure the dense reference and total lifetime bottlenecks. Record whether to retain it or add a stable receiver-grouped sparse edge kernel. If adding sparse execution, keep the dense implementation as an oracle.
  - Verify: Any sparse kernel agrees with dense intermediate states, scores, updates, and outputs on identical graph/noise fixtures within a declared tolerance. No performance claim replaces measured throughput. A documented decision not to optimize is valid.

- [ ] **M7-14 - Save the discovery report and gate-only claim limits**
  - Deliver: Package selected inherited genomes, manifests, checkpoint/replay examples, all candidate/failure summaries, budgets, and validation comparisons. Explain which modulator parameters were frozen versus searched.
  - Verify: Projection-only success is not described as evolution inventing the entire history representation. A fixed-actor gate study is not claimed superior to a separately optimized activity-only architecture.

- [ ] **M7-GATE - Verify and record milestone exit**
  - The evolutionary engine passes its known-objective and resume tests; candidate evaluation uses fresh births and fresh shared batches; selected genomes are chosen on validation only; multiple outer seeds yield reproducible evidence or an honestly documented null result. No large search proceeds without throughput, event-count, and budget evidence.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M8 - Run the primary comparisons

**Spec:** Sections 13-14, 16/M8, 19.5, 20, 22, 24.  
**Prerequisite:** M7-GATE.

Build the full comparison/analysis pipeline and rehearse it on development and validation data. The untouched final-test execution is deliberately reserved for M10 after intervention tooling is verified.

- [ ] **M8-01 - Implement the common evaluation suite runner**
  - Deliver: Load a selected frozen genome, create fresh births with P=0, execute versioned suite manifests, and preserve ordinary lifetime learning unless explicitly testing a frozen-plasticity intervention. Track completed, failed, and interrupted jobs.
  - Verify: Equivalent conditions use paired environment suites and outer initializations. Interrupted runs cannot be silently treated as completed. Ordinary evaluation freezes evolution, not learning.

- [ ] **M8-02 - Audit the baseline ladder and condition differences**
  - Deliver: Make B0/B1/B2/B3/B4/B5/B6 and O1/O2 available through common scoring/output contracts. Compare actor topology, W0, B, motor assignment, plastic masks, noise, lifetime length, tuning, and training distributions.
  - Verify: Report parameter counts and persistent state sizes alongside budgets. B3 is labeled a same-actor no-update control, not a competitively optimized activity-only agent. Oracles remain visibly privileged.

- [ ] **M8-03 - Choose and document the claim track**
  - Deliver: Record family_only for the initial fixed-actor gate study, or explicitly authorize the conditional B7 track for broader activity-only comparisons. Draft docs/study_plan.md with H1-H5, primary/secondary metrics, controls, and budget.
  - Verify: A family_only result never claims superiority over the best activity-only architecture. If the broader track is selected, B7-01 through B7-GATE must pass before final selection/evaluation for that claim.

- [ ] **M8-04 - Build and freeze generalization-suite generation rules**
  - Deliver: Define fresh seeds/mappings/role permutations, interpolation noise 0.15 and hazard 0.01, harder noise 0.30 and/or hazard 0.04, new per-cue combinations, longer delays, and longer uninterrupted lifetimes. Declare the held-out condition for H5.
  - Verify: Keep one-hot input dimension fixed for a frozen network. Hash suite manifests and ensure namespace disjointness. Development rehearsals use their own namespaces and do not inspect the reserved final-test results.

- [ ] **M8-05 - Implement audited offline aggregation**
  - Deliver: Extend analysis/validate_logs.py and implement analysis/aggregate.py to join ordinary and hidden streams by explicit keys, compute Section 14 metrics, retain failures/censoring, and write versioned derived outputs.
  - Verify: Synthetic fixtures have hand-checked reward/accuracy/regret, counts, acquisition, and recovery values. Duplicate/missing records fail audit or appear as explicit missingness; no silent zero filling or selective lifetime omission.

- [ ] **M8-06 - Implement paired outer-seed uncertainty analysis**
  - Deliver: Average evaluation lifetimes within each outer seed, compute matched condition differences, and summarize across independent outer seeds. Use a declared outer-seed bootstrap or paired analysis; optional hierarchical resampling must preserve nesting.
  - Verify: Fixtures demonstrate that adding correlated ticks/rewards/lifetimes from one genome does not create more independent discoveries. Report effect sizes, intervals, per-seed values, and analysis RNG/configuration.

- [ ] **M8-07 - Run the motor-afferent-only plasticity diagnostic**
  - Deliver: Compare full recurrent plasticity with incoming-motor-edge plasticity under matched learning rule/task settings and declared tuning. Keep this separate from the main mask-matched gate comparison.
  - Verify: If motor-afferent learning matches the full model, report that this task has not established a need for plastic internal recurrent computation. Do not hide the result as an inconvenient control.

- [ ] **M8-08 - Rehearse primary suites on development/validation data**
  - Deliver: Run fresh-lifetime comparisons, noise/hazard grid, isolated changes, and long-life tests using rehearsal suites. Generate preliminary acquisition, reversal, and generalization tables/figures.
  - Verify: Every selected genome comes from the declared validation procedure. Report all outer seeds, failures, baseline choices, and measured compute. Do not label rehearsal results untouched final evidence.

- [ ] **M8-09 - Plan confirmation replicates using pilot variability and cost**
  - Deliver: Use the spec's 3-5 exploratory outer seeds and initial suggestion of 10 or more confirmation seeds as planning guidance, not universal requirements. Declare a minimum effect of interest, precision goal, feasible budget, and stopping rule.
  - Verify: The number of lifetimes from one genome is not substituted for outer-seed replication. Do not stop at the most favorable interim result. Document any smaller feasible study and its uncertainty honestly.

- [ ] **M8-10 - Make an explicit model-scaling and extra-ablation decision**
  - Deliver: Use measured throughput to decide whether 60 actor neurons plus 4 modulators are justified or the small study should remain primary. Consider adaptation strength 0 versus a separately named nonzero ablation only after the baseline is established.
  - Verify: The decision, actor/modulator counts, search space, dynamic-state count, and any ablation budgets are recorded. A larger model or new mechanism does not silently replace the previously characterized comparison.

- [ ] **M8-11 - Draft the required report and representative-example policy**
  - Deliver: Create reproducible scripts for acquisition/continuous performance, reversal behavior, noise-by-hazard results, intervention result placeholders, and mechanism traces. Declare how ordinary, representative, and failure examples will be selected.
  - Verify: Figures/tables state condition, units, sample counts, uncertainty, and censoring where applicable. A report template has no fabricated results; missing intervention/final-study outputs are clearly pending.

- [ ] **M8-12 - Freeze comparison choices for intervention rehearsals**
  - Deliver: Record primary targeted-versus-independently-evolved-global regret comparison, secondary recovery/unchanged-cue measures, proposed mechanism tests, and held-out generalization condition, or document explicit development-stage alternatives.
  - Verify: Analysis choices are ready to freeze before final test. Changes during M9 development must be recorded and never informed by reserved test results. Save a runnable evaluation and analysis rehearsal bundle.

- [ ] **M8-GATE - Verify and record milestone exit**
  - All primary comparison runners, baseline/config audits, metrics, outer-seed analysis, and fresh-lifetime rehearsal suites work; the report includes every outer seed/failure and budget; the claim track and final analysis choices are explicit. Final-test results remain uninspected while M9 tooling is validated.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M9 - Test the mechanism causally

**Spec:** Sections 14.5-14.7, 15, 16/M9, 20.4, 24.  
**Prerequisite:** M8-GATE.

Verify branch-from-checkpoint interventions and their controls, then rehearse the causal analyses without confusing performance, correlation, and mechanism.

- [ ] **M9-01 - Build the intervention branch harness**
  - Deliver: Choose checkpoint events in advance, clone full state into exact-copy control/intervention branches, and pair future exogenous schedules. Assign branch IDs and intervention metadata without altering ordinary observations.
  - Verify: Copy-versus-copy branches match exactly on the reference platform. Changing the intended mechanism may diverge activity/actions; do not force trajectories to remain identical afterward.

- [ ] **M9-02 - Implement freezing of future plasticity**
  - Deliver: Suppress application of P updates while preserving current P, recurrent/adaptation/modulation/motor state, and sensory feedback. Declare whether eligibility keeps evolving and prohibit undefined unfreezing.
  - Verify: P stays unchanged through probe outcomes while permitted state continues evolving. Baseline and feedback bookkeeping remain exactly once. Measure stable retention separately from adaptation to new changes.

- [ ] **M9-03 - Implement weight erasure, shock control, and weight transfer**
  - Deliver: Erase P in one branch, retain an exact-copy control, and add a declared norm-matched random weight perturbation control. Transfer learned P into a fresh dynamic state of the same inherited actor for the complementary test.
  - Verify: Refresh effective-weight caches correctly. Declare settling/probe windows and distinguish immediate dynamical shock from retained useful information. Random control generation has its own evaluator RNG stream.

- [ ] **M9-04 - Implement transient-state reset probes**
  - Deliver: At a suitable quiet interval, reset h/a/z/q while preserving P; clear E and freeze future plasticity during the memory-location probe. Preserve consistent environment phase and visible action history.
  - Verify: Cached gates/derived state are consistent with the intervention. No pending reward is lost or reassigned. Interpret together with weight erasure/transfer, not as a complete memory partition from one destructive reset.

- [ ] **M9-05 - Implement validation-calibrated constant gates**
  - Deliver: Estimate one global mean gate and each receiver's own mean gate on validation lifetimes, then clamp to those constants in probes. Include a separately tuned constant-rate comparator selected on development data.
  - Verify: Constants are never estimated from final-test outcomes. Tests distinguish removal of both spatial/temporal variation from removal of temporal variation only. Compare actual updates as well as gates.

- [ ] **M9-06 - Implement spatial gate reassignment**
  - Deliver: Route the generated gate vector through a fixed seeded receiver permutation at a checkpoint while leaving modulator computation and actor topology unchanged. Predeclare several permutation seeds.
  - Verify: Each event's multiset of gate values is unchanged, but receiver assignment changes. Identity permutation matches control. Report necessity in the trained individual, not universal superiority of one spatial arrangement.

- [ ] **M9-07 - Implement event-indexed temporal gate controls**
  - Deliver: Support recorded gate replay with a fixed circular event shift and online delayed gates. Include a frozen no-shift replay control; specify recorded sequence source, alignment, delay, and initial-buffer policy in configuration.
  - Verify: Shifts/delays are chosen without inspecting final-test damage. Separate open-loop replay effects from mistiming. Label replay offline and any boundary/alignment choice explicitly; do not present future-recorded gates as an ordinary online controller.

- [ ] **M9-08 - Implement the mean-gate global control**
  - Deliver: Replace the targeted vector by its across-receiver mean at each event. Preserve the modulator and apply the resulting scalar through the common update implementation.
  - Verify: The mean gate matches exactly, but unequal eligibility norms may change total update magnitude. Tests and analysis must not describe this as a sufficient magnitude-matching control.

- [ ] **M9-09 - Implement the raw-L1-update-matched global control**
  - Deliver: At the identical branch state compute b=abs(eta*delta*E) per plastic edge and scalar g=sum(b*targeted_gate)/sum(b), or zero when the denominator is zero. Keep this global eligibility access evaluator-only.
  - Verify: Tests prove equality of unclipped L1 update magnitude at that state. Log raw and actual norms; clipping/offset bounds can break actual equality. Report all events and a predeclared clean nonclipping analysis separately.

- [ ] **M9-10 - Implement suppression of one misleading-outcome update**
  - Deliver: At an evaluator-selected misleading event, deliver the same reward input to both branches but skip only that event's synaptic update in one. Use a declared short subsequent no-learning probe.
  - Verify: Baseline and sensory feedback still advance consistently in both branches. This separates the synaptic update's behavioral effect from the sensory effect of the misleading reward; not every nonzero update is automatically called damage.

- [ ] **M9-11 - Implement paired unchanged-cue interference branches**
  - Deliver: Compare continuation after one designated hidden change with a matched no-change continuation, retaining cue sequence and random schedules. Probe unchanged cues using the declared measurement policy.
  - Verify: Only the intended hidden mapping differs at branch creation. The agent receives no branch/change label, and unchanged-cue performance is measured with aligned exposures and explicit probe counts.

- [ ] **M9-12 - Run intervention regression and development rehearsals**
  - Deliver: Test every intervention in small hand-checkable states, then run prespecified validation-selected genomes on fresh development/validation lifetimes, including ordinary and unsuccessful outer seeds.
  - Verify: Each control has a verified identity/no-op case where applicable. Keep acute interventions distinct from separately trained restricted models. A failed mechanism test is retained, not replaced by a favorable individual.

- [ ] **M9-13 - Generate causal-effect summaries and finalize protocols**
  - Deliver: Implement analysis/plot_interventions.py and mechanism traces with activity, eligibility, applied gates, raw/actual updates, and behavior. Save branch configs, checkpoint references, and paired outer-seed effects.
  - Verify: The report separates performance advantage from support for timing/targeting, static spatial bias, learning-rate scale, or other simpler explanations. All intervention parameters and analysis choices are ready for the final-study freeze.

- [ ] **M9-GATE - Verify and record milestone exit**
  - Branch replay and all specified interventions/controls are tested; pilot mechanism results include ordinary/failure cases and appropriate uncertainty; simpler explanations are explicitly considered. The intervention toolkit and protocol are frozen before final-test execution. A causal null is acceptable; an unverified intervention is not.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## M10 - Package the research result

**Spec:** Sections 14.8-14.12, 16/M10, 20, 24-25.  
**Prerequisite:** M9-GATE; additionally B7-GATE if the broader claim track is selected.

Lock the analysis, execute the untouched final study once under the declared protocol, and archive a reproducible result with defensible limitations.

- [ ] **M10-01 - Lock the final study plan and artifacts**
  - Deliver: Freeze code/configuration identifiers, primary/secondary outcomes, comparisons, selected-genome procedure, budgets, outer seeds, suite hashes, intervention parameters, censoring/failure policy, and representative-example rule.
  - Verify: All preceding gates pass. Final-test results have not informed these choices. Any remaining uncertainty or reduced scope is explicit; the broader claim track also has the matched B7 evidence.

- [ ] **M10-02 - Complete the declared independent search/selection replicates**
  - Deliver: Execute any remaining prespecified confirmation searches within the agreed budget, select genomes using validation only, and archive all populations/selections/failures and compute counts.
  - Verify: The run count follows the declared stopping rule, not favorable interim differences. This is not permission to launch an unbounded campaign. Log any infeasible or interrupted portion as incomplete.

- [ ] **M10-03 - Run the untouched final evaluation and intervention suites**
  - Deliver: Evaluate frozen selected genomes with fresh births, ordinary learning active where specified, and the reserved test manifests. Execute the predeclared branch interventions and held-out condition without tuning from their outcomes.
  - Verify: All expected jobs have explicit completion/failure status. No test seed overlaps other namespaces. Inspection ends the untouched status of that suite; later changes require a new exploratory cycle and new independent confirmation data.

- [ ] **M10-04 - Audit final raw data before aggregation**
  - Deliver: Validate event accounting, no resets/leakage, finite values, bounds, seeds, manifest/code hashes, realized changes/noise, lifetime counts, and failure records. Reproduce selected complete checkpoint continuations.
  - Verify: An audit failure is investigated and disclosed. Do not fill missing values with convenient scores, omit failed candidates, or combine incompatible schemas as if they were one study.

- [ ] **M10-05 - Generate the five required figure groups or equivalent tables**
  - Deliver: Produce acquisition/continuous performance; changed/unchanged-cue reversal behavior; noise-by-hazard performance; causal intervention effects across outer seeds; and representative plus failure mechanism traces.
  - Verify: Scripts reproduce figures from archived raw/derived data. Show observed reward and latent accuracy/regret, effect sizes/intervals, sample counts, censoring, and the predeclared example-selection rule.

- [ ] **M10-06 - Write the result-to-claim interpretation**
  - Deliver: Address H1-H5 separately with evidence, null results, limitations, and controls. Distinguish activity-mediated memory, synaptic adaptation, inherited specialization, gate timing, spatial assignment, and update-scale explanations.
  - Verify: Use Section 24 claim limits. Do not assert general intelligence, a realistic brain, biological credit-assignment equivalence, universal superiority, or publication novelty from architectural ingredients alone.

- [ ] **M10-07 - Assemble the reproducibility archive and README**
  - Deliver: Archive source revision plus dirty-tree evidence when applicable, lockfiles/toolchain, resolved configs, all seed manifests, selected genomes, representative complete checkpoints, event/hidden streams, candidate failures, metrics, figures, and decisions/experiment logs.
  - Verify: README gives actual implemented commands and real artifact paths. Separate raw evidence from derived outputs. Checksums identify the archive; no private credentials or unrelated local files are included.

- [ ] **M10-08 - Verify clean reconstruction and close the core tracker**
  - Deliver: From a clean working environment, reproduce at least a small reference simulation, a checkpoint continuation, log audit, and report-generation path using the pinned environment. Record what was and was not rerun.
  - Verify: Update every core task and gate from evidence, leave deferred extensions unchecked, and write a final handoff. Choose any next research axis only after the core result is interpretable.

- [ ] **M10-GATE - Verify and record milestone exit**
  - All mandatory tasks for the explicitly frozen core scope have verified evidence; study outputs, failures, analysis, and limitations are archived; reproducibility checks pass. If execution is incomplete, leave this gate unchecked and record the completed subset and blocker. Valid negative results do not prevent completion. Extensions remain separate work.
  - Evidence: add a ledger entry with the executed commands, results, configuration/seed identifiers, and saved artifact paths. Update the current status before beginning the next milestone.

---

## Conditional B7 track - stronger activity-only comparison

**Spec:** Sections 11.4, 12.10, 13.1, 13.5-13.6, 16/M8, and 24.  
**Prerequisite:** M8 comparison infrastructure and an explicit broader-claim decision. Complete this track before M10 final selection/evaluation when that claim is in scope.

**Initial status: deferred, not completed.** These checkboxes are outside the initial core milestone gates. A `family_only` report must explicitly state that the stronger comparator was not established. Do not count deferred work as done.

- [ ] **B7-01 - Authorize and freeze the expanded comparison**
  - Deliver: After the M8 claim-track decision, define the same inherited actor parameterization and optimization opportunity for activity-only and plastic conditions. Predeclare budgets, validation selection, and both capacity variants from Section 13.6.
  - Verify: The expanded study is a separately named scope, not a hidden replacement for the fixed-actor gate study. Preserve and report the earlier gate-only result.

- [ ] **B7-02 - Implement separately optimized activity-only agents**
  - Deliver: Keep P fixed at zero throughout all lifetimes while allowing inherited actor/input parameters to be optimized to use cue/action/reward history. Support same-actor-count and same-total-neuron-count variants where the approved study calls for them.
  - Verify: Ordinary recurrent memory is available. Unused modulator units can become ordinary recurrent units only in the explicitly named total-neuron comparator. Report neuron and full dynamic-state counts; neither variant automatically matches synaptic memory capacity.

- [ ] **B7-03 - Implement matched joint-search plastic conditions**
  - Deliver: Give fixed/global/targeted plastic models the same actor-search parameterization and opportunity for optimization as the activity-only comparator, while retaining their explicitly different learning mechanisms.
  - Verify: Pair initializations/task suites and report search dimensions, candidate-lifetime/tick budgets, validation and tuning effort. Failure to optimize one family is recorded as a limitation, not proof that its mechanism cannot work.

- [ ] **B7-04 - Run and integrate the expanded comparison evidence**
  - Deliver: Train/select the approved models using training/validation only, rehearse matched analysis, and include the expanded study in the frozen final protocol before any reserved-test inspection.
  - Verify: Report every outer seed, failure, budget, performance interval, and capacity caveat. Do not reuse test-informed settings as untouched confirmation.

- [ ] **B7-GATE - Verify the broader-claim comparator**
  - Separately optimized activity-only and matched joint-search plastic conditions have verified implementations, declared comparable optimization opportunities, validation-selected genomes, and an auditable analysis plan/evidence. State remaining capacity and search limitations.
  - Evidence: record the expanded study separately and link it in the final report. Without this gate, keep claims within the initial locally plastic actor family.

---

## Deferred extensions - do not begin automatically

**Prerequisite:** M10-GATE and an explicit next-direction decision. These entries summarize spec Section 23, not requirements for the first result. Pick one major axis at a time, write its separate contract, and rerun the relevant controls. Leave unselected entries unchecked.

- [ ] **X-01 - Remove explicit perturbation access.** Reproduce a separately sourced local rule faithfully before replacing the Gaussian score; hold the task and gate study fixed during the comparison. Source: Section 23.1.
- [ ] **X-02 - Add outcome-responsive gates.** Use stored pre-feedback eligibility/reward/baseline and one explicitly delayed update; exclude reward-processing scores from stored credit. Compare against pre-outcome and simple reward-dependent scalar gates. Source: Sections 8.4 and 23.2.
- [ ] **X-03 - Add an internal teaching-signal mechanism.** Specify its own learning rule and observable information boundary; keep fitness grounded in real environmental rewards. Source: Section 23.3.
- [ ] **X-04 - Permit multiple unresolved choices.** Specify the new credit-assignment protocol without silently giving a reward-to-cue pointer to the agent. Source: Section 23.4.
- [ ] **X-05 - Use overlapping fixed-dimensional cue representations.** Control norms/similarities and separate representation changes from modulation changes; do not claim perceptual abstraction from random-vector memorization. Source: Section 23.5.
- [ ] **X-06 - Introduce spiking dynamics.** Derive or faithfully reproduce an appropriate learning rule; do not copy the Gaussian membrane score into a changed transition model. Source: Section 23.6.
- [ ] **X-07 - Add a small embodied task.** Reestablish simple action/reward/baseline contracts before attributing success to modulation. Source: Section 23.7.
- [ ] **X-08 - Evolve topology or add structural growth.** Separate inherited topology from within-lifetime change and reestablish matched search controls. Source: Section 23.8.

## Contract and test coverage index

Use this index when modifying a cross-cutting feature. The cited tasks identify where coverage is first established; later changes must keep the corresponding tests passing.

| Spec requirement | Primary work items |
| --- | --- |
| Section 17.1: environment unit tests | M0-06 through M0-12 |
| Source/receiver orientation, old-state actor updates | M1-01 through M1-06; M2-01 |
| Section 17.2: neural/plasticity invariants | M1-03 through M1-07; M3-01 through M3-03; M4-01 through M4-04; M6-01 through M6-05 |
| Section 17.3: golden score/eligibility/update | M3-03 |
| Section 17.4: conditional fixed-sample derivative | M2-02 |
| Section 17.5: one-neuron closed-form direction | M2-03 |
| Section 17.6: finite-horizon recurrent derivative | M2-04 through M2-06 |
| Section 17.7: replay, logging, dense/sparse, serial/parallel | M1-09 through M1-10; M3-10; M4-06; M6-05 through M6-06; M7-08, M7-12, M7-13 |
| Section 17.8: statistical negative controls | M3-05, M3-09, M5-07 |
| Sections 5/8/9: information and feedback ordering | M0-06 through M0-11; M4-01; M6-04 through M6-05 |
| Sections 10/11: birth state versus inherited genome | M1-09; M3-01, M3-10; M4-03, M4-06; M7-01 |
| Sections 12/13: search, fairness, baselines | M5-03 through M5-04; M5-08 through M5-11; M7; M8-02; B7 track |
| Section 14: metrics, replication, generalization | M5-05 through M5-07; M8; M10-01 through M10-06 |
| Section 15: intervention mechanisms and controls | M9-01 through M9-13 |
| Sections 18-20: CLI/configuration/logging/checkpoints | M0-02 through M0-06; M0-13 through M0-14; M1-09; M3-10; M7-07 through M7-08; M8-05 |
| Section 22: compute and optimization budgets | M5-10; M7-09, M7-12, M7-13; M8-09 through M8-10; M10-02 |
| Section 24: honest interpretation and claim scope | M7-14; M8-03; M9-13; M10-06; B7-GATE |

## Initial command milestones

The exact CLI may be refined and documented during implementation. [README.md](README.md) lists executable commands; the following table maps current and future command work to milestones. `runs/example` and similar paths in the spec are placeholders, not files already present.

| Stage | Runnable artifact to establish |
| --- | --- |
| M0 | `cargo test`; `validate-config`; environment-only `simulate`; minimal log audit |
| M1 | Nonplastic actor `simulate`; full-state checkpoint/resume test |
| M2 | Deterministic score tests and bounded Monte Carlo diagnostic command |
| M3 | Explicitly episodic clean-learning diagnostic and paired controls |
| M4 | `simulate --config configs/continuous_stationary.toml --seed 1` |
| M5 | `benchmark --config configs/mixed_fixed.toml`; baseline sweep and metrics scripts |
| M6 | Matched gate-condition configurations and gate arithmetic regression tests |
| M7 | `evolve --config <small-search-config> --outer-seed 1`; resumable search |
| M8 | `evaluate --genome <actual-genome-path> --suite <rehearsal-manifest>`; aggregation/plots |
| M9 | `intervene --checkpoint <actual-checkpoint-path> --config configs/interventions.toml` |
| M10 | Frozen final evaluation, validated analysis, and archive reconstruction commands |

Run Rust CLI subcommands through `cargo run --release --locked -- ...` after bootstrapping the lockfile. Seed namespace resolution must be explicit in configuration/CLI/manifest and saved with every run. The example seed `1` is not a substitute for the full seed policy. Python commands use the pinned project environment.

## Completion evidence ledger - append, do not fabricate

The entries below are historical evidence and remain append-only. Add one entry per coherent verified change/run; an entry may cover several tightly related task IDs. The template below is not an actual result.

```text
Date / agent or session:
Task IDs:
Spec sections:
Change and affected files:
Code revision / dirty-tree state:
Commands actually executed:
Outcome and checks passed:
Checks not run / failures / blockers:
Configuration and suite hashes:
Seed namespace / outer seeds / lifetime count:
Artifact paths and checksums where relevant:
Interpretation and claim limits:
Tracker boxes updated:
Next eligible task:
```

For a code-only task, mark experiment-specific fields not applicable with a reason. For empirical tasks, include actual measured values and uncertainty rather than "looks good." Link fuller run records from `docs/experiments.md`; log scientific ambiguities and approved changes in `docs/decisions.md`.

### Ledger entries

```text
Date / agent or session: 2026-09-21 / agent (M0 bootstrap session)
Task IDs: M0-01
Spec sections: 16/M0 (tracker setup); AGENTS.md workflow
Change and affected files: None (read-only). Inspected root: AGENTS.md
  (188 lines), spec.md v0.1 prepared 2026-09-20 (2515 lines, root filename
  confirmed, no upload suffix), to-do.md (798 lines), Cargo.toml (stub:
  name cra 0.1.0, edition 2024, no deps), src/main.rs (hello-world stub),
  .gitignore (/target only). spec.md left byte-identical.
Code revision / dirty-tree state: f261794 (initial commit) clean at inspect.
Commands actually executed: git status/log, rustc/cargo --version,
  ls -la, reads of AGENTS.md/to-do.md/spec.md/Cargo.toml/src.
Outcome and checks passed: Initial state recorded; spec preserved.
Checks not run / failures / blockers: None.
Configuration and suite hashes: N/A (no configs yet).
Seed namespace / outer seeds / lifetime count: N/A.
Artifact paths and checksums where relevant: N/A.
Interpretation and claim limits: No implementation claimed. AGENTS.md needed
  no scientific-contract edit; it already marks its commands as targets.
Tracker boxes updated: Status block + ownership (this entry covers M0-01).
Next eligible task: M0-02 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 bootstrap session)
Task IDs: M0-02
Spec sections: 18 (repo/interface), 20 (run identity); AGENTS.md toolchain rule
Change and affected files: rust-toolchain.toml (new, channel 1.98.0);
  Cargo.toml (lib cra + bin cra, ordinary deps only: clap/serde/serde_json/
  toml/thiserror/rand/rand_chacha/rand_core/sha2; no DL framework);
  Cargo.lock (new, committed); src/lib.rs + src/main.rs (thin clap CLI:
  validate-config, scaffold simulate). No future neural/evolution modules.
Code revision / dirty-tree state: base f261794; new files untracked at run
  time (manifest records git_dirty=true honestly).
Commands actually executed:
  cargo generate-lockfile
  cargo test (4 lib tests ok)
  cargo run --locked -- --help (CLI help ok)
Outcome and checks passed: Minimal unit tests pass; CLI help runs. Actual
  versions: rustc/cargo 1.98.0, clap 4.6.7, serde 1.0.229, toml 0.8.23,
  rand 0.9.5, rand_chacha 0.9.0, sha2 0.10.9 (per Cargo.lock).
Checks not run / failures / blockers: clippy/fmt deferred to final pass
  (passed there). No failures.
Configuration and suite hashes: N/A.
Seed namespace / outer seeds / lifetime count: N/A.
Artifact paths and checksums where relevant: Cargo.lock (committed).
Interpretation and claim limits: Scaffold only; no simulator behavior.
Tracker boxes updated: M0-02 checked.
Next eligible task: M0-03 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 bootstrap session)
Task IDs: M0-03
Spec sections: 18.1 (layout), 20.1 (manifest), 20.7 (notebook log)
Change and affected files: README.md (new: implemented vs planned commands,
  pinned versions, run-dir/artifact policy); docs/decisions.md,
  docs/experiments.md (new, append-only); configs/ empty at this step;
  manifests/development.json, validation.json, final_test.json (disjoint
  outer-seed ranges 1-9999 / 10001-19999 / 90001-99999);
  analysis/README.md + requirements.txt (Python 3.14.7 pinned, no deps yet);
  runs/.gitkeep; .gitignore extended (/target, /runs/* except .gitkeep,
  __pycache__/, *.pyc, .venv/, *.tmp); src/run.rs (unique run dirs,
  manifest.json + resolved_config.toml + seed_streams.json writers).
Code revision / dirty-tree state: base f261794; scaffold untracked
  (dirty=true in manifests).
Commands actually executed: file creation; verified by simulate run writing
  runs/env_smoke-root1-outer1-1789960343/ (manifest.json,
  resolved_config.toml, seed_streams.json) and `ls runs/`.
Outcome and checks passed: Run dirs unique per profile/seeds/time and
  git-ignored; decision/experiment records exist; README separates
  implemented (validate-config, scaffold simulate) from planned
  (benchmark/evolve/evaluate/intervene/analysis scripts).
Checks not run / failures / blockers: None.
Configuration and suite hashes: N/A (no suites consumed).
Seed namespace / outer seeds / lifetime count: Manifest ranges reserved, none
  consumed.
Artifact paths and checksums where relevant:
  runs/env_smoke-root1-outer1-1789960343/{manifest.json,resolved_config.toml,
  seed_streams.json} (local, git-ignored).
Interpretation and claim limits: Conventions only; decision docs are workflow
  additions, not scientific requirements.
Tracker boxes updated: M0-03 checked.
Next eligible task: M0-04 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 bootstrap session)
Task IDs: M0-04
Spec sections: 5.8 (stream separation), 20.5 (seed derivation)
Change and affected files: src/rng.rs (canonical
  "cra-v1|root=|ns=|outer=|lifetime=|stream=" string, SHA-256 ->
  ChaCha8Rng, strict namespace validation, reserved stream table, one RNG
  per stream); tests/seed_streams.rs (4 golden vectors, disjointness,
  10k-draw independence, rejection tests).
Code revision / dirty-tree state: base f261794; scaffold untracked.
Commands actually executed:
  cargo test --all-targets --locked (5/5 seed_streams tests pass)
  printf '%s' 'cra-v1|root=1|ns=development|outer=1|lifetime=0|stream=cue_order' | sha256sum
    -> da2a722c... (matches implementation golden exactly; independent tool)
Outcome and checks passed: Golden fixtures pass; dev/training/validation/
  final_test namespaces derive pairwise-distinct seeds; all 9 reserved
  streams distinct; 10k actor_noise draws leave cue_order RNG untouched;
  unknown namespaces and malformed stream names rejected. No
  runtime-randomized hashes (only SHA-256 + ChaCha8).
Checks not run / failures / blockers: None.
Configuration and suite hashes: N/A.
Seed namespace / outer seeds / lifetime count: Goldens at
  (root 1, development|training, outer 1, lifetimes 0-1); no lifetimes run.
Artifact paths and checksums where relevant: tests/seed_streams.rs.
Interpretation and claim limits: Derivation contract only; no schedule
  content (cue/mapping/timing producers arrive M0-07).
Tracker boxes updated: M0-04 checked.
Next eligible task: M0-05 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 bootstrap session)
Task IDs: M0-05
Spec sections: 19 (profiles), 19.4 (validation), 4.3 (dt=1)
Change and affected files: src/config.rs (schema_version=1 schema with
  required simulation/environment/logging/seeds + optional
  actor/learning/modulator/evolution, deny_unknown_fields everywhere, full
  19.4 validation plus dimension/score-noise/reset-trace/evolution checks);
  src/run.rs + src/main.rs (validate-config, scaffold simulate with
  --seed alias and per-field config/cli source recording);
  configs/env_smoke.toml (env-only smoke), configs/debug_stationary.toml
  (spec 19.2 + [seeds]); tests/config_validation.rs (19-case table).
Code revision / dirty-tree state: base f261794; scaffold untracked.
Commands actually executed:
  cargo run --locked -- validate-config configs/env_smoke.toml -> OK
  cargo run --locked -- validate-config configs/debug_stationary.toml -> OK
  cargo run --release --locked -- validate-config configs/debug_stationary.toml -> OK
  cargo run --locked -- simulate --config configs/env_smoke.toml --seed 1 --outer-seed 1
    -> runs/env_smoke-root1-outer1-1789960343/
  negative: bad schema_version/missing file -> ERROR, exit 1 (both)
  cargo fmt --all -- --check -> clean
  cargo clippy --all-targets --locked -- -D warnings -> clean
  cargo test --all-targets --locked -> 11/11 pass (4 lib + 2 config + 5 seeds)
Outcome and checks passed: All 19 invalid-config cases rejected with the
  expected error class (dt, noise, hazard, timing, pending, schema,
  seed-ns, parse, actor-dims, score-noise, reset-trace, evolution,
  modulator x2, decay). Env smoke profile runs no actor code by
  construction (no actor sections parsed; simulate writes provenance only).
Checks not run / failures / blockers: Full gates run (fmt/clippy/test);
  no failures. M0-06+ suites (environment_contract/event_order/leakage)
  not yet implemented - out of scope for M0-05.
Configuration and suite hashes: configs/env_smoke.toml + debug_stationary.toml
  validated (resolved copies in run dir).
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetime_index 0 streams only; no lifetimes simulated.
Artifact paths and checksums where relevant:
  runs/env_smoke-root1-outer1-1789960343/{manifest.json,resolved_config.toml,
  seed_streams.json} (local, git-ignored).
Interpretation and claim limits: Config/seed contracts only. No environment
  stepping, no neural code, no learning claim.
Tracker boxes updated: M0-01 through M0-05 checked.
Next eligible task: M0-06.
```

```text
Date / agent or session: 2026-09-21 / agent (environment core session)
Task IDs: M0-06
Spec sections: 5.5-5.6 (observable/forbidden inputs), 18.2-18.3 (boundary
  types), 20.2 (separate ordinary/hidden streams)
Change and affected files: src/environment/observation.rs (new: Feedback,
  Observation, MotorOutput, SimError with 4 sketch variants + 6 explicit
  driver variants, Agent trait, feature_dim = K + 6);
  src/environment/hidden_state.rs (new: HiddenState, CueRole,
  HiddenAnnotation, evaluator-only accessors); tests/leakage.rs (new: 4
  boundary tests); src/environment/mod.rs re-exports boundary types.
Code revision / dirty-tree state: base 10a7ae8 (M0-01–M0-05); new files
  untracked, src/lib.rs + README.md + docs/decisions.md modified.
Commands actually executed:
  cargo test --all-targets --locked (leakage 4/4 pass)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
Outcome and checks passed: Ordinary types serialize with exactly their
  public keys; hidden annotations serialize separately (join key event_id
  only); event ids travel only in the feedback envelope with K + 6 wide
  features; a recording Agent driven on a full 6-outcome lifetime receives
  no hidden markers. SimError covers duplicate/unknown feedback,
  nonfinite state, checkpoints, and driver misuse explicitly.
Checks not run / failures / blockers: None. Feature content population is
  M0-09 (vectors are width-correct zeros, labeled scaffolding).
Configuration and suite hashes: N/A (code-only task; smoke configs
  re-validated via CLI).
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetime 0; one 6-outcome smoke lifetime driven in leakage tests.
Artifact paths and checksums where relevant: tests/leakage.rs.
Interpretation and claim limits: Boundary contract only. The Agent trait has
  no hidden-state parameters by construction; agent implementations arrive
  in M0-10/M1.
Tracker boxes updated: M0-06 checked.
Next eligible task: M0-07 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (environment core session)
Task IDs: M0-07
Spec sections: 5.1-5.4 (mappings/noise/volatility/cues), 5.7 (event
  sequence), 10.4 (birth), 17.1 (hazard items)
Change and affected files: src/environment/schedule.rs (new: Phase,
  PhaseState, exact inclusive timing draws); src/environment/hidden_state.rs
  (birth sampling, present() with per-exposure hazard); src/environment/
  mod.rs (Lifetime::new: membership shuffle on init, warmup-first quiet,
  fixed per-cycle draw order); tests/environment_contract.rs +
  tests/support/mod.rs (new: table-driven schedule tests).
Code revision / dirty-tree state: base 10a7ae8; new files untracked.
Commands actually executed:
  cargo test --all-targets --locked (contract schedule/hazard tests pass)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
Outcome and checks passed: Exact tick counts verified for gap [0,0]
  (response starts immediately, 17-tick cycles) and gap [2,2] (21-tick
  shape); hazard 0 never flips over 6 cycles; hazard 1 flips every repeat
  but not the first (unit + lifetime level); mappings invariant across
  quiet spans; sampled lengths stay in configured ranges; inverted ranges
  are explicit errors. Timing stream order (quiet, cue, gap per cycle) is
  fixed and documented.
Checks not run / failures / blockers: None. During development two test
  expectations were wrong, not the implementation: hazard-1 alternation
  (third presentation flips back to birth) and a loop range that included
  the feedback tick. Both corrected in tests with the alternation asserted
  explicitly; no source change resulted.
Configuration and suite hashes: Programmatic test configs derived from
  configs/env_smoke.toml values (validated before birth).
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetime 0; up to 6-outcome lifetimes per test.
Artifact paths and checksums where relevant: tests/environment_contract.rs,
  tests/support/mod.rs.
Interpretation and claim limits: Scheduling/hazard contracts only. Full
  factorial noise/volatility counterbalancing is deferred to M5-02
  (recorded in docs/decisions.md); M0 assignment cycles config lists.
Tracker boxes updated: M0-07 checked.
Next eligible task: M0-08 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (environment core session)
Task IDs: M0-08
Spec sections: 5.1 (reward at commit), 5.8 (commitment), 9.4 (delay
  accounting), 17.1 (reward/count items)
Change and affected files: src/environment/mod.rs (commit,
  commit_with_noise diagnostic path, PendingReward stored at commit,
  due_tick = commit_tick + delay, exactly-once delivery in advance(),
  note_feedback_consumed ledger, counts/is_complete); tests/
  environment_contract.rs (reward/commit/count tests).
Code revision / dirty-tree state: base 10a7ae8; new files untracked.
Commands actually executed:
  cargo test --all-targets --locked (full suite 38/38 pass: 14 lib + 2
    config + 13 contract + 4 leakage + 5 seeds)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo run --locked -- validate-config configs/env_smoke.toml (OK)
  cargo run --locked -- validate-config configs/debug_stationary.toml (OK)
  cargo run --locked -- simulate --config configs/env_smoke.toml --seed 1
    --outer-seed 1 (run dir runs/env_smoke-root1-outer1-1789961883/)
Outcome and checks passed: Delay 1 delivers next tick; delay 3 delivers at
  commit+3 with two ordinary delay ticks (commit 17 -> feedback 20);
  correct/wrong score 1/0 at zero noise; forced noise bit reverses either
  reward (all 4 combos); pending reward equals delivered reward and reads
  the current mapping each cycle; double commit / action 2 / advance
  without commit rejected; event ids 0,1,2,3 unique and consumed once;
  completed lifetimes have equal counts and no pending reward; duplicate
  and unknown consumption rejected without state change; 5000
  actor_noise-stream draws leave the schedule bitwise identical.
Checks not run / failures / blockers: None. Outcome-present/latch feature
  items and the tick-20/delay-3 fixture belong to M0-09/M0-11.
Configuration and suite hashes: configs/env_smoke.toml,
  configs/debug_stationary.toml validated via CLI.
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetime 0 (plus lifetime 3 schedule-divergence check); <= 6 outcomes per
  test lifetime.
Artifact paths and checksums where relevant: tests/environment_contract.rs;
  runs/env_smoke-root1-outer1-1789961883/{manifest.json,resolved_config.toml,
  seed_streams.json} (local, git-ignored).
Interpretation and claim limits: Commitment/reward contracts only. No
  agent, no learning, no feature content yet.
Tracker boxes updated: M0-06 through M0-08 checked.
Next eligible task: M0-09.
```

```text
Date / agent or session: 2026-09-21 / agent (features/baselines/fixtures session)
Task IDs: M0-09
Spec sections: 5.5 (observable channels), 9 step 9d (latch), 17.1
  (outcome-present, latch items)
Change and affected files: src/environment/mod.rs (K + 6 feature builder
  in advance(): one-hot cue + cue-present on cue ticks, go on response,
  outcome-present/value on the single feedback tick, previous-action latch
  from stored last_action); tests/environment_contract.rs (+4 tests).
Code revision / dirty-tree state: base 10a7ae8 (M0-01–M0-08); environment
  module + tests untracked/modified, spec.md untouched.
Commands actually executed:
  cargo test --all-targets --locked (contract suite 17/17 incl. 4 new)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
Outcome and checks passed: Exact per-tick feature vectors for a full
  cycle (quiet zeros, one-hot+present, go-only, feedback channels);
  reward 0 delivered as present=1/value=0, never as absent; latch empty
  through the commitment tick and flipped from the next tick on (both
  directions across two cycles); cue channels live only during Cue and
  outcome-present fires exactly once per outcome (16 cue ticks, 2 feedback
  ticks over 2 cycles).
Checks not run / failures / blockers: One test-side arithmetic slip
  (loop range included the driver's own first feedback in the count) was
  corrected by driving all cycles in a single loop; implementation
  untouched. None otherwise.
Configuration and suite hashes: Programmatic configs from base_config
  (validated before birth).
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetime 0; <= 6 outcomes per test lifetime.
Artifact paths and checksums where relevant: tests/environment_contract.rs
  (M0-09 block).
Interpretation and claim limits: Feature/latch contracts only. Values are
  exact 0/1 (finite by construction, debug-asserted).
Tracker boxes updated: M0-09 checked.
Next eligible task: M0-10 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (features/baselines/fixtures session)
Task IDs: M0-10
Spec sections: 13.1 (B0/B1/O1), 13.3 (oracle expectations), 5.8 (shared
  exogenous schedule)
Change and affected files: src/experiments/mod.rs + baseline.rs (new:
  OrdinaryPolicy trait, RandomBaseline on the tie_break stream, constant
  baselines, privilege-isolated Oracle, run_ordinary/run_oracle harnesses
  with joined ChoiceRecords, BaselineSummary); src/lib.rs (experiments
  module); tests/baselines.rs (new: 5 tests).
Code revision / dirty-tree state: base 10a7ae8; new files untracked.
Commands actually executed:
  cargo test --all-targets --locked (baselines 5/5 pass)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
Outcome and checks passed: Oracle latent accuracy exactly 1.0 on noisy
  (eps 0.1) and hazard-1 tracking configurations, with reward exactly
  !noise_bit per choice; constants commit only their action with per-choice
  XOR accounting; random baseline reproduces its 64-choice action sequence
  from seeds, explores both actions, and diverges on lifetime 3; paired
  oracle/constant lifetimes share cue, noise-bit, and event-id sequences
  while rewards differ exactly where actions differ (production commit path
  on both sides — no forced shared rewards).
Checks not run / failures / blockers: None. A mid-session edit tangled the
  two runner bodies; the file was rewritten whole and re-verified (full
  suite green). Tabular B2 belongs to M5, not this task.
Configuration and suite hashes: Programmatic configs (stationary_noisy
  eps 0.1, hazard-1 K=1); validated before birth.
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetimes 0 and 3; 6-64 outcomes per run.
Artifact paths and checksums where relevant: src/experiments/baseline.rs,
  tests/baselines.rs.
Interpretation and claim limits: Baseline/scoring contracts only. Oracle
  reward expectations are per-choice identities, never per-run upper
  bounds (spec 13.3).
Tracker boxes updated: M0-10 checked.
Next eligible task: M0-11 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (features/baselines/fixtures session)
Task IDs: M0-11
Spec sections: 9.4 (timeline), 17.1 (full deterministic list), 5.5-5.6
  (feature/label boundary)
Change and affected files: tests/event_order.rs (new: birth goldens +
  tick-20/delay-3 timeline + forced-zero variant); tests/leakage.rs
  (+1 pinned-fixture test).
Code revision / dirty-tree state: base 10a7ae8; new/extended tests only
  (no src changes needed — implementation already satisfied the fixture).
Commands actually executed:
  cargo test --all-targets --locked (full suite 52/52 pass: 15 lib + 5
    baselines + 2 config + 17 contract + 3 event_order + 5 leakage + 5 seeds)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo run --locked -- validate-config configs/env_smoke.toml (OK)
  cargo run --locked -- simulate --config configs/env_smoke.toml --seed 1
    --outer-seed 1 (run dir runs/env_smoke-root1-outer1-1789962423/)
Outcome and checks passed: Birth goldens hardcoded (mappings [1, 1], first
  cue 1; probed once via a deleted temporary test, then pinned);
  commitment only at tick 20, ordinary delay ticks 21-22, exactly one
  outcome at tick 23 with annotation (commit 20, outcome 23, exposure 1);
  exact per-tick feature vectors; forced-wrong variant delivers 0 as
  present=1/value=0; pinned ordinary JSON log matches expected public
  values with no hidden markers. Every deterministic §17.1 item now has a
  home (coverage map in docs/decisions.md); randomized frequencies belong
  to M0-12.
Checks not run / failures / blockers: One test-side count slip (24 ticks,
  not 25) corrected; implementation untouched. None otherwise.
Configuration and suite hashes: Fixture config (gap [5,5], delay [3,3])
  validated in-test; smoke configs validated via CLI.
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetime 0; 1-4 outcomes per fixture lifetime.
Artifact paths and checksums where relevant: tests/event_order.rs;
  runs/env_smoke-root1-outer1-1789962423/{manifest.json,resolved_config.toml,
  seed_streams.json} (local, git-ignored).
Interpretation and claim limits: Deterministic fixture evidence only. No
  statistical claims; M0-12 covers randomized checks.
Tracker boxes updated: M0-09 through M0-11 checked.
Next eligible task: M0-12.
```

```text
Date / agent or session: 2026-09-21 / agent (M0 completion session)
Task IDs: M0-12
Spec sections: 16/M0 (frequencies/reward accounting), 13.3 (oracle
  expectation discipline)
Change and affected files: tests/randomized_env.rs (new: 4 checks with a
  declared statistical plan — sample counts, null SEs, tolerances, and
  fixed seeds in the file header).
Code revision / dirty-tree state: base 10a7ae8; M0 work uncommitted.
Commands actually executed:
  cargo test --all-targets --locked (randomized_env 4/4 pass, 0.20s total)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
Outcome and checks passed: Birth-mapping balance (2048 births), cue
  presentation frequency (1024), B0 latent correctness vs chance (2048
  choices, noise-free by construction), and oracle mean reward at eps 0.2
  (1024 choices) all inside predeclared 3.6-4.0σ tolerances on first
  execution — no reseeding, no cherry-picked seeds. Oracle check is
  two-sided consistency, never a per-run upper bound.
Checks not run / failures / blockers: None.
Configuration and suite hashes: Programmatic configs from base_config
  (validated before birth).
Seed namespace / outer seeds / lifetime count: development, root 1, outer
  1..=2048 (births) and lifetimes 0..32, 64 outcomes each.
Artifact paths and checksums where relevant: tests/randomized_env.rs.
Interpretation and claim limits: Sanity frequencies only. Passing
  tolerances is not a learning result and not a fitted model.
Tracker boxes updated: M0-12 checked.
Next eligible task: M0-13 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 completion session)
Task IDs: M0-13
Spec sections: 20.1-20.2 (manifest, event/hidden records), 18.3 (error
  types), AGENTS.md failure behavior
Change and affected files: src/logging/mod.rs + events.rs (new:
  OrdinaryEvent schema v1, ConditionInfo, CompletionInfo/RunStatus,
  LogError, field/stream validation, atomic JSONL/JSON IO); src/run.rs
  (RunManifest gains condition_id/platform/rustc_version; run_simulation
  runner with per-lifetime-block validation; interrupted-completion path);
  src/experiments/baseline.rs (summaries carry hidden annotations for the
  evaluator stream); tests/event_logging.rs (new: 3 tests).
Code revision / dirty-tree state: base 10a7ae8; M0 work uncommitted.
Commands actually executed:
  cargo test --all-targets --locked (event_logging 3/3; logging unit 2/2)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
Outcome and checks passed: 2-lifetime run round-trips exactly (12/12
  events + hidden, counts reconstruct, condition/manifest identity
  verified); logging off yields bitwise-identical commitments/outcomes/
  mean reward with event files absent and completion still written;
  tampered logs (appended duplicate line) fail validation; NaN rewards,
  bad actions, bad schema versions, duplicate/unknown feedback ids, and
  hidden join mismatches are all explicit errors (unit + integration).
Checks not run / failures / blockers: A same-second provenance-overwrite
  bug surfaced during M0-15 evidence runs (see M0-15 entry); fixed with a
  regression test before the gate.
Configuration and suite hashes: Programmatic configs (validated).
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetimes 0-1, 6 outcomes each.
Artifact paths and checksums where relevant: src/logging/events.rs,
  tests/event_logging.rs.
Interpretation and claim limits: Provenance/logging contracts only. Event
  ids restart per lifetime in M0 (documented; global ids arrive with M1
  checkpoints).
Tracker boxes updated: M0-13 checked.
Next eligible task: M0-14 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 completion session)
Task IDs: M0-14
Spec sections: 18.6 (CLI contracts), 20.6 (log audit)
Change and affected files: src/main.rs (simulate gains --baseline
  random|constant-0|constant-1|oracle with B0/B1/O1 condition ids and
  --lifetimes); analysis/validate_logs.py (new, stdlib-only audit:
  identity, per-block order/counts, finite 0/1 rewards, duplicates,
  hidden join, completion); analysis/test_validate_logs.py (new, 6
  tests); analysis/fixtures/ (valid oracle smoke run + 5 corrupt
  variants); analysis/README.md.
Code revision / dirty-tree state: base 10a7ae8; M0 work uncommitted.
Commands actually executed:
  python3 analysis/test_validate_logs.py (6/6 pass)
  python3 analysis/validate_logs.py analysis/fixtures/valid (OK)
  python3 analysis/validate_logs.py on all 5 corrupt fixtures (each FAILs
    with the expected error class: duplicate, order, reward, completion,
    hidden)
  cargo run --release --locked -- simulate (all 4 baselines x 2 lifetimes;
    see M0-15 entry) + audit of each run dir (OK x4)
  cargo fmt/clippy/test (clean; analysis untouched by Rust gates)
Outcome and checks passed: A clean checkout runs validate-config,
  simulate with documented baseline selection, and the audit; corrupt
  fixtures fail loudly; unavailable later-stage fields are absent by
  schema (v1), never fabricated.
Checks not run / failures / blockers: None. aggregate.py belongs to M8.
Configuration and suite hashes: configs/env_smoke.toml via CLI.
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetimes 0-1 per smoke run.
Artifact paths and checksums where relevant: analysis/validate_logs.py,
  analysis/fixtures/{valid,corrupt_*}/.
Interpretation and claim limits: Audit tooling only. The audit verifies
  accounting, not learning.
Tracker boxes updated: M0-14 checked.
Next eligible task: M0-15 (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 completion session)
Task IDs: M0-15
Spec sections: 16/M0 (exit: runnable command, tests, example output)
Change and affected files: docs/experiments.md (M0 evidence bundle entry
  with exact commands/results); README.md (M0-complete status, full
  command list incl. baselines + audit, run-dir contents, layout).
Code revision / dirty-tree state: base 10a7ae8; M0 work uncommitted
  (manifests record git_dirty=true honestly).
Commands actually executed (release mode):
  cargo fmt --all -- --check — clean
  cargo clippy --all-targets --locked -- -D warnings — clean
  cargo test --all-targets --locked — 62/62 pass (18 lib + 5 baselines +
    2 config + 17 contract + 3 event_logging + 3 event_order + 5 leakage +
    4 randomized + 5 seeds)
  python3 analysis/test_validate_logs.py — 6/6 pass
  validate-config configs/env_smoke.toml — OK
  simulate --baseline random --lifetimes 2 — B0 16/16, mean 0.2500
    (runs/env_smoke-root1-outer1-1789963074/)
  simulate --baseline constant-0 --lifetimes 2 — B1 16/16, 0.5000 (-retry1)
  simulate --baseline constant-1 --lifetimes 2 — B1 16/16, 0.5000 (-retry2)
  simulate --baseline oracle --lifetimes 2 — O1 16/16, 1.0000 (-retry3)
  validate_logs.py on all four run dirs — OK x4 (16/16 events+hidden,
    completed, counts agree)
Outcome and checks passed: Every Section 16/M0 exit condition has
  runnable-command + test + saved-output evidence. No neural code,
  evolution, or renderer was needed or added.
Checks not run / failures / blockers: One real failure found: the
  run-directory retry logic broke before checking the manifest marker,
  so the first evidence pass overwrote one directory four times. Fixed
  (ownership by marker absence) with regression test
  (same_second_runs_never_share_a_directory), colliding dirs deleted,
  all four baselines re-executed into distinct directories and
  re-audited. No failures outstanding.
Configuration and suite hashes: configs/env_smoke.toml (+
  debug_stationary.toml validated); resolved copies inside each run dir.
Seed namespace / outer seeds / lifetime count: development, root 1, outer 1,
  lifetimes 0-1, 8 outcomes each per smoke run.
Artifact paths and checksums where relevant: docs/experiments.md (bundle
  entry); runs/env_smoke-root1-outer1-1789963074*/ (local, git-ignored,
  4 dirs); analysis/fixtures/valid/ (committed audit fixture).
Interpretation and claim limits: Milestone-exit evidence. Smoke means are
  single-run values, not performance claims.
Tracker boxes updated: M0-15 checked.
Next eligible task: M0-GATE (completed same session; see next entry).
```

```text
Date / agent or session: 2026-09-21 / agent (M0 completion session)
Task IDs: M0-GATE
Spec sections: 16/M0 (exit conditions)
Change and affected files: to-do.md (gate box, status, ownership);
  README.md (M0-complete status + frozen smoke command).
Code revision / dirty-tree state: base 10a7ae8; M0 work uncommitted.
Commands actually executed: Full M0-15 bundle above (nothing additional
  needed; gate re-verified from saved evidence, no re-execution with
  different seeds).
Outcome and checks passed — exit conditions, each with evidence:
  - All deterministic environment tests pass (contract/event_order/
    leakage suites, 25 tests).
  - Oracle latent accuracy is exactly 1.0, incl. hazard-1 tracking
    (tests/baselines.rs).
  - Chance controls pass declared statistical checks first try, fixed
    seeds (tests/randomized_env.rs).
  - Completed lifetimes deliver one reward per commitment (counts tests
    + 4 audited smoke runs at 16/16).
  - Hidden data and RNG isolation verified (leakage suite + stream
    independence + on/off logging parity).
  - Runnable smoke command + example output saved (README bundle block;
    docs/experiments.md M0-15 entry; runs/*/<4 dirs>).
Checks not run / failures / blockers: None outstanding. The one failure
  found this session (run-dir overwrite) was fixed and re-evidenced
  before the gate, with the negative evidence preserved in the ledger
  and decisions log.
Configuration and suite hashes: As M0-15.
Seed namespace / outer seeds / lifetime count: As M0-15. Final-test
  namespace untouched (no final_test seeds used anywhere).
Artifact paths and checksums where relevant: Same as M0-15.
Interpretation and claim limits: M0 proves observations, hidden mappings,
  timing, and reward accounting are correct. It claims no dynamics,
  learning, or comparisons — M1 starts here.
Tracker boxes updated: M0-GATE checked; status M0-GATE passed, next M1-01.
Next eligible task: M1-01.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex owner-requested M0 review
Task IDs: M0-REVIEW; M0-GATE re-verification
Spec sections: 1–10, 16/M0, 17.1, 18–20
Change and affected files: src/config.rs, environment/{mod,hidden_state,
  observation}.rs, experiments/baseline.rs, logging/events.rs, run.rs,
  main.rs; M0 regression tests; analysis auditor/tests/docs; .python-version;
  README, docs/{m0-review,decisions,experiments}.md, this tracker.
  Independent AGENTS.md edits preserved; spec.md unchanged.
Code revision / dirty-tree state: base 57b6870, dirty. Exact source and
  release-binary SHA-256 values in docs/evidence/m0-review/summary.json.
Commands actually executed: exact argv, exit codes, and full outputs in
  docs/evidence/m0-review/commands.json, including:
  cargo fmt --all -- --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo test --all-targets --locked
  python3 analysis/test_validate_logs.py
  cargo test --locked --test randomized_env birth_mapping_pairs_are_independent -- --nocapture
  cargo run --release --locked -- --help
  cargo run --release --locked -- validate-config configs/env_smoke.toml
  cargo run --release --locked -- validate-config configs/debug_stationary.toml
  cargo run --release --locked -- simulate --config configs/debug_stationary.toml --seed 1
    (expected rejection, exit 1; unsupported neural execution)
  Release simulate for random/constant-0/constant-1/oracle on env_smoke,
    plus random/oracle on saved noisy_variable.toml and oracle on
    noisy_logging_off.toml; all --seed 1 --outer-seed 1 --lifetimes 2
    --out-dir runs/m0-review. Each output audited with validate_logs.py.
  validate_logs.py on four original runs under
    runs/env_smoke-root1-outer1-1789963074[-retry1|-retry2|-retry3].
Outcome and checks passed: 73 Rust tests and 14 Python tests pass; none
  ignored; fmt/clippy clean. All four original runs pass stronger audit.
  Seven fresh runs pass (logging-off coverage explicitly reduced).
  Fresh clean events/hidden annotations match original fields except run_id.
  Mapping-pair counts 516/502/520/510 across 2048 births meet the tolerance
  declared before executing the added test; existing goldens unchanged.
Checks not run / failures / blockers: No remaining M0 blocker. Demonstrated
  old auditor false pass on a truncated seven-of-eight lifetime; fixed and
  regression-tested. During audit development a duplicate fixture initially
  produced only count errors; independent duplicate detection restored.
  Rejected neural-config execution is expected, not a successful simulation.
Configuration / seeds / resources: development namespace only; root 1,
  outer 1, lifetimes 0–1 for smoke runs. Clean four runs: 16 outcomes and
  272 ticks each, reward sums 4/8/8/16 (B0/B1-0/B1-1/O1). Noisy B0/O1:
  32 outcomes and 602 ticks each, five noise flips, four changes, rewards
  19/27; oracle latent accuracy 32/32. Logging-off oracle has same reported
  32 outcomes and reward 27/32. No final-test outcomes inspected.
Artifact paths: docs/evidence/m0-review/{commands,summary}.json plus saved
  diagnostic configs; seven immutable raw directories under runs/m0-review/.
  Full review and limits: docs/m0-review.md.
Interpretation / deviations: M0 correctness evidence, no learning claim.
  Preserved documented warmup semantics and provisional noise assignment;
  no scientific contract/golden/RNG changes. M1-07 still owns the split
  observe/finish API; M5-02 owns factorial assignment. No search launched.
Tracker boxes updated: M0-REVIEW checked; M0-GATE retained after re-verification.
Next eligible task: M1-01.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex documentation handoff
Task IDs: M0-DOCS
Spec sections: 18–20 (documentation, interfaces, provenance); no scientific changes
Change and affected files: AGENTS.md, README.md, to-do.md,
  docs/handoff.md, docs/evidence/README.md, appended docs/decisions.md and
  docs/experiments.md, analysis/README.md, manifests/README.md and the
  descriptive note in manifests/development.json; comments only in
  configs/debug_stationary.toml and src/{lib,config,run}.rs,
  src/environment/{mod,observation,schedule}.rs, src/logging/mod.rs.
Code revision / dirty-tree state: base 8216c14 (M0 refined); clean at start,
  documentation edits uncommitted at handoff.
Commands actually executed:
  git status --short; git log -4 --oneline; focused rg/sed/cat inspection
  Python standard-library document audit: resolve local Markdown links and
    anchors; parse TOML/JSON; compare Rust text excluding comments and TOML
    values against HEAD; compare reservation fields excluding the note;
    assert prior ledger entries and dated logs preserved; verify immutable
    spec/review/fixtures/evidence and historical source hashes at 8216c14.
  cargo fmt --all -- --check
  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
  python3 analysis/validate_logs.py analysis/fixtures/valid
  git diff --check
Outcome and checks passed: document consistency and preservation checks
  pass; Rust formatting passes; public Rust docs build with warnings denied;
  committed fixture audit passes; whitespace check clean. Live docs agree
  that M0 is verified and M1-01 is next. Claims and ownership released.
Checks not run / failures / blockers: Initial rustdoc build found a public
  link to private PhaseState; changed only that documentation link to plain
  code text, then build passed. Full Rust/Python suites and new simulations
  not rerun: only docs/comments and a manifest description changed. The prior
  73 Rust / 14 Python passes remain M0-REVIEW evidence, not new results.
Configuration and suite hashes: Config values, seed reservation fields,
  simulator code, dependencies, and scientific spec unchanged. Historical
  review fingerprints still match source at 8216c14; comment edits explain
  later source-file hash differences. Archived hashes were not rewritten.
Seed namespace / outer seeds / lifetime count: Not applicable; no new lifetime.
Artifact paths: docs/handoff.md (current continuation); generated public
  API docs at target/doc/cra/index.html (ignored build output). Previous
  review/evidence files retained. No new experiment bundle required.
Interpretation and claim limits: Documentation readiness only; no milestone
  promotion, learning claim, or new performance/throughput evidence.
Tracker boxes updated: M0-DOCS checked; M0-GATE retained.
Next eligible task: M1-01; no outstanding blocker.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1 topology session)
Task IDs: M1-01
Spec sections: 3.2 (starting sizes), 4.1 (W[receiver, sender]), 6.5
  (observability), 10.1 (recurrent mask), 18.4-18.5 (edge order, dense first)
Change and affected files: src/agent/mod.rs (new, declares topology only);
  src/agent/topology.rs (new: init-stream Bernoulli sampling, last-index
  motor pools, receiver-grouped edge order, cycle + per-pool reachability
  checks, rejection logging, explicit exhaustion); src/lib.rs (register
  agent module); tests/topology.rs (new, 13 tests); docs/decisions.md
  (M1-01 conventions entry); README.md (layout line);
  docs/handoff.md (M1-02 continuation).
Code revision / dirty-tree state: base 7132433 (docs updated); M1-01 files
  new or modified and uncommitted at handoff (git status: M
  docs/decisions.md, src/lib.rs, to-do.md, README.md, docs/handoff.md;
  ?? src/agent/, tests/topology.rs).
Commands actually executed:
  cargo test --locked --test topology (13/13 pass)
  cargo fmt --all -- --check (clean after one formatting pass)
  cargo clippy --all-targets --locked -- -D warnings (clean after
    iterator-form loop fixes in src/agent/topology.rs and
    tests/topology.rs)
  cargo test --all-targets --locked (88/88 pass: 21 lib incl. 2 new
    topology units + 7 baselines + 4 config + 20 contract + 5 logging +
    3 order + 5 leakage + 5 randomized + 5 seeds + 13 new topology)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and configs/env_smoke.toml (OK)
Outcome and checks passed: Same init tuple reproduces mask/edges/pools;
  outer 1 vs 2 masks differ; motor pools fixed/disjoint on last indices
  (16/2 -> [12,13]/[14,15]; 60/4 -> [52-55]/[56-59]); missing edges absent
  with counts agreeing; diagonal absent when self_edges=false (incl. p=1
  boundary) and enforced in topology_from_mask; hand-built good graph
  accepted and each bad graph yields its exact reason subset; p=0 exhausts
  with a 3-attempt fully logged error; non-init streams and max_attempts=0
  rejected; debug_stationary actor section samples and validates end to end.
Checks not run / failures / blockers: No failures. M0 smoke simulate not
  rerun (no runner change; validate-config confirms no config regression).
  Full Monte Carlo suites remain in their queued milestones.
Configuration and suite hashes: configs/debug_stationary.toml actor
  section (N=16, m=2, p=0.25, self_edges=false); seed tuple (root 1,
  development, outer 1, lifetime 0, stream init) for determinism/pairing
  fixtures; no suites consumed.
Seed namespace / outer seeds / lifetime count: development only; outer
  1/2/3/5/7/9 fixtures at lifetime_index 0; no lifetimes simulated.
Artifact paths and checksums where relevant: src/agent/topology.rs,
  src/agent/mod.rs, tests/topology.rs (committed with this entry; no
  run directories produced).
Interpretation and claim limits: Structural sampling only. No weights,
  dynamics, plasticity, or learning claim; M1-GATE remains open until
  M1-02 through M1-12 verify.
Tracker boxes updated: M1-01 checked.
Next eligible task: M1-02.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1 weights session)
Task IDs: M1-02
Spec sections: 6.3 (starting constants), 10.2 (inherited actor weights),
  20.5 (seed derivation, distribution implementation)
Change and affected files: src/agent/weights.rs (new: Box-Muller
  NormalStream, row_std = gain/sqrt(d), sample_weights_from_mask with exact
  missing-edge/bias zeros and nonfinite backstop, validate_inherited,
  sample_inherited on one init RNG in mask-then-W0-then-B order,
  InheritedParams/SampledParams/ParamsError); src/agent/topology.rs
  (behavior-preserving split: sample_topology delegates to crate-visible
  sample_topology_with_rng; validate_probability/check_init_seed widened
  for reuse); src/agent/mod.rs (register weights); tests/weights.rs (new,
  10 tests + 1 ignored probe); docs/decisions.md (M1-02 conventions entry);
  README.md (layout line); docs/handoff.md (M1-03 continuation).
Code revision / dirty-tree state: base 891d99a (m1-0); M1-02 files new or
  modified and uncommitted at handoff (M docs/decisions.md,
  src/agent/{mod,topology}.rs, to-do.md, README.md, docs/handoff.md;
  ?? src/agent/weights.rs, tests/weights.rs).
Commands actually executed:
  cargo test --locked --test weights (10/10 pass, 1 ignored probe)
  cargo test --locked --test weights probe_golden_values -- --ignored
    --nocapture (probe only: edges=52 accepted=0 w0_sum=-3.92880663437287181
    b_sum=-1.19270571160902339; values then hardcoded as the golden
    tripwire, truncated to f64 precision per clippy)
  cargo test --locked --test topology (13/13 pass; refactor unchanged)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean after removing
    one same-type cast and truncating golden literals to f64 precision)
  cargo test --all-targets --locked (100/100 pass, 1 ignored: 23 lib incl.
    2 new weights units + 7 baselines + 4 config + 20 contract + 5 logging
    + 3 order + 5 leakage + 5 randomized + 5 seeds + 13 topology + 10 new
    weights)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and configs/env_smoke.toml (OK)
Outcome and checks passed: Same seed reproduces W0/B/bias; paired resample
  equals; outer 1 vs 2 differ; N=200 full-mask fixture gives W0 variance
  within 5% of gain^2/d (mistake version would be ~300x off) and B variance
  within tolerance of input_scale^2; row_std(0.8,4)=0.4 exactly and
  row_std(*,0)=0.0; zero-degree rows exact-zero with deterministic later
  draws; missing edges exact 0.0; biases exact 0.0; gain/scale 1e308 fails
  as InvalidParams; validation table rejects wide motors, bad taus/sigma/
  gains, input_dim 0, non-init stream, max_attempts 0, p=1.5;
  debug_stationary end-to-end validates with input_dim 8; golden tripwire
  passes.
Checks not run / failures / blockers: One test expectation was initially
  wrong (hand-mask edge count 5 vs actual 4; corrected in the test with no
  source change). M0 smoke simulate not rerun (no runner change).
  Expensive Monte Carlo suites remain in their queued milestones.
Configuration and suite hashes: configs/debug_stationary.toml actor
  section (N=16, m=2, p=0.25, gain 0.8, scale 0.3); seed tuples (root 1,
  development, outer 1/2/4/7/11/21, lifetime 0, stream init); input_dim 8
  (debug, K+6) and 14 (N=200 fixture); no suites consumed.
Seed namespace / outer seeds / lifetime count: development only, fixture
  seeds as above; no lifetimes simulated.
Artifact paths and checksums where relevant: src/agent/weights.rs,
  tests/weights.rs (committed with this entry; no run directories
  produced).
Interpretation and claim limits: Inherited initialization only. No
  dynamics, plasticity, or learning claim; W0 is stored without P and the
  effective W0+P construction waits for M3. M1-GATE remains open.
Tracker boxes updated: M1-02 checked.
Next eligible task: M1-03.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1 transition session)
Task IDs: M1-03
Spec sections: 4.1 (W[receiver, sender]), 6.1 (actor update), 6.2
  (adaptation), 18.4-18.5 (preallocated buffers, dense reference)
Change and affected files: src/agent/actor.rs (new: leak_alpha via
  -expm1, ActorState with preallocated double buffers, from_state/new
  constructors, step_with_perturbations fixture path plus stochastic step
  sharing one slice core with Vec-level swaps, dense drive accumulation,
  post-integration noise, no clipping with explicit NonFiniteState,
  ActorError); src/agent/mod.rs + src/lib.rs (register/document actor);
  tests/actor.rs (new, 7 tests); docs/decisions.md (M1-03 conventions
  entry); README.md (layout line); docs/handoff.md (M1-04 continuation).
Code revision / dirty-tree state: base 891d99a (m1-0) with the uncommitted
  M1-02 work still pending underneath; M1-03 files new or modified and
  uncommitted at handoff (M README.md, docs/decisions.md, docs/handoff.md,
  src/agent/mod.rs, src/agent/topology.rs, src/lib.rs, to-do.md;
  ?? src/agent/actor.rs, src/agent/weights.rs, tests/actor.rs,
  tests/weights.rs).
Commands actually executed:
  cargo test --locked --test actor (7/7 pass)
  cargo test --locked --lib agent:: (6/6 pass incl. 2 leak_alpha units)
  cargo fmt --all -- --check (clean after one formatting pass)
  cargo clippy --all-targets --locked -- -D warnings (clean after
    slice-typed core params with Vec-level swaps)
  cargo test --all-targets --locked (109/109 pass, 1 ignored: 25 lib + 0
    bin + 7 actor + 7 baselines + 4 config + 20 contract + 5 logging +
    3 order + 5 leakage + 5 randomized + 5 seeds + 13 topology + 10
    weights)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and configs/env_smoke.toml (OK)
Outcome and checks passed: One-edge fixture proves orientation (receiver
  gains 0.6*r0 drive; sender update has no reverse term despite large
  neighbor activity) and adaptation sign; bidirectional fixture matches
  old-activity-only values at 1e-12 (in-place updates would mismatch);
  forced perturbations at tau 100 and 5 match sigma*xi at 1e-12 (the
  alpha-scaled mistake would be ~100x off at tau 100); h=+-500 leaks
  exactly with r=+-1 (no clipping); leak_alpha agrees with 1-exp to ~1
  ulp and holds 1e-9 at tau 1e9; stochastic step shares the core with
  fresh tanh activity; dim/param/nonfinite violations are explicit errors.
Checks not run / failures / blockers: Initial 1e-15 leak_alpha bound was
  tighter than the ~1 ulp formulation difference at tau 100; loosened to
  1e-14 with the reason recorded (no source change). M0 smoke simulate
  not rerun (no runner change). Perturbation distribution/schedule audit
  belongs to M1-04; watchdog summaries to M1-08.
Configuration and suite hashes: Hand-built N=2 fixtures (edge 0.6,
  reverse -0.4, tau_h 5/100, tau_a 100, strength 0.5/0.0, sigma 0.05);
  no suites consumed.
Seed namespace / outer seeds / lifetime count: development actor_noise
  seed (root 1, outer 1, lifetime 0) for the stochastic wiring check only;
  no lifetimes simulated.
Artifact paths and checksums where relevant: src/agent/actor.rs,
  tests/actor.rs (committed with this entry; no run directories produced).
Interpretation and claim limits: Transition dynamics only. No motor
  readout, learning, or continuity claim; W0 stays immutable and P
  absent. M1-GATE remains open.
Tracker boxes updated: M1-03 checked.
Next eligible task: M1-04.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1 noise-schedule session)
Task IDs: M1-04
Spec sections: 6.1/6.3 (perturbations), 18.4 (draw every tick regardless
  of gates), 20.5 (seed derivation, RNG state for resume)
Change and affected files: src/agent/actor.rs (fixture path records its
  vector; new last_perturbations getter; draw-schedule module docs);
  tests/actor_noise.rs (new, 7 tests); docs/decisions.md (M1-04
  conventions entry); docs/handoff.md (M1-05 continuation). No sampler or
  transition math changed.
Code revision / dirty-tree state: base 5f8b9b8 (M1-03 done); clean at
  start, M1-04 files modified or new and uncommitted at handoff (M
  src/agent/actor.rs, to-do.md, docs/decisions.md, docs/handoff.md;
  ?? tests/actor_noise.rs).
Commands actually executed:
  cargo test --locked --test actor_noise (7/7 pass)
  cargo test --all-targets --locked (116/116 pass, 1 ignored: 25 lib + 0
    bin + 7 actor + 7 actor_noise + 7 baselines + 4 config + 20 contract
    + 5 logging + 3 order + 5 leakage + 5 randomized + 5 seeds + 13
    topology + 10 weights)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and configs/env_smoke.toml (OK)
  throwaway probe_tmp resume-API check (passed, deleted afterwards)
Outcome and checks passed: step draws equal the reference stream tick by
  tick for N = 4 and odd N = 3 (even N additionally matches an independent
  contiguous derivation); identical schedules under zero/one-hot inputs
  and saturated state; interleaved reads leave trajectories identical;
  50k seeded normals give mean/var within ~4.5 SE of (0, 1); 100 steps
  leave the cue_order stream untouched; word position round-trips the
  uniform stream and seed-bytes-plus-position reproduces the next tick's
  perturbations for N = 4 and 3.
Checks not run / failures / blockers: Two first-draft tests encoded wrong
  expectations, both corrected in tests with no source change: (a) odd-N
  ticks re-pair deterministically instead of continuing one stream, which
  fixed the design as per-tick-local pairing (simpler M1-09 resume: seed
  bytes plus word position only); (b) a NormalStream temporary cannot hold
  its spare across statements, replaced by the tick-boundary resume test.
  Gate-variation schedule proof waits for gates (M6-05); every-phase
  stepping waits for the runner (M1-07).
Configuration and suite hashes: Hand N = 4/3 fixtures (zeros, one-hot,
  saturated states/inputs); actor_noise seeds (root 1, development, outer
  1/3/5/9, lifetime 0); no suites consumed.
Seed namespace / outer seeds / lifetime count: development only, fixture
  seeds as above; no lifetimes simulated.
Artifact paths and checksums where relevant: tests/actor_noise.rs,
  src/agent/actor.rs draw-schedule docs (no run directories produced).
Interpretation and claim limits: Generator and schedule verification
  only. No dynamics, plasticity, or learning claim. M1-GATE remains open.
Tracker boxes updated: M1-04 checked.
Next eligible task: M1-05.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1 adaptation session)
Task IDs: M1-05
Spec sections: 6.1 (actor update with adaptation drive), 6.2 (adaptation
  interpretation, strength 0 then 0.1 ablation), 6.3 (starting constants)
Change and affected files: tests/adaptation.rs (new, 5 tests);
  docs/handoff.md (M1-06 continuation). No source change: the update,
  drive term, and zero-strength config already verified under M1-02/M1-03.
Code revision / dirty-tree state: base 5f8b9b8 (M1-03 done) with
  uncommitted M1-04/M1-05 work pending; M1-05 files new or modified and
  uncommitted at handoff (M to-do.md, docs/handoff.md;
  ?? tests/adaptation.rs).
Commands actually executed:
  cargo test --locked --test adaptation (5/5 pass)
  cargo test --all-targets --locked (121/121 pass, 1 ignored: 25 lib + 0
    bin + 7 actor + 7 actor_noise + 5 adaptation + 7 baselines + 4 config
    + 20 contract + 5 logging + 3 order + 5 leakage + 5 randomized + 5
    seeds + 13 topology + 10 weights)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean after
    iterator-form loop and array-literal fixes in tests/adaptation.rs)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and configs/env_smoke.toml (OK)
  throwaway ignored probe of the opposition trajectory (passed, deleted
  afterwards; values below are its recorded output)
Outcome and checks passed: Strength 0 leaves h bit-identical across
  different a_old while a still tracks activity (inert yet evolving);
  one-tick a matches alpha(tau_a)*r_old at 1e-15 and excludes the tau_h
  swap by half the formulation gap; nonzero-strength unit fixture
  (tau_a = 2, strength 0.5) shows a > 0 every tick, h below the paired
  zero-strength control from tick 1, and overshoot below zero by tick 11
  (ha -0.0067 vs control +0.0907) while the control stays positive;
  10-tick replay of the a recurrence from recorded r across four cycling
  input patterns matches exactly (no boundary clearing);
  debug_stationary.toml holds adaptation_strength 0.0.
Checks not run / failures / blockers: Three first-draft expectations were
  wrong, all corrected in tests with no source change: tick-0 opposition
  is vacuous (a starts 0 for both runs; asserted from tick 1), the tau_h
  exclusion margin exceeded the small-neuron formulation gap (assert half
  the gap), and strict a monotonicity is false (a chases falling r and
  peaks at tick 2; asserted sign, initial build-up, and overshoot
  instead). The 0.1 ablation from spec 6.2 stays a later decision, not
  this task. M0 smoke simulate not rerun (no runner change).
Configuration and suite hashes: Hand N = 2 fixtures (tau_h 5, tau_a
  100/20/2, strength 0.0/0.5); debug_stationary.toml strength pin; no
  suites consumed.
Seed namespace / outer seeds / lifetime count: N/A (deterministic
  fixtures, no RNG).
Artifact paths and checksums where relevant: tests/adaptation.rs (no run
  directories produced).
Interpretation and claim limits: Adaptation behavior verified; the
  initial learner stays at strength 0. No dynamics change, no learning
  claim. M1-GATE remains open.
Tracker boxes updated: M1-05 checked.
Next eligible task: M1-06.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1 motor session)
Task IDs: M1-06
Spec sections: 6.3 (motor filter constant), 6.4 (pool means, leaky
  filter, commitment, tie stream), 10.4 (birth q = 0)
Change and affected files: src/agent/motor.rs (new: MotorState with birth
  zeros/from_q, update returning new MotorOutput, decide_action with
  tie-only draws, pool validation, MotorError); src/agent/mod.rs +
  src/lib.rs (register/document motor); tests/motor.rs (new, 7 tests);
  docs/decisions.md (M1-06 conventions entry); README.md (layout line);
  docs/handoff.md (M1-07 continuation).
Code revision / dirty-tree state: base 585f36c (m1-05); clean at start,
  M1-06 files new or modified and uncommitted at handoff (M README.md,
  docs/decisions.md, docs/handoff.md, src/agent/mod.rs, src/lib.rs,
  to-do.md; ?? src/agent/motor.rs, tests/motor.rs).
Commands actually executed:
  cargo test --locked --test motor (7/7 pass)
  cargo test --all-targets --locked (129/129 pass, 1 ignored: 26 lib
    incl. 1 new motor unit + 0 bin + 7 actor + 7 actor_noise + 5
    adaptation + 7 motor + 7 baselines + 4 config + 20 contract + 5
    logging + 3 order + 5 leakage + 5 randomized + 5 seeds + 13 topology
    + 10 weights)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean after
    iterator-form pool-mean loop in src/agent/motor.rs)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and configs/env_smoke.toml (OK)
Outcome and checks passed: Golden recurrences match the independent
  1-exp formulation at 1e-12 from zero and over three ticks; pool means
  read assigned populations (swaps would flip values); stale q favoring
  action 0 loses to fresh activity favoring 1 (commitment reads new q);
  strict decisions leave the tie word position unchanged while an exact
  tie draws one coin equal to the replicated random_bool (deterministic
  per seed); empty/out-of-range/overlapping pools, bad taus, and
  nonfinite inputs rejected; debug pools equal the topology assignment
  ([12,13]/[14,15]) with finite readout.
Checks not run / failures / blockers: None. M0 smoke simulate not rerun
  (no runner change). Full-lifetime integration waits for M1-07.
Configuration and suite hashes: Hand N = 4 fixtures (means +-0.15..1.0,
  tau_q 3); tie_break seed (root 1, development, outer 1, lifetime 0);
  debug_stationary motor_filter_tau; no suites consumed.
Seed namespace / outer seeds / lifetime count: development tie_break
  fixtures only; no lifetimes simulated.
Artifact paths and checksums where relevant: src/agent/motor.rs,
  tests/motor.rs (no run directories produced).
Interpretation and claim limits: Fixed readout only. No decoder, no
  exploration policy (proven by zero RNG consumption on strict
  decisions), no learning claim. M1-GATE remains open.
Tracker boxes updated: M1-06 checked.
Next eligible task: M1-07.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1-07 integration session)
Task IDs: M1-07
Spec sections: 3.4 (simulator/agent boundary), 5.5-5.8 (observable
  features, forbidden inputs, event timing, commitment), 6.1/6.4 (actor
  transition, motor readout), 9 (feedback before transition, no double
  apply), 10.4 (birth h=a=q=0), 13.1 (B3 same-actor no-update control)
Change and affected files: src/agent/no_learning.rs (new: NoLearningActor
  with birth-zero state, outer-seed init pairing, per-lifetime
  actor_noise/tie_break streams, Agent apply_feedback dedup-only plus
  advance through actor step and fixed motor filter, select_action from
  newest readout, NoLearningError); src/agent/mod.rs + src/lib.rs
  (register/document no_learning); src/config.rs (new
  validate_actor_no_learning_execution: requires [actor] and birth_only,
  allows only learning disabled/absent, modulator absent/fixed, evolution
  disabled/absent); src/experiments/baseline.rs (OrdinaryPolicy for
  NoLearningActor, run_actor_ordinary sharing the ordinary tick/commit/
  record loop with its own guard, run_ordinary refactored through
  run_ordinary_inner); tests/no_learning.rs (new, 10 tests);
  docs/decisions.md (M1-07 conventions entry); docs/handoff.md (M1-08
  continuation).
Code revision / dirty-tree state: base 0df0f04 (mislabeled m1-07 carrying
  M1-06); clean at start, M1-07 files new or modified and uncommitted at
  handoff (M src/agent/mod.rs, src/config.rs,
  src/experiments/baseline.rs, src/lib.rs, to-do.md, docs/decisions.md,
  docs/handoff.md; ?? src/agent/no_learning.rs, tests/no_learning.rs).
Commands actually executed:
  cargo test --locked --test no_learning (10/10 pass)
  cargo test --all-targets --locked (140 pass, 1 ignored: 27 lib incl. 1
    new no_learning unit + 0 bin + 7 actor + 7 actor_noise + 5 adaptation
    + 7 motor + 10 no_learning + 7 baselines + 4 config + 20 contract + 5
    logging + 3 order + 5 leakage + 5 randomized + 5 seeds + 13 topology
    + 10 weights)
  cargo fmt --all -- --check (clean after cargo fmt)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and
    cargo run --release --locked -- simulate --config
    configs/env_smoke.toml --baseline oracle --lifetimes 2 --seed 1
    (O1 2 lifetimes, 16/16 outcomes, mean 1.0; fresh run dir removed
    afterward, no new committed artifacts)
Outcome and checks passed: Birth zeros with paired init sharing across
  lifetimes under one outer seed and variation across outer seeds;
  tick-by-tick lifetime shows state moves every tick, post-feedback
  membranes/filters nonzero (no boundary reset), W0/B/bias bit-identical
  afterward; actor vs B1 paired lifetimes share cue/noise/event/tick
  schedules while rewards follow own XOR actions; reward 0 vs 1 features
  diverge trajectories while apply_feedback moves nothing and W0 stays
  fixed; duplicates/backwards/invalid rewards and width mismatches fail
  without tick or state change; interleaved reads reproduce bitwise and
  reruns reproduce choices; guards accept the no-learning profile (and
  explicitly disabled learning) while rejecting env-only-as-actor,
  enabled learning/evolution, non-fixed gates, diagnostic resets, and
  reserved env kinds; bad namespaces/sections explicit.
Checks not run / failures / blockers: None. One test initially expected
  InvalidConfig for a bad namespace and got the explicit Params BadSeed
  from init validation; fixed the expectation (no code change).
Configuration and suite hashes: actor_no_learning_test profile (env_smoke
  timing, N=16/m=2/p=0.25, 6 outcomes, no learning/modulator/evolution);
  init outer 1 vs 2 pairing fixtures; development namespace; no suites
  consumed.
Artifact paths and checksums where relevant: src/agent/no_learning.rs,
  tests/no_learning.rs (no run directories produced; smoke rerun dir
  removed).
Interpretation and claim limits: Continuous nonplastic dynamics only.
  Reward enters as sensory channels; no P/E/gates/search. B3 is the
  same-actor no-update control, not a B7 activity-only optimum. The
  configs/actor_no_learning.toml file and simulate wiring stay M1-12
  work. M1-GATE remains open.
Tracker boxes updated: M1-07 checked.
Next eligible task: M1-08.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1-08 health session)
Task IDs: M1-08
Spec sections: 10.6 (f64, no h clipping, conservative watchdog, explicit
  failures), 6.5 (observability needs: distinguishable, non-saturated
  dynamics)
Change and affected files: src/agent/health.rs (new: HEALTH_SCHEMA_VERSION
  1, WATCHDOG bounds 1e4 for h/a/q, SATURATION_R_ABS 0.9, HealthError with
  NonFinite/WatchdogTripped/InvalidConfig plus to_sim_error mapping,
  selected_trace_indices pure stable selection, check_state finiteness plus
  watchdog, HealthSummary extrema/saturation/margins with no-record-on-
  failure, TraceRecorder sampled snapshots with tick-order/file-ready
  JSON); src/agent/mod.rs + src/lib.rs (register/document health);
  src/agent/no_learning.rs (read-only health_check borrowing h/a/r/q,
  no RNG, no mutation); tests/health.rs (new, 9 tests); docs/decisions.md
  (M1-08 conventions entry); docs/handoff.md (M1-09 continuation);
  README.md (layout line).
Code revision / dirty-tree state: base 3f0f0c6 (m1-07 done); clean at
  start, M1-08 files new or modified and uncommitted at handoff
  (M src/agent/mod.rs, src/agent/no_learning.rs, src/lib.rs, to-do.md,
  docs/decisions.md, docs/handoff.md, README.md; ?? src/agent/health.rs,
  tests/health.rs).
Commands actually executed:
  cargo test --locked --test health (9/9 pass)
  cargo test --all-targets --locked (150 pass, 1 ignored: 28 lib incl. 1
    new health unit + 0 bin + 7 actor + 7 actor_noise + 5 adaptation + 7
    motor + 10 no_learning + 9 health + 7 baselines + 4 config + 20
    contract + 5 logging + 3 order + 5 leakage + 5 randomized + 5 seeds
    + 13 topology + 10 weights)
  cargo fmt --all -- --check (clean after cargo fmt)
  cargo clippy --all-targets --locked -- -D warnings (clean after
    is_multiple_of plus two test-lint fixes)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and
    cargo run --release --locked -- simulate --config
    configs/env_smoke.toml --baseline oracle --lifetimes 2 --seed 1
    (O1 2 lifetimes, 16/16 outcomes, mean 1.0; fresh run dir removed
    afterward, no new committed artifacts)
Outcome and checks passed: Forced NaN/Inf in h/a/r/q fails as NonFinite
  naming the component through check_state/observe/maybe_record/
  health_check with counters/samples untouched (no clipping); finite
  10_001 trips the watchdog naming bound 1e4 while boundary 1e4 and O(1)
  pass; saturation 2/4 = 0.5 and margins min 0.2/max 1.0 match hand
  values with None before first tick; selection [0,1,12,14] stable,
  motor-covering, <= 4 entries, degenerate cases covered, interleaved
  calls leave trajectories identical; observed vs plain actor runs agree
  bitwise; full 6-outcome lifetime stays finite (max |h/a/q| below
  bounds, fraction in [0,1], margin finite, >= 3 ordered samples at
  every 10); summary/selection/samples round-trip through a temp JSON
  file; bad every/empty/short-state configs explicit with no append on
  failure; non-sampled ticks trivially Ok(false); error mapping to
  SimError explicit.
Checks not run / failures / blockers: None. Two initial failures fixed
  without weakening: float margin 0.1999 vs 0.2 moved to 1e-12 tolerance;
  three clippy lints fixed (is_multiple_of, constant assertion removed,
  bool assert). No notebook used.
Configuration and suite hashes: actor_health_test profile (env_smoke
  timing, N=16/m=2/p=0.25, 6 outcomes, no learning/modulator/evolution);
  development namespace, outer 1; trace every 1/2/10 per test; no suites
  consumed.
Artifact paths and checksums where relevant: src/agent/health.rs,
  tests/health.rs (diagnostic JSON only in temp dir, removed; no run
  directories produced; smoke rerun dir removed).
Interpretation and claim limits: Read-only diagnostics only. No dynamics
  change (advance untouched; health_check borrows), no clipping, no
  learning claim. Watchdog enforcement in the runner stays future work;
  M1-11 judges usability from these summaries. M1-GATE remains open.
Tracker boxes updated: M1-08 checked.
Next eligible task: M1-09.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1-09 checkpoint session)
Task IDs: M1-09
Spec sections: 10.7 (full pause/resume contents; genome is not a
  checkpoint), 10.6 (no silent clipping; explicit failures), 20
  (deterministic streams resume exactly), 9 (tick/feedback ordering
  preserved across the split)
Change and affected files: src/checkpoint.rs (new: CHECKPOINT_SCHEMA_VERSION
  1, SeedIdentity, versioned payload plus SHA-256 checksum, config hash,
  atomic temp-plus-rename writes, capture/save/load/restore_env/
  restore_actor with seed-identity and tick-agreement checks, 4 unit
  tests); src/rng.rs (new RngState: seed bytes plus ChaCha8Rng word pos,
  per-tick-local pairing needs nothing more); src/environment/schedule.rs
  (PhaseState serde); src/environment/hidden_state.rs (HiddenSnapshot with
  shape/range validation); src/environment/mod.rs (LifetimeSnapshot with
  countdown/index/pending-phase/count/RNG-identity validation, plus
  snapshot/restore); src/agent/no_learning.rs (AgentSnapshot with shape/
  finiteness/seed-identity validation, snapshot/restore, read-only
  health_check unchanged); src/lib.rs (register checkpoint); Cargo.toml
  (serde_json float_roundtrip feature: correctly-rounded parsing, no lock
  change); tests/checkpoint.rs (new, 7 tests); docs/decisions.md (M1-09
  entry); docs/handoff.md (M1-10 continuation); README.md (layout line).
Code revision / dirty-tree state: base 3f0f0c6 (m1-07 done) with M1-08
  changes uncommitted at start; M1-09 files new or modified and
  uncommitted at handoff (M Cargo.toml, src/agent/no_learning.rs,
  src/environment/{hidden_state,mod,schedule}.rs, src/lib.rs, src/rng.rs,
  to-do.md, docs/decisions.md, docs/handoff.md, README.md;
  ?? src/checkpoint.rs, tests/checkpoint.rs).
Commands actually executed:
  cargo test --locked --test checkpoint (7/7 pass)
  cargo test --all-targets --locked (161 pass, 1 ignored: 32 lib incl. 4
    new checkpoint unit + 0 bin + 7 actor + 7 actor_noise + 5 adaptation
    + 7 baselines + 7 checkpoint + 4 config + 20 contract + 5 logging + 3
    order + 5 leakage + 5 randomized + 5 seeds + 13 topology + 10 weights
    + 9 health + 10 no_learning)
  cargo fmt --all -- --check (clean after cargo fmt)
  cargo clippy --all-targets --locked -- -D warnings (clean after
    too_many_arguments allow on the explicit snapshot restore signature,
    is_multiple_of, and two test-lint fixes)
  python3 analysis/test_validate_logs.py (14/14 pass, unchanged layer)
  cargo run --release --locked -- validate-config
    configs/debug_stationary.toml (OK) and
    cargo run --release --locked -- simulate --config
    configs/env_smoke.toml --baseline oracle --lifetimes 2 --seed 1
    (O1 2 lifetimes, 16/16 outcomes, mean 1.0; fresh run dir removed
    afterward, no new committed artifacts)
Outcome and checks passed: Resume at quiet, just-committed delay, and
  pending-feedback boundaries reproduces the uninterrupted reference
  bitwise (per-tick features/feedback/motor/actions, final h/a/r/q,
  readout, bookkeeping, hidden mappings/rates/roles/exposures, continued
  health summaries) with file save/load on every boundary; both halves
  stay finite; tampered payload, wrong schema, truncation, unknown
  fields, and missing paths fail explicitly; foreign seed identity,
  tick skew, config dim drift, and pending/phase mismatch fail as
  incompatible; non-executable configs cannot capture; failed saves
  leave no partial target and rewrites stay valid.
Checks not run / failures / blockers: None. Two findings fixed without
  weakening: (1) serde_json 1.0.151 default float parsing mis-rounds
  rare decimals by 1 ulp (e.g. 0.20856943026379962), silently corrupting
  weights across save/load — fixed with the float_roundtrip feature
  (verified on the pinned version; no Cargo.lock change); the
  save/load assert_eq is the regression test. (2) Clippy/test lints
  fixed as above; one unit-test expectation narrowed from Incompatible
  to repaired-hash dim drift.
Configuration and suite hashes: checkpoint_test profile (env_smoke
  timing with reward_delay [3,3] for a real Delay phase, N=16/m=2/
  p=0.25, 6 outcomes, no learning/modulator/evolution); development
  namespace, root/outer 1, lifetime 0; checkpoint temp JSON removed;
  no suites consumed.
Artifact paths and checksums where relevant: src/checkpoint.rs,
  tests/checkpoint.rs (checkpoint JSON only in temp dir, removed; no
  run directories produced; smoke rerun dir removed).
Interpretation and claim limits: Exact pause/resume for the nonplastic
  M1 actor only (no P/E/modulator/gates exist to store; files claiming
  them are rejected as unknown fields). Bitwise replay promised on this
  reference platform only. Health summaries stay diagnostic (re-derived,
  not stored). Runner/CLI wiring for checkpoints stays future work;
  M1-10 builds replay regression on this. M1-GATE remains open.
Tracker boxes updated: M1-09 checked.
Next eligible task: M1-10.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1-10 replay session)
Task IDs: M1-10
Spec sections: 9 (tick ordering across replay), 17.7 (replay and
  parallelism: identical seeds reproduce, splits continue, logging
  draws nothing)
Change and affected files: tests/replay.rs (new, 6 tests: platform
  record, reference-trajectory bitwise replay, tick-40 split replay with
  file round-trip, new-q commitment through the runner, all-phase
  continuity, diagnostics-draw-nothing with throwaway captures);
  docs/decisions.md (M1-10 entry); docs/handoff.md (M1-11 continuation).
  No production code change: every contract was proven in its home
  suite; this task consolidates them.
Code revision / dirty-tree state: base 4d6aec1 (m1-08 done) with
  M1-09/M1-10 changes uncommitted at handoff (M Cargo.toml, README.md,
  docs/*, src/*, to-do.md; ?? src/checkpoint.rs, tests/checkpoint.rs,
  tests/replay.rs).
Commands actually executed:
  cargo test --locked --test replay (6/6 pass)
  cargo fmt --all -- --check (clean after cargo fmt)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  (Full suite plus Python audit rerun at M1-GATE.)
Outcome and checks passed: Reference platform pinned (linux/x86_64,
  schema versions 1/1/1) and asserted so a platform move fails loudly;
  same seeds replay per-tick records plus final states bitwise;
  tick-40 split with save/load resumes the reference; all 6
  commitments read new-q argmax (ties: deterministic per seed);
  Quiet/Cue/Gap/Response/Delay/Feedback all visited with finite
  features and non-reset membranes over 100+ ticks; getters, health,
  selection, and unused captures leave the trajectory identical.
Checks not run / failures / blockers: None. Cross-platform tolerance
  comparison is documented policy, not an executed claim.
Configuration and suite hashes: replay_test profile (env_smoke timing
  with reward_delay [3,3], memory_gap [2,2], N=16/m=2/p=0.25, 6
  outcomes); development namespace, root/outer 1, lifetime 0; replay
  temp JSON removed; no suites consumed.
Artifact paths and checksums where relevant: tests/replay.rs (no run
  directories produced).
Interpretation and claim limits: Consolidation only. Bitwise replay
  promised on the reference platform; cross-platform needs declared
  tolerances after per-platform parity. No learning claim. M1-GATE
  remains open.
Tracker boxes updated: M1-10 checked.
Next eligible task: M1-11.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1-11 observability session)
Task IDs: M1-11
Spec sections: 6.5 (observability: distinguishable activity, both
  actions, structural sanity), 10.6 (finiteness, no clipping)
Change and affected files: tests/observability.rs (new, 4 tests:
  2000-tick fixed-input finiteness with health, alternating-cue block
  distinguishability, both-actions reachability over 8 outer seeds with
  per-init health/traces, 64-tick-quiet lifetime finiteness);
  docs/decisions.md (M1-11 entry); docs/handoff.md (M1-12 continuation).
  No production code change.
Code revision / dirty-tree state: base 4d6aec1 (m1-08 done) with
  M1-09/M1-10/M1-11 changes uncommitted at handoff (see git status).
Commands actually executed:
  cargo test --locked --test observability (4/4 pass; one threshold
    corrected from >300 to >200 after hand-computing the exact 248-tick
    cycle arithmetic — no code change)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  (Full suite plus Python audit rerun at M1-GATE.)
Outcome and checks passed: Fixed zeros finite for 2000 ticks with
  watchdog-held health and serializable summaries; cue-0 vs cue-1 block
  means differ more across cues than within (across > within mean
  distance); actions 0 and 1 both committed across 8 initializations
  with per-init finite health/margins and non-empty traces; 64-quiet
  lifetime completes 4/4 outcomes over 248 ticks staying finite.
Checks not run / failures / blockers: None.
Configuration and suite hashes: observability_test profile (env_smoke
  timing, N=16/m=2/p=0.25, 4 outcomes; quiet [64,64] variant for the
  long-quiet case); development namespace, outer 1..=8; no suites
  consumed.
Artifact paths and checksums where relevant: tests/observability.rs
  (no run directories produced).
Interpretation and claim limits: Usable dynamics only — the actor is
  cue-driven, bivalent, and finite. No learning or biological-realism
  claim. M1-GATE remains open.
Tracker boxes updated: M1-11 checked.
Next eligible task: M1-12.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1-12 command session)
Task IDs: M1-12
Spec sections: 13.1 (B3 rung), 18.6 (command interface), 19 (profiles),
  20 (seed namespaces)
Change and affected files: configs/actor_no_learning.toml (new: B3 demo
  profile, env-smoke timing plus debug actor section, no
  learning/modulator/evolution); src/run.rs (BaselineSel::Actor with
  B3(actor-no-learning) ids, actor-guarded dispatch in run_simulation,
  per-lifetime NoLearningActor through run_actor_ordinary, B3 manifest
  note via create_run_dir_with_note, 2 new tests); src/main.rs (actor
  help text); tests/config_validation.rs (new profile validates;
  guard-separation coverage both directions);
  analysis/validate_logs.py (actor-no-learning/B3 policy map) plus
  analysis/test_validate_logs.py (new B3 audit test); README.md
  (commands, B3 rung, guard notes, layout, demo run path).
Code revision / dirty-tree state: base 4d6aec1 (m1-08 done) with
  M1-09 through M1-12 changes uncommitted at handoff (see git status).
Commands actually executed:
  cargo run --release --locked -- validate-config
    configs/actor_no_learning.toml (OK: actor_no_learning, cues=2,
    development)
  cargo run --release --locked -- simulate --config
    configs/actor_no_learning.toml --baseline actor --lifetimes 2
    --seed 1 (run dir runs/actor_no_learning-root1-outer1-1789975911,
    condition B3: 2 lifetimes, 16 commitments, 16 outcomes, mean
    reward 0.5000; dir removed after audit, path recorded here)
  python3 analysis/validate_logs.py
    runs/actor_no_learning-root1-outer1-1789975911
    (OK, events + provenance — after adding the B3 policy map; before
    the fix it failed policy/condition mismatch, which is the
    regression demonstration)
  guard probes: env_smoke+actor rejected (needs [actor]); actor
    profile+random rejected (env-only required) — both explicit
  cargo test --all-targets --locked (173 pass, 1 ignored)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  python3 analysis/test_validate_logs.py (15/15 pass, incl. new B3 test)
  cargo test --locked --test checkpoint (7/7, rerun for the record)
Outcome and checks passed: Reader-reproducible B3 demo (validate +
  simulate + audit commands above); B3 manifest/condition/event streams
  share the M0 schema (one audit covers all rungs); execution guards
  separate env-only from actor profiles in both directions through CLI
  and library; checkpoint continuation stays documented via
  cargo test --locked --test checkpoint (dedicated CLI arrives later).
Checks not run / failures / blockers: None. Observability and health
  requirements from M1-08/M1-11 all pass, so no blocker carries.
Configuration and suite hashes: actor_no_learning profile (8 outcomes,
  seeds root 1/development/outer 1); demo lifetimes 0..1; no suites
  consumed.
Artifact paths and checksums where relevant:
  configs/actor_no_learning.toml (committed); demo run dir removed
  after audit with path/results recorded here and in README.md.
Interpretation and claim limits: Runnable B3 demo only. No learning,
  no gates, no search; B3 is the same-actor control, not B7.
  M1-GATE remains open.
Tracker boxes updated: M1-12 checked.
Next eligible task: M1-GATE.
```

```text
Date / agent or session: 2026-09-21 UTC / agent (M1-GATE exit session)
Task IDs: M1-GATE
Spec sections: 16/M1 (exit conditions), 6.5 (observability), 10.6
  (bounds/failures), 17.2/17.7 (unit and replay checks)
Change and affected files: Evidence only — no behavior change in this
  entry. Milestone work lives in M1-01 through M1-12 entries below.
Code revision / dirty-tree state: base 4d6aec1 (m1-08 done) with
  M1-09 through M1-GATE changes uncommitted at handoff (see git
  status); behavior code unchanged by the gate entry itself.
Commands actually executed (fresh gate battery, this session):
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (173 pass, 0 failed, 1 ignored:
    34 lib + 0 bin + 7 actor + 7 actor_noise + 5 adaptation + 7
    baselines + 7 checkpoint + 4 config_validation + 20
    environment_contract + 5 event_logging + 3 event_order + 9 health
    + 5 leakage + 7 motor + 10 no_learning + 4 observability + 5
    randomized_env + 6 replay + 5 seed_streams + 13 topology + 10
    weights)
  python3 analysis/test_validate_logs.py (15/15 pass)
  validate-config on configs/env_smoke.toml, debug_stationary.toml,
    actor_no_learning.toml (all OK, cues=2, development)
  simulate env_smoke.toml --baseline oracle --lifetimes 2 --seed 1
    (runs/env_smoke-root1-outer1-1789976257: O1, 16/16 outcomes, mean
    1.0000; audit OK events + provenance; dir removed after audit)
  simulate actor_no_learning.toml --baseline actor --lifetimes 2
    --seed 1 (runs/actor_no_learning-root1-outer1-1789976257: B3,
    16/16 outcomes, mean 0.5000; audit OK events + provenance; dir
    removed after audit)
Outcome and exit-criteria check:
  - Simultaneous-update and noise contracts obeyed: one-edge /
    reciprocal fixtures prove receiver orientation and old-state reads;
    forced perturbations prove post-integration noise; per-tick draws
    match the reference stream with input/saturation independence
    (tests/actor.rs, tests/actor_noise.rs).
  - State continuously preserved: full lifetimes advance every phase
    with no boundary resets and bit-identical W0; logging/diagnostics
    draw nothing (tests/no_learning.rs, tests/replay.rs).
  - Usable cue/motor responses across seeds: alternating cues
    distinguishable (across > within), actions 0 and 1 both reachable
    over 8 initializations, per-init finite health/margins/traces
    (tests/observability.rs).
  - Finite in the declared smoke run: B3 demo completes 16/16
    outcomes, watchdog-held, audit-clean; 2000-tick fixed-input and
    64-quiet runs finite (tests/health.rs, tests/observability.rs).
  - Exactly resumes on the reference platform: quiet/response/
    feedback splits plus tick-40 split replay bitwise with file
    round-trips; corruption/incompatibility explicitly rejected
    (tests/checkpoint.rs, 4 lib unit tests).
  - Adaptation inert at strength 0 with signed nonzero fixture;
    motor golden recurrences with tie-only draws
    (tests/adaptation.rs, tests/motor.rs).
  - M0 contracts unchanged: all environment/order/leakage/randomized/
    baseline/config/logging/seed suites pass; golden topology/weight
    values pinned.
Checks not run / failures / blockers: None. No null result to report:
  this milestone is a dynamical-system demonstration by design, not
  learning — that interpretation is the exit claim, with B3 labeled a
  same-actor control (not B7).
Configuration and suite hashes: env_smoke + actor_no_learning smoke
  profiles (development, root/outer 1); gate reruns used lifetimes 2,
  outcomes 8/8 per lifetime; no suites consumed.
Artifact paths and checksums where relevant: No new artifacts in this
  entry; runnable demo is configs/actor_no_learning.toml plus the
  README commands (verified above); replay proof is
  cargo test --locked --test checkpoint (7/7) and --test replay (6/6).
Interpretation and claim limits: M1 exits as a verified
  dynamical-system foundation for M2 (stochastic-score verification).
  No plasticity, gating, search, or learning is claimed.
Tracker boxes updated: M1-GATE checked.
Next eligible task: M2-01.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex owner-requested M1 review
Task IDs: M1-REVIEW (M1-01 through M1-12 and M1-GATE re-verification)
Spec sections: 3-10, 16/M1, 17.2, 17.7, 18-20
Change and affected files: src/agent/{actor,health,motor,no_learning,topology,
  weights}.rs; src/{checkpoint,rng,run}.rs; environment snapshot types;
  tests/{checkpoint,health,motor,observability,replay,seed_streams}.rs;
  README, handoff, review, decisions/experiments and evidence bundle.
  Fixed checkpoint relabelling/validation, complete-state restore and atomic
  concurrent saves; enforced the existing finite watchdog in production;
  saved health/traces/initialization history; validated supplied parameters
  and equal-sized unique motor pools. No scientific equation/seed change.
Code revision / dirty-tree state: clean base 13a4873; review changes
  uncommitted. Source and binary SHA-256s in the evidence summary.
Commands actually executed:
  cargo fmt --all -- --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo test --all-targets --locked
    with CRA_M1_EVIDENCE_DIR=docs/evidence/m1-review/observability
    (185 passed, 0 failed, 1 existing ignored weight-printing probe)
  python3 analysis/test_validate_logs.py (15 passed)
  cargo build --release --locked
  validate-config on env_smoke, debug_stationary, actor_no_learning (OK)
  11 corrected simulate runs + Python audits: eight clean B3 runs at
    outer 1..8, O1, noisy variable-timing B3 and its logging-off pair;
    all root 1/development/lifetimes 0..1. Exact argument lists saved.
  Build original git archive 13a4873 separately; two additional clean/noisy
    B3 comparison runs audited; ordinary/hidden records exactly equal to
    corrected runs excluding run IDs.
  Focused regression runs; before-fix checkpoint failures retained in
    docs/evidence/m1-review/regressions-before.txt.
  git diff --check (clean); local Markdown link/consistency check.
Outcome and checks passed: Dynamics/goldens/replay preserved; checkpoint
  tests now reject wrong live identity, incomplete ticks, nested unknown or
  missing state and inconsistent halves; exact continuation at every completed
  boundary of variable-timing odd-sized lifetimes. Concurrent save test passes.
  Watchdog failure is explicit and saved as interrupted. Logging-off health
  matches logging-on. Across/within cue distances 1.8681/0.2530; actions 20/12
  across 8 four-choice initializations; 2000 zero-input ticks finite; long
  quiet exactly 248 ticks. Fresh demo 16/16 outcomes, mean reward 0.5000.
Checks not run / failures / blockers: One existing ignored weight-printing
  probe remains unrun; not a milestone diagnostic. Earlier deleted M1 raw
  directories could not be re-audited; fresh runs preserved instead. No
  unresolved M1 blocker. No cross-platform study/search/final-test inspection.
Configuration and suite hashes: See docs/evidence/m1-review/summary.json;
  noisy_variable.toml and noisy_logging_off.toml committed beside evidence.
Artifact paths: docs/m1-review.md; docs/evidence/m1-review/{commands,summary,
  demo-diagnostics}.json and observability/*.json; ignored full raw output
  runs/m1-review/ and runs/m1-review-original/ (preserved).
Interpretation and claim limits: Dynamics/replay only; some individual actors
  always choose one action in the short demo. No learning claim. Checkpoint
  and health schema 2 supersede schema 1; event schema remains 1. Old checkpoints
  are explicitly rejected, not silently migrated. Spec and history preserved.
Tracker boxes updated: M1-GATE stays checked after corrective re-verification.
Next eligible task: M2-01.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex M2-01
Task IDs: M2-01
Spec sections: 7.1-7.2, 7.6, 16/M2, 17.2-17.3
Change and affected files: src/agent/score.rs exposes conditional_score
  with explicit receiver alpha_h/xi/sigma and old sender activity; module
  export in src/agent/mod.rs; tests/score.rs; README, handoff and tracker.
  Pure per-edge arithmetic, no state/RNG access, trace or learning update.
  Finite inputs, alpha_h in (0,1], positive sigma and finite result required.
Code revision / dirty-tree state: clean base 0755a4c (refined m1);
  this task's changes uncommitted. spec.md unchanged.
Commands actually executed:
  cargo test --locked --test score (8 passed)
  cargo fmt --all -- --check (first found two wrapping differences)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (193 passed, 0 failed, 1 ignored)
  cargo fmt --all (applied the two formatting wraps)
  cargo fmt --all -- --check (clean)
Outcome: Golden score 0.4; heterogeneous receiver rows prove shared xi[j]
  and receiving parameters; leak/noise scaling has exactly one alpha and
  actual sigma, without a floor. Saturated activity retains nonzero score
  (no tanh derivative); zero activity/noise realization gives zero score.
  Invalid/nonfinite inputs, nonpositive sigma even with zero numerator,
  and score overflow return explicit errors rather than clipping.
Checks not run / failures / blockers: No blocker. Existing ignored
  weight-printing probe not invoked. Python analysis and standalone CLI
  smoke not repeated: no analysis, runner, dynamics or schema changes.
  M2-02 onward derivative/Monte Carlo diagnostics remain unimplemented.
Configuration and suite hashes: No new simulation/config/seed suite;
  score fixtures are deterministic constants in tests/score.rs.
Artifact paths: src/agent/score.rs and tests/score.rs; this ledger records
  the executed verification and outcomes. Existing regression goldens intact.
Interpretation and claim limits: M2-01 arithmetic only; M2-GATE remains
  open. No continual-learning or unbiased online-gradient claim.
Tracker boxes updated: M2-01 checked.
Next eligible task: M2-02.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex M2-02 continuation
Task IDs: M2-02
Spec sections: 6.1, 7.1-7.2, 7.6, 16/M2, 17.4
Change and affected files: tests/score_log_probability.rs; README, handoff,
  tracker and docs/evidence/m2-02/summary.md. No production/spec changes.
Code revision / dirty-tree state: clean base 9d31b9d (m2-01 done);
  task changes uncommitted. Source SHA-256s in the saved evidence.
Commands actually executed:
  cargo fmt --all
  cargo test --locked --test score_log_probability -- --nocapture (3 pass)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (196 passed, 0 failed, 1 ignored)
Outcome: 288 fixed-sample derivatives pass: 12 edges x 6 tau/sigma settings
  x 4 epsilon values. Largest absolute difference 3.790982994190e-8;
  predeclared tolerance 2e-8 + 2e-7*abs(score). No tolerance adjustment.
  Gaussian-density known values pass. Invalid moving-sample control has
  near-zero derivative, while the fixed-sample score is 0.42. Old state,
  original weights, sampled h_new and unaffected receivers are preserved.
Checks not run / failures / blockers: No failures/blockers. Existing ignored
  weight-printing probe unrun; Python/standalone CLI smoke not repeated for
  this test-only change. M2-03 onward Monte Carlo work remains queued.
Configuration and suite hashes: Deterministic explicit fixture; no RNG
  draws or seed namespace consumed. Parameters/epsilons and source hashes
  saved in docs/evidence/m2-02/summary.md.
Artifact paths: tests/score_log_probability.rs; docs/evidence/m2-02/summary.md.
Interpretation and claim limits: Conditional derivative only; M2-GATE open,
  no online-learning or convergence claim and no scientific deviation.
Tracker boxes updated: M2-02 checked.
Next eligible task: M2-03.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex M2-03 continuation
Task IDs: M2-03
Spec sections: 7.2, 7.6, 16/M2, 17.5
Change and affected files: tests/score_learning_direction.rs; predeclared
  plan, result.json and summary under docs/evidence/m2-03/; README, handoff,
  experiments and tracker. No production, dependency or spec changes.
Code revision / dirty-tree state: base 9d31b9d (m2-01 done), with prior
  M2-02 changes preserved; M2-03 changes uncommitted. Result records source
  SHA-256s, actual dirty status, toolchain and linux/x86_64 platform.
Commands actually executed:
  cargo fmt --all
  cargo test --locked --test score_learning_direction (3 pass, 1 ignored)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (199 pass, 0 fail, 2 default ignores)
  CRA_M2_DIRECTION_EVIDENCE=docs/evidence/m2-03/result.json cargo test
    --release --locked --test score_learning_direction
    one_neuron_learning_direction -- --ignored --exact --nocapture
    (1 pass, 0 fail; 0.07s reported test time, 7.24s compilation)
Outcome: One million samples, positive mean 0.13871680603781847 versus
  analytic 0.1388622064964956; SE 0.00010644926671077609. Error
  0.0001454004586771418 < five-SE-plus-1e-12 tolerance
  0.0005322463345538805. Paired opposite-target mean exactly negated,
  with same SE. First execution passed; no reseeding/tolerance changes.
  Fast tests check Welford variance/SE against hand values, constant terms,
  forced reward/score cases and the spec's analytical approximation.
Checks not run / failures / blockers: None blocking. Expensive diagnostic
  is ignored by default and was explicitly run above. Existing ignored
  weight-printing probe unrun. Python and standalone CLI smoke not rerun
  for this test-only change. M2-04 onward remain incomplete.
Configuration and suite hashes: alpha=.2, input=.7, weight=.3, sigma=.4,
  baseline=.5; development/root1/outer203/lifetime0/actor_noise. Existing
  NormalStream retains paired spare across independent samples. Full seed,
  SHA-256 identities, counts, statistics and pass flags in result.json.
Artifact paths: docs/evidence/m2-03/{plan,summary}.md and result.json;
  tests/score_learning_direction.rs. Export rejects existing file paths.
Interpretation and claim limits: One-transition direction diagnostic, no
  online updates or convergence claim. M2-GATE open; no scientific deviation.
Tracker boxes updated: M2-03 checked.
Next eligible task: M2-04.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex M2-04
Task IDs: M2-04
Spec sections: 7.2, 7.6, 9, 10.5, 16/M2, 17.6
Change and affected files: src/experiments/{mod,finite_rollout}.rs;
  tests/finite_rollout.rs; README, handoff, decisions, tracker and evidence.
  Isolated diagnostic reuses unchanged actor/score. No production runner,
  config, RNG derivation, schema, dependency or spec changes.
Code revision / dirty-tree state: base 9d31b9d, prior uncommitted M2-02/03
  preserved; M2-04 uncommitted. Source and artifact hashes in summary.md.
Commands actually executed:
  cargo fmt --all
  cargo test --locked --test finite_rollout (8 passed after fixture fix)
  cargo test --locked --doc (initially 1, finally 2 compile-fail tests pass)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  CRA_M2_ROLLOUT_EVIDENCE=docs/evidence/m2-04/golden.json cargo test
    --all-targets --locked (207 passed, 0 failed, 2 default ignores)
Outcome: Frozen weights/noise/baseline, weight-independent zero state,
  exact score sums with no decay, terminal-only reward and at most one
  optional unclipped update to a W0 copy. Explicit reset between completed
  independent rollouts, not a production reset-policy reinterpretation.
  Golden 3-tick fixture: edge sum 0.4, reward-weighted score 0.3, copied
  terminal weight 3. Missing edges and original W0 remain unchanged.
  Early/duplicate finish, extra steps and live parameter/baseline mutation
  rejected. Numerical failures cannot be finalized/retried/reset. Direct
  16-tick recurrent actor and RNG parity holds with manual score accumulation.
Checks not run / failures / blockers: Initial focused compile exposed two
  test-only array repetitions of Vec; fixed to vector repetition. No changed
  equations/tolerances or blocker. M2-03 Monte Carlo and weight-printing probe
  remain ignored in fast suite and were not rerun here. No standalone CLI
  smoke/Python analysis rerun; no continuous runner/analysis change.
Configuration and suite hashes: Golden fixture fully saved in golden.json,
  injected perturbations (no seed). Stochastic parity uses development/
  root1/outer204/lifetime0/actor_noise. Source and output hashes in summary.
Artifact paths: docs/evidence/m2-04/{summary.md,golden.json};
  src/experiments/finite_rollout.rs; tests/finite_rollout.rs.
Interpretation and claim limits: Restricted diagnostic infrastructure only;
  M2-05 recurrent Monte Carlo check still required. M2-GATE open. No learning
  claim or scientific deviation; interface rationale in docs/decisions.md.
Tracker boxes updated: M2-04 checked.
Next eligible task: M2-05.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex M2-04 provenance addendum
Task IDs: M2-04
Final Git inspection: HEAD is now 72ad075 (m2-03), which committed the
  prior M2-02/03 work during this session. The base 9d31b9d in the entry
  above denotes the session start. M2-04 files remain uncommitted.
Verification: Python stdlib evidence check revalidated all saved source
  and golden-output SHA-256s, golden numeric results and local Markdown
  links; all match. git diff --check clean. No tested behavior changed.
Next eligible task: M2-05 (unchanged).
```

```text
Date / agent or session: 2026-09-21 UTC / Codex M2-05
Task IDs: M2-05
Spec sections: 7.1-7.2, 7.6, 9, 10, 17.6
Change and affected files: tests/score_recurrent.rs; README, handoff,
  experiments, tracker; docs/evidence/m2-05/. No production/spec/dependency
  or schema changes. Existing frozen-weight rollout harness reused.
Code revision / dirty-tree state: clean base 1e6fbf2; M2-05 uncommitted.
Commands actually executed:
  cargo fmt --all
  cargo test --locked --test score_recurrent (5 fast passes, 1 ignored)
  CRA_M2_RECURRENT_EVIDENCE=docs/evidence/m2-05/result.json cargo test
    --release --locked --test score_recurrent
    two_neuron_recurrent_finite_difference -- --ignored --exact --nocapture
    (1 pass, first Monte Carlo execution; 15.35s test / 7.71s compile)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (212 passed, 0 failed, 3 ignored)
  cargo test --locked --doc (2 compile-fail API checks passed)
Outcome: Score 0.41802543 (SE 0.00069159); finite differences 0.41825625,
  0.42040000, 0.41765000 at epsilon .04/.02/.01. All paired comparisons
  pass predeclared five-SE-plus-.0005 tolerance and <=.02 five-SE width
  limit; positive score/FD lower bounds. No reseeding/tolerance changes.
  Five fast tests cover statistics/precision guard, frozen weights,
  recurrence/reset assumptions, exact paired replay and failure accounting.
Checks not run / failures / blockers: M2-03 slow diagnostic and existing
  weight-printing probe not rerun; M2-05 slow diagnostic explicitly passed.
  Initial failure-accounting test used a large finite drive that correctly
  stayed finite; corrected fixture to force overflow with two f64::MAX
  summands. No production change. No blocker. Python/CLI smoke not rerun
  for this test-only change. M2-06 and M2-GATE remain incomplete.
Configuration and suite hashes: two neurons with both cross edges, six
  ticks, fixed W=[[0,-.4],[.3,0]], alpha=.5, sigma=.4, input B=[.7,-.2],
  zero birth state/baseline, no decay/updates, terminal 1[h[1]>0]. Exactly
  2,000,000 independent paired samples; development/root1/outer205/
  lifetime0/actor_noise. Seven settings, 14,000,000 rollouts, 84,000,000
  transitions, 24,000,000 independent normals, one worker, zero failures.
  Source/plan/lock SHA-256 identities and complete settings in result.json.
Artifact paths: docs/evidence/m2-05/{plan,summary}.md, result.json,
  diagnostic.log, checks.json and check-{1,2,3,4}.log. Output refuses overwrite.
Interpretation and claim limits: finite-horizon fixed-weight validation
  only, not online learning/convergence/unbiased lifetime gradients.
  Epsilon comparisons are correlated, not independent replications.
Tracker boxes updated: M2-05 checked. No scientific-contract deviation.
Next eligible task: M2-06.
```

```text
Date / agent or session: 2026-09-21 UTC / Codex M2-06
Task IDs: M2-06, M2-GATE
Spec sections: 7.1-7.2, 7.6, 16/M2, 17.4-17.6
Change and affected files: scripts/run_score_diagnostics.sh; README,
  handoff, experiments, evidence index, tracker; docs/evidence/m2-06/.
  Wrapper packages existing Rust tests; no numerical/production/dependency,
  RNG policy, schema or spec changes. Prior artifacts preserved.
Code revision / dirty-tree state: session start 1e6fbf2 with M2-05 staged;
  prior work committed during session as 28bcdd5. All package stages record
  28bcdd57ccf78c7e8294f7bf0ec4cb4f978fe40b plus dirty M2-06 work.
Commands actually executed:
  bash -n scripts/run_score_diagnostics.sh (pass)
  Python stdlib wrapper controls (four pass: usage, fake-Cargo failure
    propagation, directory reuse without mutation, zero-test rejection)
  bash scripts/run_score_diagnostics.sh docs/evidence/m2-06/suite
    fast: cargo test --locked --test score --test score_log_probability
      --test score_learning_direction --test finite_rollout
      --test score_recurrent -- --nocapture (27 pass, 2 MC ignores)
    api: cargo test --locked --doc (2 compile-fail checks pass)
    direction: cargo test --release --locked --test score_learning_direction
      one_neuron_learning_direction -- --ignored --exact --nocapture
      (1 pass; 1,000,000 samples, 0.08s)
    recurrent: cargo test --release --locked --test score_recurrent
      two_neuron_recurrent_finite_difference -- --ignored --exact --nocapture
      (1 pass; 2,000,000 trajectory groups, 14.69s)
    Output env variables route exports to fresh suite/*.json; exact expanded
    commands and exit codes are in suite/commands.txt and checks.tsv.
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (212 passed, 0 failed, 3 ignores)
  Python stdlib source/hash/result audit (original numeric/seed/config/count
    parity, golden parity, actual test totals all verified; audit.json)
Outcome: Every M2 prerequisite actually rerun and passed. Direction mean
  .1387168060, analytical .1388622065, SE .0001064493; recurrent score
  .4180254284, SE .0006915851; three finite differences .41825625/.4204/
  .41765 pass original uncertainty/precision bounds. 288 conditional-density
  comparisons pass. Exact same-seed reproduction, not independent evidence.
Checks not run / failures / blockers: No real diagnostic failures/blockers.
  Fake failures intentional wrapper controls. Existing ignored weight-print
  probe unrun; both ignored MC diagnostics explicitly pass. Unrelated Python
  log-analysis and CLI simulator smoke not rerun for orchestration-only work.
Configuration and suite hashes: original M2-03/05 plans and fixtures copied
  into suite; no tolerances/seeds changed. Development/root1/outer203 or 205/
  lifetime0/actor_noise. Combined MC 85,000,000 transitions and 25,000,000
  independent normal draws, one worker. Source/artifact hashes in suite/
  source.sha256, per-diagnostic JSON and audit.json.
Artifact paths: docs/evidence/m2-06/{plan,summary}.md, suite/, quality.json
  with quality-{1,2,3,4}.log, wrapper-checks.json and audit.json.
Interpretation and claim limits: conditional score and restricted fixed-weight
  rollout validated. No unbiasedness/convergence guarantee for online
  decaying/clipped/gated learning, no acquisition claim or M3 implementation.
Tracker boxes updated: M2-06 and M2-GATE checked after fresh verification.
Next eligible task: M3-01.
```

```text
Date / agent or session: 2026-09-21 / opencode (M3-01 session)
Task IDs: M3-01
Spec sections: 7.3 (persistent eligibility), 7.5 (offsets/effective weights),
  7.7 (event-reset diagnostic contrast), 10.3 (plastic mask), 10.4 (birth
  P = E = 0), 17.2 (missing/nonplastic never accrue; one cache location)
Change and affected files: src/agent/plasticity.rs (new: PlasticState storing
  P/E separately from immutable W0, PlasticMaskKind {all_recurrent_edges,
  motor_afferent_only}, TracePolicy {persistent, no_decay_diagnostic}, single
  refresh_effective cache writer, versioned PlasticSnapshot + restore
  validation); src/agent/actor.rs (new step_with_effective_weights and
  step_with_effective_and_perturbations sharing the W0 arithmetic core via
  advance_new_with_recurrence; step/step_with_perturbations delegate through
  &weights.w0); src/agent/mod.rs, src/lib.rs (module wiring/scope docs);
  tests/plasticity.rs (new, 16 tests); README.md, docs/{decisions,handoff}.md,
  docs/evidence/m3-01/summary.md, to-do.md (docs/evidence only).
Code revision / dirty-tree state: base 38c4ffd clean at start; M3-01 files
  untracked/modified at run time (dirty=true in any fresh manifest).
Commands actually executed:
  cargo fmt --all
  cargo test --locked --test plasticity (16 passed)
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (232 passed, 0 failed, 3 ignored)
  cargo test --locked --doc (2 compile-fail checks passed)
Outcome and checks passed: P/E birth zero with effective == W0 for both masks
  and both trace policies; masks are structural subsets with motor-only
  restricted to motor receivers; eligibility uses receiver xi on all incoming
  plastic edges, one alpha, actual sigma, rejects zero/invalid noise, zero r
  gives zero score; spec 17.3 eligibility arithmetic gives 0.67 (persistent,
  lambda 0.9) versus 0.70 (no-decay), proving the policies are distinct;
  nonzero P/E on missing/nonplastic edges rejected at construct/advance/
  refresh/restore; snapshot JSON round-trip plus explicit rejection of wrong
  schema, dimensions, mask disagreement, unknown policy, bad tau_e,
  unknown/missing fields, and nonfinite P/E; effective actor path reproduces
  step bitwise at P = 0 and applies validated offsets without touching B/bias;
  W0 never mutated. Hash of verified behavior in the evidence record.
Checks not run / failures / blockers: No CLI smoke or Python audit rerun:
  this task adds storage and an actor entry point but enables no new
  production execution mode (simulate guards still reject enabled learning).
  Slow M2 Monte Carlo diagnostics not rerun (unchanged). No failures.
Configuration and suite hashes: configs/{debug_stationary,actor_no_learning,
  env_smoke}.toml unchanged; no seeds consumed (unit/fixture tests only,
  development namespace where used).
Seed namespace / outer seeds / lifetime count: No lifetimes simulated; only
  deterministic fixtures. No new seeds consumed.
Artifact paths and checksums where relevant: docs/evidence/m3-01/summary.md;
  source hashes for src/agent/plasticity.rs (2c9909c3...), src/agent/actor.rs
  (08b9a7b4...), tests/plasticity.rs (53b82f91...) recorded there.
Interpretation and claim limits: Storage and eligibility only. No reward
  update, baseline, acquisition, or continuous-learning claim. The
  no_decay_diagnostic policy is a named diagnostic and is rejected under
  birth_only by config validation. Top-level checkpoint schema stays 2; the
  versioned PlasticSnapshot is not yet embedded and split replay with
  nonzero P/E is M3-10/M4-06. M3-02 next.
Tracker boxes updated: M3-01 checked after verification; status/ownership/
  ledger updated.
Next eligible task: M3-02.
```

```text
Date / agent or session: 2026-09-21 / opencode (M3-02 session)
Task IDs: M3-02
Spec sections: 7.4 (teaching signal/running baseline), 7.5 (gated update,
  ordered clamps, mask restriction), 9 step 2 (exactly-once pre-transition
  consumption), 10.4 (birth baseline 0.5), 10.6 (explicit errors), 17.2
  (zero-change cases, clipping order, W0 invariance, no trace reset)
Change and affected files: src/agent/plasticity.rs (extended: baseline +
  dedup lifetime state, apply_feedback_once with per-call validated
  eta/max_update/bound/beta/gates, FeedbackOutcome raw/limited/actual
  report, DuplicateFeedback error, snapshot schema 2 with baseline +
  required last_feedback, from_learning_config baseline carriage);
  tests/feedback_updates.rs (new, 11 tests); README.md,
  docs/{decisions,evidence/README,handoff}.md, docs/evidence/m3-02/summary.md,
  to-do.md (docs/evidence only).
Code revision / dirty-tree state: base 3fdc541 clean at start; M3-02 files
  modified/new at run time (dirty=true in any fresh manifest).
Commands actually executed:
  cargo test --locked --test feedback_updates (11 passed)
  cargo test --locked --test plasticity (16 passed)
  cargo fmt --all
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (243 passed, 0 failed, 3 ignored)
  cargo test --locked --doc (2 compile-fail checks passed)
Outcome and checks passed: delta from old baseline with one post-delta
  update and chained second-outcome deltas; duplicate and older ids rejected
  with bit-identical P/E/baseline/cache; eta=0, all-zero gates, and delta=0
  move no P; max_update clipping separates raw/limited on both signs;
  plastic_bound clipping separates limited/actual on both bounds; caller W0
  unchanged with effective==W0+P; motor-only nonplastic zero reports;
  per-receiver gate sharing with doubling; all invalid reward/gate/
  hyperparameter/w0 inputs rejected without state change; snapshot v2
  round-trip with nonzero P/baseline/dedup plus post-restore dedup.
Checks not run / failures / blockers: No CLI smoke or Python audit rerun:
  this task adds a unit-level entry point but enables no new production
  execution mode (simulate guards still reject enabled learning; no runner
  calls apply_feedback_once). Slow M2 Monte Carlo diagnostics not rerun
  (unchanged). No failures.
Configuration and suite hashes: configs/{debug_stationary,actor_no_learning,
  env_smoke}.toml unchanged; no seeds consumed (unit/fixture tests only,
  development namespace where used).
Seed namespace / outer seeds / lifetime count: No lifetimes simulated; only
  deterministic fixtures. No new seeds consumed.
Artifact paths and checksums where relevant: docs/evidence/m3-02/summary.md;
  source hashes for src/agent/plasticity.rs (e9975e72...), 
  tests/feedback_updates.rs (4bf26bcc...) recorded there.
Interpretation and claim limits: Update arithmetic and exactly-once
  bookkeeping only. No runner, learned gates, or acquisition/control claim.
  The 17.3 golden is M3-03; the episodic runner is M3-04; top-level
  checkpoint schema stays 2 with embedding/replay at M3-10/M4-06.
Tracker boxes updated: M3-02 checked after verification; status/ownership/
  ledger updated.
Next eligible task: M3-03.
```

```text
Date / agent or session: 2026-09-21 / opencode (M3-03 session)
Task IDs: M3-03
Spec sections: 17.3 (golden score/eligibility/update), 17.2 (clipped vs
  unclipped separation, W0 invariance)
Change and affected files: tests/golden_updates.rs (new, 4 tests:
  golden score 0.4, eligibility 0.67, unclipped feedback chain
  0.4/0.00067/0.10067/0.64, separate per-edge and bound clamps);
  README.md, docs/{decisions,evidence/README,handoff}.md,
  docs/evidence/m3-03/summary.md, to-do.md (docs/evidence only; no
  production change).
Code revision / dirty-tree state: base 3fdc541 with uncommitted M3-02/M3-03
  work in tree at run time (dirty=true in any fresh manifest).
Commands actually executed:
  cargo test --locked --test golden_updates (4 passed)
  cargo fmt --all
  cargo fmt --all -- --check (clean)
  cargo clippy --all-targets --locked -- -D warnings (clean)
  cargo test --all-targets --locked (247 passed, 0 failed, 3 ignored)
  cargo test --locked --doc (2 compile-fail checks passed)
Outcome and checks passed: score/eligibility/delta/raw/P/baseline golden
  at 1e-12-1e-15 through public entry points with explicit alpha/lambda;
  old baseline produces delta with one post-delta update; clipped cases
  distinct from unclipped on both clamp kinds. One initial overly strict
  assert_eq! (actual vs limited, 1-ulp add/subtract round-trip) fixed to a
  documented 1e-15 close; no method change.
Checks not run / failures / blockers: No CLI smoke or Python audit rerun:
  fixture-only task with no production execution change. Slow M2 Monte Carlo
  diagnostics not rerun (unchanged). No failures beyond the fixed assertion.
Configuration and suite hashes: configs unchanged; no seeds consumed
  (deterministic fixtures only).
Seed namespace / outer seeds / lifetime count: No lifetimes simulated.
Artifact paths and checksums where relevant: docs/evidence/m3-03/summary.md;
  tests/golden_updates.rs (7b81be19...) recorded there.
Interpretation and claim limits: Arithmetic chain only. No runner,
  controls, or acquisition claim. M3-04 episodic runner next.
Tracker boxes updated: M3-03 checked after verification; status/ownership/
  ledger updated.
Next eligible task: M3-04.
```

## Blockers and decision register - keep current

No blockers. M1-GATE re-verified after the 2026-09-21 UTC owner-requested
corrective review: 185 Rust passes (1 ignored probe), 15 Python passes,
clean fmt/Clippy, eleven corrected audited runs and two original comparisons.
M2-GATE passed after M2-06 package execution: 27 fast score diagnostics,
two compile-fail docs and both explicit Monte Carlo diagnostics pass with
original seeds/tolerances. Full 212-test suite and fmt/Clippy pass. Three
default ignores: both Monte Carlo checks passed separately this session;
existing weight-printing probe unrun.
M3-01 verified 2026-09-21 UTC: 16 new plasticity tests and the full 232-test
fast suite plus 2 compile-fail docs pass; plastic offsets, eligibility, both
masks, and the single effective-weight refresh exist. This is storage only:
no reward update or acquisition evidence yet.
M3-02 verified 2026-09-21 UTC: 11 new feedback-update tests and the full
243-test fast suite plus 2 compile-fail docs pass; exactly-once gated `P`
updates, running baseline, separated raw/limited/actual reports, and
snapshot v2 exist. This is unit-level arithmetic: no runner, gate, or
acquisition evidence yet.
M3-03 verified 2026-09-21 UTC: 4 new golden-fixture tests and the full
247-test fast suite plus 2 compile-fail docs pass; Section 17.3 chain and
separate clipped cases exist. Fixture only: no acquisition evidence yet.
Next: M3-04 (explicitly episodic clean-learning runner).
M1 findings, corrections and claim limits: `docs/m1-review.md`.
M0 historical evidence remains in `docs/m0-review.md`.
Scientific decisions remain in the append-only `docs/decisions.md`.

## First meaningful success

A tiny recurrent agent learns two initially unknown cue-action associations from delayed outcomes, then continues learning with no within-lifetime neural-state resets. Reach that at M4 before treating modulation or evolution as the main achievement.
