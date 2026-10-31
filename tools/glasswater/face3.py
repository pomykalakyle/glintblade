"""Cliff face texture v3: faceted rock on a 48x16 torus (Voronoi, tall cells), flat colors per facet."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, on_bg, upscale, PREVIEWS

W, H = 48, 16

def facets(seed, pts=None, sx=1.0, sy=0.45):
    rng = np.random.default_rng(seed)
    if pts is None:
        pts = [(rng.uniform(0, W), rng.uniform(0, H)) for _ in range(13)]
    yy, xx = np.mgrid[0:H, 0:W].astype(float)
    ds = []
    for (px_, py_) in pts:
        dx = np.abs(xx + 0.5 - px_); dx = np.minimum(dx, W - dx)
        dy = np.abs(yy + 0.5 - py_); dy = np.minimum(dy, H - dy)
        ds.append(np.sqrt((dx * sx) ** 2 + (dy * sy) ** 2))
    ds = np.stack(ds)
    lab = np.argmin(ds, 0)
    return lab, pts

def shade(lab, seed, tones):
    rng = np.random.default_rng(seed)
    n = lab.max() + 1
    c = np.zeros(lab.shape, np.int32)
    for k in range(n):
        c[lab == k] = I[tones[k % len(tones)]]
    # crevice: pixel whose right or lower neighbour is another facet -> w0 on the boundary if it's
    # the lower/right side of a lit facet; plus a lit edge on the upper-left of each facet
    right = np.roll(lab, -1, 1); down = np.roll(lab, -1, 0)
    left = np.roll(lab, 1, 1); up = np.roll(lab, 1, 0)
    crev = (lab != right) | (lab != down)
    c[crev] = I["w0"]
    lit = ((lab != left) | (lab != up)) & ~crev
    lighter = {I["w1"]: I["w2"], I["w2"]: I["w3"], I["w3"]: I["w4"]}
    for y in range(H):
        for x in range(W):
            if lit[y, x] and rng.random() < 0.9:
                c[y, x] = lighter.get(c[y, x], c[y, x])
    return c

if __name__ == "__main__":
    rows = []
    for seed in (1, 2, 3, 4):
        lab, _ = facets(seed)
        tones = ["w2", "w1", "w2", "w3", "w1", "w2", "w2", "w1", "w3", "w2", "w1", "w2", "w2"]
        c = shade(lab, seed, tones)
        rows.append(np.tile(c, (2, 2)))
        rows.append(np.full((2, W * 2), I["o"]))
    img = np.concatenate(rows, 0)
    upscale(on_bg(img, "#000000", pad=0), 5).save("/tmp/gw_face3.png")
