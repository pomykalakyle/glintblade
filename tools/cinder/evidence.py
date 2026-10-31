"""Capture reproducible, silent before/after gameplay and native-grid asset review evidence."""

import hashlib
import json
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "out/samples/cinder_v2style"
VIEWS = {"entry": "136,752", "river": "370,455", "foundry": "620,520", "caldera": "1144,280"}


def digest(path):
    """Return the exact SHA-256 of one build, source, palette or sheet."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(args, commands):
    """Run one silent renderer command and record its exact arguments."""
    commands.append(args)
    subprocess.run(args, cwd=ROOT, check=True)


def captures():
    """Capture identical scenes and seeded combat with the frozen and current binaries."""
    OUT.mkdir(parents=True, exist_ok=True)
    commands = []
    for phase, binary in (("before", "out/style_v2_baseline/glintblade"), ("after", "target/release/glintblade")):
        for name, pos in VIEWS.items():
            dest = f"out/samples/cinder_v2style/cinder_v2style_{phase}_{name}"
            run([binary, "zoneview", "cinder", "--at", pos, "--scale", "1", "--out", dest], commands)
        run([binary, "shot", "--zone", "cinder", "--pos", "575,510", "--god", "--seed", "7", "--script", "A2 W10 A2 W10 A2 W50", "--ticks", "90", "--at", "1,24,60,90", "--scale", "1", "--out", f"out/samples/cinder_v2style/cinder_v2style_{phase}_combat"], commands)
        dest = f"out/samples/cinder_v2style/cinder_v2style_{phase}_motion"
        run([binary, "zoneview", "cinder", "--at", "1144,280", "--scale", "1", "--ticks", "96", "--every", "6", "--out", dest], commands)
        frames = []
        for p in sorted((ROOT / dest).glob("*.png")):
            if "_3x" in p.stem:
                continue
            im = Image.open(p).convert("RGB")
            im.resize((1152, 648), Image.Resampling.NEAREST).save(p.with_name(p.stem + "_3x.png"))
            frames.append(im.resize((768, 432), Image.Resampling.NEAREST))
        frames[0].save(OUT / f"cinder_v2style_{phase}_lava_muted.gif", save_all=True, append_images=frames[1:], duration=100, loop=0)
    # A 3x comparison per view, preserving the native scale of both versions.
    for name in VIEWS:
        board = Image.new("RGB", (2304, 676), "#16151d")
        d = ImageDraw.Draw(board)
        for i, phase in enumerate(("before", "after")):
            im = Image.open(OUT / f"cinder_v2style_{phase}_{name}/cinder_t00090.png").convert("RGB")
            big = im.resize((1152, 648), Image.Resampling.NEAREST)
            big.save(OUT / f"cinder_v2style_{phase}_{name}_3x.png")
            board.paste(big, (i * 1152, 28))
            d.text((i * 1152 + 8, 8), f"{name} {phase}: tick 90, native pixels x3", fill="#a79a98")
        board.save(OUT / f"cinder_v2style_before_after_{name}_3x.png")
    sources = [ROOT / "src/regions/cinder.rs", *sorted((ROOT / "src/regions/cinder").glob("*.rs"))]
    sheets = sorted((ROOT / "assets/sprites").glob("cinder_*.*"))
    report = {
        "written": "2026-10-31, Pacific time",
        "build": "SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk cargo build --release",
        "commands": commands,
        "binary_sha256": {name: digest(ROOT / binary) for name, binary in (("before", "out/style_v2_baseline/glintblade"), ("after", "target/release/glintblade"))},
        "sheet_and_source_sha256": {str(p.relative_to(ROOT)): digest(p) for p in [*sheets, *sources]},
        "palette": "assets/concept/palettes_v2.json",
        "palette_sha256": digest(ROOT / "assets/concept/palettes_v2.json"),
        "custom_soot": ["#101017", "#16151d", "#1a1923", "#24232d"],
        "integration_limits": ["Shared build contains concurrent owners' current actor, UI and other-region revisions.", "Cinder creatures are owned separately; this pass does not restyle them.", "Vents, trees, braziers, small props and ruin sheets are not newly placed in the region; no new collision or effect layers were added.", "Cross-family review remains with Claude lead; no window, browser or audio was used."]
    }
    (OUT / "cinder_v2style_evidence.json").write_text(json.dumps(report, indent=2) + "\n")


def assets():
    """Write 8x sheets and a contact board of each animation's first frame."""
    cells = []
    for p in sorted((ROOT / "assets/sprites").glob("cinder_*.png")):
        meta = json.loads(p.with_suffix(".json").read_text())
        im = Image.open(p).convert("RGBA")
        bg = Image.new("RGBA", im.size, "#363340")
        bg.alpha_composite(im)
        bg.convert("RGB").resize((im.width * 8, im.height * 8), Image.Resampling.NEAREST).save(OUT / f"cinder_v2style_after_{p.stem[7:]}_8x.png")
        w, h = meta["cell"]
        for name, a in meta["anims"].items():
            cell = Image.new("RGBA", (64, 64), "#363340")
            cell.alpha_composite(im.crop((0, a["row"] * h, w, (a["row"] + 1) * h)), (32 - w // 2, 64 - h))
            cells.append((name, cell.convert("RGB").resize((256, 256), Image.Resampling.NEAREST)))
    board = Image.new("RGB", (256 * 6, 280 * ((len(cells) + 5) // 6)), "#16151d")
    d = ImageDraw.Draw(board)
    for i, (name, im) in enumerate(cells):
        x, y = i % 6 * 256, i // 6 * 280
        board.paste(im, (x, y + 24))
        d.text((x + 6, y + 6), name, fill="#a79a98")
    board.save(OUT / "cinder_v2style_after_asset_contact_4x.png")


if __name__ == "__main__":
    captures()
    assets()
