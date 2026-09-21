#!/usr/bin/env python3
"""Audit M0 provenance, complete lifetimes, timing, and hidden reward accounting.

Standard library only. With event_log=false only provenance/completion can
be audited; the CLI labels that reduced coverage explicitly.
"""

import hashlib
import json
import math
import sys
import tomllib
from pathlib import Path

EVENT_SCHEMA_VERSION = 1
NAMESPACES = ("development", "training", "validation", "final_test")
LEGACY_STREAMS = ("cue_order", "mapping_init", "mapping_change", "reward_noise",
                  "timing", "actor_noise", "tie_break", "init", "evolution")
STREAMS = ("cue_order", "cue_membership", "mapping_init", "mapping_change",
           "reward_noise", "timing", "actor_noise", "tie_break", "init", "evolution")
EVENT_INTS = ("schema_version", "outer_seed", "lifetime_index", "choice_index",
              "event_id", "cue_index", "commit_tick", "outcome_tick", "action")
HIDDEN_INTS = ("event_id", "choice_index", "cue_id", "target_at_commit",
               "cue_exposure_index", "commit_tick", "outcome_tick")
HIDDEN_BOOLS = ("latent_correctness", "noise_bit", "hidden_change_before_presentation")


def uint(value):
    return type(value) is int and 0 <= value <= 2**64 - 1


def number(value, lo, hi):
    return type(value) in (int, float) and lo <= value <= hi and math.isfinite(value)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def parse_object(text):
    result = json.loads(text, object_pairs_hook=unique_object)
    if not isinstance(result, dict):
        raise ValueError("expected a JSON object")
    def finite(value, path):
        if isinstance(value, float) and not math.isfinite(value):
            raise ValueError(f"nonfinite value at {path}")
        if isinstance(value, dict):
            for key, item in value.items():
                finite(item, f"{path}.{key}")
        elif isinstance(value, list):
            for index, item in enumerate(value):
                finite(item, f"{path}[{index}]")
    finite(result, "record")
    return result


def load_json(path, errors, label):
    try:
        return parse_object(path.read_text())
    except (OSError, UnicodeError, ValueError) as exc:
        errors.append(f"cannot read '{label}': {exc}")
    return None


def read_jsonl(path, errors, label):
    try:
        lines = path.read_text().splitlines()
    except (OSError, UnicodeError) as exc:
        errors.append(f"cannot read '{label}': {exc}")
        return None
    records = []
    for lineno, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            records.append(parse_object(line))
        except ValueError as exc:
            errors.append(f"'{label}' line {lineno}: {exc}")
    return records


def require(record, validators, label, errors):
    """Validate before arithmetic/indexing, so malformed logs never crash the audit."""
    valid = True
    for key, check in validators.items():
        if not check(record.get(key)):
            errors.append(f"{label}: invalid or missing {key}: {record.get(key)!r}")
            valid = False
    return valid


def audit_run(run_dir):
    errors = []
    run_dir = Path(run_dir)
    manifest = load_json(run_dir / "manifest.json", errors, "manifest.json")
    condition = load_json(run_dir / "condition.json", errors, "condition.json")
    completion = load_json(run_dir / "completion.json", errors, "completion.json")
    streams = load_json(run_dir / "seed_streams.json", errors, "seed_streams.json")
    try:
        resolved = tomllib.loads((run_dir / "resolved_config.toml").read_text())
    except (OSError, UnicodeError, ValueError) as exc:
        errors.append(f"cannot read 'resolved_config.toml': {exc}")
        resolved = None
    if any(r is None for r in (manifest, condition, completion, streams, resolved)):
        return errors

    text = lambda x: isinstance(x, str) and bool(x)
    positive = lambda x: uint(x) and x > 0
    ok = require(manifest, {"schema_version": lambda x: type(x) is int and x == 1,
                           "run_id": text, "profile_name": text, "condition_id": text,
                           "seeds": lambda x: isinstance(x, dict)}, "manifest.json", errors)
    ok &= require(condition, {"condition_id": text, "profile_name": text, "policy": text,
                              "lifetimes": positive, "outcomes_per_lifetime": positive,
                              "event_log": lambda x: type(x) is bool}, "condition.json", errors)
    ok &= require(completion, {"run_id": text, "lifetimes_completed": uint,
                               "commitments": uint, "outcomes": uint}, "completion.json", errors)
    for section in ("seeds", "simulation", "environment", "logging"):
        if not isinstance(resolved.get(section), dict):
            errors.append(f"resolved_config.toml: missing/invalid section {section}")
            ok = False
    if not ok:
        return errors
    seeds, env = manifest["seeds"], resolved["environment"]
    ok = require(seeds, {"namespace": lambda x: x in NAMESPACES,
                         "root_seed": uint, "outer_seed": uint}, "manifest seeds", errors)
    ok &= require(env, {"cue_count": positive, "cue_ticks": positive, "response_ticks": positive},
                  "resolved environment", errors)
    ok &= require(resolved["simulation"], {"warmup_ticks": uint, "outcomes_per_lifetime": positive},
                  "resolved simulation", errors)
    for key in ("quiet_ticks", "memory_gap_ticks", "reward_delay_ticks"):
        value = env.get(key)
        if not (isinstance(value, list) and len(value) == 2 and all(uint(x) for x in value)
                and value[0] <= value[1] and (key != "reward_delay_ticks" or value[0] >= 1)):
            errors.append(f"resolved environment: invalid {key}")
            ok = False
    if not ok:
        return errors

    for key in ("namespace", "root_seed", "outer_seed"):
        if resolved["seeds"].get(key) != seeds[key] or streams.get(key) != seeds[key]:
            errors.append(f"seed mismatch for {key}")
    if streams.get("lifetime_index") != 0:
        errors.append("seed_streams.json: expected lifetime_index 0")
    stream_schema = streams.get("schema_version", 1)
    if type(stream_schema) is not int:
        errors.append(f"seed_streams.json: invalid schema_version {stream_schema!r}")
        stream_names = STREAMS
    elif stream_schema == 1:
        stream_names = LEGACY_STREAMS
    elif stream_schema == 2:
        stream_names = STREAMS
    else:
        errors.append(f"seed_streams.json: unsupported schema_version {stream_schema!r}")
        stream_names = STREAMS
    expected_streams = []
    for stream in stream_names:
        canonical = (f"cra-v1|root={seeds['root_seed']}|ns={seeds['namespace']}"
                     f"|outer={seeds['outer_seed']}|lifetime=0|stream={stream}")
        expected_streams.append({"stream": stream, "seed_hex": hashlib.sha256(canonical.encode()).hexdigest()})
    if streams.get("streams") != expected_streams:
        errors.append("seed_streams.json: derived stream table mismatch")
    for key in ("condition_id", "profile_name"):
        if condition[key] != manifest[key]:
            errors.append(f"condition.json: {key} disagrees with manifest")
    if resolved.get("profile_name") != manifest["profile_name"] or resolved.get("schema_version") != 1:
        errors.append("resolved config profile/schema disagrees with manifest")
    policy_ids = {"random": "B0", "constant-0": "B1", "constant-1": "B1",
                  "actor-no-learning": "B3", "oracle": "O1"}
    if policy_ids.get(condition["policy"]) != condition["condition_id"]:
        errors.append("condition.json: policy/condition mismatch")
    if condition["outcomes_per_lifetime"] != resolved["simulation"]["outcomes_per_lifetime"]:
        errors.append("condition.json: outcomes_per_lifetime disagrees with config")
    if condition["event_log"] != resolved["logging"].get("event_log"):
        errors.append("condition.json: event_log disagrees with config")
    if completion.get("status") != "completed":
        errors.append("completion.json: status is not completed (interrupted runs are not successful)")
    if completion["run_id"] != manifest["run_id"]:
        errors.append("completion.json: run_id disagrees with manifest")
    if completion["lifetimes_completed"] != condition["lifetimes"]:
        errors.append("completion.json: incorrect number of complete lifetimes")
    expected = condition["lifetimes"] * condition["outcomes_per_lifetime"]
    if completion["commitments"] != expected or completion["outcomes"] != expected:
        errors.append("completion.json: counts disagree with declared lifetime lengths")

    if condition["event_log"]:
        events = read_jsonl(run_dir / "events.jsonl", errors, "events.jsonl")
        hidden = read_jsonl(run_dir / "hidden.jsonl", errors, "hidden.jsonl")
        if events is not None and hidden is not None:
            if len(events) != expected:
                errors.append("events.jsonl: count disagrees with declared lifetime lengths")
            audit_streams(events, hidden, env["cue_count"], errors, manifest, condition, resolved)
    elif any((run_dir / name).exists() for name in ("events.jsonl", "hidden.jsonl")):
        errors.append("event logs present despite event_log=false")
    return errors


def audit_streams(events, hidden, cue_count, errors, manifest=None, condition=None, resolved=None):
    seen = set()
    for event in events:
        key = (event.get("lifetime_index"), event.get("event_id"))
        if all(uint(x) for x in key):
            if key in seen:
                errors.append(f"duplicate event_id {key[1]} in lifetime {key[0]}")
            seen.add(key)
    if len(events) != len(hidden):
        errors.append(f"hidden stream has {len(hidden)} records for {len(events)} events")
    previous = None
    exposures = {}
    for position, (event, annotation) in enumerate(zip(events, hidden)):
        where = f"event row {position}"
        validators = {key: uint for key in EVENT_INTS}
        validators.update({key: lambda x: isinstance(x, str) and bool(x)
                           for key in ("run_id", "condition_id", "namespace")})
        validators["reward"] = lambda x: number(x, 0, 1) and x in (0, 1)
        ok = require(event, validators, where, errors)
        allowed = set(validators)
        if set(event) != allowed:
            errors.append(f"{where}: unexpected/missing ordinary fields")
        validators = {key: uint for key in HIDDEN_INTS}
        validators.update({key: lambda x: type(x) is bool for key in HIDDEN_BOOLS})
        validators.update({"epsilon": lambda x: number(x, 0, 0.5),
                           "cue_hazard": lambda x: number(x, 0, 1),
                           "stable_or_volatile": lambda x: x in ("stable", "volatile")})
        ok &= require(annotation, validators, f"hidden row {position}", errors)
        if not ok:
            continue
        if event["schema_version"] != EVENT_SCHEMA_VERSION or event["action"] > 1 or event["cue_index"] >= cue_count:
            errors.append(f"{where}: invalid schema, action, or cue_index")
        if manifest:
            for key in ("run_id", "condition_id"):
                if event[key] != manifest[key]:
                    errors.append(f"{where}: {key} identity mismatch")
            for key in ("namespace", "outer_seed"):
                if event[key] != manifest["seeds"][key]:
                    errors.append(f"{where}: {key} identity mismatch")
        if condition:
            lifetime, choice = divmod(position, condition["outcomes_per_lifetime"])
            if event["lifetime_index"] != lifetime or event["choice_index"] != choice:
                errors.append(f"{where}: lifetime/choice indices not contiguous")
            policy = condition["policy"]
            if policy in ("constant-0", "constant-1") and event["action"] != int(policy[-1]):
                errors.append(f"{where}: constant policy action mismatch")
            if policy == "oracle" and not annotation["latent_correctness"]:
                errors.append(f"{where}: oracle is not latently correct")
        new_lifetime = previous is None or event["lifetime_index"] != previous["lifetime_index"]
        if new_lifetime:
            exposures = {}
            if event["choice_index"] != 0:
                errors.append(f"{where}: choice_index not contiguous from zero")
        else:
            if event["event_id"] == previous["event_id"]:
                errors.append(f"{where}: duplicate event_id")
            elif event["event_id"] < previous["event_id"]:
                errors.append(f"{where}: event ids not increasing")
            if event["commit_tick"] <= previous["outcome_tick"]:
                errors.append(f"{where}: overlapping choices or ticks not increasing")
        if event["commit_tick"] >= event["outcome_tick"]:
            errors.append(f"{where}: commit_tick must precede outcome_tick")
        if resolved:
            env = resolved["environment"]
            delay = event["outcome_tick"] - event["commit_tick"]
            if not env["reward_delay_ticks"][0] <= delay <= env["reward_delay_ticks"][1]:
                errors.append(f"{where}: reward delay outside configured range")
            quiet = ([resolved["simulation"]["warmup_ticks"]] * 2 if new_lifetime else env["quiet_ticks"])
            base = -1 if new_lifetime else previous["outcome_tick"]
            span = event["commit_tick"] - base
            bounds = [quiet[i] + env["cue_ticks"] + env["memory_gap_ticks"][i] + env["response_ticks"] for i in (0, 1)]
            if not bounds[0] <= span <= bounds[1]:
                errors.append(f"{where}: commitment timing outside configured range")
        joins = (("event_id", "event_id"), ("choice_index", "choice_index"),
                 ("cue_index", "cue_id"), ("commit_tick", "commit_tick"), ("outcome_tick", "outcome_tick"))
        if any(event[a] != annotation[b] for a, b in joins):
            errors.append(f"{where}: hidden join mismatch")
        correct = event["action"] == annotation["target_at_commit"]
        if (annotation["target_at_commit"] > 1 or correct != annotation["latent_correctness"]
                or event["reward"] != int(correct ^ annotation["noise_bit"])):
            errors.append(f"{where}: hidden reward/correctness mismatch")
        cue = annotation["cue_id"]
        old = exposures.get(cue)
        exposure = 1 if old is None else old["cue_exposure_index"] + 1
        changed = annotation["hidden_change_before_presentation"]
        hazard = annotation["cue_hazard"]
        if (annotation["cue_exposure_index"] != exposure
                or (changed and (old is None or hazard == 0))
                or (old is not None and hazard == 1 and not changed)
                or (annotation["stable_or_volatile"] == "stable" and hazard != 0)
                or (annotation["epsilon"] == 0 and annotation["noise_bit"])):
            errors.append(f"{where}: invalid hidden exposure/change/noise accounting")
        if old is not None:
            if any(annotation[key] != old[key] for key in ("epsilon", "cue_hazard", "stable_or_volatile")):
                errors.append(f"{where}: hidden birth parameters changed")
            if (annotation["target_at_commit"] != old["target_at_commit"]) != changed:
                errors.append(f"{where}: hidden mapping/change mismatch")
        exposures[cue] = annotation
        previous = event


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
            condition = json.loads((Path(raw) / "condition.json").read_text())
            coverage = "events + provenance" if condition["event_log"] else "provenance/completion only; event logging disabled"
            print(f"OK {raw} ({coverage})")
    return int(failed)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
