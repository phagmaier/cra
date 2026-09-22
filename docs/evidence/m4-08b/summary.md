# M4-08b evidence — lock-localization probes

Date: 2026-09-22 UTC. Base revision `08e4f82`; worktree dirty by
construction (new probe manifest, test, and evidence only — no production
file modified).
Task: M4-08b (diagnostic split of the M4 failure investigation; spec 16/M4
"If it fails", 21.1–21.2).

## Pre-analysis declaration and budget

`manifests/m4_lock_localization.json` was written and fast-validated before
any M4-08b probe was executed. It reuses the frozen M3-08/M4-07 actor family
exactly (no new range), outer seeds 2 (focus) and 3 (contrast), and declares
three paired synthetic-drive probes plus a 60-outcome lock-in validity gate:

- dose_response: fresh learner, cue × scale {0, 0.5, 1, 2, 4}, 8 ticks;
- flip: locked-state clone, switch cue × scale {0, 1, 2, 4} (0 = zero-drive
  control), 8 noise seeds, 200-tick cap, readout polled per tick;
- converge: locked-then-cue-A vs fresh-then-cue-A with paired perturbation
  draws, distance at 8/16/32/64 ticks.

Pairing: flip clones share locked history and future noise draws, so leg
differences are pure drive effects; converge pairs share perturbation
draws, so distances are pure history effects. Budget: 76,768 learner
transitions maximum 100,000, serial release export into a fresh directory.
No acquisition criterion is set.

## Execution

Fast suite (debug):

```bash
cargo test --locked --test m4_lock_localization
```

Bounded export (release; target must not exist beforehand):

```bash
CRA_M4_LOCK_DIR=docs/evidence/m4-08b/probes \
  cargo test --release --locked --test m4_lock_localization \
  m4_lock_localization_export -- --ignored --exact --nocapture
```

The export ran once: exactly the declared 76,768 transitions (counter
completeness is asserted, and an incomplete counter failed loudly during
development before the fix), zero nonfinite states, same-tag lock rebuilds
bitwise identical. Magnitudes are in `probes/probes.json`.

## Result 1 — lock-in validity holds on all 16 lock-ins

Every lock-in (2 outers × 8 seeds) ends with its last 10 actions on 0,
reproducing the M4-08 600/600 closed-loop lock at 60-outcome scale.
Lock margins are +0.37..+0.96 (outer 2) and +0.17..+0.36 (outer 3).

## Result 2 — outer-2 cue drive pins the wrong action (dose + flip)

Fresh dose on outer 2: zero drive margin −0.17; 1× cue drive moves the
readout to +0.16 (cue 0) / +0.11 (cue 1), i.e. toward action 0 — while the
audit correctness labels show both birth mappings reward action 1. The
inherited input projection points the wrong way on this seed.

Flip on outer 2: zero drive escapes 8/8 (median 50 ticks — passive decay
plus noise suffices). Any cue drive at 1×/2×/4× on either cue: 0/8 flips,
with margin@40 growing in scale (+0.70 → +1.11 cue 0; +0.46 → +0.99
cue 1). Stronger ordinary drive means a deeper lock. Motor inertia alone
is escapable; the cue input actively prevents escape.

## Result 3 — outer-3 drive is aligned (contrast + control in one)

Flip on outer 3: zero drive escapes 8/8 (median 29.5). Cue-0 drive holds
action 0 on 8/8 legs at every scale (correct: cue 0 maps to 0). Cue-1
drive flips 8/8, ever faster with scale (median 14 → 6 → 3 ticks;
margin@40 −0.11 → −0.77). The probe follows drive direction rather than
favoring flips, and outer-3's projection agrees with its mappings. Fresh
dose agrees (cue 0 → +0.18; cue 1 → −0.06).

## Result 4 — history washes out 4–5× but plateaus (converge)

Locked-vs-fresh distance under continued cue drive falls from 0.79/0.83
at 8 ticks to 0.17 at 64 ticks (outer 2), and from 1.04/1.18 to 0.33/0.34
(outer 3). Recurrent activity mostly follows drive — there is no deep
attractor ignoring input — but a nonzero history offset persists at 64
ticks. History persistence is a secondary factor; drive direction is
primary.

## Interpretation and next step

The lock is localized to the **input-projection alignment**, not to motor
inertia (passively escapable) or a deep recurrent attractor (mostly
washout). On outer 2 the frozen `B` projection drives both cues toward the
wrong motor pool, and persistence removes the only escape path the
episodic condition has (unbiased reset states). Consequence for planning:
scaling `input_scale` up cannot fix a wrong-way projection — it deepens
the pin (flip margins grow with scale). The M4-level lever with headroom
is a bounded persistent-condition sweep (fixed-rule `eta` × `tau_e`, plus
`motor_filter_tau` as a declared candidate) testing whether stronger
updates can catch noise flips despite pinning; expectations should be
calibrated by Result 2. Selecting a family on persistent acquisition, or
adding exploration support, are larger scope changes needing their own
declared tasks. No rescue mechanism was introduced here; M4-07/M4-GATE
stay open. No validation or final-test seeds were inspected.

## Verification and artifacts

- Fast tests: 4 passed, 1 explicit empirical test ignored.
- Release export: 1 passed; 76,768/76,768 declared transitions, zero
  nonfinite states, 16/16 validity gates, bitwise rebuild identity.
- Full battery: `cargo test --all-targets --locked` (see ledger), 2
  compile-fail doc checks, 17 Python audit tests, committed fixture audit,
  `cargo fmt --all -- --check` and Clippy `-D warnings` clean,
  `git diff --check` clean.

SHA-256:

```text
dd8861dfdd41fe0447ca94f4a1655a9610ec7167980779185fca09446a061764  manifests/m4_lock_localization.json
c8c171a50d1ef33ee8b38c4c24a6a40d11888a73c8cc729c9303e3fbc244ba25  docs/evidence/m4-08b/probes/probes.json
```

## Claim limit

Development seeds (outers 2–3), tested locally plastic actor family,
synthetic ordinary inputs only. Dose/flip/converge magnitudes are
diagnostic characterizations of the frozen failure, not task performance
and not a tuning result.
