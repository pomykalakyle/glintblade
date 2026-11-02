"""Native three-quarter animation keys and facing-aware secondary hero states.

No bitmap is rotated or resampled. The two new anatomical views use their own
hood/chest grids, staggered foreshortened feet and authored hand trajectories.
West views are composed by the rig, including relighting and a freshly drawn
blade. Existing cardinal rows are retained unchanged by build.collect().
"""
from copy import deepcopy

import anims as a
import parts
import xf

DIRECTIONS = ("up", "up_right", "right", "down_right", "down", "down_left", "left", "up_left")
DIAGONALS = ("up_right", "down_right", "down_left", "up_left")
REST = {"down_right": ((26, 27), 3), "up_right": ((27, 25), 1)}


def canonical(d):
    """Resolve a west view to the exact native east view mirrored by the rig."""
    return {"left": "right", "down_left": "down_right", "up_left": "up_right"}.get(d, d)


def west(d):
    return d in ("left", "down_left", "up_left")


def foot(d, x, bottom, near=True, top=13):
    """Three-quarter boot: a short diagonal toe, rather than a side boot relabel."""
    if d == "down_right":
        boot = [".GG.", ".GGg"] if near else [".gg.", ".ggg"]
    else:
        boot = [".GGg", "GGG."] if near else [".ggg", "ggg."]
    rows = [".22."] * max(0, bottom - top - 1) + boot
    return (a.OX + x - 1, a.OY + bottom - len(rows) + 1, rows)


def feet(d, far=(4, 16), near=(10, 17)):
    return [foot(d, *far, near=False), foot(d, *near, near=True)]


def planted(d, lift=0, wide=0):
    far_y, near_y = (16, 17) if d == "down_right" else (17, 16)
    return feet(d, (4 - wide, far_y - lift), (10 + wide, near_y - lift))


def pose(d, hand=None, k=None, **kw):
    """Keep every diagonal pose's arm, planted feet and blade in the same rig."""
    rest, rk = REST[d]
    return a.pose(d, hand or rest, rk if k is None else k,
                  feet=kw.pop("feet", planted(d)), **kw)


def idle(d):
    hand, k = REST[d]
    body_lo = xf.squash(parts.BODY[d]["stand"], 3)
    frames = []
    plan = [(0, 0, None, False), (0, 0, 2, False), (0, 0, 4, False),
            (0, 0, 6, False), (1, 0, 7, False), (1, 1, None, False),
            (1, 1, None, True), (0, 1, None, False)]
    for low, hlow, glint, blink in plan:
        frames.append(a.spec(d, body=(body_lo, 0, 0) if low else ("stand", 0, 0),
                             hood=("n", 0, hlow), under=planted(d),
                             arm=((25, 23 + low), (hand[0], hand[1] + low)),
                             blade={"hx": hand[0], "hy": hand[1] + low, "k": k, "glint": glint},
                             eyes={"e": "v"} if blink else None))
    return frames, [14, 4, 4, 4, 8, 16, 6, 10]


def gait(d, slow=False):
    """Pairs travel along the ground diagonal; pass keys lift the swing boot."""
    x = [(3, 11), (4, 10), (6, 8), (8, 6), (11, 3), (10, 4), (8, 6), (6, 8)]
    y_dr = [(16, 18), (16, 18), (15, 17), (17, 17), (18, 16), (18, 16), (17, 15), (17, 17)]
    y_ur = [(18, 15), (18, 15), (17, 16), (16, 17), (15, 18), (15, 18), (16, 17), (17, 16)]
    if slow:
        # Shorter deliberate stride leaves the sword and pulse unchanged.
        x = [(4, 11), (4, 10), (6, 8), (8, 7), (10, 4), (10, 4), (8, 6), (7, 8)]
    return [feet(d, (xx[0], yy[0]), (xx[1], yy[1]))
            for xx, yy in zip(x, y_dr if d == "down_right" else y_ur)]


def run(d):
    frames = []
    phases = ["contact", "down", "pass", "up"] * 2
    soles = gait(d)
    hand, rest_k = REST[d]
    for i, ph in enumerate(phases):
        lift = 1 if ph == "up" else 0
        body = a.run_body(parts.BODY[d]["stand"], ph, lag=(-1 if i < 4 else 1))
        # Native forward lean, with a one-key delay in the hood and trailing hem.
        body = xf.slide(body, [0, 1, 2, 3], 1)
        hx = hand[0] + [-1, -1, 0, 1, 1, 1, 0, -1][i]
        hy = hand[1] + (1 if ph == "down" else 0) - lift
        k = rest_k + (1 if i in (3, 4, 5) else 0)
        frames.append(a.spec(d, body=(body, 0, -lift), hood=("n", 1, -lift), under=soles[i],
                             arm=((25, 23 - lift), (hx, hy)),
                             blade={"hx": hx, "hy": hy, "k": k},
                             pre=a.tip_fix(d, "stream" if ph in ("contact", "up") else "droop", 1, -lift)))
    return frames, a.RUN_T


def attack1(d):
    """Forehand follows a native hand arc centred on the facing diagonal."""
    if d == "down_right":
        keys = [((27, 19), 13), ((28, 27), 3), ((26, 29), 4), ((22, 30), 5), ((26, 27), 3)]
    else:
        keys = [((15, 22), 9), ((26, 19), 14), ((29, 21), 15), ((30, 23), 0), ((27, 25), 1)]
    out = []
    for i, (hand, k) in enumerate(keys):
        strike = i in (1, 2, 3)
        out.append(pose(d, hand, k, length=(9 if i != 2 else 10),
                        body_dx=1 if strike else 0, top_slide=(-1 if i == 0 else 1 if strike else 0),
                        hem=(1 if i == 0 else -1 if strike else 1),
                        squash=1 if strike else 0, flare=1 if i == 2 else 0,
                        hood_dx=1 if strike else -1 if i == 0 else 0,
                        tip="up" if i == 0 else "stream" if strike else "droop",
                        behind=d == "up_right" and i == 1,
                        feet=planted(d, wide=1 if strike else 0)))
    return out, a.A12_T


def attack2(d):
    if d == "down_right":
        keys = [((17, 28), 7), ((29, 24), 0), ((28, 21), 15), ((26, 18), 14), ((26, 27), 3)]
    else:
        keys = [((29, 25), 2), ((23, 18), 12), ((20, 18), 11), ((17, 20), 10), ((27, 25), 1)]
    out = []
    for i, (hand, k) in enumerate(keys):
        strike = i in (1, 2, 3)
        out.append(pose(d, hand, k, length=(10 if i in (1, 2) else 9),
                        body_dx=1 if strike else 0, top_slide=(-1 if i == 0 else 1 if strike else 0),
                        hem=(1 if i == 0 else -1 if strike else 1),
                        stretch=1 if i in (2, 3) else 0, squash=1 if i in (0, 1) else 0,
                        flare=1 if i == 2 else 0, hood_dx=1 if strike else -1 if i == 0 else 0,
                        tip="up" if i in (2, 3) else "stream" if strike else "droop",
                        behind=d == "up_right" and i in (1, 2),
                        feet=planted(d, wide=1 if strike else 0)))
    return out, a.A12_T


def attack3(d):
    if d == "down_right":
        keys = [((27, 25), 0), ((24, 15), 12), ((29, 25), 3),
                ((27, 29), 4), ((23, 30), 5), ((26, 27), 3)]
    else:
        keys = [((15, 26), 8), ((19, 15), 11), ((27, 18), 14),
                ((30, 22), 15), ((30, 25), 0), ((27, 25), 1)]
    out = []
    for i, (hand, k) in enumerate(keys):
        strike = i in (2, 3, 4)
        out.append(pose(d, hand, k, length=10 if strike else 9,
                        body_dx=2 if strike else -1 if i == 0 else 0,
                        top_slide=2 if strike else -1 if i == 0 else 0,
                        hem=-1 if strike else 1, lift=2 if i == 1 else 1 if i == 2 else 0,
                        stretch=1 if i in (1, 2) else 0, squash=2 if i in (0, 3, 4) else 1 if i == 5 else 0,
                        flare=2 if i == 3 else 1 if i in (0, 4) else 0,
                        hood_dx=1 if strike else -1 if i == 0 else 0,
                        tip="up" if i == 0 else "stream" if i < 4 else "droop",
                        behind=d == "up_right" and i in (1, 2),
                        feet=planted(d, lift=2 if i == 1 else 1 if i == 2 else 0, wide=2 if strike else 0)))
    return out, a.A3_T


def dash(d):
    hand = (17, 24) if d == "down_right" else (18, 28)
    trailing = 10 if d == "down_right" else 6
    return [pose(d, hand, trailing, length=9, squash=2, flare=1, top_slide=1,
                 hood_dx=2, hood_dy=1, hem=-1, tip="stream", behind=True,
                 feet=planted(d, wide=1)),
            pose(d, (hand[0] - 1, hand[1] - 1), trailing, length=10, squash=1,
                 top_slide=2, hood_dx=3, hem=-1, tip="up", behind=True,
                 feet=feet(d, (2, 16), (12, 16 if d == "down_right" else 14))),
            pose(d, REST[d][0], REST[d][1], squash=1, flare=1, top_slide=-1,
                 hood_dx=-1, feet=planted(d, wide=1))], a.DASH_T


def charge(d):
    hand, k = ((26, 26), 13) if d == "down_right" else ((27, 25), 12)
    return [pose(d, hand, k, length=10, squash=1, flare=1 if i % 2 == 0 else 0,
                 tip=["stream", "up", "stream", "droop"][i], charged=2 if i in (1, 2) else 1,
                 glint=[1, 4, 7, None][i], eyes={"e": "W"} if i in (1, 2) else None,
                 feet=planted(d, wide=1)) for i in range(4)], a.CHARGE_T


def charge_walk(d):
    charged, _ = charge(d)
    soles = gait(d, slow=True)
    out = []
    for i in range(8):
        p = deepcopy(charged[i % 4])
        p["under"] = soles[i]
        # Charge grip and upper body stay fixed: only the slow leg cycle changes.
        out.append(p)
    return out, a.CHARGE_WALK_T


def hurt(d):
    hand, k = REST[d]
    return [pose(d, (hand[0] - 1, hand[1] - 5), 12, body_dx=-1,
                 top_slide=-1, hood_dx=-1, hood_dy=-1, tip="up", eyes={"e": "W"},
                 feet=planted(d, lift=1)),
            pose(d, (hand[0] - 1, hand[1] + 1), k + 1, squash=1, flare=1,
                 hood_dy=1, eyes={"e": "v"}, feet=planted(d, wide=1)),
            pose(d, hand, k)], a.HURT_T


def diagonal(base, d):
    """Return a complete native diagonal animation, with explicit west relighting."""
    rows, ticks = globals()[base](canonical(d))
    if west(d):
        for p in rows:
            p["flip"] = True
    return rows, ticks


def collapse(d):
    """Same five authored death holds, bowed and settled in the held facing."""
    src = canonical(d)
    if src == "down":
        return a.death()
    if src in REST:
        recoil = hurt(src)[0][0]
        hand, k = REST[src]
        sole = lambda: planted(src, wide=1)
    else:
        recoil = a.hurt(src)[0][0]
        hand, k = (a.REST[src]["hx"], a.REST[src]["hy"]), a.REST[src]["k"]
        sole = lambda: a.stand_feet(src)
    # Bowed head is the actual three-quarter/back/side crown, its eye extinguished;
    # it sinks into the cloak rather than switching to the legacy front death art.
    out = [recoil,
           a.pose(src, hand, k, squash=2, hood_dy=1, flare=1, feet=sole()),
           a.pose(src, (hand[0], hand[1] + 1), 4, length=8, squash=3,
                  hood_dy=2, lift=-1, flare=2, eyes={"e": "c"}, feet=sole()),
           a.pose(src, (hand[0], 30), 6, length=8, squash=4,
                  hood_dy=3, lift=-2, flare=2, eyes={"e": "v"}, feet=[]),
           a.pose(src, (hand[0] - 2, 31), 8, length=8, squash=4,
                  hood_dy=4, lift=-2, flare=3, eyes={"e": "v"}, feet=[], arm=False, glove=False)]
    out[-1]["blade"]["dim"] = True
    if west(d):
        for p in out:
            p["flip"] = True
    return out, a.DEATH_T


def spin(d):
    """One clockwise revolution with an authored pose at every 45-degree step.

    Initial facing is retained. The eight native blade angles and grip positions
    share the same phase as the runtime trail; west keys are relit individually,
    never mirrored as an entire clockwise animation.
    """
    start = DIRECTIONS.index(d)
    out = []
    hands = {"up": (20, 19), "up_right": (25, 21), "right": (27, 25),
             "down_right": (25, 28), "down": (20, 29), "down_left": (15, 28),
             "left": (13, 25), "up_left": (15, 21)}
    for i in range(8):
        facing = DIRECTIONS[(start + i) % 8]
        src = canonical(facing)
        k = ((start + i) * 2 + 12) % 16
        hx, hy = hands[facing]
        flip = west(facing)
        if flip:
            hx, k = 40 - hx, (8 - k) % 16
        soles = planted(src, wide=1) if src in REST else a.stand_feet(src)
        p = a.pose(src, (hx, hy), k, length=9 if i == 7 else 10,
                   squash=2 if i == 7 else 1, flare=1 if i == 7 else 2,
                   tip="droop" if i == 7 else "stream",
                   behind=facing in ("up_left", "up", "up_right"), feet=soles)
        p["flip"] = flip
        out.append(p)
    return out, a.SPIN_T


def fall(d):
    src = canonical(d)
    hand, k = REST[src] if src in REST else ((a.REST[src]["hx"], a.REST[src]["hy"]), a.REST[src]["k"])
    soles = planted(src, lift=2) if src in REST else a.stand_feet(src, -2)
    frames = [a.pose(src, (hand[0], hand[1] - 1), k, stretch=1,
                     hood_dy=-1, tip="up", feet=soles) for _ in range(2)]
    frames[1]["hood"] = ("n", 0, 0)
    for p in frames:
        p["flip"] = west(d)
    return frames, [8, 8]


def land(d):
    src = canonical(d)
    hand, k = REST[src] if src in REST else ((a.REST[src]["hx"], a.REST[src]["hy"]), a.REST[src]["k"])
    soles = planted(src) if src in REST else a.stand_feet(src)
    frames = [a.pose(src, hand, k, squash=s, hood_dy=h, flare=f, feet=soles)
              for s, h, f in [(2, 1, 2), (1, 1, 1), (0, 0, 0)]]
    for p in frames:
        p["flip"] = west(d)
    return frames, [4, 4, 4]
