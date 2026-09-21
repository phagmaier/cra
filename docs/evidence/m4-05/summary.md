# M4-05 evidence — gradual timing and delayed outcomes

Date: 2026-09-21 UTC. Base `1e0db3d`; M4-05 worktree dirty by construction.
Task: [M4-05](../../../to-do.md#m4---remove-artificial-trial-resets) (spec
5.7, 7.3, 7.8–7.9, 9–10, 16/M4, 17.2).

## Declared development stages

`manifests/m4_timing_sensitivity.json` was written before the bounded
measurement. It fixes development root 1, outers 1–3, lifetime 0, 256
outcomes, `tau_e` values 16/32/64, 27 total lifetimes, 6,912 outcomes, and a
270,180-tick upper bound. It defines three clean stationary profiles:

| Stage | Profile | quiet | cue | gap | response | reward delay |
| ---: | --- | --- | ---: | --- | ---: | --- |
| 0 | `continuous_stationary` | 4 | 8 | 0 | 4 | 1 |
| 1 | `continuous_variable_short` | 4..8 | 8 | 0..4 | 4 | 1..4 |
| 2 | `continuous_variable_delayed` | 8..16 | 16 | 0..8 | 8 | 8..24 |

The two new profiles differ from stage 0 only in `profile_name` and the five
declared timing fields. All retain `stationary_clean`, zero hazard/noise,
`birth_only` + `persistent`, fixed gate 1, and `max_pending_choices = 1`.
Stage 2 is the spec-5.7 main starting timing table.

## Contract checks

`tests/m4_timing.rs` adds four default tests plus one explicit bounded
diagnostic:

- Each profile is table-matched to the manifest and normalized equality proves
  no non-timing setting changed.
- Low and high endpoints of every quiet/gap/delay range are converted to fixed
  schedules. Two-choice tick arithmetic pins both commitment and feedback
  ticks, including `feedback = commit + delay`, final-feedback completion, and
  the single-pending-choice phase order.
- For every profile, matched `tau_e` 16/32/64 runs share `W0` and an identical
  `(event, cue, commit tick, feedback tick, noise bit)` schedule. Agent actions
  may diverge, but the environment streams do not.
- `ContinuousChoice.eligibility_l1_before_update` records the actual live trace
  read at feedback. A fixture independently proves
  `raw_update_L1 = eta * |delta| * eligibility_L1` for fixed gate 1.

## Bounded measurement

Command:

```bash
CRA_M4_TIMING_DIR=docs/evidence/m4-05/diagnostic \
  cargo test --release --locked --test m4_timing \
  m4_timing_sensitivity_diagnostic -- --ignored --exact --nocapture
```

Result: 27/27 lifetimes, 6,912/6,912 outcomes, 220,005 measured ticks under the
270,180 maximum; all finite and complete; all within-profile schedules paired
across `tau_e`. `records.jsonl` stores one record per profile/outer/`tau_e`
with the resolved-config hash, schedule hash, delay-retention factors, actual
pre-feedback trace L1 distribution, raw/limited/actual update L1 distributions,
clipping, bound occupancy, final P/E/baseline, reward, accuracy, outcomes, and
ticks.

Means below average each record's 256-event mean over outers 1–3. They are
descriptive scales, not independent-replicate uncertainty estimates or an
acquisition criterion.

| Stage | `tau_e` | mean pre-feedback E L1 | mean actual-update L1 | mean clipped fraction | max bound occupancy |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 16 | 196.7807 | 0.029639 | 0.000688 | 0 |
| 0 | 32 | 273.9635 | 0.042888 | 0.002605 | 0 |
| 0 | 64 | 381.2909 | 0.062234 | 0.008368 | 0 |
| 1 | 16 | 194.6612 | 0.032159 | 0.000550 | 0 |
| 1 | 32 | 270.3421 | 0.043985 | 0.001728 | 0 |
| 1 | 64 | 371.7213 | 0.061677 | 0.007098 | 0 |
| 2 | 16 | 175.2961 | 0.044248 | 0.000463 | 0 |
| 2 | 32 | 270.4100 | 0.056593 | 0.001471 | 0 |
| 2 | 64 | 384.3509 | 0.079497 | 0.009094 | 0 |

Longer traces produced larger measured trace and update scales in this bounded
sample, plus more clipping; they were not assumed or required to improve
behavior. On the longest-delay stage, mean reward over the three outers was
0.4779 / 0.1810 / 0.1914 for `tau_e` 16 / 32 / 64. This short descriptive run
therefore supplies no monotonic “longer is better” result. Outer-1/2 action
locking already documented for this actor family remains visible; M4-07 owns
the declared several-seed acquisition/control comparison.

## Verification

- Focused default test: 4 passed, 1 bounded diagnostic ignored by default.
- Explicit release diagnostic above: 1 passed in 0.61 s reported test time.
- `cargo test --all-targets --locked`: 314 passed, 0 failed, 7 ignored (the six
  pre-existing diagnostics plus M4-05); compile-fail doc checks pass within the
  suite.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets --locked -- -D warnings`: clean.
- `python3 analysis/test_validate_logs.py`: 17 passed.
- `python3 analysis/validate_logs.py analysis/fixtures/valid`: OK.
- Release `validate-config`: both new profiles OK.

Artifact SHA-256:

- `diagnostic/result.json`:
  `81525ac12e85a8ebe54b33aa0d089d96cf856f4c9f7e8e312acb46f37a1d5deb`
- `diagnostic/records.jsonl`:
  `ec1c7757017943017e6e17ab6fc8856551ed7d1231ac64d33c8d91a0b54bde6e`
- `manifests/m4_timing_sensitivity.json`:
  `9457611e2ab8cdcd2521a06037f6d8ef065634d69e2b551b25bc21ff2cab0479`
- `configs/continuous_variable_short.toml`:
  `98811406c98829cbb7142e7eeff10a56ff8917766638f024d1733f59ad2ae37d`
- `configs/continuous_variable_delayed.toml`:
  `a34b3ac9eab33b4014741c1a1659fff4cb1024c368771c8e2b419b5487824e32`

## Limits

This task establishes timing profiles, exact endpoint semantics, paired
schedules, and measured trace/update sensitivity only. It does not select a
`tau_e`, establish continuous acquisition, compare continuity conditions, add
checkpoint replay, or inspect validation/final-test namespaces. M4-06 is next;
M4-07 remains the acquisition gate. Claim track stays `family_only`.
