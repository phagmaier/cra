# M2-03 one-neuron direction diagnostic

Executed 2026-09-21 UTC under the [predeclared plan](plan.md).
The immutable [result](result.json) records configuration, complete seed
identity, derived seed, platform, toolchain, source hashes and statistics.
Base `9d31b9d` with M2-02 and M2-03 changes uncommitted; production unchanged.

Exactly 1,000,000 independent one-transition samples, development/root 1/
outer 203/lifetime 0/actor_noise. The two reward targets share these samples.
No adaptive stopping, reseeding, tolerance changes or failed attempts.

| Target | Analytical derivative | Sample mean | Standard error | Absolute error |
| --- | --- | --- | --- | --- |
| h > 0 | 0.1388622064964956 | 0.13871680603781847 | 0.00010644926671077609 | 0.0001454004586771418 |
| Opposite | -0.1388622064964956 | -0.13871680603781847 | 0.00010644926671077609 | 0.0001454004586771418 |

Both pass `abs(error) <= 5*SE + 1e-12` (tolerance
0.0005322463345538805). Both sample variances are 0.011331446383261943.
Positive/opposite reward counts are 541890/458110. Estimates negate exactly,
with identical standard errors; this is one paired sign check, not two
independent experiments. No weights or baseline were updated.

Commands actually executed:

```bash
cargo fmt --all
cargo test --locked --test score_learning_direction
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
CRA_M2_DIRECTION_EVIDENCE=docs/evidence/m2-03/result.json cargo test --release --locked --test score_learning_direction one_neuron_learning_direction -- --ignored --exact --nocapture
```

Focused fast suite: 3 passed, Monte Carlo diagnostic ignored by default.
Full fast suite: 199 passed, 0 failed, 2 ignored (this diagnostic and the
existing weight-printing probe). Explicit release diagnostic: 1 passed,
0 failed, 0 ignored, 3 filtered out; reported test time 0.07s, compilation
7.24s. Formatting/Clippy clean. The weight-printing probe was not invoked.
No Python analysis or standalone simulator smoke rerun: production unchanged.

The result path rejects overwrite; reproduce with a fresh evidence path
whose parent exists, or omit `CRA_M2_DIRECTION_EVIDENCE` to print only.
This verifies the one-transition stochastic learning direction under the
specified assumptions. It is not a demonstration of online learning,
convergence or a completed M2 gate. Next task: M2-04.
