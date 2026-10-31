"""Before/after evidence for the hero craft-polish pass: headless in-engine captures in every
region, cropped motion strips around Glint, and 8x sheet boards with blade_tip / scarf_anchor
markers.

Runs only the game's headless `shot` subcommand (never a window, browser or audio):

    uv run --no-project --with pillow --with numpy python3 tools/hero/craft_polish.py capture \
        --bin out/samples/hero_craft_polish/before/glintblade_before --tag before
    uv run --no-project --with pillow --with numpy python3 tools/hero/craft_polish.py boards
    uv run --no-project --with pillow --with numpy python3 tools/hero/craft_polish.py clips
    uv run --no-project --with pillow --with numpy python3 tools/hero/craft_polish.py audit

Outputs go to out/samples/hero_craft_polish/.
"""
import argparse
import json
import os
import subprocess

import numpy as np
from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "out", "samples", "hero_craft_polish")
ZONES = {"bloomwild": None, "glasswater": "224,432", "cinder": None, "summit": None}

# One silent scripted session: idle, run right, right combo, turn and left combo, down and up
# combos, a right dash, a left turn-and-dash, then a charge released into the spin.
SCRIPT = [
    ("idle", "W40"),
    ("run", "R24"),
    ("combo_right", "A1 W9 A1 W9 A1 W40"),
    ("turn_left", "L6 W4"),
    ("combo_left", "A1 W9 A1 W9 A1 W40"),
    ("combo_down", "D3 W4 A1 W9 A1 W9 A1 W40"),
    ("combo_up", "U3 W4 A1 W9 A1 W9 A1 W40"),
    ("dash_right", "R6 RX1 W24"),
    ("dash_left", "L6 LX1 W24"),
    ("spin", "A50 W44"),
]
CROP_W, CROP_H = 88, 64


def segments():
    """Return [(name, first_tick, last_tick)] for every scripted segment (1-based ticks)."""
    out, t = [], 1
    for name, s in SCRIPT:
        n = 0
        for tok in s.split():
            digits = tok.lstrip("ABCDLPRUWX")
            n += int(digits) if digits else 1
        out.append((name, t, t + n - 1))
        t += n
    return out


def script_text():
    """Join the segment scripts into one `shot --script` argument."""
    return " ".join(s for _, s in SCRIPT)


def cyan_center(a):
    """Median of Glint's cyan eye/blade pixels (cyan belongs only to Glint in style v2)."""
    r, g, b = (a[..., i].astype(int) for i in range(3))
    m = (g > 150) & (b > 150) & (r < g - 45) & (r < b - 45)
    m[:24] = False  # HUD row
    ys, xs = np.nonzero(m)
    if len(xs) < 2:
        return None
    return float(np.median(xs)), float(np.median(ys))


def capture(binary, tag):
    """Render every tick of the scripted session headless in all four regions."""
    binary = os.path.abspath(binary)
    for zone, pos in ZONES.items():
        out = os.path.join(OUT, tag, zone)
        os.makedirs(out, exist_ok=True)
        cmd = [binary, "shot", "--zone", zone, "--peaceful", "--script", script_text(),
               "--every", "1", "--scale", "1", "--out", out]
        if pos:
            cmd[4:4] = ["--pos", pos]
        with open(os.path.join(OUT, tag, f"capture_{zone}.log"), "w") as log:
            log.write(" ".join(cmd) + "\n")
            subprocess.run(cmd, check=True, stdout=log, stderr=subprocess.STDOUT, cwd=ROOT)
        print("captured", tag, zone)


def load(tag, zone, t):
    """Open one native capture frame as an RGB array, or None when it is absent."""
    p = os.path.join(OUT, tag, zone, f"f{t:05d}.png")
    return np.asarray(Image.open(p).convert("RGB")) if os.path.exists(p) else None


def strip(tag, zone, t0, t1, step, k, center=None):
    """A row of crops around Glint for ticks t0..t1 (one fixed crop per segment, so ground
    motion stays visible), enlarged k times with nearest-neighbour."""
    frames = [(t, load(tag, zone, t)) for t in range(t0, t1 + 1, step)]
    frames = [(t, a) for t, a in frames if a is not None]
    if center is None:
        cs = [c for c in (cyan_center(a) for _, a in frames) if c]
        center = (np.median([c[0] for c in cs]), np.median([c[1] for c in cs])) if cs else (192, 108)
    x0 = int(round(center[0])) - CROP_W // 2
    y0 = int(round(center[1])) - CROP_H // 2 - 2
    tiles = [Image.fromarray(a).crop((x0, y0, x0 + CROP_W, y0 + CROP_H)) for _, a in frames]
    im = Image.new("RGB", (len(tiles) * (CROP_W + 1) - 1, CROP_H), (12, 12, 18))
    for i, tile in enumerate(tiles):
        im.paste(tile, (i * (CROP_W + 1), 0))
    return im.resize((im.width * k, im.height * k), Image.NEAREST), center


def motion_boards():
    """Before-over-after motion strips per region and segment, at 3x."""
    segs = {n: (a, b) for n, a, b in segments()}
    plan = [("combo_right", 2), ("combo_left", 2), ("combo_down", 2), ("combo_up", 2),
            ("run", 2), ("turn_left", 1), ("dash_right", 2), ("dash_left", 2), ("spin", 3)]
    for zone in ZONES:
        for name, step in plan:
            t0, t1 = segs[name]
            if name.startswith("combo"):
                t0 += 3 if name in ("combo_down", "combo_up") else 0
                t1 = min(t1, t0 + 33)
            if name == "spin":
                t0 += 30
            rows = []
            center = None
            for tag in ("before", "after"):
                if not os.path.isdir(os.path.join(OUT, tag, zone)):
                    continue
                im, center = strip(tag, zone, t0, t1, step, 3, center)
                rows.append(im)
            if not rows:
                continue
            w = max(r.width for r in rows)
            board = Image.new("RGB", (w, sum(r.height for r in rows) + 6 * (len(rows) - 1)), (40, 10, 10))
            y = 0
            for r in rows:
                board.paste(r, (0, y))
                y += r.height + 6
            os.makedirs(os.path.join(OUT, "boards"), exist_ok=True)
            board.save(os.path.join(OUT, "boards", f"{zone}_{name}_before_over_after_3x.png"))


def clips():
    """Silent GIFs, before | after side by side at 3x, sampled every 2 ticks at 33 ms (real
    time), for each region's combos, turn, dashes and spin."""
    segs = {n: (a, b) for n, a, b in segments()}
    os.makedirs(os.path.join(OUT, "clips"), exist_ok=True)
    for zone in ZONES:
        for name in ("combo_right", "combo_left", "combo_down", "combo_up", "turn_left",
                     "dash_right", "dash_left", "spin"):
            t0, t1 = segs[name]
            ticks = range(t0, t1 + 1, 2)
            cs = [cyan_center(load("before", zone, t)) for t in ticks]
            cs = [c for c in cs if c]
            cx = int(np.median([c[0] for c in cs])) if cs else 192
            cy = int(np.median([c[1] for c in cs])) if cs else 108
            box = (cx - CROP_W // 2, cy - CROP_H // 2 - 2, cx + CROP_W // 2, cy + CROP_H // 2 - 2)
            frames = []
            for t in ticks:
                pair = Image.new("RGB", (CROP_W * 2 + 2, CROP_H), (90, 0, 0))
                for i, tag in enumerate(("before", "after")):
                    a = load(tag, zone, t)
                    if a is not None:
                        pair.paste(Image.fromarray(a).crop(box), (i * (CROP_W + 2), 0))
                frames.append(pair.resize((pair.width * 3, pair.height * 3), Image.NEAREST))
            tall = Image.new("RGB", (frames[0].width, frames[0].height * len(frames)))
            for i, f in enumerate(frames):
                tall.paste(f, (0, i * f.height))
            pal = tall.quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
            ps = [f.quantize(palette=pal, dither=Image.Dither.NONE) for f in frames]
            ps[0].save(os.path.join(OUT, "clips", f"{zone}_{name}_before_vs_after_3x.gif"), save_all=True,
                       append_images=ps[1:], duration=33, loop=0, disposal=1)


def cell_board(sheet_dir, names, k=8, ground=(146, 165, 148)):
    """Selected sheet rows at k-times, cropped to the figure, with the blade_tip (red) and
    scarf_anchor (orange) metadata marked as one-pixel outlines."""
    im = Image.open(os.path.join(sheet_dir, "hero.png")).convert("RGBA")
    meta = json.load(open(os.path.join(sheet_dir, "hero.json")))
    c = meta["cell"][0]
    crop = (4, 4, 44, 44)
    cw, ch = crop[2] - crop[0], crop[3] - crop[1]
    cols = max(meta["anims"][n]["frames"] for n in names)
    board = Image.new("RGB", (cols * (cw + 1) * k, len(names) * (ch + 1) * k), (20, 20, 28))
    for r, n in enumerate(names):
        a = meta["anims"][n]
        for f in range(a["frames"]):
            cell = im.crop((f * c, a["row"] * c, f * c + c, a["row"] * c + c))
            bg = Image.new("RGBA", (c, c), ground + (255,))
            bg.alpha_composite(cell)
            bg = bg.crop(crop).resize((cw * k, ch * k), Image.NEAREST)
            d = ImageDraw.Draw(bg)
            for key, col in (("blade_tip", (230, 40, 40)), ("scarf_anchor", (255, 160, 0))):
                p = a[key][f]
                if p:
                    x, y = (p[0] - crop[0]) * k, (p[1] - crop[1]) * k
                    d.rectangle((x, y, x + k - 1, y + k - 1), outline=col)
            board.paste(bg.convert("RGB"), (f * (cw + 1) * k, r * (ch + 1) * k))
    return board


def sheet_boards():
    """8x before/after boards of every attack, dash, spin and run row."""
    groups = {
        "attacks_side": ["attack1_right", "attack2_right", "attack3_right",
                         "attack1_left", "attack2_left", "attack3_left"],
        "attacks_down_up": ["attack1_down", "attack2_down", "attack3_down",
                            "attack1_up", "attack2_up", "attack3_up"],
        "run_dash_spin": ["run_right", "run_left", "dash_right", "dash_left", "dash_down", "dash_up", "spin"],
    }
    os.makedirs(os.path.join(OUT, "boards"), exist_ok=True)
    for g, names in groups.items():
        for tag, src in (("before", os.path.join(OUT, "before")),
                         ("after", os.path.join(ROOT, "assets", "sprites"))):
            cell_board(src, names).save(os.path.join(OUT, "boards", f"sheet_{g}_{tag}_8x.png"))


def audit():
    """Check the new sheet against the frozen before sheet: same size, cell, anchor, rows,
    frame counts, durations and loop flags; binary alpha; every color in palettes_v2.json;
    blade_tip on a blade pixel and scarf_anchor on (or touching) a scarf pixel; report which
    cells changed. Writes out/samples/hero_craft_polish/audit.json."""
    before_dir = os.path.join(OUT, "before")
    bm = json.load(open(os.path.join(before_dir, "hero.json")))
    am = json.load(open(os.path.join(ROOT, "assets", "sprites", "hero.json")))
    bi = np.asarray(Image.open(os.path.join(before_dir, "hero.png")).convert("RGBA"))
    ai = np.asarray(Image.open(os.path.join(ROOT, "assets", "sprites", "hero.png")).convert("RGBA"))
    pal = json.load(open(os.path.join(ROOT, "assets", "concept", "palettes_v2.json")))

    def hexes(o):
        """Collect every #rrggbb string in a nested palette structure."""
        if isinstance(o, str):
            return {o.lower()} if o.startswith("#") and len(o) == 7 else set()
        if isinstance(o, dict):
            return set().union(*[hexes(v) for v in o.values()]) if o else set()
        if isinstance(o, list):
            return set().union(*[hexes(v) for v in o]) if o else set()
        return set()

    allowed = hexes(pal)
    assert bi.shape == ai.shape
    assert bm["cell"] == am["cell"] and bm["anchor"] == am["anchor"]
    assert set(bm["anims"]) == set(am["anims"])
    alpha = set(np.unique(ai[..., 3]).tolist())
    assert alpha <= {0, 255}, alpha
    colors = {"#%02x%02x%02x" % tuple(p[:3]) for p in ai.reshape(-1, 4) if p[3]}
    off = sorted(colors - allowed)
    blade = {"#63d7e0", "#baf5ec", "#edf9ef", "#429aa9"}
    scarf = {"#416974", "#79a6ae", "#b4d1ce"}
    c = am["cell"][0]
    changed, problems, nonempty = [], [], 0
    for name, a in am["anims"].items():
        b = bm["anims"][name]
        for key in ("row", "frames", "durations", "loop"):
            if a[key] != b[key]:
                problems.append(f"{name} {key} changed")
        for f in range(a["frames"]):
            y0, x0 = a["row"] * c, f * c
            cell = ai[y0:y0 + c, x0:x0 + c]
            nonempty += int(cell[..., 3].any())
            if (cell != bi[y0:y0 + c, x0:x0 + c]).any():
                changed.append(f"{name}#{f}")

            def col(x, y):
                """Return the hex color of a cell pixel, or None when transparent."""
                if 0 <= x < c and 0 <= y < c and cell[y, x, 3]:
                    return "#%02x%02x%02x" % tuple(cell[y, x, :3])
                return None

            tip = a["blade_tip"][f]
            if tip and col(*tip) not in blade:
                problems.append(f"{name}#{f} blade_tip {tip} not on blade ({col(*tip)})")
            k = a["scarf_anchor"][f]
            near = [col(k[0] + dx, k[1] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1)]
            if k and not any(n in scarf for n in near):
                problems.append(f"{name}#{f} scarf_anchor {k} not at scarf")
    report = {"size": list(ai.shape[:2][::-1]), "rows": len(am["anims"]), "nonempty_cells": nonempty,
              "colors": len(colors), "off_palette": off, "alpha_values": sorted(alpha),
              "changed_cells": changed, "problems": problems}
    with open(os.path.join(OUT, "audit.json"), "w") as fh:
        json.dump(report, fh, indent=1)
    print(json.dumps({k: (v if k != "changed_cells" else len(v)) for k, v in report.items()}, indent=1))


def main():
    """Dispatch the capture, audit or board-building step."""
    ap = argparse.ArgumentParser()
    ap.add_argument("step", choices=["capture", "boards", "segments", "audit", "clips"])
    ap.add_argument("--bin")
    ap.add_argument("--tag")
    a = ap.parse_args()
    if a.step == "capture":
        capture(a.bin, a.tag)
    elif a.step == "audit":
        audit()
    elif a.step == "clips":
        clips()
    elif a.step == "segments":
        print(json.dumps(segments()))
    else:
        sheet_boards()
        motion_boards()


if __name__ == "__main__":
    main()
