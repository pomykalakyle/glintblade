#!/usr/bin/env python3
"""Rebuild the sculptural Weir Keeper and its native articulated torso keys.

Offline only. No resizing, rotation, interpolation or generated credentials.
PixelLab sources and raw normal receipts live beside the registration recipe.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "art/pixellab/2026-11-02-round13/warden"
CELL = (120, 112)
ANCHOR = (60, 109)
# Exact source colors from selected PixelLab B. Merge only adjacent shades;
# never introduce a new accent or alter native pixel positions.
PALETTE = tuple(tuple(bytes.fromhex(c)) for c in (
    "1d232e", "242b3a", "303848", "3a4254", "4a5263", "585f6f",
    "6c7281", "7a808d", "8a8f9b", "979ba5", "a2a5ad", "b3b5bb", "ccccce",
))
POSES = ("awake", "brace", "opening")


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rgba(path: Path) -> Image.Image:
    im = Image.open(path).convert("RGBA")
    assert im.size == CELL, (path, im.size)
    assert set(im.getchannel("A").tobytes()) <= {0, 255}, path
    return im


def palette(im: Image.Image) -> Image.Image:
    out = im.copy()
    cache = {}
    for y in range(CELL[1]):
        for x in range(CELL[0]):
            r, g, b, a = im.getpixel((x, y))
            if a:
                c = (r, g, b)
                if c not in cache:
                    cache[c] = min(PALETTE, key=lambda p: sum((p[k]-c[k])**2 for k in range(3)))
                out.putpixel((x, y), (*cache[c], 255))
            else:
                out.putpixel((x, y), (0, 0, 0, 0))
    return out


def registered() -> dict[str, Image.Image]:
    """Keep top attachments immutable and give each native key exact feet.

    A one-pixel native footer registration gives the raw B's row110 outline
    two empty bottom rows. Pose edits are confined to articulated stone:
    brace below row30 except the fixed socket; opening inside the sluice.
    The original source planes own every registration boundary.
    """
    base = palette(rgba(ARCHIVE / "raw-design-b.png"))
    # The footer is a rigid native part, not a crop at the bottom of its cell.
    foot = base.crop((0, 82, 120, 112))
    ImageDraw.Draw(base).rectangle((0, 82, 119, 111), fill=(0, 0, 0, 0))
    base.alpha_composite(foot, (0, 81))
    out = {"awake": base}
    for name in ("brace", "opening"):
        generated = palette(rgba(ARCHIVE / f"raw-{name}.png"))
        merged = base.copy()
        if name == "brace":
            # Tower crowns and all three upper attachments retain exact B pixels.
            regions = [(1, 31, 39, 81), (82, 31, 119, 81), (39, 63, 82, 81)]
        else:
            # Only the gate can open; towers, collar and supported piers stay put.
            regions = [(36, 34, 84, 81)]
        for box in regions:
            merged.paste(generated.crop(box), box[:2])
        # B's seam pixels keep continuous structural planes at each patch boundary.
        for x in (35, 84):
            merged.paste(base.crop((x, 30, x+1, 82)), (x, 30))
        out[name] = merged
    return out


def damage(im: Image.Image) -> Image.Image:
    """Literal structural losses; fixed seats, collar and valve center survive."""
    out = im.copy()
    d = ImageDraw.Draw(out)
    for points in (
        ((14, 2), (21, 2), (14, 10)),
        ((102, 11), (108, 16), (108, 25), (102, 20)),
        ((114, 32), (119, 32), (119, 46), (116, 42)),
        ((38, 65), (47, 65), (47, 69), (42, 72), (38, 72)),
        ((74, 73), (82, 73), (82, 79), (77, 77), (74, 77)),
    ):
        d.polygon(points, fill=(0, 0, 0, 0))
    return out


def folded(im: Image.Image, depth: int) -> Image.Image:
    """Articulate two native culvert piers while the upper assembly sinks.

    At runtime the existing torso path moves down by `depth`. Each rigid pier
    moves up by the same depth inside its cell, and sideways by up to 2 pixels.
    Its top overlaps the gate apron like a sliding stone joint. Thus its feet
    remain on the old ground line, with no stretched rows or sliced-off toes.
    The exact same native remapping is applied to the normal data.
    """
    if depth == 0:
        return im.copy()
    out = im.copy()
    ImageDraw.Draw(out).rectangle((0, 81, 119, 111), fill=(0, 0, 0, 0))
    spread = depth // 8
    for box, offset in (((0, 81, 60, 110), (-spread, 81-depth)),
                        ((60, 81, 120, 110), (60+spread, 81-depth))):
        out.alpha_composite(im.crop(box), offset)
    return out


def alpha_like(normal: Image.Image, art: Image.Image) -> Image.Image:
    out = normal.copy()
    out.putalpha(art.getchannel("A"))
    for y in range(out.height):
        for x in range(out.width):
            if not out.getpixel((x, y))[3]:
                out.putpixel((x, y), (0, 0, 0, 0))
    return out


def encode_world_normal(vector: list[float]) -> tuple[int, int, int, int]:
    """Upright sprites decode PNG tangent XYZ to world X,Y,Z as X,Z,Y.

    In particular a vertical front is (128,128,255), not a blue/green
    gradient towards the foot. A roof is (128,255,128).
    """
    length = math.sqrt(sum(v*v for v in vector))
    assert length > 0
    x, y, z = (v/length for v in vector)
    return (*(round((v+1)*127.5) for v in (x, z, y)), 255)


def semantic_normal(art: Image.Image, pose: str) -> Image.Image:
    """Native geometric planes, authored against the registered albedo faces.

    Shade/color patches and the silhouette do not infer geometry. Every opaque
    pixel starts on the explicit vertical front/back-wall plane. Native polygon
    masks then describe caps, exposed roofs, and architectural side returns.
    The original generated normal maps remain archived evidence only: their
    whole-body rounded volume gave the upright pier fronts downward normals.
    """
    spec = json.loads((ARCHIVE / "semantic-planes.json").read_text())
    out = Image.new("RGBA", CELL, encode_world_normal(spec["default"]["world_normal"]))
    draw = ImageDraw.Draw(out)
    for face in spec["common"] + spec["poses"][pose]:
        draw.polygon([tuple(p) for p in face["polygon"]],
                     fill=encode_world_normal(spec["vectors"][face["plane"]]))
    return alpha_like(out, art)


def stabilize(art: Image.Image, normal: Image.Image, base: Image.Image, base_n: Image.Image) -> Image.Image:
    """Keep the registered rigid attachments and piers geometrically fixed.

    A brace can expose a different face with the same slate color. Equality of
    a 3x3 albedo patch inside an articulated part does not establish equality
    of its plane; only the explicitly fixed native assembly shares geometry.
    """
    out = normal.copy()
    for y in range(1, 111):
        for x in range(1, 119):
            fixed = y < 30 or y >= 81 or x in (35, 84)
            if fixed and art.getpixel((x, y))[3] and all(
                    art.getpixel((x+dx, y+dy)) == base.getpixel((x+dx, y+dy))
                    for dy in (-1, 0, 1) for dx in (-1, 0, 1)):
                out.putpixel((x, y), base_n.getpixel((x, y)))
    return out


def prepare() -> None:
    for name, im in registered().items():
        im.save(ARCHIVE / f"registered-{name}.png")
    print("Prepared three exact native normal inputs in", ARCHIVE)


def build(out: Path, normals_out: Path, review: Path | None = None) -> dict:
    bases = registered()
    normal_bases = {name: semantic_normal(im, name)
                    for name, im in bases.items()}
    for name in ("brace", "opening"):
        normal_bases[name] = stabilize(bases[name], normal_bases[name], bases["awake"], normal_bases["awake"])
    rows: list[tuple[str, list[Image.Image], list[Image.Image]]] = []
    for name, key, broken in (("idle", "awake", False), ("awake", "awake", False),
                              ("damaged", "awake", True), ("brace", "brace", False),
                              ("damaged_brace", "brace", True), ("opening", "opening", False),
                              ("damaged_opening", "opening", True)):
        a = damage(bases[key]) if broken else bases[key]
        n = alpha_like(normal_bases[key], a)
        rows.append((name, [a], [n]))
    for name, key, broken in (("awake_fold", "awake", False), ("opening_fold", "opening", False),
                              ("damaged_fold", "awake", True), ("damaged_opening_fold", "opening", True)):
        a = damage(bases[key]) if broken else bases[key]
        n = alpha_like(normal_bases[key], a)
        rows.append((name, [folded(a, d) for d in range(17)],
                     [folded(n, d) for d in range(17)]))
    # A static pose aliases its fold row's frame0. Only the two different brace
    # keys need extra rows. This halves empty-cell registry work without changing
    # a single visible albedo/normal pixel or a public animation contract.
    physical_names = ("awake_fold", "opening_fold", "damaged_fold", "damaged_opening_fold", "brace", "damaged_brace")
    aliases = {"idle": "awake_fold", "awake": "awake_fold", "damaged": "damaged_fold",
               "opening": "opening_fold", "damaged_opening": "damaged_opening_fold"}
    width, height = CELL[0]*17, CELL[1]*len(physical_names)
    atlas, normals = Image.new("RGBA", (width, height)), Image.new("RGBA", (width, height))
    anims = {}
    audit = []
    for name, frames, maps in rows:
        row = physical_names.index(aliases.get(name, name))
        anims[name] = {"row": row, "frames": len(frames), "fps": 1.0,
                       "durations": [60]*len(frames), "loop": False}
        for i, (art, normal) in enumerate(zip(frames, maps, strict=True)):
            assert art.getchannel("A").tobytes() == normal.getchannel("A").tobytes(), (name, i)
            assert not art.crop((0, 110, 120, 112)).getbbox(), (name, i, "footer margin")
            # Attachment and socket keys are immutable while the lower piers articulate.
            if len(frames) > 1:
                assert art.crop((0, 0, 120, 63)).tobytes() == frames[0].crop((0, 0, 120, 63)).tobytes()
            atlas.paste(art, (i*120, row*112))
            normals.paste(normal, (i*120, row*112))
            audit.append({"anim": name, "frame": i, "bbox": art.getbbox(),
                          "native_pier_fold": i if len(frames) > 1 else 0})
    meta = {"cell": list(CELL), "anchor": list(ANCHOR), "emissive": [], "anims": anims,
            "notes": "Sculptural Weir Keeper. PixelLab B's thick planar towers, projecting buttresses, "
            "recessed valve and native culvert piers. Brace and opening are separately generated native "
            "structural keys. The top30 rows, collar and crystal seats never animate; the valve center "
            "stays (60,49). *_fold frame index is the existing body_dy rounded to 0..16, not elapsed time: "
            "two rigid culvert piers fold up and splay by 0..2 native pixels while the upper torso follows "
            "its original combat path. Every normal follows the same native part remapping. Anchor, "
            "collision, heart windows, and head/fist animation are unchanged. 13 native slate shades; "
            "no torso emission. Empty atlas cells are intentional."}
    out.mkdir(parents=True, exist_ok=True)
    normals_out.mkdir(parents=True, exist_ok=True)
    atlas.save(out / "warden_body.png")
    normals.save(normals_out / "warden_body.png")
    (out / "warden_body.json").write_text(json.dumps(meta, indent=2)+"\n")
    report = {"source_sha256": {p.name: sha(p) for p in sorted(ARCHIVE.glob("raw-*.png"))},
              "atlas_sha256": sha(out / "warden_body.png"), "normal_sha256": sha(normals_out / "warden_body.png"),
              "cell": CELL, "anchor": ANCHOR, "opaque_palette_size": len(PALETTE),
              "physical_rows": list(physical_names), "static_aliases": aliases,
              "physical_cells_before": 187, "physical_cells_after": 102,
              "normal_alpha_mismatches": 0, "pose_frames": audit,
              "normal_geometry": {"source": "semantic-planes.json", "sha256": sha(ARCHIVE / "semantic-planes.json"),
                                  "method": "authored native structural face polygons in world X,Y,Z; PNG tangent X,Z,Y",
                                  "generated_normal_maps": "archived only; not used by exporter"},
              "registration": {"base": "raw-design-b.png", "footer_source_rows": [82, 111],
                               "footer_offset": [0, -1], "fixed_upper_rows": [0, 29],
                               "brace_native_regions": [[1,31,39,81],[82,31,119,81],[39,63,82,81]],
                               "opening_native_regions": [[36,34,84,81]],
                               "seam_columns_from_base": [35,84], "fold": "folded() remaps art and normal identically"}}
    if review:
        review.mkdir(parents=True, exist_ok=True)
        review_sheet(rows, review)
        (review / "audit.json").write_text(json.dumps(report, indent=2)+"\n")
    print(f"Warden: {len(audit)} native torso keys, 13 shades, matching normals: {out}")
    return report


def review_sheet(rows, dest: Path) -> None:
    choices = [("idle", 0, 0), ("brace", 0, 0), ("opening", 0, 0),
               ("awake_fold", 8, 8), ("awake_fold", 16, 16), ("opening_fold", 16, 16),
               ("damaged", 0, 0), ("damaged_opening_fold", 16, 16)]
    by_name = {name: frames for name, frames, _ in rows}
    contact = Image.new("RGBA", (160*4, 170*2), (26, 26, 34, 255))
    for k, (name, frame, down) in enumerate(choices):
        x, y = (k%4)*160+20, (k//4)*170+25
        art = by_name[name][frame]
        contact.alpha_composite(art, (x, y+down))
        d = ImageDraw.Draw(contact)
        d.line((x-3, y+110, x+123, y+110), fill=(68, 60, 75))
        d.text((x, y+122), f"{name} {frame}", fill=(195, 195, 202))
    contact.save(dest / "torso-keys-native.png")
    contact.resize((contact.width*3, contact.height*3), Image.Resampling.NEAREST).save(dest / "torso-keys-3x.png")
    assembly = Image.new("RGBA", (192*4, 192*2), (26, 26, 34, 255))

    def part(name, anim, frame=0, flip=False):
        path = ROOT / "assets/sprites" / f"warden_{name}"
        meta = json.loads(path.with_suffix(".json").read_text())
        cw, ch = meta["cell"]
        row = meta["anims"][anim]["row"]
        art = Image.open(path.with_suffix(".png")).convert("RGBA").crop((frame*cw, row*ch, (frame+1)*cw, (row+1)*ch))
        anchor = meta["anchor"]
        if flip:
            art = ImageOps.mirror(art)
            anchor = [cw-1-anchor[0], anchor[1]]
        return art, anchor

    for k, (name, frame, down) in enumerate(choices):
        x, y = (k%4)*192+96, (k//4)*192+145
        assembly.alpha_composite(by_name[name][frame], (x-60, y+down-109))
        for side in range(2):
            image, anchor = part("crystal", "intact", flip=side == 1)
            assembly.alpha_composite(image, (x+(-36 if side == 0 else 36)-anchor[0], y+down-100-anchor[1]))
        image, anchor = part("head", "idle", 1)
        assembly.alpha_composite(image, (x-anchor[0], y+down-75-anchor[1]))
        image, anchor = part("heart", "whole")
        heart_y = y+10 if "opening" in name else y+down-60
        assembly.alpha_composite(image, (x-anchor[0], heart_y-anchor[1]))
        for side in range(2):
            image, anchor = part("fist", "open" if down else "clenched", flip=side == 0)
            assembly.alpha_composite(image, (x+(-54 if side == 0 else 54)-anchor[0], y-(0 if down else 11)-anchor[1]))
        d = ImageDraw.Draw(assembly)
        d.line((x-70, y+1, x+70, y+1), fill=(68, 60, 75))
        d.text((x-86, y+28), f"{name} {frame}", fill=(195, 195, 202))
    assembly.save(dest / "assembled-keys-native.png")
    assembly.resize((assembly.width*3, assembly.height*3), Image.Resampling.NEAREST).save(dest / "assembled-keys-3x.png")
    # Diagnostic only: fixed directional sources show whether broad stone faces
    # respond coherently. The root's actual game captures own final lighting QA.
    lighting = Image.new("RGBA", (120*4, 140*3), (26, 26, 34, 255))
    normal_by_name = {name: maps for name, _, maps in rows}
    for row, name in enumerate(("awake", "brace", "opening")):
        art, normal = by_name[name][0], normal_by_name[name][0]
        for col, light in enumerate(((-.8,.6,.3),(.8,.6,.3),(0,1,.25),(0,0,1))):
            lit = art.copy()
            ln = math.sqrt(sum(v*v for v in light))
            for yy in range(112):
                for xx in range(120):
                    pixel = art.getpixel((xx, yy))
                    if not pixel[3]:
                        continue
                    tangent = [v/127.5-1 for v in normal.getpixel((xx, yy))[:3]]
                    vector = [tangent[0], tangent[2], tangent[1]]
                    length = math.sqrt(sum(v*v for v in vector)) or 1
                    gain = .4+.72*max(0, sum(vector[i]*light[i]/(length*ln) for i in range(3)))
                    lit.putpixel((xx, yy), (*(min(255, round(c*gain)) for c in pixel[:3]), 255))
            lighting.alpha_composite(lit, (col*120, row*140))
            ImageDraw.Draw(lighting).text((col*120+3, row*140+116), f"{name} light{col+1}", fill=(195,195,202))
    lighting.save(dest / "normal-light-directions-native.png")
    lighting.resize((lighting.width*3, lighting.height*3), Image.Resampling.NEAREST).save(dest / "normal-light-directions-3x.png")


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--prepare", action="store_true")
    p.add_argument("--out", type=Path, default=ROOT / "assets/sprites")
    p.add_argument("--normals-out", type=Path)
    p.add_argument("--review", type=Path)
    a = p.parse_args()
    if a.prepare:
        prepare()
    else:
        normals_out = a.normals_out or (a.out.parent / "normals")
        build(a.out, normals_out, a.review)


if __name__ == "__main__":
    main()
