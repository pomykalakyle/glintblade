"""Summit standing stones and monoliths -> assets/sprites/summit_stones.{png,json}.

Each stone = a silhouette (per-row spans), shaded by planes lit from the upper left
(cap = top surface, lightest; front face; east bevel in shade), then hand-drawn detail
layers (chips, cracks, carved runes, moss, frost) on top.
"""

from common import blank, get, put, parse_grid, paste

CELL = (32, 64)
ANCHOR = (16, 59)


def spans_from_widths(top_y, base_y, left, right):
    """left/right: functions of y -> x (inclusive) for the front silhouette."""
    return {y: (left(y), right(y)) for y in range(top_y, base_y + 1)}


def shade_stone(img, spans, cap_rows, bevel, base_y, lit_edge_to=None):
    """spans: y -> (x0, x1); cap_rows: x -> last y of the cap at that column;
    bevel: y -> first x of the east bevel."""
    for y, (x0, x1) in spans.items():
        for x in range(x0, x1 + 1):
            cb = cap_rows.get(x, -1)
            if y <= cb:
                t = "5" if x < bevel.get(y, x1) - 2 else "4"
            elif y == cb + 1:
                t = "4" if x < bevel.get(y, x1) else "3"
            else:
                t = "3" if x < bevel.get(y, x1) else "2"
                if x == x0 and (lit_edge_to is None or y <= lit_edge_to) and y > cb + 1:
                    t = "4"
                if y >= base_y - 1:
                    t = "2" if t in ("3", "4") else "1"
            put(img, x, y, t)


def outline(img, col="0"):
    h, w = img.shape[:2]
    pts = []
    for y in range(h):
        for x in range(w):
            if get(img, x, y) == ".":
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if get(img, x + dx, y + dy) not in (".",):
                        pts.append((x, y))
                        break
    for (x, y) in pts:
        put(img, x, y, col)


def cast_shadow(img, base_y, x0, x1, length=4):
    """A short shadow on the ground toward the lower right (light from upper left)."""
    for i in range(2):
        y = base_y + 1 + i
        for x in range(x0 + 2 + i * 2, x1 + length - i):
            if get(img, x, y) == ".":
                put(img, x, y, "b")


def overlay(img, rows, ox, oy):
    """Hand detail: any char except '.' and ' ' is drawn; '_' erases to transparent."""
    for yy, r in enumerate(rows):
        for xx, ch in enumerate(r):
            if ch == "_":
                put(img, ox + xx, oy + yy, ".")
            elif ch not in (".", " "):
                put(img, ox + xx, oy + yy, ch)


def from_planes(plan, lit_rows=None, base_rows=2):
    """plan rows: C = top surface (cap), F = front face, B = east bevel, '.' = empty.
    Cap: 4, with its front lip 5 on the left part. Front: 3, lit left edge 4 (rows in
    lit_rows). Bevel: 2. The bottom `base_rows` rows step one tone darker."""
    h, w = len(plan), max(len(r) for r in plan)
    img = blank(w + 2, h + 6)
    P = lambda x, y: plan[y][x] if 0 <= y < h and 0 <= x < len(plan[y]) else "."
    base_y = max(y for y in range(h) if any(c != "." for c in plan[y]))
    for y in range(h):
        for x, c in enumerate(plan[y]):
            if c == ".":
                continue
            if c == "C":
                fx = [xx for xx, cc in enumerate(plan[y]) if cc in "FC"]
                right_part = x >= (max(fx) - 2 if fx else x)
                t = "5" if (P(x, y + 1) == "F" and not right_part) else "4"
            elif c == "F":
                t = "4" if (P(x - 1, y) == "." and (lit_rows is None or y in lit_rows)) else "3"
            else:
                t = "2"
            if y > base_y - base_rows:
                t = {"5": "4", "4": "3", "3": "2", "2": "1"}[t]
            put(img, x + 1, y + 1, t)
    return img, base_y + 1


def ground_shadow(img, base_y, x0, x1, reach=4):
    """Cast shadow hugging the base, stretched to the lower right."""
    rows = [(x0 + 1, x1 + reach), (x0 + 3, x1 + reach - 2)]
    for i, (a, b) in enumerate(rows):
        for x in range(a, b + 1):
            if get(img, x, base_y + 1 + i) == ".":
                put(img, x, base_y + 1 + i, "b")


def finish(img, base_y, shadow=None):
    outline(img)
    if shadow:
        ground_shadow(img, base_y + 1, *shadow)
    return img


MONO0_PLAN = parse_grid("""
    ..CCC.............
    .CCCCCC...........
    .CCCCCCCC.........
    .FFCCCCCCCC.......
    .FFFFCCCCCCCC.....
    .FFFFFFCCCCCCC....
    .FFFFFFFFCCCCBB...
    .FFFFFFFFFFCCBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBB..
    .FFFFFFFFFFFFBBBB.
    .FFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBB.
    FFFFFFFFFFFFFBBBBB
    FFFFFFFFFFFFFBBBBB
    FFFFFFFFFFFFFBBBBB
""")

# Hand details for monolith 0, in image coords (plan + 1). '_' erases.
# Carved runes: groove core 1, lit lower-right lip 4 (light from the upper left).
MONO0_RUNES = parse_grid("""
    ..1......
    .1.14....
    1.4..14..
    ...1.....
    ...14....
    .11114...
    ...14....
    .........
    .........
    ...1.....
    ..141....
    .1.4.1...
    ..1.14...
    ...14....
    .........
    .........
    .1...1...
    ..1.14...
    ...14....
    ..1.1....
    .14..14..
""")
# a crack from the bevel edge across the face, lit lip under its sideways runs
MONO0_CRACK = parse_grid("""
    ........1
    ......11.
    .....14..
    ....1....
    ...14....
    ..1......
    ..4......
""")
# moss creeping up the shaded base, frost settled on the top surface's left
MONO0_MOSS = parse_grid("""
    ............n...
    ...........no...
    ..........nnon..
    .o........nnnnn.
    .on......nnmnnn.
    noon....nnnnnnmn
""")
MONO0_FROST = parse_grid("""
    .ff.....
    feef....
    .eeee...
""")
# ---------------------------------------------------------------------------
# A. The rune monolith (landmark, 46 px)
# ---------------------------------------------------------------------------
