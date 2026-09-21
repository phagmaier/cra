# M2-06 packaging and M2-GATE evidence

2026-09-21 UTC. Session began at `1e6fbf2` with M2-05 staged. During the
session that prior work was committed as `28bcdd5` (`m2 05`). All package
runs record full revision `28bcdd57ccf78c7e8294f7bf0ec4cb4f978fe40b`, dirty
with M2-06 work. Prior staged work and historical evidence were preserved.
Reference: linux/x86_64, pinned Rust/Cargo 1.98.0.

## Package and execution

[Plan](plan.md) fixed before execution. The new
[wrapper](../../../scripts/run_score_diagnostics.sh) orchestrates existing
Rust tests without changing their numerical code, seeds or tolerances.
The output directory must be new and its parent must exist. Relative paths
are resolved from the caller's directory; Rust commands run from repo root.
Optional inherited per-test export variables are cleared and assigned to
fresh bundle paths. The wrapper stops on failure, keeps logs/exit codes,
requires the actual named Monte Carlo tests and exported results, and checks
source hashes before writing `PASSED`. Incomplete output is never success.

Executed once:

```bash
bash scripts/run_score_diagnostics.sh docs/evidence/m2-06/suite
```

The [suite](suite/commands.txt) contains exact command arguments, stdout,
exit statuses, source/plan hashes, deterministic fixture sources, copied
original plans, golden rollout JSON and complete Monte Carlo JSON exports.
It ran 08:44:19–08:44:35 UTC; all four stages exited zero.

| Coverage | Fresh result |
| --- | --- |
| M2-01 pure score, M2-02 fixed-observation derivative, M2-03/05 fast statistics, M2-04 rollout contracts | 27 passed, 2 Monte Carlo tests ignored in this fast stage |
| Frozen parameter/baseline compile-fail API tests | 2 passed |
| M2-03 explicit one-neuron Monte Carlo | 1 passed, 1,000,000 samples, 0.08s |
| M2-05 explicit recurrent Monte Carlo | 1 passed, 2,000,000 paired trajectory groups, 14.69s |

## Measured agreement

M2-02 again passes 288 deterministic comparisons across six leak/noise
settings and four epsilons, including Gaussian-density and moving-sample
negative controls. Maximum absolute difference remains 3.790982994190e-8.
Fixtures and analytical expressions are saved in `suite/derivative-fixture.rs`;
per-setting measured errors are in [fast.log](suite/fast.log).

M2-03: mean **0.13871680603781847**, analytical **0.1388622064964956**,
SE **0.00010644926671077609**. Absolute error 0.0001454004586771418 is below
five-SE-plus-1e-12 tolerance 0.0005322463345538805. Opposite target negates
the estimate. [direction.json](suite/direction.json) records all settings,
expectations, counts, stream identity and uncertainty.

M2-05: score mean **0.41802542842538726**, SE **0.0006915850809732828**;
finite differences **0.41825625, 0.42040000, 0.41765000** for epsilons
0.04, 0.02, 0.01. All paired differences pass the prespecified five-SE-plus-
0.0005 tolerance, positive lower-bound check and <=0.02 half-width limit.
[recurrent.json](suite/recurrent.json) includes all uncertainty estimates
and zero failures. The three comparisons share samples, not independent
replications; the finite-epsilon allowance is not a proven bias bound.

The fresh settings, seeds, sampled statistics and counts match the original
M2-03 and M2-05 JSON exactly. The fresh M2-04 golden also matches exactly.
This is replay of the same pinned diagnostic samples, not new independent
empirical evidence. Combined Monte Carlo work: 85,000,000 transitions,
25,000,000 independent normal draws, one serial worker. Development/root1/
outer203 or outer205/lifetime0/actor_noise; no final-test namespace.

## Other verification and limitations

- `bash -n scripts/run_score_diagnostics.sh`: pass.
- Four wrapper controls pass: missing argument exits 2; a fake Cargo failure
  propagates exit 23 and retains failure status with no success marker;
  an existing output directory is rejected without modifying its contents;
  a fake zero-test success is rejected. Tests invoke the script outside the
  repo root. [wrapper-checks.json](wrapper-checks.json) records these checks.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets --locked -- -D warnings`: clean.
- `cargo test --all-targets --locked`: **212 passed, 0 failed, 3 ignored**.
  M2-03/M2-05 were explicitly passed above; the third ignored test is the
  existing weight-printing probe, unrun. Exact commands/timings/logs are in
  [quality.json](quality.json).
- Python stdlib audit verified source hashes, original-result parity,
  golden parity and actual test totals; [audit.json](audit.json) includes
  artifact hashes. No simulator CLI or Python analysis changes; those
  unrelated suites were not repeated. No failures or changed thresholds in
  real diagnostic runs; the fake-command failures are intentional controls.

**M2-06 and M2-GATE pass.** The required deterministic and Monte Carlo
checks all actually ran and satisfy their original criteria. No production
code, checkpoint/config schemas, dependencies or scientific contracts changed.
The tests validate the Gaussian score and restricted fixed-weight finite-
rollout estimator. They do **not** prove unbiasedness or convergence of the
main decaying, clipped, gated, changing-weight online learner, nor demonstrate
acquisition. Next task: **M3-01**; no M3 implementation started here.
