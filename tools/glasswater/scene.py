"""Test scene: assemble a 24x14 tile map from glasswater tile names, place props, render flat and mock-lit.

Run: uv run --no-project --with pillow --with numpy python3 tools/glasswater/scene.py [out_prefix]
"""
import json
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, NAMES, RGB, SPRITES, PREVIEWS, Scene, rgb, upscale, hero_cell, EMISSIVE  # noqa: E402
from autotile import layers  # noqa: E402

T = 16

# '#': raised rock, '~': shallow water, 'D': deep water, '.': floor
MAP = [
    "########################",
    "########################",
    "########################",
    "####.........#####......",
    "##...........#####......",
    "#............###........",
    "#.....~~~~~.............",
    "#....~~~~~~~~...........",
    "#...~~~DDD~~~~..........",
    "#...~~~DDD~~~...........",
    "#....~~~~~~~.......#####",
    "#.......~~.........#####",
    "#..................#####",
    "##.................#####",
]


def load_tiles():
    im = np.asarray(Image.open(os.path.join(SPRITES, "glasswater_tiles.png")).convert("RGBA"))
    man = json.load(open(os.path.join(SPRITES, "glasswater_tiles.json")))
    lut = {tuple(RGB[k]): k for k in range(len(NAMES))}
    out = {}
    for nm, (cx, cy) in man["tiles"].items():
        cell = im[cy * T:(cy + 1) * T, cx * T:(cx + 1) * T]
        c = np.full((T, T), -1, np.int32)
        for y in range(T):
            for x in range(T):
                if cell[y, x, 3]:
                    c[y, x] = lut[tuple(cell[y, x, :3])]
        out[nm] = c
    return out


def vertex_grid(MAP, chars):
    th, tw = len(MAP), len(MAP[0])
    v = np.zeros((th + 1, tw + 1), int)
    for y in range(th + 1):
        for x in range(tw + 1):
            # a vertex is set if all 4 tiles touching it are in chars (clamped)
            ok = True
            for dy in (-1, 0):
                for dx in (-1, 0):
                    ty, tx = min(max(y + dy, 0), th - 1), min(max(x + dx, 0), tw - 1)
                    if MAP[ty][tx] not in chars:
                        ok = False
            v[y, x] = ok
    return v


def any_vertex_grid(MAP, chars):
    th, tw = len(MAP), len(MAP[0])
    v = np.zeros((th + 1, tw + 1), int)
    for y in range(th + 1):
        for x in range(tw + 1):
            for dy in (-1, 0):
                for dx in (-1, 0):
                    ty, tx = min(max(y + dy, 0), th - 1), min(max(x + dx, 0), tw - 1)
                    if MAP[ty][tx] in chars:
                        v[y, x] = 1
    return v


def mk(v, x, y):
    return f"{v[y, x]}{v[y, x + 1]}{v[y + 1, x]}{v[y + 1, x + 1]}"


def hsh(x, y, s=0):
    h = (x * 73856093) ^ (y * 19349663) ^ (s * 83492791)
    h = (h ^ (h >> 13)) * 0x5bd1e995 & 0xffffffff
    return ((h ^ (h >> 15)) & 0xffff) / 65536.0


DETAILS = ["floor_pebble_a", "floor_pebble_b", "floor_pebbles_a", "floor_pebbles_b", "floor_crack_a",
           "floor_crack_b", "floor_crack_c", "floor_hollow_s", "floor_hollow_m"]


def build_ground(MAP, tiles):
    th, tw = len(MAP), len(MAP[0])
    g = np.zeros((th * T, tw * T), np.int32)
    for (tx, ty), L in layers(MAP).items():
        for nm in L:
            if nm not in tiles:
                print("missing", nm)
                continue
            src = tiles[nm]
            dst = g[ty * T:(ty + 1) * T, tx * T:(tx + 1) * T]
            dst[src >= 0] = src[src >= 0]
    return g


def load_sheet(name):
    p = os.path.join(SPRITES, name + ".png")
    if not os.path.exists(p):
        return None
    im = np.asarray(Image.open(p).convert("RGBA"))
    man = json.load(open(os.path.join(SPRITES, name + ".json")))
    return im, man


def sheet_cell(name, anim, frame=0):
    s = load_sheet(name)
    if s is None:
        return None
    im, man = s
    cw, chh = man["cell"]
    a = man["anims"][anim]
    row = a["row"]
    cell = im[row * chh:(row + 1) * chh, frame * cw:(frame + 1) * cw]
    anchor = a.get("anchor", man.get("anchor", [cw // 2, chh - 1]))
    light = a.get("light", man.get("light"))
    return cell, anchor, light, man.get("emissive", [])


def render(MAP, props, out_prefix, ambient=("#5a60c0", 0.38), hero=(150, 150), extra_lights=()):
    tiles = load_tiles()
    g = build_ground(MAP, tiles)
    H, W = g.shape
    sc = Scene(W, H, ambient=ambient[0], amb_k=ambient[1])
    sc.sprite(g, 0, 0)
    flat = np.zeros((H, W, 3), np.uint8)
    flat[:] = RGB[g]
    # y-sorted sprites (props + hero)
    items = []
    for (sheet, anim, x, y, *rest) in props:
        frame = rest[0] if rest else 0
        r = sheet_cell(sheet, anim, frame)
        if r is None:
            continue
        items.append((y, r, x))
    hc, ha = hero_cell()
    items.append((hero[1], ("hero", hc, ha), hero[0]))
    items.sort(key=lambda it: it[0])
    for (y, r, x) in items:
        if r[0] == "hero":
            kind, data = r[1]
            ax, ay = r[2]
            if kind == "idx":
                sc.sprite(data, x - ax, y - ay, emissive_names=["a4", "a3"])
                m = data >= 0
                flat[y - ay:y - ay + data.shape[0], x - ax:x - ax + data.shape[1]][m] = RGB[data[m]]
            else:
                rgba_sprite(sc, flat, data, x - ax, y - ay, ["#8ff8ff", "#5ff2ff", "#d8fbff", "#ffffff"])
            sc.add_light(x, y - 8, 64, "#9ff4ff", 0.8)
            continue
        cell, anchor, light, em = r
        rgba_sprite(sc, flat, cell, x - anchor[0], y - anchor[1], em)
        if light:
            off = light.get("offset", [0, 0])
            sc.add_light(x + off[0], y + off[1], light.get("radius", 32), light["color"], light.get("intensity", 0.8))
    for (x, y, r, col, k) in extra_lights:
        sc.add_light(x, y, r, col, k)
    sc.render()
    Image.fromarray(flat, "RGB").save(out_prefix + "_flat_1x.png")
    upscale(Image.fromarray(flat, "RGB"), 3).save(out_prefix + "_flat_3x.png")
    sc._base.save(out_prefix + "_lit_1x.png")
    sc.composite(3).save(out_prefix + "_lit_3x.png")


def rgba_sprite(sc, flat, cell, x0, y0, em_hex):
    em = {tuple(rgb(h)) for h in em_hex}
    h, w = cell.shape[:2]
    for j in range(h):
        for i in range(w):
            if cell[j, i, 3] < 128:
                continue
            yy, xx = y0 + j, x0 + i
            if not (0 <= yy < sc.h and 0 <= xx < sc.w):
                continue
            col = tuple(int(v) for v in cell[j, i, :3])
            flat[yy, xx] = col
            f = np.array(col, np.float32) / 255.0
            if col in em:
                sc.albedo[yy, xx] = 0
                sc.emit[yy, xx] = f
            else:
                sc.albedo[yy, xx] = f
                sc.emit[yy, xx] = 0


if __name__ == "__main__":
    pre = sys.argv[1] if len(sys.argv) > 1 else os.path.join(PREVIEWS, "_wip_glasswater_scene")
    render(MAP, [], pre, extra_lights=[(60, 70, 56, "#4fe0e8", 0.8), (330, 120, 48, "#a86cf0", 0.8)])
