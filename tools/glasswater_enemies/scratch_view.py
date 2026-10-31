"""Quick look: render a few grids side by side at 12x (flat + lit-sim) for iteration."""

import sys

import numpy as np
from PIL import Image

sys.path.insert(0, __import__("os").path.dirname(__file__))
from pix import GROUND, Grid, _bg, emissive_mask, hex2rgb, lit, over, upscale  # noqa: E402


def view(grids, pal, emissive_keys, path, scale=12, pad=2, light=True):
    w = sum(g.w for g in grids) + pad * (len(grids) + 1)
    h = max(g.h for g in grids) + pad * 2
    canvas = np.zeros((h, w, 4), np.uint8)
    x = pad
    for g in grids:
        over(canvas, g.to_rgba(pal), x, pad)
        x += g.w + pad
    flat = _bg(w, h, GROUND)
    over(flat, canvas, 0, 0)
    rows = [upscale(flat, scale)[..., :3]]
    if light:
        fl = _bg(w, h, "#171b35")
        over(fl, canvas, 0, 0)
        em = {pal[k].lower() for k in emissive_keys}
        m = emissive_mask(fl, em)
        rows.append(lit(fl, m, light_pos=(w * 0.15, h * 0.2), light_radius=max(48, w * 0.9), scale=scale))
        rows.append(lit(fl, m, light_pos=None, scale=scale))
    # 1x and 3x strip on the lit-sim (no light) and flat ground
    W = rows[0].shape[1]
    strip = np.zeros((h * 3 + 8, W, 3), np.uint8)
    strip[...] = hex2rgb("#0e1022")
    s1 = _bg(w, h, GROUND); over(s1, canvas, 0, 0)
    strip[4:4 + h, 4:4 + w] = s1[..., :3]
    s3 = upscale(s1, 3)[..., :3]
    if 12 + w + 3 * w <= W:
        strip[4:4 + 3 * h, 12 + w:12 + w + 3 * w] = s3
    rows.append(strip)
    img = np.concatenate(rows, 0)
    Image.fromarray(img.astype(np.uint8), "RGB").save(path)
