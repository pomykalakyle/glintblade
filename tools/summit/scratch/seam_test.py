import random, sys
sys.path.insert(0, "..")
import numpy as np
from common import *

def floor(W, H, style, seed=1):
    rng = random.Random(seed)
    img = blank(W, H)
    grout = style.get("grout", "b")
    for y in range(H):
        for x in range(W):
            put(img, x, y, grout)
    # random big slabs via a simple row-based packing (courses of varying height)
    y = 0
    while y < H:
        ch = rng.choice([16, 16, 24, 32])
        x = 0
        while x < W:
            cw = rng.choice([16, 24, 32, 32, 40])
            x0, y0, x1, y1 = x + 1, y + 1, min(W - 1, x + cw - 1), min(H - 1, y + ch - 1)
            tone = rng.choice(style["tones"])
            for yy in range(y0, y1 + 1):
                for xx in range(x0, x1 + 1):
                    put(img, xx, yy, tone)
            hl = style.get("hl", 1.0)
            for xx in range(x0 + 1, x0 + 1 + int((x1 - x0 - 1) * hl)):
                put(img, xx, y0, "e")
            if style.get("left_hl"):
                for yy in range(y0 + 1, y0 + 1 + (y1 - y0) // 2):
                    put(img, x0, yy, "e" if style["left_hl"] == 1 else "E")
            if style.get("face"):
                for xx in range(x0, x1 + 1):
                    put(img, xx, y1, "c")
            x += cw
        y += ch
    return img

styles = [
    dict(tones=["d", "d", "D", "E"], face=True, hl=1.0),
    dict(tones=["d", "d", "d", "D"], face=True, hl=0.6),
    dict(tones=["d", "d", "d", "D"], face=False, hl=0.7, grout="c"),
    dict(tones=["d", "d", "D"], face=True, hl=0.8, left_hl=2),
]
out = np.zeros((2 * 96 + 8, 2 * 160 + 8, 3), np.uint8)
for i, s in enumerate(styles):
    f = floor(160, 96, s, seed=i + 2)
    over(out, f, (i % 2) * 168, (i // 2) * 104)
save_rgb("../../../assets/previews/_summit_seamtest.png", out, 3)
