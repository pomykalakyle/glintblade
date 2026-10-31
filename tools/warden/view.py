"""Quick nearest-neighbor previews of Warden parts while drawing (writes to /tmp by default).

usage: uv run --no-project --with pillow --with numpy python3 view.py body|head|fist|heart|crystal [scale] [out]
"""

import sys

from PIL import Image

import body
import crystal
import fist
import head
import heart
from wlib import hex_rgb

PARTS = {
    "body": (body.body, ["idle", "awake", "damaged"], {"idle": 1, "awake": 1, "damaged": 1}),
    "head": (head.head, ["idle", "flare", "roar"], {"idle": 4, "flare": 4, "roar": 6}),
    "fist": (fist.fist, ["clenched", "open", "open_lit", "slam"], {"clenched": 4, "open": 4, "open_lit": 4, "slam": 5}),
    "heart": (heart.heart, ["whole", "cracked", "badly_cracked", "shattered"], {"whole": 4, "cracked": 4, "badly_cracked": 4, "shattered": 4}),
    "crystal": (crystal.crystal, ["intact", "cracked", "broken"], {"intact": 4, "cracked": 4, "broken": 1}),
}
BG = "#46526a"


def render(name, scale):
    """Render every frame of a part on the summit floor color, rows by anim."""
    fn, anims, counts = PARTS[name]
    cells = [[fn(a, i) for i in range(counts[a])] for a in anims]
    cw, ch = cells[0][0].w, cells[0][0].h
    ncol = max(len(r) for r in cells)
    img = Image.new("RGBA", ((cw + 2) * ncol, (ch + 2) * len(cells)), hex_rgb(BG) + (255,))
    for j, r in enumerate(cells):
        for i, cv in enumerate(r):
            img.alpha_composite(cv.image(), (i * (cw + 2) + 1, j * (ch + 2) + 1))
    return img.resize((img.width * scale, img.height * scale), Image.NEAREST)


if __name__ == "__main__":
    name = sys.argv[1]
    scale = int(sys.argv[2]) if len(sys.argv) > 2 else 4
    out = sys.argv[3] if len(sys.argv) > 3 else f"/tmp/wv2_{name}.png"
    render(name, scale).save(out)
    print(out)
