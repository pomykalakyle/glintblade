"""cinder_trees: charred dead trees. Cell 48x64, anchor at the trunk's base center.

Each limb is a stroke (polyline with a radius that tapers). Thick parts are rasterized as capsules and
shaded by their surface normal against the upper-left light (lit / mid / dark bands that follow the
limb, so no pillow shading). Parts thinner than 1 px are drawn as clean 1-px Bresenham lines.
"""

import math

import numpy as np

from draw import Canvas, line
from lib import SheetBuilder

W, H = 48, 64
ANCHOR = [24, 61]
L = (-0.6, -0.8)  # light direction (toward the light: up-left, mostly up)

# charred wood colors: near-black warm core, ashy grey-mauve on the lit side
C_DARK = "1"   # #1a1214
C_MID = "2"    # #30201f
C_LIT = "3"    # #4c3430
C_HI = "4"     # #6e4e44, warm lit ridge on the light side
C_TWIG = "1"


class Tree:
    def __init__(self):
        """Initialize the native pixel drawing state."""
        self.order = np.full((H, W), -1, int)   # which stroke owns the pixel (later strokes on top)
        self.shade = np.full((H, W), ".", dtype="<U1")
        self.radius = np.zeros((H, W))
        self.n = 0

    def stroke(self, pts, r0, r1, thin_col=C_TWIG):
        """Draw the stroke on the native Cinder pixel grid."""
        pts = [(float(x), float(y)) for x, y in pts]
        segs = list(zip(pts, pts[1:]))
        lens = [math.dist(a, b) for a, b in segs]
        total = sum(lens)
        acc = [0.0]
        for l in lens:
            acc.append(acc[-1] + l)
        k = self.n
        self.n += 1
        # thick part: capsule SDF
        for y in range(H):
            for x in range(W):
                px, py = x + 0.5, y + 0.5
                best = None
                for i, ((ax, ay), (bx, by)) in enumerate(segs):
                    dx, dy = bx - ax, by - ay
                    ll = dx * dx + dy * dy
                    t = 0.0 if ll == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / ll))
                    qx, qy = ax + t * dx, ay + t * dy
                    d = math.hypot(px - qx, py - qy)
                    s = (acc[i] + t * lens[i]) / total
                    r = r0 + (r1 - r0) * s
                    if best is None or d - r < best[0]:
                        best = (d - r, d, r, qx, qy)
                gap, d, r, qx, qy = best
                if r >= 0.95 and gap <= 0:
                    nx, ny = ((px - qx) / d, (py - qy) / d) if d > 1e-6 else (0.0, 0.0)
                    lam = nx * L[0] + ny * L[1]
                    rel = d / max(r, 1e-6)
                    if r >= 1.6 and lam > 0.55 and rel > 0.55:
                        ch = C_HI
                    elif lam > 0.15:
                        ch = C_LIT
                    elif lam < -0.25 and rel > 0.3:
                        ch = C_DARK
                    else:
                        ch = C_MID
                    self.shade[y, x] = ch
                    self.order[y, x] = k
                    self.radius[y, x] = r
        # thin part: 1-px line along the centerline where r < 0.95
        thin = []
        for i, (a, b) in enumerate(segs):
            s0, s1 = acc[i] / total, acc[i + 1] / total
            ra, rb = r0 + (r1 - r0) * s0, r0 + (r1 - r0) * s1
            if rb < 0.95:
                if ra >= 0.95:
                    f = (ra - 0.95) / (ra - rb)
                    a = (a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f)
                thin.append((a, b))
        if thin:
            c = Canvas(W, H)
            for a, b in thin:
                line(c, [(round(a[0] - 0.5), round(a[1] - 0.5)), (round(b[0] - 0.5), round(b[1] - 0.5))], "#")
            m = (c.a == "#") & (self.shade == ".")
            self.shade[m] = thin_col
            self.order[m] = k

    def canvas(self, outline=True):
        """Outline only the shadow side (right, below) of limbs thicker than 1.3 px radius."""
        c = Canvas(W, H)
        c.a = self.shade.copy()
        if outline:
            m = c.a != "."
            thick = self.radius >= 1.3
            add = np.zeros_like(m)
            # pixel to the right of / below a thick pixel
            add[:, 1:] |= thick[:, :-1]
            add[1:, :] |= thick[:-1, :]
            add[1:, 1:] |= thick[:-1, :-1]
            add &= ~m
            c.a[add] = "K"
        return c


def clean(c):
    """Drop isolated single pixels and 1-px notches."""
    m = c.a != "."
    p = np.pad(m, 1)
    n8 = sum(p[1 + dy:H + 1 + dy, 1 + dx:W + 1 + dx].astype(int)
             for dy in (-1, 0, 1) for dx in (-1, 0, 1) if dy or dx)
    c.a[m & (n8 == 0)] = "."
    return c


def tree_tall():
    """Draw the tree tall on the native Cinder pixel grid."""
    t = Tree()
    # roots: three tendrils gripping the ground
    t.stroke([(23.5, 57), (20, 59.5), (17, 60.5), (14, 60.5), (12, 61.5)], 2.0, 0.5)
    t.stroke([(25, 57), (28.5, 59.5), (31, 60), (34, 61.5)], 1.8, 0.5)
    t.stroke([(24, 58.5), (23, 61.8)], 1.7, 1.3)
    t.stroke([(25, 59), (27, 61.8)], 1.2, 0.9)
    # left bough with a crook, forked tips
    t.stroke([(24.5, 37), (21, 32), (16.5, 28.5), (14, 23), (11, 18.5), (10, 14), (8, 10)], 2.1, 0.5)
    t.stroke([(10, 14), (12.5, 10.5), (12.5, 8)], 0.8, 0.5)
    t.stroke([(16.5, 28.5), (12, 28.5), (8.5, 26), (6.5, 23.5), (6.5, 21)], 1.1, 0.5)
    t.stroke([(8.5, 26), (5, 27), (3, 26)], 0.7, 0.5)
    t.stroke([(14, 23), (16.5, 19), (16.5, 16), (18, 13)], 0.9, 0.5)
    # right bough, reaching higher
    t.stroke([(25, 36), (28, 30), (29.5, 25), (32.5, 20), (35, 15.5), (36, 11), (38.5, 7), (38.5, 5)], 2.1, 0.5)
    t.stroke([(32.5, 20), (36.5, 20), (39, 17.5), (42, 16.5)], 1.1, 0.5)
    t.stroke([(39, 17.5), (40, 14)], 0.7, 0.5)
    t.stroke([(35, 15.5), (33, 12), (33.5, 8.5), (32, 6)], 0.8, 0.5)
    # low limb on the right, drooping
    t.stroke([(25.5, 42), (29.5, 39.5), (34, 39), (37.5, 36.5), (40, 33), (43, 32)], 1.6, 0.5)
    t.stroke([(34, 39), (36.5, 42), (39.5, 43.5)], 0.8, 0.5)
    # small central spur
    t.stroke([(24.5, 33), (25.5, 27), (24, 22), (24.5, 18)], 1.2, 0.5)
    # trunk: gentle S curve, thick at the base
    t.stroke([(24, 60), (23, 53), (24.5, 46), (25.5, 41), (24.5, 35)], 3.6, 2.0)
    return t


def tree_snag():
    """Draw the tree snag on the native Cinder pixel grid."""
    t = Tree()
    t.stroke([(22.5, 57), (18.5, 59.5), (15, 60.5), (12, 61.5)], 2.0, 0.5)
    t.stroke([(23.5, 57), (27.5, 59.5), (30.5, 60.5), (34, 61.5)], 1.9, 0.5)
    t.stroke([(23, 58.5), (22.5, 61.8)], 1.8, 1.4)
    # one long limb reaching right, a short one left
    t.stroke([(24, 40), (28, 36), (32.5, 34.5), (36.5, 31.5), (40, 30.5), (43, 27.5), (45, 27)], 2.2, 0.5)
    t.stroke([(36.5, 31.5), (37, 27.5), (39.5, 24.5), (39.5, 22)], 1.0, 0.5)
    t.stroke([(32.5, 34.5), (34.5, 38), (37.5, 39.5)], 0.9, 0.5)
    t.stroke([(22, 36), (18, 33), (14.5, 32), (12, 28.5), (12, 26)], 1.6, 0.5)
    t.stroke([(14.5, 32), (11, 33.5), (8.5, 33)], 0.8, 0.5)
    # splinters left standing where the crown snapped off
    t.stroke([(21.5, 31), (21, 26), (20.5, 23)], 1.5, 0.5)
    t.stroke([(24.5, 31), (25, 28), (26, 26)], 1.2, 0.5)
    # trunk
    t.stroke([(23, 60), (22, 52), (23.5, 44), (23, 36), (23, 31)], 3.8, 2.8)
    return t


def tree_small():
    """Draw the tree small on the native Cinder pixel grid."""
    t = Tree()
    t.stroke([(23.5, 59.5), (20, 61.5), (17.5, 61.5)], 1.4, 0.6)
    t.stroke([(24.5, 59.5), (28, 61.5)], 1.3, 0.6)
    t.stroke([(24.5, 49), (27.5, 44), (30.5, 40), (32, 36)], 1.4, 0.5)
    t.stroke([(23.5, 50), (20.5, 46), (18, 42.5), (17, 39)], 1.3, 0.5)
    t.stroke([(27.5, 44), (31, 44.5), (33, 43)], 0.8, 0.5)
    t.stroke([(24, 47), (24.5, 42), (23.5, 38)], 0.9, 0.5)
    t.stroke([(24, 60.5), (23.5, 55), (24, 49)], 2.4, 1.6)
    return t


def smolder(c, pts):
    """Ember-filled cracks in the bark (glowing), for the smoldering variant."""
    for x, y, ch in pts:
        if c.get(x, y) != ".":
            c.set(x, y, ch)


def alligator(c, pts):
    """A few short lit ticks on the trunk: charred bark cracking into blocks."""
    for x, y in pts:
        if c.get(x, y) in "123":
            c.set(x, y, "4" if c.get(x, y) == "3" else "3")


def variants():
    """Return the variants representation for the Cinder generator."""
    a = clean(tree_tall().canvas())
    alligator(a, [(23, 49), (24, 49), (22, 54), (23, 54), (24, 43), (25, 43)])
    b = clean(tree_snag().canvas())
    alligator(b, [(21, 47), (22, 47), (21, 53), (22, 53), (22, 41), (23, 41)])
    # snapped top: lit fracture face on the stump between the splinters
    for x, y in [(22, 30), (23, 30), (24, 30), (22, 31), (23, 31)]:
        if b.get(x, y) != ".":
            b.set(x, y, "4" if y == 30 else "3")
    s = clean(tree_small().canvas())
    a2 = a.copy()
    # one continuous ember seam up the trunk's face, hottest in the middle, plus a glow at the root
    smolder(a2, [(24, 45, "w"), (24, 46, "x"), (24, 47, "x"), (23, 48, "x"), (23, 49, "y"), (23, 50, "y"),
                 (23, 51, "z"), (24, 52, "y"), (24, 53, "x"), (24, 54, "w"),
                 (25, 50, "w"), (22, 49, "w")])
    smolder(a2, [(21, 59, "w"), (22, 59, "x"), (23, 59, "y"), (22, 58, "w")])
    return a, a2, b, s


def build():
    """Build the sheet with its original cells, anchors and animation contracts."""
    a, a2, b, s = variants()
    sb = SheetBuilder("cinder_trees", (W, H), ANCHOR)
    sb.add("tree_tall", [a.rgba()], footprint=[8, 5])
    sb.add("tree_tall_smolder", [a2.rgba()], footprint=[8, 5],
           light={"color": "#efb68a", "radius": 22, "intensity": 0.6, "offset": [0, -10]})
    sb.add("tree_snag", [b.rgba()], footprint=[8, 5])
    sb.add("tree_small", [s.rgba()], footprint=[6, 4])
    return sb


if __name__ == "__main__":
    from view import show
    print(show([v.rgba() for v in variants()], "trees_v2"))
