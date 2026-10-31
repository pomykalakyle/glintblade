"""Hand-authored ground feature stamps (light from upper left, 3/4 top-down).
Keys: 0..5 = rock ramp, a/b = aqua 3/4 (glow), v/V = violet 3/4, w = water 2, W = water 3, q = water 4."""
import os, sys
sys.path.insert(0, os.path.dirname(__file__))
from px import ascii_grid

K = {"0": "r0", "1": "r1", "2": "r2", "3": "r3", "4": "r4", "5": "r5",
     "A": "a1", "a": "a3", "b": "a4", "v": "v3", "V": "v4", "u": "v1",
     "w": "q2", "W": "q3", "q": "q4", "n": "q1", "g": "m2", "G": "m3", "h": "m4", "m": "m1"}

S = {}
# shallow hollow: dark near (upper) inner wall, lit far (lower) lip
S["hollow_s"] = [
    ".1111..",
    "122221.",
    ".24442.",
]
S["hollow_m"] = [
    "..11111....",
    ".1222221111",
    "122222222221",
    ".2222222244.",
    "..44442.....",
]
S["hollow_w"] = [
    "....111111......",
    "..112222222111..",
    ".12222222222221.",
    "122222222222222.",
    ".222222222222244",
    "..4442222244442.",
    ".....44444......",
]
# cracks: dark groove, occasional lit pixel on the lower edge
S["crack_a"] = [
    "11.........",
    "..111......",
    ".4..41111..",
    "......4..11",
    "...........",
]
S["crack_b"] = [
    "......1....",
    "1....1.....",
    ".11.1......",
    "..41111....",
    "....4..111.",
    "........4.1",
]
S["crack_c"] = [
    "..1....",
    "..1....",
    ".1.....",
    ".1.....",
    "1......",
    "4......",
]
# pebbles: lit top-left, dark bottom, shadow to lower right
S["pebble_1"] = [
    "54",
    "32",
    ".1",
]
S["pebble_2"] = [
    ".54.",
    "5443",
    "3332",
    ".221",
]
S["pebbles_a"] = [
    ".54.....",
    "5432....",
    ".321.54.",
    "..1..32.",
    ".......1",
]
S["pebbles_b"] = [
    "......54..",
    ".54..5442.",
    "4321.3332.",
    ".1....2211",
]
# a flat raised slab: lit top edge, dark front edge
S["slab"] = [
    "..44444444....",
    ".4333333333444",
    "43333333333332",
    "23333333333332",
    ".222222222221.",
    "..1111111111..",
]
# glowing bits (decals only, for near crystals/mushrooms)
S["shards_aqua"] = [
    "..b....",
    ".aA..a.",
    "..1.aA.",
    "......1",
]
S["shards_violet"] = [
    ".V.....",
    "vu...V.",
    "1...vu.",
    ".....1.",
]
S["lichen"] = [
    ".G.....",
    "gh.G...",
    "..g.gh.",
    "....g..",
]
S["puddle"] = [
    "...nnnnnn...",
    ".nnwwwwwwnn.",
    "nwwwwwWqwww4",
    ".4wwwwwwww4.",
    "..44444444..",
]

def stamp(name):
    return ascii_grid(S[name], K)
