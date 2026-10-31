"""Crystal beetle: a heavy armored beetle with a crest of pale iris crystals and a crystal horn.
It walks slowly, rears and paws the ground (windup), then charges in a straight line with its
horn leveled like a lance.

Run: uv run --no-project --with pillow --with numpy python3 tools/glasswater_enemies/beetle.py
Writes assets/sprites/crystal_beetle.{png,json} and previews in out/samples/enemies_glasswater_v2style/authored_previews/.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pix import Grid, Sheet, blank  # noqa: E402
import preview  # noqa: E402

# Style v2: slate armor and one pale iris mineral/threat family.
# The chalk core uses fx.prism's highlight; shell planes use Glasswater rock/wall ramps.
PAL = {
    "O": "#222d3d",  # tinted slate outline and legs
    "1": "#29313f",  # underside
    "2": "#364657",  # shell shadow
    "3": "#4c6070",  # shell middle plane
    "4": "#73858b",  # broad upper-left shell plane
    "5": "#a6b4ad",  # small matte shell highlight
    "b": "#4d4866",  # crystal root (unlit)
    "c": "#796c91",  # crystal shadow face
    "d": "#a195b8",  # crystal middle face
    "e": "#cdc3db",  # crystal lit face
    "W": "#e2d8e9",  # chalk iris charge tip
    "m": "#a195b8",  # iris eye
    "M": "#e2d8e9",  # eye flare
    "p": "#73858b",  # slate grit
}
EMISSIVE = ["c", "d", "e", "W", "m", "M"]

CELL = (32, 26)
GROUND = 20  # feet row (down view: the head and mandibles come forward, below it)
ANCHOR = (16, GROUND)

CREST = {
    "normal": {},
    "pulse": {"b": "c"},
    "bright": {"b": "c", "c": "d", "d": "e"},
    "hot": {"b": "c", "c": "d", "d": "e", "e": "W"},
    "dim": {"e": "d", "d": "c", "c": "b"},
    "off": {"e": "c", "d": "b", "c": "b"},  # dazed: the crest goes dark until it relights
}


def colors(crest="normal", eye=False):
    """Map crest energy and eye state onto the approved pose colors."""
    m = dict(CREST[crest])
    if eye == "shut":
        m["m"] = "1"
    elif eye:
        m["m"] = "M"
    return m


def line(g, a, b, c="O"):
    """8-connected line from a to b (Bresenham)."""
    (x0, y0), (x1, y1) = a, b
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x1 > x0 else -1), (1 if y1 > y0 else -1)
    err = dx + dy
    while True:
        g.set(x0, y0, c)
        if (x0, y0) == (x1, y1):
            return
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


def grit(g, pairs):
    """Kicked-up grit: each entry is a 2 px diagonal clump at (x, y), flying outward (left half of
    the cell: up-left, right half: up-right)."""
    for x, y in pairs:
        g.set(x, y, "p")
        g.set(x - 1 if x < CELL[0] // 2 else x + 1, y - 1, "p")


# ================================================================= right view
# Shell + swept crest. Light from the upper left: lit rim top-left, shadow toward the belly and
# the head end. Bottom outline is the last row; legs hang below it.
R_SHELL = Grid.parse("""
.....O............
....OeO...........
....OedO..........
....OedcO.........
.O...OedcO..O.....
OeO..OedcO.OeO....
OedO..OedcOOedO...
.OedO.OedcO.OedO..
.OedOOOObdcOOOedO.
..ObOO44bbbO44ObO.
.O4b55544bb4443bO.
O4555554444444332O
O455554444444333O.
O444444444443332O.
O34444444443322O..
O3334444332221O...
.O233333322211O...
..O1222211111O....
...OOOOOOOOOOO....
""")
# Head, neutral: horn at 45 degrees. Column 0 sits on shell column 14, row 0 on shell row 8.
R_HEAD = Grid.parse("""
.......OO
......OeO
.....OedO
..OOOdcO.
.O444dO..
O44433O..
O4333m2O.
O233222O.
O1222211O
.O11111O.
..OOOOO..
""")
# Head pitched down for the charge: horn leveled to 2:1 like a lance.
R_HEAD_LOW = Grid.parse("""
.......OOO
.....OOeeO
..OOOeddO.
.O444dcO..
O44433O...
O4333m2O..
O233222O..
O1222211O.
.O11111O..
..OOOOO...
""")
R_HEAD_AT = {False: (14, 8), True: (14, 6)}

# Legs: hip column on the shell grid (hind, middle, front under the shell, far front under the
# head) and a foot offset per pose. The hip sits right under the body; the foot is on GROUND.
R_HIPS = [3, 7, 11, 17]
R_POSES = {
    "stand": [-1, 0, 1, 1],
    "stepA": [-2, 1, 0, 2],  # tripod A reaching forward
    "stepB": [0, -1, 2, 0],  # tripod B reaching forward
    "brace": [-2, -1, 1, 2],  # windup: planted wide
    "dig": [-3, -1, 1, 2],  # hind leg scraped back
    "dig2": [0, -1, 1, 2],  # hind leg pulled forward for the next scrape
    "gallopA": [-3, 1, 2, 3],  # stretched: hind push-off, front reaching
    "gallopB": [0, -2, 0, 1],  # gathered
}


def right(legs="stand", dx=0, dy=0, low=False, head_dy=0, crest="normal", eye=False, kick=()):
    """One right-facing frame. dy moves the body (negative = up); the feet stay on GROUND."""
    ox = 5 + dx  # shell column 0 in the cell
    oy = GROUND - 2 - 18 + dy  # standing: legs are 2 px tall under the shell's bottom outline
    m = colors(crest, eye)
    body = blank(*CELL)
    body.paste(R_SHELL, ox, oy, remap=m)
    hx, hy = R_HEAD_AT[low]
    body.paste(R_HEAD_LOW if low else R_HEAD, ox + hx, oy + hy + head_dy, remap=m)
    g = blank(*CELL)
    for hip, fdx in zip(R_HIPS, R_POSES[legs]):
        x = ox + hip
        top = max(y for y in range(CELL[1]) if body.get(x, y)) + 1
        line(g, (x, min(top, GROUND)), (x + fdx, GROUND))
    g.paste(body, 0, 0)
    grit(g, kick)
    return g


# ================================================================= down view (facing the camera)
D_SHELL = Grid.parse("""
.........O..........
........OeO.........
........OedO........
....O...OedcO..O....
...OeO..OedcO.OeO...
...OedO.OedcOOedO...
....OedOOedcOedO....
....OedOOedcOedO....
..OO4OedOedcOdO3OO..
.O4555bbObdbOb3332O.
O45555544bbb4333322O
O445554444444333222O
O444444444443332221O
O344444444433322221O
O334444443333222111O
.O2333333332221111O.
..OO122222211111OO..
....OO11111111OO....
......OOOOOOOO......
""")
# Head with the gem horn pointing at the camera, eyes on its sides, mandibles below.
D_HEAD = Grid.parse("""
.....OO.....
....OeeO....
....OedO....
...OeddcO...
..OOeddcOO..
.O4OObdOO3O.
O4m44bb432mO
O3444443322O
O2333333221O
.OO122211OO.
.O.OO11OO.O.
..O.OOOO.O..
""")
# Head lowered (charge): the horn points straight at the camera and reads as a short bright gem.
D_HEAD_LOW = Grid.parse("""
............
............
.....OO.....
....OeeO....
..OOeddcOO..
.O4OObdOO3O.
O4m44bb432mO
O3444443322O
O2333333221O
.OO122211OO.
.O.OO11OO.O.
..O.OOOO.O..
""")
D_HEAD_AT = (4, 12)

# ================================================================= up view (facing away)
# Same dome from behind, with the seam between the wing cases running down the back.
U_SHELL = D_SHELL.copy()
for _y, _c in [(11, "2"), (12, "2"), (13, "2"), (14, "1"), (15, "1"), (16, "1")]:
    U_SHELL.set(10, _y, _c)

# Back view, loaded for the charge (windup_up, attack_up): front of the shell dipped so the dome is
# flatter and the crest foreshortens, abdomen pulled back toward the camera, wider stance.
U_LOAD = Grid.parse("""
..........O...........
.........OeO..........
.........OedO.........
.....O...OedcO..O.....
....OeO..OedcO.OeO....
....OedOOObdbOOedO....
...OOOedO5bbbOedOOO...
.OO455bb544444bb332OO.
O45555544444443333322O
O44555444442444333222O
O34444444442433332221O
.O333333333232222111O.
..OO12222221211111OO..
....OOO11111111OOO....
.......OOOOOOOO.......
""")
U_LOAD_Y = 6  # sits 6 px lower than the standing shell: squat, rear toward the camera
# (hip, knee, foot) per leg, left side, in U_LOAD coords: front, middle, hind (braced, pushing back).
U_LOAD_POSES = {
    "brace": [((-1, 8), (-2, 7), (-3, 5)), ((-1, 10), (-2, 10), (-4, 11)), ((1, 12), (0, 13), (-2, 15))],
    "dig": [((-1, 8), (-2, 7), (-3, 5)), ((-1, 10), (-2, 10), (-4, 11)), ((1, 12), (0, 13), (-1, 17))],
    "gallopA": [((-1, 8), (-2, 7), (-3, 4)), ((-1, 10), (-2, 10), (-3, 12)), ((1, 12), (0, 13), (-1, 16))],
    "gallopB": [((-1, 8), (-2, 7), (-3, 7)), ((-1, 10), (-2, 10), (-4, 10)), ((1, 12), (0, 13), (-2, 14))],
}


def loaded_up(legs="brace", legs_r=None, dy=0, crest="normal", kick=()):
    """Back view in the loaded charge stance."""
    ox, oy = 5, U_LOAD_Y
    g = blank(*CELL)
    for side, pose in ((1, legs), (-1, legs_r or legs)):
        for hip, knee, foot in U_LOAD_POSES[pose]:
            fx = (lambda x: x) if side == 1 else (lambda x: U_LOAD.w - 1 - x)
            line(g, (ox + fx(hip[0]), oy + dy + hip[1]), (ox + fx(knee[0]), oy + dy + knee[1]))
            line(g, (ox + fx(knee[0]), oy + dy + knee[1]), (ox + fx(foot[0]), oy + foot[1]))
    g.paste(U_LOAD, ox, oy + dy, remap=colors(crest))
    grit(g, kick)
    return g


D_W = 20  # shell width (mirror axis for the right-hand legs)

# Vertical-view legs, left side (the right side is mirrored): (hip, knee, foot) per leg in shell
# coords. Hip and knee move with the body; the foot stays on the ground.
# Down view, top to bottom: hind, middle, front (the front legs reach forward, toward the camera).
D_POSES = {
    "stand": [((-1, 10), (-2, 9), (-3, 10)), ((-1, 13), (-2, 13), (-3, 15)), ((3, 18), (2, 19), (1, 21))],
    "stepA": [((-1, 10), (-2, 9), (-3, 11)), ((-1, 13), (-2, 13), (-3, 14)), ((3, 18), (2, 19), (1, 22))],
    "stepB": [((-1, 10), (-2, 9), (-3, 9)), ((-1, 13), (-2, 13), (-3, 16)), ((3, 18), (2, 19), (2, 21))],
    "brace": [((-1, 10), (-2, 9), (-4, 10)), ((-1, 13), (-2, 13), (-4, 15)), ((3, 18), (2, 19), (0, 21))],
    "dig": [((-1, 10), (-2, 9), (-3, 7)), ((-1, 13), (-2, 13), (-4, 15)), ((3, 18), (2, 19), (0, 21))],
    "gallopA": [((-1, 10), (-2, 9), (-4, 8)), ((-1, 13), (-2, 13), (-3, 15)), ((3, 18), (2, 19), (1, 23))],
    "gallopB": [((-1, 10), (-2, 9), (-3, 11)), ((-1, 13), (-2, 13), (-4, 14)), ((3, 18), (2, 19), (2, 20))],
}
# Up view, top to bottom: front, middle, hind (the hind legs are nearest the camera).
U_POSES = {
    "stand": [((-1, 10), (-2, 9), (-3, 8)), ((-1, 13), (-2, 13), (-3, 14)), ((1, 16), (0, 17), (-1, 19))],
    "stepA": [((-1, 10), (-2, 9), (-3, 7)), ((-1, 13), (-2, 13), (-3, 15)), ((1, 16), (0, 17), (-1, 18))],
    "stepB": [((-1, 10), (-2, 9), (-3, 9)), ((-1, 13), (-2, 13), (-3, 13)), ((1, 16), (0, 17), (0, 20))],
    "brace": [((-1, 10), (-2, 9), (-4, 9)), ((-1, 13), (-2, 13), (-4, 14)), ((1, 16), (0, 17), (-2, 19))],
    "dig": [((-1, 10), (-2, 9), (-4, 9)), ((-1, 13), (-2, 13), (-4, 14)), ((1, 16), (0, 17), (0, 21))],
    "twitch": [((-1, 10), (-2, 9), (-3, 8)), ((-1, 13), (-2, 12), (-3, 13)), ((1, 16), (0, 17), (-1, 19))],
    "gallopA": [((-1, 10), (-2, 9), (-3, 7)), ((-1, 13), (-2, 13), (-4, 14)), ((1, 16), (0, 17), (0, 21))],
    "gallopB": [((-1, 10), (-2, 9), (-3, 10)), ((-1, 13), (-2, 13), (-3, 13)), ((1, 16), (0, 17), (-1, 18))],
}
D_POSES["twitch"] = [D_POSES["stand"][0], ((-1, 13), (-2, 12), (-3, 13)), D_POSES["stand"][2]]


def vertical(view, legs="stand", legs_r=None, dy=0, low=False, head_dy=0, crest="normal", eye=False, kick=()):
    """One down- or up-facing frame. dy moves the body (negative = up); feet stay planted.
    legs_r: pose for the right-hand legs (default: same as the left)."""
    ox, oy = 6, GROUND - 20  # shell origin in the cell, standing
    m = colors(crest, eye)
    table = D_POSES if view == "down" else U_POSES
    g = blank(*CELL)
    for side, pose in ((1, legs), (-1, legs_r or legs)):
        for hip, knee, foot in table[pose]:
            fx = (lambda x: x) if side == 1 else (lambda x: D_W - 1 - x)
            p0 = (ox + fx(hip[0]), oy + dy + hip[1])
            p1 = (ox + fx(knee[0]), oy + dy + knee[1])
            p2 = (ox + fx(foot[0]), oy + foot[1])
            line(g, p0, p1)
            line(g, p1, p2)
    if view == "down":
        g.paste(D_SHELL, ox, oy + dy, remap=m)
        g.paste(D_HEAD_LOW if low else D_HEAD, ox + D_HEAD_AT[0], oy + dy + D_HEAD_AT[1] + head_dy, remap=m)
    else:
        g.paste(U_SHELL, ox, oy + dy, remap=m)
    grit(g, kick)
    return g


def main():
    """Author the beetle sheet without changing its animation contract."""
    sheet = Sheet(
        "crystal_beetle",
        CELL,
        ANCHOR,
        PAL,
        EMISSIVE,
        notes=(
            "Style v2 Glasswater beetle: slate shell and legs, pale iris crystal crest and eyes. "
            "Approved silhouettes and all pose durations are preserved, including the braced rear windup. "
            "Matte shell uses Glasswater rock/wall ramps; crest uses crystal_violet, with #e2d8e9 "
            "from fx.prism only at charge tips and eye flares. Cyan is reserved for Glint. "
            "Crystal faces and eyes retain their emissive roles; the shell, roots and legs stay matte. "
            "Feet stay on the anchor row; _right is mirrored for left."
        ),
    )
    R = right
    sheet.add("idle_right", [R(), R(crest="pulse"), R(crest="pulse", head_dy=1), R(head_dy=1)], fps=4)
    sheet.add("move_right", [R(legs="stepA"), R(dy=-1), R(legs="stepB"), R(dy=-1, head_dy=1)], fps=8)
    sheet.add(
        "windup_right",
        [
            R(legs="brace", dy=-1, head_dy=-1, crest="pulse"),
            R(legs="brace", dy=1, low=True, crest="bright"),
            R(legs="dig", dy=1, low=True, crest="bright", kick=[(4, 19), (2, 16)]),
            R(legs="dig2", dy=1, low=True, crest="hot", eye=True),
            R(legs="dig", dy=1, dx=-1, low=True, crest="hot", eye=True, kick=[(3, 19), (1, 15)]),
            R(legs="brace", dy=1, low=True, crest="hot", eye=True),
        ],
        durations=[7, 6, 5, 5, 5, 6],
    )
    sheet.add(
        "attack_right",
        [
            R(legs="gallopA", dx=1, low=True, crest="hot", eye=True, kick=[(4, 19), (2, 16)]),
            R(legs="gallopB", dx=1, dy=1, low=True, crest="hot", eye=True, kick=[(2, 17)]),
            R(legs="gallopA", dx=1, low=True, crest="hot", eye=True, kick=[(5, 18), (2, 15)]),
            R(legs="gallopB", dx=1, dy=1, low=True, crest="hot", eye=True, kick=[(3, 16)]),
        ],
        durations=[3, 5, 3, 5],
    )
    sheet.light = {"color": "#cdc3db", "radius": 30, "offset": [-2, -12], "intensity": 0.8}
    sheet.add("hurt_right", [R(legs="brace", dx=-2, head_dy=-1, crest="dim", eye=True), R(dx=-1, crest="dim")], durations=[6, 8])
    sheet.add(
        "dazed_right",
        [
            R(legs="stepA", dy=1, low=True, crest="off", eye="shut"),
            R(legs="gallopB", dy=1, low=True, head_dy=1, crest="off", eye="shut"),
            R(legs="stepB", dy=1, low=True, crest="off", eye="shut"),
            R(legs="gallopA", dy=1, low=True, head_dy=1, crest="off", eye="shut"),
        ],
        durations=[8, 8, 8, 8],
        extra={"loop": True},
    )

    for view in ("down", "up"):
        V = lambda **k: vertical(view, **k)  # noqa: E731
        back = -1 if view == "down" else 1  # screen direction of "backward"
        # grit flies backward from the hind feet: up the screen facing down, down it facing up
        if view == "down":
            dig_l, dig_r = [(3, 5), (1, 2)], [(28, 5), (30, 2)]
            run1, run2 = [(2, 7), (29, 5)], [(3, 5), (28, 7)]
        else:
            dig_l, dig_r = [(5, 23), (3, 25)], [(26, 23), (28, 25)]
            run1, run2 = [(5, 23), (27, 24)], [(6, 24), (26, 23)]
        sheet.add(
            f"idle_{view}",
            [V(), V(crest="pulse"), V(crest="pulse", legs="twitch", head_dy=1), V(legs_r="twitch", head_dy=1)],
            fps=4,
        )
        sheet.add(
            f"move_{view}",
            [V(legs="stepA", legs_r="stepB"), V(dy=-1), V(legs="stepB", legs_r="stepA"), V(dy=-1, head_dy=1)],
            fps=8,
        )
        if view == "up":
            L = loaded_up
            sheet.add(
                "windup_up",
                [
                    V(legs="brace", dy=-1, crest="pulse"),
                    L(crest="bright"),
                    L(legs="dig", crest="bright", kick=[(3, 25)]),
                    L(legs_r="dig", crest="hot", kick=[(28, 25)]),
                    L(legs="dig", crest="hot", kick=[(3, 25)]),
                    L(crest="hot"),
                ],
                durations=[7, 6, 5, 5, 5, 6],
            )
            sheet.add(
                "attack_up",
                [
                    L(legs="gallopA", legs_r="gallopB", crest="hot", kick=[(4, 24)]),
                    L(legs="gallopB", legs_r="gallopA", dy=1, crest="hot", kick=[(27, 23)]),
                    L(legs="gallopA", legs_r="gallopB", crest="hot", kick=[(5, 23)]),
                    L(legs="gallopB", legs_r="gallopA", dy=1, crest="hot", kick=[(26, 24)]),
                ],
                durations=[3, 5, 3, 5],
            )
            sheet.add("hurt_up", [V(legs="brace", dy=back, crest="dim", eye=True), V(crest="dim")], durations=[6, 8])
            sheet.add(
                "dazed_up",
                [
                    V(legs="stepA", legs_r="twitch", dy=1, crest="off"),
                    V(legs="twitch", legs_r="stepB", dy=1, crest="off"),
                    V(legs="stepB", legs_r="twitch", dy=1, crest="off"),
                    V(legs="twitch", legs_r="stepA", dy=1, crest="off"),
                ],
                durations=[8, 8, 8, 8],
                extra={"loop": True},
            )
            continue
        sheet.add(
            f"windup_{view}",
            [
                V(legs="brace", dy=-1, head_dy=-1, crest="pulse"),
                V(legs="brace", dy=1, low=True, crest="bright"),
                V(legs="dig", legs_r="brace", dy=1, low=True, crest="bright", kick=dig_l),
                V(legs="brace", legs_r="dig", dy=1, low=True, crest="hot", eye=True, kick=dig_r),
                V(legs="dig", legs_r="brace", dy=1, low=True, crest="hot", eye=True, kick=dig_l),
                V(legs="brace", dy=1, low=True, crest="hot", eye=True),
            ],
            durations=[7, 6, 5, 5, 5, 6],
        )
        sheet.add(
            f"attack_{view}",
            [
                V(legs="gallopA", legs_r="gallopB", low=True, crest="hot", eye=True, kick=run1),
                V(legs="gallopB", legs_r="gallopA", dy=1, low=True, crest="hot", eye=True, kick=run2),
                V(legs="gallopA", legs_r="gallopB", low=True, crest="hot", eye=True, kick=run2),
                V(legs="gallopB", legs_r="gallopA", dy=1, low=True, crest="hot", eye=True, kick=run1),
            ],
            durations=[3, 5, 3, 5],
        )
        sheet.add(
            f"hurt_{view}",
            [V(legs="brace", dy=back, head_dy=-1, crest="dim", eye=True), V(crest="dim")],
            durations=[6, 8],
        )
        sheet.add(
            f"dazed_{view}",
            [
                V(legs="stepA", legs_r="twitch", dy=1, low=True, crest="off", eye="shut"),
                V(legs="twitch", legs_r="stepB", dy=1, low=True, head_dy=1, crest="off", eye="shut"),
                V(legs="stepB", legs_r="twitch", dy=1, low=True, crest="off", eye="shut"),
                V(legs="twitch", legs_r="stepA", dy=1, low=True, head_dy=1, crest="off", eye="shut"),
            ],
            durations=[8, 8, 8, 8],
            extra={"loop": True},
        )
    img, used = sheet.save(extra_json={"light": {"color": "#cdc3db", "radius": 30, "offset": [-2, -12]}})
    print("colors used:", len(used))
    preview.write_all(sheet, img, picks=[(0, 0), (6, 0), (12, 0), (2, 5), (14, 3)], board_cols=5)


if __name__ == "__main__":
    main()
