"""Assemble a native 384x216 mock screen from the tiles (vertex-based terrain) to judge the look."""
import sys

import tiles as TL
from pix import *

# Vertex map: one char per tile CORNER. g = grass, p = path, r = ruin floor.
VMAP = """
gggggggggggggggggggggggggg
gggggggggggggggggggggggggg
gggggggggggrrrrrrrgggggggg
gggggggggggrrrrrrrgggggggg
gggggggggggrrrrrrrgggggggg
ggggggggggrrrrrrrrrggggggg
pppppgggggrrrrrrrrrggggggg
ppppppppprrrrrrrrrrppppppp
ppppppppprrrrrrrrrrppppppp
gggggppppgrrrrrrrrrggggggg
gggggggppggrrrrrrrgggggggg
ggggggggppggggggggggggggg
gggggggggppggggggggggggggg
ggggggggggppgggggggggggggg
gggggggggggppggggggggggggg
""".strip().split("\n")
VMAP = [r.ljust(26, "g") for r in VMAP]

GRASS_W = [(0, 30), (1, 3), (2, 2), (3, 2), (4, 1), (5, 1), (6, 1), (7, 1)]
RUIN_W = [(0, 4), (1, 4), (2, 4), (3, 4), (4, 4), (5, 4), (6, 4), (7, 4), (8, 1), (9, 1), (10, 0.4), (11, 1)]
PATH_W = [(0, 8), (1, 2), (2, 1), (3, 2)]


def pick(weights, h):
    tot = sum(w for _, w in weights)
    t = h * tot
    for v, w in weights:
        if t < w:
            return v
        t -= w
    return weights[-1][0]


def tile_for(tx, ty, vmap):
    c = [vmap[ty][tx], vmap[ty][tx + 1], vmap[ty + 1][tx], vmap[ty + 1][tx + 1]]
    h = hash01(tx, ty, 7)
    if all(k == "g" for k in c):
        if hash01(tx, ty, 3) < 0.04:
            return f"flowers_{int(hash01(tx, ty, 5) * 3)}"
        return f"grass_{pick(GRASS_W, h)}"
    if all(k == "p" for k in c):
        return f"path_{pick(PATH_W, h)}"
    if all(k == "r" for k in c):
        return f"ruin_{pick(RUIN_W, h)}"
    other = "p" if "p" in c else "r"
    bits = "".join("1" if k == other else "0" for k in c)
    pre = "grass_path" if other == "p" else "grass_ruin"
    name = f"{pre}_{bits}"
    alt = name + "_1"
    if alt in TL.TILES and h < 0.5:
        return alt
    return name


def ground(vmap):
    th, tw = len(vmap) - 1, len(vmap[0]) - 1
    img = new(tw * 16, th * 16)
    for ty in range(th):
        for tx in range(tw):
            t = TL.TILES[tile_for(tx, ty, vmap)]
            img[ty * 16:(ty + 1) * 16, tx * 16:(tx + 1) * 16] = t
    return img


if __name__ == "__main__":
    TL.build_all()
    g = ground(VMAP)
    scene = g[:216, :384].copy()
    fig = stand_in_figure()
    paste(scene, fig, 150, 120)
    save(scene, PREVIEWS / "_wip_bw_mock_1x.png")
    save(scale(scene, 3), PREVIEWS / "_wip_bw_mock_3x.png")
