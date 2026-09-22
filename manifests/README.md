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

Declared suites (not reservations): `m3_development_grid.json` is the
frozen M3-06 sweep for the episodic learner — axes, development seeds,
lengths, windows, conditions, criterion, tiebreak order, and budget,
declared before results. It is loaded and validated by
`src/experiments/grid.rs` (tested in `tests/development_grid.rs`) and
executed no earlier than M3-07. Changing it after seeing outcomes starts a
new declared revision.

`m4_timing_sensitivity.json` is the pre-results M4-05 timing/delay
diagnostic: three clean stationary timing stages, `tau_e` 16/32/64,
development outers 1–3, a 27-lifetime finite budget, measured trace/update
quantities, and an explicit no-monotonic-performance-criterion rule. It is
loaded and executed by `tests/m4_timing.rs`; changing it after the recorded
result starts a new declared revision.

`m4_continuous_acquisition.json` is the pre-results M4-07 acquisition
comparison: the frozen M3-08 full-recurrent actor family, development outers
1–3, five matched continuity/control conditions, per-cue exposure windows,
an explicit fully-persistent-versus-B3 criterion, required health/update
diagnostics, and a 15-lifetime / 510,420-tick maximum budget. It is loaded and
executed by `tests/m4_continuous_acquisition.rs`; any post-result plan change
requires a new declared revision and cannot retroactively change this verdict.

`m4_continuity_audit.json` is the pre-analysis M4-08 failure audit: it
references the frozen M4-07 manifest by SHA-256, reuses its actor family and
development seeds descriptively, and declares 8 paired audit legs (outer-2
trio plus matched B3, outer-3 contrast, outer-1 control, outer-2 tau_e 16/64
with a recorded spec-7.9 reason), the per-outcome diagnostic series, and a
16,000-outcome / 272,224-tick maximum budget. It sets no acquisition
criterion. It is loaded and executed by `tests/m4_continuity_audit.rs`.

`m4_lock_localization.json` is the pre-analysis M4-08b probe plan: the
frozen M3-08 family, outers 2–3, a 60-outcome lock-in validity gate, and
dose/flip/converge synthetic-drive matrices with paired noise streams and
a 76,768-transition budget. It introduces no new range and sets no
acquisition criterion. It is loaded and executed by
`tests/m4_lock_localization.rs`.

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

`m4_escape_sweep.json` (M4-08c) pins the 12-point eta/tau escape sweep to
the immutable M4-07 manifest and aggregate archive hashes. Development
root 1/outers 1–3/lifetime 0, 39 lifetimes / 1,327,092 ticks; the six
replication/control aggregate records run first and must match exactly.
The executed result is negative (no settings adopted). See
[M4-08c](../docs/evidence/m4-08c/summary.md) for execution, claim limits,
and the negative-package route. This is not a M5 range expansion or a
reservation of new confirmation seeds.
