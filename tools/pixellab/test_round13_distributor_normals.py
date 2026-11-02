"""Native geometric directions and rejection-safe export for the distributor."""

import contextlib
import io
import json
import math
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image

import round13_landmarks as landmarks


def upright_world(normal, xy):
    tangent = [v / 127.5 - 1 for v in normal.getpixel(xy)[:3]]
    world = [tangent[0], tangent[2], tangent[1]]
    length = math.sqrt(sum(v * v for v in world))
    return [v / length for v in world]


class DistributorSemanticNormals(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.selection = json.loads((landmarks.ARCHIVE / "selection.json").read_text())
        cls.spec = cls.selection["assets"]["distributor"]
        cls.source = Image.open(landmarks.ARCHIVE / cls.spec["source"]).convert("RGBA")
        cls.art = landmarks.aperture(cls.source.copy(), cls.spec)
        cls.planes = json.loads((landmarks.ARCHIVE / cls.spec["normal_planes"]).read_text())
        cls.normal, cls.labels, _ = landmarks.semantic_normals(cls.art, cls.planes)

    def test_actual_top_side_and_front_directions_in_engine_upright_basis(self):
        # These are independent geometric landmarks in the actual native art,
        # rather than expected values read back from the authored plane file.
        # This catches the original bump-map mistake and a green/blue swap.
        points = {
            (79, 19): (0, 0, 1),  # open cistern floor
            (44, 100): (0, 0, 1),  # exposed left foot cap
            (67, 105): (0, 0, 1),  # exposed central foot cap
            (90, 32): (0, 1, 0),  # header facade
            (69, 87): (0, 1, 0),  # arch pier facade
            (54, 35): (-1, 0, 0),  # header west end
            (35, 71): (-1, 0, 0),  # left pier west return
            (56, 82): (-1, 0, 0),  # central pier west return
            (23, 41): (-1, 0, 0),  # feeder channel exterior
            (29, 34): (1, 0, 0),  # feeder channel inner wall
            (138, 103): (0, 0, 1),  # outgoing chute bed
        }
        for xy, expected in points.items():
            with self.subTest(xy=xy, expected=expected):
                actual = upright_world(self.normal, xy)
                self.assertGreater(sum(a * b for a, b in zip(actual, expected)), 0.999)

        # The exact historical failure is a negative control: its apparent
        # cistern top points south, and would fail the semantic check above.
        historical = Image.open(landmarks.ARCHIVE / self.spec["normal"]).convert("RGBA")
        self.assertLess(upright_world(historical, (79, 19))[2], 0.25)

    def test_aperture_and_all_native_rgb_remain_exact(self):
        self.assertEqual(self.art.size, (160, 128))
        self.assertEqual(self.spec["anchor"], [80, 115])
        self.assertEqual(self.spec["support_depth"], 10)
        self.assertEqual(self.normal.getchannel("A").tobytes(), self.art.getchannel("A").tobytes())
        self.assertEqual(self.art.convert("RGB").tobytes(), self.source.convert("RGB").tobytes())
        self.assertEqual(self.art.getpixel((90, 85))[3], 0)
        self.assertEqual(self.normal.getpixel((90, 85))[3], 0)
        self.assertEqual(self.art.getpixel((69, 87))[3], 255)

    def test_inactive_exports_are_removed_without_touching_other_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sprites, normals = root / "sprites", root / "normals"
            sprites.mkdir()
            normals.mkdir()
            for key in ("garden", "press", "summit"):
                for folder, suffix in ((sprites, ".png"), (sprites, ".json"), (normals, ".png")):
                    (folder / ("round13_landmarks_" + key + suffix)).write_text("stale rejected output")
            for folder in (sprites, normals):
                (folder / "unrelated.png").write_text("untouched")
            with patch.object(sys, "argv", ["round13_landmarks.py", "--out", str(sprites)]):
                with contextlib.redirect_stdout(io.StringIO()):
                    landmarks.main()
            for key in ("garden", "press", "summit"):
                self.assertFalse((sprites / ("round13_landmarks_" + key + ".png")).exists())
                self.assertFalse((sprites / ("round13_landmarks_" + key + ".json")).exists())
                self.assertFalse((normals / ("round13_landmarks_" + key + ".png")).exists())
            for folder in (sprites, normals):
                self.assertEqual((folder / "unrelated.png").read_text(), "untouched")
            self.assertEqual((sprites / "round13_landmarks_distributor.png").read_bytes(),
                             (landmarks.ROOT / "assets/sprites/round13_landmarks_distributor.png").read_bytes())
            self.assertEqual((normals / "round13_landmarks_distributor.png").read_bytes(),
                             (landmarks.ROOT / "assets/normals/round13_landmarks_distributor.png").read_bytes())


if __name__ == "__main__":
    unittest.main()
