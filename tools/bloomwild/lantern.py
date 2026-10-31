"""bloomwild_lantern: a stone lantern in the ruins' white stone.

`unlit` is the default (Bloomwild is clear daylight). `lit` has a small flame in the firebox:
the flame is the only emissive color, and the warm light comes from it.
"""
from pix import *
from ruins import block, outline_shadow_side, moss_cluster

CW, CH = 24, 40
AX, AY = 12, 37


def body():
    """Body on the native pixel grid."""
    img = new(CW, CH)
    block(img, 5, 32, 14, 2, 3)            # base slab y 32..36
    # post: square, lit left
    for y in range(22, 32):
        for x in range(9, 15):
            put(img, x, y, "s5" if x == 9 else "s4" if x < 12 else "s3" if x < 14 else "s2")
    block(img, 6, 19, 12, 1, 2)            # firebox floor slab y 19..21
    # firebox: two pillars framing the window (y 13..18)
    for y in range(13, 19):
        for x in (7, 8):
            put(img, x, y, "s5" if x == 7 else "s4")
        for x in (15, 16):
            put(img, x, y, "s3" if x == 15 else "s2")
        for x in range(9, 15):
            put(img, x, y, "s1")           # inside, in shadow
    for x in range(9, 15):
        put(img, x, 13, "s0")              # the roof's shadow on the inside back
    # roof: wide sloped cap (top face + overhang)
    roof = ["......hh......",
            "....hhhhSS....",
            "..hhhhhSSSSm..",
            "hhhhhhSSSSSSmm",
            "SSSSSSSmmmmmmd",
            ".dddddddddddd."]
    grid(roof, {"h": "s5", "S": "s4", "m": "s3", "d": "s2"}, img, 5, 7)
    # finial
    grid([".h.", "hSm"], {"h": "s5", "S": "s4", "m": "s3"}, img, 10, 5)
    moss_cluster(img, 5, 30, [".GG", "Ggk"])
    moss_cluster(img, 6, 9, ["GG", "gk"])
    return outline_shadow_side(img)


FLAMES = [
    ["..y..", ".yYy.", ".YWY.", "..o.."],
    ["...y.", ".yYy.", ".YWY.", "..o.."],
    ["..y..", "..Yy.", ".YWY.", "..o.."],
    [".y...", ".yYy.", ".YWY.", "..o.."],
]


def lit(i):
    """Lit on the native pixel grid."""
    img = body()
    # warm light on the inner walls of the firebox (the flame lights them)
    for y in range(14, 19):
        put(img, 9, y, "au1")
        put(img, 14, y, "au1")
    for x in range(10, 14):
        put(img, x, 18, "au1")
    grid(FLAMES[i], {"y": "au3", "Y": "au3", "W": "au4", "o": "au2"}, img, 10, 14)
    return img


EMISSIVE = ["au3", "au4"]


def write():
    """Write on the native pixel grid."""
    unlit = body()
    frames = [lit(i) for i in range(4)]
    rows = [[unlit] + [new(CW, CH)] * 3, frames]
    man = {
        "cell": [CW, CH],
        "anchor": [AX, AY],
        "emissive": [hexof(c) for c in EMISSIVE],
        "anims": {
            "unlit": {"row": 0, "frames": 1, "fps": 1, "footprint": [12, 5]},
            "lit": {"row": 1, "frames": 4, "fps": 8, "durations": [9, 7, 8, 6], "footprint": [12, 5],
                    "light": {"color": "#ffd25a", "radius": 36, "intensity": 0.6, "offset": [0, -21]}},
        },
        "notes": "Stone lantern, ~33 px tall. Use 'unlit' by default (Bloomwild is daylight); 'lit' only where a "
                 "lit lantern makes sense (a shrine, a shaded spot), sparingly. The flame is the only emissive part.",
    }
    sheet = save_sheet("bloomwild_lantern", rows, CW, CH, man)
    preview(sheet, PREVIEWS / "bloomwild_lantern_6x.png", k=6, bg="g2", gridcell=(CW, CH))
    gif(frames, PREVIEWS / "bloomwild_lantern_lit.gif", k=6, bg="g2", durations=[9, 7, 8, 6])
    return sheet


if __name__ == "__main__":
    write()
