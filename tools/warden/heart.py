"""The Warden's heart, Weir Keeper: a cut iris gem held in the gate's valve ring.

Cell 24x32, anchor (12, 15) = gem centre. A tall step-cut gem, 13 x 19 px: a pointed crown,
a flat table down the middle, four girdle facets and a pointed pavilion. Light from the upper
left: the upper-left crown facet carries a bright edge, left facets are pale (P2), right
facets mid (P1), and the lower-right pavilion facet is the one dark facet (P0), so the stone
reads as cut planes, not a shaded orb. Only P3 glows: the table core and the bright edge.

whole: the table core pulses. cracked: one crack across the crown and table.
badly_cracked: three cracks and the lower-right facet chipped away; the core stutters.
shattered: the facets split off a blazing core (the death burst).
This is the Warden's light source: one point light at the anchor.
"""

from wlib import Canvas

W, H = 24, 32
CX, CY = 12, 15
ANCHOR = (CX, CY)
HALF = {-9: 0, -8: 1, -7: 2, -6: 3, -5: 4, -4: 5, -3: 6, 3: 6, 4: 5, 5: 4, 6: 3, 7: 2, 8: 1, 9: 0}


def half_width(dy):
    """Half width of the gem at a row offset (None outside it)."""
    if abs(dy) > 9:
        return None
    return HALF.get(dy, 6)


def facet(dx, dy):
    """Palette key for the facet under a gem pixel."""
    hw = half_width(dy)
    if abs(dx) <= 2 and abs(dy) <= 4:
        return "P2"  # the table
    if dy < -4 or (dy < -2 and abs(dx) <= 2):
        if dx <= 0:
            return "P3" if abs(dx) == hw else "P2"  # upper-left crown: the bright edge
        return "P1"
    if dy > 4 or (dy > 2 and abs(dx) <= 2):
        return "P1" if dx <= 0 else "P0"  # pavilion: the lower-right facet is the dark one
    return "P2" if dx < 0 else "P1"


def gem(cv, core, lost=()):
    """Draw the cut gem; `core` = (half width, half height) of the glowing table core; `lost` = cleared polygons."""
    for dy in range(-9, 10):
        hw = half_width(dy)
        for dx in range(-hw, hw + 1):
            cv.put(CX + dx, CY + dy, facet(dx, dy))
    # facet edges: the table's frame, a step darker than what it borders
    cv.hline(CX - 2, CX + 2, CY - 5, "P1")
    cv.vline(CX - 3, CY - 4, CY + 4, "P1")
    cv.vline(CX + 3, CY - 4, CY + 4, "P0")
    cv.hline(CX - 2, CX + 2, CY + 5, "P0")
    cw, ch = core
    if cw >= 0:
        cv.rect(CX - cw, CY - ch, CX + cw - (1 if cw else 0), CY + ch, "P3")
    cv.put(CX - 2, CY - 4, "P3")  # glint on the table's upper-left corner
    for pts in lost:
        cv.poly(pts, None)
    cv.outline("OL")
    return cv


def crack(cv, pts):
    """A crack: a dark line through the gem with a lit fissure pixel."""
    for x, y in pts:
        if cv.get(x, y) is not None:
            cv.put(x, y, "OL")
    x, y = pts[len(pts) // 2]
    if cv.get(x - 1, y) not in (None, "OL"):
        cv.put(x - 1, y, "P3")


CRACK_1 = [(15, 8), (15, 9), (14, 10), (14, 11), (13, 12), (13, 13)]
CRACK_2 = [(7, 14), (8, 15), (8, 16), (9, 17), (9, 18), (10, 19)]
CRACK_3 = [(17, 16), (16, 17), (16, 18), (15, 19), (15, 20)]
PULSE = [(1, 2), (1, 3), (1, 2), (0, 1)]
STUTTER = [(1, 3), (0, 1), (1, 3), (-1, 0)]


def shattered(i):
    """The facets thrown off a blazing core; frames alternate 1 and 2 px of spread."""
    cv = Canvas(W, H)
    whole = gem(Canvas(W, H), (1, 4))
    s = 1 + (i % 2)
    quads = [((0, 0, CX - 1, CY - 1), (-s, -s)), ((CX + 1, 0, W - 1, CY - 1), (s, -s)),
             ((0, CY + 1, CX - 1, H - 1), (-s, s)), ((CX + 1, CY + 1, W - 1, H - 1), (s, s))]
    for (x0, y0, x1, y1), (ox, oy) in quads:
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                v = whole.get(x, y)
                if v is not None and abs(x - CX) > 2:
                    cv.put(x + ox, y + oy, v)
    r = 3 + (i % 2)
    for y in range(CY - r - 3, CY + r + 4):
        for x in range(CX - r, CX + r + 1):
            d = abs(x - CX) + abs(y - CY) * 0.6
            if d <= r:
                cv.put(x, y, "P3" if d <= r - 1.5 else "P2")
    return cv


def heart(anim, i=0):
    """Draw one heart frame: whole, cracked, badly_cracked or shattered."""
    if anim == "shattered":
        return shattered(i)
    if anim == "whole":
        return gem(Canvas(W, H), PULSE[i])
    if anim == "cracked":
        cv = gem(Canvas(W, H), PULSE[i])
        crack(cv, CRACK_1)
        return cv
    lost = [[(14, 21), (19, 18), (19, 26), (13, 26)]]
    cv = gem(Canvas(W, H), STUTTER[i], lost)
    for c in (CRACK_1, CRACK_2, CRACK_3):
        crack(cv, c)
    return cv
