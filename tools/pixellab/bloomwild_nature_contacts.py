#!/usr/bin/env python3
"""Reapply the reviewed Bloomwild root, ceramic and boulder contact correction.

Scoped post-export step; never invokes an umbrella exporter. Example:
  uv run --with pillow python tools/pixellab/bloomwild_nature_contacts.py \
      --base assets/sprites --out /tmp/nature-contacts --audit /tmp/nature.json

Each span below describes an authored ground-facing root finger or rounded
physical foot, in native cell coordinates. Donors are adjacent material UVs in
the same cell. This is deliberately not a lowest-pixel, edge-detection or trim
operation: top edges, lateral ink, root clefts, canopies and unused cells stay
byte-identical. Only RGB changes; original alpha and external contracts stay.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image

# (contact y, first x, last x, donor y, first donor x, last donor x).
# Endpoints are inclusive. Donor x interpolates in integers across each span;
# the donor pixels themselves are copied exactly, with no color interpolation.
SPANS = {
    "bloomwild_trees": {
        "a_trunk": [
            (87, 10, 12, 86, 13, 14),       # left spread toe
            (89, 19, 20, 88, 21, 22),       # downward left toe
            (86, 30, 31, 86, 29, 29), (87, 29, 29, 86, 29, 29),
            (89, 33, 36, 88, 35, 36),       # broad front root
            (88, 37, 37, 87, 36, 36),
            (88, 45, 45, 88, 44, 44),       # right spread toe
        ],
        "b_trunk": [
            (87, 19, 21, 86, 19, 20), (88, 16, 18, 87, 18, 18),
            (91, 25, 26, 89, 26, 27),
            (88, 33, 33, 87, 34, 34), (89, 34, 34, 88, 35, 35),
            (90, 35, 36, 89, 35, 36), (91, 37, 38, 90, 37, 37),
            (89, 48, 48, 88, 47, 47),
        ],
        "c_trunk": [
            (88, 15, 18, 87, 17, 18),
            (90, 23, 25, 89, 25, 26),
            (88, 34, 35, 87, 34, 35), (89, 36, 38, 88, 36, 38),
            (88, 41, 45, 87, 41, 43),
        ],
        "d_trunk": [
            (85, 22, 25, 84, 22, 25),       # under the left shallow root
            (89, 22, 23, 88, 24, 25),
            (90, 31, 34, 89, 33, 34),
            (90, 43, 44, 89, 46, 46),
        ],
        "f_trunk": [
            (83, 17, 22, 82, 19, 22),       # first shallow left finger
            (87, 23, 26, 86, 23, 26),       # second left finger's lower edge
            (88, 19, 22, 87, 22, 22), (88, 27, 28, 87, 27, 27),
            (89, 38, 39, 89, 36, 37),       # broad central root's two toes
            (90, 34, 37, 89, 34, 37), (91, 32, 33, 90, 32, 33),
            (89, 47, 48, 88, 47, 48), (90, 49, 50, 89, 49, 49),
            (82, 59, 62, 81, 59, 60),       # separate shallow right fingers
            (83, 54, 57, 82, 50, 52),
            (84, 50, 51, 83, 50, 51),
            (85, 52, 57, 84, 52, 57), (86, 58, 61, 85, 58, 58),
        ],
        "g_trunk": [
            (84, 19, 21, 83, 19, 21), (85, 16, 18, 84, 18, 18),
            (86, 21, 23, 85, 23, 23),
            (88, 33, 35, 87, 33, 35),       # broad front root, clefts retained
            (89, 32, 32, 88, 32, 32), (90, 31, 31, 90, 30, 30),
            (91, 30, 30, 90, 30, 30),
            (88, 46, 46, 86, 45, 45), (89, 47, 47, 88, 47, 47),
            (90, 48, 51, 89, 48, 49),
            (82, 48, 52, 81, 49, 53), (85, 55, 58, 84, 55, 56),
        ],
    },
    "bloomwild_sapling": {
        "e_trunk": [
            (58, 20, 20, 57, 22, 22),
            (58, 29, 30, 57, 29, 30), (59, 27, 33, 58, 31, 33),
        ],
    },
    "bloomwild_props": {
        "urn_large": [(29, 12, 19, 28, 12, 18), (30, 13, 18, 28, 13, 18)],
        "urn_small": [(29, 13, 18, 28, 13, 17), (30, 14, 17, 28, 14, 17)],
        "boulder_large": [
            (27, 7, 10, 26, 7, 10), (27, 21, 24, 26, 21, 24),
            (28, 11, 20, 27, 11, 20), (29, 13, 18, 27, 13, 18),
        ],
    },
}

# Exact decoded-pixel hashes freeze the art that was inspected at gameplay scale.
# The corrected hash makes the operation idempotent and rejects stale/new art.
PIXEL_HASHES = {
    "bloomwild_trees": (
        "d9406e892254521474f006aa484c382fdf8514b947ab11c1ed9c58e4eba7f086",
        "a080fb1458fa73891469cb3bf43e8b705b7be8464b8e2de631aa32dd9dffa899",
    ),
    "bloomwild_sapling": (
        "b8fcc59ae175a1e8b1cb8ac74e53ec87f79c2b474bfccc9c1c8b9c10ffe3eebf",
        "c07e222871dd93f729c8742df956e86f9554643f37fd5aa1e2b678431956b240",
    ),
    "bloomwild_props": (
        "3ee46a9a5dfef8eb7de5acd7ca92f2f19ed384cc12cc4d3a20a8d03c5df181a8",
        "429cef5bc7a10fe5abed6336c5a8a00a54ba6a572d03db23f5e988fe02f8dd21",
    ),
}


def rgba_sha(image: Image.Image) -> str:
    return hashlib.sha256(image.tobytes()).hexdigest()


def correct_sheet(name: str, image: Image.Image, metadata: dict) -> tuple[Image.Image, list[dict]]:
    image = image.convert("RGBA")
    before = rgba_sha(image)
    if name in PIXEL_HASHES:
        original, corrected = PIXEL_HASHES[name]
        if before == corrected:
            return image.copy(), []
        if before != original:
            raise ValueError(f"{name}: unexpected art; review/re-author contact spans before export")
    result = image.copy()
    _, height = metadata["cell"]
    changed = []
    destinations = {
        (x, metadata["anims"][anim]["row"] * height + y)
        for anim, spans in SPANS[name].items()
        for y, x0, x1, _, _, _ in spans for x in range(x0, x1 + 1)
    }
    for anim, spans in SPANS[name].items():
        offset = metadata["anims"][anim]["row"] * height
        for y, x0, x1, donor_y, donor0, donor1 in spans:
            for x in range(x0, x1 + 1):
                donor_x = donor0 + (donor1 - donor0) * (x - x0) // max(1, x1 - x0)
                p, d = (x, offset + y), (donor_x, offset + donor_y)
                assert d not in destinations, (name, anim, "donor is changed", d)
                old, material = image.getpixel(p), image.getpixel(d)
                assert old[3] == material[3] == 255, (name, anim, p, d)
                assert max(old[:3]) <= 109, (name, anim, "not contact ink", p, old)
                assert material[:3] != old[:3], (name, anim, p, d, old)
                result.putpixel(p, (*material[:3], old[3]))
                changed.append({"anim": anim, "cell_xy": [x, y], "atlas_xy": list(p),
                                "donor_cell_xy": [donor_x, donor_y],
                                "before": list(old), "after": list(material)})
    assert result.getchannel("A").tobytes() == image.getchannel("A").tobytes()
    actual = {(x, y) for y in range(image.height) for x in range(image.width)
              if image.getpixel((x, y)) != result.getpixel((x, y))}
    assert actual == destinations, (name, "change outside exact authored mask")
    if name in PIXEL_HASHES:
        assert rgba_sha(result) == PIXEL_HASHES[name][1]
    return result, changed


def build(base: Path, out: Path, audit: Path) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    report = {"kind": "RGB-only authored native ground-contact spans", "assets": {}}
    for name in SPANS:
        image = Image.open(base / f"{name}.png").convert("RGBA")
        meta_path = base / f"{name}.json"
        result, changes = correct_sheet(name, image, json.loads(meta_path.read_text()))
        target = out / f"{name}.png"
        result.save(target)
        report["assets"][name] = {
            "source": str(base / f"{name}.png"), "output": str(target),
            "before_rgba_sha256": rgba_sha(image), "after_rgba_sha256": rgba_sha(result),
            "canvas": list(image.size), "changed_pixels": len(changes),
            "alpha_identical": True, "outside_authored_spans_identical": True,
            "metadata_sha256": hashlib.sha256(meta_path.read_bytes()).hexdigest(),
            "metadata_written": False, "normals_written": False, "pixels": changes,
        }
    audit.parent.mkdir(parents=True, exist_ok=True)
    audit.write_text(json.dumps(report, indent=2) + "\n")
    return report


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--base", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--audit", type=Path, required=True)
    args = p.parse_args()
    report = build(args.base, args.out, args.audit)
    print(json.dumps({k: v["changed_pixels"] for k, v in report["assets"].items()}))


if __name__ == "__main__":
    main()
