"""Quick look: sheetview.py module out.png [scale]  (module has W, H, ANIMS and a draw func)"""
import importlib, sys
from PIL import Image
from wlib import hex_rgb
mod = importlib.import_module(sys.argv[1])
fn = getattr(mod, sys.argv[1].replace("warden_", ""))
sc = int(sys.argv[3]) if len(sys.argv) > 3 else 5
ncol = max(n for _, n in mod.ANIMS)
img = Image.new("RGBA", (mod.W * ncol, mod.H * len(mod.ANIMS)), hex_rgb("#323a58") + (255,))
for j, (name, n) in enumerate(mod.ANIMS):
    for i in range(n):
        img.alpha_composite(fn(name, i).image(), (i * mod.W, j * mod.H))
img.resize((img.width * sc, img.height * sc), Image.NEAREST).save(sys.argv[2])
