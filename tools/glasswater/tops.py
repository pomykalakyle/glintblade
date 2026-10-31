"""Plateau-top tiles for raised rock (violet wall ramp), 16x16.

The top plane is w2 (one step above the face body, so solid rock reads as raised). Edges: 2-3 px lit lips
on N and W (w3 with short w4 runs), 2-3 px dark lips on E (w0/w1). Pixels where the floor beyond the edge
shows are transparent (-1): paint the floor first, then these tiles on top.
Straight edges have two lip patterns (rim 0/1) that meet seamlessly; interior variants 1-3 carry big,
low-contrast forms.
"""
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
from px import IDX as I  # noqa: E402

T = 16
FILL = "w2"
CH = {"0": "w0", "1": "w1", "2": "w2", "3": "w3", "4": "w4"}

# N edge: rows 0-1 are floor (transparent); rows 2-4 per variant. Columns 0 and 15 identical in both.
N_LIP = {
    0: ["3344433333344333",
        "3333332223333333",
        "2233222222333222"],
    1: ["3333334443334433",
        "3332233333333333",
        "2222222333322222"],
}
# W edge: per row (col0, col1, col2). Rows 0 and 15 identical in both.
W_LIP = {
    0: ["332", "332", "432", "432", "332", "322", "322", "332", "333", "333", "332", "432", "432", "332", "322", "332"],
    1: ["332", "333", "333", "332", "322", "322", "332", "432", "432", "432", "332", "332", "322", "332", "333", "332"],
}
# E edge: per row (col13, col14, col15). Rows 0 and 15 identical in both.
E_LIP = {
    0: ["210", "210", "100", "100", "210", "210", "210", "110", "100", "100", "210", "210", "210", "110", "210", "210"],
    1: ["210", "210", "210", "110", "100", "100", "210", "210", "210", "210", "110", "100", "100", "210", "210", "210"],
}
# interior forms, full-tile grids ('.' = keep the plane)
INTERIOR = {
    1: [  # tapered fracture: 2 px wide at the upper left, 1 px at the end; lit lower lip
        "................",
        "................",
        "................",
        "...11...........",
        "...1111.........",
        "...33111........",
        ".....33111......",
        ".......3311.....",
        ".........311....",
        "..........31....",
        "...........31...",
        "............3...",
        "................",
        "................",
        "................",
        "................",
    ],
    2: [  # rubble: three loose stones, lit tops, dark undersides
        "................",
        "................",
        "................",
        "................",
        "....33..........",
        "...3332.........",
        "...3221.........",
        "....11....33....",
        ".........3332...",
        ".........3221...",
        "..........11....",
        ".....33.........",
        "....3321........",
        ".....11.........",
        "................",
        "................",
    ],
    3: [  # a broad raised facet: lit upper-left plane, dark lower-right edge
        "................",
        "................",
        "................",
        "..3333333.......",
        "..33333333......",
        "..3333222333....",
        "...33222222.....",
        "...3222222221...",
        "....22222222....",
        "....22222221....",
        "....1111111.....",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
}


def top_tile(n, w, e, inner=(False, False, False, False), var=0, rim=0):
    c = np.full((T, T), I[FILL], np.int32)
    for y, row in enumerate(INTERIOR.get(var, [])):
        for x, ch in enumerate(row):
            if ch != ".":
                c[y, x] = I[CH[ch]]
    if n:
        c[0:2, :] = -1
        for j, row in enumerate(N_LIP[rim]):
            for x, ch in enumerate(row):
                c[2 + j, x] = I[CH[ch]]
    if w:
        for y, trip in enumerate(W_LIP[rim]):
            if n and y < 2:
                continue
            for i, ch in enumerate(trip):
                if not (n and y < 5 and i == 2):
                    c[y, i] = I[CH[ch]]
    if e:
        for y, trip in enumerate(E_LIP[rim]):
            if n and y < 2:
                continue
            for i, ch in enumerate(trip):
                c[y, 13 + i] = I[CH[ch]]
    if n and w:      # outer NW corner: rounded, lit
        c[2, 0:2] = -1
        c[3, 0] = -1
        c[2, 2] = I["w3"]; c[3, 1] = I["w3"]; c[4, 0] = I["w3"]
        c[3, 2] = I["w4"]; c[4, 1] = I["w3"]
    if n and e:      # outer NE corner: lit rim turns into the dark east lip
        c[2, 14:] = -1
        c[3, 15] = -1
        c[2, 13] = I["w3"]; c[3, 13] = I["w1"]; c[3, 14] = I["w0"]; c[4, 15] = I["w0"]; c[4, 14] = I["w0"]
    nw, ne, sw, se = inner
    if nw:   # plateau continues N and W, the NW diagonal is lower: lit concave notch
        c[0:3, 0] = I["w3"]; c[2, 1] = I["w3"]; c[3, 0] = I["w3"]; c[0, 1] = I["w3"]
    if ne:
        c[0:2, 15] = I["w0"]; c[0:2, 14] = I["w1"]; c[2, 15] = I["w3"]
    if sw:
        c[14:16, 0] = I["w3"]; c[15, 1] = I["w3"]
    if se:
        c[14:16, 15] = I["w0"]; c[15, 14] = I["w0"]; c[14, 14] = I["w1"]
    return c


def top_name(n, w, e, inner, var=0, rim=0):
    s = "cliff_top"
    rims = ("n" if n else "") + ("w" if w else "") + ("e" if e else "")
    if rims:
        s += "_" + rims
    ins = "".join(k for k, f in zip(("nw", "ne", "sw", "se"), inner) if f)
    if ins:
        s += "_in" + ins
    if var:
        s += f"_{var}"
    if rim:
        s += "_v1"
    return s


def all_top_variants():
    out = {}
    for n in (False, True):
        for w in (False, True):
            for e in (False, True):
                for bits in range(16):
                    nw, ne, sw, se = (bool(bits & 8), bool(bits & 4), bool(bits & 2), bool(bits & 1))
                    if (nw and (n or w)) or (ne and (n or e)) or (sw and w) or (se and e):
                        continue
                    inner = (nw, ne, sw, se)
                    out[top_name(n, w, e, inner)] = top_tile(n, w, e, inner)
    for (n, w, e) in [(True, False, False), (False, True, False), (False, False, True)]:
        out[top_name(n, w, e, (False,) * 4, rim=1)] = top_tile(n, w, e, rim=1)
    for v in (1, 2, 3):
        out[top_name(False, False, False, (False,) * 4, var=v)] = top_tile(False, False, False, var=v)
    return out
