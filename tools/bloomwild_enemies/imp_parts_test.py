from pix import Canvas, grid, PREVIEWS
from board import board
from imp import PAL, MASK, merge, path


def fig(spine, near_leg, far_leg, near_arm, far_arm, thorns, mask=("r", 10, 3), ox=6, oy=9, hair=None):
    c = Canvas(32, 32)
    P = lambda pts: [(x + ox, y + oy) for x, y in pts]
    for x, y in path(P(far_arm)): c.set(x, y, "w")
    for x, y in path(P(far_leg)): c.set(x, y, "w")
    # spine: 2 px, lit on top
    for x, y in path(P(spine)):
        c.set(x, y, "x"); c.set(x, y + 1, "x")
    for x, y in path(P(spine)):
        if c.get(x, y - 1) == ".":
            c.set(x, y, "y")
    if hair:
        c.paste(hair[0], hair[1] + ox, hair[2] + oy, under=True)
    for x, y, ch in thorns:
        c.set(x + ox, y + oy, ch)
    for x, y in path(P(near_leg)): c.set(x, y, "y")
    for x, y in path(P(near_arm)): c.set(x, y, "y")
    c.outline("o")
    mv, mx, my = mask
    m = Canvas(32, 32); m.paste(MASK[mv], mx + ox, my + oy); m.outline("o"); merge(c, m)
    return c

# knuckle walker: arched thorny back, mask thrust forward, long front arms to the ground
K = fig(spine=[(3, 13), (4, 10), (6, 8), (9, 8), (11, 10)],
        near_leg=[(4, 14), (6, 16), (4, 19), (6, 19)], far_leg=[(3, 14), (1, 16), (2, 19), (0, 19)],
        near_arm=[(11, 11), (13, 14), (12, 18), (13, 19)], far_arm=[(10, 11), (10, 14), (9, 18), (8, 19)],
        thorns=[(3, 9, "u"), (2, 8, "v"), (5, 7, "u"), (5, 6, "v"), (8, 7, "u"), (8, 6, "v"), (2, 12, "u"), (1, 12, "v")],
        mask=("r", 10, 2))
K2 = fig(spine=[(3, 13), (3, 10), (5, 8), (8, 7), (10, 8)],
        near_leg=[(4, 14), (7, 16), (5, 19), (7, 19)], far_leg=[(3, 14), (1, 16), (2, 19), (0, 19)],
        near_arm=[(10, 10), (13, 13), (13, 18), (14, 19)], far_arm=[(9, 10), (10, 14), (10, 18), (9, 19)],
        thorns=[(2, 9, "u"), (1, 8, "v"), (4, 7, "u"), (4, 6, "v"), (7, 6, "u"), (7, 5, "v"), (2, 12, "u"), (1, 12, "v")],
        mask=("r", 10, 1))
# lanky upright: tall spine, mask on top, long arms hanging at the sides
HAIR = (grid("""
v..v
uv.u
.uxx
..xx
"""), 5, 0)
L = fig(spine=[(6, 13), (6, 9), (8, 6)],
        near_leg=[(7, 14), (9, 16), (8, 19), (10, 19)], far_leg=[(6, 14), (4, 16), (5, 19), (3, 19)],
        near_arm=[(8, 9), (10, 12), (11, 16), (12, 17)], far_arm=[(6, 9), (4, 12), (3, 16), (2, 17)],
        thorns=[(5, 10, "u"), (4, 10, "v"), (5, 7, "u"), (4, 6, "v")], mask=("r", 8, 0), hair=HAIR)
cs = [K, K2, L]
board(cs, PAL, PREVIEWS / "_wip_imp4z.png", anchors=[(16, 29)] * 3, scale=12, with_glint=False)
board(cs, PAL, PREVIEWS / "_wip_imp4.png", anchors=[(16, 29)] * 3, scale=6)
