"""Lint the Warden sheets against the runtime rig contract and report source-pixel mass.

usage (repo root): uv run --no-project --with pillow python3 tools/warden/rigcheck.py [sprites_dir] [--json out.json]
Mirrors src/entities/boss/art.rs (bottom, solid_top, baked_lift, peak_lift) and the
expectations in src/entities/boss/tests.rs, plus binary alpha, palette and manifest checks.
Exits non-zero on any contract failure.
"""

import json
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
PALETTE = ROOT / "assets" / "concept" / "palettes_v2.json"

# The frozen runtime contract (cells, anchors, rows, durations, loop flags, baked motion).
CONTRACT = {
    "warden_body": ((120, 112), (60, 109), [("idle", 1, [60], True), ("awake", 1, [60], True), ("damaged", 1, [60], True)]),
    "warden_head": ((44, 44), (22, 41), [("idle", 4, [14, 50, 40, 50], True), ("flare", 4, [6, 5, 6, 24], False),
                                        ("roar", 6, [12, 6, 8, 5, 5, 16], False)]),
    "warden_fist": ((40, 56), (20, 52), [("clenched", 4, [20] * 4, True), ("open", 4, [20] * 4, True),
                                        ("open_lit", 4, [8, 6, 6, 8], True), ("slam", 5, [8, 26, 3, 12, 14], False)]),
    "warden_heart": ((24, 32), (12, 15), [("whole", 4, [18, 10, 14, 10], True), ("cracked", 4, [18, 10, 14, 10], True),
                                         ("badly_cracked", 4, [6, 10, 4, 14], True), ("shattered", 4, [4] * 4, True)]),
    "warden_crystal": ((20, 24), (9, 21), [("intact", 4, [18, 10, 14, 10], True), ("cracked", 4, [6, 12, 4, 16], True),
                                          ("broken", 1, [60], True)]),
}
EMISSIVE = {"warden_body": [], "warden_head": ["#e2d8e9"], "warden_fist": ["#e2d8e9"],
            "warden_heart": ["#e2d8e9"], "warden_crystal": ["#e2d8e9"]}
SLAM = [(9, 0), (23, -3), (3, -5), (0, 6), (0, 0)]  # (baked lift, crystal back) per slam frame
ROAR_PEAK = 3
FIST_SEAT = -22  # the fist crystal is drawn at fist anchor y - 22 + back


class Cell:
    """One sheet cell with an anchor, measured like the engine measures it."""

    def __init__(self, im, anchor):
        """Wrap a cropped RGBA cell."""
        self.im, self.anchor = im, anchor
        self.w, self.h = im.size
        self.px = im.load()

    def a(self, x, y):
        """Return whether a pixel is opaque (outside reads transparent)."""
        return 0 <= x < self.w and 0 <= y < self.h and self.px[x, y][3] > 0

    def bottom(self):
        """Lowest opaque row relative to the anchor (art.rs bottom)."""
        return next((y for y in range(self.h - 1, -1, -1) if any(self.a(x, y) for x in range(self.w))), 0) - self.anchor[1]

    def top(self):
        """Highest opaque row relative to the anchor (art.rs top)."""
        return next((y for y in range(self.h) if any(self.a(x, y) for x in range(self.w))), 0) - self.anchor[1]

    def solid_top(self):
        """Top of the 5 px solid column over the anchor (art.rs solid_top)."""
        ax = self.anchor[0]
        return next((y for y in range(self.h) if all(self.a(x, y) for x in range(ax - 2, ax + 3))), 0) - self.anchor[1]

    def mass(self):
        """Count opaque source pixels (what the death burst turns into particles)."""
        return sum(1 for y in range(self.h) for x in range(self.w) if self.a(x, y))


def load(sprites, name):
    """Load a sheet and its manifest; return (manifest, {anim: [Cell]}, image)."""
    man = json.loads((sprites / f"{name}.json").read_text())
    im = Image.open(sprites / f"{name}.png").convert("RGBA")
    cw, ch = man["cell"]
    anchor = tuple(man["anchor"])
    cells = {}
    for a, spec in man["anims"].items():
        r = spec["row"]
        cells[a] = [Cell(im.crop((i * cw, r * ch, i * cw + cw, r * ch + ch)), anchor) for i in range(spec["frames"])]
    return man, cells, im


def palette():
    """Allowed colors: every Summit (regions.warden) color in palettes_v2.json."""
    reg = json.loads(PALETTE.read_text())["regions"]["warden"]
    cols = {reg["outline"], reg["shadow_tint"], reg["light_tint"]}
    for ramp in reg["ramps"].values():
        cols |= set(ramp)
    return {c.lower() for c in cols}


def hexof(p):
    """Format an RGB tuple as #rrggbb."""
    return "#%02x%02x%02x" % p[:3]


def main():
    """Run every check and print a report."""
    argv = sys.argv[1:]
    out_json = None
    if "--json" in argv:
        k = argv.index("--json")
        out_json = argv[k + 1]
        argv = argv[:k] + argv[k + 2:]
    sprites = Path(argv[0]) if argv else ROOT / "assets" / "sprites"
    allowed = palette()
    fails, rep = [], {"sheets": {}}
    data = {}
    for name, (cell, anchor, anims) in CONTRACT.items():
        man, cells, im = load(sprites, name)
        data[name] = cells
        r = {}
        if tuple(man["cell"]) != cell or tuple(man["anchor"]) != anchor:
            fails.append(f"{name}: cell/anchor {man['cell']} {man['anchor']}")
        for j, (a, n, dur, loop) in enumerate(anims):
            s = man["anims"].get(a)
            if not s or (s["row"], s["frames"], s["durations"], s["loop"]) != (j, n, dur, loop):
                fails.append(f"{name}.{a}: row/frames/durations/loop changed: {s}")
        if list(man["anims"]) != [a for a, *_ in anims]:
            fails.append(f"{name}: anim set {list(man['anims'])}")
        if [e.lower() for e in man["emissive"]] != EMISSIVE[name]:
            fails.append(f"{name}: emissive {man['emissive']}")
        if im.size != (cell[0] * max(n for _, n, _, _ in anims), cell[1] * len(anims)):
            fails.append(f"{name}: sheet size {im.size}")
        alphas = {p[3] for p in im.get_flattened_data()}
        cols = {hexof(p) for p in im.get_flattened_data() if p[3]}
        off = sorted(cols - allowed)
        if not alphas <= {0, 255}:
            fails.append(f"{name}: non-binary alpha {sorted(alphas)}")
        if off:
            fails.append(f"{name}: off-palette {off}")
        # edge clipping: no opaque pixel on a cell border
        clipped = []
        for a, cs in cells.items():
            for i, c in enumerate(cs):
                if any(c.a(x, 0) or c.a(x, c.h - 1) for x in range(c.w)) or any(c.a(0, y) or c.a(c.w - 1, y) for y in range(c.h)):
                    clipped.append(f"{a}{i}")
        if clipped:
            fails.append(f"{name}: art touches the cell border in {clipped}")
        r["colors"] = len(cols)
        r["mass"] = {a: [c.mass() for c in cs] for a, cs in cells.items()}
        rep["sheets"][name] = r
    # fist slam calibration
    slam = data["warden_fist"]["slam"]
    rest = slam[4]
    got = []
    for i, c in enumerate(slam):
        lift = max(rest.bottom() - c.bottom(), 0)
        back = c.solid_top() + lift - rest.solid_top()
        got.append((lift, back))
    rep["slam_lift_back"] = got
    if got != SLAM:
        fails.append(f"fist slam lift/back {got} != {SLAM}")
    rep["fist_rest_solid_top"] = rest.solid_top()
    rep["fist_loop_solid_tops"] = {a: [c.solid_top() for c in data["warden_fist"][a]] for a in ("clenched", "open", "open_lit")}
    rep["fist_loop_bottoms"] = {a: [c.bottom() for c in data["warden_fist"][a]] for a in ("clenched", "open", "open_lit")}
    # head roar
    idle = data["warden_head"]["idle"][0]
    peak = max(max(idle.bottom() - c.bottom(), 0) for c in data["warden_head"]["roar"])
    rep["roar_peak"] = peak
    rep["roar_lifts"] = [max(idle.bottom() - c.bottom(), 0) for c in data["warden_head"]["roar"]]
    if peak != ROAR_PEAK:
        fails.append(f"roar peak {peak} != {ROAR_PEAK}")
    # body attachment: what is under the heart and crystal seats in every body state
    att = {}
    for a, cs in data["warden_body"].items():
        c = cs[0]
        att[a] = {"heart_socket(60,49)": c.a(60, 49), "seat_left(24,9..12)": any(c.a(24, y) for y in range(9, 13)),
                  "seat_right(96,9..12)": any(c.a(96, y) for y in range(9, 13)), "neck(60,34)": c.a(60, 34)}
    rep["body_attachments"] = att
    # death-burst source mass (body idle + head roar/idle + heart idle + 2 crystals; fists crumble separately)
    m = data
    rep["death_source_mass"] = {
        "body_idle": m["warden_body"]["idle"][0].mass(),
        "head_idle0": m["warden_head"]["idle"][0].mass(),
        "head_roar5": m["warden_head"]["roar"][5].mass(),
        "heart_whole0": m["warden_heart"]["whole"][0].mass(),
        "crystal_intact0_x2": 2 * m["warden_crystal"]["intact"][0].mass(),
        "fist_clenched0_x2": 2 * m["warden_fist"]["clenched"][0].mass(),
    }
    rep["death_source_mass"]["total"] = sum(rep["death_source_mass"].values())
    rep["fails"] = fails
    print(json.dumps(rep, indent=1))
    if out_json:
        Path(out_json).write_text(json.dumps(rep, indent=1) + "\n")
    print("FAIL" if fails else "OK", len(fails))
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
