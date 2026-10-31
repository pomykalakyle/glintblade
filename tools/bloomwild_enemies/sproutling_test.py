from pix import Canvas, grid, PREVIEWS
from board import board

PAL = {
    "o": "#1a1030",
    "1": "#5a1a3a", "2": "#a8445a", "3": "#e88a7a", "4": "#ffc8a8", "5": "#fff2dc",
    "a": "#145846", "b": "#1f8250", "c": "#55ad4c", "d": "#a3d65a",
    "m": "#7a1d63", "w": "#fff2dc", "W": "#ffffff",
}

BODY = grid("""
....443....
..4444433..
.445443333.
44554333333
44443333332
44433333322
33333333322
.333333221.
..2222211..
""")

EYES_R = grid("""
o..o
5o.o5
55.55
""")
EYES_R = grid("""
o...o
5o.o5
55.55
""")
EYES_D = grid("""
o....o
5o..o5
55..55
""")
SMIRK_R = grid("""
....o
oooo.
""")
SNARL_R = grid("""
.ooooo
.5o5o.
""")
OPEN_R = grid("""
o5mm5o
.ommo.
""")
SMIRK_D = grid("""
o...o
.ooo.
""")
OPEN_D = grid("""
o5mm5o
.ommo.
""")

LEAF = grid("""
.dd.....bc.
dccd...bbba
.cccd.bbba.
..ccccbba..
.....cb....
""")


def make(eyes, ex, ey, mouth, mx, my, leaf=LEAF):
    c = Canvas(20, 20)
    bx, by = 4, 8
    c.paste(leaf, bx + 5 - len(leaf[0]) // 2, by - len(leaf) + 1)
    c.paste(BODY, bx, by)
    c.paste(eyes, bx + ex, by + ey)
    c.paste(mouth, bx + mx, by + my)
    c.outline("o")
    return c


cs = [
    make(EYES_R, 5, 2, SMIRK_R, 5, 5),
    make(EYES_R, 5, 2, SNARL_R, 4, 5),
    make(EYES_R, 5, 2, OPEN_R, 4, 5),
    make(EYES_D, 2, 2, SMIRK_D, 3, 5),
    make(EYES_D, 2, 2, OPEN_D, 2, 5),
]
board(cs, PAL, PREVIEWS / "_wip_sprout.png", anchors=[(9, 17)] * 5)
