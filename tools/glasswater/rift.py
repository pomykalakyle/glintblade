# Reshape the rift in map.txt into a wandering chasm (one-off edit helper).
import math, sys
p = sys.argv[1]
rows = [list(r) for r in open(p).read().splitlines()]
H = len(rows)
orig = [r[:] for r in rows]
for y in range(9, H):
    if y == 9:
        L, R = 49, 55
    else:
        c = 53 + 1.5 * math.sin(y * 0.29 + 0.4) + 0.7 * math.sin(y * 0.83 + 1.0)
        half = 3.0 + 0.9 * math.sin(y * 0.45 + 2.0) + 0.4 * math.sin(y * 1.3)
        L, R = round(c - half), round(c + half)
    for x in range(44, 62):
        if L <= x <= R:
            rows[y][x] = '.' if y in (15, 16) else ' '
        elif orig[y][x] == ' ':
            rows[y][x] = '#'
    # floors that used to meet the void keep meeting it
    if any(orig[y][x] == '.' for x in range(57, 60)):
        for x in range(R + 1, 58):
            rows[y][x] = '.'
    if any(orig[y][x] == '.' for x in range(46, 50)) and y not in (15, 16):
        lastfloor = max(x for x in range(40, 50) if orig[y][x] == '.')
        if lastfloor >= 48:
            for x in range(lastfloor, L):
                rows[y][x] = '.'
open(p, 'w').write('\n'.join(''.join(r) for r in rows) + '\n')
