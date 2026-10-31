"""cinder_props_m (32x32): lava boulder (pulsing cracks), obsidian shard cluster, bone-white slabs.
cinder_vent (32x48): steam vent with rising steam.

Rocks are built from flat facets (top plane lit, front mid, right side dark), HLD-style, then outlined.
"""

import math

import numpy as np

from draw import Canvas, poly, line, ellipse
from lib import SheetBuilder
from util_bayer import bayer4

W, H = 32, 32
ANCHOR = [16, 29]


def facets(c, faces):
    """Draw the facets on the native Cinder pixel grid."""
    for pts, ch in faces:
        poly(c, pts, ch)


# ---------------------------------------------------------------------------
# Lava boulder
# ---------------------------------------------------------------------------
def boulder_base():
    """Draw the boulder base on the native Cinder pixel grid."""
    c = Canvas(W, H)
    facets(c, [
        ([(3, 28), (3, 21), (6, 16), (10, 12), (15, 10), (21, 10), (26, 13), (29, 18), (29, 24), (27, 28)], "2"),
        ([(6, 16), (10, 12), (15, 10), (21, 10), (24, 14), (18, 16), (11, 17)], "4"),   # top plane
        ([(21, 10), (26, 13), (29, 18), (24, 19), (24, 14)], "3"),                      # upper right plane
        ([(3, 21), (6, 16), (11, 17), (12, 23), (5, 26), (3, 26)], "3"),                # front-left plane
        ([(11, 17), (18, 16), (24, 14), (24, 19), (22, 24), (12, 23)], "3"),            # front plane
        ([(24, 19), (29, 18), (29, 24), (22, 24)], "2"),                                # right plane
        ([(3, 26), (5, 26), (12, 23), (22, 24), (29, 24), (27, 28), (3, 28)], "1"),     # underside
    ])
    line(c, [(7, 15), (10, 13), (14, 11)], "5")        # lit rim on the top plane's far-left edge
    line(c, [(11, 17), (17, 16)], "5")                  # top plane's front edge catches light
    line(c, [(4, 21), (6, 17)], "4")
    line(c, [(19, 17), (23, 15)], "2", only_on="3")     # front plane falls off to the right
    line(c, [(20, 18), (21, 22)], "2", only_on="3")
    return c


# main crack from the top plane down the front, two branches; heat 0..3 at full pulse
CRACK_LINES = [
    ([(14, 11), (15, 14), (14, 17), (16, 20), (15, 23)], 1),
    ([(16, 20), (19, 21), (22, 23)], 0),
    ([(15, 14), (18, 13), (20, 12)], 0),
    ([(14, 17), (11, 19), (9, 22)], 0),
]
HOT = {(14, 17): 3, (16, 20): 3, (15, 14): 2, (15, 18): 2, (16, 19): 2, (15, 15): 2, (15, 16): 2}


def boulder(frame=0):
    """Draw the boulder on the native Cinder pixel grid."""
    c = boulder_base()
    m = Canvas(W, H)
    for pts, h in CRACK_LINES:
        line(m, pts, str(h))
    crack = [(x, y, int(m.a[y, x])) for y in range(H) for x in range(W) if m.a[y, x] != "."]
    for (x, y), h in HOT.items():
        pass
    heat = {(x, y): max(h, HOT.get((x, y), 0)) for x, y, h in crack}
    # charred lip below/right of each crack pixel (non-glowing)
    for (x, y) in heat:
        for dx, dy in ((1, 0), (0, 1)):
            if (x + dx, y + dy) not in heat and c.get(x + dx, y + dy) not in ".":
                c.set(x + dx, y + dy, "1")
    pulse = [0, 1, 1, 0][frame % 4]
    for (x, y), h in heat.items():
        c.set(x, y, "wxyz"[min(3, h + pulse) if h > 0 else (1 if pulse else 0)])
    c.outline("K")
    return c


# ---------------------------------------------------------------------------
# Obsidian shard cluster
# ---------------------------------------------------------------------------
def shard(c, base_l, base_r, tip, lean_face=0.45):
    """A triangular glass prism: lit left face, dark right face, split along a ridge."""
    (lx, ly), (rx, ry), (tx, ty) = base_l, base_r, tip
    mx = lx + (rx - lx) * lean_face
    poly(c, [(lx, ly), (tx, ty), (mx, ly)], "s")
    poly(c, [(mx, ly), (tx, ty), (rx, ry)], "q")
    # glints: the ridge line and the left edge near the tip
    line(c, [(round(tx - 0.5), round(ty)), (round(mx - 0.5), round(ly - 1))], "r")
    line(c, [(round(tx - 1), round(ty + 1)), (round(tx - 1 - (tx - lx) * 0.3), round(ty + (ly - ty) * 0.35))], "t")
    # right face darkens toward its outer edge
    line(c, [(round(tx), round(ty + 1)), (round(rx - 1), round(ry - 1))], "p")


def shard_cluster():
    """Draw the shard cluster on the native Cinder pixel grid."""
    c = Canvas(W, H)
    # back to front so nearer shards overlap
    shard(c, (17, 22), (24, 22), (22, 5))
    shard(c, (6, 24), (13, 24), (8, 9))
    shard(c, (22, 27), (29, 26), (27, 15))
    shard(c, (10, 28), (21, 28), (14, 6), 0.42)
    shard(c, (3, 29), (10, 29), (5, 19))
    # rubble base
    c.stamp("""
        ....srqq.......sqp...
        ..srrqqpp.....srqqpo.
        .srqqqqppo...srqqqppo
    """, 4, 27)
    c.outline("K")
    return c


# ---------------------------------------------------------------------------
# Bone-white slabs
# ---------------------------------------------------------------------------
def stele():
    """A standing distributor slab with two drainage slots and a chipped top."""
    c = Canvas(W, H)
    facets(c, [
        ([(10, 29), (10, 8), (12, 5), (20, 5), (20, 29)], "9"),     # front face
        ([(20, 29), (20, 5), (23, 7), (23, 28)], "7"),               # right side (shadow)
        ([(10, 8), (12, 5), (20, 5), (23, 7), (21, 8), (11, 9)], "V"),  # top
    ])
    line(c, [(10, 9), (10, 28)], "V")
    line(c, [(12, 5), (19, 5)], "W")
    line(c, [(11, 8), (20, 8)], "W")
    # chip off the top-right corner
    for x, y in [(21, 5), (22, 6), (23, 6), (22, 7), (23, 7), (20, 5)]:
        c.set(x, y, ".")
    c.set(20, 6, "V")
    c.set(21, 7, "W")
    # Two deeply cut drains continue the ruined waterworks vocabulary.
    for x in (13, 17):
        c.rect(x, 12, x + 1, 24, "6")
        c.vline(x + 2, 13, 24, "V")
    c.hline(12, 20, 25, "8")
    # crack running down from the chip
    line(c, [(19, 9), (18, 12), (19, 15), (19, 18)], "7", only_on="9")
    # soot at the base
    line(c, [(10, 27), (19, 27)], "8", only_on="9V")
    line(c, [(10, 28), (19, 28)], "7", only_on="9V8")
    c.outline("K")
    return c


def fallen_slab():
    """A thick slab lying on the ground in 3/4 view, its right end snapped off and lying beside it."""
    c = Canvas(W, H)
    facets(c, [
        ([(2, 17), (19, 14), (21, 21), (5, 24)], "9"),      # top face
        ([(5, 24), (21, 21), (21, 24), (5, 27)], "7"),      # front edge (faces the camera, away from light)
        ([(2, 17), (5, 24), (5, 27), (2, 20)], "V"),        # left end, faces the light
    ])
    line(c, [(2, 17), (18, 14)], "W")
    line(c, [(3, 18), (3, 20)], "W")
    # snapped right end: jagged, with the fracture face lit where it faces up
    c.stamp("""
        ,,,
        9,,
        99,
        V9,,
        9V,,
        7V,,
        77,
        7,,
    """, 19, 14)
    # A broken water channel opens at the snapped end, rather than an ornamental border.
    poly(c, [(5, 19), (19, 16), (20, 19), (6, 22)], "6", only_on="9")
    line(c, [(6, 22), (18, 20)], "V", only_on="9")
    # the broken piece, tipped up on its edge beside the slab
    c.stamp("""
        ..WV9.
        .WV998
        WVV987
        V99877
        88776.
        .766..
    """, 23, 21)
    # soot along the front edge's foot
    line(c, [(6, 26), (20, 23)], "6", only_on="7")
    c.outline("K")
    return c


def slab_bits():
    """Draw the slab bits on the native Cinder pixel grid."""
    c = Canvas(W, H)
    c.stamp("""
        ..WVV....
        .WVV99...
        WVV99987.
        V9999877.
        .8877766.
    """, 4, 24)
    c.stamp("""
        .VV9.
        V9987
        88776
    """, 16, 26)
    c.stamp("""
        V9.
        877
    """, 23, 27)
    c.outline("K")
    return c


# ---------------------------------------------------------------------------
# Steam vent (separate sheet, taller cell)
# ---------------------------------------------------------------------------
VW, VH = 32, 48
V_ANCHOR = [16, 45]


def vent_base():
    """A low crusted cone with a crater: far inner wall in shadow, a glowing throat, mineral crust on the lip."""
    c = Canvas(VW, VH)
    facets(c, [
        ([(3, 45), (6, 40), (11, 36), (21, 36), (26, 40), (29, 45)], "3"),
        ([(6, 40), (11, 36), (21, 36), (26, 40), (21, 42), (11, 42)], "4"),
        ([(21, 42), (26, 40), (29, 45), (22, 45)], "2"),
        ([(3, 44), (29, 44), (29, 45), (27, 46), (5, 46), (3, 45)], "1"),
    ])
    line(c, [(6, 41), (9, 38), (12, 37)], "4")
    c.stamp("""
        ...4444433...
        .4422222213..
        4422111111123
        42211uuuu1123
        .421uwxxwu12.
        ..33uvwwv33..
    """, 10, 36)
    # crust: pale ash clusters on the lit rim, a little sulfur
    c.stamp("""
        fe......
        .ff.....
    """, 9, 37)
    c.stamp("""
        .fe
        ff.
    """, 21, 37)
    c.stamp("""
        ef
    """, 7, 41)
    c.outline("K")
    return c


N_VENT = 12
SPAWN = 2


def puffs(frame):
    """Puff list (cx, cy, r), oldest (highest) first. A new puff every SPAWN frames, each living N_VENT frames."""
    out = []
    for i in range(N_VENT // SPAWN):
        age = (frame + i * SPAWN) % N_VENT
        k = (age + 0.5) / N_VENT
        rise = 30 * (1 - (1 - k) ** 1.5)
        cy = 38.5 - rise
        cx = 15.5 + 5.0 * k * k + 1.0 * math.sin(2 * math.pi * frame / N_VENT + i * 1.7) * k
        r = 1.8 + 3.0 * k ** 0.7
        if k > 0.68:
            q = (k - 0.68) / 0.32
            rr = r * (1 - 0.75 * q) * 0.72
            out.append((k, cx + 2.0 + 2.5 * q, cy - 1.0 - 1.5 * q, rr * 0.8))
            out.append((k, cx - 1.2 - 2.0 * q, cy + 0.5, rr))
        else:
            out.append((k, cx, cy, r))
    out.sort(key=lambda p: -p[0])
    return [(x, y, r) for _, x, y, r in out]


def vent(frame, active=True):
    """Draw the vent on the native Cinder pixel grid."""
    c = vent_base()
    c.replace("u", "1")  # The recess shares the basalt core; no indistinct adjacent soot.
    if not active:
        return c
    steam = Canvas(VW, VH)
    for bx, by, br in puffs(frame):
        if br < 0.9:
            continue
        for y in range(int(by - br) - 1, int(by + br) + 2):
            for x in range(int(bx - br) - 1, int(bx + br) + 2):
                dx, dy = x + 0.5 - bx, y + 0.5 - by
                d2 = dx * dx + dy * dy
                if d2 > br * br:
                    continue
                d = math.sqrt(d2)
                rel = d / br
                lam = (dx * -0.6 + dy * -0.8) / max(d, 1e-6)
                if lam > 0.35 and rel > 0.5:
                    ch = "T"
                elif lam < -0.3 and rel > 0.62 and br > 2.2:
                    ch = "e"
                else:
                    ch = "S"
                steam.set(x, y, ch)
    for y in range(VH):
        for x in range(VW):
            ch = steam.a[y, x]
            if ch == ".":
                continue
            if c.get(x, y) not in ".1uw" and y >= 38:
                continue  # stays behind the near lip of the mound
            c.set(x, y, ch)
    c.replace("u", "1")  # The vent recess uses the basalt core, not an indistinct adjacent soot.
    return c


def build_all():
    """Build the medium props and vent sheets with their original animation contracts."""
    sb = SheetBuilder("cinder_props_m", (W, H), ANCHOR)
    sb.notes = "Broad fractured basalt, slotted stele and fallen sluice slab; cold faces stay calm."
    sb.add("lava_boulder", [boulder(f).rgba() for f in range(4)], durations=[20, 14, 20, 14], footprint=[22, 8],
           light={"color": "#efb68a", "radius": 36, "intensity": 0.7, "offset": [0, -10]})
    sb.add("shard_cluster", [shard_cluster().rgba()], footprint=[22, 6])
    sb.add("stele", [stele().rgba()], footprint=[12, 5])
    sb.add("slab_fallen", [fallen_slab().rgba()], footprint=[24, 8], anchor=[16, 28])
    sb.add("slab_bits", [slab_bits().rgba()])
    sv = SheetBuilder("cinder_vent", (VW, VH), V_ANCHOR)
    sv.notes = "Inactive in the region. Steam uses quiet ash tones, without extra scene particles."
    sv.add("vent", [vent(f).rgba() for f in range(N_VENT)], fps=8, footprint=[18, 5],
           light={"color": "#d48670", "radius": 18, "intensity": 0.5, "offset": [0, -6]})
    sv.add("vent_idle", [vent(0, active=False).rgba()], footprint=[18, 5],
           light={"color": "#d48670", "radius": 14, "intensity": 0.4, "offset": [0, -6]})
    return [sb, sv]


if __name__ == "__main__":
    from view import show
    print(show([boulder(0).rgba(), boulder(2).rgba(), shard_cluster().rgba(), stele().rgba(), fallen_slab().rgba(), slab_bits().rgba()], "props_m_v1"))
    print(show([vent(f).rgba() for f in range(0, N_VENT, 2)], "vent_v1"))
