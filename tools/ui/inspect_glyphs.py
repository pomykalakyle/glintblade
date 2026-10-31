"""Zoomed glyph inspector with a pixel grid: inspect_glyphs.py <font> <chars> <out.png> [scale]"""

import sys

import numpy as np
from PIL import Image

import fonts

name, chars, out = sys.argv[1], sys.argv[2], sys.argv[3]
k = int(sys.argv[4]) if len(sys.argv) > 4 else 10
f = getattr(fonts, name)()
pad = 2
ws = [f.masks[c].shape[1] for c in chars]
W = sum(ws) + pad * (len(chars) + 1)
H = f.cell_h + 2 * pad
img = np.zeros((H * k, W * k, 3), np.uint8)
img[:] = (30, 34, 54)
x = pad
for c in chars:
    m = f.masks[c]
    for gy in range(m.shape[0]):
        for gx in range(m.shape[1]):
            col = (242, 238, 255) if m[gy, gx] else (44, 50, 78)
            y0, x0 = (pad + gy) * k, (x + gx) * k
            img[y0:y0 + k, x0:x0 + k] = col
    x += m.shape[1] + pad
# grid
img[::k, :] = (img[::k, :] * 0.7).astype(np.uint8)
img[:, ::k] = (img[:, ::k] * 0.7).astype(np.uint8)
# baseline marker
by = (pad + f.baseline + 1) * k
img[by:by + 1, :] = (255, 80, 110)
Image.fromarray(img).save(out)
