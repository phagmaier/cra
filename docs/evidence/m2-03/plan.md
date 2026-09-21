# M2-03 diagnostic plan — fixed before execution

Date: 2026-09-21 UTC. Base revision: `9d31b9d` (`m2-01 done`).
M2-02 test/evidence/documentation changes are already present and uncommitted;
preserve them. Scope: spec 17.5 only, no online learning or recurrent rollout.

- Exactly 1,000,000 independent standard-normal draws, serial and bounded.
- Seed tuple: root 1, namespace `development`, outer 203, lifetime 0,
  stream `actor_noise`; derive via the existing `cra-v1` SHA-256 policy.
- Use the pinned ChaCha8 RNG and existing Box–Muller `NormalStream`.
  One stream retains its paired spare across draws; these are independent
  one-transition samples, not a continuous actor lifetime or checkpoint.
- Fix alpha=0.2, input=0.7, weight=0.3, sigma=0.4, baseline=0.5.
  For each xi, compute mu=alpha*weight*input, h=mu+sigma*xi,
  positive-target reward=1 if h>0 else 0, and opposite reward=1-reward.
  The two targets use the same sample and are a paired sign check, not two
  independent empirical replications.
- Reuse `agent::score::conditional_score`. Accumulate both signed terms
  `(reward-baseline)*score` with Welford sample-variance estimates.
  Standard error is `sqrt(sample_variance / sample_count)`.
- Analytical positive-target derivative:
  `(alpha*input/sigma) * exp(-0.5*(mu/sigma)^2) / sqrt(2*pi)`;
  opposite-target derivative is its negative. No rounded expected value
  is substituted in acceptance.
- Require each absolute error to be at most `5*standard_error + 1e-12`.
  Also require the positive mean >0, opposite mean <0, and paired estimates
  to negate each other within `1e-12` with equal standard errors.
- No early stopping, adaptive sample count, reseeding or tolerance changes
  after inspecting results. Investigate any failure and retain its evidence.
- Keep the Monte Carlo test ignored in the ordinary suite. Invoke explicitly
  with the command documented below; deterministic statistic and arithmetic
  tests remain in the fast suite.
- Save all parameters, seed identity and derived seed, RNG policy, counts,
  mean/variance/SE, analytical targets, errors, tolerances, pass/failure,
  platform and source identities. Evidence creation must reject overwrite.

Planned explicit execution (from repository root):

```bash
CRA_M2_DIRECTION_EVIDENCE=docs/evidence/m2-03/result.json cargo test --release --locked --test score_learning_direction one_neuron_learning_direction -- --ignored --exact --nocapture
```

This is a numerical direction check under restricted assumptions. Passing
does not demonstrate learning, convergence, or unbiased online updates.
