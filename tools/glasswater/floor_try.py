import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, on_bg, upscale, PREVIEWS
from stamps import stamp, S

def wpaste(c, s, x, y):
    n = c.shape[0]
    for j in range(s.shape[0]):
        for i in range(s.shape[1]):
            if s[j, i] >= 0:
                c[(y + j) % n, (x + i) % n] = s[j, i]

n = 64
c = np.full((n, n), I["r3"], np.int32)
place = [("hollow_w", 4, 6), ("crack_a", 40, 3), ("pebble_1", 30, 20), ("pebbles_a", 50, 26),
         ("hollow_s", 22, 36), ("crack_b", 2, 44), ("pebble_2", 44, 48), ("hollow_m", 36, 56),
         ("crack_c", 58, 40), ("pebble_1", 14, 58), ("slab", 22, 24)]
for name, x, y in place:
    wpaste(c, stamp(name), x, y)
big = np.tile(c, (2, 3))
upscale(on_bg(big, "#000000", pad=0), 4).save(os.path.join(PREVIEWS, "_wip_floor2.png"))
# stamps sheet for inspection
row = np.full((12, 200), I["r3"], np.int32)
x = 2
for k in S:
    s = stamp(k); row[2:2 + s.shape[0], x:x + s.shape[1]] = np.where(s >= 0, s, row[2:2 + s.shape[0], x:x + s.shape[1]]); x += s.shape[1] + 3
upscale(on_bg(row[:, :x], "#000000", pad=0), 6).save(os.path.join(PREVIEWS, "_wip_stamps.png"))
np.save("/tmp/gw_floor.npy", c)
