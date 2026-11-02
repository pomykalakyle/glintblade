"""Remove only the ground-facing ink from original volumetric Summit columns.

All native geometry, upper/side outlines, anchors and support remain original.
The actual ground-facing dark contour continues its own stone face, never
alpha erasure, a new footing, a floor decal or an extra contact-shadow stroke.
"""
import argparse
from copy import deepcopy
import hashlib
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "art/pixellab/2026-11-02-summit-columns"
FAMILIES = {"summit_pillars": [4, 2, 1], "summit_stones": [1, 3]}
INK = (31, 40, 56, 255)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def remove_bottom_ink(image, first_row):
    original = image.copy()
    repairs = []
    for x in range(image.width):
        bottom = next((y for y in range(image.height - 1, first_row - 1, -1)
                       if original.getpixel((x, y))[3]), None)
        if bottom is None:
            continue
        for y in range(bottom, max(first_row - 1, bottom - 2), -1):
            if original.getpixel((x, y)) != INK:
                break
            sy = next((sy for sy in range(y - 1, max(first_row - 1, y - 9), -1)
                       if original.getpixel((x, sy))[3] and original.getpixel((x, sy)) != INK), None)
            # An ink-colored shadow face has no separate outline to extend.
            # Keep it: sampling a brighter neighbor across a corner adds a rim.
            if sy is None:
                continue
            sx, color = x, original.getpixel((x, sy))
            image.putpixel((x, y), color)
            repairs.append({"xy": [x, y], "face_source": [sx, sy]})
    return repairs


def base_planes(image, normal, first_row):
    """Native masks for the existing dressed base's cap and two stone returns.

    These named source ramp entries describe authored planes, not brightness
    thresholds. Keep all fractured upper shaft normals and geometry intact.
    """
    source = image.copy()
    for x in range(image.width):
        bottom = next((y for y in range(image.height - 1, first_row - 1, -1)
                       if source.getpixel((x, y))[3]), None)
        if bottom is None:
            continue
        lo, hi = max(0, x - 2), min(image.width - 1, x + 2)
        def low(column):
            return max((y for y in range(first_row, image.height)
                        if source.getpixel((column, y))[3]), default=bottom)
        sign = -1 if low(hi) >= low(lo) else 1
        for y in range(first_row, image.height):
            p = source.getpixel((x, y))
            if not p[3]:
                continue
            if p[:3] in ((148, 161, 174), (127, 135, 151), (85, 93, 112)):
                n = (0, 1, 0)  # tangent: upward cap
            elif p[:3] == (100, 115, 138):
                n = (0.4472135955, 0, 0.894427191)
            elif p[:3] in ((51, 59, 76), (70, 82, 106), (43, 53, 72)):
                n = (-0.4472135955, 0, 0.894427191)
            else:
                n = (sign * 0.4472135955, 0, 0.894427191)
            normal.putpixel((x, y), tuple(round((v + 1) * 127.5) for v in n) + (255,))


def build(archive=ARCHIVE):
    archive = Path(archive)
    recipe = json.loads((archive / "selection.json").read_text())
    result = {}
    for name in FAMILIES:
        paths = [archive / "baseline" / folder / f"{name}.png" for folder in ("sprites", "normals")]
        for path in paths:
            assert sha(path) == recipe["baseline_sha256"][str(path.relative_to(archive))]
        atlas, normals = (Image.open(p).convert("RGBA") for p in paths)
        meta = deepcopy(json.loads((archive / "baseline/sprites" / f"{name}.json").read_text()))
        for anim in meta["anims"].values():
            row = anim["row"]
            image = atlas.crop((0, row * 112, 64, (row + 1) * 112))
            repairs = remove_bottom_ink(image, anim["support_first_row"])
            normal = normals.crop((0, row * 112, 64, (row + 1) * 112))
            base_planes(image, normal, anim["support_first_row"])
            normals.paste(normal, (0, row * 112))
            atlas.paste(image, (0, row * 112))
            anim["bottom_outline_repair"] = repairs
        meta["notes"] = ("Original native PixelLab volumetric shafts, bases, silhouette, anchors, collision; upper normals retained.  "
                         "Only the lowest ground-facing dark outline is replaced by neighboring stone-face colors. "
                         "Top and side outlines retained. Physical cap/return normals replace the old downward-rounded base normals. No added base, paving patch or AO border. "
                         "Rebuild: tools/pixellab/summit_columns.py.")
        meta["provenance"] = {"archive": str(archive.relative_to(ROOT)) if archive.is_relative_to(ROOT) else str(archive),
                              "recipe_sha256": sha(archive / "selection.json")}
        result[name] = (atlas, normals, meta)
    return result


def export(out, normals_out=None, archive=ARCHIVE):
    result = build(archive)
    out = Path(out)
    normals_out = Path(normals_out) if normals_out else out.parent / "normals"
    out.mkdir(parents=True, exist_ok=True)
    normals_out.mkdir(parents=True, exist_ok=True)
    for name, (image, normal, meta) in result.items():
        image.save(out / f"{name}.png")
        normal.save(normals_out / f"{name}.png")
        (out / f"{name}.json").write_text(json.dumps(meta, indent=2) + "\n")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "assets/sprites")
    parser.add_argument("--normals-out", type=Path)
    args = parser.parse_args()
    export(args.out, args.normals_out)
