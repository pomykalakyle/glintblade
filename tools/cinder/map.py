"""Compose the Cinder Steppes tile map from shapes, print it as ASCII and render a preview.

Legend: ' ' void, '.' ash, ',' scorched grass, '-' path, ':' obsidian paving, '#' basalt rock,
'O' obsidian masonry, '~' lava, '=' bridge (floor over lava), '^' dais (raised obsidian floor),
's' stairs.
Run: uv run --no-project --with pillow python3 tools/cinder/map.py [out.txt]
"""

import math
import os
import sys

from PIL import Image

W, H = 88, 57
g = [["." for _ in range(W)] for _ in range(H)]


def poly_inside(px, py, pts):
    """Test a tile center against an authored polygon."""
    inside = False
    n = len(pts)
    for i in range(n):
        x1, y1 = pts[i]
        x2, y2 = pts[(i + 1) % n]
        if (y1 > py) != (y2 > py):
            xi = x1 + (py - y1) * (x2 - x1) / (y2 - y1)
            if px < xi:
                inside = not inside
    return inside


def fill_poly(pts, c, only=None):
    """Paint a polygon over the allowed logical materials."""
    for y in range(H):
        for x in range(W):
            if poly_inside(x + 0.5, y + 0.5, pts) and (only is None or g[y][x] in only):
                g[y][x] = c


def fill_ellipse(cx, cy, rx, ry, c, only=None):
    """Paint an elliptical logical landform."""
    for y in range(H):
        for x in range(W):
            dx = (x + 0.5 - cx) / rx
            dy = (y + 0.5 - cy) / ry
            if dx * dx + dy * dy <= 1.0 and (only is None or g[y][x] in only):
                g[y][x] = c


def thick_line(pts, c, only=None):
    """pts: (x, y, half_width)."""
    for y in range(H):
        for x in range(W):
            px, py = x + 0.5, y + 0.5
            for (ax, ay, aw), (bx, by, bw) in zip(pts, pts[1:]):
                vx, vy = bx - ax, by - ay
                l2 = vx * vx + vy * vy
                t = 0 if l2 == 0 else max(0, min(1, ((px - ax) * vx + (py - ay) * vy) / l2))
                d = math.hypot(px - (ax + vx * t), py - (ay + vy * t))
                if d <= aw + (bw - aw) * t and (only is None or g[y][x] in only):
                    g[y][x] = c
                    break


def rect(x0, y0, x1, y1, c, only=None):
    """Paint an inclusive rectangle over the allowed logical materials."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if 0 <= x < W and 0 <= y < H and (only is None or g[y][x] in only):
                g[y][x] = c


# ---------------------------------------------------------------- island outline
# Composed in "design" coordinates; everything is shifted down by TOP void rows at the end so the
# gate on the dais has sky behind it.
island = [
    (3, 3), (10, 2.4), (18, 3), (26, 2.4), (34, 3.2), (42, 2.5), (50, 3), (58, 2.4), (66, 1.6), (76, 1.6), (84, 2.6),
    (85.5, 8), (84.6, 14), (85.4, 20), (83, 23.2),
    (78.5, 24.6), (76.6, 29), (77.2, 35), (79.5, 39.6), (84, 41.4), (85.4, 45),
    (80, 46.6), (70, 46.2), (60, 46.8), (50, 46.2), (42, 46.6), (34, 46.2), (26, 46.6), (19, 46.2), (12, 46.8), (6, 46.4), (3.2, 44),
    (2.4, 36), (3.2, 28), (2.4, 20), (3.2, 12), (2.4, 6),
]
for y in range(H):
    for x in range(W):
        if not poly_inside(x + 0.5, y + 0.5, island):
            g[y][x] = " "

# ---------------------------------------------------------------- rock masses (basalt)
# NW highland, reaching down to leave a small grass pocket
fill_poly([(2, 2), (21, 2), (22, 7), (20.5, 11), (18, 14), (14, 16.5), (10.5, 15.2), (7, 17.6), (2, 18.5)], "#", only=".")
# north strip highlands across the river (scenery seen from the ruins)
fill_poly([(22, 2), (36, 2), (37, 6.5), (33, 9.0), (28, 10.4), (24, 10.4), (22, 8)], "#", only=".")
fill_poly([(40, 2), (58, 2), (59, 4.6), (55, 7.0), (50, 8.4), (45, 8.0), (41, 6.0)], "#", only=".")
# caldera rim rocks
fill_poly([(57, 2), (66, 1), (65, 4.6), (61, 6.6), (58, 5.4)], "#", only=".")
fill_poly([(76, 1), (86, 2), (86, 10), (83, 10.6), (79.5, 6.5), (77, 4)], "#", only=".")
# field outcrops (west)
fill_ellipse(14.5, 23.5, 2.4, 1.8, "#", only=".")
fill_ellipse(6.5, 31.5, 2.2, 1.6, "#", only=".")
fill_ellipse(5, 39.5, 1.6, 1.2, "#", only=".")
fill_ellipse(16, 38.5, 1.7, 1.2, "#", only=".")
# east: rock A frames the ruins' east exit, rock B forces the cliff path along the bay
fill_poly([(49.5, 20.5), (55, 20), (57, 22.5), (56.5, 26), (53, 27.2), (50.5, 25)], "#", only=".")
fill_poly([(60, 22.5), (66, 22), (68.5, 25), (68, 30.5), (64, 32.4), (60.5, 30), (59.5, 26)], "#", only=".")
fill_ellipse(57.5, 41, 2.4, 1.6, "#", only=".")
fill_ellipse(70.5, 43.6, 3.0, 1.5, "#", only=".")
fill_ellipse(81, 20, 2.4, 1.8, "#", only=".")
fill_ellipse(46, 43.8, 1.7, 1.2, "#", only=".")

# ---------------------------------------------------------------- lava
# caldera lake (horseshoe around the dais)
fill_ellipse(71, 10.6, 11.2, 6.4, "~", only=".#")
river = [
    (61, 11.5, 2.2), (55, 12.6, 1.8), (48, 14.0, 1.6), (41, 15.4, 1.7), (34, 16.8, 1.6), (28.5, 18.6, 1.7),
    (25, 22, 1.8), (23.4, 27, 1.6), (22.6, 32, 1.7), (21.6, 37, 1.7), (20.5, 42, 1.8), (20, 47.5, 1.9),
]
thick_line(river, "~", only=".#,")
# obsidian-lined lava canal through the ruins, feeding river A
rect(23, 37, 45, 38, "~", only=".")
# small seep pool in the scorch
fill_ellipse(62, 37.4, 2.3, 1.4, "~", only=".")

# ---------------------------------------------------------------- dais (raised obsidian, in the lake)
fill_ellipse(71, 8.4, 5.0, 4.4, "^", only="~.#")
rect(69, 1, 73, 4, "^")
rect(70, 13, 72, 17, "=", only="~")
rect(70, 12, 72, 12, "s")

# ---------------------------------------------------------------- ruins courtyard (obsidian)
rect(29, 22, 47, 35, ":", only=".")
rect(29, 21, 36, 22, "O")
rect(40, 21, 47, 22, "O")
rect(29, 23, 30, 24, "O")
rect(29, 31, 30, 33, "O")
rect(46, 30, 47, 34, "O")
rect(33, 34, 36, 35, "O")
rect(41, 34, 44, 35, "O")
# terrace south of the canal
rect(27, 39, 44, 42, ":", only=".")
# small bridge over the canal
rect(34, 37, 35, 38, "=")

# ---------------------------------------------------------------- scorched grass
fill_poly([(4, 24), (9, 22.5), (12, 26), (11, 30), (6, 29.5), (3.5, 27)], ",", only=".")
fill_poly([(9, 33), (15, 31.5), (18.5, 33.5), (17, 36), (12, 36.5), (8.5, 35.5)], ",", only=".")
fill_poly([(4, 19), (8, 18.4), (11, 19.6), (9, 21.4), (4.5, 21.2)], ",", only=".")
fill_poly([(50, 31), (56, 30.5), (58.5, 33), (56, 35.4), (51, 35)], ",", only=".")
fill_poly([(66, 37.5), (72, 36.5), (74, 39.5), (70, 41.4), (66.5, 40.5)], ",", only=".")
fill_poly([(48, 40.5), (54, 40), (55, 44), (50, 44.6), (47.5, 43)], ",", only=".")
fill_poly([(68, 20.5), (76, 20), (77.5, 22.4), (72, 23.6), (67.5, 22.6)], ",", only=".")
# across the river: unreachable meadow (scenery)
fill_poly([(36, 10.5), (44, 9.6), (47, 11.4), (40, 13.4), (35, 13)], ",", only=".")

# ---------------------------------------------------------------- paths
thick_line([(8, 44.5, 0.9), (9, 40, 0.9), (11, 35, 0.8), (12.5, 30.5, 0.8), (15, 27.6, 0.8), (19, 27.4, 1.0), (21, 27.5, 1.2)], "-", only=".,")
thick_line([(26, 27.5, 1.0), (29, 27.5, 1.0)], "-", only=".")
thick_line([(48, 27.5, 0.9), (52, 30, 0.9), (58, 33.5, 0.9), (66, 34.5, 0.9), (71.5, 31.5, 0.9), (72.5, 26, 0.8), (71.5, 21, 0.9), (71, 18, 1.0)], "-", only=".,")
# land bridge over river A (natural basalt arch)
rect(21, 26, 26, 28, "b", only="~.-")

# ---------------------------------------------------------------- cleanup
# seal one-tile ground slivers inside rock, and the pockets beside the dais
for _ in range(3):
    for y in range(1, H - 1):
        for x in range(1, W - 1):
            if g[y][x] in ".,":
                n = sum(g[y + dy][x + dx] == "#" for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
                if n >= 3:
                    g[y][x] = "#"
for y in range(0, 6):
    for x in range(60, 81):
        if g[y][x] == ".":
            g[y][x] = "#"
# the canal: straight obsidian-lined lava ('c')
for y in (37, 38):
    for x in range(25, 46):
        if g[y][x] == "~":
            g[y][x] = "c"

TOP = 3
g[:] = [[" "] * W for _ in range(TOP)] + g[: H - TOP]

if __name__ == "__main__":
    rows = ["".join(r) for r in g]
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "map.txt"), "w") as f:
        f.write("\n".join(r.rstrip() for r in rows).rstrip() + "\n")
    rs = os.path.join(here, "..", "..", "src", "regions", "cinder", "map.rs")
    with open(rs, "w") as f:
        f.write("//! Generated by tools/cinder/map.py. Don't edit by hand: change the script and rerun it.\n")
        f.write("//! Legend: ' ' void, '.' ash, ',' scorched grass, '-' path, ':' obsidian paving, '#' basalt,\n")
        f.write("//! 'O' obsidian masonry, '~' lava, 'c' canal lava, '=' bridge, '^' dais, 's' stairs.\n\n")
        f.write(f"pub const MAP: [&str; {H}] = [\n")
        for r in rows:
            f.write(f'    "{r}",\n')
        f.write("];\n")
    for r in rows:
        print(r)
    pal = {
        " ": (24, 10, 40), ".": (70, 48, 62), ",": (150, 80, 40), "-": (110, 80, 92), ":": (50, 40, 90),
        "#": (40, 30, 30), "O": (20, 14, 40), "~": (255, 106, 28), "c": (255, 150, 60), "=": (120, 100, 160), "^": (90, 80, 150),
        "s": (160, 150, 200), "b": (110, 80, 92),
    }
    s = 8
    im = Image.new("RGB", (W * s, H * s))
    px = im.load()
    for y in range(H):
        for x in range(W):
            c = pal.get(g[y][x], (255, 0, 255))
            for oy in range(s):
                for ox in range(s):
                    px[x * s + ox, y * s + oy] = c
    im.save(os.path.join(os.path.dirname(os.path.abspath(__file__)), "map_preview.png"))
