"""Freeze and compare the round-two Cinder pass using explicit silent headless commands."""

import hashlib
import json
import shutil
import subprocess
from datetime import datetime
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "out/samples/cinder_polish_round2"
BASELINE = ROOT / "out/samples/polish_round_2/baseline_glintblade"
SDK = "/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk"
VIEWS = {"quiet_ruins": "720,420", "quiet_banks": "370,455", "caldera": "1144,280"}
COMBAT = "A2 W10 A2 W10 A2 W50"


def digest(path):
    """Hash one exact source, asset or executable without executing it."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs():
    """Hash all Rust sources and embedded assets to expose concurrent build drift."""
    files = [*ROOT.glob("src/**/*.rs"), ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "build.rs"]
    for family in ("sprites", "music", "sfx", "fonts", "data"):
        files += [p for p in (ROOT / "assets" / family).rglob("*") if p.is_file()]
    files += [*ROOT.glob("tools/cinder/*.py"), ROOT / "assets/concept/palettes_v2.json"]
    return {str(p.relative_to(ROOT)): digest(p) for p in sorted(files)}


def changed(before, after):
    """List changed or newly present inputs between two recorded snapshots."""
    return sorted(p for p in before.keys() | after.keys() if before.get(p) != after.get(p))


def run(args, commands):
    """Record and execute one command, requiring headless mode for every game executable."""
    if "glintblade" in Path(args[0]).name:
        assert args[1] in ("shot", "zoneview"), args
    commands.append([str(a) for a in args])
    subprocess.run(args, cwd=ROOT, check=True)


def captures(binary, phase, commands):
    """Capture real nonpeaceful fights and still scenery from an immutable executable."""
    for name, pos, script, ticks, at in (
        ("combat", "575,510", COMBAT, "90", "1,24,60,90"),
        ("foundry", "720,420", "W120", "120", "1,60,120"),
    ):
        dest = OUT / phase / name
        run([str(binary), "shot", "--zone", "cinder", "--pos", pos, "--god", "--seed", "7",
             "--script", script, "--ticks", ticks, "--at", at, "--scale", "1", "--out", str(dest)], commands)
    for name, pos in VIEWS.items():
        run([str(binary), "zoneview", "cinder", "--at", pos, "--scale", "1", "--out",
             str(OUT / phase / name)], commands)


def boards():
    """Write integer nearest-neighbor comparisons and unboosted dark-room value plates."""
    metrics = {}
    # Six broad grayscale intervals preserve the captured hierarchy without exposure changes.
    cuts = (24, 40, 64, 96, 160)
    values = (8, 32, 52, 80, 128, 208)
    table = [values[sum(v >= cut for cut in cuts)] for v in range(256)]
    for sample in sorted((OUT / "after").glob("*/*.png")):
        if sample.stem.endswith("_3x"):
            continue
        rel = sample.relative_to(OUT / "after")
        board = Image.new("RGB", (2304, 676), "#08080c")
        dark = Image.new("RGB", (2304, 676), "#08080c")
        for i, phase in enumerate(("before", "after")):
            path = OUT / phase / rel
            im = Image.open(path).convert("RGB")
            assert im.size == (384, 216), path
            big = im.resize((1152, 648), Image.Resampling.NEAREST)
            big.save(path.with_stem(path.stem + "_3x"))
            board.paste(big, (i * 1152, 28))
            gray = im.convert("L")
            dark.paste(gray.point(table).convert("RGB").resize((1152, 648), Image.Resampling.NEAREST),
                       (i * 1152, 28))
            label = f"{phase} {rel} | native 384x216 at 3x NN"
            ImageDraw.Draw(board).text((i * 1152 + 8, 8), label, fill="#a79a98")
            ImageDraw.Draw(dark).text((i * 1152 + 8, 8), label, fill="#a79a98")
            hist = gray.histogram()
            metrics[f"{phase}/{rel}"] = {
                "mean_luma_8bit": round(sum(v * n for v, n in enumerate(hist)) / (384 * 216), 3),
                "percent_below_32": round(sum(hist[:32]) * 100 / (384 * 216), 3),
                "percent_at_least_96": round(sum(hist[96:]) * 100 / (384 * 216), 3),
            }
        name = f"{rel.parent.name}_{rel.stem}"
        board.save(OUT / f"compare_{name}_3x.png")
        dark.save(OUT / f"dark_room_ValueHierarchy_{name}_3x.png")
    sheet = Image.open(ROOT / "assets/sprites/cinder_brazier.png").convert("RGBA")
    bg = Image.new("RGBA", sheet.size, "#363340")
    bg.alpha_composite(sheet)
    bg.convert("RGB").resize((sheet.width * 8, sheet.height * 8), Image.Resampling.NEAREST).save(
        OUT / "brazier_8x.png")
    return {"interval_upper_bounds": [*cuts, 256], "display_values": values, "samples": metrics}


def main():
    """Build, freeze, capture and record review evidence without opening a window or audio."""
    OUT.mkdir(parents=True, exist_ok=True)
    commands = []
    before_build = inputs()
    run(["env", f"SDKROOT={SDK}", "cargo", "build", "--release"], commands)
    after_build = inputs()
    frozen = OUT / "after_glintblade"
    shutil.copy2(ROOT / "target/release/glintblade", frozen)
    source_dir = OUT / "frozen_source"
    for path in after_build:
        if path.startswith("src/") or path in ("Cargo.toml", "Cargo.lock", "build.rs"):
            dest = source_dir / path
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / path, dest)
    captures(BASELINE, "before", commands)
    captures(frozen, "after", commands)
    hierarchy = boards()
    initial_file = OUT / "initial_hashes.json"
    initial = json.loads(initial_file.read_text()) if initial_file.exists() else before_build
    report = {
        "written": datetime.now().astimezone().isoformat(),
        "commands": commands,
        "binaries": {str(p.relative_to(ROOT)): digest(p) for p in (BASELINE, frozen)},
        "inputs_before_build": before_build,
        "frozen_inputs_after_build": after_build,
        "drift_during_build": changed(before_build, after_build),
        "drift_after_freeze": changed(after_build, inputs()),
        "changed_since_round_start": changed(initial, {p: h for p, h in after_build.items() if p in initial}),
        "value_hierarchy": hierarchy,
        "limits": [
            "Pinned baseline predates the renderer owner's six review fixes and actor-coverage work.",
            "Before/after differences include shared renderer and enemy changes; this is not an isolated art A/B.",
            "Cinder changes: fill/shadow tint, one-step paving/grout lift, duplicate baked shadow removal, visible keeper brazier.",
            "Visible brazier at 646,456 has no footprint; tile map, routes, enemy spawns, timings and gameplay are unchanged.",
            "Actor source-facing rims remain a renderer dependency: the local actor light floor does not supply a directional edge.",
            "Request a one-native-pixel source-facing actor rim from visible lava/brazier using real actor coverage, preserving far edges.",
            "Value plates use captured 8-bit luminance in six intervals; no gamma/exposure lift and no numeric beauty claim.",
            "No game invocation except explicit shot/zoneview; no browser, window, audio, external API or image generation.",
        ],
    }
    (OUT / "EVIDENCE.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: report[k] for k in ("binaries", "drift_during_build", "drift_after_freeze")}, indent=2))


if __name__ == "__main__":
    main()
