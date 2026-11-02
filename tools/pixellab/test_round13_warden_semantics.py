"""Geometric checks selected against the enlarged registered Warden albedo.

These are physical face/lighting expectations, not tests of the polygon recipe.
The foot fronts and lintel must face south; exposed roofs face up. In particular,
normalizing a generated color patch can satisfy alpha/unit checks and still fail
all of these orientation checks.
"""
from __future__ import annotations

import importlib.util
import math
from pathlib import Path
import tempfile
import unittest

from PIL import Image

MODULE = Path(__file__).with_name("round13_warden.py")
spec = importlib.util.spec_from_file_location("round13_warden_semantics", MODULE)
warden = importlib.util.module_from_spec(spec)
spec.loader.exec_module(warden)


def world(rgb):
    # Independent copy of the runtime's upright transform, X,Z,Y.
    v = [rgb[0]/127.5-1, rgb[2]/127.5-1, rgb[1]/127.5-1]
    length = math.sqrt(sum(a*a for a in v))
    return [a/length for a in v]


class WardenSemanticNormals(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.bases = warden.registered()
        cls.normals = {name: warden.semantic_normal(art, name)
                       for name, art in cls.bases.items()}
        for pose in ("brace", "opening"):
            cls.normals[pose] = warden.stabilize(
                cls.bases[pose], cls.normals[pose], cls.bases["awake"], cls.normals["awake"])

    def assertFaces(self, normal, xy, expected, label):
        pixel = normal.getpixel(xy)
        self.assertEqual(pixel[3], 255, label)
        dot = sum(a*b for a, b in zip(world(pixel), expected))
        self.assertGreater(dot, .999, f"{label} {xy}: {world(pixel)} vs {expected}")

    def test_reported_bad_vertical_stone_faces_are_vertical_in_all_poses(self):
        # Chosen from the upright rectangular fronts on the actual albedo. The
        # original generated maps turned these towards the ground by up to 49°.
        for pose, normal in self.normals.items():
            for xy in ((45,103), (78,93), (34,105), (79,105), (60,28), (22,28)):
                with self.subTest(pose=pose, xy=xy):
                    self.assertFaces(normal, xy, (0,1,0), "vertical stone front")

    def test_caps_pier_roofs_and_returns_have_distinct_physical_directions(self):
        for pose, normal in self.normals.items():
            for xy in ((24,5), (90,16), (35,87), (79,88)):
                with self.subTest(pose=pose, xy=xy):
                    self.assertFaces(normal, xy, (0,0,1), "exposed horizontal roof")
            self.assertFaces(normal, (33,18), (.8,.6,0), "tower east return")
        for pose in ("awake", "opening"):
            self.assertFaces(self.normals[pose], (101,57), (.8,.6,0), "buttress east return")

    def test_lights_separate_roofs_walls_and_side_returns(self):
        # A light straight overhead illuminates a roof, not an upright foot
        # front. Front light does the reverse; right light sees the east return.
        for normal in self.normals.values():
            front = world(normal.getpixel((34,105)))
            roof = world(normal.getpixel((35,87)))
            side = world(normal.getpixel((33,18)))
            self.assertLess(abs(front[2]), .01)
            self.assertGreater(front[1], .99)
            self.assertGreater(roof[2], .99)
            self.assertLess(abs(roof[1]), .01)
            self.assertGreater(side[0], .79)
            self.assertLess(abs(side[2]), .01)

    def test_rigid_pier_planes_follow_all_seventeen_fold_depths(self):
        for pose, normal in self.normals.items():
            for depth in range(17):
                folded = warden.folded(normal, depth)
                for source in ((34,105), (79,105), (35,87), (79,88)):
                    # The native recipe uses symmetric integer 0..2 splay.
                    spread = -(depth//8) if source[0] < 60 else depth//8
                    dest = (source[0]+spread, source[1]-depth)
                    self.assertEqual(folded.getpixel(dest), normal.getpixel(source),
                                     (pose, depth, source, dest))
                self.assertEqual(folded.crop((0,0,120,63)).tobytes(),
                                 normal.crop((0,0,120,63)).tobytes())

    def test_damage_and_registration_keep_exact_alpha_and_fixed_attachments(self):
        for pose, normal in self.normals.items():
            self.assertEqual(normal.getchannel("A").tobytes(),
                             self.bases[pose].getchannel("A").tobytes())
            damaged = warden.alpha_like(normal, warden.damage(self.bases[pose]))
            self.assertEqual(damaged.getchannel("A").tobytes(),
                             warden.damage(self.bases[pose]).getchannel("A").tobytes())
            self.assertEqual(normal.crop((0,0,120,30)).tobytes(),
                             self.normals["awake"].crop((0,0,120,30)).tobytes())

    def test_export_preserves_production_albedo_metadata_and_all_frame_alpha(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            warden.build(tmp/"sprites", tmp/"normals")
            for suffix in ("png", "json"):
                self.assertEqual((tmp/"sprites"/f"warden_body.{suffix}").read_bytes(),
                                 (warden.ROOT/"assets/sprites"/f"warden_body.{suffix}").read_bytes())
            art = Image.open(tmp/"sprites/warden_body.png").convert("RGBA")
            normals = Image.open(tmp/"normals/warden_body.png").convert("RGBA")
            self.assertEqual(art.getchannel("A").tobytes(), normals.getchannel("A").tobytes())


if __name__ == "__main__":
    unittest.main()
