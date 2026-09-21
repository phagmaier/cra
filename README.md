# Learning When to Learn

Implementation of [`spec.md`](spec.md) (v0.1, Sept 20 2026): does internally
generated gating improve a continuously running recurrent agent's adaptation
to real changes without damaging stable associations under misleading
feedback?

Status: **M0 complete and re-verified; next task M1-01.** The environment core
exists as a library (`src/environment/`, `src/experiments/baseline.rs`,
`src/logging/`) with deterministic fixtures, randomized checks, baseline
runners, and an offline log audit. No neural dynamics, plasticity,
evolution, or comparison pipeline exists yet — M1 starts from the frozen
M0 baseline below. Anything listed under "Planned" is a target from spec
Section 18 / `to-do.md`, not working code.

M0 was re-reviewed and corrected without changing the original smoke
trajectories. See [the review](docs/m0-review.md) for findings, verification,
and the remaining M1 integration work.

## Continue development

Start with [AGENTS.md](AGENTS.md), the [current tracker](to-do.md), and
the [agent continuation guide](docs/handoff.md). The guide maps existing
code/tests to M1-01 and records the integration limits to preserve.

| Document | Responsibility |
| --- | --- |
| [spec.md](spec.md) | Scientific contracts and proposed design; Section 9 owns tick ordering |
| [to-do.md](to-do.md) | Current status, ownership, ordered tasks, gates, and completion evidence |
| [AGENTS.md](AGENTS.md) | Durable work conventions and scientific safeguards |
| [docs/handoff.md](docs/handoff.md) | Current continuation notes and implementation entry points |
| [docs/decisions.md](docs/decisions.md) | Dated decisions and superseding corrections |
| [docs/experiments.md](docs/experiments.md) | Append-only run records, including failures |

The spec's original unchecked implementation list is historical design
text. Use the tracker for completion state. Historical evidence retains
the versions, counts, and limitations that applied when it was recorded.

## Toolchain (pinned)

- Rust `1.98.0` (see `rust-toolchain.toml`), Cargo `1.98.0`, resolution in
  `Cargo.lock` (committed).
- Python `3.14.7` (see `.python-version`) for the offline analysis layer;
  no third-party Python dependencies (see `analysis/requirements.txt`).
  Add a dependency lock when analysis first needs external packages.
- Key Rust deps (pinned in `Cargo.lock`): `clap 4.6.7`, `serde 1.0.229`,
  `serde_json`, `toml 0.8.23`, `thiserror 2.0`, `rand 0.9.5`,
  `rand_chacha 0.9.0`, `sha2 0.10.9`. No deep-learning framework.

## Verification

Run commands from the repository root with the pinned toolchains.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
python3 analysis/test_validate_logs.py
```

The last full M0 review recorded **73 Rust tests and 14 Python tests passing**,
with fmt/clippy clean. See [recorded commands and outputs](docs/evidence/m0-review/commands.json).
Those counts describe that execution; rerun relevant checks after changes.

## Implemented commands (M0)

```bash
cargo run --release --locked -- validate-config configs/env_smoke.toml
cargo run --release --locked -- validate-config configs/debug_stationary.toml
cargo run --release --locked -- simulate --config configs/env_smoke.toml --baseline random --lifetimes 2 --seed 1 --outer-seed 1
cargo run --release --locked -- simulate --config configs/env_smoke.toml --baseline oracle --lifetimes 2 --seed 1 --outer-seed 1
python3 analysis/validate_logs.py analysis/fixtures/valid
```

To audit a new run, pass the directory printed after `run dir:` to
`python3 analysis/validate_logs.py`. The last command above audits the
committed fixture, so it also works when local `runs/` output is absent.

Baselines: `random` (B0), `constant-0` / `constant-1` (B1), `oracle`
(O1, privileged reference). The audit checks manifest/config/condition
identity, derived seed streams, declared lifetime lengths, timing,
finite values, duplicate feedback, hidden reward/change accounting, and
completion. Fixtures and mutation tests live in `analysis/`.
With `event_log=false`, it reports reduced provenance/completion coverage.

Notes:

- `validate-config <file>` parses and validates the TOML against the
  `schema_version = 1` schema (spec 19). Unknown fields/mode names are
  rejected. Known future profiles may validate without being executable.
- M0 `simulate` requires an environment-only configuration. Neural/search
  sections, diagnostic reset modes, `isolated_reversal`, and `long_life`
  are rejected before creating a run. `debug_stationary.toml` is currently
  a validation reference, not an executable learner.
- `simulate` runs real baseline lifetimes (M0-07–M0-11 contracts) into a
  fresh run directory with `resolved_config.toml`, `manifest.json`,
  `seed_streams.json`, `condition.json`, and `completion.json`, plus
  `events.jsonl` and `hidden.jsonl` when event logging is enabled.
  Same-second runs never share a directory
  (`-retryN` suffix, atomic directory reservation). Existing incomplete
  runs are preserved. Library behavior is also exercised by
  `cargo test --locked --test environment_contract`.
- Seed namespaces (`development`, `training`, `validation`, `final_test`)
  are disjoint by construction. Final-test seeds must never enter
  tuning/search/validation. Stream derivation:
  `(root, namespace, outer, lifetime, stream) -> SHA-256 -> ChaCha8Rng`,
  one RNG instance per stream, so agent draws never perturb environment
  schedules (see `src/rng.rs`).
- The [manifest reservations](manifests/README.md) document seed policy;
  M0 does not load them or enforce their numeric ranges. Resolved config
  and explicit CLI overrides identify the seeds actually used.

## Planned commands (not implemented)

`benchmark`, `evolve`, `evaluate`, and `intervene` arrive in their
milestone tasks (M5, M7–M9). `aggregate.py` arrives with the comparison
pipeline in M8. Do not treat their absence as a failure of M0.

## Run directories and artifact policy

- Runs live in `runs/<profile>-root<R>-outer<O>-<unixsecs>[-retryN]/` and
  are **git-ignored** (see `.gitignore`). Source commits carry code, configs,
  manifests, tests, and docs — never large generated data or build output.
- Each run directory holds `resolved_config.toml` (complete effective
  configuration), `manifest.json` (code revision + dirty status, platform,
  toolchain, seed/RNG policy, condition), `seed_streams.json` (derived
  stream seeds), `condition.json`, `events.jsonl` (ordinary records),
  `hidden.jsonl` (evaluator annotations), and `completion.json` (terminal
  status + counts).
- Raw output is immutable; derived analysis goes elsewhere.
  `docs/experiments.md` is append-only, including failures.
- Small reproducibility records live in `docs/evidence/`; original raw
  runs may exist only in the originating workspace. A fresh checkout can
  use the committed fixture or rerun saved commands into a new directory.

## Layout

`spec.md`, `to-do.md`, `AGENTS.md` at root. `src/` holds `config.rs`
(versioned TOML schema), `rng.rs` (seed derivation), `environment/`
(observation boundary, hidden state, scheduling, features, rewards),
`agent/topology.rs` (M1-01 inherited mask, fixed motor pools, stable edge
order, structural validation), `experiments/baseline.rs` (B0/B1/O1 harness),
`logging/` (event records + validation), `run.rs` (provenance + simulation
runner), and thin `main.rs`.
`configs/` holds `env_smoke.toml` (M0 smoke) and `debug_stationary.toml`
(spec 19.2 reference + seeds). `manifests/` reserves disjoint seed ranges
per namespace. `analysis/` holds the stdlib-only log audit plus fixtures.
`docs/decisions.md` records scientific ambiguities/deviations;
`docs/experiments.md` is the append-only experiment log.
