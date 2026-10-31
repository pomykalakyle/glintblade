"""Sproutling concept D: a turnip lying forward like a charging bull."""
from pix import Canvas, grid, PREVIEWS
from board import board

PAL = {
    "o": "#1a1030",
    "a": "#3d1245", "b": "#7a1d63", "c": "#c22e6e", "d": "#f25e72", "e": "#ff9f8a", "f": "#ffd2b0",
    # turnip root (cream -> rose shadow)
    "5": "#fff2dc", "4": "#ffc8a8", "3": "#e88a7a", "2": "#a8445a",
    "h": "#0d4d5a", "i": "#147a74", "j": "#55ad4c", "k": "#a3d65a",
    "Y": "#ffd25a", "Z": "#fff6b0",
    "R": "#ff3d6e", "P": "#fff0f4",
    "r": "#5a1a3a",
}

# two-tone turnip: magenta crown, cream belly; face slit at the front
D1 = grid("""
.......eedd....
.....eeffeddc..
...cdeefeedddc.
.bccddeeddddddc
bbccddddddoooooo
.b2c344444oZooZo
..2334444443oob.
....22333333bb..
.......2222.....
""")
# all-magenta body with a cream snout band
D2 = grid("""
.......eedd....
.....eeffeddc..
...cdeefeedddc.
.bccddeeddddddc
bbcccddddoooooo
.bbcccdddoZooZo
..bbccccccoooc.
....bbbcccccb..
.......bbbb....
""")
# two-tone, bigger eyes (2 px tall) peering out of a deeper split
D3 = grid("""
.......eedd....
.....eeffeddc..
...cdeefeedddcc
.bccddeeddddoooo
bbccdddddddoZoZo
.b2c3444444oZoZo
..23344444443oo.
....22333333bb..
.......2222.....
""")
LEAF = grid("""
k........
jk...k...
.jjkkj...
..ijjjj..
....iijj.
""")
LEAF2 = grid("""
kk.......
.jkk.....
..ijkk.k.
...iijjk.
.....iij.
""")
ROOTS = grid("""
r..r
r...
""")


def make(body, leaf, lx=2, ly=5, bx=4, by=10):
    c = Canvas(24, 24)
    c.paste(leaf, lx, ly)
    c.paste(body, bx, by)
    c.paste(ROOTS, bx + 6, by + len(body))
    c.outline("o", skip=("r",))
    return c


cs = [make(D1, LEAF), make(D2, LEAF), make(D3, LEAF), make(D1, LEAF2)]
board(cs, PAL, PREVIEWS / "_wip_sprout3.png", anchors=[(12, 21)] * len(cs), scale=9)
