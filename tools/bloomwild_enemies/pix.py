"""Tiny pixel-art toolkit for hand-authored, palette-indexed sprites.

Sprites are ASCII grids: one char per pixel, '.' = transparent. Parts are pasted onto a
canvas at integer offsets; an optional outer outline is traced around the union silhouette.
Nothing is ever scaled, rotated or blurred: every pixel lands on the native grid.
"""

import json
import pathlib

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPRITES = ROOT / "assets" / "sprites"
PREVIEWS = ROOT / "assets" / "previews"
SAMPLES = ROOT / "out" / "samples"


def hex_rgb(h):
    """Parse a hexadecimal swatch into its RGB channels."""
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def grid(s):
    """Parse a multi-line string into a list of row strings (strips the common indent)."""
    rows = [r for r in s.strip("\n").split("\n")]
    rows = [r.rstrip() for r in rows]
    ind = min(len(r) - len(r.lstrip()) for r in rows if r.strip())
    rows = [r[ind:] for r in rows]
    w = max(len(r) for r in rows)
    return [r.ljust(w, ".") for r in rows]


def mirror(g):
    """Mirror a hand-authored pixel grid horizontally."""
    return [r[::-1] for r in g]


class Canvas:
    def __init__(self, w, h):
        """Create a transparent native-resolution pixel canvas."""
        self.w, self.h = w, h
        self.px = [["."] * w for _ in range(h)]

    def copy(self):
        """Copy all palette-indexed pixels into an independent canvas."""
        c = Canvas(self.w, self.h)
        c.px = [r[:] for r in self.px]
        return c

    def get(self, x, y):
        """Read a palette index, returning transparent outside the canvas."""
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[y][x]
        return "."

    def set(self, x, y, ch):
        """Write a palette index only within the canvas bounds."""
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = ch

    def paste(self, g, x, y, under=False):
        """Paste grid g with its top-left at (x, y). under=True only fills transparent pixels."""
        for j, row in enumerate(g):
            for i, ch in enumerate(row):
                if ch == "." or ch == " ":
                    continue
                if ch == "_":  # explicit erase
                    self.set(x + i, y + j, ".")
                    continue
                if under and self.get(x + i, y + j) != ".":
                    continue
                self.set(x + i, y + j, ch)
        return self

    def outline(self, ch="o", diagonal=False, skip=()):
        """Trace a 1 px outer outline around every opaque pixel (4-connected by default)."""
        src = [r[:] for r in self.px]
        nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        if diagonal:
            nb += [(1, 1), (-1, 1), (1, -1), (-1, -1)]
        for y in range(self.h):
            for x in range(self.w):
                if src[y][x] != ".":
                    continue
                for dx, dy in nb:
                    xx, yy = x + dx, y + dy
                    if 0 <= xx < self.w and 0 <= yy < self.h and src[yy][xx] != "." and src[yy][xx] not in skip:
                        self.px[y][x] = ch
                        break
        return self

    def edit(self, pts, ch):
        """Apply one palette index to the supplied integer coordinates."""
        for x, y in pts:
            self.set(x, y, ch)
        return self

    def shift(self, dx, dy):
        """Move canvas pixels by the supplied integer offset."""
        c = Canvas(self.w, self.h)
        for y in range(self.h):
            for x in range(self.w):
                c.set(x + dx, y + dy, self.px[y][x])
        self.px = c.px
        return self

    def flip(self):
        """Mirror the canvas pixels horizontally."""
        self.px = [r[::-1] for r in self.px]
        return self

    def rows(self):
        """Return the palette grid as row strings."""
        return ["".join(r) for r in self.px]

    def image(self, pal):
        """Render the indexed grid with binary alpha and exact palette swatches."""
        im = Image.new("RGBA", (self.w, self.h), (0, 0, 0, 0))
        p = im.load()
        for y in range(self.h):
            for x in range(self.w):
                ch = self.px[y][x]
                if ch == ".":
                    continue
                if ch not in pal:
                    raise KeyError(f"unknown palette char {ch!r} at {x},{y}")
                p[x, y] = hex_rgb(pal[ch]) + (255,)
        return im


def build_sheet(name, cell, anchor, anims, pal, emissive, extra=None):
    """anims: list of (anim_name, frames[Canvas], meta dict). One row per anim.
    Writes assets/sprites/<name>.png and .json, returns (sheet image, manifest)."""
    cw, ch = cell
    cols = max(len(f) for _, f, _ in anims)
    sheet = Image.new("RGBA", (cw * cols, ch * len(anims)), (0, 0, 0, 0))
    man = {"cell": [cw, ch], "anchor": list(anchor), "emissive": emissive, "anims": {}}
    for row, (an, frames, meta) in enumerate(anims):
        for i, fr in enumerate(frames):
            assert (fr.w, fr.h) == (cw, ch), (an, i)
            sheet.paste(fr.image(pal), (i * cw, row * ch))
        entry = {"row": row, "frames": len(frames)}
        entry.update(meta)
        man["anims"][an] = entry
    if extra:
        man.update(extra)
    SPRITES.mkdir(parents=True, exist_ok=True)
    sheet.save(SPRITES / f"{name}.png")
    with open(SPRITES / f"{name}.json", "w") as f:
        json.dump(man, f, indent=1)
    # sanity: no semi-transparent pixels
    a = sheet.getchannel("A")
    assert set(a.getdata()) <= {0, 255}
    return sheet, man


def count_colors(im):
    """Count the distinct opaque RGB swatches in a rendered sheet."""
    return len({p[:3] for p in im.getdata() if p[3] == 255})
