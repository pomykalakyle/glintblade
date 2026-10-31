"""Summit ground tiles (16x16 cells). Run from this folder:

    uv run --no-project --with pillow --with numpy python3 tiles.py

Writes assets/sprites/summit_tiles.{png,json} and previews in assets/previews/.

Layers, bottom to top (what the region builder paints into Zone::ground):
  1. paving: 64x64 blocks (4x4 cells) of irregular running-bond flagstones. Every block has
     a grout line on its top row and left column only, so any block sits next to any other.
  2. ground: natural summit rock (opaque 16x16 tiles).
  3. fg_* overlays: flag<->ground transitions by corner (transparent = paving shows through).
  4. decals: moss and frost overlays (transparent background).
  5. cliff_* tiles: the plateau edge and the faces below it (transparent = void / sky).
Light comes from the upper left.
"""

import math
import random

import numpy as np

import cliffs
from fields import corner_field, field, flag_mask, pnoise
import decals
from common import (PREVIEWS, RGB, blank, colors_used, flatten, from_rows, get, over, parse_grid,
                    put, save_rgb, save_sheet)

T = 16
COLS = 12
GROUT = "b"

tiles = {}
groups = {}
blocks = {}
sheet = None
cursor = [0, 0]  # next free (col, row) for packing
row_h = [0]


def ensure(rows):
    global sheet
    if sheet is None:
        sheet = blank(COLS * T, rows * T)
    elif sheet.shape[0] < rows * T:
        g = blank(COLS * T, rows * T)
        g[: sheet.shape[0]] = sheet
        sheet = g


def add(name, img, group=None):
    """Pack an image (multiple of 16 in each dimension) into the sheet. Multi-cell images
    become a block whose cells are named name_XY (X = column, Y = row inside the block)."""
    bw, bh = img.shape[1] // T, img.shape[0] // T
    c, r = cursor
    if c + bw > COLS:
        c, r = 0, r + max(row_h[0], 1)
        row_h[0] = 0
    ensure(r + bh)
    sheet[r * T:(r + bh) * T, c * T:(c + bw) * T] = img
    if (bw, bh) == (1, 1):
        tiles[name] = [c, r]
    else:
        for ty in range(bh):
            for tx in range(bw):
                tiles[f"{name}_{tx}{ty}"] = [c + tx, r + ty]
        blocks[name] = {"at": [c, r], "size": [bw, bh]}
    if group:
        groups.setdefault(group, []).append(name)
    cursor[0], cursor[1] = c + bw, r
    row_h[0] = max(row_h[0], bh)


def newline():
    if cursor[0]:
        cursor[0], cursor[1] = 0, cursor[1] + max(row_h[0], 1)
        row_h[0] = 0


def fill(img, ch):
    for y in range(img.shape[0]):
        for x in range(img.shape[1]):
            put(img, x, y, ch)


# ---------------------------------------------------------------------------
# Paving
# ---------------------------------------------------------------------------
LIGHTER = {"D": "d", "d": "E", "E": "e"}


def slab(img, x0, y0, x1, y1, tone, hl=0.65):
    """A cut slab whose top surface spans x0..x1, y0..y1 (grout excluded)."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(img, x, y, tone)
    w = x1 - x0 + 1
    # moonlit top edge, fading out toward the right
    for x in range(x0 + 1, x0 + 1 + max(2, int((w - 2) * hl))):
        if x < x1:
            put(img, x, y0, "e")
    # softer lit left edge on the upper part
    for y in range(y0 + 1, y0 + 1 + (y1 - y0) // 2):
        put(img, x0, y, LIGHTER[tone])
    # thin front face (3/4 view)
    for x in range(x0, x1 + 1):
        put(img, x, y1, "c")


def crack(img, pts, x0, y0, x1, y1):
    """Draw a crack through the points (clipped to the slab). Dark core, and the lower lip
    catches light where the crack runs sideways."""
    path = []
    for (ax, ay), (bx, by) in zip(pts, pts[1:]):
        n = max(abs(bx - ax), abs(by - ay))
        for i in range(n + 1):
            p = (round(ax + (bx - ax) * i / n), round(ay + (by - ay) * i / n))
            if not path or path[-1] != p:
                path.append(p)
    for i, (x, y) in enumerate(path):
        if x0 <= x <= x1 and y0 + 1 <= y <= y1 - 1:
            end = i in (0, len(path) - 1)
            put(img, x, y, "c" if end else "b")
    for i, (x, y) in enumerate(path[1:-1], 1):
        px, py = path[i - 1]
        if py == y and x0 <= x <= x1 and y0 + 1 <= y + 1 <= y1 - 1 and get(img, x, y + 1) not in ("b", "c"):
            if (x + y) % 2 == 0:
                put(img, x, y + 1, "E" if get(img, x, y + 1) == "d" else "e")


# Hand-drawn cracks: '#' = dark core, '-' = the lit far wall of the crack.
CRACKS = [
    parse_grid("""
        .#......
        .#......
        ..#.....
        ..#-....
        ...##...
        ...-.#..
        .....#..
        ....#-..
        ....#...
    """),
    parse_grid("""
        ......#.
        .....#..
        ....-#..
        ...##...
        ..#-....
        ..#.....
        ...#....
        ...#....
    """),
    parse_grid("""
        ..#.......
        ..#.......
        ...#......
        ...-##....
        ......#...
        ......-#..
        ........#.
    """),
]
CRACKS_TALL = [
    parse_grid("""
        ..#.......
        ..#.......
        ...#......
        ...#-.....
        ....##....
        ....-.#...
        ......#...
        .....#.#..
        .....#..#.
        ....#-....
        ....#.....
        .....#....
        .....#....
        ......#...
        ......#...
        .......#..
        .......#..
    """),
    parse_grid("""
        .......#..
        ......#...
        ......#...
        .....#-...
        ...##.....
        ..#-......
        ..#.......
        ...#......
        ...#......
        ....##....
        ....-.#...
        ......#...
    """),
]


def stamp_crack(img, tpl, ox, oy, x0, y0, x1, y1):
    for yy, r in enumerate(tpl):
        for xx, ch in enumerate(r):
            x, y = ox + xx, oy + yy
            if not (x0 <= x <= x1 and y0 <= y <= y1 - 1):
                continue
            if ch == "#":
                put(img, x, y, "b")
            elif ch == "-" and get(img, x, y) not in ("b", "c"):
                put(img, x, y, "e" if get(img, x, y) in ("d", "E") else "d")


def pit(img, x, y, tone):
    put(img, x, y, "c")
    put(img, x + 1, y + 1, LIGHTER[tone])


def chip(img, x, y0):
    put(img, x, y0, "b")
    put(img, x + 1, y0, "b")
    put(img, x, y0 + 1, "c")


def course(rng, H, wear=0.35, crack_n=0, split=False):
    """A 64 x H course strip (H = 16 or 32): grout on the top row, slabs between vertical
    joints. There is never a joint at the strip's left or right border: the first slab
    continues the previous strip's last slab, so strips chain in any order with no seam."""
    S = 64
    img = blank(S, H)
    fill(img, GROUT)
    for _ in range(200):
        n = rng.choice([1, 2, 2, 3]) if H == 16 else rng.choice([1, 2, 2])
        js = sorted(rng.sample(range(7, 58), n))
        if all(b - a >= 12 for a, b in zip(js, js[1:])):
            break
    edges = [-1] + js + [S]
    slabs = []
    for i in range(len(edges) - 1):
        x0, x1 = edges[i] + 1, edges[i + 1] - 1
        first, last = i == 0, i == len(edges) - 2
        tone = "d" if (first or last) else rng.choices(["d", "D", "E"], weights=[5, 3, 1])[0]
        parts = [(1, H - 1)]
        if split and H == 32 and not first and not last and x1 - x0 > 12 and rng.random() < 0.6:
            parts = [(1, 15), (17, 31)]
        for (y0, y1) in parts:
            for y in range(y0, y1 + 1):
                for x in range(x0, x1 + 1):
                    put(img, x, y, tone)
            L = x1 - x0 + 1
            if not first:
                # moonlit top edge from the slab's start, fading out
                hl = L - 1 if last and L < 12 else max(2, int((L - 2) * rng.choice([0.5, 0.6, 0.7])))
                for x in range(x0 + 1, min(x1, x0 + 1 + hl)):
                    put(img, x, y0, "e")
                for y in range(y0 + 1, y0 + 1 + (y1 - y0) // 2):
                    put(img, x0, y, LIGHTER[tone])
            for x in range(x0, x1 + 1):
                put(img, x, y1, "c")
            slabs.append((x0, y0, x1, y1, tone, first or last))
    # wear, at most one feature per slab, never on the strip-crossing slabs' cut ends
    order = list(range(len(slabs)))
    rng.shuffle(order)
    cracks_left = crack_n
    for i in order:
        x0, y0, x1, y1, tone, crossing = slabs[i]
        w, h = x1 - x0 + 1, y1 - y0 + 1
        if cracks_left and not crossing and w >= 14 and h >= 14:
            cracks_left -= 1
            tpl = rng.choice(CRACKS_TALL if h >= 20 else CRACKS)
            tw = len(tpl[0])
            sx = rng.randint(x0 + 2, max(x0 + 2, x1 - tw - 1))
            stamp_crack(img, tpl, sx, y0, x0, y0, x1, y1)
            continue
        if rng.random() > wear:
            continue
        kind = rng.choice(["pit", "pit", "chip", "corner"])
        if kind == "pit" and w > 9 and h > 6:
            pit(img, rng.randint(x0 + 3, x1 - 4), rng.randint(y0 + 2, y1 - 3), tone)
        elif kind == "chip" and w > 10 and not crossing:
            chip(img, rng.randint(x0 + 3, x1 - 4), y0)
        elif kind == "corner" and not (crossing and i == 0) and w > 6:
            put(img, x0, y0, "b")
            put(img, x0 + 1, y0, "b")
            put(img, x0, y0 + 1, "b")
            put(img, x0 + 1, y0 + 1, "c")
    return img


# ---------------------------------------------------------------------------
# Runes: original 9x9 glyphs, carved and inlaid with gold
# ---------------------------------------------------------------------------
RUNES = [
    parse_grid("""
        ....#....
        ...#.#...
        ..#...#..
        .#.....#.
        ....#....
        ....#....
        ..#####..
        ....#....
        ....#....
    """),
    parse_grid("""
        ....#....
        ...#.#...
        ..#.#.#..
        .#..#..#.
        #...#...#
        .#..#..#.
        ..#.#.#..
        ...#.#...
        ....#....
    """),
    parse_grid("""
        #.......#
        .#.....#.
        ..#...#..
        ...#.#...
        ....#....
        ....#....
        ..#.#.#..
        ...###...
        ....#....
    """),
    parse_grid("""
        ..#####..
        .#.....#.
        #...#...#
        #..#.#..#
        #...#...#
        .#.....#.
        ..##.##..
        ....#....
        ...###...
    """),
    parse_grid("""
        ....#....
        #...#...#
        .#..#..#.
        ..#.#.#..
        ....#....
        ..#.#.#..
        .#..#..#.
        #...#...#
        ....#....
    """),
    parse_grid("""
        ###...###
        #.......#
        #...#...#
        ...#.#...
        ..#...#..
        ...#.#...
        #...#...#
        #.......#
        ###...###
    """),
]


def inlay(img, glyph, ox, oy, lit, shadow=True):
    """Carve a glyph: the channel's upper-left wall is in shadow; gold sits in the channel.
    Dim: dull bronze (H). Lit: glowing gold (J) with a pale core (K) at junctions."""
    pts = [(x, y) for y, r in enumerate(glyph) for x, ch in enumerate(r) if ch == "#"]
    s = set(pts)
    gold = ("G", "H", "I", "J", "K")
    if shadow:
        for (x, y) in pts:
            if (x - 1, y - 1) not in s and get(img, ox + x - 1, oy + y - 1) not in gold:
                put(img, ox + x - 1, oy + y - 1, "c")
    for (x, y) in pts:
        put(img, ox + x, oy + y, "J" if lit else "H")
    if lit:
        for (x, y) in pts:
            n = sum((x + dx, y + dy) in s for dx, dy in
                    [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)])
            if n >= 3:
                put(img, ox + x, oy + y, "K")


def rune_inset(glyph, lit):
    """16x16 rune slab set into the paving: grout top, left and right; front face at the
    bottom (the next course's grout sits below it)."""
    img = blank(T, T)
    fill(img, GROUT)
    x0, y0, x1, y1 = 1, 1, 14, 15
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(img, x, y, "D")
    for x in range(x0 + 1, x0 + 9):
        put(img, x, y0, "e")
    for y in range(y0 + 1, y0 + 7):
        put(img, x0, y, "d")
    for x in range(x0, x1 + 1):
        put(img, x, y1, "c")
    inlay(img, glyph, 3, 3, lit)
    return img


# ---------------------------------------------------------------------------
# The Warden's seal: a 96x96 round stone set into the arena floor (overlay)
# ---------------------------------------------------------------------------

def circle_pts(cx, cy, r):
    """Midpoint circle: clean 1 px ring."""
    pts = set()
    x, y, d = r, 0, 1 - r
    while x >= y:
        for a, b in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)]:
            pts.add((cx + a, cy + b))
        y += 1
        if d < 0:
            d += 2 * y + 1
        else:
            x -= 1
            d += 2 * (y - x) + 1
    return pts


def disk(cx, cy, r):
    return {(x, y) for y in range(cy - r, cy + r + 1) for x in range(cx - r, cx + r + 1)
            if (x - cx) ** 2 + (y - cy) ** 2 <= r * r + r}


SMALL_RUNES = [
    parse_grid("""
        ..#..
        .#.#.
        #.#.#
        ..#..
        ..#..
    """),
    parse_grid("""
        #...#
        .#.#.
        ..#..
        ..#..
        .###.
    """),
    parse_grid("""
        .###.
        #...#
        ..#..
        .#.#.
        #...#
    """),
    parse_grid("""
        ..#..
        ..#..
        #####
        .#.#.
        #...#
    """),
]


def seal(lit):
    S = 96
    cx = cy = 47
    R = 45
    img = blank(S, S)
    base = disk(cx, cy, R + 1)
    for (x, y) in base:
        put(img, x, y, GROUT)  # groove around the stone
    stone = disk(cx, cy, R)
    for (x, y) in stone:
        put(img, x, y, "d")
    inner = disk(cx, cy, 34)
    for (x, y) in inner:
        put(img, x, y, "D")
    # lit rim on the upper-left arc, front face along the bottom arc
    for (x, y) in circle_pts(cx, cy, R):
        ang = math.atan2(y - cy, x - cx)
        if (y - cy) > 0 and abs(x - cx) < R - 3:
            put(img, x, y, "c")
        elif -2.9 < ang < -1.0 or (ang <= -2.9 or ang > 2.9) and y < cy:
            put(img, x, y, "e")
    gold_dim, gold_lit = "H", "J"
    g = gold_lit if lit else gold_dim
    # two inlaid gold rings framing the rune band
    for r in (38, 33):
        for (x, y) in circle_pts(cx, cy, r):
            put(img, x, y, g)
        # groove shadow just outside the ring on the upper-left
        for (x, y) in circle_pts(cx, cy, r + 1):
            if (x - cx) + (y - cy) < -r * 0.5 and get(img, x, y) in ("d", "D"):
                put(img, x, y, "c")
    # rune band: four glyphs at the cardinal points, short radial ticks between them
    for i in range(12):
        a = -math.pi / 2 + i * math.pi * 2 / 12
        if i % 3 == 0:
            gx = round(cx + math.cos(a) * 41.5) - 2
            gy = round(cy + math.sin(a) * 41.5) - 2
            inlay(img, SMALL_RUNES[i // 3], gx, gy, lit, shadow=True)
        else:
            for t in (40, 41, 42, 43):
                put(img, round(cx + math.cos(a) * t), round(cy + math.sin(a) * t), g)
    # centre: a compass-rose star (four long points, four short) around the prism socket
    def line(x0, y0, x1, y1):
        n = max(abs(x1 - x0), abs(y1 - y0))
        for i in range(n + 1):
            put(img, round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n), g)
    for i in range(8):
        a = -math.pi / 2 + i * math.pi / 4
        long_pt = i % 2 == 0
        tip = 29 if long_pt else 17
        hw = 5 if long_pt else 3
        base = 8
        bx, by = cx + math.cos(a) * base, cy + math.sin(a) * base
        px, py = -math.sin(a), math.cos(a)
        tx, ty = round(cx + math.cos(a) * tip), round(cy + math.sin(a) * tip)
        line(round(bx + px * hw), round(by + py * hw), tx, ty)
        line(round(bx - px * hw), round(by - py * hw), tx, ty)
    for (x, y) in circle_pts(cx, cy, 8):
        put(img, x, y, g)
    # the socket where the Prism once stood: a dark octagonal hollow
    sock = disk(cx, cy, 5)
    for (x, y) in sock:
        put(img, x, y, "a")
    for (x, y) in circle_pts(cx, cy, 5):
        if (y - cy) > 1:
            put(img, x, y, "e")  # far wall of the hollow catches the light
        elif (y - cy) < -1 or (x - cx) < -1:
            put(img, x, y, "b")
    if lit:
        for (x, y) in [(cx, cy - 29), (cx, cy + 29), (cx - 29, cy), (cx + 29, cy)]:
            put(img, x, y, "K")
    # a great crack running from the socket out to the rim (the Prism broke here)
    path = [(cx + 4, cy + 3), (cx + 9, cy + 7), (cx + 12, cy + 13), (cx + 17, cy + 17),
            (cx + 19, cy + 24), (cx + 25, cy + 29), (cx + 27, cy + 35), (cx + 31, cy + 37)]
    crack_line(img, path, stone)
    path2 = [(cx - 5, cy - 3), (cx - 10, cy - 5), (cx - 14, cy - 11), (cx - 20, cy - 13),
             (cx - 25, cy - 19)]
    crack_line(img, path2, stone)
    return img


def crack_line(img, pts, region):
    path = []
    for (ax, ay), (bx, by) in zip(pts, pts[1:]):
        n = max(abs(bx - ax), abs(by - ay))
        for i in range(n + 1):
            p = (round(ax + (bx - ax) * i / n), round(ay + (by - ay) * i / n))
            if not path or path[-1] != p:
                path.append(p)
    for i, (x, y) in enumerate(path):
        if (x, y) in region:
            put(img, x, y, "a" if 0 < i < len(path) - 3 else "b")
    for i in range(1, len(path) - 1):
        x, y = path[i]
        if path[i - 1][1] == y and (x, y + 1) in region and get(img, x, y + 1) in ("d", "D") and i % 2:
            put(img, x, y + 1, "E")


# ---------------------------------------------------------------------------
# Natural summit rock (the ground off the paving)
# ---------------------------------------------------------------------------
GROUND = "2"


def ground_tile():
    img = blank(T, T)
    fill(img, GROUND)
    return img


def stamp(img, rows, x, y):
    for yy, r in enumerate(rows):
        for xx, ch in enumerate(r):
            if ch not in (".", " "):
                put(img, x + xx, y + yy, ch)


PEBBLE_S = parse_grid("""
    43
    91
""")
PEBBLE_M = parse_grid("""
    .44.
    4339
    3991
    .11.
""")
PEBBLE_L = parse_grid("""
    ..443..
    .43339.
    4333999
    3999991
    .11111.
""")
TUFT_A = parse_grid("""
    .w....
    .o..w.
    .o.no.
    nno.n.
    .111..
""")
TUFT_S = parse_grid("""
    .w.
    no.
    .1.
""")
# a rock seam: dark core, a little light on its lower lip
SEAM_A = parse_grid("""
    11.......
    .91111...
    ..33.9111
    .......3.
""")
SEAM_B = parse_grid("""
    ...11
    .119.
    193..
    3....
""")
MOSS_DIP = parse_grid("""
    ..nnn...
    .nonnn..
    nooonnn.
    .nnnnm..
""")


def ground_variants():
    v = {}
    v["ground_a"] = ground_tile()
    g = ground_tile()
    stamp(g, PEBBLE_S, 5, 9)
    v["ground_b"] = g
    g = ground_tile()
    stamp(g, SEAM_A, 3, 6)
    v["ground_c"] = g
    g = ground_tile()
    stamp(g, TUFT_A, 5, 5)
    v["ground_d"] = g
    g = ground_tile()
    stamp(g, PEBBLE_M, 8, 4)
    stamp(g, ["3", "0"], 4, 11)
    v["ground_e"] = g
    g = ground_tile()
    stamp(g, MOSS_DIP, 4, 7)
    v["ground_f"] = g
    return v


def ground_big():
    """32x32 rock patches with longer seams and a boulder: break up large ground areas."""
    a = blank(32, 32)
    fill(a, GROUND)
    stamp(a, PEBBLE_L, 17, 6)
    stamp(a, SEAM_B, 13, 13)
    stamp(a, SEAM_A, 3, 20)
    stamp(a, TUFT_S, 26, 23)
    b = blank(32, 32)
    fill(b, GROUND)
    stamp(b, MOSS_DIP, 4, 4)
    stamp(b, TUFT_A, 6, 1)
    stamp(b, PEBBLE_M, 20, 18)
    stamp(b, PEBBLE_S, 12, 24)
    return a, b


# ---------------------------------------------------------------------------
# Flag <-> ground transition overlays (corner wang). Transparent = paving shows.
# ---------------------------------------------------------------------------

def transition_overlay(code, seed=11, variant=0):
    if code == "1111":
        return blank(T, T)
    m = flag_mask(code, seed, variant)

    def M(x, y):
        return m[y + 1, x + 1]

    img = blank(T, T)
    for y in range(T):
        for x in range(T):
            if not M(x, y):
                put(img, x, y, "1" if M(x, y - 1) else GROUND)
    for y in range(T):
        for x in range(T):
            if M(x, y):
                if not M(x, y + 1):
                    put(img, x, y, "c")      # front face of the broken edge
                elif not M(x, y - 1) or not M(x - 1, y):
                    put(img, x, y, "e")      # lit broken edge
    return img


# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

def build():
    rng = random.Random(21)
    for i in range(8):
        add(f"course16_{'abcdefgh'[i]}", course(rng, 16, wear=0.4, crack_n=1 if i in (2, 6) else 0),
            group="course16")
    for i in range(6):
        add(f"course32_{'abcdef'[i]}", course(rng, 32, wear=0.4, crack_n=1 if i in (1, 4) else 0, split=True),
            group="course32")
    newline()
    for i, g in enumerate(RUNES):
        add(f"rune_{i}_dim", rune_inset(g, False), group="rune_dim")
    for i, g in enumerate(RUNES):
        add(f"rune_{i}_lit", rune_inset(g, True), group="rune_lit")
    newline()
    add("seal_dim", seal(False), group="seal")
    add("seal_lit", seal(True), group="seal")
    newline()
    for name, img in ground_variants().items():
        add(name, img, group="ground")
    a, b = ground_big()
    add("ground_big_a", a, group="ground_big")
    add("ground_big_b", b, group="ground_big")
    newline()
    for i in range(1, 15):
        code = format(i, "04b")
        for v in range(3):
            add(f"fg_{code}_{'abc'[v]}", transition_overlay(code, variant=v), group=f"fg_{code}")
    newline()
    faces = cliffs.face_tiles()
    for kind in ("cliff_top", "cliff_mid", "wall_top", "cliff_bot"):
        for suf in ("", "_l", "_r"):
            for v in cliffs.VARIANTS:
                name = f"{kind}_{v}{suf}"
                add(name, faces[name], group=f"{kind}{suf}")
            newline()
    for name, img in cliffs.rim_tiles().items():
        add(name, img, group="rim")
    newline()
    for name, rows in decals.DECALS.items():
        g = "frost" if name.startswith("frost") else "moss"
        add(name, from_rows(rows), group=g)
    newline()


def manifest():
    return {
        "cell": [T, T],
        "anchor": [0, 0],
        "emissive": ["#f0c050", "#fff2b0"],
        "tiles": tiles,
        "groups": groups,
        "blocks": blocks,
        "notes": "Summit ground tiles. See assets/notes/summit.md for layers and how to use them. "
                 "Adds two in-between floor tones (#435078, #56658c) and one rock tone (#3a2d3e) to the warden ramps.",
    }


def cellimg(name):
    c, r = tiles[name]
    return sheet[r * T:(r + 1) * T, c * T:(c + 1) * T]


def blockimg(name):
    if name in tiles and name not in blocks:
        return cellimg(name)
    b = blocks[name]
    (c, r), (w, h) = b["at"], b["size"]
    return sheet[r * T:(r + h) * T, c * T:(c + w) * T]


def paving(Wpx, Hpx, rng):
    """The builder's paving recipe: courses of 16 or 32 px, each filled with random 64 px
    strips, each course shifted by a random multiple of 16 px."""
    out = np.zeros((Hpx, Wpx, 3), np.uint8)
    y = 0
    while y < Hpx:
        h = 32 if rng.random() < 0.35 else 16
        off = rng.randint(0, 3) * 16
        x = -off
        while x < Wpx:
            over(out, blockimg(rng.choice(groups[f"course{h}"])), x, y)
            x += 64
        y += h
    return out


def render_map(rows, seed=3, mids=1, extras=()):
    """Reference compositor (the recipe in assets/notes/summit.md). rows: '#' = rock ground,
    'P' = paving, '.' = void. Returns an RGB image with a plain night sky behind."""
    rng = random.Random(seed)
    H, W = len(rows), len(rows[0])

    def at(x, y):
        return rows[y][x] if 0 <= x < W and 0 <= y < H else "."

    def surf(x, y):
        return at(x, y) != "."

    out = np.zeros((H * T, W * T, 3), np.uint8)
    out[:] = RGB["y"]
    # stars (preview only)
    srng = random.Random(9)
    for _ in range(W * H // 3):
        x, y = srng.randrange(W * T), srng.randrange(H * T)
        out[y, x] = RGB[srng.choice(["t", "u", "v", "w"])]
    # 1. paving under every paved tile
    pave = paving(W * T, H * T, rng)
    for ty in range(H):
        for tx in range(W):
            if at(tx, ty) == "P":
                out[ty * T:(ty + 1) * T, tx * T:(tx + 1) * T] = pave[ty * T:(ty + 1) * T, tx * T:(tx + 1) * T]
    for (name, px, py) in extras:
        if not name.startswith(("frost", "moss")):
            over(out, blockimg(name), px, py)

    # 2/3. ground + transitions: a vertex is paved when every surface tile touching it is P
    def vpaved(vx, vy):
        ts = [at(vx - 1, vy - 1), at(vx, vy - 1), at(vx - 1, vy), at(vx, vy)]
        ss = [t for t in ts if t != "."]
        return bool(ss) and all(t == "P" for t in ss)

    for ty in range(H):
        for tx in range(W):
            if not surf(tx, ty):
                continue
            code = "".join("1" if vpaved(vx, vy) else "0" for vx, vy in
                           [(tx, ty), (tx + 1, ty), (tx, ty + 1), (tx + 1, ty + 1)])
            if code == "0000":
                name = rng.choice(["ground_a"] * 12 + groups["ground"])
                over(out, cellimg(name), tx * T, ty * T)
            elif code != "1111":
                over(out, cellimg(rng.choice(groups[f"fg_{code}"])), tx * T, ty * T)
    for (name, px, py) in extras:
        if name.startswith(("frost", "moss")):
            over(out, blockimg(name), px, py)
    # 4. south faces
    face = {}
    for tx in range(W):
        v = rng.choice(cliffs.VARIANTS)
        for ty in range(H):
            if surf(tx, ty) and not surf(tx, ty + 1):
                kinds = ["wall_top" if at(tx, ty) == "P" else "cliff_top"] + ["cliff_mid"] * mids + ["cliff_bot"]
                y = ty + 1
                for k in kinds:
                    face[(tx, y)] = (k, v, ty)
                    y += 2 if k == "cliff_bot" else 1
    for (tx, y), (k, v, ty) in face.items():
        l = not surf(tx - 1, ty) and (tx - 1, y) not in face
        r = not surf(tx + 1, ty) and (tx + 1, y) not in face
        suf = "_l" if l and not r else ("_r" if r and not l else "")
        over(out, blockimg(f"{k}_{v}{suf}"), tx * T, y * T)
    # 5. rims on void tiles that hold no face
    taken = set(face) | {(tx, y + 1) for (tx, y), (k, _, _) in face.items() if k == "cliff_bot"}
    for ty in range(H):
        for tx in range(W):
            if surf(tx, ty) or (tx, ty) in taken:
                continue

            def mat(x, y):
                return "pave" if at(x, y) == "P" else "rock"
            if surf(tx, ty + 1):
                over(out, cellimg(f"rim_n_{mat(tx, ty + 1)}"), tx * T, ty * T)
            if surf(tx + 1, ty):
                over(out, cellimg(f"rim_w_{mat(tx + 1, ty)}"), tx * T, ty * T)
            if surf(tx - 1, ty):
                over(out, cellimg(f"rim_e_{mat(tx - 1, ty)}"), tx * T, ty * T)
            if surf(tx + 1, ty + 1) and not surf(tx + 1, ty) and not surf(tx, ty + 1):
                over(out, cellimg(f"rim_nw_{mat(tx + 1, ty + 1)}"), tx * T, ty * T)
            if surf(tx - 1, ty + 1) and not surf(tx - 1, ty) and not surf(tx, ty + 1):
                over(out, cellimg(f"rim_ne_{mat(tx - 1, ty + 1)}"), tx * T, ty * T)
    return out


MOCK = [
    "........................",
    "....##########..........",
    "..##############........",
    ".###PPPPPPPPPPP####.....",
    ".##PPPPPPPPPPPPP#####...",
    ".##PPPPPPPPPPPPP######..",
    ".##PPPPPPPPPPPPP######..",
    ".##PPPPPPPPPPPPP####....",
    "..#PPPPPPPPPPPPP###.....",
    "...PPPPPPPPPPPPP##......",
    "......####PPP##.........",
    ".......###PPP#..........",
    "..........PPP...........",
    "........................",
    "........................",
    "........................",
]


def mock_floor():
    seal_x, seal_y = 9 * T - 48, 6 * T + 8 - 48
    return render_map(MOCK, extras=[("seal_lit", seal_x, seal_y), ("rune_1_dim", 4 * T, 4 * T),
                                    ("rune_4_dim", 14 * T, 8 * T),
                                    ("frost_c", 17 * T, 3 * T), ("frost_a", 2 * T + 4, 6 * T),
                                    ("frost_d", 12 * T, 2 * T), ("moss_a", 5 * T, 1 * T + 6),
                                    ("moss_c", 16 * T, 6 * T), ("moss_b", 7 * T, 10 * T),
                                    ("moss_joint", 12 * T, 4 * T - 8 + 3), ("frost_b", 19 * T, 5 * T)])


if __name__ == "__main__":
    build()
    save_sheet("summit_tiles", sheet, manifest())
    rgb = flatten(sheet, "#5a1a5a")
    save_rgb(PREVIEWS / "summit_tiles_sheet.png", rgb, 4)
    m = mock_floor()
    save_rgb(PREVIEWS / "summit_tiles_mock_1x.png", m, 1)
    save_rgb(PREVIEWS / "summit_tiles_mock_3x.png", m, 3)
    print("colors:", len(colors_used(sheet)))
    print("tiles:", len(tiles))
