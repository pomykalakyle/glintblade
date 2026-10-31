"""Capture silent baseline/current gameplay and native-grid enemy review evidence."""

import ast
import hashlib
import json
import subprocess
from pathlib import Path

from PIL import Image

from pix import ROOT, SPRITES

OUT = ROOT / "out/samples/enemies_bloomwild_v2style"
BASE = ROOT / "out/style_v2_baseline"
NAMES = ("sproutling", "thorn_imp", "seed")
COMMANDS = []


def run(args):
    """Run and record a silent capture or media encoding command."""
    COMMANDS.append([str(a) for a in args])
    subprocess.run(COMMANDS[-1], cwd=ROOT, check=True)


def load(root, name):
    """Read an exact sheet and animation manifest from the requested snapshot."""
    return Image.open(root / f"{name}.png").convert("RGBA"), json.loads((root / f"{name}.json").read_text())


def cell(sheet, manifest, anim, index):
    """Crop one authored cell without changing its native pixels."""
    w, h = manifest["cell"]
    y = manifest["anims"][anim]["row"] * h
    return sheet.crop((index * w, y, (index + 1) * w, y + h))


def save(image, name, scales=(1, 3)):
    """Write native and integer nearest-neighbor review images."""
    for scale in scales:
        suffix = "native" if scale == 1 else f"{scale}x"
        image.resize((image.width * scale, image.height * scale), Image.Resampling.NEAREST).convert("RGB").save(OUT / f"{name}_{suffix}.png")


def source_samples(root, phase):
    """Write every source animation row and a directional release continuity strip."""
    for name in NAMES:
        sheet, manifest = load(root, name)
        w, h = manifest["cell"]
        board = Image.new("RGBA", (w * 6, h * len(manifest["anims"])), "#46505b")
        sequence, durations = [], []
        for row, (anim, meta) in enumerate(manifest["anims"].items()):
            for index in range(meta["frames"]):
                sprite = cell(sheet, manifest, anim, index)
                board.alpha_composite(sprite, (index * w, row * h))
                frame = Image.new("RGBA", (w + 16, h + 8), "#92a594")
                frame.alpha_composite(sprite, (8, 4))
                sequence.append(frame.resize((frame.width * 8, frame.height * 8), Image.Resampling.NEAREST).convert("RGB"))
                durations.append(round(meta["durations"][index] * 1000 / 60))
        for group in range((len(manifest["anims"]) + 4) // 5):
            crop = board.crop((0, group * 5 * h, board.width, min(board.height, (group + 1) * 5 * h)))
            save(crop, f"{name}_{phase}_source_{group}", (8,))
        sequence[0].save(OUT / f"{name}_{phase}_motion_muted.gif", save_all=True, append_images=sequence[1:], duration=durations, loop=0, disposal=2)
    sheet, manifest = load(root, "thorn_imp")
    seed, sm = load(root, "seed")
    panel = Image.new("RGBA", (240, 180), "#92a594")
    ax, ay = manifest["anchor"]
    sx, sy = sm["anchor"]
    for row, direction in enumerate(("right", "down", "up")):
        anim = f"attack_{direction}"
        rx, ry = manifest["anims"][anim]["release_offset"]
        for column in range(5):
            x, y = 24 + column * 48, 42 + row * 60
            panel.alpha_composite(cell(sheet, manifest, anim, min(column, 1)), (x - ax, y - ay))
            if column:
                dx, dy = {"right": (8, 0), "down": (0, 5), "up": (0, -5)}[direction]
                panel.alpha_composite(cell(seed, sm, "fly", (column - 1) % 4), (x + rx - sx + (column - 1) * dx, y + ry - sy + (column - 1) * dy))
    save(panel, f"release_{phase}", (1, 8))


def structural(value):
    """Extract animation contracts apart from prose and intentional emitter removal."""
    if isinstance(value, dict):
        return {key: structural(item) for key, item in value.items() if key not in ("notes", "emissive")}
    if isinstance(value, list):
        return [structural(item) for item in value]
    return value


def main():
    """Capture both snapshots, validate contracts and record review evidence hashes."""
    OUT.mkdir(parents=True, exist_ok=True)
    for name in NAMES:
        _, old = load(BASE / "sprites", name)
        sheet, new = load(SPRITES, name)
        assert structural(old) == structural(new), f"changed animation contract: {name}"
        assert new["emissive"] == []
        assert set(sheet.getchannel("A").getdata()) <= {0, 255}
    for filename in ("sproutling.py", "imp.py", "seed.py", "pix.py", "v2_samples.py"):
        tree = ast.parse((Path(__file__).parent / filename).read_text())
        assert all(ast.get_docstring(node) for node in ast.walk(tree) if isinstance(node, ast.FunctionDef)), filename
    for phase, binary, sprites in (("before", BASE / "glintblade", BASE / "sprites"), ("after", ROOT / "target/release/glintblade", SPRITES)):
        source_samples(sprites, phase)
        for scene, pos in (("imp", "500,448"), ("meadow", "448,500")):
            frames = OUT / f"{scene}_{phase}_frames"
            run([binary, "shot", "--zone", "bloomwild", "--pos", pos, "--god", "--seed", "7", "--ticks", "360", "--every", "4", "--scale", "1", "--out", frames, "--report", OUT / f"{scene}_{phase}_report.json"])
            image = Image.open(frames / "f00300.png")
            title = f"enemies_bloomwild_v2style_{phase}" if scene == "imp" else f"meadow_{phase}"
            save(image, title)
            if scene == "imp":
                contact = Image.new("RGB", (384 * 3, 216 * 2))
                for i, tick in enumerate((40, 80, 120, 200, 280, 360)):
                    contact.paste(Image.open(frames / f"f{tick:05}.png"), ((i % 3) * 384, (i // 3) * 216))
                save(contact, f"combat_contact_{phase}", (1,))
                run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-framerate", "15", "-pattern_type", "glob", "-i", frames / "f*.png", "-vf", "scale=1152:648:flags=neighbor", "-an", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-y", OUT / f"enemies_bloomwild_v2style_{phase}_muted.mp4"])
    hashes = {str(root / f"{name}.{ext}"): hashlib.sha256((root / f"{name}.{ext}").read_bytes()).hexdigest() for root in (SPRITES, BASE / "sprites") for name in NAMES for ext in ("png", "json")}
    hashes.update({str(binary): hashlib.sha256(binary.read_bytes()).hexdigest() for binary in (BASE / "glintblade", ROOT / "target/release/glintblade")})
    report = {"commands": COMMANDS, "sha256": hashes, "palette": "assets/concept/palettes_v2.json", "contracts": "unchanged cells, anchors, animations, durations, airborne/hit/release metadata; emissive schema retained with empty swatches", "integration_dependency": "src/entities/projectiles.rs::pop unconditionally calls emissive sparks; no per-projectile switch exists, and seed.hit is not played by this shared path. Flight light/trail and owned emitters are disabled. Request shared projectile owner review rather than modifying unowned source."}
    (OUT / "evidence.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
