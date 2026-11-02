"""Tiny pixel-art toolkit for the hero sheet: char-grid canvases, outline pass, previews.

Every pixel is a palette char; '.' is transparent. Nothing is ever scaled, rotated or blurred
inside a frame: previews are nearest-neighbor enlargements of the finished 1:1 frames.
"""

from PIL import Image

# Hero palette (assets/concept/palettes_v2.json "hero"), one char per authored shade.
# The two deepest cloak steps and two darkest scarf steps share a v2 swatch.
# This preserves every approved cluster and left-facing relighting assignment.
PAL = {
    "o": "#262b40",  # outline
    "v": "#202435",  # hood void
    "1": "#303549",  # cloak shadow
    "2": "#303549",  # cloak shadow
    "3": "#414c64",  # cloak base
    "4": "#596984",  # cloak lit plane
    "5": "#81919e",  # cloak highlight
    "s": "#416974",  # scarf shadow
    "S": "#416974",  # scarf shadow
    "t": "#79a6ae",  # scarf base
    "T": "#b4d1ce",  # scarf highlight
    "g": "#37313e",  # gear shadow
    "G": "#62565b",  # gear base
    "h": "#93877e",  # gear highlight
    "b": "#817970",  # brass shadow
    "B": "#b4ae97",  # brass highlight
    "e": "#71d5db",  # eye (emissive)
    "c": "#429aa9",  # blade shadow (not emissive)
    "C": "#63d7e0",  # blade lit face (emissive)
    "w": "#baf5ec",  # blade ridge (emissive)
    "W": "#edf9ef",  # blade point / eye flash (emissive)
}
EMISSIVE_CHARS = set("eCwW")  # blade base "c" stays unlit so the glow starts above the guard
BLADE_CHARS = set("cCwW")


def hex_rgb(h):
    """Decode a hexadecimal palette swatch into an RGB tuple."""
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


RGB = {k: hex_rgb(v) for k, v in PAL.items()}


class Canvas:
    """Char grid. (ox, oy) = where authored coordinate (0, 0) sits in the raw grid, so frames
    authored for a 40x40 layout can live in a bigger cell without clipping."""

    def __init__(self, w, h, ox=0, oy=0):
        """Create a transparent char grid with an authored coordinate offset."""
        self.w, self.h = w, h
        self.ox, self.oy = ox, oy
        self.px = [["."] * w for _ in range(h)]
        self.clipped = 0

    def copy(self):
        """Copy all shade assignments while preserving the authored offset."""
        c = Canvas(self.w, self.h, self.ox, self.oy)
        c.px = [row[:] for row in self.px]
        return c

    def get(self, x, y):
        """Read an authored pixel or return transparency outside the canvas."""
        x += self.ox
        y += self.oy
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[y][x]
        return "."

    def put(self, x, y, ch):
        """Place a shade character and count any clipped authored pixel."""
        x += self.ox
        y += self.oy
        if ch == ".":
            return
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = ch
        else:
            self.clipped += 1

    def erase(self, x, y):
        """Clear an authored pixel without changing the remaining grid."""
        x += self.ox
        y += self.oy
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = "."

    def stamp(self, grid, x, y, flip=False, under=False, mapping=None):
        """Draw a grid with its top-left at (x, y). '.' is transparent.
        under=True only fills pixels that are still empty."""
        if isinstance(grid, str):
            grid = [r for r in grid.strip("\n").split("\n")]
        w = max(len(r) for r in grid)
        for j, row in enumerate(grid):
            row = row.ljust(w, ".")
            if flip:
                row = row[::-1]
            for i, ch in enumerate(row):
                if ch in (".", " "):
                    continue
                if mapping and ch in mapping:
                    ch = mapping[ch]
                if ch == "_":  # explicit erase
                    self.erase(x + i, y + j)
                    continue
                if under and self.get(x + i, y + j) != ".":
                    continue
                self.put(x + i, y + j, ch)

    def outline(self, color="o", skip=BLADE_CHARS, diag=False):
        """1px outer outline around every non-blade pixel (raw grid coords)."""
        def raw(x, y):
            """Read a raw grid cell or transparency outside the canvas."""
            if 0 <= x < self.w and 0 <= y < self.h:
                return self.px[y][x]
            return "."
        add = []
        nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        if diag:
            nb += [(1, 1), (-1, 1), (1, -1), (-1, -1)]
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y][x] != ".":
                    continue
                for dx, dy in nb:
                    ch = raw(x + dx, y + dy)
                    if ch != "." and ch not in skip and ch != color:
                        add.append((x, y))
                        break
        for x, y in add:
            self.px[y][x] = color

    def bbox(self):
        """Return the inclusive visible bounds or None for an empty grid."""
        xs, ys = [], []
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y][x] != ".":
                    xs.append(x)
                    ys.append(y)
        if not xs:
            return None
        return min(xs), min(ys), max(xs), max(ys)

    def flipped_about(self, ax):
        """Mirror around raw column ax (the anchor column stays put, like the engine's flip)."""
        c = Canvas(self.w, self.h, self.ox, self.oy)
        for y in range(self.h):
            for x in range(self.w):
                ch = self.px[y][x]
                if ch != ".":
                    nx = 2 * ax - x
                    if 0 <= nx < self.w:
                        c.px[y][nx] = ch
        return c

    def shifted(self, dx, dy):
        """Translate pixels by whole grid cells without resampling."""
        c = Canvas(self.w, self.h)
        for y in range(self.h):
            for x in range(self.w):
                ch = self.px[y][x]
                if ch != ".":
                    c.put(x + dx, y + dy, ch)
        return c

    def to_image(self):
        """Convert shade characters into fully opaque or transparent RGBA pixels."""
        im = Image.new("RGBA", (self.w, self.h), (0, 0, 0, 0))
        p = im.load()
        for y in range(self.h):
            for x in range(self.w):
                ch = self.px[y][x]
                if ch != ".":
                    p[x, y] = RGB[ch] + (255,)
        return im

    def rows(self):
        """Return the authored char grid as immutable row strings."""
        return ["".join(r) for r in self.px]


def scale(im, k):
    """Enlarge a preview by an integer nearest-neighbor factor."""
    return im.resize((im.width * k, im.height * k), Image.NEAREST)


def on_ground(im, ground):
    """Composite a sprite over a flat opaque ground swatch."""
    bg = Image.new("RGBA", im.size, hex_rgb(ground) + (255,))
    bg.alpha_composite(im)
    return bg
