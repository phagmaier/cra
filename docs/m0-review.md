# M0 review — 2026-09-21 UTC

M0 remains a sound foundation for M1 after the corrections below. The
state-machine architecture and independent RNG streams are appropriate for
this stage. There is no measured reason to replace them with a more complex
or optimized simulator. This review does not establish learning, which is
outside M0.

Reviewed base: `57b6870` (`M0 done`), initially clean worktree. Reviewed
Sections 1–10 and the M0 contracts in 16–20, implementation, tests, decision
records, and the four original saved smoke runs. `spec.md` was not changed.
An independent edit to `AGENTS.md` appeared during review and was preserved.

## Findings corrected

| Finding | Consequence before correction | Change |
| --- | --- | --- |
| Incomplete log audit | A declared eight-outcome lifetime truncated to seven passed when completion counts were also changed. Mixed identities, malformed fields, and hidden arithmetic could escape checks. | Validate declared lifetime lengths/counts, contiguous identities, seed derivation, field types, timing, and hidden reward/exposure/change consistency. Test corruptions systematically. |
| Non-atomic run reservation | Simultaneous writers could claim the same directory; a directory lacking a manifest could be overwritten. | Claim paths with exclusive directory creation; preserve incomplete runs. Test eight concurrent reservations against an existing incomplete run. |
| Feedback omitted by ordinary runner | A valid `Agent` implementation never received `apply_feedback`; existing B0/B1 ignored rewards, hiding the omission. | Apply feedback once before the ordinary sensory transition, including the last tick; reject duplicate/invalid baseline feedback. |
| Evaluator object in ordinary policy API | `select_action` accepted `TickOutput`, including evaluator metadata, despite the advertised boundary. Existing B0/B1 ignored it. | Selection now uses policy state only; public inputs still enter through `Agent`. |
| Unsupported execution silently accepted | Neural/search configs and unimplemented environment modes could run as ordinary baselines with misleading provenance. | Separate schema validation from executable-mode validation; fail before creating artifacts. |
| Error paths affected determinism | Invalid/out-of-phase commitments consumed reward RNG before failing. Direct library construction could bypass validation; invalid hidden membership could panic. | Validate before draws/allocations, validate hidden birth data, and reject arithmetic overflow. Test recovery against an untouched paired lifetime. |
| Missing Python version file | The interpreter pin existed only in prose. | Add `.python-version` for the verified installed version, 3.14.7. |

Rust stream validation also now checks hidden joins/arithmetic and overlapping
choices, and no longer copies each event block just to validate it. A new
fixed-seed mapping-pair test checks independence, beyond the original marginal
balance tests. It observed `(00, 01, 10, 11) = (516, 502, 520, 510)` in 2,048
births, within its prespecified four-percentage-point tolerance.

## Verification and results

- `cargo fmt --all -- --check` and `cargo clippy --all-targets --locked -- -D warnings`: pass.
- `cargo test --all-targets --locked`: **73 passed**, none failed or ignored.
- `python3 analysis/test_validate_logs.py`: **14 passed**, including mutation subtests.
- Release CLI help and validation of both supplied configs: pass.
- Attempting to simulate the neural reference config: expected explicit rejection, exit 1.
- Four original smoke directories: pass the stronger audit without modifying raw data.
- Seven fresh bounded runs: pass (six full event audits, one explicitly limited logging-disabled audit).

All fresh runs used development/root 1/outer 1, lifetimes 0–1. The four clean
runs each delivered 16 outcomes and 272 ticks. B0/B1-0/B1-1/O1 reward sums were
4/8/8/16, reproducing every original ordinary and hidden event field except
run identity. The noisy variable-timing B0 and O1 runs each delivered 32
outcomes over 602 ticks, with the same five flipped rewards and four hidden
changes. O1 was latently correct on all 32 choices and received 27 rewards;
B0 received 19. Logging-disabled O1 retained the same reported reward/counts.
These small runs test accounting, not comparative performance.

Exact executed commands and captured outputs are in
[evidence/m0-review/commands.json](evidence/m0-review/commands.json).
[evidence/m0-review/summary.json](evidence/m0-review/summary.json) records run
paths, artifact/source checksums, executable checksum, and measured counts.
Raw new runs remain under `runs/m0-review/`; prior runs and fixtures are
unchanged. The two additional diagnostic configs are saved beside the evidence.

## Limits and next work

M1-01 remains the next eligible task. Keep the planned observe/finish split
before integrating neural dynamics (M1-07); the M0 convenience driver advances
the environment before returning a tick. Preserve the documented choice that
warmup replaces the first quiet interval when budgeting measured ticks. The
current noise assignment is provisional; factorial counterbalancing remains
M5-02 work. Per-tick allocation and all-run event buffering are acceptable for
these bounded smoke runs but should be measured before scaling, as required
by the existing queue. No throughput optimality is claimed.

A passing audit verifies recorded consistency; it cannot certify a fabricated
schedule or fully verify outcomes when event logging is disabled. Future
checkpoint schemas and stateful actors need their own coverage. No neural,
search, final-test, or statistical-analysis milestone was advanced here.
