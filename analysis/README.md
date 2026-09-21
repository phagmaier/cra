# Analysis layer (offline, Python)

Pinned interpreter: Python `3.14.7` (`../.python-version`). Standard library only (see
`requirements.txt`) — `tomllib` reads the resolved config, `json` the
event streams. A notebook may explore results but must never be the only
executable record.

- `validate_logs.py <run_dir> [...]`: audits run/seed identity, complete
  lifetime lengths, configured timing, finite values, duplicate feedback,
  and hidden reward/change/exposure accounting. Exit 0 when clean.
  Runs with event logging disabled get explicitly labeled provenance and
  completion checks only; individual outcomes cannot be verified.
- `test_validate_logs.py`: unit tests over `fixtures/valid` (a real oracle
  smoke run), five deliberately corrupt variants, and systematic mutations
  for truncation, identity, missing fields, malformed types, and hidden truth. Run with
  `python3 analysis/test_validate_logs.py`.
- `aggregate.py` arrives with the comparison pipeline in M8 (NumPy /
  Matplotlib only when actually needed).
