"""Cliff face: a 160x32 panel (10 tile columns, wraps seamlessly) of fractured natural rock.

Rock is split into chunks by slanted joints and fractures; each chunk is shaded as a form lit from the
upper left: a lit top cap, a lit wedge-shaped plane on its west side, a dark body, 1px crevices on its
east and south edges. Rows 0-1: plateau top plane, 2-4: lip, 5-28: rock, 29-31: base.
A taller version inserts `insert` rows after row 15 (the mid strip) with its own fractures.
"""
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I  # noqa: E402

T = 16
PW, PH = 160, 32
Y0, Y1 = 5, 28
TOP_FILL = "w2"   # must match tops.FILL

JOINTS = [
    (0, (1, 8), Y0, Y1),
    (11, (1, 4), Y0, Y1),
    (19, (1, 8), Y0, 16),
    (27, (-1, 8), Y0, Y1),
    (38, (-1, 4), Y0, Y1),
    (46, (-1, 6), 12, Y1),
    (53, (-1, 8), Y0, Y1),
    (64, (1, 6), Y0, Y1),
    (72, (-1, 6), Y0, 14),
    (79, (1, 8), Y0, Y1),
    (90, (-1, 6), Y0, Y1),
    (97, (-1, 6), 15, Y1),
    (104, (-1, 8), Y0, Y1),
    (115, (1, 6), Y0, Y1),
    (124, (-1, 8), Y0, 18),
    (131, (-1, 4), Y0, Y1),
    (142, (1, 6), Y0, Y1),
    (150, (1, 8), Y0, 20),
]
FRACTURES = [
    (0, 17, (1, 4)), (1, 12, (-1, 3)), (2, 21, (-1, 5)), (3, 9, (1, 6)), (4, 19, (-1, 4)),
    (6, 13, (1, 3)), (7, 22, (1, 5)), (9, 11, (1, 4)), (9, 23, (-1, 6)), (10, 16, (1, 6)),
    (12, 10, (-1, 4)), (13, 20, (1, 3)), (15, 14, (-1, 5)), (16, 22, (-1, 3)), (17, 9, (1, 4)),
]
# extra fractures inside the inserted mid rows: (joint index, row within mid, slope)
MID_FRACS = [(1, 6, (1, 4)), (4, 10, (-1, 3)), (7, 4, (1, 6)), (10, 9, (1, 5)), (13, 7, (-1, 4)), (16, 11, (1, 3))]
CRACK_X = 124       # run-breaker: deep crevice in tile column 7
VEIN = [(84, 9), (85, 10), (85, 11), (86, 12), (86, 13), (87, 14), (87, 15), (88, 16), (88, 17)]  # tile column 5
JS = sorted(JOINTS, key=lambda j: j[0])


def jx(j, y):
    x0, (n, d), r0, r1 = j
    return x0 + math.floor((y - r0) * n / d + 0.5)


def orig_row(yo, insert):
    if yo < 16:
        return yo
    if yo < 16 + insert:
        return 15
    return yo - insert


def label_map(insert=0):
    H = PH + insert
    lab = np.full((H, PW), -1, np.int32)
    for yo in range(H):
        y = orig_row(yo, insert)
        if not (Y0 <= y <= Y1):
            continue
        act = sorted((jx(JS[k], y) % PW, k) for k in range(len(JS)) if JS[k][2] <= y <= JS[k][3])
        for i, (xa, k) in enumerate(act):
            xb = act[(i + 1) % len(act)][0]
            for dx in range((xb - xa) % PW):
                lab[yo, (xa + dx) % PW] = k * 16
    col = lab // 16
    for (k, row, (n, d)) in FRACTURES:
        xs = jx(JS[k], row) % PW
        for yo in range(H):
            y = orig_row(yo, insert)
            for x in np.where((col[yo] == k) & (lab[yo] >= 0))[0]:
                if y > row + math.floor(((x - xs) % PW) * n / d + 0.5):
                    lab[yo, x] += 1
    if insert:
        for (k, row, (n, d)) in MID_FRACS:
            xs = jx(JS[k], 15) % PW
            for yo in range(16, H):
                for x in np.where((col[yo] == k) & (lab[yo] >= 0))[0]:
                    if yo >= 16 + insert or yo - 16 > row + math.floor(((x - xs) % PW) * n / d + 0.5):
                        lab[yo, x] += 4
    return lab


def shade(lab):
    h, w = lab.shape
    c = np.full((h, w), I["w1"], np.int32)
    for y in range(h):
        for x in range(w):
            L = lab[y, x]
            if L < 0:
                continue
            right = lab[y, (x + 1) % w]
            below = lab[y + 1, x] if y + 1 < h and lab[y + 1, x] >= 0 else L
            left = lab[y, (x - 1) % w]
            k = 0
            while y - k - 1 >= 0 and lab[y - k - 1, x] == L:
                k += 1
            bx = 0
            while lab[y, (x - bx - 1) % w] == L and bx < w:
                bx += 1
            ex = 0
            while lab[y, (x + ex + 1) % w] == L and ex < w:
                ex += 1
            width = bx + ex + 1
            big = width >= 7
            lit_h = (7 if big else 4) - bx // 3
            if right != L or below != L:
                c[y, x] = I["w0"]
            elif k == 0:
                c[y, x] = I["w3"] if (left == L and big) else I["w2"]
            elif bx == 0 and k < (6 if big else 3):
                c[y, x] = I["w3"] if big else I["w2"]
            elif k < lit_h and bx < width * 0.6:
                c[y, x] = I["w2"]
            else:
                c[y, x] = I["w1"]
    return c


def panel(insert=0, breakers=False):
    lab = label_map(insert)
    c = shade(lab)
    H = PH + insert
    rock_rows = [yo for yo in range(H) if Y0 <= orig_row(yo, insert) <= Y1]
    if breakers:
        for yo in rock_rows:
            y = orig_row(yo, insert)
            c[yo, CRACK_X] = I["w0"]
            c[yo, CRACK_X + 1] = I["w0"]
            if 9 <= y <= 24:
                c[yo, CRACK_X - 1] = I["w0"]
        pts = []
        for (x, y) in VEIN:
            if y < 16:
                pts.append((x, y))
            else:
                pts.append((x, y + insert))
        for yo in range(16, 16 + insert):          # vein continues straight through the mid strip
            pts.append((87, yo))
        for (x, yo) in pts:
            c[yo, x] = I["a1"]
        for (x, yo) in [(86, 12), (87, 14), (88, 16 + insert)]:
            c[yo, x + 1] = I["a1"]
        c[12, 86] = I["a2"]
        c[14, 87] = I["a2"]
        c[11, 85] = I["a3"]     # the only two emissive pixels
        c[13, 86] = I["a3"]
    # lip
    c[0:2, :] = I[TOP_FILL]
    c[2, :] = I["w3"]
    c[3, :] = I["w1"]
    c[4, :] = I["w0"]
    rng = np.random.default_rng(17)
    x = 0
    while x < PW:
        L = int(rng.integers(3, 8))
        kind = rng.choice(["hi", "plain", "plain", "notch", "thick"])
        for i in range(L):
            xx = (x + i) % PW
            if kind == "hi" and 0 < i < L - 1:
                c[2, xx] = I["w4"]
            elif kind == "notch":
                c[2, xx] = I["w2"] if 0 < i < L - 1 else c[2, xx]
            elif kind == "thick":
                c[3, xx] = I["w3"] if 0 < i < L - 1 else I["w2"]
        x += L + int(rng.integers(2, 7))
    for x in range(PW):
        if c[Y0, x] == I["w3"] and c[Y0, (x - 1) % PW] == I["w3"]:
            c[4, x] = I["w1"]
    # base
    yb = Y1 + insert
    for x in range(PW):
        if c[yb, x] != I["w0"]:
            c[yb, x] = I["w1"]
    c[yb + 1, :] = I["w0"]
    c[yb + 2, :] = I["w0"]
    c[yb + 3, :] = I["r1"]
    rub = [(9, [".xw.", "xWww"]), (36, [".xW.", "xWww"]), (97, [".xWw", "xWwww"]), (140, [".x.", "xWw"])]
    key = {"x": "w3", "W": "w2", "w": "w1"}
    for (x0, rows) in rub:
        for j, r in enumerate(rows):
            for i, ch in enumerate(r):
                if ch != ".":
                    c[yb + 1 + j - (len(rows) - 2), (x0 + i) % PW] = I[key[ch]]
    return c


def make_end(c, part, side):
    """Face tile at the west (side 'l', floor to the west) or east ('r') end of a run. -1 = floor shows."""
    c = c.copy()
    if side == "l":
        for y in range(T):
            if part == "top":
                c[y, 0] = I["w3"] if y < 11 else I["w2"]
                if y < 2:
                    c[y, 1] = I["w3"]
                if y == 4:
                    c[y, 0] = I["w2"]
            elif part == "mid":
                c[y, 0] = I["w2"]
            else:
                c[y, 0] = I["w2"] if y < 13 else I["w0"]
            if c[y, 1] == I["w0"] and part != "top":
                c[y, 1] = I["w1"]
        if part == "bot":
            c[15, 0:2] = -1
            c[14, 0] = -1
        if part == "top":
            c[0, 0] = -1
    else:
        for y in range(T):
            c[y, T - 1] = I["w0"]
            if c[y, T - 2] in (I["w3"], I["w4"]):
                c[y, T - 2] = I["w2"]
            if part == "top" and y < 2:
                c[y, T - 2] = I["w1"]
        if part == "bot":
            c[15, T - 2:] = I["r2"]
            c[14, T - 1] = I["r2"]
    return c


def face_tiles():
    """{name: 16x16} for cliff_face_{top,mid,bot}[_l|_r]_{0..9} plus run-breaker alternates."""
    base = panel(0, False)
    tall = panel(16, False)
    base_b = panel(0, True)
    tall_b = panel(16, True)
    out = {}
    for v in range(PW // T):
        xs = slice(v * T, (v + 1) * T)
        parts = {"top": base[0:T, xs], "mid": tall[T:2 * T, xs], "bot": base[T:2 * T, xs]}
        for part, c in parts.items():
            out[f"cliff_face_{part}_{v}"] = c.copy()
            out[f"cliff_face_{part}_l_{v}"] = make_end(c, part, "l")
            out[f"cliff_face_{part}_r_{v}"] = make_end(c, part, "r")
        for (bv, tag) in ((5, "vein"), (7, "crack")):
            if v == bv:
                out[f"cliff_face_top_{v}_{tag}"] = base_b[0:T, xs].copy()
                out[f"cliff_face_mid_{v}_{tag}"] = tall_b[T:2 * T, xs].copy()
                out[f"cliff_face_bot_{v}_{tag}"] = base_b[T:2 * T, xs].copy()
    return out


if __name__ == "__main__":
    from px import on_bg, upscale
    p = panel(0, True)
    t = panel(16, False)
    fl = np.full((PH + 16 + 6, PW * 2 + 4), I["r3"], np.int32)
    fl[0:PH, 0:PW] = p
    fl[0:PH + 16, PW + 4:PW * 2 + 4] = t
    upscale(on_bg(fl, "#000000", pad=0), 3).save("/tmp/gw_face9.png")
