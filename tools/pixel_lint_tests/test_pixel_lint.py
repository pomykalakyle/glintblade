#!/usr/bin/env python3
"""Exercise actual PNG/manifest inputs and the artists' public lint command."""

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SPEC = importlib.util.spec_from_file_location("pixel_lint", ROOT / "tools/pixel_lint.py")
LINT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LINT)


def save_sheet(name, image, manifest):
    """Write an inspectable fixture PNG and its adjacent engine-style manifest."""
    path = HERE / f"{name}.png"
    image.save(path)
    path.with_suffix(".json").write_text(json.dumps(manifest, indent=2) + "\n")
    return path


def make_fixtures():
    """Build deterministic good and bad inputs from explicit pixel patterns."""
    HERE.mkdir(parents=True, exist_ok=True)
    clean = Image.new("RGBA", (32, 32))
    draw = ImageDraw.Draw(clean)
    draw.polygon([(13, 3), (20, 6), (25, 18), (22, 27), (8, 26), (6, 16)], fill="#0a0a24")
    draw.polygon([(13, 5), (19, 7), (23, 18), (20, 24), (9, 24), (8, 16)], fill="#2c3a9a")
    draw.polygon([(19, 9), (23, 18), (20, 24), (17, 23)], fill="#1d2266")
    draw.point((13, 12), fill="#8ff8ff")
    manifest = {"cell": [32, 32], "anchor": [16, 27], "emissive": ["#8ff8ff"],
                "anims": {"idle": {"row": 0, "frames": 1, "fps": 6}}}
    save_sheet("clean", clean, manifest)

    bad = Image.new("RGBA", (96, 32))
    draw = ImageDraw.Draw(bad)
    # Cell 0: edge alpha, near-duplicate off-palette paints, detached noise.
    draw.rectangle((5, 4, 26, 26), fill=(90, 80, 120, 255))
    draw.rectangle((17, 5, 25, 25), fill=(91, 80, 120, 255))
    draw.point((4, 4), fill=(90, 80, 120, 100))
    for xy in ((1, 1), (1, 7), (1, 13), (1, 19), (1, 25), (1, 30)):
        draw.point(xy, fill=(250, 1, 7, 255))
    for xy in ((8, 8), (10, 8), (8, 10)):
        draw.point(xy, fill=(250, 1, 7, 255))
    # Cell 1: an opaque interpolated gradient with hundreds of unique colors.
    for y in range(4, 28):
        for x in range(4, 28):
            bad.putpixel((32 + x, y), (30 + x * 5, 40 + y * 5, 113, 255))
    # Cell 2: repeated 2x staircase windows next to genuine 1px details.
    for y in range(3, 27):
        for x in range(3, 27):
            color = "#2c3a9a" if (x - 3) // 2 <= (y - 3) // 2 else "#1d2266"
            draw.point((64 + x, y), fill=color)
    draw.point((65, 1), fill="#8ff8ff")
    save_sheet("deliberately_bad", bad, {"cell": [32, 32], "anchor": [16, 27], "emissive": [],
                                        "anims": {"idle": {"row": 0, "frames": 3, "fps": 6}}})
    hidden = np.array(clean)
    hidden[hidden[..., 3] == 0, :3] = [37, 222, 143]
    save_sheet("hidden_rgb", Image.fromarray(hidden), manifest)
    indexed = clean.convert("P", palette=Image.Palette.ADAPTIVE)
    save_sheet("indexed", indexed, manifest)
    save_sheet("upscaled", clean.resize((64, 64), Image.Resampling.NEAREST),
               {**manifest, "cell": [64, 64]})
    save_sheet("invalid_cell", clean, {"cell": [0, 32]})
    save_sheet("invalid_row", clean, {"cell": [32, 32], "anims": {"idle": {"row": -1}}})
    save_sheet("invalid_durations", clean, {"cell": [32, 32], "anims": {
        "idle": {"row": 0, "frames": 1, "durations": [0]}}})
    save_sheet("outside", clean, {"cell": [32, 32], "anims": {
        "idle": {"row": 0, "frames": 2}}})
    # A noise image large enough to defeat an unbounded colors-by-colors array.
    yy, xx = np.indices((128, 128))
    noise = np.stack([xx * 2, yy * 2, (xx + yy) % 256, np.full_like(xx, 255)], axis=-1).astype(np.uint8)
    save_sheet("many_colors", Image.fromarray(noise), {"cell": [128, 128]})
    frame = Image.new("RGB", (384, 216), "#1d2266")
    frame.save(HERE / "frame_native.png")
    frame.resize((768, 432), Image.Resampling.NEAREST).save(HERE / "frame_clean.png")
    broken = frame.resize((768, 432), Image.Resampling.NEAREST)
    broken.putpixel((15, 15), (255, 0, 0))
    broken.save(HERE / "frame_broken.png")


def run_cli(*args, cwd=ROOT):
    """Run the same Python entry point artists use and parse its JSON reports."""
    result = subprocess.run([sys.executable, str(ROOT / "tools/pixel_lint.py"),
                             *map(str, args), "--json"], cwd=cwd, text=True, capture_output=True, timeout=45)
    return result.returncode, json.loads(result.stdout)


class PixelLintTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        """Generate real fixture files once for the suite and manual inspection."""
        make_fixtures()
        cls.palette = LINT.Palette(LINT.DEFAULT_PALETTE)

    def lint(self, name):
        """Lint a fixture using its real adjacent JSON manifest."""
        path = HERE / f"{name}.png"
        manifest = json.loads(path.with_suffix(".json").read_text())
        return LINT.lint_sprite(path, manifest, self.palette)

    def test_bad_sheet_exposes_required_defects_and_coordinates(self):
        """Detect every requested measurable defect in the deliberately bad sheet."""
        code, reports = run_cli(HERE / "deliberately_bad.png", "--palette", LINT.DEFAULT_PALETTE)
        self.assertEqual(code, 1)
        report = reports[0]
        issues = {i["check"]: i for i in report["issues"] if i["severity"] != "info"}
        self.assertTrue({"alpha", "colors", "near-dup", "palette", "specks", "staircase"} <= issues.keys())
        for key in ("alpha", "near-dup", "palette", "specks", "staircase"):
            self.assertTrue(any("cell(" in e and "pixel(" in e for e in issues[key]["examples"]), key)
        self.assertLess(report["score"], 60)

    def test_clean_and_hidden_rgb_do_not_get_penalties(self):
        """Allow a deliberate eye accent and ignore hidden RGB under alpha zero."""
        clean = self.lint("clean")
        hidden = self.lint("hidden_rgb")
        indexed = self.lint("indexed")
        self.assertEqual([clean.score, hidden.score, indexed.score], [100, 100, 100])
        self.assertEqual(clean.stats["colors"], hidden.stats["colors"])
        self.assertEqual(clean.stats["colors"], indexed.stats["colors"])
        path = save_sheet("empty", Image.new("RGBA", (32, 32)), {"cell": [32, 32]})
        empty = LINT.lint_sprite(path, {"cell": [32, 32]}, self.palette)
        self.assertTrue(empty.has_error)
        self.assertFalse(any(i["check"] == "crash" for i in empty.issues))

    def test_scale_is_a_review_warning_and_rectangles_are_not_errors(self):
        """Distinguish detectable repetition from proof of incorrect pixel density."""
        report = self.lint("upscaled")
        self.assertTrue(any(i["check"] == "scale" and i["severity"] == "WARN" for i in report.issues))
        self.assertFalse(report.has_error)
        path = save_sheet("solid", Image.new("RGBA", (16, 16), "#2c3a9a"), {"cell": [16, 16]})
        self.assertFalse(LINT.lint_sprite(path, {"cell": [16, 16]}, self.palette).has_error)

    def test_invalid_manifests_do_not_hide_later_files(self):
        """Report actionable input failures while continuing a multi-file command."""
        code, reports = run_cli(*[HERE / f"{name}.png" for name in
                                 ("invalid_cell", "invalid_row", "invalid_durations", "outside", "clean")])
        self.assertEqual(code, 1)
        self.assertEqual(len(reports), 5)
        self.assertTrue(all(any(i["severity"] == "ERROR" for i in r["issues"]) for r in reports[:4]))
        self.assertFalse(any(i["check"] == "crash" for r in reports for i in r["issues"]))
        self.assertEqual(reports[-1]["score"], 100)

    def test_palette_exceptions_require_notes(self):
        """Do not silently exempt a custom color just because it is emissive."""
        path = save_sheet("custom", Image.new("RGBA", (13, 11), "#fa0107"), {"cell": [13, 11]})
        declared = LINT.lint_sprite(path, {"cell": [13, 11], "emissive": ["#fa0107"]}, self.palette)
        noted = LINT.lint_sprite(path, {"cell": [13, 11], "notes": "Custom paint #fa0107."}, self.palette)
        self.assertTrue(any(i["check"] == "palette" and i["severity"] == "WARN" for i in declared.issues))
        self.assertFalse(any(i["check"] == "palette" and i["severity"] == "WARN" for i in noted.issues))

    def test_many_colors_is_bounded_and_still_measures_palette(self):
        """Handle 16,384 unique visible colors without a quadratic full matrix."""
        code, reports = run_cli(HERE / "many_colors.png")
        self.assertEqual(code, 0)
        self.assertEqual(reports[0]["stats"]["colors"], 16384)
        self.assertTrue(any(i["check"] == "palette" for i in reports[0]["issues"]))
        self.assertTrue(any("512 most frequent" in i["message"] for i in reports[0]["issues"]))

    def test_frame_grid_and_composition_are_distinguished(self):
        """Keep composition unscored while exposing a real display-block defect."""
        code, reports = run_cli(HERE / "frame_native.png", HERE / "frame_clean.png", HERE / "frame_broken.png")
        self.assertEqual(code, 0)
        self.assertEqual([r["score"] for r in reports[:2]], [100, 100])
        self.assertTrue(any(i["check"] == "grid" for i in reports[2]["issues"]))
        self.assertEqual(reports[2]["kind"], "frame")

    def test_real_sprites_cli_and_directory_discovery(self):
        """Check the complete live sprite directory with the preserved CLI."""
        code, reports = run_cli(ROOT / "assets/sprites", cwd=HERE)
        self.assertEqual(code, 0)
        self.assertGreaterEqual(len(reports), 30)
        self.assertFalse(any(i["check"] == "crash" for r in reports for i in r["issues"]))


if __name__ == "__main__":
    unittest.main(verbosity=2)
