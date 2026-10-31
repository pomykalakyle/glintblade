"""Cliff face v6: 48x32 panel with wandering crevices, irregular lip and rubble base."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, on_bg, upscale

W, H = 48, 32
TOP = 4      # first face row (overhang shadow row)
BOT = 29     # last face row before the base


def path(spec):
    """spec: list of (from_row, x); returns dict row -> x for rows TOP..BOT."""
    out = {}
    spec = sorted(spec)
    for i, (r, x) in enumerate(spec):
        r1 = spec[i + 1][0] if i + 1 < len(spec) else BOT + 1
        for y in range(r, r1):
            out[y] = x
    return out

# crevices (the column of w0 between masses); a mass lies between consecutive crevices.
CREV = [
    path([(TOP, 47), (BOT + 1, 47)]),                                   # wraps: mass A starts at 0
    path([(TOP, 12), (11, 13), (20, 12), (25, 11)]),
    path([(TOP, 20), (15, 21)]),
    path([(TOP, 33), (13, 32), (22, 33)]),
    path([(TOP, 40), (16, 41)]),
]
# extra width on the right of a crevice: (crevice index, from row) -> crevice is 2px wide below that row
WIDE = {1: 24, 2: 18, 4: 26}
# per mass (between CREV[i] and CREV[i+1]): (lit rows at left edge, lit rows at right edge, mass bottom row)
MASS = [
    (17, 7, 28),
    (12, 5, 29),
    (19, 8, 28),
    (12, 6, 29),
    (10, 6, 28),
]
LEDGES = [(2, 14, 26, 30), (0, 19, 4, 9)]   # (mass, row, x0, x1) crack with lit step below


def panel():
    c = np.full((H, W), I["w1"], np.int32)
    n = len(CREV)
    for y in range(TOP, BOT + 1):
        for m in range(n):
            a = CREV[m][y] + 1 if m > 0 else 0
            b = CREV[(m + 1) % n][y] - 1 if m + 1 < n else CREV[0][y] - 1
            ll, lr, mb = MASS[m]
            width = b - a
            for x in range(a, b + 1):
                t = (x - a) / max(width, 1)
                lit_end = TOP + 1 + round(ll + (lr - ll) * t)
                if y > mb:
                    c[y, x] = I["w0"]
                elif y < lit_end:
                    c[y, x] = I["w2"]
                else:
                    c[y, x] = I["w1"]
            # mass outline details
            if y == TOP + 1:
                for x in range(a + 1, a + 4):
                    c[y, x] = I["w3"]
            if TOP + 2 <= y < TOP + 2 + ll // 2:
                c[y, a] = I["w3"]
            if y == mb:
                c[y, a] = I["w0"]; c[y, b] = I["w0"]
    for i, cv in enumerate(CREV):
        for y, x in cv.items():
            c[y, x % W] = I["w0"]
            if i in WIDE and y >= WIDE[i]:
                c[y, (x + 1) % W] = I["w0"]
    for (m, row, x0, x1) in LEDGES:
        for x in range(x0, x1 + 1):
            c[row, x] = I["w0"]
            c[row + 1, x] = I["w3"] if x < x0 + 2 else I["w2"]
    # lip: top surface rows 0-1, rim row 2 (bulges up a pixel in places), rounding row 3, shadow row 4
    c[0:2, :] = I["r2"]
    c[2, :] = I["r4"]
    c[3, :] = I["r3"]
    c[4, :] = I["w0"]
    for x in range(W):
        if x in (5, 6, 7, 8, 26, 27, 28, 36, 37):
            c[1, x] = I["r4"]; c[2, x] = I["r5"] if x in (6, 7, 27) else I["r4"]
        if x in (17, 18, 44, 45):
            c[3, x] = I["w0"]          # lip notch: shadow climbs a pixel
    # shoulders of masses poke into the shadow row
    for x in (2, 3, 24, 25, 35):
        c[4, x] = I["w2"]
    # base: dark contact row and rubble
    c[30, :] = I["w0"]
    c[31, :] = I["r1"]
    RUB = [(11, 28, 3), (21, 29, 2), (40, 29, 3), (30, 30, 2)]
    for (x, y, w) in RUB:
        for i in range(w):
            c[y, x + i] = I["w2"] if i < w - 1 else I["w1"]
            c[y + 1, x + i] = I["w1"]
        c[y, x] = I["w3"]
    return c

if __name__ == "__main__":
    c = panel()
    big = np.tile(c, (1, 2))
    fl = np.full((H + 12, W * 2 + 8), I["r3"], np.int32)
    fl[0:H, 4:4 + W * 2] = big
    upscale(on_bg(fl, "#000000", pad=0), 6).save("/tmp/gw_face6.png")
    np.save("/tmp/gw_panel.npy", c)
