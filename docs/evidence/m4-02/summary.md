# M4-02 evidence — persistent eligibility and the running baseline

Date: 2026-09-21 UTC. Base `cfc9092` + M4-01 worktree changes.
Task: [M4-02](../to-do.md#m4---remove-artificial-trial-resets) (spec 7.3-7.5,
7.7-7.8, 10.4, 17.2).

## What changed

- `src/experiments/continuous.rs` (new): `ContinuousLearner`, the
  fixed-gate (`gate = 1`) fully persistent plastic learner for the main
  continuous condition. Agent-only construction requires
  `learning.enabled`, the `gaussian_transition_score` rule,
  `trace_policy = "persistent"` (the diagnostic policy is rejected,
  mirroring the episodic learner's rejection in the other direction),
  and `plastic_decay = 0.0`. Tick methods follow the M4-01 split order:
  `apply_feedback` reads the live pre-feedback `E` (including
  post-commitment delay scores) with the fixed gate and old baseline,
  one bounded update, one baseline update, exactly-once dedup;
  `advance` steps the actor on post-feedback effective weights, advances
  `E <- lambda_e * E + S` (no `(1 - lambda_e)` factor), updates motor
  filters, and health-checks. There are deliberately no reset methods:
  state persists across all phase boundaries; birth is the only reset.
  No runner, profiles, or checkpoints (M4-03 through M4-06 own those);
  condition naming belongs to M4-04.
- `src/experiments/mod.rs`: module registration.
- `tests/persistent_traces.rs` (new, 5 tests): see below.

## Fixtures

- Closed-form persistent recurrence over 5 scripted ticks on the
  motor-afferent mask (1e-12), plus a sensitivity guard proving the
  live value differs from a `(1 - lambda_e)`-normalized rule.
- Pure decay: three zero-score advances multiply `E` by exactly
  `lambda^3` (1e-12) — decay compounds once per eligibility advance,
  which the M4-01 driver runs on every tick kind.
- Live-trace end-to-end on the tick-20/delay-3 schedule with
  `birth_only` resets and `tau_e = 64`: commit correct at 20, delay
  21-22, feedback at 23. The applied update equals
  `eta * delta * E_live` per edge (1e-12) with ordered clamps and one
  baseline update; a frozen commit-time `E` and a decay-without-scores
  (`lambda^2 * E_commit`) counterfactual both give different update L1
  norms — post-commitment scores are in the trace, and the
  `commit_snapshot_credit` alternative is excluded by construction.
- Baseline event counts: rewards [1, 0, 1] with `beta = 0.02` reach
  exactly 0.509804; advances never move it; duplicate delivery is
  rejected without change.
- Construction guards (wrong policy / disabled / bad rule rejected),
  birth `P = E = 0` with `W_effective = W0`, and exact second-tick
  score wiring (`E = alpha * r_old * xi / sigma`, 1e-12).

## Verification (this session)

- `cargo test --all-targets --locked`: 298 passed, 0 failed, 6 ignored
  (pre-existing set, unchanged; 293 before + 5 new).
- `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets
  --locked -- -D warnings`: clean.
- `python3 analysis/test_validate_logs.py`: 17 passed.

## Limits

Learner only: no lifetime runner, no continuity profiles, no reset
instrumentation, no checkpoints. Traces still untested across full
multi-choice lifetimes with acquisition — that is M4-04/M4-07 work.
Claim track `family_only`; no acquisition or continuity claim.
