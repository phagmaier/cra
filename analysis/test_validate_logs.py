#!/usr/bin/env python3
"""Tests for analysis/validate_logs.py (standard library only).

Run: python3 analysis/test_validate_logs.py
"""

import unittest
from pathlib import Path

import validate_logs

FIXTURES = Path(__file__).resolve().parent / "fixtures"


class TestAudit(unittest.TestCase):
    def test_valid_run_is_clean(self):
        self.assertEqual(validate_logs.audit_run(FIXTURES / "valid"), [])

    def test_duplicate_event_detected(self):
        errors = validate_logs.audit_run(FIXTURES / "corrupt_dup_event")
        self.assertTrue(any("duplicate event_id" in e for e in errors), errors)

    def test_bad_order_detected(self):
        errors = validate_logs.audit_run(FIXTURES / "corrupt_bad_order")
        self.assertTrue(
            any("contiguous" in e or "increasing" in e for e in errors), errors
        )

    def test_nonfinite_reward_detected(self):
        errors = validate_logs.audit_run(FIXTURES / "corrupt_nonfinite")
        self.assertTrue(any("reward" in e for e in errors), errors)

    def test_missing_completion_detected(self):
        errors = validate_logs.audit_run(FIXTURES / "corrupt_missing_completion")
        self.assertTrue(any("completion" in e for e in errors), errors)

    def test_hidden_mismatch_detected(self):
        errors = validate_logs.audit_run(FIXTURES / "corrupt_hidden_mismatch")
        self.assertTrue(any("hidden" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
