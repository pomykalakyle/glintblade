"""A fractured, slotted furnace fin: basalt planes, one outlet, no decorative face or bands."""

import pix

CW, CH = 28, 36
ANCHOR = (14, 33)
GX, GY = 4, 4

# The aperture and its chipped crown belong to the same waterworks as the region ruins.
BASE = """
.......DD...........
......DEDC..........
.....DEDDCC.........
.....DCDDCC.........
....DCC##CCC........
....DCB##CCCB.......
....DCB##CCCB.......
....DCB##CCCB.......
....DCB##CCCB.......
....DCB##CCCB.......
....DCB##CCCB.......
....DCB##CCCB.......
....DCB##CCCB.......
....DCB##CCCB.......
...DCCBBBBBCCCB.....
...DCBrrrrrrBCB.....
...DCBrOOOOOrCB.....
...DCBrrOOOrrCB.....
...DCCBBBBBBCCB.....
....DCCCCCCCB.......
....DCBrrOCCB.......
....DCBrrRCCB.......
....DCBrrrCCB.......
....DCBrrrCCB.......
....DCCBBCCCB.......
....DCCBCCCCB.......
....DCCBCCCCB.......
...DCCCCCCCCCB......
..DDCCCCCCCCCCB.....
..CCCCCCBBBBBBB.....
"""
# A dark slot lit from within: the ceiling under the upper lip stays in shadow, heat lines the slot
# floor and climbs its two inner ends. The first idle frame has no drip, so its lowest emissive
# cluster is the slot alone: (10, 16 on row 20; 10..16 on row 21) has exactly the baseline centroid
# (13, 20 7/9), which preserves the source's derived projectile origin.
MAW = {
    "inhale": ["...DCB#######CB.....", "...DCBR#####RCB.....", "...DCBRRRRRRRCB.....", "...DCCBBBBBBCCB....."],
    "smolder": ["...DCB#######CB.....", "...DCBO#####OCB.....", "...DCBOOOOOOOCB.....", "...DCCBBBBBBCCB....."],
    "glow": ["...DCB#######CB.....", "...DCBO#####OCB.....", "...DCBOOOYOOOCB.....", "...DCCBBBBBBCCB....."],
    "charged": ["..DDCB#rrrrr#CCB....", "..DCB#ROOOOOR#CB....", "..DCCBOYYYYYOBCB....", "...DCCBBBBBBCCB....."],
    "white": ["..DDCB#rRRRr#CCB....", "..DCB#ROYYYOR#CB....", "..DCCBOYWWWYOBCB....", "...DCCBBBBBBCCB....."],
}
def make(look, maw="smolder", eyes="idle", drip=None, channel="r", dx=0, dy=0, head_dy=0):
    """Move only the outlet block on recoil while the fin and its base remain planted."""
    rows = pix.grid(BASE)
    for i, row in enumerate(MAW[maw]):
        rows[15 + i] = row
    cv = pix.Canvas(CW, CH)
    cv.stamp("\n".join(rows), GX + dx, GY + dy)
    # Outlet lips move apart under pressure; the long slot remains a quiet dark aperture.
    if head_dy:
        for y in range(GY + 14, GY + 19):
            for x in range(GX + 2, GX + 16):
                cv.set(x + dx, y + dy, ".")
        cv.stamp("\n".join(rows[14:19]), GX + dx, GY + 14 + dy + head_dy)
    cv.set(13 + dx, 24 + dy, "O" if drip is not None else channel)
    return cv


def spray(cv, pts, dx=0, dy=0):
    """Place connected molten droplets directly beside the outlet."""
    for x, y, ch in pts:
        cv.set(GX + x + dx, GY + y + dy, ch)
    return cv


SPRAY_A = [(1, 15, "O"), (1, 16, "Y"), (17, 15, "O"), (17, 16, "R")]
SPRAY_B = [(0, 17, "R"), (0, 18, "O"), (18, 16, "O"), (18, 17, "R")]


def anims(look):
    """Keep the original timing contract with a widening outlet and a firm recoil."""
    idle = [make(look, "smolder"), make(look, "glow", drip=2),
            make(look, "smolder", drip=4), make(look, "inhale")]
    windup = [make(look, "inhale"), make(look, "charged", channel="R"),
              make(look, "white", channel="O"), make(look, "white", channel="O"),
              make(look, "white", channel="O")]
    a0 = spray(make(look, "white", channel="O", head_dy=-1), SPRAY_A)
    a1 = spray(make(look, "charged", channel="R"), SPRAY_B)
    attack = [a0, a1, make(look, "glow"), make(look, "inhale")]
    hurt = [make(look, "charged", dx=1), make(look, "smolder")]
    return [("idle", idle, {"durations": [12, 10, 12, 10]}),
            ("windup", windup, {"durations": [10, 8, 8, 7, 7]}),
            ("attack", attack, {"durations": [4, 6, 8, 10]}),
            ("hurt", hurt, {"durations": [6, 10]})]


def all_anims():
    """Preserve both original facing sets and their row order."""
    return [(f"{an}_{look}", frames, meta)
            for look in ("down", "up") for an, frames, meta in anims(look)]
