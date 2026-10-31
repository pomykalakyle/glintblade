"""Facet plans for the four Glasswater landmark crystals (48x64 cells, base anchor 24,61).

Written 2026-10-31. Each landmark is a few leaning prisms drawn back to front on the native
grid: one large unlit back mass, one dominant crystal whose broad source-facing face is broken
by a fracture, small emissive caps, and a slate root that overlaps every crystal base.
Light comes from the upper left. Colors come only from the Glasswater rock and crystal_violet
ramps, the same ones the procedural small crystals use.
"""
from PIL import Image, ImageDraw


def palette(region):
    """Return the named colors a plan may use, from the Glasswater region palette."""
    rock = region["ramps"]["rock"]
    iris = region["ramps"]["crystal_violet"]
    return {
        "r0": rock[0], "r1": rock[1], "r2": rock[2], "r3": rock[3],
        "i0": iris[0], "i1": iris[1], "i2": iris[2], "i3": iris[3],
    }


class Prism:
    """One leaning crystal prism whose rows are rasterized so every pixel has exactly one face."""

    def __init__(self, x, by, lean, h, widths, faces, tip, apex=0.5):
        """Place the bottom-left corner at (x, by); lean is (dx, rows) of shift per height."""
        self.x, self.by, self.lean, self.h = x, by, lean, h
        self.widths, self.faces, self.tip, self.apex = widths, faces, tip, apex

    def left(self, r):
        """Return the left edge x of body row r (0 = base row)."""
        dx, rows = self.lean
        return self.x + (dx * r) // rows

    def edges(self, r):
        """Return the face boundary x positions of body row r."""
        out = [self.left(r)]
        for w in self.widths:
            out.append(out[-1] + w)
        return out

    def apex_xy(self):
        """Return the tip pixel of the pointed termination."""
        e = self.edges(self.h - 1)
        dx, rows = self.lean
        return e[0] + round(self.apex * (e[-1] - 1 - e[0])) + (dx * self.tip) // rows, self.by - self.h - self.tip + 1

    def draw(self, px):
        """Set the body rows, then a tip whose face boundaries converge on the apex."""
        for r in range(self.h):
            e = self.edges(r)
            for i, color in enumerate(self.faces):
                for x in range(e[i], e[i + 1]):
                    px[(x, self.by - r)] = color
        e = self.edges(self.h - 1)
        ax, ay = self.apex_xy()
        for k in range(1, self.tip + 1):
            b = [x + round((ax - x) * k / self.tip) for x in e[:-1]] + [e[-1] - 1 + round((ax - e[-1] + 1) * k / self.tip) + 1]
            for i, color in enumerate(self.faces):
                for x in range(b[i], max(b[i + 1], b[i] + (1 if k == self.tip and i == 0 else 0))):
                    px[(x, self.by - self.h + 1 - k)] = color

    def fracture(self, px, row, crack, ledge, ledge_color, faces=2, slope=(-1, 2)):
        """Break the left faces with a crack (default rising right at 1:2) and a shadowed ledge beneath it."""
        e = self.edges(row)
        for j in range(e[faces] - e[0]):
            y = self.by - row + (slope[0] * j) // slope[1]
            px[(e[0] + j, y)] = crack
            for k in range(1, ledge + 1):
                if px.get((e[0] + j, y + k)) == self.faces[0]:
                    px[(e[0] + j, y + k)] = ledge_color

    def foot(self, px, rows, color):
        """Recolor the lit face just above the root, where the stone occludes it."""
        for r in range(rows):
            e = self.edges(r)
            for x in range(e[0], e[1]):
                px[(x, self.by - r)] = color


def stone(d, c, outline, top, lit=(), joints=(), crumbs=()):
    """Draw a slate root mass: front face, a receiving top band, upper-left lips and dark joints."""
    d.polygon(outline, fill=c["r1"])
    d.polygon(top, fill=c["r2"])
    for seg in lit:
        d.line(seg, fill=c["r3"])
    for seg in joints:
        d.line(seg, fill=c["r0"])
    for seg in crumbs:
        d.line(seg, fill=c["i1"])


def cap(px, c, points):
    """Mark a compact emissive cap; the bake needs a few of these near the base column."""
    for p in points:
        px[p] = c["i3"]


def violet_a(px, c):
    """Lake island and gallery: a left-leaning spear in front of a broad, darker right-leaning back shard."""
    back = Prism(15, 58, (1, 8), 32, [3, 7, 6], [c["i1"], c["i0"], c["i0"]], 8, apex=0.4)
    back.draw(px)
    bx, by = back.apex_xy()
    cap(px, c, [(bx, by), (bx, by + 1)])
    Prism(35, 59, (1, 2), 6, [2, 4], [c["i1"], c["i0"]], 4, apex=0.2).draw(px)
    spear = Prism(11, 59, (-1, 5), 40, [8, 3, 3], [c["i2"], c["i1"], c["i0"]], 10, apex=0.4)
    spear.draw(px)
    spear.fracture(px, 22, c["i0"], 2, c["i1"])
    split(px, spear, 5, 21, c["i1"])
    spear.foot(px, 4, c["i1"])
    ax, ay = spear.apex_xy()
    cap(px, c, [(ax, ay), (ax, ay + 1), (ax - 1, ay + 1), (ax, ay + 2), (ax - 1, ay + 2)])
    # A short lit ridge under the cap gives the reflection one longer facet to keep.
    cap(px, c, [(spear.edges(r)[1] - 1, spear.by - r) for r in range(spear.h - 3, spear.h)])
    stub = Prism(7, 60, (-1, 2), 7, [3, 2, 2], [c["i1"], c["i1"], c["i0"]], 4, apex=0.2)
    stub.draw(px)
    sx, sy = stub.apex_xy()
    cap(px, c, [(sx, sy), (sx, sy + 1)])
    return lambda d: stone(
        d, c,
        [(3, 62), (5, 58), (11, 57), (16, 58), (21, 56), (27, 52), (34, 51), (40, 54), (45, 59), (43, 63), (5, 63)],
        [(5, 58), (11, 57), (16, 58), (21, 56), (27, 52), (34, 51), (40, 54), (41, 56), (34, 54), (27, 55), (21, 58), (11, 59), (5, 60)],
        lit=[[(27, 52), (33, 51)], [(6, 58), (10, 57)]],
        joints=[[(20, 59), (21, 62)], [(35, 55), (36, 61)], [(5, 63), (43, 63)]],
    )


def split(px, prism, keep, rows, color):
    """Below the fracture, turn all but the first `keep` columns of the lit face into a duller plane."""
    for r in range(rows):
        e = prism.edges(r)
        for x in range(e[0] + keep, e[1]):
            if px.get((x, prism.by - r)) == prism.faces[0]:
                px[(x, prism.by - r)] = color


def chisel(px, prism, rise, band, plane, lip):
    """Cut the prism's top flat, rising to the right, and show that cut as a plane facing the light."""
    e = prism.edges(prism.h - 1)
    width = e[-1] - e[0]
    for j in range(width):
        x = e[0] + j
        top = prism.by - prism.h + 1 - round(rise * j / max(width - 1, 1))
        face = next(i for i in range(len(prism.faces)) if e[i] <= x < e[i + 1])
        for y in range(top, prism.by - prism.h + 1):
            px[(x, y)] = prism.faces[face]
        for k in range(band):
            px[(x, top + k)] = plane
        if lip and j >= width - 3:
            px[(x, top)] = lip


def aqua_a(px, c):
    """Heart: a broad dark column cut flat on top, the cut facing the light, with a back needle and a side shard."""
    Prism(10, 57, (-1, 6), 32, [2, 5], [c["i1"], c["i0"]], 8, apex=0.3).draw(px)
    Prism(33, 60, (1, 2), 9, [2, 4], [c["i1"], c["i0"]], 4, apex=0.3).draw(px)
    column = Prism(14, 60, (1, 8), 34, [8, 5, 4], [c["i2"], c["i1"], c["i0"]], 0)
    column.draw(px)
    column.fracture(px, 15, c["i0"], 2, c["i1"])
    split(px, column, 3, 15, c["i1"])
    column.foot(px, 5, c["i1"])
    chisel(px, column, 10, 7, c["i2"], c["i3"])
    e = column.edges(column.h - 1)
    top = column.by - column.h + 1
    # The cut's lower lip separates the lit cut from the lit side face below it.
    for j in range(e[1] - e[0]):
        px[(e[0] + j, top - round(10 * j / (e[-1] - e[0] - 1)) + 7)] = c["i1"]
    cap(px, c, [(e[-1] - 1, top - 9), (e[-1] - 2, top - 8), (e[-1] - 1, top - 8)])
    Prism(8, 61, (-1, 2), 4, [2, 2], [c["i1"], c["i0"]], 3, apex=0.3).draw(px)
    return lambda d: stone(
        d, c,
        [(3, 62), (4, 57), (9, 53), (15, 54), (19, 57), (26, 58), (33, 58), (38, 56), (44, 59), (42, 63), (5, 63)],
        [(4, 57), (9, 53), (15, 54), (19, 57), (26, 58), (33, 58), (38, 56), (44, 59), (38, 58), (33, 60), (26, 60), (18, 59), (14, 56), (9, 56), (5, 59)],
        lit=[[(4, 57), (8, 54)], [(9, 53), (14, 54)], [(38, 56), (41, 57)]],
        joints=[[(16, 57), (17, 62)], [(31, 60), (32, 62)], [(5, 63), (42, 63)]],
    )


def aqua_b(px, c):
    """Grove: a thick lit crystal toppled across a dark upright one, resting on its root stone."""
    back = Prism(12, 58, (-1, 9), 36, [3, 4, 4], [c["i1"], c["i0"], c["i0"]], 7, apex=0.3)
    back.draw(px)
    bx, by = back.apex_xy()
    cap(px, c, [(bx, by), (bx, by + 1)])
    slab = Prism(9, 61, (1, 1), 19, [6, 4, 5], [c["i2"], c["i1"], c["i0"]], 8, apex=0.2)
    slab.draw(px)
    slab.fracture(px, 10, c["i0"], 1, c["i1"], faces=1, slope=(1, 1))
    split(px, slab, 2, 9, c["i1"])
    slab.foot(px, 5, c["i1"])
    ax, ay = slab.apex_xy()
    cap(px, c, [(ax, ay), (ax, ay + 1), (ax - 1, ay + 1), (ax - 1, ay + 2)])
    return lambda d: stone(
        d, c,
        [(4, 62), (6, 59), (12, 58), (18, 59), (24, 57), (29, 53), (36, 51), (42, 53), (45, 58), (42, 63), (6, 63)],
        [(6, 59), (12, 58), (18, 59), (24, 57), (29, 53), (36, 51), (42, 53), (40, 55), (36, 54), (30, 56), (24, 59), (12, 60), (6, 61)],
        lit=[[(29, 53), (35, 51)], [(7, 59), (11, 58)]],
        joints=[[(28, 57), (30, 62)], [(41, 56), (42, 60)], [(6, 63), (42, 63)]],
    )


def violet_b(px, c):
    """Spare: a squat crowned crystal leaning right, flanked by a low left shard and a dark right one."""
    side = Prism(28, 59, (1, 3), 16, [3, 5, 4], [c["i1"], c["i0"], c["i0"]], 6, apex=0.3)
    side.draw(px)
    sx, sy = side.apex_xy()
    cap(px, c, [(sx, sy), (sx, sy + 1)])
    Prism(10, 60, (-1, 3), 12, [3, 3, 2], [c["i1"], c["i1"], c["i0"]], 5, apex=0.3).draw(px)
    crown = Prism(13, 60, (1, 6), 26, [7, 5, 5], [c["i2"], c["i1"], c["i0"]], 10, apex=0.35)
    crown.draw(px)
    crown.fracture(px, 12, c["i0"], 2, c["i1"])
    crown.foot(px, 6, c["i1"])
    ax, ay = crown.apex_xy()
    cap(px, c, [(ax, ay), (ax, ay + 1), (ax - 1, ay + 1), (ax, ay + 2), (ax - 1, ay + 2)])
    return lambda d: stone(
        d, c,
        [(4, 62), (6, 58), (13, 57), (20, 58), (27, 56), (34, 55), (41, 57), (44, 61), (41, 63), (6, 63)],
        [(6, 58), (13, 57), (20, 58), (27, 56), (34, 55), (41, 57), (39, 59), (34, 57), (27, 58), (20, 60), (13, 59), (6, 60)],
        lit=[[(27, 56), (33, 55)], [(7, 58), (12, 57)]],
        joints=[[(26, 59), (27, 62)], [(6, 63), (41, 63)]],
    )


PLANS = {"aqua_a": aqua_a, "violet_a": violet_a, "aqua_b": aqua_b, "violet_b": violet_b}


def draw(name, region):
    """Render one landmark cell by its legacy animation name: crystals first, then the root over them."""
    px = {}
    finish = PLANS[name](px, palette(region))
    cell = Image.new("RGBA", (48, 64))
    for (x, y), color in px.items():
        if 0 <= x < 48 and 0 <= y < 64:
            cell.putpixel((x, y), tuple(int(color[i:i + 2], 16) for i in (1, 3, 5)) + (255,))
    finish(ImageDraw.Draw(cell))
    return cell
