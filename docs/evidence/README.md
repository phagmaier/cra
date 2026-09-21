# Verification evidence

Evidence bundles describe a particular execution and source snapshot.
Preserve existing files when adding new results. Use a new bundle or an
appended tracker record for later checks; do not update old hashes to match
the current working tree.

## M0 review bundle

[`m0-review/commands.json`](m0-review/commands.json) contains executed
argument lists, exit codes, expected exit codes, and captured output. The
neural-profile simulation intentionally exited 1 to verify rejection;
that entry is not a failed learning run.

[`m0-review/summary.json`](m0-review/summary.json) contains the base Git
revision and dirty status, source and release-binary SHA-256 fingerprints,
run directories and artifact hashes, and counts reconstructed from logged
events. Missing event-derived fields on a logging-disabled run are not
zero-valued measurements. CLI output records its reported reward/counts.

The bundle was collected on top of `57b6870` with corrections uncommitted;
those corrections were subsequently committed as `8216c14`. Later edits,
including documentation comments, can change source-file hashes. Compare
fingerprints against the recorded snapshot rather than assuming they are
checksums for the latest checkout.

The two TOML files are small development diagnostics for variable timing,
noise, and logging parity. They are not nominal search configurations or
reserved final-test data. Exact seed coordinates are in the summary and
resolved run configs.

## Raw data availability

Raw review output lives under `runs/m0-review/` in the originating workspace
and is ignored by Git. Saved hashes do not make those files available on
another machine. If needed, rerun the saved commands into fresh directories
and record the new run identity. The committed
[valid audit fixture](../../analysis/fixtures/valid/manifest.json) remains
available for a small audit without restoring raw runs.

For current verification commands see the [README](../../README.md#verification).
For status and completed gates use the [tracker](../../to-do.md), not the
presence of an evidence directory alone.


## M1 review bundle

[`m1-review/commands.json`](m1-review/commands.json) and
[`m1-review/summary.json`](m1-review/summary.json) record fresh commands,
results, source/binary hashes, thirteen audited run paths (eleven corrected,
two original-commit comparisons) and artifact hashes. Corrections start from
`13a4873`; no original raw run was overwritten. `regressions-before.txt`
preserves the demonstrated pre-fix checkpoint failures.

`observability/*.json` contains bounded measured diagnostics; the first
B3 demo's health/traces are in `demo-diagnostics.json`. These small artifacts
are committed; full raw outputs are ignored under `runs/m1-review/` and
`runs/m1-review-original/`. The two TOMLs define the noisy logging parity
check, not a search or final-test experiment. See [the review](../m1-review.md)
for findings and scientific claim limits.


## M2 score diagnostics

The [M2-06 bundle](m2-06/summary.md) records the completed M2 gate. Run the
[README package command](../../README.md#verification) into a fresh directory
for all deterministic checks and both explicitly bounded Monte Carlo runs.
The package records actual test execution and rejects failed or incomplete
output; merely having a directory or a zero-test Cargo success is insufficient.

[Original direction evidence](m2-03/summary.md),
[finite-rollout evidence](m2-04/summary.md), and
[original recurrent evidence](m2-05/summary.md) remain unchanged. M2-06
replays their pinned fixtures/seeds and saves fresh matching results, not
new independent samples. Source hashes identify the execution snapshot;
the complete sampled configuration and uncertainty live in each result JSON.
The conditional-density fixture and its analytical expressions are preserved
as source with the measured deterministic output. No unbiasedness or
convergence claim follows for the main online learner.
