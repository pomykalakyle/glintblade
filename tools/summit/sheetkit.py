"""Sheet assembly and previews for Summit prop sheets."""

import numpy as np

from common import (PREVIEWS, RGB, blank, check_hard_alpha, colors_used, glint_standin, hx, over,
                    paste, save_gif, save_rgb, save_sheet, scale)

GROUND_BG = "#46384a"   # summit rock ground
PAVE_BG = "#4c5a80"     # arena paving


class Sheet:
    def __init__(self, name, cell, anchor):
        self.name = name
        self.cw, self.ch = cell
        self.anchor = anchor
        self.rows = []      # list of (anim name, [frames], meta)

    def add(self, anim, frames, **meta):
        if not isinstance(frames, list):
            frames = [frames]
        for f in frames:
            assert f.shape[0] == self.ch and f.shape[1] == self.cw, (anim, f.shape)
        self.rows.append((anim, frames, meta))

    def image(self):
        cols = max(len(f) for _, f, _ in self.rows)
        img = blank(cols * self.cw, len(self.rows) * self.ch)
        for r, (_, frames, _) in enumerate(self.rows):
            for c, f in enumerate(frames):
                img[r * self.ch:(r + 1) * self.ch, c * self.cw:(c + 1) * self.cw] = f
        return img

    def manifest(self, emissive=(), notes=""):
        anims = {}
        for r, (anim, frames, meta) in enumerate(self.rows):
            a = {"row": r, "frames": len(frames)}
            a.update({k: v for k, v in meta.items() if v is not None})
            a.setdefault("fps", 1)
            anims[anim] = a
        m = {"cell": [self.cw, self.ch], "anchor": list(self.anchor), "emissive": list(emissive),
             "anims": anims}
        if notes:
            m["notes"] = notes
        return m

    def save(self, emissive=(), notes=""):
        img = self.image()
        check_hard_alpha(img)
        save_sheet(self.name, img, self.manifest(emissive, notes))
        return img


def cell_canvas(cw, ch):
    return blank(cw, ch)


def place(cell_img, sprite, anchor_in_sprite, cell_anchor):
    """Paste a sprite into a cell so its anchor lands on the cell anchor."""
    ax, ay = anchor_in_sprite
    paste(cell_img, sprite, cell_anchor[0] - ax, cell_anchor[1] - ay)
    return cell_img


def lineup(sheet, bg=GROUND_BG, gap=6, frame=0, with_hero=True):
    """All anims of a sheet side by side on a ground strip, anchors on one baseline, Glint
    at the left for scale. Returns an RGB image."""
    items = []
    if with_hero:
        g, a = glint_standin()
        items.append((g, a))
    for anim, frames, meta in sheet.rows:
        f = frames[min(frame, len(frames) - 1)]
        a = meta.get("anchor", sheet.anchor)
        # crop empty columns for a tight lineup
        cols = np.where(f[..., 3].any(axis=0))[0]
        if len(cols) == 0:
            continue
        x0, x1 = cols[0], cols[-1] + 1
        items.append((f[:, x0:x1], (a[0] - x0, a[1])))
    above = max(a[1] for _, a in items)
    below = max(img.shape[0] - a[1] for img, a in items)
    W = sum(img.shape[1] for img, _ in items) + gap * (len(items) + 1)
    H = above + below + 2 * gap
    out = np.zeros((H, W, 3), np.uint8)
    out[:] = hx(bg)
    x = gap
    base = gap + above
    for img, (ax, ay) in items:
        over(out, img, x, base - ay)
        x += img.shape[1] + gap
    return out


def preview(sheet, k=6, bgs=(GROUND_BG, PAVE_BG)):
    strips = [lineup(sheet, bg) for bg in bgs]
    W = max(s.shape[1] for s in strips)
    H = sum(s.shape[0] for s in strips)
    out = np.zeros((H, W, 3), np.uint8)
    y = 0
    for s in strips:
        out[y:y + s.shape[0], :s.shape[1]] = s
        y += s.shape[0]
    save_rgb(PREVIEWS / f"{sheet.name}_{k}x.png", out, k)
    save_rgb(PREVIEWS / f"{sheet.name}_1x.png", out, 1)
    return out


def anim_gif(sheet, anim, k=8, bg=GROUND_BG, pad=4, ticks_per_frame=None):
    for name, frames, meta in sheet.rows:
        if name != anim:
            continue
        fps = meta.get("fps", 8)
        durs = meta.get("durations")
        h, w = frames[0].shape[:2]
        out = []
        for f in frames:
            c = np.zeros((h + 2 * pad, w + 2 * pad, 3), np.uint8)
            c[:] = hx(bg)
            over(c, f, pad, pad)
            out.append(c)
        ms = [int(d * 1000 / 60) for d in durs] if durs else [int(1000 / fps)] * len(frames)
        save_gif(PREVIEWS / f"{sheet.name}_{anim}.gif", out, ms, k)
        return out
