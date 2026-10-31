"""Native-resolution port of the engine's lighting path, for previews that match the game.

Mirrors src/gfx/frame.rs (`begin`, `add_light`, `shadow`, `put_glow`) and src/gfx/post.rs
(`compose` with stepped, Bayer-dithered light; `finish` with the glow quantized to 7 levels and
applied on the native pixel grid). Everything is computed at 1x; enlarge afterwards with
nearest-neighbor only. Default Grade: exposure/contrast/saturation 1, no lift/gain,
vignette 0.25, bloom 0.9, bloom_threshold 0.85, light_steps 10.
"""

import numpy as np

BAYER4 = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]], np.float32)
LEVELS = [0.0, 0.05, 0.1, 0.17, 0.27, 0.42, 0.65]


def rgbf(h):
    """Convert a hex swatch to float RGB."""
    h = h.lstrip("#")
    return np.array([int(h[i : i + 2], 16) for i in (0, 2, 4)], np.float32) / 255.0


def bayer4(w, h, ox=0, oy=0):
    """Tile the native four-by-four ordered dither matrix."""
    ys = (np.arange(h) + oy) & 3
    xs = (np.arange(w) + ox) & 3
    return (BAYER4[ys[:, None], xs[None, :]] + 0.5) / 16.0


def soft_clip(x):
    """Compress source highlights with the engine tone curve."""
    k = 0.82
    return np.where(x <= k, x, k + (1 - k) * (1 - np.exp(-(x - k) / (1 - k))))


def blur(src, radius, sigma):
    """Separable Gaussian with clamped edges (post.rs `blur`)."""
    w = np.exp(-(np.arange(-radius, radius + 1) ** 2) / (2 * sigma * sigma)).astype(np.float32)
    w /= w.sum()
    h_, w_ = src.shape[:2]
    out = np.zeros_like(src)
    for k, wt in enumerate(w):
        xs = np.clip(np.arange(w_) + k - radius, 0, w_ - 1)
        out += src[:, xs] * wt
    out2 = np.zeros_like(src)
    for k, wt in enumerate(w):
        ys = np.clip(np.arange(h_) + k - radius, 0, h_ - 1)
        out2 += out[ys] * wt
    return out2


def bilinear(src, fx, fy):
    """Sample src (h, w, 3) at float coords (arrays), clamped, like the Rust code."""
    h, w = src.shape[:2]
    fx = np.clip(fx, 0, w - 1)
    fy = np.clip(fy, 0, h - 1)
    x0 = np.floor(fx).astype(int)
    y0 = np.floor(fy).astype(int)
    x1 = np.minimum(x0 + 1, w - 1)
    y1 = np.minimum(y0 + 1, h - 1)
    tx = (fx - x0)[..., None]
    ty = (fy - y0)[..., None]
    a = src[y0, x0] * (1 - tx) + src[y0, x1] * tx
    b = src[y1, x0] * (1 - tx) + src[y1, x1] * tx
    return a * (1 - ty) + b * ty


class NativeFrame:
    def __init__(self, w, h, bg="#000000", ambient=("#9caeb9", 0.6)):
        """Initialize the native-grid drawing data."""
        self.w, self.h = w, h
        self.albedo = np.tile(rgbf(bg), (h, w, 1))
        self.emissive = np.zeros((h, w, 3), np.float32)
        self.light = np.tile(rgbf(ambient[0]) * ambient[1], (h, w, 1))

    def put(self, rgba, x0, y0, emissive_hex=()):
        """Hard-edged RGBA sprite; colors in emissive_hex become self-lit (put_glow)."""
        em = {tuple(np.round(rgbf(e) * 255).astype(int)) for e in emissive_hex}
        hh, ww = rgba.shape[:2]
        for j in range(hh):
            y = y0 + j
            if not 0 <= y < self.h:
                continue
            for i in range(ww):
                x = x0 + i
                if not 0 <= x < self.w or rgba[j, i, 3] < 128:
                    continue
                c = rgba[j, i, :3].astype(np.float32) / 255.0
                if tuple(int(v) for v in rgba[j, i, :3]) in em:
                    self.albedo[y, x] = 0
                    self.emissive[y, x] = c
                else:
                    self.albedo[y, x] = c
                    self.emissive[y, x] = 0

    def shadow(self, cx, cy, rx, ry, strength):
        """Apply the engine native-grid contact shadow."""
        for dy in range(-ry, ry + 1):
            for dx in range(-rx, rx + 1):
                d = dx * dx / (rx * rx) + dy * dy / (ry * ry)
                if d > 1:
                    continue
                edge = 1 - d
                k = strength if edge >= 0.35 or edge / 0.35 > (BAYER4[(cy + dy) & 3, (cx + dx) & 3] + 0.5) / 16 else 0
                x, y = cx + dx, cy + dy
                if k and 0 <= x < self.w and 0 <= y < self.h:
                    self.albedo[y, x] *= 1 - k

    def add_light(self, x, y, radius, color, intensity):
        """Add the source-directed point light at native resolution."""
        c = rgbf(color) * intensity
        yy, xx = np.mgrid[0 : self.h, 0 : self.w].astype(np.float32)
        d2 = ((xx + 0.5 - x) ** 2 + (yy + 0.5 - y) ** 2) / (radius * radius)
        f = np.where(d2 < 1, (1 - d2) ** 2, 0)
        self.light += f[..., None] * c

    def render(self, vignette=0.25, light_steps=10.0, bloom=0.9, threshold=0.85):
        """compose + finish. Returns uint8 (h, w, 3) at native resolution."""
        h, w = self.h, self.w
        d = bayer4(w, h)[..., None]
        l = self.light
        if light_steps > 0:
            l = np.maximum(np.floor(l * light_steps + d) / light_steps, 0)
        c = self.albedo * l + self.emissive
        hdr = c.copy()
        if vignette > 0:
            cx, cy = w * 0.5, h * 0.5
            inv = 1 / np.sqrt(cx * cx + cy * cy)
            yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
            r = np.minimum(np.sqrt((xx + 0.5 - cx) ** 2 + ((yy + 0.5 - cy) * 1.25) ** 2) * inv, 1)
            v = np.clip((r - 0.45) / 0.55, 0, 1)
            c = c * (1 - vignette * v * v)[..., None]
        dq = bayer4(w, h)[..., None] - 0.5
        out = np.clip(soft_clip(np.maximum(c, 0)) * 255 + dq, 0, 255).astype(np.uint8)

        # finish: bright pass at half res, two blurs, stepped halo on the native grid
        bw, bh = w // 2, h // 2
        src = np.zeros((bh, bw, 3), np.float32)
        for ox, oy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            hh = hdr[oy : bh * 2 : 2, ox : bw * 2 : 2]
            src += self.emissive[oy : bh * 2 : 2, ox : bw * 2 : 2] + np.maximum(hh - threshold, 0) * 0.6
        src *= 0.25
        mid = blur(src, 5, 2.4)
        qw, qh = bw // 2, bh // 2
        q = np.zeros((qh, qw, 3), np.float32)
        for ox, oy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            q += src[oy : qh * 2 : 2, ox : qw * 2 : 2]
        q *= 0.25
        wide = blur(q, 6, 3.0)
        yy, xx = np.mgrid[0:bh, 0:bw].astype(np.float32)
        wsamp = bilinear(wide, (xx + 0.5) * 0.5 - 0.5, (yy + 0.5) * 0.5 - 0.5)
        glow_half = (mid * 0.85 + wsamp * 0.9) * bloom
        yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
        g = bilinear(glow_half, (xx + 0.5) * 0.5 - 0.5, (yy + 0.5) * 0.5 - 0.5)
        m = g.max(axis=-1)
        v = np.minimum(m, LEVELS[-1])
        k = np.zeros(m.shape, int)
        for i in range(1, len(LEVELS)):
            k[v >= LEVELS[i]] = i
        lo = np.array(LEVELS)[k]
        hi = np.array(LEVELS + [LEVELS[-1]])[np.minimum(k + 1, len(LEVELS))]
        top = k + 1 >= len(LEVELS)
        frac = np.where(top, 0, (v - lo) / np.where(hi - lo == 0, 1, hi - lo))
        level = np.where(top, lo, np.where(frac > bayer4(w, h), hi, lo))
        level = np.where(m < LEVELS[1] * 0.5, 0, level)
        add = np.where(level[..., None] > 0, g * (level / np.where(m == 0, 1, m))[..., None], 0)
        base = out.astype(np.float32) / 255.0
        res = np.minimum(base + add * (1 - base * 0.5), 1)
        return (res * 255).astype(np.uint8)
