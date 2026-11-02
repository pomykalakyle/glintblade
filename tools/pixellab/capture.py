#!/usr/bin/env python3
"""Silent native game captures for PixelLab candidates, with exact replay metadata.

python3 tools/pixellab/capture.py --tag baseline
python3 tools/pixellab/capture.py --tag trees-v1 --sprites out/pixellab/candidates/v1/sprites

No window, server, audio or paid generation is started. GLINT_SPRITES overlays the compiled
sprite registry only; the same renderer, world geometry, lighting and gameplay are retained.
"""

import argparse
import base64
import hashlib
import html
import json
import os
from pathlib import Path
import shutil
import subprocess
from datetime import datetime, timezone


ROOT = Path(__file__).resolve().parents[2]
SCENES = (
    ("landing", "258,696", "Landing: path, tree, broken waterworks and island lip"),
    ("entry", "258,640", "Entry: arch, tree silhouettes and navigation"),
    ("falls", "780,700", "Falls: water, stone and a clear route"),
    ("terrace", "664,340", "Terrace: bridge, elevation and planting"),
    ("grove", "934,188", "Grove: repeated trees and floor texture"),
)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True, help="New output folder name (e.g. baseline or trees-v1)")
    parser.add_argument("--sprites", type=Path, help="Candidate directory containing PNG/JSON sprite overrides")
    parser.add_argument("--fields", choices=("on", "off"), help="Compare native ground with/without inherited field lighting")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/glintblade")
    parser.add_argument("--out", type=Path, default=ROOT / "out/pixellab/captures")
    args = parser.parse_args()
    if not args.tag or Path(args.tag).name != args.tag or args.tag in (".", ".."):
        parser.error("--tag must be one directory name")
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f"Build the native renderer first: cargo build --release ({binary} missing)")
    sprites = args.sprites.resolve() if args.sprites else None
    if sprites and not sprites.is_dir():
        parser.error(f"Candidate sprite directory does not exist: {sprites}")
    if args.fields and (not sprites or not (sprites / "bloomwild_materials.json").is_file()):
        parser.error("--fields needs --sprites with bloomwild_materials.json")
    out = args.out.resolve() / args.tag
    # Previous evidence stays intact; every art iteration gets a new tag.
    out.mkdir(parents=True, exist_ok=False)
    source_sprites = sprites
    if sprites:
        # Each review keeps the exact candidate pixels; later artist edits cannot change replay.
        sprites = out / "sprite-inputs"
        sprites.mkdir()
        for p in sorted(source_sprites.iterdir()):
            if p.suffix in (".png", ".json") and p.is_file():
                shutil.copy2(p, sprites / p.name)
        if args.fields:
            path = sprites / "bloomwild_materials.json"
            material = json.loads(path.read_text())
            material["preserve_fields"] = args.fields == "on"
            path.write_text(json.dumps(material, indent=2) + "\n")
    env = os.environ.copy()
    env.pop("GLINT_SPRITES", None)
    if sprites:
        env["GLINT_SPRITES"] = str(sprites)
    manifest = {
        "tag": args.tag,
        "captured_utc": datetime.now(timezone.utc).isoformat(),
        "binary": str(binary),
        "binary_sha256": sha256(binary),
        "sprite_override_directory": str(sprites) if sprites else None,
        "sprite_source_directory": str(source_sprites) if source_sprites else None,
        "field_lighting_override": args.fields,
        "sprite_override_sha256": {
            p.name: sha256(p) for p in sorted(sprites.iterdir())
            if p.suffix in (".png", ".json") and p.is_file()
        } if sprites else {},
        "native_resolution": [384, 216],
        "captures": [],
    }
    cards = []
    for name, position, description in SCENES:
        folder = out / name
        command = [str(binary), "zoneview", "bloomwild", "--at", position,
                   "--warm", "90", "--scale", "1", "--out", str(folder)]
        if name == "landing":
            command.append("--overview")
        result = subprocess.run(command, cwd=ROOT, env=env, check=True, capture_output=True, text=True)
        path = folder / "bloomwild_t00090.png"
        assert path.is_file() and path.stat().st_size > 0
        manifest["captures"].append({
            "name": name, "description": description, "position": position,
            "command": command, "cwd": str(ROOT), "stderr": result.stderr,
            "native_png": str(path), "native_png_sha256": sha256(path),
        })
        data = base64.b64encode(path.read_bytes()).decode()
        cards.append(f'<figure><div class="pixels"><img src="data:image/png;base64,{data}" width="768" height="432">'
                     f'</div><figcaption>{html.escape(description)}</figcaption></figure>')
        print(f"captured {name}: {path}", flush=True)
    # A real Game::tick/render frame includes the actual HUD and actor sorting as an integration check.
    folder = out / "gameplay"
    command = [str(binary), "shot", "--zone", "bloomwild",
               "--ticks", "360", "--peaceful", "--seed", "7", "--scale", "1",
               "--out", str(folder), "--report", str(out / "gameplay.json")]
    result = subprocess.run(command, cwd=ROOT, env=env, check=True, capture_output=True, text=True)
    path = folder / "f00360.png"
    manifest["captures"].append({"name": "gameplay", "command": command, "cwd": str(ROOT),
                                  "stderr": result.stderr, "native_png": str(path),
                                  "native_png_sha256": sha256(path)})
    data = base64.b64encode(path.read_bytes()).decode()
    cards.append(f'<figure><div class="pixels"><img src="data:image/png;base64,{data}" width="768" height="432">'
                 '</div><figcaption>Default playable entry, actual HUD after intro at tick 360</figcaption></figure>')
    assert sha256(binary) == manifest["binary_sha256"], "Renderer changed during capture"
    if sprites:
        current = {p.name: sha256(p) for p in sorted(sprites.iterdir())
                   if p.suffix in (".png", ".json") and p.is_file()}
        assert current == manifest["sprite_override_sha256"], "Candidate sprites changed during capture"
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (out / "gallery.html").write_text(
        '<!doctype html><html><meta charset="utf-8"><meta name="viewport" content="width=device-width">'
        f'<title>Glintblade {html.escape(args.tag)}</title><style>'
        'body{margin:24px;background:#171b23;color:#e1e5dc;font:16px system-ui}'
        'main{display:flex;flex-wrap:wrap;gap:24px}figure{margin:0;width:min(768px,100%)}'
        '.pixels{overflow-x:auto}img{display:block;width:768px;height:432px;max-width:none;image-rendering:pixelated}figcaption{margin:8px 0 0;color:#b6c3bc}'
        f'</style><h1>Glintblade · {html.escape(args.tag)}</h1>'
        '<p>Native engine scene studies at tick 90, plus the default playable entry. Enlarged exactly 2×.</p>'
        f'<main>{"".join(cards)}</main></html>'
    )
    print(f"gallery: {out / 'gallery.html'}")
    print(f"replay metadata: {out / 'manifest.json'}")


if __name__ == "__main__":
    main()
