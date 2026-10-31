"""glasswater_crystals_big: landmark crystal clusters (48x64 cells)."""
import json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, blank, paste, save_sheet, on_bg, upscale, PREVIEWS, HEX, colors_used
from crystal import prism_layer, composite, outline
from rockart import g, BOULDER_S, BOULDER_M, BOULDER_L, STONE_XS, PEBBLE, RUBBLE, SLAB

CW, CH = 48, 64
S = (CW, CH)

# prism args: (bx, by, h, r, tilt, tl, depth, streak)
SPECS = {
    "aqua_a": dict(ramp="aqua", prisms=[
        (15, 58, 33, 5, -11, 8, 1, None),
        (33, 58, 27, 4, 9, 7, 1, None),
        (24, 58, 52, 6, 4, 10, 0, (10, 7)),
        (11, 61, 14, 3, -7, 4, 0, None),
        (37, 61, 12, 3, 6, 4, 0, None),
    ], rocks=[(BOULDER_M, 2, 55), (BOULDER_S, 36, 57), (RUBBLE, 19, 60), (PEBBLE, 44, 60)]),
    "violet_a": dict(ramp="violet", prisms=[
        (31, 58, 38, 5, 8, 8, 1, None),      # tall second, right, behind
        (13, 59, 22, 4, -8, 6, 1, None),
        (21, 58, 50, 6, -5, 10, 0, (12, 7)),  # main leans left
        (34, 61, 15, 3, 5, 4, 0, None),
        (8, 61, 9, 2, -3, 3, 0, None),
    ], rocks=[(BOULDER_S, 4, 57), (BOULDER_M, 31, 55), (STONE_XS, 15, 60), (PEBBLE, 1, 61)]),
    "aqua_b": dict(ramp="aqua", prisms=[      # wide fan, lower; good for corridor edges
        (11, 59, 26, 4, -6, 6, 1, None),
        (37, 59, 30, 4, 6, 7, 1, None),
        (17, 59, 38, 5, -6, 8, 0, (8, 6)),
        (30, 59, 42, 5, 6, 8, 0, None),
        (24, 61, 16, 3, 0, 5, 0, None),
    ], rocks=[(BOULDER_S, 2, 57), (RUBBLE, 13, 60), (BOULDER_S, 38, 57), (STONE_XS, 27, 59)]),
    "violet_b": dict(ramp="violet", prisms=[  # monolith
        (33, 59, 20, 3, 5, 5, 1, None),
        (22, 59, 56, 8, 8, 12, 0, (14, 9)),
        (12, 61, 13, 3, -6, 4, 0, None),
        (35, 61, 9, 2, 3, 3, 0, None),
    ], rocks=[(BOULDER_L, 6, 54), (BOULDER_S, 34, 57), (PEBBLE, 2, 61), (PEBBLE, 44, 60)]),
}


def build(name):
    sp = SPECS[name]
    ramp = sp["ramp"]
    layers = [prism_layer(S, ramp, *p[:6], depth=p[6], streak=p[7]) for p in sp["prisms"]]
    c = composite(layers, "o")
    c = outline(c, "o", sides=("r",))
    for (st, x, y) in sp["rocks"]:
        paste(c, g(st), x, y)
    return c


def main():
    names = list(SPECS)
    cells = {n: build(n) for n in names}
    rows = [[cells[n]] for n in names]
    anims = {}
    for i, n in enumerate(names):
        aq = SPECS[n]["ramp"] == "aqua"
        anims[n] = {"row": i, "frames": 1, "fps": 1,
                    "light": {"color": "#4fe0e8" if aq else "#a86cf0", "radius": 64 if aq else 56,
                              "intensity": 0.9, "offset": [0, -24]},
                    "footprint": [30, 8]}
    em = [HEX[k] for k in ("a3", "a4", "v3", "v4")]
    save_sheet("glasswater_crystals_big", rows, (CW, CH), {
        "anchor": [24, 61], "emissive": em, "anims": anims,
        "notes": "Landmark crystal clusters, about 3 tiles tall. Solid (footprint), not breakable. "
                 "Each casts its color's light from mid-height. Lit faces and ridges glow (emissive); "
                 "front and shadow faces are lit by the prop's own light. glasswater ramps, unchanged."})
    row = np.concatenate([np.concatenate([cells[n], blank(4, CH)], 1) for n in names], 1)
    upscale(on_bg(row, "#2f3c62", pad=4), 6).save(os.path.join(PREVIEWS, "_wip_crystals_big.png"))
    for n in names:
        print(n, len(colors_used(cells[n])), "colors")
    return cells

if __name__ == "__main__":
    main()
