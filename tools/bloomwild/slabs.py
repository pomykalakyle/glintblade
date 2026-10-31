"""White flagstone floor for Bloomwild: irregular slabs on an 8 px grid, authored as multi-tile
blocks (exported like summit's "blocks") plus 1x1 fillers and a few 1x1 accents.

Edge rule, shared by every block and filler so any packing is seamless: each slab owns the
joint on its TOP row and LEFT column, and its shaded bevel on its own bottom row and right
column. So a joint always reads as: shaded bevel (slab above/left), dark gap, lit slab below/right.

Light from the upper left. Slab styles: plain (s4), lit (s4 with a partial s5 lip on top),
worn (s3 body, s2 bevel), sunk (s3 body, the higher neighbors cast s2 shadow on its top/left).
Deep gaps (joint junctions) are s1. Details that make physical sense only: chipped corners with
grass growing in them, a few cracks, moss growing along joints.
"""
from pix import *

U = 8
SIZES = [((2, 2), 3), ((3, 2), 4), ((2, 3), 3), ((4, 2), 3), ((2, 4), 1), ((3, 3), 3), ((4, 3), 2),
         ((3, 4), 1), ((4, 4), 1), ((2, 1), 1), ((1, 2), 1)]


def pack(wu, hu, seed):
    """Greedy irregular packing (row-major, random fitting size). Retries until no joint runs
    straight across the whole block."""
    for attempt in range(200):
        grid = [[-1] * wu for _ in range(hu)]
        rects = []
        for y in range(hu):
            for x in range(wu):
                if grid[y][x] >= 0:
                    continue
                cands = []
                for (w, h), wt in SIZES:
                    if x + w <= wu and y + h <= hu and all(grid[y + j][x + i] < 0 for j in range(h) for i in range(w)):
                        cands.append(((w, h), wt))
                if not cands:
                    cands = [((1, 1), 1)]
                tot = sum(wt for _, wt in cands)
                r = hash01(seed, attempt, x, y) * tot
                for (w, h), wt in cands:
                    r -= wt
                    if r <= 0:
                        break
                for j in range(h):
                    for i in range(w):
                        grid[y + j][x + i] = len(rects)
                rects.append((x, y, w, h))
        straight = False
        for yy in range(1, hu):
            if all(grid[yy][x] != grid[yy - 1][x] for x in range(wu)):
                straight = True
        for xx in range(1, wu):
            if all(grid[y][xx] != grid[y][xx - 1] for y in range(hu)):
                straight = True
        if not straight and not any(w * h == 1 for _, _, w, h in rects):
            return rects
    return rects


def style_of(seed, i):
    """Style of on the native pixel grid."""
    r = hash01(seed, i, 1)
    return "sunk" if r < 0.05 else "worn" if r < 0.14 else "lit" if r < 0.42 else "plain"


def render(wu, hu, rects, seed, styles=None, chips=0.14, cracks=0.12, moss=0.10):
    """Render on the native pixel grid."""
    W, H = wu * U, hu * U
    img = new(W, H)
    joint = [[False] * W for _ in range(H)]
    for i, (x, y, w, h) in enumerate(rects):
        px, py, pw, ph = x * U, y * U, w * U, h * U
        st = styles[i] if styles else style_of(seed, i)
        body = "s3" if st in ("sunk", "worn") else "s4"
        bevel = "s2" if st == "worn" else "s3"
        for yy in range(py, py + ph):
            for xx in range(px, px + pw):
                if yy == py or xx == px:
                    put(img, xx, yy, "s2")
                    joint[yy][xx] = True
                elif st != "sunk" and (yy == py + ph - 1 or xx == px + pw - 1):
                    put(img, xx, yy, bevel)
                else:
                    put(img, xx, yy, body)
        if st == "lit":
            run = int((pw - 3) * (0.45 + 0.4 * hash01(seed, i, 2)))
            for xx in range(px + 1, px + 1 + run):
                put(img, xx, py + 1, "s5")
            put(img, px + 1, py + 2, "s5")
        if st == "sunk":
            for xx in range(px + 1, px + pw):
                put(img, xx, py + 1, "s2")
            for yy in range(py + 1, py + ph):
                put(img, px + 1, yy, "s2")
        # chipped corner with grass in it
        if hash01(seed, i, 3) < chips and pw >= 16 and ph >= 16:
            if hash01(seed, i, 4) < 0.5:   # bottom-right
                cx, cy = px + pw - 1, py + ph - 1
                grid(["..k", ".kg", "kgg"], {"k": "g1", "g": "g2"}, img, cx - 2, cy - 2)
            else:                          # top-left (joins the joint)
                grid(["kgg", "gk.", "k.."], {"k": "g1", "g": "g2"}, img, px, py)
                put(img, px + 1, py + 1, "g3")
        # crack: stepped line from the top edge down to the left or bottom edge
        if hash01(seed, i, 5) < cracks and pw >= 24 and ph >= 16:
            x0 = px + 4 + int(hash01(seed, i, 6) * (pw - 10))
            yy, xx = py + 1, x0
            while yy < py + ph - 2 and xx > px + 1:
                put(img, xx, yy, "s2")
                if hash01(seed, i, yy, 7) < 0.55:
                    xx -= 1
                else:
                    yy += 1
                    put(img, xx + 1, yy - 1, "s5") if yy - 1 > py + 1 else None
    # deep gaps at joint junctions
    for yy in range(H):
        for xx in range(W):
            if not joint[yy][xx]:
                continue
            n = sum(1 for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
                    if 0 <= xx + dx < W and 0 <= yy + dy < H and joint[yy + dy][xx + dx])
            if n >= 3 and is_col(img, xx, yy, "s2"):
                put(img, xx, yy, "s1")
    # moss growing along a horizontal joint
    for i, (x, y, w, h) in enumerate(rects):
        if hash01(seed, i, 8) < moss and w >= 2 and y > 0:
            px, py = x * U + 2 + int(hash01(seed, i, 9) * (w * U - 9)), y * U
            grid(["..G..", ".GgG.", "gkkkg", ".k.k."], {"G": "g3", "g": "g2", "k": "g1"}, img, px, py - 2)
    return img


def block(name, wu, hu, seed, **kw):
    """Block on the native pixel grid."""
    rects = pack(wu, hu, seed)
    return render(wu, hu, rects, seed, **kw)


# ---------------------------------------------------------------- 1x1 fillers and accents
FILLERS = [
    ([(0, 0, 2, 2)], ["plain"]),
    ([(0, 0, 2, 2)], ["lit"]),
    ([(0, 0, 2, 1), (0, 1, 2, 1)], ["lit", "plain"]),
    ([(0, 0, 1, 2), (1, 0, 1, 2)], ["plain", "lit"]),
    ([(0, 0, 2, 2)], ["worn"]),
    ([(0, 0, 2, 1), (0, 1, 2, 1)], ["plain", "worn"]),
    ([(0, 0, 1, 2), (1, 0, 1, 2)], ["worn", "plain"]),
    ([(0, 0, 2, 2)], ["sunk"]),
]


def filler(v):
    """Filler on the native pixel grid."""
    rects, styles = FILLERS[v]
    return render(2, 2, rects, 100 + v, styles=styles, chips=0, cracks=0, moss=0)


HOLE_A = [".xxx..",
          "xxxxx.",
          ".xxxxx",
          "..xx.."]
HOLE_B = ["..xx...",
          ".xxxxx.",
          "xxxxxx.",
          ".xxxxxx",
          "...xx.."]


def hole(img, mask, ox, oy, sprout=False):
    """Hole on the native pixel grid."""
    hm = {(ox + x, oy + y) for y, row in enumerate(mask) for x, ch in enumerate(row) if ch == "x"}
    for (x, y) in hm:
        if (x, y - 1) not in hm:
            put(img, x, y, "s2")          # the hole's north wall, facing south (toward us), shaded
        elif (x, y - 2) not in hm or (x - 1, y) not in hm:
            put(img, x, y, "e0")          # shadow at the foot of the wall
        else:
            put(img, x, y, "e1")          # soil
    for (x, y) in hm:                     # lit broken edge on the far (bottom/right) side
        for (dx, dy) in ((1, 0), (0, 1)):
            q = (x + dx, y + dy)
            if q not in hm and 0 <= q[0] < 16 and 0 <= q[1] < 16:
                put(img, *q, "s5")
    if sprout:
        ys = sorted({y for _, y in hm})
        by = ys[-1]
        sx = ox + 3
        put(img, sx, by, "g2"); put(img, sx - 1, by - 1, "g3"); put(img, sx + 1, by - 1, "g3"); put(img, sx, by - 1, "g2")
    return img


def accent(v):
    """8 cracked slab, 9 moss along a joint, 10 small broken hole, 11 broken hole with a sprout."""
    if v == 8:
        img = render(2, 2, [(0, 0, 2, 2)], 0, styles=["plain"], chips=0, cracks=0, moss=0)
        for (x, y) in [(11, 1), (11, 2), (10, 3), (10, 4), (9, 5), (8, 6), (8, 7), (7, 8), (7, 9), (6, 10), (5, 11)]:
            put(img, x, y, "s2")
        put(img, 12, 2, "s5"); put(img, 11, 4, "s5"); put(img, 9, 7, "s5")
        return img
    if v == 9:
        img = render(2, 2, [(0, 0, 2, 1), (0, 1, 2, 1)], 1, styles=["plain", "lit"], chips=0, cracks=0, moss=0)
        grid(["...G.....", "..GgG.G..", ".gkkkgGg.", "gkk.kkkkg", ".k...k.k."],
             {"G": "g3", "g": "g2", "k": "g1"}, img, 3, 6)
        return img
    if v == 10:
        img = render(2, 2, [(0, 0, 2, 2)], 2, styles=["worn"], chips=0, cracks=0, moss=0)
        return hole(img, HOLE_A, 6, 6)
    img = render(2, 2, [(0, 0, 2, 2)], 3, styles=["plain"], chips=0, cracks=0, moss=0)
    return hole(img, HOLE_B, 4, 5, sprout=True)


# ---------------------------------------------------------------- multi-tile blocks
BLOCK_SPECS = [  # name, width and height in tiles, seed
    ("slab_a", 3, 3, 11), ("slab_b", 3, 3, 23), ("slab_c", 3, 3, 37),
    ("slab_d", 3, 2, 41), ("slab_e", 3, 2, 53),
    ("slab_f", 2, 3, 61), ("slab_g", 2, 3, 71),
    ("slab_h", 2, 2, 83), ("slab_i", 2, 2, 97),
]


def blocks():
    """Blocks on the native pixel grid."""
    out = {}
    for name, tw, th, seed in BLOCK_SPECS:
        out[name] = block(name, tw * 2, th * 2, seed)
    return out


def floor_preview(wt, ht, seed=5):
    """Fill wt x ht tiles greedily with random blocks (largest first), fillers for leftovers."""
    bl = blocks()
    W, H = wt * 16, ht * 16
    img = new(W, H)
    used = [[False] * wt for _ in range(ht)]
    specs = {n: (tw, th) for n, tw, th, _ in BLOCK_SPECS}
    for ty in range(ht):
        for tx in range(wt):
            if used[ty][tx]:
                continue
            names = sorted(specs, key=lambda n: hash01(seed, tx, ty, list(specs).index(n)))
            placed = False
            for n in names:
                tw, th = specs[n]
                if tx + tw <= wt and ty + th <= ht and not any(used[ty + j][tx + i] for j in range(th) for i in range(tw)):
                    paste(img, bl[n], tx * 16, ty * 16)
                    for j in range(th):
                        for i in range(tw):
                            used[ty + j][tx + i] = True
                    placed = True
                    break
            if not placed:
                v = int(hash01(seed, tx, ty, 9) * 12)
                paste(img, filler(v) if v < 8 else accent(v), tx * 16, ty * 16)
                used[ty][tx] = True
    return img
