from sproutling import frame, PAL
from board import board
from pix import PREVIEWS

def selout(c, mats):
    """Recolor outline pixels on the lit (upper-left) side: their right/lower neighbor is a light tone."""
    src = [r[:] for r in c.px]
    for y in range(c.h):
        for x in range(c.w):
            if src[y][x] != "o":
                continue
            for dx, dy in ((1, 0), (0, 1)):
                xx, yy = x + dx, y + dy
                if 0 <= xx < c.w and 0 <= yy < c.h and src[yy][xx] in mats:
                    # only if the opposite side is empty (true silhouette edge)
                    ox, oy = x - dx, y - dy
                    if not (0 <= ox < c.w and 0 <= oy < c.h) or src[oy][ox] == ".":
                        c.px[y][x] = mats[src[yy][xx]]
                        break
    return c

poses = [("N", "d", "angry", "smirk", "rest", {}), ("N", "r", "angry", "smirk", "rest", {}), ("S2", "r", "hot2", "snarl", "flare", {}), ("T1", "r", "angry", "smirk", "float", {"dy": -5, "roots": "tap"})]
cs = []
for p in poses:
    cs.append(frame(p[0], p[1], p[2], p[3], p[4], **p[5]))
for p in poses:
    cs.append(selout(frame(p[0], p[1], p[2], p[3], p[4], **p[5]), {"4": "1", "5": "1", "d": "a", "c": "a"}))
PAL2 = dict(PAL)
PAL2.update({"a": "#0d4d5a", "b": "#147a74", "c": "#36a88e", "d": "#7fd6a8"})
board(cs, PAL, PREVIEWS / "_cmp_a.png", scale=7, anchors=[(12, 21)] * 8, per_row=4)
board(cs, PAL2, PREVIEWS / "_cmp_b.png", scale=7, anchors=[(12, 21)] * 8, per_row=4)
