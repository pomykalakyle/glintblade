"""zoom.py sheet anim:frame[:flip] ... -> one PNG at 12x over grass, cells side by side"""
import sys, json
from PIL import Image, ImageDraw
from pix import PREVIEWS, SPRITES
from preview import ground, up, shadow, cell, anchor
name = sys.argv[1]
scale = 12
im = Image.open(SPRITES / f"{name}.png").convert("RGBA")
man = json.load(open(SPRITES / f"{name}.json"))
cw, ch = man["cell"]
specs = sys.argv[2:]
bg = ground(cw * len(specs), ch).convert("RGBA")
for k, sp in enumerate(specs):
    an, fr = sp.split(":")[:2]
    c = cell(im, man, an, int(fr))
    a = anchor(man, an)
    shadow(bg, k * cw + a[0], a[1], 5, 2)
    bg.alpha_composite(c, (k * cw, 0))
big = up(bg, scale)
d = ImageDraw.Draw(big)
for k in range(1, len(specs)):
    d.line((k * cw * scale, 0, k * cw * scale, big.height), fill=(10, 40, 30), width=2)
out = PREVIEWS / f"_wip_zoom_{name}.png"
big.convert("RGB").save(out)
print(out)
