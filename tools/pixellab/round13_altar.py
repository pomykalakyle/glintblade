#!/usr/bin/env python3
"""Rebuild Bloomwild's native PixelLab altar and authored stone normals offline.

No image transforms or palette remaps are used: every exported opaque albedo
pixel is the identical source coordinate, including the complete lower footing.
Explicit native polygons describe stone planes in world
right/south/up axes; normal RGB encodes OpenGL right/up/toward-viewer.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "art/pixellab/2026-11-02-round13/altar"
NAME = "bloomwild_altar"
SIZE = (72, 56)
ANCHOR = (36, 25)


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encode_normal(world: list[float]) -> tuple[int, int, int, int]:
    nx, ny, nz = world
    length = math.sqrt(nx * nx + ny * ny + nz * nz)
    assert length > 0
    # World south maps to negative green for floor art.
    return tuple(round((v / length + 1) * 127.5) for v in (nx, -ny, nz)) + (255,)


def build(out: Path, normals_out: Path) -> dict:
    selection = json.loads((SOURCE / "selection.json").read_text())
    source = SOURCE / selection["source"]
    assert sha(source) == selection["source_sha256"], "archived source changed"
    raw = Image.open(source).convert("RGBA")
    assert raw.size == SIZE
    assert set(raw.getchannel("A").tobytes()) <= {0, 255}, "source alpha must already be binary"
    image = raw.copy()

    normal = Image.new("RGBA", SIZE, encode_normal([0, 0, 1]))
    draw = ImageDraw.Draw(normal)
    for plane in selection["planes"]:
        draw.polygon([tuple(p) for p in plane["polygon"]], fill=encode_normal(plane["world_normal"]))
    normal.putalpha(image.getchannel("A"))
    # Material planes have native, exact alpha; no inferred albedo relief,
    # smoothing, resizing, interpolation, rotations or generated normal noise.
    assert normal.getchannel("A").tobytes() == image.getchannel("A").tobytes()
    for y in range(SIZE[1]):
        for x in range(SIZE[0]):
            p = image.getpixel((x, y))
            if p[3]:
                assert p == raw.getpixel((x, y)), "opaque albedo lost native provenance"
    assert image.getbbox() is not None
    bbox = image.getbbox()
    assert bbox[0] >= 1 and bbox[1] >= 1 and bbox[2] <= SIZE[0]-1 and bbox[3] <= SIZE[1]-1

    out.mkdir(parents=True, exist_ok=True)
    normals_out.mkdir(parents=True, exist_ok=True)
    albedo_path = out / f"{NAME}.png"
    normal_path = normals_out / f"{NAME}.png"
    image.save(albedo_path, optimize=False)
    normal.save(normal_path, optimize=False)
    data = {
        "cell": list(SIZE),
        "anchor": list(ANCHOR),
        "emissive": [],
        "anims": {"body": {"row": 0, "frames": 1, "fps": 1}},
        "notes": (
            "Bespoke PixelLab native low limestone altar. Terrain-only; all painted "
            "surfaces stay at height zero and retain the original walkable mask "
            "and Prism position (408,176). Albedo pixels are copied unchanged at "
            "the exact source UV. Its complete footing reaches four native pixels "
            "south of the old painted plinth, on the existing dry slab. Stone top, "
            "shallow bevel and south face use explicitly "
            "authored native world planes, not albedo-derived relief. No collider, "
            "glow, transform, resizing or resampling. Archived source, prompt, "
            "selection and receipts: art/pixellab/2026-11-02-round13/altar."
        ),
    }
    (out / f"{NAME}.json").write_text(json.dumps(data, indent=2) + "\n")
    return {
        "source": str(source.relative_to(ROOT)),
        "source_sha256": sha(source),
        "albedo_sha256": sha(albedo_path),
        "normal_sha256": sha(normal_path),
        "native_size": list(SIZE),
        "anchor": list(ANCHOR),
        "bbox": list(bbox),
        "opaque_pixels": sum(a == 255 for a in image.getchannel("A").tobytes()),
        "clipped_opaque_source_pixels": 0,
        "color_count": len({p[:3] for count, p in image.getcolors(SIZE[0] * SIZE[1]) if p[3]}),
        "native_pixel_identity": True,
        "normal_alpha_matches_albedo": True,
        "physics": "Original mat::SLAB, no collider; every terrain height is zero",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "assets/sprites")
    parser.add_argument("--normals-out", type=Path, default=None)
    args = parser.parse_args()
    normals_out = args.normals_out or args.out.parent / "normals"
    print(json.dumps(build(args.out, normals_out), indent=2))


if __name__ == "__main__":
    main()
