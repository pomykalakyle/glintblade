import sys, random
sys.path.insert(0, "..")
import numpy as np
from common import *
import cliffs

W, H = 18, 12
out = np.zeros((H * 16, W * 16, 3), np.uint8); out[:] = hx("#090a1e")
rows = [
 "..................",
 "....######........",
 "...#########......",
 "..############....",
 "..#############...",
 "..#####PPPPP###...",
 "...####PPPPP##....",
 "....###PPPPP......",
 "..................",
 "..................",
 "..................",
 "..................",
]
surf = np.array([[c != "." for c in r] for r in rows])
pave = np.array([[c == "P" for c in r] for r in rows])
for ty in range(H):
    for tx in range(W):
        if surf[ty, tx]:
            out[ty*16:(ty+1)*16, tx*16:(tx+1)*16] = hx("#46384a") if not pave[ty, tx] else hx("#4c5a80")
faces = cliffs.face_tiles()
rims = cliffs.rim_tiles()
rng = random.Random(4)
def S(x, y): return 0 <= x < W and 0 <= y < H and surf[y, x]
for tx in range(W):
    v = rng.choice("abcdefgh")
    ty = 0
    while ty < H:
        if S(tx, ty) and not S(tx, ty + 1):
            y0 = ty + 1
            l = S(tx - 1, ty) and not S(tx - 1, ty + 1)
            r = S(tx + 1, ty) and not S(tx + 1, ty + 1)
            l = S(tx - 1, ty) ; r = S(tx + 1, ty)
            suf = "" if (l and r) else ("_l" if not l else "_r")
            top = ("wall_top_" if pave[ty, tx] else "cliff_top_") + v + suf
            over(out, faces[top], tx*16, y0*16)
            over(out, faces["cliff_mid_" + v + suf], tx*16, (y0+1)*16)
            over(out, faces["cliff_bot_" + v + suf], tx*16, (y0+2)*16)
        ty += 1
for ty in range(H):
    for tx in range(W):
        if S(tx, ty): continue
        mat = "rock"
        if S(tx, ty + 1) and not S(tx, ty - 1): over(out, rims["rim_n_" + mat], tx*16, ty*16)
        if S(tx + 1, ty): over(out, rims["rim_w_" + mat], tx*16, ty*16)
        if S(tx - 1, ty): over(out, rims["rim_e_" + mat], tx*16, ty*16)
        if S(tx + 1, ty + 1) and not S(tx + 1, ty) and not S(tx, ty + 1): over(out, rims["rim_nw_" + mat], tx*16, ty*16)
        if S(tx - 1, ty + 1) and not S(tx - 1, ty) and not S(tx, ty + 1): over(out, rims["rim_ne_" + mat], tx*16, ty*16)
save_rgb("../../../assets/previews/_wip_summit_cliff_test.png", out, 4)
save_rgb("../../../assets/previews/_wip_summit_cliff_test_1x.png", out, 1)
