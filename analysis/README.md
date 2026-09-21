# Analysis layer (offline, Python)

Pinned interpreter: Python `3.14.7`. Standard library only (see
`requirements.txt`) — `tomllib` reads the resolved config, `json` the
event streams. A notebook may explore results but must never be the only
executable record.

- `validate_logs.py <run_dir> [...]`: audits run identity, per-lifetime
  event order/counts, finite 0/1 rewards, duplicate feedback, the
  hidden-stream join, and the completion record. Exit 0 when clean.
- `test_validate_logs.py`: unit tests over `fixtures/valid` (a real oracle
  smoke run) and five deliberately corrupt variants. Run with
  `python3 analysis/test_validate_logs.py`.
- `aggregate.py` arrives with the comparison pipeline in M8 (NumPy /
  Matplotlib only when actually needed).
