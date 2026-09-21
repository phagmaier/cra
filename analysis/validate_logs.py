#!/usr/bin/env python3
"""Offline audit for M0 run directories (standard library only).

Checks manifest/config/condition identity, per-lifetime event order and
counts, finite 0/1 rewards, duplicate feedback, the hidden-stream join,
and the terminal completion record. Later-stage quantities (motor
margins, eligibility norms, gate statistics) are absent at M0 and must
not be fabricated: the audit only requires M0-schema fields.

Usage: python3 analysis/validate_logs.py <run_dir> [<run_dir> ...]
Exit 0 when every run is clean, 1 otherwise.
"""

import json
import math
import sys
import tomllib
from pathlib import Path

EVENT_SCHEMA_VERSION = 1
NAMESPACES = ("development", "training", "validation", "final_test")


def load_json(path, errors, label):
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        errors.append(f"missing required file '{label}'")
    except json.JSONDecodeError as exc:
        errors.append(f"cannot parse '{label}': {exc}")
    return None


def audit_run(run_dir):
    """Return a list of error strings (empty when the run is clean)."""
    errors = []
    run_dir = Path(run_dir)

    manifest = load_json(run_dir / "manifest.json", errors, "manifest.json")
    if manifest is not None:
        for key in ("run_id", "profile_name", "condition_id", "seeds"):
            if key not in manifest:
                errors.append(f"manifest.json: missing key '{key}'")
        if manifest.get("schema_version") != 1:
            errors.append("manifest.json: schema_version must be 1")
        seeds = manifest.get("seeds", {})
        if seeds.get("namespace") not in NAMESPACES:
            errors.append(f"manifest.json: bad seed namespace {seeds.get('namespace')!r}")

    try:
        resolved = tomllib.loads((run_dir / "resolved_config.toml").read_text())
    except FileNotFoundError:
        errors.append("missing required file 'resolved_config.toml'")
        resolved = None
    except tomllib.TOMLDecodeError as exc:
        errors.append(f"cannot parse 'resolved_config.toml': {exc}")
        resolved = None

    cue_count = None
    if resolved is not None:
        try:
            rseeds = resolved["seeds"]
            cue_count = resolved["environment"]["cue_count"]
            if manifest is not None:
                for key in ("namespace", "root_seed", "outer_seed"):
                    if rseeds.get(key) != manifest["seeds"].get(key):
                        errors.append(
                            f"seed mismatch for '{key}': manifest "
                            f"{manifest['seeds'].get(key)!r} vs resolved config "
                            f"{rseeds.get(key)!r}"
                        )
        except KeyError as exc:
            errors.append(f"resolved_config.toml: missing section {exc}")

    condition = load_json(run_dir / "condition.json", errors, "condition.json")
    if condition is not None and manifest is not None:
        if condition.get("condition_id") != manifest.get("condition_id"):
            errors.append("condition.json: condition_id disagrees with manifest")

    events = read_jsonl(run_dir / "events.jsonl", errors, "events.jsonl")
    hidden = read_jsonl(run_dir / "hidden.jsonl", errors, "hidden.jsonl")
    if events is not None and hidden is not None:
        audit_streams(events, hidden, cue_count, errors)

    completion = load_json(run_dir / "completion.json", errors, "completion.json")
    if completion is not None:
        if completion.get("status") != "completed":
            errors.append(
                f"completion.json: status is {completion.get('status')!r}, "
                "not 'completed' (interrupted runs are not successful runs)"
            )
        if events is not None:
            for key in ("commitments", "outcomes"):
                if completion.get(key) != len(events):
                    errors.append(
                        f"completion.json: {key}={completion.get(key)!r} "
                        f"disagrees with {len(events)} logged events"
                    )
        if manifest is not None and completion.get("run_id") != manifest.get("run_id"):
            errors.append("completion.json: run_id disagrees with manifest")
    return errors


def read_jsonl(path, errors, label):
    try:
        text = path.read_text()
    except FileNotFoundError:
        errors.append(f"missing required file '{label}'")
        return None
    records = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        if not line.strip():
            continue
        try:
            records.append(json.loads(line))
        except json.JSONDecodeError as exc:
            errors.append(f"'{label}' line {lineno}: cannot parse: {exc}")
    return records


def audit_streams(events, hidden, cue_count, errors):
    # M0 event ids restart per lifetime: validate each contiguous
    # lifetime block as its own stream.
    blocks, current = [], []
    for event in events:
        if current and event.get("lifetime_index") != current[-1].get("lifetime_index"):
            blocks.append(current)
            current = []
        current.append(event)
    if current:
        blocks.append(current)
    for block in blocks:
        audit_block(block, cue_count, errors)
    if len(hidden) != len(events):
        errors.append(
            f"hidden stream has {len(hidden)} records for {len(events)} events"
        )
        return
    for event, annotation in zip(events, hidden):
        if (
            annotation.get("event_id") != event.get("event_id")
            or annotation.get("choice_index") != event.get("choice_index")
        ):
            errors.append(
                f"hidden join mismatch at choice {event.get('choice_index')}: "
                f"event ({event.get('event_id')}, {event.get('choice_index')}) vs "
                f"hidden ({annotation.get('event_id')}, "
                f"{annotation.get('choice_index')})"
            )


def audit_block(block, cue_count, errors):
    for position, event in enumerate(block):
        where = f"lifetime {event.get('lifetime_index')} choice {event.get('choice_index')}"
        if event.get("schema_version") != EVENT_SCHEMA_VERSION:
            errors.append(f"{where}: bad schema_version {event.get('schema_version')!r}")
        if event.get("choice_index") != position:
            errors.append(
                f"{where}: choice_index not contiguous at block position {position}"
            )
        if position > 0:
            prev = block[position - 1]
            if event.get("event_id") == prev.get("event_id"):
                errors.append(f"{where}: duplicate event_id {event.get('event_id')}")
            elif event.get("event_id", -1) < prev.get("event_id", -1):
                errors.append(f"{where}: event ids not strictly increasing")
            if event.get("outcome_tick", -1) <= prev.get("outcome_tick", -1):
                errors.append(f"{where}: outcome ticks not strictly increasing")
        reward = event.get("reward")
        if not isinstance(reward, (int, float)) or not math.isfinite(reward) or reward not in (0, 1):
            errors.append(f"{where}: reward must be 0 or 1, found {reward!r}")
        if event.get("action") not in (0, 1):
            errors.append(f"{where}: action must be 0 or 1, found {event.get('action')!r}")
        if cue_count is not None and (
            not isinstance(event.get("cue_index"), int) or event.get("cue_index") >= cue_count
        ):
            errors.append(
                f"{where}: cue_index {event.get('cue_index')!r} out of range for "
                f"{cue_count} cues"
            )
        commit, outcome = event.get("commit_tick"), event.get("outcome_tick")
        if (
            isinstance(commit, int)
            and isinstance(outcome, int)
            and commit >= outcome
        ):
            errors.append(f"{where}: commit_tick {commit} must precede outcome_tick {outcome}")


def main(argv):
    if len(argv) < 2:
        print("usage: validate_logs.py <run_dir> [...]", file=sys.stderr)
        return 2
    failed = False
    for raw in argv[1:]:
        errors = audit_run(raw)
        if errors:
            failed = True
            print(f"FAIL {raw}")
            for error in errors:
                print(f"  - {error}")
        else:
            print(f"OK {raw}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
