"""Write logo.png/.json (big, with the 'shine' anim) and logo_small.png/.json, plus previews."""

import os

import numpy as np

from build_logo import PALS, SWORD_EMISSIVE, SWORD_KEY, build_big, build_small
from common import (PREVIEWS, SPRITES, GROUND, UI, flat, new, blit, rgb, save_gif, save_json, save_png,
                    save_preview, hero_img)

PAL = PALS["ivory"]

BRIGHTER = {"1": "2", "2": "3", "3": "4", "4": "4"}
STARS = [
    [".3.", "343", ".3."],
    ["..3..", "..4..", "34443", "..4..", "..3.."],
    ["...3...", "...3...", "..343..", "3344433", "..343..", "...3...", "...3..."],
    ["..3..", "..4..", "34443", "..4..", "..3.."],
    ["4"],
]


def blade_cells(info):
    """(x, y, key) of every blade-colored pixel in the sword, in image coords."""
    out = []
    for y, r in enumerate(info["rows"]):
        for x, c in enumerate(r):
            if c in "1234":
                out.append((info["x"] + x, info["y"] + y, c))
    return out


def tip(info):
    rows = info["rows"]
    y = len(rows) - 2  # last blade pixel row (the row above the final outline pixel)
    x = rows[y].index("3")
    return info["x"] + x, info["y"] + y


def stamp_star(img, star, cx, cy):
    h, w = len(star), len(star[0])
    for y, r in enumerate(star):
        for x, c in enumerate(r):
            if c in SWORD_KEY and c != "o":
                px, py = cx - w // 2 + x, cy - h // 2 + y
                if 0 <= px < img.shape[1] and 0 <= py < img.shape[0]:
                    img[py, px, :3] = rgb(SWORD_KEY[c])
                    img[py, px, 3] = 255


def shine_frames(base, info, band_rows, step, stars, pad_bottom):
    """Frame 0 = base; then a bright band runs down the blade; then the glint star blooms at the tip."""
    img = np.concatenate([base, np.zeros((pad_bottom,) + base.shape[1:], np.uint8)])
    cells = [c for c in blade_cells(info) if c[1] >= info["cap"] - 3]  # guard gem + blade, not the pommel
    y0 = min(c[1] for c in cells)
    y1 = max(c[1] for c in cells)
    frames = [img.copy()]
    y = y0 - band_rows + 1
    while y < y1:
        y += step
        f = img.copy()
        for (px, py, c) in cells:
            d = py - y
            if 0 <= d < band_rows:
                k = BRIGHTER[c] if d in (0, band_rows - 1) else BRIGHTER[BRIGHTER[c]]
                f[py, px, :3] = rgb(SWORD_KEY[k])
        frames.append(f)
    tx, ty = tip(info)
    for st in stars:
        f = img.copy()
        stamp_star(f, st, tx, ty)
        frames.append(f)
    return frames


def write_sheet(name, frames, durations, anchor, notes):
    h, w = frames[0].shape[:2]
    sheet = new(w * len(frames), h)
    for i, f in enumerate(frames):
        sheet[:, i * w:(i + 1) * w] = f
    save_png(sheet, os.path.join(SPRITES, f"{name}.png"))
    man = {
        "cell": [w, h],
        "anchor": anchor,
        "emissive": SWORD_EMISSIVE,
        "anims": {
            "idle": {"row": 0, "frames": 1, "fps": 1},
            "shine": {"row": 0, "frames": len(frames), "fps": 20, "durations": durations},
        },
        "notes": notes,
    }
    save_json(man, os.path.join(SPRITES, f"{name}.json"))
    return sheet


def sky(w, h):
    """Preview-only night sky: warden sky ramp with clean ordered-dither seams, a few stars."""
    ramp = ["#04030c", "#090a1e", "#10163a", "#1a2656", "#2a3c78"]
    b4 = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16.0
    img = np.zeros((h, w, 3), np.uint8)
    for y in range(h):
        t = y / (h - 1) * (len(ramp) - 1) * 0.9
        i = int(t)
        f = t - i
        for x in range(w):
            # dither only in a narrow seam around each ramp step
            seam = (f - 0.7) / 0.3
            j = i + 1 if (seam > b4[y % 4, x % 4]) else i
            img[y, x] = rgb(ramp[min(j, len(ramp) - 1)])
    rng = np.random.default_rng(7)
    for _ in range(40):
        x, y = rng.integers(0, w), rng.integers(0, int(h * 0.7))
        img[y, x] = rgb(["#c8d8ff", "#fff0c0", "#7a8cc8"][rng.integers(0, 3)])
    return img


def main():
    big, info = build_big(PAL)
    # +3 rows so the star at the blade tip fits in the cell
    frames = shine_frames(big, info, band_rows=4, step=5, stars=STARS, pad_bottom=3)
    nb = len(frames) - len(STARS)
    durations = [2] + [2] * (nb - 1) + [3, 3, 6, 4, 3]
    cw = frames[0].shape[1]
    write_sheet("logo", frames, durations, [cw // 2, info["base"]],
                "GLINTBLADE title logo. Ivory stone letters (bloomwild stone ramp), indigo extrusion, the I is Glint's "
                "crystal sword. Anchor = horizontal center on the letters' baseline row. 'idle' is the static logo; "
                "play 'shine' once every few seconds (a light runs down the blade, then a glint star blooms at the tip) "
                "and return to idle. Only the blade, the hilt gems and the star are emissive.")
    small, sinfo = build_small(PAL)
    sframes = shine_frames(small, sinfo, band_rows=3, step=3, stars=[STARS[0], STARS[1], STARS[0], STARS[4]], pad_bottom=2)
    nbs = len(sframes) - 4
    sdur = [2] + [2] * (nbs - 1) + [3, 5, 3, 3]
    write_sheet("logo_small", sframes, sdur, [sframes[0].shape[1] // 2, sinfo["base"]],
                "Small GLINTBLADE logo (cap height 13) for the pause screen and corners. Same design and anims as logo.")

    # ---------------- previews
    k = 6
    for name, fr, dur in (("logo", frames, durations), ("logo_small", sframes, sdur)):
        bgc = GROUND["warden"]
        save_preview(flat(fr[0], bgc), os.path.join(PREVIEWS, f"{name}_6x.png"), k)
        gif = [flat(f, bgc) for f in fr]
        # hold the static frame for a while between shines
        gdur = [1500] + [int(d * 1000 / 60) for d in dur[1:]]
        save_gif(gif, os.path.join(PREVIEWS, f"{name}_shine.gif"), 4, gdur)

    # title-screen mock at native res (preview only), shown 1x and 4x
    W, H = 384, 216
    bg = sky(W, H)
    canvas = new(W, H)
    lx = (W - frames[0].shape[1]) // 2
    blit(canvas, frames[0], lx, 46)
    from fonts import font_small
    fs = font_small()
    msg = "Press any key"
    fs.draw(canvas, (W - fs.width(msg)) // 2, 150, msg, UI["text"][0], shadow=UI["ink"][0])
    hero = hero_img()
    blit(canvas, hero, (W - hero.shape[1]) // 2, 120)
    out = bg.copy()
    m = canvas[:, :, 3] > 0
    out[m] = canvas[m][:, :3]
    save_preview(out, os.path.join(PREVIEWS, "logo_title_mock_1x.png"), 1)
    save_preview(out, os.path.join(PREVIEWS, "logo_title_mock_4x.png"), 4)
    # over each region ground with Glint for scale
    pieces = []
    for reg in ("bloomwild", "glasswater", "cinder", "warden"):
        c = new(frames[0].shape[1] + 30, frames[0].shape[0] + 4)
        blit(c, frames[0], 2, 2)
        blit(c, hero, frames[0].shape[1] + 8, info["base"] - hero.shape[0] + 3)
        pieces.append(flat(c, GROUND[reg]))
    save_preview(np.concatenate(pieces, 0), os.path.join(PREVIEWS, "logo_regions_4x.png"), 4)
    print("logo", frames[0].shape, len(frames), "small", sframes[0].shape, len(sframes))


if __name__ == "__main__":
    main()
