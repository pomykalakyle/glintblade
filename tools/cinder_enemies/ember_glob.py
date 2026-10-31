"""Ember glob: the totem's spit. A molten blob in a thin dark-red cooling skin,
folding between two lobes as it flies; the engine adds the trail, light and pop burst.

Cell 12x12, anchor (6,6) = blob center. Anims: fly (loop), pop (one-shot splash, optional).
"""

import pix

CW, CH = 12, 12
ANCHOR = (6, 6)

# Two unequal molten lobes joined by a hot neck, with one dark cooling crust plate riding the big
# lobe. The small lobe slides around the big one and is reabsorbed, so the melt folds and reforms
# instead of spinning as a shaded sphere. The hottest pixels stay deep in the big lobe.
FLY = [
    """
.rRr......
rROOr.....
rOYORrRRr.
.rROOOOORr
..rOYWYOOR
..ROYWWYOr
..r##OOOR.
...rrRRr..
""",
    """
...rRr....
..rROOr...
...rRORRr.
.rROOOOORr
rROYWWYOR.
rROYWYO#r.
.rRO##OOr.
..rrRRRr..
""",
    """
.....rRr..
..rRRROORr
.rROOOOORr
rROYYWYOOR
rROYWWYO#r
.rROOO##r.
..rRRrrr..
""",
    """
..........
..rRRRr...
.rROOOORr.
rROYWYOORr
rROYWWYO#r
.rROOO##Or
..rRRr.rOr
.......rr.
""",
]

POP = [
    """
..OY......
.......rRr
..rRRr.rOr
.rROYOr.r.
.ROYWYOR..
.rROYOR#r.
.O.rRRr...
.R........
""",
    """
.OY......YO
...........
R...rRRr...
R..rROORR.O
...ROYYOR.R
...rROORr..
...........
.O...rr..YO
.R.........
""",
    """
.R.........O
.r.........R
............
....r...r...
....r...r...
............
............
.R........OR
""",
    """
r..........r
r..........r
............
............
............
............
r.........rr
""",
]


def frame(text):
    """Center one authored grid on the glob's anchor."""
    g = pix.grid(text)
    cv = pix.Canvas(CW, CH)
    cv.stamp(text, ANCHOR[0] - len(g[0]) // 2, ANCHOR[1] - len(g) // 2)
    return cv


def all_anims():
    """Return the fly loop and pop splash with their original durations."""
    return [("fly", [frame(t) for t in FLY], {"durations": [4, 4, 4, 4]}),
            ("pop", [frame(t) for t in POP], {"durations": [3, 3, 4, 4]})]
