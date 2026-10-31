"""Render every anim of a module as zoomed strips into scratch/ for looking."""
import sys, importlib, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).parent))
import pix
mod = importlib.import_module(sys.argv[1])
palmod = importlib.import_module(sys.argv[2])
scale = int(sys.argv[3]) if len(sys.argv) > 3 else 6
only = sys.argv[4:] 
out = pathlib.Path(__file__).parent / "_wip_scratch"
from PIL import Image
rows = []
for name, fn in mod.ANIMS.items():
    if only and name not in only:
        continue
    frames, meta = fn()
    arrs = []
    for cv in frames:
        c = cv.copy()
        c.outline(skip=palmod.NO_OUTLINE)
        arrs.append(c.to_rgba(palmod.PAL))
    p = out / f"{sys.argv[1]}_{name}.png"
    pix.strip(arrs, p, scale=scale)
    rows.append(p)
# stack
ims = [Image.open(p) for p in rows]
W = max(i.width for i in ims); H = sum(i.height for i in ims) + 4 * len(ims)
st = Image.new("RGB", (W, H), (20, 10, 20))
y = 0
for i in ims:
    st.paste(i, (0, y)); y += i.height + 4
st.save(out / f"{sys.argv[1]}_all.png")
print(out / f"{sys.argv[1]}_all.png")
