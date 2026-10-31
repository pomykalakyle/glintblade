"""A 384x216 test scene built only from the shipped Bloomwild sheets (PNG + manifest), to judge
the props and floor together at native size. Writes out/samples/props_bloomwild_scene_3x.png."""
import json

import numpy as np
from PIL import Image

from pix import *
import slabs as SLABS
import tiles as TL

SPR = ROOT / "assets" / "sprites"
W, H = 384, 216


def sheet(name):
    img = np.array(Image.open(SPR / f"{name}.png").convert("RGBA"))
    man = json.load(open(SPR / f"{name}.json"))
    return img, man


SHEETS = {}


def frame(name, anim, i=0):
    if name not in SHEETS:
        SHEETS[name] = sheet(name)
    img, man = SHEETS[name]
    cw, ch = man["cell"]
    r = man["anims"][anim]["row"]
    return img[r * ch:(r + 1) * ch, i * cw:(i + 1) * cw], man["anchor"], man["anims"][anim]


def ground():
    TL.build_all()
    img = new(W, H)
    tw, th = W // 16, H // 16 + 1
    # flagstone area in vertex coords (inclusive vertex rectangle)
    vx0, vy0, vx1, vy1 = 9, 5, 19, 12

    def on(vx, vy):
        return vx0 <= vx <= vx1 and vy0 <= vy <= vy1

    floor = SLABS.floor_preview(tw, th, seed=3)
    for ty in range(th):
        for tx in range(tw):
            bits = (on(tx, ty), on(tx + 1, ty), on(tx, ty + 1), on(tx + 1, ty + 1))
            if all(bits):
                t = floor[ty * 16:(ty + 1) * 16, tx * 16:(tx + 1) * 16]
            elif not any(bits):
                v = int(hash01(tx, ty, 4) * 28)       # ~75% plain grass, as the tile docs advise
                t = TL.TILES[f"grass_{v if 1 <= v < 8 else 0}"]
            else:
                name = "grass_ruin_" + "".join(str(int(b)) for b in bits)
                if hash01(tx, ty, 7) < 0.5 and name + "_1" in TL.TILES:
                    name += "_1"
                t = TL.TILES[name]
            paste(img, t, tx * 16, ty * 16)
    return img


PLACE = [  # (sheet, anim, x, y) anchors in scene coords
    ("bloomwild_trees", "a_trunk", 46, 98),
    ("bloomwild_trees", "d_trunk", 352, 214),
    ("bloomwild_trees", "e_trunk", 120, 196),
    ("bloomwild_ruins", "statue", 236, 150),
    ("bloomwild_ruins", "pillar", 160, 104),
    ("bloomwild_ruins", "pillar_broken_tall", 290, 104),
    ("bloomwild_ruins", "pillar_broken_short", 160, 186),
    ("bloomwild_ruins", "drum", 312, 196),
    ("bloomwild_ruins", "wall_low", 34, 176),
    ("bloomwild_props", "bush_bloom", 108, 128),
    ("bloomwild_props", "bush_a", 22, 132),
    ("bloomwild_props", "urn_large", 182, 128),
    ("bloomwild_props", "urn_small", 194, 132),
    ("bloomwild_props", "flowers_a", 84, 150),
    ("bloomwild_props", "boulder_large", 360, 120),
    ("bloomwild_props", "boulder_small", 336, 126),
    ("bloomwild_lantern", "unlit", 278, 152),
    ("gate", "open", 236, 70),
    ("shard", "spin", 206, 168),
]
CANOPY = [("bloomwild_trees", "a_canopy", 46, 98), ("bloomwild_trees", "d_canopy", 352, 214),
          ("bloomwild_trees", "e_canopy", 120, 196)]


def build():
    img = ground()
    items = sorted(PLACE + [("hero", None, 214, 150)], key=lambda p: p[3])
    for (sh, an, x, y) in items:
        if sh == "hero":
            paste(img, stand_in_figure(), x - 4, y - 17)
            continue
        f, (ax, ay), meta = frame(sh, an, 0)
        paste(img, f, x - ax, y - ay)
    for (sh, an, x, y) in CANOPY:
        f, (ax, ay), _ = frame(sh, an, 0)
        paste(img, f, x - ax, y - ay)
    return img


if __name__ == "__main__":
    img = build()
    out = ROOT / "out" / "samples"
    out.mkdir(parents=True, exist_ok=True)
    save(img, PREVIEWS / "bloomwild_scene_1x.png")
    save(scale(img, 3), out / "props_bloomwild_scene_3x.png")
    save(scale(img, 2), PREVIEWS / "_wip_bw_crop.png")
