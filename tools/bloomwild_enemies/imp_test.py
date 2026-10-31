from pix import Canvas, grid, PREVIEWS
from board import board

PAL = {
    "o": "#1a1030",
    "1": "#0c1e26", "2": "#143a3a", "3": "#1e5a48", "4": "#2f7a52",
    "t": "#7a1d63", "u": "#c22e6e", "v": "#f25e72",
    "M": "#544a72", "N": "#b9aab0", "O": "#e2d2c0", "Q": "#fff2dc",
    "Y": "#ffb43a", "Z": "#fff0a0",
}

# front masks (7 wide)
MF1 = grid("""
.QQOON.
QQOOONM
QuOOOuM
QoZOZoM
OOoOoNM
OONNNNM
.OoooM.
.NOuNM.
..NNM..
""")
MF2 = grid("""
.QQQON.
QQOuONM
QOOuONM
QYoOoYM
OZoOoZM
OONONNM
.oOoNo.
.NoooM.
..NMM..
""")
MF3 = grid("""
..QOO..
.QQOONM
QQOuONM
oYZuZYo
QooOooM
OONONNM
ONoooNM
.NNuNM.
..NMM..
""")

# a simple front body to see the mask in context
BODY_F = grid("""
....3..4...
.v.4333332.v
..u3333332u.
..33433332..
.3.33333.2..
.3.33.33.2..
.4..3..2.2..
.4..3..2....
.3..3..2....
....3..2....
...33..22...
""")


def make(mask):
    c = Canvas(20, 24)
    # mane spikes behind the head
    c.paste(grid("""
..u.......u.
.3u..3...u2.
..33.333.22.
...3333322..
"""), 4, 2)
    c.paste(BODY_F, 4, 12)
    c.outline("o")
    m = Canvas(20, 24)
    m.paste(mask, 6, 4)
    m.outline("o")
    for y in range(24):
        for x in range(20):
            if m.px[y][x] != ".":
                c.px[y][x] = m.px[y][x]
    return c


cs = [make(MF1), make(MF2), make(MF3)]
board(cs, PAL, PREVIEWS / "_wip_imp.png", anchors=[(10, 23)] * 3, scale=9)
