"""Corner-based (marching squares) transition masks, 16x16, seam-safe.

mask string = corners NW NE SW SE, '1' = the second terrain (wet / water).
Field = smoothstep-bilinear of the corners + a wobble that vanishes on tile borders, so every
boundary crosses a tile edge at its midpoint, perpendicular to it.
"""
import math
import numpy as np

T = 16
MASKS = [f"{i:04b}" for i in range(16)]


def ss(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def field(mask, wob_seed=0, wob_amp=0.10, margin=4):
    """Field over the tile plus a margin (extrapolated). Returns array (T+2m, T+2m) and the margin."""
    nw, ne, sw, se = (int(ch) for ch in mask)
    n = T + 2 * margin
    yy, xx = np.mgrid[0:n, 0:n].astype(float)
    # pixel centres in tile coords
    u = (xx - margin + 0.5) / T
    v = (yy - margin + 0.5) / T
    su, sv = ss(u), ss(v)
    f = nw * (1 - su) * (1 - sv) + ne * su * (1 - sv) + sw * (1 - su) * sv + se * su * sv
    if wob_amp:
        rng = np.random.default_rng(wob_seed + int(mask, 2) * 7)
        w = np.zeros_like(f)
        for k in range(3):
            kx, ky = rng.integers(1, 3, 2)
            w += np.sin(math.pi * kx * np.clip(u, 0, 1)) * np.sin(math.pi * ky * np.clip(v, 0, 1)) * rng.uniform(-1, 1)
        # vanish on borders (sin terms already 0 at u,v in {0,1})
        f = f + wob_amp * w
    return f, margin


def mask_px(mask, thr=0.5, **kw):
    f, m = field(mask, **kw)
    return f > thr, m
