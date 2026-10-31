"""HUD sheets: hud_hearts, hud_shard, hud_bossbar (+ hud_bossbar_fill), hud_prompts."""

import os

import numpy as np

from common import SPRITES, UI, new, rgb, save_json, save_png, stamp, grid
from fonts import font_tiny

INK = UI["ink"][0]          # #252d3c
HP = UI["health"]           # 53424d 89636e b98087 d8b0ab (style v2 dried rose)
BONE = UI["text"][1]        # #dce0ce
SLATE = UI["panel"]         # 303949 46505b
ASH = UI["border"]          # 6c7a83 98a49f

# ===================================================================== health marks
# Style v2: small straight marks (5x5, one clipped lower-right corner) instead of hearts.
MARK = [
    "33332",
    "32222",
    "22221",
    "2221.",
    "111..",
]
KEY_FULL = {"1": HP[1], "2": HP[2], "3": HP[3]}
KEY_FLASH = {"1": HP[3], "2": BONE, "3": BONE}
# empty: the same cut footprint carved into the plate as a socket. Lit from the upper left, its
# top/left walls fall into ink shadow, the floor is the plate itself and only the lower-right
# lip catches ash light, so a lost mark reads as a hole rather than a dim filled mark.
SOCKET = [
    "ssssm",
    "sfffl",
    "sfffl",
    "sffl.",
    "mll..",
]
KEY_SOCKET = {"s": INK, "f": SLATE[0], "l": ASH[0], "m": SLATE[1]}

HCELL = (15, 14)
HOFF = (2, 2)


def heart_cell(key):
    """One mark in a 15x14 cell (the margin is room for the loss chips)."""
    img = new(*HCELL)
    stamp(img, grid(MARK), key, HOFF[0], HOFF[1])
    return img


def socket_cell():
    """An empty slot: the mark's footprint carved into the plate."""
    img = new(*HCELL)
    stamp(img, grid(SOCKET), KEY_SOCKET, HOFF[0], HOFF[1])
    return img


def half_cell():
    """Half health: the mark's left three columns, the socket's right two."""
    img = new(*HCELL)
    stamp(img, [r[:3] for r in grid(MARK)], KEY_FULL, HOFF[0], HOFF[1])
    stamp(img, ["..." + r[3:] for r in grid(SOCKET)], KEY_SOCKET, HOFF[0], HOFF[1])
    return img


def heart_crack():
    """Frame 2 of 'lose': the mark splits along its own cut, the halves pushed 1 px apart."""
    img = new(*HCELL)
    rows = grid(MARK)
    upper = ["".join(ch if x + y < 4 else "." for x, ch in enumerate(r)) for y, r in enumerate(rows)]
    lower = ["".join(ch if x + y > 4 else "." for x, ch in enumerate(r)) for y, r in enumerate(rows)]
    stamp(img, upper, KEY_FLASH, HOFF[0] - 1, HOFF[1] - 1)
    stamp(img, lower, KEY_FULL, HOFF[0] + 1, HOFF[1] + 1)
    return img


def bits(img, pts, col):
    """Set single pixels (loss chips) to one color."""
    for x, y in pts:
        if 0 <= x < img.shape[1] and 0 <= y < img.shape[0]:
            img[y, x, :3] = rgb(col)
            img[y, x, 3] = 255


def build_hearts():
    """hud_hearts: full, half, empty, flash, lose and gain states of the health marks."""
    full = heart_cell(KEY_FULL)
    half = half_cell()
    empty = socket_cell()
    flash = heart_cell(KEY_FLASH)
    # lose: flash -> split -> empty slot with two chips thrown down-right -> chips fall, darken -> empty
    l2 = socket_cell()
    bits(l2, [(8, 3), (9, 3)], HP[3])
    bits(l2, [(8, 7)], HP[2])
    l3 = socket_cell()
    bits(l3, [(10, 5), (11, 5)], HP[2])
    bits(l3, [(9, 10)], HP[1])
    lose = [flash, heart_crack(), l2, l3, empty]
    gain = [empty, heart_cell(KEY_FLASH), full]
    rows = [[full], [half], [empty], [flash], lose, gain]
    ncol = max(len(r) for r in rows)
    sheet = new(HCELL[0] * ncol, HCELL[1] * len(rows))
    for ri, r in enumerate(rows):
        for ci, c in enumerate(r):
            sheet[ri * HCELL[1]:(ri + 1) * HCELL[1], ci * HCELL[0]:(ci + 1) * HCELL[0]] = c
    save_png(sheet, os.path.join(SPRITES, "hud_hearts.png"))
    man = {
        "cell": list(HCELL),
        "anchor": [HOFF[0], HOFF[1]],
        "emissive": [],
        "anims": {
            "full": {"row": 0, "frames": 1, "fps": 1},
            "half": {"row": 1, "frames": 1, "fps": 1},
            "empty": {"row": 2, "frames": 1, "fps": 1},
            "flash": {"row": 3, "frames": 1, "fps": 1},
            "lose": {"row": 4, "frames": 5, "fps": 15, "durations": [3, 4, 4, 4, 1]},
            "gain": {"row": 5, "frames": 3, "fps": 15, "durations": [2, 4, 1]},
        },
        "heart_size": [5, 5],
        "spacing": 8,
        "notes": "Style v2 player health: small straight marks (5x5, lower-right corner clipped) in the dried-rose "
                 "ui.health ramp; an empty mark is the same footprint carved into the plate (ink top/left wall, plate "
                 "floor, ash lower-right lip), so it reads as a hole. The 15x14 cell leaves room for "
                 "the 'lose' chips. Anchor = the mark's top-left pixel: draw mark i at (x0 + 8*i, y0) on a slate "
                 "#303949 plate. Play 'lose' once on a mark that empties (bone flash, split along the cut, chips "
                 "fall away), 'gain' on one that fills, then hold 'full'/'empty'. UI layer, unlit, no glow.",
    }
    save_json(man, os.path.join(SPRITES, "hud_hearts.json"))
    return sheet, man


# ===================================================================== shard
# The Prism's fragment: pale iris body, one cyan lit facet (cyan belongs to Glint and the Prism).
SHARD = [
    "..o...",
    ".oco..",
    ".ocpo.",
    "occpo.",
    "ocbpqo",
    "obbpqo",
    "obpqqo",
    ".obqo.",
    ".opqo.",
    "..oo..",
]
SHARD_KEY = {"o": "#38324a", "c": "#63d7e0", "b": "#b7add0", "p": "#8d7fa8", "q": "#564d6e"}
SHARD_FLASH = {"o": "#38324a", "c": BONE, "b": BONE, "p": "#e2d8e9", "q": "#b7add0"}
SCELL = (10, 14)


def build_shard():
    """hud_shard: the shard counter icon, idle and a three-frame pop."""
    def cell(key, dy=0):
        """One icon frame, nudged dy pixels."""
        img = new(*SCELL)
        stamp(img, grid(SHARD), key, 1, 2 + dy)
        return img

    idle = cell(SHARD_KEY)
    pop = [cell(SHARD_FLASH, -2), cell(SHARD_KEY, -1), cell(SHARD_KEY, 0)]
    sheet = new(SCELL[0] * 3, SCELL[1] * 2)
    sheet[0:SCELL[1], 0:SCELL[0]] = idle
    for i, c in enumerate(pop):
        sheet[SCELL[1]:, i * SCELL[0]:(i + 1) * SCELL[0]] = c
    save_png(sheet, os.path.join(SPRITES, "hud_shard.png"))
    man = {
        "cell": list(SCELL),
        "anchor": [1, 2],
        "emissive": [],
        "anims": {
            "idle": {"row": 0, "frames": 1, "fps": 1},
            "pop": {"row": 1, "frames": 3, "fps": 15, "durations": [3, 3, 4]},
        },
        "icon_size": [6, 10],
        "notes": "Style v2 Prism shard counter icon: a 6x10 fractured fragment in the fx.prism iris ramp with one "
                 "cyan lit facet, outlined in deep iris #38324a, matching shard.png. Anchor = icon top-left. The "
                 "count follows in font_small, bone #dce0ce. Play 'pop' when the count goes up (bone flash, hop "
                 "up 2 px, settle). UI layer, unlit, no glow.",
    }
    save_json(man, os.path.join(SPRITES, "hud_shard.json"))
    return sheet, man


# ===================================================================== boss bar
# Style v2: a long, thin slate well with a blade-cut right end and a small cut-stone emblem holding
# the Warden's pale iris heart (summit prism ramp). No gold, no cyan.
WS = ["#333b4c", "#555d70", "#7f8797", "#b0b7bb", "#d9dacc"]   # summit warden_stone
IRIS = ["#564d6e", "#8d7fa8", "#b7add0", "#e2d8e9"]            # summit prism
BAR_W = 208
BAR_H = 19
EMB = 11
FILL_H = 3
FILL_Y0 = BAR_H // 2 - 1
FILL_X0 = 14
CAP = 4
# Interior rows of the empty well, top to bottom. The lower floor rows are lit slate so a missing
# stretch reads as an open trough against dark summit floors; full health covers all three rows.
WELL_FLOOR = [SLATE[0], SLATE[1], SLATE[1]]


def build_bossbar():
    """hud_bossbar frame plus its 1 px fill columns."""
    img = new(BAR_W, BAR_H)
    c = BAR_H // 2

    def px(x, y, col):
        """Set one opaque pixel."""
        img[y, x, :3] = rgb(col)
        img[y, x, 3] = 255
    # well: ink rim, slate interior, a blade-cut point on the right
    x_end = BAR_W - 1
    for x in range(FILL_X0 - 2, x_end + 1):
        k = x_end - x
        for y in range(FILL_Y0 - 1, FILL_Y0 + FILL_H + 1):
            # the right end is cut on a 45-degree diagonal running down-left
            if k < (y - (FILL_Y0 - 1)):
                continue
            edge = y in (FILL_Y0 - 1, FILL_Y0 + FILL_H) or x == FILL_X0 - 2 or k == (y - (FILL_Y0 - 1))
            if edge:
                px(x, y, INK)
            elif x < BAR_W - CAP - 1:
                px(x, y, WELL_FLOOR[y - FILL_Y0])
            else:
                # the tip past fill_rect never fills, so it keeps the plain floor: a full bar shows no gap
                px(x, y, SLATE[0])
    # a lit hairline along the top rim (light from above), stopping before the cut
    for x in range(FILL_X0, x_end - 1):
        px(x, FILL_Y0 - 2, ASH[0])
    # emblem: an 11x11 cut-stone diamond with a 3x3 iris heart
    r = EMB // 2
    for y in range(EMB):
        for x in range(EMB):
            dx, dy = x - r, y - r
            d = abs(dx) + abs(dy)
            if d > r:
                continue
            up = dy < 0 or (dy == 0 and dx < 0)
            if d == r:
                col = INK
            elif d == r - 1:
                col = WS[3] if up else WS[1]
            elif d >= 2:
                col = WS[2] if up else WS[1]
            elif d == 1:
                col = IRIS[2] if up else IRIS[1]
            else:
                col = IRIS[3]
            px(x, c - r + y, col)
    save_png(img, os.path.join(SPRITES, "hud_bossbar.png"))

    cols = {
        "fill": [HP[3], HP[2], HP[1]],
        "chip": [BONE, ASH[1], ASH[1]],
        "flash": [BONE] * FILL_H,
    }
    fsheet = new(len(cols), FILL_H)
    for i, (k, v) in enumerate(cols.items()):
        for y, cc in enumerate(v):
            fsheet[y, i, :3] = rgb(cc)
            fsheet[y, i, 3] = 255
    save_png(fsheet, os.path.join(SPRITES, "hud_bossbar_fill.png"))
    fill_rect = [FILL_X0 - 1, FILL_Y0, BAR_W - CAP - FILL_X0, FILL_H]
    man = {
        "cell": [BAR_W, BAR_H],
        "anchor": [BAR_W // 2, 0],
        "emissive": [],
        "anims": {"frame": {"row": 0, "frames": 1, "fps": 1}},
        "fill_rect": fill_rect,
        "notes": "Style v2 Warden health bar: a thin slate well (3 px fill) with an ink rim, a lit ash hairline above, "
                 "a lit slate floor where health is missing and a 45-degree blade-cut right end, plus a small cut-stone emblem holding the Warden's pale iris "
                 "heart. Anchor = top center. fill_rect [x, y, w, h] is the well in cell coords; fill it from the "
                 "left with hud_bossbar_fill columns: 0 = health (dried rose), 1 = recently lost chip (bone/ash), "
                 "2 = bone hit flash. Name: font_small 'The Warden' above the bar at x = fill_rect.x.",
    }
    save_json(man, os.path.join(SPRITES, "hud_bossbar.json"))
    save_json({"cell": [1, FILL_H], "anchor": [0, 0], "emissive": [],
               "anims": {"fill": {"row": 0, "frames": 1, "fps": 1}},
               "columns": {"fill": 0, "chip": 1, "flash": 2},
               "notes": "1 px wide fill columns for hud_bossbar's fill_rect; repeat a column horizontally."},
              os.path.join(SPRITES, "hud_bossbar_fill.json"))
    return img, fsheet, man


# ===================================================================== button prompts
P_FACE = UI["panel"][1]     # #46505b top face
P_LIT = UI["border"][0]     # #6c7a83 lit top edge
P_LIP = UI["panel"][0]      # #303949 front lip
P_INK = INK
P_TXT = UI["text"][1]       # #dce0ce glyph
P_DIM = UI["text"][0]       # #9ba79e dimmed glyph parts
PCELL = (29, 13)

ARROWS = {
    "up": ["..#..", ".###.", "#####"],
    "down": ["#####", ".###.", "..#.."],
    "left": ["..#", ".##", "###", ".##", "..#"],
    "right": ["#..", "##.", "###", "##.", "#.."],
}
MENU5 = ["#####", ".....", "#####", ".....", "#####"]


def rrect(w, h, r=1):
    """Rounded-rect mask: corners clipped by r pixels (r=1 clips the single corner pixel)."""
    m = np.ones((h, w), bool)
    for i in range(r):
        for j in range(r - i):
            for (y, x) in ((i, j), (i, w - 1 - j), (h - 1 - i, j), (h - 1 - i, w - 1 - j)):
                m[y, x] = False
    return m


def disk(d):
    """Pixel disk of diameter d, hand-checked shapes for the sizes we use."""
    shapes = {
        9: ["..#####..", ".#######.", "#########", "#########", "#########", "#########", "#########",
            ".#######.", "..#####.."],
        7: ["..###..", ".#####.", "#######", "#######", "#######", ".#####.", "..###.."],
        3: [".#.", "###", ".#."],
    }
    rows = shapes[d]
    return np.array([[c == "#" for c in r] for r in rows])


def cap(face, depth, pressed):
    """Face mask -> RGBA cap: lit top edge, front lip `depth` px, rounded 4-neighbour outline.
    Returns (img, face_offset_y). The pressed cap sits 1 px lower with a 1 px lip."""
    fh, fw = face.shape
    H = fh + depth + 2
    W = fw + 2
    d = depth - 1 if pressed else depth
    oy = 2 if pressed else 1
    F = np.zeros((H, W), bool)
    F[oy:oy + fh, 1:1 + fw] = face
    lip = np.zeros_like(F)
    for k in range(1, d + 1):
        lip[k:, :] |= F[:-k, :]
    lip &= ~F
    solid = F | lip
    out = np.zeros_like(solid)
    out[1:, :] |= solid[:-1, :]
    out[:-1, :] |= solid[1:, :]
    out[:, 1:] |= solid[:, :-1]
    out[:, :-1] |= solid[:, 1:]
    out &= ~solid
    img = new(W, H)

    def paint(m, c):
        img[m, :3] = rgb(c)
        img[m, 3] = 255
    paint(out, P_INK)
    paint(lip, P_LIP)
    paint(F, P_FACE)
    top = F & ~np.roll(F, 1, 0)
    paint(top, P_LIT)
    return img, oy


def glyph_mask(text, tiny):
    if text in ARROWS:
        rows = ARROWS[text]
        return np.array([[c == "#" for c in r] for r in rows])
    w = tiny.width(text)
    m = np.zeros((tiny.cell_h, w), bool)
    x = 0
    for ch in text:
        g = tiny.masks[ch]
        m[:, x:x + g.shape[1]] |= g
        x += g.shape[1] + tiny.tracking
    return m[:5]


def put_mask(img, m, x0, y0, col):
    ys, xs = np.nonzero(m)
    for y, x in zip(ys, xs):
        img[y0 + y, x0 + x, :3] = rgb(col)
        img[y0 + y, x0 + x, 3] = 255


def key(text, tiny, min_w=9):
    g = glyph_mask(text, tiny) if text else np.zeros((5, 1), bool)
    fw = max(min_w, g.shape[1] + 4)
    if (fw - g.shape[1]) % 2:
        fw += 1
    face = rrect(fw, 7, 1)
    frames = []
    for pressed in (False, True):
        img, oy = cap(face, 2, pressed)
        if text:
            gx = 1 + (fw - g.shape[1]) // 2
            gy = oy + 1 + (5 - g.shape[0] + 1) // 2 if g.shape[0] < 5 else oy + 1
            put_mask(img, g, gx, gy, P_TXT)
        frames.append(img)
    return frames


def round_button(text, tiny):
    face = disk(9)
    g = glyph_mask(text, tiny)
    frames = []
    for pressed in (False, True):
        img, oy = cap(face, 2, pressed)
        gx = 1 + (9 - g.shape[1]) // 2
        put_mask(img, g, gx, oy + 2, P_TXT)
        frames.append(img)
    return frames


def pad_position(which):
    """Platform-neutral face-button prompt: four small buttons in a diamond, the active one lit."""
    pos = {"north": (3, 0), "west": (0, 3), "east": (6, 3), "south": (3, 6)}
    dot = np.zeros((4, 4), bool)
    dot[1:3, 1:3] = True
    ring = np.zeros_like(dot)
    ring[:-1] |= dot[1:]
    ring[1:] |= dot[:-1]
    ring[:, :-1] |= dot[:, 1:]
    ring[:, 1:] |= dot[:, :-1]
    ring &= ~dot
    frames = []
    for pressed in (False, True):
        img = new(10, 11)
        for name, (x, y) in pos.items():
            active = name == which
            dy = 1 if (active and pressed) else 0
            put_mask(img, ring, x, y + dy, P_INK)
            put_mask(img, dot, x, y + dy, P_TXT if active else P_LIT)
        frames.append(img)
    return frames


def shoulder(text, tiny, trigger):
    """Bumper (flat pill) or trigger (taller, rounded on top) with its label."""
    g = glyph_mask(text, tiny)
    fw = g.shape[1] + 4
    face = rrect(fw, 8 if trigger else 7, 2 if trigger else 1)
    if trigger:
        face[6:, :] = True
    frames = []
    for pressed in (False, True):
        img, oy = cap(face, 2, pressed)
        put_mask(img, g, 3, oy + face.shape[0] - 6, P_TXT)
        frames.append(img)
    return frames


def dpad(which):
    """D-pad cross; the pressed direction's arm is lit."""
    solid = np.zeros((9, 9), bool)
    solid[:, 3:6] = True
    solid[3:6, :] = True
    arms = {"up": (slice(0, 3), slice(3, 6)), "down": (slice(6, 9), slice(3, 6)),
            "left": (slice(3, 6), slice(0, 3)), "right": (slice(3, 6), slice(6, 9))}
    frames = []
    for pressed in (False, True):
        img, oy = cap(solid, 2, False)
        if which in arms:
            m = np.zeros(img.shape[:2], bool)
            ys, xs = arms[which]
            sub = np.zeros((9, 9), bool)
            sub[ys, xs] = True
            m[oy:oy + 9, 1:10] = sub
            img[m, :3] = rgb(P_DIM if pressed else P_TXT)
        img[oy + 4, 5, :3] = rgb(P_LIP)   # pivot
        frames.append(img)
    return frames


def stick(text, tiny):
    frames = []
    for pressed in (False, True):
        img, oy = cap(disk(9), 2, pressed)
        # concave thumb pad: a darker inner disk
        inner = np.zeros(img.shape[:2], bool)
        inner[oy + 1:oy + 8, 2:9] = disk(7)
        img[inner, :3] = rgb(P_LIP)
        g = glyph_mask(text, tiny)
        put_mask(img, g, 1 + (9 - g.shape[1]) // 2, oy + 2, P_TXT)
        frames.append(img)
    return frames


def menu_button():
    """Start / menu button: rounded cap with three lines."""
    face = rrect(9, 7, 2)
    lines = np.array([[c == "#" for c in r] for r in MENU5])
    frames = []
    for pressed in (False, True):
        img, oy = cap(face, 2, pressed)
        put_mask(img, lines, 3, oy + 1, P_TXT)
        frames.append(img)
    return frames


def build_prompts():
    tiny = font_tiny()
    items = []
    for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        items.append((f"key_{c.lower()}", key(c, tiny)))
    for c in "0123456789":
        items.append((f"key_{c}", key(c, tiny)))
    for a in ("up", "down", "left", "right"):
        items.append((f"key_{a}", key(a, tiny)))
    items.append(("key_space", key("SPACE", tiny, 25)))
    items.append(("key_enter", key("ENTER", tiny)))
    items.append(("key_shift", key("SHIFT", tiny)))
    items.append(("key_esc", key("ESC", tiny)))
    items.append(("key_tab", key("TAB", tiny)))
    items.append(("key_blank", key("", tiny)))
    items.append(("key_blank_wide", key("", tiny, 15)))
    for c in "ABXY":
        items.append((f"pad_{c.lower()}", round_button(c, tiny)))
    for p in ("south", "east", "west", "north"):
        items.append((f"pad_{p}", pad_position(p)))
    items.append(("pad_lb", shoulder("LB", tiny, False)))
    items.append(("pad_rb", shoulder("RB", tiny, False)))
    items.append(("pad_lt", shoulder("LT", tiny, True)))
    items.append(("pad_rt", shoulder("RT", tiny, True)))
    for d in ("none", "up", "down", "left", "right"):
        items.append((f"pad_dpad" + ("" if d == "none" else f"_{d}"), dpad(d)))
    items.append(("pad_lstick", stick("L", tiny)))
    items.append(("pad_rstick", stick("R", tiny)))
    items.append(("pad_menu", menu_button()))

    cw, ch = PCELL
    sheet = new(cw * 2, ch * len(items))
    anims = {}
    sizes = {}
    for row, (name, frames) in enumerate(items):
        h, w = frames[0].shape[:2]
        assert w <= cw and h <= ch, (name, w, h)
        for i, f in enumerate(frames):
            y0 = row * ch + (ch - h)   # bottom-aligned in the cell
            sheet[y0:y0 + h, i * cw:i * cw + w] = f
        anims[name] = {"row": row, "frames": 2, "fps": 4}
        sizes[name] = [w, h]
    save_png(sheet, os.path.join(SPRITES, "hud_prompts.png"))
    man = {
        "cell": [cw, ch],
        "anchor": [0, ch - 1],
        "emissive": [],
        "anims": anims,
        "sizes": sizes,
        "notes": "Button prompt glyphs. Each prompt is a row: frame 0 = up, frame 1 = pressed (play the anim at "
                 "4 fps for a 'tap this' blink, or hold frame 1 while the button is held). Glyphs are left- and "
                 "bottom-aligned in the cell; anchor = bottom-left; sizes[name] = [w, h] of the drawn glyph, for "
                 "laying out text after it. Inline with font_small: put the glyph's bottom row on the text "
                 "baseline + 2 (key caps are 11 px tall, the text caps 7 px) and leave 2 px on each side. "
                 "pad_south/east/west/north are platform-neutral face-button positions (the lit dot is the button); "
                 "pad_a/b/x/y are labelled Xbox-style. key_blank / key_blank_wide + font_tiny can make any other key.",
    }
    save_json(man, os.path.join(SPRITES, "hud_prompts.json"))
    return sheet, man





# ===================================================================== panel + cursor
PANEL = [
    ".ooooooo.",
    "oaaaaaaao",
    "oapppppao",
    "oapppppao",
    "oapppppao",
    "oapppppao",
    "oapppppao",
    "oaaaaaaao",
    ".ooooooo.",
]
PANEL_KEY = {"o": INK, "a": UI["panel"][1], "p": UI["panel"][0]}

# a slim bone pennant with a blade-cut point; ash on its shaded lower edge
CURSOR = [
    ".......",
    "oo.....",
    "o3oo...",
    "o3332o.",
    "o2oo...",
    "oo.....",
    ".......",
]
CURSOR_KEY = {"o": INK, "3": UI["text"][1], "2": UI["border"][1]}


def build_panel():
    """hud_panel: the quiet slate nine-slice."""
    img = new(9, 9)
    stamp(img, grid(PANEL), PANEL_KEY, 0, 0)
    save_png(img, os.path.join(SPRITES, "hud_panel.png"))
    save_json({
        "cell": [9, 9], "anchor": [0, 0], "emissive": [],
        "anims": {"frame": {"row": 0, "frames": 1, "fps": 1}},
        "slice": [4, 4, 4, 4],
        "notes": "9-slice panel for dialog lines and the pause menu. slice = [left, top, right, bottom] px: "
                 "the 4x4 corners are drawn once, the 1 px middle row/column is repeated to fill edges and the "
                 "center (style v2: slate #303949 with a flat #46505b rim and ink #252d3c edge, no bevel). Min "
                 "size 9x9. Text inside: font_small in bone #dce0ce, 4 px padding from the outer edge. Dialog box suggestion: 368x40 at (8, 168).",
    }, os.path.join(SPRITES, "hud_panel.json"))
    return img


def build_cursor():
    """hud_cursor: the menu marker, nudging 1 px right and back."""
    cw, ch = 12, 7
    sheet = new(cw * 2, ch)
    stamp(sheet, grid(CURSOR), CURSOR_KEY, 0, 0)
    stamp(sheet, grid(CURSOR), CURSOR_KEY, cw + 1, 0)
    save_png(sheet, os.path.join(SPRITES, "hud_cursor.png"))
    save_json({
        "cell": [cw, ch], "anchor": [0, 3], "emissive": [],
        "anims": {"idle": {"row": 0, "frames": 2, "fps": 3}},
        "notes": "Menu cursor, style v2: a slim bone pennant with a blade-cut point (no cyan, no gold). "
                 "Anchor = left middle; put it 3 px left of the selected item with its middle row on the text's "
                 "x-height middle. 'idle' nudges it 1 px right and back.",
    }, os.path.join(SPRITES, "hud_cursor.json"))
    return sheet


if __name__ == "__main__":
    build_hearts()
    build_shard()
    build_bossbar()
    build_prompts()
    build_panel()
    build_cursor()
