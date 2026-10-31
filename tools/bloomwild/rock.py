"""Cliff face band, 48x16, wraps in x and y. Calm dark face (stone[1]) with narrow lit wedges,
partial crevices and staggered ledges, specified by hand per column."""
from pix import *

PW, PH = 48, 16

# (x0, width, crevice rows (set or 'all'), ledge rows, lit width at top of each segment, slope)
COLS = [
    (0, 7, "all", [5], 2, 1),
    (7, 9, range(0, 10), [11], 2, 0),
    (16, 6, "all", [2, 13], 1, 1),
    (22, 4, range(3, 16), [8], 1, 0),
    (26, 6, (), [4], 2, -1),
    (32, 8, "all", [10], 3, -1),
    (40, 8, range(0, 13), [7, 14], 1, 1),
]
# Every 16 px slice starts a new column at its x=0, so slices tile in any order.
MOSS = {(16, 13), (32, 10), (0, 5)}  # (column x0, ledge row) that carry moss


def band():
    """Band on the native pixel grid."""
    t = [["s1"] * PW for _ in range(PH)]
    for (x0, w, crev, ledges, lw0, slope) in COLS:
        crev = range(PH) if crev == "all" else crev
        for y in range(PH):
            ls = sorted(ledges)
            top = max([l for l in ls if l <= y], default=ls[-1] - PH)
            dy = y - top
            lw = max(1, min(w - 2, lw0 + slope * (dy // 4)))
            for rel in range(w):
                x = (x0 + rel) % PW
                if y in ls:
                    c = "s0"
                elif dy == 1:
                    c = "s3" if rel <= lw else ("s2" if rel < w - 1 else "s1")
                elif rel == 0:
                    c = "s0" if y in crev else "s2"
                elif rel <= lw:
                    c = "s2"
                else:
                    c = "s1"
                t[y][x] = c
        for l in ledges:
            if (x0, l) in MOSS:
                for rel in range(1, w - 1):
                    x = (x0 + rel) % PW
                    t[(l + 1) % PH][x] = "g3" if rel < w // 2 else "g2"
                    t[l][x] = "g1" if rel % 2 else "s0"
                t[(l + 2) % PH][(x0 + 2) % PW] = "g1"
                t[(l + 3) % PH][(x0 + 2) % PW] = "g1"
                t[(l + 2) % PH][(x0 + w - 3) % PW] = "g1"
    return t


def render(t):
    """Render on the native pixel grid."""
    img = new(PW, PH)
    for y in range(PH):
        for x in range(PW):
            put(img, x, y, t[y][x])
    return img


if __name__ == "__main__":
    img = render(band())
    big = new(PW * 2, PH * 3)
    for yy in range(3):
        for xx in range(2):
            big[yy * PH:(yy + 1) * PH, xx * PW:(xx + 1) * PW] = img
    save(scale(big, 8), PREVIEWS / "_wip_bw_rock.png")
