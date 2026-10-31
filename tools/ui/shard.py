"""shard.png: the Prism shard collectible, style v2 (2026-10-30).

A broken fragment of the Prism, faceted on both faces around a raised center, spinning 45 degrees
per frame about its vertical axis. Body in the fx.prism iris ramp, the facet facing the upper-left
light in Glint's cyan (cyan belongs to Glint and the Prism), outlined in deep iris. No white or
pink glints and no pickup star: pickup is a pale flash, then one thin ring and a broken ring.
Cell size, anchor, animation names and timings match the previous sheet.
Run from the project root: uv run --no-project --with numpy --with pillow python3 tools/ui/shard.py
"""

import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
from common import SPRITES, new, rgb, save_json, save_png  # noqa: E402

CW, CH = 16, 24
AX, AY = 8, 22
N = 8
OUT = "#38324a"                      # deep iris outline
SHADES = ["#564d6e", "#8d7fa8", "#63d7e0"]   # shaded, mid, lit facet (cyan)
PALE = "#e2d8e9"                     # prism highlight, used for the pickup flash
MID = "#b7add0"

# outline in the shard's own plane (x right, y down), clockwise from the top tip; the right side
# is the fractured one (a notch and a short step)
OUTLINE = [(0.2, -8.6), (2.4, -4.2), (3.4, -1.6), (2.0, -0.6), (3.0, 1.6), (0.9, 5.6),
           (-1.0, 4.4), (-3.4, -0.4), (-2.2, -4.8)]
APEX = (0.0, -1.0)
DEPTH = 1.5
LIGHT = np.array([-0.5, -0.6, 0.62]) / np.linalg.norm([-0.5, -0.6, 0.62])
BOB = [0, 0, -1, -1, -1, -1, 0, 0]


def mesh():
    """Triangles fanning from a raised center to the outline, on the front and back faces."""
    tris = []
    for sgn in (1, -1):
        c = np.array([APEX[0], APEX[1], DEPTH * sgn])
        for i in range(len(OUTLINE)):
            a, b = OUTLINE[i], OUTLINE[(i + 1) % len(OUTLINE)]
            pa, pb = np.array([a[0], a[1], 0.0]), np.array([b[0], b[1], 0.0])
            tris.append((c, pa, pb) if sgn > 0 else (c, pb, pa))
    return tris


def raster(theta, cx, cy):
    """Rasterize the shard turned by theta: a shade index per pixel, -1 where empty."""
    ct, st = math.cos(theta), math.sin(theta)
    zbuf = np.full((CH, CW), -1e9)
    idx = np.full((CH, CW), -1)
    for tri in mesh():
        pts = [np.array([x * ct + z * st, y, -x * st + z * ct]) for x, y, z in tri]
        n = np.cross(pts[1] - pts[0], pts[2] - pts[0])
        if np.linalg.norm(n) < 1e-6:
            continue
        n = n / np.linalg.norm(n)
        if n[2] <= 0:
            continue
        lum = float(np.dot(n, LIGHT))
        k = 2 if lum > 0.72 else 1 if lum > 0.3 else 0
        xs = [p[0] + cx for p in pts]
        ys = [p[1] + cy for p in pts]
        z = sum(p[2] for p in pts) / 3
        for py in range(max(0, int(min(ys)) - 1), min(CH, int(max(ys)) + 2)):
            for px in range(max(0, int(min(xs)) - 1), min(CW, int(max(xs)) + 2)):
                qx, qy = px + 0.5, py + 0.5
                d = [(xs[(i + 1) % 3] - xs[i]) * (qy - ys[i]) - (ys[(i + 1) % 3] - ys[i]) * (qx - xs[i]) for i in range(3)]
                if (min(d) >= 0 or max(d) <= 0) and z > zbuf[py, px]:
                    zbuf[py, px] = z
                    idx[py, px] = k
    return idx


def clean(idx):
    """Merge single pixels whose four neighbours all share another shade (no orphan specks)."""
    out = idx.copy()
    for y in range(1, CH - 1):
        for x in range(1, CW - 1):
            if idx[y, x] < 0:
                continue
            nb = [idx[y - 1, x], idx[y + 1, x], idx[y, x - 1], idx[y, x + 1]]
            if nb.count(nb[0]) == 4 and nb[0] >= 0 and nb[0] != idx[y, x]:
                out[y, x] = nb[0]
    return out


def to_img(idx, colors):
    """Paint shade indices with colors and add a 4-neighbour deep-iris outline."""
    img = new(CW, CH)
    m = idx >= 0
    ring = np.zeros_like(m)
    ring[1:] |= m[:-1]
    ring[:-1] |= m[1:]
    ring[:, 1:] |= m[:, :-1]
    ring[:, :-1] |= m[:, 1:]
    ring &= ~m
    img[ring, :3] = rgb(OUT)
    img[ring, 3] = 255
    for k, c in enumerate(colors):
        sel = idx == k
        img[sel, :3] = rgb(c)
        img[sel, 3] = 255
    return img


def spin_frames():
    """Eight 45-degree spin frames with a 1 px bob."""
    return [to_img(clean(raster(i * 2 * math.pi / N, 8.0, 12.0 + BOB[i])), SHADES) for i in range(N)]


def ring(r, col, dashed):
    """A thin ring (optionally broken into eighths) around the shard's center."""
    img = new(CW, CH)
    for y in range(CH):
        for x in range(CW):
            d = math.hypot(x + 0.5 - 8, y + 0.5 - 12.5)
            if r - 1 <= d < r:
                a = math.atan2(y + 0.5 - 12.5, x + 0.5 - 8)
                if not dashed or int((a + math.pi) / (math.pi / 4)) % 2 == 0:
                    img[y, x, :3] = rgb(col)
                    img[y, x, 3] = 255
    return img


def pickup_frames():
    """Pale flash of the front silhouette, the edge-on sliver, a thin ring, then a broken ring."""
    front = clean(raster(0.0, 8.0, 12.0))
    flash = to_img(front, [PALE, PALE, PALE])
    edge = clean(raster(math.pi / 2, 8.0, 12.0))
    return [flash, to_img(edge, [MID, MID, MID]), ring(4.5, PALE, False), ring(6.5, MID, True)]


def main():
    """Write shard.png and shard.json."""
    spin, pick = spin_frames(), pickup_frames()
    sheet = new(CW * N, CH * 2)
    for i, f in enumerate(spin):
        sheet[0:CH, i * CW:(i + 1) * CW] = f
    for i, f in enumerate(pick):
        sheet[CH:, i * CW:(i + 1) * CW] = f
    save_png(sheet, os.path.join(SPRITES, "shard.png"))
    save_json({
        "cell": [CW, CH],
        "anchor": [AX, AY],
        "emissive": [SHADES[2], PALE],
        "anims": {
            "spin": {"row": 0, "frames": N, "fps": 8,
                     "light": {"color": SHADES[2], "radius": 24, "intensity": 0.5, "offset": [0, -10]}},
            "pickup": {"row": 1, "frames": 4, "fps": 20, "durations": [3, 3, 3, 4]},
        },
        "notes": "Prism shard, style v2: a broken fragment (fractured right side) spinning 45 degrees per frame, "
                 "iris body (fx.prism) with a cyan lit facet, deep iris #38324a outline. Floats about 10 px above "
                 "its anchor with a 1 px bob; draw a small soft shadow at the anchor. pickup plays once: pale "
                 "flash, the edge-on sliver, a thin ring, a broken ring. Only the cyan facet and the pale flash glow.",
    }, os.path.join(SPRITES, "shard.json"))


if __name__ == "__main__":
    main()
