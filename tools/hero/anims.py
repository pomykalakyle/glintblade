"""Glint's animations: every frame is a spec for rig.frame (parts + offsets + blade + fixes).
Durations are 60 Hz ticks, matched to the engine's state timers in src/entities/player.rs.

Grid coords: a part's column c / row r lands at authored (OX + c, OY + r) = (12 + c, 14 + r).
Boots stand on row 17 = authored y 31 = the anchor row. The cloak ends on row 15 (front/back)
or 14 (side); legs and boots are separate stamps so every pose places the feet."""
import blade as bl
import parts
import xf
from rig import OX, OY

H = {d: parts.HOOD[d] for d in ("down", "right", "up")}


# ----------------------------------------------------------------------------- feet
def foot_front(x, bottom, out="L", top=13):
    """Front/back-view leg (2 px, dark indigo) and boot; the sole is 3 px with the toe turned
    out to screen-left ("L") or screen-right ("R"). x = the leg's left column."""
    n_leg = max(0, bottom - 1 - top)
    if out == "L":
        rows, x0 = [".22"] * n_leg + [".GG", "GGg"], x - 1
    else:
        rows, x0 = ["22."] * n_leg + ["GG.", "GGg"], x
    return (OX + x0, OY + bottom - len(rows) + 1, rows)


def foot_side(x, bottom, near=True, top=13):
    """Side-view leg and boot, toe to the right. The far boot is a step darker."""
    n_leg = max(0, bottom - 1 - top)
    boot = [".GG.", ".GGg"] if near else [".gg.", ".ggg"]
    rows = [".22."] * n_leg + boot
    return (OX + x - 1, OY + bottom - len(rows) + 1, rows)


def F(xl=3, xr=11, bl_=17, br=17):
    """Front/back feet: screen-left leg at column xl, screen-right at xr, with their bottoms."""
    return [foot_front(xl, bl_, "L"), foot_front(xr, br, "R")]


def S(xf_=1, xn=8, bf=17, bn=17):
    """Side feet: far (back) leg at column xf_, near leg at xn, with their bottoms."""
    return [foot_side(xf_, bf, near=False), foot_side(xn, bn, near=True)]


def stand_feet(d, dy=0):
    """Return planted feet for the requested facing."""
    if d == "right":
        return S(1, 8, 17 + dy, 17 + dy)
    return F(3, 11, 17 + dy, 17 + dy)


def spec(d, hood=("n", 0, 0), body=("stand", 0, 0), under=None, extra=None, blade=None, **kw):
    """Compose a frame specification without changing authored part coordinates."""
    s = {"dir": d, "hood": hood, "body": body,
         "under": stand_feet(d) if under is None else under, "extra": extra or []}
    if blade:
        s["blade"] = blade
    s.update(kw)
    return s


# default hand (grip) and resting blade direction per facing: the sword hand is held a pixel
# off the cloak so the sleeve and glove break the silhouette
REST = {
    "down": {"hx": 12, "hy": 27, "k": 5},
    "right": {"hx": 24, "hy": 26, "k": 2},
    "up": {"hx": 28, "hy": 26, "k": 3},
}


# ----------------------------------------------------------------------------- idle
# Breath: shoulders and hood sink 1px on the exhale, the blade hand dips with them.
# A glint runs up the blade once per loop, and Glint blinks once.
def idle(d):
    """Return the quiet directional idle keys and uneven holds."""
    base = parts.BODY[d]["stand"]
    body_lo = xf.squash(base, 3)
    r = REST[d]
    L = bl.LENGTH[r["k"] % 16]
    g = [max(1, round(L * f)) for f in (0.25, 0.5, 0.75)] + [L - 1]
    # Round 11: the breath has overlap instead of one rigid 1 px bob. The shoulders (and the
    # sword hand with them) lead each change and the hood follows one key later: key 4 drops the
    # shoulders under a still-held hood, key 7 lifts them while the hood is still down.
    plan = [  # (shoulders low, hood low, glint, blink, ticks)
        (0, 0, None, False, 14),
        (0, 0, g[0], False, 4),
        (0, 0, g[1], False, 4),
        (0, 0, g[2], False, 4),
        (1, 0, g[3], False, 8),
        (1, 1, None, False, 16),
        (1, 1, None, True, 6),
        (0, 1, None, False, 10),
    ]
    frames, durs = [], []
    for low, hlow, glint, blink, t in plan:
        body = (body_lo, 0, 0) if low else ("stand", 0, 0)
        b = {"k": r["k"], "hx": r["hx"], "hy": r["hy"] + low, "glint": glint}
        s = spec(d, hood=("n", 0, hlow), body=body, blade=b)
        if blink:
            if d == "up":  # no eyes to blink from behind: the hood tip twitches instead
                s["pre"] = tip_fix(d, "stream", 0, hlow)
            else:
                s["eyes"] = {"e": "v"}
        frames.append(s)
        durs.append(t)
    return frames, durs


# ----------------------------------------------------------------------------- run
# 8 frames x 4 ticks = 32 ticks: contacts on frames 0 and 4 land with the engine's footstep
# every 16 ticks. The body bobs (down on the landing, up on the push-off); the hood lags a frame
# behind the body and its tip lifts on the rise; the robe hem flares on each landing and tucks
# in on the rise; the blade swings opposite the leading foot.
RUN_T = [4] * 8


def run_body(base, phase, lag=0):
    """Squash and flare the hem on the landing, tuck it on the rise; `lag` trails the bottom two
    rows behind the direction of travel (side view)."""
    n = len(xf.lines(base[1]))
    p = base
    if phase == "down":
        p = xf.squash(p, 3)
        p = xf.widen(p, [n - 3, n - 2], 1, 1)
    elif phase == "up":
        p = xf.narrow(p, [n - 1], 1, 1)
    if lag:
        p = xf.slide(p, [n - 2, n - 1], lag)
    return p


def run_side_body(phase):
    """Side-view run torso with a held forward lean and a lagging hem (round 11).

    The body part is placed at dx 0 and its rows are sheared instead: scarf, chest and shoulders
    (rows 0-3) sit 2 px forward, the waist (row 4) 1 px, so the hood and chest lead the hips the
    whole cycle. The hem (rows 5-6) stays back on the landing keys ("contact", "down") and
    swings 1 px forward on the passing and push-off keys, so the cloak trails and then follows
    through instead of bobbing rigidly with the body. Nothing is clipped: rows only move right.
    """
    base = parts.BODY_R["run"]
    n = len(xf.lines(base[1]))
    p = base
    if phase == "down":
        p = xf.squash(p, 3)
        p = xf.widen(p, [n - 3, n - 2], 1, 1)
    elif phase == "up":
        p = xf.narrow(p, [n - 1], 1, 1)
    m = len(xf.lines(p[1]))
    p = xf.slide(p, list(range(0, m - 3)), 2)
    p = xf.slide(p, [m - 3], 1)
    if phase in ("pass", "up"):
        p = xf.slide(p, [m - 2, m - 1], 1)
    return p


def tip_fix(d, mode, hdx=0, hdy=0):
    """Pixel swaps on the hood tip (grid coords relative to the hood): 'droop' = as drawn,
    'stream' = flicked straight out behind, 'up' = flicked up-back (fast moves)."""
    if mode == "droop":
        return []
    if d == "right":
        fx = {"stream": [(0, 3, "_"), (-1, 2, "2")],
              "up": [(0, 3, "_"), (-1, 1, "2"), (0, 2, "2")]}[mode]
    elif d == "down":
        fx = {"stream": [(14, 4, "_"), (15, 3, "2")],
              "up": [(14, 4, "_"), (15, 2, "2"), (14, 3, "2")]}[mode]
    else:
        fx = {"stream": [(1, 4, "_"), (0, 3, "2")],
              "up": [(1, 4, "_"), (0, 2, "2"), (1, 3, "2")]}[mode]
    return [(OX + hdx + x, OY + hdy + y, ch) for (x, y, ch) in fx]


def run(d):
    """Contacts on frames 0 and 4 (opposite feet leading, legs spread widest, body lowest a frame
    later), passing poses on 2 and 6 (legs together under the body, the swing foot lifted high),
    push-off on 3 and 7 (body highest). The hem lags the legs; the hood lags the body."""
    phases = ["contact", "down", "pass", "up"] * 2
    body_dy = [0, 0, 0, -1, 0, 0, 0, -1]
    top = [body_dy[i] + (1 if phases[i] == "down" else 0) for i in range(8)]
    hood = [max(top[i], top[i - 1]) for i in range(8)]
    tip = ["stream", "droop", "droop", "stream", "stream", "droop", "droop", "stream"]
    frames = []
    for i, ph in enumerate(phases):
        dy = body_dy[i]
        if d in ("down", "up"):
            # screen-left / screen-right legs: (column, bottom row). Bottom 18 = the foot reaching
            # toward the camera, 16-15 = lifted behind; passing poses pull the legs together.
            Lf = [(2, 18), (2, 18), (4, 17), (3, 17), (3, 16), (3, 16), (4, 16), (3, 17)][i]
            Rf = [(11, 16), (11, 16), (10, 16), (11, 17), (12, 18), (12, 18), (10, 17), (11, 17)][i]
            under = F(Lf[0], Rf[0], Lf[1] + dy, Rf[1] + dy)
            base = parts.BODY[d]["stand"]
            sway = [-1, -1, 0, 0, 1, 1, 0, 0][i]  # the hem swings toward the planted leg's side
            body = (run_body(base, ph, lag=sway), 0, dy)
            sw = [1, 1, 0, -1, -1, -1, 0, 1][i]
            r = REST[d]
            k = [5, 5, 5, 6, 6, 6, 5, 5][i] if d == "down" else [3, 3, 3, 2, 2, 2, 3, 3][i]
            b = {"k": k, "hx": r["hx"], "hy": r["hy"] + dy + sw + (1 if ph == "down" else 0)}
            s = spec(d, hood=("n", 0, hood[i]), body=body, under=under, blade=b,
                     pre=tip_fix(d, tip[i], 0, hood[i]))
        else:
            # near / far legs: (column, bottom row)
            N = [(11, 17), (10, 17), (7, 17), (3, 17), (0, 16), (1, 16), (5, 15), (9, 16)][i]
            Fr = [(0, 16), (1, 16), (5, 15), (9, 16), (11, 17), (10, 17), (7, 17), (3, 17)][i]
            under = S(Fr[0], N[0], Fr[1] + dy, N[1] + dy)
            body = (run_side_body(ph), 0, dy)
            hx = [-1, -1, 0, 1, 1, 1, 0, -1][i]  # the sword arm swings against the near leg
            k = [1, 1, 2, 2, 2, 2, 2, 1][i]
            b = {"k": k, "hx": 26 + hx, "hy": 26 + dy + (1 if ph == "down" else 0)}
            s = spec(d, hood=("n", 2, hood[i]), body=body, under=under, blade=b,
                     pre=tip_fix(d, ["stream", "stream", "droop", "up", "stream", "stream", "droop", "up"][i], 2, hood[i]))
        frames.append(s)
    return frames, RUN_T


# ----------------------------------------------------------------------------- pose builder
SHOULDERS = {"down": [(15, 23), (25, 23)], "right": [(21, 23)], "up": [(15, 23), (25, 23)]}


def pose(d, hand, k, length=None, hood_dx=0, hood_dy=0, top_slide=0, squash=0, stretch=0,
         lift=0, flare=0, tip="droop", feet=None, body_dx=0, base=None, arm=True, behind=False,
         charged=0, glint=None, eyes=None, pre=None, post=None, blade=True, glove="G", hood="n",
         hem=0, coil=False):
    """One frame: body squash/stretch/lean/flare built from the stand part, hood riding on top,
    feet, an arm from the nearest shoulder to `hand`, and the blade from `hand` in direction k.
    `hem` slides the bottom two cloak rows against `top_slide`, so hips counter-rotate under
    the shoulders and the torso twists along one line of action. `coil` (side view) turns the
    chest away from the camera on a windup: the front of the scarf band becomes lit cloak back,
    leaving only the knot at the nape."""
    b = base or parts.BODY[d]["stand"]
    if coil and d == "right":
        g = xf.lines(b[1])
        r1 = g[1]
        cut = r1.index("4")
        g = [g[0].replace("T", "4").replace("t", "4"),
             r1[:cut] + r1[cut:].replace("t", "3").replace("S", "3").replace("s", "3")] + g[2:]
        b = (b[0], g)
    n = len(xf.lines(b[1]))
    for _ in range(squash):
        b = xf.squash(b, 3)
    for _ in range(stretch):
        b = xf.stretch(b, 4)
    pad = 0
    if hem:
        # pad the grid so rows slid toward screen-left keep their edge pixels; the body is
        # stamped `pad` further left to compensate (the arm and hood are unaffected)
        pad = 2
        b = (b[0], ["." * pad + r for r in xf.lines(b[1])])
    if top_slide:
        b = xf.slide(b, [0, 1, 2, 3], top_slide)
    if hem:
        nn = len(xf.lines(b[1]))
        b = xf.slide(b, [nn - 2, nn - 1], hem)
    if flare:
        nn = len(xf.lines(b[1]))
        b = xf.widen(b, [nn - 2, nn - 1], flare, flare)
    body_top = b[0] - 8
    hdy = body_top + hood_dy - lift
    s = {"dir": d,
         "body": (b, body_dx - pad, -lift),
         "hood": (hood, hood_dx + (top_slide if d != "right" else 0) + body_dx, hdy)}
    if feet is None:
        feet = stand_feet(d, -lift)
    s["under"] = feet
    s["extra"] = []
    hx, hy = hand
    if blade:
        s["blade"] = {"k": k, "hx": hx, "hy": hy, "length": length, "behind": behind,
                      "charged": charged, "glint": glint}
    else:
        s["hand"] = hand
    s["glove"] = glove
    if arm:
        sh = min(SHOULDERS[d], key=lambda p: (p[0] - hx) ** 2 + (p[1] - hy) ** 2)
        sh = (sh[0] + body_dx + (top_slide if d != "right" else 0), sh[1] - lift + body_top)
        if abs(sh[0] - hx) + abs(sh[1] - hy) > 2:
            s["arm"] = (sh, hand)
    fx = tip_fix(d, tip, s["hood"][1], hdy) if tip != "droop" else []
    s["pre"] = fx + (pre or [])
    s["post"] = post or []
    if eyes:
        s["eyes"] = eyes
    return s


# ----------------------------------------------------------------------------- attacks
# Engine timing (player.rs): attack1/2 last 17 ticks, hits on ticks 3-8; attack3 lasts 24 ticks,
# hits on ticks 5-11. The engine's smear arcs sweep (facing right) top->bottom on 1 and 3,
# bottom->top on 2; facing down right->left on 1 and 3; facing up left->right on 1 and 3.
# Every swing is driven by the body: weight shifts back (lean, front foot light) on the
# anticipation, the lead foot steps and the torso and hood lunge after the guard on the strike,
# the cloak flares on the follow-through, and the recovery pose pulls the back foot in.
A12_T = [3, 2, 3, 4, 5]
A3_T = [3, 2, 2, 3, 6, 8]


def attack1(d):
    """Return the first combo swing: the shoulders coil back over forward hips, then counter-
    rotate through the strike while the rear boot stays planted and the lead boot steps wide."""
    if d == "right":
        f = [
            pose(d, (21, 18), 11, length=9, body_dx=-1, top_slide=-1, hem=1, hood_dx=-2, coil=True, tip="up", feet=S(-1, 8, 17, 16)),
            pose(d, (29, 23), 2, length=10, body_dx=1, top_slide=2, hem=-1, hood_dx=2, tip="stream", feet=S(-1, 12)),
            pose(d, (29, 25), 3, length=10, body_dx=2, top_slide=2, hem=-1, hood_dx=2, squash=1, flare=1, tip="stream", feet=S(-1, 13)),
            pose(d, (28, 27), 4, length=9, body_dx=2, top_slide=1, hem=-1, hood_dx=1, squash=1, flare=1, feet=S(-1, 13)),
            pose(d, (25, 26), 2, body_dx=1, hem=1, feet=S(2, 11)),
        ]
    elif d == "down":
        f = [
            pose(d, (27, 20), 14, top_slide=2, hem=-1, tip="up", feet=F(3, 12, 16, 17)),
            pose(d, (17, 28), 6, length=10, hood_dy=1, top_slide=-1, hem=1, squash=1, tip="stream", feet=F(2, 12, 18, 17)),
            pose(d, (13, 28), 7, length=10, top_slide=-2, hem=1, squash=1, flare=1, tip="stream", feet=F(1, 12, 18, 17)),
            pose(d, (11, 27), 8, top_slide=-2, hem=1, squash=1, feet=F(1, 12, 18, 17)),
            pose(d, (12, 27), 6, hem=-1, feet=F(2, 11)),
        ]
    else:
        f = [
            pose(d, (13, 22), 9, top_slide=-2, hem=1, tip="up", feet=F(3, 11, 17, 16)),
            pose(d, (25, 19), 14, length=10, top_slide=1, hem=-1, stretch=1, tip="stream", behind=True, feet=F(3, 12, 16, 17)),
            pose(d, (28, 21), 15, length=10, top_slide=2, hem=-1, stretch=1, tip="stream", feet=F(3, 13, 16, 17)),
            pose(d, (29, 24), 0, top_slide=2, hem=-1, squash=1, flare=1, feet=F(3, 13, 16, 17)),
            pose(d, (28, 26), 2, hem=1, feet=F(3, 12)),
        ]
    return f, A12_T


def attack2(d):
    """Return the reverse combo swing: the same coil and counter-rotation, mirrored in time."""
    if d == "right":
        f = [
            pose(d, (23, 28), 6, squash=1, top_slide=-1, hem=1, hood_dx=-2, coil=True, tip="stream", feet=S(0, 9)),
            pose(d, (28, 21), 14, length=10, body_dx=1, top_slide=2, hem=-1, hood_dx=2, tip="stream", feet=S(0, 12)),
            pose(d, (27, 19), 13, length=10, body_dx=1, top_slide=2, hem=-1, hood_dx=2, stretch=1, tip="up", feet=S(0, 12, 17, 17)),
            pose(d, (25, 17), 12, length=9, body_dx=1, top_slide=1, hem=-1, hood_dx=1, stretch=1, tip="up", feet=S(0, 12, 16, 17)),
            pose(d, (25, 26), 2, body_dx=1, hem=1, feet=S(2, 10)),
        ]
    elif d == "down":
        f = [
            pose(d, (12, 24), 9, top_slide=-2, hem=1, tip="up", feet=F(2, 11, 17, 16)),
            pose(d, (24, 28), 2, length=10, hood_dy=1, top_slide=1, hem=-1, squash=1, tip="stream", feet=F(2, 12, 17, 18)),
            pose(d, (27, 27), 1, length=10, top_slide=2, hem=-1, squash=1, flare=1, tip="stream", feet=F(2, 13, 17, 18)),
            pose(d, (29, 25), 0, top_slide=2, hem=-1, squash=1, feet=F(2, 13, 17, 18)),
            pose(d, (25, 27), 3, hem=1, feet=F(3, 12)),
        ]
    else:
        f = [
            pose(d, (28, 22), 15, top_slide=2, hem=-1, tip="up", feet=F(3, 11, 16, 17)),
            pose(d, (15, 19), 10, length=10, top_slide=-1, hem=1, stretch=1, tip="stream", behind=True, feet=F(2, 11, 17, 16)),
            pose(d, (12, 21), 9, length=10, top_slide=-2, hem=1, stretch=1, tip="stream", feet=F(1, 11, 17, 16)),
            pose(d, (11, 24), 8, top_slide=-2, hem=1, squash=1, flare=1, feet=F(1, 11, 17, 16)),
            pose(d, (14, 26), 6, hem=-1, feet=F(2, 11)),
        ]
    return f, A12_T


def attack3(d):
    """The finisher: a deep coiled crouch with the blade drawn back, a leap with it raised high,
    a cleave that lands in the widest lunge of the set (rear boot planted, shoulders thrown past
    the hips), a held follow-through, then a recovery where the hem swings through."""
    if d == "right":
        f = [
            pose(d, (15, 26), 7, length=9, body_dx=-1, squash=2, flare=1, top_slide=-1, hem=1, hood_dx=-2, coil=True, tip="up",
                 behind=True, feet=S(-1, 10)),
            pose(d, (22, 15), 11, length=9, lift=2, stretch=1, tip="stream", feet=S(2, 8, 14, 14)),
            pose(d, (29, 21), 2, length=11, lift=1, body_dx=1, top_slide=2, hood_dx=2, tip="stream", feet=S(0, 12, 16, 16)),
            pose(d, (30, 26), 3, length=11, body_dx=2, top_slide=3, hem=-1, hood_dx=3, squash=2, flare=2, tip="stream", feet=S(-1, 14)),
            pose(d, (29, 28), 4, length=10, body_dx=2, top_slide=2, hem=-1, hood_dx=2, squash=2, flare=1, feet=S(-1, 14)),
            pose(d, (25, 26), 2, body_dx=1, squash=1, hem=1, feet=S(1, 10)),
        ]
    elif d == "down":
        f = [
            pose(d, (28, 25), 0, top_slide=2, hem=-1, squash=2, flare=1, tip="up", feet=F(2, 12)),
            pose(d, (25, 17), 13, stretch=1, lift=2, tip="stream", behind=True, feet=F(3, 11, 15, 15)),
            pose(d, (18, 29), 6, length=11, hood_dy=1, lift=1, top_slide=-1, tip="stream", feet=F(2, 12, 17, 17)),
            pose(d, (14, 29), 7, length=11, hood_dy=1, top_slide=-2, hem=1, squash=2, flare=2, tip="stream", feet=F(0, 12, 18, 17)),
            pose(d, (11, 27), 8, length=10, top_slide=-2, hem=1, squash=2, flare=1, feet=F(0, 12, 18, 17)),
            pose(d, (12, 27), 6, squash=1, hem=-1, feet=F(2, 11)),
        ]
    else:
        f = [
            pose(d, (13, 26), 7, top_slide=-2, hem=1, squash=2, flare=1, tip="up", feet=F(2, 12)),
            pose(d, (15, 17), 10, stretch=1, lift=2, tip="stream", behind=True, feet=F(3, 11, 15, 15)),
            pose(d, (25, 18), 14, length=11, stretch=1, lift=1, top_slide=1, tip="stream", behind=True, feet=F(3, 12, 16, 16)),
            pose(d, (29, 23), 0, length=11, top_slide=2, hem=-1, squash=2, flare=2, tip="stream", feet=F(2, 14, 17, 16)),
            pose(d, (29, 25), 1, length=10, top_slide=2, hem=-1, squash=2, flare=1, feet=F(2, 14, 17, 16)),
            pose(d, (27, 26), 2, squash=1, hem=1, feet=F(3, 12)),
        ]
    return f, A3_T


# ----------------------------------------------------------------------------- dash
# 10 ticks over three keys with distinct jobs: a coiled launch (low, hood driven ahead of the
# shoulders, rear boot pushing off), a long extension (shoulders thrown furthest along the travel,
# hem and blade streaming behind, legs split off the ground) and a planted recovery (lead boot
# braced, shoulders pulled back, hem swinging past the hips). The engine copies whichever key is
# live into its afterimages.
DASH_T = [4, 3, 3]


def dash(d):
    """Return the launch, extension and planted-recovery keys of the directional dash."""
    run = parts.BODY_R["run"]
    if d == "right":
        f = [
            pose(d, (19, 27), 9, length=9, base=run, squash=2, top_slide=1, hem=-1, hood_dx=2, hood_dy=1,
                 tip="stream", behind=True, feet=S(-2, 9, 17, 17)),
            pose(d, (19, 25), 8, length=10, base=run, squash=1, top_slide=3, hem=-2, flare=1, hood_dx=4,
                 tip="up", behind=True, feet=S(-2, 12, 15, 16)),
            pose(d, (26, 28), 3, length=9, squash=1, top_slide=-1, hem=1, flare=1, hood_dx=-1,
                 feet=S(1, 13)),
        ]
    elif d == "down":
        f = [
            pose(d, (13, 22), 10, length=9, squash=2, flare=1, hood_dy=1, top_slide=1, tip="up",
                 feet=F(1, 13)),
            pose(d, (12, 21), 11, length=10, stretch=1, top_slide=-1, hem=1, hood_dy=1, tip="stream",
                 behind=True, feet=F(3, 11, 18, 15)),
            pose(d, (12, 28), 6, squash=2, flare=2, hood_dy=1, top_slide=-1, feet=F(1, 13)),
        ]
    else:
        f = [
            pose(d, (28, 27), 4, length=9, squash=2, flare=1, top_slide=1, tip="stream",
                 feet=F(2, 12)),
            pose(d, (28, 25), 3, length=10, stretch=1, hood_dy=-2, top_slide=1, hem=-1, tip="up",
                 feet=F(3, 11, 18, 14)),
            pose(d, (28, 27), 3, squash=1, flare=2, top_slide=-1, feet=F(2, 12)),
        ]
    return f, DASH_T


# ----------------------------------------------------------------------------- charge
# Loops while the button is held: a low wide ready stance, the blade raised and brightening,
# light pulsing along it, the hood tip fluttering and the eyes flaring on the pulse.
CHARGE_T = [5, 5, 5, 5]


def charge(d):
    """Return the held directional charge poses and blade intensities."""
    flare_eyes = {"e": "W"}
    if d == "right":
        args = dict(hand=(25, 26), k=13, length=10, feet=S(0, 10))
    elif d == "down":
        args = dict(hand=(12, 26), k=12, length=10, feet=F(2, 12))
    else:
        args = dict(hand=(28, 26), k=12, length=10, feet=F(2, 12))
    f = []
    for i in range(4):
        f.append(pose(d, args["hand"], args["k"], length=args["length"],
                      hood_dx=(-1 if d == "right" else 0), squash=1, flare=(1 if i % 2 == 0 else 0),
                      tip=["stream", "up", "stream", "droop"][i], charged=(2 if i in (1, 2) else 1),
                      glint=[1, 4, 7, None][i], eyes=(flare_eyes if i in (1, 2) else None),
                      feet=args["feet"]))
    return f, CHARGE_T


# ----------------------------------------------------------------------------- charge_walk
# Walking while the charge is held (player.rs requests "charge_walk" when moving). The upper body
# is the charge pose, rigid: same grip, blade direction, length and pulse as `charge`, so going
# from standing to walking keeps the weapon still. Only the legs step, in a slow deliberate
# 8-key cycle (contact, settle, pass, reach per foot); the torso, hood and blade rise together
# 1 px on the passing keys. 8 x 5 ticks = 40, so the 20-tick charge pulse repeats in phase.
CHARGE_WALK_T = [5] * 8


def charge_walk(d):
    """Return the stepping charge-walk cycle with the held charge grip and pulse."""
    flare_eyes = {"e": "W"}
    lift = [0, 0, 1, 0, 0, 0, 1, 0]
    if d == "right":
        hand, k = (25, 26), 13
        # (far column, far bottom, near column, near bottom)
        legs = [(-1, 17, 12, 17), (0, 16, 11, 17), (5, 15, 8, 17), (9, 16, 6, 17),
                (11, 17, 0, 17), (10, 17, 1, 16), (7, 17, 5, 15), (5, 17, 9, 16)]
    else:
        hand, k = ((12, 26), 12) if d == "down" else ((28, 26), 12)
        # (screen-left column, bottom, screen-right column, bottom); 18 = toward the camera
        legs = [(2, 18, 12, 16), (2, 17, 12, 16), (3, 17, 11, 15), (3, 17, 12, 17),
                (2, 16, 12, 18), (2, 16, 12, 17), (3, 15, 11, 17), (2, 17, 12, 17)]
    f = []
    for i in range(8):
        a, ab, b_, bb = legs[i]
        feet = S(a, b_, ab, bb) if d == "right" else F(a, b_, ab, bb)
        p = i % 4
        f.append(pose(d, (hand[0], hand[1] - lift[i]), k, length=10, lift=lift[i],
                      hood_dx=(-1 if d == "right" else 0), squash=1, flare=(1 if lift[i] == 0 and i % 4 == 0 else 0),
                      tip=["stream", "up", "stream", "droop"][p], charged=(2 if p in (1, 2) else 1),
                      glint=[1, 4, 7, None][p], eyes=(flare_eyes if p in (1, 2) else None), feet=feet))
    return f, CHARGE_WALK_T


# ----------------------------------------------------------------------------- hurt
# 15 ticks: snap back (hood thrown back, blade flung wide, eyes flare), stagger, recover.
HURT_T = [4, 5, 6]


def hurt(d):
    """Return the directional recoil poses and their authored holds."""
    w = {"e": "W"}
    shut = {"e": "v"}
    if d == "right":
        f = [
            pose(d, (25, 21), 13, body_dx=-1, top_slide=-1, hood_dx=-2, tip="up", eyes=w, feet=S(0, 9, 17, 16)),
            pose(d, (24, 28), 3, body_dx=-1, hood_dx=-1, squash=1, flare=1, eyes=shut, feet=S(0, 8)),
            pose(d, (24, 26), 2, feet=S(1, 8)),
        ]
    elif d == "down":
        f = [
            pose(d, (11, 22), 10, hood_dy=-1, stretch=1, tip="up", eyes=w, feet=F(3, 11, 16, 16)),
            pose(d, (12, 28), 5, hood_dy=1, squash=1, flare=1, eyes=shut, feet=F(2, 12)),
            pose(d, (12, 27), 5),
        ]
    else:
        f = [
            pose(d, (29, 22), 14, hood_dy=1, squash=1, flare=1, tip="up", feet=F(3, 11, 18, 18)),
            pose(d, (28, 28), 3, hood_dy=1, squash=1, feet=F(2, 12)),
            pose(d, (28, 26), 3),
        ]
    return f, HURT_T


# ----------------------------------------------------------------------------- spin
# The charged spin slash (24 ticks in player.rs): the body whips round through every facing
# while the blade sweeps a full clockwise turn at arm's length, cloak flared by the spin, then a
# low held follow-through. The left-facing step is the side view mirrored and relit (rig).
SPIN_T = [2, 2, 2, 2, 2, 2, 2, 10]


def spin():
    """Return the unmirrored clockwise spin with the relit left-facing key."""
    f = []
    # (facing, blade k, hand, flip, behind)
    steps = [
        ("down", 4, (20, 29), False, False),
        ("down", 6, (15, 28), False, False),
        ("right", 8, (17, 25), True, False),    # facing left
        ("up", 10, (15, 21), False, True),
        ("up", 12, (20, 19), False, True),
        ("up", 14, (25, 21), False, True),
        ("right", 0, (26, 25), False, False),
        ("down", 2, (25, 28), False, False),
    ]
    for i, (d, k, hand, flip, behind) in enumerate(steps):
        last = i == len(steps) - 1
        hx, hy = hand
        if flip:  # author the left-facing frame in right-facing coords, then mirror it
            hx = 40 - hx
            k = (8 - k) % 16
        feet = S(0, 10) if d == "right" else F(2, 12)
        s = pose(d, (hx, hy), k, length=(9 if last else 10), squash=(2 if last else 1),
                 flare=(1 if last else 2), tip=("droop" if last else "stream"), behind=behind, feet=feet)
        s["flip"] = flip
        f.append(s)
    return f, SPIN_T


# ----------------------------------------------------------------------------- death
# head bowed: only the crown and a sliver of shadow show, no eyes
BOWED = (0, """
......5544......
....554444433...
...55444443332..
...544444433222.
...5444443332.2.
...43vvvvvv21...
""")
# the empty cloak settled on the ground, hood lying on top
HEAP = (11, """
......5544......
....5544444333..
...54444443332..
..454vvvvv33222.
.4544433333322..
445444333332221.
.44333333322211.
""")
# Frame 0 is what the engine shatters into pixels (player.rs bursts the current cell on death
# and stops drawing the body 4 ticks later), so it is the full recoil silhouette. The rest is a
# kneel-and-fade for an engine that plays it out: buckle, kneel on the planted blade, slump with
# the eyes going out, the empty cloak settling.
DEATH_T = [6, 6, 10, 12, 30]


def death():
    """Return the kneel-and-collapse sequence with its attached fallen blade."""
    d = "down"
    w = {"e": "W"}
    dim = {"e": "c"}
    f = [
        pose(d, (11, 22), 11, hood_dy=-1, stretch=1, tip="up", eyes=w, feet=F(3, 11, 16, 16)),
        pose(d, (12, 28), 6, hood_dy=1, squash=2, flare=1, feet=F(2, 12)),
        pose(d, (14, 27), 4, length=8, hood_dy=1, lift=-1, squash=3, flare=2, eyes=dim, feet=F(2, 12)),
        pose(d, (14, 29), 6, length=8, hood=BOWED, lift=-2, squash=4, flare=2, feet=[], tip="droop"),
    ]
    heap = spec(d, hood=None, body=(HEAP, 0, 0), under=[], glove=False,
                blade={"k": 8, "hx": 13, "hy": 30, "length": 8, "dim": True})
    f.append(heap)
    return f, DEATH_T
