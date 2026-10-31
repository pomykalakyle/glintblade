"""cinder_statue: a broken bone-white guardian on an obsidian plinth. Cell 64x64, anchor at the plinth's base center.

The figure is modeled from a few signed-distance primitives (hood, mantle, shoulders, arms, a robe cone whose
folds widen toward the hem), lit from the upper left and quantized to the 6-step bone ramp, so terminators
follow the forms and the arm casts a contact shadow on the torso. Face, break, cracks, sword and plinth are
then painted by hand on top.
"""

import numpy as np

from draw import Canvas, line
from lib import SheetBuilder

W, H = 64, 64
ANCHOR = [32, 60]
BONE = ["6", "7", "8", "9", "V", "W"]   # dark -> light

ZN = 72
Z0 = -18.0


def grid():
    ys, xs, zs = np.meshgrid(np.arange(H) + 0.5, np.arange(W) + 0.5, Z0 + np.arange(ZN) * 0.5 + 0.25, indexing="ij")
    return xs, ys, zs


def ellipsoid(p, c, r):
    x, y, z = p
    q = np.sqrt(((x - c[0]) / r[0]) ** 2 + ((y - c[1]) / r[1]) ** 2 + ((z - c[2]) / r[2]) ** 2)
    return (q - 1.0) * min(r)


def capsule(p, a, b, ra, rb):
    x, y, z = p
    ax, ay, az = a
    bx, by, bz = b
    dx, dy, dz = bx - ax, by - ay, bz - az
    ll = dx * dx + dy * dy + dz * dz
    t = np.clip(((x - ax) * dx + (y - ay) * dy + (z - az) * dz) / ll, 0, 1)
    d = np.sqrt((x - ax - t * dx) ** 2 + (y - ay - t * dy) ** 2 + (z - az - t * dz) ** 2)
    return d - (ra + (rb - ra) * t)


def robe(p, cx, y0, y1, r0, r1):
    x, y, z = p
    t = np.clip((y - y0) / (y1 - y0), 0, 1)
    ang = np.arctan2(z, x - cx)
    fold = (t ** 1.3) * 1.9 * np.sin(5 * ang + 0.4)
    r = r0 + (r1 - r0) * t ** 1.25 + fold
    d = np.sqrt((x - cx) ** 2 + (z / 0.85) ** 2) - r
    return np.maximum(d, np.maximum(y0 - y, y - y1))


def smin(a, b, k=2.0):
    h = np.clip(0.5 + 0.5 * (b - a) / k, 0, 1)
    return b * (1 - h) + a * h - k * h * (1 - h)


PARTS = {}


def scene(p):
    """Returns (sdf, part id) over the grid."""
    hood = smin(ellipsoid(p, (32, 15.5, 0), (7.2, 8.8, 6.8)),
                capsule(p, (32, 10, -0.5), (32.6, 5.2, -1.0), 3.6, 0.9), 2.5)
    mantle = ellipsoid(p, (32, 22.5, 0), (10.5, 4.6, 7.0))
    chest = ellipsoid(p, (32, 28, 0), (10.0, 6.5, 6.4))
    body = smin(smin(hood, mantle, 1.5), chest, 2.0)
    body = smin(body, robe(p, 32, 28, 53, 8.6, 13.2), 2.0)
    shoulder_l = ellipsoid(p, (22.2, 24.2, 0.5), (4.6, 4.2, 4.6))
    upper_l = capsule(p, (22, 25, 1.0), (20.4, 32.5, 4.5), 3.3, 2.9)
    fore_l = capsule(p, (20.8, 32.6, 5.0), (28.6, 35.4, 11.5), 2.8, 2.3)
    hand_l = ellipsoid(p, (31.4, 35.2, 12.2), (3.0, 2.6, 2.4))
    arm_l = smin(smin(smin(shoulder_l, upper_l, 1.2), fore_l, 1.0), hand_l, 0.8)
    shoulder_r = ellipsoid(p, (41.8, 24.2, 0.5), (4.6, 4.2, 4.6))
    upper_r = capsule(p, (42, 25, 0.5), (43.8, 34, 2.0), 3.4, 3.1)
    arm_r = smin(shoulder_r, upper_r, 1.2)
    # the right arm snapped off: cut everything below a tilted fracture plane
    x, y, z = p
    cut = (y - 31.0) - (x - 42.0) * 0.35 + 0.9 * (z - 1.5)
    arm_r = np.maximum(arm_r, cut)
    sdfs = [body, arm_l, arm_r]
    stack = np.stack(sdfs)
    return stack.min(0), stack.argmin(0), cut


def render():
    p = grid()
    sdf, part, cut = scene(p)
    inside = sdf < 0
    hit = inside.any(axis=2)
    # first z from the front (largest z) that is inside
    zi = ZN - 1 - np.argmax(inside[:, :, ::-1], axis=2)
    yy, xx = np.mgrid[0:H, 0:W]
    # normals from the sdf gradient at the hit cell
    s = sdf
    def g(a, axis):
        return np.gradient(a, axis=axis)
    gx, gy, gz = g(s, 1), g(s, 0), g(s, 2) / 0.5
    nx = gx[yy, xx, zi]
    ny = gy[yy, xx, zi]
    nz = gz[yy, xx, zi]
    n = np.sqrt(nx * nx + ny * ny + nz * nz) + 1e-9
    nx, ny, nz = nx / n, ny / n, nz / n
    L = np.array([-0.72, -0.5, 0.36])
    L = L / np.linalg.norm(L)
    lam = nx * L[0] + ny * L[1] + nz * L[2]
    pid = part[yy, xx, zi]
    zf = Z0 + zi * 0.5
    fracture = (cut[yy, xx, zi] > -0.8) & (pid == 2) & hit
    return hit, lam, pid, zf, fracture


# five tones on the forms (core shadow 7 .. highlight W); 6 is kept for crevices and contact shadows
THRESH = [(-0.12, 1), (0.16, 2), (0.5, 3), (0.9, 4), (9, 5)]


def quant(lam):
    for t, i in THRESH:
        if lam < t:
            return i
    return 5


def figure_canvas():
    hit, lam, pid, zf, fracture = render()
    c = Canvas(W, H)
    idx = np.full((H, W), -1)
    for y in range(H):
        for x in range(W):
            if hit[y, x]:
                idx[y, x] = quant(lam[y, x])
    # contact shadows: a pixel just behind a nearer part (other part id, > 1.5 px closer) darkens 2 steps
    for y in range(H):
        for x in range(W):
            if idx[y, x] < 0:
                continue
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                X, Y = x + dx, y + dy
                if 0 <= X < W and 0 <= Y < H and idx[Y, X] >= 0 and pid[Y, X] != pid[y, x] and zf[Y, X] - zf[y, x] > 1.5:
                    idx[y, x] = 0 if idx[y, x] <= 2 else 1
                    break
    for y in range(H):
        for x in range(W):
            if idx[y, x] >= 0:
                c.set(x, y, BONE[idx[y, x]])
            if fracture[y, x]:
                c.set(x, y, "6")
    return c, idx


def paint(c):
    # face opening: a pointed arch, darkest under the brow, lifting slightly toward the chin
    face = [
        (31, 11, "K"), (32, 11, "K"),
        (30, 12, "K"), (31, 12, "K"), (32, 12, "K"), (33, 12, "K"),
        (29, 13, "6"), (30, 13, "K"), (31, 13, "K"), (32, 13, "K"), (33, 13, "K"), (34, 13, "6"),
    ]
    for y in range(14, 19):
        for x in range(29, 35):
            face.append((x, y, "6"))
    face += [(30, 19, "6"), (31, 19, "7"), (32, 19, "7"), (33, 19, "6"),
             (30, 18, "7"), (33, 18, "7")]
    for x, y, ch in face:
        c.set(x, y, ch)
    # the hood's lit rim around the opening (left side catches light)
    for x, y in [(28, 14), (28, 15), (28, 16), (29, 12), (30, 11)]:
        if c.get(x, y) in "89":
            c.set(x, y, "V")

    # sword: copper pommel, grip under the hand, crossguard, obsidian blade in 3 values
    # pommel, then a stone fist closed around the grip: wrist from the left, a shadow gap, lit thumb and knuckles,
    # grip showing again under the fist, short shadows separating hand from chest
    c.stamp("""
        ......DC...
        .....DECB..
        776666BA667
        VV97VWVV986
        VV97WV9V986
        9986V989876
        88769878766
        777667BA767
    """, 25, 29)
    c.stamp("""
        .DDDDDDCCCCC.
        DEEDDDCCCCCBB
        .CCCCCCBBBBB.
    """, 26, 37)
    for y in range(40, 53):
        c.set(30, y, "s")
        c.set(31, y, "r")
        c.set(32, y, "r")
        c.set(33, y, "p")
    c.set(30, 53, ".")
    c.set(33, 53, ".")
    c.set(31, 53, "r")
    c.set(32, 53, "p")
    for y in (43, 44, 48):
        c.set(30, y, "t")

    # tapered folds on the lit side of the robe: a mid-tone valley widening toward the hem, lit ridge to its left
    for y in range(41, 53):
        k = (y - 41) / 11
        x0 = round(25 - 2.5 * k)
        wdt = 1 if k < 0.45 else 2
        for i in range(wdt):
            if c.get(x0 + i, y) in "VW":
                c.set(x0 + i, y, "9" if i == 0 or k > 0.8 else "V")
        if c.get(x0 - 1, y) == "V":
            c.set(x0 - 1, y, "W")
    for y in range(46, 53):
        if c.get(28, y) in "VW":
            c.set(28, y, "9")
    # cracks across the robe: dark line with a 1 px light lip below
    crack1 = [(38, 41), (39, 42), (40, 42), (41, 43), (42, 44), (43, 44)]
    crack2 = [(22, 47), (23, 46), (24, 46), (25, 45), (26, 45)]
    for pts in (crack1, crack2):
        for x, y in pts:
            if c.get(x, y) != ".":
                c.set(x, y, "6")
        for x, y in pts:
            if c.get(x, y + 1) not in ".6K" and (x, y + 1) not in pts:
                c.set(x, y + 1, "V" if x < 32 else "9")
    # chips on the hood peak and mantle edge
    return c


def plinth(c):
    """Obsidian plinth: lit top face (3 rows), front face, chipped front-right corner."""
    x0, x1 = 15, 49
    for y in (53, 54, 55):
        for x in range(x0, x1 + 1):
            ch = "r"
            if y == 55:
                ch = "s" if x < x1 - 5 else "r"
            if y == 53 and x > x1 - 8:
                ch = "q"
            c.set(x, y, ch)
    for y in range(56, 61):
        for x in range(x0, x1 + 1):
            i = x - x0
            ch = "r" if i < 2 else ("p" if i >= x1 - x0 - 1 else "q")
            c.set(x, y, ch)
    c.set(x0, 55, "t")
    c.set(x0 + 1, 55, "t")
    for y in (57, 58):
        c.set(x0 + 1, y, "t")
    # warm bounce from the lava on the lower right
    for y in (58, 59):
        c.set(x1, y, "B")
    # chipped front-right corner, fracture faces up-left so it is lit
    for x, y in [(49, 53), (49, 54), (48, 53), (49, 55), (49, 56)]:
        c.set(x, y, ".")
    c.set(48, 54, "s")
    c.set(48, 55, "t")
    c.set(49, 57, "r")


def rubble(c):
    # the snapped-off forearm lying at the plinth's right foot: broken end (dark face, lit rim) toward the
    # viewer on the left, a raised cuff ring, the fist at the right
    c.stamp("""
        ........VWV9.
        .VWWWV97VWV98
        W66VVV97W8W87
        V679998697976
        9668888687866
        .88777767676.
    """, 50, 54)
    # a chip of the shoulder
    c.stamp("""
        .V9.
        V987
    """, 9, 58)


def statue():
    fig, idx = figure_canvas()
    c = Canvas(W, H)
    plinth(c)
    # the figure stands on the plinth (its hem overlaps the plinth's top face)
    for y in range(H):
        for x in range(W):
            if fig.a[y, x] != "." and y <= 54:
                c.set(x, y, fig.a[y, x])
    # contact shadow of the hem on the plinth's top face
    for x in range(W):
        if fig.a[52, x] != "." and c.get(x, 53) in "rs":
            c.set(x, 53, "q")
            c.set(x, 54, "q" if fig.a[52, x - 1] != "." and fig.a[52, x + 1] != "." else "r")
    paint(c)
    stump(c)
    rubble(c)
    c.outline("K")
    return c


def stump(c):
    """Right arm snapped off above the elbow: the SDF cut leaves a dark fracture face (6) toward the viewer.
    Bound it clearly: a chipped lit rim along its top edge, one grain line inside, a jagged dark lower lip."""
    cols = {}
    for x in range(39, 50):
        ys = [y for y in range(24, 38) if c.get(x, y) == "6"]
        if ys:
            cols[x] = ys
    for i, (x, ys) in enumerate(sorted(cols.items())):
        top, bot = ys[0], ys[-1]
        c.set(x, top, "W" if i % 3 == 0 else "V")      # chipped rim catches the light
        if i % 3 == 1 and top + 1 <= bot:
            c.set(x, top + 1, "9")                     # a chip bitten into the rim
        if i % 2 == 0 and bot - 1 > top + 1:
            c.set(x, bot, ".")                         # jagged lower lip
    mid = sorted(cols)[len(cols) // 2] if cols else None
    if mid is not None and len(cols[mid]) > 3:
        c.set(mid, cols[mid][2], "7")


def build():
    sb = SheetBuilder("cinder_statue", (W, H), ANCHOR)
    sb.add("statue", [statue().rgba()], footprint=[36, 9])
    return sb


NOTES = ("Bone-white stone uses a custom ramp not in palettes.json: #3a2a3a #5e4a58 #8a7478 #b8a49c #ddccb8 #f4ead2 "
         "(violet shadows to warm cream lights). Plinth and blade are obsidian, guard and pommel copper.")


if __name__ == "__main__":
    from view import show
    print(show([statue().rgba()], "statue", s=5))
