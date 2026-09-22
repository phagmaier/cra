# M4-08 evidence — continuity-failure audit

Date: 2026-09-22 UTC. Base revision `08e4f82`; worktree dirty by
construction (new audit manifest, test, and evidence only — no production
file modified).
Task: [M4-08](../../../to-do.md#m4---remove-artificial-trial-resets) (spec
16/M4 "If it fails", Section 21).

## Pre-analysis declaration and budget

`manifests/m4_continuity_audit.json` was written and fast-validated before
any M4-08 audit lifetime was executed. It references the frozen M4-07
manifest by SHA-256 (`18354e…b52d`, verified on disk at audit time),
reuses its actor family (M3 grid index 11), development seeds, and
100/100 per-cue exposure windows descriptively, and declares 8 audit legs:

- outer-2 focus trio plus matched B3: `episodic_b4`, `event_reset_b4`,
  `continuous_b4`, `continuous_b3` (identical W0/schedule);
- `outer3_continuous_b4` (successful-persistent contrast) and
  `outer1_continuous_b4` (representation-locked control);
- `outer2_continuous_b4_tau16` / `_tau64`: the only new development range,
  with a recorded spec-7.9 reason (longer traces keep more stale credit but
  also more unrelated activity; eta held fixed, so this is description, not
  tuning). The matched B3 control is tau_e-invariant by construction (B3
  holds no E/P/baseline) and was rerun freshly in the current revision.

Finite budget: 8 lifetimes, 16,000 outcomes, at most 272,224 ticks, serial
release-mode, fresh output directory. No acquisition pass/fail criterion is
set: the audit records measurements.

## Execution

Fast suite (debug):

```bash
cargo test --locked --test m4_continuity_audit
```

Bounded export (release; directory must not exist beforehand):

```bash
CRA_M4_AUDIT_DIR=docs/evidence/m4-08/audit \
  cargo test --release --locked --test m4_continuity_audit \
  m4_continuity_audit_export -- --ignored --exact --nocapture
```

The export ran once to completion: 8/8 lifetimes, 16,000/16,000 outcomes,
272,224 measured ticks (within budget), zero simulation failures, all states
finite. Raw per-outcome series are `audit/{series.jsonl,audit.json}`
(16,000 series rows).

## Result 1 — archive cross-check exact

All four re-runs reproduce the frozen M4-07 records bitwise:
outer-2 episodic 0.97, outer-2 continuous 0.00, outer-3 continuous 0.94,
outer-1 continuous 0.00. Runner determinism at this revision is confirmed;
no code drift separates this audit from the negative record.

## Result 2 — continuity-induced behavioral lock with teaching-signal starvation

On outer 2, every persistent-activity leg (event-reset, continuous at
tau_e 16/32/64) answers action 0 on **all 2,000 outcomes** (0 action-1
answers in either half), hence zero reward. The baseline decays from 0.5 to
exactly 0.000, so late |delta| is 0.0000 (first-half mean 0.0250): updates
starve **despite large live traces** (late E L1 186–367). Mean raw update L1
is 0.002–0.004, ~13x below the episodic leg (0.041).

The episodic leg on the identical schedule explores (196 action-1 answers
in the first half, from reset states) and bootstraps to late 0.97; its
baseline spans [0.017, 0.984], tracking the learning curve. The outer-2
birth mappings reward action 1 on both cues (episodic late action-0 rates
0.06/0.00 at accuracy 0.94/1.00), so the persistent actor is locked onto
the fully wrong action from birth and the lock is self-reinforcing: wrong
lock → zero reward → zero delta → zero updates → lock persists.

Dissociation: outer 1 locks under *every* condition including episodic
(condition-independent lock, e.g. weak cue drive), while outer 2 locks
only without resets (continuity-specific lock). Outer 3 starts near-correct
(early accuracy 0.93/0.73) and stays there (late 0.94).

## Result 3 — event-reset isolates activity persistence as the lock

Event-reset E-clears every outcome yet still locks at 0.00 with zero
action-1 answers, while its consecutive-update cosine is ≈ −0.01
(independent updates, as in episodic ≈ 0.005). Fully persistent legs show
aligned consecutive updates (cosine 0.53 at tau_e 32; 0.32 at 16; 0.69 at
64): cross-choice trace sharing is real and grows with timescale, and the
hand-set two-step test proves the mechanism exactly (stale component
`eta·delta·E_stale > 0.001` enters the next update). But since the
E-cleared leg locks identically, **trace contamination is not the binding
constraint — persistent activity/motor state is**. The raw identity
`raw_L1 = eta·|delta|·E_L1` holds on every persistent outcome (ordering
intact), and outer-3 learns with cosine 0.35, so alignment per se does not
prevent learning when the actor is not birth-locked.

## Result 4 — timescale null, baseline correct, saturation absent

Outer-2 continuous stays locked (0.00) at tau_e 16/32/64 while E scale
(186→262→367) and alignment (0.32→0.53→0.69) move monotonically: a longer
trace adds stale credit without rescuing escape. No tau_e value is adopted.
Baseline arithmetic tracks mean reward on every leg (no drift bug);
saturation is 0.0 and motor-filter maxima stay below 0.61 on all legs.

## Result 5 — probes

- Representation/carryover (synthetic two-cue drive, outer-2 family
  learner): fresh across-cue separation is nonzero on outers 1–3
  (0.79/0.96/1.23 — cue drive reaches activity), but eight ticks of the
  other cue shift the response by 81%/78%/41% of that separation. Persistent
  state mixes previous-cue information into current encoding at order unity
  on the focus seeds.
- Persistent closed loop (constant input, synthetic reward, no resets,
  600 outcomes): with preferred action 0 (birth bias) the loop holds 1.0 in
  all twelve 50-outcome bins; with preferred action 1 (fully wrong lock) it
  holds 0.0 in all bins on outers 2 and 3 while P still moves (L1
  1.35/1.51) and the raw identity holds exactly. The escape failure
  reproduces at minimal scale without any task structure: **updates flow
  but cannot move behavior past persistent motor/activity inertia**.

## Interpretation and next step

The M4-07 failure is a continuity-induced exploration lock, not a learning-
rule, ordering, baseline, timescale, or saturation defect. The fixed-gate
persistent learner cannot generate the first rewarded action when birth
behavior is fully wrong, because nothing resets the state that produces
that behavior. This is consistent with all M4-07 health evidence (finite
states, no clipping/bounds/saturation, moving P).

M4-08 completes the declared audit. It does not rescue learning and sets no
new acceptance result: M4-GATE remains blocked. Any future mechanism aimed
at escape (e.g. noise scheduling, exploration support, or timescale/range
changes with rerun controls) belongs to a new declared task with fresh
controls; the M4-07 negative record and its criterion stay frozen. No
hidden resets, weight decay, membrane clipping, trained decoder, or gate
change were introduced. No validation or final-test seeds were inspected.

## Verification and artifacts

- Fast tests: 6 passed, 1 explicit empirical test ignored.
- Release export: 1 passed; 8/8 lifetimes, 16,000 outcomes, 272,224 ticks,
  zero failures; archive cross-check 4/4 exact.
- Full battery: `cargo test --all-targets --locked` (see ledger), 2
  compile-fail doc checks, 17 Python audit tests, committed fixture audit,
  `cargo fmt --all -- --check` and Clippy `-D warnings` clean,
  `git diff --check` clean.

SHA-256:

```text
bd6c96473df9e5c68000f4ed06436df54b9a2ebcad24b29dcae918c62db58bb4  manifests/m4_continuity_audit.json
4c399c79078b0b60560e45c23e2cf10bf71b2620793cad484a96ed701e3c9f52  docs/evidence/m4-08/audit/audit.json
8372b17bc8323d830e06099e0dae0a64010d9f2ecce006be31e54541ab16e5d8  docs/evidence/m4-08/audit/series.jsonl
```

## Claim limit

This development audit characterizes the frozen M4-07 failure on
development seeds with the tested locally plastic actor family. It is not
evidence of continuous acquisition, and the per-cue windows are descriptive
here, not an acceptance criterion. Figures and prose above derive from
`audit.json`/`series.jsonl` at the recorded revision.
