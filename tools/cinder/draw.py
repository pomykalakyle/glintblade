"""Char-canvas drawing helpers shared by the prop scripts."""

import numpy as np

from lib import LEGEND, hex2rgb


class Canvas:
    def __init__(self, w, h, fill="."):
        """Initialize the native pixel drawing state."""
        self.w, self.h = w, h
        self.a = np.full((h, w), fill, dtype="<U1")

    def copy(self):
        """Return an independent copy of the pixel canvas."""
        c = Canvas(self.w, self.h)
        c.a = self.a.copy()
        return c

    def get(self, x, y):
        """Return the requested native pixel or transparent outside the canvas."""
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.a[y, x]
        return "."

    def set(self, x, y, ch):
        """Set one native pixel inside the canvas."""
        if 0 <= x < self.w and 0 <= y < self.h:
            self.a[y, x] = ch

    def hline(self, x0, x1, y, ch):
        """Draw an inclusive horizontal segment."""
        for x in range(x0, x1 + 1):
            self.set(x, y, ch)

    def vline(self, x, y0, y1, ch):
        """Draw an inclusive vertical segment."""
        for y in range(y0, y1 + 1):
            self.set(x, y, ch)

    def rect(self, x0, y0, x1, y1, ch):
        """Fill an inclusive rectangle on the native grid."""
        for y in range(y0, y1 + 1):
            self.hline(x0, x1, y, ch)

    def stamp(self, rows, x, y, only_on=None):
        """'.' keeps the canvas, ',' erases to transparent. Rows can be a multiline string (common indent stripped)."""
        if isinstance(rows, str):
            rows = rows.strip("\n").split("\n")
            ind = min(len(r) - len(r.lstrip()) for r in rows if r.strip())
            rows = [r[ind:] for r in rows]
        for j, r in enumerate(rows):
            for i, ch in enumerate(r):
                if ch == "." or ch == " ":
                    continue
                if only_on is not None and self.get(x + i, y + j) not in only_on:
                    continue
                self.set(x + i, y + j, "." if ch == "," else ch)

    def replace(self, frm, to, region=None):
        """Draw the replace on the native Cinder pixel grid."""
        x0, y0, x1, y1 = region or (0, 0, self.w - 1, self.h - 1)
        sub = self.a[y0:y1 + 1, x0:x1 + 1]
        for f, t in zip(frm, to):
            sub[sub == f] = t

    def opaque(self):
        """Return the opaque representation for the Cinder generator."""
        return self.a != "."

    def outline(self, ch="K", sides="all", diag=False):
        """Add a 1-px outline outside the opaque shape. sides: 'all' or a subset of 'lrtb'."""
        m = self.opaque()
        h, w = m.shape
        add = np.zeros_like(m)
        dirs = []
        if sides == "all":
            sides = "lrtb"
        if "l" in sides:
            dirs.append((0, 1))   # pixel left of shape: its right neighbor is opaque
        if "r" in sides:
            dirs.append((0, -1))
        if "t" in sides:
            dirs.append((1, 0))
        if "b" in sides:
            dirs.append((-1, 0))
        if diag:
            dirs += [(1, 1), (1, -1), (-1, 1), (-1, -1)]
        for dy, dx in dirs:
            sh = np.zeros_like(m)
            ys = slice(max(0, -dy), h - max(0, dy))
            yd = slice(max(0, dy), h - max(0, -dy))
            xs = slice(max(0, -dx), w - max(0, dx))
            xd = slice(max(0, dx), w - max(0, -dx))
            sh[ys, xs] = m[yd, xd]
            add |= sh & ~m
        self.a[add] = ch
        return self

    def rgba(self, legend=None):
        """Convert the authored character grid to hard-alpha RGBA pixels."""
        lg = dict(LEGEND)
        if legend:
            lg.update(legend)
        out = np.zeros((self.h, self.w, 4), np.uint8)
        for y in range(self.h):
            for x in range(self.w):
                c = lg[self.a[y, x]]
                if c:
                    out[y, x, :3] = hex2rgb(c)
                    out[y, x, 3] = 255
        return out

    def rows(self):
        """Return the character rows of the native canvas."""
        return ["".join(r) for r in self.a]


def from_rows(rows, w=None, h=None):
    """Draw the from rows on the native Cinder pixel grid."""
    if isinstance(rows, str):
        rows = rows.strip("\n").split("\n")
        ind = min(len(r) - len(r.lstrip()) for r in rows if r.strip())
        rows = [r[ind:] for r in rows]
    W = w or max(len(r) for r in rows)
    H = h or len(rows)
    c = Canvas(W, H)
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch != " ":
                c.set(x, y, ch)
    return c


def place(dst, src, x, y):
    """Paste canvas src onto dst at x, y (src '.' is transparent)."""
    for j in range(src.h):
        for i in range(src.w):
            ch = src.a[j, i]
            if ch != ".":
                dst.set(x + i, y + j, ch)
    return dst


def hflip(c):
    """Mirror a character canvas without resampling."""
    out = c.copy()
    out.a = c.a[:, ::-1].copy()
    return out


def _inside(px, py, pts):
    """Test whether a sample lies inside a polygon."""
    n = len(pts)
    ins = False
    j = n - 1
    for i in range(n):
        xi, yi = pts[i]
        xj, yj = pts[j]
        if (yi > py) != (yj > py):
            xc = xi + (py - yi) * (xj - xi) / (yj - yi)
            if px < xc:
                ins = not ins
        j = i
    return ins


def poly(c, pts, ch, only_on=None):
    """Fill a polygon (pixel-center sampling) with ch."""
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    for y in range(int(min(ys)) - 1, int(max(ys)) + 2):
        for x in range(int(min(xs)) - 1, int(max(xs)) + 2):
            if _inside(x + 0.5, y + 0.5, pts):
                if only_on is None or c.get(x, y) in only_on:
                    c.set(x, y, ch)


def line(c, pts, ch, only_on=None):
    """Bresenham polyline."""
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        dx, dy = abs(x1 - x0), -abs(y1 - y0)
        sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
        err = dx + dy
        x, y = x0, y0
        while True:
            if only_on is None or c.get(x, y) in only_on:
                c.set(x, y, ch)
            if x == x1 and y == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x += sx
            if e2 <= dx:
                err += dx
                y += sy


def ellipse(c, cx, cy, rx, ry, ch, only_on=None):
    """Draw the ellipse on the native Cinder pixel grid."""
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            if ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1.0:
                if only_on is None or c.get(x, y) in only_on:
                    c.set(x, y, ch)
