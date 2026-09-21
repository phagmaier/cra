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
