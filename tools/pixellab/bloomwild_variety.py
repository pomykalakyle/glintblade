#!/usr/bin/env python3
"""Append selected native Bloomwild variants and matching normals, entirely offline.

The frozen baseline is authoritative. Re-running this exporter never reads a
previous export, so append-only atlases cannot accumulate duplicate variants.
"""

import argparse
from copy import deepcopy
from dataclasses import dataclass
import hashlib
from io import BytesIO
import json
import math
from pathlib import Path

from PIL import Image, ImageDraw

from native_collision import add_collision

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "art/pixellab/2026-11-02-variety"
FAMILIES = {
    "leaves": ("bloomwild_grass", (32, 32), None, None),
    "columns": ("bloomwild_ruins", (48, 56), "pillar_broken_short", 10),
    "walls": ("bloomwild_ruins", (48, 56), "wall_low", 8),
}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def _json_bytes(data):
    return (json.dumps(data, indent=2, sort_keys=True) + "\n").encode()


def _pixels(image):
    channels = iter(image.tobytes())
    return zip(channels, channels, channels, channels)


def _binary_mask(image, label):
    if not set(image.getchannel("A").tobytes()) <= {0, 255}:
        raise ValueError(f"{label}: alpha must be binary; no interpolated pixels")


def validate_pair(art, normal, label="sprite"):
    """Validate full exported pairs, including every occupied lighting texel."""
    if art.size != normal.size:
        raise ValueError(f"{label}: art/normal dimensions differ")
    _binary_mask(art, label)
    if art.getchannel("A").tobytes() != normal.getchannel("A").tobytes():
        raise ValueError(f"{label}: art/normal alpha masks differ")
    lengths, blue, occupied = [], [], 0
    for r, g, b, a in _pixels(normal):
        if not a:
            continue
        occupied += 1
        if b < 128:
            raise ValueError(f"{label}: opaque normal points behind front hemisphere")
        length = math.sqrt(sum((v / 127.5 - 1) ** 2 for v in (r, g, b)))
        if not 0.8 <= length <= 1.2:
            raise ValueError(f"{label}: missing or non-unit opaque normal ({r}, {g}, {b})")
        lengths.append(length)
        blue.append(b)
    if not occupied:
        raise ValueError(f"{label}: empty sprite")
    return {"opaque_pixels": occupied, "minimum_normal_blue": min(blue),
            "normal_length_range": [round(min(lengths), 6), round(max(lengths), 6)],
            "alpha_matches": True}


def translate_pair(art, normal, offset=(0, 0), label="sprite"):
    """Translate both native grids identically, refusing to crop occupied art.

    Extra normal coverage outside the sprite is removed. Missing coverage inside
    the sprite is an error, rather than silently lighting it with a zero normal.
    """
    if art.size != normal.size:
        raise ValueError(f"{label}: art/normal dimensions differ")
    if (not isinstance(offset, (tuple, list)) or len(offset) != 2
            or any(type(v) is not int for v in offset)):
        raise ValueError(f"{label}: offset must contain two integers")
    art, normal = art.convert("RGBA"), normal.convert("RGBA")
    _binary_mask(art, label)
    for a, n in zip(art.getchannel("A").tobytes(), normal.getchannel("A").tobytes()):
        if a and not n:
            raise ValueError(f"{label}: opaque art pixel has no source normal")
    normal.putalpha(art.getchannel("A"))
    bounds = art.getchannel("A").getbbox()
    if bounds is None:
        raise ValueError(f"{label}: empty sprite")
    dx, dy = offset
    if (bounds[0] + dx < 0 or bounds[1] + dy < 0
            or bounds[2] + dx > art.width or bounds[3] + dy > art.height):
        raise ValueError(f"{label}: offset would clip opaque native pixels")
    translated = []
    for source in (art, normal):
        image = Image.new("RGBA", source.size)
        image.paste(source, (dx, dy))
        translated.append(image)
    return tuple(translated)


def validate_normal_samples(art, normal, planes, offset=(0, 0), label="sprite"):
    """Verify authored stone faces in the engine's upright R/B/G world basis.

    Plane polygons have already been applied by the archived source-preparation
    recipe. This checks the final native pixels, including translation and any
    preserved foot replacement, without modifying normals or art.
    """
    if not isinstance(planes.get("planes"), list) or not planes["planes"]:
        raise ValueError(f"{label}: normal planes must contain authored faces")
    samples, dots = [], []
    for plane in planes["planes"]:
        name = plane["name"]
        if not isinstance(name, str) or not name:
            raise ValueError(f"{label}: normal plane requires a name")
        if not isinstance(plane.get("samples"), list) or not plane["samples"]:
            raise ValueError(f"{label}/{name}: normal plane has no semantic samples")
        for sample in plane["samples"]:
            source_xy = sample["xy"]
            if (not isinstance(source_xy, (list, tuple)) or len(source_xy) != 2
                    or any(type(value) is not int for value in source_xy)):
                raise ValueError(f"{label}/{name}: normal sample xy must contain two integers")
            xy = [source_xy[axis] + offset[axis] for axis in (0, 1)]
            if not (0 <= xy[0] < art.width and 0 <= xy[1] < art.height):
                raise ValueError(f"{label}/{name}: translated normal sample lies outside native cell")
            if not art.getpixel(tuple(xy))[3] or not normal.getpixel(tuple(xy))[3]:
                raise ValueError(f"{label}/{name}: normal sample lies on transparent art")
            expected = sample.get("expected_world_normal", plane["world_normal"])
            if (not isinstance(expected, (list, tuple)) or len(expected) != 3
                    or any(type(value) not in (int, float) or not math.isfinite(value) for value in expected)):
                raise ValueError(f"{label}/{name}: expected world normal must be a finite 3-vector")
            length = math.sqrt(sum(value * value for value in expected))
            if abs(length - 1) >= 0.001:
                raise ValueError(f"{label}/{name}: expected world normal must have unit length")
            expected = [value / length for value in expected]
            tangent = [value / 127.5 - 1 for value in normal.getpixel(tuple(xy))[:3]]
            world = [tangent[0], tangent[2], tangent[1]]
            actual_length = math.sqrt(sum(value * value for value in world))
            dot = sum(actual / actual_length * target for actual, target in zip(world, expected))
            if dot <= 0.999:
                raise ValueError(f"{label}/{name}: semantic normal sample {xy} differs from expected plane (dot={dot:.6f})")
            samples.append({"xy": xy, "world_normal": expected, "plane": name})
            dots.append(dot)
    return samples, min(dots)


def apply_shared_foot_normals(art, normal, planes, first_row, label):
    """Rasterize only explicitly authored opaque support pixels on native grid."""
    corrected = normal.copy()
    allowed = Image.new("1", art.size)
    if not isinstance(planes.get("planes"), list) or not planes["planes"]:
        raise ValueError(f"{label}: shared foot profile requires authored planes")
    for plane in planes["planes"]:
        vector = plane["world_normal"]
        if (len(vector) != 3 or any(type(v) not in (int, float) or not math.isfinite(v) for v in vector)
                or abs(math.sqrt(sum(v * v for v in vector)) - 1) >= 0.001):
            raise ValueError(f"{label}: shared foot plane must have a unit world normal")
        encoded = tuple(round((vector[axis] + 1) * 127.5) for axis in (0, 2, 1)) + (255,)
        polygons = plane.get("polygons")
        if not isinstance(polygons, list) or not polygons:
            raise ValueError(f"{label}: shared foot plane requires native polygons")
        mask = Image.new("1", art.size)
        for polygon in polygons:
            if (len(polygon) < 3 or any(not isinstance(xy, (list, tuple)) or len(xy) != 2
                    or any(type(v) is not int for v in xy)
                    or not (0 <= xy[0] < art.width and first_row <= xy[1] < art.height)
                    for xy in polygon)):
                raise ValueError(f"{label}: shared foot polygon exceeds permitted native foot bounds")
            ImageDraw.Draw(mask).polygon([tuple(xy) for xy in polygon], fill=1)
        for sample in plane.get("samples", []):
            xy = sample["xy"]
            if (len(xy) != 2 or any(type(v) is not int for v in xy)
                    or not (0 <= xy[0] < art.width and first_row <= xy[1] < art.height)
                    or not mask.getpixel(tuple(xy))):
                raise ValueError(f"{label}: shared foot sample exceeds authored native foot polygon")
        for y in range(first_row, art.height):
            for x in range(art.width):
                if mask.getpixel((x, y)) and art.getpixel((x, y))[3]:
                    corrected.putpixel((x, y), encoded)
                    allowed.putpixel((x, y), 1)
    corrected.putalpha(art.getchannel("A"))
    validate_pair(art, corrected, label)
    samples, minimum_dot = validate_normal_samples(art, corrected, planes, label=label)
    return corrected, allowed, samples, minimum_dot


def _differences(images):
    """Report pairwise diversity without turning an aesthetic metric into a gate."""
    pairs = []
    for i, (left_name, left) in enumerate(images):
        for right_name, right in images[i + 1:]:
            union = overlap = changed = 0
            for a, b in zip(_pixels(left), _pixels(right)):
                if a[3] or b[3]:
                    union += 1
                    overlap += bool(a[3] and b[3])
                    changed += a != b
            pairs.append({"a": left_name, "b": right_name,
                          "silhouette_iou": round(overlap / union, 6),
                          "visible_pixel_change_fraction": round(changed / union, 6)})
    return pairs


@dataclass
class Prepared:
    sprites: dict
    normals: dict
    metadata: dict
    report: dict


def prepare(archive=ARCHIVE, selection=None):
    """Build and validate in memory; an explicit selection dict needs no file.

    Archive layout: baseline/{sprites,normals}/bloomwild_{grass,ruins}.png,
    baseline/sprites/*.json, and selection.json. Selection has leaves, columns,
    walls arrays of {name, image, normal, offset?, preserve_from_row?,
    normal_planes?}. Per-variant normal-plane files are verified, not applied.
    Optional shared_feet profiles rasterize only the two established support
    regions, with exact original art/alpha and other normal texels retained.
    Paths are relative to this archive. No generated output is used as a source.
    """
    archive = Path(archive).resolve()
    sources = {}

    def read(relative):
        path = (archive / relative).resolve()
        if not path.is_relative_to(archive):
            raise ValueError(f"Source escapes archive: {relative}")
        data = path.read_bytes()
        sources[path.relative_to(archive).as_posix()] = sha256(data)
        return data

    def image(relative):
        return Image.open(BytesIO(read(relative))).convert("RGBA")

    if selection is None:
        selection = json.loads(read("selection.json"))
    selection = deepcopy(selection)
    unknown = set(selection) - {"leaves", "columns", "walls", "notes", "shared_feet"}
    if unknown:
        raise ValueError(f"Unknown selection families: {sorted(unknown)}")
    sprites, normals, metadata = {}, {}, {}
    for name in ("bloomwild_grass", "bloomwild_ruins"):
        sprites[name] = image(f"baseline/sprites/{name}.png")
        normals[name] = image(f"baseline/normals/{name}.png")
        metadata[name] = json.loads(read(f"baseline/sprites/{name}.json"))
        validate_pair(sprites[name], normals[name], f"baseline {name}")
    expected = {"bloomwild_grass": ((256, 32), [32, 32], [16, 28]),
                "bloomwild_ruins": ((48, 504), [48, 56], [24, 53])}
    for name, (size, cell, anchor) in expected.items():
        if (sprites[name].size != size or metadata[name]["cell"] != cell
                or metadata[name]["anchor"] != anchor):
            raise ValueError(f"{name}: unexpected frozen baseline dimensions or anchor")
    if metadata["bloomwild_grass"]["anims"]["body"]["frames"] != 8:
        raise ValueError("Frozen leaf baseline must have eight frames")
    hashes_path = archive / "baseline/hashes.json"
    if hashes_path.exists():
        expected_hashes = json.loads(read("baseline/hashes.json"))
        for relative, digest in expected_hashes.items():
            data = read(f"baseline/{relative}")
            if sha256(data) != digest:
                raise ValueError(f"Frozen baseline checksum mismatch: {relative}")

    baseline_sprites = {name: im.copy() for name, im in sprites.items()}
    baseline_normals = {name: im.copy() for name, im in normals.items()}
    baseline_metadata = deepcopy(metadata)
    shared_feet = selection.get("shared_feet", [])
    if not isinstance(shared_feet, list):
        raise ValueError("shared_feet must be an array")
    foot_profiles, foot_metadata, foot_templates = [], [], set()
    allowed_normal_changes = Image.new("1", normals["bloomwild_ruins"].size)
    for spec in shared_feet:
        template, first_row = spec["template"], spec["first_row"]
        if (template not in {"pillar_broken_short": 40, "wall_low": 45}
                or type(first_row) is not int
                or first_row != {"pillar_broken_short": 40, "wall_low": 45}[template]
                or template in foot_templates):
            raise ValueError("shared_feet may correct each short-column/low-wall foot once at its established first row")
        foot_templates.add(template)
        row = metadata["bloomwild_ruins"]["anims"][template]["row"]
        box = (0, row * 56, 48, (row + 1) * 56)
        reference = image(spec["image"])
        if (reference.size != (48, 56)
                or reference.tobytes() != baseline_sprites["bloomwild_ruins"].crop(box).tobytes()):
            raise ValueError(f"{template}: shared foot reference must equal exact baseline art cell")
        plane_bytes = read(spec["normal_planes"])
        planes = json.loads(plane_bytes)
        source_key = (archive / spec["image"]).resolve().relative_to(archive).as_posix()
        if planes["source_sha256"] != sources[source_key]:
            raise ValueError(f"{template}: shared foot source SHA256 does not match native art")
        original = baseline_normals["bloomwild_ruins"].crop(box)
        corrected, allowed, samples, minimum_dot = apply_shared_foot_normals(
            reference, original, planes, first_row, template)
        normals["bloomwild_ruins"].paste(corrected, (0, row * 56))
        allowed_normal_changes.paste(allowed, (0, row * 56))
        profile = {"template": template, "first_row": first_row,
                   "normal_planes": spec["normal_planes"], "normal_planes_sha256": sha256(plane_bytes),
                   "samples": samples}
        foot_metadata.append(profile)
        foot_profiles.append({**profile, "image": spec["image"],
                              "changed_pixels": sum(a != b for a, b in zip(_pixels(original), _pixels(corrected))),
                              "allowed_opaque_polygon_pixels": allowed.convert("L").histogram()[255],
                              "normal_sample_minimum_dot": round(minimum_dot, 9)})
    if foot_metadata:
        metadata["bloomwild_ruins"]["shared_foot_normals"] = foot_metadata
    variants, family_report = {}, {}
    existing_names = set(metadata["bloomwild_ruins"]["anims"])
    for family, (sheet, size, template, depth) in FAMILIES.items():
        specs = selection.get(family, [])
        if not isinstance(specs, list):
            raise ValueError(f"{family}: selection must be an array")
        if template:
            row = metadata[sheet]["anims"][template]["row"]
            box = (0, row * size[1], size[0], (row + 1) * size[1])
            reference = baseline_sprites[sheet].crop(box)
            reference_normal = normals[sheet].crop(box)
            comparisons = [(template, reference)]
        else:
            comparisons = [(f"baseline_leaf_{i}", baseline_sprites[sheet].crop(
                (i * 32, 0, (i + 1) * 32, 32))) for i in range(8)]
        entries, prepared = [], []
        family_names = set()
        for spec in specs:
            name = spec["name"]
            if not isinstance(name, str) or not name or name in family_names:
                raise ValueError(f"{family}: missing or duplicate variant name {name!r}")
            if template and name in existing_names:
                raise ValueError(f"{family}: variant would overwrite animation {name}")
            family_names.add(name)
            existing_names.add(name)
            art, normal = image(spec["image"]), image(spec["normal"])
            if art.size != size or normal.size != size:
                raise ValueError(f"{name}: native sources must be {size}, received {art.size}/{normal.size}")
            offset = spec.get("offset", [0, 0])
            art, normal = translate_pair(art, normal, offset, name)
            preserve = spec.get("preserve_from_row")
            if preserve is not None:
                if not template or type(preserve) is not int or not 0 <= preserve < size[1]:
                    raise ValueError(f"{name}: invalid preserved base row")
                box = (0, preserve, size[0], size[1])
                art.paste(reference.crop(box), (0, preserve))
                normal.paste(reference_normal.crop(box), (0, preserve))
            stats = validate_pair(art, normal, name)
            samples, plane_audit = None, {}
            if "normal_planes" in spec:
                if not template:
                    raise ValueError(f"{name}: upright normal planes apply only to stone variants")
                plane_bytes = read(spec["normal_planes"])
                planes = json.loads(plane_bytes)
                source_key = (archive / spec["image"]).resolve().relative_to(archive).as_posix()
                if planes["source_sha256"] != sources[source_key]:
                    raise ValueError(f"{name}: normal-plane source SHA256 does not match native art")
                samples, minimum_dot = validate_normal_samples(art, normal, planes, offset, name)
                plane_audit = {"normal_planes": spec["normal_planes"],
                               "normal_planes_sha256": sha256(plane_bytes),
                               "normal_sample_count": len(samples),
                               "normal_sample_minimum_dot": round(minimum_dot, 9)}
            anim = None
            if template:
                anim = {"frames": 1, "fps": 1}
                if samples is not None:
                    anim["normal_samples"] = samples
                add_collision(anim, art, depth)
                if not anim["collision_rects"]:
                    raise ValueError(f"{name}: structural sprite has no opaque base collision")
                # Preserving the support must also preserve its exact contacts.
                if preserve is not None and preserve <= size[1] - depth - 2:
                    if anim["collision_rects"] != metadata[sheet]["anims"][template]["collision_rects"]:
                        raise ValueError(f"{name}: preserved native foot changed collision")
            entry = {"name": name, "image": spec["image"], "normal": spec["normal"],
                     "offset": list(offset), "preserve_from_row": preserve, **stats, **plane_audit}
            if anim:
                entry["collision_rectangles"] = len(anim["collision_rects"])
            entries.append(entry)
            prepared.append((name, art, normal, anim))
            comparisons.append((name, art))
        variants[family] = prepared
        family_report[family] = {"added_count": len(prepared), "variants": entries,
                                 "diversity": _differences(comparisons)}

    leaves = variants["leaves"]
    grass = "bloomwild_grass"
    for maps in (sprites, normals):
        atlas = Image.new("RGBA", (32 * (8 + len(leaves)), 32))
        atlas.paste(maps[grass], (0, 0))
        for i, (_, art, normal, _) in enumerate(leaves, 8):
            atlas.paste(art if maps is sprites else normal, (i * 32, 0))
        maps[grass] = atlas
    metadata[grass]["anims"]["body"]["frames"] += len(leaves)
    stone = variants["columns"] + variants["walls"]
    ruins = "bloomwild_ruins"
    for maps in (sprites, normals):
        atlas = Image.new("RGBA", (48, 56 * (9 + len(stone))))
        atlas.paste(maps[ruins], (0, 0))
        for i, (_, art, normal, _) in enumerate(stone, 9):
            atlas.paste(art if maps is sprites else normal, (0, i * 56))
        maps[ruins] = atlas
    for i, (name, _, _, anim) in enumerate(stone, 9):
        metadata[ruins]["anims"][name] = {"row": i, **anim}

    # Check entire legacy regions, including transparent RGB. Only explicitly
    # authorized opaque support polygons may change existing normal texels.
    for name, baseline in baseline_sprites.items():
        box = (0, 0, *baseline.size)
        if sprites[name].crop(box).tobytes() != baseline.tobytes():
            raise ValueError(f"{name}: legacy albedo pixels changed")
        for index, (actual, original) in enumerate(zip(_pixels(normals[name].crop(box)), _pixels(baseline_normals[name]))):
            if actual != original:
                xy = (index % baseline.width, index // baseline.width)
                if name != "bloomwild_ruins" or not allowed_normal_changes.getpixel(xy):
                    raise ValueError(f"{name}: legacy normal pixel changed outside explicit shared foot polygon")
        for animation, original in baseline_metadata[name]["anims"].items():
            expected_anim = deepcopy(original)
            if name == grass and animation == "body":
                expected_anim["frames"] += len(leaves)
            if metadata[name]["anims"][animation] != expected_anim:
                raise ValueError(f"{name}/{animation}: legacy animation metadata changed")
        validate_pair(sprites[name], normals[name], f"export {name}")

    changed_normals = sum(profile["changed_pixels"] for profile in foot_profiles)
    report = {"schema": 1, "selection_sha256": sha256(_json_bytes(selection)),
              "source_sha256": dict(sorted(sources.items())), "families": family_report,
              "legacy_pixels_preserved": changed_normals == 0,
              "legacy_albedo_pixels_preserved": True,
              "legacy_normal_pixels_preserved_except_shared_feet": True,
              "legacy_normal_changed_pixels": changed_normals,
              "shared_foot_normals": foot_profiles,
              "legacy_animation_metadata_preserved": True,
              "allowed_metadata_change": "bloomwild_grass/body.frames extends for appended leaves",
              "dimensions": {name: list(im.size) for name, im in sprites.items()},
              "processing": "Native integer translation shared by art and normals; exact optional baseline foot replacement. Explicit shared-foot polygon profiles may correct ONLY short-column and low-wall original support normals, inherited by their variants. All original albedo/alpha, other normal pixels and animation fields preserved. No resizing, filtering, mirroring, rotation, palette changes, or API calls."}
    return Prepared(sprites, normals, metadata, report)


def build(archive=ARCHIVE, out=None, normals_out=None, selection=None, report_path=None):
    """Validate everything before writing either atlas; return the saved audit."""
    prepared = prepare(archive, selection)
    out = Path(out) if out is not None else ROOT / "assets/sprites"
    normals_out = Path(normals_out) if normals_out is not None else out.parent / "normals"
    report_path = Path(report_path) if report_path is not None else out.parent / "bloomwild-variety-report.json"
    out.mkdir(parents=True, exist_ok=True)
    normals_out.mkdir(parents=True, exist_ok=True)
    outputs = {}
    for name in prepared.sprites:
        for label, folder, im in (("sprites", out, prepared.sprites[name]),
                                  ("normals", normals_out, prepared.normals[name])):
            path = folder / f"{name}.png"
            im.save(path)
            outputs[f"{label}/{path.name}"] = sha256(path.read_bytes())
        path = out / f"{name}.json"
        path.write_bytes(_json_bytes(prepared.metadata[name]))
        outputs[f"sprites/{path.name}"] = sha256(path.read_bytes())
    prepared.report["output_sha256"] = dict(sorted(outputs.items()))
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_bytes(_json_bytes(prepared.report))
    return prepared.report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ARCHIVE)
    parser.add_argument("--selection", type=Path, help="Optional alternate selection JSON")
    parser.add_argument("--out", type=Path, default=ROOT / "assets/sprites")
    parser.add_argument("--normals-out", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    selection = json.loads(args.selection.read_text()) if args.selection else None
    report = build(args.archive, args.out, args.normals_out, selection, args.report)
    # Keep prepare/build's exact archived-source validation available to fixtures.
    # The shipped CLI then applies the authored RGB-only ground-contact finalizer
    # to every original and appended shared foot, without rewriting its normals
    # or metadata. Never rerun the umbrella environment exporter for this pass.
    from bloomwild_structure_contacts import export_contacts
    contact_report = export_contacts(args.out, args.out, ['bloomwild_ruins'])
    report['contact_ink'] = contact_report['families']['bloomwild_ruins']
    report['legacy_albedo_pixels_preserved_except_authored_contact_ink'] = True
    report['legacy_albedo_pixels_preserved'] = False
    report['legacy_pixels_preserved'] = False
    report['output_sha256']['sprites/bloomwild_ruins.png'] = sha256(
        (args.out / 'bloomwild_ruins.png').read_bytes())
    report_path = args.report or args.out.parent / 'bloomwild-variety-report.json'
    report_path.write_bytes(_json_bytes(report))
    counts = ", ".join(f"{key}: +{value['added_count']}" for key, value in report["families"].items())
    print(f"Exported native Bloomwild variants ({counts}); "
          f"{report['contact_ink']['changed_pixels']} authored contact-ink texels corrected, "
          f"{report['legacy_normal_changed_pixels']} shared-foot normal texels corrected.")


if __name__ == "__main__":
    main()
