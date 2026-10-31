"""Export the Warden sheets, manifests, layout and previews.

usage (repo root): uv run --no-project --with pillow --with numpy python3 tools/warden/export.py [--out DIR] [--previews DIR]
Writes the five warden_*.{png,json} part sheets and the unchanged warden_layout.json to
assets/sprites (or --out DIR). --previews DIR also writes enlarged sheet, pose and GIF previews.
"""

import json
from pathlib import Path

from PIL import Image

import body
import crystal
import fist
import head
import heart
from wlib import EMISSIVE, PAL, gif, hex_rgb, on_bg, sheet, write_json

ROOT = Path(__file__).resolve().parents[2]
SPR = ROOT / "assets" / "sprites"
PRE = ROOT / "assets" / "previews"
BG = "#46526a"  # summit floor ramp, step 2
SKY = "#1f2838"

# ------------------------------------------------------------------ sheets
# name -> (module, draw func, cell, anchor, [(anim, frames, durations, loop)], notes, extra)
SHEETS = {
    "warden_body": (
        body, lambda a, i: body.body(a, i), (body.W, body.H), body.ANCHOR,
        [("idle", 1, [60], True), ("awake", 1, [60], True), ("damaged", 1, [60], True)],
        "Weir Keeper: a section of an old garden weir that stood up. Unequal gate towers (the left "
        "tall with a spillway slot, the right snapped), outboard shoulders stepping down past them, "
        "a sluice gate of stone planks holding the heart's valve ring, a crest notch for the head, "
        "and a culvert arch between stub piers, darker below an old tide line, with stains, mineral "
        "streaks and moss. Anchor = ground under the middle of the feet. idle: dormant, channels "
        "dry. awake: the guide channels and drain carry dull iris water (not emissive). damaged: "
        "two gate planks fallen out, the left tower's crown corner and the right tower's outer slab "
        "broken away, the right shoulder's top block gone, a fracture from the valve ring. The heart "
        "socket, neck notch and both crystal seats stay in place in every state. Light from the "
        "upper left.",
        {},
    ),
    "warden_head": (
        head, lambda a, i: head.head(a, i), (head.W, head.H), head.ANCHOR,
        [("idle", 4, [14, 50, 40, 50], True), ("flare", 4, [6, 5, 6, 24], False), ("roar", 6, [12, 6, 8, 5, 5, 16], False)],
        "A heavy level lintel over a dark recess with one narrow iris slit in a black socket; the "
        "slit's ends dip and the iris is a low dome resting on the lower lid, watching the ground in "
        "front. Anchor = the neck point (bottom centre). "
        "idle: the eye watches: a short heavy blink, open, a slow glance left, back. flare: the slit fills and blazes (telegraph before an attack). "
        "roar: dip with the eye squinted, lift 2/3/3/3 with a 1 px rumble as it snaps open, settle; the lintel lifts one more pixel at the "
        "peak so the slit opens taller. The lift is baked into the frames (peak 3 px), so the body's "
        "dark neck notch shows under the head.",
        {},
    ),
    "warden_fist": (
        fist, lambda a, i: fist.fist(a, i), (fist.W, fist.H), fist.ANCHOR,
        [("clenched", 4, [20, 20, 20, 20], True), ("open", 4, [20, 20, 20, 20], True),
         ("open_lit", 4, [8, 6, 6, 8], True), ("slam", 5, [8, 26, 3, 12, 14], False)],
        "Floating sluice counterweight hand: wrist cuff (the crystal seat), a weight block with a "
        "lifting slot, a hooked thumb (toward the body) and two curled clamp fingers. Drawn as the "
        "screen-right hand; mirror it for the left. Anchor = bottom centre of the clamp (the slam "
        "contact point). clenched / open: hovering bob (open = the clamp spread flat on the floor, "
        "used kneeling and dormant). open_lit: the lifting slot fills with iris light (charging the "
        "beam). slam: 0 rise (lifted 9, slot starting to fill), 1 wind-up hold (lifted 23 and stretched 3, slot blazing = "
        "the telegraph), 2 falling (lifted 3, stretched 5, streaks), 3 impact squash 6 px, clamp pressed flat, with stone "
        "chips (spawn the shockwave here), 4 recover. The lifts are baked in and cancelled by the "
        "runtime; keep the fist anchor on the slam target for the whole anim.",
        {},
    ),
    "warden_heart": (
        heart, lambda a, i: heart.heart(a, i), (heart.W, heart.H), heart.ANCHOR,
        [("whole", 4, [18, 10, 14, 10], True), ("cracked", 4, [18, 10, 14, 10], True),
         ("badly_cracked", 4, [6, 10, 4, 14], True), ("shattered", 4, [4, 4, 4, 4], True)],
        "The cut iris heart in the gate's valve ring. Anchor = gem centre. A step-cut gem: bright "
        "upper-left edge, pale left facets, mid right facets and one dark lower-right facet. Only "
        "the table core and the bright edge are emissive. badly_cracked stutters and has lost its "
        "lower-right facet. shattered: the facets split off a blazing core (use it for the death "
        "burst). This is the Warden's light source: one point light.",
        {"light": {"color": "#e2d8e9", "radius": 28, "offset": [0, 0]}},
    ),
    "warden_crystal": (
        crystal, lambda a, i: crystal.crystal(a, i), (crystal.W, crystal.H), crystal.ANCHOR,
        [("intact", 4, [18, 10, 14, 10], True), ("cracked", 4, [6, 12, 4, 16], True), ("broken", 1, [60], True)],
        "Weak-point prism shards wedged in a stone sill (the gate towers' crowns and the hands' "
        "cuffs). Drawn for the screen-left shoulder; mirror it for the right. Anchor = where the "
        "sill sits on its seat. intact pulses in step with the heart (same durations). cracked: tip "
        "snapped, light flickers. broken: dead stumps, no glow.",
        {},
    ),
}

# emissive keys that stay plain lit material in these sheets (deliberate: the heart's colored
# bevels are glass lit by the core, not light sources)
NOT_EMISSIVE = {"P2"}

# ------------------------------------------------------------------ layout
LAYOUT = {
    "notes": (
        "Offsets are [dx, dy] from the body anchor (the Warden's ground position, body pixel "
        "(60, 109)) to each part's own anchor. Mirrored parts are flipped horizontally around their "
        "anchor (anchor x becomes cell_w - 1 - x). Draw in draw_order (back to front). The fists "
        "float: move them freely during attacks (a slam lands where the fist anchor is). Assembled "
        "height: about 120 px from the crystal tips to the feet, 114 px to the top of the hood."
    ),
    "body": {"sheet": "warden_body", "offset": [0, 0]},
    "parts": {
        "crystal_left": {"sheet": "warden_crystal", "offset": [-36, -100], "mirror": False},
        "crystal_right": {"sheet": "warden_crystal", "offset": [36, -100], "mirror": True},
        "head": {"sheet": "warden_head", "offset": [0, -75], "mirror": False},
        "heart": {"sheet": "warden_heart", "offset": [0, -60], "mirror": False},
        "fist_left": {"sheet": "warden_fist", "offset": [-45, -11], "mirror": True},
        "fist_right": {"sheet": "warden_fist", "offset": [45, -11], "mirror": False},
    },
    "draw_order": ["body", "crystal_left", "crystal_right", "head", "heart", "fist_left", "fist_right"],
    "hurtboxes": {
        "notes": "Suggested vulnerable circles, [dx, dy, r] from the body anchor.",
        "heart": [0, -60, 9],
        "crystal_left": [-35, -110, 7],
        "crystal_right": [35, -110, 7],
    },
    "bob": {"notes": "Suggested idle breathing: body, head, heart and crystals rise 1 px together every ~1 s; fists bob on their own (baked in)."},
}


def frames_of(spec):
    """Draw every authored animation frame for one part specification."""
    _, fn, cell, _, anims, _, _ = spec
    return [[fn(a, i) for i in range(n)] for a, n, _, _ in anims]


def export_sheets(spr=None):
    """Write the Warden part sheets and compatible manifests and layout."""
    spr = Path(spr) if spr else SPR
    out = {}
    for name, spec in SHEETS.items():
        mod, fn, cell, anchor, anims, notes, extra = spec
        rows = frames_of(spec)
        img = sheet(rows, cell)
        img.save(spr / f"{name}.png")
        keys = set()
        for r in rows:
            for cv in r:
                keys |= cv.keys()
        emissive = [PAL[k] for k in EMISSIVE if k in keys and k not in NOT_EMISSIVE]
        man = {"cell": list(cell), "anchor": list(anchor), "emissive": emissive, "anims": {}}
        for j, (a, n, dur, loop) in enumerate(anims):
            man["anims"][a] = {"row": j, "frames": n, "fps": round(60 * n / sum(dur), 2), "durations": dur, "loop": loop}
        man.update(extra)
        man["notes"] = notes
        write_json(spr / f"{name}.json", man)
        out[name] = (img, rows)
    write_json(spr / "warden_layout.json", LAYOUT)
    return out


# ------------------------------------------------------------------ assembly
def part_image(name, anim, frame):
    """Draw one part frame and return its authored anchor."""
    _, fn, _, anchor, _, _, _ = SHEETS[name]
    return fn(anim, frame).image(), anchor


def assemble(pose, size=(150, 140), ground=(75, 132), fist_moves=None):
    """pose: dict part -> (anim, frame). Returns an RGBA image with the Warden's anchor at `ground`."""
    img = Image.new("RGBA", size, (0, 0, 0, 0))
    gx, gy = ground
    fist_moves = fist_moves or {}
    for part in LAYOUT["draw_order"]:
        if part == "body":
            sheet_name, off, mirror = "warden_body", [0, 0], False
        else:
            p = LAYOUT["parts"][part]
            sheet_name, off, mirror = p["sheet"], list(p["offset"]), p["mirror"]
        if part not in pose:
            continue
        anim, fr = pose[part]
        im, (ax, ay) = part_image(sheet_name, anim, fr)
        if mirror:
            im = im.transpose(Image.FLIP_LEFT_RIGHT)
            ax = im.width - 1 - ax
        mx, my = fist_moves.get(part, (0, 0))
        img.alpha_composite(im, (gx + off[0] + mx - ax, gy + off[1] + my - ay))
    return img


POSES = {
    "idle": {"body": ("idle", 0), "crystal_left": ("intact", 0), "crystal_right": ("intact", 0), "head": ("idle", 1),
             "heart": ("whole", 0), "fist_left": ("clenched", 0), "fist_right": ("clenched", 0)},
    "waking": {"body": ("awake", 0), "crystal_left": ("intact", 2), "crystal_right": ("intact", 2), "head": ("flare", 3),
               "heart": ("whole", 2), "fist_left": ("open_lit", 2), "fist_right": ("open_lit", 2)},
    "slam": {"body": ("awake", 0), "crystal_left": ("intact", 1), "crystal_right": ("intact", 1), "head": ("flare", 2),
             "heart": ("whole", 1), "fist_left": ("clenched", 1), "fist_right": ("slam", 1)},
    "roar": {"body": ("awake", 0), "crystal_left": ("cracked", 0), "crystal_right": ("intact", 2), "head": ("roar", 3),
             "heart": ("cracked", 2), "fist_left": ("open", 1), "fist_right": ("open", 1)},
    "last stand": {"body": ("damaged", 0), "crystal_left": ("broken", 0), "crystal_right": ("cracked", 0), "head": ("flare", 3),
                   "heart": ("badly_cracked", 0), "fist_left": ("clenched", 0), "fist_right": ("slam", 3)},
    "dying": {"body": ("damaged", 0), "crystal_left": ("broken", 0), "crystal_right": ("broken", 0), "head": ("roar", 5),
              "heart": ("shattered", 1), "fist_left": ("open", 0), "fist_right": ("open", 0)},
}
POSE_FIST_MOVES = {"slam": {"fist_right": (4, 14)}, "last stand": {"fist_right": (4, 14)},
                   "roar": {"fist_left": (-2, -14), "fist_right": (2, -14)}, "dying": {"fist_left": (-2, 10), "fist_right": (2, 10)}}


def glint():
    """Load the current hero idle cell for scale comparisons."""
    hero = ROOT / "assets" / "sprites" / "hero.png"
    if not hero.exists():
        return None, None
    man = json.loads((ROOT / "assets" / "sprites" / "hero.json").read_text())
    cw, ch = man["cell"]
    row = man["anims"].get("idle_right", man["anims"]["idle_down"])["row"]
    im = Image.open(hero).convert("RGBA").crop((0, row * ch, cw, row * ch + ch))
    return im, man["anchor"]


def ground_patch(w, h):
    """A plain summit floor color with the sky above the horizon line, for previews only."""
    img = Image.new("RGBA", (w, h), hex_rgb(BG) + (255,))
    return img


def write_previews(out, pre=None):
    """Write nearest-neighbor sheet, pose and silent animation previews (6x sheets over the summit floor)."""
    pre = Path(pre) if pre else PRE
    pre.mkdir(parents=True, exist_ok=True)
    for name, (img, rows) in out.items():
        on_bg(img, BG, 6, pad=2).save(pre / f"{name}_sheet_6x.png")
        spec = SHEETS[name]
        for (a, n, dur, loop), r in zip(spec[4], rows):
            if n > 1:
                frames = [on_bg(cv.image(), BG, 6, pad=2) for cv in r]
                gif(frames, pre / f"{name}_{a}.gif", [max(20, round(d * 1000 / 60)) for d in dur])
    # assembled poses next to Glint, 1x and 3x
    g, ga = glint()
    names = list(POSES)
    cw, chh = 150, 140
    strip = ground_patch(cw * 3, chh * 2)
    for i, p in enumerate(names):
        cx, cy = (i % 3) * cw, (i // 3) * chh
        strip.alpha_composite(assemble(POSES[p], fist_moves=POSE_FIST_MOVES.get(p)), (cx, cy))
        if g is not None:
            strip.alpha_composite(g, (cx + 12 - ga[0], cy + 134 - ga[1]))
    strip.save(pre / "warden_poses_1x.png")
    strip.resize((strip.width * 3, strip.height * 3), Image.NEAREST).save(pre / "warden_poses_3x.png")
    # idle loop GIF (heart, crystals, head and fists animate together), 4x
    seq = []
    for t in range(8):
        f = ground_patch(cw, chh)
        pose = {"body": ("idle", 0), "crystal_left": ("intact", t % 4), "crystal_right": ("intact", t % 4),
                "head": ("idle", t % 4), "heart": ("whole", t % 4), "fist_left": ("clenched", t % 4),
                "fist_right": ("clenched", (t + 2) % 4)}
        f.alpha_composite(assemble(pose))
        if g is not None:
            f.alpha_composite(g, (12 - ga[0], 134 - ga[1]))
        seq.append(f.resize((cw * 4, chh * 4), Image.NEAREST))
    gif(seq, pre / "warden_idle_assembled.gif", [300, 160, 230, 160] * 2)
    # slam sequence GIF
    seq, durs = [], []
    steps = [("idle", 1, "clenched", 0, 400), ("flare", 0, "slam", 0, 130), ("flare", 1, "slam", 1, 150),
             ("flare", 2, "slam", 1, 280), ("flare", 3, "slam", 2, 50), ("flare", 3, "slam", 3, 200), ("flare", 3, "slam", 4, 230)]
    for hanim, hf, fanim, ff, d in steps:
        f = ground_patch(cw, chh)
        pose = {"body": ("awake", 0), "crystal_left": ("intact", 1), "crystal_right": ("intact", 1), "head": (hanim, hf),
                "heart": ("whole", 1), "fist_left": ("clenched", 1), "fist_right": (fanim, ff)}
        f.alpha_composite(assemble(pose, fist_moves={"fist_right": (4, 14)}))
        if g is not None:
            f.alpha_composite(g, (12 - ga[0], 134 - ga[1]))
        seq.append(f.resize((cw * 4, chh * 4), Image.NEAREST))
        durs.append(d)
    gif(seq, pre / "warden_slam_assembled.gif", durs)


if __name__ == "__main__":
    import sys

    argv = sys.argv[1:]
    dest = argv[argv.index("--out") + 1] if "--out" in argv else None
    out = export_sheets(dest)
    if "--previews" in argv:
        write_previews(out, argv[argv.index("--previews") + 1])
    for name in SHEETS:
        print(name, Image.open((Path(dest) if dest else SPR) / f"{name}.png").size)
