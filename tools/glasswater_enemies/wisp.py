"""Wisp and wisp bolt (Glasswater Hollow).

Wisp: a small pale iris flame spirit with a chalk-bright core and one small dark face gap above it. It floats
(the engine draws its shadow on the floor and its flickering pale iris light), so the anchor is the
flame's base, not the ground; the curling tail hangs below the anchor into the hover gap.
Anims match src/entities/enemies/wisp.rs: idle (loop), windup (played over the 38-tick windup:
the flame pulls in tall and narrow while the core goes chalk-bright), attack (32-tick recoil:
the flame bursts outward, then cools back), hurt (12 ticks: the flame gutters).
Wisp bolt: a small fireball, direction-free (the engine moves it and draws its trail); anim fly.

Run: uv run --no-project --with pillow --with numpy python3 tools/glasswater_enemies/wisp.py
"""

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pix import Grid, Sheet, blank  # noqa: E402
import preview  # noqa: E402

# palettes_v2.json regions.glasswater.enemies.wisp.flame, plus the region outline for the face gap.
PAL = {
    "o": "#4d4866",  # flame rim (not emissive: reads as a dark edge, lit by the wisp's own light)
    "a": "#796c91",  # outer flame
    "b": "#a195b8",  # flame
    "c": "#cdc3db",  # hot
    "W": "#e2d8e9",  # chalk-bright core
    "k": "#222d3d",  # the face gap (region outline)
}
EMISSIVE = ["a", "b", "c", "W"]

W_ = 12  # authored frame width; every frame is 18 rows, body base on row 13


def rows(*parts):
    """Assemble an exact twelve-pixel-wide wisp frame from authored parts."""
    lines = []
    for p in parts:
        lines += [r for r in p.strip("\n").split("\n")]
    for r in lines:
        assert len(r) == W_, (len(r), r)
    assert len(lines) == 18, len(lines)
    return Grid.parse("\n".join(lines))


SPARE2 = "............\n............"
# --- idle parts: flame tops (5 rows), body (7 rows), tails (4 rows)
TOPS = [
    """
.....o......
....oao.....
....oabo..o.
...oabbo.oao
...oabbboabo""",
    """
......o.....
.....oao....
.....oabo...
...oabbbo.o.
...oabbbboao""",
    """
....o.......
...oao...o..
...oabo.oao.
...oabbooabo
...oabbbbabo""",
    """
.....oo.....
.....ao.....
....oabo.o..
...oabbooao.
...oabbbbabo""",
]
BODY = """
..oabbbbbbao
.oabbbcbbaao
oabbcccbbaao
oabbkkkcbaao
oabbcWWcbaao
.oabbccbaao.
..oaabaaao.."""
BODY_BLINK = """
..oabbbbbbao
.oabbbcbbaao
oabbcccbbaao
oabbbkkcbaao
oabbcWWcbaao
.oabbccbaao.
..oaabaaao.."""
TAILS = [
    """
...ooaaoo...
.....oao....
......oo....
.......o....""",
    """
...ooaaoo...
....oao.....
....oo......
....o.......""",
    """
...ooaaoo...
.....oao....
.....oo.....
......o.....""",
]

# --- windup: tall straight tongue (7 rows), narrow body heating up (7 rows), tail pulled up
TALL = [
    """
............
.....o......
....oao.....
....oabo....
...oabbo.o..
...oabbooao.
...oabbbbbo.""",
    """
.....o......
....oao.....
....oabo....
....oabo....
...oabbo.o..
...oabbooao.
...oabbbbbo.""",
    """
.....o......
....oco.....
....ocbo....
...oabco....
...oabbo.o..
...oacbooao.
...oabcbbbo.""",
]
NARROW = [
    """
..oabbbbao..
.oabbcbbbao.
.oabcccbbao.
.oabkkkcbao.
.oabcWWcbao.
..oabccbao..
...oaabao...""",
    """
..oabccbao..
.oabccccbao.
.oacWWWcbao.
.oackkkWcao.
.oabcWWcbao.
..oabccbao..
...oabbao...""",
    """
..oacccbao..
.oacWWWcbao.
.oacWWWWcao.
.oaWkkkWcao.
.oabcWWcbao.
..oabccbao..
...oabbao...""",
]
TAIL_UP = [
    """
....oaao....
.....oo.....
............
............""",
    """
....oaao....
....oo......
............
............""",
]

# --- attack (recoil): the flame bursts outward around a big white core, then cools
SPLAY = """
.o.......o..
.oao..o.oao.
..oaooaooao.
..oabbabbao.
..oabbbbbao."""
BURST = """
.oabcccccbao
oabcWWWWWcbo
oabcWWWWWcbo
oabWkkkWWcbo
oabccWWWccao
.oabcccccao.
..oabbbbao.."""
WARM = """
..oabccbbbao
.oabccccbaao
oabccWWccaao
oabckkkWcbao
oabccWWccbao
.oabbccbaao.
..oaabaaao.."""

# --- bolt
BOLT = [
    """
....o....
...oao...
..oabao..
.oabcbao.
.abcWcbo.
.oabcbao.
..oaao...
...oo....
.........""",
    """
.....o...
....oao..
..oabao..
.oabcbao.
.obcWcba.
.oabcbao.
..oabo...
...oo....
.........""",
    """
...o.....
..oao....
..oabo...
.oabcbao.
.abcWcbo.
.oabcbao.
...oao...
....oo...
.........""",
    """
....o....
...oao...
..oabbo..
.oabccbo.
.obcWWbo.
.oabccbo.
..oabao..
...ooo...
.........""",
]


def wisp_frame(g, dim=None):
    """Place the flame on its unchanged sixteen-by-twenty-four cell."""
    cell = blank(16, 24)
    cell.paste(g, 2, 2, remap=dim)
    return cell


DIM = {"W": "c", "c": "b", "b": "a"}
COOL = {"W": "c"}


def build_wisp():
    """Write the approved wisp poses in the style v2 iris ramp."""
    sheet = Sheet(
        "wisp",
        (16, 24),
        (8, 15),
        PAL,
        EMISSIVE,
        notes=(
            "Glasswater wisp: a pale iris flame spirit about 12x12 (flame tips to base), with a 4 px curling tail "
            "below the anchor. The anchor is the flame's base: wisp.rs draws it at the hover height and draws the "
            "shadow and the flickering pale iris light itself, so the sheet has no shadow and no painted glow. "
            "Non-directional anims named for wisp.rs: idle (flicker loop with a blink), windup (the flame pulls in "
            "tall and narrow, the core goes chalk-bright around a face gap that stays dark; stretched over the 38-tick windup), "
            "attack (32-tick recoil: burst then cool), hurt (12 ticks: the flame gutters). Flame ramp from "
            "palettes_v2.json crystal_violet plus fx.prism #e2d8e9 for the compact core; the darkest step #4d4866 is the rim and is not emissive; the single face gap uses the region outline and never smiles."
        ),
    )
    idle = [
        rows(SPARE2, TOPS[0], BODY, TAILS[0]),
        rows(SPARE2, TOPS[1], BODY, TAILS[2]),
        rows(SPARE2, TOPS[2], BODY, TAILS[1]),
        rows(SPARE2, TOPS[3], BODY, TAILS[2]),
        rows(SPARE2, TOPS[0], BODY_BLINK, TAILS[0]),
        rows(SPARE2, TOPS[1], BODY, TAILS[1]),
    ]
    sheet.add("idle", [wisp_frame(g) for g in idle], durations=[6, 6, 6, 6, 4, 6], extra={"loop": True})
    windup = [
        rows(TALL[0], NARROW[0], TAIL_UP[0]),
        rows(TALL[1], NARROW[0], TAIL_UP[1]),
        rows(TALL[1], NARROW[1], TAIL_UP[0]),
        rows(TALL[2], NARROW[1], TAIL_UP[1]),
        rows(TALL[2], NARROW[2], TAIL_UP[0]),
        rows(TALL[2], NARROW[2], TAIL_UP[1]),
    ]
    sheet.add("windup", [wisp_frame(g) for g in windup], durations=[6, 6, 6, 6, 7, 7])
    attack = [
        rows(SPARE2, SPLAY, BURST, TAILS[1]),
        rows(SPARE2, TOPS[3], WARM, TAILS[2]),
        rows(SPARE2, TOPS[0], WARM, TAILS[0]),
        rows(SPARE2, TOPS[1], BODY, TAILS[2]),
    ]
    sheet.add("attack", [wisp_frame(g) for g in attack], durations=[4, 6, 8, 14])
    hurt = [rows(SPARE2, TOPS[2], BODY_BLINK, TAILS[1]), rows(SPARE2, TOPS[0], BODY, TAILS[0])]
    sheet.add("hurt", [wisp_frame(hurt[0], DIM), wisp_frame(hurt[1], COOL)], durations=[5, 7])
    img, used = sheet.save()
    print("wisp colors:", len(used))
    sheet.light = {"color": "#a195b8", "radius": 44, "offset": [0, -5], "intensity": 0.62}
    preview.write_all(sheet, img, picks=[(0, 0), (1, 5), (2, 0), (3, 0)], board_cols=4)


def build_bolt():
    """Write the compact four-frame bolt in the shared iris threat ramp."""
    sheet = Sheet(
        "wisp_bolt",
        (12, 12),
        (6, 6),
        PAL,
        EMISSIVE,
        notes=(
            "Wisp bolt: a small pale iris fireball (about 7x8) with a chalk-bright core whose flame tip licks upward "
            "and flickers side to side. Direction-free: projectiles.rs moves it, adds its light and trail. "
            "Anchor = center of the ball."
        ),
    )
    frames = []
    for b in BOLT:
        g = Grid.parse(b.strip("\n"))
        c = blank(12, 12)
        c.paste(g, 2, 1)
        frames.append(c)
    sheet.add("fly", frames, durations=[4, 4, 4, 4], extra={"loop": True})
    img, used = sheet.save()
    print("bolt colors:", len(used))
    sheet.light = {"color": "#a195b8", "radius": 30, "offset": [0, 0], "intensity": 0.7}
    preview.write_all(sheet, img, picks=[(0, 0), (0, 3)], board_cols=1)


if __name__ == "__main__":
    build_wisp()
    build_bolt()
