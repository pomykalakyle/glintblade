"""Cut-stone letters for style v2: narrow, uniform-weight capitals with 45-degree clipped corners.

One construction serves the title logo (cap 22), the small logo (cap 12) and font_display (cap 13),
so every title shares the same letter design. Letters are built on the native grid from straight
rectangles, 45-degree corner cuts and evenly stepped diagonals; nothing is scaled or smoothed.
"""

import numpy as np


class Glyph:
    """A boolean ink mask under construction, H rows by w columns."""

    def __init__(self, w, h):
        """Start an empty mask."""
        self.m = np.zeros((h, w), bool)
        self.w, self.h = w, h

    def rect(self, x0, y0, x1, y1, on=True):
        """Fill (or clear) an inclusive rectangle, clipped to the mask."""
        x0, x1 = max(0, min(x0, x1)), min(self.w - 1, max(x0, x1))
        y0, y1 = max(0, min(y0, y1)), min(self.h - 1, max(y0, y1))
        if x0 <= x1 and y0 <= y1:
            self.m[y0:y1 + 1, x0:x1 + 1] = on

    def cut(self, x, y, n, corner, on=False):
        """Clear a 45-degree triangle of leg n whose right angle sits at corner pixel (x, y)."""
        sx = 1 if corner[1] == "l" else -1
        sy = 1 if corner[0] == "t" else -1
        for j in range(n):
            for i in range(n - j):
                px, py = x + sx * i, y + sy * j
                if 0 <= px < self.w and 0 <= py < self.h:
                    self.m[py, px] = on

    def diag(self, x0, y0, x1, y1, t):
        """A stroke t pixels wide per row from (x0, y0) to (x1, y1), x0/x1 being left edges."""
        n = abs(y1 - y0)
        for k in range(n + 1):
            y = y0 + (k if y1 >= y0 else -k)
            x = x0 + (x1 - x0) * k / n if n else x0
            xi = int(np.floor(x + 0.5))
            self.rect(xi, y, xi + t - 1, y)


def _bowl(g, x0, y0, x1, y1, s, t, c, cuts="tl tr bl br"):
    """A rectangular ring with stem s, bars t and the listed outer corners clipped by c."""
    g.rect(x0, y0, x1, y1)
    g.rect(x0 + s, y0 + t, x1 - s, y1 - t, on=False)
    for k in cuts.split():
        x = x0 if k[1] == "l" else x1
        y = y0 if k[0] == "t" else y1
        g.cut(x, y, c, k)
        # keep the inner corner square so the stroke stays a constant weight
    return g


def letter(ch, H, s, t, N, c):
    """Build capital `ch`: cap height H, stems s, bars t, normal width N, corner cut c."""
    mid = (H - t) // 2           # top row of a middle bar
    W = {"I": s, "T": N + (1 if (N - s) % 2 else 0), "M": N + 2 * s - 1, "W": N + 2 * s - 1,
         "J": N - 1, "L": N - 1, "F": N - 1, "E": N - 1, "1": N - 2,
         "V": N + (1 if (N - s) % 2 else 0),
         "Y": N + (1 if (N - s) % 2 else 0), "X": N, "K": N}.get(ch, N)
    g = Glyph(W, H)
    R = W - 1
    B = H - 1
    if ch == "I":
        g.rect(0, 0, s - 1, B)
    elif ch == "L":
        g.rect(0, 0, s - 1, B)
        g.rect(0, B - t + 1, R, B)
    elif ch == "E":
        g.rect(0, 0, s - 1, B)
        g.rect(0, 0, R, t - 1)
        g.rect(0, mid, R - 1, mid + t - 1)
        g.rect(0, B - t + 1, R, B)
    elif ch == "F":
        g.rect(0, 0, s - 1, B)
        g.rect(0, 0, R, t - 1)
        g.rect(0, mid, R - 1, mid + t - 1)
    elif ch == "T":
        g.rect(0, 0, R, t - 1)
        x = (W - s) // 2
        g.rect(x, 0, x + s - 1, B)
    elif ch == "H":
        g.rect(0, 0, s - 1, B)
        g.rect(R - s + 1, 0, R, B)
        g.rect(0, mid, R, mid + t - 1)
    elif ch == "N":
        g.rect(0, 0, s - 1, B)
        g.rect(R - s + 1, 0, R, B)
        g.diag(0, 0, R - s + 1, B, s)
    elif ch == "M":
        g.rect(0, 0, s - 1, B)
        g.rect(R - s + 1, 0, R, B)
        cx = (W - s) // 2
        v = H * 3 // 5
        g.diag(0, 0, cx, v, s)
        g.diag(R - s + 1, 0, cx, v, s)
    elif ch == "A":
        # a flat-topped stone A: a ring with deep upper cuts, open at the foot
        _bowl(g, 0, 0, R, B + t, s, t, c + 1, "tl tr")
        g.rect(0, mid + 1, R, mid + t)
    elif ch == "V":
        cx = (W - s) // 2
        g.diag(0, 0, cx, B, s)
        g.diag(R - s + 1, 0, cx, B, s)
    elif ch == "W":
        g.rect(0, 0, s - 1, B)
        g.rect(R - s + 1, 0, R, B)
        cx = (W - s) // 2
        v = H * 2 // 5
        g.diag(0, B, cx, v, s)
        g.diag(R - s + 1, B, cx, v, s)
    elif ch == "X":
        g.diag(0, 0, R - s + 1, B, s)
        g.diag(R - s + 1, 0, 0, B, s)
    elif ch == "Y":
        cx = (W - s) // 2
        v = H // 2
        g.diag(0, 0, cx, v, s)
        g.diag(R - s + 1, 0, cx, v, s)
        g.rect(cx, v, cx + s - 1, B)
    elif ch == "K":
        g.rect(0, 0, s - 1, B)
        g.diag(R - s + 1, 0, s - 1, mid + t // 2, s)
        g.diag(s, mid + t // 2, R - s + 1, B, s)
    elif ch == "Z":
        g.rect(0, 0, R, t - 1)
        g.rect(0, B - t + 1, R, B)
        g.diag(R - s + 1, t - 1, 0, B - t + 1, s)
    elif ch == "O" or ch == "0":
        _bowl(g, 0, 0, R, B, s, t, c)
    elif ch == "Q":
        _bowl(g, 0, 0, R, B, s, t, c)
        g.cut(R, B, c, "br", on=False)
        g.rect(R - s - c + 2, B - t + 1, R, B)
        g.cut(R, B, max(1, c - 1), "br")
        g.rect(R - s + 1, B - c, R, B)
    elif ch == "C":
        _bowl(g, 0, 0, R, B, s, t, c)
        g.rect(R - s + 1, t, R, B - t, on=False)
    elif ch == "G":
        _bowl(g, 0, 0, R, B, s, t, c)
        g.rect(R - s + 1, t, R, mid - 1, on=False)
        g.rect(R - s - (W - 2 * s) // 2 + 1, mid, R, mid + t - 1)
    elif ch == "D":
        _bowl(g, 0, 0, R, B, s, t, c + 1, "tr br")
    elif ch == "U":
        _bowl(g, 0, -t, R, B, s, t, c, "bl br")
    elif ch == "J":
        _bowl(g, 0, -t, R, B, s, t, c, "bl br")
        g.rect(0, 0, s - 1, B - t - c - 1, on=False)
        g.rect(0, B - t - c - (H // 5), s - 1, B - t - c - 1)
    elif ch in "BPR":
        top_w = R - (1 if ch == "B" else 0)
        g.rect(0, 0, s - 1, B)
        _bowl(g, 0, 0, top_w, mid + t - 1, s, t, c, "tr br")
        if ch == "B":
            _bowl(g, 0, mid, R, B, s, t, c, "tr br")
        if ch == "R":
            g.rect(R - s + 1, mid + t, R, B)
    elif ch == "S":
        g.rect(0, 0, R, t - 1)
        g.rect(0, 0, s - 1, mid + t - 1)
        g.rect(0, mid, R, mid + t - 1)
        g.rect(R - s + 1, mid, R, B)
        g.rect(0, B - t + 1, R, B)
        g.cut(0, 0, c, "tl")
        g.cut(R, B, c, "br")
        g.rect(R - s + 1, t, R, t + c - 2)
        g.rect(0, B - t - c + 2, s - 1, B - t)
    elif ch == "1":
        x = R - s + 1
        g.rect(x, 0, R, B)
        g.diag(x - 1, 0, 0, x - 1, 2)
    elif ch == "2":
        g.rect(0, 0, R, t - 1)
        g.rect(R - s + 1, 0, R, mid + t - 1)
        g.rect(0, mid, R, mid + t - 1)
        g.rect(0, mid, s - 1, B)
        g.rect(0, B - t + 1, R, B)
        g.cut(R, 0, c, "tr")
        g.cut(0, mid + t - 1, c, "bl", on=False)
        g.rect(0, mid, s - 1, B)
        g.cut(R, mid + t - 1, c, "br")
    elif ch == "3":
        g.rect(0, 0, R, t - 1)
        g.rect(R - s + 1, 0, R, B)
        g.rect(s - 1, mid, R, mid + t - 1)
        g.rect(0, B - t + 1, R, B)
        g.cut(R, 0, c, "tr")
        g.cut(R, B, c, "br")
    elif ch == "4":
        g.rect(0, 0, s - 1, mid + t - 1)
        g.rect(0, mid, R, mid + t - 1)
        g.rect(R - s + 1, 0, R, B)
    elif ch == "5":
        g.rect(0, 0, R, t - 1)
        g.rect(0, 0, s - 1, mid + t - 1)
        g.rect(0, mid, R, mid + t - 1)
        g.rect(R - s + 1, mid, R, B)
        g.rect(0, B - t + 1, R, B)
        g.cut(R, mid, c, "tr")
        g.cut(R, B, c, "br")
    elif ch == "6":
        _bowl(g, 0, mid, R, B, s, t, c, "bl br")
        g.rect(0, 0, s - 1, B - c)
        g.rect(0, 0, R, t - 1)
        g.cut(0, 0, c, "tl")
    elif ch == "7":
        g.rect(0, 0, R, t - 1)
        g.diag(R - s + 1, t, (W - s) // 3, B, s)
    elif ch == "8":
        _bowl(g, 0, 0, R, mid + t - 1, s, t, c)
        _bowl(g, 0, mid, R, B, s, t, c)
        g.rect(0, mid, R, mid + t - 1)
    elif ch == "9":
        _bowl(g, 0, 0, R, mid + t - 1, s, t, c, "tl tr")
        g.rect(R - s + 1, c, R, B)
        g.rect(0, B - t + 1, R, B)
        g.cut(R, B, c, "br")
    else:
        raise KeyError(ch)
    return g.m


def word(text, H, s, t, N, c, gap, space=None):
    """Lay out capitals with `gap` px between letters; returns the mask and each letter's x."""
    masks = [None if ch == " " else letter(ch, H, s, t, N, c) for ch in text]
    sp = space if space is not None else N
    widths = [sp if m is None else m.shape[1] for m in masks]
    W = sum(widths) + gap * (len(text) - 1)
    out = np.zeros((H, W), bool)
    xs = []
    x = 0
    for m, w in zip(masks, widths):
        xs.append(x)
        if m is not None:
            out[:, x:x + w] |= m
        x += w + gap
    return out, xs
