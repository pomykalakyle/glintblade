"""Zoomed view of a region with a pixel grid and coordinate labels, for placing details.

usage: gridview.py <module>:<func> out.png [x0 y0 x1 y1] [scale]
"""

import importlib
import sys

from PIL import Image, ImageDraw

from wlib import hex_rgb

spec, out = sys.argv[1], sys.argv[2]
mod, fn = spec.split(":")
args = []
if "(" in fn:
    fn, a = fn.split("(")
    args = [eval(v) for v in a.rstrip(")").split(",") if v]
cv = getattr(importlib.import_module(mod), fn)(*args)
img = cv.image()
x0, y0, x1, y1 = (0, 0, img.width, img.height)
if len(sys.argv) >= 7:
    x0, y0, x1, y1 = map(int, sys.argv[3:7])
s = int(sys.argv[7]) if len(sys.argv) >= 8 else 12
crop = img.crop((x0, y0, x1, y1))
bg = Image.new("RGBA", crop.size, hex_rgb("#323a58") + (255,))
bg.alpha_composite(crop)
big = bg.resize((crop.width * s, crop.height * s), Image.NEAREST)
pad = 22
canvas = Image.new("RGB", (big.width + pad, big.height + pad), (20, 20, 30))
canvas.paste(big, (pad, pad))
d = ImageDraw.Draw(canvas)
for i in range(crop.width + 1):
    x = pad + i * s
    col = (90, 90, 110) if (x0 + i) % 10 else (220, 220, 120)
    d.line([(x, pad), (x, pad + big.height)], fill=col, width=1)
    if (x0 + i) % 5 == 0 and i < crop.width:
        d.text((x + 1, 4), str(x0 + i), fill=(230, 230, 230))
for j in range(crop.height + 1):
    y = pad + j * s
    col = (90, 90, 110) if (y0 + j) % 10 else (220, 220, 120)
    d.line([(pad, y), (pad + big.width, y)], fill=col, width=1)
    if (y0 + j) % 5 == 0 and j < crop.height:
        d.text((1, y + 1), str(y0 + j), fill=(230, 230, 230))
canvas.save(out)
