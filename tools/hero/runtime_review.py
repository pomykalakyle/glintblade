"""Assemble pinned production hero captures into native boards and60fps clips.

Input is four region-specific `eightway-audit --mode player --every1` runs.
No source asset or game source is modified. The exact384x216 PNGs remain primary
evidence; videos enlarge each source pixel by3 before lossless RGB encoding.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess

from PIL import Image, ImageDraw

from eightway import DIRECTIONS

STARTS = {"bloomwild": (152, 696), "glasswater": (32, 425.6),
          "cinder": (136, 752), "summit": (196, 782)}


def run(stage):
    out = stage / "runtime-review"
    out.mkdir(exist_ok=True)
    regions = list(STARTS)
    records, reports = {}, {}
    for region in regions:
        source = stage / f"runtime-{region}-v1"
        data = json.loads((source / "render-manifest.json").read_text())
        captures = data["captures"]
        assert len(captures) == 8 * 220
        lookup = {(r["direction_case"], r["local_tick"]): r for r in captures}
        assert len(lookup) == len(captures)
        assert all(r["render"]["actor_surface_pixels"] == r["render"]["valid_actor_normal_pixels"] for r in captures)
        assert all(r["pose"] and r["pose"]["exact_matches"] for r in captures)
        assert {r["visual_direction"] for r in captures} == set(DIRECTIONS)
        records[region] = lookup
        reports[region] = {"frames": len(captures), "fps": 60, "native_size": [384, 216],
                           "directions": list(DIRECTIONS),
                           "states": dict(Counter(r["state"].split(" {")[0] for r in captures)),
                           "all_visible_actor_normal_pixels_valid": True,
                           "all_poses_match_native_sheet": True,
                           "fixed_camera_actor_outside_viewport": [{"direction": r["direction_case"], "tick": r["local_tick"], "state": r["state"]}
                                                                    for r in captures if r["render"]["actor_surface_pixels"] == 0],
                           "manifest": str(source / "render-manifest.json")}
        sequence = out / f"{region}-frames"
        sequence.mkdir(exist_ok=True)
        for i, (d, tick) in enumerate((d, t) for d in DIRECTIONS for t in range(220)):
            link = sequence / f"f{i:05}.png"
            if not link.exists():
                link.symlink_to(lookup[d, tick]["path"])

    def crop(region, direction, tick):
        r = records[region][direction, tick]
        sx, sy = STARTS[region]
        cam = (int(sx - 192), int(sy - 108))
        cx, cy = round(r["position"][0] - cam[0]), round(r["position"][1] - r["position"][2] - cam[1])
        return Image.open(r["path"]).convert("RGB").crop((cx - 56, cy - 52, cx + 56, cy + 28))

    categories = {"northeast_turn_stop": [("run NE", 8), ("stop NE", 21), ("turn E", 23), ("return NE", 28)],
                  "northeast_combat": [("forehand", 35), ("backhand", 50), ("finisher", 68),
                                        ("charge", 128), ("spin", 148), ("dash", 172), ("hurt", 192)]}
    for name, picks in categories.items():
        board = Image.new("RGB", (112 * 4, 12 + 92 * len(picks)), "#202435")
        draw = ImageDraw.Draw(board)
        for col, region in enumerate(regions):
            draw.text((112 * col + 2, 0), region, fill="#b4d1ce")
            for row, (label, tick) in enumerate(picks):
                y = 12 + row * 92
                draw.text((112 * col + 2, y), f"{label} t{tick}", fill="#b4d1ce")
                board.paste(crop(region, "up_right", tick), (112 * col, y + 12))
        board.save(out / f"{name}_native.png")
        board.resize((board.width * 3, board.height * 3), Image.Resampling.NEAREST).save(out / f"{name}_3x.png")

    # Every direction's run/stop/strike/charge/spin/dash/hurt in a native grid.
    for region in regions:
        picks = [("run", 8), ("idle", 21), ("attack1", 35), ("attack2", 50),
                 ("attack3", 68), ("charge", 128), ("spin", 148), ("dash", 172), ("hurt", 192)]
        board = Image.new("RGB", (112 * 8, 12 + 92 * len(picks)), "#202435")
        draw = ImageDraw.Draw(board)
        for col, d in enumerate(DIRECTIONS):
            draw.text((112 * col + 2, 0), d, fill="#b4d1ce")
            for row, (label, tick) in enumerate(picks):
                y = 12 + row * 92
                draw.text((112 * col + 2, y), label, fill="#b4d1ce")
                board.paste(crop(region, d, tick), (112 * col, y + 12))
        board.save(out / f"{region}_all_directions_native.png")
        board.resize((board.width * 2, board.height * 2), Image.Resampling.NEAREST).save(out / f"{region}_all_directions_2x.png")

    commands = []
    for region in regions:
        command = ["ffmpeg", "-y", "-v", "error", "-framerate", "60", "-i", str(out / f"{region}-frames/f%05d.png"),
                   "-vf", "scale=iw*3:ih*3:flags=neighbor", "-an", "-c:v", "libx264rgb", "-crf", "0", "-preset", "fast",
                   str(out / f"hero_{region}_eightway_60fps.mp4")]
        commands.append(command)
    processes = [subprocess.Popen(command) for command in commands]
    assert all(p.wait() == 0 for p in processes)
    for region in regions:
        video = out / f"hero_{region}_eightway_60fps.mp4"
        probe = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_streams", "-of", "json", str(video)]))
        assert len(probe["streams"]) == 1 and probe["streams"][0]["codec_type"] == "video"
        assert probe["streams"][0]["r_frame_rate"] == "60/1"
        reports[region]["video"] = str(video)
        reports[region]["video_sha256"] = hashlib.sha256(video.read_bytes()).hexdigest()
    report = {"regions": reports, "commands": commands, "frames": 7040, "fps": 60,
              "coverage": ["all8 native facing sectors", "walking", "idle stopping", "45degree turns", "all3combo attacks",
                           "held charge", "clockwise spin and native blade trail", "dash and exactcell afterimages", "hurt recoil",
                           "production scarf", "source-dependent normal lighting", "anatomical support and shadow anchoring"],
              "separate_supplement_required": ["charge walking", "180degree reversals", "lethal collapse/debris", "respawn fall/land"]}
    report["camera_staging_limit"] = ("Fixed start camera lets north dash leave the viewport for15ticks in Bloomwild and Cinder. "
                                      "Those30frames are excluded from visible normal/pose review; source atlas and following transition supplement cover them.")
    (out / "review.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"frames": 7040, "fps": 60, "review": str(out / "review.json")}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("stage", type=Path)
    run(parser.parse_args().stage)
