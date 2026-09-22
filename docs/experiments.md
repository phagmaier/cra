# Experiments

Append-only experiment log. Newest entries go at the end. Record the
question, code/configuration hashes, what changed, observed result,
interpretation, and next decision. Include null results, failures, and
interrupted runs — missing data is not a zero score.

## 2026-09-21 — M0-01–M0-05 bootstrap (no empirical result)

- Question: none (engineering scaffold, not an experiment).
- Code/config: M0 scaffold (`src/config.rs`, `src/rng.rs`, `src/run.rs`,
  thin CLI); configs `env_smoke.toml`, `debug_stationary.toml`; manifests
  reserve disjoint outer-seed ranges. See the `to-do.md` completion ledger
  for exact commands, outcomes, and artifact paths.
- Result: `cargo test`, `validate-config` on both profiles, and scaffold
  `simulate` provenance runs. No learning claim; no statistical check.
- Next: M0-06 (observation boundary types).

## 2026-09-21 — M0 environment evidence bundle (M0-15, no empirical claim)

- Question: none (milestone exit evidence, not an experiment).
- Code/config: environment core + baselines + logging + audit at base
  10a7ae8 with M0 work uncommitted (`git_dirty=true` in manifests);
  `configs/env_smoke.toml` (warmup 4, quiet [4,4], cue 8, gap [0,0],
  response 4, delay [1,1], K=2, eps 0, hazard 0, 8 outcomes/lifetime);
  seeds (root 1, development, outer 1, lifetimes 0-1).
- Commands and results (all release mode unless noted):
  - `cargo fmt --all -- --check` — clean.
  - `cargo clippy --all-targets --locked -- -D warnings` — clean.
  - `cargo test --all-targets --locked` — 62/62 pass (18 lib + 5
    baselines + 2 config + 17 contract + 3 event_logging + 3 event_order
    + 5 leakage + 4 randomized + 5 seeds).
  - `python3 analysis/test_validate_logs.py` — 6/6 pass.
  - `validate-config configs/env_smoke.toml` — OK.
  - `simulate --baseline random --lifetimes 2` → B0, 16/16 outcomes,
    mean reward 0.2500 (`runs/env_smoke-root1-outer1-1789963074/`).
  - `simulate --baseline constant-0 --lifetimes 2` → B1, 16/16, 0.5000
    (`...-retry1/`).
  - `simulate --baseline constant-1 --lifetimes 2` → B1, 16/16, 0.5000
    (`...-retry2/`).
  - `simulate --baseline oracle --lifetimes 2` → O1, 16/16, 1.0000
    (`...-retry3/`).
  - `python3 analysis/validate_logs.py` on all four run dirs — OK x4.
  - Randomized checks (fixed seeds, declared tolerances): mapping
    balance, cue frequency, B0 chance correctness, oracle eps-0.2 reward
    — all inside tolerance on first execution (no reseeding).
- Failure found and fixed: same-second runs overwrote one directory
  (broken retry break); fixed with regression test, colliding runs
  deleted and re-executed. See `docs/decisions.md`.
- Next: M1-01 (inherited topology). M0 smoke command for the M1 reader:
  `cargo run --release --locked -- simulate --config
  configs/env_smoke.toml --baseline random --seed 1` then
  `python3 analysis/validate_logs.py runs/<run-id>`.

## 2026-09-21 UTC — M0 corrective review and re-verification

- Scope: owner-requested review of completed M0, base `57b6870`, corrections
  in a dirty tree. No new scientific experiment or change to `spec.md`.
- Found and reproduced a false audit pass for a seven-of-eight truncated
  lifetime with matching shortened completion counts. Corrected audit,
  runner feedback/interface, run-directory ownership, execution validation,
  and invalid-input RNG handling; details in `docs/m0-review.md` and the
  appended decision record. Original outputs were preserved.
- Full checks: 73 Rust tests, 14 Python tests, fmt, and clippy pass. Existing
  timing/RNG goldens retained. The mapping-pair diagnostic used 2,048 births
  and a tolerance declared before execution: counts 516/502/520/510 pass.
- All four original smoke runs pass the stricter audit. Seven fresh runs
  (development/root 1/outer 1, two lifetimes each) also pass, including
  noisy variable timing and logging disabled. Every original clean smoke
  event is reproduced apart from run identity. The noisy oracle is 32/32
  latently correct with 27 rewards, five flipped outcomes, four changes,
  and 602 ticks; its logging-disabled counterpart reports the same results.
- Evidence: `docs/evidence/m0-review/commands.json` (exact commands/outputs),
  `summary.json` (code/binary/artifact hashes and counts), two diagnostic
  configs, and raw directories under `runs/m0-review/`. Logged-off audit
  coverage is explicitly limited to provenance/completion.
- Failure during audit development: the original duplicate-event fixture
  initially reported only a count mismatch; independent duplicate checking
  restored the diagnostic and all tests pass. No seeds or golden values
  changed to obtain a pass. The intentionally unsupported neural simulation
  exits 1 as expected and is recorded in the evidence.
- Result: M0-GATE re-verified. These are correctness checks, not evidence of
  learning or relative performance. Next eligible task remains M1-01.

## 2026-09-21 UTC — Agent documentation handoff (M0-DOCS)

- Documentation maintenance from `8216c14`; no simulation or experiment run.
- Current continuation guide: `docs/handoff.md`. Commands remain in README;
  task order and ownership remain in the tracker. Corrected bootstrap-era
  descriptions in live docs, source/config comments, and the development
  reservation note. Prior records and M0 review evidence are unchanged.
- The 73 Rust / 14 Python test results above belong to M0-REVIEW. This
  documentation session uses document-consistency checks, not new empirical
  evidence. See the M0-DOCS tracker entry for executed validation.
- Next implementation task remains M1-01; no milestone was advanced.

## 2026-09-21 UTC — M1 corrective review and fresh evidence

- Scope: owner-requested review from clean `13a4873`, with checkpoint,
  validation and runtime diagnostic corrections; no learning/search code.
- Fresh quality battery: 185 Rust tests passed, 0 failed, 1 existing ignored
  weight-printing probe; 15 Python audit tests; clean fmt/Clippy. Three
  checkpoint regressions were demonstrated before the fix and retained.
- Eleven corrected runs plus two separately built original-commit comparisons
  audited successfully. All use development/root 1, outer 1–8 as listed in
  [the evidence summary](evidence/m1-review/summary.json), lifetimes 0–1.
  Clean/noisy event trajectories match the original after removing run IDs;
  logged/unlogged noisy runs have identical health summaries.
- Saved observability measurements: across/within cue distances 1.8681/0.2530;
  actions 20/12 over eight four-choice initializations; 2,000 zero-input ticks
  finite; long-quiet case exactly 248 ticks. Fresh clean demo reward 0.5000,
  all 16 commitments action 0. This last result is an action-biased nonlearner,
  not acquisition evidence.
- Commands, source/binary/artifact hashes and diagnostic data are under
  `docs/evidence/m1-review/`; full raw runs remain in `runs/m1-review/` and
  `runs/m1-review-original/`. The original comparison binaries were built from
  an unmodified `git archive 13a4873` in `/tmp/cra-m1-original-13a4873`.
- Interpretation: M1-GATE re-verified for dynamics/replay only. No final-test
  data, search, M2 score evidence, or M3/M4 learning claim. Next: M2-01.

## 2026-09-21 UTC — M2-03 one-neuron analytical direction diagnostic

- Base `9d31b9d`, with M2-02/M2-03 test and documentation work uncommitted;
  production code unchanged. [Plan](evidence/m2-03/plan.md) saved before
  execution: one million samples, development/root 1/outer 203/lifetime 0/
  actor_noise, existing ChaCha8/Box–Muller stream, alpha 0.2, input 0.7,
  weight 0.3, sigma 0.4, fixed baseline 0.5, tolerance five SE plus 1e-12.
- First execution passed: mean 0.13871680603781847 versus analytical
  0.1388622064964956; SE 0.00010644926671077609. Absolute error
  0.0001454004586771418 is below tolerance 0.0005322463345538805.
  Opposite reward uses the same draws and produces the negative mean with
  identical uncertainty. No reseeding, extra samples or tolerance changes.
- [Result JSON](evidence/m2-03/result.json) includes parameters, seed/hash,
  sample counts, variance/SE, outcome counts, pass flags, toolchain/platform
  and source hashes. [Summary](evidence/m2-03/summary.md) records exact commands.
  Explicit release diagnostic: 1 pass, reported 0.07s test time. Full fast
  suite: 199 pass, 2 default ignores; this diagnostic was run separately,
  the existing weight-printing probe was not. Formatting/Clippy clean.
- One-transition numerical direction evidence only, not online learning or
  convergence. M2-GATE open; next M2-04. No final-test inspection or search.

## 2026-09-21 UTC — M2-05 two-neuron recurrent finite differences

- Clean base `1e6fbf2`; test/documentation work uncommitted at execution.
  [Plan](evidence/m2-05/plan.md) fixed before the first run: six ticks,
  two recurrent neurons, baseline zero, fixed weights/noise, exact score
  sums, binary terminal reward, two million development trajectory groups.
  Seed root 1/outer 205/lifetime 0/actor_noise. Three +/- perturbation pairs
  (0.04, 0.02, 0.01) share Gaussian draws with the base trajectory.
- First run passes without reseeding or threshold changes. Score mean
  0.41802543, SE 0.00069159; finite differences 0.41825625, 0.42040000,
  0.41765000. Each paired discrepancy meets five SE plus 0.0005, every
  five-SE half-width is <=0.02, and score/FD lower bounds are positive.
  No failed samples; 14 million rollouts / 84 million transitions in
  15.35 seconds reported test time, serial. Epsilon checks are correlated.
- [Result](evidence/m2-05/result.json) saves configuration, all uncertainty
  estimates, seed/provenance/source hashes and resource counts.
  [Summary](evidence/m2-05/summary.md) records commands, final quality logs
  and the corrected overflow fixture used during test development.
  Five new fast checks; 212 fast Rust tests pass, three default ignores,
  two compile-fail docs, clean fmt/Clippy. M2-03 was not rerun this session.
- Restricted numerical validation only; no online-learning, convergence,
  or lifetime-gradient claim. No production/spec change or final-test data.
  M2-05 complete; M2-06 next; M2-GATE remains open.


## 2026-09-21 UTC — M2-06 packaging and M2 gate verification

- Package command: `bash scripts/run_score_diagnostics.sh docs/evidence/m2-06/suite`.
  Session began at `1e6fbf2` with prior M2-05 staged; it was committed as
  `28bcdd5` before package execution. New packaging/docs uncommitted.
  [Plan](evidence/m2-06/plan.md) preserves original seeds, sample counts,
  acceptance criteria and numerical code; previous evidence unchanged.
- Fresh 27 fast diagnostic tests, two compile-fail API tests and both bounded
  Monte Carlo checks pass. Direction mean 0.1387168060 (analytic 0.1388622065,
  SE 0.0001064493); recurrent score 0.4180254284 (SE 0.0006915851), all three
  finite-difference comparisons pass agreement and precision criteria.
  All sampled statistics exactly reproduce original JSON; these are same-seed
  replay checks, not independent replications. Zero simulation failures.
- Full Rust suite: 212 passed, three default ignores; both Monte Carlo tests
  passed explicitly, weight-printing probe unrun. Formatting/Clippy clean.
  Four wrapper controls pass, including intentional fake-command failures;
  package stops on error and refuses reused/incomplete output.
- [Evidence](evidence/m2-06/summary.md) includes commands/logs/status, source
  and artifact hashes, complete JSON, deterministic fixture source and plans.
  Combined Monte Carlo budget: 85 million transitions / 25 million normal
  draws, serial; development root1/outer203 or 205/lifetime0/actor_noise.
- M2-06 and M2-GATE complete; next M3-01. No production/spec changes,
  final-test inspection, acquisition or online-unbiasedness/convergence claim.

## 2026-09-21 UTC — M4-05 timing and delay sensitivity

- Question: how do the actual live eligibility and bounded update scales change
  as clean stationary timing moves from delay 1 to variable delay 1..4 and
  then 8..24, across `tau_e` 16/32/64? This is descriptive sensitivity, not
  an acquisition or hyperparameter-selection experiment.
- Predeclared plan: `manifests/m4_timing_sensitivity.json`; development root 1,
  outers 1–3, lifetime 0, 256 outcomes per point, 3 timing profiles x 3
  `tau_e` values x 3 outers = 27 lifetimes / 6,912 outcomes, maximum 270,180
  ticks. No monotonic performance criterion.
- Result: 27/27 complete, 220,005 measured ticks, finite state, no bound
  occupancy, and identical within-profile exogenous schedules across paired
  `tau_e` values. Mean pre-feedback E L1 over outers increased from
  196.8/194.7/175.3 at `tau_e=16` to 381.3/371.7/384.4 at `tau_e=64` for
  stages 0/1/2; mean actual-update L1 increased from
  0.0296/0.0322/0.0442 to 0.0622/0.0617/0.0795. Mean clipping remained below
  0.0091 for every aggregate.
- Longer traces were not behaviorally monotonic: on the longest-delay stage,
  mean reward over outers was 0.4779/0.1810/0.1914 for `tau_e` 16/32/64.
  These short runs include the already documented action-locked family cases
  and do not replace M4-07's acquisition/control comparison.
- Evidence: [summary](evidence/m4-05/summary.md), per-point
  `evidence/m4-05/diagnostic/records.jsonl`, and run metadata
  `evidence/m4-05/diagnostic/result.json`. Full checks: 314 Rust pass, 7
  ignored; 17 Python pass; fixture audit, fmt, Clippy, and both new profile
  validations pass. No validation/final-test seeds inspected. Next: M4-06.

## 2026-09-21 UTC — M4-07 continuous acquisition comparison (negative)

- Predeclared plan: `manifests/m4_continuous_acquisition.json`; development
  root 1, outers 1–3, lifetime 0, 2,000 outcomes, first/final 100 exposures
  per cue, M3-08 full-recurrent winner family, five continuity/control
  conditions, 15 lifetimes / 30,000 outcomes / at most 510,420 ticks.
- Criterion: fully persistent late macro latent accuracy >=0.70 and margin
  over matched continuous B3 >=0.15, with healthy clipping/bounds/nonzero P,
  zero failures, on at least 2/3 outer seeds. It was frozen before execution.
- Result: all 15 lifetimes completed, exact within-outer W0 and exogenous
  schedule pairing passed, zero failures, but the criterion passed 0/3.
  Continuous B4 late accuracy was 0.00/0.00/0.94 versus B3
  0.00/0.00/0.875 (margins 0.00/0.00/0.065). Episodic B4 was
  0.00/0.97/0.935; event-reset B4 was 0.00/0.00/0.955.
- Numerics do not explain the failure: clipping stayed below 0.003, bound
  occupancy and actor saturation were zero, motor-filter maxima stayed below
  0.61, and every continuous run moved P. No validation/final-test seeds were
  inspected. M4-07 stays open and M4-08 is next.
- Evidence: [summary](evidence/m4-07/summary.md), raw condition records
  `evidence/m4-07/run/records.jsonl`, and verdict
  `evidence/m4-07/run/verdict.json`.

## 2026-09-22 UTC — M4-08 continuity-failure audit

- Pre-analysis plan: `manifests/m4_continuity_audit.json`; frozen M4-07
  manifest referenced by SHA-256 (verified untouched at execution), same
  family/seeds/windows used descriptively, 8 paired legs (outer-2 trio plus
  matched B3, outer-3 contrast, outer-1 control, outer-2 tau_e 16/64 with a
  recorded spec-7.9 reason), 16,000 outcomes / at most 272,224 ticks. No
  acquisition criterion; no production change.
- Result: 8/8 complete, 272,224 measured ticks, zero failures, exact
  schedule/W0 pairing, 4/4 archive re-runs bitwise identical to M4-07.
  Outer-2 persistent legs answer action 0 on all 2,000 outcomes each (zero
  reward); baseline decays to 0.000 and late |delta| to 0.0000 despite late
  E L1 of 186–367. Episodic on the same schedule explores (196 first-half
  action-1) and reaches 0.97. Event-reset locks identically with
  consecutive-update cosine ≈ −0.01 (vs 0.32–0.69 growing with tau_e on
  fully persistent legs), isolating persistent activity as the lock.
  Timescale does not rescue (0.00 at 16/32/64). Baseline/saturation/raw-
  identity all healthy. Probes: synthetic two-cue carryover ≈ 80% of fresh
  separation on outers 2–3; constant-input loop holds the wrong action
  600/600 with preferred 1 while P moves (L1 ≈ 1.4).
- Interpretation: continuity-induced behavioral lock with teaching-signal
  starvation; distinct from outer-1's condition-independent lock. M4-08
  verified as audit; M4-GATE remains blocked.
- Evidence: [summary](evidence/m4-08/summary.md), per-outcome series
  `evidence/m4-08/audit/series.jsonl` (16,000 rows), leg/probe aggregates
  `evidence/m4-08/audit/audit.json`.

## 2026-09-22 UTC — M4-08b lock-localization probes

- Pre-analysis plan: `manifests/m4_lock_localization.json`; frozen family,
  outers 2–3, lock-in (60 outcomes, M4-08 protocol) plus dose/flip/converge
  matrices, 76,768 declared transitions. No new range, no production change.
- Result: exactly 76,768 transitions, zero nonfinite states, 16/16 lock
  validity gates, bitwise rebuild identity. Outer-2 1× drive moves the fresh
  readout toward action 0 (+0.11..+0.16) against mappings rewarding 1;
  zero-drive flips locked clones 8/8 (median 50 ticks) while 1×/2×/4× drive
  pins 0/8 with scale-growing margins. Outer-3 cue-1 drive flips 8/8
  (median 14→6→3 ticks); cue-0 drive holds 8/8 correctly. Converge
  distances fall 4–5× by 64 ticks but plateau (0.17/0.33).
- Interpretation: lock localized to input-projection alignment; motor
  inertia escapable, attractor persistence secondary. Scaling input drive
  deepens the pin rather than fixing it.
- Evidence: [summary](evidence/m4-08b/summary.md), matrix
  `evidence/m4-08b/probes/probes.json`.

## 2026-09-22 UTC — M4-08c persistent escape sweep (valid null)

- Pre-analysis `manifests/m4_escape_sweep.json`: eta
  `{1e-4,3e-4,1e-3,3e-3}` × tau_e `{16,32,64}`, unchanged M4-07 actor,
  development root 1/outers 1–3/lifetime 0, windows and bar. One fresh B3
  per outer shared across points; anchor first; no input/motor tuning.
- Executed once: 39/39 lifetimes, 78,000 outcomes, 1,327,092 ticks,
  4.408 seconds export-body time, zero failures. Exact W0/schedule/mapping
  pairing; all three anchor and three B3 aggregate records match M4-07.
  Independent series audit covers all rows and recomputes windows/verdicts.
- Every point passes 0/3 seeds. Outers 1/2 choose action 0 on all 2,000
  outcomes at every setting; late mean abs(delta) ~2e-17 with E L1 133–376.
  Outer 3 late accuracy ranges 0.875–0.970 versus B3 0.875; its frozen
  0.15 margin is unreachable even at perfect accuracy. Maximum clipping
  0.053904, bound occupancy 0, all P norms positive, healthy numerics.
- Nothing adopted. Null applies to this grid and these reused development
  lifetimes only. [M4-08c summary](evidence/m4-08c/summary.md) links all
  series, configs, hashes and provenance; [negative M4-09 bundle](evidence/m4-09/summary.md)
  packages the result. M4 remains blocked; next is a separately declared
  persistent-exploration design investigation. No post-result tuning ran.
