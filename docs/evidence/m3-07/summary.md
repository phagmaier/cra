# M3-07 motor-afferent acquisition evidence

Date: 2026-09-21 UTC
Task: M3-07
Base revision: `f405230` plus M3-06/M3-07 worktree changes (hashes below).
Sweep executed in release profile on linux/x86_64.

## Scope

First learning result: the frozen M3-06 grid run exactly as declared —
24 eta/input-scale/gain/sigma combinations x outer seeds 1-3 x matched
B3/B4/B4-shuffled, 2,000-outcome episodic lifetimes with the
motor-afferent mask. No grid/criterion change after seeing outcomes.

## Execution

```text
CRA_M3_SWEEP_DIR=runs/m3-sweep-fresh cargo test --release --locked \
  --test m3_acquisition m3_motor_afferent_sweep -- --ignored --exact --nocapture
```

- 216/216 lifetimes completed, zero failures, all values finite, matched
  schedules verified per (point, outer) inside the sweep (cue orders and
  reset ticks identical across conditions or the run aborts).
- Measured 7,343,136 ticks vs the declared 7,344,000 nominal estimate
  (864 fewer; nominal uses maximum cycle lengths, realized quiet draws vary).
- Raw aggregates: `runs/m3-sweep-fresh/{seed_records.jsonl,verdict.json}`
  (git-ignored workspace copy); archived copies:
  `docs/evidence/m3-07/{seed_records.jsonl,verdict.json}` (72 records,
  24 verdicts).
- Wall time ~16 s in release (~94 ms per 2,000-outcome lifetime).

## Verdict under the frozen criterion

Margins are late-window (final 200 choices) latent accuracy; a seed passes
on B4−B3 ≥ 0.15 AND B4−shuffled ≥ 0.10 with < 10% clipped updates and real
B4 `P` movement. A point passes on ≥ 2/3 outer seeds.

| Points passing | Points with 1 seed | Points with 0 seeds |
| --- | --- | --- |
| 3 (indices 11, 17, 19) | 7 (9, 10, 14, 15, 18, 22, 23) | 14 |

Winner by the declared tiebreak (larger minimum margin, then smaller eta,
…): **grid index 11** — eta 0.001, input_scale 0.2, recurrent_gain 0.8,
noise_sigma 0.05. (All three passers tie at minimum margin 0.0 because the
birth-locked outer-1 actor contributes 0.0 everywhere; smaller eta then
selects 11 over the eta-0.003 points 17/19, exactly as the manifest orders.)

Winner detail (late accuracy / late reward):

| Seed | B4 early → late | B3 late | Shuffled late | Margins | B4 P L1 | Clipped | Bound occ. |
| --- | --- | --- | --- | --- | --- | --- | --- |
| outer 2 | 0.035 → **0.860** / 0.860 | 0.040 | 0.040 | 0.82 / 0.82 | 2.448 | 0.0011 | 0.0000 |
| outer 3 | 0.795 → **0.935** / 0.935 | 0.770 | 0.765 | 0.165 / 0.170 | 2.509 | 0.0006 | 0.0000 |
| outer 1 | 0.000 → 0.000 | 0.000 | 0.000 | 0.00 / 0.00 | 0.065 | 0.0000 | 0.0000 |

- Outer 2 is the showcase acquisition trajectory: from chance-level early
  behavior to 0.86 late while both controls sit at 0.04 — weight change
  driven by the reward contingency, since the shuffled control (real
  updates from corrupted signals) does not follow.
- Outer 3 improves a responsive actor from 0.795 early to 0.935 late,
  clearing both margins; outer-3 B4 late accuracy reaches 0.83–0.995
  across all 24 points, so learning on a responsive actor is robust, not a
  one-point accident.
- Outer 1 is a birth-locked actor (always action 0 against an all-ones
  mapping, B3 identical): a documented family property (M1), not a
  learning failure — no offsets can help a readout the grid never unsticks.

Sweep-wide guardrails: maximum clipped-update fraction 0.017, maximum
bound occupancy 0.004, minimum B4 final-`P` L1 0.018 (every lifetime moved
real offsets), zero nonfinite values, zero failed lifetimes. No widespread
clipping or numerical failure anywhere.

## Interpretation and claim limits

- M3-07 passes: several seeds (outer 2 and outer 3) show the declared
  learning improvement over matched no-update and shuffled-reward controls
  at three grid points, with interpretable numerics. The selected
  configuration for M3-08 is grid index 11.
- The result is episodic-diagnostic acquisition with motor-afferent
  plasticity only. Continuous acquisition (no within-lifetime resets) is
  still unproven — that is M4's job. Do not cite this as the main result.
- Birth-locked actors (outer 1 here) bound the family: the claim covers
  responsive actors in the tested family, not every random initialization.
- The 0.8 debugging-target figure was not used as a threshold; the
  declared margins were. Late accuracies are reported as measured.
