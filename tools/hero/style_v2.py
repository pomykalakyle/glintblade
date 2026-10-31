"""Audit the approved hero geometry and capture silent, reproducible v2 comparisons.

Run after building: uv run --no-project --with pillow python3 tools/hero/style_v2.py
Outputs, pinned binaries and exact commands live in out/samples/hero_v2style/.
"""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from PIL import Image

import build
from pix import PAL, hex_rgb

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "out/style_v2_baseline"
OUT = ROOT / "out/samples/hero_v2style"
SCRIPT = "W50 R16 W8 L32 W72 A1 W9 A1 W9 A1 W38 A50 W40 LX1 W24"
ZONES = {"bloomwild": None, "glasswater": "224,432", "cinder": None, "summit": None}
KEYS = [50, 62, 86, 106, 178, 182, 186, 194, 208, 222, 284, 290, 296, 328, 330, 333, 335, 339, 345]


def sha(path):
    """Return the exact SHA-256 identity of a capture input."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def geometry_audit():
    """Verify every approved mask, point, duration and emissive assignment survives recoloring."""
    before_meta = json.loads((BASE / "sprites/hero.json").read_text())
    after_meta = json.loads((ROOT / "assets/sprites/hero.json").read_text())
    for key in ("cell", "anchor", "anims"):
        assert before_meta[key] == after_meta[key], key
    before = Image.open(BASE / "sprites/hero.png").convert("RGBA")
    after = Image.open(ROOT / "assets/sprites/hero.png").convert("RGBA")
    assert before.size == after.size
    old_pal = json.loads((ROOT / "assets/concept/palettes.json").read_text())["hero"]
    new_pal = json.loads((ROOT / "assets/concept/palettes_v2.json").read_text())["hero"]
    # Authored shade characters retain their spatial assignments; v2 folds one cloak step.
    mapping = {}
    for material in ("outline", "gear", "brass", "blade"):
        mapping.update(zip(old_pal[material], new_pal[material]))
    mapping.update(zip(old_pal["cloak"], [new_pal["cloak"][0]] + new_pal["cloak"]))
    mapping.update(zip(old_pal["scarf"], [new_pal["scarf"][0]] + new_pal["scarf"]))
    mapping[old_pal["eyes"][0]] = new_pal["eyes"][0]
    rgb_map = {hex_rgb(a): hex_rgb(b) for a, b in mapping.items()}
    old_em = {hex_rgb(c) for c in before_meta["emissive"]}
    new_em = {hex_rgb(c) for c in after_meta["emissive"]}
    visible = 0
    before_pixels, after_pixels = before.tobytes(), after.tobytes()
    for offset in range(0, len(before_pixels), 4):
        a, b = tuple(before_pixels[offset:offset + 4]), tuple(after_pixels[offset:offset + 4])
        assert a[3] == b[3] and b[3] in (0, 255), (a, b)
        if a[3]:
            assert rgb_map[a[:3]] == b[:3], (a, b)
            assert (a[:3] in old_em) == (b[:3] in new_em), (a, b)
            visible += 1
    rows = build.collect()
    problems, _ = build.check(rows)
    assert not problems, problems
    for name, frames, metas, durations in rows:
        anim = after_meta["anims"][name]
        assert durations == anim["durations"]
        for i, (frame, meta) in enumerate(zip(frames, metas)):
            box = (i * 48, anim["row"] * 48, (i + 1) * 48, (anim["row"] + 1) * 48)
            assert frame.to_image().tobytes() == after.crop(box).tobytes(), (name, i)
            assert meta["knot"] == anim["scarf_anchor"][i]
            assert meta["tip"] == anim["blade_tip"][i]
    # Player behavior, including the approved distance-spaced dash echoes, is byte-identical.
    player = ROOT / "src/entities/player.rs"
    old_source = (BASE / "src/entities/player.rs").read_text()
    new_source = player.read_text()
    prefix = "// ------------------------------------------------------------------------------------ palette"
    suffix = "/// One combo swing's timing, movement, hitbox and smear."
    assert old_source.split(prefix)[0] == new_source.split(prefix)[0]
    assert old_source.split(suffix)[1] == new_source.split(suffix)[1]
    return {"rows": len(rows), "cells": sum(len(r[1]) for r in rows), "visible_pixels": visible,
            "opaque_colors": len(after.getcolors(maxcolors=256)) - 1,
            "geometry_metadata_emissive_masks_exact": True, "non_palette_player_source_exact": True}


def source_boards(phase, sprite_dir):
    """Save native and 8x keys including directional lighting, attacks, dash and fallen blade."""
    sheet = Image.open(sprite_dir / "hero.png").convert("RGBA")
    meta = json.loads((sprite_dir / "hero.json").read_text())
    picks = [("idle_right", 0), ("idle_left", 0), ("idle_down", 0), ("idle_up", 0),
             ("run_right", 0), ("run_left", 4), ("attack1_right", 1), ("attack1_left", 1),
             ("attack1_right", 2), ("attack1_left", 2), ("attack2_right", 2), ("attack2_left", 2),
             ("attack3_right", 3), ("attack3_left", 3), ("dash_right", 0), ("dash_left", 0),
             ("charge_left", 1), ("spin", 2), ("hurt_left", 0), ("death", 2),
             ("death", 4), ("death_left", 4), ("run_up", 0), ("run_down", 4)]
    board = Image.new("RGB", (6 * 48, 4 * 48), "#747d8d")
    for n, (name, index) in enumerate(picks):
        row = meta["anims"][name]["row"]
        cell = sheet.crop((index * 48, row * 48, (index + 1) * 48, (row + 1) * 48))
        board.paste(cell, ((n % 6) * 48, (n // 6) * 48), cell)
    for scale in (1, 8):
        board.resize((board.width * scale, board.height * scale), Image.Resampling.NEAREST).save(
            OUT / f"hero_v2style_{phase}_source_{scale}x.png")
    return picks


def motion_board(phase, zone):
    """Keep exact-pixel action keys together for inspecting scarf, smear and echo progression."""
    ticks = [178, 180, 182, 184, 186, 188, 190, 192, 194, 196, 198, 200,
             202, 204, 206, 208, 212, 222, 284, 288, 292, 296, 300, 304,
             328, 330, 332, 334, 336, 338]
    cx, cy = {"bloomwild": (132, 108), "glasswater": (192, 108),
              "cinder": (117, 108), "summit": (176, 108)}[zone]
    board = Image.new("RGB", (6 * 112, 5 * 80))
    for n, tick in enumerate(ticks):
        frame = Image.open(OUT / phase / zone / f"f{tick:05}.png").convert("RGB")
        # Combo lunges and the final left dash move the actor across a clamped camera.
        # Keep both phase crops identical and leave room for the live leader and its tail.
        center = cx - (48 if tick >= 284 else 0) - (16 if tick >= 328 else 0)
        board.paste(frame.crop((center - 56, cy - 40, center + 56, cy + 40)),
                    ((n % 6) * 112, (n // 6) * 80))
    for scale in (1, 3):
        board.resize((board.width * scale, board.height * scale), Image.Resampling.NEAREST).save(
            OUT / f"hero_v2style_{phase}_{zone}_motion_{scale}x.png")
    return ticks


def capture(phase, binary):
    """Render real gameplay without a window or audio and save exact native/3x evidence."""
    commands = []
    for zone, pos in ZONES.items():
        folder = OUT / phase / zone
        cmd = [str(binary), "shot", "--zone", zone, "--peaceful", "--seed", "7",
               "--script", SCRIPT, "--every", "1", "--scale", "1", "--out", str(folder),
               "--report", str(OUT / f"{phase}_{zone}_report.json")]
        if pos:
            cmd += ["--pos", pos]
        subprocess.run(cmd, cwd=ROOT, check=True)
        commands.append(cmd)
        frames = sorted(folder.glob("f*.png"))
        for tick in KEYS:
            im = Image.open(folder / f"f{tick:05}.png").convert("RGB")
            assert im.size == (384, 216)
            for scale in (1, 3):
                im.resize((384 * scale, 216 * scale), Image.Resampling.NEAREST).save(
                    OUT / f"hero_v2style_{phase}_{zone}_t{tick:03}_{scale}x.png")
        motion_board(phase, zone)
        # Lossless muted motion evidence; exact PNGs remain available for pixel inspection.
        video = OUT / f"hero_v2style_{phase}_{zone}_muted.mp4"
        video_cmd = ["ffmpeg", "-y", "-v", "error", "-framerate", "60", "-i",
                     str(folder / "f%05d.png"), "-an", "-c:v", "libx264rgb", "-crf", "0",
                     "-preset", "fast", str(video)]
        subprocess.run(video_cmd, check=True)
        streams = json.loads(subprocess.check_output(
            ["ffprobe", "-v", "error", "-show_streams", "-of", "json", str(video)]))
        assert [s["codec_type"] for s in streams["streams"]] == ["video"]
        commands.append(video_cmd)
        print(phase, zone, len(frames), "silent frames", flush=True)
    return commands


def main():
    """Pin the rendered inputs and record exact identities and reproduction commands."""
    OUT.mkdir(parents=True, exist_ok=True)
    audit = geometry_audit()
    print("geometry audit:", audit, flush=True)
    pinned = OUT / "renderer_after"
    shutil.copy2(ROOT / "target/release/glintblade", pinned)
    for name in ("hero.png", "hero.json"):
        shutil.copy2(ROOT / "assets/sprites" / name, OUT / name)
    inputs = [BASE / "glintblade", BASE / "sprites/hero.png", BASE / "sprites/hero.json",
              pinned, ROOT / "assets/sprites/hero.png", ROOT / "assets/sprites/hero.json",
              ROOT / "src/entities/player.rs", ROOT / "assets/concept/palettes_v2.json"]
    inputs += sorted((ROOT / "tools/hero").glob("*.py"))
    report = {"written": "2026-10-30", "audit": audit, "hashes": {str(p.relative_to(ROOT)): sha(p) for p in inputs},
              "build": "SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk cargo build --release",
              "palette": PAL, "script": SCRIPT, "seed": 7, "peaceful": True,
              "note": "After captures include concurrent family changes; review hero locally against A/C mocks.",
              "source_keys": source_boards("before", BASE / "sprites")}
    source_boards("after", ROOT / "assets/sprites")
    report["commands"] = capture("before", BASE / "glintblade") + capture("after", pinned)
    report["motion_ticks"] = motion_board("after", "summit")
    (OUT / "identity_and_commands.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
