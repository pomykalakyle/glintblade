"""Build assets/sprites/hero.png + hero.json and the hero_* previews.

uv run --no-project --with pillow --with numpy python3 tools/hero/build.py [--sheet-only]
"""
import json
import os
import sys
from PIL import Image

import anims
import eightway
from pix import scale, on_ground, hex_rgb, PAL, EMISSIVE_CHARS
from rig import frame, CELL, ANCHOR

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SPRITES = os.path.join(ROOT, "assets", "sprites")
PREVIEWS = os.path.join(ROOT, "out", "samples", "hero_v2style", "source_previews")

DIRS = ["down", "right", "up"]
DIRECTIONAL = ["idle", "run", "attack1", "attack2", "attack3", "dash", "charge", "hurt"]
SINGLE = ["spin", "death"]
# Directional anims added after the approved sheet; appended so existing row indices stay put.
APPENDED = ["charge_walk"]
LOOPING = {"idle", "run", "charge"}

# Region ground swatches from v2; use Cinder's dark obsidian for the chosen C graft.
GROUNDS = {
    "bloomwild": "#92a594",
    "glasswater": "#364657",
    "cinder": "#292c3c",
    "summit": "#46526a",
}


def directional_rows(base):
    """The down/right/up/left rows of one directional anim; left is mirrored and relit (rig.frame)."""
    out = []
    for d in DIRS + ["left"]:
        specs, durs = getattr(anims, base)("right" if d == "left" else d)
        if d == "left":
            for s in specs:
                s["flip"] = True
        res = [frame(s) for s in specs]
        out.append((f"{base}_{d}", [r[0] for r in res], [r[1] for r in res], durs))
    return out


def collect():
    """[(anim_name, frames, metas, durations)] in sheet row order."""
    out = []
    for base in DIRECTIONAL:
        out.extend(directional_rows(base))
    for base in SINGLE:
        specs, durs = getattr(anims, base)()
        res = [frame(s) for s in specs]
        out.append((base, [r[0] for r in res], [r[1] for r in res], durs))
        if base == "death":
            # a front view: the engine would mirror it facing left (reversing the light), so
            # left gets its own unflipped copy
            out.append(("death_left", [r[0] for r in res], [r[1] for r in res], durs))
    for base in APPENDED:
        out.extend(directional_rows(base))
    # Preserve all 39 approved rows exactly; new anatomy is appended only.
    for base in DIRECTIONAL + APPENDED:
        for d in eightway.DIAGONALS:
            specs, durs = eightway.diagonal(base, d)
            res = [frame(s) for s in specs]
            out.append((f"{base}_{d}", [r[0] for r in res], [r[1] for r in res], durs))
    for base in ("collapse", "spin", "fall", "land"):
        for d in eightway.DIRECTIONS:
            specs, durs = getattr(eightway, base)(d)
            res = [frame(s) for s in specs]
            out.append((f"{base}_{d}", [r[0] for r in res], [r[1] for r in res], durs))
    return out


def check(rows):
    """Collect used shade characters and report any clipped frame pixels."""
    problems = []
    used = set()
    for name, frames, metas, durs in rows:
        assert len(frames) == len(durs), name
        for i, (f, m) in enumerate(zip(frames, metas)):
            if m["clipped"]:
                problems.append(f"{name}[{i}] clipped {m['clipped']} px")
            for row in f.px:
                used.update(ch for ch in row if ch != ".")
    return problems, used


def main():
    """Export the hero sheet, manifest and source previews from authored grids."""
    global SPRITES, PREVIEWS
    if "--output" in sys.argv:
        output = os.path.abspath(sys.argv[sys.argv.index("--output") + 1])
        SPRITES = os.path.join(output, "sprites")
        PREVIEWS = os.path.join(output, "previews")
    rows = collect()
    problems, used = check(rows)
    for p in problems:
        print("PROBLEM", p)
    cols = max(len(r[1]) for r in rows)
    sheet = Image.new("RGBA", (cols * CELL, len(rows) * CELL), (0, 0, 0, 0))
    meta = {
        "cell": [CELL, CELL],
        "anchor": list(ANCHOR),
        "emissive": sorted({PAL[c] for c in used if c in EMISSIVE_CHARS}),
        "anims": {},
        "notes": (
            "Glint, by the hero pixel artist (hand-authored grids + a code rig, every pixel on the "
            "native grid; source in tools/hero/). The Long Quiet hero palette from "
            "assets/concept/palettes_v2.json; 16 colors. The two deepest cloak "
            "shades share the v2 shadow. Craft polish (2026-10-31): combo keys coil the "
            "shoulders over forward hips on the windup (side view hides the chest band), then "
            "counter-rotate through the strike with the rear boot planted and the lead boot "
            "stepping wide, and the hem swings through on recovery; timing, rows, cells and "
            "anchor unchanged, blade_tip/scarf_anchor regenerated from the art. Design: big hood with a shadow face and two cyan eyes, scarf "
            "knot at the throat, an A-line cloak ending above the knees with a front split, dark "
            "legs and boots planted apart, the sword hand held off the cloak with a sleeve. The "
            "crystal blade has the same construction in all 16 directions: white ridge, cyan lit "
            "face toward the upper-left light, deep-cyan shadow face, one white facet, 1 px point. "
            "Emissive = eyes (#71d5db, #edf9ef when flaring) and the blade's ridge, lit face and "
            "point (#63d7e0, #baf5ec, #edf9ef); the shadow face #429aa9 is deliberately not "
            "emissive, so the glow stays a hard-edged on-grid shape. The scarf tail and slash "
            "smears are left to the engine: scarf_anchor is the knot (per frame), blade_tip the "
            "blade's tip pixel (per frame), both in cell pixels. Durations are 60 Hz ticks matched "
            "to player.rs: attack1/2 = 17 ticks (hits on 3-8), attack3 = 24 (hits on 5-11), dash = "
            "10, hurt = 15, spin = 24. Dash frame 0 is the streak pose the engine copies for "
            "afterimages; death frame 0 is the full silhouette the engine shatters into pixels (the "
            "rest is a kneel-and-collapse for an engine that plays it out). Smear direction: facing "
            "right the blade sweeps top->bottom on attack1/3 and bottom->top on attack2, matching "
            "draw_slash. Every directional anim has a _left variant: the right-facing pose "
            "mirrored and re-shaded for upper-left light, with the blade drawn natively at the "
            "mirrored hand, and scarf_anchor/blade_tip mirrored to match. death_left is an "
            "unflipped copy of the front-view death. spin is directionless and must not be "
            "flipped (drawn turning clockwise from facing down; its left-facing step is relit). "
            "Silent before/after in-engine captures and exact geometry audit: tools/hero/style_v2.py. "
            "Locomotion pass (2026-10-31): the dash keys are a coiled launch, a long extension and "
            "a planted recovery (rows, counts, durations and anchor unchanged; afterimages copy "
            "whichever dash key is live). charge_walk_{down,right,up,left} are appended after "
            "death_left: the held charge upper body (same grip, blade and 20-tick pulse) over an "
            "8-key stepping cycle of 5 ticks, for player.rs's existing charge_walk request. "
            "Gait pass (2026-10-31, round 11): run_right/run_left hold a forward lean (hood 1 px "
            "and chest 1 px further ahead of the hips) with the hem trailing on the landing keys "
            "and following through on the pass and push-off keys; idle breath overlaps (shoulders "
            "and sword hand lead on keys 4 and 7, the hood follows one key later). Rows, counts, "
            "durations, loop flags, anchor and every other cell unchanged."
        ),
    }
    for r, (name, frames, metas, durs) in enumerate(rows):
        for c, f in enumerate(frames):
            sheet.paste(f.to_image(), (c * CELL, r * CELL))
        a = {
            "row": r,
            "frames": len(frames),
            "fps": round(60 * len(durs) / sum(durs), 2),
            "durations": durs,
            "loop": name.split("_")[0] in LOOPING,
            "scarf_anchor": [m["knot"] for m in metas],
            "blade_tip": [m["tip"] for m in metas],
            "support_spans": [m["support_spans"] for m in metas],
        }
        meta["anims"][name] = a
    os.makedirs(SPRITES, exist_ok=True)
    sheet.save(os.path.join(SPRITES, "hero.png"))
    with open(os.path.join(SPRITES, "hero.json"), "w") as fh:
        json.dump(meta, fh, indent=1)
    print("wrote hero.png", sheet.size, "rows", len(rows), "colors", len({PAL[c] for c in used}))
    if "--sheet-only" not in sys.argv:
        previews(rows, sheet)


def label_free_strip(frames, ground, k, pad=1):
    """Place key poses on a ground swatch and enlarge by an integer factor."""
    w = CELL * len(frames) + pad * (len(frames) - 1)
    im = Image.new("RGBA", (w, CELL), (12, 12, 20, 255))
    for i, f in enumerate(frames):
        im.paste(on_ground(f.to_image(), ground), (i * (CELL + pad), 0))
    return scale(im, k)


def save_gif(rgb_frames, durs, path, *, exact_ticks=False):
    """Write an animated GIF with one exact shared palette (the art has far fewer than 256
    colors, so nothing is quantized away and colors can't shift between frames)."""
    colors = []
    seen = set()
    for f in rgb_frames:
        for _, c in f.getcolors(maxcolors=1 << 16):
            if c not in seen:
                seen.add(c)
                colors.append(c)
    assert len(colors) <= 256, f"{path}: {len(colors)} colors"
    pal = Image.new("P", (1, 1))
    flat = [v for c in colors for v in c]
    pal.putpalette(flat + [0] * (768 - len(flat)))
    ims = [f.quantize(palette=pal, dither=Image.Dither.NONE) for f in rgb_frames]
    if exact_ticks:
        # GIF clocks use10ms quanta. Round cumulative edges so a60Hz sweep
        # alternates20/10/20ms rather than slowing every tick to20ms.
        edges, total = [0], 0
        for t in durs:
            total += t
            edges.append(round(total * 1000 / 60 / 10) * 10)
        ms = [max(10, b - a) for a, b in zip(edges, edges[1:])]
    else:
        ms = [max(20, round(t * 1000 / 60 / 10) * 10) for t in durs]
    ims[0].save(path, save_all=True, append_images=ims[1:], duration=ms, loop=0, disposal=1, optimize=False)


def gif(frames, durs, path, ground, k=6, crop=None):
    """Save an exact-palette animation of authored frames on a ground swatch."""
    ims = []
    for f in frames:
        im = on_ground(f.to_image(), ground)
        if crop:
            im = im.crop(crop)
        ims.append(scale(im, k).convert("RGB"))
    save_gif(ims, durs, path)


def previews(rows, sheet):
    """Export native-grid source previews using the v2 region swatches."""
    os.makedirs(PREVIEWS, exist_ok=True)
    # whole sheet at 6x on each region ground
    for region, g in GROUNDS.items():
        bg = Image.new("RGBA", sheet.size, hex_rgb(g) + (255,))
        bg.alpha_composite(sheet)
        scale(bg, 6 if region == "bloomwild" else 3).save(os.path.join(PREVIEWS, f"hero_sheet_{region}.png"))
    # per-anim GIFs at 6x (bloomwild grass) and strips at 8x
    crop = (0, 0, CELL, CELL)
    cw = CELL
    for name, frames, metas, durs in rows:
        gif(frames, durs, os.path.join(PREVIEWS, f"hero_{name}.gif"), GROUNDS["bloomwild"], crop=crop)
    # one GIF per base anim with all directions side by side, on the cave ground too
    for base in DIRECTIONAL + APPENDED:
        group = [r for r in rows if r[0][:-len(r[0].split("_")[-1]) - 1] == base and not r[0].endswith("_left")]
        n = max(len(r[1]) for r in group)
        durs = group[0][3]
        ims = []
        for i in range(n):
            canvas = Image.new("RGBA", (cw * 3 * 2, cw), (0, 0, 0, 255))
            for gi, gname in enumerate(["bloomwild", "glasswater"]):
                for j, r in enumerate(group):
                    f = r[1][min(i, len(r[1]) - 1)]
                    canvas.paste(on_ground(f.to_image(), GROUNDS[gname]).crop(crop), ((gi * 3 + j) * cw, 0))
            ims.append(scale(canvas, 4).convert("RGB"))
        save_gif(ims, durs, os.path.join(PREVIEWS, f"hero_{base}.gif"))
    # key poses strip at 8x
    keys = []
    pick = [("idle_down", 0), ("idle_right", 0), ("idle_up", 0), ("run_right", 3), ("attack1_right", 0),
            ("attack1_right", 2), ("attack3_right", 1), ("attack3_down", 3), ("dash_right", 0),
            ("charge_down", 1), ("spin", 2), ("hurt_down", 0)]
    by = {r[0]: r for r in rows}
    for n_, i in pick:
        keys.append(by[n_][1][i])
    for region, g in GROUNDS.items():
        label_free_strip(keys, g, 6 if region in ("bloomwild", "glasswater") else 3).save(
            os.path.join(PREVIEWS, f"hero_keyposes_{region}.png"))
    # in-context views come from the real engine: tools/hero/capture.py

if __name__ == "__main__":
    main()
