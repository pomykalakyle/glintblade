"""Quick look: render canvases side by side at 1x and 8x over ash ground, plus the scale figure."""
import sys
import numpy as np
from PIL import Image
from lib import over, scale, figure18, WORK, GROUND_BG


def strip(imgs, bg=GROUND_BG, gap=4, fig=True):
    if fig:
        imgs = imgs + [figure18()]
    h = max(i.shape[0] for i in imgs)
    w = sum(i.shape[1] for i in imgs) + gap * (len(imgs) + 1)
    out = np.zeros((h + 2 * gap, w, 4), np.uint8)
    x = gap
    for i in imgs:
        y = gap + h - i.shape[0]
        sub = out[y:y + i.shape[0], x:x + i.shape[1]]
        m = i[..., 3] > 0
        sub[m] = i[m]
        x += i.shape[1] + gap
    return out


def show(imgs, name, s=8, bg=GROUND_BG, fig=True):
    st = strip(imgs, bg, fig=fig)
    rgb = over(st, bg)
    big = scale(rgb, s)
    two = scale(rgb, 2)
    W = big.shape[1]
    top = np.zeros((two.shape[0] + 4, W, 3), np.uint8)
    top[:] = rgb[0, 0]
    top[:rgb.shape[0], :rgb.shape[1]] = rgb
    x = rgb.shape[1] + 8
    top[:two.shape[0], x:x + two.shape[1]] = two
    full = np.concatenate([top, big], 0)
    p = WORK / f"_wip_{name}.png"
    Image.fromarray(full).save(p)
    return p
