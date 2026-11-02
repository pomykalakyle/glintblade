"""Projection, native support, independent plane directions and isolated export."""
import json
import math
from pathlib import Path
import tempfile
import unittest

from PIL import Image

import distributor_rework as rework


class DistributorRework(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.art, cls.normal, cls.labels, _, cls.recipe = rework.build()

    def test_known_geometric_faces_in_upright_engine_basis(self):
        # Independent source witnesses: a normal map that merely follows RGB
        # brightness or swaps upright G/B fails these specific surface checks.
        for xy, expected in {
            (80, 32): (0, 0, 1),       # empty cistern bed
            (70, 48): (0, 1, 0),       # header facade
            (40, 94): (0, 0, 1),       # narrow left root top
            (114, 94): (0, 0, 1),      # broken right root top
            (40, 98): (0, 1, 0),      # fitted foundation front
            (42, 30): (1, 0, 0),       # basin west inner return
            (117, 30): (-1, 0, 0),     # basin east inner return
            (66, 82): (1, 0, 0),       # inspection arch west return
            (91, 82): (-1, 0, 0),      # inspection arch east return
        }.items():
            with self.subTest(xy=xy):
                r, g, b, a = self.normal.getpixel(xy)
                world = [v / 127.5 - 1 for v in (r, b, g)]
                length = math.sqrt(sum(v*v for v in world))
                self.assertEqual(a, 255)
                self.assertGreater(sum(v / length * e for v, e in zip(world, expected)), 0.999)

    def test_native_hard_alpha_finished_feet_and_open_inspection_route(self):
        self.assertEqual(self.art.size, (144, 112))
        self.assertEqual(self.art.getbbox(), (14, 17, 130, 100))
        self.assertEqual(self.recipe["anchor"], [79, 103])
        self.assertEqual(self.normal.getchannel("A").tobytes(), self.art.getchannel("A").tobytes())
        self.assertEqual(set(self.art.getchannel("A").tobytes()), {0, 255})
        for xy in [(40,99),(114,99),(65,93),(95,93),(40,92)]:
            self.assertEqual(self.art.getpixel(xy)[3],255)
        for xy in [(49,101),(114,102),(29,90),(130,90)]:
            self.assertEqual(self.art.getpixel(xy)[3],0)
        for y in range(92, 100):
            for x in range(71, 89):
                self.assertEqual(self.art.getpixel((x, y))[3], 0)
        original = Image.open(rework.ARCHIVE/self.recipe["source"]).convert("RGBA")
        changed = set()
        for y in range(86,112):
            for x in range(144):
                self.assertEqual(self.art.getpixel((x,y))[3], original.getpixel((x,y))[3])
                if self.art.getpixel((x,y)) != original.getpixel((x,y)): changed.add((x,y))
        expected = {(x,y) for x0,y0,x1,y1 in [(31,96,56,99),(103,96,128,99),(58,93,66,93),(91,93,101,93)]
            for y in range(y0,y1+1) for x in range(x0,x1+1)
            if original.getpixel((x,y))[:3] in [(28,40,56),(34,45,61)]}
        self.assertEqual(changed,expected,"only authored ground-contact ink changes the native lower stone volume")
        self.assertGreater(len(changed),200)
        for xy in [(30,98),(57,98),(102,98),(129,98),(34,85),(66,84)]:
            self.assertEqual(self.art.getpixel(xy),original.getpixel(xy),"lateral/arch outline retained")
        self.assertEqual(self.art.getpixel((79, 80))[3], 0)

    def test_export_is_byte_exact_scoped_and_collider_follows_native_support(self):
        with tempfile.TemporaryDirectory() as tmp:
            sprites, normals = Path(tmp)/"sprites", Path(tmp)/"normals"
            sprites.mkdir(); normals.mkdir()
            (sprites/"unrelated.png").write_bytes(b"untouched")
            rework.export(sprites, normals)
            self.assertEqual((sprites/"unrelated.png").read_bytes(), b"untouched")
            for folder, live, suffix in [(sprites,"sprites",".png"), (sprites,"sprites",".json"), (normals,"normals",".png")]:
                name = rework.NAME + suffix
                self.assertEqual((folder/name).read_bytes(), (rework.ROOT/"assets"/live/name).read_bytes())
            metadata = json.loads((sprites/(rework.NAME+".json")).read_text())
            anim = metadata["anims"]["body"]
            for y in range(92, 100):
                for x in range(144):
                    blocked = any(x0 <= x < x1 and y0 <= y < y1 for x0,y0,x1,y1 in anim["collision_rects"])
                    self.assertEqual(blocked, bool(self.art.getpixel((x,y))[3]))
            self.assertEqual(anim["support_first_row"], 92)
            self.assertEqual(anim["contact"], "none")


if __name__ == "__main__":
    unittest.main()
