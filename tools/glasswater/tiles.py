"""Glasswater Hollow ground tiles (16x16): cave floor, wet stone, shallow water + shore, raised rock cliffs.

Run: uv run --no-project --with pillow --with numpy python3 tools/glasswater/tiles.py
"""

import math
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(__file__))
from px import IDX, NAMES, PREVIEWS, ascii_grid, blank, emissive_hex, on_bg, save_sheet, upscale, to_rgba  # noqa: E402

T = 16
I = IDX


def periodic_noise(n, period, seed, octaves=((2, 1.0), (4, 0.5), (8, 0.25))):
    """Smooth noise on an n x n grid that wraps with `period` (n must be a multiple of period)."""
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:n, 0:n].astype(np.float64)
    out = np.zeros((n, n))
    for freq, amp in octaves:
        for _ in range(4):
            kx, ky = rng.integers(-freq, freq + 1, 2)
            if kx == 0 and ky == 0:
                kx = freq
            ph = rng.uniform(0, 2 * math.pi)
            out += amp * np.cos(2 * math.pi * (kx * xx + ky * yy) / period + ph)
    out -= out.min()
    out /= out.max()
    return out


def clean_mask(m, min_size=6, wrap=True):
    """Remove islands and holes smaller than min_size from a boolean mask (4-connected), wrap-aware."""
    h, w = m.shape
    m = m.copy()
    for target in (True, False):
        seen = np.zeros_like(m)
        for y in range(h):
            for x in range(w):
                if seen[y, x] or m[y, x] != target:
                    continue
                stack = [(y, x)]
                seen[y, x] = True
                comp = []
                while stack:
                    cy, cx = stack.pop()
                    comp.append((cy, cx))
                    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        ny, nx = cy + dy, cx + dx
                        if wrap:
                            ny %= h
                            nx %= w
                        elif not (0 <= ny < h and 0 <= nx < w):
                            continue
                        if not seen[ny, nx] and m[ny, nx] == target:
                            seen[ny, nx] = True
                            stack.append((ny, nx))
                if len(comp) < min_size:
                    for cy, cx in comp:
                        m[cy, cx] = not target
    return m


def smooth_mask(m, iters=2, wrap=True):
    """Majority filter: removes single-pixel jags so edges step cleanly."""
    for _ in range(iters):
        s = np.zeros(m.shape, np.int32)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                s += np.roll(np.roll(m, dy, 0), dx, 1) if wrap else m  # wrap only
        m = s >= 5
    return m


def kill_orphans(c, wrap=True):
    """Replace pixels whose color matches none of their 4 neighbours with the most common neighbour."""
    h, w = c.shape
    out = c.copy()
    for y in range(h):
        for x in range(w):
            v = c[y, x]
            nb = []
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                ny, nx = y + dy, x + dx
                if wrap:
                    ny %= h
                    nx %= w
                elif not (0 <= ny < h and 0 <= nx < w):
                    continue
                nb.append(c[ny, nx])
            if v not in nb and nb:
                vals, counts = np.unique(nb, return_counts=True)
                out[y, x] = vals[np.argmax(counts)]
    return out


# ------------------------------------------------------------------ floor

def floor_block(seed=3):
    """64x64 seamless cave floor: calm r3 stone, a few darker damp hollows, cracks and pebbles."""
    n = 64
    c = np.full((n, n), I["r3"], np.int32)
    nz = periodic_noise(n, n, seed, octaves=((1, 1.0), (2, 0.7), (4, 0.25)))
    hollow = nz > 0.66
    hollow = smooth_mask(hollow, 3)
    hollow = clean_mask(hollow, 20)
    c[hollow] = I["r2"]
    # a soft one-pixel rim on the lower-right inside edge of each hollow: the hollow's far wall catches light
    # (light from upper left) -> lighter r4 where hollow meets stone below/right of it.
    for y in range(n):
        for x in range(n):
            if hollow[y, x]:
                continue
            up = hollow[(y - 1) % n, x]
            left = hollow[y, (x - 1) % n]
            if up and left and hollow[(y - 1) % n, (x - 1) % n]:
                pass
    return c, hollow


def draw_cracks(c, cracks):
    """cracks: list of polylines [(x, y), ...] drawn with r1 and a sparse r4 lit lip below-right."""
    n = c.shape[0]
    for pts in cracks:
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            steps = max(abs(x1 - x0), abs(y1 - y0))
            for s in range(steps + 1):
                x = round(x0 + (x1 - x0) * s / steps) % n
                y = round(y0 + (y1 - y0) * s / steps) % n
                c[y, x] = I["r1"]


def build_floor():
    c, hollow = floor_block(seed=11)
    n = 64
    # cracks: hand-placed, clean slopes (2:1, 1:1, straight runs), short branches
    cracks = [
        [(6, 9), (10, 9), (12, 11), (13, 13), (17, 13)],
        [(12, 11), (12, 15)],
        [(40, 5), (43, 5), (45, 7), (49, 7), (50, 8)],
        [(26, 40), (28, 40), (30, 42), (30, 45), (32, 47), (35, 47)],
        [(30, 45), (27, 47)],
        [(52, 50), (55, 50), (57, 52), (61, 52)],
        [(3, 34), (6, 34), (7, 35)],
    ]
    draw_cracks(c, cracks)
    # light lip under each crack pixel that sits on bright stone, only where the crack runs horizontally
    lip = np.zeros_like(c, dtype=bool)
    for y in range(n):
        for x in range(n):
            if c[y, x] == I["r1"]:
                b = ((y + 1) % n, x)
                if c[b] == I["r3"] and c[(y + 1) % n, (x + 1) % n] != I["r1"]:
                    lip[b] = True
    c[lip & (np.arange(n)[None, :] % 2 == 0)] = I["r4"]
    # pebbles: (x, y, kind)
    pebbles = [(22, 6, 0), (55, 20, 1), (8, 24, 2), (36, 28, 0), (47, 37, 2), (14, 52, 1), (60, 60, 0), (33, 60, 2)]
    shapes = {
        0: ["Lh", "hd"],
        1: [".Lh", "Lhd", ".dd"],
        2: ["L."],
    }
    for x, y, k in pebbles:
        rows = shapes[k]
        for j, r in enumerate(rows):
            for i, ch in enumerate(r):
                if ch == ".":
                    continue
                col = {"L": "r5", "h": "r4", "d": "r2"}[ch]
                c[(y + j) % n, (x + i) % n] = I[col]
    return c


def tiles_from_block(block):
    n = block.shape[0] // T
    out = {}
    for ty in range(n):
        for tx in range(n):
            out[(tx, ty)] = block[ty * T:(ty + 1) * T, tx * T:(tx + 1) * T].copy()
    return out


def main():
    floor = build_floor()
    img = on_bg(floor, "#0e1022", pad=0)
    # 2x2 repeats to check seams
    big = np.tile(floor, (2, 3))
    upscale(on_bg(big, "#000000", pad=0), 4).save(os.path.join(PREVIEWS, "_wip_glasswater_floor.png"))
    print("ok")


if __name__ == "__main__":
    main()
