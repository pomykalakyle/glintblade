"""Export exact opaque structural-base spans on the unchanged native sprite grid."""


def base_collision(image, depth):
    """Merge equal horizontal alpha runs across the lowest `depth` occupied rows.

    This selects the ground support rather than the tall facade/crown above it.
    Off-center roots and true gaps stay in source coordinates, just like anchors.
    """
    bounds = image.getbbox()
    if bounds is None:
        return []
    top, bottom = max(bounds[1], bounds[3] - depth), bounds[3]
    rectangles, active = [], {}
    for y in range(top, bottom):
        runs = []
        x = 0
        while x < image.width:
            if not image.getpixel((x, y))[3]:
                x += 1
                continue
            start = x
            while x < image.width and image.getpixel((x, y))[3]:
                x += 1
            runs.append((start, x))
        current = {}
        for run in runs:
            if run in active:
                r = active[run]
                rectangles[r][3] = y + 1
            else:
                r = len(rectangles)
                rectangles.append([run[0], y, run[1], y + 1])
            current[run] = r
        active = current
    assert all(image.getpixel((x, y))[3] for x0, y0, x1, y1 in rectangles
               for y in range(y0, y1) for x in range(x0, x1))
    return rectangles


def add_collision(anim, image, depth):
    rects = base_collision(image, depth)
    anim['collision_rects'] = rects
    if rects:
        anim['footprint'] = [max(r[2] for r in rects) - min(r[0] for r in rects),
                             max(r[3] for r in rects) - min(r[1] for r in rects)]
