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
