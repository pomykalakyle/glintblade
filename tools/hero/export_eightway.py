"""Scoped hero export plus exhaustive native-grid art/normal/attachment evidence.

uv run --no-project --with pillow --with numpy python3 tools/hero/export_eightway.py \
  --stage /private/hero --baseline /private/hero/baseline

Only --stage/assets/{sprites/hero.*,normals/hero.png} is written. Installation is
a separate deliberate copy of those three files; no umbrella exporter runs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

import numpy as np
from PIL import Image, ImageDraw

import build
import eightway
import native_normals
from pix import on_ground, scale
from rig import CELL, ANCHOR

BASES = build.DIRECTIONAL + build.APPENDED + ["collapse", "spin", "fall", "land"]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sample(row, tick):
    frames, durations = row[1], row[3]
    tick %= sum(durations)
    for i, t in enumerate(durations):
        if tick < t:
            return frames[i]
        tick -= t
    raise AssertionError("unreachable timeline")


def export(stage, baseline):
    stage.mkdir(parents=True, exist_ok=True)
    asset_dir = stage / "assets"
    previous_argv = sys.argv
    sys.argv = ["build.py", "--sheet-only", "--output", str(asset_dir)]
    build.main()
    sys.argv = previous_argv
    rows = build.collect()
    problems, _ = build.check(rows)
    assert not problems, problems
    by = {row[0]: row for row in rows}
    meta = json.loads((asset_dir / "sprites/hero.json").read_text())
    meta["notes"] += (" Eight-direction pass (2026-11-02): original 39 rows are byte-exact; "
                       "four native three-quarter families are appended for every locomotion/combat state. "
                       "West diagonal anatomy is mirrored and relit; the blade is redrawn natively. "
                       "collapse_{direction} replaces legacy front-only death, with unchanged five holds. "
                       "spin_{direction} begins at the held facing and makes one clockwise turn through "
                       "eight native anatomical keys; runtime never mirrors that complete sequence. "
                       "fall/land are explicit in all eight facings. Native geometric normals are derived "
                       "from independent authored hood/cloak/boot/sleeve/prism volumes, not albedo luminance. "
                       "Old normal rows are preserved. Contact sheets, timing/attachment/coverage checks "
                       "and source receipts: art/eightway/hero and tools/hero/export_eightway.py.")
    (asset_dir / "sprites/hero.json").write_text(json.dumps(meta, indent=1) + "\n")

    art = Image.open(asset_dir / "sprites/hero.png").convert("RGBA")
    old_art = Image.open(baseline / "assets/sprites/hero.png").convert("RGBA")
    old_normal = Image.open(baseline / "assets/normals/hero.png").convert("RGBA")
    old_meta = json.loads((baseline / "assets/sprites/hero.json").read_text())
    assert old_art.size == old_normal.size
    assert art.crop((0, 0, *old_art.size)).tobytes() == old_art.tobytes(), "legacy art changed"
    for name, anim in old_meta["anims"].items():
        assert all(meta["anims"][name][key] == value for key, value in anim.items()), f"legacy metadata changed: {name}"
    normal = Image.new("RGBA", art.size)
    normal.paste(old_normal, (0, 0))
    # Identical native poses must retain identical surface geometry, including
    # newly named landing/death keys that reuse old art. Otherwise a motionless
    # silhouette can visibly pop when it enters a different animation row.
    normal_by_art = {}
    for anim in old_meta["anims"].values():
        for i in range(anim["frames"]):
            box = (i * CELL, anim["row"] * CELL, (i + 1) * CELL, (anim["row"] + 1) * CELL)
            normal_by_art.setdefault(old_art.crop(box).tobytes(), old_normal.crop(box))
    for row, (name, frames, metas, durations) in enumerate(rows):
        if row * CELL < old_art.height:
            continue
        for i, (frame, data) in enumerate(zip(frames, metas)):
            key = frame.to_image().tobytes()
            n = normal_by_art.get(key)
            if n is None:
                n = native_normals.render(data["source_spec"], frame)
                normal_by_art[key] = n
            normal.paste(n, (i * CELL, row * CELL))
    (asset_dir / "normals").mkdir(parents=True, exist_ok=True)
    normal.save(asset_dir / "normals/hero.png")
    assert normal.crop((0, 0, *old_normal.size)).tobytes() == old_normal.tobytes()
    a, n = np.array(art), np.array(normal)
    assert np.array_equal(a[..., 3], n[..., 3]), "normal alpha drift"
    vectors = n[..., :3].astype(float) / 127.5 - 1
    opaque = a[..., 3] > 0
    lengths = np.linalg.norm(vectors[opaque], axis=1)
    assert lengths.min() > .97 and lengths.max() < 1.03
    assert (vectors[..., 2][opaque] > 0).all()

    evidence = stage / "evidence"
    evidence.mkdir(exist_ok=True)
    coverage = []
    for base in BASES:
        group = [by[f"{base}_{d}"] for d in eightway.DIRECTIONS]
        counts = [len(r[1]) for r in group]
        assert len(set(counts)) == 1, (base, counts)
        holds = [r[3] for r in group]
        assert all(t == holds[0] for t in holds), f"directional timing drift: {base}"
        # All frames, with directions as columns and timeline keys as rows.
        board = Image.new("RGB", (CELL * 8, 12 + CELL * counts[0]), "#202435")
        draw = ImageDraw.Draw(board)
        for i, d in enumerate(eightway.DIRECTIONS):
            draw.text((i * CELL + 1, 0), d.replace("right", "R").replace("left", "L"), fill="#b4d1ce")
            name, frames, metas, durations = group[i]
            entry = {"actor": "Glint", "appearance": "authored_cloak", "animation": base,
                     "direction": d, "lookup": name, "row": meta["anims"][name]["row"],
                     "frames": len(frames), "durations": durations, "ticks": sum(durations),
                     "scarf_anchor": [m["knot"] for m in metas], "blade_tip": [m["tip"] for m in metas],
                     "support_spans": [m["support_spans"] for m in metas],
                     "sheet": "assets/sprites/hero.png", "normals": "assets/normals/hero.png",
                     "evidence": f"evidence/{base}_all_frames_1x.png"}
            for j, (frame, data) in enumerate(zip(frames, metas)):
                image = frame.to_image()
                assert image.getbbox() is not None
                for key in ("knot", "tip"):
                    p = data[key]
                    assert p and 0 <= p[0] < CELL and 0 <= p[1] < CELL, (name, j, key, p)
                # Tip is an actual authored visible blade pixel except when a
                # deliberately rear-layer blade is occluded by the body.
                tip = data["tip"]
                assert image.getpixel(tuple(tip))[3] == 255, (name, j, "tip outside alpha")
                board.paste(on_ground(image, build.GROUNDS["glasswater"]).convert("RGB"), (i * CELL, 12 + j * CELL))
            coverage.append(entry)
        board.save(evidence / f"{base}_all_frames_1x.png")
        scale(board, 4).save(evidence / f"{base}_all_frames_4x.png")
        # Actual authored timing, all directions concurrently; no frame skipping.
        frames = []
        for tick in range(sum(holds[0])):
            line = Image.new("RGB", (CELL * 8, CELL), "#364657")
            for i, row in enumerate(group):
                line.paste(on_ground(sample(row, tick).to_image(), build.GROUNDS["glasswater"]), (i * CELL, 0))
            frames.append(scale(line, 4))
        # Pillow GIF 60Hz is quantized to10ms, lossless frame PNG sheets are primary.
        build.save_gif(frames, [1] * len(frames), str(evidence / f"{base}_all_directions.gif"), exact_ticks=True)

    # Direction sweeps deliberately hold, turn, stop, reverse and enter combat.
    sweep = []
    schedule = [("idle", d, 12) for d in eightway.DIRECTIONS]
    schedule += [("run", d, 32) for d in eightway.DIRECTIONS]
    schedule += [("run", "up_right", 24), ("run", "right", 16), ("idle", "right", 24),
                 ("run", "up_left", 24), ("dash", "up_left", 10), ("hurt", "up_left", 15),
                 ("idle", "up_left", 20)]
    schedule += [(base, d, sum(by[f"{base}_{d}"][3])) for d in eightway.DIRECTIONS
                 for base in ("attack1", "attack2", "attack3", "charge_walk", "spin")]
    for base, d, ticks in schedule:
        row = by[f"{base}_{d}"]
        for tick in range(ticks):
            frame = scale(on_ground(sample(row, tick).to_image(), build.GROUNDS["glasswater"]).convert("RGB"), 6)
            ImageDraw.Draw(frame).text((5, 5), f"{base} {d}", fill="#b4d1ce")
            sweep.append(frame)
    build.save_gif(sweep, [1] * len(sweep), str(evidence / "hero_direction_sweep.gif"), exact_ticks=True)

    manifest = {"actor": "Glint", "directions": list(eightway.DIRECTIONS), "cell": [CELL, CELL],
                "anchor": list(ANCHOR), "legacy_rows_preserved": old_art.height // CELL,
                "rows": len(rows), "coverage_entries": len(coverage), "coverage": coverage,
                "normal_checks": {"exact_alpha": True, "old_normal_rows_exact": True,
                                  "encoded_length_min": float(lengths.min()), "encoded_length_max": float(lengths.max()),
                                  "positive_viewer_component": True},
                "semantics": {"spin": "facing-indexed clockwise native turn;8keys with unchanged24ticks",
                              "collapse": "eight facing death poses with original64tick duration",
                              "legacy_spin_death": "preserved unused compatibility rows",
                              "ground_fx": "directionless rings remain intentional",
                              "dash_afterimages": "copy exact live native cell, anchor and flip",
                              "scarf": "runtime cloth driven by per-frame native nape attachment",
                              "supports": "body-only native support_spans per frame/column [lo,hi_exclusive] excludes entire blade/hilt;48px cell/24,33anchor;central4px runtime footdepth"},
                "hashes": {str(p.relative_to(stage)): sha(p) for p in sorted(asset_dir.rglob("*")) if p.is_file()},
                "source_hashes": {str(p.relative_to(Path(build.ROOT))): sha(p)
                                  for p in sorted(Path(__file__).parent.glob("*.py"))}}
    (stage / "coverage.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({k: manifest[k] for k in ("rows", "coverage_entries", "normal_checks")}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    options = parser.parse_args()
    export(options.stage, options.baseline)
