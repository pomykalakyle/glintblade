"""Sculpture invariants that protect the integrated head and native planted feet."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from PIL import Image

import towering as t


class ToweringSculpture(unittest.TestCase):
    def test_head_has_one_rigid_stone_silhouette_and_original_timing(self):
        pairs = [t.head(name, i) for name, count in (('idle', 4), ('flare', 4), ('roar', 6))
                 for i in range(count)]
        alpha = pairs[0][0].getchannel('A').tobytes()
        normal = pairs[0][1].tobytes()
        stone = pairs[0][0]
        for art, normals in pairs:
            self.assertEqual(art.getchannel('A').tobytes(), alpha)
            self.assertEqual(normals.tobytes(), normal)
            # Outside the recessed slit, every stone pixel is absolutely rigid.
            for y in range(64):
                for x in range(64):
                    if not (19 <= x <= 45 and 36 <= y <= 41):
                        self.assertEqual(art.getpixel((x, y)), stone.getpixel((x, y)))
        # The last native rows are open planes: no closed dark lower contour,
        # flange, or narrowed chin that turns the eye recess into a helmet.
        for x in range(8, 57):
            self.assertGreaterEqual(stone.getpixel((x, 63))[3], 255)
            self.assertNotEqual(stone.getpixel((x, 63))[:3], t.PALETTE[0])

    def test_head_and_body_have_a_substantial_structural_joint(self):
        art, _ = t.body()
        head, _ = t.head('idle', 1)
        # Body-relative head origin is (40,-10). Its open lower face joins
        # physically continuous, same-material front planes at body y53.
        overlap = 0
        for y in range(46, 52):
            for x in range(40, 104):
                overlap += bool(art.getpixel((x, y))[3] and head.getpixel((x-40, y+10))[3])
        self.assertGreater(overlap, 250)
        _, head_normal = t.head('idle', 1)
        _, body_normal = t.body()
        for x in range(8, 57):
            for y in (53, 54):
                self.assertEqual(head.getpixel((x, 63)), art.getpixel((x+40, y)))
                self.assertEqual(head_normal.getpixel((x, 63)), body_normal.getpixel((x+40, y)))
        # Jambs run alongside the vulnerable gate, rather than ending at a shelf.
        self.assertEqual(art.getpixel((46, 85))[:3], t.PALETTE[8])
        self.assertEqual(art.getpixel((96, 85))[:3], t.PALETTE[5])
        # Entire body should read as a single masonry assembly in every key.
        mask = {(x, y) for y in range(160) for x in range(144) if art.getpixel((x, y))[3]}
        stack = [mask.pop()]
        while stack:
            x, y = stack.pop()
            for point in ((x-1, y), (x+1, y), (x, y-1), (x, y+1)):
                if point in mask:
                    mask.remove(point)
                    stack.append(point)
        self.assertFalse(mask)

    def test_all_108_native_folds_keep_exact_world_soles_and_upper_assembly(self):
        for opening in (False, True):
            for damaged in (False, True):
                art, normals = t.body(opening, damaged)
                for depth in range(27):
                    a, n = t.folded(art, normals, depth)
                    self.assertEqual(a.getchannel('A').tobytes(), n.getchannel('A').tobytes())
                    for actual, source in ((a, art), (n, normals)):
                        self.assertEqual(actual.crop((0, 0, 144, 113)).tobytes(),
                                         source.crop((0, 0, 144, 113)).tobytes())
                        # Includes transparent texels as well as toe/heel art.
                        # No horizontal displacement or widened foot is allowed.
                        self.assertEqual(actual.crop((0, 152-depth, 144, 158-depth)).tobytes(),
                                         source.crop((0, 152, 144, 158)).tobytes())
                    self.assertEqual(a.getbbox()[3]+depth, 158)
                    self.assertFalse(a.crop((0, 158-depth, 144, 160)).getbbox())

    def test_all_articulated_keys_have_complete_connected_joints(self):
        for opening in (False, True):
            for damaged in (False, True):
                art, normals = t.body(opening, damaged)
                for depth in range(27):
                    a, _ = t.folded(art, normals, depth)
                    mask = {(x, y) for y in range(160) for x in range(144)
                            if a.getpixel((x, y))[3]}
                    stack = [mask.pop()]
                    while stack:
                        x, y = stack.pop()
                        for point in ((x-1, y), (x+1, y), (x, y-1), (x, y+1)):
                            if point in mask:
                                mask.remove(point)
                                stack.append(point)
                    self.assertFalse(mask, (opening, damaged, depth, len(mask)))
                    # Substantial stone carries both soles into the chest, not
                    # one-pixel threads or disconnected knee/hip ornaments.
                    for y in range(119, 152-depth):
                        for x0, x1 in ((12, 62), (82, 132)):
                            width = sum(bool(a.getpixel((x, y))[3]) for x in range(x0, x1))
                            self.assertGreaterEqual(width, 18, (opening, damaged, depth, y))

    def test_deep_kneel_has_bent_native_planes_instead_of_lifted_columns(self):
        art, normals = t.body()
        for depth in (16, 24, 26):
            a, n = t.folded(art, normals, depth)
            # The knee projects laterally beyond the unchanged toe; its
            # substantial top face becomes visible as the femur folds down.
            kx = 39-round(10*min(depth/24, 1))
            ky = 133-round(15*depth/26)-5
            self.assertEqual(n.getpixel((kx-7, ky+3)), t.encode(t.PLANES['roof']))
            self.assertEqual(n.getpixel((143-kx+7, ky+3)), t.encode(t.PLANES['roof']))
            # A translated complete old pier has a different silhouette and
            # lighting geometry. Protect against accidentally restoring it.
            lifted = t.Image.new('RGBA', t.CELL)
            lifted.paste(art.crop((0, 129, 144, 158)), (0, 129-depth))
            alpha_changed = sum(a.getpixel((x, y))[3] != lifted.getpixel((x, y))[3]
                                for y in range(113, 152-depth) for x in range(144))
            self.assertGreater(alpha_changed, 100)
        for bad in (-1, 27):
            with self.assertRaises(ValueError):
                t.folded(art, normals, bad)

    def test_standing_and_first_fold_use_one_continuous_joint_rig(self):
        for opening in (False, True):
            for damaged in (False, True):
                art, normals = t.body(opening, damaged)
                frames = [t.folded(art, normals, d) for d in range(27)]
                # First movement must be comparable to the adjacent native
                # steps. It must never replace the old standing pier with a
                # different articulated sculpture on the first nonzero frame.
                changes = [sum(p != q for p, q in zip(frames[d][0].get_flattened_data(),
                                                     frames[d+1][0].get_flattened_data()))
                           for d in range(26)]
                self.assertLessEqual(changes[0], max(changes[1:]))
                self.assertLess(changes[0], 500)

    def test_export_reproducibility_metadata_and_palette(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folders = [root/'first', root/'second']
            for folder in folders:
                t.build(folder/'sprites', folder/'normals')
            for path in folders[0].rglob('*'):
                if path.is_file():
                    self.assertEqual(path.read_bytes(), (folders[1]/path.relative_to(folders[0])).read_bytes())
            sprites = folders[0]/'sprites'
            b = json.loads((sprites/'warden_body.json').read_text())
            h = json.loads((sprites/'warden_head.json').read_text())
            old = json.loads((t.BASELINE/'sprites/warden_head.json').read_text())
            self.assertEqual(b['sculpture'], t.SCULPTURE)
            self.assertEqual(b['anims']['awake_fold']['frames'], 27)
            self.assertEqual(b['sculpture']['kneel_depth'], 24)
            self.assertEqual(b['sculpture']['fold_max'], 26)
            self.assertEqual(h['anims'], old['anims'])
            self.assertEqual(b['anims']['brace'], b['anims']['awake'])
            self.assertEqual(b['anims']['damaged_brace'], b['anims']['damaged'])
            for name, allowed in (('body', set(t.PALETTE)), ('head', set(t.PALETTE+t.PRISM))):
                art = Image.open(sprites/f'warden_{name}.png').convert('RGBA')
                colors = {p[:3] for p in art.get_flattened_data() if p[3]}
                self.assertLessEqual(colors, allowed)
            self.assertEqual(h['emissive'], ['#e2d8e9'])
            self.assertFalse(b['emissive'])


if __name__ == '__main__':
    unittest.main()
