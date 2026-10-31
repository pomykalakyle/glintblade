"""Crop a band of anim rows from a sheet preview for review: crop.py name row0 row1 scale"""
import sys
from PIL import Image
from pix import PREVIEWS, SPRITES
from preview import ground, up, shadow
import json
name, r0, r1 = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
scale = int(sys.argv[4]) if len(sys.argv) > 4 else 6
im = Image.open(SPRITES / f"{name}.png").convert("RGBA")
man = json.load(open(SPRITES / f"{name}.json"))
cw, ch = man["cell"]
band = im.crop((0, r0 * ch, im.width, (r1 + 1) * ch))
bg = ground(band.width, band.height).convert("RGBA")
ax, ay = man["anchor"]
for r in range(r1 - r0 + 1):
    for c in range(im.width // cw):
        shadow(bg, c * cw + ax, r * ch + ay, 5, 2)
bg.alpha_composite(band)
# draw faint cell separators
big = up(bg, scale)
from PIL import ImageDraw
d = ImageDraw.Draw(big)
for c in range(1, im.width // cw):
    d.line((c * cw * scale, 0, c * cw * scale, big.height), fill=(20, 60, 50))
for r in range(1, r1 - r0 + 1):
    d.line((0, r * ch * scale, big.width, r * ch * scale), fill=(20, 60, 50))
out = PREVIEWS / f"_wip_crop_{name}_{r0}_{r1}.png"
big.convert("RGB").save(out)
print(out)
