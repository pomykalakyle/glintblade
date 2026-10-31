"""Capture silent before/after gameplay and exact native-grid source comparisons for style v2.

Run after a release build: uv run --no-project --with pillow --with numpy python3 tools/glasswater_enemies/samples_v2.py
Outputs are review evidence only, under out/samples/enemies_glasswater_v2style/.
"""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "out/samples/enemies_glasswater_v2style"
BASE = ROOT / "out/style_v2_baseline"
NAMES = ("crystal_beetle", "wisp", "wisp_bolt")
SCENES = {"beetle": "500,250", "wisps": "680,90"}


def digest(path):
    """Return the exact file SHA-256 for repeatable review evidence."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compare_source():
    """Verify pose/manifest contracts and draw all authored frames beside the frozen sheets."""
    evidence = {}
    for name in NAMES:
        old_path = BASE / "sprites" / f"{name}.png"
        new_path = ROOT / "assets/sprites" / f"{name}.png"
        old, new = [Image.open(p).convert("RGBA") for p in (old_path, new_path)]
        old_meta, new_meta = [json.loads(p.with_suffix(".json").read_text()) for p in (old_path, new_path)]
        assert old.size == new.size
        assert np.array_equal(np.asarray(old)[..., 3], np.asarray(new)[..., 3]), name
        for key in ("cell", "anchor", "anims"):
            assert old_meta[key] == new_meta[key], (name, key)
        old_em, new_em = [], []
        for im, meta, mask in ((old, old_meta, old_em), (new, new_meta, new_em)):
            em = {tuple(bytes.fromhex(h[1:])) for h in meta["emissive"]}
            mask.extend(tuple(px[:3]) in em and px[3] != 0 for px in np.asarray(im).reshape(-1, 4))
        assert old_em == new_em, (name, "emissive roles")
        cw, ch = new_meta["cell"]
        rows = new.height // ch
        # Separate three-row strips preserve exact 8x pixels in the image viewer.
        for first in range(0, rows, 3):
            last = min(first + 3, rows)
            h = (last - first) * ch
            for label, im in (("before", old), ("after", new)):
                crop = im.crop((0, first * ch, im.width, last * ch))
                bg = Image.new("RGBA", crop.size, "#29313f")
                bg.alpha_composite(crop)
                bg.convert("RGB").resize((im.width * 8, h * 8), Image.Resampling.NEAREST).save(
                    OUT / f"{name}_{label}_rows{first:02}_{last - 1:02}_8x.png"
                )
        evidence[name] = {
            "before_sha256": digest(old_path), "after_sha256": digest(new_path),
            "cell": new_meta["cell"], "anchor": new_meta["anchor"],
            "identical_alpha_and_animation_contract": True,
            "identical_emissive_pixel_roles": True, "colors": new_meta["colors_used"],
        }
    return evidence


def capture(binary, label):
    """Run the real game's audio-free shot path with deterministic inputs and make muted GIFs."""
    commands = []
    board = Image.new("RGB", (768, 432), "#222d3d")
    for row, (scene, pos) in enumerate(SCENES.items()):
        dest = OUT / f"{label}_{scene}"
        command = [str(binary), "shot", "--zone", "glasswater", "--pos", pos, "--god", "--seed", "7",
                   "--ticks", "480", "--every", "6", "--scale", "1", "--out", str(dest)]
        subprocess.run(command, cwd=ROOT, check=True)
        commands.append(command)
        frames = [Image.open(p).convert("RGB") for p in sorted(dest.glob("f*.png"))]
        scaled = [im.resize((1152, 648), Image.Resampling.NEAREST) for im in frames]
        scaled[0].save(OUT / f"{scene}_{label}_muted.gif", save_all=True,
                       append_images=scaled[1:], duration=100, loop=0)
        for col, tick in enumerate((96, 420)):
            source = Image.open(dest / f"f{tick:05}.png").convert("RGB")
            source.resize((1152, 648), Image.Resampling.NEAREST).save(OUT / f"{scene}_{label}_t{tick:03}_3x.png")
            board.paste(source, (col * 384, row * 216))
    name = f"enemies_glasswater_v2style_{label}"
    board.save(OUT / f"{name}.png")
    board.resize((2304, 1296), Image.Resampling.NEAREST).save(OUT / f"{name}_3x.png")
    return commands


def main():
    """Freeze the just-built binary and record review artifacts, commands and integration context."""
    OUT.mkdir(parents=True, exist_ok=True)
    binary = OUT / "glintblade_after"
    shutil.copy2(ROOT / "target/release/glintblade", binary)
    report = {"written": "2026-10-31", "sheets": compare_source(), "commands": {},
              "binary_sha256": {"before": digest(BASE / "glintblade"), "after": digest(binary)},
              "palette": "assets/concept/palettes_v2.json",
              "palette_sha256": digest(ROOT / "assets/concept/palettes_v2.json"),
              "integration": [
                  "After frames include concurrent region/hero/UI work captured in the release binary.",
                  "Shared UI color integration remains its owner's responsibility; latest captured large crystals use slate/iris.",
                  "No mechanics, timing, collision, source API or sprite geometry changes.",
                  "The near-equal slate outline/underside intentionally merge in shadow; lit upper-left shell planes remain distinct.",
                  "Exact emissive swatches change compatibly; emissive pixel roles remain identical.",
              ]}
    for label, executable in (("before", BASE / "glintblade"), ("after", binary)):
        report["commands"][label] = capture(executable, label)
    (OUT / "evidence.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
