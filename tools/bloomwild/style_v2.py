"""Build Bloomwild's Long Quiet sheets with compatible cells, anchors and animation contracts.

Run: uv run --no-project --with pillow --with numpy python3 tools/bloomwild/style_v2.py
Legacy generators supply secondary tiles and small ceramic props; all prominent forms are redrawn here.
"""
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

import gate
import lantern
import pix
import props
import ruins
import tiles

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'out/samples/bloomwild_v2style'
SPRITES = ROOT / 'assets/sprites'


def color(name):
    """Resolve an authored ramp name to an opaque native pixel."""
    return pix.rgba(name)


def canvas(w, h, fill=None):
    """Create an unfiltered RGBA canvas and its integer drawing context."""
    im = Image.new('RGBA', (w, h), color(fill) if fill else (0, 0, 0, 0))
    return im, ImageDraw.Draw(im)


def poly(d, points, c):
    """Paint a connected flat polygon on the native grid."""
    d.polygon(points, fill=color(c))


def line(d, points, c, width=1):
    """Paint an integer branch, joint or channel without smoothing."""
    d.line(points, fill=color(c), width=width)


def box(d, bounds, c):
    """Paint one rectangular stone plane or recess."""
    d.rectangle(bounds, fill=color(c))


def cell_write(sheet, manifest, name, im, frame=0):
    """Replace a named animation cell without changing its layout contract."""
    cw, ch = manifest['cell']
    row = manifest['anims'][name]['row']
    sheet.paste(im, (frame * cw, row * ch))


LEAF = ['b0', 'b1', 'b2', 'b3', 'b4']
KEY = np.array([-0.55, -0.7, 0.45]) / np.linalg.norm([-0.55, -0.7, 0.45])
SHEAR = 0.3

# Crowns are built at three scales: the whole asymmetric crown, branch clusters (cx, cy, rx, ry[, bias])
# and staggered leaf bundles inside each cluster. Every tree streams downwind (east), toward the ruins.
TREES = {
    # a: entry tree; the crown streams east toward the ruin from a high windward shoulder.
    'a': dict(seed=1, sway_above=40, base=39,
              clusters=[(33, 31, 19, 6), (57, 37, 14, 5), (19, 45, 13, 5), (49, 51, 15, 6), (35, 43, 8, 4, -0.08)],
              clefts=[(42, 45, 2.5, 2), (29, 40, 2, 1.5)], gaps=[(41, 48, 2, 1.5)],
              twigs=[[(40, 49), (42, 44)], [(28, 41), (30, 39)]],
              spine=[(39, 89), (40, 78), (38, 68), (40, 58), (38, 46), (35, 34)], width=[6, 5, 4, 3, 2],
              limbs=[([(38, 68), (29, 57), (20, 47)], 2), ([(40, 60), (47, 55), (50, 51)], 2),
                     ([(40, 58), (49, 47), (57, 39)], 2), ([(39, 52), (35, 45)], 1)],
              roots=[([(31, 90), (35, 89)], 'e0', 1), ([(45, 89), (50, 90)], 'e0', 1)]),
    # b: tall; a narrow windward top over a broad, low downwind mass.
    'b': dict(seed=2, sway_above=40, base=39,
              clusters=[(31, 29, 12, 5), (47, 37, 16, 5), (25, 47, 12, 5), (49, 55, 14, 5), (37, 45, 7, 3, -0.1)],
              clefts=[(38, 51, 2, 2)], gaps=[(36, 52, 1.5, 1.2)],
              twigs=[[(37, 53), (39, 49)]],
              spine=[(39, 89), (38, 79), (40, 69), (37, 57), (35, 44), (32, 31)], width=[5, 4, 4, 3, 2],
              limbs=[([(40, 69), (31, 58), (25, 48)], 2), ([(39, 64), (46, 59), (49, 55)], 2),
                     ([(37, 55), (44, 46), (47, 38)], 2)],
              roots=[([(32, 90), (35, 89)], 'e0', 1), ([(44, 89), (49, 90)], 'e0', 1)]),
    # c: broad; two shoulders split by a deep cleft over one low front shelf.
    'c': dict(seed=3, sway_above=40, base=41,
              clusters=[(23, 36, 15, 6), (54, 34, 16, 6), (41, 53, 19, 5), (39, 43, 7, 4, -0.12)],
              clefts=[(38, 38, 3, 4), (31, 48, 2, 1.5)], gaps=[(39, 35, 2, 3)],
              twigs=[[(39, 45), (38, 38), (36, 33)], [(30, 49), (32, 46)]],
              spine=[(41, 89), (40, 79), (42, 69), (40, 59)], width=[6, 5, 4],
              limbs=[([(40, 60), (32, 49), (23, 39)], 3), ([(41, 62), (48, 48), (54, 37)], 3),
                     ([(40, 60), (39, 50), (37, 36)], 1)],
              roots=[([(33, 90), (36, 89)], 'e0', 1), ([(47, 89), (53, 90)], 'e0', 1)]),
    # d: leaning; the trunk bows downwind and the crown streams off its tip in two long shelves.
    'd': dict(seed=4, sway_above=42, base=42,
              clusters=[(40, 39, 19, 5), (50, 53, 15, 5), (24, 49, 9, 4, -0.05), (63, 44, 8, 3)],
              clefts=[(44, 47, 2, 1.5)], gaps=[(36, 48, 2, 1.2)],
              twigs=[[(44, 49), (46, 45)]],
              spine=[(42, 89), (40, 80), (41, 70), (45, 60), (46, 50), (42, 41)], width=[5, 4, 4, 3, 2],
              limbs=[([(41, 70), (32, 60), (25, 51)], 2), ([(45, 60), (50, 56)], 2), ([(46, 50), (56, 46), (63, 44)], 1)],
              roots=[([(34, 90), (37, 89)], 'e0', 1), ([(47, 89), (52, 90)], 'e0', 1)]),
    # e: young; two small combed shelves on a thin forked stem.
    'e': dict(seed=5, sway_above=66, base=41,
              clusters=[(40, 63, 11, 4), (47, 71, 10, 4)],
              clefts=[], gaps=[], twigs=[],
              spine=[(41, 89), (40, 81), (41, 72), (39, 64)], width=[3, 3, 2],
              limbs=[([(41, 77), (46, 72)], 1)],
              roots=[([(36, 90), (38, 89)], 'e0', 1), ([(44, 89), (46, 90)], 'e0', 1)]),
}


def lit(u, v):
    """Lambert term of a unit ellipsoid point (u, v) under the upper-left key light."""
    d = np.hypot(u, v)
    if d > 1:
        u, v = u / d, v / d
    w = np.sqrt(max(0.0, 1 - u * u - v * v))
    return u * KEY[0] + v * KEY[1] + w * KEY[2]


def leaf_bundles(cx, cy, rx, ry, seed):
    """Tile one branch cluster with staggered, wind-skewed leaf bundles, far rows first."""
    out = []
    rows = max(2, int(round(2 * ry / 3.4)))
    for r in range(rows):
        by = cy - ry + 1.5 + r * (2 * ry - 2) / max(1, rows - 1)
        t = (by - cy) / ry
        half = rx * np.sqrt(max(0.0, 1 - t * t)) + 1
        n = max(1, int(round(2 * half / 7.5)))
        for k in range(n):
            j = pix.hash01(seed, r, k)
            bx = cx - half + (k + 0.5 + 0.35 * (r % 2) - 0.17) * (2 * half / n) + (j - 0.5) * 1.6 - SHEAR * (by - cy)
            out.append((bx, by, 4.4 + 1.8 * pix.hash01(seed, r, k, 1), 1.9 + 0.5 * pix.hash01(seed, r, k, 2),
                        k == n - 1))
    return out


def crown_levels(spec, shift=0):
    """Ramp level and cluster rank per pixel from the cluster, bundle and whole-crown light."""
    cl = spec['clusters']
    lvl = -np.ones((96, 80), int)
    own = -np.ones((96, 80), int)
    gx = sum(c[0] for c in cl) / len(cl)
    gy = sum(c[1] for c in cl) / len(cl)
    grx = max(abs(c[0] - gx) + c[2] for c in cl)
    gry = max(abs(c[1] - gy) + c[3] for c in cl)
    order = sorted(range(len(cl)), key=lambda i: cl[i][1])
    for rank, ci in enumerate(order):
        cx, cy, rx, ry = cl[ci][:4]
        bias = cl[ci][4] if len(cl[ci]) > 4 else 0.0
        if cy < spec['sway_above']:
            cx += shift
        bs = leaf_bundles(cx, cy, rx, ry, spec['seed'] * 13 + ci)
        for y in range(96):
            for x in range(80):
                hit = None
                for (bx, by, brx, bry, last) in bs:
                    # The downwind bundle of each row trails a longer combed tip.
                    comb = last and x + 0.5 >= bx
                    u = (x + 0.5 - bx + 0.45 * (y + 0.5 - by)) / (brx * (1.9 if comb else 1.0))
                    v = (y + 0.5 - by) / (bry * (1 - 0.5 * min(1, abs(u))) if comb else bry)
                    if u * u + v * v <= 1.0:
                        hit = (u * 0.6 if comb else u, v)  # nearer rows are later and win
                cu = (x + 0.5 - cx + SHEAR * (y + 0.5 - cy)) / rx
                cv = (y + 0.5 - cy) / ry
                if hit is None:
                    if cu * cu + cv * cv > 0.62:
                        continue
                    hit = (0.0, 0.0)  # a solid core: no daylight pinholes inside a cluster
                gu, gv = (x - gx) / grx, (y - gy) / gry
                lum = 0.56 * lit(cu, cv) + 0.36 * lit(*hit) + 0.34 * (-1.0 * gv - 0.45 * gu) + bias
                lvl[y, x] = 4 if lum > 1.16 else 3 if lum > 0.74 else 2 if lum > 0.36 else 1
                own[y, x] = rank
    return lvl, own


def ellipse_px(cx, cy, rx, ry):
    """Native pixels whose centers fall inside an axis-aligned ellipse."""
    return [(x, y) for y in range(96) for x in range(80)
            if ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1]


def crown(spec, shift=0):
    """Finish one crown frame: two-step undersides, deep clefts, daylight gaps and limb glimpses."""
    lvl, own = crown_levels(spec, shift)
    out = lvl.copy()
    for y in range(96):
        for x in range(80):
            if lvl[y, x] < 0:
                continue
            below = lvl[y + 1, x] if y + 1 < 96 else -1
            below2 = lvl[y + 2, x] if y + 2 < 96 else -1
            if below < 0:
                out[y, x] = 0
            elif below2 < 0 and own[y + 1, x] == own[y, x]:
                out[y, x] = min(out[y, x], 1)
            elif own[y + 1, x] > own[y, x] and lvl[y, x] <= 2:
                out[y, x] = 0  # a shaded far cluster just above a nearer one
    for e in spec['clefts']:
        for x, y in ellipse_px(*e):
            if out[y, x] >= 0:
                out[y, x] = 0
    for e in spec['gaps']:
        for x, y in ellipse_px(*e):
            out[y, x] = -1
    src = out.copy()
    for y in range(1, 95):
        for x in range(1, 79):
            v = src[y, x]
            nb = [src[y - 1, x], src[y + 1, x], src[y, x - 1], src[y, x + 1]]
            if v >= 0 and v not in nb and min(nb) >= 0:
                out[y, x] = max(set(nb), key=nb.count)  # no isolated confetti pixels
            elif v == 4 and nb.count(4) < 2:
                out[y, x] = 3  # chalky highlights only in small clumps on lit caps
    im, d = canvas(80, 96)
    px = im.load()
    for y in range(96):
        for x in range(80):
            if out[y, x] >= 0:
                px[x, y] = color(LEAF[out[y, x]])
    for pts in spec['twigs']:
        line(d, pts, 's0')
    return im


def trunk(spec):
    """Draw a rooted, tapering trunk with lit-left bark and limbs that end inside their clusters."""
    im, d = canvas(80, 96)
    bx = spec['base']
    for pts, c, w in spec['roots']:
        line(d, pts, c, w)
    for pts, w in spec['limbs']:
        line(d, pts, 's0', w)
        if w >= 2:
            line(d, [(x - 1, y) for x, y in pts[:-1]] + [pts[-1]], 's1')
    sp, ws = spec['spine'], spec['width']
    for i in range(len(sp) - 1):
        line(d, [sp[i], sp[i + 1]], 's0', ws[i])
    for i in range(len(sp) - 1):
        off = (ws[i] - 1) // 2
        line(d, [(sp[i][0] - off, sp[i][1]), (sp[i + 1][0] - off, sp[i + 1][1])], 's2')
        if ws[i] >= 4:
            line(d, [(sp[i][0] - off + 1, sp[i][1]), (sp[i + 1][0] - off + 1, sp[i + 1][1])], 's1')
    # Splayed roots grip the ground; the contact shade sits downwind.
    h = ws[0] // 2
    line(d, [(bx - h, 85), (bx - h - 2, 89), (bx - h - 3, 90)], 's0', 2)
    line(d, [(bx + h, 84), (bx + h + 3, 88), (bx + h + 7, 90)], 's0', 2)
    line(d, [(bx - h - 1, 86), (bx - h - 3, 89)], 's2')
    line(d, [(bx + h + 1, 91), (bx + h + 7, 91)], 'e0')
    return im


def tree_sheet():
    """Build wind-combed crowns from branch clusters and leaf bundles over rooted, connected limbs."""
    man = json.loads((SPRITES / 'bloomwild_trees.json').read_text())
    sh, _ = canvas(160, 960)
    for name, spec in TREES.items():
        cell_write(sh, man, f'{name}_trunk', trunk(spec))
        for frame in range(2):
            cell_write(sh, man, f'{name}_canopy', crown(spec, frame), frame)
    man['notes'] = ('Style v2: wind-combed dried-rose crowns built from branch clusters and leaf bundles, '
                    'streaming east, with dark clefts, limb glimpses and rooted mineral-gray trunks. '
                    'Upper clusters move one pixel downwind; trunk and canopy share the original anchors and footprints.')
    save('bloomwild_trees', sh, man)


def fin(d, x, base, height, width=14, broken=False):
    """Draw a tall slotted distributor fin with an ivory face and blue-lilac side."""
    top = base-height
    poly(d, [(x-width//2,top+3),(x-width//2+3,top),(x+width//2-2,top),
             (x+width//2,top+4),(x+width//2,base),(x-width//2,base)], 's3')
    box(d,(x+width//2-3,top+3,x+width//2,base),'s1')
    line(d,[(x-width//2,base-1),(x-width//2,top+3),(x-width//2+3,top),(x+width//2-2,top)],'s5')
    box(d,(x-2,top+7,x+1,base-7),'s0')
    line(d,[(x+2,top+7),(x+2,base-7)],'s2')
    box(d,(x-width//2-3,base-5,x+width//2+2,base-3),'s4')
    box(d,(x-width//2-3,base-2,x+width//2+2,base),'s2')
    if broken:
        poly(d,[(x-width//2,top),(x+2,top+4),(x+3,top),(x+width//2,top+3),
                (x+width//2,top+7),(x-1,top+6),(x-width//2,top+3)],'s2')
        line(d,[(x-width//2,top+1),(x+2,top+5),(x+4,top+2)],'s4')


def architecture():
    """Redraw colonnades and entry as remnants of a slotted garden water distributor."""
    man = json.loads((SPRITES/'bloomwild_ruins.json').read_text())
    sh = Image.open(SPRITES/'bloomwild_ruins.png').convert('RGBA')
    for name,height in [('pillar',47),('pillar_broken_tall',29),('pillar_broken_short',18)]:
        im,d = canvas(48,56)
        fin(d,24,52,height,14,name!='pillar')
        cell_write(sh,man,name,im)
    for name,height in [('wall_low',23),('wall_tall',41)]:
        im,d = canvas(48,56)
        y=52-height
        poly(d,[(2,52),(2,y+2),(5,y),(17,y),(19,y+5),(25,y+7),
                (25,y+12),(33,y+14),(36,y+height-6),(46,48),(46,52)],'s3')
        poly(d,[(35,y+14),(39,43),(46,48),(46,52),(34,52)],'s1')
        line(d,[(2,51),(2,y+2),(5,y),(17,y)],'s5')
        for x in [8,19]:
            box(d,(x,y+8,x+4,45),'s0')
            line(d,[(x+5,y+8),(x+5,45)],'s2')
        box(d,(3,47,33,48),'s4')
        line(d,[(3,49),(32,49),(36,51)],'s1')
        line(d,[(27,y+10),(23,y+15),(28,y+20)],'s2')
        cell_write(sh,man,name,im)
    # The old held prism is replaced by a broken stone distributor fragment, not a second accent.
    im = sh.crop((0,6*56,48,7*56)); d=ImageDraw.Draw(im)
    box(d,(21,27,28,35),'s3')
    poly(d,[(24,27),(28,31),(26,35),(21,32)],'s1')
    poly(d,[(24,27),(25,32),(21,32)],'s4')
    line(d,[(17,29),(19,34),(22,35)],'s5',2)
    line(d,[(31,30),(29,35),(26,36)],'s4',2)
    cell_write(sh,man,'statue',im)
    man['notes']='Old ivory slotted waterworks. Narrow recessed channels survive across standing fins and broken distributors; statue cradles a dull broken stone fragment. Original footprints and anchors retained.'
    save('bloomwild_ruins',sh,man)
    man=json.loads((SPRITES/'bloomwild_arch.json').read_text())
    sh,_=canvas(64,144)
    im,d=canvas(64,72); fin(d,32,68,48,10)
    cell_write(sh,man,'arch_post',im)
    im,d=canvas(64,72)
    poly(d,[(9,23),(12,20),(51,20),(54,23),(54,30),(9,30)],'s3')
    line(d,[(9,23),(12,20),(51,20)],'s5')
    box(d,(10,28,53,31),'s1')
    for x in [15,23,38,46]: box(d,(x,23,x+3,26),'s0')
    poly(d,[(30,19),(34,19),(34,26),(31,28),(29,25)],'s4')
    cell_write(sh,man,'arch_span',im)
    man['notes']='Broken slotted water-channel lintel on paired narrow fins. Original opening, cell, layer and footprints retained.'
    save('bloomwild_arch',sh,man)


BUSHES = {
    # (x, base_y, height) leaf fans, far to near; bush_b is two fan groups split by a shaded cleft.
    'bush_a': [(6, 28, 9), (12, 27, 14), (19, 28, 11), (24, 28, 7)],
    'bush_b': [(4, 28, 7), (9, 27, 12), (14, 28, 9), (20, 27, 13), (26, 28, 8)],
    'bush_bloom': [(7, 28, 8), (12, 27, 13), (19, 28, 10), (24, 28, 7)],
}


def leaf_fan(d, x, y, h):
    """Draw one sage leaf fan: a shaded back blade, a lit front blade and its midrib."""
    poly(d, [(x-4,y-h),(x,y-h+3),(x+3,y),(x-1,y-1)], 'g1')
    poly(d, [(x,y-h+2),(x+6,y-h-2),(x+3,y-2),(x,y)], 'g2')
    line(d, [(x,y-3),(x+4,y-h)], 'g3')


def blossom(d, x, y):
    """Draw one small dried-rose blossom clump, lit upper-left and shaded lower-right."""
    box(d, (x, y, x+2, y+1), 'b2')
    box(d, (x, y, x, y), 'b3')
    box(d, (x+2, y+1, x+2, y+1), 'b1')


def foliage_props():
    """Root sage leaf-fan clumps in broken contact shade and give the flowering bush a blossom cluster."""
    man=json.loads((SPRITES/'bloomwild_props.json').read_text())
    sh=Image.open(SPRITES/'bloomwild_props.png').convert('RGBA')
    for name, fans in BUSHES.items():
        im,d=canvas(32,32)
        # Contact shade sits under each fan base, longer downwind, with daylight gaps between bases.
        for x,y,h in fans:
            line(d,[(x-2,29),(x+3,29)],'g0')
        x,y,h=fans[-1]
        line(d,[(x+3,29),(x+6,29)],'g0')
        line(d,[(fans[-2][0],30),(x+4,30)],'g0')
        if name=='bush_bloom':
            # A forked stem rises from the clump's center; three blossoms make one small cluster.
            line(d,[(15,23),(17,15),(18,12)],'e1')
            line(d,[(17,16),(14,13)],'e1')
            line(d,[(17,15),(21,13)],'e1')
        for x,y,h in fans:
            leaf_fan(d,x,y,h)
        if name=='bush_b':
            poly(d,[(16,29),(18,29),(17,26)],'g0')  # the shaded cleft between the two fan groups
        if name=='bush_bloom':
            blossom(d,13,12); blossom(d,17,11); blossom(d,20,12)
        cell_write(sh,man,name,im)
    man['notes']='Sage leaf fans rooted in broken contact shade, with sparse dried-rose blossom clusters; weathered gray ceramic urns and mineral boulders. Original breakability and footprints retained.'
    save('bloomwild_props',sh,man)


def floor_block(w,h,seed):
    """Design a quiet worn slab block with staggered interrupted seams and a chipped joint."""
    im,d=canvas(w,h,'s3')
    # All bodies share a value; joints are sparse and only one structural shade apart.
    y=11+(seed%3)*3
    while y<h:
        mid=w//2+(seed%7)-3
        line(d,[(2,y),(mid-4,y),(mid,y-2),(w-2,y-2)],'s2')
        if y+14<h: line(d,[(mid,y),(mid,y+12)],'s2')
        line(d,[(3,y+1),(11,y+1)],'s4')
        y+=20+(seed%3)
    if min(w,h)>16 and seed%3==0:
        mx=5+(seed*7)%max(1,w-16)
        my=0 if seed%2 else h-7
        poly(d,[(mx,my),(mx+6,my),(mx+4,my+4),(mx-1,my+6),(mx-3,my+4)],'s2')
        line(d,[(mx,my+1),(mx+3,my+1),(mx,my+4)],'g1')
    if seed%4==0:
        line(d,[(3,h-2),(8,h-5),(8,h-9),(11,h-12)],'s2')
    return im


def floors():
    """Replace shipped blocks and fillers while preserving every 16px tile coordinate."""
    man=json.loads((SPRITES/'bloomwild_tiles.json').read_text())
    sh=Image.open(SPRITES/'bloomwild_tiles.png').convert('RGBA')
    for i,(name,b) in enumerate(man['blocks'].items()):
        w,h=[v*16 for v in b['size']]
        im=floor_block(w,h,i+11)
        sh.paste(im,tuple(v*16 for v in b['at']))
    for i in range(12):
        x,y=man['tiles'][f'ruin_{i}']
        im=floor_block(16,16,i+31)
        sh.paste(im,(x*16,y*16))
    man['notes']='Style v2 dusty sage terrain and worn old-ivory paving. Calm matching slab bodies, interrupted seams, selective fractures and joint moss. All original tile/block coordinates retained; path base uses earth[3].'
    man['groups']['grass']='dust-sage grass with quiet plain variant and sparse rooted details'
    man['groups']['teal']='cool sage damp-ground variants'
    save('bloomwild_tiles',sh,man)


def landmark():
    """Build one monumental fin that reuses the entrance post's exact collision footprint."""
    im,d=canvas(64,128)
    fin(d,32,124,109,10)
    # The surviving upper distributor is a broad stone mass above the original narrow foot.
    poly(d,[(18,18),(23,12),(42,12),(47,18),(47,84),(40,91),(25,91),(18,84)],'s3')
    poly(d,[(39,14),(47,18),(47,84),(40,91),(39,83)],'s1')
    line(d,[(18,82),(18,18),(23,12),(41,12)],'s5')
    box(d,(28,27,34,78),'s0')
    line(d,[(35,28),(35,77)],'s2')
    # A displaced upper channel and one substantial break identify its abandoned purpose.
    poly(d,[(27,15),(30,12),(35,12),(38,16),(38,25),(34,28),(27,25)],'s3')
    line(d,[(27,24),(27,15),(30,12),(35,12)],'s5')
    box(d,(31,17,33,25),'s0')
    line(d,[(28,71),(32,73),(36,70)],'s1')
    line(d,[(28,72),(32,74)],'s4')
    man={'cell':[64,128],'anchor':[32,125],
         'anims':{'fin':{'row':0,'frames':1,'fps':1,'footprint':[12,6]}},
         'notes':'109px surviving distributor fin at the entrance. Replaces one arch_post at the same world point with the same 12x6 footprint; the low connecting channel uses the original arch_span.'}
    save('bloomwild_waterworks',im,man)


def gate_body():
    """Frame the shared Prism seal with clipped ivory fins and a recessed channel lintel."""
    im,d=canvas(48,64)
    poly(d,[(3,10),(7,6),(40,6),(44,10),(44,24),(34,24),(34,17),(29,14),
            (18,14),(13,17),(13,24),(3,24)],'s3')
    line(d,[(3,23),(3,10),(7,6),(40,6)],'s5')
    box(d,(40,10,44,53),'s1')
    for a in [5,34]:
        box(d,(a,23,a+8,53),'s3')
        line(d,[(a,24),(a,53)],'s5')
        box(d,(a+6,25,a+8,53),'s1')
        box(d,(a+2,26,a+4,47),'s0')
        box(d,(a-1,50,a+9,53),'s2')
        line(d,[(a-1,50),(a+8,50)],'s4')
    for y in range(14,25):
        for x in range(13,35):
            if not gate.in_opening(x,y):
                box(d,(x,y,x,y),'s1' if gate.in_opening(x,y+1) else 's3')
    # Bridge from the faceted socket into the two channels, rather than a carved arch ring.
    box(d,(7,10,18,11),'s0')
    box(d,(29,10,39,11),'s0')
    box(d,(19,5,28,18),'s4')
    line(d,[(19,17),(19,5),(28,5)],'s5')
    box(d,(3,54,44,57),'s4')
    box(d,(3,58,44,60),'s2')
    box(d,(3,61,44,61),'s0')
    return np.array(im)


def save(name,im,man):
    """Write the finished sheet and its compatible metadata deterministically."""
    im.save(SPRITES/f'{name}.png')
    (SPRITES/f'{name}.json').write_text(json.dumps(man,indent=1)+'\n')


def build():
    """Regenerate the complete owned family with v2 ramps and authored structural redraws."""
    OUT.mkdir(parents=True,exist_ok=True)
    for mod in [pix,gate,lantern,props,ruins,tiles]:
        mod.PREVIEWS=OUT
    tiles.write_sheet()
    props.write()
    ruins.write()
    ruins.write_arch()
    lantern.write()
    gate.stone_body=gate_body
    gate.write()
    tree_sheet()
    architecture()
    foliage_props()
    floors()
    landmark()
    # Gate activation is the inserted Prism's iris light, never unrelated cyan ornament.
    man=json.loads((SPRITES/'gate.json').read_text())
    man['anims']['open']['light']['color']=pix.HEX['p3']
    man['notes']+=' Style v2: old ivory frame; iris light belongs to the inserted Prism. No cyan architecture.'
    (SPRITES/'gate.json').write_text(json.dumps(man,indent=1)+'\n')
    man=json.loads((SPRITES/'bloomwild_lantern.json').read_text())
    man['anims']['lit']['light']['color']=pix.HEX['au3']
    (SPRITES/'bloomwild_lantern.json').write_text(json.dumps(man,indent=1)+'\n')


if __name__=='__main__':
    build()
