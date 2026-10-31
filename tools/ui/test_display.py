import sys

from common import GROUND, UI, flat, new, save_preview
from fonts import font_display, font_tiny

out = sys.argv[1] if len(sys.argv) > 1 else "/tmp/ui_display.png"
f = font_display()
lines = ["ABCDEFGHIJKLM", "NOPQRSTUVWXYZ", "0123456789", "!\"#$%&'()*+,-./:;", "<=>?@[\\]^_`{|}~",
         "BLOOMWILD RUINS", "GLASSWATER HOLLOW", "CINDER STEPPES", "THE WARDEN'S SUMMIT"]
W, H = 230, 20 * len(lines) + 6
img = new(W, H)
for i, s in enumerate(lines):
    f.draw(img, 4, 4 + i * 20, s, "#fff2dc", shadow=UI["ink"][0])
save_preview(flat(img, GROUND["warden"]), out, 4)
save_preview(flat(img, GROUND["warden"]), out.replace(".png", "_1x.png"), 1)

t = font_tiny()
lines = ["ABCDEFGHIJKLMNOPQRSTUVWXYZ", "0123456789 !\"#$%&'()*+,-./", ":;<=>?@[\\]^_`{|}~", "PRESS SPACE TO DASH  x12  LB RB LT RT"]
img = new(170, 8 * len(lines) + 6)
for i, s in enumerate(lines):
    t.draw(img, 3, 3 + i * 8, s, UI["text"][1], shadow=UI["ink"][0])
save_preview(flat(img, GROUND["warden"]), out.replace(".png", "_tiny.png"), 6)
