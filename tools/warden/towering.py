#!/usr/bin/env python3
"""Deterministic native-pixel towering Warden sculpture.

No network, generated credentials, albedo resizing, rotation or interpolation.
Archived PixelLab native tower crowns, chest and complete feet are registered
around explicitly authored long masonry planes. Head and collar are one mass;
only recessed glass/lids animate. Production installation is deliberately external.

uv run --with pillow python tools/warden/towering.py --review out/warden-towering/review
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parents[2]
BASELINE = ROOT / 'art/warden-towering/2026-11-02/baseline'
DEFAULT = ROOT / 'out/warden-towering/candidate'
CELL = (144, 160)
ANCHOR = (72, 157)
HEAD_CELL = (64, 64)
HEAD_ANCHOR = (32, 61)
PALETTE = tuple(tuple(bytes.fromhex(c)) for c in (
    '1d232e', '242b3a', '303848', '3a4254', '4a5263', '585f6f',
    '6c7281', '7a808d', '8a8f9b', '979ba5', 'a2a5ad', 'b3b5bb', 'ccccce',
))
PRISM = tuple(tuple(bytes.fromhex(c)) for c in ('564d6e', '8d7fa8', 'b7add0', 'e2d8e9'))
PLANES = {
    'front': (0, 1, 0), 'top': (0, 0, 1),
    'left': (-.75, .66, 0), 'right': (.75, .66, 0),
    'left_bevel': (-.48, .75, .45), 'right_bevel': (.48, .75, .45),
    'roof': (0, .55, .84), 'recess': (0, 1, 0),
}
SCULPTURE = {'schema': 2, 'gate': [52, 67, 92, 96], 'pier_top': 129,
             'pier_bottom': 158, 'center': 72, 'apron': [61, 114, 82, 129],
             'fixed_upper_rows': 46, 'integrated_head': True,
             'articulated_legs': True, 'joint_top': 113, 'sole_top': 152,
             'kneel_depth': 24, 'fold_max': 26}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encode(vector) -> tuple[int, int, int, int]:
    """World XYZ -> upright PNG XZY, matching the renderer's normal contract."""
    length = math.sqrt(sum(v*v for v in vector))
    x, y, z = (v/length for v in vector)
    return (*(round((v+1)*127.5) for v in (x, z, y)), 255)


def aligned(normal: Image.Image, art: Image.Image) -> Image.Image:
    out = normal.copy()
    out.putalpha(art.getchannel('A'))
    # Zero hidden channels as well; no stale detached stone in later extraction.
    px, ap = out.load(), art.load()
    for y in range(out.height):
        for x in range(out.width):
            if not ap[x, y][3]:
                px[x, y] = (0, 0, 0, 0)
    return out


class Stone:
    def __init__(self, size):
        self.art = Image.new('RGBA', size)
        self.normal = Image.new('RGBA', size)

    def poly(self, points, shade, plane='front'):
        color = (*PALETTE[shade], 255) if isinstance(shade, int) else (*shade, 255)
        ImageDraw.Draw(self.art).polygon(points, fill=color)
        ImageDraw.Draw(self.normal).polygon(points, fill=encode(PLANES[plane] if isinstance(plane, str) else plane))

    def rect(self, box, shade, plane='front'):
        x0, y0, x1, y1 = box
        self.poly(((x0, y0), (x1, y0), (x1, y1), (x0, y1)), shade, plane)

    def line(self, points, shade, plane='front', width=1):
        color = (*PALETTE[shade], 255) if isinstance(shade, int) else (*shade, 255)
        ImageDraw.Draw(self.art).line(points, fill=color, width=width)
        ImageDraw.Draw(self.normal).line(points, fill=encode(PLANES[plane]), width=width)

    def source(self, art, normal, box, pos):
        self.art.alpha_composite(art.crop(box), pos)
        self.normal.alpha_composite(normal.crop(box), pos)

    def pair(self):
        return self.art, aligned(self.normal, self.art)


def source_pair(name, anim, frame=0):
    meta = json.loads((BASELINE/'sprites'/f'warden_{name}.json').read_text())
    w, h = meta['cell']
    row = meta['anims'][anim]['row']
    box = (frame*w, row*h, (frame+1)*w, (row+1)*h)
    return tuple(Image.open(BASELINE/p/f'warden_{name}.png').convert('RGBA').crop(box)
                 for p in ('sprites', 'normals'))


def body(opening=False, damaged=False):
    """Architectural weight: continuous buttress fronts carry a recessed chest.

    Native PixelLab crowns/chest/piers remain one-to-one pixel registrations.
    Added height consists of deliberate trapezoidal stone planes and sparse
    staggered masonry courses, never repeated/resampled source rows.
    """
    s = Stone(CELL)
    # Upper chest is an unbroken front plane: no full-width collar shelf.
    # Its left/right edges are continued by the head's descending stone cheeks.
    s.poly(((31, 28), (110, 28), (112, 58), (103, 73), (39, 73), (30, 55)), 2)
    s.poly(((45, 38), (98, 38), (100, 67), (94, 74), (49, 74), (43, 63)), 5)
    # The counterforts change section at a real projecting elbow. Their narrow
    # lower fronts rake back into the native supports rather than forming walls.
    s.poly(((5, 31), (36, 31), (41, 58), (38, 80), (35, 91), (40, 111),
            (38, 132), (10, 132), (3, 119), (7, 105), (0, 89), (1, 79)), 1)
    s.poly(((6, 33), (24, 37), (27, 78), (23, 87), (5, 88), (2, 80)), 8)
    s.poly(((24, 37), (35, 33), (39, 58), (36, 81), (27, 89), (27, 78)), 3, 'right')
    s.poly(((6, 33), (13, 38), (11, 76), (5, 84), (2, 80)), 9, 'left_bevel')
    s.poly(((4, 79), (26, 78), (35, 89), (17, 97), (1, 88)), 9, 'roof')
    s.poly(((1, 89), (17, 97), (14, 113), (19, 128), (10, 131), (4, 118), (8, 105)), 8)
    s.poly(((17, 97), (29, 94), (34, 106), (34, 127), (19, 128), (14, 113)), 7)
    s.poly(((29, 94), (35, 90), (40, 111), (38, 131), (34, 127), (34, 106)), 2, 'right')
    s.poly(((1, 89), (5, 92), (11, 106), (8, 118), (12, 128), (10, 131), (4, 118), (8, 105)), 10, 'left_bevel')
    s.poly(((107, 31), (138, 31), (142, 79), (143, 89), (136, 106),
            (140, 119), (133, 132), (105, 132), (104, 111), (110, 92), (105, 80), (103, 58)), 1)
    s.poly(((109, 34), (124, 37), (135, 33), (140, 80), (138, 88), (119, 87), (117, 77)), 7)
    s.poly(((109, 34), (117, 40), (119, 77), (117, 87), (107, 80), (106, 59)), 3, 'left')
    s.poly(((135, 33), (140, 37), (142, 80), (138, 87), (136, 77)), 2, 'right')
    s.poly(((119, 78), (139, 79), (143, 88), (128, 97), (109, 90)), 8, 'roof')
    s.poly(((128, 97), (142, 89), (135, 106), (139, 118), (133, 131), (126, 128), (130, 112)), 6)
    s.poly(((116, 94), (128, 97), (130, 112), (126, 128), (111, 127), (111, 106)), 7)
    s.poly(((109, 91), (116, 94), (111, 106), (111, 127), (105, 131), (105, 111)), 2, 'left')
    # Only two actual stone courses; their offsets follow the changed sections.
    s.line(((6, 65), (23, 67), (26, 66)), 5)
    s.line(((6, 66), (23, 68), (26, 67)), 9)
    s.line(((119, 65), (137, 64)), 4)
    s.line(((119, 66), (137, 65)), 8)
    s.line(((14, 116), (28, 116), (33, 118)), 5)
    s.line(((14, 117), (28, 117), (33, 119)), 8)
    s.line(((113, 116), (128, 114)), 4)
    s.line(((113, 117), (128, 115)), 8)
    # Front structure bridges to the two feet: no suspended central T-shaped tab.
    s.poly(((37, 60), (50, 62), (95, 62), (107, 59), (107, 116), (99, 130),
            (86, 132), (80, 120), (65, 120), (58, 132), (42, 131), (37, 116)), 3)
    s.poly(((42, 64), (101, 64), (104, 112), (96, 124), (83, 124), (79, 118),
            (65, 118), (61, 126), (45, 126), (41, 112)), 5)
    s.poly(((43, 105), (99, 105), (104, 113), (96, 118), (47, 118), (39, 112)), 7, 'roof')
    s.poly(((46, 115), (97, 115), (96, 129), (84, 129), (80, 122),
            (64, 122), (59, 129), (46, 129)), 4)
    # A rounded arch springing carries the belly into both piers.
    s.poly(((46, 118), (59, 118), (66, 120), (62, 124), (58, 132), (44, 132)), 7)
    s.poly(((84, 118), (98, 118), (100, 132), (86, 132), (81, 123), (77, 120)), 6)
    s.line(((48, 118), (59, 118), (65, 121)), 10, 'roof')
    s.line(((81, 121), (85, 118), (95, 118)), 8, 'roof')
    # A few structural losses belong to their geometric planes, not noise.
    s.poly(((8, 45), (10, 47), (9, 54), (7, 55)), 10, 'left_bevel')
    s.poly(((125, 54), (128, 53), (130, 55), (128, 58)), 9, 'roof')
    a, n = source_pair('body', 'damaged_opening' if damaged and opening else
                        'damaged' if damaged else 'opening' if opening else 'awake')
    # Native immutable attachment towers, native valve/chest, native complete feet.
    s.source(a, n, (0, 0, 40, 32), (0, 0))
    s.source(a, n, (82, 0, 120, 32), (106, 0))
    # Shoulder buttresses are sloped stone continuations of the hood cheeks.
    # They descend beside the heart to y95, with matching shade AND plane.
    s.poly(((28, 30), (35, 31), (53, 47), (54, 60), (43, 52), (32, 42)), 7, 'roof')
    s.poly(((29, 31), (35, 33), (49, 48), (46, 55), (40, 49), (31, 41)), 9, 'left_bevel')
    s.poly(((116, 30), (109, 31), (91, 47), (90, 60), (101, 52), (112, 42)), 6, 'roof')
    s.poly(((115, 31), (109, 33), (95, 48), (98, 55), (104, 49), (113, 41)), 4, 'right_bevel')
    s.poly(((44, 40), (50, 43), (49, 66), (45, 89), (42, 100), (37, 94), (40, 65)), 6, 'left')
    s.poly(((47, 42), (57, 44), (57, 65), (52, 86), (50, 96), (43, 101), (45, 75)), 8)
    s.poly(((98, 40), (93, 43), (94, 66), (99, 89), (102, 100), (107, 94), (104, 65)), 3, 'right')
    s.poly(((96, 42), (87, 44), (87, 65), (92, 86), (94, 96), (101, 101), (99, 75)), 5)
    # This center plane continues the lower face without a seam or pedestal.
    s.rect((57, 45, 86, 65), 5)
    s.source(a, n, (36, 34, 84, 81), (48, 65))
    s.source(a, n, (0, 81, 60, 110), (0, 129))
    s.source(a, n, (60, 81, 120, 110), (84, 129))
    if damaged:
        d = ImageDraw.Draw(s.art)
        for pts in (((6, 72), (10, 74), (9, 81), (5, 79)),
                    ((137, 112), (142, 109), (142, 119), (138, 121))):
            d.polygon(pts, fill=(0, 0, 0, 0))
        s.line(((27, 88), (31, 92), (30, 98)), 0, 'right')
    # The fixed inner arch springing cannot descend below the chest beam:
    # during fold12..16 the complete foreground piers rise and splay out from
    # underneath it, exposing two pale blocks that appear to hover. Carve that
    # unsupported mass away, leaving a native chamfered opening. The actual
    # rigid piers begin at129 and remain byte-for-byte complete; their native
    # upper arches slide behind the beam. Aligned normals lose the same pixels.
    ImageDraw.Draw(s.art).polygon(((51, 119), (93, 119), (96, 122),
                                  (96, 128), (48, 128), (48, 122)),
                                 fill=(0, 0, 0, 0))
    return s.pair()


def folded(art: Image.Image, normal: Image.Image, depth: int):
    """Fold two faceted legs above invariant native soles, without foot sliding.

    The upper assembly descends by ``depth`` in world space. Its hip stays at
    image y113, the sole moves to y152-depth, and the intervening thigh swings
    outwards over a broad knee. The shin becomes shorter/foreshortened beneath
    that knee. This is native joint construction, not a translated or squashed
    pier. The last six original rows are registered without changing any X.
    """
    if not 0 <= depth <= SCULPTURE['fold_max']:
        raise ValueError(f'fold depth outside native range: {depth}')
    s = Stone(CELL)
    s.source(art, normal, (0, 0, 144, 113), (0, 0))
    sole_y = SCULPTURE['sole_top']-depth
    # Preserve the arm/counterfort ends and the chest beam above the changing
    # joints. The deep crouch brings these heavy ends down beside the knees.
    end = min(129, sole_y)
    s.source(art, normal, (0, 113, 39, end), (0, 113))
    s.source(art, normal, (105, 113, 144, end), (105, 113))
    s.source(art, normal, (39, 113, 105, min(119, sole_y)), (39, 113))

    amount = min(depth/24, 1)
    knee_x = 39-round(10*amount)
    knee_y = 133-round(15*depth/26)
    knee_bottom = min(knee_y+4, sole_y-2)
    knee_top = knee_y-5
    hip_inner = 49+round(9*amount)
    thigh_normal = (-.12, 1-.62*amount, .12+.8*amount)

    def leg(mirror=False):
        def points(items):
            return tuple((143-x if mirror else x, y) for x, y in items)

        def plane(v):
            return (-v[0], v[1], v[2]) if mirror else v

        def poly(items, shade, facing='front'):
            s.poly(points(items), shade, plane(facing) if isinstance(facing, tuple) else facing)

        k, ky, kb = knee_x, knee_top, knee_bottom
        # Dark connected body: the inner hip never ends in a hanging block.
        poly(((39, 113), (hip_inner+2, 113), (k+16, ky+2), (k+15, kb),
              (52, sole_y), (26, sole_y), (k-16, kb), (k-16, ky+3)), 1)
        # A long oblique thigh face slopes from the inner hip to an outward
        # knee. Its lit plane turns upward as the upper leg becomes horizontal.
        poly(((40, 113), (hip_inner, 113), (k+13, ky), (k-13, ky+2)),
             5 if mirror else 7, thigh_normal)
        poly(((40, 113), (45, 114), (k-8, ky+2), (k-13, ky+2)),
             7 if mirror else 9, 'left_bevel' if not mirror else 'right_bevel')
        poly(((hip_inner, 113), (hip_inner+2, 114), (k+16, ky+3), (k+13, ky)),
             2 if mirror else 3, 'right' if not mirror else 'left')
        # The knee has a real cap, front and chamfer. It overlaps both thigh
        # and shin substantially, keeping a complete load-bearing silhouette.
        poly(((k-13, ky+1), (k+11, ky), (k+15, ky+4),
              (k+10, ky+6), (k-15, ky+6), (k-16, ky+4)),
             7 if mirror else 9, 'roof')
        poly(((k-15, ky+5), (k+10, ky+5), (k+14, kb),
              (k-13, kb)), 5 if mirror else 7)
        poly(((k+10, ky+5), (k+15, ky+4), (k+15, kb),
              (k+11, kb)), 2 if mirror else 3, 'right' if not mirror else 'left')
        # The shank rakes back under the projecting knee into the planted toe.
        # Its silhouette remains broad even at maximum impact compression.
        poly(((k-13, kb-1), (k+11, kb-1), (51, sole_y),
              (27, sole_y)), 3 if mirror else 4)
        poly(((k-13, kb-1), (k-7, kb), (31, sole_y), (26, sole_y)),
             5 if mirror else 7, 'left_bevel' if not mirror else 'right_bevel')
        poly(((k+11, kb-1), (k+15, kb-1), (53, sole_y), (49, sole_y)),
             1 if mirror else 2, 'right' if not mirror else 'left')

    leg()
    leg(True)
    # New geometry is confined to the joint region. Register the complete
    # source sole strip last, including transparency: no widened/clipped toes,
    # drifting heel, or former column underneath the new foot.
    s.art.paste(art.crop((0, 0, 144, 113)), (0, 0))
    s.normal.paste(normal.crop((0, 0, 144, 113)), (0, 0))
    isolated_chip = art.getpixel((139, 121))[3] and all(
        not art.getpixel(point)[3] for point in ((138, 121), (140, 121), (139, 120), (139, 122)))
    for image, source in ((s.art, art), (s.normal, normal)):
        ImageDraw.Draw(image).rectangle((0, sole_y, 143, 159), fill=(0, 0, 0, 0))
        image.paste(source.crop((0, 152, 144, 158)), (0, sole_y))
        # The damaged standing key contains one isolated native chip at the
        # outside arm edge. Do not carry this orphan into new joint poses.
        if isolated_chip:
            image.putpixel((139, 121), (0, 0, 0, 0))
    return s.pair()


def head(anim: str, frame: int):
    """The upper eye recess of one carved facade, open into continuing chest planes."""
    s = Stone(HEAD_CELL)
    # Straight descending jambs have no lower contour or flared mounting rim.
    # Their exact colors/normals continue past this sprite into body y53..95.
    s.poly(((14, 10), (48, 10), (56, 16), (60, 27), (58, 44),
            (58, 63), (5, 63), (5, 44), (3, 27), (8, 16)), 0)
    s.poly(((14, 11), (47, 11), (54, 17), (10, 17)), 10, 'roof')
    s.poly(((10, 17), (54, 17), (58, 26), (7, 26)), 7)
    s.poly(((14, 12), (23, 12), (20, 17), (10, 17)), 11, 'left_bevel')
    s.poly(((47, 12), (54, 17), (58, 25), (50, 23)), 4, 'right_bevel')
    s.poly(((11, 18), (30, 18), (28, 25), (7, 25)), 8)
    s.poly(((31, 18), (53, 18), (57, 25), (31, 25)), 6)
    s.line(((31, 18), (30, 22), (31, 25)), 3)
    s.line(((15, 13), (23, 13)), 12, 'roof')
    # The brow slopes down one native step towards the iris. Its deep underside
    # is part of the stone recess; only glass/lids inside that recess animate.
    s.poly(((6, 26), (57, 26), (59, 30), (55, 34), (33, 36), (30, 36), (8, 33), (4, 30)), 1)
    s.poly(((7, 25), (30, 27), (32, 29), (34, 27), (56, 25), (58, 27),
            (34, 30), (31, 31), (8, 28)), 9, 'roof')
    s.poly(((6, 28), (29, 30), (32, 32), (34, 30), (56, 28), (55, 31),
            (34, 34), (31, 35), (9, 31)), 5)
    s.poly(((8, 32), (30, 35), (33, 36), (55, 32), (53, 37), (11, 37)), 0)
    # Cheeks are tall load-bearing jambs, never an hourglass helmet or a jaw.
    s.poly(((5, 32), (13, 34), (17, 43), (17, 63), (5, 63)), 6, 'left')
    s.poly(((8, 34), (14, 36), (18, 45), (17, 63), (7, 63)), 8)
    s.poly(((53, 34), (59, 31), (58, 63), (47, 63), (46, 44)), 3, 'right')
    s.poly(((52, 35), (56, 34), (56, 63), (47, 63), (46, 45)), 5)
    s.poly(((17, 36), (48, 36), (46, 44), (40, 48), (24, 48), (18, 44)), 1, 'recess')
    s.poly(((19, 36), (46, 36), (44, 42), (39, 46), (25, 46), (20, 42)), 0)
    # An unbounded facade front, color-identical to the upper chest below.
    s.poly(((18, 45), (23, 48), (40, 48), (46, 45), (47, 63), (17, 63)), 5)
    s.line(((18, 46), (23, 49), (28, 49)), 7, 'roof')
    s.line(((46, 46), (41, 49), (38, 49)), 6, 'roof')
    s.line(((9, 42), (10, 45), (10, 51)), 6, 'left')
    s.line(((48, 18), (47, 21), (49, 24), (48, 27)), 2)
    s.line(((46, 19), (46, 21)), 8)
    # Distinct acting within one rigid deep socket: smouldering narrow iris,
    # widening slit during flare, then a broader hot aperture in the roar.
    levels = {'idle': (0, 1, 1, 1), 'flare': (1, 2, 3, 3), 'roar': (2, 3, 3, 3, 3, 2)}
    level = levels[anim][frame]
    blink = (anim == 'idle' and frame == 0) or (anim == 'roar' and frame == 0)
    gaze = (0, 0, -2, -1)[frame] if anim == 'idle' else 0
    roar = anim == 'roar' and 1 <= frame <= 4
    flare = anim == 'flare' and frame >= 1
    s.rect((19, 36, 45, 42), 0)
    cx, y = 32 + gaze, 39
    width = 25 if roar else 23 if flare else 19
    half = width//2
    ramp = PRISM[:3] if level < 2 else (PRISM[1], PRISM[2], PRISM[2])
    if not blink:
        for x in range(32-half, 33+half):
            distance = abs(x-cx)
            color = ramp[0] if distance >= 6 else ramp[1] if distance >= 3 else ramp[2]
            s.rect((x, y, x, y), color)
            if roar or flare:
                s.rect((x+1 if x<32 else x-1, y+1, x+1 if x<32 else x-1, y+1), color)
        low = 10 if roar else 8 if flare else 6
        s.rect((cx-low//2, y+1, cx+(low-1)//2, y+1), PRISM[3])
        s.rect((cx-2, y, cx+2, y), PRISM[3])
        if roar:
            s.rect((cx-3, y-1, cx+3, y-1), PRISM[2])
            s.rect((cx-2, y+2, cx+2, y+2), PRISM[3])
        elif flare:
            s.rect((cx-2, y+2, cx+2, y+2), PRISM[1])
    else:
        s.rect((cx-2, y+1, cx+1, y+1), PRISM[2] if level == 0 else PRISM[3])
    return s.pair()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2)+'\n')


def build(out: Path, normals_out: Path, review: Path | None = None):
    out.mkdir(parents=True, exist_ok=True)
    normals_out.mkdir(parents=True, exist_ok=True)
    # Unchanged subparts come only from frozen evidence, never shared live assets.
    for name in ('fist', 'heart', 'crystal'):
        for directory, dest in (('sprites', out), ('normals', normals_out)):
            shutil.copyfile(BASELINE/directory/f'warden_{name}.png', dest/f'warden_{name}.png')
        shutil.copyfile(BASELINE/'sprites'/f'warden_{name}.json', out/f'warden_{name}.json')
    count = SCULPTURE['fold_max']+1
    atlas = Image.new('RGBA', (CELL[0]*count, CELL[1]*4))
    normals = Image.new('RGBA', atlas.size)
    anims, body_frames, audit = {}, {}, []
    for row, (name, opening, damaged) in enumerate((('awake_fold', False, False),
            ('opening_fold', True, False), ('damaged_fold', False, True), ('damaged_opening_fold', True, True))):
        a, n = body(opening, damaged)
        pairs = [folded(a, n, d) for d in range(count)]
        frames, maps = [pair[0] for pair in pairs], [pair[1] for pair in pairs]
        body_frames[name] = frames
        anims[name] = {'row': row, 'frames': count, 'fps': 1.0, 'durations': [60]*count, 'loop': False}
        for d, (art, normal) in enumerate(zip(frames, maps, strict=True)):
            assert art.getchannel('A').tobytes() == normal.getchannel('A').tobytes()
            assert not art.crop((0, 158, 144, 160)).getbbox()
            assert art.crop((0, 0, 144, 113)).tobytes() == frames[0].crop((0, 0, 144, 113)).tobytes()
            atlas.paste(art, (d*144, row*160)); normals.paste(normal, (d*144, row*160))
            audit.append({'anim': name, 'frame': d, 'opaque': sum(art.getchannel('A').histogram()[1:]), 'bbox': art.getbbox()})
    for alias, physical in {'idle': 'awake_fold', 'awake': 'awake_fold', 'brace': 'awake_fold',
            'damaged': 'damaged_fold', 'damaged_brace': 'damaged_fold',
            'opening': 'opening_fold', 'damaged_opening': 'damaged_opening_fold'}.items():
        anims[alias] = dict(anims[physical], frames=1, durations=[60])
    body_meta = {'cell': CELL, 'anchor': ANCHOR, 'emissive': [], 'anims': anims, 'sculpture': SCULPTURE,
        'notes': 'Towering native-pixel sculpture: native PixelLab crowns, recessed chest and complete culvert feet joined by explicit tall stone planes. Hood cheeks continue as chest pilasters beside the heart to y95, without a closed head rim or collar shelf. Sloped shoulder buttresses join the hood; outer counterforts have projecting elbows and raked forearms. Top113 rows remain rigid. Valve center (72,80). Fold index0..26 equals body_dy. A native faceted thigh bends outward into a broad knee and raked shin; its plane tilts upward as the joint folds. Settled kneel24, impact26. Native sole rows152..157 translate Y only by depth, preserving every foot pixel and worldX/Y. No foot splay, albedo scaling or resampling; only PW glass emits.'}
    atlas.save(out/'warden_body.png'); normals.save(normals_out/'warden_body.png'); write_json(out/'warden_body.json', body_meta)
    source_head = json.loads((BASELINE/'sprites/warden_head.json').read_text())
    h_atlas, h_normals = Image.new('RGBA', (384, 192)), Image.new('RGBA', (384, 192))
    head_frames = {}
    for row, name in enumerate(('idle', 'flare', 'roar')):
        head_frames[name] = []
        for i in range(source_head['anims'][name]['frames']):
            art, normal = head(name, i); head_frames[name].append(art)
            h_atlas.paste(art, (i*64, row*64)); h_normals.paste(normal, (i*64, row*64))
    alpha = head_frames['idle'][0].getchannel('A').tobytes()
    assert all(im.getchannel('A').tobytes() == alpha for frames in head_frames.values() for im in frames)
    h_atlas.save(out/'warden_head.png'); h_normals.save(normals_out/'warden_head.png')
    head_meta = dict(source_head, cell=HEAD_CELL, anchor=HEAD_ANCHOR,
        integrated_head=True, emissive=['#e2d8e9'],
        notes='Upper eye recess of one carved facade: heavy inward-stepped overhanging brow, deep slit and straight stone cheek planes continuing past the open lower sprite boundary into chest pilasters. No neck, chin wedge, bottom outline, flange or collar shelf. Static stone silhouette in every animation; inset glass widens through flare/roar. Original frame durations. Native13-slate+4-prism palette. Only PW emits. Normals are explicit front/top/chamfer planes in PNG XZY world encoding.')
    write_json(out/'warden_head.json', head_meta)
    layout = json.loads((BASELINE/'sprites/warden_layout.json').read_text())
    for name, offset in {'crystal_left': [-48, -148], 'crystal_right': [48, -148],
            'head': [0, -106], 'heart': [0, -77], 'fist_left': [-57, -11], 'fist_right': [57, -11]}.items():
        layout['parts'][name]['offset'] = offset
    layout['hurtboxes']['heart'] = [0, -77, 9]
    layout['hurtboxes']['crystal_left'] = [-47, -158, 7]
    layout['hurtboxes']['crystal_right'] = [47, -158, 7]
    layout['notes'] = 'Offsets from native body anchor (72,157). Towering sculpture about170px high. Head cheek planes cross the sprite boundary directly into y53..95 chest pilasters; no collar shelf. Crystal seats fixed at +/-48,-148. No head lift/bob independent of torso. Native complete pier feet cancel body_dy through fold remapping.'
    layout['bob']['notes'] = 'Body, integrated head, heart and crystals form one rigid upper assembly; no independent stone-head breath/lift. Fists keep original animation and combat timing.'
    write_json(out/'warden_layout.json', layout)
    report = {'source': str(BASELINE.relative_to(ROOT)),
        'source_sha256': {str(p.relative_to(BASELINE)): sha(p) for p in sorted(BASELINE.rglob('*')) if p.is_file()},
        'cell': CELL, 'anchor': ANCHOR, 'head_cell': HEAD_CELL, 'head_anchor': HEAD_ANCHOR,
        'sculpture': SCULPTURE, 'body_frames': audit,
        'body_opaque': audit[0]['opaque'], 'head_opaque': sum(head_frames['idle'][0].getchannel('A').histogram()[1:]),
        'body_palette': len(set(p[:3] for p in atlas.get_flattened_data() if p[3])),
        'head_palette': len(set(p[:3] for p in h_atlas.get_flattened_data() if p[3])),
        'normal_alpha_mismatches': 0, 'resampled_albedo_pixels': 0,
        'outputs': {str(p.relative_to(out.parent)): sha(p) for p in sorted(out.parent.rglob('warden*')) if p.is_file()}}
    if review:
        review.mkdir(parents=True, exist_ok=True)
        gallery(out, normals_out, review)
        write_json(review/'audit.json', report)
    return report


def part(directory: Path, name: str, anim: str, frame=0, mirror=False):
    meta = json.loads((directory/f'warden_{name}.json').read_text())
    w, h = meta['cell']; row = meta['anims'][anim]['row']
    im = Image.open(directory/f'warden_{name}.png').convert('RGBA').crop((frame*w, row*h, (frame+1)*w, (row+1)*h))
    anchor = list(meta['anchor'])
    if mirror:
        im = ImageOps.mirror(im); anchor[0] = w-1-anchor[0]
    return im, anchor


def assembly(directory: Path, pose='awake_fold', fold=0, head_anim='idle', head_frame=1):
    layout = json.loads((directory/'warden_layout.json').read_text())
    canvas = Image.new('RGBA', (236, 210))
    ground = (118, 183)
    # Old body folds support comparison without modifying the frozen registration.
    for name in layout['draw_order']:
        if name == 'body':
            im, anchor = part(directory, 'body', pose, fold)
            off = [0, fold]
        else:
            desc = layout['parts'][name]; off = list(desc['offset'])
            kind = desc['sheet'].removeprefix('warden_')
            anim = {'crystal': 'intact', 'head': head_anim, 'heart': 'whole', 'fist': 'clenched'}[kind]
            frame = head_frame if kind == 'head' else 0
            im, anchor = part(directory, kind, anim, frame, desc.get('mirror', False))
            if kind != 'fist': off[1] += fold
            if kind == 'heart' and 'opening' in pose: off[1] = 10
        canvas.alpha_composite(im, (ground[0]+off[0]-anchor[0], ground[1]+off[1]-anchor[1]))
    return canvas


def gallery(out, normals_out, review):
    choices = [('awake_fold', 0, 'idle', 1), ('awake_fold', 8, 'idle', 2),
               ('awake_fold', 16, 'flare', 3), ('opening_fold', 24, 'roar', 2),
               ('damaged_fold', 26, 'idle', 1), ('damaged_opening_fold', 24, 'roar', 4)]
    board = Image.new('RGBA', (236*4, 238*2), (24, 26, 34, 255))
    draw = ImageDraw.Draw(board)
    hm = json.loads((ROOT/'assets/sprites/hero.json').read_text()); hw, hh = hm['cell']
    hr = hm['anims']['idle_down']['row']; hero = Image.open(ROOT/'assets/sprites/hero.png').convert('RGBA').crop((0,hr*hh,hw,(hr+1)*hh))
    panels = [('Previous', assembly(BASELINE/'sprites'))] + [(f'{p} depth{f}', assembly(out,p,f,h,hf)) for p,f,h,hf in choices]
    for i, (label, im) in enumerate(panels):
        x, y = (i%4)*236, (i//4)*238
        board.alpha_composite(im, (x,y+16))
        board.alpha_composite(hero, (x+32-hm['anchor'][0], y+199-hm['anchor'][1]))
        draw.line((x+17,y+200,x+218,y+200), fill=(59,61,73))
        draw.text((x+10,y+216),label,fill=(198,200,210))
    board.save(review/'assembled-native.png')
    board.resize((board.width*3,board.height*3), Image.Resampling.NEAREST).save(review/'assembled-3x.png')
    # Single comparison with no analytical overlays: quick silhouette/read test.
    simple = board.crop((0,0,472,238));simple.save(review/'before-after-native.png')
    simple.resize((944,476),Image.Resampling.NEAREST).save(review/'before-after-2x.png')
    heads = Image.new('RGBA',(7*80,2*88),(24,26,34,255));d=ImageDraw.Draw(heads)
    poses = [(name,i) for name in ('idle','flare','roar') for i in range(4 if name != 'roar' else 6)]
    for k,(name,i) in enumerate(poses):
        x,y=(k%7)*80,(k//7)*88;im,_=part(out,'head',name,i);heads.alpha_composite(im,(x+8,y+4));d.text((x+5,y+72),f'{name}{i}',fill=(198,200,210))
    heads.save(review/'head-acting-native.png');heads.resize((heads.width*3,heads.height*3),Image.Resampling.NEAREST).save(review/'head-acting-3x.png')
    # Native normal geometry under diagnostic cardinal lights. Actual game QA
    # remains the caller's responsibility; this contains no renderer imitation.
    lighting = Image.new('RGBA',(144*4,182*2),(24,26,34,255));d=ImageDraw.Draw(lighting)
    for row, name in enumerate(('awake','damaged')):
        a,n=source_pair_from(out,normals_out,name)
        for col,light in enumerate(((-.8,.6,.3),(.8,.6,.3),(0,1,.25),(0,0,1))):
            ln=math.sqrt(sum(v*v for v in light));lit=a.copy()
            for y in range(160):
                for x in range(144):
                    p=a.getpixel((x,y))
                    if not p[3]:continue
                    q=n.getpixel((x,y));v=[q[0]/127.5-1,q[2]/127.5-1,q[1]/127.5-1]
                    vl=math.sqrt(sum(c*c for c in v));gain=.4+.72*max(0,sum(v[i]*light[i]/(vl*ln) for i in range(3)))
                    lit.putpixel((x,y),(*(min(255,round(c*gain)) for c in p[:3]),255))
            lighting.alpha_composite(lit,(col*144,row*182));d.text((col*144+4,row*182+164),f'{name} light{col+1}',fill=(198,200,210))
    lighting.save(review/'normal-lights-native.png');lighting.resize((lighting.width*3,lighting.height*3),Image.Resampling.NEAREST).save(review/'normal-lights-3x.png')
    kneel_gallery(out, normals_out, review)


def kneel_gallery(out, normals_out, review):
    """World-registered joint sheets and source motion, without renderer effects."""
    backdrop = (24, 26, 34, 255)
    for key in ('awake_fold', 'opening_fold', 'damaged_fold', 'damaged_opening_fold'):
        sheet = Image.new('RGBA', (154*7, 88*4), backdrop)
        draw = ImageDraw.Draw(sheet)
        for depth in range(27):
            art, _ = part(out, 'body', key, depth)
            world = Image.new('RGBA', (144, 186))
            world.alpha_composite(art, (0, depth))
            x, y = (depth % 7)*154, (depth//7)*88
            sheet.alpha_composite(world.crop((0, 103, 144, 161)), (x+5, y+2))
            draw.line((x+2, y+57, x+150, y+57), fill=(59, 61, 73))
            draw.text((x+6, y+66), f'depth {depth}', fill=(198, 200, 210))
        sheet.save(review/f'{key}-joints-native.png')
        sheet.resize((sheet.width*2, sheet.height*2), Image.Resampling.NEAREST).save(
            review/f'{key}-joints-2x.png')

    baseline = ROOT/'out/warden-kneel/baseline-assets/sprites'
    if baseline.exists():
        panels = [('Before standing', assembly(baseline, fold=0)),
                  ('Before lowered16', assembly(baseline, fold=16)),
                  ('Articulated standing', assembly(out, fold=0)),
                  ('Articulated kneel24', assembly(out, fold=24))]
        board = Image.new('RGBA', (236*4, 238), backdrop)
        draw = ImageDraw.Draw(board)
        for i, (label, image) in enumerate(panels):
            board.alpha_composite(image, (i*236, 16))
            draw.line((i*236+17, 200, i*236+218, 200), fill=(59, 61, 73))
            draw.text((i*236+8, 218), label, fill=(198, 200, 210))
        board.save(review/'kneel-before-after-native.png')
        board.resize((board.width*2, board.height*2), Image.Resampling.NEAREST).save(
            review/'kneel-before-after-2x.png')

    motion = []
    depths = [0]*8+list(range(27))+[24]*18+list(range(24, -1, -1))+[0]*8
    for depth in depths:
        panel = Image.new('RGBA', (236, 238), backdrop)
        panel.alpha_composite(assembly(out, fold=depth), (0, 16))
        draw = ImageDraw.Draw(panel)
        draw.line((17, 200, 218, 200), fill=(59, 61, 73))
        draw.text((10, 218), f'native joint depth{depth}', fill=(198, 200, 210))
        motion.append(panel.resize((708, 714), Image.Resampling.NEAREST))
    motion[0].save(review/'articulated-motion-source.gif', save_all=True,
                   append_images=motion[1:], duration=45, loop=0, disposal=2)


def source_pair_from(out, normals_out, anim):
    a,_=part(out,'body',anim)
    meta=json.loads((out/'warden_body.json').read_text());row=meta['anims'][anim]['row']
    n=Image.open(normals_out/'warden_body.png').convert('RGBA').crop((0,row*160,144,(row+1)*160))
    return a,n


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out',type=Path,default=DEFAULT/'sprites')
    p.add_argument('--normals-out',type=Path)
    p.add_argument('--review',type=Path)
    a=p.parse_args();report=build(a.out,a.normals_out or a.out.parent/'normals',a.review)
    print(json.dumps({k:report[k] for k in ('cell','anchor','body_opaque','head_opaque','body_palette','head_palette','normal_alpha_mismatches')}))


if __name__=='__main__':main()
