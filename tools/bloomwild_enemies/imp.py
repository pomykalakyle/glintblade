"""Style v2 sage bramble imp with a long seed-husk mask and matte held seed.

Hand-authored mask grids and joint paths preserve the reviewed crawl and overhead throw.
Run from the repository root with uv run --no-project --with pillow python3 tools/bloomwild_enemies/imp.py.
"""

import math

from pix import Canvas, grid, build_sheet

PAL = {
    "o": "#303b4d",
    "w": "#3b535b", "x": "#465b61", "y": "#647c77", "z": "#829b8e",
    "u": "#927c91", "v": "#bf9da9",
    "M": "#747d8d", "N": "#a1a4a5", "O": "#c2c1b3", "Q": "#dfdac1",
    "Y": "#bf9da9", "Z": "#dfdac1",  # matte seed seam, distinct from husk detection
    "s": "#4d505f", "S": "#747678", "k": "#a3a38f",
}
EMISSIVE = []

CW, CH = 32, 32
AX, AY = 16, 29
OX, OY = 6, 9  # local pose coords -> cell coords (local y 19 = ground row 28)


def line(p0, p1):
    """Trace a one-pixel Bresenham segment between integer joints."""
    (x0, y0), (x1, y1) = p0, p1
    pts = []
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        pts.append((x0, y0))
        if (x0, y0) == (x1, y1):
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy
    return pts


def path(joints):
    """Polyline through joints with L-corners removed (pixel-perfect 1 px line)."""
    pts = []
    for a, b in zip(joints, joints[1:]):
        seg = line(a, b)
        pts += seg if not pts else seg[1:]
    out = []
    for p in pts:
        if len(out) >= 2:
            a, b = out[-2], out[-1]
            if abs(p[0] - a[0]) == 1 and abs(p[1] - a[1]) == 1 and b[0] in (a[0], p[0]) and b[1] in (a[1], p[1]):
                out.pop()
        out.append(p)
    return out


def merge(c, l):
    """Composite nontransparent indexed pixels from one canvas onto another."""
    for y in range(c.h):
        for x in range(c.w):
            if l.px[y][x] != ".":
                c.px[y][x] = l.px[y][x]


# ----------------------------------------------------------------------------- masks
MASK = {
    "r": grid("""
..QO..
.QQON.
QQOON.
QOoON.
QooOM.
QOoOM.
.OONM.
.OONM.
..ONM.
..NM..
...M..
"""),
    "r_hot": grid("""
..QO..
.QQON.
QQOON.
QOoON.
QooOM.
QOoOM.
.OONM.
.OoNM.
..ONM.
..NM..
...M..
"""),
    "r_shut": grid("""
..QO..
.QQON.
QQOON.
QOOON.
QOoOM.
QOOOM.
.OONM.
.OONM.
..ONM.
..NM..
...M..
"""),
    "d": grid("""
..QO..
.QQON.
QQOONM
QOoONM
QoOoOM
QOoOoM
.OOON.
.OONM.
..ONM.
..NM..
...M..
"""),
    "d_hot": grid("""
..QO..
.QQON.
QQOONM
QOoONM
QoOoOM
QOoOoM
.OOoN.
.OONM.
..ONM.
..NM..
...M..
"""),
    "d_shut": grid("""
..QO..
.QQON.
QQOONM
QOOONM
QOoOOM
QOOOoM
.OOON.
.OONM.
..ONM.
..NM..
...M..
"""),
    "u": grid("""
..NOM.
.NyxwM
NyxxwM
OyxxwM
NxxwwM
.xxww.
..wwM.
...M..
"""),
}
MASK["u_hot"] = MASK["u_shut"] = MASK["u"]

SEED = {
    # Held seed: the same matte rose split and ivory seam as the projectile.
    "a": grid("""
.Sk.
SkYk
sSZS
.ss.
"""),
}


# ----------------------------------------------------------------------------- figure
def fig(p):
    """Draw a pose dict (local coords). Keys: spine, near_leg, far_leg, near_arm, far_arm,
    thorns [(x, y)], mask (variant, x, y), optional seed (x, y), smear [(x, y, ch)],
    near_on_top (outline the near arm separately so it reads over the body)."""
    c = Canvas(CW, CH)
    P = lambda pts: [(x + OX, y + OY) for x, y in pts]
    for x, y in path(P(p["far_arm"])):
        c.set(x, y, "w")
    for x, y in path(P(p["far_leg"])):
        c.set(x, y, "w")
    for x, y in path(P(p.get("far_arm2", []))) if p.get("far_arm2") else []:
        c.set(x, y, "x")
    # spine: 2 px, lit on top
    sp = path(P(p["spine"]))
    for x, y in sp:
        c.set(x, y, "x")
        c.set(x, y + 1, "x")
    for x, y in p.get("chest", []):
        c.set(x + OX, y + OY, "x")
    for x, y in p.get("chest_dark", []):
        c.set(x + OX, y + OY, "w")
    # torso mass: fill each column from the spine down to the belly line, shadow on the underside
    if p.get("belly"):
        top, bot = {}, {}
        for x, y in sp:
            top[x] = min(top.get(x, 99), y)
        for x, y in path(P(p["belly"])):
            bot[x] = max(bot.get(x, -99), y)
        for x in top:
            if x in bot and bot[x] > top[x]:
                for y in range(top[x], bot[x] + 1):
                    c.set(x, y, "x")
                c.set(x, bot[x], "w")
    # arm sweep (smear): part of the body layer, so it shares the silhouette and outline
    for x, y, ch in p.get("sweep", []):
        if c.get(x + OX, y + OY) == ".":
            c.set(x + OX, y + OY, ch)
    for x, y in sp:
        if c.get(x, y - 1) == ".":
            c.set(x, y, "y")
    for x, y in p.get("hi", []):
        c.set(x + OX, y + OY, "z")
    # thorns: (x, y) is the thorn tip; a darker base sits under it toward the spine
    for x, y, *rest in p["thorns"]:
        c.set(x + OX, y + OY, rest[0] if rest else "v")
    for x, y in path(P(p["near_leg"][:2])):
        if c.get(x + 1, y) in (".", "w"):
            c.set(x + 1, y, "x")  # thigh: 2 px
    for x, y in path(P(p["near_leg"])):
        c.set(x, y, "y")
    if not p.get("near_on_top"):
        for x, y in path(P(p["near_arm"][:2])):
            if c.get(x, y + 1) in (".", "w"):
                c.set(x, y + 1, "x")  # upper arm: 2 px
        for x, y in path(P(p["near_arm"])):
            c.set(x, y, "y")
    for x, y, ch in p.get("claws", []):
        c.set(x + OX, y + OY, ch)
    c.outline("o")
    mv, mx, my = p["mask"]
    m = Canvas(CW, CH)
    m.paste(MASK[mv], mx + OX, my + OY)
    m.outline("o")
    merge(c, m)
    if p.get("near_on_top"):
        a = Canvas(CW, CH)
        for x, y in path(P(p["near_arm"][:2])):
            a.set(x, y + 1, "x")
        for x, y in path(P(p["near_arm"])):
            a.set(x, y, "y")
        if p.get("seed"):
            sx, sy = p["seed"]
            a.paste(SEED["a"], sx + OX, sy + OY)
        a.outline("o")
        merge(c, a)
    elif p.get("seed"):
        sx, sy = p["seed"]
        a = Canvas(CW, CH)
        a.paste(SEED["a"], sx + OX, sy + OY)
        a.outline("o")
        merge(c, a)
    for x, y, ch in p.get("smear", []):
        c.set(x + OX, y + OY, ch)
    return c


def sweep(sh, hand_from, hand_to, thick=4):
    """Smear for a throwing arm: the band the forearm swept through, from the cocked position to the
    current one, about the shoulder. Thin (1 px) at the start, `thick` px at the hand. Local coords."""
    sx, sy = sh
    a0 = math.atan2(hand_from[1] - sy, hand_from[0] - sx)
    a1 = math.atan2(hand_to[1] - sy, hand_to[0] - sx)
    r = math.hypot(hand_to[0] - sx, hand_to[1] - sy)
    out = []
    for y in range(int(sy - r - 2), int(sy + r + 3)):
        for x in range(int(sx - r - 2), int(sx + r + 3)):
            a = math.atan2(y - sy, x - sx)
            f = (a - a0) / (a1 - a0) if a1 != a0 else 0
            if not 0 <= f <= 0.92:
                continue
            d = math.hypot(x - sx, y - sy)
            t = 1 + (thick - 1) * f
            if r - t <= d <= r:
                out.append((x, y, "y" if d > r - 1.2 else "x"))
    return out


def seed_at_hand(hand):
    """Top-left for the held seed grid so it sits in the hand, just ahead of the fingers."""
    return (hand[0] - 1, hand[1] - 3)


def shift(pts, dx, dy):
    """Move every integer joint by the supplied offset."""
    return [(x + dx, y + dy) for x, y in pts]


# ----------------------------------------------------------------------------- right-facing poses
def spine_thorns(spine_pts, every=(1, 3, 5), lift=1, alt=True):
    """Thorn tips above chosen spine pixels (local coords), alternating colors."""
    pts = path(spine_pts)
    out = []
    for k, i in enumerate(every):
        if i < len(pts):
            x, y = pts[i]
            out.append((x - 1, y - lift - 1, "v" if (k % 2 == 0 or not alt) else "u"))
            out.append((x, y - lift, "u"))
    return out


def crawl(sdy=0, hx=0, mask="r", mdx=0, mdy=0, near_hand=(14, 19), far_hand=(9, 19),
          near_foot=((5, 19), (7, 19)), far_foot=((1, 19), (3, 19)), near_lift=0, far_lift=0, bristle=0):
    """Crawling stance. sdy moves the back/shoulders (hands and feet stay planted)."""
    spine = [(3, 13), (3, 10 + sdy), (5, 8 + sdy), (8, 7 + sdy), (10, 8 + sdy)]
    sh = (10, 10 + sdy)
    nh = (near_hand[0], near_hand[1] - near_lift)
    fh = (far_hand[0], far_hand[1] - far_lift)
    near_arm = [sh, ((sh[0] + nh[0]) // 2 + 2, (sh[1] + nh[1]) // 2 - 1), (nh[0] - 1, nh[1] - 1), nh]
    far_arm = [(9, 10 + sdy), ((9 + fh[0]) // 2 + 1, (10 + sdy + fh[1]) // 2), (fh[0] + 1, fh[1] - 1), fh]
    near_leg = [(4, 14), (7, 16), near_foot[0], near_foot[1]]
    far_leg = [(3, 14), (1, 16), far_foot[0], far_foot[1]]
    thorns = [(1, 8 + sdy - bristle, "v"), (2, 9 + sdy, "u"), (4, 6 + sdy - bristle, "v"), (4, 7 + sdy, "u"),
              (7, 5 + sdy - bristle, "v"), (7, 6 + sdy, "u"), (1, 12, "v"), (2, 12, "u")]
    chest = [(7, 9 + sdy), (8, 9 + sdy), (9, 9 + sdy), (8, 10 + sdy), (9, 10 + sdy)]
    chest_dark = [(6, 9 + sdy), (7, 10 + sdy)]
    belly = [(4, 13), (5, 11 + sdy), (8, 10 + sdy), (10, 11 + sdy)]
    hi = [(4, 9 + sdy), (5, 8 + sdy), (6, 8 + sdy)]
    return dict(spine=spine, near_arm=near_arm, far_arm=far_arm, near_leg=near_leg, far_leg=far_leg,
                chest=chest, chest_dark=chest_dark, belly=belly, hi=hi,
                thorns=thorns, mask=(mask, 10 + mdx, 1 + sdy + mdy),
                claws=[(nh[0] - 2, nh[1], "y")] if near_lift == 0 else [])


def rear(lean=0, arm="cock", mask="r_hot", seed=True, bristle=1, sdx=0, sdy=0, far="brace"):
    """Reared up on the hind legs, throwing arm cocked back over the head."""
    spine = [(4, 14), (4, 11), (5, 8), (7, 6), (9, 5)]
    spine = [(x + (lean if i > 1 else 0) + sdx, y + (sdy if i > 0 else 0)) for i, (x, y) in enumerate(spine)]
    top = spine[-1]
    chest = [(x + 1, y) for x, y in path(spine[1:])] + [(x + 1, y + 1) for x, y in path(spine[2:])]
    chest_dark = [(x + 2, y + 1) for x, y in path(spine[1:4])]
    if arm == "cock":
        near_arm = [(top[0] - 1, top[1] + 2), (top[0] - 4, top[1] + 1), (top[0] - 6, top[1] - 2), (top[0] - 6, top[1] - 4)]
        seed_at = (top[0] - 8, top[1] - 7)
    else:
        near_arm = [(top[0] + a, top[1] + b) for a, b in arm]
        seed_at = None
    if far == "brace":
        far_arm = [(top[0], top[1] + 3), (top[0] + 3, top[1] + 5), (top[0] + 5, top[1] + 4)]
    else:
        far_arm = [(top[0] + a, top[1] + b) for a, b in far]
    thorns = [(2 + sdx, 10 + sdy - bristle, "v"), (3 + sdx, 10 + sdy, "u"),
              (3 + sdx + lean, 7 + sdy - bristle, "v"), (4 + sdx + lean, 7 + sdy, "u"),
              (5 + sdx + lean, 4 + sdy - bristle, "v"), (6 + sdx + lean, 5 + sdy, "u"),
              (2 + sdx, 13, "v"), (3 + sdx, 13, "u")]
    return dict(spine=spine, near_arm=near_arm, far_arm=far_arm, chest=chest, chest_dark=chest_dark,
                near_leg=[(5 + sdx, 15), (8 + sdx, 17), (6, 19), (8, 19)], far_leg=[(4 + sdx, 15), (2 + sdx, 17), (2, 19), (4, 19)],
                thorns=thorns, mask=(mask, top[0], top[1] - 6), near_on_top=True,
                seed=seed_at if seed else None)


def throw(phase, mask="r_hot"):
    """Throw: 0 = whip (arm over the top, smear arc), 1 = release (arm forward), 2 = follow-through."""
    if phase == 0:
        p = rear(lean=1, arm=[(-1, 2), (3, -2), (7, -5)], seed=False, mask=mask, bristle=1)
        top = p["spine"][-1]
        sh = (top[0] - 1, top[1] + 2)
        hand = (top[0] + 7, top[1] - 5)
        cocked = (top[0] - 6, top[1] - 4)
        p["sweep"] = sweep(sh, cocked, hand)
        p["seed"] = seed_at_hand(hand)
        p["near_on_top"] = False  # the arm swings past behind the mask, so the face stays clear
        return p
    if phase == 1:
        c = crawl(sdy=-1, mdx=1, mask=mask, near_hand=(17, 13), near_lift=0, bristle=0)
        c["near_arm"] = [(10, 9), (13, 10), (16, 11), (18, 11)]
        c["release"] = (19, 11)
        c["claws"] = [(19, 10, "y"), (19, 12, "y")]
        c["near_on_top"] = True
        return c
    c = crawl(sdy=1, mdx=1, mdy=1, mask="r", near_hand=(16, 19))
    return c


def hurt(phase):
    """Assemble the imp recoil and recovery key poses."""
    if phase == 0:
        p = rear(lean=-1, arm=[(-1, 2), (-4, 4), (-6, 3)], far=[(0, 3), (3, 2), (5, 0)], seed=False,
                 mask="r_shut", bristle=2, sdx=-1, sdy=2)
        return p
    return crawl(sdy=1, mask="r_shut", mdx=-1, bristle=1)


# ----------------------------------------------------------------------------- front / back views
def fb(view, sdy=0, mdx=0, mdy=0, arms=None, legs=None, mask=None, bristle=0, rear_up=0, seed=None,
       smear=None, lean=0, sweep_px=None):
    """Front (view 'd') or back (view 'u') crawl figure. Local x center = 10.
    rear_up lifts the back and head (windup); arms/legs override (left, right) joint lists."""
    c = Canvas(CW, CH)
    P = lambda pts: [(x + OX, y + OY) for x, y in pts]
    up = view == "u"
    h = sdy - rear_up
    if up:
        spine = [(5, 12 + h), (7, 9 + h), (10, 8 + h), (13, 9 + h), (15, 12 + h)]
        head = (7 + mdx + lean, 3 + h + mdy)
    else:
        spine = [(5, 8 + h), (7, 5 + h), (10, 4 + h), (13, 5 + h), (15, 8 + h)]
        head = (7 + mdx + lean, 4 + h + mdy)
    if rear_up:
        spine = [(x, y + (rear_up if i in (0, 4) else 0)) for i, (x, y) in enumerate(spine)]
    if arms is None:
        if up:
            arms = ([(8, 10 + h), (5, 12 + h), (4, 15)], [(12, 10 + h), (15, 12 + h), (16, 15)])
        else:
            arms = ([(8, 11 + h), (5, 14 + h), (6, 19)], [(12, 11 + h), (15, 14 + h), (14, 19)])
    if legs is None:
        if up:
            legs = ([(7, 13), (4, 15), (5, 19)], [(13, 13), (16, 15), (15, 19)])
        else:
            legs = ([(6, 10 + h), (2, 12), (3, 16)], [(14, 10 + h), (18, 12), (17, 16)])
    # far parts first
    if up:
        for j, arm in enumerate(arms):
            for x, y in path(P(arm)):
                c.set(x, y, "w")
    else:
        for leg in legs:
            for x, y in path(P(leg)):
                c.set(x, y, "w")
    sp = path(P(spine))
    for x, y in sp:
        c.set(x, y, "x")
        c.set(x, y + 1, "x")
        if up:
            c.set(x, y + 2, "w")
    for x, y in sp:
        if c.get(x, y - 1) in (".", "N", "M", "y", "w") and not (up and c.get(x, y - 1) in ("N", "M")):
            c.set(x, y, "y")
    if up:
        # the back's mass between the spine and the hips
        for x in range(7, 14):
            for y in range(10 + h, 13 + h):
                if c.get(x + OX, y + OY) == ".":
                    c.set(x + OX, y + OY, "w" if x > 10 else "x")
    for x, y, ch in (sweep_px or []):
        if c.get(x + OX, y + OY) == ".":
            c.set(x + OX, y + OY, ch)
    if seed and seed.get("behind"):
        for x, y in path(P(seed["arm"])):
            c.set(x, y, "y")
    if up:
        mv = mask or "u"
        c.paste(MASK[mv], head[0] + OX, head[1] + OY)
    # thorns along the arch
    tips = [(4, 6), (6, 3), (10, 1), (14, 3), (16, 6)] if not up else [(5, 10), (7, 7), (10, 6), (13, 7), (15, 10)]
    for k, (x, y) in enumerate(tips):
        yy = y + h - bristle + (rear_up if k in (0, 4) else 0)
        c.set(x + OX, yy + OY, "v")
        c.set(x + OX + (1 if x < 10 else -1 if x > 10 else 0), yy + 1 + OY, "u")
    if up:
        for k, leg in enumerate(legs):
            for x, y in path(P(leg)):
                c.set(x, y, "y" if k == 0 else "x")
    else:
        for k, arm in enumerate(arms):
            for x, y in path(P(arm)):
                c.set(x, y, "y" if k == 0 else "x")
    c.outline("o")
    if not up:
        mv = mask or "d"
        m = Canvas(CW, CH)
        m.paste(MASK[mv], head[0] + OX, head[1] + OY)
        m.outline("o")
        merge(c, m)
    if seed:
        a = Canvas(CW, CH)
        if seed.get("arm") and not seed.get("behind"):
            for x, y in path(P(seed["arm"])):
                a.set(x, y, "y")
        if seed.get("at"):
            a.paste(SEED["a"], seed["at"][0] + OX, seed["at"][1] + OY)
        a.outline("o")
        merge(c, a)
    for x, y, ch in (smear or []):
        c.set(x + OX, y + OY, ch)
    return c


def arc(cx, cy, r, a0, a1, t0, t1):
    """Tapered smear arc (local coords): angle a0 -> a1 (degrees), thickness t0 -> t1."""
    out = []
    for yy in range(int(cy - r - 2), int(cy + r + 3)):
        for xx in range(int(cx - r - 2), int(cx + r + 3)):
            dx, dy = xx - cx, yy - cy
            a = math.degrees(math.atan2(dy, dx))
            lo, hi = min(a0, a1), max(a0, a1)
            if not lo <= a <= hi:
                continue
            f = (a - a0) / (a1 - a0)
            t = t0 + (t1 - t0) * f
            rr = math.hypot(dx, dy)
            if r - t <= rr <= r:
                out.append((xx, yy, "z" if rr >= r - t * 0.45 else "y"))
    return out


def anims_fb(view):
    """Assemble the front or back crawl and throwing animation rows."""
    sfx = "down" if view == "d" else "up"
    hot = "d_hot" if view == "d" else "u"
    shut = "d_shut" if view == "d" else "u"
    up = view == "u"
    F_ = lambda **k: fb(view, **k)
    out = []
    out.append((f"idle_{sfx}", [F_(), F_(sdy=-1), F_(sdy=-1, mdx=1), F_(sdy=-1)],
                {"fps": 6, "durations": [12, 8, 16, 8]}))
    if up:
        walk = [
            dict(legs=([(7, 13), (4, 15), (5, 19)], [(13, 13), (16, 15), (16, 18)]), arms=([(8, 10), (5, 12), (4, 14)], [(12, 10), (15, 12), (16, 15)])),
            dict(sdy=-1),
            dict(legs=([(7, 13), (4, 15), (4, 18)], [(13, 13), (16, 15), (15, 19)]), arms=([(8, 10), (5, 12), (4, 15)], [(12, 10), (15, 12), (16, 14)])),
            dict(sdy=-1),
        ]
    else:
        walk = [
            dict(arms=([(8, 11), (5, 14), (6, 19)], [(12, 11), (15, 13), (15, 18)]), legs=([(6, 10), (2, 12), (3, 16)], [(14, 10), (18, 12), (18, 15)])),
            dict(sdy=-1),
            dict(arms=([(8, 11), (5, 13), (5, 18)], [(12, 11), (15, 14), (14, 19)]), legs=([(6, 10), (2, 12), (2, 15)], [(14, 10), (18, 12), (17, 16)])),
            dict(sdy=-1),
        ]
    out.append((f"move_{sfx}", [F_(**w) for w in walk], {"fps": 10, "durations": [6, 5, 6, 5]}))
    # windup: rear up, the left arm cocks a matte split seed above and behind the head
    if up:
        cock = dict(arm=[(8, 8), (5, 5), (4, 1)], at=(2, -3))
        rear_kw = dict(rear_up=3, mask="u", arms=([(8, 9), (5, 8), (4, 5)], [(12, 7), (15, 9), (16, 12)]))
    else:
        cock = dict(arm=[(8, 8), (5, 5), (4, 1)], at=(2, -3))
        rear_kw = dict(rear_up=3, mask=hot, arms=([(8, 8), (5, 8), (4, 6)], [(12, 8), (15, 10), (16, 13)]))
    out.append((f"windup_{sfx}", [
        F_(sdy=1, mdy=1),
        F_(seed=cock, bristle=1, **rear_kw),
        F_(seed=cock, bristle=2, **rear_kw),
        F_(seed=dict(cock, at=(1, -3)), bristle=2, lean=-1, **rear_kw),
        F_(seed=cock, bristle=2, **rear_kw),
    ], {"fps": 12, "durations": [5, 6, 6, 5, 6]}))
    # whip: the seed is still in the hand at the end of the sweep; it leaves on frame 1
    if up:
        hand, rel_hand = (13, 0), (11, -1)
        whip = dict(rear_up=2, mask="u", seed=dict(arm=[(8, 8), (11, 2), hand], at=seed_at_hand(hand)),
                    sweep_px=sweep((8, 8), (4, 1), hand))
        rel = dict(sdy=-1, seed=dict(arm=[(8, 9), (10, 4), (11, 0)]))
    else:
        hand, rel_hand = (15, 3), (13, 19)
        whip = dict(rear_up=2, mask=hot, seed=dict(arm=[(8, 8), (12, 4), hand], at=seed_at_hand(hand), behind=True),
                    sweep_px=sweep((8, 8), (4, 1), hand))
        rel = dict(sdy=1, mask=hot, seed=dict(arm=[(8, 10), (11, 15), (12, 19)]))
    off = [rel_hand[0] + OX - AX, rel_hand[1] + OY - AY]
    out.append((f"attack_{sfx}", [F_(**whip), F_(**rel), F_(sdy=1, mdy=1), F_()],
                {"fps": 15, "durations": [3, 5, 8, 6], "release_frame": 1, "release_offset": off,
                 "notes": "seed is in the hand through frame 0; spawn the projectile on release_frame at release_offset (cell px from the anchor, screen space)"}))
    out.append((f"hurt_{sfx}", [
        F_(rear_up=2, mask=shut, bristle=2, lean=-1, arms=None if up else ([(8, 9), (4, 9), (2, 7)], [(12, 9), (16, 9), (18, 7)])),
        F_(sdy=1, mask=shut, bristle=1),
    ], {"fps": 10, "durations": [6, 8]}))
    return out


def anims_right():
    """Assemble the side crawl and throwing animation rows."""
    out = []
    out.append(("idle_right", [
        fig(crawl()), fig(crawl(sdy=-1)), fig(crawl(sdy=-1, mdx=1, mdy=-1)), fig(crawl(sdy=-1)),
    ], {"fps": 6, "durations": [12, 8, 16, 8]}))
    # crawl gait: near arm + far leg, then far arm + near leg
    out.append(("move_right", [
        fig(crawl(near_hand=(15, 19), far_hand=(8, 19), near_foot=((4, 19), (6, 19)), far_foot=((2, 19), (4, 19)))),
        fig(crawl(sdy=-1, near_hand=(13, 19), far_hand=(10, 19), far_lift=2, near_foot=((5, 19), (7, 19)), far_foot=((1, 18), (3, 18)))),
        fig(crawl(near_hand=(12, 19), far_hand=(11, 19), near_foot=((6, 19), (8, 19)), far_foot=((0, 19), (2, 19)))),
        fig(crawl(sdy=-1, near_hand=(14, 19), near_lift=2, far_hand=(9, 19), near_foot=((5, 18), (7, 18)), far_foot=((1, 19), (3, 19)))),
    ], {"fps": 10, "durations": [6, 5, 6, 5], "notes": "skittering crawl; one cycle moves about 6 px"}))
    out.append(("windup_right", [
        fig(crawl(sdy=1, mdy=1, bristle=0)),
        fig(rear(lean=-1, bristle=1)),
        fig(rear(lean=-1, bristle=2)),
        fig(rear(lean=-1, bristle=2, sdx=-1)),
        fig(rear(lean=-1, bristle=2)),
    ], {"fps": 12, "durations": [5, 6, 6, 5, 6],
        "notes": "telegraph: dips, then rears up on its hind legs with a matte split seed cocked over its head; mask tips and thorns bristle. 28 ticks; hold the last frame if longer"}))
    rx, ry = throw(1)["release"]
    out.append(("attack_right", [
        fig(throw(0)), fig(throw(1)), fig(throw(2)), fig(crawl()),
    ], {"fps": 15, "durations": [3, 5, 8, 6], "release_frame": 1,
        "release_offset": [rx + OX - AX, ry + OY - AY],
        "notes": "seed is in the hand through frame 0 (end of the sweep); spawn the projectile on release_frame at release_offset "
                 "(cell px from the anchor, screen space; mirror x for left)"}))
    out.append(("hurt_right", [fig(hurt(0)), fig(hurt(1))], {"fps": 10, "durations": [6, 8]}))
    return out


def build():
    """Write the authored sheet and its compatible animation manifest."""
    anims = anims_right() + anims_fb("d") + anims_fb("u")
    return build_sheet(
        "thorn_imp", (CW, CH), (AX, AY), anims, PAL, EMISSIVE,
        extra={"notes": "Style v2: long split seed-husk mask, sage thorn canes and sparse dried-rose tips. "
                        "Mask sockets and held seed are matte, without glow. The arched torso and bent arm masses keep open gaps. "
                        "Reviewed crawl, overhead throw, held seed through frame 0, release offsets and timings retained. "
                        "Cell and anchor retained; _right mirrors for left."},
    )


if __name__ == "__main__":
    sheet, manifest = build()
    print(f"{__file__}: {sheet.size}, {len(manifest['anims'])} animation rows")
