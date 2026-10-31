"""Compose and shade the GLINTBLADE logo (big + small), with the sword "I".

Usage: build_logo.py [variants]   -> writes WIP comparison to /tmp
"""

import sys

import numpy as np

from common import rgb
from logo import from_rows, letters, letters_small

# ----------------------------------------------------------------- sword grids
# Keys: o outline, B brass light, b brass dark, g grip dark, h grip light,
#       1 blade #2bb8e0 (edge, not emissive), 2 #5ff2ff, 3 #d8fbff, 4 #ffffff
SWORD_KEY = {
    "o": "#1a1030", "B": "#f0c860", "b": "#9a6424", "g": "#1e1224", "h": "#5c3a44", "m": "#3a2232",
    "1": "#2bb8e0", "2": "#5ff2ff", "3": "#d8fbff", "4": "#ffffff",
}
SWORD_EMISSIVE = ["#5ff2ff", "#d8fbff", "#ffffff"]


def sword_big(H):
    """Sword for the big logo. Returns (rows, x_of_blade_left, y_of_cap_line).
    Width 21, blade inner cols 8..12. The crossguard sits just above the cap line."""
    top = [
        ".........ooo.........",
        "........oB3bo........",
        "........obbbo........",
        ".........obo.........",
        "........ohhmo........",
        "........ommgo........",
        "........ohhmo........",
        "........ommgo........",
        "........ohhmo........",
        ".oo.....ommgo.....oo.",
        "oBBoooooobbboooooobbo",
        "obBBBBBBB343BBBBbbbbo",
        ".obbbbbbbb3bbbbbbbbo.",
        "..oooooooooooooooooo.",
    ]
    blade = ["......o33421o........"] * (H + 3)
    tip = [
        "......o3342o.........",
        ".......o342o.........",
        ".......o34o..........",
        "........o4o..........",
        "........o3o..........",
        ".........o...........",
    ]
    rows = top + blade + tip
    return rows, 7, len(top)


def sword_small(H):
    """Sword for the small logo: width 11, blade inner cols 4..6."""
    top = [
        "....ooo....",
        "....o3o....",
        "....oho....",
        "....omo....",
        "oo..ogo..oo",
        "oBoooooooBo",
        "obBBB4bbbbo",
        ".ooooooooo.",
    ]
    blade = ["...o341o..."] * (H + 1)
    tip = [
        "...o34o....",
        "....o4o....",
        "....o3o....",
        ".....o.....",
    ]
    return top + blade + tip, 4, len(top)


# ----------------------------------------------------------------- composition

def compose(L, word, H, gap, sword_rows, sword_bx, sword_cap, blade_w, i_gaps=(0, 0), kern=None):
    """Lay out letters; returns (face mask, sword rows, sword pos, size, top margin)."""
    xs = []
    x = 0
    kern = kern or {}
    for i, ch in enumerate(word):
        if i > 0:
            x += kern.get(word[i - 1] + ch, 0)
        if ch == "I":
            x += i_gaps[0]
            xs.append(x)
            x += blade_w + gap + i_gaps[1]
        else:
            xs.append(x)
            x += L[ch].shape[1] + gap
    width = x - gap
    top = sword_cap  # sword rises this many rows above the cap line
    below = len(sword_rows) - sword_cap - 1 - H
    hgt = top + H + max(below, 0)
    face = np.zeros((hgt, width), bool)
    sx = sy = None
    for ch, x in zip(word, xs):
        if ch == "I":
            sx, sy = x - sword_bx, 0
        else:
            m = L[ch]
            face[top:top + H, x:x + m.shape[1]] |= m
    return face, (sx, sy), top


def shade(face, top, H, pal, depth, lit_edge=True, split=None):
    """Return dict of layers (masks) and a color image function."""
    h, w = face.shape
    pad = 2
    W, Hh = w + 2 * pad, h + 2 * pad + depth
    F = np.zeros((Hh, W), bool)
    F[pad:pad + h, pad:pad + w] = face
    ext = np.zeros_like(F)
    for d in range(1, depth + 1):
        ext[d:, :] |= F[:-d, :]
    ext &= ~F
    solid = F | ext
    out = np.zeros_like(solid)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            out |= np.roll(np.roll(solid, dy, 0), dx, 1)
    out &= ~solid
    img = np.zeros((Hh, W, 4), np.uint8)

    def paint(mask, col):
        img[mask, :3] = rgb(col)
        img[mask, 3] = 255

    paint(out, pal["outline"])
    # extrusion: first row after the face lighter, the rest darker
    ext1 = ext & np.roll(F, 1, 0)
    paint(ext, pal["ext_dark"])
    paint(ext1, pal["ext_light"])
    paint(F, pal["face"])
    if split is not None:
        yy = np.arange(Hh)[:, None]
        lower = F & (yy >= pad + top + split)
        paint(lower, pal["face_low"])
    if lit_edge:
        # lit edges: face pixels whose upper or left neighbor is not face
        up = ~np.roll(F, 1, 0)
        left = ~np.roll(F, 1, 1)
        edge = F & (up | left)
        # only the top edges of horizontal runs and left edges of stems, not single steps everywhere
        paint(edge & up, pal["lit"])
        paint(edge & left & ~up, pal["lit_side"])
    return img, pad


def stamp_rows(img, rows, key, x0, y0):
    H, W = img.shape[:2]
    for y, r in enumerate(rows):
        for x, c in enumerate(r):
            if c in key and 0 <= x0 + x < W and 0 <= y0 + y < H:
                img[y0 + y, x0 + x, :3] = rgb(key[c])
                img[y0 + y, x0 + x, 3] = 255


KERN_SMALL = {}  # every small pair keeps a 2 px face gap (LA included)
KERN_BIG = {"LA": -2, "GL": -1, "TB": -1, "AD": -1, "NT": -1}  # closest face gap 3 px, LA included

PALS = {
    "ivory": dict(outline="#1a1030", face="#e2d2c0", face_low="#b9aab0", lit="#fff2dc", lit_side="#fff2dc",
                  ext_light="#857a98", ext_dark="#544a72"),
    "moon": dict(outline="#0b0a1a", face="#d6d0f0", face_low="#aaa2d6", lit="#f2eeff", lit_side="#f2eeff",
                 ext_light="#6a62a8", ext_dark="#3a3468"),
    "ivory_cool": dict(outline="#0b0a1a", face="#e2d2c0", face_low="#c4b4b4", lit="#fff2dc", lit_side="#fff2dc",
                       ext_light="#6a62a8", ext_dark="#3a3468"),
}


def build_big(pal, depth=3, lit_edge=True, split=None):
    H = 26
    L = letters()
    rows, bx, cap = sword_big(H)
    face, (sx, sy), top = compose(L, "GLINTBLADE", H, 4, rows, bx, cap, 5, kern=KERN_BIG)
    img, pad = shade(face, top, H, pal, depth, lit_edge, split)
    # grow canvas for the sword tip below
    need = sy + len(rows) + pad + 1
    if need > img.shape[0]:
        img = np.concatenate([img, np.zeros((need - img.shape[0],) + img.shape[1:], np.uint8)])
    stamp_rows(img, rows, SWORD_KEY, sx + pad, sy + pad)
    return img, dict(rows=rows, x=sx + pad, y=sy + pad, cap=cap + pad, base=cap + pad + H - 1)


def build_small(pal, depth=2, lit_edge=True):
    H = 13
    L = letters_small()
    rows, bx, cap = sword_small(H)
    face, (sx, sy), top = compose(L, "GLINTBLADE", H, 2, rows, bx, cap, 3, kern=KERN_SMALL)
    img, pad = shade(face, top, H, pal, depth, lit_edge)
    need = sy + len(rows) + pad + 1
    if need > img.shape[0]:
        img = np.concatenate([img, np.zeros((need - img.shape[0],) + img.shape[1:], np.uint8)])
    stamp_rows(img, rows, SWORD_KEY, sx + pad, sy + pad)
    return img, dict(rows=rows, x=sx + pad, y=sy + pad, cap=cap + pad, base=cap + pad + H - 1)


