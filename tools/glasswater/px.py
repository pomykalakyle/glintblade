"""Tiny pixel-art toolkit for the Glasswater Hollow sheets.

Canvases are numpy int arrays of palette indices (-1 = transparent). Everything is authored 1:1.
"""

import json
import os

import numpy as np
from PIL import Image

ROOT = str(__import__("pathlib").Path(__file__).resolve().parents[2])
SPRITES = os.path.join(ROOT, "assets/sprites")
PREVIEWS = os.path.join(ROOT, "assets/previews")

# Region ramps from assets/concept/palettes.json (glasswater), plus two small additions noted in manifests.
HEX = {
    # rock (floor, boulders): cool blue-grey
    "r0": "#0e1022", "r1": "#171b35", "r2": "#222a4a", "r3": "#2f3c62", "r4": "#42557e", "r5": "#5e76a0",
    # wall (cliff faces, carved stone): indigo-violet
    "w0": "#120a2a", "w1": "#21164a", "w2": "#33286c", "w3": "#463e90", "w4": "#5e5cb4", "w5": "#8a90d8",
    # crystal aqua
    "a0": "#0b3a5a", "a1": "#0f6a8c", "a2": "#16a3b8", "a3": "#3fd8d8", "a4": "#a8fff4",
    # crystal violet
    "v0": "#24104f", "v1": "#46208a", "v2": "#7438c8", "v3": "#a86cf0", "v4": "#e0c4ff",
    # water
    "q0": "#08122e", "q1": "#0e2450", "q2": "#173e78", "q3": "#2a64a0", "q4": "#5aa8d0", "q5": "#c8f4ff",
    # mushroom cap (also leafy greens for pads)
    "m0": "#1c3a34", "m1": "#2f6a3c", "m2": "#62a83a", "m3": "#b4e04a", "m4": "#eeff9a",
    # mushroom stem (pale blue-white)
    "s0": "#2a2a50", "s1": "#4a5a88", "s2": "#8098c0", "s3": "#c4d4ec",
    # region outline
    "o": "#06051a",
    # addition: muted plum for roots (no brown in the region palette; plum sits between wall and rock)
    "p0": "#1a1028", "p1": "#2e1f3e", "p2": "#4a3354", "p3": "#6a4c6c",
}
NAMES = list(HEX.keys())
IDX = {n: i for i, n in enumerate(NAMES)}


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


RGB = np.array([rgb(HEX[n]) for n in NAMES], dtype=np.uint8)

# Glowing colors (match palettes.json glasswater emissive).
EMISSIVE = ["a3", "a4", "v3", "v4", "m3", "m4", "q5"]


def blank(w, h):
    return np.full((h, w), -1, dtype=np.int32)


def ascii_grid(rows, key):
    """rows: list of strings; key: char -> palette name. '.' and ' ' are transparent."""
    h = len(rows)
    w = max(len(r) for r in rows)
    c = blank(w, h)
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch in ". ":
                continue
            if ch not in key:
                raise KeyError(f"unknown char {ch!r} at {x},{y}")
            c[y, x] = IDX[key[ch]]
    return c


def paste(dst, src, x, y):
    """Paste src onto dst with its top-left at (x, y); transparent pixels skipped."""
    h, w = src.shape
    for j in range(h):
        for i in range(w):
            v = src[j, i]
            if v >= 0:
                yy, xx = y + j, x + i
                if 0 <= yy < dst.shape[0] and 0 <= xx < dst.shape[1]:
                    dst[yy, xx] = v


def to_rgba(c):
    h, w = c.shape
    out = np.zeros((h, w, 4), dtype=np.uint8)
    m = c >= 0
    out[m, :3] = RGB[c[m]]
    out[m, 3] = 255
    return out


def save_sheet(name, rows, cell, manifest):
    """rows: list of lists of canvases (each cell-sized). Writes PNG + JSON into assets/sprites."""
    cw, ch = cell
    ncols = max(len(r) for r in rows)
    sheet = blank(cw * ncols, ch * len(rows))
    for r, row in enumerate(rows):
        for c, cv in enumerate(row):
            assert cv.shape == (ch, cw), (name, r, c, cv.shape)
            sheet[r * ch:(r + 1) * ch, c * cw:(c + 1) * cw] = cv
    Image.fromarray(to_rgba(sheet), "RGBA").save(os.path.join(SPRITES, f"{name}.png"))
    manifest = dict(manifest)
    manifest["cell"] = [cw, ch]
    with open(os.path.join(SPRITES, f"{name}.json"), "w") as f:
        json.dump(manifest, f, indent=1)
    return sheet


def emissive_hex():
    return [HEX[n] for n in EMISSIVE]


def colors_used(c):
    return sorted({NAMES[v] for v in np.unique(c) if v >= 0})


# ------------------------------------------------------------------ previews

def upscale(img, s):
    return img.resize((img.width * s, img.height * s), Image.NEAREST)


def on_bg(c, bg_hex, pad=2):
    """RGBA image of canvas c over a solid background."""
    h, w = c.shape
    out = np.zeros((h + 2 * pad, w + 2 * pad, 3), dtype=np.uint8)
    out[:] = rgb(bg_hex)
    rgba = to_rgba(c)
    m = rgba[..., 3] > 0
    out[pad:pad + h, pad:pad + w][m] = rgba[..., :3][m]
    return Image.fromarray(out, "RGB")


# ------------------------------------------------------------------ mock lighting (mirrors src/gfx)

def bayer4(x, y):
    m = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]], dtype=np.float32)
    return (m[y % 4, x % 4] + 0.5) / 16.0


class Scene:
    """Approximates the engine: albedo * light + emissive, stepped light, vignette, bloom."""

    def __init__(self, w, h, ambient="#5a60c0", amb_k=0.38, gain=1.25):
        self.w, self.h = w, h
        self.albedo = np.zeros((h, w, 3), np.float32)
        self.emit = np.zeros((h, w, 3), np.float32)
        a = np.array(rgb(ambient), np.float32) / 255.0 * amb_k
        self.light = np.zeros((h, w, 3), np.float32) + a
        self.gain = gain

    def ground(self, c, x=0, y=0):
        self.sprite(c, x, y)

    def sprite(self, c, x, y, emissive_names=EMISSIVE):
        h, w = c.shape
        em = {IDX[n] for n in emissive_names}
        for j in range(h):
            for i in range(w):
                v = c[j, i]
                if v < 0:
                    continue
                yy, xx = y + j, x + i
                if not (0 <= yy < self.h and 0 <= xx < self.w):
                    continue
                col = RGB[v].astype(np.float32) / 255.0
                if v in em:
                    self.albedo[yy, xx] = 0
                    self.emit[yy, xx] = col
                else:
                    self.albedo[yy, xx] = col
                    self.emit[yy, xx] = 0

    def add_light(self, x, y, radius, color, intensity=0.8):
        col = np.array(rgb(color), np.float32) / 255.0 * intensity * self.gain
        yy, xx = np.mgrid[0:self.h, 0:self.w].astype(np.float32)
        d2 = ((xx + 0.5 - x) ** 2 + (yy + 0.5 - y) ** 2) / (radius * radius)
        f = np.clip(1.0 - d2, 0, None) ** 2
        self.light += f[..., None] * col

    def render(self, steps=10.0, vignette=0.25, bloom=0.9):
        h, w = self.h, self.w
        yy, xx = np.mgrid[0:h, 0:w]
        d = bayer4(xx, yy)[..., None]
        l = np.maximum(np.floor(self.light * steps + d) / steps, 0)
        c = self.albedo * l + self.emit
        cx, cy = w * 0.5, h * 0.5
        inv = 1.0 / np.sqrt(cx * cx + cy * cy)
        r = np.minimum(np.sqrt((xx + 0.5 - cx) ** 2 + ((yy + 0.5 - cy) * 1.25) ** 2) * inv, 1.0)
        v = np.clip((r - 0.45) / 0.55, 0, 1)
        c = c * (1.0 - vignette * v * v)[..., None]

        def soft(x):
            K = 0.82
            return np.where(x <= K, x, K + (1 - K) * (1 - np.exp(-(x - K) / (1 - K))))

        out = np.clip(soft(np.maximum(c, 0)) * 255.0 + (bayer4(xx, yy)[..., None] - 0.5), 0, 255).astype(np.float32)
        # glow like the engine (post.rs): blurred emissive, then quantized ON the pixel grid to a few
        # levels with ordered dithering, added to the base. No smooth off-grid bloom.
        src = self.emit + np.maximum(c - 0.85, 0) * 0.6
        bimg = Image.fromarray(np.clip(src * 255, 0, 255).astype(np.uint8), "RGB")
        from PIL import ImageFilter
        mid = bimg.resize((w // 2, h // 2), Image.BILINEAR).filter(ImageFilter.GaussianBlur(2.4))
        wide = bimg.resize((w // 4, h // 4), Image.BILINEAR).filter(ImageFilter.GaussianBlur(3.0))
        gh = (np.asarray(mid, np.float32) * 0.85 + np.asarray(wide.resize((w // 2, h // 2), Image.BILINEAR), np.float32) * 0.9) * bloom / 255.0
        g = np.asarray(Image.fromarray(np.clip(gh * 255, 0, 255).astype(np.uint8), "RGB").resize((w, h), Image.BILINEAR), np.float32) / 255.0
        LV = np.array([0.0, 0.05, 0.1, 0.17, 0.27, 0.42, 0.65], np.float32)
        m = g.max(axis=2)
        v = np.minimum(m, LV[-1])
        k = np.clip(np.searchsorted(LV, v, side="right") - 1, 0, len(LV) - 2)
        lo, hi = LV[k], LV[k + 1]
        frac = (v - lo) / (hi - lo)
        level = np.where(frac > bayer4(xx, yy), hi, lo)
        level = np.where(m < LV[1] * 0.5, 0, level)
        add = g * (level / np.maximum(m, 1e-6))[..., None]
        base01 = out / 255.0
        out = np.clip(base01 + add * (1.0 - base01 * 0.5), 0, 1) * 255.0
        self._base = Image.fromarray(out.astype(np.uint8), "RGB")
        return self._base

    def composite(self, scale):
        """Nearest upscale of the final native frame (glow already on the pixel grid)."""
        return upscale(self._base, scale)


# ------------------------------------------------------------------ scale figure

def rough_hero():
    """A rough 18 px tall cloaked figure for scale (until hero.png exists)."""
    rows = [
        "....oooo....",
        "...oWWWWo...",
        "..oWwwwwWo..",
        "..oWkkkkWo..",
        "..oWkekekWo.",
        "..oWkkkkWo..",
        ".ooWWwwWWoo.",
        ".oSSSWWWWWo.",
        "oSSsWWwwWWWo",
        "oSs.WwwwwWWo",
        "....WwwwwWWo",
        "...oWwwwwWWo",
        "...oWwwwwwWo",
        "...oWWwwwWWo",
        "...oWWWWWWWo",
        "....oWWoWWo.",
        "....o00o00o.",
        ".....oo.oo..",
    ]
    key = {"o": "o", "W": "w1", "w": "w2", "k": "o", "e": "a4", "S": "q3", "s": "q4", "0": "w0"}
    return ascii_grid(rows, key)


def hero_cell():
    """The real hero idle frame if it exists, else the rough figure. Returns (canvas, anchor)."""
    p = os.path.join(SPRITES, "hero.png")
    j = os.path.join(SPRITES, "hero.json")
    if os.path.exists(p) and os.path.exists(j):
        try:
            man = json.load(open(j))
            cw, ch = man["cell"]
            anim = man["anims"].get("idle_down") or next(iter(man["anims"].values()))
            row = anim["row"]
            im = np.asarray(Image.open(p).convert("RGBA"))
            cell = im[row * ch:(row + 1) * ch, 0:cw]
            return ("rgba", cell), tuple(anim.get("anchor", man.get("anchor", [cw // 2, ch - 1])))
        except Exception:
            pass
    c = rough_hero()
    return ("idx", c), (c.shape[1] // 2, c.shape[0] - 1)
