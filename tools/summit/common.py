"""Shared palette and helpers for the Summit art (tiles and props).

Every color used by a Summit sheet comes from PAL below (the warden ramps in
assets/concept/palettes.json, plus a few in-between floor tones noted in the manifests).
Images are numpy uint8 RGBA arrays, fully opaque or fully transparent.
"""

import json
import pathlib

import numpy as np
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPRITES = ROOT / "assets" / "sprites"
PREVIEWS = ROOT / "assets" / "previews"
SAMPLES = ROOT / "out" / "samples"


def hx(s):
    s = s.lstrip("#")
    return (int(s[0:2], 16), int(s[2:4], 16), int(s[4:6], 16))


# One character per color. Lowercase = floor (blue-gray cut stone), digits = warden stone
# (mauve rock), m-p = moss, G-K = gold, u-x = frost / night cloud, P-V = prism, k = outline.
PAL = {
    # floor ramp (cut flagstone, blue-gray)
    "a": "#10121e",
    "b": "#1e2236",
    "c": "#323a58",
    "d": "#4c5a80",
    "e": "#7486ac",
    "f": "#aab8d6",
    # in-between floor tones for slab-to-slab variety (adjusted for beauty)
    "D": "#435078",  # slightly darker slab
    "E": "#56658c",  # slightly lighter slab
    # warden stone ramp (natural rock, standing stones; mauve)
    "0": "#16101a",
    "1": "#2c2230",
    "2": "#46384a",
    "3": "#665468",
    "4": "#8c7a88",
    "5": "#b8a8ac",
    # in-between rock tone
    "9": "#3a2d3e",
    # moss
    "m": "#16262a",
    "n": "#2a4438",
    "o": "#4a6a48",
    "p": "#7a9a5a",
    # gold
    "G": "#3a2410",
    "H": "#7a4a18",
    "I": "#c0802a",
    "J": "#f0c050",
    "K": "#fff2b0",
    # frost / night cloud
    "t": "#2e3c78",
    "u": "#4c5ea0",
    "v": "#7a8cc8",
    "w": "#b4c4ea",
    "x": "#e4ecff",
    # prism
    "P": "#3a2a8a",
    "Q": "#6a5ad0",
    "R": "#5ff2ff",
    "S": "#ff5ad0",
    "T": "#ffd84a",
    "U": "#e8fcff",
    "V": "#ffffff",
    # outline (warden outline)
    "k": "#050414",
    # night sky (for previews / transparent-looking backdrops only)
    "y": "#090a1e",
    "z": "#04030c",
}
RGB = {k: hx(v) for k, v in PAL.items()}
TRANSPARENT = (".", " ")


def blank(w, h):
    return np.zeros((h, w, 4), np.uint8)


def put(img, x, y, ch):
    """Set one pixel to palette char `ch` ('.' clears it). Out of bounds is ignored."""
    h, w = img.shape[:2]
    if 0 <= x < w and 0 <= y < h:
        if ch in TRANSPARENT:
            img[y, x] = (0, 0, 0, 0)
        else:
            r, g, b = RGB[ch]
            img[y, x] = (r, g, b, 255)


def get(img, x, y):
    """Palette char at a pixel ('.' if transparent or outside)."""
    h, w = img.shape[:2]
    if not (0 <= x < w and 0 <= y < h) or img[y, x, 3] == 0:
        return "."
    return INV[tuple(int(v) for v in img[y, x, :3])]


INV = {v: k for k, v in RGB.items()}


def from_rows(rows):
    """Image from a list of palette-char strings (all the same width)."""
    rows = [r for r in rows]
    h = len(rows)
    w = max(len(r) for r in rows)
    img = blank(w, h)
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch not in TRANSPARENT:
                put(img, x, y, ch)
    return img


def parse_grid(text):
    """Parse a triple-quoted grid: strips the common indent and blank first/last lines."""
    lines = text.split("\n")
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    indent = min(len(l) - len(l.lstrip()) for l in lines if l.strip())
    return [l[indent:] for l in lines]


def to_rows(img):
    h, w = img.shape[:2]
    return ["".join(get(img, x, y) for x in range(w)) for y in range(h)]


def paste(dst, src, x, y):
    """Alpha-paste src onto dst at (x, y) (hard alpha)."""
    sh, sw = src.shape[:2]
    dh, dw = dst.shape[:2]
    for yy in range(sh):
        ty = y + yy
        if not 0 <= ty < dh:
            continue
        for xx in range(sw):
            tx = x + xx
            if 0 <= tx < dw and src[yy, xx, 3]:
                dst[ty, tx] = src[yy, xx]


def flip_h(img):
    return img[:, ::-1].copy()


def colors_used(img):
    a = img[img[..., 3] > 0][:, :3]
    return sorted({"#%02x%02x%02x" % tuple(c) for c in a})


def check_hard_alpha(img):
    a = img[..., 3]
    assert np.all((a == 0) | (a == 255)), "soft alpha found"


def save_sheet(name, img, manifest):
    check_hard_alpha(img)
    SPRITES.mkdir(parents=True, exist_ok=True)
    Image.fromarray(img, "RGBA").save(SPRITES / f"{name}.png")
    (SPRITES / f"{name}.json").write_text(json.dumps(manifest, indent=1) + "\n")


def load_sheet(name):
    img = np.array(Image.open(SPRITES / f"{name}.png").convert("RGBA"))
    man = json.loads((SPRITES / f"{name}.json").read_text())
    return img, man


def cell_of(img, man, col, row):
    cw, ch = man["cell"]
    return img[row * ch:(row + 1) * ch, col * cw:(col + 1) * cw].copy()


def scale(img, k):
    return np.repeat(np.repeat(img, k, axis=0), k, axis=1)


def flatten(img, bg):
    """Composite RGBA over a solid background color (rgb tuple or char) -> RGB array."""
    if isinstance(bg, str):
        bg = RGB[bg] if len(bg) == 1 else hx(bg)
    out = np.empty(img.shape[:2] + (3,), np.uint8)
    out[:] = bg
    m = img[..., 3] > 0
    out[m] = img[..., :3][m]
    return out


def over(dst_rgb, src, x, y):
    """Hard-alpha paste an RGBA image onto an RGB canvas."""
    sh, sw = src.shape[:2]
    dh, dw = dst_rgb.shape[:2]
    x0, y0 = max(0, x), max(0, y)
    x1, y1 = min(dw, x + sw), min(dh, y + sh)
    if x1 <= x0 or y1 <= y0:
        return
    s = src[y0 - y:y1 - y, x0 - x:x1 - x]
    m = s[..., 3] > 0
    d = dst_rgb[y0:y1, x0:x1]
    d[m] = s[..., :3][m]


def save_rgb(path, rgb, k=1):
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(scale(rgb, k) if k > 1 else rgb, "RGB").save(path)


def save_gif(path, frames_rgb, durations_ms, k=6):
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    ims = [Image.fromarray(scale(f, k), "RGB") for f in frames_rgb]
    ims[0].save(path, save_all=True, append_images=ims[1:], duration=durations_ms, loop=0,
                disposal=1, optimize=False)


# A rough 18 px stand-in for Glint (hero colors), used only in previews for scale until
# assets/sprites/hero.png exists.
GLINT_STANDIN = parse_grid("""
    ....kkkk....
    ...kHHHHk...
    ..kHHHHHHk..
    ..kHHHHHHk..
    ..kH#RR#Hk..
    ..kHH##HHk..
    .kZZZZZZZZk.
    kZYYZZZZZZYk
    kYZZHHHHZZZk
    .kZHHHHHHHk.
    .kHHHHHHHHk.
    .kHHHHHHHHk.
    .kHHHHHHHHk.
    .kHHHHHHHHk.
    ..kHHHHHHk..
    ..kHHkkHHk..
    ..kkk..kkk..
""")
_GLINT_PAL = {"k": "#0a0a24", "H": "#2c3a9a", "#": "#05051a", "R": "#8ff8ff",
              "Z": "#62c4f6", "Y": "#b6ecff"}


def glint_standin():
    """(rgba, anchor) for the hero: real hero.png idle frame if present, else the stand-in."""
    hero_png = SPRITES / "hero.png"
    hero_json = SPRITES / "hero.json"
    if hero_png.exists() and hero_json.exists():
        try:
            man = json.loads(hero_json.read_text())
            img = np.array(Image.open(hero_png).convert("RGBA"))
            row = man["anims"].get("idle_down", {"row": 0})["row"]
            cw, ch = man["cell"]
            cell = img[row * ch:(row + 1) * ch, 0:cw].copy()
            ax, ay = man.get("anchor", [cw // 2, ch - 1])
            if cell[..., 3].any():
                return cell, (ax, ay)
        except Exception:
            pass
    rows = GLINT_STANDIN
    h, w = len(rows), len(rows[0])
    img = blank(w, h)
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch != ".":
                img[y, x] = hx(_GLINT_PAL[ch]) + (255,)
    return img, (w // 2, h - 1)
