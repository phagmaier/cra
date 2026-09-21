# M2-02 fixed-sample conditional derivative evidence

Executed 2026-09-21 UTC from clean base `9d31b9d` (`m2-01 done`),
with the new diagnostic and documentation uncommitted. Production code and
specification unchanged. No seed namespace consumed: all perturbations and
states are explicit deterministic fixtures, not Monte Carlo samples.

Command: `cargo test --locked --test score_log_probability -- --nocapture`.
Result: 3 passed, 0 failed. The primary fixture checks all 12 existing edges
of a four-neuron actor, with nonzero input, bias and adaptation drives,
zero/signed/nearly saturated old activities, and perturbations
`[0.3, -1.1, 0.0, 2.0]`. Parameters and old state remain unchanged.

For each setting, sample `h_new` once using the production actor. Then vary
one weight at a time and obtain each conditional mean from the same saved
old state using a zero-perturbation transition. The scalar Gaussian log
density always receives the original `h_new[j]`; it never resamples.
Other receivers' means are checked unchanged. The score uses the saved
old sender activity and original receiver perturbation.

Epsilon values declared before execution: `1e-5, 3e-6, 1e-6, 3e-7`.
Acceptance declared before execution:
`abs(numerical - analytical) <= 2e-8 + 2e-7 * abs(analytical)`.
All 288 comparisons passed on first execution.

| tau_h | sigma | Comparisons | Maximum absolute difference |
| --- | --- | --- | --- |
| 0.5 | 0.05 | 48 | 5.143718340150e-9 |
| 0.5 | 0.4 | 48 | 1.063786614353e-9 |
| 5 | 0.05 | 48 | 3.790982994190e-8 |
| 5 | 0.4 | 48 | 4.692469524059e-9 |
| 100 | 0.05 | 48 | 1.734158852629e-8 |
| 100 | 0.4 | 48 | 2.375865382903e-9 |

Two additional tests verify the Gaussian density against known values and
demonstrate the invalid moving-sample control: reusing the same xi while
moving the observation with each perturbed mean yields a derivative near
zero, while the fixed-observation derivative agrees with score `0.42`.

Other executed commands: `cargo fmt --all`, `cargo fmt --all -- --check`,
`cargo clippy --all-targets --locked -- -D warnings`, and
`cargo test --all-targets --locked`. Formatting and Clippy clean; full suite
196 passed, 0 failed, 1 existing ignored weight-printing probe.
Python analysis and standalone CLI smoke runs were not repeated for this
test-only change. No failures, tolerance changes, or scientific deviations.

SHA-256 identities:

| File | SHA-256 |
| --- | --- |
| `tests/score_log_probability.rs` | `8942909a214b916a4f4400dfb9d1812559c3515e3836f8c80bc6652ec908619a` |
| `src/agent/score.rs` | `b5d9f0fcf4d6d6f089bf2c17f3814c039f3e61b67557dbd1a7806f2754c3100f` |
| `src/agent/actor.rs` | `f28196ea9e8f886c8e3ea0ec684d224e020ff31cb5ad2960fc93805581ca3015` |

This checks the conditional Gaussian derivative only. The analytical
learning-direction and recurrent Monte Carlo diagnostics are still queued.
M2-GATE remains open; no online-learning or convergence claim follows.
