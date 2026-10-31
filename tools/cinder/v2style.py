"""Draw the V2 Cinder waterworks: slotted fins and a broken keeper, at native pixel scale."""

from draw import Canvas, line, poly
from lib import SheetBuilder


def fin(kind):
    """Carve one asymmetrical black-glass sluice fin whose broad planes can receive local heat."""
    c = Canvas(32, 48)
    top = 3 if kind < 2 else (16 if kind == 2 else 29)
    # Mid obsidian faces (lit left, cool right) give the renderer's lava and brazier light a
    # value to warm; black stays in the narrow slot, the far edge and the plinth's underside.
    poly(c, [(7, 40), (10, top + 3), (17, top), (23, top + 5), (23, 40)], "p")
    poly(c, [(7, 40), (10, top + 3), (17, top), (16, 39)], "s")
    poly(c, [(17, top + 3), (23, top + 5), (23, 40), (16, 39)], "r")
    c.vline(22, top + 6, 39, "q")
    poly(c, [(10, top + 3), (17, top), (22, top + 4), (17, top + 6)], "t")
    line(c, [(10, top + 5), (9, min(top + 14, 38))], "t")
    line(c, [(8, 31), (8, 36)], "r")
    if kind < 3:
        c.vline(14, top + 9, 35, "o")
        c.vline(15, top + 9, 35, "p")
        c.vline(16, top + 9, 33, "q")
        c.hline(13, 17, top + 8, "r")
    if kind == 1:
        c.vline(14, 22, 29, "v")
        c.vline(14, 24, 27, "x")
        c.hline(14, 16, 27, "y")
    if kind >= 2:
        line(c, [(11, top + 4), (14, top + 2), (17, top + 4), (20, top + 2)], "W")
        poly(c, [(24, 42), (25, 38), (29, 39), (30, 42)], "r")
        line(c, [(25, 38), (28, 39)], "t")
    poly(c, [(5, 41), (10, 38), (23, 38), (26, 41), (23, 45), (5, 45)], "q")
    poly(c, [(5, 41), (10, 38), (23, 38), (26, 41)], "s")
    line(c, [(7, 41), (16, 41)], "t")
    c.rect(6, 43, 23, 44, "r")
    c.hline(6, 23, 45, "p")
    return c


def keeper():
    """Cut a lean stone keeper whose source-facing robe and plinth stay light enough to receive heat."""
    c = Canvas(64, 64)
    # Raised platform, tilted top and missing right corner.
    poly(c, [(14, 56), (21, 52), (45, 52), (48, 56), (45, 61), (14, 61)], "q")
    poly(c, [(14, 56), (21, 52), (45, 52), (48, 56)], "r")
    line(c, [(15, 56), (29, 56)], "s")
    c.rect(15, 58, 44, 60, "q")
    c.hline(15, 44, 61, "p")
    # Long robe has broad broken planes, rather than a symmetrical bell hem.
    poly(c, [(25, 20), (36, 20), (42, 51), (35, 54), (21, 52)], "8")
    poly(c, [(25, 20), (30, 23), (28, 47), (23, 51), (21, 50)], "9")
    poly(c, [(32, 25), (36, 23), (41, 51), (35, 53)], "7")
    poly(c, [(28, 39), (30, 38), (28, 51), (25, 52)], "8")
    # Hood: clipped peak and a long, empty face slot.
    poly(c, [(24, 18), (25, 8), (31, 3), (36, 9), (37, 22), (30, 25)], "8")
    poly(c, [(25, 8), (31, 3), (31, 10), (27, 19), (24, 18)], "V")
    poly(c, [(31, 10), (34, 12), (34, 21), (28, 21), (28, 14)], "o")
    line(c, [(26, 10), (25, 16)], "W")
    # The hood's lower lip shades the robe so both mid planes stay separate.
    line(c, [(31, 26), (37, 23)], "7")
    # Surviving arm curves inward to the sword; chest occlusion makes the grip legible.
    poly(c, [(24, 20), (20, 23), (18, 32), (23, 36), (32, 35), (32, 31), (23, 30), (25, 24)], "6")
    poly(c, [(22, 22), (20, 24), (19, 31), (23, 33), (28, 33), (27, 30), (22, 29), (25, 23)], "9")
    line(c, [(20, 26), (20, 30), (23, 32)], "V")
    # Snapped arm has a bounded fracture, not a dangling sleeve.
    poly(c, [(36, 21), (41, 23), (44, 30), (39, 33), (36, 29)], "8")
    poly(c, [(38, 29), (43, 28), (44, 30), (39, 33)], "7")
    line(c, [(38, 29), (41, 28), (43, 29)], "V")
    # Pommel above a compact closed hand, and a narrow cold obsidian blade.
    c.rect(31, 28, 33, 31, "C")
    c.rect(29, 32, 33, 35, "6")
    c.rect(29, 32, 32, 33, "V")
    c.rect(30, 34, 33, 35, "9")
    c.rect(31, 36, 32, 38, "C")
    c.hline(27, 36, 39, "8")
    poly(c, [(30, 40), (33, 40), (33, 51), (31, 54)], "p")
    c.vline(30, 40, 50, "s")
    c.vline(31, 40, 50, "r")
    line(c, [(35, 43), (37, 45), (39, 45)], "6")
    line(c, [(35, 44), (37, 46), (39, 46)], "8")
    # Cuff preserves the same fracture width as the broken upper arm.
    poly(c, [(49, 56), (52, 53), (59, 56), (60, 59), (54, 60), (49, 58)], "7")
    poly(c, [(49, 56), (52, 53), (55, 55), (52, 58)], "6")
    line(c, [(50, 55), (52, 54), (54, 55)], "V")
    c.vline(56, 56, 58, "9")
    c.rect(58, 57, 60, 58, "8")
    return c


def ruin(kind):
    """Draw distributor walls with broken fins and narrow functional water slots."""
    c = Canvas(64, 48)
    if kind == 2:
        # These solid piers retain the original walkable gap at x20..43.
        for x in (9, 44):
            poly(c, [(x, 44), (x, 16), (x+5, 11), (x+10, 16), (x+10, 44)], "p")
            poly(c, [(x, 42), (x, 16), (x+5, 11), (x+4, 42)], "r")
            line(c, [(x+1, 17), (x+1, 27)], "s")
        poly(c, [(14, 15), (18, 7), (30, 4), (34, 7), (28, 10), (21, 11), (19, 17)], "q")
        line(c, [(18, 8), (29, 5)], "s")
        poly(c, [(45, 16), (42, 10), (35, 8), (38, 5), (47, 10), (50, 17)], "r")
    else:
        top = 20 if kind == 0 else 32
        poly(c, [(5, 44), (5, top+4), (11, top), (56, top), (60, top+4), (60, 44)], "p")
        c.rect(6, top+4, 52, 42, "q")
        poly(c, [(5, top+4), (11, top), (56, top), (60, top+4)], "r")
        line(c, [(7, top+4), (23, top+4)], "s")
        for x in (13, 28, 43):
            c.rect(x, top+7, x+4, 39, "o")
            c.vline(x+5, top+7, 38, "r")
        c.hline(7, 28, 41, "r")
        poly(c, [(49, top), (53, top+5), (55, top+8), (58, top+8), (60, top+4), (56, top)], "q")
    return c


def build(name):
    """Build replacement sheets while retaining animation, anchor and footprint contracts."""
    if name == "pillars":
        sb = SheetBuilder("cinder_pillars", (32, 48), [16, 44])
        for i, anim in enumerate(("pillar", "pillar_rune", "pillar_broken_tall", "pillar_broken_short")):
            meta = {"footprint": [14, 7]}
            if i == 1:
                meta["light"] = {"color": "#efb68a", "radius": 26, "intensity": 0.6, "offset": [0, -18]}
            sb.add(anim, fin(i).rgba(), **meta)
    elif name == "statue":
        sb = SheetBuilder("cinder_statue", (64, 64), [32, 60])
        sb.add("statue", keeper().rgba(), footprint=[36, 9])
    else:
        sb = SheetBuilder("cinder_ruins", (64, 48), [32, 45])
        for i, (anim, width) in enumerate((("wall_long", 54), ("wall_low", 44), ("arch_broken", 46))):
            meta = {"footprint": [width, 8]}
            if i == 2:
                meta["notes"] = "Solid piers x9-19 and x44-54; the gap remains walkable."
            sb.add(anim, ruin(i).rgba(), **meta)
    return sb
