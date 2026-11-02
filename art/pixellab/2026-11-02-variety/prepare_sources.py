"""Replay native registration and create the normal-generator inputs; no API calls."""
from pathlib import Path
import hashlib
import json
import math
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent
LEAVES = ["leaves-arched", "leaves-split", "leaves-triple", "leaves-shallow-curl",
          "leaves-shallow-five", "leaves-shallow-left", "leaves-shallow-pair",
          "leaves-shallow-right", "leaves-shallow-spread", "leaves-shallow-young"]
COLUMNS = ["column-scallop", "column-corner", "column-hollow", "column-slant"]
WALLS = ["wall-notch", "wall-step", "wall-gap", "wall-lean", "wall-sag"]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def apply_planes(image, normal, path, end_row, first_row=0):
    """Geometric faces, never albedo-luminance bumps; fine bevels stay generated."""
    planes = json.loads(path.read_text())
    assert planes["source_sha256"] == sha(image), path
    art = Image.open(image).convert("RGBA")
    for plane in planes["planes"]:
        world = plane["world_normal"]
        assert abs(math.sqrt(sum(v * v for v in world)) - 1.0) < 0.001, plane["name"]
        # Upright stone maps tangent R/B/G into world right/south/up.
        encoded = tuple(round((world[axis] + 1.0) * 127.5) for axis in (0, 2, 1)) + (255,)
        mask = Image.new("1", normal.size)
        for polygon in plane["polygons"]:
            assert len(polygon) >= 3
            assert all(0 <= x < normal.width and first_row <= y < end_row for x, y in polygon), (path, plane["name"])
            ImageDraw.Draw(mask).polygon([tuple(xy) for xy in polygon], fill=1)
        # Transparent pixels retain their original bytes. Authored geometry can
        # cross a chipped silhouette without creating a new normal receiver.
        for y in range(first_row, end_row):
            for x in range(normal.width):
                if mask.getpixel((x, y)) and art.getpixel((x, y))[3]:
                    normal.putpixel((x, y), encoded)
    return normal


def main():
    meta = json.loads((ROOT / "baseline/sprites/bloomwild_ruins.json").read_text())
    ruins = Image.open(ROOT / "baseline/sprites/bloomwild_ruins.png").convert("RGBA")
    selection = {"leaves": [], "columns": [], "walls": [], "notes": [
        "Ten new low native leaf fans, four broken stump shapes, five low wall shapes.",
        "Rejected tall plants and a tall column fragment; no scaling, recoloring or mirroring.",
        "Integer registration preserves every accepted source pixel above each retained base.",
        "Shared original short-column and low-wall foot normals have scoped geometric corrections; art and collision stay exact."],
        "shared_feet": [
            {"template": "pillar_broken_short", "image": "references/column.png", "first_row": 40,
             "normal_planes": "normal-planes/column-foot.json"},
            {"template": "wall_low", "image": "references/wall.png", "first_row": 45,
             "normal_planes": "normal-planes/wall-foot.json"}]}
    registration = []
    inputs = {"leaves": Image.new("RGBA", (32 * len(LEAVES), 32)),
              "stone": Image.new("RGBA", (48 * (len(COLUMNS) + len(WALLS)), 56))}
    stone_index = 0
    for family, names in [("leaves", LEAVES), ("columns", COLUMNS), ("walls", WALLS)]:
        for i, key in enumerate(names):
            raw_path = ROOT / "raw" / (key + ".png")
            source = Image.open(raw_path).convert("RGBA")
            assert set(source.getchannel("A").tobytes()) <= {0, 255}, key
            box = source.getbbox()
            size = (32, 32) if family == "leaves" else (48, 56)
            bottom = 29 if family == "leaves" else 54 if family == "columns" else 53
            dy = bottom - box[3]
            assert box[1] + dy >= 0 and box[3] + dy <= size[1], key
            native = Image.new("RGBA", size)
            native.paste(source, (0, dy))
            assert sum(source.getchannel("A").tobytes()) == sum(native.getchannel("A").tobytes()), key
            preserve = None
            if family != "leaves":
                template = "pillar_broken_short" if family == "columns" else "wall_low"
                row = meta["anims"][template]["row"]
                preserve = 40 if family == "columns" else 45
                native.paste(ruins.crop((0, row * 56 + preserve, 48, (row + 1) * 56)), (0, preserve))
                name = template + "_" + chr(ord("b") + i)
                group, index = "stone", stone_index
                stone_index += 1
            else:
                name, group, index = key, "leaves", i
                assert native.getbbox()[1] >= 15, (key, "ground cover grew too tall")
            selected = ROOT / "selected" / (key + ".png")
            native.save(selected)
            inputs[group].paste(native, (index * size[0], 0))
            spec = {"name": name, "image": "selected/" + selected.name,
                    "normal": "normals/" + selected.name, "offset": [0, 0]}
            if preserve is not None:
                spec["preserve_from_row"] = preserve
            selection[family].append(spec)
            registration.append({"name": name, "raw": "raw/" + raw_path.name,
                "raw_size": list(source.size), "raw_sha256": sha(raw_path),
                "translation": [0, dy], "preserve_from_row": preserve,
                "selected": spec["image"], "selected_sha256": sha(selected),
                "normal_group": group, "normal_rect": [index * size[0], 0, (index + 1) * size[0], size[1]],
                "native_pixel_registration_only": True})
    for name, image in inputs.items():
        image.save(ROOT / "normals" / (name + "-input.png"))
    # Once the two archived PixelLab responses exist, replay their exact crops.
    # The bottom stone planes retain the original geometry and albedo. Correct
    # the two shared original normal fields first, then inherit them exactly.
    original_normals = Image.open(ROOT / "baseline/normals/bloomwild_ruins.png").convert("RGBA")
    for foot in selection["shared_feet"]:
        row = meta["anims"][foot["template"]]["row"]
        box = (0, row * 56, 48, (row + 1) * 56)
        corrected = apply_planes(ROOT / foot["image"], original_normals.crop(box),
                                 ROOT / foot["normal_planes"], 56, foot["first_row"])
        original_normals.paste(corrected, (0, row * 56))
    for entry in registration:
        output_name = entry["normal_group"] + "-output.png"
        if entry["normal_group"] == "leaves" and (ROOT / "normals/leaves-folds-output.png").exists():
            output_name = "leaves-folds-output.png"
        output = ROOT / "normals" / output_name
        if not output.exists():
            continue
        generated = Image.open(output).convert("RGBA")
        assert generated.size == inputs[entry["normal_group"]].size
        normal = generated.crop(tuple(entry["normal_rect"]))
        image = Image.open(ROOT / entry["selected"]).convert("RGBA")
        assert all(not a or n for a, n in zip(image.getchannel("A").tobytes(), normal.getchannel("A").tobytes()))
        normal.putalpha(image.getchannel("A"))
        preserve = entry["preserve_from_row"]
        key = Path(entry["selected"]).stem
        plane_path = ROOT / "normal-planes" / (key + ".json")
        if preserve is not None and plane_path.exists():
            normal = apply_planes(ROOT / entry["selected"], normal, plane_path, preserve)
            entry["normal_planes"] = "normal-planes/" + plane_path.name
            entry["normal_planes_sha256"] = sha(plane_path)
            family = "columns" if entry["name"].startswith("pillar") else "walls"
            spec = next(s for s in selection[family] if s["name"] == entry["name"])
            spec["normal_planes"] = entry["normal_planes"]
        if preserve is not None:
            template = "pillar_broken_short" if entry["name"].startswith("pillar") else "wall_low"
            row = meta["anims"][template]["row"]
            normal.paste(original_normals.crop((0, row * 56 + preserve, 48, (row + 1) * 56)), (0, preserve))
            foot = next(f for f in selection["shared_feet"] if f["template"] == template)
            entry["shared_foot_normal_planes"] = foot["normal_planes"]
            entry["shared_foot_normal_planes_sha256"] = sha(ROOT / foot["normal_planes"])
        normal.putalpha(image.getchannel("A"))
        assert normal.getchannel("A").tobytes() == image.getchannel("A").tobytes()
        normal.save(ROOT / "normals" / Path(entry["selected"]).name)
        entry["generated_normal_source"] = "normals/" + output_name
        entry["generated_normal_sha256"] = sha(output)
    (ROOT / "selection.json").write_text(json.dumps(selection, indent=2) + "\n")
    (ROOT / "registration.json").write_text(json.dumps(registration, indent=2) + "\n")
    print("Registered", len(LEAVES), "leaves,", len(COLUMNS), "stumps,", len(WALLS), "walls")


if __name__ == "__main__":
    main()
