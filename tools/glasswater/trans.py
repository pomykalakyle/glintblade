"""Shore (floor <-> shallow water) and wet-stone transition tiles."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I
from corner import MASKS, mask_px, T


def destair(w, iters=3):
    """Flip pixels with >=3 opposite 4-neighbours (kills 1px bumps/notches and alternating jitter)."""
    w = w.copy()
    for _ in range(iters):
        opp = np.zeros(w.shape, np.int32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nb = np.roll(np.roll(w, dy, 0), dx, 1)
            opp += (nb != w)
        flip = opp >= 3
        # don't touch the outer ring (roll wraps there)
        flip[0, :] = flip[-1, :] = flip[:, 0] = flip[:, -1] = False
        if not flip.any():
            break
        w[flip] = ~w[flip]
    return w


def shifted(a, dy, dx, fill=False):
    out = np.full_like(a, fill)
    h, w = a.shape
    ys = slice(max(dy, 0), h + min(dy, 0)); yd = slice(max(-dy, 0), h + min(-dy, 0))
    xs = slice(max(dx, 0), w + min(dx, 0)); xd = slice(max(-dx, 0), w + min(-dx, 0))
    out[ys, xs] = a[yd, xd]
    return out


def shore_px(w, ox):
    """Shade a water mask (with margin). ox = x of the tile's left edge in the array (for patterns)."""
    land = ~w
    c = np.full(w.shape, I["r2"], np.int32)   # land next to water is wet stone
    c[w] = I["q2"]
    n = w.shape[1]
    xs = (np.arange(n) - ox) % T
    # bank height varies along x (period 16 so seams match): 2 or 3 px
    bank_h = np.where(np.isin(xs, [3, 4, 5, 11, 12]), 3, 2)[None, :]
    land_above = [shifted(land, k, 0) for k in range(0, 5)]   # land_above[k]: pixel k rows up is land
    water_above1 = shifted(w, 1, 0)
    water_below1 = shifted(w, -1, 0)
    side_water = shifted(w, 0, 1) | shifted(w, 0, -1)
    # land: damp rim on the sides, lit lip on north-facing edges, dark lip on south-facing edges
    c[land & water_below1] = I["r3"]          # bank lip before the drop
    c[land & water_above1] = I["r3"]          # north-facing shore lip catches light
    # water: bank face under south-facing land edges, then a shadow line
    b1 = w & land_above[1]
    b2 = w & land_above[2] & ~b1 & (bank_h >= 3)
    sh = w & ~b1 & ~b2 & (land_above[2] | (land_above[3] & (bank_h >= 3)))
    c[b1] = I["r1"]
    c[b2] = I["r1"]
    c[sh] = I["q1"]
    # light shallow band just above lit lips (water north of land)
    land_below1 = shifted(land, -1, 0)
    lb = w & land_below1 & ~b1
    run = lb & (shifted(lb, 0, 1) | shifted(lb, 0, -1))
    c[run] = I["q3"]
    return c


def shore_tile(mask, seed=0):
    w, m = mask_px(mask, wob_seed=seed, wob_amp=0.07)
    w = destair(w)
    c = shore_px(w, m)
    return c[m:m + T, m:m + T]


def wet_px(w, ox, oy, seed, glints=1):
    c = np.full(w.shape, I["r3"], np.int32)
    c[w] = I["r2"]
    # glossy reflections: short horizontal dashes inside the wet area, fixed per tile position
    rng = np.random.default_rng(seed)
    for _ in range(glints):
        x = rng.integers(1, T - 4) + ox
        y = rng.integers(2, T - 2) + oy
        L = rng.integers(2, 4)
        if w[y, x - 1:x + L + 1].all() and w[y - 1, x:x + L].all() and w[y + 1, x:x + L].all():
            c[y, x:x + L] = I["q2"]
            if L == 3 and rng.random() < 0.5:
                c[y, x + 1] = I["q3"]
    # wet edge: where wet meets dry on the north side, a 1px darker line (water film edge)
    dry_above = shifted(~w, 1, 0)
    c[w & dry_above] = I["r1"]
    return c


def wet_tile(mask, seed=0, glints=None):
    w, m = mask_px(mask, wob_seed=seed + 50, wob_amp=0.08)
    w = destair(w)
    if glints is None:
        glints = 0
    c = wet_px(w, m, m, seed * 31 + int(mask, 2), glints)
    return c[m:m + T, m:m + T]


def assemble(fn, verts, seed=0):
    h, wdt = verts.shape
    out = np.zeros(((h - 1) * T, (wdt - 1) * T), np.int32)
    for ty in range(h - 1):
        for tx in range(wdt - 1):
            mk = f"{verts[ty, tx]}{verts[ty, tx + 1]}{verts[ty + 1, tx]}{verts[ty + 1, tx + 1]}"
            out[ty * T:(ty + 1) * T, tx * T:(tx + 1) * T] = fn(mk, seed)
    return out

if __name__ == "__main__":
    from px import on_bg, upscale, PREVIEWS
    rng = np.random.default_rng(3)
    verts = (rng.random((7, 11)) > 0.55).astype(int)
    verts[2:5, 3:8] = 1
    a = assemble(shore_tile, verts)
    b = assemble(wet_tile, verts)
    img = np.concatenate([a, np.full((a.shape[0], 4), I["o"]), b], 1)
    upscale(on_bg(img, "#000000", pad=0), 3).save(os.path.join(PREVIEWS, "_wip_trans.png"))
    upscale(on_bg(a[:, :160], "#000000", pad=0), 6).save("/tmp/gw_shore_zoom.png")
    upscale(on_bg(b[:, :160], "#000000", pad=0), 6).save("/tmp/gw_wet_zoom.png")
