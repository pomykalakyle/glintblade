"""Reference autotiler for glasswater_tiles v2 (port this to the zone builder).

Input: a grid of chars per tile: '#' raised rock (Tile::Wall), '~' shallow water, 'D' deep water, '.' floor.
Output: per tile a list of tile names, painted bottom to top (later layers skip transparent pixels).
"""
import numpy as np

FH = 2          # face height in tiles (top + bot); taller walls insert mid tiles
COLS = 10       # face texture columns (160 px panel)


def hsh(x, y, s=0):
    h = (x * 73856093) ^ (y * 19349663) ^ (s * 83492791)
    h = (h ^ (h >> 13)) * 0x5bd1e995 & 0xffffffff
    return ((h ^ (h >> 15)) & 0xffff) / 65536.0


def classify(rock):
    th, tw = rock.shape

    def R(x, y):
        if x < 0 or x >= tw or y < 0 or y >= th:
            return True
        return bool(rock[y, x])

    kind, depth = {}, {}
    for x in range(tw):
        for y in range(th):
            if not R(x, y):
                continue
            d = 0
            while y + d + 1 < th and R(x, y + d + 1):
                d += 1
            depth[(x, y)] = d
            if y + d + 1 >= th:
                kind[(x, y)] = "top"
            elif d == 0:
                kind[(x, y)] = "bot"
            elif d < FH - 1:
                kind[(x, y)] = "mid"
            elif d == FH - 1:
                kind[(x, y)] = "facetop"
            else:
                kind[(x, y)] = "top"
    return kind, depth, R


def vgrid(MAP, chars, any_=False):
    th, tw = len(MAP), len(MAP[0])
    v = np.zeros((th + 1, tw + 1), int)
    for y in range(th + 1):
        for x in range(tw + 1):
            hits = []
            for dy in (-1, 0):
                for dx in (-1, 0):
                    ty, tx = min(max(y + dy, 0), th - 1), min(max(x + dx, 0), tw - 1)
                    hits.append(MAP[ty][tx] in chars)
            v[y, x] = any(hits) if any_ else all(hits)
    return v


def bits(v, x, y):
    return f"{v[y, x]}{v[y, x + 1]}{v[y + 1, x]}{v[y + 1, x + 1]}"


DECALS = ["decal_pebbles_a", "decal_pebbles_b", "decal_stone", "decal_crack_a", "decal_crack_b"]


def layers(MAP):
    th, tw = len(MAP), len(MAP[0])
    rock = np.array([[ch == "#" for ch in r] for r in MAP])
    kind, depth, R = classify(rock)

    def K(x, y):
        if x < 0 or y < 0 or x >= tw or y >= th:
            return "top"
        return kind.get((x, y))

    water = vgrid(MAP, "~D")
    deep = vgrid(MAP, "D")
    out = {}
    for ty in range(th):
        for tx in range(tw):
            L = [f"floor_{tx % 4}_{ty % 4}"]
            ch = MAP[ty][tx]
            k = kind.get((tx, ty))
            if k in ("bot", "mid", "facetop"):
                yb = ty + depth[(tx, ty)]
                v = (tx + int(hsh(0, yb, 11) * COLS)) % COLS
                part = {"bot": "bot", "mid": "mid", "facetop": "top"}[k]
                side = "_l" if not R(tx - 1, ty) else ("_r" if not R(tx + 1, ty) else "")
                nm = f"cliff_face_{part}{side}_{v}"
                if not side and v == 5 and hsh(tx, yb, 12) < 0.4:
                    nm += "_vein"
                if not side and v == 7 and hsh(tx, yb, 13) < 0.5:
                    nm += "_crack"
                L.append(nm)
            elif k == "top":
                n = K(tx, ty - 1) != "top"
                w = K(tx - 1, ty) != "top"
                e = K(tx + 1, ty) != "top"
                nw = not n and not w and K(tx - 1, ty - 1) != "top"
                ne = not n and not e and K(tx + 1, ty - 1) != "top"
                sw = not w and K(tx - 1, ty + 1) not in ("top", "facetop")
                se = not e and K(tx + 1, ty + 1) not in ("top", "facetop")
                s = "cliff_top"
                rims = ("n" if n else "") + ("w" if w else "") + ("e" if e else "")
                if rims:
                    s += "_" + rims
                ins = "".join(kk for kk, f in zip(("nw", "ne", "sw", "se"), (nw, ne, sw, se)) if f)
                if ins:
                    s += "_in" + ins
                r = hsh(tx, ty, 14)
                if not rims and not ins and r < 0.12:
                    s += f"_{1 + int(r / 0.04)}"
                elif rims in ("n", "w", "e") and not ins and r < 0.5:
                    s += "_v1"
                L.append(s)
            elif deep[ty:ty + 2, tx:tx + 2].any():
                L = [f"deep_{bits(deep, tx, ty)}"]
            elif water[ty:ty + 2, tx:tx + 2].any():
                b = bits(water, tx, ty)
                if b == "1111":
                    r = hsh(tx, ty, 3)
                    L = ["water_1" if r < 0.1 else "water_2" if r < 0.18 else "water_0"]
                else:
                    L = [L[0], f"shore_{b}"]
            else:
                sn, sw_ = R(tx, ty - 1), R(tx - 1, ty)
                if sn and sw_:
                    L.append("floor_shade_nw")
                elif sn:
                    L.append(f"floor_shade_n_{int(hsh(tx, ty, 4) * 3)}")
                elif sw_:
                    L.append(f"floor_shade_w_{int(hsh(tx, ty, 5) * 2)}")
                elif R(tx - 1, ty - 1):
                    L.append("floor_shade_c")
                near_water = any(0 <= ty + dy < th and 0 <= tx + dx < tw and MAP[ty + dy][tx + dx] in "~D"
                                 for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)))
                if near_water and len(L) == 1 and hsh(tx, ty, 10) < 0.2:
                    L.append("decal_moss")
                elif not near_water and hsh(tx, ty, 8) < 0.10:
                    L.append(DECALS[int(hsh(tx, ty, 9) * len(DECALS))])
            out[(tx, ty)] = L
    return out
