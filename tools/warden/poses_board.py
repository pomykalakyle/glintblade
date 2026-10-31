"""Assemble Warden key poses from a sprites folder over the headless summit arena (review only).

usage (repo root): uv run --no-project --with pillow python3 tools/warden/poses_board.py SPRITES OUT_PREFIX
Uses the production warden_layout.json offsets and draw order, the engine's 16 px kneel with
clipping at the ground line, and the real Glint idle_up frame for scale. Writes OUT_PREFIX_1x.png
and an exact 3x enlargement. Slam frames are drawn at their baked lift (no runtime z).
"""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
ARENA = ROOT / "out" / "samples" / "warden_concepts" / "arena_src" / "summit_t00090.png"
FEET = (192, 130)  # the Warden's ground anchor in the arena capture

POSES = [
    ("dormant", dict(body="idle", head=("idle", 2), heart=("whole", 3), cr=("intact", 0), fists=("open", 0), kneel=16)),
    ("awake", dict(body="awake", head=("idle", 1), heart=("whole", 1), cr=("intact", 1), fists=("clenched", 0))),
    ("slam wind-up", dict(body="awake", head=("flare", 3), heart=("whole", 1), cr=("intact", 1), fists=("clenched", 1), slam=1)),
    ("slam impact", dict(body="awake", head=("flare", 3), heart=("cracked", 1), cr=("intact", 2), fists=("clenched", 2), slam=3)),
    ("roar", dict(body="awake", head=("roar", 3), heart=("cracked", 0), cr=("cracked", 0), fists=("open_lit", 1))),
    ("phase 3", dict(body="damaged", head=("flare", 3), heart=("badly_cracked", 0), cr=("broken", 0), crr=("cracked", 1), fists=("clenched", 0))),
    ("phase 3 kneel", dict(body="damaged", head=("idle", 0), heart=("badly_cracked", 2), cr=("broken", 0), crr=("broken", 0), fists=("open", 0), kneel=16)),
]


class Sheets:
    """Frame lookup over a folder of warden_*.{png,json} sheets."""

    def __init__(self, folder):
        """Remember the folder; sheets load lazily."""
        self.folder, self.cache = Path(folder), {}

    def cell(self, name, anim, i, mirror=False):
        """Crop one frame; return (image, anchor), mirrored around its anchor if asked."""
        if name not in self.cache:
            self.cache[name] = (json.loads((self.folder / f"{name}.json").read_text()),
                                Image.open(self.folder / f"{name}.png").convert("RGBA"))
        m, im = self.cache[name]
        cw, ch = m["cell"]
        r = m["anims"][anim]["row"]
        c = im.crop((i * cw, r * ch, i * cw + cw, r * ch + ch))
        ax, ay = m["anchor"]
        if mirror:
            c = c.transpose(Image.FLIP_LEFT_RIGHT)
            ax = cw - 1 - ax
        return c, (ax, ay)


def put(cv, c, a, x, y, clip=None):
    """Paste a frame with its anchor at (x, y), hiding rows below `clip` (the kneel's ground line)."""
    if clip is not None:
        c = c.copy()
        keep = clip - (y - a[1]) + 1
        if keep < c.height:
            ImageDraw.Draw(c).rectangle((0, max(keep, 0), c.width, c.height), fill=(0, 0, 0, 0))
    cv.alpha_composite(c, (x - a[0], y - a[1]))


def glint():
    """The real hero idle_up frame and anchor."""
    hero = json.loads((ROOT / "assets" / "sprites" / "hero.json").read_text())
    im = Image.open(ROOT / "assets" / "sprites" / "hero.png").convert("RGBA")
    hw, hh = hero["cell"]
    r = hero["anims"]["idle_up"]["row"]
    return im.crop((0, r * hh, hw, r * hh + hh)), tuple(hero["anchor"])


def pose(s, layout, arena, g, p):
    """Draw one pose into a 200x170 arena crop."""
    cv = arena.crop((FEET[0] - 100, FEET[1] - 125, FEET[0] + 100, FEET[1] + 45))
    gx, gy = 100, 125
    kn = p.get("kneel", 0)
    clip = gy if kn else None
    put(cv, *s.cell("warden_body", p["body"], 0), gx, gy + kn, clip)
    for side, mir in (("crystal_left", False), ("crystal_right", True)):
        st = p.get("crr", p["cr"]) if side == "crystal_right" else p["cr"]
        dx, dy = layout["parts"][side]["offset"]
        put(cv, *s.cell("warden_crystal", st[0], st[1], mir), gx + dx, gy + dy + kn, clip)
    put(cv, *s.cell("warden_head", *p["head"]), gx, gy + layout["parts"]["head"]["offset"][1] + kn, clip)
    put(cv, *s.cell("warden_heart", *p["heart"]), gx, gy + layout["parts"]["heart"]["offset"][1] + kn, clip)
    for side, mir in (("fist_left", True), ("fist_right", False)):
        dx, dy = layout["parts"][side]["offset"]
        fx, fy = gx + dx, (gy + 4 if kn else gy + dy)
        if p.get("slam") is not None and side == "fist_right":
            put(cv, *s.cell("warden_fist", "slam", p["slam"], mir), gx + 30, gy + 26)
            continue
        put(cv, *s.cell("warden_fist", *p["fists"], mir), fx, fy)
        put(cv, *s.cell("warden_crystal", "intact", 0, mir), fx, fy - 22)
    put(cv, *g, 100, 162)
    return cv


def main():
    """Write the 1x board and its 3x enlargement."""
    s = Sheets(sys.argv[1])
    out = sys.argv[2]
    layout = json.loads((ROOT / "assets" / "sprites" / "warden_layout.json").read_text())
    arena = Image.open(ARENA).convert("RGBA")
    g = glint()
    board = Image.new("RGBA", (200 * 4, 170 * 2), (0, 0, 0, 255))
    for k, (_, p) in enumerate(POSES):
        board.alpha_composite(pose(s, layout, arena, g, p), ((k % 4) * 200, (k // 4) * 170))
    board.save(out + "_1x.png")
    board.resize((board.width * 3, board.height * 3), Image.NEAREST).save(out + "_3x.png")


if __name__ == "__main__":
    main()
