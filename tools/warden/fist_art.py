"""Hand-drawn pixel grids for the Warden's hand (chars per wlib.CHARS; '.' = transparent).
Screen-right hand: thumb on the left (toward the body). The bottom row is the ground contact.
Grid is 30 wide. Fingers: F1 x2-7, F2 x9-14, F3 x16-21, F4 x23-27; gaps at x8, x15, x22;
x28 is the far side of the hand.
"""

TOP = [
    ".........455555444433.........",
    ".........444444444333.........",
    ".........cccccccbbbbb.........",
    ".........bbbbbbbaaaaa.........",
    ".........333333333222.........",
    ".....4555222222221111443......",
    "....4555555443333333333443....",
    "...455554444444444444444433...",
    "...455544444444444444444433...",
    "..44444444444444444444443333..",
]


def frow(f1, f2, f3, f4, gap, side, left=".."):
    """One finger row: four finger pieces, a gap char (or 3 gap chars), the far side char."""
    g = gap if len(gap) == 3 else gap * 3
    s = left + f1 + g[0] + f2 + g[1] + f3 + g[2] + f4 + side
    return s + "." * (30 - len(s))


FINGERS = [
    # knuckle ridge: four rounded bumps, lit on top
    frow("455544", "455444", "444443", "44433", "2", "2"),
    frow("444433", "444333", "443332", "33322", "1", "1"),
    # crease under the knuckles
    frow("122221", "122221", "122211", "11110", "O", "0"),
    # middle segments of the curled fingers: lit left edge, dark right edge
    frow("455432", "455432", "444321", "43321", "O", "0"),
    frow("443332", "443332", "433321", "33221", "O", "0"),
    frow("433332", "433332", "333221", "32221", "O", "0"),
    frow("433332", "433332", "333221", "32221", "O", "0"),
    frow("433332", "433332", "333221", "32221", "O", "0"),
    frow("433332", "433332", "333221", "32221", "O", "0"),
    frow("433322", "433322", "332221", "22211", "O", "0"),
    frow("322221", "322221", "222211", "21110", "O", "0"),
    frow(".1111.", ".1111.", ".1111.", ".000.", ".", "."),
]

# the thumb crosses the first two fingers: (row index into FINGERS, x, chars)
THUMB = [
    (3, 0, "45"),
    (4, 0, "4554"),
    (5, 0, "455554444"),
    (6, 0, "4444444455541"),
    (7, 0, "33333332444431"),
    (8, 0, "33333332333321"),
    (9, 0, "2222222122221O"),
    (10, 1, "11111111111O"),
]


def clenched(n_back=0, n_finger=0):
    """Clenched fist rows. n_back / n_finger add (or, if negative, drop) rows inside the flat
    back-of-hand and finger areas: stretch and squash without rescaling."""
    top = list(TOP)
    if n_back > 0:
        top = top[:8] + [top[8]] * n_back + top[8:]
    elif n_back < 0:
        top = top[:8 + n_back] + top[8:]
    fingers = [list(r) for r in FINGERS]
    thumb = list(THUMB)
    # rows are added or dropped among the plain finger rows (index 5..8); the thumb moves with them
    if n_finger > 0:
        fingers = fingers[:5] + [list(fingers[5]) for _ in range(n_finger)] + fingers[5:]
    elif n_finger < 0:
        fingers = fingers[:5] + fingers[5 - n_finger:]
    thumb = [(r + n_finger, x, s) for r, x, s in thumb]
    for r, x, s in thumb:
        row = fingers[r]
        for i, ch in enumerate(s):
            if ch != ".":
                row[x + i] = ch
    return top + ["".join(r) for r in fingers]


# rune ring carved on the back of the hand (x is the groove, o the center)
RING = [".xxx.", "x...x", "x.o.x", "x...x", ".xxx."]
RING_AT = (13, 5)  # grid col, row of the ring's top-left


def put_ring(rows, groove, center):
    """rows: list of mutable char lists."""
    x0, y0 = RING_AT
    for j, r in enumerate(RING):
        for i, ch in enumerate(r):
            if ch == "x":
                rows[y0 + j][x0 + i] = groove
            elif ch == "o":
                rows[y0 + j][x0 + i] = center
    return rows


# extended, spread finger, row by row (knuckle ridge, crease, two segments, curled tip)
FINGER_OPEN = ["45543", "44332", "22221", "45432", "44332", "43332", "43332", "33221", "22211", "44321", "43321",
               "33221", "22211", "32221", ".111."]
DARKER = {"5": "4", "4": "3", "3": "2", "2": "1", "1": "0"}


def open_rows():
    """Open hand seen from the back: fingers extended and spread, thumb out to the side."""
    top = [list(r) for r in TOP]
    w = 30
    fingers = [["."] * w for _ in range(len(FINGER_OPEN))]
    # (base col, lean per row index thresholds, darken)
    specs = [(2, [(5, -1), (10, -2)], 0), (9, [(9, -1)], 0), (16, [(9, 1)], 0), (23, [(4, 1), (9, 2)], 1)]
    for x0, lean, dark in specs:
        n = len(FINGER_OPEN) - (1 if x0 == 23 else 0)
        for j in range(n):
            pat = FINGER_OPEN[j] if x0 != 23 or j < n - 1 else FINGER_OPEN[-1]
            dx = 0
            for t, d in lean:
                if j >= t:
                    dx = d
            for i, ch in enumerate(pat):
                if ch == ".":
                    continue
                c = DARKER.get(ch, ch) if dark else ch
                x = x0 + i + dx
                if 0 <= x < w:
                    fingers[j][x] = c
    return ["".join(r) for r in top + fingers]


# thumb: a thick stone digit angled down and out from the side of the hand
OPEN_THUMB = {7: (4, "455"), 8: (3, "45544"), 9: (2, "455443"), 10: (1, "454433"), 11: (0, "44433"),
              12: (0, "43332"), 13: (0, "43322"), 14: (0, "22221"), 15: (0, "3221"), 16: (1, "221"), 17: (1, "11")}
