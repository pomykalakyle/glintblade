"""Font sheets: build the grid sheet + manifest from a glyph module, and render text."""

import os

import numpy as np

from common import SPRITES, new, rgb, save_json, save_png

PRINTABLE = [chr(c) for c in range(32, 127)]
COLS = 16


class Font:
    def __init__(self, name, glyphs, cell_h, baseline, cap_h, x_h, tracking, line_h, space=None,
                 kerning=None, notes=""):
        self.name = name
        self.cell_h = cell_h
        self.baseline = baseline
        self.cap_h = cap_h
        self.x_h = x_h
        self.tracking = tracking
        self.line_h = line_h
        self.kerning = kerning or {}
        self.notes = notes
        self.masks = {}  # char -> bool array (cell_h, w)
        for ch, (top, rows) in glyphs.items():
            w = max(len(r) for r in rows)
            m = np.zeros((cell_h, w), bool)
            for y, r in enumerate(rows):
                for x, c in enumerate(r):
                    if c == "#":
                        assert top + y < cell_h, (name, ch)
                        m[top + y, x] = True
            self.masks[ch] = m
        if space is not None:
            self.masks[" "] = np.zeros((cell_h, space), bool)
        missing = [c for c in PRINTABLE if c not in self.masks]
        assert not missing, f"{name} missing {missing}"
        self.cell_w = max(m.shape[1] for m in self.masks.values()) + 1

    def width(self, s):
        w = 0
        for i, ch in enumerate(s):
            w += self.masks[ch].shape[1]
            if i + 1 < len(s):
                w += self.tracking + self.kerning.get(ch + s[i + 1], 0)
        return w

    def draw(self, img, x, y, s, color, shadow=None, shadow_off=(1, 1), outline=None):
        """Draw text with its cell top at y. Optional drop shadow and/or 1px outline."""
        if outline:
            for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)):
                self._draw(img, x + dx, y + dy, s, outline)
        if shadow:
            self._draw(img, x + shadow_off[0], y + shadow_off[1], s, shadow)
        self._draw(img, x, y, s, color)

    def _draw(self, img, x, y, s, color):
        H, W = img.shape[:2]
        c = rgb(color)
        cx = x
        for i, ch in enumerate(s):
            m = self.masks[ch]
            ys, xs = np.nonzero(m)
            for gy, gx in zip(ys, xs):
                px, py = cx + gx, y + gy
                if 0 <= px < W and 0 <= py < H:
                    img[py, px, :3] = c
                    img[py, px, 3] = 255
            cx += m.shape[1] + self.tracking
            if i + 1 < len(s):
                cx += self.kerning.get(ch + s[i + 1], 0)

    def build(self):
        rows = (len(PRINTABLE) + COLS - 1) // COLS
        sheet = new(self.cell_w * COLS, self.cell_h * rows)
        glyphs = {}
        for i, ch in enumerate(PRINTABLE):
            gx, gy = (i % COLS) * self.cell_w, (i // COLS) * self.cell_h
            m = self.masks[ch]
            sheet[gy:gy + self.cell_h, gx:gx + m.shape[1]][m] = (255, 255, 255, 255)
            glyphs[ch] = [gx, gy, int(m.shape[1])]
        save_png(sheet, os.path.join(SPRITES, f"{self.name}.png"))
        man = {
            "cell": [self.cell_w, self.cell_h],
            "anchor": [0, self.baseline],
            "emissive": [],
            "anims": {},
            "font": {
                "glyphs_format": "char -> [x, y, w]: the glyph is the w x cell_h rect at (x, y); ink is white (#ffffff), tint it to any color",
                "cell_h": self.cell_h,
                "baseline": self.baseline,
                "cap_height": self.cap_h,
                "x_height": self.x_h,
                "tracking": self.tracking,
                "line_height": self.line_h,
                "advance": "w + tracking (+ kerning[pair] if present)",
                "kerning": self.kerning,
            },
            "glyphs": glyphs,
            "notes": self.notes,
        }
        save_json(man, os.path.join(SPRITES, f"{self.name}.json"))
        return sheet
