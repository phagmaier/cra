# M3-09 failure-isolation path and negative checks evidence

Date: 2026-09-21 UTC
Task: M3-09
Base revision: `0d20d45` plus M3-08/M3-09 worktree changes (hashes below).
All checks run on linux/x86_64.

## Scope

The learner acquired successfully (M3-07/M3-08), so this task builds
and validates the troubleshooting instruments rather than rescuing a
failure — spec M3 "If it fails" recipe (single noisy motor unit,
constant input, known preferred action, inspect update sign) and the
Section 21.1 reduction ladder, mechanized as regression tests:

- `src/experiments/reduction.rs` (new, diagnostic-only):
  `run_single_motor_diagnostic` (one plastic edge, constant features,
  synthetic reward for a known preferred action, per-outcome
  pre-feedback-trace identity proof) and
  `run_episodic_permuted_lifetime` (the verified episodic driver with
  receiver-permuted perturbations).
- `src/experiments/episodic.rs`: `advance` refactored through a shared
  private core plus the explicitly diagnostic
  `advance_with_receiver_permutation` hook (bijection-validated; the
  identity reproduces `advance` exactly). Ordinary runners
  (`run_episodic_lifetime`, `run_episodic_no_learning`,
  `run_episodic_shuffled`) still call `advance` only — verified by
  source search: the hook is referenced solely from `reduction.rs`
  and `tests/m3_reduction.rs`.
- `tests/m3_reduction.rs` (new, 5 tests): single-edge sign/order,
  single-motor closed loop, outer-1 representation separation,
  permutation-argument rejection, and permutation sensitivity with
  identity-hook fidelity.

## Reduction path (documented procedure for failed acquisition)

Apply in order; stop at the first failing step (spec 21.1):

```text
1. Sign: hand-set E on one plastic edge, apply +/-delta, require P to
   move as eta * delta * E with separated raw/limited/actual reports.
2. Order: require every applied update to equal eta * delta * E_old
   read before the feedback transition (max identity error ~ 0).
3. Minimal closed loop: single noisy motor unit + constant input +
   known preferred action must drift offsets toward the preferred
   action with step 2 holding on every outcome and no clipping.
4. Representation: matched B3/B4 where P moves but behavior and both
   controls stay locked is a representation/birth-lock failure, not
   plasticity. Never "fix" it with evolution or hidden resets.
5. Sensitivity: receiver-permuted perturbations must change the
   updates. Exact equivalence with the correct assignment is a score-
   wiring bug to investigate, never a tuned-away non-result.
```

## Verification

Commands executed (repository root, pinned toolchains):

```text
cargo test --locked --test m3_reduction
cargo test --locked --test m3_reduction -- --nocapture (measured prints)
cargo fmt --all -- --check (after cargo fmt --all)
cargo clippy --all-targets --locked -- -D warnings
git diff --check
```

Results (fixed seeds throughout; diagnostic constants documented in
the test source — seeds were never rerolled):

- `single_edge_update_sign_and_order`: one plastic edge (0,1);
  hand-set E[0][1] = 0.25 exactly; reward 1 gives raw +0.00125 and
  P +0.00125 with baseline 0.51; reward 0 on a fresh state gives
  raw/P −0.00125 (sign flips, magnitude equal); all nonplastic
  reports exactly zero; duplicate delivery rejected with unchanged
  state.
- `single_noisy_motor_closed_loop`: 30 synthetic outcomes, every
  outcome's max raw-identity error < 1e-12 (order proof end to end);
  only edge (0,1) ever moves; preferred-action rate 0.400 (first 10)
  → 0.700 (last 10); final P[0][1] = +0.076576 (correct drift
  direction); 0/30 clipped; baseline 0.5380.
- `birth_lock_is_representation_failure_not_plasticity`: outer-1,
  60-outcome winner config — B4 moves real offsets (P L1 > 0,
  dedup through event 59) while B4 and B3 late accuracy are both
  exactly 0.0 with zero margin. Ladder step 4 demonstrated on the
  real family bound.
- `permutation_hook_rejects_bad_permutations`: wrong-length,
  out-of-range, and duplicated permutations rejected; valid reversal
  advances normally.
- `permuted_perturbations_change_updates`: 600-outcome outer-2
  winner run — reversed-xi completes with finite values on the
  identical schedule (resets match) but a different final `P`
  (no unexpected equivalence); identity permutation reproduces the
  verified runner bitwise (final P/E, baseline, action sequence).
  Reported without a demanded magnitude: correct late-20 accuracy
  0.050 with P L1 1.567 vs reversed 0.000 with P L1 0.685 at this
  horizon (outer-2 needs ~2000 outcomes to acquire per M3-08).

## Findings

- The troubleshooting path was **not needed** to rescue acquisition:
  M3-07/M3-08 pass on responsive actors. It is validated here as
  working tooling for M4 and beyond.
- Outer-1 is confirmed a representation failure by ladder step 4:
  the update sign (step 1), ordering (step 2), and closed-loop drift
  (step 3) all verify, so the locked behavior cannot be blamed on
  plasticity. This matches the M3-07/M3-08 family-bound conclusion
  with independent mechanics, not a repeated assertion.
- Permutation sensitivity holds at the mechanism level (P differs)
  with no behavioral magnitude demanded or claimed. The identity
  control proves the hook adds no transition-path artifact.

## Source hashes (worktree at verification)

```text
c2f5b4bea8b00f837f990ce0e42d68183e8ae9ece32b50ec23b56f436f278195  src/experiments/reduction.rs
8c6cd9ca1e5f31103290e87d41c3e59754955aa9616917ba668dc0a2bd735a98  src/experiments/episodic.rs
4bd56bf621f308a867f630110756e1c7412db4c43baecbc17f0934bfb4a26526  src/experiments/mod.rs
e23c71d14b7f614f6ab5d4d2cd1103b62f4de598d4c800355254d9d374128f78  tests/m3_reduction.rs
```

Base commit `0d20d45` with uncommitted M3-08/M3-09 files plus
tracker/docs edits at verification time.

## Interpretation and claim limits

- Diagnostic tooling only. Synthetic constant-input rewards are a
  mechanism check, never a task result and never a comparison
  against environment-driven runners.
- No scientific rule changed: the hook deliberately mis-wires the
  score for diagnosis and is unreachable from ordinary runners; the
  shared transition core is proven unchanged by the bitwise
  identity control plus the still-passing M3-04/M3-05 suites.
- Checkpoint embedding and split replay with nonzero `P`/`E`
  remain M3-10 work; the top-level checkpoint schema stays 2.
