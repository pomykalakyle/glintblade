"""Raised rock in 3/4 view, 16x16 tiles.

Face: a 48x32 panel (3 horizontal variants by tx % 3; face_top = upper tile, face_bot = lower tile) plus a
repeatable 48x16 mid strip for walls taller than 2 tiles. Left/right end tiles per variant.
Top surface: flat dark stone with rims where it borders floor or a face; inner (concave) corner notches.

Autotile rule (see autotile()): in every vertical run of rock tiles, the bottom FH tiles are face
(face_top, face_mid x (FH-2), face_bot), everything above is top surface.
"""
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I  # noqa: E402
from cliff_face import face_panel as _face_panel, mid_strip as _mid_strip  # noqa: E402

T = 16
PERIOD = 3
PW, PH = T * PERIOD, 32
TOP = 4
BOT = 29


def path(spec, last=BOT):
    out = {}
    spec = sorted(spec)
    for i, (r, x) in enumerate(spec):
        r1 = spec[i + 1][0] if i + 1 < len(spec) else last + 1
        for y in range(r, r1):
            out[y] = x
    return out


CREV = [
    path([(TOP, 47)]),
    path([(TOP, 12), (11, 13), (20, 12), (25, 11)]),
    path([(TOP, 20), (15, 21)]),
    path([(TOP, 33), (13, 32), (22, 33)]),
    path([(TOP, 40), (16, 41)]),
]
WIDE = {1: 24, 2: 18, 4: 26}
# (lit rows at mass's left edge, at its right edge, bottom row of the mass)
MASS = [(10, 4, 28), (9, 3, 29), (10, 5, 28), (9, 3, 29), (8, 4, 28)]
LEDGES = [(19, 2, 8), (14, 26, 30), (22, 35, 38)]
RUBBLE = [
    # (x, y, rows) small boulders at the foot; '.' = keep
    (10, 27, [".lm.", "lmmd", "#dd#"]),
    (20, 28, [".lm", "ldd", "##."]),
    (39, 27, [".lmm.", "lmmmd", "#ddd#"]),
    (29, 28, ["lm.", "md#"]),
]
RKEY = {"l": "w3", "m": "w2", "d": "w1", "#": "w0"}


def face_panel(base=True, rows=(0, PH)):
    c = np.full((PH, PW), I["w1"], np.int32)
    n = len(CREV)
    for y in range(TOP, BOT + 1):
        for m in range(n):
            a = CREV[m][y] + 1 if m > 0 else 0
            b = CREV[m + 1][y] - 1 if m + 1 < n else CREV[0][y] - 1
            ll, lr, mb = MASS[m]
            if not base:
                mb = 99
            width = b - a
            for x in range(a, b + 1):
                t = (x - a) / max(width, 1)
                lit_end = TOP + 1 + round(ll + (lr - ll) * t)
                if y > mb:
                    c[y, x] = I["w0"]
                elif y < lit_end:
                    c[y, x] = I["w2"]
            if y == TOP + 1:
                for x in range(a + 1, a + 4):
                    c[y, x] = I["w3"]
            if TOP + 2 <= y < TOP + 2 + ll // 2:
                c[y, a] = I["w3"]
            if y == mb:
                c[y, a] = I["w0"]
                c[y, b] = I["w0"]
    for i, cv in enumerate(CREV):
        for y, x in cv.items():
            c[y, x % PW] = I["w0"]
            if i in WIDE and y >= WIDE[i]:
                c[y, (x + 1) % PW] = I["w0"]
    for (row, x0, x1) in LEDGES:
        for x in range(x0, x1 + 1):
            if c[row, x] != I["w0"] or True:
                c[row, x] = I["w0"]
                c[row + 1, x] = I["w2"]
        c[row + 1, x0] = I["w3"]
    # lip
    c[0:2, :] = I["w1"]
    c[2, :] = I["w3"]
    c[3, :] = I["w2"]
    c[4, :] = I["w0"]
    for x in range(PW):
        if x in (5, 6, 7, 8, 26, 27, 28, 36, 37):
            c[1, x] = I["w3"]
            c[2, x] = I["w4"] if x in (6, 7, 27) else I["w3"]
        if x in (17, 18, 44, 45):
            c[3, x] = I["w0"]
    for x in (2, 3, 24, 25, 35):
        c[4, x] = I["w2"]
    if base:
        c[30, :] = I["w0"]
        c[31, :] = I["r1"]
        for (x, y, rows_) in RUBBLE:
            for j, r in enumerate(rows_):
                for i, ch in enumerate(r):
                    if ch != ".":
                        c[y + j, x + i] = I[RKEY[ch]]
    return c


def mid_strip():
    """48x16 repeatable: crevices at their row-16 positions, dark rock, a couple of ledges."""
    c = np.full((T, PW), I["w1"], np.int32)
    for i, cv in enumerate(CREV):
        x = cv[16]
        c[:, x % PW] = I["w0"]
        if i in WIDE and WIDE[i] <= 16:
            c[:, (x + 1) % PW] = I["w0"]
    for (row, x0, x1) in [(6, 3, 8), (11, 24, 29), (3, 42, 45)]:
        for x in range(x0, x1 + 1):
            c[row, x] = I["w0"]
            c[row + 1, x] = I["w2"]
        c[row + 1, x0] = I["w3"]
    return c


# ---------------------------------------------------------------- end tiles

L_PROF = {"top": [1, 1, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
          "mid": [2] * 16,
          "bot": [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 4, 5]}
R_PROF = {"top": [2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
          "mid": [2] * 16,
          "bot": [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 4, 5]}


def make_end(c, kind, side):
    c = c.copy()
    prof = L_PROF[kind] if side == "l" else R_PROF[kind]
    for y in range(T):
        k = prof[y]
        if side == "l":
            c[y, :k] = I["r3"]
            if kind == "top" and y <= 3:
                c[y, k] = I["w3"]                       # west rim of the top surface rounds into the lip
            else:
                lit = c[y, k + 1] in (I["w2"], I["w3"]) or (kind == "top" and y < 10)
                c[y, k] = I["w3"] if lit else I["w2"]
                if c[y, k + 1] == I["w0"]:
                    c[y, k + 1] = I["w1"]
        else:
            c[y, T - k:] = I["r2"] if not (kind == "top" and y < 2) else I["r3"]
            c[y, T - k - 1] = I["w0"]
    if kind == "top" and side == "l":
        c[2, 2] = I["w4"]
        c[4, 3] = I["w3"]
    if kind == "bot":
        if side == "l":
            c[15, :] = I["r3"]
            c[14, 4] = I["w1"]
        else:
            c[15, :] = np.where(np.arange(T) >= T - 6, I["r2"], c[15, :])
    return c


# ---------------------------------------------------------------- top surface


TOP_FILL = "w1"
TOP_FEATURES = {
    1: [(4, 6, "w2"), (5, 6, "w2"), (6, 6, "w2"), (5, 7, "w0"), (6, 7, "w0")],
    2: [(9, 9, "w0"), (10, 9, "w0"), (11, 10, "w0"), (12, 10, "w0"), (10, 10, "w2")],
}


def top_tile(n, w, e, inner=(False, False, False, False), var=0):
    c = np.full((T, T), I[TOP_FILL], np.int32)
    for (x, y, col) in TOP_FEATURES.get(var, []):
        c[y, x] = I[col]
    if n:
        c[0:2, :] = I["r3"]
        c[2, :] = I["w3"]
        c[3, :] = np.where(c[3, :] == I[TOP_FILL], I["w2"], c[3, :])
        for x in (5, 6, 12):
            c[2, x] = I["w4"]
    if w:
        c[:, 0] = I["w3"]
        c[:, 1] = np.where(c[:, 1] == I[TOP_FILL], I["w2"], c[:, 1])
    if e:
        c[:, T - 1] = I["w0"]
    if n and w:
        c[0:3, 0:2] = I["r3"]
        c[2, 2] = I["r3"]
        c[3, 0] = I["r3"]
        c[3, 1] = I["w3"]
        c[2, 3] = I["w3"]
        c[4, 0] = I["w3"]
        c[3, 2] = I["w2"]
    if n and e:
        c[0:3, T - 2:] = I["r3"]
        c[3, T - 1] = I["r3"]
        c[2, T - 3] = I["w3"]
        c[3, T - 2] = I["w0"]
    nw, ne, sw, se = inner
    if nw:
        c[0, 0] = I["r3"]; c[0, 1] = I["w3"]; c[1, 0] = I["w3"]
    if ne:
        c[0, T - 1] = I["r3"]; c[0, T - 2] = I["r3"]; c[1, T - 1] = I["r3"]
        c[0, T - 3] = I["w0"]; c[1, T - 2] = I["w0"]; c[2, T - 1] = I["w0"]
    if sw:
        c[T - 1, 0] = I["r3"]; c[T - 1, 1] = I["w3"]; c[T - 2, 0] = I["w3"]
    if se:
        c[T - 1, T - 1] = I["r3"]; c[T - 1, T - 2] = I["r3"]; c[T - 2, T - 1] = I["r3"]
        c[T - 1, T - 3] = I["w0"]; c[T - 2, T - 2] = I["w0"]
    return c


def top_name(n, w, e, inner):
    s = "cliff_top"
    rims = ("n" if n else "") + ("w" if w else "") + ("e" if e else "")
    if rims:
        s += "_" + rims
    ins = "".join(k for k, f in zip(("nw", "ne", "sw", "se"), inner) if f)
    if ins:
        s += "_in" + ins
    return s


def all_top_variants():
    out = {}
    for n in (False, True):
        for w in (False, True):
            for e in (False, True):
                for bits in range(16):
                    nw, ne, sw, se = (bool(bits & 8), bool(bits & 4), bool(bits & 2), bool(bits & 1))
                    if nw and (n or w):
                        continue
                    if ne and (n or e):
                        continue
                    if sw and w:
                        continue
                    if se and e:
                        continue
                    out[top_name(n, w, e, (nw, ne, sw, se))] = top_tile(n, w, e, (nw, ne, sw, se))
    out["cliff_top_1"] = top_tile(False, False, False, var=1)
    out["cliff_top_2"] = top_tile(False, False, False, var=2)
    return out


def all_face_tiles():
    p = _face_panel()
    ms = _mid_strip()
    out = {}
    for v in range(PERIOD):
        xs = slice(v * T, (v + 1) * T)
        parts = {"top": p[0:T, xs], "mid": ms[:, xs], "bot": p[T:2 * T, xs]}
        for kind, c in parts.items():
            out[f"cliff_face_{kind}_{v}"] = c.copy()
            out[f"cliff_face_{kind}_l_{v}"] = make_end(c, kind, "l")
            out[f"cliff_face_{kind}_r_{v}"] = make_end(c, kind, "r")
    return out


# ---------------------------------------------------------------- floor shading next to cliffs


def floor_shade(n=False, w=False):
    c = np.full((T, T), I["r3"], np.int32)
    if n:
        prof = [3, 3, 2, 2, 3, 3, 3, 2, 2, 2, 3, 3, 2, 2, 3, 3]
        for x in range(T):
            c[0:prof[x], x] = I["r2"]
    if w:
        prof = [3, 3, 3, 3, 2, 2, 2, 3, 3, 3, 3, 2, 2, 2, 3, 3]
        for y in range(T):
            c[y, 0:prof[y]] = I["r2"]
    return c


# ---------------------------------------------------------------- autotiler (reference for the region builder)


def classify(rock, fh=2):
    th, tw = rock.shape

    def R(x, y):
        if x < 0 or x >= tw or y < 0:
            return True
        if y >= th:
            return True
        return bool(rock[y, x])

    kind = {}
    for x in range(tw):
        for y in range(th):
            if not R(x, y):
                continue
            d = 0
            while y + d + 1 < th and R(x, y + d + 1):
                d += 1
            if y + d + 1 >= th:
                kind[(x, y)] = "top"
            elif d == 0:
                kind[(x, y)] = "bot"
            elif d < fh - 1:
                kind[(x, y)] = "mid"
            elif d == fh - 1:
                kind[(x, y)] = "facetop"
            else:
                kind[(x, y)] = "top"
    return kind, R


def autotile(rock, fh=2):
    """rock: 2D bool array [ty][tx]. Returns {(tx, ty): tile name} for rock tiles and shaded floor."""
    th, tw = rock.shape
    kind, R = classify(rock, fh)

    def K(x, y):
        if x < 0 or y < 0 or x >= tw or y >= th:
            return "top"
        return kind.get((x, y))

    names = {}
    for (x, y), k in kind.items():
        v = x % PERIOD
        if k in ("bot", "mid", "facetop"):
            part = {"bot": "bot", "mid": "mid", "facetop": "top"}[k]
            side = ""
            if not R(x - 1, y):
                side = "_l"
            elif not R(x + 1, y):
                side = "_r"
            names[(x, y)] = f"cliff_face_{part}{side}_{v}"
        else:
            n = K(x, y - 1) != "top"
            w = K(x - 1, y) != "top"
            e = K(x + 1, y) != "top"
            nw = not n and not w and K(x - 1, y - 1) != "top"
            ne = not n and not e and K(x + 1, y - 1) != "top"
            sw = not w and K(x - 1, y + 1) not in ("top", "facetop")
            se = not e and K(x + 1, y + 1) not in ("top", "facetop")
            names[(x, y)] = top_name(n, w, e, (nw, ne, sw, se))
    for x in range(tw):
        for y in range(th):
            if R(x, y):
                continue
            sn = R(x, y - 1)
            sw_ = R(x - 1, y)
            if sn or sw_:
                names[(x, y)] = "floor_shade_" + ("n" if sn else "") + ("w" if sw_ else "")
    return names
