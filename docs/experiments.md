# Experiments

Append-only experiment log. Newest entries go at the end. Record the
question, code/configuration hashes, what changed, observed result,
interpretation, and next decision. Include null results, failures, and
interrupted runs — missing data is not a zero score.

## 2026-09-21 — M0-01–M0-05 bootstrap (no empirical result)

- Question: none (engineering scaffold, not an experiment).
- Code/config: M0 scaffold (`src/config.rs`, `src/rng.rs`, `src/run.rs`,
  thin CLI); configs `env_smoke.toml`, `debug_stationary.toml`; manifests
  reserve disjoint outer-seed ranges. See the `to-do.md` completion ledger
  for exact commands, outcomes, and artifact paths.
- Result: `cargo test`, `validate-config` on both profiles, and scaffold
  `simulate` provenance runs. No learning claim; no statistical check.
- Next: M0-06 (observation boundary types).
