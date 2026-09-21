# M2-05 recurrent diagnostic plan — fixed before execution

Date: 2026-09-21 UTC. Base revision: `1e6fbf2` (clean before task claim).
Scope: spec 7.6 and 17.6; isolated numerical diagnostic, no online learning.

- Two neurons, edges 1->0 and 0->1, no self edges. Row-receiver weights
  `W=[[0,-0.4],[0.3,0]]`; perturb only `W[1,0]`.
- Six transitions per rollout, zero initial h/a/r independent of weights,
  alpha_h=0.5 (`tau_h=1/ln(2)`), tau_a=100, adaptation strength=0,
  sigma=0.4. Constant scalar input 1, input weights `[0.7,-0.2]`, zero bias.
  Terminal binary reward is `1[h[1]>0]`. No motor filtering or environment.
- Reuse `FiniteRollout`: fixed weights/noise, exact no-decay score sums,
  fixed baseline zero, `finish(reward,None)` with no weight update. Explicit
  zero-state resets only between independent diagnostic rollouts.
- Exactly 2,000,000 independent six-tick noise trajectories. Root 1,
  namespace development, outer 205, lifetime 0, stream actor_noise, existing
  SHA-256 derivation / pinned ChaCha8 / per-tick Box–Muller NormalStream.
- Base plus six perturbed settings: epsilon 0.04, 0.02, 0.01, each +/-.
  Base draws two independent normals each tick; replay these same actual
  Gaussian draws into all six perturbed settings (common random numbers).
  State evolves separately at each weight. Unlike the conditional-density
  check, trajectories and terminal rewards MUST change with the weight.
- Sample score estimator `S=R_base*sum_t score_t[1,0]`; central finite
  differences `D_eps=(R_plus-R_minus)/(2*eps)`. Welford means, unbiased sample
  variances and SEs for S, both rewards, D and paired discrepancy D-S.
  Discrepancy uncertainty comes from paired samples, not independent SEs.
- Require every `abs(mean(D-S)) <= 5*SE(D-S)+0.0005`. The fixed 0.0005
  allowance is for finite-epsilon bias, not an analytical error guarantee.
  Report each epsilon separately to expose numerical sensitivity.
- Precision requirements: each five-SE half-width for S, D and D-S <=0.02;
  S and every D must have positive five-SE lower bounds. This is an absolute
  precision/sign safeguard against accepting a broad interval as evidence.
  A failed precision condition means unresolved uncertainty, not agreement.
- No early success stopping, reseeding, adaptive sample count, or changed
  tolerances after inspection. Numerical failures stop execution, retain
  sample/weight/tick/error information and completed resource counts, and
  fail the diagnostic; no failed samples are silently dropped.
- One serial worker; finite budget 14,000,000 rollouts / 84,000,000 neural
  transitions / 24,000,000 independent normal draws. No search or final-test
  namespace. Monte Carlo is ignored in the default test suite.
- Save complete fixture, plan/source/config hashes, code revision/dirty
  state, platform/toolchain, seed, counts, statistics and all pass flags.
  Evidence uses exclusive creation; do not overwrite existing output.

```bash
CRA_M2_RECURRENT_EVIDENCE=docs/evidence/m2-05/result.json cargo test --release --locked --test score_recurrent two_neuron_recurrent_finite_difference -- --ignored --exact --nocapture
```

Passing validates only this fixed-weight finite-horizon diagnostic. It does
not establish continuous learning, convergence or unbiased online updates.
