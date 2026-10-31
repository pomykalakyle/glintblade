from pix import Canvas, grid, PREVIEWS
from board import board
from imp import PAL

# right-facing: hunched, slimmer bramble body with a thorn ridge down the back
BODY_C = grid("""
...v......
...u3..v..
....33.u3.
..v.33333.
..u333333.
.v3443333.
.u34433332
..3443332.
.v334332..
.u333332..
..333322..
...3322...
""")
LEGS_C = grid("""
.43..43.
.43..43.
43..43..
43..43..
443.443.
""")
MASK_C = grid("""
.QQOO.
QQOuON
QOOuNN
QOYoYo
OOZoZN
.NOONM
.NoooM
..NNM.
""")
ARM_C = grid("""
43...
.43..
..43.
..43.
..43.
.4.4.
""")
ARM_D = grid("""
43..
.43.
.43.
.43.
.43.
4.4.
""")

# front (down-facing)
BODY_DF = grid("""
v..........v
u3..v..v..3u
.33.u..u.33.
..33333333..
.v33333333v.
.u34433332u.
..34443332..
...344332...
...333322...
....3322....
""")
LEGS_DF = grid("""
.43..43.
.43..43.
43....43
43....43
443..443
""")
MASK_DF = grid("""
.QQQON.
QQOuONM
QOOuONM
QYoOoYM
OZoOoZM
.ONONM.
.NoooM.
..NMM..
""")
ARM_DL = grid("""
.4
43
43
43
43
4.4
""")
ARM_DR = grid("""
3.
32
32
32
32
3.3
""")


def comp(layers, w=24, h=26):
    c = Canvas(w, h)
    for g, x, y in layers:
        l = Canvas(w, h)
        l.paste(g, x, y)
        l.outline("o")
        for yy in range(h):
            for xx in range(w):
                if l.px[yy][xx] != ".":
                    c.px[yy][xx] = l.px[yy][xx]
    return c


a = comp([(LEGS_C, 5, 16), (BODY_C, 4, 5), (MASK_C, 10, 7), (ARM_C, 11, 13)])
b = comp([(LEGS_C, 5, 16), (BODY_C, 4, 5), (MASK_C, 10, 7), (ARM_D, 12, 13)])
d = comp([(LEGS_DF, 8, 16), (BODY_DF, 6, 6), (ARM_DL, 5, 13), (ARM_DR, 17, 13), (MASK_DF, 9, 6)])
board([a, b, d], PAL, PREVIEWS / "_wip_imp2.png", anchors=[(9, 21), (9, 21), (12, 21)], scale=11)
