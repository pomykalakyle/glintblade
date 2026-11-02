#!/usr/bin/env python3
"""Pack the accepted round13 native landmark and semantic normals, offline.

uv run --with pillow python tools/pixellab/round13_landmarks.py
Selected RGB pixels are copied verbatim: no resize, rotation, mirroring, palette
repair, blur, painted ground shadows or source cropping. The cave's erroneously
opaque arch aperture receives the same literal native alpha cut in art and normals.
"""

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageDraw

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


def semantic_normals(image, planes):
    """Rasterize authored geometric faces, independent of albedo brightness.

    The engine's upright basis maps tangent R/B/G to world X/Y/Z. A top
    therefore encodes green=255; blue=255 describes a south-facing wall.
    Plane polygons use the native source coordinate grid, with no resampling.
    Later faces replace earlier ones, including explicitly exposed cap strips.
    """
    import math

    assert tuple(planes["cell"]) == image.size
    labels = Image.new("I", image.size, 0)
    draw = ImageDraw.Draw(labels)
    normals = {}
    names = {}
    for index, plane in enumerate(planes["planes"], 1):
        world = plane["world_normal"]
        length = math.sqrt(sum(v * v for v in world))
        assert abs(length - 1) < 0.001, (plane["name"], length)
        normals[index] = tuple(round((world[axis] + 1) * 127.5) for axis in (0, 2, 1))
        names[index] = plane["name"]
        for polygon in plane["polygons"]:
            assert len(polygon) >= 3, plane["name"]
            assert all(0 <= x < image.width and 0 <= y < image.height for x, y in polygon)
            draw.polygon([tuple(p) for p in polygon], fill=index)
    normal = Image.new("RGBA", image.size, (128, 128, 255, 0))
    counts = dict.fromkeys(names.values(), 0)
    for y in range(image.height):
        for x in range(image.width):
            if image.getpixel((x, y))[3] == 0:
                labels.putpixel((x, y), 0)
                continue
            face = labels.getpixel((x, y))
            assert face in normals, ("unclassified opaque geometric face", x, y)
            normal.putpixel((x, y), (*normals[face], 255))
            counts[names[face]] += 1
    for sample in planes["acceptance_samples"]:
        xy = tuple(sample["xy"])
        assert normal.getpixel(xy)[3], (sample["name"], "transparent acceptance point")
        face = labels.getpixel(xy)
        assert names[face] == sample["plane"], (sample["name"], names[face], sample["plane"])
        tangent = [v / 127.5 - 1 for v in normal.getpixel(xy)[:3]]
        world = [tangent[0], tangent[2], tangent[1]]
        length = math.sqrt(sum(v * v for v in world))
        dot = sum(v / length * e for v, e in zip(world, sample["expected_world_normal"]))
        assert dot > 0.999, (sample["name"], dot)
    assert all(count > 0 for count in counts.values()), counts
    return normal, labels, counts


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
        if not spec.get("active", True):
            # A full offline rebuild must not resurrect a rejected atlas. Only
            # this exporter's exact known outputs in the requested dirs change.
            for path in (args.out / (name + ".png"), args.out / (name + ".json"), normal_out / (name + ".png")):
                path.unlink(missing_ok=True)
            print(name, "inactive:", spec["rejection_reason"])
            continue
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
            generated_normal = Image.open(normal_source).convert("RGBA")
            assert generated_normal.size == image.size, name
            assert generated_normal.getchannel("A").tobytes() == original.getchannel("A").tobytes(), (name, "original normal coverage")
            if "normal_planes" in spec:
                plane_source = ARCHIVE / spec["normal_planes"]
                planes = json.loads(plane_source.read_text())
                assert planes["source_sha256"] == sha(source), "authored face coordinates require the exact source image"
                normal, _, counts = semantic_normals(image, planes)
                data["provenance"].update({"normal_method": "authored native geometric planes, upright R/B/G world X/Y/Z",
                                           "normal_planes": spec["normal_planes"], "normal_planes_sha256": sha(plane_source),
                                           "historical_generated_normal": spec["normal"], "historical_generated_normal_sha256": sha(normal_source)})
                (args.out / (name + ".json")).write_text(json.dumps(data, indent=2) + "\n")
                print(name, "semantic face pixels", json.dumps(counts, sort_keys=True))
            else:
                normal = aperture(generated_normal, spec)
            assert normal.getchannel("A").tobytes() == image.getchannel("A").tobytes(), (name, "normal coverage")
            normal.save(normal_out / (name + ".png"))
        print(name, "background" if background else "native support", bounds)


if __name__ == "__main__":
    main()
