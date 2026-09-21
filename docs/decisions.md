# Decisions

Append-only record of scientific ambiguities, approved deviations, and
engineering choices. Each entry cites affected spec sections, alternatives
considered, and consequences. Workflow additions here (evidence records,
quality commands, run-dir conventions) are engineering choices, not new
scientific requirements. **Never silently revise a scientific contract to
make code or a result look successful.**

## 2026-09-21 — M0 bootstrap choices (M0-01–M0-05)

- **Toolchain pin (spec 20, AGENTS.md).** Pinned Rust `1.98.0` via
  `rust-toolchain.toml`; dependency resolution in committed `Cargo.lock`
  (`clap 4.6.7`, `serde 1.0.229`, `toml 0.8.23`, `rand 0.9.5`,
  `rand_chacha 0.9.0`, `sha2 0.10.9`). Python interpreter `3.14.7` recorded
  in README; no third-party Python deps yet. Alternative (floating stable)
  rejected: not a reproducibility pin.
- **Seed derivation (spec 20.5, M0-04).** Canonical string
  `cra-v1|root={}|ns={}|outer={}|lifetime={}|stream={}` hashed with SHA-256;
  digest seeds `ChaCha8Rng`. Namespaces
  `development|training|validation|final_test` strictly validated; streams
  require `[a-z0-9_]` (≤64 chars) with nine reserved names
  (`cue_order`, `mapping_init`, `mapping_change`, `reward_noise`, `timing`,
  `actor_noise`, `tie_break`, `init`, `evolution`), one RNG instance per
  stream. Alternative (SeedableRng::seed_from_u64 with mixed integers)
  rejected: less auditable canonical form. Consequence: derivation format is
  a reproducibility contract; changes require a domain bump + migration note.
- **Config schema (spec 19, M0-05).** `schema_version = 1` only; required
  sections `simulation|environment|logging|seeds`, optional
  `actor|learning|modulator|evolution` (validated when present, executed in
  later milestones). All structs `deny_unknown_fields`. Added explicit
  `[seeds]` (`root_seed`, `namespace`, `outer_seed`) since the spec's example
  profiles omit the seed source while 19.4/20.5 require it; CLI `--root-seed`
  / `--outer-seed` overrides are recorded per-field as `config`- vs
  `cli`-sourced. `plastic_decay != 0.0` rejected (no hidden forgetting);
  `birth_only` requires `trace_policy = "persistent"`.
- **Smoke profiles (M0-05).** `configs/env_smoke.toml` (env-only, no neural
  sections) for the M0 scaffold; `configs/debug_stationary.toml` (spec 19.2
  verbatim + `[seeds]`) validated from M0 but never executed as an actor run
  until its milestone. Alternative (single full profile executed partially)
  rejected: would pretend to run an unimplemented actor.
- **Run provenance (M0-03/M0-05).** Unique
  `runs/<profile>-root<R>-outer<O>-<unixsecs>/` dirs, git-ignored, holding
  `resolved_config.toml` (effective seeds baked in), `manifest.json` (code
  version, git hash + dirty flag, seed/RNG policy), `seed_streams.json`
  (derived hex per reserved stream). `simulate` at M0 writes only this
  provenance; full stepping/logging arrive M0-07+/M0-13.
- **Manifest seed ranges (M0-03).** `manifests/development.json`,
  `validation.json`, `final_test.json` reserve disjoint outer-seed ranges
  (1–9999 / 10001–19999 / 90001–99999). Training batches are per-generation
  manifests created by the search engine (M7); no training range is consumed
  yet.

## 2026-09-21 — Environment core (M0-06–M0-08)

- **Warmup is the first quiet (spec 5.7/10.4, M0-07).** The spec mandates a
  quiet interval per cycle and allows a fixed quiet warmup before the first
  cue without fixing their composition. Decision: the birth quiet lasts
  `warmup_ticks`, replacing the first sampled quiet; later quiets sample
  `quiet_ticks`. Zero-length warmup/quiet/gap intervals skip their phase
  entirely so every interval lasts exactly its declared tick count.
  Alternative (warmup *plus* a sampled quiet) rejected: adds undeclared
  ticks before the first cue. Consequence: `warmup_ticks = 0` starts the
  first cue presentation at tick 0.
- **Birth noise/hazard assignment (spec 5.2–5.3, M0-07).** `y[c]` sampled
  independently (p = 1/2) on `mapping_init`; membership shuffles cue ids on
  `init` with `n_stable = round(K * stable_fraction)`; volatile hazards
  cycle `volatile_hazard_values` in shuffled order; noise rates cycle
  `feedback_noise_values` in cue-index order. Limitation recorded: index
  cycling is not factorial counterbalancing — noisy-stable and
  reliable-volatile cues are not guaranteed for every (K, list) pair. Full
  stratified assignment arrives in M5-02; until then this rule is the
  documented behavior, not a hidden default.
- **Timing draw order (spec 5.7–5.8, M0-07).** Per cycle: quiet length, cue
  choice, gap length at quiet entry; delay length at commitment; noise bit
  at commitment on its own stream. Fixed order per phase entry keeps the
  timing stream reproducible. Agent streams (`actor_noise`, `tie_break`,
  `evolution`) are never drawn by the environment (tested).
- **Single-call tick driver for M0 (spec 9.3, M0-07).** `Lifetime::advance`
  delivers one tick (observation + any due feedback) and advances the clock;
  `commit` is called between ticks when `commitment_due`. The split
  `observe` / `finish_tick` API from the spec pseudocode arrives with the
  agent loop in M1-07, where acting must interleave between observing and
  clock advance. No scientific content changes: feedback still attaches at
  tick start, commitment still lands at the final response tick.
- **Extended `SimError` (spec 18.3, M0-06).** The spec sketch is explicitly
  "not the full simulator", so driver errors get explicit variants instead
  of stringly-typed failures: `CommitOutOfPhase`, `InvalidAction`,
  `MissingCommitment`, `InconsistentCounts`, `UnknownFeedback`,
  `LifetimeComplete` alongside the sketched four. Consequence: exhaustive
  matching forces new call sites to handle driver misuse.
- **`commit_with_noise` forcing path (M0-08/M0-11).** Production commits
  sample one noise bit from `reward_noise`; the forcing variant takes the
  bit as an argument for deterministic fixtures (forced reversal tests).
  Precedent: the injected-noise fixture path required for the actor in
  M1-04. The two paths share one implementation; only the bit source
  differs.
- **Reward freeze is structural (spec 5.1, M0-08).** `PendingReward` stores
  the computed reward at commitment; the single-pending protocol means no
  cue presentation (the only hazard site) can interleave before delivery,
  so a later mapping change cannot alter it. Tested behaviorally: commits
  read the current mapping each cycle while delivered rewards match the
  commit-time snapshot.
- **Consumption ledger backstop (spec 9.2, M0-08).**
  `note_feedback_consumed` accepts the first confirmation of a delivered id
  and rejects repeats (`DuplicateFeedback`) or never-delivered ids
  (`UnknownFeedback`) without state change. The future agent
  `apply_feedback` path (M3) calls it; the environment's own delivery is
  already exactly-once by construction.
- **Feature vector placeholder (M0-09 boundary).** `Observation.features`
  is allocated at the contractual `K + 6` width but zero-filled until M0-09
  populates channels. This is labeled scaffolding in code, not a result;
  no test asserts feature content yet.

## 2026-09-21 — Features, baselines, fixtures (M0-09–M0-11)

- **Latch timing falls out of call order (spec 9 step 9d, M0-09).** The
  output for tick `t` is built before any commit for `t`, so reading the
  stored `last_action` directly yields exactly the required semantics:
  empty before the first commitment, new action visible starting the next
  tick. No separate "latch update" step exists to drift out of sync.
  Channel order is cue content, cue-present, go, outcome-present,
  outcome-value, previous-action-0/1.
- **B0 draws use the `tie_break` stream (spec 5.8, M0-10).** The random
  baseline is an action-selection producer, so it owns the action-selection
  stream rather than gaining a tenth stream. The environment never draws it
  (tested), and B0/B1 conform to the ordinary `Agent` trait — the
  type-level proof they use only permitted information. The oracle
  deliberately does *not* implement `Agent`; it reads `HiddenState` through
  a separate runner, keeping privilege out of ordinary code paths.
- **Shared schedule without shared rewards (spec 5.8, M0-10).** Paired
  lifetimes on identical seed tuples draw identical cue/noise sequences
  through the production `commit` path; each reward still follows its own
  action (`R = correct XOR noise_bit` asserted per choice on both sides).
  No forced or copied rewards cross agents; `commit_with_noise` is used
  only in explicitly labeled fixture tests.
- **Golden fixture values (M0-11).** gap [5,5] + delay [3,3] on the smoke
  profile with seeds (1, development, 1, 0) gives birth mappings [1, 1],
  first cue 1, commitment at tick 20, feedback at tick 23. Values were read
  from a temporary probe test (deleted afterward), then hardcoded into
  `tests/event_order.rs` so the fixture is explicit, not self-confirming.
- **Section 17.1 coverage map (M0-11).** Correct/wrong zero-noise rewards,
  forced noise reversal, hazard 0/1, mapping-at-commit, delay-1 delivery,
  one-commitment-one-feedback, completed-lifetime counts, schedule
  independence, and no-hidden-labels live in `environment_contract.rs` /
  `leakage.rs`; the tick-20/delay-3 timeline with exact per-tick features
  lives in `event_order.rs`; outcome-present/value (including reward 0)
  and latch transitions live in the M0-09 block of
  `environment_contract.rs`. Randomized frequency checks (not fixed
  seeds cherry-picked to pass) arrive in M0-12.

## 2026-09-21 — Randomized checks, logging, audit, gate (M0-12–M0-GATE)

- **Statistical plan declared up front (M0-12).** Sample counts and
  tolerances are constants in `tests/randomized_env.rs` with sigma
  justification (3.6–4.0σ per check, ~1e-4 false-alarm rate each):
  2048 births for mapping balance, 1024 presentations for cue frequency,
  2048 choices for B0 chance correctness (latent, so noise-free by
  construction), 1024 oracle choices at eps 0.2 for the noise-conditioned
  expectation. Seeds are fixed (outer 1..=2048, lifetimes 0..32);
  failures are investigated, never reseeded. The oracle check is
  two-sided consistency, never a per-run upper bound (spec 13.3).
- **M0 event schema is minimal by design (M0-13).** `events.jsonl` carries
  `schema_version = 1` and only M0-available fields; motor margins,
  eligibility/update norms, and gate statistics arrive under new schema
  versions with their milestones. Event ids restart per lifetime in M0
  (globally unique ids arrive with M1 checkpoint/continuation work), so
  validation is per contiguous lifetime block — enforced identically by
  the Rust writer and the Python audit.
- **Run-directory collision was a real bug (M0-13/M0-15).** The retry
  fallback broke out of its loop on creation success before checking the
  manifest marker, so same-second runs overwrote each other's provenance.
  Caught by re-running all four baselines in one second during evidence
  collection. Fixed (ownership decided by marker absence) with a
  regression test creating two runs back-to-back; the overwritten runs
  were deleted and re-executed into distinct `-retryN` directories.
- **Audit duplicates the accounting in Python (M0-14).** `validate_logs.py`
  (stdlib only) re-checks everything the Rust writer guarantees plus
  cross-file identity (manifest vs resolved-config seeds, condition vs
  manifest, completion counts vs logged events). Six fixtures: one real
  oracle smoke run plus five single-mutation corrupt variants. A missing
  or non-completed `completion.json` fails the audit — interrupted runs
  are never silent zeros.

## 2026-09-21 UTC — M0 corrective review (M0-REVIEW)

- **Ordinary runner boundary/order (spec 3.4, 5.6, 9, 18.3).** The
  baseline harness never called `apply_feedback`, and `OrdinaryPolicy`
  accepted evaluator `TickOutput`. B0/B1 ignored that argument, so there
  is no evidence their recorded actions used hidden truth. Nevertheless,
  that API could not enforce the advertised boundary. Selection now reads
  only policy state; feedback is delivered/deduplicated before advancing
  ordinary features, including the last feedback transition. The existing
  M0 environment `advance` API remains; its observe/finish split is still
  M1-07 work. No neural behavior was implemented.
- **Execution versus schema validation (M0-05/14, spec 19).** The public
  runner accepted neural profiles and ignored their sections. It now
  rejects them, diagnostic reset modes, and unimplemented isolated-reversal
  and long-life modes before writing artifacts. Future profiles can still
  be parsed and validated. Environment birth also validates direct library
  callers; hidden-state construction rejects malformed membership/rates.
  Checked tick arithmetic prevents wraparound. Rejected commitments are
  checked before drawing noise, preserving paired schedules after errors.
- **Run ownership (M0-03/13).** This supersedes the prior marker-absence
  fix: checking for `manifest.json` is not an atomic claim and overwrites
  incomplete runs. Exclusive `create_dir` now owns a run; only an existing
  path triggers a suffix retry. A fixed-name regression reserves an
  incomplete run and races eight independent directory allocations.
  Simulation itself remains serial.
- **Audit acceptance (M0-13/14, spec 20.6).** Reproduced an incorrect pass
  after truncating an eight-outcome lifetime to seven and matching the
  completion count. Audits now require declared lifetime counts/lengths,
  contiguous lifetime identities, consistent run/seed/config identity,
  recomputed SHA-256 stream seeds, valid field types, configured timing,
  and hidden reward/change/exposure consistency. Rust stream validation
  also checks overlap, identity, and hidden arithmetic. Old valid fixtures
  and original raw runs remain unchanged. Logging-disabled runs receive
  explicitly limited provenance/completion checks, not event verification.
- **Reproducibility and scope.** Added the missing `.python-version` pin
  for the actually installed 3.14.7; Python remains dependency-free. Existing
  RNG and tick golden values are unchanged. The documented warmup-as-first-
  quiet choice and provisional noise assignment are preserved. The former
  differs from the illustrative additive-warmup arithmetic in spec 22.2;
  use measured ticks in M1/M5 budgets. M5-02 still owns factorial assignment.
  No change to `spec.md`, distribution algorithms, or scientific equations.

## 2026-09-21 UTC — Documentation authority and continuation (M0-DOCS)

- **Scope:** documentation maintenance from `8216c14`; no scientific or
  simulator behavior change. `spec.md`, prior dated records, and review
  evidence remain historical sources and were not rewritten.
- **Current versus durable guidance:** `to-do.md` owns live status and
  claims; `AGENTS.md` owns durable conventions; `README.md` owns command
  examples; `docs/handoff.md` maps the current implementation to the next
  task. This avoids copying the full scientific model into another guide.
  Update the handoff when entry points or integration constraints change.
- **Historical records:** bootstrap-only descriptions in earlier decisions
  are retained as history. M0-REVIEW supersedes their provenance-only runner,
  placeholder features, marker-based directory ownership, and initial audit
  descriptions. The current runner delivers real observations/feedback and
  reserves directories atomically. M1 continuation must define event identity
  explicitly; bare per-lifetime IDs are not global keys, and the earlier note
  predicting globally unique IDs is not an additional scientific requirement.
- **Seed manifests:** reservations document policy, while the M0 CLI resolves
  seeds from config/explicit overrides and does not load or enforce numeric
  ranges in these files. Corrected the development note saying no seeds had
  been used. No seed coordinates, ranges, or namespace algorithms changed.
- **Validation:** documentation changes receive link/anchor checks, structured
  file parsing, and assertions that source/config edits are comment-only.
  Previously recorded simulator tests remain prior evidence, not new runs.

## 2026-09-21 UTC — M1-01 inherited topology conventions (spec 3.2, 6.5, 10.1, 18.4–18.5)

- **Scope:** structural sampling only (mask, motor assignment, edge order,
  acceptance checks). No weights (`W0`/`B`), dynamics, plasticity, or search.
  Affected spec sections: 3.2 (sizes), 6.5 (observability), 10.1 (mask),
  18.4–18.5 (ordering, dense-first).
- **Motor assignment (spec 6.4 underspecifies indices).** Fixed disjoint
  pools on the last `2 * motor_neurons_per_action` actor indices:
  `M0 = [N-2m, N-m)`, `M1 = [N-m, N)`; non-motor are `[0, N-2m)`.
  Alternative (first indices as motor) rejected only to keep non-motor
  `[0, ...)` contiguous for the reachability proxy below. Consequence: motor
  identity is an engineering convention recorded here; M1-06 commitment and
  M7 pairing tests must use this assignment, and any change needs new
  evidence.
- **Draw order and self-edges (spec 10.1).** Receiver-major, sender-inner
  (`j = 0..N-1`, `i = 0..N-1`), one Bernoulli(`edge_probability`) draw per
  directed pair. When `self_edges = false` (both shipped profiles),
  diagonal pairs consume no RNG. Alternative (draw-then-zero the diagonal)
  rejected: it couples the mask to discarded draws. Orientation stays
  `W[receiver, sender]` (rows receive, columns send) per spec 4.1.
- **Initialization seed identity (spec 10.1, 20.5).** Inherited sampling
  uses stream `init` only; the outer-seed-level convention is
  `(root_seed, namespace, outer_seed, lifetime_index = 0, stream = "init")`.
  All lifetimes under one outer seed therefore share the mask, while
  independent outer seeds vary it. Gate mode is not an input to sampling,
  so fixed/global/targeted conditions pair by construction (spec 10.1,
  12.7). The implementation rejects non-`init` streams to keep agent
  (`actor_noise`, `tie_break`) and environment streams unperturbed.
  Consequence: a new lifetime is never a new inherited topology; evolution
  (M7) reuses this identity per genome/outer seed.
- **Cue-driven proxy before sensory weights exist (spec 6.5 vs 10.2).**
  `B` (M1-02) is planned dense over all actor neurons, so every non-motor
  neuron is potentially cue-driven. Acceptance requires (a) at least one
  directed path from the non-motor set to each motor pool separately, and
  (b) at least one directed recurrent cycle (length >= 2 while self-edges
  are disabled; length 1 counts only when `self_edges = true`). Requiring
  each pool (not just any motor) keeps both actions structurally drivable;
  requiring only existence (not every non-motor reaches motor) avoids
  rejecting most random reservoirs at the debug (`N = 16`, `p = 0.25`) and
  main (`N = 60`, `p = 0.15`) densities. Degenerate `N = 2m` (no
  non-motor) treats all neurons as sources so each pool is trivially
  reachable; only the cycle check then discriminates. Alternative
  (all-non-motor-must-reach-motor) rejected as over-strict for a sanity
  check. Consequence: if `B` later becomes sparse, this proxy must be
  refined to actual `B`-supported neurons with new tests.
- **Rejection logging, never performance selection (spec 10.1).** Sampling
  retries sequentially on one `init` RNG (each attempt consumes its draws
  in the fixed order above) up to an explicit `max_attempts`, recording
  per-attempt edge counts and reasons (`no_cycle`, `m0_unreachable`,
  `m1_unreachable`). The sampler takes no reward, hidden state, or fitness
  input, so topology cannot be selected by task performance. Exhaustion is
  an explicit error carrying the full attempt log, not a silent fallback.

## 2026-09-21 UTC — M1-02 inherited weight conventions (spec 6.3, 10.2)

- **Scope:** inherited `W0`, input projection `B`, zero actor biases, and
  parameter validation. No dynamics (M1-03), no plastic offsets `P` (M3);
  `InheritedParams` stores `W0` alone, and the effective weight `W0 + P`
  is constructed only once plasticity arrives. Affected spec sections: 6.3
  (starting constants), 10.2 (inherited weights), 20.5 (RNG policy).
- **Gaussian implementation without a new dependency (spec 20.5).** Standard
  normals come from a documented Box–Muller transform over the existing
  pinned `rand 0.9` uniform output (`ChaCha8Rng`, see `Cargo.lock`):
  `u1 = 1 - uniform[0,1)` so `u1` lies in `(0,1]` and `ln(u1)` is always
  finite; `r = sqrt(-2 ln u1)`, `z0 = r cos(2 pi u2)`, `z1 = r sin(2 pi u2)`.
  Alternative (adding `rand_distr`) rejected: a second RNG crate adds
  version surface for one transform this module can audit directly.
  Consequence: the transform is part of the reproducibility contract; any
  change of implementation or uniform source needs a derivation note and
  new golden evidence.
- **One `init` RNG, fixed order: mask, then `W0`, then `B` (spec 10.1,
  20.5).** Combined sampling draws the mask attempts first, then `W0` on
  existing edges in receiver-grouped edge order, then dense `B` row-major
  (`j` outer, `d` inner). A Box–Muller spare is cached only inside one
  sampling call, so the normal stream is a pure function of position in
  the draw sequence. Consequence: standalone `sample_topology` results are
  unchanged (mask draws still come first from a fresh `init` RNG), and one
  outer seed pairs mask and weights across gate conditions together.
- **Row scale uses the standard deviation, not the variance (spec 10.2).**
  `std = recurrent_gain / sqrt(in_degree)`; variance is its square.
  Zero-in-degree rows stay exactly zero and consume no normal draws
  (division by zero would otherwise be a silent `NaN` factory). Missing
  edges stay exactly `0.0`. Biases are exactly `0.0` (spec 10.2).
- **`B` width is an explicit caller argument.** The sampler takes
  `input_dim` (the runner passes `K + 6` observable features) rather than
  reading an environment config, keeping the agent crate boundary intact.
  No dimension normalization is applied to `B` (spec 10.2: inspect the
  actual input-current distribution instead).

## 2026-09-21 UTC — M1-03 actor transition conventions (spec 4.1, 6.1–6.2, 18.4–18.5)

- **Scope:** the double-buffered `f64` transition only (`h`, `a`, `r`).
  Motor filters/commitment (M1-06), the noise schedule audit (M1-04), and
  the watchdog (M1-08) are separate tasks. Affected spec sections: 4.1
  (orientation), 6.1–6.2 (update, adaptation), 18.4–18.5 (buffers, dense
  reference).
- **Scalar broadcast of time constants and noise.** The `[actor]` schema
  carries scalar `tau_h`/`tau_a`/`adaptation_strength`/`noise_sigma`, so
  the transition broadcasts each scalar to all neurons; the per-neuron
  `tau_h[j]`/`sigma[j]` indexing in spec 6.1 becomes a schema extension
  only if a later milestone needs heterogeneous neurons. Consequence:
  `leak_alpha` is computed once per step, not per neuron.
- **Dense `N x N` recurrent accumulation as the reference.** The drive sums
  every `(j, i)` pair over the dense `W0` storage (missing edges contribute
  their exact `0.0`), rather than iterating the edge list. Mathematically
  identical to edge-ordered accumulation, but unambiguously the dense
  oracle that a later sparse kernel (M7-13) must match with parity tests.
- **Noise after leaky integration, never clipped.** `h_new = mu + sigma *
  xi` with `mu` the leaked mixture; no `tanh`/clamp touches `h` (that
  would change the transition distribution behind the M2 score). A
  nonfinite `h_new`/`a_new` is an explicit `NonFiniteState` error; the
  M1-08 watchdog builds its health summary on this failure path rather
  than replacing it.
- **Two entry points, split verification.** `step_with_perturbations`
  takes an explicit `xi` slice (the injected-noise fixture path the M1-04
  schedule audit needs); `step` draws into a preallocated buffer through
  the M1-02 `NormalStream` and shares one private core, so both paths
  compute identically. `sigma = 0` is permitted at the transition level
  for deterministic diagnostics; configured learning profiles still
  require `sigma > 0` (M1-02 validation), and the M2 score rejects zero
  noise where it is active.
- **`from_state` doubles as test injection and restore path.** Besides the
  zero `new` birth state (spec 10.4), an explicit `(h, a)` constructor
  (with `r = tanh(h)`) serves exact fixtures now and checkpoint restore
  in M1-09; both validate shapes and finiteness instead of trusting
  callers.

## 2026-09-21 UTC — M1-04 perturbation schedule conventions (spec 6.1, 6.3, 18.4, 20.5)

- **Scope:** verification of the generator and draw schedule, not a new
  sampler. `NormalStream` (M1-02) and both step entry points (M1-03) are
  reused unchanged except that the fixture path now also records its
  injected vector. Affected spec sections: 6.1/6.3 (noise), 18.4 (draw
  every tick regardless of gates), 20.5 (RNG state).
- **One draw per neuron per tick, unconditional (spec 18.4).** `step`
  draws exactly `N` normals from the dedicated `actor_noise` stream on
  every call; the count and order never depend on input values, membrane
  saturation, logging reads, or future gate settings. The runner (M1-07)
  will call `step` on every tick including quiet periods, so the schedule
  holds across phases by construction — the transition takes no phase
  input at all. Consequence: changing a gate must not shift the noise
  stream; the M6-05 modulation-only isolation test re-proves this with
  paired schedules once gates exist.
- **Fresh Box–Muller pairing per tick.** `step` builds one `NormalStream`
  per call and drops it afterwards, so the spare deviate never crosses a
  tick boundary. For even `N` the applied vector is a contiguous slice of
  one reference stream; for odd `N` the unpaired deviate is discarded and
  the next tick re-pairs deterministically. Either way the schedule is a
  pure function of `(seed, tick, N)`. Alternative (a persistent stream
  cached in `ActorState`) rejected: it would entangle the RNG borrow with
  state lifetime for no schedule benefit.
- **Observation draws nothing.** Getters (`h`, `a`, `r`,
  `last_perturbations`) borrow state immutably; the applied perturbation
  vector is observable for diagnostics and the M2 eligibility trace
  without perturbing the stream.
- **Resume state for M1-09 checkpoints (spec 10.7, 20.5).** Because pairing
  is per-tick-local, exact continuation needs only two pieces at a tick
  boundary: the 32-byte `init`-derived `actor_noise` seed bytes and the
  `ChaCha8Rng` word position (`get_word_pos`/`set_word_pos`, verified to
  round-trip on the pinned `rand_chacha 0.9.0`; a restored stream
  reproduces the next tick's perturbations exactly). No spare deviate
  crosses ticks, so none is stored. Alternative (re-seed plus skip-N
  redraws) rejected as needlessly linear; the word position seeks in O(1).
  Consequence: the M1-09 schema must carry seed bytes plus word position
  for the noise stream, and any change of normal implementation re-opens
  this entry.

## 2026-09-21 UTC — M1-06 motor readout conventions (spec 6.3–6.4, 10.4)

- **Scope:** fixed pool means, the leaky motor filter, and commitment with
  a dedicated tie RNG. No trained decoder, softmax, or epsilon-greedy
  exploration (spec 6.4 forbids them initially). Affected spec sections:
  6.3 (filter constant), 6.4 (readout/commitment), 10.4 (birth `q = 0`).
- **One assignment source.** Pools come from `topology::motor_pools`
  (last indices, M1-01); `motor.rs` takes pool slices as arguments and
  revalidates non-emptiness, index range, and disjointness rather than
  trusting callers or duplicating the assignment rule. Consequence: M1-06
  cannot drift from the M1-01 assignment; a future assignment change
  touches one function.
- **Filter alpha reused.** `alpha_q = leak_alpha(motor_filter_tau)` shares
  the M1-03 helper (same `-expm1` contract); `motor_filter_tau = 3` is the
  configured starting value, validated finite and positive per call.
- **Ties draw, decisions otherwise don't.** `decide_action` reads the new
  `q` values: strict inequality returns the winner with zero RNG
  consumption, exact equality draws one `random_bool(0.5)` from the
  dedicated `tie_break` stream. Consequence: the word position of the tie
  RNG is unchanged by any non-tie decision, which doubles as the
  no-epsilon-greedy proof (an exploratory policy would draw every call).
  M1-07 pairs the tie stream per lifetime like every other agent stream.
- **`MotorOutput` reused for the readout.** The filter returns the
  boundary `MotorOutput` type directly, so commitment callers cannot
  mistake stale `q` for new: `update` returns the new values and
  `decide_action` takes them explicitly. Birth state is `q = [0, 0]`
  (spec 10.4); `from_q` serves fixtures and M1-09 restore with a
  finiteness check.

## 2026-09-21 UTC — M1-07 nonplastic actor integration (spec 3.4, 5.5–5.8, 6.1/6.4, 9–10, 13.1)

- **Scope:** the continuously running no-learning actor (B3) through the
  common ordinary runner. No plastic offsets, eligibility, gates, or
  search; `W0` is immutable and `apply_feedback` performs no learning.
  Affected spec sections: 3.4 (agent boundary), 5.5–5.8 (public
  features, forbidden inputs, timing, commitment), 6.1/6.4 (transition,
  readout), 9 (feedback before transition, no double apply), 10.4
  (birth zeros), 13.1 (B3 control).
- **One runner, two guards.** `run_actor_ordinary` shares the exact
  tick/feedback/commit/record loop with `run_ordinary` through
  `run_ordinary_inner`; only the execution guard differs
  (`validate_actor_no_learning_execution` vs
  `validate_baseline_execution`). Alternative (one guard accepting both)
  rejected: M0 baselines must still refuse neural sections, and actor
  profiles must refuse env-only configs. Consequence: scheduling
  fairness is structural — same `Lifetime` driver, same production
  commit, same exogenous streams.
- **Seed pairing by construction.** Inherited sampling uses
  `(root, namespace, outer, lifetime 0, "init")` so lifetimes under one
  outer seed share mask/`W0`/`B` (future B4/B5/B6 pair on this);
  per-lifetime `actor_noise`/`tie_break` streams use the lifetime index.
  The environment never draws agent streams, so actor stepping cannot
  shift cue/change/noise/timing schedules (tested against B1).
- **Reward as input, not as update.** `apply_feedback` validates
  `event_id` ordering and `reward in {0, 1}` then records the id only;
  `advance` carries the public outcome channels through `B` on the next
  transition. Consequence: different outcome values diverge trajectories
  while `W0` stays bit-identical; duplicate/invalid feedback changes
  nothing (tested with snapshots).
- **No new CLI yet.** `simulate --baseline` stays env-only/oracle; the
  `configs/actor_no_learning.toml` file and actor simulate wiring are
  M1-12 work. M1-07 tests use an explicit in-code no-learning profile
  (env-smoke timing, `N = 16`, six outcomes, no learning/modulator/
  evolution sections). `OrdinaryPolicy` for the actor is implemented in
  `experiments::baseline` so `agent` never depends on the harness.

## 2026-09-21 UTC — M1-08 numerical health conventions (spec 6.5, 10.6)

- **Scope:** read-only watchdog, running summary, and sampled traces over
  live `h`/`a`/`r`/`q`. No dynamics change (`advance` untouched), no
  clipping, no plasticity. Affected spec sections: 10.6 (f64, explicit
  failures, conservative watchdog) and 6.5 (saturation/margins for the
  M1-11 usability judgment).
- **Conservative bounds, named in errors.** `|h|`, `|a|`, `|q| <= 1e4`
  (module docs: healthy operation is `O(1)`, so the bound sits ~1000x
  above normal for both `N = 16` and `N = 60`). Alternative (tight
  threshold near observed maxima) rejected: it would false-alarm on new
  seeds. Finiteness is checked first; a finite breach names value and
  bound (`WatchdogTripped`), a nonfinite names the component
  (`NonFinite`). Failed ticks leave counters/samples untouched.
- **Observer never perturbs.** `check_state`, `HealthSummary::observe`,
  `TraceRecorder::maybe_record`, and `NoLearningActor::health_check`
  borrow immutably and import no RNG; selection is a pure function of
  topology (`[0, 1, motor0[0], motor1[0]]`, sorted/deduped, budget 4).
  Alternative (recorder owning RNG or mutating actor) rejected: logging
  must not draw simulation randomness (AGENTS.md, spec 18.4). Paired
  runs with/without observation are asserted bitwise identical.
- **File-ready, versioned, notebook-free.** Summaries, selections, and
  samples derive `Serialize`/`Deserialize` under `HEALTH_SCHEMA_VERSION
  = 1`; tests round-trip them through a temp JSON file. No Python,
  notebook, or dataframe dependency is introduced (analysis stays
  stdlib-only). Runner wiring of watchdog enforcement stays future
  work; M1-08 defines and proves the instrument, M1-11 uses it.

## 2026-09-21 UTC — M1-09 lifetime checkpoint conventions (spec 9, 10.6–10.7, 20)

- **Scope:** exact pause/resume for the nonplastic M1 actor plus the
  continuous environment. No plastic offsets, eligibility, modulator
  state, or gates exist yet; files claiming them fail as unknown
  fields. Affected spec sections: 10.7 (checkpoint contents), 10.6
  (explicit failures, no clipping), 20 (deterministic streams).
- **Module placement preserves the information boundary (spec 3.4,
  18.2).** Snapshots live in their home modules (`LifetimeSnapshot` in
  `environment`, `AgentSnapshot` in `agent::no_learning`,
  `HiddenSnapshot` beside `HiddenState`); only the top-level
  `checkpoint` module holds both halves at once, like
  `experiments::baseline`. Agent code never reads hidden state: hidden
  vectors enter the file through `HiddenState::snapshot`, and
  `NoLearningActor::restore` takes no hidden input. Alternative (one
  shared struct with the mapping visible to agent code) rejected.
- **RNG resume is seed bytes plus word position (spec 20.5).** New
  `RngState` in `rng.rs` captures all six live streams (`cue_order`,
  `mapping_change`, `timing`, `reward_noise`, `actor_noise`,
  `tie_break`); `init`/`mapping_init` are birth-consumed and survive as
  stored weights/mappings. Per-tick-local perturbation pairing (M1-04)
  means no Box–Muller spare crosses a tick boundary, so nothing more is
  stored. Restore re-derives each seed from the recorded identity and
  rejects mismatch instead of reseeding.
- **Integrity before trust.** File = versioned payload (`schema 1`,
  code version, full resolved `Config` plus its SHA-256, seed identity,
  both snapshots, inherited parameters) plus SHA-256 over the canonical
  payload bytes; writes use temp-plus-atomic-rename. Restore checks
  schema, checksum, config executability and hash, seed identity,
  agent/environment tick agreement, countdown/index ranges,
  pending-phase consistency, count identities
  (`next_event_id == commitments`, `outcomes == consumed.len`),
  confirmed-subset bookkeeping, and RNG seeds. Missing fields fail via
  `deny_unknown_fields`/required-field parsing — never silent defaults.
- **Scientific-contract bug found and fixed: JSON float parsing silently
  corrupted weights.** `serde_json` 1.0.151 default parsing mis-rounds
  rare decimals by 1 ulp (e.g. `0.20856943026379962`), so save/load
  drifted `W0` and broke bitwise replay with only a checksum mismatch
  as evidence. Fixed with the `float_roundtrip` feature
  (correctly-rounded parsing, verified on the pinned version with no
  `Cargo.lock` change); the save/load `assert_eq` is the regression
  test. Alternative (binary bincode checkpoint) rejected to keep the
  repo's JSON inspectability; alternative (hex-bit float encoding)
  rejected as invasive. Consequence: any future float-bearing JSON
  output inherits exact round-trips, and any serializer change re-opens
  this entry.
- **Resume takes an expected seed identity.** `restore_env`/`restore_actor`
  require the caller-declared lifetime identity and reject foreign files
  before trusting state — a runner cannot silently continue lifetime 3
  as lifetime 5. Health summaries stay out of the file (diagnostics are
  re-derived identically after resume, proven by continued-summary
  equality across splits).

## 2026-09-21 UTC — M1-10 replay consolidation (spec 9, 17.7)

- **Scope:** consolidation only — `tests/replay.rs` pins the home-suite
  contracts together (simultaneity, new-q commitment, continuity,
  logging invariance, checkpoint splits). No production code changed.
  Affected spec sections: 9 (ordering across replay), 17.7 (replay and
  parallelism tests).
- **Reference platform asserted, not printed.** `REFERENCE_OS/ARCH`
  (`linux`/`x86_64`) plus checkpoint/health/event schema versions are
  `assert_eq` constants, so a platform move or schema bump fails loudly
  instead of silently redefining "bitwise". Toolchain pin stays in
  `rust-toolchain.toml`/`Cargo.lock` (Rust 1.98.0).
- **Tolerance policy documented, not executed.** Bitwise `==` on the
  reference platform; cross-platform work must keep integer bookkeeping
  exact and compare float trajectories within a declared tolerance after
  per-platform serial parity. No tolerance was weakened to pass: the
  suite is exact-equality throughout.

## 2026-09-21 UTC — M1-11 observability smoke (spec 6.5, 10.6)

- **Scope:** usable-dynamics evidence only (`tests/observability.rs`, no
  production change). Fixed zeros, alternating cue blocks, 8 outer-seed
  initializations, and 64-tick quiets — all bounded, deterministic, and
  health-observed.
- **Distinguishability as across-vs-within.** Alternating 50-tick cue
  blocks must satisfy mean across-cue block distance > mean within-cue
  distance (not an absolute threshold that a gain change could strand).
- **One threshold corrected by arithmetic, not by rerunning.** The
  long-quiet test first asserted >300 ticks but the exact cycle count is
  248 (warmup 4 + cue 8 + response 4 + feedback 1, then three 77-tick
  cycles); the bound moved to >200 with the arithmetic recorded. No
  seed was changed to pass.

## 2026-09-21 UTC — M1-12 actor command conventions (spec 13.1, 18.6, 19)

- **Scope:** one new rung on the existing command, not a new runner.
  `BaselineSel::Actor` (`--baseline actor`, condition B3,
  policy `actor-no-learning`) reuses the tick/commit/record loop via
  `run_actor_ordinary`; only the execution guard and manifest note
  differ. `configs/actor_no_learning.toml` pairs env-smoke timing with
  the debug actor section and omits learning/modulator/evolution —
  absence is the explicit statement, matching the M1-07 guard.
- **Guards separate both directions.** Env-only configs reject `actor`
  (needs `[actor]`); actor configs reject B0/B1/O1 (env-only
  required). Proven through the CLI, not just the library.
- **One audit covers all rungs.** B3 event/hidden streams share the M0
  schema, so `validate_logs.py` needed only the policy map entry
  (`actor-no-learning` → B3); the pre-fix mismatch failure is the
  regression demonstration, plus a relabeled-fixture Python test.
  Checkpoint continuation stays a documented test command
  (`cargo test --locked --test checkpoint`); a dedicated checkpoint
  CLI arrives with later milestones.

## 2026-09-21 UTC — M1-GATE milestone exit (spec 16/M1)

- **Exit claim:** verified dynamical-system foundation, not learning.
  All five exit conditions hold with fresh evidence (173 Rust passes +
  1 ignored probe, 15 Python passes, clean fmt/clippy, three validated
  profiles, O1 + B3 smoke runs audited OK): simultaneous-update/noise
  contracts, continuous state preservation, usable cue/motor responses
  across seeds, finiteness in the smoke run, bitwise resume on the
  reference platform (linux/x86_64).
- **Nothing was weakened to pass.** Two thresholds were corrected by
  hand arithmetic with the reasoning recorded (M1-11 quiet-tick count
  300 → 200 for the exact 248-tick total); one real
  scientific-contract bug was fixed instead of hidden (serde_json
  1-ulp float mis-rounding → `float_roundtrip`, M1-09). Failing seeds
  were never rerun for luck: every suite is deterministic on fixed
  development seeds.
- **Scope guard for M2.** Plasticity, eligibility, gates, and search
  do not exist in the tree; B3 stays labeled a same-actor no-update
  control, never a B7 activity-only optimum. M2-01 is next.

## 2026-09-21 UTC — M1 corrective review (M1-REVIEW)

- **Checkpoint trust and complete ticks (spec 9, 10.7, 17.7, 20).** The
  M1-09 checks were insufficient: deriving an alleged seed from caller metadata
  did not verify the live generator, and nested state was not strict. Capture
  now checks the generator's actual seed and stream; schema 2 requires the
  last perturbations, initialization record, warmup duration and reference
  platform. Nullable state must be explicitly present. Arrays, topology,
  configuration, pending feedback and cross-half ledgers are validated on
  capture/load/restore. Captures occur after commitment; a mid-tick capture
  is rejected rather than inventing a continuation protocol. Old schema-1
  checkpoints are rejected, not migrated with guessed state. RNG distribution,
  seed derivation and valid-run draws are unchanged. Context7 supplied current
  Rand ChaCha documentation; getter/position semantics were also checked in
  the installed pinned `rand_chacha 0.9.0` source. Nonzero ChaCha substreams
  are rejected because the production schema supports stream zero only.
- **Atomic checkpoint writes (spec 10.7).** PID-only temporary filenames
  were not unique for concurrent callers. Exclusive reservation plus a local
  nonce, complete write/sync and atomic rename now ensure concurrent saves
  leave a whole checkpoint. No parallel simulation was introduced.
- **Runtime diagnostics (spec 6.5, 10.1, 10.6; M1-08/11).** This supersedes
  the M1-08 note deferring runner watchdog enforcement and the M1-GATE claim
  that the original CLI demo was watchdog-held. Production transitions now
  enforce the previously declared 1e4 bounds; no membrane clipping was added.
  A wrapper around the same ordinary runner saves health, selected h/a/r/q
  traces, sampling rejection history and failures. Health schema 2 adds
  explicit sampled activity and finite empty summaries. Event schema remains 1.
  Log settings affect observation only; exact healthy-event parity with the
  original commit and logged/unlogged health parity were verified.
- **Evidence and claim boundary.** Original M1 smoke files were removed by
  the earlier sessions; those historical claims remain recorded but are not
  newly audited originals. Fresh preserved evidence supports the gate after
  corrections. Some initializations remain strongly action-biased; both-action
  reachability is across seeds, not a within-lifetime balance claim. M2-01
  remains next. See [the review](m1-review.md) and its evidence bundle.

## 2026-09-21 UTC — M2-04 restricted finite-rollout interface

- **Spec 7.6, 10.5, 16/M2 and 17.6.** Implement the diagnostic under
  `experiments::finite_rollout`, with explicit mode name
  `fixed_weight_no_decay_rollout`. It reuses the unchanged actor and score
  implementations, starting at zero state independent of weights. Do not
  reinterpret a main reset policy or approximate no decay with a large tau.
- **Frozen assumptions.** Owned parameters, sigma and baseline have no live
  mutators. The caller chooses a fixed baseline before drawing this rollout's
  noise. Input/reward rules must not depend directly on the differentiated
  weight. Scores sum over existing edges; there are no gates or main learning
  bounds. Terminal reward is accepted only after the fixed horizon.
- **Terminal update and reset.** `finish(reward, None)` returns the estimator
  without an update; `Some(eta)` produces one updated copy of W0. The original
  parameters never change. Duplicate finalization and extra transitions fail.
  Explicit reset is allowed only after a successful finish and restores zero
  activity/adaptation/scores with the same frozen weights and baseline; RNG
  ownership remains with the caller. Numerical failure is terminal, cannot be
  reset/retried in-place, and must be reported as a failed diagnostic sample.
- **Scope.** No production runner/config/checkpoint change, no change to
  continuous tick ordering, and no empirical recurrent-gradient claim yet.
  These are engineering choices implementing the restricted diagnostic, not
  revisions to the main learning rule. M2-05 supplies the recurrent empirical
  check. Verification and artifact hashes: [M2-04 evidence](evidence/m2-04/summary.md).

## 2026-09-21 UTC — M3-01 plastic state, eligibility, masks (spec 7.3, 7.5, 10.3–10.4, 17.2)

- **Scope.** Lifetime plastic storage first: `P`/`E` separate from immutable
  `W0`, two plastic masks, two trace policies, one effective-weight refresh,
  and a versioned snapshot. Feedback-gated updates, the reward baseline, and
  the episodic runner are M3-02/M3-04; gating is M6. Affected spec sections:
  7.3 (persistent eligibility), 7.5 (offsets/effective weights), 7.7
  (event-reset diagnostic contrast), 10.3 (plastic mask), 10.4 (birth
  `P = E = 0`), 17.2 (missing/nonplastic never accrue; one cache location).
- **Trace policies are named configurations, not a large `tau_e`.** The
  module exposes `persistent` (`lambda_e = exp(-1 / tau_e)`, applied on
  every transition) and `no_decay_diagnostic` (`lambda = 1.0`, exact
  summation). Config validation (M0) already rejects `no_decay_diagnostic`
  under `birth_only`, so the diagnostic can never masquerade as the main
  continuous rule. `tau_e` is validated finite and `> 0` for both policies
  to keep one config-checking path; the diagnostic ignores its value.
- **Effective weights have one writer.** `PlasticState::refresh_effective`
  is the only function that writes the `W0 + P` cache, and it re-validates
  `w0` finiteness, zero-on-missing, and the `P`-zero-on-nonplastic
  invariant first. The cache is `N x N` dense with exact `0.0` on missing
  edges. The new actor entry points
  (`step_with_effective_weights`, `step_with_effective_and_perturbations`)
  read that cache; sensory `B` and bias still come from inherited
  parameters, preserving the 7.5 rule that they are not lifetime-plastic.
  The no-learning `step` path delegates to the same core through
  `&weights.w0`, so B3 behavior is bitwise unchanged.
- **`PlasticSnapshot` now, checkpoint bump later.** The snapshot is versioned
  (`PLASTIC_SNAPSHOT_SCHEMA_VERSION = 1`, `deny_unknown_fields`) and its
  restore validates schema, dimensions, finiteness, mask agreement, and
  `tau_e`, recomputing the cache rather than trusting stored bytes. The
  top-level M1 `Checkpoint` schema stays 2 and still rejects files claiming
  plasticity; embedding this snapshot and proving split replay with nonzero
  `P`/`E` is M3-10/M4-06 work. This avoids claiming exact continuation
  before the feedback-update and runner paths exist.
- **Eligibility contract.** `advance_eligibility` takes old presynaptic
  activity plus this transition's receiver perturbations, one `alpha_h`,
  and the actual positive `sigma`; it calls the verified M2-01 score and
  writes only plastic edges. Zero noise is rejected even when activity and
  perturbation are zero (7.2/6.3). Scores created on the feedback tick
  cannot explain that tick's feedback: the public entry point is
  transition-ordered and the future runner (M4-01) owns apply-before-advance
  ordering. Verification and hashes: [M3-01 evidence](evidence/m3-01/summary.md).
