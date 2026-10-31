"""Glasswater floor and water tiles, v2.

Base layer: floor_<x>_<y> (x, y in 0..3) = a seamless 64x64 macro block of worn stone, picked by (tx % 4, ty % 4).
Overlays (transparent -1 where the floor shows): floor_shade_*, wet_*, decal_*.
Opaque: shore_*, water_*, deep_*.
"""
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, ascii_grid  # noqa: E402
from corner import MASKS, mask_px, T  # noqa: E402
from trans import destair, shifted, shore_px  # noqa: E402

N = 64


# ---------------------------------------------------------------- macro floor

def periodic_noise(seed, kmin, kmax, power=1.3, sx=1.0):
    rng = np.random.default_rng(seed)
    F = np.zeros((N, N), complex)
    for ky in range(-kmax, kmax + 1):
        for kx in range(-kmax, kmax + 1):
            r = math.hypot(kx * sx, ky)
            if kmin <= r <= kmax + 0.2:
                F[ky % N, kx % N] = (rng.normal() + 1j * rng.normal()) / r ** power
    n = np.real(np.fft.ifft2(F))
    return (n - n.min()) / (n.max() - n.min())


def components(mask):
    """4-connected components with wrap-around. Returns list of index arrays."""
    seen = np.zeros_like(mask, bool)
    out = []
    for y0, x0 in zip(*np.where(mask)):
        if seen[y0, x0]:
            continue
        stack = [(y0, x0)]
        seen[y0, x0] = True
        comp = []
        while stack:
            y, x = stack.pop()
            comp.append((y, x))
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                yy, xx = (y + dy) % N, (x + dx) % N
                if mask[yy, xx] and not seen[yy, xx]:
                    seen[yy, xx] = True
                    stack.append((yy, xx))
        out.append(comp)
    return out


def clean(mask, min_size):
    for _ in range(3):
        opp = np.zeros(mask.shape, np.int32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            opp += np.roll(np.roll(mask, dy, 0), dx, 1) != mask
        mask = np.where(opp >= 3, ~mask, mask)
    for comp in components(mask):
        if len(comp) < min_size:
            for (y, x) in comp:
                mask[y, x] = False
    return mask


def R(a, dy, dx):
    return np.roll(np.roll(a, dy, 0), dx, 1)


def macro_floor():
    low = periodic_noise(31, 2, 6, power=1.2, sx=2.5)
    plate_n = periodic_noise(23, 2, 5, power=1.1, sx=2.0)
    dep = clean(low < np.quantile(low, 0.075), 24)          # worn hollows (darker)
    plate = clean((plate_n > np.quantile(plate_n, 0.9)) & ~dep, 24)   # raised worn plates
    c = np.full((N, N), I["r3"], np.int32)
    c[dep] = I["r2"]
    # plates: lit rim on their upper edge (horizontal runs only), cast shadow below/right outside them
    rim = plate & ~R(plate, 1, 0)
    c[rim] = I["r4"]
    shadow = ~plate & ~dep & (R(plate, 1, 0) | R(plate, 0, 1)) & ~R(plate, 1, 1) | (~plate & ~dep & R(plate, 1, 0))
    c[shadow] = I["r2"]
    # hollows: the lower-right inner wall catches light: short r4 runs where the hollow ends going down
    far = dep & ~R(dep, -1, 0) & R(dep, 1, 0)
    runs = far & (R(far, 0, 1) | R(far, 0, -1))
    c[runs] = I["r4"]
    # kill single-pixel islands of any colour
    for _ in range(2):
        for v in (I["r4"], I["r2"]):
            m = c == v
            nb = sum((R(m, dy, dx)).astype(int) for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)))
            c[m & (nb == 0)] = I["r3"]
    return c


# ---------------------------------------------------------------- overlays: decals

DK = {"5": "r5", "4": "r4", "3": "r3", "2": "r2", "1": "r1",
      "A": "a3", "a": "a2", "b": "a1", "c": "a0",
      "V": "v3", "v": "v2", "w": "v1", "x": "v0",
      "m": "m0", "M": "m1", "n": "r1"}

DECALS = {
    "decal_pebbles_a": [
        "................",
        "................",
        "................",
        "................",
        ".....443........",
        "....44332.......",
        "....43322.......",
        ".....2221.......",
        "......11..43....",
        ".........4332...",
        "...443...3221...",
        "..44322...11....",
        "..43221.........",
        "...2211.........",
        "....11..........",
        "................",
    ],
    "decal_pebbles_b": [
        "................",
        "................",
        "................",
        ".......43.......",
        "......4432......",
        "......3321......",
        ".......11.......",
        "...43.......443.",
        "..4432.....4332.",
        "..3321.....3221.",
        "...11.......11..",
        ".......443......",
        "......43321.....",
        "......32211.....",
        ".......111......",
        "................",
    ],
    "decal_stone": [
        "................",
        "................",
        "................",
        "................",
        ".....4443.......",
        "....445433......",
        "...44433332.....",
        "...433333322....",
        "...333332221....",
        "....2222211.....",
        ".....11111......",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
    "decal_crack_a": [
        "................",
        "................",
        ".11.............",
        ".1111...........",
        "..1111..........",
        "....1111........",
        "......111.......",
        "........11......",
        ".........11.....",
        "..........11....",
        "........22..22..",
        "......22......2.",
        "................",
        "................",
        "................",
        "................",
    ],
    "decal_crack_b": [
        "................",
        "................",
        "................",
        "................",
        "................",
        "..111...........",
        "..11111.........",
        "....111111......",
        ".......11111....",
        "..........1122..",
        "........22......",
        ".......22.......",
        "......22........",
        "................",
        "................",
        "................",
    ],
    # broken crystal pieces (only beside a crystal cluster). Only the top pixel of the biggest shard glows.
    "decal_shards_aqua": [
        "................",
        "................",
        "................",
        "................",
        "......A.........",
        ".....ab.........",
        ".....ab..a......",
        "....cbb.ab......",
        "....111.cb1.....",
        "..ab.....11.....",
        ".cbb1...........",
        "..111....ab.....",
        ".........cb1....",
        "..........11....",
        "................",
        "................",
    ],
    "decal_shards_violet": [
        "................",
        "................",
        "................",
        "................",
        "................",
        ".......V........",
        "......vw........",
        "...v..vw........",
        "..vw.xww........",
        "..xw1111..vw....",
        "...11.....xw1...",
        "...........11...",
        "................",
        "................",
        "................",
        "................",
    ],
    # quiet moss by the water (not emissive)
    "decal_moss": [
        "................",
        "................",
        "................",
        "................",
        "................",
        ".......mm.......",
        "....mmmMm.......",
        "...mmMMmmm......",
        "..nmmmMmnn......",
        "...nnmmn........",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
}


# ---------------------------------------------------------------- overlays: cast shade from rock

SHADE_N = [
    [5, 5, 5, 6, 6, 6, 6, 5, 5, 4, 4, 4, 5, 5, 5, 5],
    [5, 5, 4, 4, 4, 4, 5, 5, 6, 6, 6, 5, 5, 5, 5, 5],
    [5, 5, 5, 5, 6, 6, 5, 5, 5, 5, 4, 4, 4, 4, 5, 5],
]
SHADE_W = [
    [5, 5, 5, 4, 4, 4, 4, 5, 5, 6, 6, 6, 6, 5, 5, 5],
    [5, 5, 6, 6, 6, 5, 5, 5, 4, 4, 4, 5, 5, 5, 5, 5],
]


def shade_tiles():
    out = {}
    for i, prof in enumerate(SHADE_N):
        c = np.full((T, T), -1, np.int32)
        for x in range(T):
            c[0:prof[x], x] = I["r2"]
        out[f"floor_shade_n_{i}"] = c
    for i, prof in enumerate(SHADE_W):
        c = np.full((T, T), -1, np.int32)
        for y in range(T):
            c[y, 0:prof[y]] = I["r2"]
        out[f"floor_shade_w_{i}"] = c
    c = np.full((T, T), -1, np.int32)
    for x in range(T):
        c[0:SHADE_N[0][x], x] = I["r2"]
    for y in range(T):
        c[y, 0:SHADE_W[0][y]] = I["r2"]
    out["floor_shade_nw"] = c
    c = np.full((T, T), -1, np.int32)
    for y in range(5):
        for x in range(5):
            if x + y < 8:
                c[y, x] = I["r2"]
    out["floor_shade_c"] = c
    return out


# ---------------------------------------------------------------- shore, wet, deep, water

def glint(c, x, y, mask):
    """A wet-stone glint lit from the upper left: r5 top pixel, r4 run under it."""
    pts = [(x, y, "r5"), (x, y + 1, "r4"), (x + 1, y + 1, "r4"), (x + 2, y + 1, "r4")]
    if all(mask[yy, xx] for xx, yy, _ in pts):
        for xx, yy, col in pts:
            c[yy, xx] = I[col]
        return True
    return False


def dist_to(w):
    """4-neighbour distance (in px) of every pixel to the nearest True pixel of w (capped)."""
    d = np.where(w, 0, 99)
    for k in range(1, 8):
        near = np.zeros_like(w)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            near |= shifted(d == k - 1, dy, dx)
        d = np.where((d == 99) & near, k, d)
    return d


BUMP_X = [0, 1, 1, 2, 2, 1, 0, 0, 1, 2, 1, 1, 0, 0, 1, 0]
BUMP_Y = [0, 0, 1, 1, 0, 0, 1, 2, 1, 0, 0, 1, 1, 0, 0, 0]


def shore_tile(mask, seed=0):
    """Opaque water + a narrow wet-stone band (3-6 px, irregular, seam-safe) on the land; drier land is
    transparent so the macro floor shows. Glints sit 1-3 px from the waterline; a puddle further out."""
    M = 8
    w, m = mask_px(mask, wob_seed=seed, wob_amp=0.07, margin=M)
    w = destair(w)
    c = shore_px(w, m)
    land = ~w
    yy, xx = np.mgrid[0:w.shape[0], 0:w.shape[1]]
    wy, wx = np.where(w)
    if len(wy):
        de = np.sqrt(((yy[..., None] - wy) ** 2 + (xx[..., None] - wx) ** 2).min(axis=2))
    else:
        de = np.full(w.shape, 99.0)
    d = np.ceil(de).astype(int)
    bump = np.array(BUMP_X)[(xx - m) % T] + np.array(BUMP_Y)[(yy - m) % T]
    dry = destair(land & (de > 4.0 + bump * 0.6))
    dry &= land
    c[dry] = -1
    wetband = land & ~dry
    rng = np.random.default_rng(int(mask, 2) * 13 + 5)
    ok = wetband & (d >= 1) & (d <= 3)
    cand = [(y, x) for y, x in zip(*np.where(ok)) if m + 1 <= x <= m + T - 4 and m + 1 <= y <= m + T - 3]
    rng.shuffle(cand)
    if int(mask, 2) % 3 != 0:
        for (y, x) in cand:
            if glint(c, x, y, wetband & (c == I["r2"]) | (c == I["r3"]) & wetband):
                break
    # one puddle notch on the dry side of some tiles
    if int(mask, 2) % 2 == 1:
        pc = [(y, x) for y, x in zip(*np.where(dry & (d >= 6) & (d <= 8)))
              if m + 1 <= x <= m + T - 6 and m + 1 <= y <= m + T - 3]
        rng.shuffle(pc)
        for (y, x) in pc:
            if dry[y - 1:y + 3, x - 1:x + 6].all():
                c[y, x:x + 3] = I["r1"]
                c[y + 1, x - 1:x + 4] = I["r2"]
                break
    return c[m:m + T, m:m + T]


def wet_tile(mask, glints=0, seed=0):
    """Overlay: wet stone (r2) where mask, transparent elsewhere. Wet film edge r1 on its north side.
    Dry-stone intrusions come from a strong wobble; puddle notches sit just outside the edge."""
    w, m = mask_px(mask, thr=0.6, wob_seed=seed + 50, wob_amp=0.16)
    w = destair(w)
    c = np.full(w.shape, -1, np.int32)
    c[w] = I["r2"]
    c[w & shifted(~w, 1, 0)] = I["r1"]
    if 0 < mask.count("1") < 4:
        # one puddle notch: a small detached pool 2 px outside the edge
        d = dist_to(w)
        rng = np.random.default_rng(int(mask, 2) * 7 + seed)
        cand = [(y, x) for y, x in zip(*np.where(~w & (d == 3))) if m + 1 <= x <= m + T - 6 and m + 1 <= y <= m + T - 3]
        rng.shuffle(cand)
        for (y, x) in cand:
            box = ~w[y - 1:y + 3, x - 1:x + 6]
            if box.all():
                c[y, x:x + 3] = I["r1"]
                c[y + 1, x:x + 4] = I["r2"]
                break
    rng = np.random.default_rng(seed + 3)
    for _ in range(glints):
        for _try in range(20):
            x, y = int(rng.integers(m + 2, m + T - 5)), int(rng.integers(m + 2, m + T - 4))
            if glint(c, x, y, w & ~shifted(~w, 1, 0)):
                break
    return c[m:m + T, m:m + T]


def deep_tile(mask):
    d, m = mask_px(mask, wob_seed=91, wob_amp=0.07)
    d = destair(d)
    c = np.full(d.shape, I["q2"], np.int32)
    c[d] = I["q1"]
    # the drop-off: the deep edge facing north (shallow ledge above) gets a q0 line, like the shore's bank
    c[d & shifted(~d, 1, 0)] = I["q0"]
    return c[m:m + T, m:m + T]


STONES = {
    "water_1": [
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        ".....3333.......",
        "....3111111.....",
        "....11111110....",
        ".....000000.....",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
    "water_2": [
        "................",
        "................",
        "................",
        "..333...........",
        ".311111.........",
        ".1111110........",
        "..00000.........",
        "................",
        "................",
        ".........3333...",
        "........311111..",
        "........1111110.",
        ".........00000..",
        "................",
        "................",
        "................",
    ],
}


def water_tiles():
    out = {"water_0": np.full((T, T), I["q2"], np.int32)}
    for nm, rows in STONES.items():
        c = np.full((T, T), I["q2"], np.int32)
        g = ascii_grid(rows, {"3": "q3", "1": "q1", "0": "q0"})
        c[g >= 0] = g[g >= 0]
        out[nm] = c
    return out


def ground_tiles():
    out = {}
    mf = macro_floor()
    for ty in range(4):
        for tx in range(4):
            out[f"floor_{tx}_{ty}"] = mf[ty * T:(ty + 1) * T, tx * T:(tx + 1) * T].copy()
    for nm, rows in DECALS.items():
        out[nm] = ascii_grid(rows, DK)
    out.update(shade_tiles())
    for mk in MASKS:
        out[f"shore_{mk}"] = shore_tile(mk)
        out[f"deep_{mk}"] = deep_tile(mk)
    out.update(water_tiles())
    return out


if __name__ == "__main__":
    from px import on_bg, upscale
    mf = macro_floor()
    big = np.tile(mf, (2, 3))
    upscale(on_bg(big, "#000000", pad=0), 3).save("/tmp/gw_macro.png")
    t = ground_tiles()
    names = [k for k in t if k.startswith("decal") or k.startswith("floor_shade") or k.startswith("water")]
    row = np.full((T + 2, len(names) * (T + 2)), I["r3"], np.int32)
    for i, k in enumerate(names):
        g = t[k]
        cell = np.where(g >= 0, g, I["r3"])
        row[1:T + 1, i * (T + 2) + 1:i * (T + 2) + 1 + T] = cell
    upscale(on_bg(row, "#000000", pad=0), 4).save("/tmp/gw_decals.png")
