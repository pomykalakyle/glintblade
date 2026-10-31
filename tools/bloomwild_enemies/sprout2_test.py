"""Sproutling redesign concepts (Kyle: the first one was 'very ugly')."""
from pix import Canvas, grid, PREVIEWS
from board import board

PAL = {
    "o": "#1a1030",
    # blossom ramp (red-onion magenta), hue-shifted: violet shadow -> coral light
    "a": "#3d1245", "b": "#7a1d63", "c": "#c22e6e", "d": "#f25e72", "e": "#ff9f8a", "f": "#ffd2b0",
    # leaf ramp: teal shadow -> yellow-green light
    "h": "#0d4d5a", "i": "#147a74", "j": "#55ad4c", "k": "#a3d65a",
    "Y": "#ffd25a", "Z": "#fff6b0",
    "R": "#ff3d6e", "P": "#fff0f4",
}

# A: red-onion bulb, forward horn shoot, slit face with pinpoint eyes, leaf rosette at the base
A = grid("""
..........kk....
.........jjk....
........jj......
.......ij.......
......edc.......
.....eddcb......
....eedddcb.....
...efedddccb....
..eefddddcccb...
..efdddoooooob..
.eedddoZooZoob..
.edddddcooobbb..
.ddddcccccbbbb..
..dcccccbbbbb...
""")
A_ROS = grid("""
.kj............ji.
kjjj..........jjih
.jiii........iiih.
""")

# B: armored bud: overlapping petal plates like a helmet, beak tip forward, eyes in the gap
B = grid("""
.......ed.......
.....eeddc......
....effedcc.....
...effeddccb....
..eefedddccbb...
..eeddddcccbbb..
.eedddcccbbbbbb.
.eddaaaaaaaab...
.ddaZaaZaaab....
.dcaaaaaaab.....
.dccccccbbb.....
..cccbbbbba.....
""")
B_ROS = A_ROS

# C: onion bulb with two forward ram-curl leaf horns, slit face
C = grid("""
...kj......kj...
..jjij....jjij..
..j..i....j..i..
...ji......ji...
......edc.......
.....eddcb......
....eedddcb.....
...efedddccb....
..eefddddcccb...
..efdddoooooob..
.eedddoZooZoob..
.edddddcooobbb..
.ddddcccccbbbb..
..dcccccbbbbb...
""")



# A2: onion belly, sharp neck, hooked shoot, spiky leaf collar, slit with eyes
A2 = grid("""
......jkk...
.....ij..k..
.....i......
....edc.....
...eddcb....
..efedddcb..
.eefddddccb.
eefdddoooooo
efdddoZooZob
eddddcoooobb
.ddcccccbbb.
..ccccbbba..
""")
# A3: same, grin seam with teeth instead of plain slit
A3 = grid("""
......jkk...
.....ij..k..
.....i......
....edc.....
...eddcb....
..efedddcb..
.eefddddccb.
eefdddoooooo
efdddoZooZob
eddddcofofob
.ddcccccbbb.
..ccccbbba..
""")
# A4: no slit: two separate glowing eyes under a heavy brow ridge, jagged grin
A4 = grid("""
......jkk...
.....ij..k..
.....i......
....edc.....
...eddcb....
..efedddcb..
.eefddoocoo.
eefdddoZcoZb
efdddddcccbb
eddddoffofob
.ddcccoooobb
..ccccbbba..
""")
COLLAR = grid("""
k..............k
jk............kj
.jj..........jj.
.ijjk......kjii.
..hiij....jiih..
""")
COLLAR2 = grid("""
.k............k.
kj............jk
.jjk........kjj.
..iijk....kjih..
""")


def make(body, ros, bx=4, by=6, rx=3, ry=17):
    c = Canvas(24, 24)
    c.paste(ros, rx, ry)
    c.paste(body, bx, by)
    c.outline("o")
    return c


cs = [make(A2, COLLAR, bx=6, by=7, rx=4, ry=15), make(A3, COLLAR, bx=6, by=7, rx=4, ry=15),
      make(A4, COLLAR, bx=6, by=7, rx=4, ry=15), make(A3, COLLAR2, bx=6, by=7, rx=4, ry=16)]
board(cs, PAL, PREVIEWS / "_wip_sprout2.png", anchors=[(12, 21)] * len(cs), scale=9)
