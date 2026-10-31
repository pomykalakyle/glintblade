# Overlay a labeled tile grid on a zone overview PNG (for planning prop positions).
# usage: grid.py in.png out.png [x0 y0 x1 y1 (tiles)] [scale]
import sys
from PIL import Image, ImageDraw
im = Image.open(sys.argv[1]).convert("RGB")
x0, y0, x1, y1 = (int(v) for v in sys.argv[3:7]) if len(sys.argv) > 6 else (0, 0, im.width // 16, im.height // 16)
s = int(sys.argv[7]) if len(sys.argv) > 7 else 2
im = im.crop((x0 * 16, y0 * 16, x1 * 16, y1 * 16)).resize(((x1 - x0) * 16 * s, (y1 - y0) * 16 * s), Image.NEAREST)
d = ImageDraw.Draw(im)
for tx in range(x0, x1 + 1):
    X = (tx - x0) * 16 * s
    d.line([(X, 0), (X, im.height)], fill=(255, 255, 0) if tx % 4 == 0 else (90, 90, 40), width=1)
    if tx % 4 == 0:
        d.text((X + 2, 2), str(tx), fill=(255, 255, 0))
for ty in range(y0, y1 + 1):
    Y = (ty - y0) * 16 * s
    d.line([(0, Y), (im.width, Y)], fill=(255, 255, 0) if ty % 4 == 0 else (90, 90, 40), width=1)
    if ty % 4 == 0:
        d.text((2, Y + 2), str(ty), fill=(255, 255, 0))
im.save(sys.argv[2])
