# M2-04 fixed-weight finite-rollout diagnostic

Executed 2026-09-21 UTC on linux/x86_64, base `9d31b9d` (`m2-01 done`).
M2-02/M2-03 uncommitted work was preserved; M2-04 adds an isolated library
harness and tests. The actor transition, stochastic score, continuous runner,
configuration, RNG mapping and checkpoint/event schemas are unchanged.

`experiments::finite_rollout::FiniteRollout` owns frozen parameters and a
fixed baseline. Each independent rollout begins at zero state, accumulates
scores with no decay, and accepts exactly one terminal reward after its
declared horizon. `finish(reward, None)` returns the reward-weighted score;
`finish(reward, Some(eta))` also performs one unclipped terminal update on a
copy of W0. Original W0 and baseline stay fixed. A reset is available only
between successfully finished rollouts; it does not carry updated weights
or rewind the caller-owned RNG. Numerical failure prevents further use.

The [golden output](golden.json) contains the complete deterministic fixture:
two neurons, three transitions, alpha=0.5, sigma=0.1, initial W0=0, baseline
0.25, reward=1 and terminal eta=10. Perturbations establish old activities
0.2/-0.4, then add score 0.4 on each existing edge. A final zero perturbation
leaves those sums exactly unchanged. Reward-weighted score is 0.3 and the
terminal copied weight is 3 (within 1e-12 floating-point tolerance). Missing
edges remain zero. The intentionally large update verifies no production
plastic-bound clamp has entered the diagnostic.

Eight integration tests cover score/terminal arithmetic, lifecycle errors,
no-update and zero-delta behavior, fixed baseline/weights, explicit resets,
zero initial state under different weights, no membrane clipping, invalid
inputs, and all numerical failure stages (transition, score, sum, delta,
weighted score and terminal weight). A 16-tick recurrent fixture matches the
existing actor state and RNG exactly, with independently accumulated scores;
its seed is development/root 1/outer 204/lifetime 0/actor_noise. The golden
fixture uses injected perturbations and consumes no seed namespace.
Two compile-fail documentation tests reject live weight/baseline mutation.

Commands actually executed:

```bash
cargo fmt --all
cargo test --locked --test finite_rollout
cargo test --locked --doc
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
CRA_M2_ROLLOUT_EVIDENCE=docs/evidence/m2-04/golden.json cargo test --all-targets --locked
```

Initial focused compile found two invalid array-repeat expressions involving
non-Copy Vec values in tests. Replaced with vector repetition; no production
or scientific change was needed. Focused suite then passed 8/8. Final full
fast suite: 207 passed, 0 failed, 2 default ignores; documentation tests: 2/2
passed; formatting and Clippy clean. The default ignores are the M2-03
million-sample diagnostic (passed in its prior recorded execution, not rerun
here) and the historical weight-printing probe. No standalone CLI smoke or
Python analysis tests were rerun; this isolated diagnostic is not a new
production simulation mode.

SHA-256 identities of verified behavior and saved output:

| File | SHA-256 |
| --- | --- |
| `src/experiments/finite_rollout.rs` | `c00f9e79f353606592743b981c2f0be8bdbf5a8ebea3bf45550be23c4e0385da` |
| `tests/finite_rollout.rs` | `d9cc0a6812217d3112ae1180445a98fe3cf29c9ae550083dc4c4cfcac0f7ec17` |
| `src/agent/actor.rs` | `f28196ea9e8f886c8e3ea0ec684d224e020ff31cb5ad2960fc93805581ca3015` |
| `src/agent/score.rs` | `b5d9f0fcf4d6d6f089bf2c17f3814c039f3e61b67557dbd1a7806f2754c3100f` |
| `docs/evidence/m2-04/golden.json` | `c7bece70df8431769f800c3bd07c6e9a74ceaa4e56ad9e42605fa36d81b29d19` |

No scientific deviations, tolerance changes or unresolved blockers. M2-05
still must test the recurrent estimator against expected-reward finite
differences with prespecified sampling/uncertainty. M2-GATE remains open.

Provenance addendum at handoff: HEAD advanced to `72ad075` (`m2-03`) during
this session, committing the previously uncommitted M2-02/03 work. The base
above identifies the session start. M2-04 remains uncommitted; all tested
behavior and golden-output hashes above were rechecked and still match.
