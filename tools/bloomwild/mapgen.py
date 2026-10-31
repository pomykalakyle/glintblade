# Draft generator for the Bloomwild tile map. Prints ASCII to paste into
# src/regions/bloomwild/map.rs, which is then hand-tuned there.
#
# Legend (see map.rs): ' ' void, '.' grass, '"' tall grass, '=' flagstones, ':' dirt,
# 'o' slabs, 'A' altar dais, '#' cliff face, '~' deep water, '-' shallow, 'b' bridge,
# 's' stairs, 'F' waterfall over the cliff, 'f' waterfall off the island edge,
# 'w' low ruin wall, 'W' tall ruin wall, 'p' pillar, 'q' broken pillar, 'T' magenta tree,
# 'Y' coral tree, 'r' boulder, 'n' glyph niche in the cliff face, 'a' arch pillar.

W, H = 84, 52
g = [[' '] * W for _ in range(H)]


def put(x, y, c):
    if 0 <= x < W and 0 <= y < H:
        g[y][x] = c


def rect(x0, y0, x1, y1, c, only=None):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if 0 <= x < W and 0 <= y < H and (only is None or g[y][x] in only):
                put(x, y, c)


def ell(cx, cy, rx, ry, c, only=None):
    for y in range(H):
        for x in range(W):
            if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0:
                if only is None or g[y][x] in only:
                    put(x, y, c)


# island body: upper level rows 3..19, cliff faces 20..21, lower level 22..45
west_lower = [4, 4, 3, 3, 3, 4, 4, 5, 5, 4, 4, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5, 6, 7]
east_lower = [80, 80, 79, 79, 80, 80, 80, 81, 81, 80, 80, 79, 79, 79, 80, 80, 80, 79, 79, 78, 78, 77, 77, 76]
for i, y in enumerate(range(22, 46)):
    rect(west_lower[i], y, east_lower[i], y, '.')
# start landing juts out to the south-west
rect(5, 44, 13, 45, '.')
rect(6, 46, 12, 46, '.')
west_up = [22, 9, 7, 6, 5, 5, 5, 4, 4, 4, 5, 5, 5, 4, 4, 4, 4]
east_up = [28, 70, 74, 76, 77, 78, 78, 79, 79, 79, 80, 80, 80, 80, 80, 80, 80]
for i, y in enumerate(range(3, 20)):
    rect(west_up[i], y, east_up[i], y, '.')
for y in (20, 21):
    rect(4, y, 80, y, '#')

# ---------------- upper level
# plaza with altar dais and the gate approach on the north lip
ell(25, 10.5, 10.5, 6.5, 'o', only='.')
rect(22, 3, 28, 5, 'o')
ell(25, 10.5, 2.6, 1.8, 'A')
# colonnade ring
for (x, y, c) in [(16, 8, 'p'), (16, 13, 'q'), (19, 5, 'p'), (31, 5, 'p'), (34, 8, 'q'), (34, 13, 'p'),
                  (19, 16, 'p'), (31, 16, 'q'), (22, 3, 'p'), (28, 3, 'p')]:
    put(x, y, c)
# ruined tower on the west terrace and its broken wall
rect(7, 8, 9, 9, 'W')
rect(6, 10, 10, 10, 'w')
put(10, 9, 'w')
ell(9, 14, 4, 3.5, '"', only='.')
put(12, 17, 'T')
put(6, 5, 'Y')
# upper pool, stream, bridge, waterfall
ell(44, 9.5, 5.4, 4.2, '-', only='.')
ell(44, 9.5, 4.2, 3.2, '~')
rect(42, 13, 46, 19, '-', only='.')
rect(43, 13, 45, 19, '~')
rect(41, 16, 47, 17, 'b')
rect(43, 20, 45, 21, 'F')
put(38, 6, 'Y')
put(49, 12, 'r')
# grove
ell(58, 11, 7, 4, '"', only='.')
ell(74, 7, 4, 2.5, '"', only='.')
for (x, y, c) in [(52, 7, 'T'), (60, 5, 'Y'), (66, 9, 'T'), (56, 15, 'Y'), (63, 13, 'T'), (73, 12, 'Y'),
                  (76, 17, 'T'), (69, 5, 'Y')]:
    put(x, y, c)
for (x, y, c) in [(53, 11, 'q'), (59, 9, 'p'), (64, 17, 'q')]:
    put(x, y, c)
# upper path from the stairs west to the bridge and plaza
for x in range(35, 72):
    y = 17 if x < 48 else (16 if x < 62 else 17)
    rect(x, y, x, y, '=', only='."')
rect(70, 17, 70, 19, '=', only='."')

# ---------------- lower level
# splash pool and stream to the south edge
ell(44, 23.5, 4.8, 3.0, '-', only='.')
ell(44, 23.5, 3.6, 2.2, '~')
for i in range(8):
    x, y = 45 + i, 24 + i
    rect(x - 1, y, x + 3, y + 1, '-', only='.')
for i in range(8):
    x, y = 45 + i, 24 + i
    rect(x, y, x + 2, y + 1, '~')
rect(50, 31, 54, 45, '-')
rect(51, 31, 53, 45, '~')
rect(49, 36, 55, 37, 'b')
# stairs up the cliff on the east
rect(69, 20, 71, 21, 's')
# glyph niches in the retaining wall under the plaza
put(18, 21, 'n')
put(32, 21, 'n')
# start landing + arch, then the path north and east
rect(6, 41, 12, 45, '=', only='.')
rect(8, 34, 10, 40, '=', only='.')
for x in range(10, 50):
    y = 33 if x < 18 else (34 if x < 26 else (35 if x < 40 else 36))
    rect(x, y, x, y + 1, '=', only='.')
put(7, 38, 'a')
put(11, 38, 'a')
for x in range(55, 72):
    y = 36 if x < 60 else (34 if x < 64 else 32)
    rect(x, y, x, y + 1, '=', only='.')
rect(69, 22, 71, 33, '=', only='.')
# meadows (tall grass)
ell(29, 40, 9, 4.0, '"', only='.')
ell(27, 29, 8, 3.0, '"', only='.')
ell(12, 25, 6, 2.5, '"', only='.')
ell(63, 41, 6, 2.4, '"', only='.')
ell(76, 26, 3, 3, '"', only='.')
# courtyard walls in the meadow
rect(20, 31, 24, 31, 'w')
rect(31, 31, 37, 31, 'w')
rect(20, 32, 20, 33, 'w')
rect(37, 32, 37, 32, 'w')
for (x, y, c) in [(22, 38, 'p'), (34, 38, 'q'), (17, 29, 'q')]:
    put(x, y, c)
put(15, 39, 'T')
put(40, 42, 'Y')
put(5, 30, 'Y')
put(40, 27, 'T')
put(25, 44, 'r')
put(36, 25, 'r')
# east ruins: slab floor and a two-row colonnade
ell(66, 28, 9, 5, 'o', only='.=')
for x in (60, 63, 66, 72, 75):
    put(x, 25, 'p' if x not in (63, 75) else 'q')
for x in (60, 66, 69, 72):
    put(x, 31, 'p' if x != 69 else 'q')
rect(57, 39, 60, 39, 'W')
rect(57, 40, 57, 41, 'w')
put(77, 34, 'T')
put(58, 23, 'Y')
put(75, 41, 'T')
put(65, 43, 'r')
put(61, 44, 'q')

print('\n'.join(''.join(r) for r in g))
