#!/usr/bin/env python3
"""Rebuild the native Bloomwild planting plan without generating or editing assets.

uv run --with pillow python tools/pixellab/round13_vegetation.py

The Rust plan is the only authored placement source. For exact accepted runtime
roots, first run its native invariant test with GLINT_VEGETATION_EXPORT pointing
at this output directory's runtime-roots.json, then rerun this tool. Captures
remain separate evidence; this diagram labels its authored envelopes explicitly.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUT = ROOT / "art/pixellab/2026-11-02-round13/vegetation"
PLAN = ROOT / "src/regions/bloomwild/composition.rs"
SOURCE = ROOT / "art/pixellab/2026-11-01/grass/cuttable-selected-strip.png"
ATLAS = ROOT / "assets/sprites/bloomwild_grass.png"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative(path):
    return str(path.resolve().relative_to(ROOT))


def parse_plan():
    source = PLAN.read_text()
    bed_text = source.split("pub(super) const BEDS:", 1)[1].split("\n];", 1)[0]
    clearing_text = source.split("pub(super) const CLEARINGS:", 1)[1].split("\n];", 1)[0]
    beds = []
    for name, values in re.findall(r'\("([^"\n]+)", &\[([^\]\n]+)\]\)', bed_text):
        lobes = [list(map(int, oval)) for oval in re.findall(r'\((\d+), (\d+), (\d+), (\d+)\)', values)]
        assert lobes, name
        beds.append({"name": name, "lobes": lobes})
    clearings = [
        {"name": name, "oval": list(map(int, oval))}
        for name, *oval in re.findall(r'\("([^"\n]+)", \((\d+), (\d+), (\d+), (\d+)\)\)', clearing_text)
    ]
    assert len(beds) == 11 and len(clearings) == 7, "Rust planting declarations must remain completely parsed"
    return beds, clearings


def ellipse(x, y, oval):
    cx, cy, rx, ry = oval
    return ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2


def plan_cover(x, y, beds, clearings):
    if any(ellipse(x, y, clearing["oval"]) < 1 for clearing in clearings):
        return 0.0
    # Match the opening encounter's full moving-fan margin in the Rust plan.
    for clearing in clearings[:3]:
        cx, cy, _, _ = clearing["oval"]
        if ellipse(max(x - 17, min(cx, x + 16)),
                   max(y - 13, min(cy, y + 1)), clearing["oval"]) < 1:
            return 0.0
    return max((max(0.0, min(1.0, (1 - ellipse(x, y, oval)) * 3))
                for bed in beds for oval in bed["lobes"]), default=0.0)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--overview", type=Path,
                        default=ROOT / "out/pixellab/world/round8-selected/bloomwild/landing/bloomwild_overview.png")
    parser.add_argument("--roots", type=Path, help="Exact runtime export from the native invariant test")
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    beds, clearings = parse_plan()
    overview = args.overview.resolve()
    base = Image.open(overview).convert("RGB")
    assert base.size == (1344, 832), "native map diagram must retain one image pixel per world pixel"
    assert Image.open(SOURCE).tobytes() == Image.open(ATLAS).tobytes(), "reuse every approved PixelLab leaf unchanged"
    assert digest(SOURCE) == digest(ATLAS), "native packed strip must retain archived source bytes"
    roots_path = args.roots.resolve() if args.roots else out / "runtime-roots.json"
    runtime = json.loads(roots_path.read_text()) if roots_path.is_file() else None

    # This is a placement diagram, never a substitute for the final game frame.
    diagram = base.copy()
    pixels = diagram.load()
    bed_pixels = 0
    for bed in beds:
        x0 = max(0, min(cx - rx for cx, cy, rx, ry in bed["lobes"]))
        x1 = min(base.width, max(cx + rx for cx, cy, rx, ry in bed["lobes"]) + 1)
        y0 = max(0, min(cy - ry for cx, cy, rx, ry in bed["lobes"]))
        y1 = min(base.height, max(cy + ry for cx, cy, rx, ry in bed["lobes"]) + 1)
        for y in range(y0, y1):
            for x in range(x0, x1):
                cover = plan_cover(x, y, beds, clearings)
                if cover > 0:
                    r, g, b = base.getpixel((x, y))
                    pixels[x, y] = (int(r * 0.48 + 36), int(g * 0.56 + 57), int(b * 0.48 + 25))
                    bed_pixels += 1
    draw = ImageDraw.Draw(diagram)
    for index, bed in enumerate(beds, 1):
        cx, cy, _, _ = bed["lobes"][0]
        draw.rectangle((cx - 6, cy - 5, cx + 9, cy + 5), fill=(32, 39, 43))
        draw.text((cx - 4, cy - 5), str(index), fill=(233, 234, 207))
    for clearing in clearings:
        cx, cy, rx, ry = clearing["oval"]
        draw.ellipse((cx - rx, cy - ry, cx + rx, cy + ry), outline=(197, 205, 211))
    if runtime:
        assert runtime["native_grid"] == [1344, 832]
        for x, y in runtime["roots"]:
            assert plan_cover(x, y, beds, clearings) > 0, f"unplanned runtime root at{x},{y}"
            draw.point((x, y), fill=(242, 229, 181))
        for l, t, r, b in runtime["tree_support"]:
            draw.rectangle((int(l) - 18, int(t) - 3, int(r) + 18, int(b) + 16), outline=(180, 146, 139))
    draw.rectangle((0, 0, 557, 13), fill=(30, 38, 43))
    draw.text((4, 2), "PLANTING PLAN / authored beds + clearings / native world grid", fill=(235, 234, 218))
    diagram.save(out / "planting-plan-native.png")

    manifest = {
        "date": "2026-11-02",
        "native_grid": [1344, 832],
        "purpose": "Connected planted beds around quiet stone routes, the altar, exposed native tree roots and a clear aqueduct opening. Full moving leaf fans stay outside the opening encounter clearings so active Sproutlings remain readable.",
        "asset_work": "Existing native PixelLab vegetation reused byte-for-byte; no new species, generated images, recoloring, scaling, palette changes or lighting changes.",
        "included_generations_used": 0,
        "purchases": 0,
        "plan_source": {"path": relative(PLAN), "sha256": digest(PLAN)},
        "provenance": [
            {"path": relative(path), "sha256": digest(path)}
            for path in [SOURCE, ATLAS, ROOT / "assets/sprites/bloomwild_grass.json",
                         ROOT / "art/pixellab/2026-11-01/grass/selection.json",
                         ROOT / "art/pixellab/2026-11-01/manifest.json"]
        ],
        "diagram_background": {"path": relative(overview), "sha256": digest(overview),
                               "notes": "Earlier actual native map used for spatial registration only. Tint and outlines are a plan overlay, not final render evidence."},
        "beds": beds,
        "clearings": clearings,
        "runtime_roots": {"path": relative(roots_path), "sha256": digest(roots_path), "count": len(runtime["roots"])} if runtime else None,
        "review_probes": [
            {"at": [500, 637], "check": "Dense native leaf mass, sword cut, persistent stubble, bending and per-pixel foreground reveal."},
            {"at": [335, 710], "check": "Aqueduct opening stays clear; planting attaches to external pier toes at239,716 and438,715."},
            {"at": [400, 652], "check": "Connected meadow arena keeps approved terrain relief visible."},
            {"at": [465, 650], "check": "Actual 720-tick combat replay: sleeping camouflage, windup26, aimlock48, lunge58, first hurt65, and later attackers."},
            {"at": [908, 224], "check": "Visible grove root spread with planting at its shoulders; walk behind and in front."},
            {"at": [408, 176], "check": "Altar retains its quiet center and approach; no incidental live leaves."},
            {"at": [1210, 430], "check": "Temple-side bed silhouette, collision and sorting against native stone bases."}
        ],
        "checks": {"source_bytes_unchanged": True, "all_rust_beds_parsed": True,
                   "runtime_roots_inside_authored_envelopes": True if runtime else None},
        "limits": "Inspect final real frames and motion separately. Diagram coverage is the authored envelope before terrain, prop and source-specific route clearance."
    }
    (out / "composition.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"diagram": relative(out / "planting-plan-native.png"),
                      "beds": len(beds), "clearings": len(clearings),
                      "runtime_roots": len(runtime["roots"]) if runtime else None,
                      "generations_used": 0}))


if __name__ == "__main__":
    main()
