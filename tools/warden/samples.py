"""Build the Weir Keeper before/after review set in out/samples/warden_weir_keeper_v2style/.

usage (repo root): uv run --no-project --with pillow python3 tools/warden/samples.py BEFORE_SPRITES AFTER_SPRITES CAPTURES
BEFORE_SPRITES / AFTER_SPRITES hold the five warden_*.{png,json} sheets; CAPTURES holds
sel_{before,after}_{1,3}/fNNNNN.png headless shots and ticks_{before,after}.json (label -> tick).
Every output keeps native pixels: _1x is the source, _3x/_8x are exact nearest-neighbor enlargements.
"""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "out" / "samples" / "warden_weir_keeper_v2style"
BG = (70, 82, 106, 255)  # summit floor step 1 (#46526a)
INK = (217, 218, 204, 255)
NAMES = ["body", "head", "fist", "heart", "crystal"]


def big(im, s):
    """Exact nearest-neighbor enlargement."""
    return im.resize((im.width * s, im.height * s), Image.NEAREST)


def on_bg(im):
    """Composite a sheet over the summit floor color."""
    b = Image.new("RGBA", im.size, BG)
    b.alpha_composite(im.convert("RGBA"))
    return b


def sheets(before, after):
    """Side-by-side sheet comparisons (before left, after right) at 1x, 3x and 8x."""
    for n in NAMES:
        a = on_bg(Image.open(before / f"warden_{n}.png"))
        b = on_bg(Image.open(after / f"warden_{n}.png"))
        c = Image.new("RGBA", (a.width + b.width + 6, max(a.height, b.height)), (20, 24, 34, 255))
        c.paste(a, (0, 0))
        c.paste(b, (a.width + 6, 0))
        for s in (1, 3, 8):
            big(c, s).save(OUT / f"sheet_{n}_before_after_{s}x.png")


def label(im, text):
    """Write a small label strip above a frame (default bitmap font, review use only)."""
    out = Image.new("RGBA", (im.width, im.height + 12), (0, 0, 0, 255))
    out.paste(im, (0, 12))
    ImageDraw.Draw(out).text((2, 0), text, fill=INK)
    return out


def fights(cap):
    """Before/after headless fight frames: one 1x board and one 3x pair per moment."""
    tb = json.loads((cap / "ticks_before.json").read_text())
    ta = json.loads((cap / "ticks_after.json").read_text())
    (OUT / "fight_3x").mkdir(parents=True, exist_ok=True)
    rows = []
    for k in tb:
        fb = Image.open(cap / "sel_before_1" / f"f{tb[k]:05}.png").convert("RGBA")
        fa = Image.open(cap / "sel_after_1" / f"f{ta[k]:05}.png").convert("RGBA")
        row = Image.new("RGBA", (fb.width * 2 + 4, fb.height), (0, 0, 0, 255))
        row.paste(fb, (0, 0))
        row.paste(fa, (fb.width + 4, 0))
        rows.append(label(row, f"{k}: before t{tb[k]} | after t{ta[k]}"))
        b3 = Image.open(cap / "sel_before_3" / f"f{tb[k]:05}.png").convert("RGBA")
        a3 = Image.open(cap / "sel_after_3" / f"f{ta[k]:05}.png").convert("RGBA")
        p = Image.new("RGBA", (b3.width * 2 + 12, b3.height), (0, 0, 0, 255))
        p.paste(b3, (0, 0))
        p.paste(a3, (b3.width + 12, 0))
        p.save(OUT / "fight_3x" / f"{k}_before_after_3x.png")
        a3.save(OUT / "fight_3x" / f"{k}_after_3x.png")
    half = (len(rows) + 1) // 2
    for part, rs in (("a", rows[:half]), ("b", rows[half:])):
        h = sum(r.height for r in rs)
        board = Image.new("RGBA", (rows[0].width, h), (0, 0, 0, 255))
        y = 0
        for r in rs:
            board.paste(r, (0, y))
            y += r.height
        board.save(OUT / f"fight_before_after_1x_{part}.png")


if __name__ == "__main__":
    before, after, cap = (Path(p) for p in sys.argv[1:4])
    OUT.mkdir(parents=True, exist_ok=True)
    sheets(before, after)
    fights(cap)
