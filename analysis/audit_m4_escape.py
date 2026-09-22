"""Read-only integrity audit of the M4-08c export; no simulator or selection tuning."""

import hashlib
import json
import math
from collections import defaultdict
from pathlib import Path
import sys


def audit(directory):
    directory = Path(directory)
    verdict = json.loads((directory / "verdict.json").read_text())
    provenance = json.loads((directory / "provenance.json").read_text())
    plan = provenance["plan"]
    assert not (directory / "FAILED.json").exists()
    assert verdict["integrity_passes"] and verdict["failures"] == []
    assert verdict["plan_sha256"] == provenance["plan_sha256"]
    for name in ("records", "series"):
        assert hashlib.sha256((directory / f"{name}.jsonl").read_bytes()).hexdigest() == verdict[f"{name}_sha256"]
    records = [json.loads(line) for line in (directory / "records.jsonl").read_text().splitlines()]
    groups = defaultdict(list)
    with (directory / "series.jsonl").open() as stream:
        for line in stream:
            row = json.loads(line)
            assert all(not isinstance(v, float) or math.isfinite(v) for v in row.values())
            groups[row["grid_index"], row["outer_seed"]].append(row)
    expected = {(point, outer) for point in [None, *range(12)] for outer in [1, 2, 3]}
    assert set(groups) == expected
    assert len(records) == len(groups) == plan["budget"]["total_lifetimes"]
    assert sum(map(len, groups.values())) == plan["budget"]["total_outcomes"]
    keys = [(r["grid_index"], r["m4_07_record"]["outer_seed"]) for r in records]
    assert len(set(keys)) == len(keys) and set(keys) == expected
    points = {p["grid_index"]: p for p in verdict["points"]}
    assert set(points) == set(range(12))
    for record in records:
        summary = record["m4_07_record"]
        key = record["grid_index"], summary["outer_seed"]
        rows = groups[key]
        assert len(rows) == summary["outcomes"] == 2000
        assert summary["resets"] == [0] and summary["status"] == "complete"
        assert summary["ticks"] == rows[-1]["feedback_tick"] + 1
        assert len({r["event_id"] for r in rows}) == len(rows)
        assert all(a["event_id"] < b["event_id"] for a, b in zip(rows, rows[1:]))
        assert all(r["reward"] == float(r["correct"]) and not r["noise_bit"] for r in rows)
        late = []
        for cue, stats in enumerate(summary["behavior"]["per_cue"]):
            cue_rows = [r for r in rows if r["cue"] == cue]
            assert len(cue_rows) == stats["exposures"] >= 200
            for label, window in [("early", cue_rows[:100]), ("late", cue_rows[-100:]), ("full", cue_rows)]:
                accuracy = sum(r["correct"] for r in window) / len(window)
                assert accuracy == stats[f"{label}_accuracy"] == stats[f"{label}_reward"]
            late.extend(cue_rows[-100:])
        accuracy = sum(s["late_accuracy"] for s in summary["behavior"]["per_cue"]) / 2
        assert accuracy == summary["behavior"]["late_macro_accuracy"]
        if key[0] is None:
            continue
        for row, control in zip(rows, groups[None, key[1]]):
            for field in ("event_id", "cue", "noise_bit", "commit_tick", "feedback_tick"):
                assert row[field] == control[field]
            assert row["action"] ^ int(not row["correct"]) == control["action"] ^ int(not control["correct"])
            assert row["delta"] == row["reward"] - row["baseline_old"]
            assert math.isclose(row["raw_l1"], record["eta"] * abs(row["delta"]) * row["eligibility_l1"], rel_tol=1e-12, abs_tol=1e-12)
        mechanism = record["mechanism"]
        assert [sum(r["action"] == action for r in rows) for action in (0, 1)] == mechanism["action_counts"]
        assert [sum(r["action"] == action for r in late) for action in (0, 1)] == mechanism["late_action_counts"]
        for field, reported in [("delta", "late_mean_abs_delta"), ("eligibility_l1", "late_mean_eligibility_l1"), ("actual_l1", "late_mean_actual_l1")]:
            assert math.isclose(math.fsum(abs(r[field]) for r in late) / len(late), mechanism[reported], rel_tol=1e-12, abs_tol=1e-30)
        seed = next(s for s in points[key[0]]["seeds"] if s["outer_seed"] == key[1])
        assert seed["late_accuracy"] == accuracy and seed["mechanism"] == mechanism
        control_rows = groups[None, key[1]]
        control_accuracy = sum(sum(r["correct"] for r in [x for x in control_rows if x["cue"] == cue][-100:]) / 100 for cue in (0, 1)) / 2
        assert seed["b3_late_accuracy"] == control_accuracy
        assert seed["margin"] == accuracy - control_accuracy
        c, u = verdict["criterion"], summary["updates"]
        passes = (accuracy >= c["min_continuous_late_accuracy"]
                  and seed["margin"] >= c["min_continuous_minus_matched_b3"]
                  and u["clipped_update_fraction"] <= c["max_clipped_update_fraction"]
                  and u["plastic_bound_occupancy"] <= c["max_plastic_bound_occupancy"]
                  and u["final_p_l1"] > 0)
        assert seed["passes"] == passes
    for point in points.values():
        count = sum(s["passes"] for s in point["seeds"])
        assert point["seeds_passing"] == count
        assert point["criterion_passes"] == (count >= 2)
        assert point["eligible"] == (count >= 2 and all(s["mechanism"]["screen_passes"] for s in point["seeds"] if s["passes"]))
    ranked = sorted((p for p in points.values() if p["eligible"]), key=lambda p: (-p["seeds_passing"], -p["minimum_margin"], p["departure_distance"], p["grid_index"]))
    assert verdict["selected_grid_index"] == (ranked[0]["grid_index"] if ranked else None)
    assert verdict["measured_budget"] == {"lifetimes": len(records), "outcomes": sum(map(len, groups.values())), "ticks": sum(r["m4_07_record"]["ticks"] for r in records)}
    assert verdict["measured_budget"]["ticks"] <= plan["budget"]["maximum_total_ticks"]
    print(f"OK {directory}: 39 lifetimes, 78000 scalar rows, per-cue windows, pairing, raw identity, hashes, verdicts")


if __name__ == "__main__":
    audit(sys.argv[1])
