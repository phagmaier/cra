# Offline analysis

Use Python `3.14.7`, pinned in [`.python-version`](../.python-version).
The M0 audit uses only the standard library; [requirements.txt](requirements.txt)
therefore has no third-party dependencies. Add a dependency lock with the
first actual external dependency. Rust remains the authoritative simulator.

## Commands

Run from the repository root:

```bash
python3 analysis/test_validate_logs.py
python3 analysis/validate_logs.py analysis/fixtures/valid
```

For a new run, replace the fixture path with the directory printed by
`simulate`. Multiple run directories can be passed to one audit invocation.
Exit 0 means every supplied run passed its applicable checks; exit 1 means
at least one failed; exit 2 indicates missing CLI arguments.

## Coverage and limits

The audit checks run/seed identity, declared complete lifetime lengths,
configured timing, finite values, duplicate feedback, hidden joins, and
reward/change/exposure accounting. It rejects missing or unsuccessful
completion records. With event logging disabled, it explicitly reports
provenance/completion coverage only; individual outcomes are unavailable.

`test_validate_logs.py` covers the real `fixtures/valid` oracle run, five
stored corrupt variants, and additional mutation cases. Fixtures are
historical test data: create separate new fixtures for schema changes and
explain any changes to old expected behavior. Extend Rust validation and
Python audit coverage together when adding logged quantities.

Keep raw outputs immutable and derived analysis separate. Audit logs before
aggregating them. `aggregate.py` and statistical figures remain M8 work;
a notebook must not be the only executable analysis record.

See the [current handoff](../docs/handoff.md) and
[evidence guide](../docs/evidence/README.md) for continuation and saved-run
availability. The earlier M0 review's 14 passing Python tests are recorded
results, not a claim that every future checkout has already been verified.
