#!/usr/bin/env python3
"""Tests for analysis/validate_logs.py (standard library only).

Run: python3 analysis/test_validate_logs.py
"""

import unittest
import json
import shutil
import tempfile
from pathlib import Path

import validate_logs

FIXTURES = Path(__file__).resolve().parent / "fixtures"


class TestAudit(unittest.TestCase):
    def mutated_run(self, mutate):
        with tempfile.TemporaryDirectory() as tmp:
            run = Path(tmp) / "run"
            shutil.copytree(FIXTURES / "valid", run)
            mutate(run)
            return validate_logs.audit_run(run)

    @staticmethod
    def change_json(run, name, mutate):
        path = run / name
        value = json.loads(path.read_text())
        mutate(value)
        path.write_text(json.dumps(value))

    @staticmethod
    def change_rows(run, name, mutate):
        path = run / name
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        mutate(rows)
        path.write_text("".join(json.dumps(row) + "\n" for row in rows))

    def test_truncated_and_empty_runs_cannot_self_certify_completion(self):
        for size in (0, 7):
            with self.subTest(size=size):
                def mutate(run):
                    for name in ("events.jsonl", "hidden.jsonl"):
                        self.change_rows(run, name, lambda rows: rows.__delitem__(slice(size, None)))
                    self.change_json(run, "completion.json", lambda c: c.update(commitments=size, outcomes=size))
                self.assertTrue(self.mutated_run(mutate))

    def test_identity_and_declared_counts_are_checked(self):
        for name, key, value in (
            ("completion.json", "lifetimes_completed", 0),
            ("condition.json", "lifetimes", 2),
            ("condition.json", "outcomes_per_lifetime", 7),
            ("condition.json", "policy", "constant-invalid"),
            ("condition.json", "profile_name", "wrong"),
            ("condition.json", "event_log", False),
            ("seed_streams.json", "streams", []),
        ):
            with self.subTest(name=name, key=key):
                self.assertTrue(self.mutated_run(lambda p: self.change_json(p, name, lambda v: v.update({key: value}))))

    def test_ordinary_corruptions_are_rejected(self):
        for key, value in (
            ("run_id", "wrong"), ("condition_id", "B0"), ("namespace", "training"),
            ("outer_seed", 9), ("lifetime_index", 1), ("cue_index", -1),
            ("commit_tick", None), ("outcome_tick", "17"), ("event_id", -1),
            ("action", True), ("reward", True), ("reward", None),
            ("target_at_commit", 1),
        ):
            with self.subTest(key=key, value=value):
                self.assertTrue(self.mutated_run(lambda p: self.change_rows(p, "events.jsonl", lambda rows: rows[0].update({key: value}))))

    def test_hidden_corruptions_are_rejected(self):
        for key, value in (
            ("cue_id", 0), ("target_at_commit", 0), ("latent_correctness", False),
            ("noise_bit", True), ("epsilon", float("nan")), ("cue_hazard", -1),
            ("cue_exposure_index", 2), ("hidden_change_before_presentation", True),
            ("commit_tick", 14), ("outcome_tick", 17), ("stable_or_volatile", "unknown"),
        ):
            with self.subTest(key=key):
                self.assertTrue(self.mutated_run(lambda p: self.change_rows(p, "hidden.jsonl", lambda rows: rows[0].update({key: value}))))

    def test_jointly_tampered_timing_is_rejected(self):
        def mutate(run):
            for name in ("events.jsonl", "hidden.jsonl"):
                self.change_rows(run, name, lambda rows: rows[0].update(outcome_tick=18))
        self.assertTrue(self.mutated_run(mutate))

    def test_malformed_records_produce_errors_not_exceptions(self):
        for name, text in (
            ("manifest.json", "[]"), ("manifest.json", '{"seeds": null}'),
            ("manifest.json", '{"schema_version": 1, "schema_version": 1}'),
            ("completion.json", "null"), ("events.jsonl", "null\n"),
            ("events.jsonl", "{}\n"), ("hidden.jsonl", "[]\n"),
            ("resolved_config.toml", "seeds = 0\n"),
        ):
            with self.subTest(name=name, text=text):
                self.assertTrue(self.mutated_run(lambda p: (p / name).write_text(text)))

    def test_missing_seed_streams_is_rejected(self):
        self.assertTrue(self.mutated_run(lambda p: (p / "seed_streams.json").unlink()))

    def test_logging_disabled_allows_only_provenance_audit(self):
        def mutate(run):
            for name in ("events.jsonl", "hidden.jsonl"):
                (run / name).unlink()
            self.change_json(run, "condition.json", lambda c: c.update(event_log=False))
            path = run / "resolved_config.toml"
            path.write_text(path.read_text().replace("event_log = true", "event_log = false"))
        self.assertEqual(self.mutated_run(mutate), [])

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
