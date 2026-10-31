"""Previews for the Glasswater enemy sheets: sheet PNG, animated board GIFs (flat + lit cave sim),
and a scale comparison next to Glint (assets/sprites/hero.png if present, else a rough 18 px figure)."""

import json
import os

import numpy as np
from PIL import Image, ImageDraw

from pix import (
    GROUND,
    PREVIEWS,
    SPRITES,
    Grid,
    _bg,
    emissive_mask,
    frame_durations,
    gif,
    hex2rgb,
    lit,
    over,
    sheet_preview,
    upscale,
)

LABEL = (150, 160, 215)

# Rough 18 px hooded figure for scale when the real hero sheet doesn't exist yet.
FIGURE = Grid.parse("""
....OOOO....
...O1222O...
..O122222O..
..O1OOOO2O..
..O1OeOe2O..
..O12OO22O..
..OS1222SO..
.OSS22222SO.
.O1222222SO.
.O12222222O.
.O12222222O.
.O11222222O.
..O1122222O.
..O1112222O.
..O11111O2O.
...OO1O1OO..
....O.O.O...
....O...O...
""")
FIGURE_PAL = {"O": "#0a0a24", "1": "#11113a", "2": "#2c3a9a", "S": "#62c4f6", "e": "#8ff8ff"}


def hero_cell():
    """(rgba array, anchor, emissive hex list) of Glint's first idle frame, or the rough figure."""
    png = os.path.join(SPRITES, "hero.png")
    js = os.path.join(SPRITES, "hero.json")
    if os.path.exists(png) and os.path.exists(js):
        try:
            man = json.load(open(js))
            img = np.asarray(Image.open(png).convert("RGBA"))
            cw, ch = man["cell"]
            anims = man.get("anims", {})
            a = anims.get("idle_down") or anims.get("idle_right") or next(iter(anims.values()))
            r = a["row"]
            cell = img[r * ch : (r + 1) * ch, 0:cw].copy()
            anc = tuple(a.get("anchor", man["anchor"]))
            return cell, anc, man.get("emissive", []), True
        except Exception as e:  # noqa: BLE001 - preview only; fall back to the figure
            print("hero sheet unreadable, using rough figure:", e)
    g = FIGURE
    return g.to_rgba(FIGURE_PAL), (6, 17), ["#8ff8ff"], False


def cell_of(sheet_rgba, sheet, row, i):
    """Read a single cell from a rendered sheet array."""
    return sheet_rgba[row * sheet.ch : (row + 1) * sheet.ch, i * sheet.cw : (i + 1) * sheet.cw]


def frame_at(meta, n, tick):
    """Choose a looping frame from the authored duration sequence."""
    d = frame_durations(meta, n)
    t = tick % sum(d)
    for i, x in enumerate(d):
        if t < x:
            return i
        t -= x
    return n - 1


def board(sheet_rgba, sheet, path_flat, path_lit, scale=6, cols=None, ticks=120, step=2, names=None):
    """Animated board: every anim plays in its own slot (looping at its real timing)."""
    rows = [(i, a, f, m) for i, (a, f, m) in enumerate(sheet.rows) if names is None or a in names]
    cols = cols or min(5, len(rows))
    pad = 4
    lab = 9  # label strip in native px (drawn at scale)
    sw, sh = sheet.cw + pad, sheet.ch + pad + 2
    W = cols * sw + pad
    H = ((len(rows) + cols - 1) // cols) * sh + pad
    em_hex = sheet.emissive_hex()
    flat_frames, lit_frames, durs = [], [], []
    for tick in range(0, ticks, step):
        canvas = np.zeros((H, W, 4), np.uint8)
        for k, (r, a, frames, meta) in enumerate(rows):
            cx, cy = pad + (k % cols) * sw, pad + (k // cols) * sh + 2
            i = frame_at(meta, len(frames), tick)
            over(canvas, cell_of(sheet_rgba, sheet, r, i), cx, cy)
        flat = _bg(W, H, GROUND)
        over(flat, canvas, 0, 0)
        big = upscale(flat, scale)[..., :3].copy()
        dark = _bg(W, H, "#29313f")
        over(dark, canvas, 0, 0)
        m = emissive_mask(dark, em_hex)
        # each slot gets its own sword-ish light from the upper left (as if Glint were nearby)
        lights = []
        own = getattr(sheet, "light", None)
        for k in range(len(rows)):
            cx, cy = pad + (k % cols) * sw, pad + (k // cols) * sh + 2
            lights.append(((cx + 4, cy + 4), "#9ff4ff", 26, 0.8))
            if own:  # the sprite's own declared light (manifest "light"), at its anchor + offset
                ax, ay = sheet.anchor
                lights.append(((cx + ax + own["offset"][0], cy + ay + own["offset"][1]), own["color"], own["radius"], own.get("intensity", 0.8)))
        litimg = lit(dark, m, extra_lights=lights, scale=scale)
        for im in (big, litimg):
            pil = Image.fromarray(im)
            d = ImageDraw.Draw(pil)
            for k, (r, a, frames, meta) in enumerate(rows):
                cx, cy = pad + (k % cols) * sw, pad + (k // cols) * sh
                d.text((cx * scale + 2, cy * scale - 2), a, fill=LABEL)
            im[...] = np.asarray(pil)
        flat_frames.append(big)
        lit_frames.append(litimg)
        durs.append(step)
    gif(flat_frames, durs, path_flat)
    gif(lit_frames, durs, path_lit)
    return flat_frames[0], lit_frames[0]


def scale_compare(sheet_rgba, sheet, picks, path, scale=6):
    """The sheet's frames next to Glint, standing on the same ground line, flat and lit."""
    hero, hanc, hem, real = hero_cell()
    pad = 6
    items = [(hero, hanc)] + [(cell_of(sheet_rgba, sheet, r, i), tuple(sheet.rows[r][2].get("anchor", sheet.anchor))) for r, i in picks]
    ground_y = max(a[1] for _, a in items) + pad + 2
    W = sum(c.shape[1] for c, _ in items) + pad * (len(items) + 1)
    H = ground_y + 8
    canvas = np.zeros((H, W, 4), np.uint8)
    x = pad
    for c, (ax, ay) in items:
        over(canvas, c, x, ground_y - ay)
        x += c.shape[1] + pad
    flat = _bg(W, H, GROUND)
    over(flat, canvas, 0, 0)
    dark = _bg(W, H, "#29313f")
    over(dark, canvas, 0, 0)
    m = emissive_mask(dark, sheet.emissive_hex() + list(hem))
    hx = pad + hero.shape[1] // 2
    # Glint's idle blade light (player.rs: radius 22, #7cf4ff, 0.35) plus each sprite's own light
    lights = [((hx, ground_y - 6), "#7cf4ff", 22, 0.35)]
    own = getattr(sheet, "light", None)
    if own:
        x = pad + hero.shape[1] + pad
        for c, (ax, ay) in items[1:]:
            lights.append(((x + ax + own["offset"][0], ground_y + own["offset"][1]), own["color"], own["radius"], own.get("intensity", 0.8)))
            x += c.shape[1] + pad
    litimg = lit(dark, m, extra_lights=lights, scale=scale)
    img = np.concatenate([upscale(flat, scale)[..., :3], litimg], 0)
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    d.text((4, 2), "Glint (hero.png)" if real else "rough 18px figure (hero.png not found)", fill=LABEL)
    pil.save(path)
    return img


def write_all(sheet, sheet_rgba, picks, board_cols=None, board_ticks=120, scale=8):
    """Write source, motion and scale evidence under the family sample directory."""
    os.makedirs(PREVIEWS, exist_ok=True)
    n = sheet.name
    sheet_preview(sheet_rgba, sheet, os.path.join(PREVIEWS, f"{n}_sheet_8x.png"), scale=scale)
    board(sheet_rgba, sheet, os.path.join(PREVIEWS, f"{n}_anims.gif"), os.path.join(PREVIEWS, f"{n}_anims_lit.gif"), scale=6, cols=board_cols, ticks=board_ticks)
    scale_compare(sheet_rgba, sheet, picks, os.path.join(PREVIEWS, f"{n}_scale_6x.png"), scale=6)
