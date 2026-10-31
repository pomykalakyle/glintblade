"""The Warden's floating hands, Weir Keeper: a sluice counterweight that grips like a clamp.

Cell 40x56, anchor (20, 52) = bottom centre of the clamp (where a slam lands). Drawn as the
screen-right hand (thumb on its left, toward the body); the runtime mirrors it for the left.
One construction in every pose: a wrist cuff (its top is the fist crystal's seat), a
counterweight block with a lifting slot down its middle, a hooked thumb on the body side and
two broad curled clamp fingers. Planes match the body's stone size; light from the upper left.
The slot carries dull iris light (P1/P2) when the hand charges, P3 at its peak (the only glow).

Rig contract (rows relative to the anchor, - is up; `solid` = the 5 px column over the anchor):
rest (slam 4, clenched 0/3): outline top -25, bottom +1. clenched 1/2 bob up 1 px.
slam bottoms -8, -22, -2, +1, +1 and solid tops -34, -51, -33, -19, -25, i.e. baked lifts
9, 23, 3, 0, 0 and crystal shifts 0, -3, -5, +6, 0 (the runtime cancels the lifts).
"""

from wlib import Canvas

W, H = 40, 56
ANCHOR = (20, 52)
CX = 20
SLOT = [None, "P1", "P2", "P3"]


def cuff(cv, y0, rows=5):
    """The wrist cuff: lit top plane, front and dark underside; its top is the crystal seat."""
    cv.rect(CX - 5, y0, CX + 5, y0 + rows - 1, "S1")
    cv.hline(CX - 5, CX + 5, y0, "S2")
    cv.hline(CX - 4, CX, y0, "S3")
    cv.vline(CX - 5, y0 + 1, y0 + rows - 2, "S2")
    cv.rect(CX + 3, y0 + 1, CX + 5, y0 + rows - 1, "S0")
    cv.hline(CX - 5, CX + 5, y0 + rows - 1, "S0")


def weight(cv, y0, y1, half, lit):
    """The counterweight block with rounded top corners and its lifting slot."""
    for k, y in enumerate(range(y0, y1 + 1)):
        cut = 2 if k == 0 else (1 if k == 1 else 0)
        a, b = CX - half + cut, CX + half - cut
        cv.hline(a, b, y, "S1")
        cv.put(a, y, "S2")
        cv.hline(b - 3, b, y, "S0")
    cv.hline(CX - half + 2, CX + half - 4, y0, "S2")
    cv.hline(CX - half + 2, CX - 1, y0, "S3")
    cv.hline(CX - half + 1, CX + half - 4, y0 + 1, "S2")
    cv.hline(CX - half, CX + half, y1, "S0")
    # upper-left key light: one stepped band on the left of the weight
    cv.replace("S1", "S2", (CX - half, y0 + 2, CX - 3, y0 + 2 + (y1 - y0) // 2))
    # the lifting slot
    cv.rect(CX - 1, y0 + 2, CX, y1 - 2, "OL")
    if lit:
        cv.vline(CX, y0 + 3, y1 - 3, SLOT[min(lit, 2)])
        mid = (y0 + y1) // 2
        if lit >= 3:
            cv.vline(CX, mid - 1, mid + 1, "P3")
        elif lit == 2:
            cv.put(CX, mid, "P3")


def lobe(cv, x0, x1, y0, y1, dark=False, flat=False):
    """One curled clamp finger: lit lip, front, rounded tip and a dark underside.

    `flat` presses the tip square onto the ground (the slam's contact): the rounded corners fill
    in, so the finger reads as a stone block bearing down rather than a curled knuckle.
    """
    n = y1 - y0 + 1
    for k, y in enumerate(range(y0, y1 + 1)):
        cut = 0 if flat else 2 if k == n - 1 else (1 if k == n - 2 else 0)
        cv.hline(x0 + cut, x1 - cut, y, "S0" if (k >= n - 2 or dark and k >= n // 2) else "S1")
        if k < n - 2:
            cv.put(x0, y, "S2")
            cv.hline(x1 - 2, x1, y, "S0")
    cv.hline(x0, x1 - 3, y0, "S2")
    if not dark:
        cv.replace("S1", "S2", (x0, y0 + 1, x0 + (x1 - x0) // 2, y0 + 2))


def clamp(cv, y0, y1, half, flat=False):
    """Two broad curled clamp fingers under the weight, split down the middle."""
    cv.hline(CX - half, CX + half, y0, "OL")  # the shadowed joint under the weight
    lobe(cv, CX - half, CX - 1, y0 + 1, y1, flat=flat)
    lobe(cv, CX + 1, CX + half, y0 + 1, y1, dark=True, flat=flat)
    cv.vline(CX, y0 + 1, y1, "OL")


def thumb(cv, y0, y1, half):
    """The hooked thumb on the body side, its tip curling over the first finger."""
    x0 = CX - half - 5
    n = y1 - y0 + 1
    for k, y in enumerate(range(y0, y1 + 1)):
        cut = 1 if k in (0, n - 1) else 0
        hook = 2 if k >= n - 3 else 0
        cv.hline(x0 + cut + hook, x0 + 6 - cut + hook, y, "S1")
        cv.put(x0 + cut + hook, y, "S2")
    cv.hline(x0 + 1, x0 + 5, y0, "S3")
    cv.hline(x0 + 3, x0 + 7, y1, "S0")
    cv.vline(x0 + 7, y0 + 1, y1 - 3, "OL")  # its seam against the weight


def hand(top, bot, half=11, fingers=9, lit=0, cuff_rows=5, flat=False):
    """A closed hand whose outline spans rows top..bot (inclusive), centred on column 20."""
    cv = Canvas(W, H)
    a0, a1 = top + 1, bot - 1
    cuff(cv, a0, cuff_rows)
    wy0, wy1 = a0 + cuff_rows, a1 - fingers
    weight(cv, wy0, wy1, half, lit)
    clamp(cv, wy1 + 1, a1, half, flat)
    thumb(cv, wy0 + 2, wy1 + 4, half)
    cv.outline("OL")
    return cv


def open_hand(top, bot, lit=0):
    """The clamp spread flat on the floor (kneel, dormant and beam charge); the cuff stays at the seat."""
    cv = Canvas(W, H)
    a0, a1 = top + 1, bot - 1
    cuff(cv, a0)
    wy0, wy1 = a0 + 5, a0 + 14
    weight(cv, wy0, wy1, 11, lit)
    # the two clamp fingers splayed out and lying flat, the floor showing between them
    cv.hline(CX - 11, CX + 11, wy1, "OL")
    lobe(cv, CX - 17, CX - 3, wy1 + 1, a1)
    lobe(cv, CX + 3, CX + 17, wy1 + 1, a1, dark=True)
    # the thumb laid out toward the body
    t = wy0 + 3
    cv.rect(CX - 17, t, CX - 11, t + 4, "S1")
    cv.hline(CX - 16, CX - 12, t, "S3")
    cv.vline(CX - 17, t + 1, t + 3, "S2")
    cv.hline(CX - 17, CX - 11, t + 4, "S0")
    cv.vline(CX - 10, t + 1, t + 4, "OL")
    cv.outline("OL")
    return cv


def speed_lines(cv, top):
    """Thin streaks above a falling hand (kept off the anchor columns)."""
    for x, n in ((8, 8), (12, 12), (28, 11), (32, 6)):
        cv.vline(x, top - n - 1, top - 2, "S1")


def chips(cv):
    """Stone chips thrown sideways on impact (never below the contact row)."""
    for x, y in ((2, 49), (4, 45), (36, 50), (34, 46)):
        cv.rect(x, y, x + 1, y + 1, "S2")
        cv.put(x + 1, y + 1, "S1")


BOB = [0, -1, -1, 0]
OPEN_LIT = [1, 2, 2, 1]
REST = (27, 53)


def fist(anim, i=0):
    """Draw one hand frame: clenched, open, open_lit or slam."""
    t, b = REST
    if anim == "clenched":
        return hand(t + BOB[i], b + BOB[i])
    if anim in ("open", "open_lit"):
        return open_hand(t - 1 + BOB[i], b + BOB[i], OPEN_LIT[i] if anim == "open_lit" else 0)
    if i == 0:  # rise: the rest hand lifted 9 px, the slot starting to fill (the build-up begins)
        return hand(t - 9, b - 9, lit=1)
    if i == 1:  # wind-up hold: lifted 23 and stretched 3, the slot blazing (the telegraph)
        return hand(1, 30, lit=3)
    if i == 2:  # falling: stretched 5, a little narrower, streaks above
        cv = hand(19, 50, half=10, lit=1)
        speed_lines(cv, 19)
        return cv
    if i == 3:  # impact: squashed 6 and widened, the clamp pressed flat on the ground, chips flying
        cv = hand(33, 53, half=13, fingers=7, cuff_rows=3, flat=True)
        chips(cv)
        cv.outline("OL")
        return cv
    return hand(t, b)  # recover
