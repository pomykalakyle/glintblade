"""bloomwild_tiles: 16x16 ground tiles for Bloomwild Ruins.

Light comes from the upper left. Ground gets no outlines.
Run: uv run --no-project --with pillow --with numpy python3 tiles.py
"""
import math

import numpy as np

from pix import *

T = 16
TILES = {}  # name -> 16x16 image
ORDER = []  # names in sheet order (rows of 16)


def add(name, img):
    """Add on the native pixel grid."""
    assert img.shape[:2] == (T, T), name
    TILES[name] = img
    ORDER.append(name)


# ================================================================ grass
GRASS = {"base": "g2", "lit": "g3", "tip": "g4", "dark": "g1", "deep": "g0"}
TEAL = {"base": "t2", "lit": "t3", "tip": "t4", "dark": "t1", "deep": "t0"}


def blade_tuft(img, x, y, blades, pal, tip=False, root=True):
    """blades: list of (dx, height). Base of the tuft sits on row y."""
    for i, (dx, h) in enumerate(blades):
        for k in range(h):
            put(img, x + dx, y - k, pal["lit"])
        if tip and h >= 2 and i == 0:
            put(img, x + dx, y - h + 1, pal["tip"])
        if root and h >= 2:
            put(img, x + dx, y + 1, pal["dark"])


TUFTS = [
    [(0, 2), (2, 3)],
    [(0, 1), (1, 2), (3, 2)],
    [(0, 3), (2, 2), (4, 1)],
    [(0, 2), (2, 1)],
    [(1, 2), (3, 3), (5, 2)],
]


def grass_tile(pal, var):
    """0 is plain (use it for most of the ground); the rest carry one deliberate detail."""
    g = new(T, T, pal["base"])
    if var == 0:
        pass
    elif var == 1:
        blade_tuft(g, 5, 9, [(0, 2), (2, 1)], pal)
    elif var == 2:
        blade_tuft(g, 6, 8, [(0, 2), (2, 3), (4, 1)], pal, tip=True)
    elif var == 3:
        blade_tuft(g, 3, 6, [(0, 2), (2, 1)], pal)
        blade_tuft(g, 7, 9, [(0, 1), (1, 2), (3, 3), (5, 1)], pal, tip=True)
    elif var == 4:
        # shadowed hollow with a tuft on its lip
        grid(["..DDDD..", ".DDDDDD.", "..DDDD.."], {"D": pal["dark"]}, g, 4, 9)
        blade_tuft(g, 5, 9, [(0, 2), (2, 1), (4, 2)], pal, root=False)
    elif var == 5:
        # white stone chip fallen from the ruins
        grid([".hs", "hss", "ss."], {"h": "s5", "s": "s3"}, g, 9, 9)
        put(g, 10, 12, pal["dark"]); put(g, 11, 11, pal["dark"]); put(g, 9, 12, pal["dark"])
        blade_tuft(g, 7, 11, [(0, 2)], pal)
    elif var == 6:
        # fallen petals
        for (x, y, c) in [(4, 5, "b4"), (5, 5, "b3"), (11, 9, "b3"), (7, 12, "b4")]:
            put(g, x, y, c)
    elif var == 7:
        # big clump
        blade_tuft(g, 4, 10, [(0, 1), (1, 3), (3, 2), (5, 3), (7, 1)], pal, tip=True)
    return g


def flower_tile(var):
    """Flower tile on the native pixel grid."""
    g = new(T, T, "g2")
    if var == 0:  # small coral daisies
        for (x, y) in [(4, 5), (10, 10)]:
            grid([".b.", "bYb", ".b."], {"b": "b4", "Y": "au3"}, g, x, y)
            put(g, x + 1, y + 3, "g1")
        blade_tuft(g, 7, 6, TUFTS[3], GRASS)
    elif var == 1:  # magenta cluster
        grid(["m.M", ".Mm"], {"m": "b2", "M": "b3"}, g, 6, 6)
        put(g, 6, 8, "g1"); put(g, 8, 8, "g1")
        grid(["M."], {"M": "b3"}, g, 11, 11)
        put(g, 11, 12, "g1")
        blade_tuft(g, 2, 12, TUFTS[1], GRASS)
    elif var == 2:  # cream star flowers
        for (x, y) in [(3, 9), (9, 4), (11, 11)]:
            grid([".w.", "wYw", ".w."], {"w": "s5", "Y": "au3"}, g, x, y)
            put(g, x + 1, y + 3, "g1")
    return g


# ================================================================ path (packed sand)
# Added color: sand #dcaa78 (between earth[4] and stone[4]) so paths read light and warm.
HEX["sa"] = HEX["e3"]
PATH_BASE = "sa"
PATH_SHADE = "e4"


def path_tile(var):
    """Path tile on the native pixel grid."""
    p = new(T, T, PATH_BASE)

    def pebble(x, y, big=False):
        """Pebble on the native pixel grid."""
        if big:
            grid(["hs.", "sss", ".dd"], {"h": "s5", "s": "s4", "d": "e4"}, p, x, y)
        else:
            grid(["h", "d"], {"h": "s4", "d": "e4"}, p, x, y)

    if var == 0:
        pass
    elif var == 1:
        pebble(6, 6, big=True)
        pebble(12, 12)
    elif var == 2:
        # worn rut
        for (x, y) in [(3, 9), (4, 9), (5, 10), (6, 10), (7, 10), (8, 11), (9, 11)]:
            put(p, x, y, PATH_SHADE)
        pebble(11, 4)
    elif var == 3:
        pebble(3, 11)
        pebble(10, 4)
    return p


# ================================================================ ruin floor (white flagstones)
RUIN_LEG = {"S": "s4", "h": "s5", "m": "s3", "d": "s2", "G": "g3", "g": "g2", "k": "g1",
            "E": "e1", "e": "e2", "r": "e3", "L": "g4"}
RUIN = [
    [  # 0 plain slab
        "mmmmmmmmmmmmmmmm",
        "mhhhhhSSSSSSSSSS",
        "mhSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
    ],
    [  # 1 two half slabs
        "mmmmmmmmmmmmmmmm",
        "mhhhSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "dmmmmmmmmmmmmmmm",
        "mhhhhhhSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
    ],
    [  # 2 four small slabs
        "mmmmmmmmmmmmmmmm",
        "mhhhSSSmhhSSSSSS",
        "mhSSSSSmSSSSSSSS",
        "mSSSSSSmSSSSSSSS",
        "mSSSSSSmSSSSSSSS",
        "mSSSSSSmSSSSSSSS",
        "mSSSSSSmSSSSSSSS",
        "mSSSSSSmSSSSSSSS",
        "mmmmmmmdmmmmmmmm",
        "mhhSSSSSmhhhSSSS",
        "mSSSSSSSmSSSSSSS",
        "mSSSSSSSmSSSSSSS",
        "mSSSSSSSmSSSSSSS",
        "mSSSSSSSmSSSSSSS",
        "mSSSSSSSmSSSSSSS",
        "mSSSSSSSmSSSSSSS",
    ],
    [  # 3 long + narrow slab
        "mmmmmmmmmmmmmmmm",
        "mhhhhhSSSmhSSSSS",
        "mhSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSSmSSSSSS",
    ],
    [  # 4 cracked slab, moss in the crack
        "mmmmmmmmmmmmmmmm",
        "mhhhhSSSSSSmSSSS",
        "mhSSSSSSSSSmSSSS",
        "mSSSSSSSSSmSSSSS",
        "mSSSSSSSSSmSSSSS",
        "mSSSSSSSSmSSSSSS",
        "mSSSSSSSmgSSSSSS",
        "mSSSSSSSdSSSSSSS",
        "mSSSSSSmgSSSSSSS",
        "mSSSSSmmSSSSSSSS",
        "mSSSSmSSSSSSSSSS",
        "mSSSSmSSSSSSSSSS",
        "mSSSmSSSSSSSSSSS",
        "mSSmSSSSSSSSSSSS",
        "mSSmSSSSSSSSSSSS",
        "mSmSSSSSSSSSSSSS",
    ],
    [  # 5 half slabs, moss creeping along the joint
        "mmmmmmmmmmmmmmmm",
        "mhhhhhhhSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSgSSSSS",
        "dmmmmmmmgGgGgmmm",
        "mSSSSSSSSgSgSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
    ],
    [  # 6 broken slab: a chunk gone, earth and a sprout below
        "mmmmmmmmmmmmmmmm",
        "mhhhhSSSSSSSSSSS",
        "mhSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSmmmmmSSSS",
        "mSSSSSmEEEEEmmSS",
        "mSSSSmEEeeeEEEmS",
        "mSSSSmEeeGeeeeES",
        "mSSSSmeeGkGeeemS",
        "mSSSSSmeekeeemSS",
        "mSSSSSSmmeeemhSS",
        "mSSSSSSShhmmhSSS",
        "mSSSSSSSSShhSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
    ],
    [  # 7 mossy slab
        "mmmmmmmmmmmmmmmm",
        "mGGGGGhhSSSSSSSS",
        "mGgGGSSSSSSSSSSS",
        "mgGgSSSSSSSSSSSS",
        "mGgSSSSSSSSSSSSS",
        "mgSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
        "mSSSSSSSSSSSSSSS",
    ],
]


import slabs as SLABS


def ruin_tile(var):
    """0-7: 1x1 flagstone fillers (one slab or two half slabs, a few worn/sunk). 8 cracked,
    9 moss along a joint, 10 small broken hole, 11 broken hole with a sprout. Most of a floor
    should come from the multi-tile slab_* blocks (see slabs.py)."""
    return SLABS.filler(var) if var < 8 else SLABS.accent(var)


# ================================================================ transitions (corner-based)
def smooth(u):
    """Smooth on the native pixel grid."""
    u = min(max(u, 0.0), 1.0)
    return u * u * (3 - 2 * u)


CORNERS = [(0.0, 0.0), (16.0, 0.0), (0.0, 16.0), (16.0, 16.0)]  # nw ne sw se


def _dseg(px, py, a, b):
    """Dseg on the native pixel grid."""
    ax, ay = a
    bx, by = b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)


def _region_dist(px, py, on):
    """Distance from p to the 'core' of the on-corners: the corners plus tile edges joining two on-corners."""
    pts = [CORNERS[i] for i in range(4) if on[i]]
    d = min(math.hypot(px - x, py - y) for x, y in pts)
    for i, j in ((0, 1), (2, 3), (0, 2), (1, 3)):
        if on[i] and on[j]:
            d = min(d, _dseg(px, py, CORNERS[i], CORNERS[j]))
    return d


def corner_mask(bits, var, R=8.0):
    """bits = (nw, ne, sw, se), 1 = the other terrain. Returns bool[16,16] True = other terrain.
    The boundary is a radius-8 curve around the 'on' corners (or around the 'off' corners when
    three are on / on the saddle), so it always crosses tile edges at their midpoints and
    tiles with matching corners connect. `var` changes the wobble inside the tile."""
    on = list(bits)
    n = sum(on)
    invert = n == 3 or tuple(bits) in ((1, 0, 0, 1), (0, 1, 1, 0))
    core = [1 - b for b in on] if invert else on
    m = np.zeros((T, T), bool)
    ph = var * 2.1 + sum(i * b for i, b in enumerate(bits)) * 0.7
    for y in range(T):
        for x in range(T):
            px, py = x + 0.5, y + 0.5
            d = _region_dist(px, py, core)
            # wobble in pixels, faded to zero at the tile border so crossings stay fixed
            e = min(1.0, min(px, py, T - px, T - py) / 4.0)
            w = 1.1 * e * math.sin(px * 0.42 + py * 0.31 + ph) + 0.4 * e * math.sin(py * 0.6 - px * 0.25 + ph * 1.7)
            inside = d + w < R
            m[y, x] = (not inside) if invert else inside
    # tidy: remove isolated pixels / notches (interior only so seams stay identical)
    for _ in range(2):
        src = m.copy()
        for y in range(1, T - 1):
            for x in range(1, T - 1):
                nn = [src[y - 1, x], src[y + 1, x], src[y, x - 1], src[y, x + 1]]
                if sum(nn) >= 3 and not src[y, x]:
                    m[y, x] = True
                elif sum(nn) <= 1 and src[y, x]:
                    m[y, x] = False
    return m


def grass_over(base_other, grass_img, m, other_shadow, pal=GRASS, var=0):
    """Compose grass over another terrain using mask m (True = other terrain shows).
    Grass sits slightly above the other terrain: its north-facing lip catches the light, it casts
    a 1px shadow onto the terrain below / right of it, and a few blades poke over the edge."""
    out = grass_img.copy()
    out[m] = base_other[m]
    for y in range(T):
        for x in range(T):
            if m[y, x]:
                if (y > 0 and not m[y - 1, x]) or (x > 0 and not m[y, x - 1] and not m[y, x - 1 if x > 0 else x]):
                    put(out, x, y, other_shadow)
            else:
                if y > 0 and m[y - 1, x]:
                    put(out, x, y, pal["lit"])
    # blade tips poking up from north-facing lips (interior only, so seams stay identical)
    for y in range(2, T - 2):
        for x in range(2, T - 2):
            if m[y, x] and not m[y + 1, x] and m[y - 1, x] and hash01(x, y, var, 5) < 0.22:
                put(out, x, y, pal["lit"])
    # blades hanging over south-facing edges, with their shadow below
    for y in range(2, T - 2):
        for x in range(2, T - 2):
            if m[y, x] and not m[y - 1, x] and m[y + 1, x] and hash01(x, y, var, 7) < 0.2:
                put(out, x, y, pal["base"])
                put(out, x, y + 1, other_shadow)
    return out


def grass_blend(teal_img, grass_img, m, var=0):
    """Emerald grass meeting teal grass: no lip, just an organic edge with a few blades of
    each color reaching across it."""
    out = grass_img.copy()
    out[m] = teal_img[m]
    for y in range(2, T - 2):
        for x in range(2, T - 2):
            if m[y, x] and not m[y + 1, x] and hash01(x, y, var, 31) < 0.3:
                put(out, x, y, "g2")
                if hash01(x, y, 32) < 0.5:
                    put(out, x, y - 1, "g3")
            if not m[y, x] and m[y + 1, x] and hash01(x, y, var, 33) < 0.3:
                put(out, x, y + 1, "t3")
    return out


def bits_name(bits):
    """Bits name on the native pixel grid."""
    return "".join(str(b) for b in bits)


ALL_BITS = [(a, b, c, d) for a in (0, 1) for b in (0, 1) for c in (0, 1) for d in (0, 1)]


def build_transition(prefix, other_fn, other_shadow, grass_var=0, blend=False):
    """Build transition on the native pixel grid."""
    for bits in ALL_BITS:
        if bits in ((0, 0, 0, 0), (1, 1, 1, 1)):
            continue
        nvar = 2 if sum(bits) == 2 and bits not in ((1, 0, 0, 1), (0, 1, 1, 0)) else 1
        for var in range(nvar):
            m = corner_mask(bits, var)
            g = grass_tile(GRASS, 0)
            o = other_fn(var)
            t = grass_blend(o, g, m, var) if blend else grass_over(o, g, m, other_shadow, var=var)
            name = f"{prefix}_{bits_name(bits)}" + (f"_{var}" if var else "")
            add(name, t)


# ================================================================ cliffs and island edge
# Rock face comes from rock.py's 48x16 band (wraps in x and y). Tile variant k uses band
# columns 16k..16k+15, so the builder picks `_c_{tx % 3}` by tile column for a seamless face.
import rock as ROCK

BAND = ROCK.band()


def rock_texture(seed):
    """Rock texture on the native pixel grid."""
    k = seed % 3
    return [[BAND[y][16 * k + x] for x in range(16)] for y in range(16)]


HAZE1 = {"s0": "s0", "s1": "c0", "s2": "c1", "s3": "c1", "g0": "c0", "g1": "c0", "g2": "c1", "g3": "c1", "e1": "c0"}
HAZE2 = {"s0": "c0", "s1": "c1", "s2": "c2", "s3": "c2", "g0": "c1", "g1": "c1", "g2": "c2", "g3": "c2", "e1": "c1"}


def lip(img, seed, y0=0):
    """Grass top surface ending at a south-facing lip, with overhanging grass and a vine."""
    for x in range(T):
        for y in range(y0, y0 + 3):
            put(img, x, y, "g2")
        put(img, x, y0 + 3, "g1")
        put(img, x, y0 + 4, "s0")
    hang = [(1, 1), (4, 2), (5, 1), (9, 1), (12, 2), (13, 1)] if seed % 2 == 0 else [(2, 2), (3, 1), (7, 1), (10, 2), (14, 1)]
    for hx, hl in hang:
        for k in range(hl):
            put(img, hx, y0 + 4 + k, "g1")
    # sunlit grass along the lip in broken runs
    for x in range(T):
        if hash01(x // 2, seed, 21) > 0.45:
            put(img, x, y0 + 2, "g3")
    # a vine with a few leaves
    vx = 8 if seed % 2 == 0 else 11
    vl = 7 + seed % 3
    for k in range(vl):
        put(img, vx + (1 if 3 <= k <= 5 else 0), y0 + 5 + k, "g1")
    put(img, vx - 1, y0 + 7, "g2"); put(img, vx + 2, y0 + 9, "g2"); put(img, vx + 1, y0 + 5 + vl, "g2")


# Island underside: bottom edge profile over the 48 px band (rows of the edge_under tile;
# transparent below). Three hanging chunks of rock with clean, evenly stepping sides.
UNDER = [6, 6, 7, 7, 8, 9, 10, 11, 12, 13, 13, 14, 13, 12, 10, 9, 8, 7, 7, 6, 6, 5, 5, 5,
         6, 6, 7, 8, 9, 9, 10, 10, 11, 11, 10, 9, 8, 7, 7, 6, 6, 6, 5, 5, 5, 5, 6, 6]
ROOTS = {3: 3, 17: 5, 29: 2, 44: 4}  # band x -> length of a dangling root below the edge
# Haze lines per rock column (band x0 -> row where the next haze level starts), so the
# aerial-perspective fade follows the rock's own structure instead of a straight seam.
HAZE_ROW1 = {0: 10, 7: 13, 16: 8, 22: 12, 26: 9, 32: 14, 40: 11}   # inside edge_mid
HAZE_ROW2 = {0: 5, 7: 3, 16: 7, 22: 4, 26: 6, 32: 2, 40: 5}       # inside edge_haze


def _col_start(bx):
    """Col start on the native pixel grid."""
    starts = sorted(HAZE_ROW1)
    return max([s for s in starts if s <= bx])


def cliff_tile(kind, seed=0, haze=None, end=None):
    """kind: top | mid | bot | haze | under. Island-edge kinds: 'emid' (face with the first haze
    step), 'haze' (fully hazy face), 'under' (underside; transparent below a jagged edge)."""
    k = seed % 3
    tex = rock_texture(k)
    img = new(T, T)
    oy = {"top": -5}.get(kind, 11)
    for y in range(T):
        for x in range(T):
            put(img, x, y, tex[(y + oy) % 16][x])
    if end:
        side_strip(img, "w" if end == "l" else "e", 0, T)
    if kind == "top":
        lip(img, seed)
        if end == "l":
            for y in range(0, 3):
                for x in range(0, 5):
                    put(img, x, y, "s2" if x else "s0")
            for y in range(0, 4):
                put(img, 5, y, "g3")
            put(img, 4, 3, "s0"); put(img, 5, 4, "s0")
        elif end == "r":
            for y in range(0, 3):
                for x in range(12, 16):
                    put(img, x, y, "s1" if x < 15 else "s0")
                put(img, 11, y, "g1")
            put(img, 11, 3, "g1")
    if kind == "bot":
        base_y = 10
        for x in range(T):
            for y in range(base_y, T):
                put(img, x, y, "g2")
            put(img, x, base_y - 1, "s0")
            put(img, x, base_y, "g0")
            put(img, x, base_y + 1, "g1")
            if hash01(x, seed, 4) < 0.5:
                put(img, x, base_y + 2, "g1")
        grid([".lm", "lmd", "dd."], {"l": "s3", "m": "s2", "d": "s1"}, img, 3 + 4 * k, base_y - 1)
        blade_tuft(img, 10 - 2 * k, base_y + 1, [(0, 2), (2, 3), (3, 1)], GRASS, root=False)
    if kind in ("emid", "haze", "under"):
        for x in range(T):
            bx = 16 * k + x
            c0 = _col_start(bx)
            for y in range(T):
                if kind == "emid":
                    lvl = 1 if y >= HAZE_ROW1[c0] else 0
                elif kind == "haze":
                    lvl = 2 if y >= HAZE_ROW2[c0] else 1
                else:
                    lvl = 2
                if lvl:
                    c = get(img, x, y)
                    m = HAZE1 if lvl == 1 else HAZE2
                    for a, b in m.items():
                        if c == rgba(a):
                            put(img, x, y, b)
                            break
    if kind == "under":
        for x in range(T):
            bx = 16 * k + x
            b = UNDER[bx]
            for y in range(b, T):
                put(img, x, y, None)
            # chunk sides: lit on the left-facing slope, dark on the right-facing one
            put(img, x, b - 1, "c0")
            left_b = UNDER[(bx - 1) % 48]
            if b > left_b:
                for y in range(left_b, b - 1):
                    put(img, x, y, "c2")
            if bx in ROOTS:
                for i in range(ROOTS[bx]):
                    put(img, x + (1 if i == ROOTS[bx] - 1 and x < 15 else 0), b + i, "c0")
    if haze:
        img = recolor(img, haze)
    return img


def side_strip(img, side, y0=0, y1=T, haze=None):
    """The narrow visible face along a raised area's west (lit) or east (shaded) edge:
    columns 0-4 (west) or 11-15 (east); stacked blocks with lit ledges."""
    for y in range(y0, y1):
        r = y % 16
        ledge = r in (4, 11)
        top = r in (5, 12)
        if side == "w":
            row = ["s0", "s0", "s0", "s0"] if ledge else (["s0", "s3", "s3", "s2"] if top else ["s0", "s2", "s2", "s1"])
            for i, c in enumerate(row):
                put(img, i, y, c)
            put(img, 4, y, "s1")
        else:
            row = ["s0", "s0", "s0", "s0"] if ledge else (["s2", "s2", "s1", "s0"] if top else ["s1", "s1", "s1", "s0"])
            for i, c in enumerate(row):
                put(img, 12 + i, y, c)
    if haze:
        return recolor(img, haze)
    return img


def rim_tile(side):
    """Top-surface edges of a raised area where the face isn't seen head-on.
    n: lit lip; w: lit side strip; e: shaded side strip; corners combine them.
    Transparent pixels = lower ground (or the void, on the island edge)."""
    img = grass_tile(GRASS, 0)
    if "n" in side:
        for x in range(T):
            put(img, x, 0, "g1")
            put(img, x, 1, "g3")
    if "w" in side:
        side_strip(img, "w", 1 if "n" in side else 0)
        for y in range(T):
            put(img, 5, y, "g3")
        if "n" in side:
            for x in range(5):
                put(img, x, 0, None)
            put(img, 5, 0, "g1"); put(img, 4, 1, "g1")
    if "e" in side:
        side_strip(img, "e", 1 if "n" in side else 0)
        for y in range(T):
            put(img, 11, y, "g1")
        if "n" in side:
            for x in range(12, 16):
                put(img, x, 0, None)
            put(img, 11, 0, "g1")
    return img


def _haze_px(img, x, y, lvl):
    """Haze px on the native pixel grid."""
    if lvl == 0:
        return
    c = get(img, x, y)
    m = HAZE1 if lvl == 1 else HAZE2
    for a, b in m.items():
        if c == rgba(a):
            put(img, x, y, b)
            return


def cliff_end(kind, end, seed=0, haze=None):
    """Left (lit) or right (shaded) end of a cliff/edge run: the face turns a corner and shows
    a narrow side strip; for 'top' the grass surface wraps around the corner."""
    img = cliff_tile(kind, seed)
    y0 = 3 if kind == "top" else 0
    y1 = 10 if kind == "bot" else T
    xs = range(0, 5) if end == "l" else range(11, 16)
    # redraw the strip from plain stone, then apply this kind's haze to it
    side_strip(img, "w" if end == "l" else "e", y0, y1)
    if kind == "top":
        if end == "l":
            for y in range(0, 3):
                for x in range(0, 5):
                    put(img, x, y, "s3" if 0 < x < 4 else "s0")
            put(img, 5, 0, "g3"); put(img, 5, 1, "g3"); put(img, 5, 2, "g3"); put(img, 5, 3, "g1")
        else:
            for y in range(0, 3):
                for x in range(11, 16):
                    put(img, x, y, "s1" if 11 < x < 15 else "s0")
                put(img, 10, y, "g1")
    if kind == "bot":
        for x in xs:
            put(img, x, y1 - 1, "s0")
    for x in xs:
        for y in range(T):
            if kind == "emid":
                _haze_px(img, x, y, 1 if y >= 10 else 0)
            elif kind == "haze":
                _haze_px(img, x, y, 2 if y >= 4 else 1)
            elif kind == "under":
                _haze_px(img, x, y, 2)
    if kind == "under":
        # the underside's corner: taper the strip up toward the outer side
        for x in xs:
            d = x if end == "l" else 15 - x
            b = 4 + d  # outermost column shortest
            b = min(b, UNDER[(16 * (seed % 3) + x) % 48])
            for y in range(b, T):
                put(img, x, y, None)
            put(img, x, b - 1, "c0")
    if haze:
        img = recolor(img, haze)
    return img


# ================================================================ build
def build_all():
    """Build all on the native pixel grid."""
    TILES.clear(); ORDER.clear()
    for v in range(8):
        add(f"grass_{v}", grass_tile(GRASS, v))
    for v in range(8):
        add(f"teal_{v}", grass_tile(TEAL, v))
    for v in range(3):
        add(f"flowers_{v}", flower_tile(v))
    for v in range(4):
        add(f"path_{v}", path_tile(v))
    for v in range(12):
        add(f"ruin_{v}", ruin_tile(v))
    build_transition("grass_path", lambda v: path_tile(0), PATH_SHADE)
    build_transition("grass_ruin", lambda v: ruin_tile(0), "s3")
    build_transition("grass_teal", lambda v: grass_tile(TEAL, 0), None, blend=True)
    for kind in ("top", "mid", "bot"):
        add(f"cliff_{kind}_l", cliff_end(kind, "l", 0))
        for k in range(3):
            add(f"cliff_{kind}_c_{k}", cliff_tile(kind, k))
        add(f"cliff_{kind}_r", cliff_end(kind, "r", 2))
    for side in ("n", "w", "e", "nw", "ne"):
        add(f"rim_{side}", rim_tile(side))
    # island south edge: lip + face (edge_top/mid), hazier lower face, then the underside
    for kind, kk in (("top", "top"), ("mid", "emid"), ("haze", "haze"), ("under", "under")):
        add(f"edge_{kind}_l", cliff_end(kk, "l", 0))
        for k in range(3):
            add(f"edge_{kind}_c_{k}", cliff_tile(kk, k))
        add(f"edge_{kind}_r", cliff_end(kk, "r", 2))

GROUPS_DOC = {
    "grass": "emerald grass; _0 is plain (use for ~70% of cells), 1-7 carry one detail each",
    "teal": "teal grass (shade, damp ground), same variants as grass",
    "flowers": "grass with small flowers",
    "path": "packed sand path; _0 plain",
    "slab_{a..i}": "white flagstone floor as multi-tile blocks (see \"blocks\": a,b,c 3x3 tiles; d,e 3x2; f,g 2x3; h,i 2x2). Tile names are slab_x_{cx}{cy}. Fill a floor greedily with random blocks (try bigger ones first), then fill leftover cells with ruin_0..7. Every block and filler has its joints on its top/left edges, so any packing is seamless",
    "ruin": "1x1 flagstone pieces with the same edge rule as the slab blocks: ruin_0..7 fillers (use for leftover cells only), ruin_8 cracked, ruin_9 moss along a joint, ruin_10 small broken hole, ruin_11 broken hole with a sprout (accents: a few per floor at most)",
    "grass_path_NNNN / grass_ruin_NNNN / grass_teal_NNNN": "corner transitions. Terrain is defined per tile CORNER (vertex). Bits are the corners nw,ne,sw,se; 1 = path/ruin/teal at that corner, 0 = grass. Straight edges (0011, 1100, 0101, 1010) have a _1 variant: mix them",
    "cliff_{top,mid,bot}_{l,c_0,c_1,c_2,r}": "south-facing face of raised ground, 3 rows tall (repeat mid for taller). Pick c_{tx % 3} by tile column so the rock is seamless. l/r are the ends (they include the side strip). bot ends with lower grass at rows 10-15",
    "rim_{n,w,e,nw,ne}": "top-surface edges of raised ground (or the island) where no face is seen: n = lit lip, w = lit side strip, e = shaded side strip. Transparent pixels show what is below: lower grass for a plateau, the void/backdrop on the island edge",
    "edge_{top,mid,haze,under}_{l,c_0,c_1,c_2,r}": "island's south edge falling away into the clouds: lip, rock face, hazier face, underside (transparent below its jagged bottom). Stack top, mid, haze, under; pick c_{tx % 3}",
}


def write_sheet():
    """Write sheet on the native pixel grid."""
    build_all()
    cols = 16
    rows = []
    pos = {}
    for i, n in enumerate(ORDER):
        r, c = divmod(i, cols)
        if c == 0:
            rows.append([])
        rows[-1].append(TILES[n])
        pos[n] = [c, r]
    while len(rows[-1]) < cols:
        rows[-1].append(new(T, T))
    # multi-tile slab blocks: shelf-pack below the single tiles
    blocks_doc = {}
    bl = SLABS.blocks()
    r0 = len(rows)
    shelf_y, shelf_x, shelf_h = r0, 0, 0
    placed = []
    for name, tw, th, _ in SLABS.BLOCK_SPECS:
        if shelf_x + tw > cols:
            shelf_y += shelf_h; shelf_x, shelf_h = 0, 0
        placed.append((name, shelf_x, shelf_y, tw, th))
        shelf_x += tw; shelf_h = max(shelf_h, th)
    total_rows = shelf_y + shelf_h
    while len(rows) < total_rows:
        rows.append([new(T, T) for _ in range(cols)])
    for name, bx, by, tw, th in placed:
        img = bl[name]
        for cy in range(th):
            for cx in range(tw):
                rows[by + cy][bx + cx] = img[cy * T:(cy + 1) * T, cx * T:(cx + 1) * T].copy()
                pos[f"{name}_{cx}{cy}"] = [bx + cx, by + cy]
        blocks_doc[name] = {"at": [bx, by], "size": [tw, th]}
    manifest = {
        "cell": [16, 16],
        "anchor": [0, 0],
        "tiles": pos,
        "groups": GROUPS_DOC,
        "blocks": blocks_doc,
        "anims": {},
        "notes": "Bloomwild ground tiles, 16x16, light from the upper left. Tiles are opaque except rim_* (transparent outside the lip) and edge_under_* (transparent below the island's underside). Added color #dcaa78 (sand) as the path base. Collision suggestion: cliff_top/mid and the face part of cliff_bot = Wall; edge_* rows = Void; rim_* = Floor on the grass part.",
    }
    sheet = save_sheet("bloomwild_tiles", rows, T, T, manifest)
    preview(sheet, PREVIEWS / "bloomwild_tiles_8x.png", k=8, bg="o", gridcell=(16, 16))
    fl = SLABS.floor_preview(24, 14)
    save(scale(fl, 3), PREVIEWS / "bloomwild_slab_floor_3x.png")
    return sheet


if __name__ == "__main__":
    sh = write_sheet()
    print(len(ORDER), "tiles", sh.shape, len(colors_used(sh)), "colors")
