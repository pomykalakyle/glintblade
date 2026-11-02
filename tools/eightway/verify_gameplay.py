#!/usr/bin/env python3
"""Compare exact combat/physics/RNG receipts from frozen before/after builds."""
import argparse
import itertools
import json
from collections import Counter
from pathlib import Path


def compare(before, after):
    counts = Counter()
    differences = []
    checked = 0
    with before.open() as left, after.open() as right:
        for row, (a, b) in enumerate(itertools.zip_longest(left, right), 1):
            if a is None or b is None:
                differences.append({"row": row, "reason": "receipt count differs"})
                break
            a, b = json.loads(a), json.loads(b)
            checked += 1
            counts[a["actor"]] += 1
            if a != b and len(differences) < 20:
                differences.append({"row": row, "case": {k: a[k] for k in ("actor", "skin", "direction", "tick") if k in a},
                    "changed_fields": {k: {"before": a.get(k), "after": b.get(k)} for k in a.keys() | b.keys() if a.get(k) != b.get(k)}})
    return {"passed": not differences, "checked_ticks": checked,
            "ticks_by_actor": dict(counts), "first_differences": differences}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument("--tour-before", type=Path)
    parser.add_argument("--tour-after", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = compare(args.before, args.after)
    result["before"] = str(args.before.resolve())
    result["after"] = str(args.after.resolve())
    if args.tour_before and args.tour_after:
        before = json.loads(args.tour_before.read_text())
        after = json.loads(args.tour_after.read_text())
        keys = ["ticks", "scene", "victory_ready", "milestones", "visited_zones", "end_pos", "audio_events", "trace"]
        differences = [key for key in keys if before.get(key) != after.get(key)]
        result["tour"] = {"passed": not differences, "changed_fields": differences,
            "before_ticks": before["ticks"], "after_ticks": after["ticks"],
            "visited_zones": after["visited_zones"], "milestones": after["milestones"]}
        result["passed"] &= not differences
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: result[k] for k in ("passed", "checked_ticks", "ticks_by_actor")}))
    if result["first_differences"]:
        print(json.dumps(result["first_differences"][:2], indent=2))
    raise SystemExit(0 if result["passed"] else 1)


if __name__ == "__main__":
    main()
