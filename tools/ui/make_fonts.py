"""Write font_small, font_tiny and font_display sheets + manifests, and their previews."""

import os

import numpy as np

from common import GROUND, PREVIEWS, UI, blit, flat, hero_img, new, save_preview
from fonts import font_display, font_small, font_tiny


def specimen(font, lines, w, bg, color, k, name, line_gap=None, spacing_extra=0):
    lh = line_gap or font.line_h
    img = new(w, lh * len(lines) + 8)
    for i, s in enumerate(lines):
        font.draw(img, 4, 4 + i * lh, s, color, shadow=UI["ink"][0])
    save_preview(flat(img, bg), os.path.join(PREVIEWS, name), k)
    return img


def sheet_preview(sheet, name, k=8):
    save_preview(flat(sheet, UI["panel"][0]), os.path.join(PREVIEWS, name), k)


def main():
    fs, ft, fd = font_small(), font_tiny(), font_display()
    for f in (fs, ft, fd):
        sheet = f.build()
        sheet_preview(sheet, f"{f.name}_sheet_8x.png" if f.name != "font_display" else f"{f.name}_sheet_6x.png",
                      8 if f.name != "font_display" else 6)

    # font_small specimen: dialog over each region ground, Glint for scale
    hero = hero_img()
    lines = [
        "The Prism has cracked. Follow the shards!",
        "Glint, you came all this way? Quickly now...",
        "Hold [X] to charge a spin slash. 3/5 shards",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789",
        "abcdefghijklmnopqrstuvwxyz !?.,:;'\"()-+/%&#@",
    ]
    pieces = []
    for reg in ("bloomwild", "glasswater", "cinder", "warden"):
        c = new(250, fs.line_h * len(lines) + 8)
        for i, s in enumerate(lines):
            fs.draw(c, 4, 4 + i * fs.line_h, s, UI["text"][1], shadow=UI["ink"][0])
        blit(c, hero, 228, c.shape[0] - hero.shape[0] - 2)
        pieces.append(flat(c, GROUND[reg]))
    save_preview(np.concatenate(pieces, 0), os.path.join(PREVIEWS, "font_small_specimen_4x.png"), 4)

    # font_tiny specimen
    lines = ["ABCDEFGHIJKLMNOPQRSTUVWXYZ", "0123456789 !?.,:;'\"()-+/%&#@", "PRESS SPACE  HOLD X  LB RB"]
    specimen(ft, lines, 140, GROUND["warden"], UI["text"][1], 6, "font_tiny_specimen_6x.png")

    # font_display: region titles over their grounds
    titles = [("bloomwild", "BLOOMWILD RUINS"), ("glasswater", "GLASSWATER HOLLOW"),
              ("cinder", "CINDER STEPPES"), ("warden", "THE WARDEN'S SUMMIT")]
    pieces = []
    tw = max(fd.width(t) for _, t in titles) + 12 + hero.shape[1] + 8
    for reg, t in titles:
        c = new(tw, fd.cell_h + 10)
        fd.draw(c, 6, 4, t, "#fff2dc", shadow=UI["ink"][0])
        blit(c, hero, tw - hero.shape[1] - 4, c.shape[0] - hero.shape[0] - 1)
        pieces.append(flat(c, GROUND[reg]))
    save_preview(np.concatenate(pieces, 0), os.path.join(PREVIEWS, "font_display_titles_3x.png"), 3)
    specimen(fd, ["ABCDEFGHIJKLM", "NOPQRSTUVWXYZ", "0123456789", "!?&%#$@'.,:;-()"], 200,
             GROUND["warden"], "#fff2dc", 4, "font_display_specimen_4x.png", line_gap=19)


if __name__ == "__main__":
    main()
