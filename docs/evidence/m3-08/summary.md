# M3-08 all-recurrent acquisition evidence

Date: 2026-09-21 UTC
Task: M3-08
Base revision: `0d20d45` plus M3-08 worktree changes (hashes below).
Comparison executed in release profile on linux/x86_64.

## Scope

Mask extension at fixed hyperparameters — not a re-tune. The frozen
M3-06 manifest (`manifests/m3_development_grid.json`, schema 1) is
untouched; the M3-07 motor-only sweep is preserved as a diagnostic
(`docs/evidence/m3-07/`) and never overwritten. This task runs the
declared development comparisons (matched B3/B4/B4-shuffled, development
root 1, outers 1–3, 2,000-outcome lifetimes, first-200/final-200
windows, margins B4−B3 ≥ 0.15 and B4−shuffled ≥ 0.10 on ≥ 2/3 seeds
with zero-failure/<10%-clipping/`P`-movement guardrails) at the single
frozen winner point — grid index 11 (eta 0.001, input_scale 0.2,
recurrent_gain 0.8, noise_sigma 0.05) — with the plastic mask changed
from `motor_afferent_only` to `all_recurrent_edges`. No production
code changed: the runner, score, update, and analysis are the verified
M3-01/M3-02/M3-04/M3-05/M3-07 paths.

## Execution

```text
CRA_M3_FULL_SWEEP_DIR=runs/m3-full-final cargo test --release --locked \
  --test m3_full_recurrent m3_full_recurrent_comparison -- --ignored --exact --nocapture
```

- 9/9 lifetimes completed, zero failures, all values finite, matched
  schedules verified per outer seed inside the comparison (cue orders
  and reset ticks identical across B3/B4/shuffled or the run aborts).
- Measured 305,964 ticks vs the nominal 306,000 estimate for this
  9-lifetime subset (36 fewer, exactly 4 per lifetime: warmup 0 starts
  the first rollout directly at cue presentation per the documented M0
  warmup convention, so each lifetime skips one 4-tick quiet interval —
  the same deterministic shortfall as the M3-07 sweep, which measured
  7,343,136 vs 7,344,000 nominal over 216 lifetimes).
- Raw aggregates: `runs/m3-full-final/{seed_records.jsonl,verdict.json}`
  (git-ignored workspace copy); archived copies:
  `docs/evidence/m3-08/{seed_records.jsonl,verdict.json}` (3 records,
  1 verdict). An identical pilot run (`runs/m3-full-fresh`) confirmed
  the verdict before a behavior-preserving test cleanup; the archived
  records are the final re-run.
- Wall time ~0.7 s in release (~78 ms per 2,000-outcome lifetime).

## Verdict under the frozen criterion

Margins are late-window (final 200 choices) latent accuracy; a seed
passes on B4−B3 ≥ 0.15 AND B4−shuffled ≥ 0.10 with < 10% clipped
updates and real B4 `P` movement. The point passes on ≥ 2/3 outer
seeds. Result: **PASSES 2/3 seeds** (outers 2 and 3; outer 1 is the
known birth-locked actor, 0.0 everywhere under every mask).

Winner-point detail, full-recurrent mask (late accuracy / late reward):

| Seed | B4 early → late | B3 late | Shuffled late | Margins | B4 P L1 | Clipped | Bound occ. |
| --- | --- | --- | --- | --- | --- | --- | --- |
| outer 2 | 0.045 → **0.975** / 0.975 | 0.040 | 0.030 | 0.935 / 0.945 | 4.697 | 0.0007 | 0.0000 |
| outer 3 | 0.790 → **0.955** / 0.955 | 0.770 | 0.775 | 0.185 / 0.180 | 3.671 | 0.0006 | 0.0000 |
| outer 1 | 0.000 → 0.000 | 0.000 | 0.000 | 0.00 / 0.00 | 0.380 | 0.0000 | 0.0000 |

Motor-only reference at the same point (M3-07, preserved):

| Seed | B4 early → late | B3 late | Shuffled late | Margins | B4 P L1 |
| --- | --- | --- | --- | --- | --- |
| outer 2 | 0.035 → 0.860 | 0.040 | 0.040 | 0.82 / 0.82 | 2.448 |
| outer 3 | 0.795 → 0.935 | 0.770 | 0.765 | 0.165 / 0.170 | 2.509 |
| outer 1 | 0.000 → 0.000 | 0.000 | 0.000 | 0.00 / 0.00 | 0.065 |

- Both responsive seeds clear both margins under the full mask, with
  stronger late accuracy than the motor-only diagnostic (0.975 vs
  0.860 on outer 2; 0.955 vs 0.935 on outer 3) and larger acquired
  offset norms (4.70/3.67 vs 2.45/2.51), as expected with ~4× the
  plastic parameters at the same eta.
- The shuffled control moves real offsets (P L1 2.28–3.68) without
  systematic latent-accuracy gain, so the B4 improvement is driven by
  the reward contingency, not by update magnitude alone.
- Guardrails pass: maximum clipped-update fraction 0.0007, maximum
  bound occupancy 0.0000, every lifetime moved P, zero nonfinite
  values, zero failed lifetimes.
- Mask change is proven structural and behavioral in the fast test:
  on the same inheritance the full plastic set strictly contains the
  motor-afferent subset, missing/nonplastic edges stay exactly zero,
  effective weights equal `W0 + P`, and the 12-outcome full-mask B4
  run moves a non-motor offset.

## Interpretation and claim limits

- M3-08 passes: the ungated learner with **all recurrent plastic
  edges** shows the declared learning improvement over matched
  no-update and shuffled-reward controls on responsive actors. This
  is the actor family carried into M4.
- The result is still episodic-diagnostic acquisition (explicit
  per-rollout resets, `no_decay_diagnostic` traces). Continuous
  acquisition with no within-lifetime resets is still unproven —
  that is M4's job. Do not cite this as the main result.
- Birth-locked actors (outer 1 here, identical under both masks)
  bound the family: the claim covers responsive actors in the tested
  family, not every random initialization.
- The two masks were compared at one fixed hyperparameter point;
  later gate comparisons (M6–M8) must stay mask-matched and must not
  attribute every mask difference to gates.
- Checkpoint embedding and split replay with nonzero `P`/`E`
  remain M3-10/M4-06 work; the top-level checkpoint schema stays 2.

## Source hashes (worktree at verification)

```text
68f249a872f7be64f30facc4a10a370847117724cdb32e67e4502c8cc1ef1579  tests/m3_full_recurrent.rs
08024e707915208bc9c0a384c1c47cdef7c1ec005738f4ce0c6b203777c85de5  src/experiments/sweep.rs
f20711d20b7ae2c204f85c95125a65f69398e3a652c0679f799da62337941696  src/experiments/episodic.rs
574116ca4dd2d250862c70b7090ab019adafbc2309b5f2eaf6415c47012bcbe1  manifests/m3_development_grid.json
70a5266b5e15d15b8cd73612af82f428c56d949aa0871590d0f3e070dc0266fc  configs/episodic_stationary.toml
```

Base commit `0d20d45`; `git status` at the archived comparison time
showed `M README.md`, `M docs/decisions.md`, `M docs/handoff.md`,
`M to-do.md` plus untracked `docs/evidence/m3-08/` and
`tests/m3_full_recurrent.rs` (full status in `verdict.json`
provenance).
The 0.8 debugging-target figure was not used as a threshold; the
declared margins were.
