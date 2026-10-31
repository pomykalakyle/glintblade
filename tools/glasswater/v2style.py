"""Rebuild the owned Glasswater sheets in The Long Quiet style, preserving manifest layouts.

Run with: uv run --no-project --with pillow python3 tools/glasswater/v2style.py
The checked-in tile manifest is the stable layout registry; every named cell is redrawn here.
"""
import json
from pathlib import Path

from PIL import Image, ImageDraw

import landmarks

ROOT = Path(__file__).resolve().parents[2]
SPRITES = ROOT / "assets/sprites"
PALETTE = json.loads((ROOT / "assets/concept/palettes_v2.json").read_text())["regions"]["glasswater"]
ROCK = PALETTE["ramps"]["rock"]
WALL = PALETTE["ramps"]["wall"]
WATER = PALETTE["ramps"]["water"]
IRIS = PALETTE["ramps"]["crystal_violet"]


def stone(draw, x, y, size=5):
    """Draw one clipped stone with attached down-right contact and an upper-left plane."""
    draw.polygon([(x, y + 1), (x + size - 2, y), (x + size, y + 2), (x + size - 1, y + 4), (x + 1, y + 4)], fill=ROCK[0])
    draw.polygon([(x, y), (x + size - 2, y - 1), (x + size - 1, y + 1), (x + size - 2, y + 2), (x, y + 2)], fill=ROCK[2])
    draw.line([(x, y), (x + size - 2, y - 1)], fill=ROCK[3])


def tiles():
    """Redraw every existing 16px cell while preserving its name and sheet coordinate."""
    dest = SPRITES / "glasswater_tiles"
    manifest = json.loads(dest.with_suffix(".json").read_text())
    table = manifest["tiles"]
    width = (max(x for x, y in table.values()) + 1) * 16
    height = (max(y for x, y in table.values()) + 1) * 16
    sheet = Image.new("RGBA", (width, height))
    macro = Image.new("RGBA", (64, 64), ROCK[2])
    md = ImageDraw.Draw(macro)
    # Wrap authored polygons across all edges so a tiled 64px block has no seam.
    for oy in (-64, 0, 64):
        for ox in (-64, 0, 64):
            md.polygon([(x + ox, y + oy) for x, y in [(-10, 36), (29, 28), (53, 34), (74, 32), (74, 47), (24, 46), (-10, 53)]], fill=ROCK[1])
            md.polygon([(x + ox, y + oy) for x, y in [(13, -4), (51, -4), (57, 10), (25, 17), (5, 10)]], fill=ROCK[3])
    for name, (col, row) in table.items():
        cell = Image.new("RGBA", (16, 16))
        d = ImageDraw.Draw(cell)
        if name.startswith("floor_") and name.split("_")[1].isdigit():
            _, x, y = name.split("_")
            cell = macro.crop((int(x) * 16, int(y) * 16, int(x) * 16 + 16, int(y) * 16 + 16))
        elif name.startswith("decal_"):
            if "pebble" in name or name == "decal_stone":
                stone(d, 4, 7, 7 if name == "decal_stone" else 5)
                if "pebble" in name:
                    stone(d, 10, 11, 3)
                    if name.endswith("b"):
                        stone(d, 2, 3, 3)
            elif "crack" in name:
                points = [(2, 4), (5, 7), (10, 8), (13, 11)] if name.endswith("a") else [(3, 12), (6, 8), (11, 5), (12, 3)]
                d.line(points[:3], fill=ROCK[1], width=2)
                d.line(points[2:], fill=ROCK[1])
            elif "shards" in name:
                d.polygon([(5, 9), (7, 3), (9, 9)], fill=IRIS[1])
                d.polygon([(6, 8), (7, 4), (7, 9)], fill=IRIS[2])
                d.rectangle((4, 10, 10, 11), fill=ROCK[0])
                d.line([(7, 3), (7, 4)], fill=IRIS[3])
                d.polygon([(10, 10), (12, 7), (13, 11)], fill=IRIS[0])
            else:
                d.polygon([(3, 8), (6, 5), (10, 6), (12, 9), (9, 11), (5, 10)], fill=ROCK[1])
        elif name.startswith("floor_shade"):
            for y in range(16):
                for x in range(16):
                    north = "_n" in name and y < [4, 4, 5, 5, 5, 6, 6, 5][x // 2]
                    west = ("_w" in name or "nw" in name) and x < [4, 4, 4, 5, 5, 4, 3, 3][y // 2]
                    if north or west or name.endswith("_c") and x + y < 7:
                        d.point((x, y), fill=ROCK[1])
        elif name.startswith(("shore_", "deep_")):
            bits = list(map(int, name.split("_")[1]))
            deep = name.startswith("deep_")
            for y in range(16):
                v = (y + 0.5) / 16
                for x in range(16):
                    u = (x + 0.5) / 16
                    field = (bits[0] * (1 - u) + bits[1] * u) * (1 - v) + (bits[2] * (1 - u) + bits[3] * u) * v
                    fill = WATER[0 if field > 0.5 else 1] if deep else WATER[1] if field > 0.5 else ROCK[1]
                    d.point((x, y), fill=fill)
        elif name.startswith("water_"):
            d.rectangle((0, 0, 15, 15), fill=WATER[1])
            if name != "water_0":
                d.polygon([(4, 8), (7, 6), (11, 7), (12, 10), (6, 11)], fill=WATER[0])
                d.line([(4, 8), (7, 6), (10, 7)], fill=WATER[2])
        elif name.startswith("cliff_face"):
            parts = name.split("_")
            variant = next(int(p) for p in parts if p.isdigit())
            y0 = {"top": 0, "mid": 16, "bot": 32}[parts[2]]
            for y in range(16):
                for x in range(16):
                    wx, wy = variant * 16 + x, y0 + y
                    pos = (wx + wy // 5) % 53
                    c = WALL[2] if pos < 17 - wy // 8 else WALL[1]
                    if pos > 50 or parts[2] == "bot" and y > 13:
                        c = WALL[0]
                    if parts[2] == "top" and y < 3 and pos < 41:
                        c = WALL[3]
                    d.point((x, y), fill=c)
        elif name.startswith("cliff_top"):
            d.rectangle((0, 0, 15, 15), fill=WALL[1])
            if name[-1:].isdigit():
                d.polygon([(0, 0), (15, 0), (13, 8), (5, 12), (0, 10)], fill=WALL[2])
            edge = name.removeprefix("cliff_top").split("_in")[0].strip("_")
            if "n" in edge:
                d.polygon([(0, 0), (15, 0), (13, 2), (3, 3), (0, 2)], fill=WALL[3])
            if "w" in edge:
                d.rectangle((0, 0, 2, 15), fill=WALL[2])
            if "e" in edge:
                d.rectangle((14, 0, 15, 15), fill=WALL[0])
        else:
            raise ValueError(f"Unpainted tile {name}")
        sheet.paste(cell, (col * 16, row * 16))
    manifest["emissive"] = [IRIS[3]]
    manifest["notes"] = "Style v2: slate rock, calm water and one pale iris mineral family. Existing layout and names preserved, including legacy aqua names rendered as iris. Macro floor is 64px. Fractured cliff faces span tile seams with unequal planes. Decals are sparse connected stones with attached contact; only broken crystal tips emit. No luminous lichen or wall veins. Rebuild with tools/glasswater/v2style.py."
    sheet.save(dest.with_suffix(".png"))
    dest.with_suffix(".json").write_text(json.dumps(manifest, indent=1) + "\n")


def crystals():
    """Redraw four distinct iris landmarks inside the existing 48x64 cells and anchors."""
    dest = SPRITES / "glasswater_crystals_big"
    manifest = json.loads(dest.with_suffix(".json").read_text())
    sheet = Image.new("RGBA", (48, 256))
    names = ["aqua_a", "violet_a", "aqua_b", "violet_b"]
    for row, name in enumerate(names):
        cell = landmarks.draw(name, PALETTE)
        assert cell.getbbox()[0] >= 2 and cell.getbbox()[2] <= 46, "Crystal needs a transparent margin"
        sheet.paste(cell, (0, row * 64))
        light = manifest["anims"][name]["light"]
        light["color"] = IRIS[3]
        light["radius"] = 44
        light["intensity"] = 0.28
    manifest["emissive"] = [IRIS[3]]
    manifest["notes"] = "Style v2 pale iris landmarks on slate mineral roots (round 6, 2026-10-31). Four distinct silhouettes: aqua_a a dark column with a light-facing chisel cut, violet_a a left-leaning spear before a broad unlit back shard, aqua_b a lit crystal toppled across a dark upright one, violet_b a squat crown. Each has one broad lit plane broken by a fracture, a larger unlit mass, small emissive tip caps (at least 5 within 18 px of the anchor column, which the lighting bake needs to add the crystal source) and a slate root overlapping every base. Upper-left light, so these read best unflipped. Legacy aqua/violet animation names, 48x64 cells, 24,61 anchor, footprints and light metadata preserved. Alpha is binary and a transparent margin prevents clipping or wrapped pixels. Rebuild with tools/glasswater/v2style.py (facet plans in tools/glasswater/landmarks.py)."
    sheet.save(dest.with_suffix(".png"))
    dest.with_suffix(".json").write_text(json.dumps(manifest, indent=1) + "\n")


if __name__ == "__main__":
    tiles()
    crystals()
