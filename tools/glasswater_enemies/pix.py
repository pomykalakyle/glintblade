"""Small pixel-art toolkit for the Glasswater enemy sheets.

Frames are palette-indexed character grids (one char = one pixel, '.' = transparent).
Frames can be composed from parts (grids pasted at offsets, with optional color remaps),
then written to a sprite sheet PNG + JSON manifest following assets/SPRITE_FORMAT.md.
Previews: flat 8x over the region ground, a lit simulation (albedo * light + emissive + bloom)
that approximates the engine's look in the dark cave, and animated GIFs.
"""

import json
import os

import numpy as np
from PIL import Image, ImageDraw

from engine_light import NativeFrame

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SPRITES = os.path.join(ROOT, "assets", "sprites")
PREVIEWS = os.path.join(ROOT, "out", "samples", "enemies_glasswater_v2style", "authored_previews")
SAMPLES = os.path.join(ROOT, "out", "samples")
WORK = os.path.join(ROOT, "tools", "glasswater_enemies", "work")

GROUND = "#364657"  # glasswater rock[1]: flat preview background
FLOOR_DARK = "#29313f"  # wall[0]: lit-sim floor albedo
AMBIENT = ("#9caeb9", 0.6)
LIGHT_GAIN = 1.25


def hex2rgb(h):
    """Convert a hex swatch to byte RGB."""
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


class Grid:
    """A frame: 2D array of palette keys (None = transparent)."""

    def __init__(self, w, h):
        """Initialize the native-grid drawing data."""
        self.w, self.h = w, h
        self.px = [[None] * w for _ in range(h)]

    @staticmethod
    def parse(text):
        """Read a character grid with transparent dots and shared indentation."""
        lines = [ln.rstrip() for ln in text.strip("\n").split("\n")]
        # allow an indented block: strip the common leading whitespace
        ind = min(len(ln) - len(ln.lstrip()) for ln in lines if ln.strip())
        lines = [ln[ind:] for ln in lines]
        w = max(len(ln) for ln in lines)
        g = Grid(w, len(lines))
        for y, ln in enumerate(lines):
            for x, ch in enumerate(ln):
                if ch not in ". ":
                    g.px[y][x] = ch
        return g

    def copy(self):
        """Copy the palette-indexed pixel grid."""
        g = Grid(self.w, self.h)
        g.px = [row[:] for row in self.px]
        return g

    def paste(self, other, ox, oy, remap=None, under=False):
        """Paste `other` at (ox, oy). remap: dict char->char (None value = drop pixel).
        under=True only fills transparent pixels."""
        for y in range(other.h):
            for x in range(other.w):
                c = other.px[y][x]
                if c is None:
                    continue
                if remap and c in remap:
                    c = remap[c]
                    if c is None:
                        continue
                tx, ty = ox + x, oy + y
                if 0 <= tx < self.w and 0 <= ty < self.h:
                    if under and self.px[ty][tx] is not None:
                        continue
                    self.px[ty][tx] = c
        return self

    def set(self, x, y, c):
        """Set one pixel if it lies inside the grid."""
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = c
        return self

    def get(self, x, y):
        """Read one grid pixel or return transparent outside its bounds."""
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[y][x]
        return None

    def remap(self, m):
        """Copy the grid with selected palette keys substituted."""
        g = self.copy()
        for y in range(g.h):
            for x in range(g.w):
                c = g.px[y][x]
                if c in m:
                    g.px[y][x] = m[c]
        return g

    def flip(self):
        """Mirror the grid horizontally."""
        g = Grid(self.w, self.h)
        g.px = [row[::-1] for row in self.px]
        return g

    def bbox(self):
        """Return the occupied native pixel bounds."""
        xs, ys = [], []
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y][x] is not None:
                    xs.append(x)
                    ys.append(y)
        if not xs:
            return None
        return min(xs), min(ys), max(xs), max(ys)

    def outline(self, key, exclude=()):
        """Add a 4-connected outline of `key` around opaque pixels (not over existing ones).
        Pixels whose key is in `exclude` don't cast outline."""
        g = self.copy()
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y][x] is not None:
                    continue
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    c = self.get(x + dx, y + dy)
                    if c is not None and c not in exclude:
                        g.px[y][x] = key
                        break
        return g

    def to_rgba(self, pal):
        """Convert palette keys to an opaque-or-transparent RGBA array."""
        a = np.zeros((self.h, self.w, 4), np.uint8)
        for y in range(self.h):
            for x in range(self.w):
                c = self.px[y][x]
                if c is None:
                    continue
                if c not in pal:
                    raise KeyError(f"palette has no key {c!r}")
                r, g_, b = hex2rgb(pal[c])
                a[y, x] = (r, g_, b, 255)
        return a


def blank(w, h):
    """Create an empty native-grid cell."""
    return Grid(w, h)


# ---------------------------------------------------------------- sheet writing


class Sheet:
    def __init__(self, name, cell, anchor, pal, emissive_keys, notes=""):
        """Initialize the native-grid drawing data."""
        self.name = name
        self.cw, self.ch = cell
        self.anchor = anchor
        self.pal = pal
        self.emissive_keys = emissive_keys
        self.notes = notes
        self.rows = []  # (anim_name, [Grid], meta)

    def add(self, anim, frames, fps=None, durations=None, anchor=None, extra=None):
        """Append one animation row with its original timing and anchor metadata."""
        for f in frames:
            assert (f.w, f.h) == (self.cw, self.ch), (anim, f.w, f.h)
        meta = {}
        if fps is not None:
            meta["fps"] = fps
        if durations is not None:
            assert len(durations) == len(frames), anim
            meta["durations"] = durations
            if fps is None:
                meta["fps"] = round(60 * len(durations) / sum(durations), 2)
        if anchor is not None:
            meta["anchor"] = list(anchor)
        if extra:
            meta.update(extra)
        self.rows.append((anim, frames, meta))

    def emissive_hex(self):
        """Return the unique emissive swatches."""
        return sorted({self.pal[k].lower() for k in self.emissive_keys})

    def check(self):
        """Hard edges, and emissive colors used only by emissive keys."""
        em = set(self.emissive_hex())
        for k, v in self.pal.items():
            if v.lower() in em and k not in self.emissive_keys:
                raise ValueError(f"{self.name}: key {k!r} shares emissive color {v} but isn't emissive")
        # colors used
        used = set()
        for _, frames, _ in self.rows:
            for f in frames:
                for row in f.px:
                    for c in row:
                        if c is not None:
                            used.add(self.pal[c].lower())
        return sorted(used)

    def save(self, extra_json=None):
        """Write the reproducible PNG sheet and matching sprite manifest."""
        os.makedirs(SPRITES, exist_ok=True)
        cols = max(len(fr) for _, fr, _ in self.rows)
        img = np.zeros((self.ch * len(self.rows), self.cw * cols, 4), np.uint8)
        anims = {}
        for r, (anim, frames, meta) in enumerate(self.rows):
            for i, f in enumerate(frames):
                img[r * self.ch : (r + 1) * self.ch, i * self.cw : (i + 1) * self.cw] = f.to_rgba(self.pal)
            d = {"row": r, "frames": len(frames)}
            d.update(meta)
            anims[anim] = d
        Image.fromarray(img, "RGBA").save(os.path.join(SPRITES, self.name + ".png"))
        used = self.check()
        man = {
            "cell": [self.cw, self.ch],
            "anchor": list(self.anchor),
            "emissive": self.emissive_hex(),
            "anims": anims,
        }
        if extra_json:
            man.update(extra_json)
        man["colors_used"] = len(used)
        if self.notes:
            man["notes"] = self.notes
        with open(os.path.join(SPRITES, self.name + ".json"), "w") as fh:
            json.dump(man, fh, indent=1)
            fh.write("\n")
        return img, used


# ---------------------------------------------------------------- previews


def _bg(w, h, color):
    """Create an opaque flat preview background."""
    a = np.zeros((h, w, 4), np.uint8)
    a[..., :3] = hex2rgb(color)
    a[..., 3] = 255
    return a


def over(dst, src, ox, oy):
    """Alpha-over a hard-edged RGBA array onto dst (in place)."""
    h, w = src.shape[:2]
    for y in range(h):
        ty = oy + y
        if not (0 <= ty < dst.shape[0]):
            continue
        for x in range(w):
            tx = ox + x
            if 0 <= tx < dst.shape[1] and src[y, x, 3]:
                dst[ty, tx] = src[y, x]
    return dst


def upscale(a, s):
    """Enlarge an array using exact nearest-neighbor pixel replication."""
    return np.repeat(np.repeat(a, s, 0), s, 1)


def lit(albedo_rgba, emissive_mask, light_pos=None, light_color="#9ff4ff", light_radius=64, bloom=True, scale=8, extra_lights=()):
    """The engine's own lighting at native resolution (engine_light.py: albedo * stepped light +
    emissive, glow quantized to 7 dithered levels on the pixel grid), then nearest-neighbor
    enlargement. No smooth bloom. Lights: ((x, y), color, radius, intensity)."""
    h, w = albedo_rgba.shape[:2]
    fr = NativeFrame(w, h, ambient=AMBIENT)
    rgb = albedo_rgba[..., :3].astype(np.float32) / 255.0
    fr.albedo = np.where(emissive_mask[..., None], 0, rgb).astype(np.float32)
    fr.emissive = np.where(emissive_mask[..., None], rgb, 0).astype(np.float32)
    lights = list(extra_lights)
    if light_pos is not None:
        lights.append((light_pos, light_color, light_radius, 0.8))
    for (lx, ly), col, rad, inten in lights:
        fr.add_light(lx, ly, rad, col, inten)
    out = fr.render(vignette=0.0, bloom=0.9 if bloom else 0.0)
    return upscale(out, scale)


def emissive_mask(rgba, emissive_hex):
    """Identify only opaque pixels whose RGB is declared emissive."""
    em = {hex2rgb(h) for h in emissive_hex}
    h, w = rgba.shape[:2]
    m = np.zeros((h, w), bool)
    for y in range(h):
        for x in range(w):
            if rgba[y, x, 3] and tuple(int(v) for v in rgba[y, x, :3]) in em:
                m[y, x] = True
    return m


def sheet_preview(sheet_rgba, sheet, path, scale=8, grid=True):
    """Whole sheet at `scale` over the ground color with faint cell lines + anchor ticks."""
    h, w = sheet_rgba.shape[:2]
    bg = _bg(w, h, GROUND)
    over(bg, sheet_rgba, 0, 0)
    big = upscale(bg, scale)
    im = Image.fromarray(big, "RGBA").convert("RGB")
    if grid:
        d = ImageDraw.Draw(im)
        for cx in range(0, w + 1, sheet.cw):
            d.line([(cx * scale, 0), (cx * scale, h * scale)], fill=(48, 58, 96))
        for cy in range(0, h + 1, sheet.ch):
            d.line([(0, cy * scale), (w * scale, cy * scale)], fill=(48, 58, 96))
        for r, (anim, frames, meta) in enumerate(sheet.rows):
            ax, ay = meta.get("anchor", sheet.anchor)
            for i in range(len(frames)):
                x0 = (i * sheet.cw + ax) * scale
                y0 = (r * sheet.ch + ay) * scale
                d.rectangle([x0 + scale // 2 - 1, y0 + scale // 2 - 1, x0 + scale // 2 + 1, y0 + scale // 2 + 1], outline=(255, 80, 80))
            d.text((2, r * sheet.ch * scale + 2), anim, fill=(160, 170, 220))
    im.save(path)
    return im


def gif(frames_rgb, durations_ticks, path):
    """frames_rgb: list of HxWx3 uint8. durations in 60 Hz ticks."""
    ims = [Image.fromarray(f, "RGB") for f in frames_rgb]
    # quantize each frame adaptively for decent colors
    pal_ims = [im.convert("P", palette=Image.ADAPTIVE, colors=255) for im in ims]
    ms = [max(20, int(round(d * 1000 / 60 / 10)) * 10) for d in durations_ticks]
    pal_ims[0].save(path, save_all=True, append_images=pal_ims[1:], duration=ms, loop=0, disposal=1)


def frame_durations(meta, n):
    """Resolve authored durations or the animation FPS to sixty-Hz ticks."""
    if "durations" in meta:
        return meta["durations"]
    fps = meta.get("fps", 8)
    return [max(1, round(60 / fps))] * n
