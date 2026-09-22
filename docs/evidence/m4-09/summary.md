# M4-09 negative continuous-system bundle

2026-09-22 UTC. This packages the verified implementation and negative
development evidence; it does **not** satisfy the continuous-learning exit.
M4-09's successful-system handoff remains incomplete, as do M4-07/M4-GATE.

The main learner is a **heuristic online local system**. Restricted M2
score-gradient checks establish conditional arithmetic under their stated
assumptions, not an unbiased online lifetime gradient or convergence.

| Required evidence | Artifact and current limit |
| --- | --- |
| Exact tick order, no within-lifetime reset | [M4-01](../m4-01/summary.md), [M4-03](../m4-03/summary.md); every new persistent sweep record has reset audit `[0]` |
| Resolved persistent/control profiles | [M4-08c run](../m4-08c/run/provenance.json), `point_00.toml`–`point_11.toml` and `continuous_b3.toml` beside it |
| Several-seed acquisition and controls | [M4-07](../m4-07/summary.md) five-condition negative result; [M4-08c](../m4-08c/summary.md) all 12 eta/tau settings negative |
| Replay with learned state | [M4-06](../m4-06/summary.md); five continuous checkpoint integration tests passed again this session on their existing fixture, not all grid settings |
| Failure mechanism and traces | [M4-08](../m4-08/summary.md), [M4-08b](../m4-08b/summary.md), [78,000 scalar rows](../m4-08c/run/series.jsonl) |
| Exact historical reproduction | Three B4 anchor and three B3 aggregate records equal M4-07 archives field-for-field; no historical full-trajectory claim |
| Runnable continuous-clean experiment | Explicit bounded Rust test command below; production `simulate` still has no plastic continuous CLI route |

Inspect or audit existing evidence without rerunning:

```bash
python3 analysis/audit_m4_escape.py docs/evidence/m4-08c/run
cargo test --locked --test continuous_checkpoint
```

The implemented continuous-clean experiment command is:

```bash
CRA_M4_ESCAPE_DIR=runs/NEW_M4_ESCAPE_DIRECTORY cargo test --release --locked --test m4_continuous_acquisition escape::m4_escape_sweep_export -- --ignored --exact --nocapture
```

The destination must be new and its parent must exist. A future intentional
replication repeats development data; it is not independent confirmation.
This session executed the export once at the archived M4-08c destination.
No scientific result authorizes rerunning to seek a favorable outcome.

Disposition: nothing adopted. All learning-rate/trace settings retain
complete wrong-action locks on outers 1 and 2. The next research step is
a separately declared design investigation of persistent actor exploration,
starting with input-drive/noise balance while preserving the current learning
rule. The drive probes do not rule out weaker inputs, and the actor was
selected for episodic operation. Do not launch another eta/tau sweep or
add a new exploration mechanism implicitly. See the dated escalation in
[decisions](../../decisions.md); no additional design sweep ran here.
