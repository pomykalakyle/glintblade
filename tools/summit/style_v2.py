"""Rebuild Summit's hand-drawn v2 atlas and props with the regions.warden ramps."""

import json
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SPRITES = ROOT / 'assets/sprites'
REGION = json.loads((ROOT / 'assets/concept/palettes_v2.json').read_text())['regions']['warden']
F = REGION['ramps']['floor']
S = REGION['ramps']['warden_stone']
M = REGION['ramps']['moss']
I = REGION['ramps']['prism']
G = REGION['ramps']['gold']
INK = REGION['outline']
CLEAR = (0, 0, 0, 0)


def canvas(size):
    """Create one native-grid transparent canvas."""
    return Image.new('RGBA', size, CLEAR)


def save(name, image, manifest):
    """Write a deterministic sheet and its engine contract."""
    image.save(SPRITES / f'{name}.png')
    (SPRITES / f'{name}.json').write_text(json.dumps(manifest, indent=1) + '\n')


def flags(size, variant):
    """Draw broad offset slabs with selected chipped joints instead of repeated pits."""
    im = Image.new('RGBA', size, F[1])
    d = ImageDraw.Draw(im)
    w, h = size
    step = 16 if h <= 16 else 32
    for y in range(0, h, step):
        d.line((0, y, w - 1, y), fill=S[0])
        x = (variant * 13 + y * 3) % 42
        d.line((x, y, x, min(h - 1, y + step - 1)), fill=S[0])
        if variant % 3 == 0:
            d.line((x + 3, y + 1, min(w - 1, x + 17), y + 1), fill=F[2])
        if variant % 4 == 1:
            d.polygon([(x, y), (x + 4, y), (x + 4, y + 2), (x + 2, y + 3), (x, y + 3)], fill=S[0])
    return im


def seal(lit):
    """Carve a broken water distributor around an empty socket, with a dormant state."""
    im = Image.new('RGBA', (96, 96), F[1])
    d = ImageDraw.Draw(im)
    # An intelligible machine remnant: square collector, four interrupted water channels.
    for box in [(21, 29, 73, 65), (24, 32, 70, 62)]:
        d.rectangle(box, outline=S[0])
    d.line((22, 66, 71, 66), fill=F[2])
    for pts in [[(0, 44), (21, 44)], [(73, 49), (95, 49)], [(44, 0), (44, 29)], [(51, 65), (51, 95)]]:
        d.line(pts, fill=S[0], width=2)
    d.polygon([(47, 38), (56, 46), (49, 55), (39, 48)], fill=F[0])
    d.line([(40, 49), (49, 56), (57, 47)], fill=F[2])
    # Displaced joint damage is grouped at one corner; no compass rose or pseudo-runes.
    d.line([(69, 31), (65, 37), (65, 41), (61, 45)], fill=S[0])
    d.line([(70, 31), (67, 37), (67, 40)], fill=F[2])
    d.rectangle((21, 54, 24, 59), fill=F[1])
    d.rectangle((57, 29, 63, 32), fill=F[1])
    if lit:
        d.line([(44, 9), (44, 29), (40, 29)], fill=I[2])
        d.line([(73, 49), (87, 49)], fill=I[3])
    return im


def tiles():
    """Reauthor all named tiles in place, retaining every atlas coordinate and block size."""
    m = json.loads((SPRITES / 'summit_tiles.json').read_text())
    old_size = Image.open(SPRITES / 'summit_tiles.png').size
    atlas = canvas(old_size)
    block_cells = set()
    for name, block in m['blocks'].items():
        x, y = [v * 16 for v in block['at']]
        w, h = [v * 16 for v in block['size']]
        v = ord(name.split('_')[-1][0]) % 8
        if name.startswith('course'):
            im = flags((w, h), v)
        elif name.startswith('seal'):
            im = seal(name.endswith('lit'))
        elif name.startswith('ground'):
            im = Image.new('RGBA', (w, h), S[0])
            d = ImageDraw.Draw(im)
            d.polygon([(0, 0), (w, 0), (w - 8, 5), (12, 7), (0, 5)], fill=F[1])
        elif name.startswith('cliff_bot'):
            im = canvas((w, h))
            d = ImageDraw.Draw(im)
            # Contiguous unequal masses across variants, rather than one point per tile.
            edge = [24, 26, 30, 28, 17, 19, 23, 15][v]
            d.polygon([(0, 0), (w - 1, 0), (w - 1, edge - 3), (0, edge)], fill=F[0])
            if v in (1, 2, 5):
                d.polygon([(0, 0), (9, 0), (8, edge - 7), (0, edge - 4)], fill=S[0])
        else:
            im = canvas((w, h))
            d = ImageDraw.Draw(im)
            col = M[0] if name.startswith('moss') else F[2]
            d.polygon([(2, 11), (9, 8), (17, 9), (20, 11), (w - 3, 10), (w - 1, 14), (3, 14)], fill=col)
        atlas.paste(im, (x, y))
        block_cells.update((x // 16 + xx, y // 16 + yy) for yy in range(h // 16) for xx in range(w // 16))
    for name, (tx, ty) in m['tiles'].items():
        if (tx, ty) in block_cells:
            continue
        im = canvas((16, 16))
        d = ImageDraw.Draw(im)
        if name.startswith('rune'):
            im = Image.new('RGBA', (16, 16), F[1])
            d = ImageDraw.Draw(im)
            d.line([(4, 3), (4, 11), (11, 11)], fill=S[0])
            d.line((5, 12, 11, 12), fill=F[2])
            if name.endswith('lit'):
                d.line((4, 4, 4, 9), fill=I[2])
        elif name.startswith('ground'):
            d.rectangle((0, 0, 15, 15), fill=S[0])
        elif name.startswith('cliff') or name.startswith('wall'):
            d.rectangle((0, 0, 15, 15), fill=F[0])
            v = ord(name.split('_')[2][0]) % 8
            if v in (0, 1, 4, 5):
                d.rectangle((0, 0, 9, 15), fill=S[0])
            if 'top' in name:
                d.rectangle((0, 0, 15, 2), fill=F[1])
                d.line((1, 0, 12, 0), fill=F[2])
        elif name.startswith('fg_'):
            code = name.split('_')[1]
            tl, tr, bl, br = [int(c) for c in code]
            for y in range(16):
                for x in range(16):
                    u, v = x / 15, y / 15
                    a = (tl * (1 - u) + tr * u) * (1 - v) + (bl * (1 - u) + br * u) * v
                    if a > 0.5:
                        d.point((x, y), fill=S[0])
        elif name.startswith('rim'):
            rock = name.endswith('rock')
            d.polygon([(0, 3), (5, 2), (10, 4), (15, 4), (15, 15), (0, 15)], fill=S[0] if rock else F[1])
            d.line([(0, 3), (5, 2), (10, 4), (15, 4)], fill=F[2])
        elif name.startswith('moss') or name.startswith('frost'):
            col = M[0] if name.startswith('moss') else F[2]
            d.polygon([(2, 12), (5, 9), (10, 9), (14, 12), (12, 14), (3, 14)], fill=col)
        atlas.paste(im, (tx * 16, ty * 16))
    m['emissive'] = [I[3]]
    m['notes'] = 'The Long Quiet v2. Authored by tools/summit/style_v2.py; regions.warden only. Dormant distributor carving is non-emissive; iris pixels appear only in explicit lit-event tiles. Coordinates, tile names and block dimensions retained.'
    save('summit_tiles', atlas, m)


# Front-face columns stop this many pixels before a fin's east edge; the rest is its shadowed side.
SIDE = 6


def seat(d, cx, base, half):
    """Seat a fin on its plinth, with a short down-right ground shadow kept inside the cell."""
    d.polygon([(cx - half + 1, base - 6), (cx + half - 3, base - 6), (cx + half, base - 3), (cx - half, base - 3)], fill=F[3])
    d.rectangle((cx - half, base - 2, cx + half, base), fill=F[1])


def ground_shadow(d, cx, base, half):
    """Lay the fin's contact shadow before the stone so the body overlaps it."""
    d.polygon([(cx - 14, base - 4), (cx + 13, base - 4), (cx + half + 2, base + 3), (cx - 5, base + 3)], fill=INK)


def body(d, left, right, base, tops, cap=3):
    """Fill a fin column by column: a lit top plane over the front face, then the shadowed east side.

    `tops` holds one top row per column from `left` to `right`; None leaves that column open.
    """
    for x in range(left, right + 1):
        y = tops[x - left]
        if y is None:
            continue
        if x > right - SIDE:
            d.line((x, y, x, base - 6), fill=F[0])
        else:
            d.line((x, y, x, y + cap - 1), fill=F[3])
            d.line((x, y + cap, x, base - 6), fill=F[2])


def side_tops(front_top, n=SIDE, run=1):
    """Slope the east face's top edge down and away from the last front column."""
    return [front_top + 1 + j * run for j in range(n)]


def slot(d, cx, top, bottom):
    """Cut the fin's water channel as real negative space with a shaded east wall and a lit sill."""
    d.rectangle((cx - 4, top, cx + 2, bottom), fill=CLEAR)
    d.line((cx + 3, top, cx + 3, bottom + 1), fill=F[1])
    d.line((cx - 4, bottom + 1, cx + 2, bottom + 1), fill=F[3])


def notch(d, x0, x1, y0, y1, floor, wall_x, wall):
    """Remove one whole block: open air, a two-row top plane of the block below and a recessed wall."""
    d.rectangle((x0, y0, x1, y1), fill=CLEAR)
    d.rectangle((x0, y1 - 1, x1, y1), fill=floor)
    d.line((wall_x, y0, wall_x, y1 - 2), fill=wall)
    d.line((x0, y0, x1, y0), fill=F[0])


def fin_sheared(size, anchor, height):
    """A tall fin whose crown sheared off on a long down-east diagonal, with its west corner block missing."""
    im = canvas(size)
    d = ImageDraw.Draw(im)
    cx, base = anchor
    top = base - height
    left, right = cx - 16, cx + 15
    ground_shadow(d, cx, base, 19)
    # The shear falls one row every two columns and drops three rows once at a fracture step.
    front = [top + i // 2 + (3 if i >= 17 else 0) for i in range(right - left + 1 - SIDE)]
    body(d, left, right, base, front + side_tops(front[-1], run=2))
    d.line((left, top + 3, left, base - 12), fill=F[3])
    d.line((left + 17, top + 11, left + 17, top + 13), fill=F[1])
    slot(d, cx, top + 24, base - 31)
    notch(d, left, left + 4, base - 76, base - 61, F[3], left + 5, F[1])
    d.line((left + 1, base - 24, right - 6, base - 24), fill=F[1])
    seat(d, cx, base, 19)
    return im


def fin_stepped(size, anchor, height):
    """A snapped stump broken course by course into three falling steps, its top block shoved west."""
    im = canvas(size)
    d = ImageDraw.Draw(im)
    cx, base = anchor
    top = base - height
    left, right = cx - 16, cx + 15
    ground_shadow(d, cx, base, 19)
    front = [top + (0 if i < 8 else 9 if i < 17 else 18) for i in range(right - left + 1 - SIDE)]
    body(d, left, right, base, front + side_tops(front[-1]))
    # The top block slid two pixels west on its joint; its underside shades the course below.
    d.rectangle((left - 2, top, left - 1, top + 2), fill=F[3])
    d.rectangle((left - 2, top + 3, left - 1, top + 13), fill=F[2])
    d.line((left - 2, top + 3, left - 2, top + 13), fill=F[3])
    d.line((left - 2, top + 14, left + 7, top + 14), fill=F[0])
    d.line((left, top + 15, left, base - 12), fill=F[3])
    # Each higher block shows a short shadowed riser where the next course broke away.
    d.rectangle((left + 6, top + 3, left + 7, top + 11), fill=F[0])
    d.rectangle((left + 15, top + 12, left + 16, top + 20), fill=F[0])
    for y in (top + 32, base - 24):
        d.line((left + 1, y, right - 6, y), fill=F[1])
    slot(d, cx, top + 27, base - 19)
    seat(d, cx, base, 19)
    # The fallen capstone rests on the plinth's west end.
    d.rectangle((cx - 20, base - 12, cx - 10, base - 10), fill=F[3])
    d.rectangle((cx - 20, base - 9, cx - 10, base - 4), fill=F[2])
    d.rectangle((cx - 11, base - 11, cx - 10, base - 4), fill=F[0])
    d.line((cx - 9, base - 9, cx - 9, base - 4), fill=INK)
    return im


def fin_split(size, anchor, height):
    """A fin split along its channel: a tall west blade beside a lower broken east leaf with a missing block."""
    im = canvas(size)
    d = ImageDraw.Draw(im)
    cx, base = anchor
    top = base - height
    left, right = cx - 16, cx + 15
    ground_shadow(d, cx, base, 19)
    sill = base - 56
    leaf = top + 40
    blade = [top + (0 if i < 9 else 3) for i in range(12)]
    gap = [sill + 1] * 7
    east = [leaf] * 7
    body(d, left, right, base, blade + gap + east + side_tops(leaf))
    d.line((left, top + 3, left, base - 12), fill=F[3])
    # The free-standing blade's east face and the open channel between both leaves.
    d.line((left + 11, top + 6, left + 11, leaf - 3), fill=F[1])
    slot(d, cx, leaf + 3, sill)
    notch(d, right - SIDE + 1, right, base - 52, base - 40, F[2], right - SIDE, F[1])
    d.line((left + 1, base - 30, right - 6, base - 30), fill=F[1])
    seat(d, cx, base, 19)
    return im


def fin_whole(size, anchor, height):
    """An intact landing fin with a clean crown, kept as the reference that makes the others read as broken."""
    im = canvas(size)
    d = ImageDraw.Draw(im)
    cx, base = anchor
    top = base - height
    left, right = cx - 16, cx + 15
    ground_shadow(d, cx, base, 16)
    front = [top] * (right - left + 1 - SIDE)
    body(d, left, right, base, front + side_tops(top), cap=2)
    d.line((left, top + 2, left, base - 12), fill=F[3])
    slot(d, cx, top + 16, base - 31)
    d.line((left + 1, base - 24, right - 6, base - 24), fill=F[1])
    d.polygon([(left, base - 22), (left + 4, base - 22), (left + 4, base - 18), (left, base - 16)], fill=F[1])
    seat(d, cx, base, 16)
    return im


def fin_snapped(size, anchor, height):
    """A short landing fin snapped on a break that rises east, so its wide fracture plane faces the light."""
    im = canvas(size)
    d = ImageDraw.Draw(im)
    cx, base = anchor
    top = base - height
    left, right = cx - 16, cx + 15
    ground_shadow(d, cx, base, 16)
    front = [top + max(0, 9 - i // 2) for i in range(right - left + 1 - SIDE)]
    body(d, left, right, base, front + side_tops(top), cap=4)
    d.line((left, top + 13, left, base - 12), fill=F[3])
    slot(d, cx, top + 20, base - 19)
    d.line((left + 1, base - 26, right - 6, base - 26), fill=F[1])
    seat(d, cx, base, 16)
    return im


def brazier(frame, cold=False):
    """Draw a small stone burner whose fire is confined to a shallow brass bowl.

    The lit bowl shows its inner back wall beside the coals and a lip that steps from bright under
    the flame to dark brass at its ends; the cold row keeps its original dormant look.
    """
    im = canvas((24, 40))
    d = ImageDraw.Draw(im)
    d.polygon([(5, 36), (17, 36), (21, 38), (8, 38)], fill=INK)
    d.rectangle((9, 23, 13, 33), fill=F[1])
    d.line((9, 23, 9, 33), fill=F[2])
    d.polygon([(6, 33), (15, 33), (17, 36), (5, 36)], fill=F[2])
    d.line((6, 33, 14, 33), fill=F[3])
    d.polygon([(5, 19), (18, 19), (16, 23), (7, 23)], fill=F[0])
    if cold:
        d.line((5, 19, 18, 19), fill=F[2])
        return im
    d.line((7, 18, 16, 18), fill=G[1])
    d.point([(6, 18), (17, 18)], fill=G[0])
    d.line((5, 19, 18, 19), fill=G[1])
    d.line((9, 19, 14, 19), fill=G[2])
    d.point([(5, 19), (18, 19)], fill=G[0])
    shift = [0, 1, 1, 0, -1, 0][frame]
    d.polygon([(9, 18), (9, 15), (11 + shift, 10 + frame % 2), (12 + shift, 15), (14, 17), (13, 19)], fill=G[2])
    d.rectangle((10, 16, 12, 18), fill=G[3])
    return im


def props():
    """Build every integrated Summit prop with original collision footprints."""
    definitions = [
        ('summit_pillars', (64, 160), (32, 155), [(f'pillar_{v}', [draw((64, 160), (32, 155), h)], {'footprint': [16, 7]}) for v, (draw, h) in enumerate([(fin_sheared, 140), (fin_stepped, 101), (fin_split, 127)])]),
        ('summit_stones', (40, 112), (20, 107), [(f'monolith_{v}', [draw((40, 112), (20, 107), h)], {'footprint': [14, 6]}) for v, (draw, h) in enumerate([(fin_whole, 92), (fin_snapped, 66)])]),
        ('summit_brazier', (24, 40), (12, 36), [('brazier', [brazier(i) for i in range(6)], {'fps': 8, 'footprint': [14, 6], 'light': {'color': G[3], 'radius': 34, 'intensity': 0.45, 'offset': [0, -20]}}), ('brazier_cold', [brazier(0, True)], {'footprint': [14, 6]})]),
    ]
    post = canvas((16, 24))
    d = ImageDraw.Draw(post)
    d.rectangle((5, 9, 9, 20), fill=F[1])
    d.rectangle((4, 8, 10, 10), fill=F[2])
    d.line((4, 8, 9, 8), fill=F[3])
    d.rectangle((3, 20, 11, 22), fill=F[1])
    definitions.append(('summit_walls', (16, 24), (8, 22), [('post', [post], {})]))
    pedestal = canvas((64, 48))
    d = ImageDraw.Draw(pedestal)
    d.polygon([(13, 30), (45, 30), (53, 40), (18, 40)], fill=INK)
    d.polygon([(12, 28), (39, 28), (49, 32), (22, 34)], fill=F[2])
    d.polygon([(12, 28), (22, 34), (49, 32), (49, 38), (22, 39), (12, 34)], fill=F[1])
    d.polygon([(21, 29), (31, 27), (41, 30), (32, 33)], fill=F[0])
    d.line([(15, 28), (20, 21), (23, 29)], fill=F[3], width=2)
    d.line([(38, 29), (42, 25), (44, 31)], fill=F[1], width=2)
    definitions.append(('summit_props', (64, 48), (32, 38), [('pedestal', [pedestal], {})]))
    banner = canvas((32, 64))
    d = ImageDraw.Draw(banner)
    d.rectangle((8, 11, 9, 59), fill=F[0])
    d.line((8, 11, 8, 57), fill=F[2])
    d.line((7, 14, 23, 14), fill=F[2])
    d.polygon([(10, 15), (23, 15), (24, 36), (20, 34), (18, 40), (15, 36), (12, 38)], fill=S[0])
    d.polygon([(11, 15), (14, 15), (16, 34), (13, 35)], fill=F[1])
    d.line((17, 19, 19, 27), fill=F[2])
    definitions.append(('summit_banners', (32, 64), (8, 59), [('banner', [banner], {'footprint': [4, 3]})]))
    for name, size, anchor, rows in definitions:
        cols = max(len(frames) for _, frames, _ in rows)
        atlas = canvas((size[0] * cols, size[1] * len(rows)))
        anims = {}
        for row, (anim, frames, meta) in enumerate(rows):
            for col, frame in enumerate(frames):
                atlas.paste(frame, (col * size[0], row * size[1]))
            anims[anim] = {'row': row, 'frames': len(frames), 'fps': 1, **meta}
        save(name, atlas, {'cell': list(size), 'anchor': list(anchor), 'emissive': [G[3]] if name == 'summit_brazier' else [], 'anims': anims, 'notes': 'The Long Quiet v2. regions.warden palette. Broad slotted waterworks, quiet unlit stone; only burner coals emit. Rebuild: uv run --no-project --with pillow python3 tools/summit/style_v2.py'})


if __name__ == '__main__':
    tiles()
    props()
