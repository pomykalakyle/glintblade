"""Plate-based shading: draw stone plates back to front into an id map, then shade each pixel
from its plate's base tone with consistent edge rules (light from the upper left):

- top edge (pixel above is empty or a plate behind):   tone + top_hi
- pixel above belongs to a plate in front (overhang):   tone - 1 (cast shadow), for `cast` rows
- bottom edge (pixel below empty or behind):            tone - bot_lo
- right edge on shadow plates:                          tone - 1
Tones index the stone ramp S0..S5; values are clamped.
"""

import math

STONE = ["S0", "S1", "S2", "S3", "S4", "S5"]


class Plate:
    def __init__(self, base, top_hi=1, bot_lo=1, cast=1, left_hi=0, right_lo=0, ramp=None, flat=False):
        self.base = base
        self.top_hi = top_hi
        self.bot_lo = bot_lo
        self.cast = cast
        self.left_hi = left_hi
        self.right_lo = right_lo
        self.ramp = ramp or STONE
        self.flat = flat


class PlateMap:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.id = [[-1] * w for _ in range(h)]
        self.plates = []

    def add(self, pts, plate, clip=None):
        """Fill polygon (pixel-center sampling) with a new plate id. `clip`: only over these ids."""
        k = len(self.plates)
        self.plates.append(plate)
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
                    if clip is None or self.id[y][x] in clip:
                        self.id[y][x] = k
        return k

    def add_pixels(self, pix, plate):
        k = len(self.plates)
        self.plates.append(plate)
        for x, y in pix:
            if 0 <= x < self.w and 0 <= y < self.h:
                self.id[y][x] = k
        return k

    def ident(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.id[y][x]
        return -1

    def render(self, cv, offset=None):
        """offset(x, y) -> int tone offset (form shadows), applied after edge rules."""
        for y in range(self.h):
            for x in range(self.w):
                k = self.id[y][x]
                if k < 0:
                    continue
                p = self.plates[k]
                t = p.base
                if not p.flat:
                    up = self.ident(x, y - 1)
                    up2 = self.ident(x, y - 2)
                    dn = self.ident(x, y + 1)
                    lf = self.ident(x - 1, y)
                    rt = self.ident(x + 1, y)
                    if up != k and up > k and p.cast:
                        t -= 1
                    elif up2 != k and up2 > k and p.cast >= 2:
                        t -= 1
                    elif up != k:
                        t += p.top_hi
                    elif dn != k and dn < k:
                        t -= p.bot_lo
                    if lf != k and lf < k and p.left_hi:
                        t += p.left_hi if up == k else 0
                    if rt != k and rt < k and p.right_lo:
                        t -= p.right_lo
                if offset is not None and not p.flat:
                    t += offset(x, y)
                t = max(0, min(len(p.ramp) - 1, t))
                cv.put(x, y, p.ramp[t])
