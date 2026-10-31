"""Build every cinder_* sheet into assets/sprites and its previews into assets/previews.

Run: uv run --no-project --with pillow --with numpy python3 build_all.py [sheet ...]
"""

import importlib
import json
import sys

import numpy as np
from PIL import Image

from lib import PREVIEWS, GROUND_BG, figure18, over, scale, save_gif, SPRITES, EMISSIVE, colors_in

MODULES = ["pillars", "statue", "brazier", "trees", "props_m", "props", "ruins", "tiles"]


def hero_or_figure():
    """Draw the hero or figure on the native Cinder pixel grid."""
    p = SPRITES / "hero.png"
    j = SPRITES / "hero.json"
    if p.exists() and j.exists():
        meta = json.loads(j.read_text())
        im = np.array(Image.open(p).convert("RGBA"))
        cw, ch = meta["cell"]
        cell = im[:ch, :cw].copy()
        # crop to content
        ys, xs = np.nonzero(cell[..., 3])
        if len(ys):
            return cell[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    return figure18()


def scale_strip(frames, gap=6):
    """Frame 0 of each anim, bottoms aligned, plus the hero for scale."""
    fig = hero_or_figure()
    items = frames + [fig]
    crops = []
    for f in items:
        ys, xs = np.nonzero(f[..., 3])
        crops.append(f[: ys.max() + 1, xs.min(): xs.max() + 1] if len(ys) else f)
    h = max(c.shape[0] for c in crops)
    w = sum(c.shape[1] for c in crops) + gap * (len(crops) + 1)
    out = np.zeros((h + 2 * gap, w, 4), np.uint8)
    x = gap
    for c in crops:
        y = gap + h - c.shape[0]
        m = c[..., 3] > 0
        out[y:y + c.shape[0], x:x + c.shape[1]][m] = c[m]
        x += c.shape[1] + gap
    return out


def previews(sb, img, anims):
    """Return the previews representation for the Cinder generator."""
    name = sb.name
    # whole sheet, 6x, faint cell grid
    rgb = over(img, GROUND_BG)
    big = scale(rgb, 6).copy()
    for x in range(0, big.shape[1], sb.cw * 6):
        big[:, x] = (big[:, x] * 0.7).astype(np.uint8)
    for y in range(0, big.shape[0], sb.ch * 6):
        big[y, :] = (big[y, :] * 0.7).astype(np.uint8)
    Image.fromarray(big).save(PREVIEWS / f"{name}.png")
    # scale strip: 1x and 6x
    firsts = [frames[0] for _, frames, _ in sb.rows]
    st = scale_strip(firsts)
    st_rgb = over(st, GROUND_BG)
    Image.fromarray(scale(st_rgb, 6)).save(PREVIEWS / f"{name}_scale.png")
    # gifs for animated anims
    for anim, frames, meta in sb.rows:
        if len(frames) > 1:
            dur = meta.get("durations")
            if not dur:
                dur = [round(60 / meta.get("fps", 8))] * len(frames)
            save_gif(frames, f"{name}_{anim}", s=8, durations_ticks=dur)


def main(names):
    """Build the requested Cinder sheets and inspection previews."""
    for mod in names:
        m = importlib.import_module("v2style" if mod in ("pillars", "statue", "ruins") else mod)
        if mod in ("pillars", "statue", "ruins"):
            sheets = [m.build(mod)]
        else:
            sheets = m.build_all() if hasattr(m, "build_all") else [m.build()]
        for sb in sheets:
            notes = getattr(sb, "notes", None) or getattr(m, "NOTES", None)
            img, anims = sb.save(emissive=getattr(m, "EMISSIVE_USED", None) or default_emissive(sb),
                                 extra={"notes": notes} if notes else None)
            previews(sb, img, anims)
            print(f"{sb.name}: {img.shape[1]}x{img.shape[0]}, {len(anims)} anims")


def default_emissive(sb):
    """Select only declared heat swatches present in this sheet."""
    img, _ = sb.build()
    used = colors_in(img)
    em = [c for c in EMISSIVE if c in used]
    return em or None


if __name__ == "__main__":
    main(sys.argv[1:] or MODULES)
