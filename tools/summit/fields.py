"""Seamless corner fields for wang-style transitions (shared by tiles.py and edges.py)."""

import math
import random

import numpy as np

T = 16


def pnoise(x, y, seed, period=16, cells=4):
    rng = random.Random(seed)
    g = [[rng.uniform(-1, 1) for _ in range(cells)] for _ in range(cells)]

    def smooth(t):
        return t * t * (3 - 2 * t)

    fx = (x / period) * cells
    fy = (y / period) * cells
    ix, iy = int(math.floor(fx)), int(math.floor(fy))
    tx, ty = smooth(fx - ix), smooth(fy - iy)
    a = g[iy % cells][ix % cells]
    b = g[iy % cells][(ix + 1) % cells]
    c = g[(iy + 1) % cells][ix % cells]
    d = g[(iy + 1) % cells][(ix + 1) % cells]
    return (a * (1 - tx) + b * tx) * (1 - ty) + (c * (1 - tx) + d * tx) * ty


def corner_field(code, x, y):
    tl, tr, bl, br = [1.0 if ch == "1" else 0.0 for ch in code]
    u = (x + 0.5) / T
    v = (y + 0.5) / T
    return (tl * (1 - u) + tr * u) * (1 - v) + (bl * (1 - u) + br * u) * v


def field(code, x, y, seed, variant=0):
    v = corner_field(code, x, y) + pnoise(x, y, seed, cells=2) * 0.11 + pnoise(x, y, seed + 5, cells=4) * 0.04
    if variant:
        # extra wobble that fades to nothing at the tile borders, so variants stay seamless
        wx = math.sin(math.pi * min(max(x + 0.5, 0), T) / T)
        wy = math.sin(math.pi * min(max(y + 0.5, 0), T) / T)
        v += pnoise(x, y, seed + 31 * variant, cells=2) * 0.30 * wx * wy
    return v


def flag_mask(code, seed=11, variant=0):
    m = np.zeros((T + 2, T + 2), bool)  # 1 px margin so rims at tile borders are decided right
    for y in range(-1, T + 1):
        for x in range(-1, T + 1):
            m[y + 1, x + 1] = field(code, x, y, seed, variant) > 0.5
    # majority clean-up: no lonely pixels or one-pixel notches
    for _ in range(2):
        n = m.copy()
        for y in range(1, T + 1):
            for x in range(1, T + 1):
                s = m[y - 1:y + 2, x - 1:x + 2].sum() - m[y, x]
                if m[y, x] and s <= 2:
                    n[y, x] = False
                elif not m[y, x] and s >= 6:
                    n[y, x] = True
        m = n
    return m
