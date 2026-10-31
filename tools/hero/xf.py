"""Pixel-exact transforms that make part variants from a base grid (no resampling: rows are
dropped, duplicated or slid whole, so every cluster keeps its shape)."""


def lines(g):
    """Normalize an authored part into individual row strings."""
    if isinstance(g, str):
        return [r for r in g.strip("\n").split("\n")]
    return list(g)


def squash(part, drop):
    """Remove local row `drop`; everything above moves down one (bottom stays put)."""
    row0, g = part
    g = lines(g)
    return (row0 + 1, g[:drop] + g[drop + 1:])


def stretch(part, dup):
    """Duplicate local row `dup`; everything above moves up one (bottom stays put)."""
    row0, g = part
    g = lines(g)
    return (row0 - 1, g[:dup + 1] + g[dup:])


def slide(part, rows, dx):
    """Slide the given local rows horizontally by dx (lean, sway)."""
    row0, g = part
    g = lines(g)
    out = []
    for i, r in enumerate(g):
        if i in rows:
            if dx > 0:
                r = "." * dx + r[:-dx] if r.endswith("." * dx) else "." * dx + r
            elif dx < 0:
                r = r[-dx:] + "." * (-dx) if r.startswith("." * (-dx)) else r[-dx:] + "." * (-dx)
        out.append(r)
    return (row0, out)


def widen(part, rows, left=1, right=1):
    """Grow the given rows outward by repeating their edge pixels (hem flare)."""
    row0, g = part
    g = lines(g)
    out = []
    for i, r in enumerate(g):
        if i in rows:
            chars = list(r)
            filled = [j for j, ch in enumerate(chars) if ch != "."]
            if filled:
                a, b = filled[0], filled[-1]
                for k in range(1, left + 1):
                    if a - k >= 0:
                        chars[a - k] = chars[a]
                for k in range(1, right + 1):
                    if b + k < len(chars):
                        chars[b + k] = chars[b]
            r = "".join(chars)
        out.append(r)
    return (row0, out)


def narrow(part, rows, left=1, right=1):
    """Trim selected edge pixels to make a narrower authored pose."""
    row0, g = part
    g = lines(g)
    out = []
    for i, r in enumerate(g):
        if i in rows:
            chars = list(r)
            filled = [j for j, ch in enumerate(chars) if ch != "."]
            if filled:
                a, b = filled[0], filled[-1]
                for k in range(left):
                    chars[a + k] = "."
                for k in range(right):
                    chars[b - k] = "."
            r = "".join(chars)
        out.append(r)
    return (row0, out)


def edit(part, fixes):
    """Set local pixels: fixes = [(x, local_row, ch)]."""
    row0, g = part
    g = [list(r) for r in lines(g)]
    for x, y, ch in fixes:
        while y >= len(g):
            g.append(list("." * len(g[0])))
        g[y][x] = ch
    return (row0, ["".join(r) for r in g])


def mirror(part):
    """Reverse each grid row without resampling the part."""
    row0, g = part
    return (row0, [r[::-1] for r in lines(g)])
