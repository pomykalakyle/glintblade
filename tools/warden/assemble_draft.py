import sys

from PIL import Image

import body
import fist
import head
from wlib import hex_rgb

out = sys.argv[1]
NECK = (60, 34)  # head anchor in body cell coords
FIST_R = (100, 98)  # fist anchor (bottom of knuckles) in body cell coords
b = body.body().image()
h = head.head("idle", 1).image()
f = fist.fist("clenched", 0).image()
fl = f.transpose(Image.FLIP_LEFT_RIGHT)

W, H = 240, 150
base = Image.new("RGBA", (W, H), hex_rgb("#323a58") + (255,))
gx, gy = 120, 140
ox, oy = gx - body.ANCHOR[0], gy - body.ANCHOR[1]
base.alpha_composite(b, (ox, oy))
base.alpha_composite(h, (ox + NECK[0] - head.ANCHOR[0], oy + NECK[1] - head.ANCHOR[1]))
for side, img in ((1, f), (-1, fl)):
    fx = FIST_R[0] if side > 0 else 120 - FIST_R[0]
    ax = fist.ANCHOR[0] if side > 0 else fist.W - 1 - fist.ANCHOR[0]
    base.alpha_composite(img, (ox + fx - ax, oy + FIST_R[1] - fist.ANCHOR[1]))
for y in range(18):
    for x in range(8):
        if (x - 3.5) ** 2 / 16 + (y - 9) ** 2 / 81 <= 1:
            base.putpixel((40 + x, 122 + y), hex_rgb("#2c3a9a") + (255,))
base.save(out.replace(".png", "_1x.png"))
base.resize((W * 4, H * 4), Image.NEAREST).save(out)
