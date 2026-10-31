"""Build glasswater_tiles.png/.json v2 (16x16 tiles, named in the manifest's "tiles" map).

Run: uv run --no-project --with pillow --with numpy python3 tools/glasswater/build_tiles.py
"""
import json
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(__file__))
from px import NAMES, HEX, SPRITES, PREVIEWS, blank, emissive_hex, to_rgba, on_bg, upscale  # noqa: E402
from ground import ground_tiles  # noqa: E402
from face_panel import face_tiles  # noqa: E402
from tops import all_top_variants  # noqa: E402

T = 16
COLS = 16

NOTES = (
    "Ground tiles v2 for Glasswater Hollow. Look up a tile by name in 'tiles' -> [col, row]. "
    "Layers: paint floor_<tx%4>_<ty%4> (a seamless 64x64 macro block) on every tile first, then paint the "
    "listed tiles on top, skipping transparent pixels (cliff tiles, floor_shade_*, wet_*, decal_* are partly "
    "transparent). shore_*, water_*, deep_* are opaque. Only emissive pixels: the vein in cliff_face_*_5_vein and "
    "the top pixel of decal_shards_* (put those in ground_emit). Full rules and a Python reference autotiler: "
    "assets/notes/glasswater.md and tools/glasswater/autotile.py. Palette: glasswater ramps, unchanged."
)


def all_tiles():
    t = {}
    t.update(ground_tiles())
    t.update(face_tiles())
    t.update(all_top_variants())
    return t


def main():
    tiles = all_tiles()
    names = list(tiles.keys())
    rows = (len(names) + COLS - 1) // COLS
    sheet = blank(COLS * T, rows * T)
    table = {}
    for k, nm in enumerate(names):
        cx, cy = k % COLS, k // COLS
        sheet[cy * T:(cy + 1) * T, cx * T:(cx + 1) * T] = tiles[nm]
        table[nm] = [cx, cy]
    Image.fromarray(to_rgba(sheet), "RGBA").save(os.path.join(SPRITES, "glasswater_tiles.png"))
    used = sorted({NAMES[v] for v in np.unique(sheet) if v >= 0})
    em = [HEX[n] for n in used if HEX[n] in emissive_hex()]
    manifest = {"cell": [T, T], "anchor": [0, 0], "emissive": em, "tiles": table, "anims": {}, "notes": NOTES}
    with open(os.path.join(SPRITES, "glasswater_tiles.json"), "w") as f:
        json.dump(manifest, f, indent=1)
    print(len(names), "tiles,", rows, "rows; colors:", len(used))
    gap = 2
    pv = np.full((rows * (T + gap) + gap, COLS * (T + gap) + gap), -1, np.int32)
    for nm, (cx, cy) in table.items():
        pv[gap + cy * (T + gap):gap + cy * (T + gap) + T, gap + cx * (T + gap):gap + cx * (T + gap) + T] = tiles[nm]
    upscale(on_bg(pv, "#05040c", pad=0), 3).save(os.path.join(PREVIEWS, "glasswater_tiles_sheet.png"))
    return tiles


if __name__ == "__main__":
    main()
