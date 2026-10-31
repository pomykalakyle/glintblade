import sys

from common import GROUND, UI, flat, new, save_preview
from fonts import font_small

f = font_small()
lines = [
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    "abcdefghijklmnopqrstuvwxyz",
    "0123456789 !\"#$%&'()*+,-./",
    ":;<=>?@[\\]^_`{|}~",
    "The Prism has cracked. Follow the shards!",
    "Glint, you came all this way? Quickly now...",
    "Hold [X] to charge a spin slash.",
    "Bloomwild Ruins - Glasswater Hollow",
]
W, H = 260, 12 * len(lines) + 6
img = new(W, H)
for i, s in enumerate(lines):
    f.draw(img, 4, 4 + i * 12, s, UI["text"][1], shadow=UI["ink"][0])
out = sys.argv[1] if len(sys.argv) > 1 else "/tmp/ui_small.png"
save_preview(flat(img, GROUND["warden"]), out, 4)
save_preview(flat(img, GROUND["warden"]), out.replace(".png", "_1x.png"), 1)
