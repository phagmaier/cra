# M3-10 reproducible learning evidence and updated checkpoints

Date: 2026-09-21 UTC
Task: M3-10
Base revision: `0d20d45` plus M3-08/M3-09/M3-10 worktree changes (hashes below).
All checks run on linux/x86_64 (reference platform).

## Scope

What "reproducible learning evidence" means for the episodic
diagnostic, archived in one bundle:

- Working and failure configurations (`working_config.toml`,
  `failure_case.json`).
- All development grid results and paired controls (M3-06/M3-07/M3-08
  records, referenced — not duplicated — with the cross-check below).
- Actual update summaries (`update_summary_example.json`: first/last
  learning events of the showcase run with separated raw/limited/
  actual norms).
- Learned-offset checkpoints (`checkpoints/boundary_r1000.json`,
  `checkpoints/final.json`): real schema-3 captures with nonzero
  `P`/`E`.
- Exact replay through learning events: the M1 schema-2 checkpoint
  is untouched; the plastic learner gets its own versioned
  `LearningCheckpoint` (schema 3), and splits at three
  learning-sensitive points resume bit-identically.

Production changes:

- `src/experiments/episodic.rs`: `EpisodicAgentSnapshot`
  (live `h`/`a`/`xi`/`q`, readout, tick count, both RNG positions,
  sampling record, versioned `PlasticSnapshot`) plus
  `snapshot()`/`restore()` with shape/finiteness/RNG-identity and
  learning-section (mask/trace/`tau_e`/bound) agreement checks.
  Stream names are now single-spelled associated constants (values
  unchanged: `actor_noise`/`tie_break`).
- `src/checkpoint.rs`: `LearningCheckpoint` (schema 3, atomic
  write/checksum/seed-identity/config-hash/tick-agreement/ledger-
  agreement/`Committed`-rejection/hidden-assignment/init-seed rules
  mirrored from the M1 envelope) with `capture`, `save_to_path`,
  `load_from_path`, `restore_env`, `restore_learner`, and a
  `From<EpisodicError>` mapping. The M1 `Checkpoint` (schema 2) is
  byte-for-byte the same envelope as before; neither loader reads
  the other's files.
- `tests/episodic_checkpoint.rs` (new): faithful manual driver plus
  8 fast tests and 1 ignored bounded archival capture.

## Replay evidence (the M3-10 verify)

A manual driver mirroring `run_episodic_lifetime` tick for tick is
first proven bit-identical to it (actions, rewards, correctness,
resets, final `P`/`E`/baseline/dedup on a 60-outcome winner run),
so the splits resume the same scientific object. Then, on the same
seeds:

| Split point | State carried | Result |
| --- | --- | --- |
| Rollout-20 boundary | `P != 0` (20 updates), moved baseline, dedup 0–19, fresh `E` | continuation identical (actions, rewards, correctness, resets, final `P`/`E`/baseline/dedup/tick) |
| Just before feedback (outcome 20) | full-rollout `E`, prior `P`, aligned ticks, nothing pending; resumed tick re-delivers bit-identically | continuation identical |
| Post-feedback, pre-reset (outcome 20) | fresh `P`, post-outcome scores in `E`, dedup through 20 | continuation identical, dedup ends at 59 (no double-apply) |

Rejections are explicit: tampered bytes fail the checksum;
foreign seed identity is incompatible; garbage fails to parse;
missing files are I/O errors; the M1 loader rejects schema-3 files
(and vice versa — verified by mutual load failure).

Design note: a delivered-but-unconsumed reward is driver-held state
(the environment's pending slot is already taken), so the checkpoint
deliberately refuses that instant — the compat rules demand
`confirmed == consumed` and dedup agreement. The just-before-
feedback split is therefore the last fully-processed tick, whose
resume re-delivers deterministically (asserted bit-equal). An
earlier draft relaxed the tick rule for pending captures; it was
reverted once this gap was understood, and the re-delivery equality
is now an explicit assertion.

## Archive contents

- `working_config.toml`: frozen grid-index-11 winner values with the
  full mask at 2,000 outcomes (four-value derivation from
  `configs/episodic_stationary.toml` recorded in its header); pinned
  by `working_config_archive_matches_the_winner`.
- `failure_case.json`: the M3-08 outer-1 record verbatim
  (B3/B4/shuffled late accuracy 0.0 with B4 `P` L1 0.3796);
  pinned by `failure_case_archive_is_the_documented_bound`.
- `checkpoints/boundary_r1000.json` + `checkpoints/final.json`:
  real captures from the 2,000-outcome outer-2 winner run
  (`runs/m3-10-capture/`, git-ignored; ~0.2 s in release) at the
  rollout-1000 boundary and at completion. The final file reloads
  with `P` L1 4.6972 and dedup through event 1999
  (`archived_final_checkpoint_reloads_with_offsets`).
- `update_summary_example.json` + `meta.json`: first/last learning
  events distilled (event 0: delta −0.5, raw/actual L1 0.0816,
  unclipped; event 1999: delta −0.9577, raw/actual L1 0.1558,
  unclipped; 57/2,000 outcomes had ≥1 clipped edge) with run
  identity and provenance.
- Cross-check: the archival run's final `P` L1 (4.697157004558481),
  baseline (0.9385005855860952), and full accuracy (0.528) exactly
  match the M3-08 outer-2 seed record through an independent driver,
  tying the grid results, the replay tests, and the checkpoints to
  one trajectory.

## Verification

```text
cargo test --locked --test episodic_checkpoint (8 passed)
CRA_M3_10_DIR=runs/m3-10-capture cargo test --release --locked --test episodic_checkpoint m3_learning_evidence_capture -- --ignored --exact
cargo fmt --all -- --check (after cargo fmt --all)
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked (289 passed, 0 failed, 6 ignored)
cargo test --locked --doc (2 passed)
python3 analysis/test_validate_logs.py (17 passed)
git diff --check
```

Six ignores: both Monte Carlo diagnostics, the weight-printing
probe, and the three bounded captures (M3-07 sweep, M3-08
comparison, M3-10 archival) — each invoked separately per its
evidence record.

## Source and artifact hashes (worktree at verification)

```text
87923e1991c7fbbd76bdf26816232c1e0b6002d75bb1924e86b019a2f3b78afb  src/checkpoint.rs
4cc0d2affb0991ed895874df3c6a01fea5fe9db08afdaa223ad0e16aaa76a870  src/experiments/episodic.rs
4bd56bf621f308a867f630110756e1c7412db4c43baecbc17f0934bfb4a26526  src/experiments/mod.rs
6227e04ddc8d951cb01081de27185c413f9ebc0e3a075a3dd948f6f2dc657bb7  tests/episodic_checkpoint.rs
906a95f66785055f7f3951f3e9cb1c4fbb1601e519293c70e64dbed5b743953e  docs/evidence/m3-10/working_config.toml
644ae7e1ba2c3bd882204630e997e72d9cba5b3d72a7e75c61ddb61159c038b4  docs/evidence/m3-10/failure_case.json
c3bf0f92e38a0498fe777a452fe9836ebfd4ba8973a8845f25742a55f35234be  docs/evidence/m3-10/checkpoints/final.json
cd5d495bc1a4ce8aa094fc4e59d71166efb14045d8f0fc20e336ee1360ccdcab  docs/evidence/m3-10/checkpoints/boundary_r1000.json
174c12191bfd1230993a5823bfb8a0d83617f5d80530c006d18afc82ea0d91a3  docs/evidence/m3-10/update_summary_example.json
```

Base commit `0d20d45` with uncommitted M3-08/M3-09/M3-10 files plus
tracker/docs edits at verification time.

## Interpretation and claim limits

- What is proven: correct arithmetic (M3-01–M3-03 unit/golden
  fixtures, still passing), episodic acquisition on responsive
  actors (M3-07/M3-08 sweeps, controls matched), and exact
  pause/resume of the learning lifetime with nonzero `P`/`E`
  (this task). The three are distinguished, not conflated.
- What is NOT proven: continuous acquisition without
  within-lifetime resets — that is M4's explicitly gated claim, and
  nothing here substitutes for it. No gate, search, or evolution
  claim follows.
- Archived checkpoints pin the reference platform
  (linux/x86_64, this code version): loading elsewhere fails the
  platform check by design (AGENTS.md tolerance policy).
