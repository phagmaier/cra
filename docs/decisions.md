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

## 2026-09-21 UTC — M3-02 exactly-once feedback updates and baseline (spec 7.4–7.5, 9, 10.4, 10.6, 17.2)

- **Scope.** Unit-level gated update only: `delta` from the old baseline,
  per-edge raw/limited clamps in order, `P` bound clamp, one baseline
  update after `delta`, dedup marking, and the single cache refresh.
  Affected spec sections: 7.4 (teaching signal and running baseline), 7.5
  (update equation and mask restriction), 9 step 2 (exactly-once
  pre-transition consumption), 10.4 (birth baseline `0.5`), 10.6 (explicit
  duplicate/nonfinite errors), 17.2 (zero-change cases, clipping order,
  `W0` invariance, no trace reset). The Section 17.3 golden is M3-03; the
  episodic runner is M3-04; gate heads are M6.
- **Hyperparameters ride per call.** `apply_feedback_once` takes `eta`,
  `max_update`, `plastic_bound`, and baseline `beta` as validated arguments
  rather than storing inherited values, so fixture tests state exact
  numbers and the future runner passes its resolved configuration
  unchanged each outcome. Lifetime memory (`P`, baseline, `last_feedback`)
  is stored; `E` is read-only here. An `eta = 0`, all-zero-gate, or
  `delta = 0` call moves no `P` while still performing the one baseline
  update when `delta != 0`. No `P` decay is applied.
- **Reports stay separated for logging.** The outcome returns full `N x N`
  raw, limited, and actual matrices (zero on missing/nonplastic) so the
  `max_update` and `plastic_bound` boundaries are distinguishable even
  when both clip on the same event. This feeds the future
  `record_raw_and_applied_updates` event fields; current tests assert both
  boundaries on both signs. Duplicate and other rejected calls mutate
  nothing, preserving the exactly-once guarantee needed for split replay.
- **`PlasticSnapshot` schema 2, top-level checkpoint still 2.** The snapshot
  gains the finite baseline and the required `last_feedback` (v1 files are
  incompatible, never migrated). The M1 top-level `Checkpoint` is untouched
  and still rejects plasticity; embedding plus nonzero-`P`/`E` replay is
  M3-10/M4-06. The fixed-baseline diagnostic policy stays in
  `experiments::finite_rollout` and is not conflated with this running
  baseline. Verification and hashes: [M3-02 evidence](evidence/m3-02/summary.md).

## 2026-09-21 UTC — M3-03 golden update fixture (spec 17.3)

- **Fixture-only, no production change.** `tests/golden_updates.rs` drives
  the Section 17.3 chain (`0.4` / `0.67` / `0.4` / `0.00067` / `0.10067` /
  `0.64`) through `conditional_score`, `advance_eligibility`, and
  `apply_feedback_once` with explicit `alpha_h = 0.5` and `lambda_e = 0.9`
  (`tau_e = -1 / ln(0.9)`), independent of actor/config time constants.
  Clipped and unclipped cases are separate tests. Affected spec section:
  17.3; method under test is the M3-02 entry point, unchanged here.
- **One tolerance is 1 ulp, not bitwise.** `limited == raw` without
  per-edge clipping is an exact `clamp` identity (`assert_eq!`), but
  `actual = (P_old + limited) - P_old` adds then subtracts, so the
  unclipped `actual == limited` comparison uses `1e-15` (observed
  `3.8e-18` difference). All golden values otherwise compare at
  `1e-12`–`1e-15`. Verification and hashes:
  [M3-03 evidence](evidence/m3-03/summary.md).

## 2026-09-21 UTC — M3 pre-integration hardening (spec 5.3, 5.8, 7.5, 10.6, 17.2)

- **Cue-role assignment has its own stream.** Hidden stable/volatile
  membership now uses `cue_membership`; inherited actor topology/weights keep
  the existing `init` tuple. This supersedes the M0-era statement that both
  owners use `init`. It intentionally migrates only hidden cue-role assignment
  for mixed stable/volatile births. Historical artifacts remain tied to their
  recorded revisions; all other stream identities are unchanged.
- **Seed provenance is versioned without abandoning old runs.** New
  `seed_streams.json` files use schema 2 and include `cue_membership`. The
  Python auditor treats the historical missing-schema shape as schema 1 with
  the old nine streams, while rejecting unknown or non-integer versions. The
  current committed fixture exercises schema 2 and a separate test downgrades
  it to prove legacy acceptance.
- **The plastic bound is lifetime state, not an event argument.** Production
  construction reads it from the resolved `Learning` section. Snapshot schema
  3 records it; restore also receives the expected resolved bound, rejects a
  mismatch, and rejects any finite `P` outside the bound. This supersedes the
  M3-02 per-call-bound decision and prevents a resumed state from being
  silently clamped by its next nominally zero update.
- **Feedback parameters are named.** `FeedbackUpdateParams` carries `eta`,
  `max_update`, and baseline `beta`; `apply_feedback_once` validates it before
  mutation. This removes interchangeable positional `f64` arguments without
  changing the update arithmetic. `FeedbackUpdateParams::from_learning_config`
  is the intended M3-04 runner path.
- **Scope and compatibility.** The top-level nonplastic checkpoint remains
  schema 2 and still does not embed plastic state. Plastic schema 2 snapshots
  were never top-level runnable checkpoints and are now explicitly
  incompatible. Golden update values, actor dynamics, and environment tick
  ordering are unchanged. Verification: [M3 preflight evidence](evidence/m3-preflight/summary.md).

## 2026-09-21 UTC — M3-04 episodic clean-learning runner (spec 5, 7.6-7.7, 9-10, 16/M3, 19.2)

- **Rollout equals one choice cycle with logged resets.** Rollout 0 starts at
  tick 0; each later rollout starts the tick after the previous feedback.
  Each rollout starts with `h = a = q = E = 0` and preserves `P`, the running
  baseline, dedup, tick count, and RNG positions. Post-feedback-tick scores
  are discarded by the next reset, so each terminal update uses only its own
  rollout's `lambda = 1` accumulation. No reset follows the final feedback.
  Affected spec sections: 7.6-7.7 (diagnostic resets, no decay, terminal
  update), 9 (apply-before-advance, exactly once), 10.4 (birth state).
- **Trace reset is a diagnostic-only entry point.** `PlasticState` gains
  `reset_traces_episodic_diagnostic`, which requires the
  `no_decay_diagnostic` policy and rejects `persistent`. The episodic learner
  couples exactly one `advance_eligibility` call to each actor transition and
  consumes every raw/limited/actual report into its summary.
- **Agent-only construction.** `EpisodicLearner::from_agent_parts` takes the
  `[actor]`/`[learning]` sections, inherited params, cue count, and two
  dedicated RNGs only. The driver samples inheritance from the outer-seed
  `init` tuple (pairing masks by construction) and derives per-lifetime
  `actor_noise`/`tie_break`; it never passes hidden state, schedules, or
  master seeds into the learner. Fixed gate 1; modulator stays `fixed`.
- **Config separation is machine-checked.** `validate_environment_execution`
  allows named diagnostic resets (the env schedule is unchanged); baseline
  and B3 guards now explicitly require `birth_only`, and
  `validate_episodic_execution` enforces the clean task (2 cues, zero
  noise/hazard, no gap, delay `[1, 1]`, `episodic_diagnostic` +
  `no_decay_diagnostic`, enabled learning, fixed gates, disabled search).
  `simulate` therefore rejects `episodic_stationary` on both rungs, and the
  summary carries mode/policies/profile/resets so the diagnostic cannot be
  cited as continuous. Checkpoint schema stays 2; embedding is M3-10/M4-06.
  Verification: [M3-04 evidence](evidence/m3-04/summary.md).

## 2026-09-21 UTC — M3-05 matched no-update and shuffled-reward controls (spec 13.1, 16/M3, 17.8)

- **One shared inheritance, three mechanisms.** `sample_matched_inheritance`
  samples once from the outer-seed `init` tuple; B3, B4, and shuffled all
  build agent-only from it with the same per-lifetime `actor_noise`/
  `tie_break` draws and the same `Lifetime` schedule and rollout resets.
  Summaries prove the pairing (`W0`, init record, cue order, reset ticks
  identical) and name the only declared differences (`condition_id`,
  `learning_enabled`, `reward_protocol`). Affected spec sections: 13.1 (B3
  negative control, B4 always-on plasticity), 13.5 (matched topology,
  weights, inputs, noise, duration), 16/M3 (controls before acquisition
  claims).
- **B3 owns no plastic state.** `NoLearningActor` gains the public
  diagnostic-only `reset_state_episodic_diagnostic` (zeros `h`/`a`/`q`/
  readout; preserves `W0`, dedup, ticks, RNGs). Its summary type has no
  `P`/`E`/baseline/update fields, so the absence is structural rather than
  a dropped log. First-rollout actions match B4 exactly (`P = 0` plus paired
  draws on both sides), isolating later divergence to acquired offsets.
- **Shuffle corrupts the signal, never the record.** `IndependentFairCoin`
  draws each applied reward iid Bernoulli(0.5) from the dedicated
  `shuffle_reward` stream (public seed tuple only — no hidden mapping,
  correctness, noise, or hazard). `choice.reward` keeps the observed
  environment outcome; `choice.applied_reward` drives the `P` update and the
  running baseline, and `update.reward` equals the applied value. Tests
  re-derive the exact coin sequence without hidden state, proving both
  protocol fidelity and the information boundary. The expected finding is
  weight movement without systematic latent gain; an improvement here would
  indicate leakage, not learning.
- **Behavior and `P` together.** B4 and shuffled must show real final-`P`
  movement with consumed per-event raw/limited/actual reports and exact
  `W0 + P` effectiveness; reward alone never passes. The focused 4-outcome
  runs verify machinery, not acquisition — the grid, criterion, and
  several-seed comparison are M3-06/M3-07. Verification:
  [M3-05 evidence](evidence/m3-05/summary.md).

## 2026-09-21 UTC — M3-06 development grid and acquisition criterion (spec 16/M3)

- **The plan is frozen before results.** `manifests/m3_development_grid.json`
  (schema 1) declares all 24 combinations of `eta` {3e-4, 1e-3, 3e-3},
  `input_scale` {0.2, 0.3}, `recurrent_gain` {0.5, 0.8}, `noise_sigma`
  {0.03, 0.05} on the `episodic_stationary` base: development root 1, outer
  seeds 1-3, lifetime 0; 2,000 outcomes per lifetime with early-first-200
  and late-final-200 windows; matched B3/B4/B4-shuffled at every point.
  Affected spec section: 16/M3 (small explicit grid on development seeds;
  suggested target as debugging target, not benchmark).
- **Margins, not the 0.8 figure, are the threshold.** A grid point passes
  when B4 late-window latent accuracy beats B3 by >= 0.15 and shuffled by >=
  0.10 on >= 2/3 outer seeds, with zero failed lifetimes, < 10% clipped
  updates, and real B4 final-`P` movement. Selection among passers takes the
  largest minimum margin with a declared deterministic tiebreak; no passer
  means M3-09 reduction with the gate left open — never grid/criterion
  edits after seeing outcomes. The spec's 0.8 number is recorded verbatim in
  the manifest as a debugging target.
- **Budget and validation are mechanical.** 24 points x 3 seeds x 3
  conditions = 216 lifetimes; the 7,344,000-tick estimate is derived from
  the base profile's maximum cycle length and checked exactly, so the
  manifest cannot drift from the code. `src/experiments/grid.rs` expands
  points into candidate configs and proves each passes
  `validate_episodic_execution` without executing anything; combination
  order (`eta` major, `noise_sigma` minor) is fixed and tested. M3-07 runs
  this declaration as written. Verification:
  [M3-06 evidence](evidence/m3-06/summary.md).

## 2026-09-21 UTC — M3-07 motor-afferent acquisition (spec 16/M3)

- **The frozen grid was run as written.** All 24 points x outers 1-3 x
  B3/B4/B4-shuffled with 2,000-outcome lifetimes completed in release
  (~16 s, 7,343,136 measured ticks, zero failures); per-seed aggregates
  and the criterion verdict are archived with provenance. The ignored sweep
  test asserts execution integrity only — the verdict is measured and
  recorded, so a negative finding could never look like a harness failure.
  Affected spec section: 16/M3 (exit over matched controls, several seeds,
  valid numerics).
- **Winner index 11, selected by the declared rule.** Eta 0.001,
  input_scale 0.2, recurrent_gain 0.8, noise_sigma 0.05. Outer 2 acquires
  0.035 early to 0.860 late against 0.040 controls; outer 3 rises 0.795 to
  0.935 against 0.770/0.765; points 17/19 also clear 2/3 seeds. All three
  passers tie at minimum margin 0.0 (the stuck outer-1 seed contributes
  zero everywhere), so smaller eta selects 11 exactly as the manifest
  orders — no post-hoc judgment entered.
- **Guardrails pass sweep-wide.** Maximum clipped-update fraction 0.017,
  maximum bound occupancy 0.004, every lifetime moved real `P` offsets,
  zero nonfinite values. Clipping/bound occupancy stand in for
  activity-saturation traces, which episodic summaries do not record; that
  substitution is stated in the module docs, not hidden.
- **Birth-locked actors bound the claim.** Outer-1 actors emit action 0 on
  every rollout against all-ones mappings at all 24 points (B3 identical),
  consistent with previous-action-latch plus motor-bias lock-in from the
  first commitment — a property of the specified K+6 observation family,
  investigated and ruled not-an-environment-bug (mappings vary correctly
  across outers) and not a harness ordering bug (outer-3 learning proves
  updates act). The acquisition claim therefore covers responsive actors
  in the tested family. M3-08 carries index 11 to the full recurrent mask;
  M4 must confront the same lock-in without resets. Verification:
  [M3-07 evidence](evidence/m3-07/summary.md).

## 2026-09-21 UTC — M3-08 all-recurrent acquisition (spec 7, 16/M3)

- **Mask extension at fixed hyperparameters, not a re-tune.** The frozen
  M3-06 manifest is untouched and the M3-07 motor-only sweep is preserved
  as a diagnostic; this task runs the declared comparisons (matched
  B3/B4/B4-shuffled, development root 1, outers 1-3, 2,000 outcomes,
  first-200/final-200 windows, same margins and guardrails) at the single
  winner point (grid index 11: eta 0.001, input_scale 0.2,
  recurrent_gain 0.8, noise_sigma 0.05) with `plastic_mask` changed to
  `all_recurrent_edges`. Affected spec sections: 7 (same score/update
  machinery), 16/M3 (several-seed learning over controls). No production
  code changed, so no new learning equation entered.
- **Full mask passes 2/3 seeds with stronger responsive acquisition.**
  Outer 2 reaches 0.975 late (vs 0.860 motor-only) against 0.040/0.030
  controls; outer 3 reaches 0.955 (vs 0.935) against 0.770/0.775; outer 1
  stays 0.0 under both masks (birth-locked, documented family bound).
  Larger offset norms (4.70/3.67 vs 2.45/2.51 L1) follow from ~4x the
  plastic parameters at the same eta; clipping stays tiny (max 0.0007)
  with zero bound occupancy. The shuffled control moves real offsets
  without systematic accuracy gain, so the finding is contingency-driven.
- **Mask difference is measured, not attributed to gates.** The fast test
  proves the structural superset on shared inheritance plus behavioral
  non-motor offset movement with W0/effective/bound compliance. Later
  gate comparisons must stay mask-matched; this record must not be cited
  as a gate effect. Continuous acquisition remains M4 work; checkpoint
  schema stays 2. Verification:
  [M3-08 evidence](evidence/m3-08/summary.md).

## 2026-09-21 UTC — M3-09 failure-isolation path (spec M3 "If it fails", 21.1–21.2)

- **Instruments, not a rescue.** The learner acquired successfully, so no
  rule was changed to manufacture learning. `src/experiments/reduction.rs`
  holds two diagnostic-only runners: a synthetic single-motor closed loop
  (constant features, known preferred action — a mechanism check, never a
  task result or a comparison against environment-driven runners) and a
  receiver-permuted episodic runner that mirrors the verified driver tick
  for tick except for the perturbation assignment. Affected sections: the
  M3 failure recipe plus 21.1 (smallest-system order) and 21.2 (learning
  vs representation failure).
- **One narrow hook with a bitwise identity control.** `advance` now
  delegates to a shared private core; `advance_with_receiver_permutation`
  validates the bijection and is reachable only from `reduction.rs` and
  its tests (proven by source search). The identity permutation
  reproduces the verified runner's final `P`/`E`/baseline/action sequence
  exactly, so the refactor carries no transition-path artifact.
- **Outer-1 diagnosed independently.** Hand-set sign/order checks plus the
  closed-loop drift direction (preferred rate 0.400 → 0.700, single
  offset +0.0766, every update trace-identical) all verify, while outer-1
  moves real offsets with locked behavior — a representation failure by
  ladder step 4, consistent with the M3-07/M3-08 bound through new
  mechanics rather than repeated assertion.
- **Permutation sensitivity without a demanded magnitude.** Reversed
  perturbations complete on the identical schedule with finite values but
  different updates; no behavioral failure size is asserted (correct
  0.050 vs reversed 0.000 late accuracy at the 600-outcome horizon, both
  pre-acquisition). Fixed seeds throughout; diagnostic constants, not
  seeds, are the tuning surface. Verification:
  [M3-09 evidence](evidence/m3-09/summary.md).

## 2026-09-21 UTC — M3-10 learned-offset checkpoints (spec 7, 10.7, 16/M3)

- **A separate schema-3 envelope, not a migrated schema 2.** The M1
  nonplastic `Checkpoint` is byte-identical to before; `LearningCheckpoint`
  carries the episodic snapshot (`h`/`a`/`xi`/`q`, readout, ticks, RNG
  positions, sampling record, versioned `PlasticSnapshot` with
  `P`/`E`/baseline/dedup) with mirrored envelope rules (atomic write,
  checksum, seed identity, config hash, tick/ledger agreement,
  `Committed` rejection, hidden-assignment and init-seed binding).
  Neither loader reads the other's files — cross-schema resume is a
  parse failure, never a misread lifetime. Learner restore re-checks
  learning-section agreement (mask/trace/`tau_e`/bound), not just
  shapes. Affected sections: 10.7 (checkpoint contents), 16/M3 (exit
  evidence).
- **Pending delivery is not checkpointable state.** A
  delivered-but-unconsumed reward lives in driver-held memory (the
  environment's pending slot is already taken), so compat deliberately
  demands `confirmed == consumed` plus dedup agreement. The
  just-before-feedback split is therefore the last fully-processed
  tick, and deterministic re-delivery is an explicit assertion — a
  drafted tick-rule relaxation for pending captures was reverted once
  this gap was understood, rather than weakened to pass.
- **Replay proves continuation, archives pin the trajectory.** Three
  splits (boundary with nonzero `P`, just-before-feedback with full
  `E`, post-feedback with fresh `P` plus post-scores and no
  double-apply) resume bit-identically against uninterrupted
  references, after a faithful-driver equality proof makes the manual
  driver the same scientific object. The pinned working config,
  verbatim failure case, real boundary/final files, and first/last
  update digests cross-match the M3-08 outer-2 record exactly through
  an independent driver. Checkpoints pin the reference platform by
  design. Verification: [M3-10 evidence](evidence/m3-10/summary.md).

## 2026-09-21 UTC — M3-GATE milestone exit (spec 16/M3)

- **The gate re-ran the evidence.** Fresh release executions reproduce
  the archived M3-07 verdict (winner index 11; all 72 seed records
  identical) and the M3-08 verdict (passes 2/3) exactly — provenance
  alone differs, as it must in a dirty worktree. The gate therefore
  rests on this session's executions, not on cited files. Full battery
  alongside: 289 fast Rust tests (every M2 derivative/finite-difference/
  golden check included), 2 compile-fail doc checks, 17 Python audits,
  fixture audit OK, clean fmt/Clippy/diff. Six ignores are exactly the
  two Monte Carlo diagnostics, the weight probe, and the three bounded
  captures, each invoked separately per its record.
- **Exit conditions (a)–(d)** (ledger): several-seed learning over
  matched B3/shuffled controls at two masks; still-valid score/golden
  arithmetic; interpretable numerics (clipping/bound/`P`-movement
  guardrails plus separated update norms); no lucky trajectory (2/3-seed
  rule with per-seed matched controls; the outer-1 bound diagnosed, not
  tuned away). No evolution, search, gate, or M4 work started.
- **What M3 proves, and what it does not.** Proven: an ungated local
  learner acquires clean episodic associations from delayed terminal
  rewards on responsive actors, with exact pause/resume through
  learning events. Not proven: continuous acquisition without
  within-lifetime resets — that is M4's explicitly open gate, and no
  M3 artifact substitutes for it. Claim track stays `family_only`;
  B3 is still not B7. Next: M4-01 (authoritative main tick order).

## 2026-09-21 UTC — M4-01 authoritative main tick order (spec 9, 17.2)

- **Split the tick, kept the state machine.** `Lifetime::advance()` is now
  `observe()` (build tick + attach due feedback, no clock advance, no RNG
  draws) followed by public `finish_tick()` (phase machine + clock).
  Delivery bookkeeping lives in `observe`; all six production drivers
  (ordinary B0/B1/B3, oracle, B4/B3/shuffled episodic, permuted
  reduction) run observe → apply → agent-step → finish → commit with
  spec-9 step comments. `advance()` remains as the fused primitive for
  environment-only tests, pinned identical by a parity test.
- **Finish-before-commit equivalence (not a scientific deviation).**
  The `Committed` transient phase, `commit_tick = tick - 1`, and
  `Delay{delay - 1}` scheduling are preserved, so commit runs after
  finish rather than in the spec-9.3 pseudocode position. Observable
  behavior is identical: same features/feedback per tick, same
  commit/due ticks, same next-tick latch, and RNG streams are separate
  per-stream ChaCha instances so agent/env draw interleaving cannot
  couple schedules. The learning-sensitive causality (pre-tick
  `E`/gate/baseline in, post-feedback scores excluded) is what the
  tick-20/delay-3 fixture pins, and it holds exactly.
- **Eligibility before motor is documentary.** `advance_inner` now runs
  the trace update (step 6) before the motor filter (step 7) per spec
  order; the two touch disjoint state (`r_old`/`xi`/`E` vs new
  `r`/`q`), so values are unchanged. Gate stays fixed 1; no modulator
  until M6; exactly one `P` writer (`apply_feedback_once`) kept.
- **What M4-01 does not claim.** Traces are still
  `no_decay_diagnostic`, resets still episodic-diagnostic, no
  continuous profile exists. Those are M4-02/M4-03/M4-04. Verification:
  [M4-01 evidence](evidence/m4-01/summary.md).

## 2026-09-21 UTC — M4-02 persistent eligibility and running baseline (spec 7.3-7.5, 7.7-7.8)

- **New type, not a relaxed guard.** The persistent learner is a
  separate `experiments::continuous::ContinuousLearner`, not an
  `EpisodicLearner` with the `persistent` guard lifted: the episodic
  type stays the explicitly resetting diagnostic, and M4-04 condition
  names will rest on distinct types plus drivers/configs rather than
  one struct with two behaviors. It shares no code with the episodic
  path beyond the common `PlasticState`/`ActorState`/`MotorState` APIs,
  so M3-verified behavior is untouched.
- **No reset methods exist.** Birth construction is the only reset by
  construction; reset instrumentation/auditing is M4-03's job on top
  of this type. No runner, profiles, checkpoints, or gates yet
  (M4-03 through M4-06).
- **Anti-snapshot pin.** The tick-20/delay-3 persistent fixture proves
  the feedback update uses live `E` (decayed commit-time trace plus
  delay scores) and differs from both a frozen commit-time `E` and a
  decay-without-scores counterfactual — excluding the spec-7.8
  `commit_snapshot_credit` alternative by construction, not by label.
  Baseline closed form (0.509804 over [1,0,1]) pins once-per-feedback
  counting. Verification: [M4-02 evidence](evidence/m4-02/summary.md).

## 2026-09-21 UTC — M4-03 birth-only resets and warmup (spec 9-10)

- **Runner with an audited empty reset log.** `run_continuous_lifetime`
  contains no reset call; its `resets == [0]` audit plus the telescoping
  `P`-sum tripwire (final `P` equals summed per-choice actuals at
  1e-12) would fail on any mid-lifetime reset. Reset reasons need no
  enum yet because no reset path exists; M4-04 adds conditions without
  adding silent resets.
- **Guard pins the condition, not the task.** `birth_only` +
  `persistent` + enabled learning + fixed gates + no search; kind,
  cues, timing, and noise/hazard stay open so M4-05/M5 profiles run
  through the same guard. Inheritance/RNG tuples are shared with the
  episodic family for by-construction pairing.
- **Warmup is ordinary ticks.** Leading quiet ticks accrue live
  eligibility (nonzero entering the first cue); no post-warmup
  boundary hook exists. No library log-rotation path exists either
  (`run.rs` writes per-lifetime files) — documented, not tested.
  Verification: [M4-03 evidence](evidence/m4-03/summary.md).

## 2026-09-21 UTC — M4-04 continuity conditions and profiles (spec 16/M4, 7.7)

- **Same type, different drivers — distinction by label and audit.**
  `ContinuousLearner` gained one `E`-only reset method for the
  event-reset diagnostic; the fully persistent runner never calls it
  (M4-03 audit still proves `resets == [0]` per primary run). This
  supersedes the M4-02/03 "no reset methods" phrasing: the guarantee
  was always per-run (driver + audit), and the two runners plus
  `mode`/`reset_policy` labels plus divergent trajectories now carry
  it. Same pattern as M3-05 conditions sharing one learner type.
- **Guards are pairwise disjoint by test.** Each continuity runner
  rejects the other two reset policies, so the verify clause holds
  literally: a `persistent`-labeled trace-clearing config cannot enter
  the continuous runner, and a `birth_only` config cannot enter the
  event-reset runner.
- **Profiles: source plus executable twin.** `debug_stationary` stays
  the frozen spec-19.2 reference (values untouched); the new
  `continuous_stationary` is section-identical except `profile_name`
  and is the executable primary profile. Timing variability stays out
  (M4-05). Verification: [M4-04 evidence](evidence/m4-04/summary.md).

## 2026-09-21 UTC — M4-05 gradual timing and delay sensitivity (spec 5.7, 7.8–7.9)

- **Three explicit stages, timing only.** `continuous_stationary` remains the
  fixed short-delay stage; `continuous_variable_short` adds moderate
  quiet/gap/delay variability; `continuous_variable_delayed` reaches the
  spec-5.7 main timing endpoints. After normalizing `profile_name` and the five
  timing fields, the configs are equal. All keep clean stationary mappings,
  one pending choice, birth-only resets, persistent traces, and fixed gates.
- **Pairing is by environment streams, not equal actions.** Within each
  profile/outer, `tau_e` 16/32/64 use identical root/namespace/outer/lifetime
  tuples. Tests require identical cue/noise/commit/feedback schedules and
  inherited `W0`; actions may diverge as learning diverges.
- **Measure the live trace directly.** `ContinuousChoice` now carries
  `eligibility_l1_before_update`, sampled immediately before the feedback
  update. Its fixed-gate raw-update relation is independently pinned, avoiding
  reconstruction from `delta` when the teaching signal could be zero.
- **No monotonic trace-timescale criterion.** The predeclared 27-lifetime
  development diagnostic records trace and update norms, clipping, bounds, and
  reward for `tau_e` 16/32/64 but selects no winner. Longer traces increased
  measured trace/update scale and clipping in this sample; longest-delay mean
  reward was not monotonic. M4-07, not this sensitivity record, owns
  acquisition claims. Verification:
  [M4-05 evidence](evidence/m4-05/summary.md).

## 2026-09-21 UTC — M4-06 continuous-learning checkpoints (spec 10.7, 17.7, 20)

- **A separate schema-4 envelope preserves the earlier contracts.**
  `ContinuousCheckpoint` carries the fully persistent learner and shared
  environment snapshot without changing M1 schema 2 or episodic schema 3.
  Cross-loads reject. The continuous learner snapshot contains live
  `h`/`a`/last-`xi`, motor filters/readout, tick count, agent RNG positions,
  and the versioned `PlasticSnapshot` with nonzero `P`/`E`, baseline, and
  dedup identity. The envelope retains resolved config/hash, seed/platform
  identity, inherited parameters, environment RNGs, phase/pending reward,
  action latch, and delivery/confirmation ledgers.
- **The effective cache is asserted and re-derived.** Schema 4 records the
  live `W_effective` value, but restore never trusts it as an independent
  weight source: `PlasticState::restore` recomputes `W0 + P` through the
  single refresh path, then rejects any mismatch with the stored assertion.
  This keeps inherited `W0` and acquired `P` authoritative while detecting a
  stale or incompatible cache.
- **Checkpoint boundaries remain completed ticks.** Immediately before due
  feedback means the last completed tick with environment phase `Feedback`
  and the sampled reward still pending. After feedback means the first fully
  processed boundary after apply/confirm, neural transition, and tick finish.
  A delivered-but-unfinished tick is still not checkpointable driver state.
  This preserves M4-01 ordering and makes resume re-delivery deterministic.
- **Replay and rejection are state-complete.** A production-faithful manual
  driver matches the continuous runner, then splits during cue activity,
  immediately before feedback, and after feedback with nonzero `P/E`; all
  continuations are exact on linux/x86_64. Duplicate learner/environment
  delivery rejects without mutation. Missing cache/baseline/dedup/pending/
  latch fields, mismatched resolved learning settings, corrupt checksums, and
  cross-schema files reject rather than resetting. Verification:
  [M4-06 evidence](evidence/m4-06/summary.md).

## 2026-09-21 UTC — M4-07 declared acquisition comparison and negative-result handling (spec 14.3, 16/M4)

- **Freeze exposure-indexed acquisition before execution.** The M4-07 plan
  uses the first and final 100 exposures of each cue, then macro-averages cues
  equally. It carries forward the M3-08 full-recurrent winner settings and
  compares episodic, event-reset, and fully persistent B4 against the
  appropriate episodic or birth-only-reset B3. The criterion and finite
  15-lifetime development budget live in
  `manifests/m4_continuous_acquisition.json`; no post-result tuning is folded
  into this verdict.
- **Health is measured in the production runners.** Episodic, continuous,
  event-reset, receiver-permutation, and ordinary B3 runners now accumulate
  the existing read-only `HealthSummary` on every neural transition. This
  adds no RNG draws or state mutation and lets the comparison report actor
  saturation, motor-filter maxima, and motor margins from the same lifetime
  that produced behavior. Non-neural B0/B1/O1 summaries retain no health
  payload.
- **A completed empirical instrument is not a passed scientific task.** All
  15 declared lifetimes completed with exact pairing and healthy numerics,
  but fully persistent learning passed 0/3 seeds. M4-07 remains unchecked and
  M4-GATE remains blocked. The next eligible task is M4-08's declared failure
  audit; hidden resets, altered seeds/windows/criteria, and retrospective
  threshold weakening are prohibited. Verification and result:
  [M4-07 evidence](evidence/m4-07/summary.md).

## 2026-09-22 UTC — M4-08 continuity-failure audit (spec 16/M4, 21)

- **Audit before scope expansion, with the negative record frozen.** The
  M4-08 plan (`manifests/m4_continuity_audit.json`) references the M4-07
  manifest by SHA-256, reuses its family/seeds/windows descriptively, and
  declares 8 paired legs plus probes before execution. The only new
  development range is tau_e 16/64 with a recorded spec-7.9 reason; eta is
  held fixed. No production file changed; all diagnostics drive the public
  runners or the public learner API.
- **The failure is a continuity-induced behavioral lock, not a rule,
  ordering, baseline, timescale, or saturation defect.** On outer 2 every
  persistent-activity leg answers action 0 on all 2,000 outcomes (zero
  reward), the baseline decays to 0, and late |delta| is 0 despite large
  live traces — updates starve. The identical-schedule episodic leg explores
  from reset states and reaches late 0.97. Event-reset (E cleared, activity
  persistent) locks identically with independent consecutive updates, so
  persistent activity — not trace contamination — is the binding constraint,
  even though the audit proves stale-trace leakage exactly and measures
  alignment growing with tau_e (0.32/0.53/0.69). Timescale does not rescue
  (0.00 at 16/32/64); baseline tracks mean reward correctly; saturation is
  zero; raw identity holds on every persistent outcome.
- **The lock reproduces at minimal scale.** A constant-input synthetic-
  reward loop with no resets holds the wrong action on 600/600 outcomes
  while P still moves; a synthetic two-cue probe shows fresh cue separation
  with prior-cue carryover at ~80% of that separation on the focus seeds.
  Outer 1 (locked under every condition) and outer 2 (locked only without
  resets) are different failures; outer 3 starts near-correct and holds.
- **M4-08 is verified as an audit; M4-GATE stays blocked.** No rescue
  mechanism was introduced and none follows from this evidence. Any future
  escape mechanism needs a new declared task with fresh controls; the M4-07
  criterion stays frozen. Verification:
  [M4-08 evidence](evidence/m4-08/summary.md).

## 2026-09-22 UTC — M4-08b lock localization (spec 16/M4, 21.1–21.2)

- **Paired synthetic-drive probes on the frozen family, no new range.**
  `manifests/m4_lock_localization.json` declares lock-in (M4-08 protocol,
  60 outcomes), dose/flip/converge matrices on outers 2–3, and a 76,768-
  transition budget before execution. Flip clones share history and noise;
  converge pairs share perturbation draws. No production change.
- **The lock sits in the input-projection alignment.** Outer-2 1× cue drive
  moves the fresh readout toward action 0 while both mappings reward 1;
  zero drive escapes locked clones 8/8 (~50 ticks) but any cue drive pins
  0/8 with margins growing in scale. Outer 3 is the aligned contrast
  (cue-1 drive flips 8/8 ever faster; cue-0 drive correctly holds).
  Converge distances wash out 4–5× by 64 ticks but plateau above zero, so
  history persistence is secondary. Motor inertia alone is escapable.
- **Consequence:** scaling `input_scale` cannot fix a wrong-way projection
  (deeper pin with scale). Headroom, if any, is in a bounded persistent
  `eta` × `tau_e` (+`motor_filter_tau`) sweep with pre-registered rules, or
  in larger scope changes (family selected on persistence, exploration
  support) needing their own tasks. Verification:
  [M4-08b evidence](evidence/m4-08b/summary.md).

## 2026-09-22 UTC — M4-08c null and persistent-exploration design escalation (spec 6.4–6.5, 7.9, 10.2, 16/M4)

- **Owner instruction authorizes the bounded sweep.** The request to choose
  and proceed superseded the earlier approval hold. The plan was frozen in
  `manifests/m4_escape_sweep.json`, then executed once for 39 lifetimes.
  No production changes; M4-07 manifest/criterion/archive preserved.
- **Correct the interpretation before running.** Absolute thresholds and
  deterministic replay do not remove development selection bias. Outer-3
  B3 accuracy 0.875 means the 0.15 margin is unattainable there, requiring
  rescue of both locked seeds. Prior drive probes do not rule out weaker
  inputs in closed-loop lifetimes; excluding input/motor axes isolates this
  sweep rather than proving those axes irrelevant. Omitting tau_e 128 is a
  finite-budget choice, not a monotonic failure theorem. These corrections
  are in the [preflight](evidence/m4-08c/preflight.md), written before runs.
- **Measured null, no adoption.** All 12 settings pass 0/3 seeds. All 24
  B4 lifetimes on outers 1/2 choose action 0 for every outcome, with tiny
  late teaching signals despite live traces. All 39 lifetimes completed;
  pairing, six exact archived aggregate records, and numerical checks pass.
  The [negative M4-09 bundle](evidence/m4-09/summary.md) is an evidence
  package, not a milestone pass or an impossibility result.
- **Choose persistent exploration as the next design question.** Stop
  tuning eta/tau on this frozen episodic-selected actor for now. Prefer a
  separately preregistered input-drive/noise balance investigation using
  the existing ordinary neural perturbations and unchanged learning rule.
  It should first measure B3 exploration in actual persistent lifetimes,
  then acquisition against paired B3 at declared settings and budgets;
  retain every initialization rather than screening away unfavorable
  mappings. Any selected family needs fresh development confirmation and
  replay coverage before the M4-GATE question. The old threshold stays
  frozen; any better future acquisition criterion must be a new declaration
  with its own rationale, not a replacement verdict for M4-07.
- **Alternatives and consequences.** Motor-filter changes remain a possible
  separate diagnostic, but cue-driven pinning and passively escapable motor
  inertia make them less directly motivated than input/noise balance. Adding
  another exploration rule or changing the baseline would alter the model
  and is not authorized implicitly by this null. Weakening input may lose
  cue information; stronger neural noise changes both exploration and score
  variance, so neither is an assumed rescue. No design sweep, new family,
  or scientific contract change was executed after the null.

Verification: [M4-08c evidence](evidence/m4-08c/summary.md). M4-07,
successful M4-09 handoff, and M4-GATE remain open; no M5/gates/search unlock.
