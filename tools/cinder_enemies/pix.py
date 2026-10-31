"""Tiny pixel toolkit for the Cinder enemies (ember hound, spitting totem, ember glob).

Frames are hand-authored character grids. One char = one native pixel; '.' or ' ' is
transparent. Each sheet module maps chars to palette colors, builds frames, then calls
`write_sheet` which writes assets/sprites/<name>.png + .json and the previews.
"""

import json
import pathlib

import numpy as np
from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPRITES = ROOT / "assets" / "sprites"
PREVIEWS = ROOT / "assets" / "previews"

GROUND = "#363340"  # cinder ash ramp 2: the region's walkable ground
GROUND_DARK = "#292c3c"
GROUND_LIGHT = "#514854"


def rgb(h):
    """Rgb."""
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))


def grid(text):
    """Parse a char grid. Leading/trailing blank lines are dropped; rows keep their columns
    (a '|' at the start of a row is an optional left margin marker and is stripped)."""
    rows = text.split("\n")
    while rows and not rows[0].strip():
        rows.pop(0)
    while rows and not rows[-1].strip():
        rows.pop()
    rows = [r[1:] if r.startswith("|") else r for r in rows]
    w = max(len(r) for r in rows)
    return [r.ljust(w, ".") for r in rows]


class Canvas:
    """A cell-sized char canvas. Grids are stamped into it, then it is outlined and colored."""

    def __init__(self, w, h):
        """Init."""
        self.w, self.h = w, h
        self.c = [["." for _ in range(w)] for _ in range(h)]

    def stamp(self, g, x, y, transparent=".", only_empty=False):
        """Stamp."""
        if isinstance(g, str):
            g = grid(g)
        for j, row in enumerate(g):
            for i, ch in enumerate(row):
                if ch in (transparent, " "):
                    continue
                X, Y = x + i, y + j
                if 0 <= X < self.w and 0 <= Y < self.h:
                    if only_empty and self.c[Y][X] != ".":
                        continue
                    self.c[Y][X] = ch
        return self

    def get(self, x, y):
        """Get."""
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.c[y][x]
        return "."

    def set(self, x, y, ch):
        """Set."""
        if 0 <= x < self.w and 0 <= y < self.h:
            self.c[y][x] = ch

    def outline(self, ch="#", skip="#", diag=False):
        """Add an outline `ch` on empty pixels 4-adjacent (or 8 with diag) to filled pixels
        whose char is not in `skip` (hand-drawn outline chars don't get a second ring)."""
        add = []
        n4 = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        n8 = n4 + [(1, 1), (1, -1), (-1, 1), (-1, -1)]
        for y in range(self.h):
            for x in range(self.w):
                if self.c[y][x] != ".":
                    continue
                for dx, dy in (n8 if diag else n4):
                    v = self.get(x + dx, y + dy)
                    if v != "." and v not in skip:
                        add.append((x, y))
                        break
        for x, y in add:
            self.c[y][x] = ch
        return self

    def remap(self, mapping):
        """Remap."""
        for y in range(self.h):
            for x in range(self.w):
                v = self.c[y][x]
                if v in mapping:
                    self.c[y][x] = mapping[v]
        return self

    def shift(self, dx, dy):
        """Shift."""
        n = Canvas(self.w, self.h)
        for y in range(self.h):
            for x in range(self.w):
                v = self.c[y][x]
                if v != ".":
                    n.set(x + dx, y + dy, v)
        return n

    def flip(self):
        """Flip."""
        n = Canvas(self.w, self.h)
        for y in range(self.h):
            n.c[y] = list(reversed(self.c[y]))
        return n

    def copy(self):
        """Copy."""
        n = Canvas(self.w, self.h)
        n.c = [r[:] for r in self.c]
        return n

    def text(self):
        """Text."""
        return "\n".join("".join(r) for r in self.c)

    def to_rgba(self, pal):
        """To rgba."""
        a = np.zeros((self.h, self.w, 4), np.uint8)
        for y in range(self.h):
            for x in range(self.w):
                v = self.c[y][x]
                if v == ".":
                    continue
                if v not in pal:
                    raise KeyError(f"char {v!r} at {x},{y} has no color")
                a[y, x, :3] = rgb(pal[v])
                a[y, x, 3] = 255
        return a


def count_colors(arrs):
    """Count colors."""
    s = set()
    for a in arrs:
        m = a[..., 3] > 0
        for p in a[m][:, :3]:
            s.add(tuple(int(v) for v in p))
    return s


def write_sheet(name, cell, anchor, anims, pal, emissive, extra=None, preview_bg=GROUND, scale=8,
                ref=None, lit=None):
    """anims: list of (anim_name, [Canvas...], meta dict with fps/durations/anchor...).
    Each anim gets its own row. Writes the PNG + JSON manifest and previews."""
    cw, ch = cell
    ncols = max(len(fr) for _, fr, _ in anims)
    sheet = np.zeros((ch * len(anims), cw * ncols, 4), np.uint8)
    manifest = {"cell": [cw, ch], "anchor": list(anchor), "emissive": emissive, "anims": {}}
    frames_rgba = {}
    for row, (an, frames, meta) in enumerate(anims):
        arrs = []
        for col, cv in enumerate(frames):
            a = cv.to_rgba(pal)
            sheet[row * ch:(row + 1) * ch, col * cw:(col + 1) * cw] = a
            arrs.append(a)
        frames_rgba[an] = (arrs, meta)
        m = {"row": row, "frames": len(frames)}
        m.update({k: v for k, v in meta.items() if not k.startswith("_")})
        manifest["anims"][an] = m
    if extra:
        manifest.update(extra)
    SPRITES.mkdir(parents=True, exist_ok=True)
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    Image.fromarray(sheet, "RGBA").save(SPRITES / f"{name}.png")
    with open(SPRITES / f"{name}.json", "w") as f:
        json.dump(manifest, f, indent=1)
        f.write("\n")
    colors = count_colors([sheet])
    print(f"{name}: sheet {sheet.shape[1]}x{sheet.shape[0]}, {len(colors)} colors, {len(anims)} anims")
    # alpha must be binary
    assert set(np.unique(sheet[..., 3])) <= {0, 255}
    preview_sheet(name, sheet, cell, anims, preview_bg, scale)
    for an, (arrs, meta) in frames_rgba.items():
        gif(f"{name}_{an}", arrs, meta, anchor_of(meta, anchor), preview_bg, scale, ref)
    return sheet, frames_rgba


def anchor_of(meta, anchor):
    """Anchor of."""
    return tuple(meta.get("anchor", anchor))


def ticks_of(meta, n):
    """Ticks of."""
    if "durations" in meta:
        return meta["durations"]
    t = max(1, round(60 / meta.get("fps", 8)))
    return [t] * n


def preview_sheet(name, sheet, cell, anims, bg, scale):
    """Preview sheet."""
    cw, ch = cell
    h, w = sheet.shape[:2]
    img = Image.new("RGB", (w, h), rgb(bg))
    img.paste(Image.fromarray(sheet, "RGBA"), (0, 0), Image.fromarray(sheet, "RGBA"))
    big = img.resize((w * scale, h * scale), Image.NEAREST)
    d = ImageDraw.Draw(big)
    grid_c = tuple(max(0, v - 14) for v in rgb(bg))
    for x in range(0, w + 1, cw):
        d.line([(x * scale, 0), (x * scale, h * scale)], fill=grid_c)
    for y in range(0, h + 1, ch):
        d.line([(0, y * scale), (w * scale, y * scale)], fill=grid_c)
    # label column
    lab_w = 150
    out = Image.new("RGB", (big.width + lab_w, big.height), rgb("#1c0e1c"))
    out.paste(big, (lab_w, 0))
    d = ImageDraw.Draw(out)
    for row, (an, _, _) in enumerate(anims):
        d.text((6, row * ch * scale + 6), an, fill=(230, 210, 200))
    out.save(PREVIEWS / f"{name}_sheet_{scale}x.png")


def gif(fname, arrs, meta, anchor, bg, scale, ref=None):
    """Animated GIF of one anim over the ground color. `ref` = (rgba array, anchor) of a
    scale figure drawn to the left."""
    cw, ch = arrs[0].shape[1], arrs[0].shape[0]
    pad = 4
    ref_w = 0
    if ref is not None:
        ref_w = ref[0].shape[1] + 2
    W = cw + ref_w + pad * 2
    H = ch + pad * 2
    frames = []
    for a in arrs:
        img = Image.new("RGB", (W, H), rgb(bg))
        if ref is not None:
            ra, (rax, ray) = ref
            # place ref so its anchor is on the same ground row as the sprite anchor
            ox = pad - 2
            oy = pad + anchor[1] - ray
            im = Image.fromarray(ra, "RGBA")
            img.paste(im, (ox, oy), im)
        im = Image.fromarray(a, "RGBA")
        img.paste(im, (pad + ref_w, pad), im)
        frames.append(img.resize((W * scale, H * scale), Image.NEAREST))
    durs = [round(t * 1000 / 60) for t in ticks_of(meta, len(arrs))]
    durs = [max(20, d) for d in durs]
    frames[0].save(PREVIEWS / f"{fname}.gif", save_all=True, append_images=frames[1:],
                   duration=durs, loop=0, disposal=1)


def zoom_png(arr, path, scale=8, bg=GROUND):
    """Zoom png."""
    h, w = arr.shape[:2]
    img = Image.new("RGB", (w, h), rgb(bg))
    im = Image.fromarray(arr, "RGBA")
    img.paste(im, (0, 0), im)
    img.resize((w * scale, h * scale), Image.NEAREST).save(path)


def strip(arrs, path, scale=8, bg=GROUND, gap=2):
    """All frames side by side, zoomed: for quick looking while authoring."""
    h = max(a.shape[0] for a in arrs)
    w = sum(a.shape[1] for a in arrs) + gap * (len(arrs) - 1)
    img = Image.new("RGB", (w, h), rgb(bg))
    x = 0
    for a in arrs:
        im = Image.fromarray(a, "RGBA")
        img.paste(im, (x, 0), im)
        x += a.shape[1] + gap
    img.resize((w * scale, h * scale), Image.NEAREST).save(path)
