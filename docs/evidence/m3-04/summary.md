# M3-04 episodic clean-learning runner evidence

Date: 2026-09-21 UTC
Task: M3-04
Base revision: `4ad7aaa` with M3-04 worktree changes (see source hashes below)

## Scope

Explicitly episodic diagnostic runner only. No acquisition claim, no
continuous-learning claim, no gates, no search.

- `configs/episodic_stationary.toml`: clean task (2 cues, zero noise/hazard,
  no gap, delay `[1, 1]`, `stationary_clean`, `stable_fraction 1.0`) with
  `reset_policy = "episodic_diagnostic"`,
  `trace_policy = "no_decay_diagnostic"` (`lambda = 1`), fixed gates,
  motor-afferent plasticity, 50 outcomes, warmup 0. Separate from continuous
  `debug_stationary` (`birth_only` + `persistent`).
- `src/agent/plasticity.rs`: new explicitly diagnostic
  `reset_traces_episodic_diagnostic` (zeros `E`, preserves `P`/baseline/
  dedup/cache; rejects any other trace policy so the continuous rule can
  never call it).
- `src/config.rs`: `validate_environment_execution` now allows the named
  diagnostic reset policies (env schedule is identical; resets are
  agent-side); `validate_baseline_execution` and
  `validate_actor_no_learning_execution` explicitly require `birth_only` so
  M0/M1 cannot run a diagnostic config; new `validate_episodic_execution`
  enforces the clean task plus enabled learning, `no_decay_diagnostic`,
  fixed gates, and disabled search.
- `src/experiments/episodic.rs`: agent-only `EpisodicLearner`
  (`Actor` + `Learning` + inherited params + cue count + dedicated
  noise/tie RNGs; never the full config or master seeds) with
  apply-before-advance ordering, one coupled eligibility update per actor
  transition, fixed gate 1, and logged diagnostic resets. `run_episodic_lifetime`
  drives one shared `Lifetime`, applies exactly one terminal update per
  rollout (pre-transition `E` only), consumes every raw/limited/actual
  report into `EpisodicChoice`, and returns reset ticks, mode, policies,
  final `W0`/`P`/`E`/effective/baseline for audit. Hidden annotations are
  recorded evaluator-side only.
- Rollout definition: rollout 0 starts at tick 0 (birth-zero state); each
  later rollout starts the tick after the previous feedback. Each rollout
  holds exactly one commitment and one feedback; `P`/baseline/dedup persist
  across rollouts within a lifetime and reset only at the next birth.
  Post-feedback-tick scores are discarded by the next reset (no
  cross-choice contamination). No reset after the final feedback.
- Checkpoint embedding with nonzero `P`/`E` remains M3-10/M4-06. The M1
  checkpoint schema is untouched at 2. The episodic runner is library-only;
  `simulate` still rejects enabled learning (see negative CLI checks below).

## Verification

Commands executed (repository root, pinned toolchains):

```text
cargo test --locked --test episodic_runner
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
python3 analysis/test_validate_logs.py
cargo run --locked -- validate-config configs/episodic_stationary.toml
cargo run --locked -- simulate --config configs/episodic_stationary.toml --baseline random --lifetimes 1
cargo run --locked -- simulate --config configs/episodic_stationary.toml --baseline actor --lifetimes 1
git diff --check
```

Results:

- `episodic_runner`: 10 passed, 0 failed. Covers profile separation,
  agent-only birth, non-diagnostic construction rejection, diagnostic-only
  reset policy, reset preserves `P`/baseline/dedup while zeroing
  `h/a/q/E`, `P` immobility on non-feedback ticks, pre-transition
  eligibility ordering (`raw == eta*delta*E_old`), one terminal update per
  rollout with logged resets and consumed reports, full 50-outcome finite
  run, and cue-schedule independence from learner noise draws plus exact
  rerun determinism.
- Full fast Rust suite: 260 passed, 0 failed (10 new episodic integration
  + 1 new episodic unit guard test; 249 pre-existing all still pass), 3
  intentional ignores (M2-03 million-sample, M2-05 recurrent Monte Carlo,
  weight-printing probe), 2 compile-fail doc checks pass.
- Python audit suite: 17 passed.
- `validate-config episodic_stationary.toml`: `OK` (profile
  `episodic_stationary`, `stationary_clean`, 2 cues, development).
- `simulate` with `random` rejected: `unsupported execution: baseline
  execution requires reset_policy 'birth_only'` (diagnostic separation).
- `simulate` with `actor` rejected: `unsupported execution: no-learning
  actor execution requires reset_policy 'birth_only'`.
- fmt, Clippy `-D warnings`, and `git diff --check` clean.
- Slow M2 Monte Carlo diagnostics were not rerun (score mathematics
  unchanged). No final-test seeds consumed (development namespace only).

## Source hashes (worktree at verification)

```text
70a5266b5e15d15b8cd73612af82f428c56d949aa0871590d0f3e070dc0266fc  configs/episodic_stationary.toml
d2bfa9e7e40896ab34633ff4a6dee7040eb267456d2db96a7704bb3cc9112830  src/config.rs
cc112e9a335bda91bf0df611bde0d89160238defb7dafaa013c14662cdad8011  src/agent/plasticity.rs
6a311545fd341a43c4a133ea56da1cf63c5f9d73090b8c71c1ce2c85d0a76391  src/experiments/episodic.rs
a8aa16e016d6b817909b2934dec9a15577e13fe380b5018da86e086a3106c486  src/experiments/mod.rs
ed9dcc99ebccbcbaba4249908ea857f08072e6dce8855918f8e6345316459bdb  src/lib.rs
220af456933c868f02d434427bd412e1a1ee692b4cd9ed200986a4f19778561d  tests/episodic_runner.rs
```

Base commit `4ad7aaa`; `git status` showed only the M3-04 files above plus
`to-do.md` at verification time.

## Interpretation and claim limits

- Correct episodic runner arithmetic, ordering, reset isolation, RNG
  separation, and config/log identification are verified. This is
  infrastructure, not learning evidence.
- No acquisition vs no-update/shuffled-reward comparison exists yet (M3-05
  through M3-07 own controls, grid, and acquisition). The 50-outcome smoke
  run completing with finite values does not pass M3-GATE.
- Continuous acquisition remains unverified; M4 owns the persistent
  condition without diagnostic resets. Do not cite this diagnostic as the
  main result.
