"""In-engine captures of Glint: full 384x216 frames and short idle/run/attack/dash GIFs cropped
around the hero, played by the real game (headless `glintblade shot`) in every region.

cargo build --release
uv run --no-project --with pillow --with numpy python3 tools/hero/capture.py
"""
import os
import shutil
import subprocess
import tempfile

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BIN = os.path.join(ROOT, "target", "release", "glintblade")
PREVIEWS = os.path.join(ROOT, "assets", "previews")
ZONES = ["bloomwild", "glasswater", "cinder", "summit"]

# one scripted run per zone (see tools.rs parse_script; W = no input)
SCRIPT = "W50 R36 W6 A1 W9 A1 W9 A1 W30 R8 RX1 W24 U20 W40"
SEGMENTS = {  # name: (first tick, last tick)
    "idle": (14, 50),
    "run": (55, 86),
    "attack": (93, 140),
    "dash": (150, 176),
}
KEY_FRAME = 98  # mid-combo, smear on screen: the full-frame capture
CROP_W, CROP_H, K = 72, 52, 4


def hero_center(a):
    """Median position of the cloak's indigo pixels in a 1x RGB frame."""
    r, g, b = (a[..., i].astype(int) for i in range(3))
    m = (b > r + 60) & (b > g + 50) & (b > 120)
    m[:70] = False  # the zone title banner's blue panel sits at the top
    ys, xs = np.nonzero(m)
    if len(xs) < 20:
        return None
    return int(np.median(xs)), int(np.median(ys))


def main():
    tmp = tempfile.mkdtemp(prefix="hero_cap_")
    try:
        os.makedirs(PREVIEWS, exist_ok=True)
        for zone in ZONES:
            out = os.path.join(tmp, zone)
            subprocess.run([BIN, "shot", "--zone", zone, "--peaceful", "--script", SCRIPT,
                            "--every", "1", "--scale", "1", "--out", out], check=True,
                           capture_output=True, cwd=ROOT)
            frames = {}
            for name in os.listdir(out):
                if name.startswith("f") and name.endswith(".png"):
                    frames[int(name[1:6])] = os.path.join(out, name)
            key = Image.open(frames[KEY_FRAME]).convert("RGB")
            key.save(os.path.join(PREVIEWS, f"hero_ingame_{zone}_1x.png"))
            key.resize((key.width * 3, key.height * 3), Image.NEAREST).save(
                os.path.join(PREVIEWS, f"hero_ingame_{zone}_3x.png"))
            for seg, (t0, t1) in SEGMENTS.items():
                ims = []
                for t in range(t0, t1 + 1, 2):
                    a = np.asarray(Image.open(frames[t]).convert("RGB"))
                    c = hero_center(a)
                    if c is None:
                        continue
                    cx, cy = c
                    box = (cx - CROP_W // 2, cy - CROP_H // 2 - 4, cx + CROP_W // 2, cy + CROP_H // 2 - 4)
                    im = Image.fromarray(a).crop(box)
                    ims.append(im.resize((CROP_W * K, CROP_H * K), Image.NEAREST))
                # one palette from every frame (gameplay frames carry lighting, so quantize once)
                tall = Image.new("RGB", (ims[0].width, ims[0].height * len(ims)))
                for i, im in enumerate(ims):
                    tall.paste(im, (0, i * im.height))
                pal = tall.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
                ps = [im.quantize(palette=pal, dither=Image.Dither.NONE) for im in ims]
                ps[0].save(os.path.join(PREVIEWS, f"hero_ingame_{zone}_{seg}.gif"), save_all=True,
                           append_images=ps[1:], duration=33, loop=0, disposal=1)
            print("captured", zone)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
