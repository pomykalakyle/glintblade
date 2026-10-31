"""Check the frozen sprite contracts and assemble silent before/after inspection evidence."""

import hashlib
import json
import pathlib

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "out/samples/enemies_cinder_v2style"
BASE = ROOT / "out/style_v2_baseline"
NAMES = ("ember_hound", "spitting_totem", "ember_glob")


def digest(file):
    """Return the exact artifact SHA-256 for integration review."""
    return hashlib.sha256(file.read_bytes()).hexdigest()


def load(name, before=False):
    """Read a frozen or current sheet with its matching manifest."""
    folder = BASE / "sprites" if before else ROOT / "assets/sprites"
    return Image.open(folder / f"{name}.png").convert("RGBA"), json.loads((folder / f"{name}.json").read_text())


def cell(image, manifest, anim, frame):
    """Extract one cell without trimming or moving its anchor."""
    w, h = manifest["cell"]
    row = manifest["anims"][anim]["row"]
    return image.crop((frame * w, row * h, (frame + 1) * w, (row + 1) * h))


def maw(image, manifest):
    """Compute the same lowest emissive-cluster centroid used by the live totem source."""
    image = cell(image, manifest, "idle_down", 0)
    em = {tuple(bytes.fromhex(h[1:])) for h in manifest["emissive"]}
    points = [(x, y) for y in range(image.height) for x in range(image.width)
              if image.getpixel((x, y))[3] and image.getpixel((x, y))[:3] in em]
    low = max(y for _, y in points)
    points = [(x, y) for x, y in points if y >= low - 4]
    return [sum(x for x, _ in points) / len(points), sum(y for _, y in points) / len(points)]


def source_board(name, before):
    """Save native and exact 8x key-pose strips for every owned facing set."""
    image, m = load(name, before)
    if name == "ember_hound":
        rows = [[(f"idle_{d}", 0), (f"windup_{d}", 3), (f"attack_{d}", 1), (f"attack_{d}", 3)]
                for d in ("right", "down", "up")]
    elif name == "spitting_totem":
        rows = [[(f"idle_{d}", 0), (f"windup_{d}", 0), (f"windup_{d}", 4), (f"attack_{d}", 0)]
                for d in ("down", "up")]
    else:
        rows = [[(an, i) for i in range(4)] for an in ("fly", "pop")]
    w, h = m["cell"]
    board = Image.new("RGB", (w * 4, h * len(rows)), "#363340")
    for y, row in enumerate(rows):
        for x, (an, frame) in enumerate(row):
            im = cell(image, m, an, frame)
            board.paste(im, (x * w, y * h), im)
    suffix = "before" if before else "after"
    board.save(OUT / f"{name}_source_{suffix}_native.png")
    board.resize((board.width * 8, board.height * 8), Image.Resampling.NEAREST).save(OUT / f"{name}_source_{suffix}_8x.png")


def gameplay_samples():
    """Assemble identical-tick gameplay PNGs, 3x stills and audio-free motion samples."""
    for label in ("before", "after"):
        folder = OUT / f"{label}_foundry"
        native = Image.open(folder / "f00300.png").convert("RGB")
        native.save(OUT / f"enemies_cinder_v2style_{label}.png")
        native.resize((1152, 648), Image.Resampling.NEAREST).save(OUT / f"enemies_cinder_v2style_{label}_3x.png")
        for site in ("foundry", "caldera"):
            files = sorted(p for p in (OUT / f"{label}_{site}").glob("f*.png") if len(p.stem) == 6)
            frames = [Image.open(p).convert("RGB").resize((1152, 648), Image.Resampling.NEAREST) for p in files]
            frames[0].save(OUT / f"enemies_cinder_v2style_{label}_{site}_muted.gif", save_all=True,
                           append_images=frames[1:], duration=50, loop=0)
            for tick in (60, 120, 240, 360):
                source = OUT / f"{label}_{site}" / f"f{tick:05}.png"
                im = Image.open(source)
                im.resize((1152, 648), Image.Resampling.NEAREST).save(source.with_name(source.stem + "_3x.png"))


def main():
    """Verify API/geometry preservation and write reproducible review evidence."""
    OUT.mkdir(parents=True, exist_ok=True)
    evidence = {"date": "2026-10-30", "palette": "assets/concept/palettes_v2.json", "sheets": {},
                "capture": "shot --zone cinder --pos {600,490|1135,405} --seed 7 --god --ticks {360|420} --every 3 --scale 1",
                "before_binary": str(BASE / "glintblade"),
                "after_binary": str(OUT / "glintblade_after"),
                "build": "SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk cargo build --release",
                "integration": "Shared checkout captures include other owners' region/hero work; source boards isolate the owned redraw. No gameplay or shared-contract changes required. Cross-family review remains with the lead."}
    for name in NAMES:
        before, bm = load(name, True)
        after, am = load(name)
        for key in ("cell", "anchor", "anims", "footprint"):
            assert bm.get(key) == am.get(key), (name, key, "contract drift")
        assert before.size == after.size
        colors = {p[:3] for _, p in after.getcolors(after.width * after.height) if p[3]}
        evidence["sheets"][name] = {"before_sha256": digest(BASE / "sprites" / f"{name}.png"),
                                   "after_sha256": digest(ROOT / "assets/sprites" / f"{name}.png"),
                                   "manifest_sha256": digest(ROOT / "assets/sprites" / f"{name}.json"),
                                   "colors": len(colors), "contract": "unchanged cells, anchors, anims, durations and footprint"}
        if name == "spitting_totem":
            assert maw(before, bm) == maw(after, am), "projectile source origin drift"
            evidence["maw_centroid"] = maw(after, am)
        source_board(name, True)
        source_board(name, False)
    gameplay_samples()
    evidence["before_binary_sha256"] = digest(BASE / "glintblade")
    evidence["after_binary_sha256"] = digest(OUT / "glintblade_after")
    (OUT / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
