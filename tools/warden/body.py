"""The Warden's body, Weir Keeper: a section of an abandoned garden weir that stood up.

Cell 120x112, anchor (60, 109) = ground under the middle of the feet. Fixed attachments (in
every state): heart socket centre (60, 49), neck point (60, 34), shoulder crystal seats
(24, 9) and (96, 9), wrist sockets above the floating hands at x 15 and 105.

Construction, left to right: a low outboard shoulder steps down past a tall left gate tower
(a slotted spillway fin holds the left shard), a sluice gate of stone planks between two
guide channels holds the cut iris heart in a valve ring, the right gate tower is shorter and
snapped (its shard sits on the break), and the right shoulder steps lower still. A crest
beam with a dark neck notch carries the head. Below, a culvert arch opens between two stub
piers on plinths; under an old tide line the stone is a step darker, mineral streaks run
down from the gate seams and the culvert, and moss sits on the culvert lip.
Light from the upper left: top planes S2/S3, fronts S1, turned faces S0, cuts OL/D0.

idle: dormant, channels dry. awake: the guide channels and the heart drain run with dull
iris water (P0, never emissive). damaged: two gate planks have fallen out, the left
tower's crown corner and the right tower's outer slab have broken away, the right shoulder
lost its top block and a fracture runs from the valve ring to the left pier.
"""

from wlib import Canvas

W, H = 120, 112
ANCHOR = (60, 109)
HEART = (60, 49)
TIDE = 93  # below this row the stone is one step darker (where water stood)
DARKER = {"S4": "S3", "S3": "S2", "S2": "S1", "S1": "S0", "S0": "D0"}


def box(cv, x0, y0, x1, y1, c, only=None):
    """Fill an inclusive pixel rectangle."""
    cv.rect(x0, y0, x1, y1, c, only)


def block(cv, x0, y0, x1, y1, top=2, front="S1", lit_col=True, shade_col=0):
    """A stone block: `top` rows of lit top plane, a front face, a lit left column and shade on the right."""
    box(cv, x0, y0, x1, y1, front)
    if top:
        box(cv, x0, y0, x1, y0 + top - 1, "S2")
        cv.hline(x0 + 1, x1 - 2, y0, "S3")
    if lit_col:
        cv.vline(x0, y0 + top, y1 - 1, "S2")
    if shade_col:
        box(cv, x1 - shade_col + 1, y0 + top, x1, y1, "S0")


def chip(cv, pts):
    """Clear a few pixels (chipped corners and broken edges)."""
    for x, y in pts:
        cv.put(x, y, None)


# ---------------------------------------------------------------- masses
def shoulders(cv):
    """Outboard shoulder blocks that step down past the gate towers, each ending in a wrist socket."""
    # left: higher, lit; its outer top corner is clipped
    block(cv, 2, 23, 22, 72, top=3)
    chip(cv, [(2, 23), (3, 23), (2, 24)])
    cv.hline(4, 12, 23, "S3")
    box(cv, 2, 70, 22, 72, "S0")
    # one displaced course joint and one long settled fracture (not scratches)
    cv.hline(2, 9, 47, "OL")
    cv.hline(10, 17, 48, "OL")
    cv.hline(2, 9, 48, "S2")
    cv.hline(10, 17, 49, "S2")
    cv.path([(5, 28), (7, 31), (8, 35), (10, 38), (11, 42)], "OL")
    cv.path([(6, 28), (8, 31), (9, 35), (11, 38)], "S2")
    # right: lower and turned away from the light
    block(cv, 99, 29, 117, 74, top=3, front="S1", lit_col=False, shade_col=5)
    chip(cv, [(117, 29), (116, 29), (117, 30)])
    box(cv, 99, 72, 117, 74, "S0")
    cv.hline(100, 112, 52, "OL")
    cv.hline(100, 112, 53, "S2")


def wrist_sockets(cv):
    """Dark sockets under each shoulder where the floating hands once joined."""
    for x0, x1, y in ((8, 21, 69), (99, 112, 71)):
        box(cv, x0, y, x1, y + 3, "S0")
        cv.hline(x0, x1, y, "S2")
        box(cv, x0 + 2, y + 1, x1 - 2, y + 3, "OL")
        box(cv, x0 + 3, y + 1, x1 - 3, y + 1, "D0")


def towers(cv):
    """Two unequal gate towers: the tall left fin with a spillway slot, the short snapped right one."""
    # left tower: tall, its inner face is the left guide channel
    cv.poly([(16, 5), (19, 2), (33, 2), (36, 5), (36, 81), (28, 81), (28, 72), (16, 62)], "S1")
    cv.poly([(16, 5), (19, 2), (33, 2), (36, 5), (36, 7), (16, 7)], "S2")
    cv.hline(19, 27, 2, "S3")
    cv.pts([(17, 4), (18, 3)], "S3")
    cv.vline(16, 7, 61, "S2")
    box(cv, 32, 8, 35, 80, "S0")  # inner face turned toward the gate
    # the spillway slot through the fin, under the shard's collar (sky shows through)
    box(cv, 22, 11, 25, 22, None)
    cv.vline(21, 11, 23, "S0")
    cv.vline(26, 12, 23, "S2")
    cv.hline(21, 26, 23, "S2")
    # missing stone: one block of the tower face fell out long ago
    box(cv, 18, 38, 24, 43, "D0")
    box(cv, 19, 39, 24, 42, "OL")
    cv.hline(18, 24, 44, "S2")
    cv.hline(17, 31, 52, "OL")  # course joint
    cv.hline(17, 31, 53, "S2")

    # right tower: shorter, its crown snapped off in a ragged step (the shard sits on the break)
    cv.poly([(84, 13), (88, 11), (92, 12), (95, 11), (101, 11), (103, 14), (106, 16), (106, 30),
             (104, 30), (104, 81), (84, 81)], "S1")
    cv.poly([(84, 13), (88, 11), (92, 12), (95, 11), (101, 11), (103, 14), (102, 15), (85, 15)], "S2")
    cv.pts([(88, 11), (89, 11), (95, 11), (96, 11), (97, 11)], "S3")
    box(cv, 100, 15, 105, 29, "S0")
    box(cv, 99, 31, 103, 80, "S0")
    cv.vline(85, 16, 80, "S2")
    cv.hline(86, 98, 40, "OL")  # course joints, the upper one displaced
    cv.hline(86, 98, 41, "S2")
    cv.hline(86, 92, 60, "OL")
    cv.hline(93, 98, 61, "OL")
    cv.hline(86, 92, 61, "S2")
    cv.hline(93, 98, 62, "S2")
    # deep joints where the towers meet the shoulders; the right tower shades its shoulder
    cv.vline(15, 27, 60, "OL")
    cv.vline(106, 31, 72, "OL")
    box(cv, 107, 32, 109, 47, "S0")
    box(cv, 107, 48, 108, 55, "S0")


def crest(cv):
    """The crest beam over the gate, with the dark notch the head sits in."""
    box(cv, 36, 25, 84, 33, "S1")
    box(cv, 36, 25, 84, 26, "S2")
    cv.hline(37, 52, 25, "S3")
    box(cv, 36, 32, 84, 33, "S0")
    # neck notch: the head sits here; a roar bares it
    box(cv, 48, 27, 72, 35, "D0")
    box(cv, 50, 29, 70, 35, "OL")
    cv.hline(48, 72, 26, "S0")


def gate(cv, state):
    """The sluice gate: stone planks between two guide channels, a valve ring and a drain."""
    wet = state != "idle"
    box(cv, 37, 34, 83, 80, "S1")
    # planks: full-width timbers of stone, a lit lip over each dark seam; two old butt joints only
    seams = [41, 49, 57, 65, 73]
    for y in seams:
        cv.hline(37, 83, y, "OL")
        cv.hline(38, 82, y + 1, "S2")
    cv.vline(47, 50, 56, "OL")
    cv.vline(48, 51, 56, "S2")
    cv.vline(71, 74, 80, "OL")
    cv.hline(38, 82, 36, "S2")
    box(cv, 37, 34, 83, 35, "D0")  # contact shadow under the crest beam
    box(cv, 79, 36, 83, 80, "S0")  # right end of the planks, turned away
    cv.vline(38, 37, 80, "S0")  # the plank ends sit deep in the left channel
    # plank 62..67 sags one pixel at its left end (an old failure)
    cv.hline(37, 45, 66, "OL")
    cv.hline(38, 45, 67, "S2")
    cv.put(46, 66, "S0")
    # guide channels (the gate's slots in each tower face)
    for gx in (36, 84):
        cv.vline(gx, 34, 81, "OL")
        cv.vline(gx + (1 if gx == 36 else -1), 34, 81, "P0" if wet else "S0")
    # broken plank: the lower right plank has snapped and fallen in; dark water space behind
    cv.poly([(65, 66), (84, 66), (84, 74), (72, 74), (70, 71), (67, 72)], "D0")
    cv.pts([(65, 66), (66, 66), (67, 67)], "S2")
    cv.pts([(69, 71), (71, 73)], "OL")
    cv.hline(72, 83, 73, "S0")
    # valve ring around the heart socket (octagon), stone stops at the four points
    cx, cy = HEART
    for dy in range(-12, 13):
        for dx in range(-11, 12):
            d8 = max(abs(dx) * 1.0, abs(dy) * 0.85, (abs(dx) + abs(dy)) * 0.68)
            if d8 <= 9.6:
                if d8 > 8.0:
                    c = "S2" if (dx + dy) < -2 else ("S1" if (dx + dy) < 6 else "S0")
                elif d8 > 7.0:
                    c = "OL"
                else:
                    c = "D0" if (dx + dy) < 0 else "OL"
                cv.put(cx + dx, cy + dy, c)
    for x, y in ((cx, cy - 10), (cx - 1, cy - 10), (cx - 10, cy), (cx - 10, cy - 1), (cx + 9, cy), (cx, cy + 10)):
        cv.put(x, y, "S3")
    # the drain from the ring down to the culvert crown
    cv.vline(59, 61, 80, "OL")
    cv.vline(60, 61, 80, "P0" if wet else "S0")
    if wet:
        cv.put(60, 70, "P1")


def lower(cv):
    """Hips, two stub piers with a culvert arch between them, and the plinths."""
    cv.poly([(29, 81), (91, 81), (93, 85), (90, 104), (30, 104), (27, 85)], "S1")
    cv.hline(29, 90, 81, "S2")
    cv.hline(30, 50, 81, "S3")
    box(cv, 83, 82, 91, 104, "S0")
    cv.vline(28, 84, 103, "S2")
    # culvert arch (open through: the floor shows) with its voussoir stones
    arch = [(56, 64, 86), (54, 66, 87), (53, 67, 88), (52, 68, 89)] + [(51, 69, y) for y in range(90, 110)]
    for x0, x1, y in arch:
        cv.hline(x0, x1, y, None)
    for x0, x1, y in arch[:4]:
        cv.pts([(x0 - 1, y), (x1 + 1, y)], "S2" if x0 < 60 else "S0")
    box(cv, 55, 84, 65, 85, "S2")
    cv.hline(56, 64, 84, "S3")
    cv.pts([(54, 85), (66, 85)], "OL")
    # plinths
    cv.poly([(29, 104), (52, 104), (52, 109), (28, 109)], "S1")
    cv.poly([(68, 104), (92, 104), (93, 109), (68, 109)], "S1")
    cv.hline(29, 51, 104, "S2")
    cv.hline(69, 91, 104, "S2")
    box(cv, 88, 105, 92, 108, "S0")


def history(cv):
    """Old dam history: the tide line, stains and mineral streaks from the seams, moss at the culvert."""
    # below the tide line every stone is one step darker
    for y in range(TIDE + 1, H):
        for x in range(W):
            v = cv.get(x, y)
            if v in DARKER:
                cv.put(x, y, DARKER[v])
    for x in list(range(29, 41)) + list(range(71, 83)):
        if cv.get(x, TIDE) is not None:
            cv.put(x, TIDE, "S2")
    # water stains (dark) and mineral streaks (pale) running down from the gate seams
    for x, y0, y1 in ((43, 58, 63), (66, 42, 46), (54, 74, 79)):
        cv.vline(x, y0, y1, "S0")
    cv.vline(44, 58, 61, "S3")
    for x, y0, y1, c in ((41, 82, 90, "D0"), (42, 82, 87, "S0"), (77, 75, 92, "D0"), (78, 82, 89, "S0"),
                         (33, 74, 84, "S0"), (88, 76, 86, "D0")):
        cv.vline(x, y0, y1, c)
    for x, y0, y1 in ((44, 82, 90), (75, 82, 87), (34, 75, 79)):
        cv.vline(x, y0, y1, "S3")
    # from the culvert lip down the pier faces, darkest where the water fell longest
    for x, y0, y1, c in ((50, 90, 101, "D0"), (49, 91, 97, "S0"), (70, 90, 99, "D0")):
        cv.vline(x, y0, y1, c)
    # streaks under the left tower's spillway slot
    cv.vline(23, 24, 35, "S0")
    cv.vline(24, 24, 31, "S3")
    # moss on the culvert lip and where water pooled at the tide line
    cv.hline(55, 58, 83, "M1")
    cv.hline(54, 59, 84, "M0")
    cv.pts([(53, 86), (52, 87), (51, 88), (52, 86)], "M0")
    cv.pts([(67, 86), (68, 87), (66, 85)], "M0")
    cv.pts([(53, 85)], "M1")
    cv.hline(30, 35, 93, "M0")
    cv.hline(31, 33, 92, "M1")
    cv.hline(84, 87, 93, "M0")
    cv.pts([(51, 108), (51, 107), (69, 108)], "M0")


def form_light(cv):
    """Upper-left key light on the big lit faces: one stepped band brighter on the left shoulder and tower."""
    for (x0, y0, x1, y1) in ((3, 26, 14, 46), (17, 8, 31, 37), (17, 38, 27, 51), (37, 27, 47, 31), (86, 16, 92, 39)):
        cv.replace("S1", "S2", (x0, y0, x1, y1))
    # a lip of top light where the upper courses turn toward the sun
    cv.replace("S2", "S3", (3, 26, 4, 34))
    cv.replace("S2", "S3", (17, 8, 18, 20))


def damage(cv):
    """Last phase: planks fall, towers crack and lose stone, a fracture opens from the ring."""
    # two gate planks on the left have fallen out: the dark water space behind shows
    box(cv, 38, 58, 56, 72, "D0")
    cv.pts([(57, 58), (57, 59), (56, 72), (55, 72), (57, 65)], "OL")
    cv.hline(38, 52, 73, "S2")  # the plank below's lip catches light
    cv.pts([(38, 62), (39, 62), (40, 63), (38, 68), (39, 69)], "S0")  # stubs left in the guide channel
    # left tower: its crown's outer corner broke off; a crack runs down from the break
    cv.poly([(15, 1), (21, 1), (21, 4), (19, 8), (17, 13), (15, 14)], None)
    cv.pts([(20, 4), (19, 6), (19, 7), (18, 9), (17, 11)], "S2")
    cv.path([(20, 8), (21, 12), (20, 17), (21, 22), (19, 28), (20, 34)], "OL")
    cv.path([(21, 8), (22, 12)], "S3")
    # right tower: the outer slab has fallen, a long crack splits the rest
    cv.poly([(101, 10), (107, 10), (107, 46), (103, 46), (102, 36), (101, 24), (103, 16)], None)
    cv.pts([(102, 17), (101, 22), (101, 28), (101, 34), (102, 40)], "S0")
    cv.path([(97, 15), (96, 20), (97, 26), (95, 33), (96, 40), (94, 48), (95, 56)], "OL")
    cv.path([(96, 15), (95, 20), (96, 26)], "S3")
    # right shoulder lost its top block
    cv.poly([(104, 28), (119, 28), (119, 44), (114, 44), (110, 37), (104, 34)], None)
    cv.pts([(105, 35), (106, 35), (107, 36), (109, 37), (110, 38), (111, 40), (112, 42), (113, 43)], "S2")
    # fracture from the valve ring down through the hips to the left pier
    crack = [(52, 57), (51, 59), (50, 61), (49, 63), (48, 65), (47, 67), (46, 69), (45, 71),
             (44, 73), (43, 75), (42, 77), (41, 79), (40, 81), (39, 83), (38, 85), (37, 87)]
    cv.pts(crack, "OL")
    cv.pts([(x - 1, y) for x, y in crack], "OL")
    cv.pts([(x + 1, y) for x, y in crack[1:]], "S2")


def body(anim, i=0):
    """Draw one body frame: anim is idle, awake or damaged."""
    cv = Canvas(W, H)
    shoulders(cv)
    wrist_sockets(cv)
    lower(cv)
    towers(cv)
    crest(cv)
    gate(cv, anim)
    history(cv)
    form_light(cv)
    if anim == "damaged":
        damage(cv)
    cv.outline("OL")
    return cv
