import sys
from PIL import Image
import head
from wlib import hex_rgb
rows = []
for name, n in head.ANIMS:
    rows.append([head.head(name, i).image() for i in range(n)])
W, H = head.W, head.H
img = Image.new("RGBA", (W * 6, H * len(rows)), hex_rgb("#323a58") + (255,))
for j, r in enumerate(rows):
    for i, f in enumerate(r):
        img.alpha_composite(f, (i * W, j * H))
img.resize((img.width * 5, img.height * 5), Image.NEAREST).save(sys.argv[1])
