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
