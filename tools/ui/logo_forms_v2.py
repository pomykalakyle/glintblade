"""Private native-grid GLINTBLADE letterforms, independent of the display font."""

import numpy as np
from PIL import Image, ImageDraw


def polygon(size, points):
    """Rasterize an authored integer polygon without scaling or antialiasing."""
    image = Image.new("1", size)
    ImageDraw.Draw(image).polygon(points, fill=1)
    return np.array(image, dtype=bool)


def title_letter(ch):
    """Construct a narrow 26-pixel waterworks capital on its own native grid."""
    width = {"G": 13, "L": 10, "I": 3, "N": 13, "T": 13,
             "B": 13, "A": 15, "D": 13, "E": 11}[ch]
    size = (width, 26)
    mask = np.zeros((26, width), bool)
    if ch == "G":
        mask |= polygon(size, [(4, 0), (12, 0), (12, 2), (5, 2),
                               (2, 5), (2, 21), (4, 23), (9, 23),
                               (9, 16), (7, 16), (7, 13), (12, 13),
                               (12, 22), (9, 25), (3, 25), (0, 22), (0, 4)])
    elif ch == "L":
        mask |= polygon(size, [(1, 0), (2, 0), (2, 23), (9, 23),
                               (7, 25), (0, 25), (0, 1)])
    elif ch == "I":
        mask |= polygon(size, [(1, 0), (2, 0), (2, 24), (1, 25),
                               (0, 25), (0, 1)])
    elif ch == "N":
        mask[:, :3] = True
        mask[:, 10:] = True
        mask |= polygon(size, [(1, 0), (3, 0), (11, 21), (11, 25),
                               (9, 25), (1, 4)])
    elif ch == "T":
        mask |= polygon(size, [(0, 0), (12, 0), (10, 2), (7, 2),
                               (7, 25), (5, 25), (5, 2), (0, 2)])
    elif ch == "B":
        mask |= polygon(size, [(0, 0), (8, 0), (11, 3), (11, 9),
                               (9, 12), (12, 16), (12, 22), (9, 25), (0, 25)])
        mask &= ~polygon(size, [(3, 3), (7, 3), (8, 5), (8, 9),
                                (7, 11), (3, 11)])
        mask &= ~polygon(size, [(3, 16), (8, 16), (9, 18), (9, 21),
                                (8, 23), (3, 23)])
    elif ch == "A":
        mask |= polygon(size, [(6, 0), (8, 0), (14, 25), (11, 25),
                               (7, 7), (3, 25), (0, 25)])
        mask[18:21, 4:11] = True
    elif ch == "D":
        mask |= polygon(size, [(0, 0), (6, 0), (12, 6), (12, 20),
                               (7, 25), (0, 25)])
        mask &= ~polygon(size, [(3, 3), (5, 3), (9, 7), (9, 19),
                                (6, 23), (3, 23)])
    elif ch == "E":
        mask |= polygon(size, [(0, 0), (10, 0), (8, 2), (2, 2),
                               (2, 11), (8, 11), (6, 13), (2, 13),
                               (2, 23), (10, 23), (8, 25), (0, 25)])
    return mask


# The credit mark is redrawn at its own grid, never reduced from the title.
# A keeps a two-row crossbar and a separate open upper counter at this size.
SMALL = {
    "G": ["..#####", ".######", "###....", "##.....", "##.....", "##.....",
          "##..###", "##...##", "##...##", "###..##", ".######", "..####."],
    "L": [".##...", "###...", "###...", "###...", "###...", "###...",
          "###...", "###...", "###...", "######", "#####.", "####.."],
    "I": [".##", "###", "###", "###", "###", "###",
          "###", "###", "###", "###", "###", "##."],
    "N": ["##...##", "###..##", "###..##", "####.##", "####.##", "##.####",
          "##.####", "##.####", "##..###", "##..###", "##...##", "##...##"],
    "T": ["#######", "######.", "..###..", "..###..", "..###..", "..###..",
          "..###..", "..###..", "..###..", "..###..", "..###..", "..###.."],
    "B": ["######.", "#######", "##...##", "##...##", "##..###", "######.",
          "######.", "##...##", "##...##", "##...##", "#######", "######."],
    "A": ["...###...", "...###...", "..##.##..", "..##.##..", "..##.##..", ".##...##.",
          ".##...##.", ".#######.", ".#######.", "##.....##", "##.....##", "##.....##"],
    "D": ["#####..", "######.", "##..###", "##...##", "##...##", "##...##",
          "##...##", "##...##", "##...##", "##..###", "######.", "#####.."],
    "E": ["######", "#####.", "##....", "##....", "##....", "#####.",
          "####..", "##....", "##....", "##....", "######", "#####."],
}


def compose(small=False):
    """Space the ten exact letters optically and return their mask, cut, and bounds."""
    letters = list("GLINTBLADE")
    glyphs = [np.array([[c == "#" for c in row] for row in SMALL[ch]], bool)
              if small else title_letter(ch) for ch in letters]
    # I opens the LI/IN rhythm; LA retains a background slit beyond the cast edge.
    gaps = [4, 4, 5, 4, 5, 4, 4, 4, 4] if small else [9, 9, 10, 8, 9, 9, 8, 9, 9]
    h = 12 if small else 26
    width = sum(g.shape[1] for g in glyphs) + sum(gaps)
    word = np.zeros((h, width), bool)
    cut = np.zeros_like(word)
    bounds = []
    x = 0
    for i, (ch, glyph) in enumerate(zip(letters, glyphs)):
        w = glyph.shape[1]
        word[:, x:x + w] = glyph
        bounds.append((ch, x, w))
        if ch == "B":
            # One rising blade incision crosses the waist of B, away from A's crossbar.
            for dx in range(2 if small else 3, w):
                y = (6 - dx // 6) if small else (15 - (dx - 3) // 2)
                cut[y, x + dx] = True
                if not small:
                    cut[y - 1, x + dx] = True
                    if dx >= w - 4:
                        word[y - 1:y + 1, x + dx] = False
        x += w + (gaps[i] if i < len(gaps) else 0)
    return word, cut, bounds
