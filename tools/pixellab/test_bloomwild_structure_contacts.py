"""Native-source regression checks for scoped masonry ground-contact correction."""

from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image

import bloomwild_structure_contacts as contacts
import bloomwild_variety as variety
from build_assets import arch


class StructureContacts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.prepared = variety.prepare()

    def test_native_diverse_atlas_changes_only_authored_rgb_and_is_idempotent(self):
        sheet = "bloomwild_ruins"
        original = self.prepared.sprites[sheet]
        metadata = deepcopy(self.prepared.metadata[sheet])
        frozen_metadata = deepcopy(metadata)
        corrected, mask, report = contacts.correct_atlas(original, metadata, sheet)
        self.assertEqual(metadata, frozen_metadata)
        self.assertEqual(corrected.size, original.size)
        self.assertEqual(corrected.getchannel("A").tobytes(), original.getchannel("A").tobytes())
        self.assertEqual(report["changed_pixels"], 470)
        changed = 0
        for y in range(original.height):
            for x in range(original.width):
                before, after = original.getpixel((x, y)), corrected.getpixel((x, y))
                if before != after:
                    changed += 1
                    self.assertEqual(mask.getpixel((x, y)), 255)
                    self.assertEqual(before[3], after[3])
                elif not mask.getpixel((x, y)):
                    self.assertEqual(before, after)
        self.assertEqual(changed, 470)
        second, second_mask, again = contacts.correct_atlas(corrected, metadata, sheet)
        self.assertEqual(second.tobytes(), corrected.tobytes())
        self.assertEqual(second_mask.tobytes(), mask.tobytes())
        self.assertEqual(again["changed_pixels"], 0)
        # Preserve the visible upper tier and a real masonry undercut even
        # though both contain the same palette ink as the corrected supports.
        for name, xy in [("pillar_broken_short", (17, 47)), ("wall_low", (28, 49))]:
            x, y = xy
            y += metadata["anims"][name]["row"] * 56
            self.assertEqual(original.getpixel((x, y))[:3], contacts.INK)
            self.assertEqual(corrected.getpixel((x, y)), original.getpixel((x, y)))

    def test_original_and_appended_feet_remain_exact_shared_supports(self):
        metadata = self.prepared.metadata["bloomwild_ruins"]
        corrected, _, _ = contacts.correct_atlas(
            self.prepared.sprites["bloomwild_ruins"], metadata, "bloomwild_ruins")
        for template, first in [("pillar_broken_short", 40), ("wall_low", 45)]:
            reference_row = metadata["anims"][template]["row"]
            expected = corrected.crop((0, reference_row * 56 + first, 48, (reference_row + 1) * 56))
            for name, anim in metadata["anims"].items():
                if name.startswith(template + "_"):
                    row = anim["row"]
                    self.assertEqual(corrected.crop((0, row * 56 + first, 48, (row + 1) * 56)).tobytes(),
                                     expected.tobytes(), name)

    def test_arch_contact_changes_leave_overhead_span_and_upper_sides_exact(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td)
            arch(path)  # Only this scoped native packer; never the umbrella main.
            original = Image.open(path / "bloomwild_arch.png").convert("RGBA")
            metadata = json.loads((path / "bloomwild_arch.json").read_text())
            corrected, _, report = contacts.correct_atlas(original, metadata, "bloomwild_arch")
            self.assertEqual(report["changed_pixels"], 32)
            self.assertEqual(corrected.crop((0, 0, 64, 58)).tobytes(),
                             original.crop((0, 0, 64, 58)).tobytes())
            self.assertEqual(corrected.crop((0, 72, 64, 144)).tobytes(),
                             original.crop((0, 72, 64, 144)).tobytes())
            self.assertEqual(corrected.crop((0, 144, 64, 202)).tobytes(),
                             original.crop((0, 144, 64, 202)).tobytes())

    def test_revised_support_is_rejected_before_any_atlas_is_written(self):
        with tempfile.TemporaryDirectory() as td:
            source = Path(td) / "source"
            source.mkdir()
            sheet = "bloomwild_ruins"
            changed = self.prepared.sprites[sheet].copy()
            row = self.prepared.metadata[sheet]["anims"]["pillar"]["row"]
            changed.putpixel((21, row * 56 + 51), (255, 0, 0, 255))
            changed.save(source / f"{sheet}.png")
            (source / f"{sheet}.json").write_text(json.dumps(self.prepared.metadata[sheet]))
            out = Path(td) / "out"
            with self.assertRaisesRegex(ValueError, "unexpected contact RGB"):
                contacts.export_contacts(source, out, [sheet])
            self.assertFalse(out.exists())


if __name__ == "__main__":
    unittest.main()
