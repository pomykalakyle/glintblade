"""Cliff face v5: a 48x32 panel (3 horizontal variants x 2 tiles tall), rock masses lit from upper left.
Rows 0-3: top surface + lip. Row 4: overhang shadow. Rows 5-28: masses. Rows 29-31: base."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, on_bg, upscale

W, H = 48, 32

# masses: (x0, x1, lit_rows_left, lit_rows_right, ledge_row or None, ledge_x0, ledge_x1)
MASSES = [
    (0, 12, 15, 8, 18, 3, 9),
    (14, 20, 11, 6, None, 0, 0),
    (23, 33, 17, 9, 13, 3, 8),
    (35, 41, 13, 7, None, 0, 0),
    (43, 46, 9, 6, None, 0, 0),
]
CREVICE_WIDE = {21: (12, 31), 22: (6, 31)}  # x: (from row, to row) for a 2px deep crevice


def panel():
    c = np.full((H, W), I["w1"], np.int32)
    for (x0, x1, ll, lr, ledge, lx0, lx1) in MASSES:
        wdt = x1 - x0 + 1
        for x in range(x0, x1 + 1):
            t = (x - x0) / max(wdt - 1, 1)
            lit_end = 5 + round(ll + (lr - ll) * t)
            for y in range(5, lit_end):
                c[y, x] = I["w2"]
        # lit top edge (rounded shoulder): row 5 from x0+1, left column from row 6
        for x in range(x0 + 1, x0 + max(3, wdt // 2)):
            c[5, x] = I["w3"]
        for y in range(6, 5 + max(4, ll // 2)):
            c[y, x0] = I["w3"]
        c[5, x0] = I["w2"]  # round the corner
        # shadow side: last column before the crevice stays w1 (already), crevice
        for y in range(4, 30):
            c[y, (x1 + 1) % W] = I["w0"]
        # ledge: crack with a lit step under it
        if ledge:
            for x in range(lx0 + x0 - x0, lx1 + 1):
                xx = x0 + (x - x0) if False else x
            for x in range(x0 + lx0, x0 + lx1 + 1):
                c[ledge, x] = I["w0"]
                c[ledge + 1, x] = I["w2"]
            c[ledge + 1, x0 + lx0] = I["w3"]
            c[ledge + 1, x0 + lx0 + 1] = I["w3"]
    for x, (y0, y1) in CREVICE_WIDE.items():
        for y in range(y0, y1):
            c[y, x] = I["w0"]
    # lip
    c[0:3, :] = I["r2"]
    c[2, :] = I["r4"]
    for x in range(W):
        if x % 12 in (3, 4, 5):
            c[2, x] = I["r5"]
        if x % 12 in (8,):
            c[1, x] = I["r4"]
    c[3, :] = I["r3"]
    c[4, :] = I["w0"]
    # masses poke up into the overhang shadow at their lit shoulders
    for (x0, x1, *_r) in MASSES:
        for x in range(x0 + 2, x0 + 4):
            c[4, x] = I["w1"]
    # base: masses end in rounded bottoms, contact shadow
    for x in range(W):
        c[29, x] = I["w1"] if c[29, x] != I["w0"] else I["w0"]
        c[30, x] = I["w0"]
        c[31, x] = I["r1"]
    for (x0, x1, *_r) in MASSES:
        for x in range(x0 + 1, x1):
            c[30, x] = I["w1"]
        c[29, x0] = I["w0"]
    return c

if __name__ == "__main__":
    c = panel()
    big = np.tile(c, (1, 2))
    # place over floor to judge
    fl = np.full((H + 12, W * 2 + 8), I["r3"], np.int32)
    fl[0:H, 4:4 + W * 2] = big
    upscale(on_bg(fl, "#000000", pad=0), 6).save("/tmp/gw_face5.png")
