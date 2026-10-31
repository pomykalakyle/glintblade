"""Tiny pixel-art toolkit for the Bloomwild sheets.

Images are numpy uint8 arrays shaped (h, w, 4). Alpha is always 0 or 255.
Colors are named by ramp + index (g2 = grass[2], s4 = stone[4] ...) from palettes.json.
"""

import json
import math
import pathlib

import numpy as np
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPRITES = ROOT / "assets" / "sprites"
PREVIEWS = ROOT / "assets" / "previews"
PALJ = json.loads((ROOT / "assets" / "concept" / "palettes_v2.json").read_text())
BWR = PALJ["regions"]["bloomwild"]["ramps"]

# ---------------------------------------------------------------- palette
HEX = {}
_short = {"grass": "g", "grass_teal": "t", "stone": "s", "earth": "e", "blossom": "b",
          "water": "w", "cloud": "c", "gold": "au"}
for ramp, k in _short.items():
    for i, h in enumerate(BWR[ramp]):
        HEX[f"{k}{i}"] = h
# Keep the legacy authoring names compatible with the shorter v2 ramps.
for prefix, count in {"g": 6, "t": 5, "b": 6, "w": 6, "c": 6, "au": 5}.items():
    last = next(HEX[f"{prefix}{i}"] for i in range(count - 1, -1, -1) if f"{prefix}{i}" in HEX)
    for i in range(count):
        HEX.setdefault(f"{prefix}{i}", last)
HEX["o"] = PALJ["regions"]["bloomwild"]["outline"]  # #1a1030
for i, h in enumerate(PALJ["fx"]["prism"]):
    HEX[f"p{i}"] = h
for i, h in enumerate(PALJ["regions"]["warden"]["ramps"]["warden_stone"]):
    HEX[f"ws{i}"] = h
for i, h in enumerate(PALJ["regions"]["warden"]["ramps"]["gold"]):
    HEX[f"wg{i}"] = h
for i, h in enumerate(PALJ["hero"]["cloak"]):
    HEX[f"hc{i}"] = h
for i, h in enumerate(PALJ["hero"]["scarf"]):
    HEX[f"hs{i}"] = h
HEX["hs3"] = HEX["hs2"]
HEX["p4"] = HEX["p3"]
HEX["p5"] = HEX["p3"]
HEX["hv"] = PALJ["hero"]["hood_void"][0]
HEX["ho"] = PALJ["hero"]["outline"][0]
HEX["he"] = PALJ["hero"]["eyes"][0]


def rgba(c):
    """Name ('g2'), '#rrggbb' or tuple -> (r, g, b, 255). None -> transparent."""
    if c is None:
        return (0, 0, 0, 0)
    if isinstance(c, tuple):
        return c if len(c) == 4 else (*c, 255)
    h = HEX.get(c, c)
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def hexof(c):
    """Hexof on the native pixel grid."""
    r, g, b, _ = rgba(c)
    return f"#{r:02x}{g:02x}{b:02x}"


# ---------------------------------------------------------------- canvas
def new(w, h, fill=None):
    """New on the native pixel grid."""
    a = np.zeros((h, w, 4), np.uint8)
    if fill is not None:
        a[:, :] = rgba(fill)
    return a


def put(img, x, y, c):
    """Put on the native pixel grid."""
    h, w = img.shape[:2]
    if 0 <= x < w and 0 <= y < h:
        img[y, x] = rgba(c)


def get(img, x, y):
    """Get on the native pixel grid."""
    h, w = img.shape[:2]
    if 0 <= x < w and 0 <= y < h:
        return tuple(int(v) for v in img[y, x])
    return (0, 0, 0, 0)


def is_col(img, x, y, c):
    """Is col on the native pixel grid."""
    return get(img, x, y) == rgba(c)


def opaque(img, x, y):
    """Opaque on the native pixel grid."""
    return get(img, x, y)[3] > 0


def rect(img, x, y, w, h, c):
    """Rect on the native pixel grid."""
    for yy in range(y, y + h):
        for xx in range(x, x + w):
            put(img, xx, yy, c)


def hline(img, x0, x1, y, c):
    """Hline on the native pixel grid."""
    for x in range(min(x0, x1), max(x0, x1) + 1):
        put(img, x, y, c)


def vline(img, x, y0, y1, c):
    """Vline on the native pixel grid."""
    for y in range(min(y0, y1), max(y0, y1) + 1):
        put(img, x, y, c)


def grid(rows, legend, img=None, ox=0, oy=0):
    """Draw a char grid. legend maps char -> color name; '.' and ' ' are skipped."""
    if img is None:
        w = max(len(r) for r in rows)
        img = new(w, len(rows))
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch in ". ":
                continue
            put(img, ox + x, oy + y, legend[ch])
    return img


def paste(dst, src, x, y, flip=False):
    """Paste on the native pixel grid."""
    sh, sw = src.shape[:2]
    for yy in range(sh):
        for xx in range(sw):
            sx = sw - 1 - xx if flip else xx
            if src[yy, sx, 3]:
                put(dst, x + xx, y + yy, tuple(int(v) for v in src[yy, sx]))


def recolor(img, mapping):
    """Recolor on the native pixel grid."""
    out = img.copy()
    for a, b in mapping.items():
        ra, rb = np.array(rgba(a), np.uint8), np.array(rgba(b), np.uint8)
        m = np.all(img == ra, axis=-1)
        out[m] = rb
    return out


def shift(img, dx, dy):
    """Shift on the native pixel grid."""
    h, w = img.shape[:2]
    out = new(w, h)
    for y in range(h):
        for x in range(w):
            if img[y, x, 3]:
                put(out, x + dx, y + dy, tuple(int(v) for v in img[y, x]))
    return out


def mask_of(img):
    """Mask of on the native pixel grid."""
    return img[:, :, 3] > 0


def outline(img, c, diag=False, only_where=None):
    """Add a 1px outline in transparent pixels next to opaque ones."""
    h, w = img.shape[:2]
    m = mask_of(img)
    out = img.copy()
    nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
    if diag:
        nb += [(1, 1), (-1, 1), (1, -1), (-1, -1)]
    for y in range(h):
        for x in range(w):
            if m[y, x]:
                continue
            for dx, dy in nb:
                xx, yy = x + dx, y + dy
                if 0 <= xx < w and 0 <= yy < h and m[yy, xx]:
                    if only_where is None or only_where(x, y, dx, dy):
                        out[y, x] = rgba(c)
                    break
    return out


def clean_orphans(img, protect=(), passes=2):
    """Replace single pixels with no same-colored 4-neighbor by their most common neighbor color."""
    prot = {rgba(p) for p in protect}
    h, w = img.shape[:2]
    for _ in range(passes):
        src = img.copy()
        for y in range(h):
            for x in range(w):
                c = tuple(int(v) for v in src[y, x])
                if c[3] == 0 or c in prot:
                    continue
                ns = []
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    xx, yy = x + dx, y + dy
                    if 0 <= xx < w and 0 <= yy < h:
                        ns.append(tuple(int(v) for v in src[yy, xx]))
                if c in ns:
                    continue
                opq = [n for n in ns if n[3]]
                if len(opq) < 3:
                    continue
                best = max(set(opq), key=opq.count)
                img[y, x] = best
    return img


# ---------------------------------------------------------------- sheets
def assemble(rows, cw, ch):
    """rows: list of lists of cell images -> sheet image."""
    ncols = max(len(r) for r in rows)
    sheet = new(cw * ncols, ch * len(rows))
    for r, row in enumerate(rows):
        for c, cell in enumerate(row):
            assert cell.shape[0] == ch and cell.shape[1] == cw, (r, c, cell.shape)
            sheet[r * ch:(r + 1) * ch, c * cw:(c + 1) * cw] = cell
    return sheet


def save(img, path):
    """Save on the native pixel grid."""
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    a = img.copy()
    a[a[:, :, 3] == 0] = 0
    Image.fromarray(a, "RGBA").save(path)


def save_sheet(name, rows, cw, ch, manifest):
    """Save sheet on the native pixel grid."""
    sheet = assemble(rows, cw, ch)
    save(sheet, SPRITES / f"{name}.png")
    (SPRITES / f"{name}.json").write_text(json.dumps(manifest, indent=1) + "\n")
    return sheet


def colors_used(img):
    """Colors used on the native pixel grid."""
    m = img[:, :, 3] > 0
    cols = {tuple(int(v) for v in p[:3]) for p in img[m]}
    return sorted("#%02x%02x%02x" % c for c in cols)


# ---------------------------------------------------------------- previews
def over_bg(img, bg):
    """Over bg on the native pixel grid."""
    h, w = img.shape[:2]
    out = new(w, h, bg)
    m = img[:, :, 3] > 0
    out[m] = img[m]
    return out


def scale(img, k):
    """Scale on the native pixel grid."""
    return np.repeat(np.repeat(img, k, axis=0), k, axis=1)


def preview(img, path, k=8, bg="g2", gridcell=None, gridcol=(0, 0, 0, 60)):
    """Preview on the native pixel grid."""
    out = over_bg(img, bg)
    big = scale(out, k)
    if gridcell:
        cw, ch = gridcell
        H, W = big.shape[:2]
        for x in range(0, W, cw * k):
            big[:, x] = (big[:, x].astype(int) * 3 // 4).astype(np.uint8)
            big[:, x, 3] = 255
        for y in range(0, H, ch * k):
            big[y, :] = (big[y, :].astype(int) * 3 // 4).astype(np.uint8)
            big[y, :, 3] = 255
    save(big, path)


def gif(frames, path, k=6, bg="g2", durations=None):
    """frames: list of images; durations in 60 Hz ticks."""
    ims = [Image.fromarray(scale(over_bg(f, bg), k)[:, :, :3], "RGB") for f in frames]
    durs = [int(round(d * 1000 / 60)) for d in (durations or [10] * len(frames))]
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    ims[0].save(path, save_all=True, append_images=ims[1:], duration=durs, loop=0, disposal=1)


# ---------------------------------------------------------------- shapes
def ellipse_mask(w, h, cx, cy, rx, ry):
    """Ellipse mask on the native pixel grid."""
    yy, xx = np.mgrid[0:h, 0:w]
    return ((xx + 0.5 - cx) / rx) ** 2 + ((yy + 0.5 - cy) / ry) ** 2 <= 1.0


def hash01(*a):
    """Deterministic hash -> [0, 1)."""
    h = 2166136261
    for v in a:
        h ^= (int(v) * 2654435761) & 0xFFFFFFFF
        h = (h * 16777619) & 0xFFFFFFFF
        h ^= h >> 13
    return (h & 0xFFFFFF) / float(0x1000000)


def stand_in_figure():
    """Rough 18 px Glint stand-in for scale previews (not game art)."""
    rows = [
        "....oooo....",
        "...oDDDDo...",
        "..oDCCCCDo..",
        "..oCvvvvCo..",
        "..oCvEvEvCo.",
        "..oCvvvvCo..",
        ".oSSSSSSSSo.",
        ".oSsSSSSDCo.",
        "oCCCSsCCCCDo",
        "oCLCCCCCCDDo",
        "oCLCCCCCCDDo",
        "oCLCCCCCCDDo",
        ".oCCCCCCCDo.",
        ".oCCCCCCCDo.",
        ".oDCCCCCDDo.",
        "..oDDoDDDo..",
        "..oGGooGGo..",
        "...oo..oo...",
    ]
    leg = {"o": "ho", "D": "hc1", "C": "hc2", "L": "hc3", "v": "hv", "E": "he",
           "S": "hs2", "s": "hs3", "G": "hc0"}
    return grid(rows, leg)


def fix_staircases(img, max_iter=40):
    """Preserve authored geometry for visual review instead of changing pixels to optimize lint."""
    return img
