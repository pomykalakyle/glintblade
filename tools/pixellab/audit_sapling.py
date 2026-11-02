#!/usr/bin/env python3
"""Pack a native young Bloomwild tree while retaining its exact physical root.

The new crown/trunk is PixelLab's unchanged 48x64 native output above the
original eight root rows. The old root and its normals move by an integer
(-16,-32) atlas offset only: their world pixels and collision stay identical.
No source pixels are resized, rotated, interpolated, or palette remapped.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image

from pack_trees import layers

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "art/pixellab/2026-11-02-audit-fixes/sapling"
NAME = "bloomwild_sapling"
SIZE = (48, 64)
ANCHOR = (24, 58)
ROOT_TOP = 52
ROOT_BOTTOM = 60


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def composite() -> tuple[Image.Image, dict]:
    selection = json.loads((ARCHIVE / "selection.json").read_text())
    source = ARCHIVE / selection["source"]
    assert sha(source) == selection["source_sha256"]
    raw = Image.open(source).convert("RGBA")
    assert raw.size == SIZE
    assert set(raw.getchannel("A").tobytes()) <= {0, 255}
    # Some PixelLab PNGs carry RGB under zero alpha. Canonicalize invisible
    # storage only; every opaque source pixel remains unchanged.
    for y in range(SIZE[1]):
        for x in range(SIZE[0]):
            if raw.getpixel((x,y))[3] == 0:
                raw.putpixel((x,y),(0,0,0,0))
    old_sheet = ROOT / "assets/sprites/bloomwild_trees.png"
    old_metadata = ROOT / "assets/sprites/bloomwild_trees.json"
    old = Image.open(old_sheet).convert("RGBA")
    legacy = json.loads(old_metadata.read_text())["anims"]["e_trunk"]
    foot = old.crop((16, 8 * 96 + 84, 64, 8 * 96 + 92))
    image = raw.copy()
    # Retain the physical root at its exact world location, rather than
    # introducing a wider generated footprint or invisible collision proxy.
    image.paste((0, 0, 0, 0), (0, ROOT_TOP, SIZE[0], SIZE[1]))
    image.paste(foot, (0, ROOT_TOP))
    assert image.crop((0, ROOT_TOP, 48, ROOT_BOTTOM)).tobytes() == foot.tobytes()
    assert image.crop((0, 0, 48, ROOT_TOP)).tobytes() == raw.crop((0, 0, 48, ROOT_TOP)).tobytes()
    assert all(image.getpixel((x, ROOT_TOP-1))[3] for x in range(23, 26)), "native trunk must meet preserved root"
    assert image.getbbox()[3] == ROOT_BOTTOM
    rects = [[x0-16, y0-32, x1-16, y1-32] for x0,y0,x1,y1 in legacy["collision_rects"]]
    assert legacy["footprint"] == [14, 8]
    old_world = [[x0-40,y0-90,x1-40,y1-90] for x0,y0,x1,y1 in legacy["collision_rects"]]
    new_world = [[x0-ANCHOR[0],y0-ANCHOR[1],x1-ANCHOR[0],y1-ANCHOR[1]] for x0,y0,x1,y1 in rects]
    assert new_world == old_world, "every world collision span must remain exact"
    for x0,y0,x1,y1 in rects:
        assert all(image.getpixel((x,y))[3] for y in range(y0,y1) for x in range(x0,x1))
    selection["collision_rects"] = rects
    selection["legacy_footprint"] = legacy["footprint"]
    selection["legacy_albedo_sha256"] = sha(old_sheet)
    selection["legacy_metadata_sha256"] = sha(old_metadata)
    return image, selection


def build(out: Path, normal_out: Path) -> dict:
    image, selection = composite()
    trunk, canopy = layers(image)
    # Physical roots never become canopy, even if an old bark pixel happens
    # to meet the leaf palette's classification.
    canopy.paste((0,0,0,0), (0,ROOT_TOP,48,64))
    trunk.paste(image.crop((0,ROOT_TOP,48,64)), (0,ROOT_TOP))
    joined = trunk.copy(); joined.alpha_composite(canopy)
    assert joined.tobytes() == image.tobytes()
    sheet = Image.new("RGBA", (48,128))
    sheet.paste(trunk,(0,0)); sheet.paste(canopy,(0,64))
    normal_source = ARCHIVE / selection["normal_source"]
    assert sha(normal_source) == selection["normal_source_sha256"]
    native_normal = Image.open(normal_source).convert("RGBA")
    assert native_normal.size == SIZE
    native_normal.putalpha(image.getchannel("A"))
    old_normal = Image.open(ROOT / "assets/normals/bloomwild_trees.png").convert("RGBA")
    native_normal.paste(old_normal.crop((16,8*96+84,64,8*96+92)), (0,ROOT_TOP))
    native_normal.putalpha(image.getchannel("A"))
    normal = Image.new("RGBA",(48,128))
    for offset, albedo in [(0,trunk),(64,canopy)]:
        part = native_normal.copy(); part.putalpha(albedo.getchannel("A"))
        normal.paste(part,(0,offset))
    assert normal.getchannel("A").tobytes() == sheet.getchannel("A").tobytes()
    out.mkdir(parents=True, exist_ok=True); normal_out.mkdir(parents=True, exist_ok=True)
    albedo_path = out/f"{NAME}.png"; normal_path = normal_out/f"{NAME}.png"
    sheet.save(albedo_path); normal.save(normal_path)
    metadata = {
        "cell":list(SIZE), "anchor":list(ANCHOR),
        "anims":{
            "e_trunk":{"row":0,"frames":1,"fps":1,"footprint":[14,8],"collision_rects":selection["collision_rects"]},
            "e_canopy":{"row":1,"frames":1,"fps":1,"durations":[96],"layer":"canopy"},
        },
        "notes":"Native 48x64 PixelLab young rose tree, separate from the mature tree family. Existing e_trunk's eight opaque root rows, world-space foot, 14x8 footprint, collision rectangles and root normals are retained exactly by an integer atlas translation. Above-root pixels are unchanged native source UVs. Crown is split for ordinary foreground reveal/shadows and remains fixed, not resampled or animated as a sliding mass. Source, prompt, selected candidate, root contract and normal receipts: art/pixellab/2026-11-02-audit-fixes/sapling. Rebuild: uv run --with pillow python tools/pixellab/audit_sapling.py",
    }
    (out/f"{NAME}.json").write_text(json.dumps(metadata,indent=2)+"\n")
    image.save(ARCHIVE/"selected-native.png")
    native_normal.save(ARCHIVE/"selected-normal-native.png")
    audit={
        "source":selection["source"],"source_sha256":selection["source_sha256"],
        "normal_source":selection["normal_source"],"normal_source_sha256":selection["normal_source_sha256"],
        "albedo_sha256":sha(albedo_path),"normal_sha256":sha(normal_path),
        "native_size":list(SIZE),"atlas_size":[48,128],"anchor":list(ANCHOR),
        "opaque_bounds":list(image.getbbox()),"alpha_values":sorted(set(sheet.getchannel("A").tobytes())),
        "footprint":[14,8],"collision_rects":len(selection["collision_rects"]),
        "all_collision_pixels_opaque":True,"source_upper_uv_identity":True,
        "preserved_root_uv_identity":True,"normal_alpha_matches_albedo":True,
        "original_world_position":[1224,280],"original_world_bounds":[1220,274,1234,282],
        "world_collision_identical":True,"mature_atlas_modified":False,
    }
    (ARCHIVE/"asset-audit.json").write_text(json.dumps(audit,indent=2)+"\n")
    return audit


def main() -> None:
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out",type=Path,default=ROOT/"assets/sprites")
    p.add_argument("--normals-out",type=Path,default=ROOT/"assets/normals")
    p.add_argument("--composite-only",action="store_true")
    args=p.parse_args()
    if args.composite_only:
        image,_=composite(); image.save(ARCHIVE/"normal-input.png")
        print(ARCHIVE/"normal-input.png")
    else:
        print(json.dumps(build(args.out,args.normals_out),indent=2))


if __name__=="__main__":
    main()
