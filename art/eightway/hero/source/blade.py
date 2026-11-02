"""The crystal blade, drawn on the native grid in 16 directions with clean slopes only
(0, 1:2, 1:1, 2:1, vertical). Direction k: 0 = right, 4 = down, 8 = left, 12 = up (screen)."""
import math

# repeating unit steps for each of the 16 directions
STEPS = {
    0: [(1, 0)],
    1: [(1, 0), (1, 1)],
    2: [(1, 1)],
    3: [(0, 1), (1, 1)],
    4: [(0, 1)],
    5: [(0, 1), (-1, 1)],
    6: [(-1, 1)],
    7: [(-1, 0), (-1, 1)],
    8: [(-1, 0)],
    9: [(-1, 0), (-1, -1)],
    10: [(-1, -1)],
    11: [(0, -1), (-1, -1)],
    12: [(0, -1)],
    13: [(0, -1), (1, -1)],
    14: [(1, -1)],
    15: [(1, 0), (1, -1)],
}

# guard (crossguard) offset: perpendicular-ish to the blade
GUARD = {
    0: [(0, -1), (0, 1)], 1: [(0, -1), (0, 1)], 2: [(1, -1), (-1, 1)], 3: [(-1, 0), (1, 0)],
    4: [(-1, 0), (1, 0)], 5: [(-1, 0), (1, 0)], 6: [(-1, -1), (1, 1)], 7: [(0, -1), (0, 1)],
    8: [(0, -1), (0, 1)], 9: [(0, -1), (0, 1)], 10: [(1, -1), (-1, 1)], 11: [(-1, 0), (1, 0)],
    12: [(-1, 0), (1, 0)], 13: [(-1, 0), (1, 0)], 14: [(-1, -1), (1, 1)], 15: [(0, -1), (0, 1)],
}

# default blade length (pixels from guard to tip) so every direction looks about as long
LENGTH = {0: 9, 1: 8, 2: 7, 3: 8, 4: 9, 5: 8, 6: 7, 7: 8, 8: 9, 9: 8, 10: 7, 11: 8, 12: 9, 13: 8, 14: 7, 15: 8}


def points(hx, hy, k, n):
    """n points stepping away from (hx, hy) in direction k (not including the start)."""
    pts = []
    x, y = hx, hy
    steps = STEPS[k % 16]
    for i in range(n):
        dx, dy = steps[i % len(steps)]
        x += dx
        y += dy
        pts.append((x, y))
    return pts


def offsets(k):
    """(lit, shadow): one-pixel axis offsets to either side of a blade pointing in direction k.
    The lit face is the side facing the upper-left light; on exact diagonals the vertical
    neighbour is used so the faces tile into a solid band."""
    a = k * math.pi / 8
    dx, dy = math.cos(a), math.sin(a)
    perps = [(dy, -dx), (-dy, dx)]
    lit = max(perps, key=lambda p: -p[0] - p[1] * 1.01)

    def axis(p):
        """Choose a grid-axis offset for a blade face without resampling."""
        if abs(abs(p[0]) - abs(p[1])) < 1e-6:
            return (0, 1 if p[1] > 0 else -1)
        if abs(p[0]) > abs(p[1]):
            return (1 if p[0] > 0 else -1, 0)
        return (0, 1 if p[1] > 0 else -1)

    return axis(lit), axis((-lit[0], -lit[1]))


def draw(c, hx, hy, k, length=None, glint=None, charged=0, guard=True, hilt=True, dim=False):
    """Draw the crystal blade from the hand at (hx, hy).

    Construction (same in every direction): a white-cyan ridge down the middle, a cyan face on
    the lit (upper-left) side, a deep-cyan face on the shadow side, so the body is 3 px wide in the
    middle, 2 px near the tip, and a 1 px white point. One white facet sits on the lit face a third
    of the way up. `glint` = index along the ridge of a travelling white sparkle; `charged` = 1 or 2
    brightens the faces while charging; `dim` = the light gone out (dead). Returns the tip."""
    k %= 16
    n = (length or LENGTH[k]) + 1
    pts = points(hx, hy, k, n)
    gx, gy = pts[0]
    bp = pts[1:]
    L = len(bp)
    lit, sh = offsets(k)
    face_sh, face_lit, ridge, tip = "c", "C", "w", "W"
    if charged:
        face_sh, face_lit = "C", "w"
    if dim:
        face_sh, face_lit, ridge, tip = "S", "t", "t", "T"
    for i, (x, y) in enumerate(bp):
        if i == L - 1:
            continue
        if 1 <= i <= L - 4:
            c.put(x + sh[0], y + sh[1], face_sh)
        if i <= L - 3:
            c.put(x + lit[0], y + lit[1], face_lit)
    for i, (x, y) in enumerate(bp):
        ch = tip if i == L - 1 else ridge
        if charged >= 2 and i % 3 == 2:
            ch = "W"
        if glint is not None and i == glint and not dim:
            ch = "W"
        c.put(x, y, ch)
    if L > 4 and not dim:
        x, y = bp[2]
        c.put(x + lit[0], y + lit[1], "W")
    if hilt:
        c.put(gx, gy, "B")
        if guard:
            for ox, oy in GUARD[k][:1]:
                c.put(gx + ox, gy + oy, "b")
    return bp[-1]
