# M4-06 evidence — exact continuous-learning pause/resume

Date: 2026-09-21 UTC. Base `6d6f9ee`; M4-06 implementation and evidence
dirty during verification. Reference platform: Linux x86_64
(`rustc 1.98.0`, `cargo 1.98.0`).
Task: [M4-06](../../../to-do.md#m4---remove-artificial-trial-resets) (spec
9, 10.7, 16/M4, 17.7, 20.4–20.5).

## Implementation

- `ContinuousCheckpoint` is a separate schema-4 envelope. M1 `Checkpoint`
  remains schema 2 and episodic `LearningCheckpoint` remains schema 3; all
  three loaders reject the other envelopes.
- The environment snapshot carries phase/countdowns, the pending reward,
  previous-action latch, hidden schedule state, delivered/confirmed event
  ledgers, and the four live environment RNG positions.
- `ContinuousAgentSnapshot` carries live `h`/`a`, derived `r` through `h`,
  last perturbations, motor filters/readout, tick count, both agent RNG
  positions, and the schema-3 `PlasticSnapshot` (`P`, `E`, baseline, dedup,
  mask/trace/tau/bound identity).
- Inherited topology, immutable `W0`, sensory weights, biases, motor pools,
  resolved config, config hash, seed identity, platform, and checksum remain
  in the established envelope. Schema 4 also stores the effective-weight
  cache as an assertion: restore recomputes it only through
  `PlasticState::restore` / `refresh_effective` and rejects disagreement with
  stored `W0 + P`.
- Capture/load use the existing unique temporary file, flush, and atomic
  rename pattern. Every serialized field is required; unknown fields and
  incomplete files fail instead of receiving defaults.

## Replay proof

`tests/continuous_checkpoint.rs` first proves its manual driver matches
`run_continuous_lifetime` exactly over 40 outcomes on M4-05's
`continuous_variable_short` profile, exercising the live timing stream. It
uses the authoritative `observe -> apply feedback -> advance learner ->
finish tick -> commit` order.
Three same-seed splits then resume to exact equality with an uninterrupted
reference:

| Split | Required live state | Result |
| --- | --- | --- |
| During cue activity after 10 outcomes | nonzero `P`, nonzero persistent `E`, nonzero neural activity | exact choices/updates and final neural, motor, plastic, environment state |
| Immediately before due feedback | `Feedback` phase, pending sampled reward, previous-action latch, nonzero `P/E` | pending/latch round-trip and resumed delivery equals the uninterrupted delivery bit-for-bit |
| First completed boundary after feedback | updated `P`/baseline/dedup plus feedback-tick scores in live `E` | exact continuation; replaying the event is rejected by learner and environment dedup with no state mutation |

The final equality includes actions, rewards, latent correctness records,
full update reports, `h/a/r/xi/q`, `P/E/W_effective`, baseline, last feedback,
learner/environment ticks, phase, pending reward, action latch, consumed IDs,
outcome count, and commitment count. Matching the rest of the trajectory also
tests restored RNG positions. Checkpoint methods take shared references, so
capture cannot advance simulation randomness.

Rejection coverage removes required effective-cache, baseline, dedup,
pending-reward, and latch fields and observes parse failures; checksum
tampering is rejected; direct cache disagreement and resolved-learning
configuration mismatch are incompatible; schema-2/3/4 cross-loads fail.

## Verification

```text
cargo test --release --locked --test continuous_checkpoint
  5 passed
cargo test --locked --test episodic_checkpoint
  8 passed, 1 ignored M3-10 bounded capture
cargo fmt --all -- --check
  clean
cargo clippy --all-targets --locked -- -D warnings
  clean
cargo test --all-targets --locked
  323 passed, 0 failed, 7 ignored
cargo test --locked --doc
  2 compile-fail doc tests passed
python3 analysis/test_validate_logs.py
  17 passed
python3 analysis/validate_logs.py analysis/fixtures/valid
  OK (events + provenance)
cargo run --release --locked -- validate-config configs/continuous_stationary.toml
cargo run --release --locked -- validate-config configs/continuous_variable_short.toml
  both OK
git diff --check
  clean
```

The seven default ignores are unchanged from M4-05: two bounded score
diagnostics, the weight-printing probe, the M3-07/M3-08/M3-10 bounded
captures, and the M4-05 timing diagnostic. M4-06 adds no ignored test or
empirical run.

Source SHA-256 at verification:

```text
f34e3f89352beb63e1721432ad9a475788e9a73d80c89baa8d248ea9d4ac7080  src/checkpoint.rs
85e52ddd32a3244a62e8abc3bcec0d5af0649fe0a01de7d54d97508640f3bc30  src/experiments/continuous.rs
acf419bdc101c249f7681454c0408896c3d3e29c6d4b96f7a81850f953934bd4  tests/continuous_checkpoint.rs
98811406c98829cbb7142e7eeff10a56ff8917766638f024d1733f59ad2ae37d  configs/continuous_variable_short.toml
```

## Limits

This establishes exact continuous lifetime continuation on the recorded
reference platform. It does not establish that the continuous learner
acquires the task, select a timing/trace setting, or compare continuity
conditions. M4-07 owns that development-seed acquisition/control comparison.
No validation or final-test outcomes were inspected; claim track remains
`family_only`.
