"""Mock-lit check: a sprite canvas on the cave floor with its light, next to the hero for scale."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, Scene, upscale, rough_hero, EMISSIVE

def litcheck(items, out, W=160, H=96, ambient=0.38, scale=4):
    """items: list of (canvas, x, y_bottom, light or None) where light = (dx, dy, radius, color, k)."""
    sc = Scene(W, H, amb_k=ambient)
    floor = np.full((H, W), I["r3"], np.int32)
    sc.sprite(floor, 0, 0)
    hero = rough_hero()
    allitems = list(items) + [(hero, W - 24, H - 14, (6, -8, 64, "#9ff4ff", 0.8))]
    allitems.sort(key=lambda it: it[2])
    for (c, x, yb, light) in allitems:
        sc.sprite(c, x, yb - c.shape[0] + 1)
        if light:
            dx, dy, r, col, k = light
            sc.add_light(x + c.shape[1] / 2 + dx, yb + dy, r, col, k)
    sc.render()
    sc.composite(scale).save(out)
    return sc

if __name__ == "__main__":
    a = np.load("/tmp/gw_crys_a.npy")
    litcheck([(a, 20, 80, (0, -24, 56, "#4fe0e8", 0.8))], "/tmp/gw_crys_lit.png")
