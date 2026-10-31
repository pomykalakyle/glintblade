"""Hand-placed damage for the Warden body (state "damaged"). Coordinates are body-cell pixels.
Cracks are a dark core with a 1 px lit lip on the lower-right wall (light from the upper left),
tapering to a single dark pixel at each end."""

# crack paths: list of (x, y, lip) where lip=True adds the lit lower-right wall
CRACKS = [
    # from the heart socket's left edge down across the chest
    [(47, 46, 1), (46, 47, 1), (46, 48, 1), (45, 49, 1), (44, 50, 1), (44, 51, 1), (43, 52, 1), (42, 53, 1),
     (42, 54, 1), (41, 55, 1), (40, 56, 1), (40, 57, 0), (39, 58, 0)],
    [(44, 50, 0), (43, 50, 1), (42, 49, 1), (41, 49, 1), (40, 48, 1), (39, 48, 0), (38, 47, 0)],
    # from the socket's top-right edge up and out across the shadowed chest
    [(66, 39, 1), (67, 38, 1), (68, 37, 1), (69, 37, 1), (70, 36, 1), (71, 35, 1), (72, 35, 1), (73, 34, 1),
     (74, 33, 0), (75, 33, 0)],
    [(72, 42, 1), (73, 43, 1), (74, 44, 1), (74, 45, 1), (75, 46, 1), (76, 47, 1), (77, 48, 0)],
    # across the front of the left shoulder, starting at the gilded band
    [(31, 28, 0), (30, 29, 1), (30, 30, 1), (29, 31, 1), (29, 32, 1), (28, 33, 1), (27, 34, 1), (27, 35, 0), (26, 36, 0)],
    # a short branch
    [(29, 32, 0), (30, 33, 1), (31, 34, 0)],
]

LIP = {"S1": "S2", "S2": "S3", "S3": "S4", "S4": "S5", "S5": "S5"}
CORE = {"S1": "S0", "S2": "S0", "S3": "S1", "S4": "S1", "S5": "S2"}


def crack(cv, path):
    on = {(x, y) for x, y, _ in path}
    for x, y, lip in path:
        u = cv.get(x, y)
        if u in CORE:
            cv.put(x, y, CORE[u])
        if lip and (x + 1, y) not in on and cv.get(x + 1, y) in LIP:
            cv.put(x + 1, y, LIP[cv.get(x + 1, y)])


def chip_right_shoulder(cv):
    """A chunk is broken out of the right shoulder's outer top corner: a stepped bite whose
    floor faces up (pale, freshly broken) and whose inner wall faces right (shadow)."""
    bite = [(97, 4), (117, 4), (117, 21), (111, 21), (110, 17), (107, 16), (106, 13), (103, 12), (102, 9), (98, 8)]
    cv.poly(bite, None)
    # fresh floors (face up): one pixel band just under each horizontal step
    for x0, x1, y in [(98, 101, 8), (102, 105, 12), (106, 109, 16), (110, 112, 20)]:
        for x in range(x0, x1 + 1):
            if cv.get(x, y) is not None:
                cv.put(x, y, "S4")
            if cv.get(x, y + 1) is not None:
                cv.put(x, y + 1, "S3")
    # inner walls (face right, into shadow)
    for x, y0, y1 in [(102, 9, 11), (106, 13, 15), (110, 17, 19)]:
        for y in range(y0, y1 + 1):
            if cv.get(x, y) is not None:
                cv.put(x, y, "S1")


def apply_patches(cv, state, frame):
    if state != "damaged":
        return
    chip_right_shoulder(cv)
    for path in CRACKS:
        crack(cv, path)
