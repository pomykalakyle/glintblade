"""Regression checks for the exact inputs of reproducible review builds."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("showcase_capture", Path(__file__).with_name("capture.py"))
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)


class FrozenInputTests(unittest.TestCase):
    def test_normal_maps_are_hashed_and_copied_with_runtime_assets(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("// fixture\n")
            normal = root / "assets/normals/landmark.png"
            normal.parent.mkdir(parents=True)
            normal.write_bytes(b"native normal data")
            sprite = root / "assets/sprites/landmark.png"
            sprite.parent.mkdir(parents=True)
            sprite.write_bytes(b"native albedo data")
            with patch.object(capture, "ROOT", root), patch.dict(os.environ, {"GLINT_ASSETS": ""}):
                inputs = capture.input_files(root / "assets")
                self.assertIn("assets/normals/landmark.png", inputs)
                frozen = root / "review/source"
                state = capture.freeze_sources(frozen)
            key = "assets/normals/landmark.png"
            self.assertEqual((frozen / key).read_bytes(), normal.read_bytes())
            self.assertEqual(state["files_sha256"][key], capture.sha256(normal))

    def test_changed_normal_map_invalidates_an_in_progress_freeze(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("// fixture\n")
            normal = root / "assets/normals/landmark.png"
            normal.parent.mkdir(parents=True)
            normal.write_bytes(b"before")
            original_copy = capture.shutil.copy2

            def copy_then_change(source, dest):
                result = original_copy(source, dest)
                if source == normal:
                    normal.write_bytes(b"changed while capturing")
                return result

            with patch.object(capture, "ROOT", root), patch.dict(os.environ, {"GLINT_ASSETS": ""}), \
                    patch.object(capture.shutil, "copy2", side_effect=copy_then_change):
                with self.assertRaisesRegex(AssertionError, "runtime inputs changed"):
                    capture.freeze_sources(root / "review/source")


if __name__ == "__main__":
    unittest.main()
