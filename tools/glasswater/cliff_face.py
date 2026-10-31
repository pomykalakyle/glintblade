"""Shading pass for the hand-drawn face block map -> 48x32 panel + 48x16 mid strip."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I
from face_labels import ROWS, LIT

PW, PH = 48, 32
Y0 = 5


def labels():
    lab = np.full((PH, PW), "#", dtype="<U1")
    lab[0:Y0, :] = "^"
    for j, r in enumerate(ROWS):
        assert len(r) == PW, (j, len(r))
        lab[Y0 + j, :] = list(r)
    lab[Y0 + len(ROWS):, :] = "_"
    return lab


def shade_blocks(lab, wrap_y=False):
    h, w = lab.shape
    c = np.full((h, w), I["w1"], np.int32)
    for y in range(h):
        for x in range(w):
            L = lab[y, x]
            if L not in LIT:
                continue
            right = lab[y, (x + 1) % w]
            below = lab[(y + 1) % h, x] if (wrap_y or y + 1 < h) else L
            left = lab[y, (x - 1) % w]
            # rows since this block began in this column
            k = 0
            while True:
                yy = y - k - 1
                if yy < 0:
                    if wrap_y:
                        yy %= h
                    else:
                        break
                if lab[yy, x] != L or k > h:
                    break
                k += 1
            lit_h, edge_h = LIT[L]
            # lit plane is a wedge: one row shorter every 4 px to the right of the block's west edge
            bx = x
            while lab[y, (bx - 1) % w] == L and x - bx < w:
                bx -= 1
            lit_h = max(1, lit_h - (x - bx) // 4)
            if right != L or below != L:
                c[y, x] = I["w0"]                          # crevice / fracture
            elif k == 0:
                c[y, x] = I["w3"] if left == L else I["w2"]  # top cap catches light; round its left corner
            elif left != L and k < edge_h:
                c[y, x] = I["w3"]                          # lit west edge
            elif k < lit_h:
                c[y, x] = I["w2"]
            else:
                c[y, x] = I["w1"]
    c[lab == "."] = I["w0"]
    return c


def face_panel():
    lab = labels()
    c = shade_blocks(lab)
    # round bottom corners: a block pixel whose below is '.'/'_' and left or right differs -> w0
    for y in range(Y0, PH - 3):
        for x in range(PW):
            L = lab[y, x]
            if L in LIT and lab[y + 1, x] in "._" and (lab[y, (x - 1) % PW] != L or lab[y, (x + 1) % PW] != L):
                c[y, x] = I["w0"]
    # bottom rows of blocks just above the base get a darker last row (w1 already) - ok
    # specular accents (rare): top-left of the three biggest blocks
    for (x, y) in [(1, 6), (20, 6), (34, 6)]:
        c[y, x] = I["w4"]
    # lip
    c[0:2, :] = I["w1"]
    c[2, :] = I["w3"]
    c[3, :] = I["w2"]
    c[4, :] = I["w0"]
    for x in range(PW):
        if x in (5, 6, 7, 8, 26, 27, 28, 36, 37):
            c[1, x] = I["w3"]
            c[2, x] = I["w4"] if x in (6, 7, 27) else I["w3"]
        if x in (16, 17, 44):
            c[3, x] = I["w0"]
    # base
    c[29, :] = np.where(np.isin(lab[29, :], list(LIT)), c[29, :], I["w0"])
    c[29, :] = I["w0"]
    c[30, :] = I["w0"]
    c[31, :] = I["r1"]
    # rubble at the foot
    RUB = [(11, 28, ["lm", "dd"]), (18, 28, ["lmd", "ddd"]), (32, 27, [".l", "lm", "dd"]), (46, 28, ["md", "dd"]),
           (40, 28, ["l", "d"])]
    key = {"l": "w3", "m": "w2", "d": "w1"}
    for (x, y, rows) in RUB:
        for j, r in enumerate(rows):
            for i, ch in enumerate(r):
                if ch != ".":
                    c[y + j, (x + i) % PW] = I[key[ch]]
    return c


def mid_strip():
    """Repeat panel row 16's profile, with a fracture + lit cap in two blocks."""
    lab = labels()
    prof = lab[16, :]
    m = np.tile(prof, (16, 1))
    # a fracture through G (x 0..11) at rows 7..: new block 'g' below; through D at rows 3..
    m[8:, 0:12] = "a"  # lower part of G becomes block a
    m[4:, 33:41] = "b"
    m[11:, 19:33] = "c"
    LIT.update({"a": (2, 2), "b": (2, 2), "c": (2, 2)})
    # keep row 0 and 15 equal to prof for seamless stacking: blocks must not start at row 0/end at 15
    c = shade_blocks(m, wrap_y=False)
    # the strip's first row continues the block above (no cap): recolour as body
    return c


if __name__ == "__main__":
    from px import on_bg, upscale
    p = face_panel()
    ms = mid_strip()
    tall = np.concatenate([p[:16], ms, ms, p[16:]], 0)
    both = np.concatenate([np.tile(p, (1, 2)), np.full((32, 4), I["r3"]), np.tile(tall[:32], (1, 1))], 1)
    fl = np.full((tall.shape[0] + 8, 48 * 3 + 16), I["r3"], np.int32)
    fl[0:32, 4:100] = np.tile(p, (1, 2))
    fl[0:tall.shape[0], 108:156] = tall
    upscale(on_bg(fl, "#000000", pad=0), 6).save("/tmp/gw_face7.png")
