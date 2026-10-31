"""Build the private, native-grid GLINTBLADE wordmarks in restrained bone and slate.

Run: uv run --no-project --with numpy --with pillow python3 tools/ui/logo_v2.py
The title and credit mark have separately authored contours and optical spacing.
The shared cutstone display font and its generators remain independent.
"""

import os

import numpy as np
from common import SPRITES, UI, new, rgb, save_json, save_png
from logo_forms_v2 import compose

INK = UI["ink"][0]
FACE = UI["text"][1]
SHADE = UI["border"][1]
CAST = UI["panel"][0]


def sh(a, dx, dy):
    """Shift a mask by (dx, dy), filling with False."""
    b = np.zeros_like(a)
    h, w = a.shape
    b[max(0, dy):h + min(0, dy), max(0, dx):w + min(0, dx)] = a[max(0, -dy):h - max(0, dy), max(0, -dx):w - max(0, dx)]
    return b


def shade(mask, incision, small=False):
    """Give broad stone faces selective right-hand facets and one dark blade incision."""
    h, w = mask.shape
    pad = 2
    img = new(w + 2 * pad + 1, h + 2 * pad + 1)
    m = np.zeros(img.shape[:2], bool)
    m[pad:pad + h, pad:pad + w] = mask
    cut = np.zeros_like(m)
    cut[pad:pad + h, pad:pad + w] = incision
    outline = (sh(m, 1, 0) | sh(m, -1, 0) | sh(m, 0, 1) | sh(m, 0, -1)) & ~m
    shadow = sh(m | outline, 1, 1) & ~m & ~outline
    # Flat material planes, not a bright bevel traced around every stroke.
    side = m & ~sh(m, -1, 0) & sh(m, 1, 0)
    sole = m & ~sh(m, 0, -1)
    facet = side | sole
    fresh = sh(cut, 0, 1) & m & ~cut

    def paint(where, color):
        """Write one opaque palette color into the selected pixels."""
        img[where, :3] = rgb(color)
        img[where, 3] = 255

    paint(shadow, CAST)
    paint(outline, INK)
    paint(m, FACE)
    paint(facet, SHADE)
    paint(fresh, FACE)
    # The incision exposes dark stone instead of revealing busy scenery through the word.
    paint(cut & m, INK)
    return img, pad


def build(name, small=False):
    """Write one static wordmark and retain its centered baseline anchor contract."""
    mask, incision, _ = compose(small)
    img, pad = shade(mask, incision, small)
    save_png(img, os.path.join(SPRITES, f"{name}.png"))
    h, w = img.shape[:2]
    manifest = {
        "cell": [w, h],
        "anchor": [w // 2, pad + mask.shape[0] - 1],
        "emissive": [],
        "anims": {"idle": {"row": 0, "frames": 1, "fps": 1}},
        "notes": (
            f"GLINTBLADE style v2 crafted {'credit' if small else 'title'} wordmark, 2026-10-31. "
            f"Private {'12' if small else '26'} px native narrow capital forms: open hooked G, cantilever T, "
            "unequal B chambers, wedge-foot L, intact flat-apex A and diagonal-ended E. "
            "One angled blade incision at the B waist, preserving its left stem. Optical spacing keeps every letter separate. "
            "Bone #dce0ce faces, slate #98a49f side facets, ink #252d3c and a 1 px #303949 cast edge. "
            "Centered letter-baseline anchor; static, binary alpha, no shine or emissive colors. "
            "Rebuild both sheets with tools/ui/logo_v2.py; no display-font dependency."
        ),
    }
    save_json(manifest, os.path.join(SPRITES, f"{name}.json"))
    return img


def main():
    """Write the title and independently authored credit sheets."""
    build("logo")
    build("logo_small", small=True)


if __name__ == "__main__":
    main()
