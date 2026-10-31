"""Shared helpers for the GLINTBLADE UI art (fonts, logo, HUD).

Everything is authored as character grids on the native pixel grid. Images are numpy
RGBA uint8 arrays; alpha is only ever 0 or 255.
"""

import json
import os

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SPRITES = os.path.join(ROOT, "assets", "sprites")
PREVIEWS = os.path.join(ROOT, "assets", "previews")
SAMPLES = os.path.join(ROOT, "out", "samples")

# Style v2 (2026-10-30): all UI art uses the v2 palettes.
with open(os.path.join(ROOT, "assets", "concept", "palettes_v2.json")) as f:
    PAL = json.load(f)

UI = PAL["ui"]
HERO = PAL["hero"]
FX = PAL["fx"]
REG = PAL["regions"]

# Region ground colors used as preview backgrounds.
GROUND = {
    "bloomwild": REG["bloomwild"]["ramps"]["grass"][2],
    "glasswater": REG["glasswater"]["ramps"]["rock"][2],
    "cinder": REG["cinder"]["ramps"]["ash"][2],
    "warden": REG["warden"]["ramps"]["floor"][1],
    "ui": UI["panel"][0],
}


def rgb(h):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))


def new(w, h):
    return np.zeros((h, w, 4), np.uint8)


def grid(rows):
    """Normalize a list of strings to equal width (pads right with '.')."""
    w = max(len(r) for r in rows)
    return [r.ljust(w, ".") for r in rows]


def stamp(img, rows, key, x0, y0):
    """Draw a char grid into img. key maps char -> hex color; other chars are skipped."""
    H, W = img.shape[:2]
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            c = key.get(ch)
            if c is None:
                continue
            px, py = x0 + x, y0 + y
            if 0 <= px < W and 0 <= py < H:
                img[py, px, :3] = rgb(c)
                img[py, px, 3] = 255


def put(img, x, y, color):
    H, W = img.shape[:2]
    if 0 <= x < W and 0 <= y < H:
        img[y, x, :3] = rgb(color)
        img[y, x, 3] = 255


def blit(dst, src, x0, y0):
    """Alpha-test blit (src alpha 255 wins)."""
    h, w = src.shape[:2]
    H, W = dst.shape[:2]
    for y in range(h):
        for x in range(w):
            if src[y, x, 3] and 0 <= x0 + x < W and 0 <= y0 + y < H:
                dst[y0 + y, x0 + x] = src[y, x]


def save_png(img, path):
    assert set(np.unique(img[:, :, 3])).issubset({0, 255}), "semi-transparent pixel"
    # Clear color of transparent pixels so the PNG is clean.
    out = img.copy()
    out[out[:, :, 3] == 0] = 0
    Image.fromarray(out, "RGBA").save(path)


def save_json(obj, path):
    with open(path, "w") as f:
        json.dump(obj, f, indent=1)
        f.write("\n")


def flat(img, bg):
    """Composite over a solid background color -> RGB array."""
    out = np.zeros(img.shape[:2] + (3,), np.uint8)
    out[:] = rgb(bg)
    m = img[:, :, 3] > 0
    out[m] = img[m][:, :3]
    return out


def scale(arr, k):
    return np.repeat(np.repeat(arr, k, axis=0), k, axis=1)


def save_preview(rgb_arr, path, k=6):
    """Write an integer-scaled preview, creating its output directory when needed."""
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    Image.fromarray(scale(rgb_arr, k), "RGB").save(path)


def save_gif(frames_rgb, path, k=6, durations_ms=100):
    ims = [Image.fromarray(scale(f, k), "RGB") for f in frames_rgb]
    if isinstance(durations_ms, int):
        durations_ms = [durations_ms] * len(ims)
    # Quantize each frame to its exact (small) palette; no dithering.
    pims = [im.convert("P", palette=Image.ADAPTIVE, colors=255, dither=Image.NONE) for im in ims]
    pims[0].save(path, save_all=True, append_images=pims[1:], duration=durations_ms, loop=0, disposal=1, optimize=False)


# Rough 18 px hero stand-in for scale (from tools/art/swatches.py's palette key mock).
HERO_MOCK = [
    ".....OOOO.......",
    "....OeeddO......",
    "...OeddcccO.....",
    "..OeddccccbO....",
    "..OddvvvvcbO....",
    "..OdvvvvvvbO....",
    "..OdvEvvEvbO....",
    "..OcvvvvvvaO....",
    ".OwwwuuuuuttO...",
    ".OdwwuuuuttsO...",
    "OddcBcccccbbaO..",
    "OdccccccccbhaO..",
    "OdcccccccbbbaOy.",
    "OdccccccbbbbaOx.",
    ".OccccbbbbbaaO.x",
    ".OccbbbbbbbaaO.x",
    "..OOgOOOOOgOO..y",
    "...OhgO..OhgO...",
]
HERO_KEY = {
    "O": HERO["outline"][0], "v": HERO["hood_void"][0],
    "a": HERO["cloak"][0], "b": HERO["cloak"][1], "c": HERO["cloak"][2],
    "d": HERO["cloak"][3], "e": HERO["cloak"][-1],
    "s": HERO["scarf"][0], "t": HERO["scarf"][1], "u": HERO["scarf"][2], "w": HERO["scarf"][-1],
    "g": HERO["gear"][1], "h": HERO["gear"][2], "B": HERO["brass"][1],
    "E": HERO["eyes"][0], "x": HERO["blade"][1], "y": HERO["blade"][2],
}


def hero_img():
    """Glint's first idle frame if the hero sheet exists, else the rough mock."""
    p = os.path.join(SPRITES, "hero.png")
    j = os.path.join(SPRITES, "hero.json")
    if os.path.exists(p) and os.path.exists(j):
        try:
            sheet = np.array(Image.open(p).convert("RGBA"))
            man = json.load(open(j))
            cw, ch = man["cell"]
            cell = sheet[0:ch, 0:cw].copy()
            cell[:, :, 3] = np.where(cell[:, :, 3] > 127, 255, 0)
            if cell[:, :, 3].any():
                ys, xs = np.nonzero(cell[:, :, 3])
                return cell[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
        except Exception:
            pass
    rows = grid(HERO_MOCK)
    img = new(len(rows[0]), len(rows))
    stamp(img, rows, HERO_KEY, 0, 0)
    return img
