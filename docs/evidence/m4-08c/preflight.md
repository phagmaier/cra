# M4-08c pre-execution decision — 2026-09-22 UTC

The owner asked Codex to choose the best path and proceed. This authorizes
the proposed bounded 39-lifetime experiment; no further approval is needed.
The earlier tracker approval hold is superseded by that instruction.

Choose the 12-point eta × tau_e stationary-clean sweep before changing
actor dynamics. It is small (1,327,092 ticks maximum), reuses the exact
M4-07 comparison, and tests the remaining joint fixed-plasticity setting
question at the frozen family. Existing M4-07 throughput (~0.098 seconds
per representative release lifetime) makes this a bounded diagnostic,
not a nominal search. No new production behavior or dependencies.

`manifests/m4_escape_sweep.json` freezes the plan before full execution.
The harness is a child module of `tests/m4_continuous_acquisition.rs` so it
can directly reuse the frozen configuration, window, health, and update
analyzers. Execute the replication anchor and B3 controls before new points;
any mismatch halts. Export to a fresh directory, preserving partial artifacts
and a failure record if execution fails. No automatic empirical reruns.

Corrections to the supplied report, declared before results:

- Reusing three development seeds answers a development selection question.
  A fixed absolute bar and deterministic replay do not remove selection bias
  or establish fresh-seed generalization. Any winner is provisional.
- B3 outer 3 has late accuracy 0.875. Its largest possible accuracy margin
  is 0.125, below the frozen 0.15 requirement. Both locked seeds 1 and 2
  must therefore pass. Preserve this criterion; disclose the ceiling.
- Synthetic drive probes support a wrong-way projection and stronger-drive
  pinning. They do not rule out weaker drive in the actual closed loop.
  Keep input and motor parameters fixed here to isolate eta/tau, not because
  their potential is disproved. Omitting tau_e 128 bounds this experiment;
  no monotonic impossibility claim follows from prior measurements.
- The archive can support exact equality of all saved aggregate fields,
  not full trajectory identity: M4-07 did not archive per-outcome trajectories.
- Mechanism screening uses both actions over the lifetime and late mean
  abs(delta) > 1e-12 on each passing seed. This descriptive, predeclared
  screen prevents automatic adoption without evidence of escape. It is not
  a universal criterion (a successful deterministic policy can also have a
  small teaching signal).

If none passes, adopt nothing and assemble a negative M4-09 bundle with a
design-escalation entry. M4-07 and M4-GATE remain open. No further tuning
is included in this experiment. All prior dirty M4-08/08b work is preserved.
