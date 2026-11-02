#!/usr/bin/env python3
"""Pack reviewed round13 native PixelLab landmarks and OpenGL normals, offline.

uv run --with pillow python tools/pixellab/round13_landmarks.py
Selected RGB pixels are copied verbatim: no resize, rotation, mirroring, palette
repair, blur, painted ground shadows or source cropping. The cave's erroneously
opaque arch aperture receives the same literal native alpha cut in art and normals.
"""

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image

from native_collision import add_collision

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "art/pixellab/2026-11-02-round13/landmarks"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def aperture(image, spec):
    """Remove only documented false opaque aperture pixels; keep native RGB."""
    for x0, y0, x1, y1 in spec.get("alpha_cutouts", []):
        assert 0 <= x0 < x1 <= image.width and 0 <= y0 < y1 <= image.height
        for y in range(y0, y1):
            for x in range(x0, x1):
                r, g, b, _ = image.getpixel((x, y))
                image.putpixel((x, y), (r, g, b, 0))
    return image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "assets/sprites")
    parser.add_argument("--normals-out", type=Path)
    args = parser.parse_args()
    normal_out = args.normals_out or args.out.parent / "normals"
    args.out.mkdir(parents=True, exist_ok=True)
    normal_out.mkdir(parents=True, exist_ok=True)
    selection = json.loads((ARCHIVE / "selection.json").read_text())
    for key, spec in selection["assets"].items():
        name = "round13_landmarks_" + key
        source = ARCHIVE / spec["source"]
        original = Image.open(source).convert("RGBA")
        image = aperture(original.copy(), spec)
        assert image.size == tuple(spec["cell"]), (name, image.size)
        assert set(image.getchannel("A").tobytes()) <= {0, 255}, name
        bounds = image.getbbox()
        assert bounds is not None, name
        anim = {"row": 0, "frames": 1, "fps": 1}
        background = spec.get("background", False)
        if not background:
            assert bounds[0] >= 1 and bounds[1] >= 1, (name, bounds)
            assert bounds[2] < image.width and bounds[3] < image.height, (name, bounds)
            assert spec["anchor"][1] == bounds[3], (name, "ground anchor must follow real foot")
            depth = spec["support_depth"]
            add_collision(anim, image, depth)
            anim["support_first_row"] = bounds[3] - depth
        prefix = "Native PixelLab Pro RGB; documented aperture alpha repair with identically masked normals. " if spec.get("alpha_cutouts") else "Unchanged native PixelLab Pro RGBA. "
        data = {"cell": list(image.size), "anchor": spec["anchor"], "anims": {"body": anim},
                "background": background,
                "notes": prefix + spec["selection_note"],
                "provenance": {"source": spec["source"], "source_sha256": sha(source),
                               "alpha_cutouts": spec.get("alpha_cutouts", []),
                               "archive": "art/pixellab/2026-11-02-round13/landmarks/"}}
        image.save(args.out / (name + ".png"))
        (args.out / (name + ".json")).write_text(json.dumps(data, indent=2) + "\n")
        if not background:
            normal_source = ARCHIVE / spec["normal"]
            normal = Image.open(normal_source).convert("RGBA")
            assert normal.size == image.size, name
            assert normal.getchannel("A").tobytes() == original.getchannel("A").tobytes(), (name, "original normal coverage")
            normal = aperture(normal, spec)
            assert normal.getchannel("A").tobytes() == image.getchannel("A").tobytes(), (name, "normal coverage")
            normal.save(normal_out / (name + ".png"))
        print(name, "background" if background else "native support", bounds)


if __name__ == "__main__":
    main()
