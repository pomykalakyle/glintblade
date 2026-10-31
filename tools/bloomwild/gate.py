"""gate: the exit gate shared by every region (closed, opening, open).

Pale ruin stone. A round arch (mirrored exactly, clean runs) with a big keystone holding a
shard-shaped socket: the gate's focal point. Two carved facet glyphs on the pillars echo the
shard. No regional vegetation (the gate is shared).

closed: a dormant crystal seal (deep violet) with an engraved shard-shaped division; the arch and
left pillar cast shadow onto its top and left. opening: socket lights, glyphs light, the engraved
lines crack open, the pane splits into pieces, flash. open: a portal with a bright narrow rim,
dark depth in the middle, light pooling on the threshold (its stepped top rises and falls) and a
few motes rising 2 px per frame. Light from the upper left.
"""
from pix import *

CW, CH = 48, 64
AX, AY = 24, 61
OUT = "s0"

# outer arch silhouette: half-widths per row from the top (center between x 23 and 24)
OUTER_TOP = 10
OUTER_HW = [5, 9, 12, 14, 16, 17, 18, 19, 20, 20, 20, 20]       # rows 10..21
INNER_TOP = 17
INNER_HW = [3, 6, 8, 9]                                          # rows 17..20, then 10
OPEN_BOT = 53


def hw_inner(y):
    """Hw inner on the native pixel grid."""
    if y < INNER_TOP or y > OPEN_BOT:
        return None
    i = y - INNER_TOP
    return INNER_HW[i] if i < len(INNER_HW) else 10


def in_opening(x, y):
    """In opening on the native pixel grid."""
    h = hw_inner(y)
    return h is not None and 24 - h <= x <= 23 + h


def in_outer(x, y):
    """In outer on the native pixel grid."""
    if OUTER_TOP <= y < OUTER_TOP + len(OUTER_HW):
        h = OUTER_HW[y - OUTER_TOP]
        return 24 - h <= x <= 23 + h
    return False


def opening_depth(x, y):
    """Pixels from the opening's edge (1 = rim)."""
    d = 0
    while in_opening(x - d, y) and in_opening(x + d, y) and in_opening(x, y - d) and d < 12:
        d += 1
    return d


KEY_X0, KEY_X1, KEY_Y0, KEY_Y1 = 19, 28, 6, 18
SOCKET = ["...##...",
          "..####..",
          "..#####.",
          ".######.",
          ".#####..",
          "..####..",
          "..###...",
          "...#...."]
SOCK_X, SOCK_Y = 20, 8
GLYPH = [".g.",
         "gg.",
         "g.g",
         "g.g",
         ".gg",
         ".g."]
GLYPH_POS = [(8, 31), (37, 31)]


def socket_px():
    """Socket px on the native pixel grid."""
    return [(SOCK_X + i, SOCK_Y + j) for j, r in enumerate(SOCKET) for i, ch in enumerate(r) if ch == "#"]


def stone_body():
    """Stone body on the native pixel grid."""
    img = new(CW, CH)
    # plinth
    for y in range(54, 62):
        for x in range(3, 45):
            if y <= 57:
                c = "s5" if y == 54 else "s4"
                if x == 44:
                    c = "s3"
            else:
                c = "s3" if y == 58 else "s1" if y == 61 else "s2"
            put(img, x, y, c)
    for x in (12, 24, 35):
        for y in range(59, 61):
            put(img, x, y, "s1")
    # pillars
    for (a, b) in [(5, 13), (34, 42)]:
        for y in range(25, 54):
            for x in range(a, b + 1):
                rel = x - a
                c = "s5" if rel == 0 else "s4" if rel <= 4 else "s3" if rel <= 6 else "s2"
                if rel == b - a:
                    c = "s1"
                put(img, x, y, c)
        for y in (50, 51, 52, 53):
            for x in range(a - 1, b + 2):
                rel = x - (a - 1)
                c = "s5" if y == 50 and rel <= 7 else "s4" if rel <= 5 else "s3" if rel <= 8 else "s2"
                if rel == b - a + 2:
                    c = "s1"
                put(img, x, y, c)
        for x in range(a, b + 1):
            put(img, x, 41, "s2" if x < b else "s1")
    for y in range(25, 54):          # inner faces: left pillar's faces right (shade), right's faces left (lit)
        put(img, 13, y, "s1"); put(img, 12, y, "s2")
        put(img, 34, y, "s4"); put(img, 35, y, "s3")
    # capitals
    for (a, b) in [(5, 13), (34, 42)]:
        for y in (22, 23, 24):
            for x in range(a - 1, b + 2):
                rel = x - (a - 1)
                c = "s5" if y == 22 else ("s3" if rel <= 7 else "s2")
                if rel == b - a + 2:
                    c = "s1"
                if y == 24:
                    c = DARK.get(c, c)
                put(img, x, y, c)
    # arch ring: rows 10..21, between the outer silhouette and the opening (left half, mirrored)
    for y in range(OUTER_TOP, 22):
        for x in range(0, 24):
            if not in_outer(x, y) or in_opening(x, y):
                continue
            top = not in_outer(x, y - 1)
            top2 = not in_outer(x, y - 2)
            soffit = in_opening(x, y + 1) or in_opening(x + 1, y) or in_opening(x, y + 2)
            c = "s5" if top else "s4" if top2 else "s4"
            if soffit:
                c = "s2"
            put(img, x, y, c)
            # mirror with the shade side one step darker
            mc = {"s5": "s4", "s4": "s3", "s2": "s2"}[c]
            put(img, 47 - x, y, mc)
    # voussoir joints (left side, mirrored)
    joints = [[(9, 14), (10, 15), (11, 16)], [(5, 19), (6, 19), (7, 19), (8, 19), (9, 19)], [(14, 11), (15, 12), (15, 13), (16, 14)]]
    for line in joints:
        for (x, y) in line:
            if in_outer(x, y) and not in_opening(x, y):
                put(img, x, y, "s2")
                put(img, 47 - x, y, "s2")
    # keystone (proud of the arch): top face, lit left edge, shaded right, its own soffit
    for y in range(KEY_Y0, KEY_Y1 + 1):
        for x in range(KEY_X0, KEY_X1 + 1):
            rel = x - KEY_X0
            c = "s5" if (y == KEY_Y0 or rel == 0) else "s4" if rel < 7 else "s3" if rel < 9 else "s2"
            if y == KEY_Y1:
                c = "s2"
            put(img, x, y, c)
    for x in range(KEY_X0 + 1, KEY_X1 + 1):
        put(img, x, KEY_Y0 + 1, "s4" if x < KEY_X1 - 1 else "s3")
    # carved facet glyphs (dormant: just cut into the stone)
    for (gx, gy) in GLYPH_POS:
        grid(GLYPH, {"g": "s2"}, img, gx, gy)
    return img


DARK = {"s5": "s4", "s4": "s3", "s3": "s2", "s2": "s1", "s1": "s1"}


def socket(img, state):
    """state 0: empty (recess in shadow); 1: the shard sits in it, lit."""
    px = socket_px()
    pset = set(px)
    for (x, y) in px:
        if state == 0:
            c = "s0" if ((x, y - 1) not in pset or (x - 1, y) not in pset) else "s1"
        else:
            c = "p2" if x < 23 else "p1"
            if (x + 1, y) not in pset:
                c = "p0"
        put(img, x, y, c)
    if state:
        put(img, 23, 9, "p5"); put(img, 22, 10, "p5")
    return img


def glyphs_lit(img, which):
    """Glyphs lit on the native pixel grid."""
    for k in which:
        gx, gy = GLYPH_POS[k]
        grid(GLYPH, {"g": "p2"}, img, gx, gy)
        put(img, gx + 1, gy, "p5")
    return img


# engraved division in the seal: a big shard outline + the center ridge
SEAL_LINES = [
    [(23, 19), (19, 26), (16, 35), (18, 44), (22, 52)],
    [(24, 19), (28, 26), (31, 35), (29, 44), (25, 52)],
    [(23, 21), (23, 51)],
]
SEAL_FORKS = [[(19, 26), (15, 24)], [(28, 26), (32, 23)], [(16, 35), (14, 38)], [(31, 35), (33, 32)],
              [(23, 33), (20, 30)], [(23, 40), (27, 43)]]


def line_px(x0, y0, x1, y1):
    """Line px on the native pixel grid."""
    pts = []
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        pts.append((x0, y0))
        if x0 == x1 and y0 == y1:
            return pts
        e2 = 2 * err
        if e2 >= dy:
            err += dy; x0 += sx
        if e2 <= dx:
            err += dx; y0 += sy


def poly_px(poly):
    """Poly px on the native pixel grid."""
    out = []
    for a, b in zip(poly, poly[1:]):
        out += line_px(*a, *b)
    return out


def seal(img):
    """Dormant seal: a deep violet pane; the arch and the left pillar cast shadow onto its top
    and left edge; an engraved (non-emissive) shard outline and ridge."""
    for y in range(CH):
        for x in range(CW):
            if in_opening(x, y):
                d_top = y - (INNER_TOP + 0)
                shadow = (not in_opening(x, y - 2)) or (not in_opening(x - 2, y)) or (not in_opening(x - 1, y - 3))
                put(img, x, y, "s0" if shadow else "p0")
    for poly in SEAL_LINES:
        for (x, y) in poly_px(poly):
            if in_opening(x, y) and is_col(img, x, y, "p0"):
                put(img, x, y, "p1")
    return img


def cracks(img, amount, forks=False):
    """Light the engraved lines from the top down; amount 0..1 of their length."""
    for poly in SEAL_LINES:
        pts = poly_px(poly)
        n = int(len(pts) * amount)
        for (x, y) in pts[:n]:
            if in_opening(x, y):
                put(img, x, y, "p2")
    if forks:
        for f in SEAL_FORKS:
            for (x, y) in poly_px(f):
                if in_opening(x, y):
                    put(img, x, y, "p2")
    return img


def split(img, gap):
    """The pane splits along the engraved lines: gaps of light open between the pieces, the
    pieces' edges catch the light."""
    lines = set()
    for poly in SEAL_LINES:
        lines |= set(poly_px(poly))
    for f in SEAL_FORKS:
        lines |= set(poly_px(f))
    for (x, y) in lines:
        for dx in range(-gap + 1, gap):
            if in_opening(x + dx, y):
                put(img, x + dx, y, "p5" if abs(dx) < gap - 1 or gap == 1 else "p2")
    return img


BAYER2 = [[0, 2], [3, 1]]
POOL_H = [6, 7, 7, 8, 8, 7, 7, 6]        # height of the light pool's flat top, per frame
MOTES = [(17, 0), (21, 3), (26, 5), (30, 2)]   # (column, phase)


def portal(img, t):
    """Portal on the native pixel grid."""
    for y in range(CH):
        for x in range(CW):
            if not in_opening(x, y):
                continue
            d = opening_depth(x, y)
            v = OPEN_BOT - y
            if d <= 1:
                c = "p5"
            elif d == 2:
                c = "p2"
            elif d <= 4:
                c = "p1"
            else:
                c = "p0"
            ph = POOL_H[t] + (1 if 19 <= x <= 28 else 0)      # a stepped top, higher in the middle
            if v < ph and d > 1:
                c = "p2"
            elif v == ph and d > 2:
                c = "p2" if BAYER2[y % 2][x % 2] < 2 else "p1"   # one-row ordered edge
            put(img, x, y, c)
    # motes: rise 2 px per frame from the pool, cyan fading to violet, then reset
    for (mx, phase) in MOTES:
        k = (t + phase) % 8
        y = OPEN_BOT - POOL_H[t] - 2 - 2 * k
        if not in_opening(mx, y) or opening_depth(mx, y) < 3:
            continue
        if k < 5:
            put(img, mx, y, "p5" if k < 2 else "p2")
            put(img, mx, y + 1, "p2" if k < 2 else "p1")
        elif k < 7:
            put(img, mx, y, "p2" if k == 5 else "p1")
    return img


def outline_gate(img):
    """Outline gate on the native pixel grid."""
    return outline(img, OUT)


def frame(state, t=0, step=0):
    """Frame on the native pixel grid."""
    img = stone_body()
    if state == "closed":
        seal(img)
        socket(img, 0)
    elif state == "opening":
        seal(img)
        socket(img, 1)
        if step == 1:
            glyphs_lit(img, [0, 1])
            cracks(img, 0.35)
        elif step == 2:
            glyphs_lit(img, [0, 1])
            cracks(img, 1.0, forks=True)
        elif step == 3:
            glyphs_lit(img, [0, 1])
            split(img, 1)
        elif step == 4:
            glyphs_lit(img, [0, 1])
            split(img, 2)
        elif step == 5:
            glyphs_lit(img, [0, 1])
            for y in range(CH):
                for x in range(CW):
                    if in_opening(x, y):
                        put(img, x, y, "p5" if opening_depth(x, y) > 1 else "p2")
    else:
        portal(img, t)
        socket(img, 1)
        glyphs_lit(img, [0, 1])
    return outline_gate(img)


def build():
    """Build on the native pixel grid."""
    closed = [frame("closed")]
    opening = [frame("opening", step=i) for i in range(6)]
    opn = [frame("open", t=i) for i in range(8)]
    return closed, opening, opn


EMISSIVE = ["p2", "p3", "p4", "p5"]
OPEN_DUR = [8] * 8
OPENING_DUR = [14, 8, 8, 6, 5, 6]


def write():
    """Write on the native pixel grid."""
    closed, opening, opn = build()
    rows = [closed + [new(CW, CH)] * 7, opening + [new(CW, CH)] * 2, opn]
    man = {
        "cell": [CW, CH],
        "anchor": [AX, AY],
        "emissive": [hexof(c) for c in EMISSIVE],
        "footprint": [42, 8],
        "anims": {
            "closed": {"row": 0, "frames": 1, "fps": 1},
            "opening": {"row": 1, "frames": 6, "fps": 10, "durations": OPENING_DUR},
            "open": {"row": 2, "frames": 8, "fps": 8, "durations": OPEN_DUR,
                     "light": {"color": "#5ff2ff", "radius": 56, "intensity": 0.7, "offset": [0, -16]}},
        },
        "notes": "Exit gate shared by every region (no regional vegetation). closed: dormant seal with an engraved "
                 "shard outline; the keystone socket is empty. opening: play once after the shard pickup: the "
                 "socket lights (hold), the glyphs light and the engraved lines crack open, the pane splits, flash. "
                 "open: looping portal. Walk-in zone: the opening, x 14..33 of the cell = anchor x -10..+9, up to "
                 "8 px in front of the anchor. footprint covers the plinth; pillars should block too (two 10x6 "
                 "boxes at anchor x -19..-10 and +10..+19) if the engine supports it.",
    }
    sheet = save_sheet("gate", rows, CW, CH, man)
    preview(sheet, PREVIEWS / "gate_8x.png", k=4, bg="g2", gridcell=(CW, CH))
    gif(opn, PREVIEWS / "gate_open.gif", k=5, bg="g2", durations=OPEN_DUR)
    gif(closed * 1 + opening + opn * 3, PREVIEWS / "gate_sequence.gif", k=5, bg="g2",
        durations=[40] + OPENING_DUR + OPEN_DUR * 3)
    return sheet


if __name__ == "__main__":
    write()
