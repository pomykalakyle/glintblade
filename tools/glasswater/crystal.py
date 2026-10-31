"""Crystal prism rasterizer: clean 3-face prisms lit from the upper left, composited back to front.

Each prism is drawn into its own layer; when a nearer prism overlaps an earlier one, the earlier one gets
a 1px dark separation line along the nearer prism's silhouette (it sits in that prism's shadow)."""
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, blank  # noqa: E402

RAMPS = {
    "aqua": ["a0", "a1", "a2", "a3", "a4"],
    "violet": ["v0", "v1", "v2", "v3", "v4"],
}


def prism_layer(shape, ramp, bx, by, h, r, tilt, tl, depth=0, streak=None, bury=2, shift=1):
    """One prism as a canvas (shape = (w, h)); writes outside the canvas are dropped, never wrapped.
    depth 0 = front prism, depth 1 = one step darker (behind). shift 1 = calm shading: faces are lit by the
    prop's own light and only the ridge and tip are emissive; shift 0 = the old bright shading.
    bx, by: base centre (by = lowest row); h: height incl. tip; r: half width; tilt: tip x offset vs base;
    tl: tip rows. bury: bottom rows drawn one step darker (where it sinks into the ground)."""
    R = [I[n] for n in RAMPS[ramp]] + [I["o"]]
    o = R[5]
    s = shift + depth
    lit, mid, dk = R[max(3 - s, 0)], R[max(2 - s, 0)], (R[1 - s] if 1 - s >= 0 else o)
    hi = R[4 - s]
    apex = R[4] if depth == 0 else hi
    c = blank(shape[0], shape[1])
    W, H = shape

    def put(y, x, v):
        if 0 <= y < H and 0 <= x < W:
            c[y, x] = v

    def get(y, x):
        return c[y, x] if (0 <= y < H and 0 <= x < W) else -1

    top = by - h + 1

    def axis(y):
        t = (by - y) / max(h - 1, 1)
        return bx + tilt * t

    lw = max(1, round(r * 0.55))
    rw = max(1, round(r * 0.45))
    for y in range(top, by + 1):
        cx = axis(y)
        ft = y - top
        hw = r * min(1.0, (ft + 1) / tl) if ft < tl else r
        x0 = math.floor(cx - hw + 0.5)
        x1 = max(x0, math.floor(cx + hw - 0.5))
        if ft < tl:
            xm = math.floor(cx)
            for x in range(x0, x1 + 1):
                put(y, x, (apex if ft < 2 else hi) if x < xm else (lit if x == xm else mid))
            continue
        l_end = x0 + lw
        r_start = x1 - rw + 1
        for x in range(x0, x1 + 1):
            put(y, x, lit if x < l_end else (mid if x < r_start else dk))
        if ft < tl + (h - tl) * 0.45:
            put(y, l_end - 1, hi)
        if by - y < bury:
            for x in range(x0, x1 + 1):
                v = get(y, x)
                if v >= 0:
                    put(y, x, {lit: mid, mid: dk, dk: o, hi: lit}.get(v, v))
    if streak:
        s0, L = streak
        y0 = top + tl + s0
        xl = None
        for i in range(L):
            y = y0 + i
            if not 0 <= y < H:
                break
            row = np.where(c[y] == mid)[0]
            if len(row) == 0:
                break
            if xl is None:
                xl = row[0]
            x = xl + (i + 1) // 2
            if x > row[-1] - 1:
                break
            c[y, x] = lit
    return c


def composite(layers, ramp_dark):
    """Paste layers in order; where a later layer borders earlier pixels, darken those to the outline color."""
    h, w = layers[0].shape
    out = blank(w, h)
    od = I[ramp_dark]
    for L in layers:
        m = L >= 0
        # pixels of earlier layers adjacent (4-neighbour) to this layer's silhouette, on its right/bottom or left
        ring = np.zeros_like(m)
        for dy, dx in ((0, 1), (0, -1), (1, 0)):
            ring |= np.roll(np.roll(m, dy, 0), dx, 1)
        ring &= ~m & (out >= 0)
        out[ring] = od
        out[m] = L[m]
    return out


def outline(c, color, sides=("r", "b", "l")):
    """Outline the silhouette with `color` on the given sides (r = right, b = bottom, l = left, t = top)."""
    h, w = c.shape
    out = c.copy()
    filled = c >= 0
    for y in range(h):
        for x in range(w):
            if filled[y, x]:
                continue
            if ("r" in sides and x > 0 and filled[y, x - 1]) or ("b" in sides and y > 0 and filled[y - 1, x]) or \
               ("l" in sides and x + 1 < w and filled[y, x + 1]) or ("t" in sides and y + 1 < h and filled[y + 1, x]):
                out[y, x] = I[color]
    return out
