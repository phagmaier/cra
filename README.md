# Learning When to Learn

Implementation of [`spec.md`](spec.md) (v0.1, Sept 20 2026): does internally
generated gating improve a continuously running recurrent agent's adaptation
to real changes without damaging stable associations under misleading
feedback?

Status: **M0 complete and re-verified; M1 complete and re-verified (continuous
nonplastic actor demo, bitwise replay on linux/x86_64 — explicitly not
learning); M2-GATE passed (restricted score diagnostics); M3-01 implemented
(plastic offsets, eligibility, masks, effective-weight refresh); M3-02
implemented (exactly-once gated `P` updates, running baseline, separated
raw/limited/actual reports — unit level, no runner yet); M3-03 implemented
(Section 17.3 golden fixture, fixture-only); M3 preflight hardening verified;
M3-04 implemented (explicitly episodic clean-learning diagnostic runner,
fixed-gate learner with logged rollout resets — not the continuous result);
M3-05 implemented (matched B3/B4/shuffled-reward controls with recorded
corruption protocol — machinery only, no acquisition claim);
M3-06 implemented (frozen pre-results development grid, criterion, and
budget — plan only, nothing executed);
M3-07 implemented (216-lifetime acquisition sweep passes with winner grid
index 11 — episodic motor-afferent learning on responsive actors);
M3-08 implemented (9-lifetime full-recurrent comparison at grid index 11
passes 2/3 seeds — episodic all-recurrent learning on responsive actors,
motor-only run preserved);
M3-09 implemented (diagnostic-only failure-isolation path: single-motor
sign/order/drift, outer-1 representation separation, permutation
sensitivity with bitwise identity control — 5 new tests);
M3-10 implemented (learned-offset checkpoints: schema-3 replay exact
through learning events with nonzero P/E, pinned archives — 9 new tests);
**M3-GATE passed** (fresh release re-runs reproduce the M3-07/M3-08
verdicts exactly; full battery green — clean episodic acquisition
proven, continuous acquisition explicitly still open); **M4-01 done**
(authoritative main tick order: split `observe`/`finish_tick`, six
drivers migrated, tick-20/delay-3 learning fixture — 4 new tests);
**M4-02 done** (fully persistent learner: exact decaying traces with no
resets, live-trace anti-snapshot, baseline event counts — 5 new tests);
**M4-03 done** (birth-only resets: reset-audited continuous runner,
P-telescoping tripwire, live warmup, reversals without resets — 6 new
tests); **M4-04 done** (three continuity conditions with disjoint
guards, same-seed pairing, `continuous_stationary` executable twin of
the `debug_stationary` source — 6 new tests); **M4-05 done** (three
gradual clean timing stages, exact endpoint/pairing checks, and a saved
27-lifetime `tau_e`/trace/update sensitivity record — 4 new default
tests plus one explicit bounded diagnostic); **M4-06 done** (schema-4 exact
continuous pause/resume with nonzero P/E at ongoing, pre-feedback, and
post-feedback boundaries; schema 2/3 preserved — 9 new tests); **M4-07
declared comparison executed but did not pass** (15/15 paired development
lifetimes completed with healthy numerics, but fully persistent acquisition
met the frozen criterion on 0/3 seeds). M4-07 and M4-GATE remain open; next
task M4-08 (continuity-failure audit).
The simulator core exists as
a library
(`src/environment/`, `src/agent/` nonplastic dynamics plus `B3`
harness, `src/checkpoint.rs`, `src/logging/`, `src/experiments/episodic.rs`
diagnostic runner) with deterministic
fixtures, randomized checks, baseline/actor runners, replay proofs, and
an offline log audit. Lifetime plastic state, the feedback update, and the
episodic diagnostic runner now
exist; the M4-07 bounded comparison harness also exists, but no gating or
evolution pipeline is wired into a runner yet. Anything listed under "Planned" is a
target from spec Section 18 / `to-do.md`, not working code.

M0 was re-reviewed and corrected without changing the original smoke
trajectories. See [the review](docs/m0-review.md) for findings, verification,
and the historical M1 integration work. The [M1 corrective review](docs/m1-review.md)
records checkpoint/diagnostic fixes and fresh measured evidence.

## Continue development

Start with [AGENTS.md](AGENTS.md), the [current tracker](to-do.md), and
the [agent continuation guide](docs/handoff.md). The guide maps existing
code/tests to M3-04 and records the integration limits to preserve.

| Document | Responsibility |
| --- | --- |
| [spec.md](spec.md) | Scientific contracts and proposed design; Section 9 owns tick ordering |
| [to-do.md](to-do.md) | Current status, ownership, ordered tasks, gates, and completion evidence |
| [AGENTS.md](AGENTS.md) | Durable work conventions and scientific safeguards |
| [docs/handoff.md](docs/handoff.md) | Current continuation notes and implementation entry points |
| [docs/decisions.md](docs/decisions.md) | Dated decisions and superseding corrections |
| [docs/experiments.md](docs/experiments.md) | Append-only run records, including failures |

The spec's original unchecked implementation list is historical design
text. Use the tracker for completion state. Historical evidence retains
the versions, counts, and limitations that applied when it was recorded.

## Toolchain (pinned)

- Rust `1.98.0` (see `rust-toolchain.toml`), Cargo `1.98.0`, resolution in
  `Cargo.lock` (committed).
- Python `3.14.7` (see `.python-version`) for the offline analysis layer;
  no third-party Python dependencies (see `analysis/requirements.txt`).
  Add a dependency lock when analysis first needs external packages.
- Key Rust deps (pinned in `Cargo.lock`): `clap 4.6.7`, `serde 1.0.229`,
  `serde_json`, `toml 0.8.23`, `thiserror 2.0`, `rand 0.9.5`,
  `rand_chacha 0.9.0`, `sha2 0.10.9`. No deep-learning framework.

## Verification

Run commands from the repository root with the pinned toolchains.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
python3 analysis/test_validate_logs.py
```

The last full M0 review recorded **73 Rust tests and 14 Python tests passing**,
with fmt/clippy clean. See [recorded commands and outputs](docs/evidence/m0-review/commands.json).
Those counts describe that execution. The fresh [M1 review](docs/m1-review.md)
recorded **185 Rust tests and 15 Python tests passing**, with one existing
ignored weight-printing probe and clean fmt/Clippy. Rerun relevant checks after
changes.

M2-01 adds eight deterministic score-contract tests, callable with
`cargo test --locked --test score`. Its verification recorded **193 Rust
tests passing**, one existing ignored weight-printing probe, and clean
fmt/Clippy. The pure function in `src/agent/score.rs` validates finite inputs
and positive noise, then computes the per-edge conditional Gaussian score.
M2-02 checks the fixed-sample conditional derivative at four epsilon values
across six leak/noise settings (288 comparisons), with a moving-sample
negative control. Run it with:

```bash
cargo test --locked --test score_log_probability -- --nocapture
```

Its [saved evidence](docs/evidence/m2-02/summary.md) records three new tests,
**196 Rust tests passing** overall, one existing ignored probe, and clean
fmt/Clippy.

M2-03 adds the bounded one-neuron learning-direction diagnostic. Its
[predeclared plan and results](docs/evidence/m2-03/summary.md) record one
million development samples: mean 0.1387168, SE 0.00010645, analytical
derivative 0.1388622; the opposite target negates the estimate. Both pass
the declared five-SE tolerance. Run explicitly (it is ignored by default):

```bash
cargo test --release --locked --test score_learning_direction one_neuron_learning_direction -- --ignored --exact --nocapture
```

To save immutable JSON, set `CRA_M2_DIRECTION_EVIDENCE` to a fresh file in
an existing directory; existing files are rejected. M2-03 verification:
**199 fast Rust tests passed**, two default ignores (this separately passed
diagnostic and the existing weight-printing probe), clean fmt/Clippy.
M2-04 implements `experiments::finite_rollout::FiniteRollout`, an isolated
fixed-weight diagnostic with zero initial state, no score decay, a constant
baseline and optional single terminal update on a weight copy. Its explicit
between-rollout resets do not change `simulate` or the continuous agent.

```bash
cargo test --locked --test finite_rollout
cargo test --locked --doc
```

Set `CRA_M2_ROLLOUT_EVIDENCE` to a fresh file in an existing directory to
save the golden test's result. [M2-04 evidence](docs/evidence/m2-04/summary.md):
eight integration tests and two compile-fail API checks pass; **207 fast
Rust tests pass** overall, two default ignores, clean fmt/Clippy.

M2-05 compares a six-tick two-neuron recurrent score estimator with central
finite differences at three perturbation sizes using paired Gaussian draws.
Its [predeclared plan and results](docs/evidence/m2-05/summary.md) record two
million independent trajectory groups: score 0.41802543 (SE 0.00069159),
finite differences 0.41765–0.42040. All agreement and precision checks pass.

```bash
cargo test --locked --test score_recurrent
cargo test --release --locked --test score_recurrent two_neuron_recurrent_finite_difference -- --ignored --exact --nocapture
```

Set `CRA_M2_RECURRENT_EVIDENCE` to a fresh file in an existing directory for
immutable JSON output; existing files are rejected. Five new fast checks
pass. Full M2-05 verification: **212 fast Rust tests passed**, three default
ignores (M2-03, M2-05, weight-printing probe), two compile-fail doc checks,
clean fmt/Clippy. M2-05's ignored diagnostic was separately executed; M2-03
was not rerun in that M2-05 session. These checks are not evidence of online
learning or convergence.

M2-06 packages every required score check into one explicit bounded run:

```bash
bash scripts/run_score_diagnostics.sh runs/m2-diagnostics-fresh
```

Choose a **new directory** whose parent exists. The script requires Bash and
standard Linux utilities (`sha256sum` included), uses the pinned Rust tools,
and saves commands, exit codes, logs, source hashes, deterministic fixtures,
plans and result JSON. It refuses an existing directory, stops on failure,
and writes `PASSED` only after the required tests and exports succeed. Do not
reuse failed/interrupted output. Individual commands above remain available;
ordinary `cargo test` keeps Monte Carlo ignored.

The finite budget is one million one-neuron samples plus two million
six-tick recurrent trajectory groups (85 million total Monte Carlo neural
transitions), using the original development seeds and tolerances. The
[complete M2 verification](docs/evidence/m2-06/summary.md) freshly passed
27 fast diagnostic tests, two compile-fail API checks and both Monte Carlo
tests; sampled results exactly reproduce the original evidence. Full suite:
212 passed, three default ignores, clean fmt/Clippy. **M2-GATE passed.**

M3-01 adds `agent::plasticity`: `PlasticState` stores plastic offsets `P`
and eligibility `E` separately from immutable `W0`, builds the
`all_recurrent_edges` or `motor_afferent_only` mask, accumulates
eligibility under either the `persistent` or `no_decay_diagnostic` trace
policy, and refreshes the `W0 + P` effective-weight cache in one validated
location. `PlasticSnapshot` is a versioned, strict serialization unit with
explicit restore validation. The actor gains
`step_with_effective_weights` sharing the `W0` arithmetic core, so the
no-learning path is bitwise unchanged. This is storage/eligibility only:
no reward update, baseline, gating, or runner is enabled yet.

```bash
cargo test --locked --test plasticity
```

Its [evidence record](docs/evidence/m3-01/summary.md) documents 16 new
tests, **232 fast Rust tests passed**, three default ignores, two
compile-fail doc checks, and clean fmt/Clippy. These
checks do not demonstrate learning.

M3-02 adds `PlasticState::apply_feedback_once`: exactly-once gated `P`
updates with old-baseline `delta`, ordered `max_update`/`plastic_bound`
clamps, one baseline update, dedup bookkeeping, separated raw/limited/
actual reports, and snapshot schema 2 carrying baseline plus dedup. Fixed
mode passes gate `1`; the finite-rollout diagnostic keeps its frozen
baseline separately.

```bash
cargo test --locked --test feedback_updates
```

Its [evidence record](docs/evidence/m3-02/summary.md) documents 11 new
tests, **243 fast Rust tests passed**, three default ignores, two
compile-fail doc checks, and clean fmt/Clippy. These
checks prove update arithmetic, not acquisition.

M3-03 adds `tests/golden_updates.rs`: the Section 17.3 chain (`0.4` /
`0.67` / `0.4` / `0.00067` / `0.10067` / `0.64`) through the public
entry points with tight tolerances, plus separate clipped cases. No
production change.

```bash
cargo test --locked --test golden_updates
```

Its [evidence record](docs/evidence/m3-03/summary.md) documents 4 new
tests, **247 fast Rust tests passed**, three default ignores, two
compile-fail doc checks, and clean fmt/Clippy. This
fixture proves arithmetic, not acquisition.

M3-04 adds `src/experiments/episodic.rs` plus `configs/episodic_stationary.toml`:
an explicitly episodic clean-learning diagnostic (2 cues, zero noise/hazard,
no gap, delay 1, `episodic_diagnostic` resets with `no_decay_diagnostic`
traces, fixed gate 1, one terminal update per one-choice rollout). The
learner is built from agent-only inputs and the driver logs reset ticks,
mode, policies, and every raw/limited/actual update report.

```bash
cargo test --locked --test episodic_runner
cargo run --locked -- validate-config configs/episodic_stationary.toml
```

Its [evidence record](docs/evidence/m3-04/summary.md) documents 10 new
integration tests plus 1 unit guard test, **260 fast Rust tests passed**,
three default ignores, two compile-fail doc checks, 17 Python audit tests,
and clean fmt/Clippy. `simulate` still rejects this profile on both the
baseline and B3 rungs (it requires `birth_only`), so the diagnostic cannot
masquerade as the continuous result. No acquisition or
continuous-learning claim exists yet.

M3-05 adds the matched control family in `src/experiments/episodic.rs`:
`run_episodic_no_learning` (B3: same schedule/inheritance/streams/resets,
no plastic state by construction), `run_episodic_shuffled` (B4 learner with
the recorded independent-fair-coin corruption on a dedicated public-seed
stream; observed rewards kept separately), and `run_episodic_conditions`
(all three paired by construction).

```bash
cargo test --locked --test episodic_controls
```

Its [evidence record](docs/evidence/m3-05/summary.md) documents 5 new
tests, **265 fast Rust tests passed**, three default ignores, two
compile-fail doc checks, 17 Python audit tests, and clean fmt/Clippy.
Control machinery only — the grid, criterion, and several-seed comparison
are M3-06/M3-07.

M3-06 freezes the pre-results sweep in `manifests/m3_development_grid.json`:
24 combinations of eta/input-scale/gain/sigma on the episodic base,
development outers 1–3, 2,000 outcomes per lifetime with early-first-200
and late-final-200 windows, matched B3/B4/B4-shuffled at every point,
declared margins (B4−B3 ≥ 0.15, B4−shuffled ≥ 0.10 on ≥ 2/3 seeds) with
failure/clipping/`P`-movement guardrails, deterministic selection, and a
216-lifetime budget. `src/experiments/grid.rs` loads and validates the
declaration; expansion proves each point executable without running
anything.

```bash
cargo test --locked --test development_grid
```

Its [evidence record](docs/evidence/m3-06/summary.md) documents 6 new
tests, **271 fast Rust tests passed**, three default ignores, two
compile-fail doc checks, 17 Python audit tests, and clean fmt/Clippy. Plan
only — no sweep executed, no outcome observed.

M3-07 executes that grid in release (`src/experiments/sweep.rs` analysis:
windows, margins, clipping/bound health, criterion, tiebreak selection;
`tests/m3_acquisition.rs` fast analysis test plus the ignored bounded
sweep). Winner: grid index 11 (eta 0.001, input 0.2, gain 0.8, sigma
0.05) — outer-2 B4 rises 0.035 early to 0.860 late against 0.040 controls,
outer-3 reaches 0.935 against 0.770/0.765; points 17/19 also pass 2/3
seeds with guardrails green sweep-wide.

```bash
cargo test --locked --test m3_acquisition
CRA_M3_SWEEP_DIR=runs/m3-sweep-fresh cargo test --release --locked --test m3_acquisition m3_motor_afferent_sweep -- --ignored --exact
```

Its [evidence record](docs/evidence/m3-07/summary.md) documents the full
216-lifetime sweep (archived `seed_records.jsonl` + `verdict.json`),
**274 fast Rust tests passed**, three default ignores (the sweep itself
among the separately invoked diagnostics), two compile-fail doc checks, 17
Python audit tests, and clean fmt/Clippy. Episodic motor-afferent
acquisition on responsive actors; birth-locked initializations documented
as a family bound. Selected config for M3-08: grid index 11. **M3-08 is
next.**

M3-08 extends that learner to `all_recurrent_edges` at the frozen winner
point (grid index 11) with no production change and the M3-06 manifest
untouched (`tests/m3_full_recurrent.rs` fast analysis test plus the
ignored bounded comparison; the M3-07 motor-only run stays preserved as
a diagnostic). The 9-lifetime release comparison passes 2/3 seeds under
the same criterion — outer-2 B4 rises 0.045 early to 0.975 late against
0.040/0.030 controls, outer-3 reaches 0.955 against 0.770/0.775 —
with guardrails green sweep-wide.

```bash
cargo test --locked --test m3_full_recurrent
CRA_M3_FULL_SWEEP_DIR=runs/m3-full-fresh cargo test --release --locked --test m3_full_recurrent m3_full_recurrent_comparison -- --ignored --exact
```

Its [evidence record](docs/evidence/m3-08/summary.md) documents the
9/9-lifetime comparison (archived `seed_records.jsonl` + `verdict.json`),
**275 fast Rust tests passed**, five ignores (both Monte Carlo checks,
the weight-printing probe, both M3 sweeps — the comparisons among the
separately invoked diagnostics), two compile-fail doc checks, 17 Python
audit tests, and clean fmt/Clippy. Episodic all-recurrent acquisition
on responsive actors; birth-locked initializations documented as a
family bound under both masks. Actor family for M4: the full-recurrent
episodic learner at grid index 11. **M3-09 is
next.**

M3-09 adds `src/experiments/reduction.rs` (diagnostic-only): the
spec-M3 "if it fails" instruments — a single-motor closed-loop check
(one plastic edge, constant input, known preferred action, synthetic
reward) and a receiver-permutation sensitivity probe over the episodic
driver — plus a diagnostic-only learner hook on a shared transition
core (ordinary runners still call `advance` only; the identity
permutation reproduces the verified runner bitwise).

```bash
cargo test --locked --test m3_reduction
```

Its [evidence record](docs/evidence/m3-09/summary.md) documents the
reduction ladder, 5 new tests (**280 fast Rust tests passed**, five
ignores, two compile-fail doc checks, 17 Python audit tests, clean
fmt/Clippy), and the findings: the path was not needed to rescue
acquisition; outer-1 is independently confirmed a representation
failure (offsets move, behavior locked); permuted perturbations change
the updates with no behavioral magnitude demanded. Diagnostic tooling
only — synthetic rewards are never a task result. **M3-10 is next.**

M3-10 adds learned-offset checkpoints (`LearningCheckpoint`, schema 3
in `src/checkpoint.rs`; the M1 schema 2 is untouched and neither
loader reads the other's files) with learner snapshot/restore in
`src/experiments/episodic.rs`. A faithful manual driver is proven
bit-identical to the verified runner, then splits at the rollout
boundary, just before feedback, and post-feedback pre-reset resume
exactly with nonzero `P`/`E` and explicit rejections.

```bash
cargo test --locked --test episodic_checkpoint
CRA_M3_10_DIR=runs/m3-10-fresh cargo test --release --locked --test episodic_checkpoint m3_learning_evidence_capture -- --ignored --exact
```

Its [evidence record](docs/evidence/m3-10/summary.md) documents 8 new
tests plus the ignored bounded capture (**289 fast Rust tests
passed**, six ignores, two compile-fail doc checks, 17 Python audit
tests, clean fmt/Clippy) and the pinned archives: winner working
config, verbatim outer-1 failure case, real boundary/final checkpoint
files (final reloads with `P` L1 4.6972), and first/last update
digests cross-matching the M3-08 record exactly. Arithmetic, episodic
acquisition, and replay are distinguished; continuous acquisition
remains unverified M4 work. **M3-GATE is next.**

M3-GATE re-ran the evidence rather than citing it: the frozen 216-
lifetime grid sweep reproduces the archived M3-07 verdict (winner
index 11, 72/72 seed records identical, ~16 s) and the 9-lifetime
comparison reproduces the M3-08 verdict (passes 2/3, ~0.7 s) —
both in release into fresh directories. Full battery: **289 fast
Rust tests passed**, six ignores (both Monte Carlo checks, the
weight-printing probe, three bounded captures — each invoked
separately per its evidence record), two compile-fail doc checks, 17
Python audit tests, fixture audit OK, clean fmt/Clippy. The gate
rests on exit conditions (a)–(d) in the [tracker ledger](to-do.md):
several-seed learning over matched controls, valid score/golden
tests, interpretable numerics, no lucky trajectory. **M3 COMPLETE —
M4-01 through M4-06 done.** M4-05 adds
[`continuous_variable_short`](configs/continuous_variable_short.toml) and
[`continuous_variable_delayed`](configs/continuous_variable_delayed.toml),
table-pins all timing endpoints, and saves paired development measurements for
`tau_e` 16/32/64. The 27-lifetime diagnostic completed 6,912 outcomes and
220,005 ticks; longer traces increased measured trace/update scale and
clipping but did not monotonically improve reward. See the
[M4-05 evidence](docs/evidence/m4-05/summary.md). M4-06 adds the separate
schema-4 `ContinuousCheckpoint`: exact splits during ongoing activity,
immediately before pending feedback, and after feedback preserve all live
learner/environment/RNG state and reject duplicate delivery or incomplete and
incompatible files. Schema 2 and schema 3 compatibility tests remain green.
See the [M4-06 evidence](docs/evidence/m4-06/summary.md). M4-07 then executed
the frozen five-condition comparison: all 15 lifetimes completed with exact
pairing and healthy update/bound/saturation diagnostics, but fully persistent
B4 late accuracy was 0.00/0.00/0.94 against B3 0.00/0.00/0.875, so no seed
cleared the declared accuracy-plus-margin criterion. See the
[M4-07 negative evidence](docs/evidence/m4-07/summary.md). **M4-08 is next.**
Continuous acquisition beyond matched B3 remains unverified; no modulation,
evolution, or broader-track claim follows.
The owner-requested [M3 preflight hardening](docs/evidence/m3-preflight/summary.md)
separates hidden cue-role RNG from actor initialization, replaces positional
feedback hyperparameters with `FeedbackUpdateParams`, and makes
`plastic_bound` a validated lifetime/snapshot invariant. Plastic snapshots
are now schema 3; the top-level nonplastic checkpoint remains schema 2.
The full fast suite passes with **249 tests**, three intentional ignores,
17 Python audit tests pass, and a fresh release-mode smoke run audits clean.
M3-03 golden values are unchanged. Mixed stable/volatile cue-role assignment
has an explicitly recorded deterministic stream migration; no acquisition
result exists yet.

To rerun the bounded M4-05 timing sensitivity diagnostic, choose a fresh
output path:

```bash
CRA_M4_TIMING_DIR=/tmp/cra-m4-timing-fresh \
  cargo test --release --locked --test m4_timing \
  m4_timing_sensitivity_diagnostic -- --ignored --exact --nocapture
```

To reproduce the frozen M4-07 development comparison into a fresh directory
(the recorded verdict is negative; do not alter seeds or thresholds):

```bash
CRA_M4_ACQUISITION_DIR=/tmp/cra-m4-acquisition-fresh \
  cargo test --release --locked --test m4_continuous_acquisition \
  m4_continuous_acquisition_comparison -- --ignored --exact --nocapture
```

To verify exact continuous-learning pause/resume and the unchanged episodic
checkpoint contract:

```bash
cargo test --release --locked --test continuous_checkpoint
cargo test --locked --test episodic_checkpoint
```

To save the bounded observability tests' measured diagnostics, choose a fresh
output directory (existing evidence files are never overwritten):

```bash
CRA_M1_EVIDENCE_DIR=runs/m1-observability-fresh cargo test --locked --test observability
```

## Implemented commands

```bash
cargo run --release --locked -- validate-config configs/env_smoke.toml
cargo run --release --locked -- validate-config configs/debug_stationary.toml
cargo run --release --locked -- validate-config configs/actor_no_learning.toml
cargo run --release --locked -- validate-config configs/continuous_variable_short.toml
cargo run --release --locked -- validate-config configs/continuous_variable_delayed.toml
cargo run --release --locked -- simulate --config configs/env_smoke.toml --baseline random --lifetimes 2 --seed 1 --outer-seed 1
cargo run --release --locked -- simulate --config configs/env_smoke.toml --baseline oracle --lifetimes 2 --seed 1 --outer-seed 1
cargo run --release --locked -- simulate --config configs/actor_no_learning.toml --baseline actor --lifetimes 2 --seed 1
python3 analysis/validate_logs.py analysis/fixtures/valid
```

To audit a new run, pass the directory printed after `run dir:` to
`python3 analysis/validate_logs.py`. The last command above audits the
committed fixture, so it also works when local `runs/` output is absent.

Baselines: `random` (B0), `constant-0` / `constant-1` (B1), `actor`
(B3, the same inherited actor with plasticity disabled — needs an
`[actor]` config such as `configs/actor_no_learning.toml`),
`oracle` (O1, privileged reference). The audit checks manifest/config/condition
identity, derived seed streams, declared lifetime lengths, timing,
finite values, duplicate feedback, hidden reward/change accounting, and
completion. Fixtures and mutation tests live in `analysis/`.
With `event_log=false`, it reports reduced provenance/completion coverage.

Notes:

- `validate-config <file>` parses and validates the TOML against the
  `schema_version = 1` schema (spec 19). Unknown fields/mode names are
  rejected. Known future profiles may validate without being executable.
- M0 `simulate` requires an environment-only configuration. Neural/search
  sections, diagnostic reset modes, `isolated_reversal`, and `long_life`
  are rejected before creating a run. `debug_stationary.toml` is currently
  a validation reference, not an executable learner.
- `simulate --baseline actor` (B3) requires an `[actor]` configuration
  and runs the continuous nonplastic actor through the same tick,
  commitment, and event-logging loop as the baselines; an env-only
  config is rejected for `actor`, and an actor config is rejected for
  the B0/B1/O1 rungs. Example M1 demo (condition B3, 2 lifetimes,
  16 commitments/outcomes): `runs/actor_no_learning-root1-outer1-1789975911`
  (historical mean reward 0.5000; original raw directory was removed).
  Fresh preserved review demo:
  `runs/m1-review/actor_no_learning-root1-outer1-1789977523`
  (mean reward 0.5000, audit `OK (events + provenance)`); rerunning the
  command prints a fresh directory. Checkpoint pause/resume is exercised
  by `cargo test --locked --test checkpoint` (dedicated checkpoint CLI
  arrives with later milestones). Checkpoint schema 2 rejects schema-1
  files; it records complete tick state and the reference platform.
- B3 runs also save `actor-inherited.json` and `actor-diagnostics-<index>.json`.
  Diagnostics include structural sampling history, watchdog bounds, health,
  and selected h/a/r/q traces (health schema 2). `full_trace_lifetimes` selects
  the first N lifetimes and `trace_every_ticks` controls their sampling;
  health/watchdog checks remain enabled with event logging or traces disabled.
  Failures have explicit diagnostic/terminal records. The ordinary event audit
  does not independently recompute neural dynamics from these diagnostics.
- `simulate` runs real baseline lifetimes (M0-07–M0-11 contracts) into a
  fresh run directory with `resolved_config.toml`, `manifest.json`,
  `seed_streams.json`, `condition.json`, and `completion.json`, plus
  `events.jsonl` and `hidden.jsonl` when event logging is enabled.
  Same-second runs never share a directory
  (`-retryN` suffix, atomic directory reservation). Existing incomplete
  runs are preserved. Library behavior is also exercised by
  `cargo test --locked --test environment_contract`.
- Seed namespaces (`development`, `training`, `validation`, `final_test`)
  are disjoint by construction. Final-test seeds must never enter
  tuning/search/validation. Stream derivation:
  `(root, namespace, outer, lifetime, stream) -> SHA-256 -> ChaCha8Rng`,
  one RNG instance per stream, so agent draws never perturb environment
  schedules (see `src/rng.rs`).
- The [manifest reservations](manifests/README.md) document seed policy;
  M0 does not load them or enforce their numeric ranges. Resolved config
  and explicit CLI overrides identify the seeds actually used.

## Planned commands (not implemented)

`benchmark`, `evolve`, `evaluate`, and `intervene` arrive in their
milestone tasks (M5, M7–M9). `aggregate.py` arrives with the comparison
pipeline in M8. Do not treat their absence as a failure of M0.

## Run directories and artifact policy

- Runs live in `runs/<profile>-root<R>-outer<O>-<unixsecs>[-retryN]/` and
  are **git-ignored** (see `.gitignore`). Source commits carry code, configs,
  manifests, tests, and docs — never large generated data or build output.
- Each run directory holds `resolved_config.toml` (complete effective
  configuration), `manifest.json` (code revision + dirty status, platform,
  toolchain, seed/RNG policy, condition), `seed_streams.json` (derived
  stream seeds), `condition.json`, `events.jsonl` (ordinary records),
  `hidden.jsonl` (evaluator annotations), and `completion.json` (terminal
  status + counts).
- Raw output is immutable; derived analysis goes elsewhere.
  `docs/experiments.md` is append-only, including failures.
- Small reproducibility records live in `docs/evidence/`; original raw
  runs may exist only in the originating workspace. A fresh checkout can
  use the committed fixture or rerun saved commands into a new directory.

## Layout

`spec.md`, `to-do.md`, `AGENTS.md` at root. `src/` holds `config.rs`
(versioned TOML schema), `rng.rs` (seed derivation), `environment/`
(observation boundary, hidden state, scheduling, features, rewards),
`agent/topology.rs` (M1-01 inherited mask, fixed motor pools, stable edge
order, structural validation), `agent/weights.rs` (M1-02 row-scaled `W0`,
dense `B`, zero biases, parameter validation), `agent/actor.rs` (M1-03
double-buffered transition, `-expm1` leaks, post-integration noise),
`agent/score.rs` (M2-01 pure per-edge conditional Gaussian score),
`agent/motor.rs` (M1-06 pool means, leaky filter, new-q commitment),
`agent/no_learning.rs` (M1-07 B3 continuous actor through the ordinary
runner), `agent/health.rs` (M1-08 read-only watchdog, summaries, stable
traces), `checkpoint.rs` (schema-2 nonplastic, schema-3 episodic, and
schema-4 continuous lifetime files with config hash, checksum, and atomic
writes), `experiments/baseline.rs` (B0/B1/B3/O1 harness),
`experiments/finite_rollout.rs` (M2-04 fixed-weight, no-decay diagnostic),
`experiments/episodic.rs` (M3-04 fixed-gate episodic diagnostic runner with
logged rollout resets), `experiments/continuous.rs` (M4 persistent learner,
continuity runners, pre-feedback trace measurements, and continuous
snapshot/restore),
`logging/` (event records + validation), `run.rs` (provenance + simulation
runner), and thin `main.rs`.
`configs/` holds `env_smoke.toml` (M0 smoke), `debug_stationary.toml`
(spec 19.2 reference + seeds), `continuous_stationary.toml` plus
`continuous_variable_short.toml` / `continuous_variable_delayed.toml`
(M4 clean continuous timing curriculum), `episodic_stationary.toml`
(M3 diagnostic: `episodic_diagnostic` resets and `no_decay_diagnostic`
traces — library-only, rejected by `simulate`), and
`actor_no_learning.toml` (M1 B3 demo: env-smoke timing plus the debug actor
section, no learning/modulator/evolution). `manifests/` reserves disjoint seed
ranges and stores declared development suites. `analysis/` holds the
stdlib-only log audit plus fixtures.
`docs/decisions.md` records scientific ambiguities/deviations;
`docs/experiments.md` is the append-only experiment log.
