import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, blank, on_bg, upscale, paste
from crystal import prism_layer, composite, outline
from rockart import g, BASE_BIG

S = (48, 64)
def cluster(ramp):
    L = [
        prism_layer(S, ramp, 15, 58, 33, 5, -11, 8, depth=1, streak=(4, 6)),
        prism_layer(S, ramp, 33, 58, 27, 4, 9, 7, depth=1),
        prism_layer(S, ramp, 24, 57, 50, 6, 5, 10, depth=0, streak=(8, 10)),
        prism_layer(S, ramp, 10, 60, 14, 3, -7, 4, depth=0),
        prism_layer(S, ramp, 38, 60, 12, 3, 6, 4, depth=0),
    ]
    dark = "a0" if ramp == "aqua" else "v0"
    c = composite(L, dark)
    c = outline(c, dark, sides=("r",))
    paste(c, g(BASE_BIG), 2, 55)
    return c

a = cluster("aqua")
v = cluster("violet")
row = np.concatenate([a, blank(4, 64), v], 1)
upscale(on_bg(row, "#2f3c62", pad=4), 6).save("/tmp/gw_crys2.png")
np.save("/tmp/gw_crys_a.npy", a)
