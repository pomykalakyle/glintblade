"""Native shape, contact-ink, support and face-lighting contracts."""
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image
import summit_columns as columns


class SummitColumns(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.result = columns.build()

    def original(self, name):
        return (Image.open(columns.ARCHIVE / "baseline/sprites" / f"{name}.png").convert("RGBA"),
                Image.open(columns.ARCHIVE / "baseline/normals" / f"{name}.png").convert("RGBA"),
                json.loads((columns.ARCHIVE / "baseline/sprites" / f"{name}.json").read_text()))

    def test_original_dimensional_shape_anchors_and_collision_are_exact(self):
        for name, (art, normals, meta) in self.result.items():
            old, _, old_meta = self.original(name)
            self.assertEqual(art.size, old.size)
            self.assertEqual(art.getchannel("A").tobytes(), old.getchannel("A").tobytes())
            self.assertEqual(art.getchannel("A").tobytes(), normals.getchannel("A").tobytes())
            self.assertEqual(set(art.getchannel("A").tobytes()), {0, 255})
            self.assertEqual(meta["anchor"], old_meta["anchor"])
            for key, anim in meta["anims"].items():
                for field in ("row", "frames", "fps", "anchor", "footprint", "collision_rects", "support_first_row"):
                    self.assertEqual(anim[field], old_meta["anims"][key][field], (name, key, field))

    def test_only_ground_facing_ink_changes_not_caps_or_side_outlines(self):
        count = 0
        for name, (art, _, meta) in self.result.items():
            old, _, _ = self.original(name)
            for anim in meta["anims"].values():
                row, first = anim["row"], anim["support_first_row"]
                repairs = {tuple(r["xy"]): tuple(r["face_source"]) for r in anim["bottom_outline_repair"]}
                for y in range(112):
                    for x in range(64):
                        xy = (x, y)
                        a, b = old.getpixel((x, row * 112 + y)), art.getpixel((x, row * 112 + y))
                        if xy not in repairs:
                            self.assertEqual(a, b, (name, row, xy))
                            continue
                        self.assertEqual(a, columns.INK)
                        bottom = max(yy for yy in range(first, 112) if old.getpixel((x, row * 112 + yy))[3])
                        self.assertIn(y, (bottom, bottom - 1))
                        sx, sy = repairs[xy]
                        self.assertEqual(sx, x, "never borrow light across a stone corner")
                        self.assertTrue(first <= sy < y)
                        self.assertEqual(b, old.getpixel((sx, row * 112 + sy)))
                        self.assertNotEqual(b, columns.INK)
                        count += 1
        self.assertEqual(count, 179, "explicit reviewed contour coverage across five native cells")

    def test_normals_retain_upper_art_and_describe_base_cap_and_returns(self):
        for name, (_, normals, meta) in self.result.items():
            _, original, _ = self.original(name)
            for anim in meta["anims"].values():
                row, first = anim["row"], anim["support_first_row"]
                box = (0, row * 112, 64, row * 112 + first)
                self.assertEqual(normals.crop(box).tobytes(), original.crop(box).tobytes())
                faces = set()
                for y in range(first, 112):
                    for x in range(64):
                        p = normals.getpixel((x, row * 112 + y))
                        if not p[3]:
                            continue
                        n = [p[0] / 127.5 - 1, p[2] / 127.5 - 1, p[1] / 127.5 - 1]
                        self.assertLess(abs(sum(v * v for v in n) - 1), 0.015)
                        self.assertGreater(n[2], -0.01, "a dressed base has no visible downward cushion")
                        faces.add("cap" if n[2] > 0.9 else "left" if n[0] < 0 else "right")
                self.assertEqual(faces, {"cap", "left", "right"})

    def test_semantic_base_samples_have_correct_world_facing(self):
        _, normals, _ = self.result["summit_pillars"]
        # Source4: exposed top at30,84, shadow return at19,94, front return at30,96.
        for xy, expected in [((30, 84), (0, 0, 1)),
                             ((19, 94), (-0.4472135955, 0.894427191, 0)),
                             ((30, 96), (0.4472135955, 0.894427191, 0))]:
            p = normals.getpixel(xy)
            n = [p[0] / 127.5 - 1, p[2] / 127.5 - 1, p[1] / 127.5 - 1]
            self.assertGreater(sum(a*b for a,b in zip(n, expected)), 0.99, xy)

    def test_export_is_scoped_and_repeatable(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "sprites").mkdir()
            (root / "sprites/unrelated.png").write_bytes(b"preserve")
            columns.export(root / "sprites", root / "normals")
            first = {str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}
            columns.export(root / "sprites", root / "normals")
            second = {str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}
            self.assertEqual(first, second)
            self.assertEqual(len(first), 7)


if __name__ == "__main__":
    unittest.main()
