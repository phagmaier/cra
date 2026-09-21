# M3-06 development grid and acquisition criterion evidence

Date: 2026-09-21 UTC
Task: M3-06
Base revision: `f405230` (committed M3-04/M3-05) plus M3-06 worktree
changes (see source hashes below)

## Scope

Pre-results declaration only. No sweep executed, no lifetime run, no outcome
observed, no tuning performed.

- `manifests/m3_development_grid.json` (new, schema 1): frozen grid over
  `eta` {3e-4, 1e-3, 3e-3}, `input_scale` {0.2, 0.3}, `recurrent_gain`
  {0.5, 0.8}, `noise_sigma` {0.03, 0.05} (24 points) on the
  `episodic_stationary` base; development namespace, root 1, outer seeds
  1-3, lifetime 0; 2,000 outcomes per lifetime with early-first-200 and
  late-final-200 windows; matched conditions B3/B4/B4-shuffled at every
  point; late-window latent-accuracy margins (B4-B3 >= 0.15, B4-shuffled >=
  0.10) on >= 2/3 outer seeds plus zero-failure, <10% clipping, and B4
  P-movement guardrails; deterministic tiebreak order; no-pass rule
  (M3-09 reduction, gate stays open); self-consistent budget of 216
  lifetimes / 7,344,000 nominal ticks. The spec's suggested final-200
  accuracy above 0.8 is recorded as a debugging target, explicitly not the
  threshold.
- `src/experiments/grid.rs` (new): loader plus validator. `instantiate`
  expands each point into a candidate config and proves it passes
  `validate_episodic_execution` — validation only, never execution.
  Combination order is fixed (`eta` major, `noise_sigma` minor) and
  documented. The tick estimate is derived from the base profile's maximum
  cycle length, not invented.
- `tests/development_grid.rs` (new): manifest frozen-ness, fixed ordering,
  validation-only instantiation of all 24 points (only the four swept
  values plus lifetime length differ from base), derived budget equality,
  spec-target wording, and rejection of nine invalid-declaration mutations.
- `manifests/README.md`: notes the grid file as a declared M3-06 suite
  alongside the seed reservations.
- Checkpoint schema untouched at 2. `simulate` guards unchanged. No
  production learning behavior changed.

## Verification

Commands executed (repository root, pinned toolchains):

```text
cargo test --locked --test development_grid
cargo test --locked --test episodic_runner
cargo test --locked --test episodic_controls
cargo fmt --all -- --check (after cargo fmt --all)
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
python3 analysis/test_validate_logs.py
git diff --check
```

Results:

- `development_grid`: 6 passed, 0 failed. Covers frozen values, fixed
  combination order with distinctness and out-of-range `None`,
  validation-only instantiation of all 24 points, derived budget equality
  (7,344,000 ticks), spec-target wording, and nine invalid-mutation
  rejections (namespace, empty axis, zero noise, overlapping windows,
  swapped conditions, inconsistent budget, unknown tiebreak token,
  out-of-range and duplicate outer seeds, wrong base profile).
- `episodic_runner`: still 10/10 pass. `episodic_controls`: still 5/5 pass.
- Full fast Rust suite: 271 passed, 0 failed (6 new grid tests),
  3 intentional ignores (M2-03 million-sample, M2-05 recurrent Monte
  Carlo, weight-printing probe), 2 compile-fail doc checks pass.
- Python audit suite: 17 passed.
- fmt, Clippy `-D warnings` (one doc-indentation finding fixed), and
  `git diff --check` clean.
- Slow M2 Monte Carlo diagnostics were not rerun (score mathematics
  unchanged). No lifetimes executed for this task by design; development
  namespace declared for M3-07. No final-test seeds involved.

## Source hashes (worktree at verification)

```text
574116ca4dd2d250862c70b7090ab019adafbc2309b5f2eaf6415c47012bcbe1  manifests/m3_development_grid.json
669c349202e889b13ea71ef640784ff363dd5b0e5329d3b7ce6ae80096be6d42  src/experiments/grid.rs
febbbd229570b60bcbdf86c1b4f6f7b8e58912666c4bba48241fd2211499acb8  tests/development_grid.rs
```

Base commit `f405230` (M3-04/M3-05 committed); `git status` showed only
the M3-06 files above plus tracker/docs edits at verification time.

## Interpretation and claim limits

- The sweep plan is frozen before results: axes, seeds, lengths, windows,
  conditions, margins, tiebreak, no-pass rule, and budget are all declared
  and machine-checked. M3-07 must run this plan as written; changing axes,
  seeds, windows, or thresholds after seeing outcomes starts a new declared
  revision, not a silent edit.
- No learning evidence exists yet. The 0.8 figure is a debugging target
  from the spec, not a claim about attainability; the declared margins are
  the only thresholds.
- Continuous acquisition remains unverified; M4 owns the persistent
  condition. Do not cite this declaration as a result.
