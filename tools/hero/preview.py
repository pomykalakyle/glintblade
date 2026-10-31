"""preview.py NAME [dirs...] : render anim NAME as a big strip (PNG) and GIFs to /tmp/hero_prev/."""
import os
import sys
from PIL import Image
from pix import scale, on_ground, hex_rgb
from rig import frame
import anims

OUT = "/tmp/hero_prev"
os.makedirs(OUT, exist_ok=True)
GROUND = "#1f8250"


def strip(frames, path, k=8, ground=GROUND, box=(0, 0, 40, 40)):
    w = box[2] - box[0]
    h = box[3] - box[1]
    out = Image.new("RGBA", (w * len(frames) + (len(frames) - 1), h), (12, 12, 20, 255))
    for i, f in enumerate(frames):
        out.paste(on_ground(f.to_image(), ground).crop(box), (i * (w + 1), 0))
    big = scale(out, k)
    small = scale(out, 2)
    full = Image.new("RGBA", (big.width, big.height + small.height + 6), (12, 12, 20, 255))
    full.paste(big, (0, 0))
    full.paste(small, (0, big.height + 6))
    full.save(path)


def gif(frames, durs, path, k=6, ground=GROUND, box=(0, 0, 40, 40)):
    ims = []
    for f in frames:
        im = on_ground(f.to_image(), ground).crop(box)
        ims.append(scale(im, k).convert("RGB"))
    ms = [max(20, round(t * 1000 / 60 / 10) * 10) for t in durs]
    ims[0].save(path, save_all=True, append_images=ims[1:], duration=ms, loop=0, disposal=1)


if __name__ == "__main__":
    name = sys.argv[1]
    dirs = sys.argv[2:] or ["down", "right", "up"]
    box = (0, 0, 48, 48)
    allf = []
    for d in dirs:
        fn = getattr(anims, name)
        specs, durs = fn(d) if d != "-" else fn()
        res = [frame(s) for s in specs]
        frames = [r[0] for r in res]
        for i, r in enumerate(res):
            if r[1]["clipped"]:
                print(f"CLIPPED {name}_{d} frame {i}: {r[1]['clipped']} px")
        allf.append(frames)
        gif(frames, durs, f"{OUT}/{name}_{d}.gif", box=box)
    # one strip with all dirs stacked
    rows = []
    for frames in allf:
        p = f"{OUT}/_tmp.png"
        strip(frames, p, box=box)
        rows.append(Image.open(p).copy())
    W = max(r.width for r in rows)
    H = sum(r.height for r in rows) + 8 * len(rows)
    out = Image.new("RGBA", (W, H), (12, 12, 20, 255))
    y = 0
    for r in rows:
        out.paste(r, (0, y))
        y += r.height + 8
    out.save(f"{OUT}/{name}.png")
    print(f"{OUT}/{name}.png")
