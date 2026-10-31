"""bloomwild_ruins: white-stone ruin pieces (pillars, drum, walls, statue) and bloomwild_arch.

3/4 top-down: top faces are lit (s4/s5), south-facing fronts mid (s3/s4), the shadow side and
undersides dark (s1/s2). Cylinders are shaded across their width (lit left, shaded right).
Dark outline (s0) only on the shadow side and the ground contact. Moss only as clusters that
sit on top faces and ledges, where it would actually grow.
"""
import math

from pix import *

CW, CH = 48, 56
AX, AY = 24, 53

# cylinder shading by column, left to right (width 11)
CYL11 = ["s4", "s5", "s5", "s5", "s4", "s4", "s4", "s3", "s3", "s2", "s1"]


def cyl_cols(w):
    """Shading for a cylinder w px wide, resampled from the 11-wide reference."""
    return [CYL11[min(10, int((i + 0.5) / w * 11))] for i in range(w)]


DARKER = {"s5": "s4", "s4": "s3", "s3": "s2", "s2": "s1", "s1": "s1"}
LIGHTER = {"s1": "s2", "s2": "s3", "s3": "s4", "s4": "s5", "s5": "s5"}


def block(img, x0, y_top, w, top_d, front_h, top_shade=("s5", "s4"), front=("s4", "s3", "s2")):
    """A box seen in 3/4: top face (top_d rows) then front face (front_h rows).
    Top: front edge row s5, rest s4. Front: left col lighter, right col darker, bottom row darker."""
    for y in range(y_top, y_top + top_d):
        for x in range(x0, x0 + w):
            c = top_shade[0] if y == y_top + top_d - 1 else top_shade[1]
            if x == x0 + w - 1:
                c = "s3"
            put(img, x, y, c)
    for y in range(y_top + top_d, y_top + top_d + front_h):
        for x in range(x0, x0 + w):
            c = front[1]
            if x == x0:
                c = front[0]
            if x >= x0 + w - 2:
                c = front[2]
            if y == y_top + top_d + front_h - 1:
                c = DARKER[c]
            put(img, x, y, c)


def shaft(img, cx, y0, y1, w, flutes=True):
    """Shaft on the native pixel grid."""
    cols = cyl_cols(w)
    x0 = cx - w // 2
    for y in range(y0, y1 + 1):
        for i, c in enumerate(cols):
            put(img, x0 + i, y, c)
    if flutes:
        for i in range(2, w - 1, 3):
            for y in range(y0, y1 + 1):
                put(img, x0 + i, y, DARKER[cols[i]])


def ellipse_top(img, cx, cy, rx, ry, jag=None, seed=0):
    """Top face of a cylinder (lit), with an optional jagged broken edge."""
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            u, v = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
            if u * u + v * v <= 1.0:
                c = "s5" if (u + v) < 0.2 else "s4"
                put(img, x, y, c)


def outline_shadow_side(img, base_rows=1):
    """s0 outline on transparent pixels right of / below the shape, plus the ground contact."""
    m = mask_of(img)
    h, w = m.shape
    out = img.copy()
    for y in range(h):
        for x in range(w):
            if m[y, x]:
                continue
            if x > 0 and m[y, x - 1]:
                out[y, x] = rgba("s0")
            elif y > 0 and m[y - 1, x]:
                out[y, x] = rgba("s0")
    return out


def moss(img, pts):
    """Moss on the native pixel grid."""
    for (x, y, c) in pts:
        put(img, x, y, c)


def moss_cluster(img, x, y, shape):
    """shape rows: G lit (g3), g body (g2), k dark (g1), t tip (g4)."""
    grid(shape, {"G": "g3", "g": "g2", "k": "g1", "t": "g4"}, img, x, y)


MOSS_TOP_A = ["..GGG..",
              ".GgGgk.",
              "Ggggkk.",
              ".kk.k.."]
MOSS_TOP_B = [".GG.",
              "Ggk.",
              ".k.."]
MOSS_DRAPE = ["GGgk",
              "gk..",
              "k...",
              "g...",
              "k..."]


# ================================================================ pieces
def pillar_intact():
    """Pillar intact on the native pixel grid."""
    img = new(CW, CH)
    cx = 24
    block(img, 16, 44, 16, 3, 6)                 # plinth y 44..52
    shaft(img, cx, 17, 43, 11)                   # shaft
    block(img, 17, 14, 14, 2, 3)                 # capital lower slab (y 14..18)
    block(img, 16, 10, 16, 3, 3)                 # abacus (y 10..15)
    # necking ring under the capital
    for x in range(19, 30):
        put(img, x, 19, "s2" if x > 26 else "s3")
    moss_cluster(img, 16, 8, MOSS_TOP_A)
    moss_cluster(img, 16, 44, ["GG..", "gk.."])
    img = outline_shadow_side(img)
    return img


def pillar_broken(top_y, seed):
    """Broken column: plinth, shaft up to top_y, jagged broken top face."""
    img = new(CW, CH)
    cx = 24
    block(img, 16, 44, 16, 3, 6)
    shaft(img, cx, top_y + 2, 43, 11)
    # broken top: an ellipse top face with a jagged rim, higher on the left
    prof = [2, 1, 0, 0, 1, 2, 3, 2, 3, 4, 4] if seed == 0 else [1, 0, 1, 2, 2, 3, 2, 1, 2, 3, 4]
    for i, d in enumerate(prof):
        x = cx - 5 + i
        top = top_y + d
        for y in range(top, top_y + 4):
            put(img, x, y, "s5" if i < 5 else "s4")
        # break surface front lip
        put(img, x, top_y + 4, "s4" if i < 6 else "s3")
    # clear above the jag
    for i, d in enumerate(prof):
        for y in range(top_y - 2, top_y + d):
            put(img, cx - 5 + i, y, None)
    # rubble chunks at the base
    grid(["hS", "Sm"], {"h": "s5", "S": "s4", "m": "s2"}, img, 31, 50)
    grid(["hSS", "Smm"], {"h": "s5", "S": "s4", "m": "s2"}, img, 12, 50)
    if seed == 0:
        moss_cluster(img, cx - 5, top_y - 1, MOSS_TOP_B)
    else:
        moss_cluster(img, cx - 3, top_y - 2, MOSS_TOP_A[:3])
    img = outline_shadow_side(img)
    return img


def drum():
    """A fluted column drum lying on its side, its end face turned toward the upper left.
    Shaded as a cylinder (lit top, mid front, dark underside); the flutes run along its length
    and crowd together toward the top and bottom edges as the surface turns away."""
    img = new(CW, CH)
    D = 13
    y0, x0, x1 = 39, 15, 38
    def shade(f):
        """Shade on the native pixel grid."""
        a = math.pi * f
        lum = 0.66 * math.cos(a) + 0.56 * math.sin(a)
        return "s5" if lum > 0.72 else "s4" if lum > 0.42 else "s3" if lum > 0.05 else "s2" if lum > -0.4 else "s1"
    rowc = [shade((i + 0.5) / D) for i in range(D)]
    for i in range(D):
        for x in range(x0, x1 + 1):
            put(img, x, y0 + i, rowc[i])
    for k in range(1, 7):
        f = (1 - math.cos(k * math.pi / 7)) / 2
        i = min(D - 1, int(f * D))
        for x in range(x0 + 3, x1 + 1):
            put(img, x, y0 + i, DARKER[rowc[i]])
    # broken right end: jagged profile with the rough break face (s3/s2) showing
    prof = [1, 2, 2, 3, 2, 1, 2, 3, 4, 3, 3, 2, 1]
    for i, d in enumerate(prof):
        for k in range(d + 1):
            put(img, x1 - k + 1, y0 + i, None)
        put(img, x1 - d, y0 + i, "s3" if i < 7 else "s2")
    # end face: an upright ellipse turned toward the light, its rim scalloped by the flutes
    cy = y0 + D / 2
    for y in range(y0 - 1, y0 + D + 1):
        for x in range(x0 - 5, x0 + 3):
            u, v = (x + 0.5 - (x0 - 0.5)) / 4.2, (y + 0.5 - cy) / (D / 2 + 0.3)
            if u * u + v * v <= 1.0:
                put(img, x, y, "s5" if (u + v) < -0.3 else "s4")
    for (x, y) in ((x0 - 4, y0 + 3), (x0 - 4, y0 + 8), (x0 - 2, y0), (x0 - 2, y0 + D - 1)):
        put(img, x, y, "s3")
    put(img, x0 - 1, int(cy), "s3"); put(img, x0 - 1, int(cy) + 1, "s3")   # dowel socket
    # the end face's right rim casts a thin shade line on the body
    for y in range(y0, y0 + D):
        if get(img, x0 + 3, y) != (0, 0, 0, 0):
            put(img, x0 + 3, y, DARKER[rowc[y - y0]])
    moss_cluster(img, 24, 36, ["..GG..", ".GggG.", "Ggkkgk"])
    return fix_staircases(outline_shadow_side(img))


def broken_wall(img, x0, x1, base, prof, courses, lip_until, chips=(), window=None):
    """prof[i] = top y of the front face at column x0+i. Columns before lip_until are the intact
    coping (top face with a lit front lip); after it the wall is fractured: the top face is the
    rough broken core (s3 with s2 clusters), no lip, and the fracture plane faces up-right."""
    for i, x in enumerate(range(x0, x1 + 1)):
        t = prof[i]
        for y in range(t, base + 1):
            put(img, x, y, "s3")
        if x < lip_until:
            for y in range(t - 3, t):
                put(img, x, y, "s5" if y == t - 1 else "s4")
        else:
            rough = "s2" if hash01(x, 3) < 0.35 else "s3"
            for y in range(t - 2, t):
                put(img, x, y, rough if y == t - 2 else "s3")
            # fracture plane: where the profile drops, the exposed step faces right: shade it
            if i > 0 and prof[i] > prof[i - 1]:
                for y in range(prof[i - 1], prof[i]):
                    put(img, x, y, "s2")
    for cy, joints in courses:
        for x in range(x0, x1 + 1):
            if get(img, x, cy) == rgba("s3"):
                put(img, x, cy, "s2")
        for jx in joints:
            for y in range(cy + 1, cy + 7):
                if get(img, jx, y) == rgba("s3"):
                    put(img, jx, y, "s2")
    if window:
        wx, wy, ww, wh = window
        for y in range(wy, wy + wh):
            for x in range(wx, wx + ww):
                u = (x + 0.5 - (wx + ww / 2)) / (ww / 2)
                if y >= wy + 4 - math.sqrt(max(0.0, 1 - u * u)) * 4:
                    put(img, x, y, "s1")
        for y in range(wy + 2, wy + wh):
            if get(img, wx + ww - 1, y) == rgba("s1"):
                put(img, wx + ww - 1, y, "s3")
        for x in range(wx, wx + ww):
            put(img, x, wy + wh, "s4")
    for y in range(0, base + 1):
        if get(img, x0, y) == rgba("s3"):
            put(img, x0, y, "s4")
        if get(img, x1, y) == rgba("s3"):
            put(img, x1, y, "s2")
    for x in range(x0, x1 + 1):
        if get(img, x, base) == rgba("s3"):
            put(img, x, base, "s2")
    for (cx, cy, shape) in chips:   # missing chunks: cut out, with a lit break face on the lower-left
        for j, row in enumerate(shape):
            for i, ch in enumerate(row):
                if ch == "x":
                    put(img, cx + i, cy + j, None)
                elif ch == "f":
                    put(img, cx + i, cy + j, "s4")
                elif ch == "d":
                    put(img, cx + i, cy + j, "s2")


def wall_low():
    """Low wall chunk ~32x24: intact coping on the left, then a fracture running down to the right
    through the courses; a chunk knocked out of the front corner; two fallen pieces."""
    img = new(CW, CH)
    x0, x1, base = 8, 39, 52
    prof = []
    for i in range(x1 - x0 + 1):
        if i < 11:
            prof.append(33)
        else:
            prof.append(min(45, 33 + int((i - 11) * 0.62 + 0.8 * math.sin(i * 1.7))))
    broken_wall(img, x0, x1, base, prof, [(46, [14, 23, 33]), (40, [11, 19, 28])], lip_until=x0 + 11,
                chips=[(x0 + 15, 39, ["xxx.", "xxf.", "xf..", "d..."])])
    grid(["hS.", "Smd"], {"h": "s5", "S": "s4", "m": "s3", "d": "s2"}, img, 41, 50)
    grid(["hSS", "Smd"], {"h": "s5", "S": "s4", "m": "s3", "d": "s2"}, img, 31, 50)
    moss_cluster(img, x0 + 1, 29, ["..GGG", ".Gggk", "Ggkk."])
    return fix_staircases(outline_shadow_side(img))


def wall_tall():
    """Tall broken wall ~46x43 with an arched window: the left pier stands to full height, a long
    angled fracture drops to the right, with one big chunk missing from its edge."""
    img = new(CW, CH)
    x0, x1, base = 1, 46, 52
    prof = []
    for i in range(x1 - x0 + 1):
        if i < 12:
            prof.append(12)
        else:
            prof.append(min(34, 12 + int((i - 12) * 0.68 + 1.2 * math.sin(i * 1.3))))
    broken_wall(img, x0, x1, base, prof, [(45, [7, 18, 29, 40]), (38, [12, 24, 35]), (31, [5, 17, 30]),
                                          (24, [10, 22]), (17, [6])], lip_until=x0 + 12,
                chips=[(x0 + 27, 26, ["xxxx", "xxxf", "xxf.", "xf..", "d..."])],
                window=(15, 27, 9, 17))
    grid(["hSS", "Smd"], {"h": "s5", "S": "s4", "m": "s3", "d": "s2"}, img, 40, 50)
    moss_cluster(img, x0, 8, [".GGGG.", "GgggGk", "gkgkk.", "k..k.."])
    moss_cluster(img, 20, 15, ["..GG", ".Ggk", "Ggk."])
    return fix_staircases(outline_shadow_side(img))


def statue():
    """The landmark: a hooded stone wanderer cradling a dull, faceted crystal at the chest.
    Hood opening (deep shadow) separated from the chest by the cowl; shoulders and both elbows
    stand out of the silhouette; the forearms meet under the crystal; the lower robe is one calm
    plane with a single fold."""
    img = new(CW, CH)
    block(img, 12, 43, 24, 3, 7)  # plinth y 43..52
    fig = [
        "..........hSm...........",
        ".........hSSSm..........",
        "........hSSSSSm.........",
        ".......hSSSSSSSm........",
        ".......hSooooSSmd.......",
        "......hSoDDDDoSmd.......",
        "......hSoDDDDDomd.......",
        "......hSSoDDDoSmd.......",
        ".....hhSSSSSSSSmmd......",
        "....hSSSSSSSSSSSSmdd....",
        "...hSSSSSSSSSSSSSSmmd...",
        "..hSSSSSSSSSxSSSSSmmdd..",
        "..hSSSmSSSSxxcSSSSmmmd..",
        ".hSSSSmSSSSxxcSSSSSmmdd.",
        ".hSSSSmSSSSxccSSSSSmmdd.",
        "hSSSSSmSSSSxccSSSSSmmddd",
        "hSSSSSSmhhhhccmmmmmmdddd",
        ".hSSSSSSmSSSSSSmmmddddd.",
        "..hSSSSSSmmmmmmmmmddd...",
        "..hSSSSSSSSSSmSSSSmdd...",
        "..hSSSSSSSSSSmSSSSmdd...",
        "..hSSSSSSSSSSmSSSSmdd...",
        "..hSSSSSSSSSmSSSSSmdd...",
        ".hSSSSSSSSSSmSSSSSmmdd..",
        ".hSSSSSSSSSmSSSSSSmmdd..",
        "hSSSSSSSSSSmSSSSSSmmddd.",
        "hhSSSSSSSSmSSSSSSSmmddd.",
    ]
    leg = {"h": "s5", "S": "s4", "m": "s3", "d": "s2", "D": "s1", "o": "s0", "c": "p0", "x": "p1"}
    grid(fig, leg, img, 12, 16)
    # moss only where it would settle: the hood's crown and the plinth's top corner
    moss_cluster(img, 20, 15, [".GG.", "GggG", ".k.k"])
    moss_cluster(img, 12, 41, ["..GG.", ".Gggk", "Ggkk."])
    return fix_staircases(outline_shadow_side(img))


# ================================================================ arch (separate sheet)
ACW, ACH = 64, 72
AAX, AAY = 32, 69
POST_DX = 17        # post centers sit at span anchor x -17 and +17
IMPOST = AAY - 36   # top of the posts' capitals / spring line of the arch
R_IN, R_OUT = 12.5, 22.0


def arch_post():
    """Arch post on the native pixel grid."""
    img = new(ACW, ACH)
    x0, w = AAX - 5, 10
    for y in range(IMPOST + 4, AAY):
        for x in range(x0, x0 + w):
            rel = x - x0
            c = "s5" if rel == 0 else "s4" if rel < 6 else "s3" if rel < 8 else "s2"
            put(img, x, y, c)
    # courses with staggered vertical joints
    for k, cy in enumerate(range(IMPOST + 10, AAY - 2, 7)):
        for x in range(x0 + 1, x0 + w):
            put(img, x, cy, "s3" if x < x0 + 7 else "s2")
        jx = x0 + (3 if k % 2 else 6)
        for y in range(cy + 1, min(AAY - 1, cy + 7)):
            put(img, jx, y, "s3")
    # capital: top face + front
    block(img, x0 - 2, IMPOST, w + 4, 2, 2)
    # plinth
    block(img, x0 - 1, AAY - 4, w + 2, 1, 3)
    moss_cluster(img, x0 - 2, AAY - 6, ["GG.", "gkG"])
    return fix_staircases(outline_shadow_side(img))


def arch_span():
    """Arch span on the native pixel grid."""
    img = new(ACW, ACH)
    cy = IMPOST + 0.5
    for y in range(0, IMPOST + 1):
        for x in range(ACW):
            d = math.hypot(x + 0.5 - AAX, y + 0.5 - cy)
            if not (R_IN <= d <= R_OUT):
                continue
            ang = math.degrees(math.atan2(cy - (y + 0.5), x + 0.5 - AAX))  # 0 right, 90 top
            c = "s4" if ang > 80 else "s3"
            if d > R_OUT - 2.2:          # extrados: the ring's top surface, lit on the left
                c = "s5" if ang > 95 else "s4" if ang > 45 else "s3"
            if d < R_IN + 2.0:           # intrados (soffit) faces down: shade
                c = "s2"
            put(img, x, y, c)
    # radial voussoir joints
    for ang in (25, 50, 72, 108, 130, 155):
        a = math.radians(ang)
        for r10 in range(int(R_IN * 10) + 20, int(R_OUT * 10) - 22, 3):
            r = r10 / 10
            x = int(math.floor(AAX + r * math.cos(a)))
            y = int(math.floor(cy - r * math.sin(a)))
            put(img, x, y, "s3" if ang > 90 else "s2")
    # keystone, slightly proud
    for y in range(int(cy - R_OUT) - 1, int(cy - R_IN) + 1):
        for x in range(AAX - 2, AAX + 2):
            put(img, x, y, "s5" if x < AAX else "s4")
    put(img, AAX + 1, int(cy - R_IN), "s2")
    # a missing chunk on the right extrados (weathering) and moss on the left haunch
    for (x, y) in ((49, 18), (50, 18), (50, 19), (51, 19), (51, 20), (52, 20), (52, 21)):
        put(img, x, y, None)
    moss_cluster(img, 13, 18, ["..GG", ".Ggk", "Ggk.", "gk..", "k..."])
    moss_cluster(img, 22, 10, ["GGg", ".gk"])
    return fix_staircases(outline_shadow_side(img))


def write_arch():
    """Write arch on the native pixel grid."""
    post, span = arch_post(), arch_span()
    man = {
        "cell": [ACW, ACH],
        "anchor": [AAX, AAY],
        "anims": {
            "arch_post": {"row": 0, "frames": 1, "fps": 1, "footprint": [12, 6]},
            "arch_span": {"row": 1, "frames": 1, "fps": 1, "layer": "canopy"},
        },
        "notes": "Ruined arch. Place arch_span at the arch's ground midpoint (x, y) and two arch_post props at "
                 "(x - 17, y) and (x + 17, y). The opening between the posts is 24 px wide; the span's spring "
                 "line is 36 px above the ground, so characters walk under it.",
    }
    sheet = save_sheet("bloomwild_arch", [[post], [span]], ACW, ACH, man)
    whole = post.copy()
    whole = new(ACW + 40, ACH, "g2")
    paste(whole, post, -POST_DX + 20, 0)
    paste(whole, post, POST_DX + 20, 0)
    paste(whole, stand_in_figure(), 20 + AAX - 4, AAY - 17)
    paste(whole, span, 20, 0)
    return sheet, whole


def build():
    """Build on the native pixel grid."""
    pieces = [
        ("pillar", pillar_intact(), [14, 6]),
        ("pillar_broken_tall", pillar_broken(26, 0), [14, 6]),
        ("pillar_broken_short", pillar_broken(38, 1), [14, 6]),
        ("drum", drum(), [26, 8]),
        ("wall_low", wall_low(), [32, 8]),
        ("wall_tall", wall_tall(), [46, 8]),
        ("statue", statue(), [20, 8]),
    ]
    return pieces


def write():
    """Write on the native pixel grid."""
    pieces = build()
    rows = [[p[1]] for p in pieces]
    anims = {}
    for i, (name, _, fp) in enumerate(pieces):
        anims[name] = {"row": i, "frames": 1, "fps": 1, "footprint": fp}
    man = {
        "cell": [CW, CH],
        "anchor": [AX, AY],
        "anims": anims,
        "notes": "White-stone ruin pieces, 1 frame each, anchor = middle of the base's front edge. "
                 "pillar ~44 px; pillar_broken_tall ~30 px; pillar_broken_short ~18 px; drum = toppled column "
                 "drum lying on its side (28x15); wall_low 32x24; wall_tall 46x43 with a window; statue = hooded "
                 "stone wanderer cradling a dull crystal (~43 px, the landmark).",
    }
    sheet = save_sheet("bloomwild_ruins", rows, CW, CH, man)
    return sheet, pieces


if __name__ == "__main__":
    sheet, pieces = write()
    scene = new(CW * len(pieces) + 16, CH, "g2")
    for i, (_, img, _) in enumerate(pieces):
        paste(scene, img, i * CW, 0)
    paste(scene, stand_in_figure(), CW * len(pieces) + 2, AY - 17)
    save(scale(scene, 4), PREVIEWS / "_wip_bw_ruins.png")
    preview(sheet, PREVIEWS / "bloomwild_ruins_6x.png", k=6, bg="g2", gridcell=(CW, CH))
    _, whole = write_arch()
    save(scale(whole, 5), PREVIEWS / "bloomwild_arch_5x.png")
