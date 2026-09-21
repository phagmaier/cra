# M2-06 and M2-GATE verification plan

2026-09-21 UTC; base `1e6fbf2`, staged M2-05 work preserved.

Package existing tests with `scripts/run_score_diagnostics.sh`. No changes
to actor/score/rollout behavior, diagnostic fixtures, samples, tolerances,
or RNG policy. Do not consolidate numerical code merely for packaging.

One fresh execution of the complete package: deterministic score and
conditional-density tests, finite-rollout and statistics contracts, two
compile-fail API checks, then both explicitly invoked Monte Carlo tests.
Use existing M2-03/M2-05 plans without modification: 1,000,000 one-transition
samples at development/root1/outer203/lifetime0/actor_noise and 2,000,000
six-tick recurrent groups at development/root1/outer205/lifetime0/actor_noise.
Finite combined Monte Carlo budget: 85,000,000 transitions and 25,000,000
normal draws; one serial worker. No search, final-test data or adaptive runs.

Fresh evidence directory only, with source/plan hashes, deterministic
fixture source, commands, exit codes, logs, complete per-diagnostic JSON,
and a completion marker only after required tests and artifacts exist.
A failed/interrupted directory stays failed/incomplete; never reuse it.
Check wrapper syntax, refusal to reuse a directory, and failure propagation
using a fake failing Cargo process (no Monte Carlo in that control).

```bash
bash scripts/run_score_diagnostics.sh docs/evidence/m2-06/suite
```

Review current source identities and result statistics against the original
saved results. Run normal formatting/Clippy/full Rust/doc checks as
applicable. Mark M2-06 only with verified packaging; mark M2-GATE only after
all prerequisite score diagnostics pass. These diagnostics establish neither
unbiasedness nor convergence of the main changing-weight online learner.
Stop at the M2 gate; M3 behavior belongs to a subsequent task.
