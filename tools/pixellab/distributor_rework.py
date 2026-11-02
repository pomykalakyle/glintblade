#!/usr/bin/env python3
"""Export only Glasswater's square-on native distributor and authored planes.

The PixelLab originals are immutable. Literal native repairs retain its stone
volume, replace authored contact ink, open the header's outlets, and give
its cistern a horizontal bed. No pixel is scaled, rotated or interpolated.
"""
import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageDraw

from native_collision import add_collision
from round13_landmarks import semantic_normals

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "art/pixellab/2026-11-02-distributor-rework"
NAME = "round13_landmarks_distributor"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build():
    recipe = json.loads((ARCHIVE / "recipe.json").read_text())
    source = ARCHIVE / recipe["source"]
    assert sha(source) == recipe["source_sha256"]
    original = Image.open(source).convert("RGBA")
    image = original.copy()
    # Keep the native lower volume. Only explicitly authored contact strokes
    # lose their ink; top/side outlines and physical alpha remain intact.
    draw = ImageDraw.Draw(image)
    for edit in recipe["edits"]:
        if edit["operation"] == "clear":
            draw.rectangle(edit["rect"], fill=(0, 0, 0, 0))
        elif edit["operation"] == "rectangle":
            draw.rectangle(edit["rect"], fill=edit["color"])
        elif edit["operation"] == "polygon":
            draw.polygon([tuple(p) for p in edit["points"]], fill=edit["color"])
        elif edit["operation"] == "restore":
            x0, y0, x1, y1 = edit["rect"]
            patch = original.crop((x0, y0, x1 + 1, y1 + 1))
            image.paste(patch, (x0, y0), patch)
        elif edit["operation"] == "contact_ink":
            old = {tuple(bytes.fromhex(c.lstrip("#"))) for c in edit["from"]}
            new = tuple(bytes.fromhex(edit["to"].lstrip("#")))
            for x0, y0, x1, y1 in edit["rects"]:
                for y in range(y0,y1+1):
                    for x in range(x0,x1+1):
                        p = image.getpixel((x,y))
                        assert p[3] == 255 and (p[:3] in old or p[:3] == new), ("authored contact ink moved",x,y,p)
                        if p[:3] in old:
                            image.putpixel((x,y),(*new,255))
        else:
            raise AssertionError(edit)
    assert image.size == (144, 112)
    assert set(image.getchannel("A").tobytes()) == {0, 255}
    planes = json.loads((ARCHIVE / "normal-planes.json").read_text())
    normal, labels, counts = semantic_normals(image, planes)
    return image, normal, labels, counts, recipe


def export(out, normals_out):
    out, normals_out = Path(out), Path(normals_out)
    out.mkdir(parents=True, exist_ok=True)
    normals_out.mkdir(parents=True, exist_ok=True)
    image, normal, _, counts, recipe = build()
    assert image.getbbox() == (14, 17, 130, 100)
    anim = {"row": 0, "frames": 1, "fps": 1, "contact": "none"}
    add_collision(anim, image, recipe["support_depth"])
    anim["support_first_row"] = image.getbbox()[3] - recipe["support_depth"]
    metadata = {
        "cell": [144, 112], "anchor": recipe["anchor"],
        "anims": {"body": anim}, "background": False,
        "notes": "Native PixelLab candidate 2 with restored native lower stone faces, authored contact-ink correction and cistern bed. "
                 "Front aligns east-west; depth recedes north; uprights remain vertical. "
                 "Separated supports and open passage. No ground, contact or cast shadow in sprite alpha. "
                 "Roof/bed, facade, arch returns, soffit and native cap/front faces have authored physical normals. Artificial contact AO is disabled; geometric casts remain.",
        "provenance": {
            "archive": str(ARCHIVE.relative_to(ROOT)) + "/",
            "source": recipe["source"], "source_sha256": recipe["source_sha256"],
            "recipe": "recipe.json", "recipe_sha256": sha(ARCHIVE / "recipe.json"),
            "normal_planes": "normal-planes.json", "normal_planes_sha256": sha(ARCHIVE / "normal-planes.json"),
            "normal_method": "authored native geometric planes, upright R/B/G world X/Y/Z",
            "historical_source": "art/pixellab/2026-11-02-round13/landmarks/selection.json",
        },
    }
    image.save(out / (NAME + ".png"))
    normal.save(normals_out / (NAME + ".png"))
    (out / (NAME + ".json")).write_text(json.dumps(metadata, indent=2) + "\n")
    print(NAME, image.getbbox(), "face pixels", counts)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "assets/sprites")
    parser.add_argument("--normals-out", type=Path)
    args = parser.parse_args()
    export(args.out, args.normals_out or args.out.parent / "normals")


if __name__ == "__main__":
    main()
