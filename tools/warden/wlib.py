"""Tiny palette-keyed pixel canvas for the Warden sprites.

Every pixel is a palette key (str) or None (transparent). Shapes are rasterized by sampling
pixel centers, so integer vertices give clean, predictable stair steps. Output PNGs are fully
opaque or fully transparent.
"""

import json
import math

import numpy as np
from PIL import Image

# ------------------------------------------------------------------ palette
# Style v2 (assets/concept/palettes_v2.json, regions.warden): blue-gray stone and one iris prism.
PAL = {
    "OL": "#1f2838",  # summit outline
    "D0": "#2b3548",  # deep recess (summit shadow tint)
    # warden_stone ramp, dark to light
    "S0": "#333b4c",
    "S1": "#555d70",
    "S2": "#7f8797",
    "S3": "#b0b7bb",
    "S4": "#d9dacc",
    # prism ramp (iris), dark to light; P3 is the heart's light color
    "P0": "#564d6e",
    "P1": "#8d7fa8",
    "P2": "#b7add0",
    "P3": "#e2d8e9",
    # moss ramp (dark two steps): only where water pooled on the old weir
    "M0": "#3e5258",
    "M1": "#627975",
}

EMISSIVE = ["P3", "P2"]


def hex_rgb(h):
    """Decode a hex swatch into RGB bytes."""
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


class Canvas:
    def __init__(self, w, h):
        """Create a transparent palette-keyed canvas."""
        self.w, self.h = w, h
        self.px = [[None] * w for _ in range(h)]

    # ---------------------------------------------------------- basics
    def inb(self, x, y):
        """Return whether the pixel lies within the canvas."""
        return 0 <= x < self.w and 0 <= y < self.h

    def put(self, x, y, c):
        """Place a palette key at a clipped canvas coordinate."""
        if self.inb(x, y):
            self.px[y][x] = c

    def get(self, x, y):
        """Read a palette key, treating the exterior as transparent."""
        return self.px[y][x] if self.inb(x, y) else None

    def copy(self):
        """Copy the canvas without sharing its pixel rows."""
        c = Canvas(self.w, self.h)
        c.px = [row[:] for row in self.px]
        return c

    def rect(self, x0, y0, x1, y1, c, only=None):
        """Inclusive rectangle. `only`: restrict to pixels whose current key is in this set."""
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                if only is None or self.get(x, y) in only:
                    self.put(x, y, c)

    def hline(self, x0, x1, y, c, only=None):
        """Draw an inclusive horizontal line."""
        self.rect(min(x0, x1), y, max(x0, x1), y, c, only)

    def vline(self, x, y0, y1, c, only=None):
        """Draw an inclusive vertical line."""
        self.rect(x, min(y0, y1), x, max(y0, y1), c, only)

    def pts(self, pts, c, only=None):
        """Place a list of pixels in one palette color."""
        for x, y in pts:
            if only is None or self.get(x, y) in only:
                self.put(x, y, c)

    def line(self, x0, y0, x1, y1, c, only=None):
        """Draw a native-grid line using integer rasterization."""
        dx, dy = abs(x1 - x0), -abs(y1 - y0)
        sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
        err = dx + dy
        while True:
            if only is None or self.get(x0, y0) in only:
                self.put(x0, y0, c)
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x0 += sx
            if e2 <= dx:
                err += dx
                y0 += sy

    def path(self, pts, c, only=None):
        """Connect consecutive points with native-grid line segments."""
        for (a, b), (d, e) in zip(pts, pts[1:]):
            self.line(a, b, d, e, c, only)

    def poly(self, pts, c, only=None):
        """Fill polygon by sampling pixel centers (x+0.5, y+0.5)."""
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        n = len(pts)
        for y in range(max(0, int(min(ys))), min(self.h, int(math.ceil(max(ys))) + 1)):
            py = y + 0.5
            nodes = []
            for i in range(n):
                x0, y0 = pts[i]
                x1, y1 = pts[(i + 1) % n]
                if (y0 <= py < y1) or (y1 <= py < y0):
                    nodes.append(x0 + (py - y0) / (y1 - y0) * (x1 - x0))
            nodes.sort()
            for a, b in zip(nodes[::2], nodes[1::2]):
                for x in range(max(0, int(math.ceil(a - 0.5))), min(self.w, int(math.floor(b - 0.5)) + 1)):
                    if only is None or self.px[y][x] in only:
                        self.px[y][x] = c

    def ellipse(self, cx, cy, rx, ry, c, only=None):
        """Filled ellipse sampled at pixel centers; cx, cy may be half-integers."""
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                dx = (x + 0.5 - cx) / rx
                dy = (y + 0.5 - cy) / ry
                if dx * dx + dy * dy <= 1.0:
                    if only is None or self.get(x, y) in only:
                        self.put(x, y, c)

    def stamp(self, x0, y0, rows, mapping, mirror=False):
        """Stamp a char grid. Chars missing from `mapping` (and ' ') are skipped."""
        for j, row in enumerate(rows):
            if mirror:
                row = row[::-1]
            for i, ch in enumerate(row):
                if ch in mapping and mapping[ch] is not ...:
                    self.put(x0 + i, y0 + j, mapping[ch])

    def replace(self, frm, to, box=None):
        """Replace selected palette keys inside an optional rectangle."""
        x0, y0, x1, y1 = box if box else (0, 0, self.w - 1, self.h - 1)
        frm = set(frm) if isinstance(frm, (list, tuple, set)) else {frm}
        for y in range(max(0, y0), min(self.h, y1 + 1)):
            for x in range(max(0, x0), min(self.w, x1 + 1)):
                if self.px[y][x] in frm:
                    self.px[y][x] = to

    def opaque(self, x, y):
        """Return whether a canvas pixel is opaque."""
        return self.get(x, y) is not None

    def outline(self, c="OL", diag=False):
        """Outer outline on transparent pixels touching opaque ones."""
        add = []
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y][x] is not None:
                    continue
                nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                if diag:
                    nb += [(1, 1), (-1, 1), (1, -1), (-1, -1)]
                if any(self.opaque(x + dx, y + dy) and self.get(x + dx, y + dy) != c for dx, dy in nb):
                    add.append((x, y))
        for x, y in add:
            self.px[y][x] = c

    def blit(self, other, ox, oy, mirror=False):
        """Copy opaque pixels from another canvas with optional horizontal mirroring."""
        for y in range(other.h):
            for x in range(other.w):
                v = other.px[y][other.w - 1 - x] if mirror else other.px[y][x]
                if v is not None:
                    self.put(ox + x, oy + y, v)

    def flipped(self):
        """Return a horizontally mirrored canvas."""
        c = Canvas(self.w, self.h)
        c.px = [row[::-1] for row in self.px]
        return c

    def shifted(self, dx, dy):
        """Return a translated canvas clipped to its original dimensions."""
        c = Canvas(self.w, self.h)
        c.blit(self, dx, dy)
        return c

    def keys(self):
        """Collect the visible palette keys in the canvas."""
        s = set()
        for row in self.px:
            s.update(v for v in row if v is not None)
        return s

    # ---------------------------------------------------------- output
    def rgba(self):
        """Resolve palette keys into a binary-alpha RGBA array."""
        a = np.zeros((self.h, self.w, 4), np.uint8)
        for y in range(self.h):
            for x in range(self.w):
                v = self.px[y][x]
                if v is not None:
                    r, g, b = hex_rgb(PAL[v])
                    a[y, x] = (r, g, b, 255)
        return a

    def image(self):
        """Return the canvas as a Pillow RGBA image."""
        return Image.fromarray(self.rgba(), "RGBA")


def sheet(rows, cell):
    """rows: list of lists of Canvas (each cell-sized). Returns RGBA Image."""
    cw, ch = cell
    ncol = max(len(r) for r in rows)
    img = Image.new("RGBA", (cw * ncol, ch * len(rows)), (0, 0, 0, 0))
    for j, r in enumerate(rows):
        for i, cv in enumerate(r):
            assert (cv.w, cv.h) == (cw, ch), (cv.w, cv.h, cell)
            img.paste(cv.image(), (i * cw, j * ch))
    return img


def used_hex(rows):
    """Collect the palette keys used by these animation rows."""
    ks = set()
    for r in rows:
        for cv in r:
            ks |= cv.keys()
    return ks


def emissive_hex(keys):
    """Resolve the emissive palette keys used by a sheet."""
    return [PAL[k] for k in EMISSIVE if k in keys]


def write_json(path, obj):
    """Write readable deterministic JSON with a trailing newline."""
    with open(path, "w") as f:
        json.dump(obj, f, indent=2)
        f.write("\n")


# ------------------------------------------------------------------ previews
def on_bg(img, bg, scale, pad=0):
    """Composite a sprite over a solid background and enlarge with nearest-neighbor pixels."""
    w, h = img.size
    base = Image.new("RGBA", (w + 2 * pad, h + 2 * pad), hex_rgb(bg) + (255,))
    base.alpha_composite(img, (pad, pad))
    return base.resize(((w + 2 * pad) * scale, (h + 2 * pad) * scale), Image.NEAREST)


def gif(frames, path, durations_ms, bg=None):
    """frames: list of RGBA images (same size). Writes a looping GIF with nearest scaling done by caller."""
    pal_frames = []
    for f in frames:
        if bg is not None:
            b = Image.new("RGBA", f.size, hex_rgb(bg) + (255,))
            b.alpha_composite(f)
            f = b
        pal_frames.append(f.convert("RGB").convert("P", palette=Image.ADAPTIVE, colors=255))
    pal_frames[0].save(
        path, save_all=True, append_images=pal_frames[1:], duration=durations_ms, loop=0, disposal=2, optimize=False
    )


def rrect(cv, x0, y0, x1, y1, fill, top=None, left=None, right=None, bottom=None, tl=None, cut=1, only=None):
    """Rounded rect (inclusive) with optional 1px edge colors; corners cut by `cut` px."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            dx = min(x - x0, x1 - x)
            dy = min(y - y0, y1 - y)
            if cut and dx + dy < cut:
                continue
            c = fill
            if bottom and y == y1:
                c = bottom
            elif right and x == x1:
                c = right
            elif top and y == y0:
                c = top
            elif left and x == x0:
                c = left
            if only is None or cv.get(x, y) in only:
                cv.put(x, y, c)
    if tl:
        cv.put(x0 + 1, y0, tl)
        cv.put(x0, y0 + 1, tl)


# char codes for hand-typed patch blocks
CHARS = {
    "O": "OL", "d": "D0", "0": "S0", "1": "S1", "2": "S2", "3": "S3", "4": "S4",
    "p": "P0", "q": "P1", "r": "P2", "s": "P3", "m": "M0", "n": "M1",
}


def patch(cv, x0, y0, block, mirror=0, chars=None):
    """Apply a char block (mirror = 0, or the mirror sum, e.g. 120 for x -> 120 - x): ' ' keeps the pixel, '_' clears it, other chars per CHARS.
    `block` is a multi-line string; leading/trailing blank lines are dropped, and every line
    is used as-is from column 0 (use '|' as an optional left margin marker)."""
    chars = chars or CHARS
    lines = block.split("\n")
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    for j, line in enumerate(lines):
        if "|" in line:
            line = line.split("|", 1)[1]
        for i, ch in enumerate(line):
            # mirror: x0 is the left-half origin; char i lands at its mirror image 120 - (x0 + i)
            x = (mirror - (x0 + i)) if mirror else (x0 + i)
            if ch == " " or ch == ".":
                continue
            if ch == "_":
                cv.put(x, y0 + j, None)
            elif ch in chars:
                cv.put(x, y0 + j, chars[ch])
