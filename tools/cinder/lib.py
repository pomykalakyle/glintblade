"""Shared helpers for the Cinder Steppes sprite scripts: palette, char grids, sheet export, previews."""

import json
import pathlib
import re

import numpy as np
from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPRITES = ROOT / "assets" / "sprites"
PREVIEWS = ROOT / "assets" / "previews"
SAMPLES = ROOT / "out" / "samples"
WORK = ROOT / "tools" / "cinder" / "work"
for d in (SPRITES, PREVIEWS, WORK):
    d.mkdir(parents=True, exist_ok=True)

# ---------------------------------------------------------------------------
# Palette V2 with the approved Cinder soot-black graft.
# ---------------------------------------------------------------------------
RAMPS = {
    "ash": ["#16151d", "#24232d", "#363340", "#514854", "#6f616b", "#8b7c81"],
    "grass": ["#292c3c", "#393746", "#514854", "#6f616b", "#79707a"],
    "obs": ["#101017", "#1a1923", "#292c3c", "#3f414f", "#5b5966", "#7c7780"],
    "bas": ["#1a1923", "#292c3c", "#393746", "#56505d", "#79707a"],
    "lava": ["#24232d", "#6f454b", "#a36059", "#d48670", "#efb68a", "#f1d4a6"],
    "cop": ["#292c3c", "#393746", "#63515a", "#93766f", "#b69b85"],
    "bone": ["#292c3c", "#3f414f", "#5b5966", "#7c7780", "#8b7c81", "#a19b9b"],
    "steam": ["#514854", "#6f616b", "#8b7c81"],
}
OUTLINE = "#101017"
EMISSIVE = ["#d48670", "#efb68a", "#f1d4a6"]
STYLE_NOTES = ("Style V2 Cinder graft: palettes_v2.json ash/obsidian/basalt and salmon lava. "
               "Custom soot shades #101017 #16151d #1a1923 #24232d preserve C mock dark values. "
               "Cold stone and burned wood share the mineral ramp; copper is weathered, not gold. "
               "All pixels are authored at native scale; lava/fire alone carry emissive heat.")

# One char per color. Ramps are lettered dark -> light.
LEGEND = {
    ".": None,
    " ": None,
    "K": OUTLINE,
    # ash a..f
    "a": RAMPS["ash"][0], "b": RAMPS["ash"][1], "c": RAMPS["ash"][2],
    "d": RAMPS["ash"][3], "e": RAMPS["ash"][4], "f": RAMPS["ash"][5],
    # basalt 1..5
    "1": RAMPS["bas"][0], "2": RAMPS["bas"][1], "3": RAMPS["bas"][2], "4": RAMPS["bas"][3], "5": RAMPS["bas"][4],
    # obsidian o p q r s t
    "o": RAMPS["obs"][0], "p": RAMPS["obs"][1], "q": RAMPS["obs"][2],
    "r": RAMPS["obs"][3], "s": RAMPS["obs"][4], "t": RAMPS["obs"][5],
    # lava u v w x y z (x y z glow)
    "u": RAMPS["lava"][0], "v": RAMPS["lava"][1], "w": RAMPS["lava"][2],
    "x": RAMPS["lava"][3], "y": RAMPS["lava"][4], "z": RAMPS["lava"][5],
    # copper A B C D E
    "A": RAMPS["cop"][0], "B": RAMPS["cop"][1], "C": RAMPS["cop"][2], "D": RAMPS["cop"][3], "E": RAMPS["cop"][4],
    # scorched grass G H I J U
    "G": RAMPS["grass"][0], "H": RAMPS["grass"][1], "I": RAMPS["grass"][2], "J": RAMPS["grass"][3], "U": RAMPS["grass"][4],
    # bone 6..9, V W  (6 darkest)
    "6": RAMPS["bone"][0], "7": RAMPS["bone"][1], "8": RAMPS["bone"][2], "9": RAMPS["bone"][3],
    "V": RAMPS["bone"][4], "W": RAMPS["bone"][5],
    # steam
    "S": RAMPS["steam"][1], "T": RAMPS["steam"][2],
}

ALL_COLORS = {v.lower() for v in LEGEND.values() if v}


def hex2rgb(h):
    """Convert a hex swatch to an RGB tuple."""
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def rgb2hex(c):
    """Return the hex swatch of an RGB pixel."""
    return "#%02x%02x%02x" % tuple(int(v) for v in c[:3])


def grid(rows, legend=None, w=None, h=None):
    """Char rows -> RGBA uint8 array. Rows may be a multiline string."""
    if isinstance(rows, str):
        rows = [r for r in rows.strip("\n").split("\n")]
        # strip common indentation
        ind = min((len(r) - len(r.lstrip(" ")) for r in rows if r.strip()), default=0)
        rows = [r[ind:] for r in rows]
    lg = dict(LEGEND)
    if legend:
        lg.update(legend)
    H = h or len(rows)
    W = w or max(len(r) for r in rows)
    out = np.zeros((H, W, 4), np.uint8)
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch not in lg:
                raise ValueError(f"unknown char {ch!r} at {x},{y}")
            c = lg[ch]
            if c:
                out[y, x, :3] = hex2rgb(c)
                out[y, x, 3] = 255
    return out


def to_chars(img, legend=None):
    """RGBA array -> char rows (reverse of grid), for hand-editing generated art."""
    lg = dict(LEGEND)
    if legend:
        lg.update(legend)
    rev = {}
    for k, v in lg.items():
        if v and v.lower() not in rev:
            rev[v.lower()] = k
    rows = []
    for y in range(img.shape[0]):
        r = ""
        for x in range(img.shape[1]):
            if img[y, x, 3] == 0:
                r += "."
            else:
                r += rev.get(rgb2hex(img[y, x]), "?")
        rows.append(r)
    return rows


def paste(dst, src, x, y):
    """Alpha-paste src onto dst at (x, y) (hard alpha), clipped."""
    H, W = dst.shape[:2]
    h, w = src.shape[:2]
    for yy in range(h):
        ty = y + yy
        if ty < 0 or ty >= H:
            continue
        for xx in range(w):
            tx = x + xx
            if tx < 0 or tx >= W:
                continue
            if src[yy, xx, 3]:
                dst[ty, tx] = src[yy, xx]
    return dst


def flip(img):
    """Mirror the native pixel grid without resampling."""
    return img[:, ::-1].copy()


class SheetBuilder:
    """Rows of animation frames with a fixed cell size."""

    def __init__(self, name, cell, anchor):
        """Initialize the native pixel drawing state."""
        self.name = name
        self.cw, self.ch = cell
        self.anchor = list(anchor)
        self.rows = []  # list of (anim_name, [frames], meta)
        self.meta = {}

    def add(self, anim, frames, **meta):
        """Append an animation after checking every cell dimension."""
        if not isinstance(frames, list):
            frames = [frames]
        fixed = []
        for f in frames:
            if f.shape[0] != self.ch or f.shape[1] != self.cw:
                raise ValueError(f"{self.name}/{anim}: frame {f.shape[1]}x{f.shape[0]} != cell {self.cw}x{self.ch}")
            fixed.append(f)
        self.rows.append((anim, fixed, meta))

    def build(self):
        """Build the sheet with its original cells, anchors and animation contracts."""
        cols = max(len(f) for _, f, _ in self.rows)
        img = np.zeros((self.ch * len(self.rows), self.cw * cols, 4), np.uint8)
        anims = {}
        for r, (anim, frames, meta) in enumerate(self.rows):
            for c, f in enumerate(frames):
                img[r * self.ch:(r + 1) * self.ch, c * self.cw:(c + 1) * self.cw] = f
            a = {"row": r, "frames": len(frames)}
            a.update(meta)
            if "fps" not in a and "durations" not in a:
                a["fps"] = 1
            anims[anim] = a
        return img, anims

    def save(self, emissive=None, extra=None):
        """Export the sheet and manifest with documented material ramps."""
        img, anims = self.build()
        check_image(img, self.name)
        Image.fromarray(img, "RGBA").save(SPRITES / f"{self.name}.png")
        j = {"cell": [self.cw, self.ch], "anchor": self.anchor}
        if emissive:
            j["emissive"] = emissive
        j["anims"] = anims
        if extra:
            j.update(extra)
        j["notes"] = STYLE_NOTES + " " + j.get("notes", "")
        (SPRITES / f"{self.name}.json").write_text(json_dump(j))
        return img, anims


def json_dump(j):
    # Compact inner lists/dicts on one line for readability.
    """Draw the json dump on the native Cinder pixel grid."""
    s = json.dumps(j, indent=1)
    s = re.sub(r"\[\s+([^\[\]{}]*?)\s+\]", lambda m: "[" + " ".join(m.group(1).split()) + "]", s)
    return s + "\n"


def check_image(img, name):
    """Draw the check image on the native Cinder pixel grid."""
    a = img[..., 3]
    bad = ((a != 0) & (a != 255)).sum()
    if bad:
        raise ValueError(f"{name}: {bad} semi-transparent pixels")
    cols = {rgb2hex(p) for p in img[a == 255]}
    off = sorted(c for c in cols if c not in ALL_COLORS)
    if off:
        print(f"  note {name}: {len(off)} colors outside the legend: {off[:8]}")
    return cols


def colors_in(img):
    """Return the set of opaque RGB swatches."""
    a = img[..., 3]
    return {rgb2hex(p) for p in img[a == 255]}


# ---------------------------------------------------------------------------
# Previews
# ---------------------------------------------------------------------------
GROUND_BG = "#363340"  # ash[2], the region's base ground


def scale(img, s):
    """Return the scale representation for the Cinder generator."""
    return np.repeat(np.repeat(img, s, 0), s, 1)


def over(img, bg):
    """Composite RGBA over a solid color -> RGB."""
    out = np.zeros(img.shape[:2] + (3,), np.uint8)
    out[:] = hex2rgb(bg) if isinstance(bg, str) else bg
    m = img[..., 3] > 0
    out[m] = img[..., :3][m]
    return out


def save_preview(img, name, s=6, bg=GROUND_BG, grid_cell=None, pad=0):
    """Draw the save preview on the native Cinder pixel grid."""
    rgb = over(img, bg)
    big = scale(rgb, s)
    if grid_cell:
        # faint cell lines so the sheet layout is visible
        cw, ch = grid_cell
        big = big.copy()
        for x in range(0, big.shape[1], cw * s):
            big[:, x] = (big[:, x] * 0.6).astype(np.uint8)
        for y in range(0, big.shape[0], ch * s):
            big[y, :] = (big[y, :] * 0.6).astype(np.uint8)
    Image.fromarray(big, "RGB").save(PREVIEWS / f"{name}.png")
    return big


def save_gif(frames, name, s=6, bg=GROUND_BG, durations_ticks=None, fps=8):
    """Draw the save gif on the native Cinder pixel grid."""
    ims = [Image.fromarray(scale(over(f, bg), s), "RGB") for f in frames]
    if durations_ticks:
        dur = [int(round(d * 1000 / 60)) for d in durations_ticks]
    else:
        dur = [int(round(1000 / fps))] * len(frames)
    ims[0].save(PREVIEWS / f"{name}.gif", save_all=True, append_images=ims[1:], duration=dur, loop=0, disposal=1)


def figure18():
    """A rough 18 px tall cloaked figure for scale (stand-in until hero.png exists)."""
    return grid("""
        ....KKKK....
        ...KqrrqK...
        ..KqrsrrqK..
        ..KrpppprK..
        ..Kp?pp?pK..
        ..KqppppqK..
        .KKqrrrqqKK.
        KqrrsrrrqqqK
        KqrsrrrrqqqK
        KqrrrrrrqqpK
        .KqrrrrqqpK.
        .KqrrrrqqpK.
        .KqqrrqqqpK.
        .KqqqqqqppK.
        ..KqqqqqpK..
        ..KpK..KpK..
        ..KKK..KKK..
        ............
    """, legend={"?": "#8ff8ff"})


def label_strip(big, text, s=1):
    """Draw the label strip on the native Cinder pixel grid."""
    im = Image.fromarray(big)
    d = ImageDraw.Draw(im)
    d.text((4, 2), text, fill=(240, 230, 230))
    return np.array(im)
