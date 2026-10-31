"""Leg rasterizer for quadruped frames: draws a leg as joint-to-joint pixel lines so legs
keep the same length and thickness from frame to frame. Paws are 2 px, pointing forward."""


def line(x0, y0, x1, y1):
    """Line."""
    pts = []
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx = 1 if x0 < x1 else -1
    sy = 1 if y0 < y1 else -1
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


def leg(cv, joints, near=True, paw=1, thick_upper=True, lit="d", mid="c", far="z", only_empty=False):
    """joints: [(x, y), ...] from body to paw. Near legs: first segment 2 px wide (lit pixel on
    the left), rest 1 px of `mid`. Far legs: all `far` (a dark, un-outlined char).
    paw: +1 forward (right), -1 backward, 0 none."""
    for si in range(len(joints) - 1):
        (x0, y0), (x1, y1) = joints[si], joints[si + 1]
        for (x, y) in line(x0, y0, x1, y1):
            if only_empty and cv.get(x, y) != ".":
                continue
            if near:
                if si == 0 and thick_upper:
                    cv.set(x - 1, y, lit)
                    cv.set(x, y, mid)
                else:
                    cv.set(x, y, mid)
            else:
                cv.set(x, y, far)
    if paw:
        x, y = joints[-1]
        cv.set(x + paw, y, mid if near else far)
