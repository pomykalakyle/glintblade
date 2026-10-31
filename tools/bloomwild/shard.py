"""shard: the prism shard collectible (shared by every region).

A broken fragment of the Prism: an irregular outline with one fractured side, cut into facets on
both faces (a fan of facets around a raised center on the front and the back). It spins about its
vertical axis, 45 degrees per frame, so its width cycles broad face -> edge -> broad reverse face
and the lit facet moves steadily around it. One body (violet with cyan lit facets); white only as
a glint on the front-facing frame, a pink glint briefly traveling down an edge.
"""
import math

import numpy as np

from pix import *

CW, CH = 16, 24
AX, AY = 8, 22
N = 8

# outline in the shard's own plane (x right, y down), clockwise from the top tip; the right side
# is the fractured one (a notch and a short step)
OUTLINE = [(0.2, -8.6), (2.4, -4.2), (3.4, -1.6), (2.0, -0.6), (3.0, 1.6), (0.9, 5.6),
           (-1.0, 4.4), (-3.4, -0.4), (-2.2, -4.8)]
APEX = (0.0, -1.0)
DEPTH = 1.5


def mesh():
    tris = []
    for sgn in (1, -1):
        c = np.array([APEX[0], APEX[1], DEPTH * sgn])
        for i in range(len(OUTLINE)):
            a = OUTLINE[i]
            b = OUTLINE[(i + 1) % len(OUTLINE)]
            pa, pb = np.array([a[0], a[1], 0.0]), np.array([b[0], b[1], 0.0])
            tris.append(((c, pa, pb) if sgn > 0 else (c, pb, pa), (i, sgn)))
    return tris


L = np.array([-0.5, -0.6, 0.62])
L = L / np.linalg.norm(L)


def raster(theta, cx, cy):
    ct, st = math.cos(theta), math.sin(theta)
    zbuf = np.full((CH, CW), -1e9)
    col = [[None] * CW for _ in range(CH)]
    for (a, b, c), key in mesh():
        pts = []
        for p in (a, b, c):
            x, y, z = p
            pts.append(np.array([x * ct + z * st, y, -x * st + z * ct]))
        n = np.cross(pts[1] - pts[0], pts[2] - pts[0])
        if np.linalg.norm(n) < 1e-6:
            continue
        n = n / np.linalg.norm(n)
        if n[2] <= 0:
            continue
        lum = float(np.dot(n, L))
        shade = "p2" if lum > 0.72 else "p1" if lum > 0.3 else "p0"
        xs = [p[0] + cx for p in pts]
        ys = [p[1] + cy for p in pts]
        zs = [p[2] for p in pts]
        for py in range(max(0, int(min(ys)) - 1), min(CH, int(max(ys)) + 2)):
            for px in range(max(0, int(min(xs)) - 1), min(CW, int(max(xs)) + 2)):
                qx, qy = px + 0.5, py + 0.5
                d0 = (xs[1] - xs[0]) * (qy - ys[0]) - (ys[1] - ys[0]) * (qx - xs[0])
                d1 = (xs[2] - xs[1]) * (qy - ys[1]) - (ys[2] - ys[1]) * (qx - xs[1])
                d2 = (xs[0] - xs[2]) * (qy - ys[2]) - (ys[0] - ys[2]) * (qx - xs[2])
                if (d0 >= 0 and d1 >= 0 and d2 >= 0) or (d0 <= 0 and d1 <= 0 and d2 <= 0):
                    z = sum(zs) / 3
                    if z > zbuf[py, px]:
                        zbuf[py, px] = z
                        col[py][px] = shade
    img = new(CW, CH)
    for y in range(CH):
        for x in range(CW):
            if col[y][x]:
                put(img, x, y, col[y][x])
    return img


BOB = [0, 0, -1, -1, -1, -1, 0, 0]


def left_edge(img, y):
    xs = [x for x in range(CW) if opaque(img, x, y)]
    return xs[0] if xs else None


def spin_frames():
    frames = []
    for i in range(N):
        theta = i * 2 * math.pi / N
        img = raster(theta, 8.0, 12.0 + BOB[i])
        img = clean_orphans(img, passes=1)
        rows = [y for y in range(CH) if mask_of(img)[y].any()]
        top = rows[0]
        if i == 0:   # front-facing: a small white glint on the upper-left facet
            x = left_edge(img, top + 4)
            put(img, x + 1, top + 4, "p5"); put(img, x + 1, top + 5, "p5")
        if i in (1, 2):   # pink glint traveling down the left edge
            y = top + (5 if i == 1 else 8)
            x = left_edge(img, y)
            if x is not None:
                put(img, x, y, "p3")
        img = outline(img, "s0")
        frames.append(img)
    return frames


def pickup_frames():
    out = []
    base = spin_frames()[0]
    out.append(recolor(base, {"p0": "p5", "p1": "p5", "p2": "p5", "p3": "p5"}))
    star = new(CW, CH)
    grid(["...w...", "...w...", "..www..", "wwwwwww", "..www..", "...w...", "...w..."],
         {"w": "p5"}, star, 5, 9)
    out.append(star)
    for r, c, dash in ((4.5, "p5", False), (6.5, "p2", True)):
        img = new(CW, CH)
        for y in range(CH):
            for x in range(CW):
                d = math.hypot(x + 0.5 - 8, y + 0.5 - 12.5)
                if r - 1 <= d < r:
                    a = math.atan2(y + 0.5 - 12.5, x + 0.5 - 8)
                    if not dash or int((a + math.pi) / (math.pi / 4)) % 2 == 0:
                        put(img, x, y, c)
        out.append(img)
    return out


EMISSIVE = ["p2", "p3", "p5"]


def write():
    spin = spin_frames()
    pick = pickup_frames()
    rows = [spin, pick + [new(CW, CH)] * (N - len(pick))]
    man = {
        "cell": [CW, CH],
        "anchor": [AX, AY],
        "emissive": [hexof(c) for c in EMISSIVE],
        "anims": {
            "spin": {"row": 0, "frames": N, "fps": 8,
                     "light": {"color": "#5ff2ff", "radius": 24, "intensity": 0.5, "offset": [0, -10]}},
            "pickup": {"row": 1, "frames": 4, "fps": 20, "durations": [3, 3, 3, 4]},
        },
        "notes": "Prism shard: a broken fragment (fractured right side) spinning 45 degrees per frame. Floats "
                 "about 10 px above its anchor with a 1 px bob; draw a small soft shadow (f.shadow) at the anchor. "
                 "pickup plays once: white flash, a star, then an expanding ring.",
    }
    sheet = save_sheet("shard", rows, CW, CH, man)
    preview(sheet, PREVIEWS / "shard_8x.png", k=6, bg="g2", gridcell=(CW, CH))
    gif(spin, PREVIEWS / "shard_spin.gif", k=8, bg="g2", durations=[7] * N)
    return sheet


if __name__ == "__main__":
    write()
