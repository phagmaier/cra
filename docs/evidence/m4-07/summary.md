# M4-07 evidence — declared continuous acquisition comparison

Date: 2026-09-21 UTC. Base revision `0aa6e73`; worktree dirty by construction.
Task: [M4-07](../../../to-do.md#m4---remove-artificial-trial-resets) (spec
7.3–7.9, 9–10, 14.3, 16/M4, 17.8).

## Pre-results declaration and budget

`manifests/m4_continuous_acquisition.json` was written and fast-validated
before any M4-07 acquisition comparison was executed. It fixes development
root 1, outers 1–3, lifetime 0, 2,000 outcomes, the M3-08 full-recurrent
winner family (grid index 11: eta 0.001, input scale 0.2, recurrent gain 0.8,
noise sigma 0.05), `tau_e = 32`, and five conditions:

- episodic B4 and its matched episodic B3;
- persistent activity with post-outcome trace clearing (`event_reset_b4`);
- fully persistent activity/traces (`continuous_b4`) and its matched
  birth-only-reset B3.

All use the same clean stationary environment, warmup, timing, actor
inheritance, agent streams, and exogenous schedule within each outer seed.
Acquisition is measured by cue exposure rather than global choice index: the
first and final 100 exposures of each cue are summarized, then cues receive
equal weight. Each cue must have at least 200 exposures.

The predeclared fully persistent criterion is late macro latent accuracy at
least 0.70 and a margin of at least 0.15 over continuous B3, with clipping at
most 0.10, bound occupancy at most 0.05, nonzero final `P`, zero failed
lifetimes, and at least 2/3 outer seeds passing. A representative release
benchmark completed 2,000 outcomes / 34,028 ticks in 0.098 s. The full serial
budget was therefore fixed at 15 lifetimes, 30,000 outcomes, and at most
510,420 ticks.

## Execution

Benchmark:

```bash
cargo test --release --locked --test m4_continuous_acquisition \
  m4_continuous_acquisition_benchmark -- --ignored --exact --nocapture
```

Comparison:

```bash
CRA_M4_ACQUISITION_DIR=docs/evidence/m4-07/run \
  cargo test --release --locked --test m4_continuous_acquisition \
  m4_continuous_acquisition_comparison -- --ignored --exact --nocapture
```

The first comparison invocation failed before running any lifetime because
the parent `docs/evidence/m4-07/` directory did not yet exist. After creating
that parent, the unchanged command, manifest, and seeds ran once. The final
run completed all 15/15 lifetimes, 30,000/30,000 outcomes, and exactly the
510,420-tick maximum. There were zero simulation failures. Within each outer,
all five conditions had identical `W0` and event/cue/commit/feedback/noise
schedules; reset audits were 2,000 rollout starts for episodic, birth plus
1,999 trace clears for event-reset, and `[0]` only for fully persistent.

Raw records and the machine-readable verdict are
`run/{records.jsonl,verdict.json}`.

## Result: criterion not met

Late values below are equal-weight macro latent accuracy over the final 100
exposures of each cue. The fully persistent margin is against matched
continuous B3.

| Outer | Episodic B4 | Episodic B3 | Event-reset B4 | Continuous B4 | Continuous B3 | Continuous margin | Seed pass |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | no |
| 2 | 0.970 | 0.030 | 0.000 | 0.000 | 0.000 | 0.000 | no |
| 3 | 0.935 | 0.830 | 0.955 | 0.940 | 0.875 | 0.065 | no |

The declared verdict is **0/3 seeds passing; M4-07 acceptance is not met**.
Outer 2 preserves the earlier episodic acquisition result but loses it when
activity persists, both with and without trace clearing. Outer 3 retains
above-chance fully persistent performance, but its inherited continuous B3 is
already strong and the learning margin is only 0.065, below the frozen 0.15
criterion. Outer 1 remains the documented locked initialization. Thus this is
not evidence of several-seed continuous acquisition beyond the matched
nonplastic actor.

## Update, bound, and saturation diagnostics

Values are per-event matrix L1 means. Full mean/p95/max fields are in
`records.jsonl`.

| Outer | Plastic condition | raw L1 mean | actual L1 mean | clipped fraction | bound occupancy | final P L1 | actor saturation | max abs motor filter |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | episodic | 0.001737 | 0.001737 | 0 | 0 | 0.390 | 0 | 0.542 |
| 1 | event-reset | 0.001939 | 0.001939 | 0 | 0 | 0.433 | 0 | 0.604 |
| 1 | continuous | 0.002450 | 0.002447 | 0.000019 | 0 | 0.989 | 0 | 0.602 |
| 2 | episodic | 0.041035 | 0.040943 | 0.000660 | 0 | 5.992 | 0 | 0.520 |
| 2 | event-reset | 0.002547 | 0.002547 | 0 | 0 | 0.567 | 0 | 0.553 |
| 2 | continuous | 0.003164 | 0.003164 | 0 | 0 | 1.245 | 0 | 0.574 |
| 3 | episodic | 0.024942 | 0.024887 | 0.000579 | 0 | 3.692 | 0 | 0.389 |
| 3 | event-reset | 0.022870 | 0.022770 | 0.000912 | 0 | 3.161 | 0 | 0.442 |
| 3 | continuous | 0.036244 | 0.035853 | 0.002702 | 0 | 3.496 | 0 | 0.437 |

The negative result is not explained by numerical failure, plastic-bound
occupancy, or actor/motor saturation: all states were finite, no actor
neuron-ticks exceeded the `|r| > 0.9` saturation threshold, motor-filter
maxima stayed below 0.61, clipping stayed below 0.003, and bound occupancy
was zero. Real updates and `P` movement occurred on every continuous seed,
including the two behaviorally locked cases. M4-08 should therefore inspect
cross-choice interference, persistent-state representation, baseline drift,
trace timescale, and update direction rather than weakening the frozen
criterion or adding hidden resets.

## Verification and artifacts

- Focused default test: 2 passed, 2 explicit empirical tests ignored.
- Release benchmark: 1 passed, 34,028 ticks, 0.098 s measured runner time.
- Release comparison: 1 passed as an execution instrument; empirical verdict
  false, 15/15 lifetimes complete, zero failures.
- Focused affected-runner regression set: 55 passed, 3 explicit empirical
  diagnostics ignored.
- Full Rust suite: 325 passed, 0 failed, 9 ignored; two compile-fail doc tests
  passed. `cargo fmt --all -- --check` and Clippy with `-D warnings` are clean.
- Python log-audit tests: 17 passed; committed fixture audit OK. Release
  validation of `continuous_stationary` passed; `git diff --check` is clean.
- The only post-result source cleanup replaced a complex borrowed tuple type
  with a Rust type alias to satisfy Clippy; it changed no runtime behavior.

SHA-256:

```text
18354ebb175aafc266cd0c1a01fa5039e52da178f738ea91dfe6b1520d04a1e8  manifests/m4_continuous_acquisition.json
b6569730802356c6c94338b1802ed4943f288eaefeedaf0f7a9cf3303eb5aa84  docs/evidence/m4-07/run/records.jsonl
96488cc340eb1242d73c6bff2aafb5acce042366edba5172d2df6cbcac4b0b80  docs/evidence/m4-07/run/verdict.json
```

## Claim limit and next step

This development result verifies that the declared comparison ran correctly;
it does not satisfy M4-07's scientific acceptance criterion. The M4-07
checkbox and M4-GATE remain open. No validation or final-test seeds were
inspected. Per the predeclared `on_no_pass` rule, the next eligible work is
M4-08's failure audit; seeds, windows, and criteria must not be revised to
retroactively turn this run into a pass.
