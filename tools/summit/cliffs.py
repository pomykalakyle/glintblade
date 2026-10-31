"""Cliff faces and edge rims for the Summit plateau (imported by tiles.py).

Everything here goes in VOID tiles (where the player can't walk). Transparent = sky.

South faces hang below the plateau's south edge, in 3/4 view:
  cliff_top_<v>[_l|_r]   first void row under the surface (row 0 = the moonlit lip)
  cliff_mid_<v>[_l|_r]   repeat 0..n times for a taller face
  cliff_bot_<v>[_l|_r]   2 tiles tall: the face darkens and breaks into hanging points
  wall_top_<v>[_l|_r]    the arena's masonry retaining wall; use instead of cliff_top
                         under paving (mid / bot stay the same)
Pick ONE variant letter per tile column and use it for the whole stack (top, mids, bot):
the rock columns line up vertically only within a variant. Every tile has a crevice at
x=0, so any variant sits next to any other. _l / _r = the face's left / right end.

Rims (overlays for void tiles next to the surface; several can stack in one tile):
  rim_n_<mat>  surface is SOUTH of this tile: the plateau's far lip
  rim_w_<mat>  surface is EAST of this tile: the plateau's west edge (moonlit)
  rim_e_<mat>  surface is WEST of this tile: the plateau's east edge (shadowed)
  rim_nw_<mat> / rim_ne_<mat>  outer corners (surface only diagonally SE / SW)
<mat> = rock (natural ground) or pave (flagstones).
"""

import math
import random

from common import blank, get, put

T = 16

# Face ramp (dark mauve rock sliding into blue shadow toward the void).
LIP = "4"
UNDERLIP = "1"
HI = "2"
BODY = "9"
SHADE = "1"
DEEP = "b"
DEEPEST = "a"
BOUNCE = "t"  # cool light bouncing up from the cloud sea far below

RAMP = [HI, BODY, SHADE, DEEP, DEEPEST]
VARIANTS = "abcdefgh"


def walk(rng, x, y0, y1, lo, hi):
    """A wavy vertical line: x per row from y0 to y1, shifting by 1 every 4-8 rows."""
    xs = {}
    nxt = y0 + rng.randint(3, 8)
    for y in range(y0, y1):
        if y == nxt:
            x = min(hi, max(lo, x + rng.choice([-1, 1])))
            nxt = y + rng.randint(4, 8)
        xs[y] = x
    return xs


def variant_spec(v):
    """Fold lines and points for one 16 px wide variant (seeded, so stable)."""
    rng = random.Random("summit-cliff-" + v)
    for _ in range(100):
        n = rng.choice([1, 1, 2])
        xs = sorted(rng.sample(range(4, 12), n))
        if all(b - a >= 4 for a, b in zip(xs, xs[1:])):
            break
    folds = []
    for x in xs:
        y0 = rng.randint(2, 6) if rng.random() < 0.7 else rng.randint(10, 26)
        y1 = 64 if rng.random() < 0.75 else rng.randint(36, 50)
        line = walk(rng, x, y0, y1, 3, 12)
        deep = [(y0 + rng.randint(2, 8), y0 + rng.randint(10, 22))]
        folds.append({"line": line, "deep": deep})
    # points: the border fold's point sits on the tile seam; each internal fold gets one
    seam_tip = 52
    tips = [(-0.5, seam_tip, 1.5), (15.5, seam_tip, 1.5)]
    bounds = [0] + xs + [16]
    for i in range(1, len(bounds) - 1):
        c = (bounds[i] + bounds[i + 1]) / 2 - 0.5
        if i == len(bounds) - 2 and 16 - bounds[i] < 6:
            continue
        tips.append((c + rng.choice([-1, 0, 1]), rng.choice([49, 53, 57, 61, 64]), rng.choice([1.2, 1.6, 2.0, 2.4])))
    ledges = []
    for _ in range(0):
        ledges.append((rng.randint(2, 9), rng.randint(5, 30), rng.randint(3, 6)))
    drips = [rng.randint(-3, 3) for _ in range(len(bounds))]
    return {"folds": folds, "tips": tips, "ledges": ledges, "drips": drips, "bounds": bounds}


def underside(spec):
    """Lowest solid row + 1 for each x (rock above it)."""
    prof = []
    for x in range(T):
        best = 45
        for (c, tip, slope) in spec["tips"]:
            best = max(best, int(tip - slope * abs(x - c)))
        prof.append(min(best, 64))
    return prof


def stack(v):
    """The full 16 x 64 stack for variant v: top (0-15), mid (16-31), bot (32-63)."""
    spec = variant_spec(v)
    img = blank(T, 64)
    prof = underside(spec)

    def fold_at(y):
        return sorted(((f["line"][y], f) for f in spec["folds"] if y in f["line"]), key=lambda p: p[0])

    for y in range(64):
        active = fold_at(y)
        bxs = [bx for bx, _ in active]
        for x in range(T):
            if y >= prof[x]:
                continue
            # which fold region am I in (index into bounds-ish ordering)
            k = sum(1 for bx in bxs if bx < x)
            drip = spec["drips"][min(k, len(spec["drips"]) - 1)]
            lb = max([bx for bx in bxs if bx < x], default=-8)
            rb = min([bx for bx in bxs if bx > x], default=24)
            drip += min(x - lb, rb - x) // 2
            # base level by depth (mid rows stay BODY so mid tiles repeat)
            if y < 32:
                lvl = 1
            else:
                by = y - 32
                lvl = 1 + (by >= 6 + drip) + (by >= 17 + drip)
            if x in bxs:
                lvl += 2
            elif x - 1 in bxs and y >= 2:
                lvl -= 1          # lit left edge of the fold to the right
            lvl = max(0, min(4, lvl))
            put(img, x, y, RAMP[lvl])
        # fold lines that are only soft (outside their "deep" span) use one step darker only
        for bx, f in active:
            deep = any(a <= y <= b for a, b in f["deep"])
            if not deep and get(img, bx, y) != "." and y < 32:
                put(img, bx, y, SHADE)
    # ledges: a short shelf of rock: lit top, shadow under
    for (x0, y, L) in spec["ledges"]:
        for x in range(x0, min(T - 2, x0 + L)):
            if get(img, x, y) != "." and get(img, x, y + 1) != ".":
                put(img, x, y, HI)
                put(img, x, y + 1, "0")
    # underside: deepest tones, then the cloud-sea bounce on the very bottom pixel
    for x in range(T):
        bottom = prof[x] - 1
        for y in range(max(46, bottom - 3), bottom + 1):
            put(img, x, y, DEEPEST)
    for x in range(T):
        bottom = prof[x] - 1
        # the bounce catches only flat-ish undersides (not the steep flanks of points)
        l = prof[x - 1] if x > 0 else prof[x]
        r = prof[x + 1] if x < T - 1 else prof[x]
        if abs(l - prof[x]) <= 1 or abs(r - prof[x]) <= 1:
            put(img, x, bottom, BOUNCE)
    # the lip
    for x in range(T):
        put(img, x, 0, LIP)
        put(img, x, 1, UNDERLIP)
    return img


def cut_end(img, end, masonry=False):
    """Make the face's left or right end: the face rounds off and tapers as it falls."""
    def inset(y):
        if y < 16:
            return [0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3][y]
        if y < 32:
            return 3 + (1 if 21 <= y <= 26 else 0)
        return 3 + int((y - 32) * 0.5 + 0.25 * ((y // 3) % 2))

    for y in range(64):
        k = min(inset(y), T)
        for i in range(k):
            x = i if end == "l" else T - 1 - i
            put(img, x, y, ".")
        x = k if end == "l" else T - 1 - k
        if 0 <= x < T and get(img, x, y) != ".":
            if end == "l":
                put(img, x, y, LIP if y < 2 else (HI if y < 40 else get(img, x, y)))
            else:
                put(img, x, y, LIP if y == 0 else ("0" if y < 48 else get(img, x, y)))
    return img


def masonry(img, v, end=None):
    """Top tile as the arena's retaining wall: a coping lip over one course of big blocks,
    standing on the natural rock (whose crevices continue below)."""
    joints = {"a": [9], "b": [5], "c": [3, 12], "d": [7], "e": [10], "f": [4, 11], "g": [8], "h": [6]}[v]
    for y in range(16):
        for x in range(T):
            if get(img, x, y) == ".":
                continue
            if y == 0:
                t = "f" if x < 12 else "e"
            elif y == 1:
                t = "d"
            elif y == 2:
                t = "a"
            elif y < 13:
                t = "c" if y > 3 else "d"
            elif y == 13:
                t = "a"
            else:
                t = get(img, x, y)
            put(img, x, y, t)
    for j in joints:
        for y in range(3, 13):
            if get(img, j, y) != ".":
                put(img, j, y, "a")
        if get(img, j + 1, 3) != ".":
            for y in range(3, 7):
                put(img, j + 1, y, "d")
    if end:
        x = 0
        if end == "l":
            while get(img, x, 5) == "." and x < T - 1:
                x += 1
            for y in range(0, 13):
                if get(img, x, y) != ".":
                    put(img, x, y, "e" if y < 2 else "d")
    return img


def face_tiles():
    """{name: 16x16 image} for every face piece."""
    out = {}
    for v in VARIANTS:
        for end in (None, "l", "r"):
            base = stack(v)
            if end:
                base = cut_end(base, end)
            suf = f"_{end}" if end else ""
            out[f"cliff_top_{v}{suf}"] = base[0:16].copy()
            out[f"cliff_mid_{v}{suf}"] = base[16:32].copy()
            out[f"cliff_bot_{v}{suf}"] = base[32:64].copy()
            wall = masonry(base.copy(), v, end)
            out[f"wall_top_{v}{suf}"] = wall[0:16].copy()
    return out


def wobble(i, n, amp, phase):
    """Smooth offset 0..amp that returns to its start at both tile borders (chains)."""
    return round(amp * (0.5 - 0.5 * math.cos(2 * math.pi * (i / n) * 1 + phase) * math.cos(phase)))


def rim_tiles():
    out = {}
    for mat in ("rock", "pave"):
        lit, body, shade = ("4", "3", "1") if mat == "rock" else ("f", "e", "c")
        # north lip: surface south of this tile -> the lip sits on the bottom rows
        img = blank(T, T)
        prof = [1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1]
        for x in range(T):
            h = prof[x]
            for y in range(T - 1 - h, T):
                put(img, x, y, body if mat == "rock" else ("d" if mat == "pave" else body))
            put(img, x, T - 1 - h, lit)
        out[f"rim_n_{mat}"] = img
        # west edge: surface east of this tile -> moonlit edge on the right columns
        img = blank(T, T)
        prof = [1, 1, 1, 1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1]
        for y in range(T):
            w = prof[y]
            for x in range(T - 1 - w, T):
                put(img, x, y, body)
            put(img, T - 1 - w, y, lit)
        out[f"rim_w_{mat}"] = img
        # east edge: surface west of this tile -> a dark edge
        img = blank(T, T)
        prof = [1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1]
        for y in range(T):
            w = prof[y]
            for x in range(0, w + 1):
                put(img, x, y, "0" if x == w else shade)
        out[f"rim_e_{mat}"] = img
        # outer corners
        img = blank(T, T)
        for (x, y) in [(15, 14), (14, 15), (15, 15)]:
            put(img, x, y, lit)
        put(img, 13, 15, lit)
        out[f"rim_nw_{mat}"] = img
        img = blank(T, T)
        for (x, y) in [(0, 14), (0, 15), (1, 15)]:
            put(img, x, y, lit)
        put(img, 2, 15, "0")
        out[f"rim_ne_{mat}"] = img
    return out
