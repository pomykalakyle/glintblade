# Bootstrap the Glasswater Hollow tile map from shapes, print ASCII, render a preview.
# The printed ASCII is then hand-tuned inside src/regions/glasswater.rs (the source of truth).
# usage: layout.py out.png > map.txt
import math, sys
from PIL import Image

W, H = 92, 52
g = [['#'] * W for _ in range(H)]

def h(x, y, s=0):
    n = (x * 374761393 + y * 668265263 + s * 2147483647) & 0xffffffff
    n = (n ^ (n >> 13)) * 1274126177 & 0xffffffff
    return ((n ^ (n >> 16)) & 0xffff) / 65535.0

def vnoise(x, y, s):
    xi, yi = math.floor(x), math.floor(y)
    fx, fy = x - xi, y - yi
    fx = fx * fx * (3 - 2 * fx); fy = fy * fy * (3 - 2 * fy)
    a, b, c, d = h(xi, yi, s), h(xi + 1, yi, s), h(xi, yi + 1, s), h(xi + 1, yi + 1, s)
    return (a + (b - a) * fx) + ((c + (d - c) * fx) - (a + (b - a) * fx)) * fy

def blob(cx, cy, rx, ry, ch, jit=0.18, seed=1, only=None):
    for y in range(H):
        for x in range(W):
            dx, dy = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
            d = math.sqrt(dx * dx + dy * dy)
            n = vnoise(x * 0.35, y * 0.35, seed) - 0.5
            if d < 1.0 + n * jit * 2:
                if only is None or g[y][x] in only:
                    g[y][x] = ch

def path(pts, wdt, ch, only=None):
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = int(max(abs(x1 - x0), abs(y1 - y0)) * 3) + 1
        for i in range(n + 1):
            t = i / n
            px, py = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            for y in range(int(py - wdt), int(py + wdt) + 2):
                for x in range(int(px - wdt), int(px + wdt) + 2):
                    if 0 <= x < W and 0 <= y < H and (x + 0.5 - px) ** 2 + (y + 0.5 - py) ** 2 <= wdt * wdt:
                        if only is None or g[y][x] in only:
                            g[y][x] = ch

def rect(x0, y0, x1, y1, ch, only=None):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if only is None or g[y][x] in only:
                g[y][x] = ch

F = '.'
# Start chamber (west) and its entrance tunnel
blob(7.5, 26, 6.2, 5.6, F, seed=3)
rect(0, 25, 3, 27, F)
# Lake chamber
blob(31, 25.5, 13.5, 12.5, F, seed=5)
path([(12, 26.5), (19, 26.5)], 1.5, F)
# Balcony from the lake chamber's SE looking into the rift
path([(42, 31), (49, 31.5)], 1.6, F)
# Gallery (north)
blob(36, 5.8, 14.5, 4.2, F, seed=7)
path([(38, 9), (38, 14)], 1.9, F)
# Rift west ledge path from the gallery's east end down to the bridge
path([(46, 6), (47, 10), (47, 16)], 1.8, F)
# Grove (north-east)
blob(75.5, 15, 14.5, 12.5, F, seed=9)
# Heart (south-east) and the path from the grove
blob(73, 43.5, 15.5, 7, F, seed=11)
path([(74, 26), (73, 37)], 2.4, F)
# Rift void (N-S), north end at row 9
path([(52.5, 10.2), (53.5, 14), (54, 20), (53, 26), (54, 32), (53.5, 40), (54, 52)], 3.4, ' ')
rect(50, 9, 55, 9, ' ')
# Bridge (rows 15-16)
rect(47, 15, 60, 16, F)
# Lake water
blob(30.5, 26.5, 11.5, 8.5, '~', seed=13, only=F)
blob(31, 26.5, 7.0, 4.6, '=', seed=15, only='~')
rect(31, 25, 33, 26, '#')
# Gallery channel: flows east, turns south, spills over the lip into the rift
path([(29, 5.5), (40, 6), (46, 6.5), (51, 7.5), (52, 8.5)], 1.15, '~', only=F)
rect(50, 8, 53, 8, '~')
# Start pool (under a light shaft)
blob(8, 24.2, 3.0, 1.9, '~', seed=19, only=F)
# Grove stream from a spring in the NE to a pool
path([(84, 4), (80, 9), (75, 12.5), (71, 17), (68, 21)], 1.2, '~', only=F)
blob(67, 22, 3.2, 2.2, '~', seed=25, only=F)
# Heart pool with the shard island
blob(73, 44.5, 8.5, 4.6, '~', seed=21, only=F)
rect(72, 44, 73, 45, F)
rect(71, 44, 74, 44, F)

# Fixups: the rift starts at row 9, right under the channel's lip.
for y in range(0, 9):
    for x in range(W):
        if g[y][x] == ' ':
            g[y][x] = '#'
rect(48, 8, 53, 8, '~')
rect(54, 7, 56, 8, '#')
# Gate alcove in the Heart's north wall
rect(80, 36, 82, 37, F)

for row in g:
    print(''.join(row))

img = Image.new('RGB', (W * 6, H * 6))
col = {'#': (40, 30, 80), '.': (60, 80, 120), '~': (60, 140, 200), '=': (20, 50, 120), ' ': (0, 0, 0)}
for y in range(H):
    for x in range(W):
        for yy in range(6):
            for xx in range(6):
                img.putpixel((x * 6 + xx, y * 6 + yy), col.get(g[y][x], (255, 0, 255)))
img.save(sys.argv[1] if len(sys.argv) > 1 else '/tmp/gw_layout.png')
