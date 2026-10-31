"""Style v2 pointed seed bulb on two root canes, carrying its hard ivory seed point forward as a ram.

Hand-authored grids preserve the reviewed hop, braced anticipation and lunge timings.
Run from the repository root with uv run --no-project --with pillow python3 tools/bloomwild_enemies/sproutling.py.
"""

from pix import Canvas, grid, mirror, build_sheet

PAL = {
    "o": "#303b4d",  # tinted structural outline and recessed eyes
    "b": "#655f78", "c": "#927c91", "d": "#bf9da9", "e": "#dbb9bd", "f": "#dfdac1",
    "i": "#3b535b", "j": "#587278", "k": "#829b8e",
    "r": "#3b535b", "5": "#c2c1b3",
    "R": "#bd7a85", "P": "#e6b4ac",  # matte rose warning, no emitter
}
EMISSIVE = []

CW, CH = 24, 24
AX, AY = 12, 21  # anchor: under the middle of the body, on the ground line

# side-view faces: letter -> color
FACE_SIDE = {
    "calm": {"E": "o", "F": "5", "G": "f", "H": "f"},   # scowl: 1 px eyes under the brow
    "blink": {"E": "o", "F": "o", "G": "f", "H": "f"},
    "hot": {"E": "P", "F": "R", "G": "f", "H": "f"},    # matte rose threat
    "teeth": {"E": "P", "F": "R", "G": "5", "H": "o"},  # the split mask opens
    "ouch": {"E": "5", "F": "5", "G": "f", "H": "f"},   # eyes wide
}


def body(s):
    """Parse a deliberately authored body grid and pad transparent margins."""
    return grid(s)


# ----------------------------------------------------------------------------- right-facing bodies
# (grid, leaf root (x, y), center x). A pointed seed bulb carried head-low on two root canes:
# the husk swells at the back shoulder and tapers forward into the hard ivory seed point, which is
# the ram it charges with. One dark husk seam runs from the shoulder to the brow. "E"/"F" is the
# recessed slit under the point's brow and "GH" the jaw seam. Lit rim only on the upper-left edge.
R = {}
# Raised and forward-loaded: rear cane pushing back, front cane planted under the point.
R["N"] = (body("""
...edc..........
.eddcccc........
edccbbcccbff....
dcccccccboEff5..
ccccbbbbboFff55f
.cbbbbbbbbGH55..
..bbbbbbcc......
...bi....ic.....
..bi.....ic.....
.ii......icc....
"""), (4, 0), 7)
# Settled breath: knees give, the bulb sinks onto its canes, the point stays aimed.
R["SQ"] = (body("""
....edc..........
..eddcccc........
.edccbbccccbff...
edcccccccccboEff5
dccccbbbbbbboFff5
.cbbbbbbbbbbGH55.
...bi......icc...
.iii.......iccc..
"""), (5, 0), 8)
# Takeoff: canes straighten under a narrower, taller bulb.
R["ST"] = (body("""
...edc.........
..eddccc.......
.edcbbcccff....
edcccccboEff5..
dcccbbbboFff55f
.cbbbbbbbGH55..
..bbbbbcc......
...bi...ic.....
...bi...ic.....
..bi.....ic....
..i......icc...
"""), (4, 0), 7)
# Windup: rocks back onto the rear cane, point raised, front cane lifted off the ground.
R["REAR"] = (body("""
.......edcff....
.....eddcboEf5..
....edcbbcoFf55f
...edccccccGH55.
..edcccccbbbc...
..dcccccbbbb....
.dcccbbbbbb.ic..
.ccbbbbbb....ic.
..bbbbc.........
...bi...........
..bi............
.ii.............
"""), (6, 0), 6)
# Lunge: the whole husk lies along the strike line behind the point.
R["LUNGE"] = (body("""
......eddc..........
...eddcccbbbb.ff....
.edccccccccccoEff5..
dccccbbbbbbbboFff55f
.cbbbbbbbbbbbbGH55..
...bbbbcc...........
...ii.....ic........
.ii........icc......
"""), (7, 0), 10)
# Impact: the point stops dead and the bulb piles up behind it.
R["IMP"] = (body("""
....edc.......
..eddcccc.....
.edccbbcccff..
edcccccccoEff5
dccccbbbboFff5
.cbbbbbbbbGH5.
..bbbbbbcc....
..bi.....ic...
.bi.....ic....
ii......icc...
"""), (5, 0), 7)
# Hurt: knocked back on its heels, point tipped up.
R["HURT"] = (body("""
.....edcff......
...eddcboEf5....
..edcbbcoFf55f..
.edcccccccGH5...
edcccccccbbb....
dccccbbbbbc.....
.cbbbbbb..ic....
..bbcc.....ic...
...bi......icc..
..ii............
"""), (5, 0), 7)

# smear: the lunge with the body dragged into a tapered streak behind it
SMEAR_TRAIL = grid("""
....
...c
.ccc
bbbb
..bb
""")


# ----------------------------------------------------------------------------- front bodies (down/up)
# Head-on, the husk is an almond pointed at both ends: the sprout end on top and the seed point
# aimed at the camera as an ivory wedge under one dark brow slit. Canes stand wide. (grid, face pos, leaf root, center x)
F = {}
F["N"] = (body("""
....edc....
...eddcb...
..edcccbb..
.edcccccbb.
edcccccccbb
dcccccccbbb
.ccccccbbb.
..bcccbbb..
.bibcbbbib.
.bi.....ib.
ii.......ii
"""), (3, 4), (5, 0), 5)
F["SQ"] = (body("""
.....edc.....
...eeddcbb...
.eedcccccbbb.
edcccccccccbb
dccccccccccbb
.cccccccccbb.
...bcccccbb..
..bi.bcb..ib.
.ii.......ii.
"""), (4, 3), (6, 0), 6)
F["ST"] = (body("""
....edc....
...eddcb...
..edcccbb..
.edcccccbb.
edcccccccbb
dcccccccbbb
.ccccccbbb.
..bcccbbb..
..bbcbbb...
..bi...ib..
..bi...ib..
..i.....i..
"""), (3, 4), (5, 0), 5)
F["BRACE"] = (body("""
......edc......
....eeddcbb....
..eedcccccbbb..
.edcccccccccbb.
edcccccccccccbb
.ccccccccccbbb.
...bbcccccbb...
..bi..bcb..ib..
.ii.........ii.
"""), (5, 3), (7, 0), 7)
F["LUNGE"] = (body("""
....edc....
..eeddcbb..
.edcccccbb.
edcccccccbb
dcccccccbbb
.cccccccbb.
..bcccbbb..
.bi.bcb.ib.
ii.......ii
"""), (3, 3), (5, 0), 5)
F["HURT"] = F["SQ"]

# The seed point seen end-on: one brow slit over a tapering ivory wedge, never two round eyes.
FACE_F = {
    "calm": grid("""
ooooo
fff55
.ff5.
.f55.
..5..
"""),
    "blink": grid("""
ooooo
fff55
.ff5.
.f55.
..5..
"""),
    "hot": grid("""
oRRRo
fff55
.ff5.
.f55.
..5..
"""),
    "teeth": grid("""
oRPRo
ffo55
.fo5.
.f55.
..5..
"""),
    "ouch": grid("""
oo5oo
fff55
.fo5.
.f55.
..5..
"""),
}

# ----------------------------------------------------------------------------- leaves
# Right view: (grid, attach point (x, y) = the pixel that sits on the body's leaf root)
LEAF_R = {
    "rest": (grid("""
k........
jk...k...
.jjkkj...
..ijjjj..
....iijj.
"""), (7, 4)),
    "sway": (grid("""
.........
kk...k...
.jjkkj...
..ijjjj..
....iijj.
"""), (7, 4)),
    # hop: the two leaves stream up and apart
    "lift": (grid("""
k....kk..
jk..jk...
.jj.j....
..ijj....
...iiijj.
"""), (7, 4)),
    "flat": (grid("""
kkk......
.jjjkk...
...iijjjj
"""), (8, 2)),
    # windup: three leaves fan out, back one leaning back, middle up, front one curling forward
    "flare": (grid("""
.k...k......
k.j..j...kk.
..jj.j..j..k
...ij.jjj...
....iiij....
.....ii.....
"""), (5, 5)),
    "streak": (grid("""
kkkk.......
..jjjjkk...
.....iijjjj
"""), (10, 2)),
    "jolt": (grid("""
..k......
..jk..k..
...jkkj..
...ijjj..
....iijj.
"""), (7, 4)),
}
# Front view: two leaves with different bends (the left one long and curling up, the right one
# short and bent outward). The back view mirrors them so the same leaf stays on the same side.
LEAF_F = {
    "rest": (grid("""
k.......
jk....kk
.jj..jj.
..ijji..
"""), (3, 3)),
    "sway": (grid("""
.......k
kk....jk
.jj..jj.
..ijji..
"""), (3, 3)),
    "lift": (grid("""
k.......
jk.....k
.j....kj
.jj..jj.
..ijji..
"""), (3, 4)),
    # windup: three leaves fanned and bent like the side view's crest
    "flare": (grid("""
k..........
jk.......kk
.jj.....j..
..ij...jj..
...iiij....
"""), (4, 4)),
    # lunge: swept flat
    "flat": (grid("""
kk....k
.jjjjjk
"""), (3, 1)),
}

ROOTS = {
    "r": grid("""
r..r
r...
"""),
    # windup (front/back): root hairs dug in and splayed to both sides
    "brace": grid("""
rr.............rr
..r...........r..
"""),
    "f": grid("""
r.r.r
"""),
}
TAIL_UP = grid("""
.b.
.b.
r.r
""")  # back view: the root tail's tip, pointing at the camera


def back_view(g, cx):
    """The same silhouette seen from behind, split by the dark husk seam down its spine."""
    out = []
    for j, row in enumerate(g):
        if 1 <= j and row[cx] in "cd" and "i" not in row:
            row = row[:cx] + "b" + row[cx + 1:]
        out.append(row)
    return out


def side(g, face):
    """Resolve the side mask glyphs for a selected expression."""
    m = FACE_SIDE[face or "calm"]
    return ["".join(m.get(ch, ch) for ch in r) for r in g]


def frame(view, bname, face="calm", leaf="rest", dx=0, dy=0, roots=None, trail=False):
    """Assemble one 24x24 cell. view: 'r' (right), 'd' (front), 'u' (back)."""
    if view == "r":
        g, (lx, ly), cx = R[bname]
        fpos = None
    else:
        g, fpos, (lx, ly), cx = F[bname]
    bw, bh = len(g[0]), len(g)
    c = Canvas(CW, CH)
    bx = AX - cx + dx
    by = AY - bh + dy
    lg, (ax, ay) = (LEAF_R if view == "r" else LEAF_F)[leaf]
    if view == "u":
        lg, ax = mirror(lg), len(lg[0]) - 1 - ax
    c.paste(lg, bx + lx - ax, by + ly - ay)
    if view == "r":
        c.paste(side(g, face), bx, by)
    elif view == "u":
        c.paste(back_view(g, cx), bx, by)
        c.paste(TAIL_UP, bx + cx - 1, by + bh - 1)
    else:
        c.paste(g, bx, by)
        c.paste(FACE_F[face], bx + fpos[0], by + fpos[1])
    if trail:
        c.paste(SMEAR_TRAIL, bx - len(SMEAR_TRAIL[0]) + 1, by + 2, under=True)
    if roots == "brace":
        rg = ROOTS[roots]
        c.paste(rg, bx + cx - len(rg[0]) // 2, by + bh - 1, under=True)
    elif roots:
        rg = ROOTS[roots]
        c.paste(rg, bx + cx - len(rg[0]) // 2, by + bh)
    c.outline("o", skip=("r",))
    return c


def anims_for(view):
    """Assemble the reviewed directional hop, brace, attack and hurt sequences."""
    sfx = {"r": "right", "d": "down", "u": "up"}[view]
    F_ = lambda *a, **k: frame(view, *a, **k)
    rt = "r" if view == "r" else "f"
    back = -1 if view == "r" else 0
    out = []
    out.append((f"idle_{sfx}", [
        F_("N", "calm", "rest"),
        F_("N", "calm", "sway"),
        F_("SQ", "calm", "sway"),
        F_("N", "calm", "rest"),
        F_("N", "blink", "rest"),
        F_("N", "calm", "rest"),
    ], {"fps": 8, "durations": [10, 8, 10, 14, 4, 10]}))
    out.append((f"move_{sfx}", [
        F_("SQ", "calm", "sway"),
        F_("ST", "calm", "flat", dy=-2, roots=rt),
        F_("N", "calm", "lift", dy=-5, roots=rt),
        F_("N", "calm", "lift", dy=-5, roots=rt),
        F_("ST", "calm", "rest", dy=-2, roots=rt),
        F_("SQ", "calm", "sway"),
    ], {"fps": 12, "durations": [5, 3, 4, 4, 3, 5], "airborne": [1, 2, 3, 4],
        "notes": "one hop per loop (24 ticks). Frames 1-4 are airborne: move the entity only then, so the base never slides"}))
    if view == "r":
        wind = [
            F_("SQ", "calm", "sway", dx=back),
            F_("REAR", "hot", "flare", dx=back),
            F_("REAR", "teeth", "flare", dx=back),
            F_("REAR", "teeth", "flare", dx=back - 1),
            F_("REAR", "teeth", "flare", dx=back),
            F_("REAR", "teeth", "flare", dx=back - 1),
        ]
    else:
        # pulled back and braced: wide, low, roots splayed, leaves fanned (reads from behind too)
        pull = 1 if view == "u" else 0
        wind = [
            F_("SQ", "calm", "sway"),
            F_("BRACE", "hot", "flare", dy=pull, roots="brace"),
            F_("BRACE", "teeth", "flare", dy=pull, roots="brace"),
            F_("BRACE", "teeth", "flare", dx=1, dy=pull, roots="brace"),
            F_("BRACE", "teeth", "flare", dy=pull, roots="brace"),
            F_("BRACE", "teeth", "flare", dx=-1, dy=pull, roots="brace"),
        ]
    out.append((f"windup_{sfx}", wind, {"fps": 15, "durations": [4, 5, 4, 3, 4, 4],
        "notes": "telegraph: side view rears up; front/back views pull back into a wide braced crouch with roots dug in. "
                 "Thorns fan out, mask slits turn dried rose (matte), jaw opens, then a shiver. 24 ticks; hold the last frame if the windup runs longer"}))
    if view == "r":
        atk = [
            F_("LUNGE", "teeth", "flat", dx=1),
            F_("LUNGE", "teeth", "streak", dx=2, trail=True),
            F_("IMP", "hot", "jolt", dx=3),
            F_("N", "calm", "rest", dx=1),
        ]
    elif view == "d":
        atk = [
            F_("LUNGE", "teeth", "flat", dy=1),
            F_("LUNGE", "teeth", "flat", dy=2),
            F_("SQ", "hot", "flare", dy=2),
            F_("N", "calm", "rest", dy=1),
        ]
    else:
        # driving away from the camera: compressed narrow and tall, leaves swept flat behind
        atk = [
            F_("ST", "calm", "flat", dy=-2),
            F_("ST", "calm", "flat", dy=-3),
            F_("SQ", "calm", "lift", dy=-3),
            F_("N", "calm", "rest", dy=-1),
        ]
    out.append((f"attack_{sfx}", atk, {"fps": 15, "durations": [3, 3, 4, 8], "hit_frames": [0, 1, 2],
        "notes": "headbutt; frames 0-2 cover the engine's 10-tick lunge, frame 3 is the follow-through"}))
    out.append((f"hurt_{sfx}", [
        F_("HURT", "ouch", "jolt" if view == "r" else "lift", dx=-1 if view == "r" else 0),
        F_("SQ", "ouch", "sway"),
    ], {"fps": 10, "durations": [6, 6]}))
    return out


def build():
    """Write the authored sheet and its compatible animation manifest."""
    anims = anims_for("r") + anims_for("d") + anims_for("u")
    return build_sheet(
        "sproutling", (CW, CH), (AX, AY), anims, PAL, EMISSIVE,
        extra={"notes": "Style v2: pointed dried-rose seed bulb carried head-low on two hooked root canes, its hard ivory seed point leading as the ram. "
                        "One dark husk seam and one recessed slit; front and back views are an almond pointed at both ends with the sprout on top. "
                        "Upper-left rim light over a dark underside plane, open limb gaps, no glow. Reviewed hop, brace, lunge and hurt timings retained. "
                        "Cell dimensions, anchor and animation metadata retained; _right mirrors for left."},
    )


if __name__ == "__main__":
    sheet, manifest = build()
    print(f"{__file__}: {sheet.size}, {len(manifest['anims'])} animation rows")
