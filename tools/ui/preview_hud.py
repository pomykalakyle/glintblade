"""HUD previews: each sheet on every region ground, plus an in-game HUD mock at native res."""

import json
import os
import sys

import numpy as np
from PIL import Image

from common import GROUND, PREVIEWS, SPRITES, UI, blit, flat, hero_img, new, save_preview, scale
from fonts import font_display, font_small

WIP = len(sys.argv) > 1 and sys.argv[1] == "wip"
PFX = "_wip_" if WIP else ""


def load(name):
    return np.array(Image.open(os.path.join(SPRITES, name + ".png")).convert("RGBA"))


def on_grounds(img, k, name, extra_hero=True):
    hero = hero_img()
    pieces = []
    for reg in ("bloomwild", "glasswater", "cinder", "warden"):
        w = img.shape[1] + (hero.shape[1] + 6 if extra_hero else 0) + 4
        h = max(img.shape[0], hero.shape[0]) + 4
        c = new(w, h)
        blit(c, img, 2, 2)
        if extra_hero:
            blit(c, hero, img.shape[1] + 6, h - 2 - hero.shape[0])
        pieces.append(flat(c, GROUND[reg]))
    save_preview(np.concatenate(pieces, 0), os.path.join(PREVIEWS, f"{PFX}{name}.png"), k)


def filled_bar(frac, chip, flash=False):
    bar = load("hud_bossbar")
    fill = load("hud_bossbar_fill")
    man = json.load(open(os.path.join(SPRITES, "hud_bossbar.json")))
    x, y, w, h = man["fill_rect"]
    n = int(round(w * frac))
    nc = int(round(w * chip))
    for i in range(min(w, n + nc)):
        col = 2 if flash else (0 if i < n else 1)
        bar[y:y + h, x + i] = fill[:, col]
    return bar


def prompts_grid(per=6):
    """All prompt rows, up and pressed frames side by side, 6 per line."""
    sh = load("hud_prompts")
    m = json.load(open(os.path.join(SPRITES, "hud_prompts.json")))
    cw, ch = m["cell"]
    names = list(m["anims"])
    lines = []
    for i in range(0, len(names), per):
        row = []
        for n in names[i:i + per]:
            r = m["anims"][n]["row"]
            row.append(np.concatenate([sh[r * ch:(r + 1) * ch, :cw * 2], new(4, ch)], 1))
        while len(row) < per:
            row.append(new(cw * 2 + 4, ch))
        lines.append(np.concatenate(row, 1))
        lines.append(new(lines[-1].shape[1], 3))
    g = np.concatenate(lines, 0)
    save_preview(flat(g, GROUND["warden"]), os.path.join(PREVIEWS, f"{PFX}hud_prompts_4x.png"), 4)


def native_backdrop(path, crop_h=216, fallback="bloomwild"):
    """An engine frame (3x) or a 1x mock, as a 384x216 RGB array (flat region ground if it's gone)."""
    if not os.path.exists(path):
        return flat(new(384, crop_h), GROUND[fallback])
    im = Image.open(path).convert("RGB")
    if im.width == 1152:
        im = im.resize((384, 216), Image.NEAREST)
    a = np.array(im)[:crop_h, :384]
    return a


def nine_slice(panel, man, w, h):
    l, t, r, b = man["slice"]
    ph, pw = panel.shape[:2]
    out = new(w, h)
    for y in range(h):
        sy = y if y < t else (ph - (h - y) if y >= h - b else t)
        for x in range(w):
            sx = x if x < l else (pw - (w - x) if x >= w - r else l)
            out[y, x] = panel[sy, sx]
    return out


def hud_mock(bg_path, out_name, boss=False, dialog=None, hp=(2, 1, 1), shards=3, title=None):
    fs = font_small()
    bg = native_backdrop(bg_path)
    ui = new(384, 216)
    hearts = load("hud_hearts")
    hm = json.load(open(os.path.join(SPRITES, "hud_hearts.json")))
    cw, ch = hm["cell"]
    ax, ay = hm["anchor"]
    full, half, empty = hp
    rows = ["full"] * full + ["half"] * half + ["empty"] * empty
    for i, r in enumerate(rows):
        row = hm["anims"][r]["row"]
        cell = hearts[row * ch:(row + 1) * ch, 0:cw]
        blit(ui, cell, 6 + 12 * i - ax, 6 - ay)
    shard = load("hud_shard")
    sm = json.load(open(os.path.join(SPRITES, "hud_shard.json")))
    scw, sch = sm["cell"]
    sx, sy = 6, 20
    blit(ui, shard[0:sch, 0:scw], sx - sm["anchor"][0], sy - sm["anchor"][1])
    fs.draw(ui, sx + 11, sy + 2, f"{shards}", UI["text"][1], shadow=UI["ink"][0])
    if boss:
        bar = filled_bar(0.55, 0.1)
        bm = json.load(open(os.path.join(SPRITES, "hud_bossbar.json")))
        bx = 192 - bm["anchor"][0]
        by = 192
        fs.draw(ui, bx + bm["fill_rect"][0], by - 10, "The Warden", UI["text"][1], shadow=UI["ink"][0])
        blit(ui, bar, bx, by)
    if dialog:
        panel = load("hud_panel")
        pm = json.load(open(os.path.join(SPRITES, "hud_panel.json")))
        box = nine_slice(panel, pm, 368, 40)
        blit(ui, box, 8, 168)
        for i, line in enumerate(dialog):
            fs.draw(ui, 8 + 6, 168 + 6 + i * fs.line_h, line, UI["text"][1], shadow=UI["ink"][0])
        prompts = load("hud_prompts")
        prm = json.load(open(os.path.join(SPRITES, "hud_prompts.json")))
        pcw, pch = prm["cell"]
        r = prm["anims"]["key_space"]["row"]
        w, h = prm["sizes"]["key_space"]
        glyph = prompts[r * pch:(r + 1) * pch, 0:pcw][pch - h:, :w]
        blit(ui, glyph, 8 + 368 - 6 - w, 168 + 40 - 5 - h)
    if title:
        fd = font_display()
        fd.draw(ui, (384 - fd.width(title)) // 2, 24, title, "#fff2dc", shadow=UI["ink"][0])
    out = bg.copy()
    m = ui[:, :, 3] > 0
    out[m] = ui[m][:, :3]
    save_preview(out, os.path.join(PREVIEWS, f"{PFX}{out_name}_1x.png"), 1)
    save_preview(out, os.path.join(PREVIEWS, f"{PFX}{out_name}_3x.png"), 3)
    return out


if __name__ == "__main__":
    for n, k in (("hud_hearts", 8), ("hud_shard", 8), ("hud_bossbar", 4)):
        on_grounds(load(n), k, f"{n}_{k}x")
    on_grounds(filled_bar(0.62, 0.12), 4, "hud_bossbar_filled_4x")
    prompts_grid()
    root = os.path.join(SPRITES, "..", "..")
    # backdrops are other artists' native-res mocks; flat ground is used if they're gone
    hud_mock(os.path.join(root, "assets", "previews", "_wip_bw_mock2_1x.png"), "hud_mock_dialog",
             title="BLOOMWILD RUINS",
             dialog=["The Prism has cracked. Its shards fell all across the islands.",
                     "Follow them, Glint, before the Warden wakes."])
    hud_mock(os.path.join(root, "assets", "previews", "summit_tiles_mock_1x.png"), "hud_mock_boss",
             boss=True, hp=(1, 1, 2), shards=12)
