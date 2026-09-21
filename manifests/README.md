# Seed namespace reservations

These JSON files record policy reservations. The M0 runner does not load
these files, enforce their numeric ranges, or treat them as executable
suites. Actual seeds come from `[seeds]` in the config, with explicit root
and outer overrides from the CLI, and are recorded in each run directory.

| Namespace | Reservation | Current use |
| --- | --- | --- |
| `development` | `development.json`, outer seeds 1–9999 | M0 diagnostics and smoke runs have used this namespace; M1 development continues here |
| `training` | No batch manifest yet | M7 will define fresh per-generation batches and record their coordinates |
| `validation` | `validation.json`, outer seeds 10001–19999 | Reserved for later checkpoint selection; no selection runs recorded |
| `final_test` | `final_test.json`, outer seeds 90001–99999 | Reserved; no final-test outcomes inspected |

Separation is derived from the full tuple
`(root_seed, namespace, outer_seed, lifetime_index, stream_name)` in
[`src/rng.rs`](../src/rng.rs). Distinct namespace strings separate streams
independently of numeric range reservations. Seed-derivation tests may
check namespace separation without simulating reserved final-test outcomes.

Use [`docs/experiments.md`](../docs/experiments.md), the tracker ledger,
and each run's `manifest.json`/`seed_streams.json` for actual execution
evidence. The M0 review records development root 1/outer 1/lifetimes 0–1
for smoke runs; randomized tests declare their additional seeds in source.
A reservation is not a claim that every seed in the range was used.

When suite execution is implemented, document its schema, seed allocation,
selection rules, and consumed work in the relevant milestone. Keep final
study inspection behind the declared M10 protocol. Do not change reserved
namespaces or derivation merely to avoid an unfavorable result.
