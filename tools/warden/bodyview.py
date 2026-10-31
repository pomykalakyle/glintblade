import sys
from PIL import Image
import body
from wlib import hex_rgb
imgs = [body.body(s).image() for s in ("idle", "awake", "damaged")]
W, H = body.W, body.H
img = Image.new("RGBA", (W * 3 + 8, H), hex_rgb("#323a58") + (255,))
for i, im in enumerate(imgs):
    img.alpha_composite(im, (i * (W + 4), 0))
img.resize((img.width * 4, img.height * 4), Image.NEAREST).save(sys.argv[1])
