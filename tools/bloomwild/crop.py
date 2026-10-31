"""crop.py in.png x y w h k out.png : crop a 1x image and scale it."""
import sys
from PIL import Image
a = sys.argv
im = Image.open(a[1])
x, y, w, h, k = map(int, a[2:7])
im.crop((x, y, x + w, y + h)).resize((w * k, h * k), Image.NEAREST).save(a[7])
