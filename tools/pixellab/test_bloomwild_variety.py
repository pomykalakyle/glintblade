"""Native export invariants for Bloomwild diversity, with no paid/API sources."""

from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image, ImageDraw

import bloomwild_variety as variety
from native_collision import add_collision


class BloomwildVariety(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.archive = self.root / "archive"
        self.originals = {}
        self.metadata = {}
        for name, size, cell, anchor in (
            ("bloomwild_grass", (256, 32), (32, 32), (16, 28)),
            ("bloomwild_ruins", (48, 504), (48, 56), (24, 53)),
        ):
            art = Image.new("RGBA", size)
            normal = Image.new("RGBA", size)
            if name.endswith("grass"):
                for i in range(8):
                    # Every original frame is distinguishable, including its
                    # transparent RGB, which must survive ordinary atlas paste.
                    frame = Image.new("RGBA", cell, (i, 8, 15, 0))
                    ImageDraw.Draw(frame).polygon([(16, 28), (3 + i, 8), (28, 12)],
                                                  fill=(25 + i, 90, 50, 255))
                    art.paste(frame, (i * 32, 0))
                    n = Image.new("RGBA", cell, (128, 128, 255, 0))
                    n.putalpha(frame.getchannel("A"))
                    normal.paste(n, (i * 32, 0))
                anims = {"body": {"row": 0, "frames": 8, "fps": 1, "footprint": [0, 0]}}
            else:
                anims = {}
                names = ["pillar", "pillar_broken_tall", "pillar_broken_short", "drum",
                         "wall_low", "wall_tall", "statue", "pillar_b", "pillar_c"]
                for row, key in enumerate(names):
                    frame = Image.new("RGBA", cell, (row, 12, 18, 0))
                    draw = ImageDraw.Draw(frame)
                    draw.rectangle((12, 18, 35, 44), fill=(110 + row, 100, 80, 255))
                    if key == "wall_low":
                        draw.rectangle((3, 45, 44, 49), fill=(90, 85, 70, 255))
                        draw.rectangle((5, 50, 30, 52), fill=(80, 75, 60, 255))
                    else:
                        draw.rectangle((5, 44, 41, 48), fill=(90, 85, 70, 255))
                        draw.rectangle((10, 49, 36, 53), fill=(80, 75, 60, 255))
                    art.paste(frame, (0, row * 56))
                    n = Image.new("RGBA", cell, (150, 110, 252, 0))
                    n.putalpha(frame.getchannel("A"))
                    normal.paste(n, (0, row * 56))
                    anim = {"row": row, "frames": 1, "fps": 1, "custom": "retain me"}
                    add_collision(anim, frame, 10 if key.startswith("pillar") else 8)
                    anims[key] = anim
            data = {"cell": list(cell), "anchor": list(anchor), "anims": anims,
                    "notes": "Synthetic frozen baseline"}
            for folder, source in (("sprites", art), ("normals", normal)):
                path = self.archive / "baseline" / folder / f"{name}.png"
                path.parent.mkdir(parents=True, exist_ok=True)
                source.save(path)
            (self.archive / "baseline/sprites" / f"{name}.json").write_text(json.dumps(data))
            self.originals[name] = (art, normal)
            self.metadata[name] = data
        self.selection = {"leaves": [], "columns": [], "walls": []}

    def candidate(self, family, name, points, offset=(0, 0), preserve=None):
        size = (32, 32) if family == "leaves" else (48, 56)
        art = Image.new("RGBA", size)
        for xy, color in points:
            art.putpixel(xy, color)
        normal = Image.new("RGBA", size, (128, 128, 255, 255))
        for i, (xy, _) in enumerate(points):
            normal.putpixel(xy, (140 + i, 120, 254, 255))
        for folder, source in (("raw", art), ("normals", normal)):
            path = self.archive / folder / f"{name}.png"
            path.parent.mkdir(parents=True, exist_ok=True)
            source.save(path)
        spec = {"name": name, "image": f"raw/{name}.png", "normal": f"normals/{name}.png",
                "offset": list(offset)}
        if preserve is not None:
            spec["preserve_from_row"] = preserve
        self.selection[family].append(spec)
        return art, normal

    def test_integer_translation_keeps_art_and_directional_normals_registered(self):
        art, normal = self.candidate("leaves", "open", [
            ((10, 12), (33, 111, 55, 255)), ((15, 23), (40, 123, 65, 255))], offset=(3, -2))
        result = variety.prepare(self.archive, self.selection)
        exported = result.sprites["bloomwild_grass"]
        normals = result.normals["bloomwild_grass"]
        for source, target in (((10, 12), (13, 10)), ((15, 23), (18, 21))):
            self.assertEqual(exported.getpixel((256 + target[0], target[1])), art.getpixel(source))
            self.assertEqual(normals.getpixel((256 + target[0], target[1])), normal.getpixel(source))
        cell_art = exported.crop((256, 0, 288, 32))
        cell_normal = normals.crop((256, 0, 288, 32))
        self.assertEqual(cell_art.getchannel("A").tobytes(), cell_normal.getchannel("A").tobytes())
        self.assertEqual(cell_normal.getpixel((0, 0))[3], 0)  # extra source normal alpha removed
        self.assertEqual(result.metadata["bloomwild_grass"]["anchor"], [16, 28])
        self.assertEqual(result.metadata["bloomwild_grass"]["anims"]["body"]["footprint"], [0, 0])

    def test_clipping_rejected_before_any_outputs_are_written(self):
        self.candidate("leaves", "would_crop", [((0, 20), (40, 120, 60, 255))], offset=(-1, 0))
        out = self.root / "candidate/sprites"
        with self.assertRaisesRegex(ValueError, "clip opaque"):
            variety.build(self.archive, out, selection=self.selection)
        self.assertFalse(out.exists())

    def test_stone_feet_collision_and_every_old_pixel_and_animation_are_preserved(self):
        self.candidate("columns", "pillar_broken_short_b", [
            ((20, 22), (130, 112, 85, 255)), ((21, 50), (255, 0, 0, 255))], preserve=40)
        self.candidate("walls", "wall_low_b", [
            ((5, 30), (124, 105, 80, 255)), ((2, 50), (255, 0, 0, 255))], preserve=45)
        self.candidate("leaves", "curl", [((17, 27), (35, 110, 65, 255))])
        result = variety.prepare(self.archive, self.selection)
        self.assertEqual(result.sprites["bloomwild_grass"].size, (288, 32))
        self.assertEqual(result.sprites["bloomwild_ruins"].size, (48, 616))
        self.assertEqual(result.normals["bloomwild_ruins"].size, (48, 616))
        for sheet, (art, normal) in self.originals.items():
            self.assertEqual(result.sprites[sheet].crop((0, 0, *art.size)).tobytes(), art.tobytes())
            self.assertEqual(result.normals[sheet].crop((0, 0, *normal.size)).tobytes(), normal.tobytes())
            for name, anim in self.metadata[sheet]["anims"].items():
                expected = deepcopy(anim)
                if sheet.endswith("grass"):
                    expected["frames"] = 9
                self.assertEqual(result.metadata[sheet]["anims"][name], expected)
        stone = "bloomwild_ruins"
        for name, template, row, keep in (
            ("pillar_broken_short_b", "pillar_broken_short", 9, 40),
            ("wall_low_b", "wall_low", 10, 45),
        ):
            baseline = self.metadata[stone]["anims"][template]
            anim = result.metadata[stone]["anims"][name]
            self.assertEqual(anim["row"], row)
            self.assertEqual(anim["frames"], 1)
            self.assertEqual(anim["collision_rects"], baseline["collision_rects"])
            for outputs, old in ((result.sprites, self.originals[stone][0]),
                                 (result.normals, self.originals[stone][1])):
                self.assertEqual(outputs[stone].crop((0, row * 56 + keep, 48, (row + 1) * 56)).tobytes(),
                                 old.crop((0, baseline["row"] * 56 + keep, 48, (baseline["row"] + 1) * 56)).tobytes())
            # Collider rectangles must cover occupied art only, including each
            # pixel at contacts, rather than an approximate bounding rectangle.
            image = result.sprites[stone].crop((0, row * 56, 48, (row + 1) * 56))
            for x0, y0, x1, y1 in anim["collision_rects"]:
                self.assertTrue(0 <= x0 < x1 <= 48 and 0 <= y0 < y1 <= 56)
                for y in range(y0, y1):
                    for x in range(x0, x1):
                        self.assertEqual(image.getpixel((x, y))[3], 255)

    def test_build_is_reproducible_and_does_not_append_to_previous_export(self):
        self.candidate("leaves", "open", [((9, 18), (35, 111, 55, 255))])
        out, normals = self.root / "export/art", self.root / "export/lighting"
        audit = self.root / "archive/audit/export.json"
        first = variety.build(self.archive, out, normals, self.selection, audit)
        first_audit = audit.read_bytes()
        second = variety.build(self.archive, out, normals, self.selection, audit)
        self.assertEqual(first, second)
        self.assertEqual(first_audit, audit.read_bytes())
        self.assertEqual(len(first["output_sha256"]), 6)
        for key, digest in first["output_sha256"].items():
            family, filename = key.split("/")
            path = (out if family == "sprites" else normals) / filename
            self.assertEqual(variety.sha256(path.read_bytes()), digest)
        with Image.open(out / "bloomwild_grass.png") as exported:
            self.assertEqual(exported.size, (288, 32))
        self.assertIn("raw/open.png", first["source_sha256"])
        pairs = first["families"]["leaves"]["diversity"]
        self.assertTrue(any(p["b"] == "open" and p["visible_pixel_change_fraction"] > 0 for p in pairs))

    def test_missing_opaque_normals_and_backfacing_normals_are_rejected(self):
        self.candidate("leaves", "bad", [((16, 20), (35, 111, 55, 255))])
        path = self.archive / "normals/bad.png"
        normal = Image.open(path).convert("RGBA")
        normal.putpixel((16, 20), (128, 128, 255, 0))
        normal.save(path)
        with self.assertRaisesRegex(ValueError, "no source normal"):
            variety.prepare(self.archive, self.selection)
        normal.putpixel((16, 20), (128, 128, 0, 255))
        normal.save(path)
        with self.assertRaisesRegex(ValueError, "front hemisphere"):
            variety.prepare(self.archive, self.selection)

    def test_source_dimensions_and_noninteger_offsets_rejected(self):
        self.candidate("leaves", "bad", [((16, 20), (35, 111, 55, 255))])
        self.selection["leaves"][0]["offset"] = [0.5, 0]
        with self.assertRaisesRegex(ValueError, "two integers"):
            variety.prepare(self.archive, self.selection)
        self.selection["leaves"][0]["offset"] = [0, 0]
        Image.new("RGBA", (64, 64), (100, 100, 100, 255)).save(self.archive / "raw/bad.png")
        with self.assertRaisesRegex(ValueError, "native sources must be"):
            variety.prepare(self.archive, self.selection)

    def semantic_candidate(self, offset=(0, 0)):
        name = "pillar_broken_short_b"
        self.candidate("columns", name, [
            ((17, 20), (135, 115, 85, 255)), ((20, 23), (110, 90, 75, 255))],
            offset=offset, preserve=40)
        normal_path = self.archive / "normals" / f"{name}.png"
        normal = Image.open(normal_path).convert("RGBA")
        # In the upright engine basis green=255 is a top, blue=255 a front.
        # Both are valid unit tangent normals; a transport-only check cannot
        # catch swapping these geometric faces.
        normal.putpixel((17, 20), (128, 255, 128, 255))
        normal.putpixel((20, 23), (128, 128, 255, 255))
        normal.save(normal_path)
        planes = {
            "source_sha256": variety.sha256((self.archive / "raw" / f"{name}.png").read_bytes()),
            "planes": [
                {"name": "upward broken cap", "world_normal": [0, 0, 1],
                 "samples": [{"xy": [17, 20], "expected_world_normal": [0, 0, 1]}]},
                {"name": "front shaft face", "world_normal": [0, 1, 0],
                 "samples": [{"xy": [20, 23], "expected_world_normal": [0, 1, 0]}]},
            ],
        }
        relative = f"normal-planes/{name}.json"
        path = self.archive / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(planes))
        self.selection["columns"][0]["normal_planes"] = relative
        return path, planes

    def test_semantic_stone_normals_use_upright_basis_and_translated_native_samples(self):
        path, _ = self.semantic_candidate(offset=(2, -1))
        result = variety.prepare(self.archive, self.selection)
        anim = result.metadata["bloomwild_ruins"]["anims"]["pillar_broken_short_b"]
        self.assertEqual(anim["normal_samples"], [
            {"xy": [19, 19], "world_normal": [0, 0, 1], "plane": "upward broken cap"},
            {"xy": [22, 22], "world_normal": [0, 1, 0], "plane": "front shaft face"},
        ])
        relative = path.relative_to(self.archive).as_posix()
        digest = variety.sha256(path.read_bytes())
        entry = result.report["families"]["columns"]["variants"][0]
        self.assertEqual(entry["normal_planes"], relative)
        self.assertEqual(entry["normal_planes_sha256"], digest)
        self.assertEqual(entry["normal_sample_count"], 2)
        self.assertGreater(entry["normal_sample_minimum_dot"], 0.999)
        self.assertEqual(result.report["source_sha256"][relative], digest)
        # Authoring a semantic plane file does not change the already prepared
        # source pixels, or alter the retained contact normals below row40.
        source = Image.open(self.archive / "normals/pillar_broken_short_b.png").convert("RGBA")
        self.assertEqual(result.normals["bloomwild_ruins"].getpixel((19, 9 * 56 + 19)),
                         source.getpixel((17, 20)))
        original = self.originals["bloomwild_ruins"][1]
        self.assertEqual(result.normals["bloomwild_ruins"].crop((0, 9 * 56 + 40, 48, 10 * 56)).tobytes(),
                         original.crop((0, 2 * 56 + 40, 48, 3 * 56)).tobytes())

    def test_semantic_sample_rejects_valid_transport_with_wrong_face_direction(self):
        path, planes = self.semantic_candidate()
        planes["planes"][0]["samples"][0]["expected_world_normal"] = [0, 1, 0]
        path.write_text(json.dumps(planes))
        with self.assertRaisesRegex(ValueError, "semantic normal sample.*differs from expected plane"):
            variety.prepare(self.archive, self.selection)

    def test_semantic_planes_require_the_exact_native_art_source_hash(self):
        path, planes = self.semantic_candidate()
        planes["source_sha256"] = "0" * 64
        path.write_text(json.dumps(planes))
        with self.assertRaisesRegex(ValueError, "normal-plane source SHA256 does not match"):
            variety.prepare(self.archive, self.selection)

    def shared_foot_profile(self, template="pillar_broken_short"):
        row = self.metadata["bloomwild_ruins"]["anims"][template]["row"]
        reference = self.originals["bloomwild_ruins"][0].crop((0, row * 56, 48, (row + 1) * 56))
        reference_path = self.archive / "references" / f"{template}.png"
        reference_path.parent.mkdir(parents=True, exist_ok=True)
        reference.save(reference_path)
        first_row = 40 if template == "pillar_broken_short" else 45
        polygon = [[2, 45], [45, 45], [45, 54], [2, 54]]
        profile = {"source_sha256": variety.sha256(reference_path.read_bytes()), "planes": [
            {"name": "vertical support front", "world_normal": [0, 1, 0], "polygons": [polygon],
             "samples": [{"xy": [20, 50] if first_row == 40 else [10, 48],
                          "expected_world_normal": [0, 1, 0]}]},
        ]}
        plane_path = self.archive / "normal-planes" / f"{template}-foot.json"
        plane_path.parent.mkdir(parents=True, exist_ok=True)
        plane_path.write_text(json.dumps(profile))
        spec = {"template": template, "image": reference_path.relative_to(self.archive).as_posix(),
                "first_row": first_row, "normal_planes": plane_path.relative_to(self.archive).as_posix()}
        self.selection.setdefault("shared_feet", []).append(spec)
        return spec, plane_path, profile

    def test_shared_feet_correct_only_explicit_original_support_normals_and_variant_contacts(self):
        self.shared_foot_profile()
        self.shared_foot_profile("wall_low")
        self.candidate("columns", "pillar_broken_short_b", [((18, 25), (130, 110, 80, 255))], preserve=40)
        self.candidate("walls", "wall_low_b", [((15, 28), (120, 100, 75, 255))], preserve=45)
        result = variety.prepare(self.archive, self.selection)
        sheet = "bloomwild_ruins"
        original_art, original_normals = self.originals[sheet]
        self.assertEqual(result.sprites[sheet].crop((0, 0, 48, 504)).tobytes(), original_art.tobytes())
        self.assertEqual(result.normals[sheet].getchannel("A").crop((0, 0, 48, 504)).tobytes(),
                         original_normals.getchannel("A").tobytes())
        self.assertEqual(result.metadata[sheet]["anims"] | {},
                         self.metadata[sheet]["anims"] | {
                             name: result.metadata[sheet]["anims"][name]
                             for name in ("pillar_broken_short_b", "wall_low_b")})
        changed = 0
        for y in range(504):
            for x in range(48):
                old = original_normals.getpixel((x, y))
                new = result.normals[sheet].getpixel((x, y))
                if old != new:
                    changed += 1
                    self.assertIn(y // 56, (2, 4))
                    self.assertTrue(2 <= x <= 45 and 45 <= y % 56 <= 54)
                    self.assertEqual(old[3], 255)
                if old[3] == 0:
                    self.assertEqual(old, new)  # even transparent RGB stays exact
        self.assertGreater(changed, 0)
        self.assertEqual(changed, result.report["legacy_normal_changed_pixels"])
        self.assertFalse(result.report["legacy_pixels_preserved"])
        self.assertTrue(result.report["legacy_albedo_pixels_preserved"])
        self.assertTrue(result.report["legacy_normal_pixels_preserved_except_shared_feet"])
        for row, variant_row, keep in ((2, 9, 40), (4, 10, 45)):
            self.assertEqual(result.normals[sheet].crop((0, row * 56 + keep, 48, (row + 1) * 56)).tobytes(),
                             result.normals[sheet].crop((0, variant_row * 56 + keep, 48, (variant_row + 1) * 56)).tobytes())
        foot_metadata = result.metadata[sheet]["shared_foot_normals"]
        self.assertEqual(len(foot_metadata), 2)
        self.assertEqual(foot_metadata[0]["samples"], [
            {"xy": [20, 50], "world_normal": [0, 1, 0], "plane": "vertical support front"}])
        self.assertIn("normal_planes_sha256", foot_metadata[0])

    def test_shared_foot_profile_rejects_bad_source_hash(self):
        _, path, profile = self.shared_foot_profile()
        profile["source_sha256"] = "0" * 64
        path.write_text(json.dumps(profile))
        with self.assertRaisesRegex(ValueError, "shared foot source SHA256"):
            variety.prepare(self.archive, self.selection)

    def test_shared_foot_profile_rejects_polygons_and_samples_outside_permitted_foot(self):
        _, path, profile = self.shared_foot_profile()
        for xy in ([2, 39], [48, 45], [2, 56]):
            with self.subTest(xy=xy):
                invalid = deepcopy(profile)
                invalid["planes"][0]["polygons"][0][0] = xy
                path.write_text(json.dumps(invalid))
                with self.assertRaisesRegex(ValueError, "permitted native foot bounds"):
                    variety.prepare(self.archive, self.selection)
        invalid = deepcopy(profile)
        invalid["planes"][0]["samples"][0]["xy"] = [20, 39]
        path.write_text(json.dumps(invalid))
        with self.assertRaisesRegex(ValueError, "sample exceeds authored native foot polygon"):
            variety.prepare(self.archive, self.selection)

    def test_shared_foot_profiles_cannot_target_other_families_or_start_rows(self):
        spec, _, _ = self.shared_foot_profile()
        for template, row in (("pillar", 40), ("pillar_broken_tall", 40), ("pillar_broken_short", 39), ("wall_low", 40)):
            with self.subTest(template=template, row=row):
                spec["template"], spec["first_row"] = template, row
                with self.assertRaisesRegex(ValueError, "established first row"):
                    variety.prepare(self.archive, self.selection)

    def test_baseline_hashes_guard_frozen_source_provenance(self):
        hashes = {"sprites/bloomwild_grass.png": "0" * 64}
        (self.archive / "baseline/hashes.json").write_text(json.dumps(hashes))
        with self.assertRaisesRegex(ValueError, "baseline checksum mismatch"):
            variety.prepare(self.archive, self.selection)

    def test_empty_selection_can_be_prepared_without_selection_file(self):
        result = variety.prepare(self.archive, {})
        self.assertEqual(result.sprites["bloomwild_grass"].size, (256, 32))
        self.assertEqual(result.sprites["bloomwild_ruins"].size, (48, 504))
        self.assertEqual(result.metadata, self.metadata)


if __name__ == "__main__":
    unittest.main()
