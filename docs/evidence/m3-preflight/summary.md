# M3 pre-integration hardening evidence

Date: 2026-09-21 UTC  
Task: M3-PREFLIGHT  
Base revision: `9fe089b` with a clean worktree at session start

## Scope

This owner-requested corrective task addressed three bounded hazards before
the first learning runner:

- hidden stable/volatile cue membership now uses `cue_membership`, distinct
  from the actor's unchanged `init` stream;
- `apply_feedback_once` receives named `FeedbackUpdateParams` rather than
  interchangeable scalar update arguments;
- `plastic_bound` is a lifetime-state and snapshot invariant. Plastic
  snapshot schema 3 records it, restore requires agreement with the resolved
  bound, and out-of-bound `P` is rejected.

The feedback equation, actor dynamics, environment timing, top-level
checkpoint schema, configured parameter values, and M3-03 golden values did
not change. No learner runner or acquisition result was added.

## Deterministic migration

Mixed stable/volatile births now assign hidden cue roles from the new
`cue_membership` stream. This intentionally changes only that hidden
assignment relative to revisions through `9fe089b`. Actor initialization,
birth mappings, mapping changes, cue order, timing, reward noise, actor noise,
and tie-breaking retain their previous stream identities. Historical run
artifacts remain interpreted under their recorded code revisions.

## Verification

Commands executed:

```text
cargo test --locked --test seed_streams --test environment_contract --test randomized_env --test replay --test checkpoint
cargo test --locked --test plasticity --test feedback_updates --test golden_updates
cargo fmt --all
cargo test --locked --test seed_streams --test environment_contract --test randomized_env --test replay --test checkpoint --test plasticity --test feedback_updates --test golden_updates
cargo test --locked run::tests::actor_simulation_writes_b3_provenance_and_streams --lib
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --locked --doc
python3 analysis/test_validate_logs.py
python3 analysis/validate_logs.py analysis/fixtures/valid
cargo run --release --locked -- simulate --config configs/env_smoke.toml --baseline random --lifetimes 1 --seed 1 --out-dir /tmp/opencode/m3-preflight-audit
python3 analysis/validate_logs.py /tmp/opencode/m3-preflight-audit/env_smoke-root1-outer1-1789985082
git diff --check
```

Results:

- focused affected suites passed without failure;
- full fast Rust suite: 249 passed, 0 failed, 3 intentionally ignored;
- doc tests: 2 compile-fail checks passed;
- Python audit suite: 17 passed; current and legacy seed-stream schemas covered;
- one fresh release-mode B0 lifetime completed and its current-schema output audited clean;
- formatting and warning-free Clippy passed;
- `git diff --check` passed;
- M3-03 arithmetic remains `0.4 / 0.67 / 0.4 / 0.00067 / 0.10067 / 0.64`;
- checkpoint and replay suites pass unchanged.

Slow M2 Monte Carlo diagnostics were not rerun because score mathematics are
unchanged. The fresh CLI smoke used the development namespace only; no
final-test seed or learning lifetime was consumed.

## Source hashes

```text
f842429bdf2507c4819c36d14f231ea7fd2d323dd1b9a2faecf438d340c30297  src/rng.rs
c00663e4241164451b8ce9c549574400813f471cc84babc33f7e0932fc72cd12  src/environment/mod.rs
d6b0267f79be441f733353f77c353db1f30de13e03a3eec6cad30ab283911b5a  src/agent/no_learning.rs
be849ed9afd6caee451cafa990d491e327c28613e3d3f8465a3df2b24b6f30e9  src/agent/topology.rs
58a65d81c2d0edb5b3f108f254e4a20aae3781caf07a5ef1829643819a812aa4  src/agent/plasticity.rs
ffcae436085c25fa68e4e86609aa13957351c07a3bbc8f97bf273f987d379231  src/checkpoint.rs
72384fa556407ff1577f1364ca35994ab071f9a4bd971aca87208cff2e6fc989  src/run.rs
e0bfde71f5214f46fd024e242d16660f68009cad36e59ea2735452c0ee53f9aa  tests/seed_streams.rs
0086d7eabfcf00588ad50a1a4576547ed52fa46ba07151f16f1af6e9b67e7754  tests/environment_contract.rs
ac8940a1d4287dfd2f6eba001586c32110a63e75951593a05644be9fc0842b04  tests/plasticity.rs
fb6424089101765550ec7b4217e87c73981d78cb01a5f1411964f18178cf72df  tests/feedback_updates.rs
9496483612ebd07f938bf93b73776f861588c2576cf4472fc5ff40e42d95a092  tests/golden_updates.rs
d0e2207ffd192a2cbaf91c933a630471526d07d0c6ebcdabb47a9c489dc6ac68  analysis/validate_logs.py
0a5622207404e819e2b532a9780c5c6766277062d7265ab63a0b77f22f3a996f  analysis/test_validate_logs.py
c58167971c844cf46587edcaf7168a8d98a50796050862bec91211c7108d58bb  analysis/fixtures/valid/seed_streams.json
```

These hashes precede documentation-only packaging edits and identify the
verified behavior and tests.

## Remaining limits

M3-04 still must couple each eligibility update to exactly one actor
transition, add a learning-sensitive feedback-order regression, consume
update reports, and avoid giving the learner broad environment configuration
or master seed authority. M4-01 still owns the full continuous tick-order API.
