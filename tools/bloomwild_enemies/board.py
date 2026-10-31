"""Render a quick test board of canvases at high zoom over grass (for iteration)."""

import sys

from PIL import Image

from pix import PREVIEWS
from preview import ground, shadow, up, glint


def board(canvases, pal, path, scale=10, anchors=None, with_glint=True, per_row=5):
    if len(canvases) > per_row:
        parts = []
        for i in range(0, len(canvases), per_row):
            pp = str(path).replace(".png", f"_{i // per_row}.png")
            board(canvases[i:i + per_row], pal, pp, scale, anchors[i:i + per_row] if anchors else None, with_glint, per_row)
            parts.append(pp)
        return parts
    imgs = [c.image(pal) for c in canvases]
    cw = max(i.width for i in imgs) + 4
    ch = max(i.height for i in imgs) + 4
    g, ga = glint()
    n = len(imgs) + (1 if with_glint else 0)
    bg = ground(cw * n, ch).convert("RGBA")
    k = 0
    if with_glint:
        gx, gy = cw // 2, ch - 4
        bg.alpha_composite(g, (gx - ga[0], gy - ga[1]))
        k = 1
    for i, im in enumerate(imgs):
        ax, ay = anchors[i] if anchors else (im.width // 2, im.height - 2)
        gx, gy = (i + k) * cw + cw // 2, ch - 4
        shadow(bg, gx, gy, 6, 2)
        bg.alpha_composite(im, (gx - ax, gy - ay))
    # 1x strip under the zoomed one
    big = up(bg, scale)
    out = Image.new("RGB", (big.width, big.height + bg.height * 2 + 8), (16, 14, 30))
    out.paste(big.convert("RGB"), (0, 0))
    out.paste(up(bg, 2).convert("RGB"), (0, big.height + 8))
    out.paste(bg.convert("RGB"), (bg.width * 2 + 8, big.height + 8))
    out.save(path)
    return path
