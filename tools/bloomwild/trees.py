"""bloomwild_trees: blossom trees. Each tree = a trunk anim (y-sorted, footprint) and a canopy anim
("layer": "canopy", drawn over characters) sharing the same cell and anchor, so the builder places
two props at the same point.

Canopy = hand-placed clusters of blossom, drawn back to front (lower = nearer). Each cluster is
shaded by its own normal against light from the upper left, plus a small whole-canopy term (the
underside is in shadow), quantized to the ramp. A one-level crease separates overlapping clumps.
The shadow-side silhouette gets the darkest ramp color; the lit side has no outline.
"""
import math

from pix import *

CW, CH = 80, 96
AX, AY = 40, 90

BLOSSOM = ["b0", "b1", "b2", "b3", "b4", "b5"]
CORAL = ["b1", "b2", "b3", "b4", "b5", "s5"]


def crown(masses, ramp, sway=0, sway_above=0, blossoms=(), notches=(), seed=0):
    """masses: (cx, cy, rx, ry) broad crown masses, drawn back (top) to front (bottom).
    Each mass is shaded as ONE form (ellipsoid lit from the upper left), so the interior stays
    calm: a lit top-left, a mid body, and a distinct dark underside. The silhouette is scalloped
    by small leaf puffs along the upper/side rim (they take the mass's shading). notches:
    (x, y, rx, ry) gaps cut through the crown so the supporting limb shows."""
    L = (-0.5, -0.66, 0.56)
    lvl = [[-1] * CW for _ in range(CH)]
    for mi, (mx, my, rx, ry) in enumerate(sorted(masses, key=lambda m: m[1])):
        dx = sway if (sway and my < sway_above) else 0
        mx = mx + dx
        # puffs: varied spacing along the rim from lower-left, over the top, to lower-right
        pf = []
        a = 200.0
        k = 0
        while a > -160.0:
            rad = math.radians(a)
            lower = a < -20.0
            pr = (1.8 + 1.4 * hash01(seed, mi, k, 1)) if lower else (2.6 + 2.2 * hash01(seed, mi, k, 1))
            ex, ey = mx + math.cos(rad) * rx * 0.92, my - math.sin(rad) * ry * 0.92
            pf.append((ex, ey, pr))
            a -= 16 + 18 * hash01(seed, mi, k, 2)
            k += 1
        for y in range(CH):
            for x in range(CW):
                u, v = (x + 0.5 - mx) / rx, (y + 0.5 - my) / ry
                inside = u * u + v * v <= 1.0
                if not inside:
                    for (px, py, pr) in pf:
                        if (x + 0.5 - px) ** 2 + (y + 0.5 - py) ** 2 <= pr * pr:
                            inside = True
                            break
                if not inside:
                    continue
                d = math.sqrt(u * u + v * v)
                if d > 1.0:
                    u, v = u / d, v / d
                w = math.sqrt(max(0.0, 1 - u * u - v * v))
                lum = u * L[0] + v * L[1] + w * L[2]
                lvl[y][x] = 4 if lum > 0.8 else 3 if lum > 0.42 else 2 if lum > -0.12 else 1
    for (nx, ny, nrx, nry) in notches:
        for y in range(CH):
            for x in range(CW):
                if ((x + 0.5 - nx) / nrx) ** 2 + ((y + 0.5 - ny) / nry) ** 2 <= 1.0:
                    lvl[y][x] = -1
    # the lightest level only in solid clumps (no lone highlight pixels)
    src = [r[:] for r in lvl]
    for y in range(1, CH - 1):
        for x in range(1, CW - 1):
            v = src[y][x]
            if v < 0:
                continue
            nb = [src[y - 1][x], src[y + 1][x], src[y][x - 1], src[y][x + 1]]
            if sum(1 for n in nb if n == v) <= 1:
                nn = [n for n in nb if n >= 0]
                if len(nn) >= 3:
                    lvl[y][x] = max(set(nn), key=nn.count)
    img = new(CW, CH)
    for y in range(CH):
        for x in range(CW):
            if lvl[y][x] >= 0:
                put(img, x, y, ramp[lvl[y][x]])
    # blossom clusters on lit edges only
    for (bx, by) in blossoms:
        bx += sway if by < sway_above else 0
        for (ddx, ddy, c) in ((0, 0, 5), (1, 0, 4), (0, 1, 4), (1, 1, 3)):
            if opaque(img, bx + ddx, by + ddy):
                put(img, bx + ddx, by + ddy, ramp[c])
    # selective outline: darkest color on the shadow-side silhouette (right and bottom)
    m = mask_of(img)
    out = img.copy()
    xs = [x for x in range(CW) if m[:, x].any()]
    cx = (xs[0] + xs[-1]) / 2
    for y in range(CH):
        for x in range(CW):
            if not m[y, x]:
                continue
            right_open = x + 1 >= CW or not m[y, x + 1]
            down_open = y + 1 >= CH or not m[y + 1, x]
            if (right_open and x > cx - 4) or down_open:
                out[y, x] = rgba(ramp[0])
    return fix_staircases(out)


def trunk(base_x, base_y, fork_y, w, limbs, roots, lean=0.0):
    """A trunk that flares into roots at the base and forks into limbs at fork_y.
    Bark lit on the left (e4 edge, e3), mid e2, shaded right e1. limbs: (dx_top, top_y, width)."""
    img = new(CW, CH)

    def edges(y):
        t = (base_y - y) / max(1, base_y - fork_y)  # 0 at base, 1 at fork
        c = base_x + lean * t * t * 3
        hw = w / 2 * (1.0 - 0.18 * t)
        flare = max(0.0, 3.5 - (base_y - y)) * 1.1
        return c - hw - flare, c + hw + flare

    def bark(x, x0, x1):
        rel = (x + 0.5 - x0) / max(1.0, x1 - x0)
        if x1 - x0 < 5:  # limbs: no top highlight
            return "e3" if rel < 0.4 else "e2" if rel < 0.8 else "e1"
        return "e4" if rel < 0.14 else "e3" if rel < 0.38 else "e2" if rel < 0.76 else "e1"

    for y in range(fork_y, base_y + 1):
        x0, x1 = edges(y)
        for x in range(int(math.floor(x0 + 0.5)), int(math.floor(x1 + 0.5)) + 1):
            put(img, x, y, bark(x, x0, x1))
    fx0, fx1 = edges(fork_y)
    fc = (fx0 + fx1) / 2
    for (dxt, ty, lw) in limbs:
        steps = fork_y - ty
        for k in range(steps + 1):
            t = k / max(1, steps)
            cxk = fc + dxt * (t ** 0.8)
            y = fork_y - k
            for x in range(int(round(cxk - lw / 2)), int(round(cxk + lw / 2)) + 1):
                put(img, x, y, bark(x, cxk - lw / 2, cxk + lw / 2))
    # bark detail: broken grooves and one knot
    mid = base_x + 0.5
    for y in range(fork_y + 2, base_y - 4):
        if (y // 4) % 3 == 0:
            put(img, int(mid) + 1, y, "e1")
    ky = (fork_y + base_y) // 2
    put(img, int(mid) - 1, ky, "e1"); put(img, int(mid), ky, "e0"); put(img, int(mid) - 1, ky + 1, "e0")
    for (d, ln) in roots:
        x0, x1 = edges(base_y)
        sx = x0 if d < 0 else x1
        for k in range(ln):
            x = int(round(sx + d * (k + 1)))
            if k < ln // 2:
                put(img, x, base_y - 1, "e3" if d < 0 else "e2")
            put(img, x, base_y, "e2" if d < 0 else "e1")
    xs = [x for x in range(CW) if opaque(img, x, base_y)]
    for x in xs[2:-2]:
        put(img, x, base_y + 1, "e0")
    return img


MAG = ["b0", "b1", "b2", "b3", "b4", "b5"]
COR = ["b1", "b2", "b3", "b4", "b5", "s5"]

TREES = {
    "a": dict(  # big magenta: a high back mass and two front masses split by a notch over the fork
        masses=[(38, 24, 24, 15), (22, 42, 17, 12), (56, 44, 19, 11)],
        notches=[(39, 55, 4, 5)], ramp=MAG, sway_above=30,
        blossoms=[(20, 12), (30, 8), (9, 33), (44, 29)],
        trunk=dict(base_x=38, base_y=89, fork_y=64, w=9, lean=0.3,
                   limbs=[(-9, 46, 4), (14, 48, 3), (1, 40, 3)], roots=[(-1, 5), (1, 4)]),
        fp=11),
    "b": dict(  # tall magenta: a narrow upper mass leaning left over a broad lower one
        masses=[(33, 25, 15, 13), (56, 36, 8, 7), (42, 44, 20, 13)],
        notches=[(33, 56, 3, 4)], ramp=MAG, sway_above=30,
        blossoms=[(27, 13), (24, 36), (36, 32)],
        trunk=dict(base_x=39, base_y=89, fork_y=66, w=7, lean=-0.4,
                   limbs=[(-8, 52, 3), (6, 50, 3)], roots=[(-1, 4), (1, 3)]),
        fp=9),
    "c": dict(  # broad coral: two side-by-side masses with a deep notch, a low front mass
        masses=[(28, 33, 17, 13), (54, 31, 16, 13), (42, 50, 21, 9)],
        notches=[(43, 58, 3, 4)], ramp=COR, sway_above=36,
        blossoms=[(18, 24), (46, 21), (30, 42)],
        trunk=dict(base_x=41, base_y=89, fork_y=66, w=8, lean=-0.3,
                   limbs=[(-8, 50, 3), (8, 48, 3), (0, 40, 2)], roots=[(-1, 4), (1, 4)]),
        fp=10),
    "d": dict(  # leaning coral: crown pushed left of the trunk, small trailing mass on the right
        masses=[(31, 34, 17, 13), (36, 51, 16, 9), (57, 40, 8, 7)],
        notches=[(49, 58, 3, 4)], ramp=COR, sway_above=38,
        blossoms=[(22, 25), (34, 23), (54, 35)],
        trunk=dict(base_x=43, base_y=89, fork_y=68, w=6, lean=-1.2,
                   limbs=[(-6, 54, 3), (12, 44, 2)], roots=[(-1, 3), (1, 4)]),
        fp=8),
    "e": dict(  # young magenta tree, ~40 px
        masses=[(40, 60, 11, 9), (44, 68, 10, 6)],
        notches=[], ramp=MAG, sway_above=60, blossoms=[(34, 55)],
        trunk=dict(base_x=41, base_y=89, fork_y=76, w=4, lean=0.2,
                   limbs=[(-3, 68, 2), (4, 66, 2)], roots=[(-1, 2), (1, 2)]),
        fp=6),
}


SHIFT = {"a": 16, "b": 14, "c": 6, "d": 4, "e": 0}   # lower the crowns to keep trees 48-70 px tall


def shifted_tree(name, t):
    s = SHIFT[name]
    t = dict(t)
    t["masses"] = [(x, y + s, rx, ry) for x, y, rx, ry in t["masses"]]
    t["notches"] = [(x, y + s, rx, ry) for x, y, rx, ry in t["notches"]]
    t["blossoms"] = [(x, y + s) for x, y in t["blossoms"]]
    t["sway_above"] = t["sway_above"] + s
    tr = dict(t["trunk"])
    tr["fork_y"] = tr["fork_y"] + s // 2
    tr["limbs"] = [(dx, ty + s, w) for dx, ty, w in tr["limbs"]]
    t["trunk"] = tr
    return t


def build():
    rows = []
    anims = {}
    for i, (name, t) in enumerate(TREES.items()):
        t = shifted_tree(name, t)
        tr = trunk(**t["trunk"])
        cans = [crown(t["masses"], t["ramp"], sway=sw, sway_above=t["sway_above"],
                      blossoms=t["blossoms"], notches=t["notches"], seed=i + 1)
                for sw in (0, 1)]
        rows.append([tr, new(CW, CH)])
        rows.append(cans)
        anims[f"{name}_trunk"] = {"row": 2 * i, "frames": 1, "fps": 1, "footprint": [t["fp"], 5]}
        anims[f"{name}_canopy"] = {"row": 2 * i + 1, "frames": 2, "fps": 2,
                                    "durations": [56, 40], "layer": "canopy"}
    return rows, anims


def write():
    rows, anims = build()
    man = {
        "cell": [CW, CH],
        "anchor": [AX, AY],
        "anims": anims,
        "notes": "Blossom trees. Place BOTH '<x>_trunk' and '<x>_canopy' props at the same point (the trunk's base). "
                 "a = big magenta (~69 px tall, 76 px wide crown), b = tall magenta (~68), c = broad coral (~70), d = leaning coral (~66), e = young magenta (~40). Draw a soft ground shadow "
                 "(f.shadow) under each: a ~ rx 30 ry 9 at the anchor; b rx 20 ry 7; c rx 24 ry 8; d rx 20 ry 7; e rx 12 ry 4. "
                 "Canopy sway: upper clusters shift 1 px.",
    }
    sheet = save_sheet("bloomwild_trees", rows, CW, CH, man)
    return sheet, rows


def composite(rows, k_trunk_row, k_canopy_row, frame=0):
    img = rows[k_trunk_row][0].copy()
    paste(img, rows[k_canopy_row][frame], 0, 0)
    return img


if __name__ == "__main__":
    sheet, rows = write()
    preview(sheet, PREVIEWS / "bloomwild_trees_6x.png", k=6, bg="g2", gridcell=(CW, CH))
    n = len(TREES)
    scene = new(CW * n + 20, CH, "g2")
    for i in range(n):
        paste(scene, composite(rows, 2 * i, 2 * i + 1), i * CW, 0)
    paste(scene, stand_in_figure(), CW * n + 4, AY - 17)
    save(scale(scene, 4), PREVIEWS / "_wip_bw_trees.png")
    save(scale(scene, 6), PREVIEWS / "bloomwild_trees_scale_6x.png")
    gif([composite(rows, 0, 1, f) for f in range(2)], PREVIEWS / "bloomwild_tree_a_sway.gif", k=6, bg="g2",
        durations=[56, 40])
