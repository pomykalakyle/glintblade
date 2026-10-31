"""cinder_ruins: broken obsidian masonry. Cell 64x48, anchor at the base center (32, 45).

Same obsidian rules as the pillars: calm block faces (lit bevel top/left, body, shadow right, dark joints),
a few short pale glints on chosen blocks only, warm lava bounce on the lower right, lit fracture tops.
Walls break down in whole blocks (stepped courses), so every break reads as masonry that fell.
"""

import math

from draw import Canvas
from lib import SheetBuilder

W, H = 64, 48
GROUND = 46
ANCHOR = [32, 45]
CH = 6        # course height
TD = 3        # top-face depth in rows (3/4 view)


def block_px(bx, row, bw):
    if row == CH - 1 or bx == bw - 1:
        return "o"
    if row == 0 or bx == 0:
        return "r"
    if bx == bw - 2:
        return "p"
    return "q"


def wall(c, courses, glints=(), bw=9):
    """courses: bottom-up list of (x0, x1). Draws front faces and every exposed top face."""
    n = len(courses)
    for i, (x0, x1) in enumerate(courses):
        y0 = GROUND - (i + 1) * CH
        off = 4 if i % 2 else 0
        for y in range(y0, y0 + CH):
            for x in range(x0, x1 + 1):
                bx = (x - x0 + off) % bw
                ch = block_px(bx, y - y0, bw)
                if x == x1 and ch != "o":
                    ch = "p"
                c.set(x, y, ch)
        # warm bounce from the lava along the wall's lower right
        if i == 0:
            for y in range(y0 + 1, y0 + CH - 1):
                c.set(x1, y, "B")
        # exposed top face: where the course above doesn't cover
        above = courses[i + 1] if i + 1 < n else None
        for x in range(x0, x1 + 1):
            if above and above[0] <= x <= above[1]:
                continue
            for k in range(TD):
                y = y0 - TD + k
                ch = "s" if k == TD - 1 else "r"
                if x == x0 and k == TD - 1:
                    ch = "t"
                c.set(x, y, ch)
    for (cx, i) in glints:
        y0 = GROUND - (i + 1) * CH
        c.set(cx, y0 + 1, "t")
        c.set(cx, y0 + 2, "t")
        c.set(cx + 1, y0 + 1, "t")


def chunk(c, x, y, kind):
    shapes = {
        "block": """
            .rrrrr.
            sssssr.
            rqqqqpo
            rqqqqpo
            .oooo..
        """,
        "chip": """
            .ts.
            srqp
            .qp.
        """,
        "small": """
            tsr
            rqp
        """,
    }
    c.stamp(shapes[kind], x, y)


def wall_long():
    """A long wall run, intact on the left, broken down in steps to the right."""
    c = Canvas(W, H)
    courses = [(4, 57), (4, 50), (4, 41), (4, 27), (6, 18)]
    wall(c, courses, glints=[(6, 0), (14, 2), (24, 1), (8, 3)])
    chunk(c, 51, 40, "block")
    chunk(c, 59, 42, "small")
    chunk(c, 33, 23, "chip")
    c.outline("K")
    return c


def wall_corner():
    """A low wall stub with a broken-off end on both sides and fallen blocks in front."""
    c = Canvas(W, H)
    courses = [(10, 53), (14, 49), (18, 38)]
    wall(c, courses, glints=[(12, 0), (20, 2), (30, 1)])
    chunk(c, 2, 40, "block")
    chunk(c, 54, 42, "chip")
    chunk(c, 42, 30, "small")
    c.outline("K")
    return c


def arch():
    """Two piers and a half-fallen arch: the right side of the arch and the right pier's top have collapsed."""
    c = Canvas(W, H)
    wall(c, [(9, 19), (9, 19), (9, 19), (9, 19)], glints=[(11, 1), (11, 3)], bw=11)
    wall(c, [(44, 54), (44, 54), (44, 54), (44, 54)], glints=[(46, 0), (46, 2)], bw=11)
    cx, cy = 31.5, 22.0
    rxi, ryi, rxo, ryo = 12.5, 9.0, 22.5, 16.0
    # front face of the arch ring, kept from the left springer over the top to ~65 degrees
    ring = {}
    for y in range(0, 23):
        for x in range(W):
            px, py = x + 0.5 - cx, cy - (y + 0.5)
            qi = (px / rxi) ** 2 + (py / ryi) ** 2
            qo = (px / rxo) ** 2 + (py / ryo) ** 2
            if qi >= 1.0 and qo < 1.0 and py > -0.5:
                ang = math.degrees(math.atan2(py / ryo, px / rxo))
                # jagged break: the cut angle wobbles with the radius
                t = (math.sqrt(qo) - 0.6) / 0.4
                cut = 84 + 7 * math.sin(t * 9.0)
                stub = 26 + 6 * math.sin(t * 8.0 + 1.0)
                if ang >= cut or ang <= stub:
                    ring[(x, y)] = ang
    for (x, y), ang in ring.items():
        px, py = x + 0.5 - cx, cy - (y + 0.5)
        qo = (px / rxo) ** 2 + (py / ryo) ** 2
        k = int((180 - ang) // 15)            # voussoir index
        edge = abs(((180 - ang) % 15)) < 1.6
        if edge:
            ch = "o"
        elif qo > 0.86:
            ch = "r"                           # outer bevel catches light
        elif ((x, y + 1) not in ring) and y < 22:
            ch = "p"                           # intrados edge, in shadow
        else:
            ch = "q"
        c.set(x, y, ch)
    # top face (extrados): visible 2 rows above the ring's outer edge, lit
    tops = {}
    for (x, y) in ring:
        if (x, y - 1) not in ring:
            tops[x] = min(tops.get(x, 99), y)
    for x, y in tops.items():
        for k in (1, 2):
            if c.get(x, y - k) == ".":
                c.set(x, y - k, "s" if k == 1 else "r")
    # fracture face at the break: lit chips on the top of the broken end
    ends = [kv for kv in ring.items() if 70 < kv[1] < 100]
    ends = sorted(ends, key=lambda kv: kv[1])[:6]
    stub_ends = sorted([kv for kv in ring.items() if kv[1] < 45], key=lambda kv: -kv[1])[:5]
    for (x, y), _ in ends + stub_ends:
        if c.get(x, y - 1) in ".sr":
            c.set(x, y, "s")
    # a few glints on the left half of the arch
    for x, y in [(14, 14), (15, 13), (20, 8), (21, 8), (27, 6)]:
        if c.get(x, y) in "qr":
            c.set(x, y, "t")
    # fallen voussoirs under the broken side
    chunk(c, 52, 40, "block")
    chunk(c, 38, 42, "chip")
    chunk(c, 40, 43, "small")
    c.outline("K")
    return c


def build():
    sb = SheetBuilder("cinder_ruins", (W, H), ANCHOR)
    sb.add("wall_long", [wall_long().rgba()], footprint=[54, 8])
    sb.add("wall_low", [wall_corner().rgba()], footprint=[44, 8])
    sb.add("arch_broken", [arch().rgba()], footprint=[46, 8],
           notes="Two solid piers (x 9-19 and 44-54 in the cell); the gap under the arch is walkable.")
    return sb


if __name__ == "__main__":
    from view import show
    print(show([wall_long().rgba(), wall_corner().rgba(), arch().rgba()], "ruins", s=4))
