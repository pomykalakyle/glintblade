"""The Warden's prism shards, Weir Keeper: a leaning shard wedged in a stone sill.

Cell 20x24, anchor (9, 21) = where the sill sits on its seat (the left gate tower's crown, the
right tower's break, or the top of a hand's cuff). Drawn for the screen-left shoulder; the
runtime mirrors it for the right. Pylons draw three of these in a cluster.
The shard uses the darker prism steps (P0-P2) with one bright tip pixel and a short lit
ridge; only P3 glows. A small second sliver leans the other way behind it.

intact: pulses in step with the heart (same durations). cracked: the tip has snapped and the
ridge flickers. broken: dead stumps in the sill, no glow.
"""

from wlib import Canvas

W, H = 20, 24
ANCHOR = (9, 21)

# main shard leaning right, with a small sliver behind it on the right; '1' P1, '2' P2, '0' P0
SHARD = [
    "......2......",
    ".....221.....",
    ".....2210....",
    "....22210....",
    "....22210....",
    "...222100..0.",
    "...22210..01.",
    "..222110.001.",
    "..222110.011.",
    "..2221100011.",
    ".22211100011.",
]
SNAPPED = [
    ".....0.......",
    "....2210.....",
    "....22210....",
    "...222100..0.",
    "...22210..01.",
    "..222110.001.",
    "..222110.011.",
    "..2221100011.",
    ".22211100011.",
]
STUMPS = [
    "...0.....0..",
    "..220...011.",
    ".2221..0011.",
]
KEY = {"0": "P0", "1": "P1", "2": "P2"}
RIDGE = [(9, 7), (9, 8), (9, 9), (9, 10), (8, 11), (8, 12), (8, 13)]


def sill(cv):
    """The stone sill the shard is wedged into: a lit top, a front and a shaded right end."""
    cv.rect(3, 17, 15, 21, "S1")
    cv.hline(3, 15, 17, "S2")
    cv.hline(4, 9, 17, "S3")
    cv.vline(3, 18, 20, "S2")
    cv.rect(13, 18, 15, 21, "S0")
    cv.put(15, 17, None)  # an old chip in the sill
    cv.put(14, 17, "S0")


def crystal(anim, i=0):
    """Draw one crystal frame: intact, cracked or broken."""
    cv = Canvas(W, H)
    if anim == "broken":
        cv.stamp(4, 14, STUMPS, KEY)
        sill(cv)
        cv.outline("OL")
        return cv
    rows = SHARD if anim == "intact" else SNAPPED
    cv.stamp(3, 17 - len(rows), rows, KEY)
    bright = (anim == "intact" and i == 1) or (anim == "cracked" and i in (0, 2))
    ridge = RIDGE if anim == "intact" else RIDGE[2:]
    n = len(ridge) if bright else 3
    for x, y in ridge[:n]:
        cv.put(x, y, "P3" if bright else "P2")
    if anim == "intact":
        cv.put(9, 6, "P3")  # the bright tip
    else:
        cv.pts([(8, 9), (9, 10), (9, 11)], "P0")  # the snap line
    sill(cv)
    cv.outline("OL")
    return cv
