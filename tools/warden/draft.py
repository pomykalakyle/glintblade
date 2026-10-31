import sys

from PIL import Image

import body
from wlib import on_bg

out = sys.argv[1]
b = body.body()
img = b.image()
on_bg(img, "#323a58", 6, pad=4).save(out)
# 1x composite on a 192x150 patch of the summit floor
base = Image.new("RGBA", (192, 140), (0x32, 0x3a, 0x58, 255))
base.alpha_composite(img, (36, 20))
base.resize((192 * 3, 140 * 3), Image.NEAREST).save(out.replace(".png", "_1x3.png"))
base.save(out.replace(".png", "_1x.png"))
