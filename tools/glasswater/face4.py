"""Cliff face texture v4: calm, hand-placed. 48x16 torus. Base w1; lit planes w2; crevices w0; rare w3."""
import os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I, on_bg, upscale

W, H = 48, 16

# (x, y, w, h, color) painted in order; wraps on the torus.
OPS = [
    # plane 1: wide lit plane, full height, slightly wider in the middle
    (2, 0, 4, 16, "w2"), (6, 5, 1, 7, "w2"),
    (2, 0, 1, 4, "w3"), (3, 1, 1, 2, "w3"),
    (6, 0, 1, 5, "w0"), (7, 5, 1, 7, "w0"), (6, 12, 1, 3, "w0"),
    # plane 2: short plane
    (11, 4, 3, 9, "w2"), (11, 4, 1, 3, "w3"),
    (14, 2, 1, 12, "w0"),
    # plane 3: big plane with a ledge
    (18, 0, 8, 16, "w2"),
    (18, 0, 1, 7, "w3"), (19, 0, 1, 3, "w3"),
    (19, 9, 5, 1, "w0"), (19, 10, 5, 1, "w3"), (24, 9, 1, 1, "w1"),
    (26, 0, 1, 16, "w0"),
    (23, 13, 2, 2, "w1"),
    # plane 4: narrow sliver that wraps vertically
    (30, 7, 2, 11, "w2"), (30, 7, 1, 2, "w3"),
    (32, 6, 1, 13, "w0"),
    # plane 5: upper block and lower block
    (37, 0, 6, 10, "w2"), (37, 1, 1, 4, "w3"), (38, 1, 1, 1, "w3"),
    (43, 0, 1, 12, "w0"),
    (37, 10, 6, 1, "w0"),
    (38, 11, 4, 5, "w2"), (38, 11, 3, 1, "w3"),
    (42, 11, 1, 5, "w0"),
    # plane 6: small wrapped block
    (45, 12, 2, 7, "w2"), (45, 12, 1, 1, "w3"),
    (47, 11, 1, 8, "w0"),
]


def face_tex():
    c = np.full((H, W), I["w1"], np.int32)
    for (x, y, w, h, col) in OPS:
        for j in range(h):
            for i in range(w):
                c[(y + j) % H, (x + i) % W] = I[col]
    return c

if __name__ == "__main__":
    c = face_tex()
    big = np.tile(c, (3, 2))
    upscale(on_bg(big, "#000000", pad=0), 6).save("/tmp/gw_face4.png")
