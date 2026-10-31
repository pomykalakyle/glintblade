"""Scratch: render a grid file (frames separated by blank lines) zoomed, for quick looks."""
import sys
import pathlib
import numpy as np
sys.path.insert(0, str(pathlib.Path(__file__).parent))
import pix

PAL = {
    "#": "#0c0610",  # outline
    "z": "#151024", "y": "#08060e", "x": "#251c40",
    "a": "#08060e", "b": "#151024", "c": "#251c40", "d": "#3c3068", "e": "#6658a8", "f": "#b0a4f0",
    "r": "#7a1418", "R": "#c8321a", "O": "#ff6a1c", "Y": "#ffb03a", "W": "#fff0a0",
}

src = pathlib.Path(sys.argv[1]).read_text()
blocks = [b for b in src.split("\n\n") if b.strip()]
arrs = []
for b in blocks:
    g = pix.grid(b)
    cv = pix.Canvas(len(g[0]) + 4, len(g) + 4)
    cv.stamp(g, 2, 2)
    cv.outline(skip='#zyx')
    arrs.append(cv.to_rgba(PAL))
out = pathlib.Path(sys.argv[2])
pix.strip(arrs, out, scale=int(sys.argv[3]) if len(sys.argv) > 3 else 10)
# also 1x and 3x
pix.strip(arrs, out.with_name(out.stem + "_3x.png"), scale=3)
