# Learning When to Learn

Implementation of [`spec.md`](spec.md) (v0.1, Sept 20 2026): does internally
generated gating improve a continuously running recurrent agent's adaptation
to real changes without damaging stable associations under misleading
feedback?

Status: **M0 in progress (tasks M0-01–M0-05)**. No neural dynamics,
plasticity, evolution, or full environment stepping exists yet. Anything
listed below under "Planned" is a target from spec Section 18 / `to-do.md`,
not working code.

## Toolchain (pinned)

- Rust `1.98.0` (see `rust-toolchain.toml`), Cargo `1.98.0`, resolution in
  `Cargo.lock` (committed).
- Python `3.14.7` for the offline analysis layer; no third-party Python deps
  yet (see `analysis/requirements.txt`). NumPy/Matplotlib arrive when log
  analysis needs them (M0-14).
- Key Rust deps (pinned in `Cargo.lock`): `clap 4.6.7`, `serde 1.0.229`,
  `serde_json`, `toml 0.8.23`, `thiserror 2.0`, `rand 0.9.5`,
  `rand_chacha 0.9.0`, `sha2 0.10.9`. No deep-learning framework.

## Quality gates

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

## Implemented commands (M0)

```bash
cargo run --locked -- validate-config configs/env_smoke.toml
cargo run --locked -- validate-config configs/debug_stationary.toml
cargo run --locked -- simulate --config configs/env_smoke.toml --seed 1 --outer-seed 1
```

Notes:

- `validate-config <file>` parses and validates the TOML against the
  `schema_version = 1` schema (spec 19). Unknown fields and unsupported
  modes are rejected, never silently ignored.
- `simulate` at M0 is scaffolding only: it validates, resolves effective
  seeds (`--seed`/`--root-seed` and `--outer-seed` overrides recorded
  per-field as `config`- vs `cli`-sourced),
  and writes a unique run directory with `resolved_config.toml`,
  `manifest.json`, and `seed_streams.json`. It does **not** step the
  environment or any actor; claiming otherwise would violate M0-05. Full
  phase scheduling arrives in M0-07+.
- Seed namespaces (`development`, `training`, `validation`, `final_test`)
  are disjoint by construction. Final-test seeds must never enter
  tuning/search/validation. Stream derivation:
  `(root, namespace, outer, lifetime, stream) -> SHA-256 -> ChaCha8Rng`,
  one RNG instance per stream, so agent draws never perturb environment
  schedules (see `src/rng.rs`).

## Planned commands (not implemented)

`benchmark`, `evolve`, `evaluate`, `intervene`, and
`python analysis/validate_logs.py` arrive in their milestone tasks
(M0-14, M5, M7–M9). Do not treat their absence as a failure of M0.

## Run directories and artifact policy

- Runs live in `runs/<profile>-root<R>-outer<O>-<unixsecs>[/-retryN]/` and
  are **git-ignored** (see `.gitignore`). Source commits carry code, configs,
  manifests, tests, and docs — never large generated data or build output.
- Each run directory holds at minimum `resolved_config.toml` (complete
  effective configuration, not the small override), `manifest.json` (code
  revision + dirty status, seed policy, RNG policy), and
  `seed_streams.json` (derived stream seeds for audit).
- Raw output is immutable; derived analysis goes elsewhere.
  `docs/experiments.md` is append-only, including failures.

## Layout

`spec.md`, `to-do.md`, `AGENTS.md` at root. `src/` holds `config.rs`
(versioned TOML schema), `rng.rs` (seed derivation), `run.rs` (provenance),
and thin `main.rs`. `configs/` holds `env_smoke.toml` (M0 smoke) and
`debug_stationary.toml` (spec 19.2 reference + seeds). `manifests/` reserves
disjoint seed ranges per namespace. `analysis/` is stubbed until M0-14.
`docs/decisions.md` records scientific ambiguities/deviations;
`docs/experiments.md` is the append-only experiment log.
