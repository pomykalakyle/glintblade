"""Assemble a test map from the tile generators to check seams and the overall look."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, on_bg, upscale, PREVIEWS
from cliff import autotile, all_top_variants, all_face_tiles, floor_shade, T

MAP = [
    "########################",
    "########################",
    "########################",
    "##....................##",
    "##....................##",
    "##.......#####........##",
    "##......######........##",
    "##......#######.......##",
    "##......###.###.......##",
    "##......###.###.......##",
    "##........#...........##",
    "##....................##",
    "###..................###",
    "########################",
]

def tiles_dict():
    d = {}
    d.update(all_top_variants())
    d.update(all_face_tiles())
    d["floor_shade_n"] = floor_shade(n=True)
    d["floor_shade_w"] = floor_shade(w=True)
    d["floor_shade_nw"] = floor_shade(n=True, w=True)
    d["floor"] = floor_shade()
    return d

def render(MAP, fh=2):
    rock = np.array([[ch == "#" for ch in row] for row in MAP])
    names = autotile(rock, fh)
    td = tiles_dict()
    th, tw = rock.shape
    out = np.zeros((th * T, tw * T), np.int32)
    missing = set()
    for ty in range(th):
        for tx in range(tw):
            nm = names.get((tx, ty), "floor")
            if nm not in td:
                missing.add(nm); nm = "floor"
            out[ty * T:(ty + 1) * T, tx * T:(tx + 1) * T] = td[nm]
    return out, missing, names

if __name__ == "__main__":
    out, missing, names = render(MAP)
    print("missing:", missing)
    upscale(on_bg(out, "#000000", pad=0), 3).save(os.path.join(PREVIEWS, "_wip_cliffmap.png"))
