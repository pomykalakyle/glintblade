"""GLINTBLADE logo letterforms: Roman capitals with thick/thin contrast and bracketed serifs.

Built from stems, hairlines, serifs and elliptical bowls on the pixel grid (no anti-aliasing),
then hand-patched where the math leaves an uneven step. Shading/outline/extrusion and the
sword "I" live in build_logo.py.
"""

import numpy as np


def from_rows(rows):
    w = max(len(r) for r in rows)
    m = np.zeros((len(rows), w), bool)
    for y, r in enumerate(rows):
        for x, c in enumerate(r):
            m[y, x] = c == "#"
    return m


class M:
    def __init__(self, w, h):
        self.m = np.zeros((h, w), bool)
        self.w, self.h = w, h

    def rect(self, x0, y0, x1, y1, v=True):
        x0, x1 = sorted((x0, x1))
        y0, y1 = sorted((y0, y1))
        self.m[max(y0, 0):min(y1, self.h - 1) + 1, max(x0, 0):min(x1, self.w - 1) + 1] = v

    def px(self, x, y, v=True):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.m[y, x] = v

    def ring(self, cx, cy, rx, ry, rxi, ryi, x_min=None, x_max=None, y_min=None, y_max=None):
        """Elliptical ring: inside (rx, ry) and outside (rxi, ryi), pixel-center test.
        Optional clipping box."""
        for y in range(self.h):
            for x in range(self.w):
                if x_min is not None and x < x_min or x_max is not None and x > x_max:
                    continue
                if y_min is not None and y < y_min or y_max is not None and y > y_max:
                    continue
                dx, dy = x + 0.5 - cx, y + 0.5 - cy
                o = (dx / rx) ** 2 + (dy / ry) ** 2 <= 1.0
                i = rxi > 0 and ryi > 0 and (dx / rxi) ** 2 + (dy / ryi) ** 2 <= 1.0
                if o and not i:
                    self.m[y, x] = True

    def diag(self, x_top, y0, y1, step, width, direction):
        """Diagonal band: at row y the band starts at x_top + direction*((y-y0)//step), `width` wide."""
        for y in range(y0, y1 + 1):
            x = x_top + direction * ((y - y0) // step)
            self.rect(x, y, x + width - 1, y)

    def serif_bottom(self, x0, x1, b, left=3, right=3):
        """Bracketed foot serif for a stem spanning cols x0..x1 ending at row b."""
        self.rect(x0 - left, b, x1 + right, b)
        self.rect(x0 - min(left, 1), b - 1, x1 + min(right, 1), b - 1)

    def serif_top(self, x0, x1, t=0, left=3, right=3):
        self.rect(x0 - left, t, x1 + right, t)
        self.rect(x0 - min(left, 1), t + 1, x1 + min(right, 1), t + 1)


def letters(H=26, S=5, t=2, e=3):
    """Big logo letters. H cap height, S thick stem, t hairline, e serif reach."""
    b = H - 1
    L = {}

    # ---- L
    w = e + S + 10
    m = M(w, H)
    x0 = e
    m.rect(x0, 0, x0 + S - 1, b)
    m.serif_top(x0, x0 + S - 1)
    m.serif_bottom(x0, x0 + S - 1, b, right=0)
    m.rect(x0, b - t + 1, w - 1, b)  # foot hairline
    m.rect(w - 2, b - 6, w - 1, b)   # beak rising at the end
    m.px(w - 3, b - 2)
    L["L"] = m.m

    # ---- E
    w = e + S + 11
    m = M(w, H)
    m.rect(x0, 0, x0 + S - 1, b)
    m.serif_top(x0, x0 + S - 1, right=0)
    m.serif_bottom(x0, x0 + S - 1, b, right=0)
    m.rect(x0, 0, w - 1, t - 1)
    m.rect(w - 2, 0, w - 1, 5)
    m.px(w - 3, 2)
    m.rect(x0, b - t + 1, w - 1, b)
    m.rect(w - 2, b - 6, w - 1, b)
    m.px(w - 3, b - 2)
    my = H // 2 - 1
    m.rect(x0, my, w - 4, my + t - 1)
    m.rect(w - 5, my - 2, w - 4, my + t + 1)
    L["E"] = m.m

    # ---- T
    w = 22
    m = M(w, H)
    sx = (w - S) // 2
    m.rect(0, 0, w - 1, t - 1)
    m.rect(0, 0, 1, 6)
    m.px(2, 2)
    m.rect(w - 2, 0, w - 1, 6)
    m.px(w - 3, 2)
    m.rect(sx, 0, sx + S - 1, b)
    m.serif_bottom(sx, sx + S - 1, b)
    L["T"] = m.m

    # ---- N: thin stems, thick diagonal
    w = 24
    m = M(w, H)
    lx, rx = e, w - e - t
    m.rect(lx, 0, lx + t - 1, b)
    m.rect(rx, 0, rx + t - 1, b)
    # diagonal from top-left (starting at lx) to bottom-right (ending at rx+t-1)
    D = S + 1
    span = (rx + t - 1) - (lx + D - 1)
    for y in range(H):
        x = lx + (y * span) // b
        m.rect(x, y, x + D - 1, y)
    m.rect(lx - e, 0, lx + 1, 0)           # top-left serif (diagonal starts here)
    m.serif_bottom(lx, lx + t - 1, b)
    m.serif_top(rx, rx + t - 1)
    L["N"] = m.m

    # ---- A: thin left, thick right, pointed apex with a small left serif
    w = 26
    m = M(w, H)
    apex = w // 2
    # right thick stroke: 1:3 slope
    for y in range(H):
        xr = apex - 1 + y // 3
        m.rect(xr, y, xr + S - 1 - (2 if y < 2 else 0), y)
        xl = apex - 1 - (y + 1) // 3
        m.rect(xl, y, xl + t - 1, y)
    m.rect(apex - 3, 0, apex, 0)  # apex serif to the left
    cb = H * 2 // 3 - 1
    xl = apex - 1 - (cb + 1) // 3
    xr = apex - 1 + cb // 3
    m.rect(xl, cb, xr, cb + t - 1)
    # feet
    xl = apex - 1 - (b + 1) // 3
    xr = apex - 1 + b // 3
    m.serif_bottom(xl, xl + t - 1, b)
    m.serif_bottom(xr, xr + S - 1, b)
    L["A"] = m.m

    # ---- D: stem + half ring
    w = 25
    m = M(w, H)
    m.rect(x0, 0, x0 + S - 1, b)
    m.serif_top(x0, x0 + S - 1, right=0)
    m.serif_bottom(x0, x0 + S - 1, b, right=0)
    bx = x0 + S + 2  # where the bowl's straight hairlines turn into the curve
    m.rect(x0, 0, bx, t - 1)
    m.rect(x0, b - t + 1, bx, b)
    m.ring(bx, H / 2, w - bx, H / 2, w - bx - S, H / 2 - t, x_min=bx + 1)
    L["D"] = m.m

    # ---- B: stem + two bowls (upper smaller)
    w = 22
    m = M(w, H)
    m.rect(x0, 0, x0 + S - 1, b)
    m.serif_top(x0, x0 + S - 1, right=0)
    m.serif_bottom(x0, x0 + S - 1, b, right=0)
    mid = H // 2 - 1  # middle hairline rows mid..mid+t-1
    bx = x0 + S + 1
    # upper bowl rows 0..mid+t-1
    uh = (mid + t) / 2
    m.rect(x0, 0, bx, t - 1)
    m.rect(x0, mid, bx, mid + t - 1)
    m.ring(bx, uh, w - 3 - bx, uh, w - 3 - bx - S + 1, uh - t, x_min=bx + 1)
    # lower bowl rows mid..b
    lh = (H - mid) / 2
    m.rect(x0, b - t + 1, bx, b)
    m.ring(bx, mid + lh, w - bx, lh, w - bx - S, lh - t, x_min=bx + 1)
    L["B"] = m.m

    # ---- G: hand-authored from the O ring (24 x 26)
    L["G"] = from_rows([
        ".........######.........",
        "......############......",
        ".....#####....######....",
        "....####.........####...",
        "...#####..........###...",
        "..#####...........###...",
        "..####............###...",
        ".#####.............##...",
        ".#####..................",
        "#####...................",
        "#####...................",
        "#####...................",
        "#####...................",
        "#####............#######",
        "#####.............#####.",
        "#####.............#####.",
        "#####.............#####.",
        ".#####............#####.",
        ".#####............#####.",
        "..####............####..",
        "..#####..........#####..",
        "...#####........#####...",
        "....####........####....",
        ".....#####....#####.....",
        "......############......",
        ".........######.........",
    ])
    return L


def letters_small(H=13):
    """Small-logo letters, hand-authored (shapes match the big logo; no scaling)."""
    rows = {
        "G": ["...#####.",
              ".##....#.",
              "##.....#.",
              "##.......",
              "##.......",
              "##.......",
              "##...####",
              "##....##.",
              "##....##.",
              "##....##.",
              "##....##.",
              ".##..##..",
              "..####..."],
        "L": ["####....",
              ".##.....",
              ".##.....",
              ".##.....",
              ".##.....",
              ".##.....",
              ".##.....",
              ".##.....",
              ".##.....",
              ".##.....",
              ".##....#",
              ".##....#",
              "########"],
        "N": ["###.....###",
              ".##......#.",
              ".###.....#.",
              ".###.....#.",
              ".#.##....#.",
              ".#.##....#.",
              ".#..##...#.",
              ".#..##...#.",
              ".#...##..#.",
              ".#...##..#.",
              ".#....##.#.",
              ".#....##.#.",
              "###....###."],
        "T": ["##########",
              "#...##...#",
              "....##....",
              "....##....",
              "....##....",
              "....##....",
              "....##....",
              "....##....",
              "....##....",
              "....##....",
              "....##....",
              "....##....",
              "...####..."],
        "B": ["#######...",
              ".##....##.",
              ".##.....##",
              ".##.....##",
              ".##....##.",
              ".######...",
              ".##....##.",
              ".##.....##",
              ".##.....##",
              ".##.....##",
              ".##.....##",
              ".##....##.",
              "#######..."],
        "A": [".....##......",
              ".....#.##....",
              "....#..##....",
              "....#..##....",
              "....#...##...",
              "...#....##...",
              "...#....##...",
              "...#.....##..",
              "..#########..",
              "..#......##..",
              "..#.......##.",
              ".#........##.",
              "###......####"],
        "D": ["#######....",
              ".##....##..",
              ".##.....##.",
              ".##......##",
              ".##......##",
              ".##......##",
              ".##......##",
              ".##......##",
              ".##......##",
              ".##......##",
              ".##.....##.",
              ".##....##..",
              "#######...."],
        "E": ["#########",
              ".##.....#",
              ".##.....#",
              ".##......",
              ".##......",
              ".##...#..",
              ".######..",
              ".##...#..",
              ".##......",
              ".##......",
              ".##.....#",
              ".##.....#",
              "#########"],
    }
    out = {}
    for k, r in rows.items():
        w = max(len(x) for x in r)
        m = np.zeros((len(r), w), bool)
        for y, row in enumerate(r):
            for x, c in enumerate(row):
                m[y, x] = c == "#"
        out[k] = m
    return out
