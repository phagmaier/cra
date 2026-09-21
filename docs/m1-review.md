# M1 corrective review — 2026-09-21 UTC

M1 is re-verified after corrections. The actor equations, receiver/source
orientation, simultaneous old-state reads, noise placement and draw schedule,
adaptation, motor filtering, ordinary-information boundary, and continuous
lifetime execution agree with the specification. This establishes a nonplastic
dynamical-system foundation for M2, not learning or adaptation performance.

The review started from clean commit `13a4873` (`finished m1`). Corrections are
uncommitted in the review snapshot. No scientific equations, seeds, golden
weight values, dependencies, or `spec.md` were changed.

## Findings and corrections

1. **High — checkpoint capture could silently change continuation.**
   `RngState::capture` derived seed bytes from caller metadata without checking
   the live RNG. Capturing an outer-1 trajectory as outer-2 therefore changed
   its future randomness while passing the previous identity check. Capture
   also accepted a different actor configuration and unfinished ticks.
   It now verifies the live seed and supported ChaCha substream, binds live
   configuration and complete-tick bookkeeping, and rejects mismatched state
   before saving. Regression failures before the fix are retained in
   [regressions-before.txt](evidence/m1-review/regressions-before.txt).

2. **High — checkpoint validation was incomplete.**
   Nested unknown fields were silently dropped; absent nullable fields could
   silently become `None`. Restore trusted ragged/mismatched inherited arrays,
   topology metadata, inconsistent motor output, feedback ledgers, pending
   reward timing, and environment/config disagreements. Those paths now fail
   explicitly on capture/load/restore. Both restoration entry points validate
   both halves. The last perturbation vector is preserved immediately after
   restore; warmup duration and reference OS/architecture are recorded.
   Concurrent writers previously shared a PID-only temporary filename; saves
   now reserve distinct temporary files exclusively, sync them, and rename.
   Checkpoint schema **2** intentionally rejects old schema-1 files rather
   than inferring missing state. No checkpoint CLI is claimed.

3. **High — the runnable demo did not enforce or preserve M1 health evidence.**
   The finite watchdog and trace recorder existed only as optional library
   calls/tests. `simulate --baseline actor` could complete with finite states
   beyond the declared watchdog; its trace settings produced no actor files.
   Successful initialization also discarded the structural rejection history.
   Production actor transitions now enforce the existing bounds without
   clipping. Runs save inherited parameters, sampling history, per-lifetime
   health and selected traces, and explicit diagnostic failures. Trace selection
   obeys `full_trace_lifetimes` and `trace_every_ticks`; summaries and watchdog
   enforcement remain enabled when event/trace logging is disabled.
   Initialization failures retain their structured error/rejection log.

4. **Medium — motor and supplied-parameter validation missed invariants.**
   Pool slices could contain duplicate indices or unequal populations. Supplied
   actor parameters were only partially checked. Pool sets, array shapes,
   finite weights, edge-list/mask agreement, missing-edge zeros and topology
   metadata are now checked before use. Hand-built numerical fixtures remain
   free to use acyclic graphs; structural acceptance remains the sampler's job.

5. **Evidence/documentation gaps.**
   The earlier smoke directories had been deleted and observability tests did
   not save their measured diagnostics. Their historic audits cannot be
   independently reconstructed from those missing files. Fresh runs and small
   committed evidence replace that gap without rewriting historical records.
   Stale README/handoff/tracker statements pointing back to M1-01 were corrected.
   Health schema **2** adds sampled activity and makes an empty summary finite
   and JSON-round-trippable for failures before the first successful tick.

The replay instrumentation test previously captured between the final neural
transition and commitment. It now captures after commitment, as required for a
complete tick. This changes the test's observation point, not simulation order.
The long-quiet test now asserts the exact 248 ticks instead of a loose lower
bound. Schema assertions changed because the stored formats changed, not to
hide a numerical mismatch.

## Fresh verification and results

Executed commands, exit codes and output are in
[commands.json](evidence/m1-review/commands.json). Configuration, source and
binary hashes, run paths, file hashes and results are in
[summary.json](evidence/m1-review/summary.json).

- **185 Rust tests passed; 0 failed; 1 ignored** (the existing weight-value
  printing probe, not a required statistical diagnostic).
- **15 Python audit tests passed**; formatting, Clippy with warnings denied,
  `git diff --check`, and three reference-profile validations passed.
- All original actor/topology/weight golden tests still pass. Checkpoint tests
  cover file round-trips, corruption, cross-half consistency, concurrent saves,
  and exact one-tick continuation from every completed boundary of three
  variable-timing lifetimes with an odd 17-neuron actor.
- **11 corrected smoke runs audited successfully:** eight clean B3 runs,
  each with two lifetimes at outer seeds 1–8; one O1 run; one variable-timing,
  noisy B3 run; and its logging-disabled pair. Development namespace, root
  seed 1, lifetime indices 0–1 throughout. Logging-disabled audit coverage is
  provenance/completion only; its health summaries match the logged pair.
- Built the original `13a4873` archive separately and audited **two original
  comparison runs**. Clean and noisy ordinary/hidden event records agree
  exactly with corrected runs after removing run IDs. No healthy action,
  reward, cue, reversal, or timing behavior changed in those comparisons.

| Bounded diagnostic | Measured result |
| --- | --- |
| Alternating cues, six 50-tick blocks | Across-cue activity distance 1.8681; within-cue distance 0.2530 |
| Eight initializations, four choices each | Actions 0/1 occurred 20/12 times |
| Fixed zero input, 2,000 ticks | Maximum absolute membrane 0.3865; zero saturated neuron-ticks |
| Four-outcome lifetime with 64-tick quiets | Exactly 248 ticks; maximum absolute membrane 1.4960 |
| Corrected B3 smoke runs, including logging pair | 3,380 observed ticks; maximum absolute membrane 2.4113; saturation fraction 0.004641 |
| Clean demo, outer 1, two lifetimes | 16/16 delivered outcomes; mean reward 0.5000 |

Individual initializations can be strongly action-biased: the clean outer-1
demo committed action 0 on all 16 choices. M1 requires both actions across
initializations and distinguishable cue activity, which these diagnostics
establish. It does **not** establish balanced choices within every lifetime,
usable learning gradients, or acquisition. The demo's reward 0.5 is not evidence
of learning. M2 score verification and M3/M4 empirical learning gates remain
necessary and have not been advanced.

The committed [observability reports](evidence/m1-review/observability/) and
[demo diagnostics](evidence/m1-review/demo-diagnostics.json) retain small measured
artifacts. Full raw runs remain under ignored `runs/m1-review/` and
`runs/m1-review-original/`; do not delete or overwrite them. A fresh checkout
can rerun the recorded commands into new directories. Bitwise replay remains
a same-code, same-toolchain, reference-platform claim (linux/x86_64);
no cross-platform tolerance study, long nominal experiment, search, or final-test
evaluation was performed.
