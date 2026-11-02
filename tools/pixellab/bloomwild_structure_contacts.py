#!/usr/bin/env python3
"""Remove authored ground-support ink from Bloomwild masonry, entirely offline.

The profiles below are hand-selected native support contours, not PNG trimming
or a lowest-pixel rule. Each run copies adjacent material RGB at fixed source
coordinates. Alpha, transparent RGB, all other outlines, cells and metadata stay
exact. Arch undersides, wall undercuts and the aqueduct/altar are deliberately
outside these profiles. Appended short-column/wall variants use the identical
established shared-foot profiles.

Run after a scoped legacy export, or on the installed diversity atlas:
  uv run --with pillow python tools/pixellab/bloomwild_structure_contacts.py \
    --sprites assets/sprites --out /private/stage/sprites --report /private/audit.json

The normal atlas and JSON files are never written. Existing targets must be the
known ink color or their exact replacement, making this final stage idempotent
and refusing to silently repaint revised art.
"""

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
INK = (32, 39, 52)
OLD_GRAY_INK = (78, 88, 109)

# (target y, inclusive target x0/x1, donor y, donor x0). Donor runs
# advance horizontally with the target. None of the donors are patched pixels.
SHORT = [
    (48, 6, 6, 47, 7), (49, 7, 7, 48, 8), (50, 8, 9, 49, 8),
    (51, 10, 11, 50, 10), (52, 12, 14, 51, 12),
    (53, 15, 31, 52, 15), (52, 32, 34, 51, 32),
    (51, 35, 36, 50, 35), (50, 37, 38, 49, 35),
    (49, 39, 39, 48, 39), (48, 40, 40, 47, 39),
]
TALL = [
    (48, 6, 6, 47, 7), (49, 7, 8, 48, 8), (50, 9, 10, 49, 9),
    (51, 11, 12, 50, 11), (52, 13, 15, 51, 13),
    (53, 16, 26, 52, 16), (53, 27, 27, 52, 28), (53, 28, 30, 52, 28),
    (52, 31, 33, 51, 30), (51, 34, 35, 50, 34),
    (50, 36, 37, 49, 35), (49, 38, 39, 48, 38), (48, 40, 40, 47, 39),
]
PILLAR = [
    (49, 16, 18, 48, 17), (50, 19, 20, 49, 19), (51, 21, 24, 50, 21),
    (50, 25, 27, 49, 25), (50, 34, 37, 49, 34),
]
PILLAR_B = [
    (49, 17, 18, 48, 18), (50, 19, 20, 49, 19), (51, 21, 24, 50, 21),
    (50, 25, 27, 49, 25), (50, 35, 37, 49, 35), (51, 32, 32, 50, 31),
]
PILLAR_C = [
    (49, 17, 18, 48, 17), (50, 19, 20, 49, 19), (51, 21, 25, 50, 21),
    (50, 26, 27, 49, 26), (50, 35, 36, 49, 35),
]
LOW_WALL = [
    (52, 5, 10, 51, 5), (51, 11, 14, 50, 11), (50, 15, 26, 49, 15),
]
TALL_WALL = [
    (51, 3, 15, 50, 3), (51, 17, 18, 50, 17), (52, 16, 16, 51, 16),
    (52, 19, 21, 51, 19), (51, 22, 22, 50, 21),
    (50, 22, 23, 49, 22), (50, 24, 24, 49, 25), (50, 25, 36, 49, 25),
]
LEFT_POST = [
    (64, 28, 30, 63, 29), (64, 33, 34, 63, 32),
    (64, 37, 37, 63, 37), (64, 38, 38, 63, 37),
    (65, 26, 26, 64, 27), (65, 27, 27, 64, 27),
    (65, 30, 31, 64, 31), (65, 32, 32, 64, 32),
    (65, 35, 36, 64, 35), (66, 25, 25, 64, 27),
    (66, 33, 33, 65, 33), (66, 37, 37, 64, 36),
]
RIGHT_POST = [
    (58, 27, 28, 57, 28), (58, 38, 40, 57, 38),
    (59, 29, 30, 58, 29), (59, 31, 32, 58, 30),
    (59, 33, 34, 58, 36), (59, 36, 37, 58, 36),
    (60, 35, 35, 59, 35), (60, 38, 38, 59, 35),
]
PROFILES = {
    "bloomwild_ruins": {
        "pillar": PILLAR, "pillar_b": PILLAR_B, "pillar_c": PILLAR_C,
        "pillar_broken_short": SHORT, "pillar_broken_tall": TALL,
        "wall_low": LOW_WALL, "wall_tall": TALL_WALL,
        "drum": [(52, 13, 37, 51, 13)],
        "statue": [(53, 12, 35, 52, 12)],
    },
    "bloomwild_arch": {"arch_post_left": LEFT_POST, "arch_post_right": RIGHT_POST},
}


def profile_for(sheet, name):
    if sheet == "bloomwild_ruins":
        for template in ("pillar_broken_short", "wall_low"):
            if name.startswith(template + "_"):
                return PROFILES[sheet][template]
    return PROFILES.get(sheet, {}).get(name)


def correct_atlas(image, metadata, sheet):
    """Return corrected RGBA, exact allowed-pixel mask and a per-cell audit."""
    expected_cell = (48, 56) if sheet == "bloomwild_ruins" else (64, 72)
    if sheet not in PROFILES or tuple(metadata["cell"]) != expected_cell:
        raise ValueError(f"{sheet}: unexpected contact atlas/cell")
    image = image.convert("RGBA")
    original, corrected = image.copy(), image.copy()
    allowed = Image.new("L", image.size)
    rows, cells = set(), []
    for name, anim in metadata["anims"].items():
        profile = profile_for(sheet, name)
        if profile is None:
            continue
        row = anim["row"]
        if row in rows:
            raise ValueError(f"{sheet}/{name}: duplicate authored contact row")
        rows.add(row)
        y0 = row * expected_cell[1]
        edits = []
        targets = {(x, y) for y, x0, x1, _, _ in profile for x in range(x0, x1 + 1)}
        for y, x0, x1, sy, sx0 in profile:
            for x in range(x0, x1 + 1):
                sx = sx0 + x - x0
                if (sx, sy) in targets:
                    raise ValueError(f"{sheet}/{name}: donor is another contact target")
                before = original.getpixel((x, y0 + y))
                donor = original.getpixel((sx, y0 + sy))
                if before[3] != 255 or donor[3] != 255 or donor[:3] in (INK, OLD_GRAY_INK):
                    raise ValueError(f"{sheet}/{name}: revised support/donor at {x},{y}")
                replacement = donor[:3] + (before[3],)
                expected_ink = OLD_GRAY_INK if name in {"drum", "statue"} else INK
                if before[:3] not in (expected_ink, replacement[:3]):
                    raise ValueError(f"{sheet}/{name}: unexpected contact RGB at {x},{y}: {before}")
                allowed.putpixel((x, y0 + y), 255)
                corrected.putpixel((x, y0 + y), replacement)
                if before != replacement:
                    edits.append({"xy": [x, y], "atlas_xy": [x, y0 + y],
                                  "donor_xy": [sx, sy], "before": list(before[:3]),
                                  "after": list(replacement[:3])})
        cells.append({"animation": name, "row": row, "changed_pixels": len(edits), "edits": edits})
    if corrected.getchannel("A").tobytes() != original.getchannel("A").tobytes():
        raise AssertionError("contact correction changed alpha")
    before_bytes, after_bytes = original.tobytes(), corrected.tobytes()
    for i in range(image.width * image.height):
        before, after = before_bytes[i * 4:i * 4 + 4], after_bytes[i * 4:i * 4 + 4]
        if before != after and not allowed.getpixel((i % image.width, i // image.width)):
            raise AssertionError("contact correction escaped authored mask")
    return corrected, allowed, {"sheet": sheet, "cells": cells,
                                "changed_pixels": sum(c["changed_pixels"] for c in cells),
                                "alpha_unchanged": True, "outside_authored_mask_unchanged": True}


def export_contacts(sprites, out, names=tuple(PROFILES), report_path=None):
    """Write only selected albedo PNGs and optional evidence; no normal/JSON writes."""
    sprites, out = Path(sprites), Path(out)
    staged, report = [], {"schema": 1, "families": {}}
    for name in names:
        source = sprites / f"{name}.png"
        data = json.loads((sprites / f"{name}.json").read_text())
        corrected, mask, audit = correct_atlas(Image.open(source), data, name)
        audit["source_sha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
        staged.append((name, corrected, mask))
        report["families"][name] = audit
    out.mkdir(parents=True, exist_ok=True)
    for name, corrected, mask in staged:
        target = out / f"{name}.png"
        corrected.save(target)
        report["families"][name]["output_sha256"] = hashlib.sha256(target.read_bytes()).hexdigest()
        if report_path is not None:
            mask_dir = Path(report_path).parent / "contact-masks"
            mask_dir.mkdir(parents=True, exist_ok=True)
            mask.save(mask_dir / f"{name}.png")
    if report_path is not None:
        Path(report_path).parent.mkdir(parents=True, exist_ok=True)
        Path(report_path).write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sprites", type=Path, default=ROOT / "assets/sprites")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--families", nargs="+", choices=PROFILES, default=list(PROFILES))
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    report = export_contacts(args.sprites, args.out, args.families, args.report)
    print(json.dumps({name: entry["changed_pixels"] for name, entry in report["families"].items()}))


if __name__ == "__main__":
    main()
