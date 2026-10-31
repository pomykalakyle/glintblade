"""Cliff face texture v2: irregular vertical rock ribs on a 48x16 torus."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, on_bg, upscale, PREVIEWS

W, H = 48, 16
# ribs: (start x, width, lit profile, dark profile, cleft shift profile)
# profiles are 16 values (row 0..15); keep row 15 close to row 0 for the vertical wrap.
RIBS = [
    (0, 11, "2233322111222222", "2222333332222222", "0000000000000000"),
    (11, 8,  "1111222221111111", "2222222222333322", "0000000111111000"),
    (19, 13, "3322221111223333", "3333444333333333", "0000000000000000"),
    (32, 9,  "1122222222111111", "2223333332222222", "0011111110000000"),
    (41, 7,  "1111111222211111", "2222222222222222", "0000000000000000"),
]


def face_tex():
    c = np.full((H, W), I["w2"], np.int32)
    for (x0, wdt, lit, dark, sh) in RIBS:
        for y in range(H):
            s = int(sh[y])
            a = x0 + s                      # rib's left edge this row (cleft of previous rib shifts with it)
            b = x0 + wdt - 1                 # this rib's cleft column
            L = int(lit[y]); D = int(dark[y])
            for x in range(a, b + 1):
                xx = x % W
                dx = x - a
                if x == b:
                    col = "w0"
                elif x >= b - D:
                    col = "w1"
                elif dx < L:
                    col = "w3"
                else:
                    col = "w2"
                c[y, xx] = I[col]
            # if the left edge shifted right, the previous rib's cleft widens into shadow
            for x in range(x0, a):
                c[y, x % W] = I["w1"] if x < a - 1 else I["w0"]
    # hand features: highlights on rib tops, pits, a ledge
    feats = {
        # (x, y): color
        (1, 2): "w4", (1, 3): "w4", (2, 3): "w4",
        (20, 1): "w4", (21, 1): "w4", (20, 2): "w4",
        (33, 4): "w4",
        (6, 9): "w1", (7, 9): "w1", (6, 10): "w0", (7, 10): "w1",          # pit in rib A
        (25, 12): "w1", (26, 12): "w1", (27, 12): "w1", (25, 11): "w3", (26, 11): "w3",  # ledge in rib C
        (44, 6): "w1", (45, 6): "w1", (44, 5): "w3",
        (14, 13): "w1", (14, 14): "w1",
    }
    for (x, y), col in feats.items():
        c[y, x] = I[col]
    return c

if __name__ == "__main__":
    c = face_tex()
    big = np.tile(c, (3, 2))
    upscale(on_bg(big, "#000000", pad=0), 6).save("/tmp/gw_face2.png")
