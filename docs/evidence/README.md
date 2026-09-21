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
