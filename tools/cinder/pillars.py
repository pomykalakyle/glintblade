"""cinder_pillars: obsidian pillars (intact, rune, broken tall, broken short). Cell 32x48.

Obsidian is volcanic glass: calm 3-value body (lit / body / shadow), a sharp pale specular line broken
into uneven segments per drum, and a thin warm rim on the lower right where lava light bounces.
"""

from draw import Canvas, line
from lib import SheetBuilder

W, H = 32, 48
GROUND = 45          # first row below the plinth
ANCHOR = [16, 44]    # base center, on the plinth's bottom row
SX = 11              # shaft left x (shaft is 10 px wide: SX .. SX+9)
SW = 10


def drum(c, x0, y0, h, glints, rim=True):
    """One stacked drum: lit 2 px, body, shadow 2 px; joint row at the bottom.
    glints: list of (start_row, length) for the specular segments on the lit band."""
    for j in range(h):
        y = y0 + j
        last = j == h - 1
        for i in range(SW):
            if i < 2:
                ch = "r"
            elif i >= SW - 2:
                ch = "p"
            else:
                ch = "q"
            if last:
                ch = "p" if i < SW - 2 else "o"   # joint: a dark seam under each drum
            c.set(x0 + i, y, ch)
    for s, n in glints:
        for k in range(n):
            if 0 <= s + k < h - 1:
                c.set(x0 + 1, y0 + s + k, "t")
    if rim:
        # warm bounce light from the lava on the lower right edge of the drum
        for k in range(max(1, h // 2 - 1)):
            yy = y0 + h - 2 - k
            if yy > y0:
                c.set(x0 + SW - 1, yy, "B")


def ring(c, x0, y, w):
    """Copper ring, 2 rows, 1 px proud of the shaft each side. 3 values."""
    for i in range(w + 2):
        x = x0 - 1 + i
        a = "E" if i in (1, 2) else ("D" if i < w - 1 else "C")
        b = "C" if i < w - 1 else "B"
        c.set(x, y, a)
        c.set(x, y + 1, b)


def block(c, x0, x1, y_top, top_rows, front_rows):
    """3/4 view block. Top plane: back rows inset 1 px per row (chamfered corners, so it recedes), lit fill
    falling off to the right, a short glint only at the front-left corner. Front face: lit left bevel,
    body, shadow right, darker underside row."""
    w = x1 - x0 + 1
    for j in range(top_rows):
        inset = top_rows - 1 - j
        for x in range(x0 + inset, x1 - inset + 1):
            i = x - x0
            ch = "s"
            if j == 0 and top_rows > 1:
                ch = "r"                       # far edge recedes
            if i >= w - 4:
                ch = "r"                       # light falls off toward the right
            c.set(x, y_top + j, ch)
    if top_rows:
        c.set(x0, y_top + top_rows - 1, "t")
        c.set(x0 + 1, y_top + top_rows - 1, "t")
    for j in range(front_rows):
        for x in range(x0, x1 + 1):
            i = x - x0
            ch = "r" if i == 0 else ("p" if i >= w - 2 else "q")
            if j == front_rows - 1 and front_rows > 1 and i > 0:
                ch = "p"                       # underside of the lip
            c.set(x, y_top + top_rows + j, ch)


def plinth(c):
    block(c, 7, 24, GROUND - 4, 2, 2)
    block(c, 9, 22, GROUND - 8, 2, 2)
    # contact shadow of the shaft on the plinth's top plane
    for x in range(SX, SX + SW):
        c.set(x, GROUND - 8, "q" if x < SX + SW - 2 else "p")
        c.set(x, GROUND - 7, "r" if x < SX + SW - 2 else "q")


def capital(c, y):
    block(c, 8, 23, y, 3, 3)
    block(c, 10, 21, y + 6, 0, 2)
    ring(c, SX, y + 8, SW)
    # chip the capital's front-left corner: the fracture faces up-left, so it is lit
    c.set(8, y + 3, ".")
    c.set(8, y + 4, ".")
    c.set(9, y + 3, "t")
    c.set(9, y + 4, "s")


DRUMS = [  # (height, glint segments)
    (8, [(0, 3), (5, 1)]),
    (7, [(1, 2)]),
    (8, [(0, 1), (2, 3)]),
    (7, [(0, 2), (4, 1)]),
    (8, [(1, 4)]),
]


def shaft(c, top, bottom, drums=DRUMS, offsets=(0, 1, 0, 0, -1)):
    y = bottom
    i = 0
    placed = []
    while y >= top:
        h, g = drums[i % len(drums)]
        y0 = max(top, y - h + 1)
        dx = offsets[i % len(offsets)]
        drum(c, SX + dx, y0, y - y0 + 1, [(s - (h - (y - y0 + 1)), n) for s, n in g])
        placed.append((y0, y - y0 + 1, dx))
        y = y0 - 1
        i += 1
    return placed


def pillar_intact():
    c = Canvas(W, H)
    plinth(c)
    shaft(c, 13, GROUND - 9)
    capital(c, 2)
    # one drum has a chipped left edge (lit fracture), another a hairline crack with a lit lip
    c.set(SX, 25, ".")
    c.set(SX, 26, ".")
    c.set(SX + 1, 25, "t")
    c.set(SX + 1, 26, "s")
    for x, y in [(SX + 4, 34), (SX + 5, 35), (SX + 6, 35), (SX + 7, 36)]:
        c.set(x, y, "o")
    for x, y in [(SX + 5, 36), (SX + 6, 36)]:
        c.set(x, y, "r")
    c.outline("K")
    return c


RUNE = [  # carved vertical rune; '#' groove glow, 'h' hot spot
    "#...#",
    ".#.#.",
    "..#..",
    "..#..",
    "..h..",
    "..#..",
    "..#..",
    ".#.#.",
    "#...#",
]


def pillar_rune():
    c = Canvas(W, H)
    plinth(c)
    # a taller drum carries the rune
    drums = [(8, [(0, 2), (5, 1)]), (13, [(0, 1)]), (8, [(1, 3)]), (7, [(0, 2)])]
    shaft(c, 13, GROUND - 9, drums=drums, offsets=(0, 0, 1, 0))
    capital(c, 2)
    rx, ry = SX + 3, 22
    glow = []
    for j, r in enumerate(RUNE):
        for i, ch in enumerate(r):
            if ch in "#h":
                glow.append((rx + i, ry + j, "z" if ch == "h" else ("y" if j in (3, 4, 5) else "x")))
    g = {(x, y) for x, y, _ in glow}
    # recessed groove walls: dark on the upper-left side (in shadow), warm spill on the lower right
    for x, y, _ in glow:
        for dx, dy in ((-1, 0), (0, -1)):
            if (x + dx, y + dy) not in g:
                c.set(x + dx, y + dy, "o")
    for x, y, _ in glow:
        for dx, dy in ((1, 0), (0, 1)):
            if (x + dx, y + dy) not in g and c.get(x + dx, y + dy) in "qp":
                c.set(x + dx, y + dy, "B")
    for x, y, ch in glow:
        c.set(x, y, ch)
    c.outline("K")
    return c


def rubble(c, x, y, kind):
    shapes = {
        "chunk": """
            .ts.
            srqp
            rqqp
        """,
        "chip": """
            ts.
            rqp
        """,
        "slab": """
            .tssr..
            srrrqqp
            rqqqqpp
        """,
        # a fallen drum lying on its side: lit end disc on the left, glassy body to the right
        "drum": """
            .ttsKrrrrqq.
            tsrrKrqqqqqp
            srrqKrqqqqqp
            rrqqKqqqqqpp
            .qqpKBBBBBB.
        """,
    }
    c.stamp(shapes[kind], x, y)


def broken(height, profile, drums, offsets):
    """Shaft of `height` px over the plinth, its top cut down by `profile` (px removed per shaft column).
    The exposed break is a 3 px deep fracture plane: lit, rough, darkening to the right, chipped at the edges."""
    c = Canvas(W, H)
    plinth(c)
    top = GROUND - 9 - height + 1
    placed = shaft(c, top, GROUND - 9, drums=drums, offsets=offsets)
    dx = placed[-1][2]
    for i, n in enumerate(profile):
        x = SX + dx + i
        for k in range(n):
            c.set(x, top + k, ".")
        for k in range(3):
            y = top + n + k
            if c.get(x, y) == ".":
                continue
            if k == 0:
                ch = "s" if i < 6 else "r"
            elif k == 1:
                ch = "s" if i < 4 else "r"
            else:
                ch = "r" if i < 7 else "q"     # the plane meets the shaft's front face
            c.set(x, y, ch)
        if i in (1, 2) and c.get(x, top + n) != ".":
            c.set(x, top + n, "t")             # one short glint where the plane is highest
    # a dark crack running into the fracture plane
    mid = SX + dx + 4
    c.set(mid, top + profile[4] + 1, "p")
    c.set(mid + 1, top + profile[5] + 2, "p")
    return c, top


def broken_tall():
    c, top = broken(24, [0, 0, 1, 1, 2, 3, 3, 4, 5, 6], DRUMS[:3] + [(8, [(0, 2)])], (0, 1, 0, 0))
    rubble(c, 1, GROUND - 3, "chunk")
    rubble(c, 26, GROUND - 2, "chip")
    rubble(c, 3, GROUND - 6, "chip")
    c.outline("K")
    return c


def broken_short():
    c, top = broken(11, [1, 0, 0, 1, 1, 2, 2, 3, 3, 4], [(5, [(1, 2)]), (6, [(0, 3)])], (0, 1))
    rubble(c, 20, GROUND - 3, "drum")
    rubble(c, 1, GROUND - 3, "slab")
    c.outline("K")
    return c


def build():
    sb = SheetBuilder("cinder_pillars", (W, H), ANCHOR)
    fp = [14, 7]
    sb.add("pillar", [pillar_intact().rgba()], footprint=fp)
    sb.add("pillar_rune", [pillar_rune().rgba()], footprint=fp,
           light={"color": "#ff7a2a", "radius": 26, "intensity": 0.6, "offset": [0, -18]})
    sb.add("pillar_broken_tall", [broken_tall().rgba()], footprint=fp)
    sb.add("pillar_broken_short", [broken_short().rgba()], footprint=fp)
    return sb


if __name__ == "__main__":
    from view import show
    imgs = [pillar_intact().rgba(), pillar_rune().rgba(), broken_tall().rgba(), broken_short().rgba()]
    print(show(imgs, "pillars", s=4))
