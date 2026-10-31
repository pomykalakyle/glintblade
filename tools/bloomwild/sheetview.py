"""Preview helpers for the tile set: each transition tile shown in a 3x3 neighborhood."""
import tiles as TL
from pix import *


def neighborhood_sheet(prefix, path):
    TL.build_all()
    names = [n for n in TL.ORDER if n.startswith(prefix + "_")]
    cols = 6
    rows = (len(names) + cols - 1) // cols
    img = new(cols * 56, rows * 56, "o")
    for i, n in enumerate(names):
        cx, cy = (i % cols) * 56 + 4, (i // cols) * 56 + 4
        # the tile itself in the middle of 3x3 copies, to check seams with itself
        t = TL.TILES[n]
        img[cy + 16:cy + 32, cx + 16:cx + 32] = t
        # neighbors: pick compatible tiles by corner bits
        bits = n.split("_")[2]
    save(scale(img, 4), path)


if __name__ == "__main__":
    TL.build_all()
    names = TL.ORDER
    cols = 16
    rows = (len(names) + cols - 1) // cols
    img = new(cols * 18, rows * 18, "o")
    for i, n in enumerate(names):
        x, y = (i % cols) * 18 + 1, (i // cols) * 18 + 1
        img[y:y + 16, x:x + 16] = TL.TILES[n]
    save(scale(img, 4), PREVIEWS / "_wip_bw_tiles_all.png")
