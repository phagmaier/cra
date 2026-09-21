# M3-05 matched no-update and shuffled-reward controls evidence

Date: 2026-09-21 UTC
Task: M3-05
Base revision: `4ad7aaa` with M3-04 (uncommitted) plus M3-05 worktree changes
(see source hashes below)

## Scope

Matched control machinery only. No acquisition claim, no grid, no
continuous-learning claim, no gates, no search.

- `src/agent/no_learning.rs`: new public
  `reset_state_episodic_diagnostic` (zeros `h`/`a`/`q`/readout, preserves
  `W0`, dedup, tick count, and both RNG positions), mirroring the plastic
  learner's reset minus the eligibility term this actor does not own.
- `src/experiments/episodic.rs`:
  - `sample_matched_inheritance` (shared outer-seed `init` sampling, no
    performance selection) now backs all runners; `run_episodic_lifetime`
    is unchanged in behavior and reports `condition_id = "B4"`,
    `learning_enabled = true`, `reward_protocol = "observed"`.
  - `run_episodic_no_learning` (B3): same profile, schedule, inheritance,
    agent streams, and rollout resets as B4, but the plasticity-free
    `NoLearningActor` built agent-only via `from_parts`. Feedback advances
    dedup only. Its choice type carries no update report by construction.
  - `run_episodic_shuffled`: B4 learner with the recorded
    `ShuffleProtocol::IndependentFairCoin` corruption — each applied reward
    is an independent fair coin from the dedicated `shuffle_reward` stream
    (public seed tuple only; never hidden mapping/correctness/noise/hazard).
    Observed rewards stay separately recorded (`choice.reward`) alongside
    the applied signal (`choice.applied_reward`), which drives both the `P`
    update and the running baseline.
  - `run_episodic_conditions` runs the full B3/B4/shuffled set for one
    development seed with pairing by construction.
  - `EpisodicChoice` gains `applied_reward` (equals `reward` under the
    observed protocol); `EpisodicSummary` gains `condition_id`,
    `learning_enabled`, and `reward_protocol` so conditions differ only in
    declared mechanism fields.
- Checkpoint embedding with nonzero `P`/`E` remains M3-10/M4-06. The M1
  checkpoint schema is untouched at 2. Control runs are library-only;
  `simulate` guards are unchanged.

## Verification

Commands executed (repository root, pinned toolchains):

```text
cargo test --locked --test episodic_controls
cargo test --locked --test episodic_runner
cargo fmt --all -- --check (after cargo fmt --all)
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
python3 analysis/test_validate_logs.py
git diff --check
```

Results:

- `episodic_controls`: 5 passed, 0 failed. Covers shared `W0`/
  initialization/cue-order/reset-tick identity across B3/B4/shuffled;
  condition-ID/mechanism flags as the only declared differences; B3 owning
  no plastic state by construction; first-rollout action parity across
  three lifetimes (both agents start with `P = 0` and paired draws);
  B4 and shuffled final-`P` L1 movement with consumed per-event reports and
  `W0 + P` exactness; shuffled applied-signal equality with the re-derived
  public-coin sequence (protocol fidelity plus information boundary),
  observed/applied separation, and applied-driven deltas; B3 determinism
  and rejection of the non-episodic `debug_stationary` profile.
- `episodic_runner` (M3-04): still 10/10 pass; the new summary/choice
  fields are additive only.
- Full fast Rust suite: 265 passed, 0 failed (5 new control tests),
  3 intentional ignores (M2-03 million-sample, M2-05 recurrent Monte
  Carlo, weight-printing probe), 2 compile-fail doc checks pass.
- Python audit suite: 17 passed.
- fmt, Clippy `-D warnings`, and `git diff --check` clean.
- Slow M2 Monte Carlo diagnostics were not rerun (score mathematics
  unchanged). Development namespace only; no final-test seeds consumed.

## Source hashes (worktree at verification)

```text
276faf9fc23531c08fc94b5e427cdcc613c84dbf26cb2d15b27b1444adb1bc47  src/agent/no_learning.rs
f20711d20b7ae2c204f85c95125a65f69398e3a652c0679f799da62337941696  src/experiments/episodic.rs
91cd81f4eefeae850cfe88846383aa3b1a9cebb5ed9c64f38f336b4dd92042b6  tests/episodic_controls.rs
```

Base commit `4ad7aaa`; `git status` showed the M3-04 files (still
uncommitted) plus the M3-05 files above and tracker/docs edits at
verification time.

## Interpretation and claim limits

- Control machinery is verified: matched schedules/inheritance, mechanism
  isolation, `P`-movement reporting, and a privilege-free recorded shuffle
  protocol with observed/applied separation. This is infrastructure, not
  evidence that the learner acquires the task.
- No development grid, criterion, or several-seed learning-vs-control
  comparison exists yet (M3-06/M3-07 own those). The 4-outcome focused runs
  establish control correctness, not acquisition.
- Continuous acquisition remains unverified; M4 owns the persistent
  condition without diagnostic resets. Do not cite these diagnostics as the
  main result.
