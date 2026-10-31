"""Mock 2: a raised plateau with a cliff, and the island's south edge over a cloud sea."""
import tiles as TL
from pix import *


def cloud_backdrop(w, h):
    """Rough stand-in for the region's cloud sea (the region code draws the real one)."""
    img = new(w, h, "c2")
    bands = [(0, "c1"), (0.25, "c2"), (0.5, "c3"), (0.75, "c4")]
    for y in range(h):
        for x in range(w):
            t = y / h
            c = "c1"
            for th, col in bands:
                if t >= th:
                    c = col
            put(img, x, y, c)
    return img


def layout():
    W, H = 24, 16
    names = [["grass_0"] * W for _ in range(H)]
    # plateau: columns 3..9, top rows 1..4, face rows 5..6
    for y in range(1, 5):
        for x in range(3, 10):
            names[y][x] = "grass_%d" % (0 if TL.hash01(x, y) < 0.7 else int(TL.hash01(y, x) * 7) + 1)
    for x in range(3, 10):
        names[1][x] = "rim_n"
        names[4][x] = "grass_0"
    names[1][3] = "rim_nw"; names[1][9] = "rim_ne"
    for y in range(2, 5):
        names[y][3] = "rim_w"; names[y][9] = "rim_e"
    for x in range(3, 10):
        e = "l" if x == 3 else "r" if x == 9 else "c"
        names[5][x] = f"cliff_top_{e}" if e != "c" else f"cliff_top_c_{int(TL.hash01(x, 5) * 3)}"
        names[6][x] = f"cliff_mid_{e}" if e != "c" else f"cliff_mid_c_{int(TL.hash01(x, 6) * 3)}"
        names[7][x] = f"cliff_bot_{e}" if e != "c" else f"cliff_bot_c_{int(TL.hash01(x, 7) * 3)}"
    # island south edge from row 10
    for x in range(W):
        e = "l" if x == 0 else "r" if x == W - 1 else "c"
        c = f"c_{int(TL.hash01(x, 77) * 3)}"
        names[9][x] = f"edge_top_{e if e != 'c' else c}"
        names[10][x] = f"edge_mid_{e if e != 'c' else c}"
        names[11][x] = f"edge_haze_{e if e != 'c' else c}"
        names[12][x] = f"edge_under_{e if e != 'c' else c}"
    # some path across
    for x in range(12, 24):
        names[3][x] = "path_0"
        names[4][x] = "path_%d" % (x % 4)
    for x in range(12, 24):
        names[2][x] = "grass_path_0011" if x % 2 else "grass_path_0011_1"
        names[5][x] = "grass_path_1100" if x % 2 else "grass_path_1100_1"
    names[2][11] = "grass_path_0001"; names[3][11] = "grass_path_0101"; names[4][11] = "grass_path_0101_1"; names[5][11] = "grass_path_0100"
    return names


def render(names, bg):
    H, W = len(names), len(names[0])
    img = bg.copy()
    for y in range(H):
        for x in range(W):
            n = names[y][x]
            if n is None:
                continue
            if n.startswith(("rim", "cliff")):
                paste(img, TL.TILES["grass_0"], x * 16, y * 16)
            paste(img, TL.TILES[n], x * 16, y * 16)
    return img


if __name__ == "__main__":
    TL.build_all()
    names = layout()
    scene = render(names, cloud_backdrop(384, 256))
    paste(scene, stand_in_figure(), 230, 115)
    scene = scene[:216]
    save(scene, PREVIEWS / "_wip_bw_mock2_1x.png")
    save(scale(scene, 3), PREVIEWS / "_wip_bw_mock2_3x.png")
