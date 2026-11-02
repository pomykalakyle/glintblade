#!/usr/bin/env python3
"""Fault-injection regression tests for audit_round13, using independent tiny PNGs.

uv run --offline --with pillow python -m unittest discover -s tools/pixellab -p 'test_round13*.py'
"""
import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image

import audit_round13 as audit


class AuditContracts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.sprites = self.root / "assets/sprites"
        self.normals = self.root / "assets/normals"
        self.sprites.mkdir(parents=True)
        self.normals.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def export(self, name="stone", size=(8,8), cell=(8,8), anchor=(4,6), art=None, meta=None):
        if art is None:
            art = Image.new("RGBA", size)
            for y in range(2,7):
                for x in range(2,6):
                    art.putpixel((x,y),(80,100,120,255))
        if meta is None:
            meta = {"cell": list(cell), "anchor": list(anchor),
                    "anims": {"body": {"row": 0, "frames": 1, "fps": 1}}}
        path = self.sprites / f"{name}.png"
        art.save(path)
        path.with_suffix(".json").write_text(json.dumps(meta))
        return path, art, meta

    def inspect(self, path, strict=True, require=False):
        issues = []
        result = audit.audit_sheet(path, self.normals, strict, require, issues)
        return result, issues

    def codes(self, issues):
        return {i["code"] for i in issues}

    def normal(self, name, art):
        n = Image.new("RGBA", art.size, (128,128,255,0))
        n.putalpha(art.getchannel("A"))
        n.save(self.normals / f"{name}.png")
        return n

    def test_clean_native_prop_has_exact_feet_and_gutters(self):
        path, art, _ = self.export()
        self.normal("stone", art)
        result, issues = self.inspect(path, require=True)
        self.assertEqual(issues, [])
        self.assertEqual(result["frames"][0]["gutters_ltrb"], [2,2,2,1])
        self.assertEqual(result["frames"][0]["foot_columns"], [-1,-1,6,6,6,6,-1,-1])
        self.assertEqual(result["frames"][0]["bottom_minus_anchor"], 0)

    def test_nonbinary_alpha_is_a_failure_even_in_a_gutter(self):
        path, art, _ = self.export()
        art.putpixel((0,0),(255,255,255,1))
        art.save(path)
        _, issues = self.inspect(path)
        self.assertTrue({"binary_alpha", "transparent_gutter"} <= self.codes(issues))

    def test_strict_gutter_detects_cropped_base(self):
        path, art, _ = self.export()
        art.putpixel((4,7),(80,100,120,255))
        art.save(path)
        _, issues = self.inspect(path)
        self.assertIn("transparent_gutter", self.codes(issues))
        # Ground tiles may intentionally meet a cell boundary; do not impose
        # freestanding-prop geometry on an unrelated atlas.
        _, relaxed = self.inspect(path, strict=False)
        self.assertNotIn("transparent_gutter", self.codes(relaxed))

    def test_manifest_outside_actual_canvas_cannot_fall_back_silently(self):
        path, _, meta = self.export()
        meta["anims"]["body"].update(row=1, frames=2)
        path.with_suffix(".json").write_text(json.dumps(meta))
        _, issues = self.inspect(path)
        self.assertIn("animation_bounds", self.codes(issues))

    def test_duration_count_does_not_use_engine_fallback(self):
        path, _, meta = self.export()
        meta["anims"]["body"]["durations"] = [6,6]
        path.with_suffix(".json").write_text(json.dumps(meta))
        _, issues = self.inspect(path)
        self.assertIn("animation_durations", self.codes(issues))

    def test_collision_rect_floating_in_transparent_gutter_is_rejected(self):
        path, _, meta = self.export()
        meta["anims"]["body"]["collision_rects"] = [[0,0,2,2]]
        path.with_suffix(".json").write_text(json.dumps(meta))
        _, issues = self.inspect(path)
        self.assertIn("collision_rect_without_visible_support", self.codes(issues))

    def test_normal_shape_and_alpha_alignment_are_independent_checks(self):
        path, art, _ = self.export()
        n = self.normal("stone", art)
        n.putpixel((1,1),(128,128,255,255))
        n.save(self.normals / path.name)
        _, issues = self.inspect(path)
        self.assertIn("normal_alpha", self.codes(issues))
        n.crop((0,0,7,8)).save(self.normals / path.name)
        _, issues = self.inspect(path)
        self.assertIn("normal_dimensions", self.codes(issues))

    def test_normal_nonunit_and_backward_vectors_are_rejected(self):
        path, art, _ = self.export()
        n = self.normal("stone", art)
        n.putpixel((2,2),(255,255,255,255))
        n.putpixel((3,2),(128,128,0,255))
        n.save(self.normals / path.name)
        _, issues = self.inspect(path)
        self.assertTrue({"normal_unit_length", "normal_backfacing"} <= self.codes(issues))

    def test_duplicate_visible_pose_catches_normal_row_drift(self):
        path, art, _ = self.export()
        two = Image.new("RGBA",(8,16))
        two.paste(art,(0,0))
        two.paste(art,(0,8))
        # Invisible RGB must not defeat duplicate-visible-pose recognition.
        two.putpixel((0,8),(99,3,20,0))
        meta = {"cell":[8,8],"anchor":[4,6],"anims":{"a":{"row":0,"frames":1},"b":{"row":1,"frames":1}}}
        path, _, _ = self.export(art=two, meta=meta)
        n = self.normal("stone",two)
        n.putpixel((2,10),(255,128,128,255))
        n.save(self.normals / path.name)
        result, issues = self.inspect(path)
        self.assertEqual(result["normal"]["duplicate_visible_poses_checked"],1)
        self.assertIn("duplicate_pose_normal_drift",self.codes(issues))

    def test_warden_fixed_socket_canvas_contract(self):
        path, _, _ = self.export("warden_body")
        _, issues = self.inspect(path)
        self.assertTrue({"warden_fixed_canvas", "warden_pose_keys"} <= self.codes(issues))

    def test_folded_warden_feet_cancel_the_existing_world_torso_motion(self):
        base = Image.new("RGBA",(120,112))
        base.putpixel((60,5),(80,100,120,255))
        base.putpixel((30,109),(80,100,120,255))
        lifted = base.copy()
        lifted.putpixel((30,109),(0,0,0,0))
        lifted.putpixel((30,108),(80,100,120,255))
        art = Image.new("RGBA",(240,448))
        for row in range(4):
            art.paste(base,(0,row*112))
        art.paste(lifted,(120,336))
        meta = {"cell":[120,112],"anchor":[60,109],"anims":{
            "idle":{"row":0,"frames":1},"awake":{"row":1,"frames":1},
            "damaged":{"row":2,"frames":1},"awake_fold":{"row":3,"frames":2}}}
        path, _, _ = self.export("warden_body",art=art,meta=meta)
        result, issues = self.inspect(path)
        self.assertEqual(issues,[])
        folded = [f for f in result["frames"] if f["anim"] == "awake_fold" and f["frame"] == 1][0]
        self.assertEqual(folded["warden_world_foot_delta"],0)
        art.putpixel((150,445),(80,100,120,255))
        art.save(path)
        _, issues = self.inspect(path)
        self.assertIn("warden_foot_anchor",self.codes(issues))

    def test_altar_rejects_clipping_the_approved_complete_source(self):
        art = Image.new("RGBA",(72,56))
        art.putpixel((6,4),(100,100,100,255))
        art.putpixel((65,52),(100,100,100,255))
        path, _, _ = self.export("bloomwild_altar",art=art,cell=(72,56),anchor=(36,25))
        _, issues = self.inspect(path)
        self.assertIn("altar_complete_source_bounds",self.codes(issues))

    def test_direct_tile_and_font_layouts_do_not_require_animation_keys(self):
        meta = {"cell":[8,8],"anchor":[0,0],"tiles":{"slab":[0,0]},"anims":{}}
        path, _, _ = self.export(meta=meta)
        _, issues = self.inspect(path,strict=False)
        self.assertEqual(issues,[])
        meta["tiles"]["slab"] = [1,0]
        path.with_suffix(".json").write_text(json.dumps(meta))
        _, issues = self.inspect(path,strict=False)
        self.assertIn("tile_cell_bounds",self.codes(issues))
        meta = {"cell":[8,8],"anchor":[0,0],"glyphs":{"A":[2,0,7]}}
        path.with_suffix(".json").write_text(json.dumps(meta))
        _, issues = self.inspect(path,strict=False)
        self.assertIn("glyph_pixel_bounds",self.codes(issues))

    def campaign(self):
        trace = [{"tick":15*(i+1),"zone":z,"pos":[100,200],"shard_taken":True,"deaths":0}
                 for i,z in enumerate(["bloomwild","glasswater","cinder","summit"])]
        milestones = {k:60+i*10 for i,k in enumerate(audit.REQUIRED_MILESTONES)}
        # An opening within phase2 occurs before phase3, unlike the display list.
        milestones["phase_2_heart_open"] = milestones["phase_2_damage"]+1
        return {"trace":trace,"milestones":milestones,"scene":"victory","victory_ready":True,
                "pilot":"tour","simulation_hz":60,"god":True}

    def test_campaign_requires_actual_three_shard_trace_and_later_phase_progress(self):
        path = self.root / "report.json"
        data = self.campaign()
        path.write_text(json.dumps(data))
        issues = []
        audit.campaign_metrics(path,issues)
        self.assertEqual(issues,[])
        data["trace"][1]["shard_taken"] = False
        del data["milestones"]["phase_3_damage"]
        path.write_text(json.dumps(data))
        issues = []
        audit.campaign_metrics(path,issues)
        self.assertTrue({"campaign_shard_trace","campaign_milestones"} <= self.codes(issues))

    def test_victory_only_warden_pilot_is_not_a_campaign(self):
        path = self.root / "report.json"
        data = self.campaign()
        data["pilot"] = "warden"
        data["trace"] = data["trace"][-1:]
        path.write_text(json.dumps(data))
        issues = []
        audit.campaign_metrics(path,issues)
        self.assertTrue({"campaign_regions","campaign_real_input_tour"} <= self.codes(issues))

    def test_capture_is_hashed_and_native_png_grid_is_measured(self):
        path, art, _ = self.export()
        manifest = self.root / "world.json"
        manifest.write_text(json.dumps({"captures":[{"zone":"bloomwild","name":"entry","file":str(path),"sha256":"0"*64}]}))
        issues = []
        audit.capture_metrics([manifest],self.root,issues)
        self.assertTrue({"capture_hash","capture_native_grid"} <= self.codes(issues))
        issues = []
        audit.capture_metrics([manifest,manifest],self.root,issues)
        self.assertIn("duplicate_capture_identity",self.codes(issues))

    def test_new_recursively_embedded_preview_is_an_error(self):
        (self.root / "build.rs").write_text('for sub in ["sprites", "normals"] { if matches!(ext, "png" | "json") {} }')
        path, _, _ = self.export("preview")
        baseline = self.root / "old"
        baseline.mkdir()
        issues = []
        measured = audit.embedding_metrics(self.root,baseline,issues)
        self.assertEqual(measured["file_count"],2)
        self.assertTrue(any(i["code"] == "suspicious_embedded_asset" and i["severity"] == "error" for i in issues))

    def test_exact_replay_rejects_one_changed_pixel_but_comparison_only_measures(self):
        a = self.root / "a.png"
        b = self.root / "b.png"
        im = Image.new("RGBA",(384,216),(100,100,100,255))
        im.save(a)
        im.putpixel((1,1),(101,100,100,255))
        im.save(b)
        before = {"manifests":[{"captures":[{"key":"scene","file":str(a),"binary_sha256":"a"*64,"render_settings":{}}]}]}
        after = {"manifests":[{"captures":[{"key":"scene","file":str(b),"binary_sha256":"a"*64,"render_settings":{}}]}]}
        issues = []
        result = audit.capture_comparison(before,after,issues)
        self.assertEqual(result["matched"][0]["changed_pixels"],1)
        self.assertEqual(issues,[])
        audit.capture_comparison(before,after,issues,exact_replay=True)
        self.assertEqual(self.codes(issues),{"replay_pixel_difference"})
        after["manifests"][0]["captures"][0].update(file=str(a),binary_sha256="b"*64,render_settings={"position":[2,3]})
        issues = []
        audit.capture_comparison(before,after,issues,exact_replay=True)
        self.assertEqual(self.codes(issues),{"replay_renderer_identity","replay_settings_mismatch"})

    def test_report_is_exclusive_and_does_not_claim_beauty(self):
        self.export()
        (self.root / "build.rs").write_text('for sub in ["sprites", "normals"] { if matches!(ext, "png" | "json") {} }')
        out = self.root / "evidence.json"
        with contextlib.redirect_stdout(io.StringIO()):
            code = audit.main(["--root",str(self.root),"--out",str(out)])
        self.assertEqual(code,0)
        report = json.loads(out.read_text())
        self.assertEqual(report["verdict"],"measured_with_unverified_gates")
        self.assertTrue(report["unverified_gates"])
        self.assertEqual(report["aesthetic_verdict"],"not assessed by this automated harness")
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            audit.main(["--root",str(self.root),"--out",str(out)])
        final_out = self.root / "require-final.json"
        with contextlib.redirect_stdout(io.StringIO()):
            code = audit.main(["--root",str(self.root),"--require-round13","--out",str(final_out)])
        final = json.loads(final_out.read_text())
        self.assertEqual(code,1)
        self.assertTrue({"round13_family_missing","round13_warden_pose_coverage"} <= self.codes(final["issues"]))


if __name__ == "__main__":
    unittest.main()
