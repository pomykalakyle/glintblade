from pix import Canvas, grid, PREVIEWS
from board import board

PAL = {
    "o": "#1a1030",
    "b": "#7a1d63", "c": "#c22e6e", "d": "#f25e72", "e": "#ff9f8a", "f": "#ffd2b0",
    "i": "#147a74", "j": "#55ad4c", "k": "#a3d65a", "h": "#0d4d5a",
    "5": "#fff2dc", "R": "#ff3d6e", "P": "#fff0f4",
}
# E/F = eye top/bottom, G/H = mouth row (lip or teeth)
FACE = {"calm": {"E": "5", "F": "5", "G": "f", "H": "f"},
        "scowl": {"E": "o", "F": "5", "G": "f", "H": "f"},
        "scowl2": {"E": "o", "F": "5", "G": "f", "H": "f", "X": "5"},
        "fang": {"E": "P", "F": "R", "G": "5", "H": "o"}, "hot": {"E": "P", "F": "R", "G": "f", "H": "f"},
        "teeth": {"E": "P", "F": "R", "G": "5", "H": "o"}, "blink": {"E": "o", "F": "5", "G": "f", "H": "f"}}
FACE["scowl2"].pop("X")

N = grid("""
.......eeed......
.....effffedd....
...deffffeeddd...
.cdeeffeeedddcccc
cddeeeeddddcoEoEc
.bcddddddddcoFoFc
..bccddddddeGHGHe
...bbccccccceffe.
.....bbbbbccc....
""")
LEAF = grid("""
k........
jk...k...
.jjkkj...
..ijjjj..
....iijj.
""")
FLARE = grid("""
.k...k......
k.j..j...kk.
..jj.j..j..k
...ij.jjj...
....iiij....
.....ii.....
""")


def make(g, face, leaf=LEAF, lx=1, ly=5):
    c = Canvas(24, 24)
    c.paste(leaf, lx, ly)
    rows = ["".join(FACE[face].get(ch, ch) for ch in r) for r in g]
    c.paste(rows, 3, 9)
    c.outline("o")
    return c

N2 = [r.replace("GHGH", "GHHG") for r in N]
N3 = [r for r in N]
N3[4] = N3[4].replace("coEoEc", "coooEc")
N3[5] = N3[5].replace("coFoFc", "coFoFc")
cs = [make(N, "scowl"), make(N3, "scowl"), make(N, "hot", FLARE, 3, 4), make(N2, "fang", FLARE, 3, 4)]
board(cs, PAL, PREVIEWS / "_wip_sprout_v3z.png", anchors=[(12, 19)] * 4, scale=12, with_glint=False)
