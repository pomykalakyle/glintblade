#!/usr/bin/env python3
"""Pixel lint for GLINTBLADE sprite sheets and screenshots (written 2026-10-30).

Flags measurable patterns listed in ART_RESEARCH.md: semi-transparent pixels,
possible upscaled sheets and doubled staircases, excessive or near-duplicate
colors, colors off the project palette, neutral colors, lone specks, and
manifest/animation mistakes. Lighting and pillow-shading heuristics are
unscored context, since pixel statistics cannot infer the intended geometry.
For screenshots (384x216 or an integer multiple) it checks the pixel grid.
Exploratory value/color measurements from reference games are shown with -v
as composition prompts, never scored approval gates.

A clean report does not mean the art is good. It only means these mechanical
tells are absent; taste is still the critic's job (see ART_RESEARCH.md).

Usage, from the project root:
  uv run --no-project --with pillow --with numpy python3 tools/pixel_lint.py assets/sprites
  uv run --no-project --with pillow --with numpy python3 tools/pixel_lint.py assets/sprites/gate.png -v
  uv run --no-project --with pillow --with numpy python3 tools/pixel_lint.py out/shot1/f00020.png
Options: --palette PATH (default assets/concept/palettes.json), -v (info notes
and more examples), --json (machine-readable output), --frame (treat every
input as a screenshot). Exit code 1 if any file has an ERROR.

Coordinates: "idle_right#2 (5,11) @(69,11)" is pixel x=5, y=11 inside frame 2
of anim idle_right, which is pixel (69,11) of the whole sheet.
Severity: ERROR breaks the format or the pixel grid (must fix). WARN is a
likely craft problem (fix, or say in the manifest notes why it is deliberate).
info is context for the critic and costs no points.
"""

import argparse
import json
import math
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_PALETTE = ROOT / "assets" / "concept" / "palettes.json"
NATIVE_W, NATIVE_H = 384, 216

# Color distances are CIEDE2000: about 1 is just noticeable side by side, 2 to 3
# is hard to see in a sprite at 1x. Tuned on the first sheets in assets/sprites
# and on tools/pixel_lint_tests/.
NEAR_DUP_DE = 2.5           # review cutoff, not a universal perceptual boundary
TOUCH_DE = 3.5              # touching colors this close blur into each other at 1x
PALETTE_NEAR_MISS_DE = 3.0  # this close to a palette color but not equal: probably a typo
MATERIAL_DE = 8.0           # off-palette colors this close to a ramp count as that ramp's material

# Colors per cell and per sheet above which a warning fires. ART_DIRECTION.md asks
# for about 8 to 14 colors per character and 4 to 10 per prop; sheets may add a
# few FX or emissive colors on top, and tile sheets hold several ramps.
COLOR_LIMITS = {
    "character": (16, 22),
    "prop": (14, 22),
    "ui": (16, 24),
    "tiles": (10, 32),
    "image": (16, 24),
}
BIG_CELL_EXTRA = 4          # extra colors allowed per cell for cells of 48x48 or more
PAIR_SCAN_LIMIT = 512       # bound quadratic work on noisy or filtered input

CHAR_ANIM_RE = re.compile(r"^(idle|move|run|walk|attack|hurt|windup|dash|die|death|slash|spin|charge|hop)")
TIMED_ANIM_RE = re.compile(r"(attack|slash|windup|swing|lunge|slam|spin)")
UI_NAME_RE = re.compile(r"^(hud|ui|logo|font|icon|title)")
OFFS8 = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)]
OFFS4 = [(-1, 0), (1, 0), (0, -1), (0, 1)]
SEVERITY_ORDER = {"ERROR": 0, "WARN": 1, "info": 2}


# ---------------------------------------------------------------- color math

def srgb_to_lab(rgb):
    """rgb: (..., 3) values 0-255 -> (..., 3) CIELAB (D65)."""
    c = np.asarray(rgb, dtype=np.float64) / 255.0
    c = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    m = np.array([[0.4124564, 0.3575761, 0.1804375],
                  [0.2126729, 0.7151522, 0.0721750],
                  [0.0193339, 0.1191920, 0.9503041]])
    xyz = c @ m.T / np.array([0.95047, 1.0, 1.08883])
    f = np.where(xyz > 216 / 24389, np.cbrt(xyz), (24389 / 27 * xyz + 16) / 116)
    return np.stack([116 * f[..., 1] - 16, 500 * (f[..., 0] - f[..., 1]), 200 * (f[..., 1] - f[..., 2])], axis=-1)


def ciede2000(lab1, lab2):
    """Vectorized CIEDE2000 between broadcastable (..., 3) Lab arrays."""
    l1, a1, b1 = lab1[..., 0], lab1[..., 1], lab1[..., 2]
    l2, a2, b2 = lab2[..., 0], lab2[..., 1], lab2[..., 2]
    c1, c2 = np.hypot(a1, b1), np.hypot(a2, b2)
    cbar7 = ((c1 + c2) / 2) ** 7
    g = 0.5 * (1 - np.sqrt(cbar7 / (cbar7 + 25.0 ** 7)))
    a1p, a2p = (1 + g) * a1, (1 + g) * a2
    c1p, c2p = np.hypot(a1p, b1), np.hypot(a2p, b2)
    h1p = np.degrees(np.arctan2(b1, a1p)) % 360
    h2p = np.degrees(np.arctan2(b2, a2p)) % 360
    zero = (c1p * c2p) == 0
    dlp, dcp = l2 - l1, c2p - c1p
    dhp = h2p - h1p
    dhp = np.where(dhp > 180, dhp - 360, dhp)
    dhp = np.where(dhp < -180, dhp + 360, dhp)
    dhp = np.where(zero, 0, dhp)
    dHp = 2 * np.sqrt(c1p * c2p) * np.sin(np.radians(dhp / 2))
    lbar, cbarp = (l1 + l2) / 2, (c1p + c2p) / 2
    hsum = h1p + h2p
    hbar = np.where(np.abs(h1p - h2p) <= 180, hsum / 2, np.where(hsum < 360, (hsum + 360) / 2, (hsum - 360) / 2))
    hbar = np.where(zero, hsum, hbar)
    t = (1 - 0.17 * np.cos(np.radians(hbar - 30)) + 0.24 * np.cos(np.radians(2 * hbar))
         + 0.32 * np.cos(np.radians(3 * hbar + 6)) - 0.20 * np.cos(np.radians(4 * hbar - 63)))
    dtheta = 30 * np.exp(-(((hbar - 275) / 25) ** 2))
    rc = 2 * np.sqrt(cbarp ** 7 / (cbarp ** 7 + 25.0 ** 7))
    sl = 1 + 0.015 * (lbar - 50) ** 2 / np.sqrt(20 + (lbar - 50) ** 2)
    sc = 1 + 0.045 * cbarp
    sh = 1 + 0.015 * cbarp * t
    rt = -np.sin(np.radians(2 * dtheta)) * rc
    return np.sqrt((dlp / sl) ** 2 + (dcp / sc) ** 2 + (dHp / sh) ** 2 + rt * (dcp / sc) * (dHp / sh))


def hexc(rgb):
    return "#%02x%02x%02x" % tuple(int(v) for v in rgb[:3])


def parse_hex(s):
    s = s.strip().lstrip("#")
    return tuple(int(s[i:i + 2], 16) for i in (0, 2, 4))


def nearest_palette(colors_lab, palette_lab):
    """Find exact nearest palette colors in bounded batches of CIEDE2000 work."""
    indices = np.empty(len(colors_lab), dtype=int)
    distances = np.empty(len(colors_lab), dtype=float)
    for start in range(0, len(colors_lab), 128):
        stop = min(start + 128, len(colors_lab))
        delta = ciede2000(colors_lab[start:stop, None, :], palette_lab[None, :, :])
        indices[start:stop] = delta.argmin(axis=1)
        distances[start:stop] = delta.min(axis=1)
    return indices, distances


# ---------------------------------------------------------------- palette

# Keys whose colors are light or tint settings for the engine, not paint.
NON_PAINT_KEYS = {"lights", "ambient", "light_tint", "shadow_tint"}


class Palette:
    """Every paint color in palettes.json with the ramps it belongs to."""

    def __init__(self, path=None):
        self.names = {}
        if path:
            self._walk(json.loads(Path(path).read_text()), [])
        self.hex = list(self.names)
        self.lab = srgb_to_lab(np.array([parse_hex(h) for h in self.hex])) if self.hex else np.zeros((0, 3))

    def _walk(self, o, trail):
        if isinstance(o, dict):
            for k, v in o.items():
                if k not in NON_PAINT_KEYS:
                    self._walk(v, trail + [k])
        elif isinstance(o, list):
            for v in o:
                self._walk(v, trail)
        elif isinstance(o, str) and re.fullmatch(r"#[0-9a-fA-F]{6}", o):
            name = "/".join(t for t in trail if t not in ("regions", "ramps", "enemies"))
            lst = self.names.setdefault(o.lower(), [])
            if name not in lst:
                lst.append(name)

    def __bool__(self):
        return bool(self.hex)

    def __contains__(self, h):
        return h in self.names


# ---------------------------------------------------------------- reporting

class Report:
    def __init__(self, path, kind):
        self.path = path
        self.kind = kind
        self.header = ""
        self.stats = {}
        self.issues = []

    def add(self, severity, check, message, examples=(), penalty=0.0, count=None):
        self.issues.append({
            "severity": severity, "check": check, "message": message,
            "examples": [str(e) for e in examples][:12], "penalty": round(float(penalty), 1), "count": count,
        })

    @property
    def score(self):
        return max(0, round(100 - sum(i["penalty"] for i in self.issues)))

    @property
    def has_error(self):
        return any(i["severity"] == "ERROR" for i in self.issues)

    def to_json(self):
        return {"file": str(self.path), "kind": self.kind, "score": self.score, "header": self.header,
                "stats": self.stats, "issues": self.issues}

    def print(self, verbose):
        print(f"{self.path}  [{self.kind}] {self.header}  score {self.score}/100")
        if self.stats:
            print("  " + ", ".join(f"{k} {v}" for k, v in self.stats.items()))
        shown = sorted(self.issues, key=lambda i: (SEVERITY_ORDER[i["severity"]], -i["penalty"]))
        hidden = 0
        for i in shown:
            if i["severity"] == "info" and not verbose:
                hidden += 1
                continue
            print(f"  {i['severity']:<5} {i['message']}")
            ex = i["examples"][: (12 if verbose else 3)]
            if ex:
                more = len(i["examples"]) - len(ex)
                print("        e.g. " + "; ".join(ex) + (f"; +{more} more" if more > 0 else ""))
        if hidden:
            print(f"  ({hidden} info note{'s' if hidden > 1 else ''}; -v to show)")
        if not shown:
            print("  no issues found")
        print()


# ---------------------------------------------------------------- helpers

def shifted(a, dy, dx, fill):
    """Neighbor map: value of a[y+dy, x+dx], or fill outside the array."""
    p = np.pad(a, 1, constant_values=fill)
    h, w = a.shape
    return p[1 + dy:1 + dy + h, 1 + dx:1 + dx + w]


def run_lengths_gcd(idx):
    """GCD of all horizontal and vertical same-color run lengths (1 for normal art)."""
    g = 0
    for arr in (idx, idx.T):
        for row in arr:
            change = np.flatnonzero(np.diff(row)) + 1
            bounds = np.concatenate(([0], change, [row.size]))
            for v in np.unique(np.diff(bounds)):
                g = math.gcd(g, int(v))
                if g == 1:
                    return 1
    return g


def staircase_hits(idx):
    """Find 45-degree edges drawn with 2x2 steps: a part drawn at double scale, or a
    fat jaggy. The pattern, in 2x2-block space with every block a single color:
        a b b        row r
        a a b        row r+1
    plus its mirror and transposes. Returns sorted (y, x) of each window's corner."""
    hits = set()
    for transpose in (False, True):
        m = idx.T if transpose else idx
        mh, mw = m.shape
        for oy in (0, 1):
            for ox in (0, 1):
                bh, bw = (mh - oy) // 2, (mw - ox) // 2
                if bh < 2 or bw < 3:
                    continue
                sub = m[oy:oy + 2 * bh, ox:ox + 2 * bw]
                blocks = sub.reshape(bh, 2, bw, 2).transpose(0, 2, 1, 3).reshape(bh, bw, 4)
                uni = np.all(blocks == blocks[..., :1], axis=-1)
                d = np.where(uni, blocks[..., 0], -2)  # -2 marks a mixed block
                for flip in (False, True):
                    dd = d[:, ::-1] if flip else d
                    uu = uni[:, ::-1] if flip else uni
                    a = dd[:-1, :-2]
                    ok = uu[:-1, :-2] & uu[:-1, 1:-1] & uu[:-1, 2:] & uu[1:, :-2] & uu[1:, 1:-1] & uu[1:, 2:]
                    pat = (ok & (a != dd[:-1, 1:-1]) & (dd[:-1, 1:-1] == dd[:-1, 2:])
                           & (dd[1:, :-2] == a) & (dd[1:, 1:-1] == a) & (dd[1:, 2:] == dd[:-1, 2:]))
                    for r, c in zip(*np.nonzero(pat)):
                        cc = (bw - 3 - c) if flip else c
                        y, x = oy + 2 * r, ox + 2 * cc
                        hits.add((x, y) if transpose else (y, x))
    return sorted(hits)


class UnionFind:
    def __init__(self, n):
        self.p = list(range(n))

    def find(self, i):
        while self.p[i] != i:
            self.p[i] = self.p[self.p[i]]
            i = self.p[i]
        return i

    def union(self, i, j):
        self.p[self.find(i)] = self.find(j)


# ---------------------------------------------------------------- sheet model

class Sheet:
    def __init__(self, path, manifest, palette):
        self.path = Path(path)
        self.manifest = manifest if manifest is not None else {}
        validate_manifest(self.manifest)
        self.palette = palette
        img = Image.open(path)
        self.mode = img.mode
        self.has_transparency_key = "transparency" in img.info
        self.rgba = np.array(img.convert("RGBA"))
        self.h, self.w = self.rgba.shape[:2]
        self.alpha = self.rgba[..., 3]
        colored = self.alpha > 0
        r, g, b = (self.rgba[..., i].astype(np.int64) for i in range(3))
        packed = (r << 16) | (g << 8) | b
        keys, first, inv, counts = np.unique(packed[colored], return_index=True, return_inverse=True, return_counts=True)
        self.first_positions = np.array(np.unravel_index(np.flatnonzero(colored)[first], colored.shape)).T
        self.idx = np.full((self.h, self.w), -1, dtype=np.int64)
        self.idx[colored] = inv.ravel()
        self.colors = np.stack([(keys >> 16) & 255, (keys >> 8) & 255, keys & 255], axis=-1).astype(np.uint8)
        self.counts = counts
        self.hex = [hexc(c) for c in self.colors]
        self.lab = srgb_to_lab(self.colors) if len(self.colors) else np.zeros((0, 3))
        self.L = self.lab[:, 0] if len(self.colors) else np.zeros(0)
        emis = {h.lower() for h in self.manifest.get("emissive", []) if isinstance(h, str)}
        self.declared_emissive = emis
        self.emissive = np.array([h in emis for h in self.hex], dtype=bool)
        self.stats = {}

        cell = self.manifest.get("cell")
        self.cell = (int(cell[0]), int(cell[1])) if isinstance(cell, list) and len(cell) >= 2 else (self.w, self.h)
        anims = self.manifest.get("anims", {}) if isinstance(self.manifest.get("anims"), dict) else {}
        if "tiles" in self.manifest:
            self.kind = "tiles"
        elif UI_NAME_RE.match(self.path.stem):
            self.kind = "ui"
        elif any(CHAR_ANIM_RE.match(n) for n in anims):
            self.kind = "character"
        elif self.manifest:
            self.kind = "prop"
        else:
            self.kind = "image"
        self.labels, self.anims = {}, {}
        for name, a in anims.items():
            if not isinstance(a, dict) or "row" not in a:
                continue
            self.anims[name] = a
            for i in range(int(a.get("frames", 1))):
                self.labels.setdefault((int(a.get("col", 0)) + i, int(a["row"])), f"{name}#{i}")
        tiles = self.manifest.get("tiles", {})
        if isinstance(tiles, dict):
            for name, pos in tiles.items():
                if isinstance(pos, (list, tuple)) and len(pos) >= 2:
                    self.labels.setdefault((int(pos[0]), int(pos[1])), name)
        self.single_cell = self.cell == (self.w, self.h)

    def cells(self):
        cw, ch = self.cell
        for row in range(self.h // ch):
            for col in range(self.w // cw):
                y0, x0 = row * ch, col * cw
                sub = self.idx[y0:y0 + ch, x0:x0 + cw]
                if (sub >= 0).any():
                    yield col, row, sub

    def label(self, col, row):
        return self.labels.get((col, row), f"c{col}r{row}")

    def where(self, col, row, x, y):
        if self.single_cell:
            return f"cell(0,0) pixel({x},{y})"
        cw, ch = self.cell
        return f"{self.label(col, row)} cell({col},{row}) pixel({x},{y}) @({col * cw + x},{row * ch + y})"

    def color_where(self, color_id):
        """Locate the first visible pixel of a color in cell and sheet coordinates."""
        y, x = map(int, self.first_positions[color_id])
        cw, ch = self.cell
        return self.where(x // cw, y // ch, x % cw, y % ch)

    def materials(self):
        """Material id per color: colors sharing a palette ramp are one material.
        -1 for outline-dark and emissive colors (they are not lit surfaces)."""
        n = len(self.colors)
        names = []
        if self.palette:
            near, distances = nearest_palette(self.lab, self.palette.lab)
        for i in range(n):
            if self.emissive[i] or self.L[i] < 8:
                names.append(set())
                continue
            if self.palette and distances[i] < MATERIAL_DE:
                s = set(self.palette.names[self.palette.hex[near[i]]])
                s = {x for x in s if not x.endswith(("outline", "hood_void", "ink"))}
            elif self.palette:
                s = {f"own:{self.hex[i]}"}
            else:  # no palette: group by hue
                a, b = self.lab[i, 1], self.lab[i, 2]
                s = {f"hue{int(((math.degrees(math.atan2(b, a)) + 360) % 360) // 40)}"} if math.hypot(a, b) > 8 else {"neutral"}
            names.append(s)
        uf = UnionFind(n)
        owner = {}
        for i, s in enumerate(names):
            for x in s:
                if x in owner:
                    uf.union(i, owner[x])
                else:
                    owner[x] = i
        return np.array([uf.find(i) if names[i] else -1 for i in range(n)], dtype=np.int64)


# ---------------------------------------------------------------- sprite checks

def validate_manifest(manifest):
    """Reject malformed sheet geometry and timing before indexing cells."""
    if not isinstance(manifest, dict):
        raise ValueError("manifest must be a JSON object")
    if "cell" in manifest:
        cell = manifest["cell"]
        if not isinstance(cell, list) or len(cell) != 2 or any(type(v) is not int or v <= 0 for v in cell):
            raise ValueError("manifest cell must be two positive integers")
    anims = manifest.get("anims", {})
    if not isinstance(anims, dict):
        raise ValueError("manifest anims must be an object")
    for name, anim in anims.items():
        if not isinstance(anim, dict):
            raise ValueError(f"anim {name} must be an object")
        for key, default, minimum in (("row", None, 0), ("frames", 1, 1), ("col", 0, 0)):
            value = anim.get(key, default)
            if type(value) is not int or value < minimum:
                raise ValueError(f"anim {name}: {key} must be an integer >= {minimum}")
        if "durations" in anim:
            values = anim["durations"]
            if not isinstance(values, list) or any(type(v) is not int or v <= 0 for v in values):
                raise ValueError(f"anim {name}: durations must be positive integer ticks")
        fps = anim.get("fps", 1)
        if isinstance(fps, bool) or not isinstance(fps, (int, float)) or not math.isfinite(fps) or fps <= 0:
            raise ValueError(f"anim {name}: fps must be positive and finite")
    emissive = manifest.get("emissive", [])
    if not isinstance(emissive, list) or any(not isinstance(v, str) or not re.fullmatch(r"#[0-9a-fA-F]{6}", v) for v in emissive):
        raise ValueError("manifest emissive must be a list of six-digit hex colors")

def check_manifest(sheet, rep):
    m = sheet.manifest
    if not m:
        rep.add("info", "manifest", "no manifest next to the PNG: the whole image is treated as one cell")
        return
    cw, ch = sheet.cell
    probs = []
    if "cell" not in m:
        probs.append("manifest has no \"cell\"")
    if sheet.w % cw or sheet.h % ch:
        probs.append(f"sheet {sheet.w}x{sheet.h} is not a whole number of {cw}x{ch} cells")
    anchor = m.get("anchor")
    if isinstance(anchor, list) and len(anchor) >= 2 and not (0 <= anchor[0] < cw and 0 <= anchor[1] < ch):
        probs.append(f"anchor {anchor[0]},{anchor[1]} lies outside the {cw}x{ch} cell")
    nrows, ncols = sheet.h // ch, sheet.w // cw
    dup_frames, even_timing, long_cycles = [], [], []
    for name, a in sheet.anims.items():
        row, n, col0 = int(a["row"]), int(a.get("frames", 1)), int(a.get("col", 0))
        if row >= nrows or col0 + n > ncols:
            probs.append(f"anim {name} (row {row}, {n} frames) runs past the {ncols}x{nrows}-cell sheet")
            continue
        frames = [sheet.rgba[row * ch:(row + 1) * ch, (col0 + i) * cw:(col0 + i + 1) * cw] for i in range(n)]
        empty = [i for i, f in enumerate(frames) if not (f[..., 3] > 0).any()]
        if empty:
            probs.append(f"anim {name} has empty frame(s) {empty}")
        d = a.get("durations")
        if d is not None and len(d) != n:
            probs.append(f"anim {name} has {len(d)} durations for {n} frames")
        for i in range(1, n):
            if np.array_equal(frames[i], frames[i - 1]):
                dup_frames.append(f"{name}#{i - 1}=#{i}")
        if TIMED_ANIM_RE.search(name) and n >= 3 and (d is None or len(set(d)) == 1):
            even_timing.append(name)
        if n >= 12:
            long_cycles.append(f"{name} ({n})")
    if probs:
        rep.add("ERROR", "manifest", f"{len(probs)} manifest problems", probs, penalty=min(30, 10 * len(probs)), count=len(probs))
    if dup_frames:
        rep.add("info", "anim", f"{len(dup_frames)} identical consecutive frames: hold one frame with durations instead", dup_frames)
    if even_timing:
        rep.add("info", "anim", f"{len(even_timing)} attack/windup anims have even frame timing: hold anticipation and follow-through about 2x longer than the fast frames (Slynyrd)", even_timing)
    if long_cycles:
        rep.add("info", "anim", "cycles of 12+ frames often look floaty: fewer, stronger key poses (Slynyrd)", long_cycles)
    unref = [sheet.label(c, r) for c, r, _ in sheet.cells() if (c, r) not in sheet.labels]
    if unref and sheet.labels:
        rep.add("info", "manifest", f"{len(unref)} non-empty cells that no anim or tile points at", unref)
    missing = sorted(h for h in sheet.declared_emissive if h not in sheet.hex)
    if missing:
        rep.add("info", "manifest", "emissive colors listed in the manifest but not used in the sheet (typo?)", missing)


def check_alpha(sheet, rep):
    if sheet.mode not in ("RGBA", "LA", "PA") and not sheet.has_transparency_key:
        rep.add("WARN", "alpha", f"PNG mode {sheet.mode} has no alpha channel: save RGBA with a transparent background", penalty=5)
    semi = (sheet.alpha > 0) & (sheet.alpha < 255)
    n = int(semi.sum())
    if n:
        cw, ch = sheet.cell
        ex = [sheet.where(x // cw, y // ch, x % cw, y % ch) + f" a={sheet.alpha[y, x]}" for y, x in list(zip(*np.nonzero(semi)))[:12]]
        rep.add("ERROR", "alpha", f"{n} semi-transparent pixels: every pixel must be alpha 0 or 255 (soft edges read as filtered or AI-made)",
                ex, penalty=min(40, 20 + n / 5), count=n)
    if sheet.kind in ("character", "prop") and not (sheet.alpha == 0).any():
        rep.add("WARN", "alpha", "no transparent pixels at all: was the background cleared?", penalty=10)


def check_scale(sheet, rep):
    """Return True for an apparent common pixel scale, which may be intentional."""
    g = run_lengths_gcd(sheet.idx)
    if g >= 2:
        rep.add("WARN" if len(sheet.colors) > 1 else "info", "scale", f"all horizontal/vertical runs have a common factor {g}: possible {g}x-authored grid; simple rectangles can also do this. Verify at 1x",
                [sheet.color_where(0)], penalty=15 if len(sheet.colors) > 1 else 0)
        return True
    total, cells, ex = 0, [], []
    for col, row, sub in sheet.cells():
        hits = staircase_hits(sub)
        if hits:
            total += len(hits)
            cells.append(f"{sheet.label(col, row)}: {len(hits)}")
            ex += [sheet.where(col, row, x, y) for y, x in hits[:2]]
    if total:
        repeated = total >= 4
        rep.add("WARN" if repeated else "info", "staircase", f"{total} doubled staircase windows in {len(cells)} cells: possible mixed pixel scale; ordinary 2px clusters can also match. Check neighboring 1px detail before redrawing",
                ex + cells, penalty=min(10, total) if repeated else 0, count=total)
    return False


def pair_examples(sheet, pairs, de, fmt, locations=None):
    return [fmt.format(a=sheet.hex[i], b=sheet.hex[j], d=d, ca=sheet.counts[i], cb=sheet.counts[j], n=n)
            + " at " + (locations[(i, j)] if locations is not None else sheet.color_where(i) + " / " + sheet.color_where(j))
            for i, j, n, d in pairs[:12]]


def check_colors(sheet, rep):
    n = len(sheet.colors)
    cell_lim, sheet_lim = COLOR_LIMITS[sheet.kind]
    cw, ch = sheet.cell
    if cw >= 48 and ch >= 48:
        cell_lim += BIG_CELL_EXTRA
    over, max_cell = [], 0
    for col, row, sub in sheet.cells():
        k = len(np.unique(sub[sub >= 0]))
        max_cell = max(max_cell, k)
        if k > cell_lim:
            y, x = np.argwhere(sub >= 0)[0]
            over.append(f"{sheet.where(col, row, x, y)}: {k} colors")
    sheet.stats["colors"] = n
    if not sheet.single_cell:
        sheet.stats["max/cell"] = max_cell
    if sheet.single_cell:
        if n > cell_lim:
            rep.add("WARN", "colors", f"{n} colors (aim for at most about {cell_lim} for a {sheet.kind}): fewer colors, each with its own job",
                    [sheet.color_where(0)], penalty=min(10, 2 + (n - cell_lim)))
    else:
        if over:
            rep.add("WARN", "colors", f"{len(over)} cells use more than {cell_lim} colors: fewer colors, each with its own job",
                    over, penalty=min(10, 2 + len(over) / 2), count=len(over))
        if n > sheet_lim:
            rep.add("WARN", "colors", f"the sheet uses {n} colors (review budget about {sheet_lim} for a {sheet.kind} sheet)", penalty=min(8, 2 + (n - sheet_lim)))

    in_pal = np.array([h in sheet.palette for h in sheet.hex], dtype=bool)
    selected = np.argsort(-sheet.counts, kind="stable")[:PAIR_SCAN_LIMIT]
    de = ciede2000(sheet.lab[selected, None, :], sheet.lab[None, selected, :])

    # near-duplicates anywhere in the sheet
    iu = np.triu_indices(len(selected), 1)
    pairs = sorted(((int(selected[i]), int(selected[j]), 0, float(de[i, j])) for i, j in zip(*iu) if de[i, j] < NEAR_DUP_DE), key=lambda p: p[3])
    if n > PAIR_SCAN_LIMIT:
        rep.add("info", "near-dup", f"near-duplicate pair scan uses the {PAIR_SCAN_LIMIT} most frequent colors of {n}; palette distances still cover every color")
    own = [p for p in pairs if not (in_pal[p[0]] and in_pal[p[1]])]
    pal = [p for p in pairs if in_pal[p[0]] and in_pal[p[1]]]
    fmt = "{a}~{b} dE {d:.1f} ({ca}px/{cb}px)"
    if own:
        rep.add("WARN", "near-dup", f"{len(own)} pairs of near-identical colors (dE < {NEAR_DUP_DE}): merge each pair into one",
                pair_examples(sheet, own, de, fmt), penalty=min(10, 3 * len(own)), count=len(own))
    if pal:
        rep.add("info", "near-dup", f"{len(pal)} near-identical pairs that both come from palettes.json (consider using just one)",
                pair_examples(sheet, pal, de, fmt))

    # touching colors too similar to tell apart at 1x
    touch, touch_locations = {}, {}
    for col, row, sub in sheet.cells():
        for dy, dx in ((0, 1), (1, 0)):
            a, b = sub[: sub.shape[0] - dy, : sub.shape[1] - dx], sub[dy:, dx:]
            m = (a >= 0) & (b >= 0) & (a != b)
            if m.any():
                aa, bb = a[m], b[m]
                close = ciede2000(sheet.lab[aa], sheet.lab[bb]) < TOUCH_DE
                for i, j, (y, x) in zip(aa[close], bb[close], np.argwhere(m)[close]):
                    key = (min(i, j), max(i, j))
                    touch[key] = touch.get(key, 0) + 1
                    if key not in touch_locations:
                        touch_locations[key] = sheet.where(col, row, x, y) + " / " + sheet.where(col, row, x + dx, y + dy)
    tp = sorted(((i, j, v, float(ciede2000(sheet.lab[i], sheet.lab[j]))) for (i, j), v in touch.items() if v >= 3), key=lambda p: -p[2])
    own = [p for p in tp if not (in_pal[p[0]] and in_pal[p[1]])]
    pal = [p for p in tp if in_pal[p[0]] and in_pal[p[1]]]
    fmt = "{a}|{b} dE {d:.1f}, {n} edges"
    if own:
        rep.add("WARN", "touching", f"{len(own)} low-contrast touching pairs (dE < {TOUCH_DE}); check whether the boundary needs to read at 1x",
                pair_examples(sheet, own, de, fmt, touch_locations), penalty=min(8, 2 * len(own)), count=len(own))
    if pal:
        rep.add("info", "touching", f"{len(pal)} pairs of touching palette colors that look the same at 1x (fine if they are meant to merge, e.g. outline into darkest shadow)",
                pair_examples(sheet, pal, de, fmt, touch_locations))

    # colors used once or twice in the whole sheet ("dirty tool")
    single = [i for i in range(n) if sheet.counts[i] <= 2 and not sheet.emissive[i]]
    s_own = [f"{sheet.hex[i]} x{sheet.counts[i]} at {sheet.color_where(i)}" for i in single if not in_pal[i]]
    s_pal = [f"{sheet.hex[i]} x{sheet.counts[i]} at {sheet.color_where(i)}" for i in single if in_pal[i]]
    if s_own:
        rep.add("WARN" if sheet.palette else "info", "single-use", f"{len(s_own)} {'off-palette ' if sheet.palette else ''}colors used in only 1-2 pixels of the whole sheet: verify deliberate accents or reuse a ramp color",
                s_own[:12], penalty=min(6, 2 * len(s_own)) if sheet.palette else 0, count=len(s_own))
    if s_pal:
        rep.add("info", "single-use", f"{len(s_pal)} palette colors used in only 1-2 pixels (check each is a deliberate accent)", s_pal)

    # untinted black, neutral gray, pure primaries
    bad = []
    for i, (r, g, b) in enumerate(sheet.colors.astype(int)):
        h = sheet.hex[i]
        if h == "#ffffff":
            continue
        if r == g == b:
            bad.append(f"{h} {'pure black' if r == 0 else 'neutral gray'} x{sheet.counts[i]} at {sheet.color_where(i)}")
        elif sorted((r, g, b)) == [0, 0, 255]:
            bad.append(f"{h} pure primary x{sheet.counts[i]} at {sheet.color_where(i)}")
    if bad:
        rep.add("WARN", "untinted", f"{len(bad)} untinted colors: tint blacks and grays toward the region's shadow or light hue",
                bad, penalty=min(6, 3 * len(bad)), count=len(bad))

    # palette match
    if not sheet.palette:
        return
    nearest, distances = nearest_palette(sheet.lab, sheet.palette.lab)
    manifest_text = str(sheet.manifest.get("notes", "")).lower()
    near, far, declared, ramps = [], [], [], set()
    for i in range(n):
        h = sheet.hex[i]
        if in_pal[i]:
            ramps.add(sheet.palette.names[h][0])
            continue
        ph = sheet.palette.hex[nearest[i]]
        d = distances[i]
        entry = (f"{h} x{sheet.counts[i]} (nearest {ph} {sheet.palette.names[ph][0]}, dE {d:.1f}) at {sheet.color_where(i)}", sheet.counts[i])
        if h in manifest_text:
            declared.append(entry)
        elif d < PALETTE_NEAR_MISS_DE:
            near.append(entry)
        else:
            far.append(entry)
    total = int(sheet.counts.sum())
    sheet.stats["on-palette"] = f"{100 * sheet.counts[in_pal].sum() / total:.0f}%"
    if near:
        rep.add("WARN", "palette", f"{len(near)} colors are near-misses of a palette color: use the exact palette hex",
                [e for e, _ in near], penalty=min(10, 3 * len(near)), count=len(near))
    if far:
        px = sum(c for _, c in far)
        rep.add("WARN", "palette", f"{len(far)} colors ({100 * px / total:.0f}% of pixels) are not in palettes.json: use the region ramps, or name the hex in the manifest notes if the change is deliberate",
                [e for e, _ in far], penalty=min(15, 2 * len(far)), count=len(far))
    if declared:
        rep.add("info", "palette", f"{len(declared)} off-palette colors documented in manifest notes (review the stated intent)", [e for e, _ in declared])
    if ramps:
        rep.add("info", "palette", "ramps used (first ramp per color): " + ", ".join(sorted(ramps)))


def check_lone_pixels(sheet, rep):
    """Pixels with no same-color 8-neighbor. Only two kinds are flagged:
    detached (no opaque neighbor at all) and in-fill (all 8 neighbors are one other
    color: a speck or sparkle dropped on a flat area). Lone pixels at edges and
    junctions are normal (stair steps, AA, corner highlights) and only counted."""
    tile = sheet.kind == "tiles"
    found = {"detached": [], "glint": [], "sparkle": [], "speck": []}
    other, opaque_px, orphans4, orphans8 = 0, 0, 0, 0
    busy_cells, patch_cells = [], []
    for col, row, sub in sheet.cells():
        opaque_px += int((sub >= 0).sum())
        nbs = np.stack([shifted(sub, dy, dx, -1) for dy, dx in OFFS8])
        lone = (sub >= 0) & ~np.any(nbs == sub, axis=0)
        nbs4 = np.stack([shifted(sub, dy, dx, -1) for dy, dx in OFFS4])
        orphans4 += int(((sub >= 0) & ~np.any(nbs4 == sub, axis=0)).sum())
        orphans8 += int(lone.sum())
        if not lone.any():
            continue
        cell_pts = []
        for y, x in zip(*np.nonzero(lone)):
            around = nbs[:, y, x]
            c = sub[y, x]
            tag = sheet.where(col, row, x, y) + f" {sheet.hex[c]}"
            if (around < 0).all():
                found["detached"].append(tag)
                if not sheet.emissive[c]:
                    cell_pts.append((y, x))
            elif (around == around[0]).all():
                kind = "glint" if sheet.emissive[c] else ("sparkle" if sheet.L[c] > sheet.L[around[0]] else "speck")
                found[kind].append(tag)
                if kind != "glint":
                    cell_pts.append((y, x))
            else:
                other += 1
        limit = max(3 if tile else 4, math.ceil(0.01 * (sub >= 0).sum()))
        if len(cell_pts) > limit:
            busy_cells.append(f"{sheet.label(col, row)}: {len(cell_pts)}")
        if len(cell_pts) >= 3:
            pts = np.array(cell_pts)
            d = np.abs(pts[:, None, :] - pts[None, :, :]).max(axis=-1)
            if ((d <= 2).sum(axis=1) >= 3).any():
                patch_cells.append(sheet.label(col, row))
    n_spk, n_det = len(found["speck"]) + len(found["sparkle"]), len(found["detached"])
    if opaque_px:
        sheet.stats["specks/1000px"] = f"{1000 * (n_spk + n_det) / opaque_px:.1f}"
        sheet.stats["orphans 4/8"] = f"{orphans4}/{orphans8}"
    if busy_cells:
        rep.add("WARN", "specks", f"{len(busy_cells)} cells exceed the lone-fill/detached pixel budget (max of {'3' if tile else '4'} and 1% of visible area): review texture noise; eyes, glyphs and particles can be intentional",
                found["speck"][:3] + found["sparkle"][:3] + found["detached"][:3] + busy_cells,
                penalty=min(15, 2 + len(busy_cells)), count=len(busy_cells))
    if patch_cells:
        rep.add("WARN", "speckle", f"{len(patch_cells)} cells have 3+ lone pixels within a 5x5 area (reads as static or noise)",
                patch_cells, penalty=min(10, 2 * len(patch_cells)), count=len(patch_cells))
    if n_det and not busy_cells:
        rep.add("info", "detached", f"{n_det} pixels have no opaque neighbor: inspect for stray pixels versus intentional particles or glyph details", found["detached"])
    if n_spk and not busy_cells:
        rep.add("info", "specks", f"{n_spk} lone pixels in flat fills (each should be on purpose: an eye, a rivet, a star)", found["speck"] + found["sparkle"])
    if found["glint"]:
        rep.add("info", "glint", f"{len(found['glint'])} lone emissive pixels in fills (glints)", found["glint"])
    if other:
        rep.add("info", "lone", f"{other} isolated-color pixels at edges/junctions: inspect for deliberate stairs and highlights versus noisy interpolation")


def run_stats(mat, lmap):
    """For runs of 3+ same-material pixels along each row: (first L, last L, interior mean L, length)."""
    out = []
    for mrow, lrow in zip(mat, lmap):
        w, x = len(mrow), 0
        while x < w:
            m = mrow[x]
            e = x
            while e + 1 < w and mrow[e + 1] == m:
                e += 1
            if m >= 0 and e - x >= 2:
                seg = lrow[x:e + 1]
                out.append((seg[0], seg[-1], seg[1:-1].mean(), e - x + 1))
            x = e + 1
    return out


def check_lighting(sheet, rep):
    """Light comes from the upper left. Along every row run of one material (one ramp)
    the left end should be lighter than the right end, and along every column run the
    top lighter than the bottom. Comparing within a material keeps local color (a
    dark cloth next to pale skin) out of it. Pillow shading shows as both ends of
    runs darker than the middle with no left/right or top/bottom difference."""
    if sheet.kind in ("tiles", "ui") or not len(sheet.colors):
        return
    mat_of = sheet.materials()
    wrong_x, wrong_y, pillow = [], [], []
    hs, vs = [], []
    for col, row, sub in sheet.cells():
        mat = np.where(sub >= 0, mat_of[np.maximum(sub, 0)], -1)
        lmap = np.where(sub >= 0, sheet.L[np.maximum(sub, 0)], 0.0)
        rr = run_stats(mat, lmap)
        cr = run_stats(mat.T, lmap.T)
        # only runs that actually change value say anything about light
        rr_v = [r for r in rr if r[0] != r[1] or r[2] != r[0]]
        cr_v = [r for r in cr if r[0] != r[1] or r[2] != r[0]]
        if len(rr_v) < 4 or len(cr_v) < 4:
            continue
        h = float(np.mean([a - b for a, b, _, _ in rr_v]))
        v = float(np.mean([a - b for a, b, _, _ in cr_v]))
        long_runs = [r for r in rr_v + cr_v if r[3] >= 4]
        ends = float(np.mean([mid - (a + b) / 2 for a, b, mid, _ in long_runs])) if long_runs else 0.0
        hs.append(h)
        vs.append(v)
        name = sheet.label(col, row)
        if h < -2.5:
            wrong_x.append(f"{name}: {h:+.1f}")
        if v < -2.5:
            wrong_y.append(f"{name}: {v:+.1f}")
        if ends > 4 and abs(h) < 1.5 and abs(v) < 1.5:
            pillow.append(f"{name}: ends {ends:.1f} darker, L-R {h:+.1f}, T-B {v:+.1f}")
    if not hs:
        return
    sheet.stats["light L-R/T-B"] = f"{np.median(hs):+.1f}/{np.median(vs):+.1f}"
    ncell = len(hs)
    for lst, side in ((wrong_x, "lighter on the right than the left"), (wrong_y, "lighter at the bottom than the top")):
        if not lst:
            continue
        if len(lst) >= max(2, ncell // 4):
            rep.add("info", "light", f"{len(lst)} of {ncell} cells have ramp runs {side} (L* step shown): inspect light direction manually; material grouping, pose and local emitters can explain this",
                    lst, count=len(lst))
        else:
            rep.add("info", "light", f"{len(lst)} of {ncell} cells are {side} within the same material", lst)
    if pillow and len(pillow) >= max(1, ncell // 4):
        rep.add("info", "pillow", f"{len(pillow)} of {ncell} cells have bright-centered ramp runs: inspect for pillow shading; this statistic cannot infer surface geometry",
                pillow, count=len(pillow))
    elif pillow:
        rep.add("info", "pillow", f"{len(pillow)} cells may be pillow-shaded", pillow)


def check_glow_and_saturation(sheet, rep):
    total = int(sheet.counts.sum())
    emis_px = int(sheet.counts[sheet.emissive].sum())
    if sheet.declared_emissive:
        sheet.stats["emissive"] = f"{100 * emis_px / total:.0f}%"
        if sheet.kind in ("character", "prop") and emis_px / total > 0.25:
            rep.add("info", "glow", f"{100 * emis_px / total:.0f}% of the opaque pixels are emissive: fine for a glowing object (crystal, lava, magic), too much for anything else")
    rgb = sheet.colors.astype(float) / 255
    mx, mn = rgb.max(axis=1), rgb.min(axis=1)
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-9), 0)
    burn = (sat > 0.85) & (mx > 0.85) & ~sheet.emissive
    burn_px = int(sheet.counts[burn].sum())
    if burn_px >= 12 and burn_px / total > 0.10:
        rep.add("WARN", "eye-burn", f"{100 * burn_px / total:.0f}% of the pixels are non-emissive colors with saturation and brightness both above 85% (eye-burn): keep those for small accents",
                [sheet.hex[i] for i in np.flatnonzero(burn)], penalty=5)


def check_tiles(sheet, rep):
    if sheet.kind != "tiles":
        return
    seen, dups, ring_diff = {}, [], []
    for col, row, sub in sheet.cells():
        name = sheet.label(col, row)
        key = sub.tobytes()
        if key in seen:
            dups.append(f"{name}={seen[key]}")
        else:
            seen[key] = name
        if (sub >= 0).all() and min(sub.shape) >= 8:
            lmap = sheet.L[sub]
            ring = np.ones(sub.shape, dtype=bool)
            ring[1:-1, 1:-1] = False
            d = lmap[ring].mean() - lmap[~ring].mean()
            if abs(d) > 8:
                ring_diff.append(f"{name}: {d:+.0f}")
    if dups:
        rep.add("info", "tiles", f"{len(dups)} tiles are pixel-identical to another tile (a wasted variant?)", dups)
    if ring_diff:
        rep.add("info", "tiles", f"{len(ring_diff)} full tiles have a border ring much lighter or darker than the middle (L* shown): fine for a designed edge, but on repeating ground it shows the 16px grid",
                ring_diff)


def lint_sprite(path, manifest, palette):
    sheet = Sheet(path, manifest, palette)
    rep = Report(sheet.path, sheet.kind)
    cw, ch = sheet.cell
    ncells = sum(1 for _ in sheet.cells())
    rep.header = f"{sheet.w}x{sheet.h}" + ("" if sheet.single_cell else f", cell {cw}x{ch}, {ncells} non-empty cells")
    check_manifest(sheet, rep)
    check_alpha(sheet, rep)
    if not len(sheet.colors):
        rep.add("ERROR", "colors", "the image is fully transparent", penalty=50)
        return rep
    upscaled = check_scale(sheet, rep)
    check_colors(sheet, rep)
    if not upscaled:
        check_lone_pixels(sheet, rep)
        check_lighting(sheet, rep)
    check_glow_and_saturation(sheet, rep)
    check_tiles(sheet, rep)
    rep.stats = sheet.stats
    return rep


# ---------------------------------------------------------------- screenshots

# Reference ranges the research measured on 60 Steam screenshots of Hyper Light
# Drifter, Eastward, CrossCode, Sea of Stars, Noita and Celeste (JPEGs, so rough).
FRAME_TARGETS = [
    ("L_spread", "value spread (L* p95-p5)", lambda v: v >= 45, "at least 45 (references: median 54)", "{:.0f}"),
    ("center_border", "center/border brightness", lambda v: v >= 1.15, "at least 1.15 (center brighter in 55 of 60 references)", "{:.2f}"),
    ("accent", "bright saturated pixels", lambda v: v <= 0.05, "at most 5% (references: 0.2-1.9%)", "{:.1%}"),
    ("near_white", "near-white pixels (L* > 80)", lambda v: v <= 0.05, "at most 5% (references: 0.2-3.7%)", "{:.1%}"),
    ("top2hue", "share of the top two hue families", lambda v: v >= 0.6, "at least 60% (references: 64-80%)", "{:.0%}"),
    ("shadow_chroma", "chroma of the darkest 5%", lambda v: v >= 4, "at least 4 (tinted shadows, not neutral black)", "{:.1f}"),
]


def lint_frame(path):
    rgba = np.array(Image.open(path).convert("RGBA"))
    a = rgba[..., :3]
    h, w = a.shape[:2]
    rep = Report(Path(path), "frame")
    semi = (rgba[..., 3] > 0) & (rgba[..., 3] < 255)
    if semi.any():
        rep.add("ERROR", "alpha", f"{int(semi.sum())} semi-transparent screenshot pixels",
                [f"pixel({x},{y})" for y, x in np.argwhere(semi)[:12]], penalty=25, count=int(semi.sum()))
    k = w // NATIVE_W
    if k < 1 or w != k * NATIVE_W or h != k * NATIVE_H:
        rep.add("WARN", "grid", f"{w}x{h} is not 384x216 times a whole number: a non-integer scale, or not a full frame", penalty=5)
        k = 1
    if k > 1:
        blocks = a.reshape(NATIVE_H, k, NATIVE_W, k, 3)
        uni = np.all(blocks == blocks[:, :1, :, :1, :], axis=(1, 3, 4))
        bad = np.argwhere(~uni)
        if len(bad):
            share = len(bad) / uni.size
            rep.add("ERROR" if share > 0.002 else "WARN", "grid",
                    f"{len(bad)} native pixels ({share:.1%}) are not uniform {k}x{k} blocks: something is off the pixel grid (smooth scaling, sub-pixel position, rotation, or a full-resolution overlay)",
                    [f"native ({x},{y})" for y, x in bad[:12]], penalty=min(40, 10 + 2000 * share), count=len(bad))
        native = blocks[:, 0, :, 0, :]
    else:
        native = a
    lab = srgb_to_lab(native)
    L = lab[..., 0]
    C = np.hypot(lab[..., 1], lab[..., 2])
    H = (np.degrees(np.arctan2(lab[..., 2], lab[..., 1])) + 360) % 360
    hh, ww = L.shape
    by, bx = int(hh * 0.12), int(ww * 0.12)
    border = np.ones(L.shape, dtype=bool)
    border[by:hh - by, bx:ww - bx] = False
    center = L[int(hh * 0.25):int(hh * 0.75), int(ww * 0.25):int(ww * 0.75)]
    p5, p95 = np.percentile(L, [5, 95])
    chrom = C > 15
    top2 = 0.0
    if chrom.any():
        hist, _ = np.histogram(H[chrom], bins=12, range=(0, 360))
        top2 = np.sort(hist)[::-1][:2].sum() / hist.sum()
    m = {
        "L_spread": p95 - p5,
        "center_border": center.mean() / max(L[border].mean(), 1e-6),
        "accent": float(((C > 50) & (L > 60)).mean()),
        "near_white": float((L > 80).mean()),
        "top2hue": float(top2),
        "shadow_chroma": float(C[L <= np.percentile(L, 5)].mean()),
    }
    fails = [f"{label} {fmt.format(m[key])}, target {target}" for key, label, ok, target, fmt in FRAME_TARGETS if not ok(m[key])]
    if fails:
        rep.add("info", "frame", f"{len(fails)} composition prompts from exploratory reference measurements (not pass/fail limits)", fails, count=len(fails))
    black = int(np.all(native.reshape(-1, 3) == 0, axis=1).sum())
    if black:
        rep.add("WARN", "untinted", f"{black} pure black pixels: tint the darkest shadows (deep blue or violet, not black)", penalty=3)
    rep.header = f"{w}x{h} (scale {k}), {len(np.unique(native.reshape(-1, 3), axis=0))} colors at native size"
    rep.stats = {"L p5-p95": f"{p5:.0f}-{p95:.0f}", "dark<20": f"{(L < 20).mean():.0%}", "center/border": f"{m['center_border']:.2f}",
                 "accent": f"{m['accent']:.1%}", "near-white": f"{m['near_white']:.1%}", "top2hue": f"{m['top2hue']:.0%}"}
    return rep


# ---------------------------------------------------------------- main

def is_frame_size(path):
    with Image.open(path) as im:
        w, h = im.size
    k = w // NATIVE_W
    return k >= 1 and w == k * NATIVE_W and h == k * NATIVE_H


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("paths", nargs="+", help="PNG files or directories of PNGs")
    ap.add_argument("--palette", default=str(DEFAULT_PALETTE), help="palettes.json (default: assets/concept/palettes.json)")
    ap.add_argument("--frame", action="store_true", help="treat every input as a screenshot")
    ap.add_argument("-v", "--verbose", action="store_true", help="show info notes and more examples")
    ap.add_argument("--json", action="store_true", help="print JSON instead of text")
    args = ap.parse_args()

    if Path(args.palette).exists():
        palette = Palette(args.palette)
    elif args.palette == str(DEFAULT_PALETTE):
        palette = Palette()
    else:
        sys.exit(f"palette not found: {args.palette}")

    files = []
    for p in map(Path, args.paths):
        if p.is_dir():
            files += sorted(f for f in p.glob("*.png") if f.is_file())
        elif p.exists():
            files.append(p)
        else:
            sys.exit(f"not found: {p}")
    if not files:
        sys.exit("no PNG files found")

    reports = []
    for f in files:
        mpath = f.with_suffix(".json")
        manifest = None
        try:
            if mpath.exists():
                manifest = json.loads(mpath.read_text())
            if args.frame or (manifest is None and is_frame_size(f)):
                reports.append(lint_frame(f))
            else:
                reports.append(lint_sprite(f, manifest, palette))
        except Exception as e:  # report and keep going: one broken file shouldn't hide the rest
            rep = Report(f, "?")
            check = "input" if isinstance(e, (ValueError, OSError)) else "crash"
            rep.add("ERROR", check, f"could not lint this file: {type(e).__name__}: {e}", penalty=100)
            reports.append(rep)

    if args.json:
        print(json.dumps([r.to_json() for r in reports], indent=1, default=str))
    else:
        for r in reports:
            r.print(args.verbose)
        if len(reports) > 1:
            print("summary (worst first)")
            for r in sorted(reports, key=lambda r: r.score):
                n_err = sum(i["severity"] == "ERROR" for i in r.issues)
                n_warn = sum(i["severity"] == "WARN" for i in r.issues)
                print(f"  {r.score:>3}  {r.path.name:<34} {n_err} errors, {n_warn} warnings")
            print(f"  overall {round(sum(r.score for r in reports) / len(reports))}/100 over {len(reports)} files")
    sys.exit(1 if any(r.has_error for r in reports) else 0)


if __name__ == "__main__":
    main()
