"""Compose hero frames from hand-drawn parts + the procedural crystal blade + per-frame pixel fixes.

A frame spec (dict):
  dir         "down" | "right" | "up"
  hood        (variant | (row0, grid), dx, dy)
  body        (variant | (row0, grid), dx, dy)
  under       [(x, y, grid)] stamped before the body (feet behind the hem)
  extra       [(x, y, grid)] stamped after the hood (a foot stepping in front)
  arm         ((sx, sy), (hx, hy)) sleeve from shoulder to hand, drawn over the body
  blade       {k, hx, hy, length, glint, charged, behind, hilt}
  glove       glove color char at the grip (default "G"), or False for no hand
  eyes        {char: char} recolor (blink: {"e": "v"}; flash: {"e": "W"})
  pre         [(x, y, ch)] pixel fixes before the outline ("_" erases)
  post        [(x, y, ch)] pixel fixes after everything
  knot        (x, y) scarf knot override
  flip        mirror the finished frame
"""
from pix import Canvas
import blade as bl
import parts
import xf

# Frames are authored in a 40x40 layout (part grid origin at (12, 14), feet on y=31, anchor
# (20, 31)) and placed into a 48x48 cell at offset SHIFT so long blades never clip.
CELL = 48
SHIFT = (4, 2)
OX, OY = 12, 14  # where part grids' (0, 0) lands in authored coords
ANCHOR = (20 + SHIFT[0], 31 + SHIFT[1])  # in the 48x48 cell


# Colors folded into their neighbours (boot highlight -> boot, guard shadow -> glove dark, darkest
# scarf -> scarf mid, hood void -> outline: L 1.9 vs 3.7, indistinguishable).
REMAP = {"h": "G", "b": "G", "s": "S", "v": "o"}


def new_canvas():
    """Create one hero cell with the approved authored coordinate offset."""
    return Canvas(CELL, CELL, SHIFT[0], SHIFT[1])


def overlay(dst, src):
    """Copy visible source pixels over a destination cell."""
    for y in range(CELL):
        for x in range(CELL):
            if src.px[y][x] != ".":
                dst.px[y][x] = src.px[y][x]

# scarf knot (where the engine's code-drawn tail starts), in grid coords of the body part
KNOT = {"down": (8, 9), "right": (3, 9), "up": (8, 9),
        "down_right": (5, 9), "up_right": (5, 9)}


def grid_of(table, name):
    """Resolve a named hand-authored part into its offset and rows."""
    row0, g = table[name]
    return row0, xf.lines(g)


def line(x0, y0, x1, y1):
    """Return the integer Bresenham pixels between two authored points."""
    pts = []
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        pts.append((x0, y0))
        if x0 == x1 and y0 == y1:
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy
    return pts


def draw_arm(c, shoulder, hand, color="3", under="2"):
    """The sword arm's sleeve from shoulder to hand, drawn only where it leaves the cloak's
    silhouette (inside it the arm is implied): a lit top pixel with a shadowed pixel beneath, so
    the sleeve reads as a limb following the guard. The outline pass then edges it."""
    pts = line(shoulder[0], shoulder[1], hand[0], hand[1])[:-1]
    outside = [(x, y) for (x, y) in pts if c.get(x, y) == "."]
    for (x, y) in outside:
        c.put(x, y, color)
    for (x, y) in outside:
        if c.get(x, y + 1) == ".":
            c.put(x, y + 1, under)


RAMP = set("12345")


def relight(flipped, original):
    """A mirrored side view is lit from the upper right. Re-shade it for upper-left light: in
    each row, the cloak pixels take the original row's shades in left-to-right order, so the
    lit side moves back to screen-left while the mirrored silhouette stays."""
    for y in range(CELL):
        dst = [x for x in range(CELL) if flipped.px[y][x] in RAMP]
        src = [original.px[y][x] for x in range(CELL) if original.px[y][x] in RAMP]
        if len(dst) == len(src):
            for x, ch in zip(dst, src):
                flipped.px[y][x] = ch


def stamp_part(c, table, p):
    """Place a named or inline grid with its authored pose offsets."""
    name, dx, dy = p
    if isinstance(name, tuple):
        row0, g = name[0], xf.lines(name[1])
    else:
        row0, g = grid_of(table, name)
    c.stamp(g, OX + dx, OY + row0 + dy)


def scarf_knot(c, d, flipped):
    """The knot = where the engine's scarf tail leaves the neck, read off the finished frame's
    scarf pixels: the middle of the band's top row facing down/up, the back end of the band
    facing sideways."""
    pts = [(x, y) for y in range(CELL) for x in range(CELL) if c.px[y][x] in "TtSs"]
    if not pts:
        return None
    if d in ("right", "down_right", "up_right"):
        top = min(y for _, y in pts)
        band = [(x, y) for x, y in pts if y <= top + 1]
        return min(band)
    top = min(y for _, y in pts)
    xs = [x for x, y in pts if y == top]
    if len(xs) < 3:  # band hidden under a bowed hood
        return None
    return ((min(xs) + max(xs)) // 2, top)


def frame(spec):
    """Compose one approved pose and its scarf and blade metadata."""
    d = spec.get("dir", "down")
    body = new_canvas()
    for (gx, gy, g) in spec.get("under", []):
        body.stamp(g, gx, gy)
    if spec.get("body") is not None:
        stamp_part(body, parts.BODY[d], spec["body"])
    if spec.get("hood") is not None:
        stamp_part(body, parts.HOOD[d], spec["hood"])
    for (gx, gy, g) in spec.get("extra", []):
        body.stamp(g, gx, gy)
    b = spec.get("blade")
    if spec.get("arm"):
        s_, h_ = spec["arm"]
        draw_arm(body, s_, h_, spec.get("arm_color", "3"))
    glove = spec.get("glove", "G")
    if glove:
        gp = spec.get("hand") or ((b["hx"], b["hy"]) if b else None)
        # a hand raised behind the head/body is hidden by it
        hidden = b and b.get("behind") and body.get(gp[0], gp[1]) not in (".", "o") if gp else False
        if gp and not hidden:
            body.put(gp[0], gp[1], glove)
    eyes = spec.get("eyes")
    if eyes:
        for y in range(CELL):
            for x in range(CELL):
                ch = body.px[y][x]
                if ch in eyes:
                    body.px[y][x] = eyes[ch]
    for (x, y, ch) in spec.get("pre", []):
        if ch == "_":
            body.erase(x, y)
        else:
            body.put(x, y, ch)
    body.outline()
    flip = spec.get("flip", False)
    unflipped = body
    if flip:
        # Facing left: mirror the figure around the anchor column and re-shade its cloak for
        # upper-left light. The blade is then drawn natively at the mirrored hand and direction,
        # so its lit face also stays toward the upper left.
        body = body.flipped_about(ANCHOR[0])
        relight(body, unflipped)

    out = new_canvas()
    tip = None
    blayer = None
    if b:
        hx, hy, k = b["hx"], b["hy"], b["k"]
        if flip:
            hx, k = 2 * (ANCHOR[0] - SHIFT[0]) - hx, (8 - k) % 16
        blayer = new_canvas()
        tip = bl.draw(blayer, hx, hy, k, b.get("length"), b.get("glint"),
                      b.get("charged", 0), guard=b.get("guard", True), hilt=b.get("hilt", True),
                      dim=b.get("dim", False))
    if blayer and b.get("behind"):
        overlay(out, blayer)
    overlay(out, body)
    if blayer and not b.get("behind"):
        overlay(out, blayer)
    clipped = body.clipped + (blayer.clipped if blayer else 0)
    for (x, y, ch) in spec.get("post", []):
        if flip:
            x = 2 * (ANCHOR[0] - SHIFT[0]) - x
        if ch == "_":
            out.erase(x, y)
        else:
            out.put(x, y, ch)
    tip = spec.get("tip") if tip is None else tip
    if tip:
        tip = (tip[0] + SHIFT[0], tip[1] + SHIFT[1])
    for y in range(CELL):
        for x in range(CELL):
            ch = out.px[y][x]
            if ch in REMAP:
                out.px[y][x] = REMAP[ch]
    # the scarf knot, read off the unflipped body layer (the dimmed dead blade shares the
    # scarf's blues), then mirrored with the figure
    knot = spec.get("knot")
    if knot is not None:
        knot = (knot[0] + SHIFT[0], knot[1] + SHIFT[1])
    else:
        knot = scarf_knot(unflipped, d, False)
        if knot is None:
            bp = spec.get("body") or ("stand", 0, 0)
            kx, ky = KNOT[d]
            knot = (OX + kx + bp[1] + SHIFT[0], OY + ky + bp[2] + SHIFT[1])
    if flip:
        knot = (2 * ANCHOR[0] - knot[0], knot[1])
    # Native anatomical volume before the held sword is composited. A downward
    # non-emissive blade edge must never be inferred as a boot/contact surface.
    support = []
    for x in range(CELL):
        ys = [y for y in range(CELL) if body.px[y][x] != "."]
        support.append([min(ys), max(ys) + 1] if ys else None)
    return out, {"knot": list(knot), "tip": list(tip) if tip else None,
                 "support_spans": support, "clipped": clipped, "source_spec": spec}
