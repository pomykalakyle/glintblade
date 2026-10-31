"""zoom.py out.png k i0 i1 ... : big crops of frames from a module's FRAMES list (via exec)."""
import sys
from PIL import Image
from pix import scale, on_ground

def zoom(frames, path, k=14, ground="#222a4a", box=(6, 10, 34, 36)):
    w = box[2] - box[0]
    h = box[3] - box[1]
    out = Image.new("RGBA", (w * len(frames), h))
    for i, f in enumerate(frames):
        out.paste(on_ground(f.to_image(), ground).crop(box), (i * w, 0))
    out = scale(out, k)
    # faint pixel grid for judging clusters
    px = out.load()
    for x in range(0, out.width, k):
        for y in range(out.height):
            r, g, b, a = px[x, y]
            px[x, y] = (min(255, r + 14), min(255, g + 14), min(255, b + 14), a)
    out.save(path)
