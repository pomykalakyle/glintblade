"""zoom.py SHEET anim:frame [anim:frame ...] -> work/zoom.png (12x flat + lit + 1x/3x)."""
import json, os, sys
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(__file__))
from pix import SPRITES, GROUND, _bg, over, upscale, emissive_mask, lit, hex2rgb

name = sys.argv[1]
man = json.load(open(os.path.join(SPRITES, name + ".json")))
img = np.asarray(Image.open(os.path.join(SPRITES, name + ".png")).convert("RGBA"))
cw, ch = man["cell"]
cells = []
for spec in sys.argv[2:]:
    a, i = spec.split(":")
    r = man["anims"][a]["row"]
    cells.append(img[r * ch:(r + 1) * ch, int(i) * cw:(int(i) + 1) * cw])
pad = 2
W = len(cells) * (cw + pad) + pad
H = ch + 2 * pad
canvas = np.zeros((H, W, 4), np.uint8)
for k, c in enumerate(cells):
    over(canvas, c, pad + k * (cw + pad), pad)
S = int(os.environ.get("S", 12))
flat = _bg(W, H, GROUND); over(flat, canvas, 0, 0)
dark = _bg(W, H, "#171b35"); over(dark, canvas, 0, 0)
m = emissive_mask(dark, man["emissive"])
lights = [((pad + k * (cw + pad) + 4, pad + 6), "#9ff4ff", 30, 0.8) for k in range(len(cells))]
rows = [upscale(flat, S)[..., :3], lit(dark, m, extra_lights=lights, scale=S)]
strip = np.zeros((H * 3 + 4, W * S, 3), np.uint8); strip[...] = hex2rgb("#0e1022")
strip[2:2 + H, 2:2 + W] = flat[..., :3]
s3 = upscale(flat, 3)[..., :3]
if W + 6 + 3 * W <= W * S:
    strip[2:2 + 3 * H, W + 6:W + 6 + 3 * W] = s3
rows.append(strip)
Image.fromarray(np.concatenate(rows, 0)).save(os.path.join(os.path.dirname(__file__), "work", "zoom.png"))
