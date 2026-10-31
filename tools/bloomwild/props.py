"""bloomwild_props: bushes, urns, flower clumps, mossy boulders.

Same rendering rules as the trees: forms shaded as one lit volume from the upper left with a
little per-clump detail, quantized to the ramp; darkest ramp color only on the shadow-side
silhouette; a short contact line at the base. Detail is kept to what each object really has
(leaf clumps, blossoms on the flowering bush, a painted band on the urns, moss on top of rocks).
"""
import math

from pix import *

CW, CH = 32, 32
AX, AY = 16, 29

GREEN = ["g0", "g1", "g2", "g3", "g4", "g5"]
STONE = ["s0", "s1", "s2", "s3", "s4", "s5"]


def blob(clusters, ramp, th=(0.8, 0.52, 0.12), glints=(), leafy=True, flat_base=None):
    """clusters: (x, y, rx, ry). Returns (img, lvl grid)."""
    x_lo = min(c[0] - c[2] for c in clusters); x_hi = max(c[0] + c[2] for c in clusters)
    y_lo = min(c[1] - c[3] for c in clusters); y_hi = max(c[1] + c[3] for c in clusters)
    cx, cy = (x_lo + x_hi) / 2, (y_lo + y_hi) / 2
    rx, ry = (x_hi - x_lo) / 2, (y_hi - y_lo) / 2
    L = (-0.52, -0.62, 0.58)
    lvl = [[-1] * CW for _ in range(CH)]
    for (x0, y0, crx, cry) in sorted(clusters, key=lambda c: c[1]):
        for y in range(CH):
            if flat_base is not None and y > flat_base:
                continue
            for x in range(CW):
                ux, uy = (x + 0.5 - x0) / crx, (y + 0.5 - y0) / cry
                if ux * ux + uy * uy > 1.0:
                    continue
                uz = math.sqrt(max(0.0, 1 - ux * ux - uy * uy))
                gx, gy = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
                gz = math.sqrt(max(0.05, 1 - min(1.0, gx * gx + gy * gy)))
                nx, ny, nz = 0.6 * gx + 0.4 * ux, 0.6 * gy + 0.4 * uy, 0.6 * gz + 0.4 * uz
                lum = (nx * L[0] + ny * L[1] + nz * L[2]) / math.sqrt(nx * nx + ny * ny + nz * nz)
                if leafy:
                    lum += 0.09 * math.sin(x * 1.3 + math.sin(y * 0.9) * 1.6) * math.sin(y * 1.1 + x * 0.45)
                lvl[y][x] = 4 if lum > th[0] else 3 if lum > th[1] else 2 if lum > th[2] else 1
    # tidy single pixels
    src = [r[:] for r in lvl]
    for y in range(1, CH - 1):
        for x in range(1, CW - 1):
            v = src[y][x]
            if v < 0:
                continue
            nb = [src[y - 1][x], src[y + 1][x], src[y][x - 1], src[y][x + 1]]
            if v not in nb:
                nn = [n for n in nb if n >= 0]
                if len(nn) >= 3:
                    lvl[y][x] = max(set(nn), key=nn.count)
    img = new(CW, CH)
    for y in range(CH):
        for x in range(CW):
            if lvl[y][x] >= 0:
                put(img, x, y, ramp[lvl[y][x]])
    for (gx, gy) in glints:
        for (dx, dy) in ((0, 0), (1, 0), (0, 1)):
            if opaque(img, gx + dx, gy + dy):
                put(img, gx + dx, gy + dy, ramp[5])
    return img, lvl, (cx, cy, rx, ry)


def shadow_rim(img, ramp0, cx, cy, ry):
    """Shadow rim on the native pixel grid."""
    m = mask_of(img)
    out = img.copy()
    for y in range(CH):
        for x in range(CW):
            if not m[y, x]:
                continue
            right_open = x + 1 >= CW or not m[y, x + 1]
            down_open = y + 1 >= CH or not m[y + 1, x]
            if (right_open and x > cx - 3) or (down_open and y > cy - ry * 0.2):
                out[y, x] = rgba(ramp0)
    return out


def contact(img, c="e0", inset=2):
    """A short dark contact line under the base row."""
    rows = [y for y in range(CH) if mask_of(img)[y].any()]
    by = rows[-1]
    xs = [x for x in range(CW) if opaque(img, x, by)]
    for x in xs[inset:len(xs) - inset]:
        put(img, x, by + 1, c)
    return img


# ================================================================ bushes
def bush(clusters, glints, blossoms=()):
    """Bush on the native pixel grid."""
    img, lvl, (cx, cy, rx, ry) = blob(clusters, GREEN, th=(0.9, 0.66, 0.26), glints=glints)
    for (bx, by, kind) in blossoms:
        # a blossom: 2x2 petals (b4 lit / b3) with a b2 shadow pixel; kind 1 = small (2 px)
        if kind == 0:
            grid(["ab", "bc"], {"a": "b5", "b": "b4", "c": "b3"}, img, bx, by)
        else:
            grid(["ab", ".c"], {"a": "b4", "b": "b3", "c": "b2"}, img, bx, by)
    img = shadow_rim(img, "g0", cx, cy, ry)
    return fix_staircases(contact(img, "g0"))


def bush_a():
    """Bush a on the native pixel grid."""
    return bush([(12, 22, 7, 6), (19, 21, 7, 6), (16, 17, 6, 5), (22, 25, 5, 4), (10, 25, 5, 4)],
                glints=[(12, 14)])


def bush_b():
    """Bush b on the native pixel grid."""
    return bush([(7, 24, 5, 4), (13, 22, 6, 5), (20, 22, 6, 5), (26, 24, 4, 4), (16, 26, 8, 3)],
                glints=[(10, 19)])


def bush_bloom():
    """Bush bloom on the native pixel grid."""
    return bush([(11, 21, 7, 6), (19, 20, 7, 7), (15, 15, 6, 5), (23, 25, 5, 4), (9, 25, 5, 4)],
                glints=[],
                blossoms=[(11, 13, 0), (17, 11, 0), (8, 18, 0), (21, 16, 1), (14, 19, 1), (24, 21, 1),
                          (18, 23, 1)])


# ================================================================ urns
EARTH = ["e0", "e1", "e2", "e3", "e4", "s5"]


def urn(h, belly_r, neck_w, band_y):
    """Terracotta urn: belly (sphere), short neck, rim lip; one painted white band.
    Lit from the upper left; the mouth shows as a dark ellipse."""
    img = new(CW, CH)
    base = AY
    bcy = base - belly_r * 0.9
    lvl = [[-1] * CW for _ in range(CH)]
    L = (-0.55, -0.55, 0.63)
    top = base - h
    for y in range(top, base + 1):
        for x in range(CW):
            u = x + 0.5 - AX
            # profile: belly circle below the neck, straight neck above
            if y + 0.5 >= bcy - belly_r * 0.8:
                dy = y + 0.5 - bcy
                if dy > belly_r * 0.85:
                    half = belly_r * 0.55  # foot
                else:
                    half = math.sqrt(max(0.0, belly_r ** 2 - dy * dy))
                half = max(half, neck_w / 2)
            else:
                half = neck_w / 2
            if abs(u) > half:
                continue
            nx = u / max(1.0, half)
            nz = math.sqrt(max(0.0, 1 - nx * nx))
            ny = (y + 0.5 - bcy) / (belly_r * 1.4) if y + 0.5 > bcy - belly_r * 0.8 else -0.1
            lum = nx * L[0] + ny * L[1] + nz * L[2]
            lvl[y][x] = 4 if lum > 0.78 else 3 if lum > 0.42 else 2 if lum > 0.0 else 1
    for y in range(CH):
        for x in range(CW):
            if lvl[y][x] >= 0:
                put(img, x, y, EARTH[lvl[y][x]])
    # painted band: s4 on the lit side, s3 in shade
    for x in range(CW):
        if opaque(img, x, band_y):
            c = get(img, x, band_y)
            put(img, x, band_y, "s4" if c in (rgba("e4"), rgba("e3")) else "s3")
    # rim lip (one row wider than the neck) and the mouth
    rim_y = top - 1
    for x in range(AX - neck_w // 2 - 1, AX + neck_w // 2 + 1):
        put(img, x, rim_y, "e4" if x < AX + 1 else "e3")
        put(img, x, rim_y - 1, "e3")
    for x in range(AX - neck_w // 2, AX + neck_w // 2):
        put(img, x, rim_y - 1, "e0")
    put(img, AX - neck_w // 2 - 1, rim_y - 1, "e4")
    img = shadow_rim(img, "e0", AX, bcy, belly_r)
    return fix_staircases(contact(img, "e0", 1))


# ================================================================ flowers
def flowers(spec):
    """spec: list of (x, y, stem_len, petal colors (lit, mid), leaf side)."""
    img = new(CW, CH)
    for (x, y, sl, (lit, mid), leaf) in spec:
        for k in range(sl):
            put(img, x, y + 1 + k, "g2" if k < sl - 1 else "g1")
        if leaf:
            put(img, x + leaf, y + sl - 1, "g3")
            put(img, x + 2 * leaf, y + sl - 2, "g3")
        # 5-petal head: plus shape, lit on the upper left, center pixel au2
        put(img, x, y - 1, lit); put(img, x - 1, y, lit); put(img, x + 1, y, mid); put(img, x, y + 1, mid)
        put(img, x, y, "au2")
    # a few grass blades at the base
    for (x, h) in ((11, 2), (14, 3), (19, 2), (21, 3)):
        for k in range(h):
            put(img, x, AY - k, "g3" if k == h - 1 else "g2")
    return img


def flowers_a():
    """Flowers a on the native pixel grid."""
    return flowers([(12, 20, 8, ("s5", "s4"), -1), (17, 18, 10, ("b4", "b3"), 1), (21, 22, 6, ("s5", "s4"), 1),
                    (15, 23, 5, ("b4", "b3"), 0)])


def flowers_b():
    """Flowers b on the native pixel grid."""
    return flowers([(13, 21, 7, ("b5", "b4"), -1), (18, 22, 6, ("b4", "b3"), 1), (16, 18, 10, ("s5", "s4"), 0)])


# ================================================================ boulders
def boulder(clusters, moss_rows, glints):
    """Boulder on the native pixel grid."""
    img, lvl, (cx, cy, rx, ry) = blob(clusters, STONE, th=(0.82, 0.5, 0.08), glints=glints, leafy=False)
    # moss cap: on the upward-facing top surface (first moss_rows rows of each column)
    for x in range(CW):
        ys = [y for y in range(CH) if opaque(img, x, y)]
        if not ys:
            continue
        rel = (x + 0.5 - (cx - rx)) / (2 * rx)
        n = int(round(moss_rows * (1.0 - abs(rel - 0.4) * 1.6)))
        for k in range(max(0, n)):
            y = ys[0] + k
            put(img, x, y, "g3" if k == 0 and rel < 0.6 else "g2" if k < n - 1 else "g1")
    img = shadow_rim(img, "s0", cx, cy, ry)
    return fix_staircases(contact(img, "s0"))


def boulder_large():
    """Boulder large on the native pixel grid."""
    return boulder([(13, 21, 9, 7), (20, 22, 8, 6), (16, 25, 11, 4)], moss_rows=3, glints=[])


def boulder_small():
    """Boulder small on the native pixel grid."""
    return boulder([(15, 24, 6, 5), (19, 25, 4, 4)], moss_rows=2, glints=[])


# ================================================================ sheet
def build():
    """Build on the native pixel grid."""
    return [
        ("bush_a", bush_a(), {"footprint": [16, 5]}),
        ("bush_b", bush_b(), {"footprint": [20, 5]}),
        ("bush_bloom", bush_bloom(), {"footprint": [16, 5]}),
        ("urn_large", urn(15, 7.0, 6, 20), {"footprint": [12, 5], "breakable": True}),
        ("urn_small", urn(11, 5.0, 4, 23), {"footprint": [8, 4], "breakable": True}),
        ("flowers_a", flowers_a(), {}),
        ("flowers_b", flowers_b(), {}),
        ("boulder_large", boulder_large(), {"footprint": [20, 6]}),
        ("boulder_small", boulder_small(), {"footprint": [10, 4]}),
    ]


def write():
    """Write on the native pixel grid."""
    items = build()
    rows = [[img] for _, img, _ in items]
    anims = {}
    for i, (name, _, extra) in enumerate(items):
        anims[name] = {"row": i, "frames": 1, "fps": 1, **extra}
    man = {
        "cell": [CW, CH],
        "anchor": [AX, AY],
        "anims": anims,
        "notes": "Bloomwild props, one 32x32 cell per row, anchor = base center. Bushes: round, "
                 "wide-low, and flowering. Urns (terracotta with a white band) are breakable. flowers_* have no "
                 "footprint (walk through them). Boulders have moss on their top surface.",
    }
    sheet = save_sheet("bloomwild_props", rows, CW, CH, man)
    return sheet, items


if __name__ == "__main__":
    sheet, items = write()
    scene = new(CW * len(items) + 12, CH, "g2")
    for i, (_, img, _) in enumerate(items):
        paste(scene, img, i * CW, 0)
    paste(scene, stand_in_figure(), CW * len(items) + 2, AY - 17)
    save(scale(scene, 4), PREVIEWS / "bloomwild_props_4x.png")
