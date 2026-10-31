"""The Warden's head, Weir Keeper: a heavy level lintel over one narrow iris slit.

Cell 44x44, anchor (22, 41) = the neck point (bottom centre); it sits in the body's crest notch.
Construction: a broad, level lintel stone overhangs a dark recessed face and throws a deep
shadow band. In the recess a black socket holds one short tapered slit; the slit's ends dip a
pixel (a frown without slanted brows) and the bright iris is a low dome resting on the lower
lid, watching the ground in front (where Glint stands), with its light caught dully on the lid
under it. Two short jambs frame the recess and step in below the eye like cheekbones, so the
recess tapers toward the set-back jaw wedge that narrows to the neck.
Light from the upper left: lintel top S2/S3, fronts S1, right ends S0, undersides OL/D0.
Only P3 glows (the manifest's emissive color); P0-P2 are the dull iris glass around it.

idle: the eye watches: a short heavy blink (lid down, one low glint), open on the ground in
front, a slow 2 px glance left and a drift back (the iris a 6-7 px dome under a 3-4 px cap; the
heart's gem stays the larger, steadier light). flare: the slit fills and blazes (the attack telegraph).
roar: dip 1 with the eye squinted to a hot glint, lift 2, 3, 3, 3 (1 px rumble left/right) with
the eye snapping open, settle 1. At the peak the lintel
rises one more pixel than the jaw, so the slit opens a row taller.
"""

from wlib import Canvas

W, H = 44, 44
ANCHOR = (22, 41)


def lintel(cv, dx, dy):
    """The heavy level lintel: lit top plane, front, dark underside, one chipped corner and one crack."""
    x0, x1, t = 3 + dx, 40 + dx, 13 + dy
    cv.rect(x0, t, x1, t + 8, "S1")
    cv.rect(x0, t, x1, t + 1, "S2")
    cv.hline(x0 + 2, x0 + 19, t, "S3")
    cv.put(x0 + 1, t + 1, "S3")
    cv.put(x0, t, None)
    cv.vline(x0, t + 1, t + 7, "S2")
    cv.rect(x0 + 1, t + 2, x0 + 12, t + 6, "S2")  # the key light on the lintel's left face
    cv.vline(x0 + 1, t + 2, t + 5, "S3")
    cv.rect(x1 - 4, t + 2, x1, t + 8, "S0")
    cv.hline(x0, x1, t + 8, "S0")
    cv.pts([(x1, t), (x1 - 1, t), (x1 - 2, t), (x1, t + 1)], None)  # chipped right corner
    cv.pts([(x1 - 1, t + 1), (x1 - 2, t + 1)], "S1")
    cv.path([(x0 + 26, t + 2), (x0 + 27, t + 4), (x0 + 26, t + 6), (x0 + 27, t + 8)], "OL")
    cv.path([(x0 + 25, t + 3), (x0 + 25, t + 5)], "S2")


def face(cv, dx, dy):
    """Jambs, the carved recess under the lintel and the jaw wedge narrowing to the neck.

    The recess is graded like a cut socket: black under the lintel and around the eye, lighter
    stone low down, out of the lintel's shadow, and a lit inner wall on the right, which faces the
    upper-left key light. Below the eye the jambs step in like cheekbones.
    """
    t = 22 + dy
    cv.rect(6 + dx, t, 10 + dx, t + 10, "S1")  # left jamb, lit
    cv.vline(6 + dx, t + 1, t + 9, "S2")
    cv.rect(33 + dx, t, 37 + dx, t + 10, "S0")  # right jamb, turned away
    cv.rect(11 + dx, t, 32 + dx, t + 8, "S0")  # the recess
    cv.rect(11 + dx, t, 32 + dx, t + 6, "D0")  # its upper part, in the lintel's shadow
    cv.hline(6 + dx, 37 + dx, t, "OL")  # shadow cast by the lintel
    cv.hline(11 + dx, 32 + dx, t + 1, "OL")
    cv.vline(32 + dx, t + 2, t + 5, "S1")  # the right inner wall faces the light
    # below the eye the jambs step in like cheekbones, so the recess tapers toward the jaw
    for k, (a, b) in enumerate([(12, 31), (13, 30), (14, 29)]):
        y = t + 6 + k
        cv.hline(11 + dx, a - 1 + dx, y, "S1")
        cv.put(a - 1 + dx, y, "S2")  # the step's top edge catches the light
        cv.hline(b + 1 + dx, 32 + dx, y, "S0")
        cv.put(b + 1 + dx, y, "S1")  # the right step's inner face, toward the light
    # jaw wedge, set back, narrowing to the neck
    rows = [(11, 32), (12, 31), (12, 31), (13, 30), (13, 30), (14, 29), (14, 29), (15, 28), (15, 28), (16, 27)]
    j = t + 9
    for k, (a, b) in enumerate(rows):
        cv.hline(a + dx, b + dx, j + k, "S1")
        cv.put(a + dx, j + k, "S2")
        cv.hline(b - 5 + dx, b + dx, j + k, "S0")
    cv.hline(11 + dx, 32 + dx, j, "OL")  # the seam where the jaw sets back
    cv.hline(12 + dx, 21 + dx, j + 1, "S2")


# Eye levels: (glass ramp from the slit's ends to the iris, iris width on the low row,
# iris width on the upper row, spill)
EYES = {
    0: (("P0", "P0", "P1"), 5, 3, False),   # dim smoulder
    1: (("P0", "P1", "P2"), 7, 4, False),   # smoulder
    2: (("P1", "P2", "P2"), 7, 4, False),   # bright
    3: (("P2", "P2", "P2"), 8, 5, True),    # blaze
}


def glass(ramp, x, cx):
    """The slit glass at column x: dull at the slit's ends, brightest beside the iris."""
    d = abs(x - cx)
    return ramp[0] if d >= 4.5 else ramp[1] if d >= 2.5 else ramp[2]


def slit(cv, level, dx, dy, tall=0, low=None, gaze=0, lid=False):
    """The tapered slit in its black socket, ends dipping, iris low in it; `tall` opens it a row higher (roar).

    The socket around the slit is black, the slit is short (12 px plus its dipped ends) and the
    iris is a dome of P3 resting on the lower lid, so at 1x the eye reads as one lit pupil looking
    down at the ground in front, rather than a visor stripe across the face.
    `gaze` slides the iris sideways in the slit (a glance); `lid` lowers the black upper lid over
    the slit's main row, leaving only the iris's low row lit (a slow, heavy blink or a squint).
    All of it stays inside the black socket, so the head's alpha mask never changes.
    """
    ramp, low0, up, spill = EYES[level]
    low = low0 if low is None else low
    t = 25 + dy - tall
    cx = 21.5 + dx
    r = t + tall  # the slit's main row
    cv.rect(12 + dx, t - 1, 31 + dx, r + 2, "OL")  # the socket: upper lid to lower lid
    for k in range(1 + tall):
        for x in range(16 + dx, 28 + dx):
            cv.put(x, t + k, glass(ramp, x, cx))
    cv.pts([(14 + dx, r + 1), (15 + dx, r + 1), (28 + dx, r + 1), (29 + dx, r + 1)], ramp[0])  # the ends dip
    # the iris: low in the slit, centred, with a smaller dome above it
    a = 22 + dx + gaze - (low + 1) // 2
    b = a + low - 1
    if lid:  # the upper lid comes down over the main row: only the low glint is left
        cv.hline(15 + dx, 28 + dx, r, "OL")
    cv.hline(a - 1, b + 1, r + 1, ramp[2])
    cv.hline(a, b, r + 1, "P3")
    if up and not lid:
        ua = 22 + dx + gaze - (up + 1) // 2
        cv.hline(ua, ua + up - 1, r, "P3")
    # the iris lights the lower lid just under it (dull glass, not emissive): it looks down
    cv.hline(a + 1, b - 1, r + 2, "P0" if level < 2 else "P1")
    if spill:  # the blaze lights the lintel's shadow band just above the slit
        cv.hline(18 + dx, 25 + dx, t - 2, "P0")


def draw(eyes=1, dx=0, dy=0, brow=0, low=None, gaze=0, lid=False):
    """Draw the head; dy moves it (negative = up), brow lifts the lintel further and opens the slit."""
    cv = Canvas(W, H)
    face(cv, dx, dy)
    slit(cv, eyes, dx, dy, tall=brow, low=low, gaze=gaze, lid=lid)
    lintel(cv, dx, dy - brow)
    cv.outline("OL")
    return cv


# idle acting, one beat per frame (14/50/40/50 ticks): a short heavy blink (the lid down, one
# dull low glint), the eye opening on the ground in front, a slow glance toward the arena's
# centre-left where Glint usually stands, then back. It watches; it never glares.
IDLE = [0, 1, 1, 1]
IDLE_LOW = [4, 7, 6, 6]
IDLE_GAZE = [0, 0, -2, -1]
IDLE_LID = [True, False, False, False]
FLARE = [2, 2, 3, 3]
# roar: (dy, dx, brow, eyes). The dip squints (lid down, a narrow hot glint: it gathers), then
# the eye snaps open as the head lifts, blazes through the rumble and settles bright.
ROAR = [(1, 0, 0, 2), (-2, 0, 0, 3), (-3, 0, 1, 3), (-3, -1, 1, 3), (-3, 1, 1, 3), (-1, 0, 0, 2)]
ROAR_LID = [True, False, False, False, False, False]
ROAR_LOW = [4, None, None, None, None, None]


def head(anim, i=0):
    """Draw one head frame: idle, flare or roar."""
    if anim == "idle":
        return draw(IDLE[i], low=IDLE_LOW[i], gaze=IDLE_GAZE[i], lid=IDLE_LID[i])
    if anim == "flare":
        return draw(FLARE[i], low=[4, 6, 7, 9][i])
    dy, dx, brow, eyes = ROAR[i]
    return draw(eyes, dx, dy, brow, low=ROAR_LOW[i], lid=ROAR_LID[i])
