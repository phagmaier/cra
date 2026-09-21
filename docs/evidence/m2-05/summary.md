# M2-05 evidence — short recurrent finite-difference diagnostic

Date: 2026-09-21 UTC. Base HEAD: `1e6fbf2d8fcec86191876a389f3ebdae6adee06f`.
Initial tree clean; task implementation/evidence uncommitted at execution.
Reference platform: linux/x86_64, rustc 1.98.0 (88d9e12ae 2026-08-18).

## Design and first execution

The [plan](plan.md) was written before sampling. Two neurons with both
cross-edges, six transitions, fixed weights/noise, weight-independent zero
initial state, exact score sum, baseline zero and binary terminal reward.
The base and +/- perturbations share Gaussian draws while each trajectory
evolves at its own fixed weights. No online or terminal weight updates.

Exactly 2,000,000 independent trajectory groups using development/root 1/
outer 205/lifetime 0/actor_noise. Seven weight settings produce 14,000,000
rollouts and 84,000,000 transitions from 24,000,000 independent normal draws.
One serial worker; no failed samples. This is the first Monte Carlo execution;
no reseeding, added samples or changed thresholds. Test time 15.35 seconds
(release compilation 7.71 seconds); measured diagnostic time 15.277 seconds.

Base reward mean: 0.45876850. Score estimate `R*sum(score[1,0])`:
**0.41802543**, SE **0.00069159**, five-SE half-width **0.00345793**.

| Epsilon | Finite difference | FD SE | FD minus score | Paired difference SE | Allowed absolute difference | Result |
| --- | --- | --- | --- | --- | --- | --- |
| 0.04 | 0.41825625 | 0.00160185 | +0.00023082 | 0.00179419 | 0.00947096 | Pass |
| 0.02 | 0.42040000 | 0.00229001 | +0.00237457 | 0.00242919 | 0.01264594 | Pass |
| 0.01 | 0.41765000 | 0.00324311 | -0.00037543 | 0.00334308 | 0.01721539 | Pass |

Every comparison passes `abs(mean(D-S)) <= 5*SE(D-S)+0.0005` and the
separate precision requirement (all five-SE half-widths <=0.02). All score
and FD five-SE lower bounds are positive. Largest paired half-width is
0.01671539, about 4% of the score estimate. The paired sample is the
uncertainty unit; the three epsilon comparisons are correlated, not three
independent replications. Five-SE bands are Monte Carlo estimates, not
certified confidence bounds; the finite-epsilon allowance is prespecified,
not a proven bias bound. All three epsilon results are retained.

Full machine-readable parameters, statistics, pass flags, seed hash,
resource counts, source/lock/plan SHA-256 identities and provenance are in
[result.json](result.json). Captured diagnostic stdout is [diagnostic.log](diagnostic.log).

## Commands actually executed

```bash
cargo fmt --all
cargo test --locked --test score_recurrent
cargo clippy --all-targets --locked -- -D warnings
CRA_M2_RECURRENT_EVIDENCE=docs/evidence/m2-05/result.json cargo test --release --locked --test score_recurrent two_neuron_recurrent_finite_difference -- --ignored --exact --nocapture
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
```

- Five new fast checks pass: paired variance/SE arithmetic, rejection of
  broad intervals or precise disagreement, forced recurrent trajectory and
  frozen-weight/reset assumptions, paired sampler replay/counts, and failed
  sample accounting. The Monte Carlo test is ignored by default and passed
  separately above.
- Full fast suite: **212 passed, 0 failed, 3 ignored**. Ignored tests are this
  separately executed M2-05 diagnostic, M2-03's previously executed direction
  diagnostic, and the existing weight-printing probe. The latter two were
  not rerun this session. Two compile-fail API doc tests passed separately.
- Formatting and Clippy pass. Exact final quality commands, exit codes and
  timings: [checks.json](checks.json), with four referenced captured logs.
- During test development, the failure-accounting fixture initially used a
  large but finite drive (1e100), which correctly did not trigger the
  diagnostic's nonfinite check. Corrected the fixture to sum two finite
  `f64::MAX` values and force actual overflow. Final failure-accounting check
  passes; no production behavior or scientific acceptance criteria changed.
- Final documentation checks: `git diff --check` clean; Python stdlib
  verification found all ten saved source/plan/lock hashes unchanged, all
  local Markdown links present, and diagnostic/quality records passing.
- Python/CLI simulation checks were not repeated for this Rust test-only
  diagnostic. No production code, dependencies, RNG policy, config schemas,
  checkpoints, main runner or spec changed.

## Interpretation and continuation

M2-05 is verified for this fixed-weight, no-decay finite-horizon fixture.
It does not show acquisition, convergence, or unbiasedness of the online
clipped/gated learner. M2-GATE remains open; next eligible task is M2-06,
which owns packaging/review of all required diagnostic evidence before the
gate. No search, final-test inspection, or scientific-contract deviation.
