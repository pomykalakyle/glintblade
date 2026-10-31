"""Enemies A samples for out/samples: the sproutling, thorn imp and seed on real Bloomwild tiles
next to Glint for scale, at 1x and 4x. Run after sproutling.py, imp.py and seed.py:
  uv run --no-project --with pillow --with numpy python3 samples.py
"""

import json
import shutil

from PIL import Image

from pix import SPRITES, PREVIEWS, SAMPLES
from preview import glint
from seed import floor, sprite, shadow


def load(name):
    return Image.open(SPRITES / f"{name}.png").convert("RGBA"), json.load(open(SPRITES / f"{name}.json"))


def lineup():
    sp, spm = load("sproutling")
    im, imm = load("thorn_imp")
    sd, sdm = load("seed")
    W, H = 240, 72
    bg = floor(W, 40, ["grass_0", "grass_1", "grass_2", "grass_3", "grass_5"])
    bg2 = floor(W, 32, ["path_0", "path_1", "path_2", "ruin_0", "ruin_2"])
    out = Image.new("RGBA", (W, H))
    out.paste(bg, (0, 0))
    out.paste(bg2, (0, 40))
    g, ga = glint()
    # top row on grass: Glint, sproutling idle/windup (side + front), imp idle/windup
    row = [("glint", None, 0), ("sproutling", "idle_right", 0), ("sproutling", "windup_right", 2),
           ("sproutling", "idle_down", 0), ("sproutling", "windup_down", 2), ("thorn_imp", "idle_right", 0),
           ("thorn_imp", "windup_right", 2), ("thorn_imp", "idle_down", 0)]
    xs = [14, 40, 66, 92, 116, 148, 182, 214]
    for (name, an, fr), x in zip(row, xs):
        gy = 35
        if name == "glint":
            shadow(out, x, gy, 4)
            out.alpha_composite(g, (x - ga[0], gy - ga[1]))
            continue
        s, m = (sp, spm) if name == "sproutling" else (im, imm)
        shadow(out, x, gy, 5 if name == "sproutling" else 4)
        out.alpha_composite(sprite(s, m, an, fr), (x - m["anchor"][0], gy - m["anchor"][1]))
    # bottom row on the path: imp throw with the seed leaving the hand, sproutling charging at Glint
    gy = 66
    ax, ay = imm["anchor"]
    off = imm["anims"]["attack_right"]["release_offset"]
    for k, (an, fr) in enumerate((("attack_right", 0), ("attack_right", 1))):
        x = 20 + k * 34
        shadow(out, x, gy, 4)
        out.alpha_composite(sprite(im, imm, an, fr), (x - ax, gy - ay))
    x0 = 54 + off[0]
    for n in range(3):
        sx = x0 + n * 13
        shadow(out, sx, gy, 2)
        out.alpha_composite(sprite(sd, sdm, "fly", n % 4), (sx - 6, gy + off[1] - n - 6))
    shadow(out, 160, gy, 5)
    out.alpha_composite(sprite(sp, spm, "attack_right", 1), (160 - spm["anchor"][0], gy - spm["anchor"][1]))
    shadow(out, 196, gy, 4)
    out.alpha_composite(g.transpose(Image.FLIP_LEFT_RIGHT), (196 - (g.width - 1 - ga[0]), gy - ga[1]))
    return out


if __name__ == "__main__":
    SAMPLES.mkdir(parents=True, exist_ok=True)
    im = lineup()
    im.convert("RGB").save(PREVIEWS / "enemies_a_lineup_1x.png")
    big = im.resize((im.width * 4, im.height * 4), Image.NEAREST).convert("RGB")
    big.save(PREVIEWS / "enemies_a_lineup_4x.png")
    big.save(SAMPLES / "enemies_a_v3_lineup_4x.png")
    shutil.copy(PREVIEWS / "seed_release_4x.png", SAMPLES / "enemies_a_v3_seed_release_4x.png")
    print(SAMPLES / "enemies_a_v3_lineup_4x.png")
