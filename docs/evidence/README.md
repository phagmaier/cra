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

## M3-01 plastic state

The [M3-01 summary](m3-01/summary.md) records the first plastic-state
implementation: `P`/`E` separate from immutable `W0`, both plastic masks,
`persistent` versus `no_decay_diagnostic` trace policies, one
effective-weight cache writer, and validated `PlasticSnapshot`
serialization. Source hashes identify the reviewed snapshot. It is a
code-only task with deterministic fixture tests and no lifetime simulation,
so it contains no run directory, seed consumption, or acquisition claim.

## M3-02 feedback updates

The [M3-02 summary](m3-02/summary.md) records the exactly-once gated `P`
update and running baseline: old-baseline `delta`, ordered raw/limited/`P`
clamps with separated reports at both boundaries, duplicate rejection with
unchanged state, and snapshot v2 carrying baseline plus dedup. Source
hashes identify the reviewed snapshot. It is a code-only task with
deterministic fixture tests and no lifetime simulation or acquisition
claim.

## M3-03 golden fixture

The [M3-03 summary](m3-03/summary.md) records the Section 17.3
hand-calculated chain (`0.4` / `0.67` / `0.4` / `0.00067` / `0.10067` /
`0.64`) through the public score/eligibility/update entry points, plus
separate clipped cases. Fixture-only task with no production change and
no acquisition claim.

## M3 pre-integration hardening

The [M3 preflight summary](m3-preflight/summary.md) records the corrective
stream/API/state-invariant work before M3-04: dedicated hidden cue-membership
RNG, named feedback-update parameters, and plastic snapshot v3 with enforced
bound compatibility. It records the intentional hidden-role seed migration,
full verification, and remaining runner-order/boundary work. No learner run or
acquisition claim was added.

## M4 continuous checkpoint replay

The [M4-06 summary](m4-06/summary.md) records schema-4 exact continuation of
the fully persistent learner at three nonzero-`P/E` boundaries, including
pending feedback/latch state, post-feedback deduplication, all live RNGs, and
validated effective-cache derivation. It preserves and re-runs the schema-3
episodic compatibility suite; schema 2 remains unchanged. This is replay
evidence only and does not establish continuous acquisition.
