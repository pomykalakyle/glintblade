#!/usr/bin/env python3
"""Export only the reviewed native ravine atlas and registered upright normals.

uv run --with pillow python tools/pixellab/glasswater_ravine.py --out STAGING/sprites
No network, resampling, umbrella exporter or unrelated asset writes.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'art/pixellab/2026-11-02-ravine'
NAME = 'glasswater_ravine'
HEIGHT = 704


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def quilt(images, first):
    """Vertical minimum-error cuts copy whole native pixels and their provenance.

    Each128px patch overlaps32px. A connected cut follows matching slate
    clusters instead of exposing a straight source-canvas edge every128px.
    """
    result = Image.new('RGBA', (128, HEIGHT))
    provenance = Image.new('RGBA', result.size)
    dst, uv = result.load(), provenance.load()
    for patch, y0 in enumerate(range(0, HEIGHT, 96)):
        frame = (first + patch) % 2
        src = images[frame].load()
        overlap = 0 if patch == 0 else 32
        seam = [0] * 128
        if overlap:
            costs, parents = [], []
            for x in range(128):
                column, parent = [], []
                for y in range(overlap):
                    error = sum((dst[x,y0+y][c] - src[x,y][c])**2 for c in range(3))
                    if x == 0:
                        column.append(error); parent.append(y)
                    else:
                        previous = min(range(max(0,y-1), min(overlap,y+2)), key=lambda k: costs[x-1][k])
                        column.append(error + costs[x-1][previous]); parent.append(previous)
                costs.append(column); parents.append(parent)
            seam[-1] = min(range(overlap), key=lambda y: costs[-1][y])
            for x in range(126,-1,-1):
                seam[x] = parents[x+1][seam[x+1]]
        for x in range(128):
            for y in range(min(128, HEIGHT-y0)):
                if y < seam[x]:
                    continue
                dst[x,y0+y] = src[x,y]
                uv[x,y0+y] = (frame,x,y,255)
    return result, provenance


def export(out, normals_out):
    out.mkdir(parents=True, exist_ok=True)
    normals_out.mkdir(parents=True, exist_ok=True)
    atlas = Image.new('RGBA', (256, HEIGHT))
    provenance = Image.new('RGBA', atlas.size)
    selected = [SOURCE / 'raw/candidate-0.png', SOURCE / 'raw/candidate-3.png']
    images = []
    for path in selected:
        image = Image.open(path).convert('RGBA')
        assert image.size == (128, 128)
        assert set(image.getchannel('A').tobytes()) == {255}
        assert len(image.getcolors(16384)) <= 9
        images.append(image)
    for frame in range(2):
        strip, uv = quilt(images, frame)
        atlas.paste(strip, (128 * frame, 0))
        provenance.paste(uv, (128 * frame, 0))
    atlas.save(out / f'{NAME}.png')
    raw = Image.open(SOURCE / 'raw/normal-pro.png').convert('RGBA')
    assert raw.size == (256,128)
    normal = Image.new('RGBA', atlas.size)
    # The entire sheet is an upright exposed rock face, never a horizontal cap.
    # Retain PixelLab's registered mineral facets in a restrained front-facing
    # tangent hemisphere. Vector data is normalized, never colour-graded.
    values = []
    tilt = 0.0
    for py in range(HEIGHT):
        for px in range(256):
            frame, sx, sy, _ = provenance.getpixel((px,py))
            pixel = raw.getpixel((128*frame+sx,sy))
            albedo = atlas.getpixel((px,py))
            x, y, z = [value / 127.5 - 1 for value in pixel[:3]]
            x, y, z = x * 0.7, y * 0.7, max(0.75, z)
            length = math.sqrt(x*x + y*y + z*z)
            x, y, z = x/length, y/length, z/length
            tilt = max(tilt, math.degrees(math.acos(z)))
            values.append(tuple(round((v + 1) * 127.5) for v in (x,y,z)) + (albedo[3],))
    normal.putdata(values)
    normal.save(normals_out / f'{NAME}.png')
    manifest = {'cell': [128,HEIGHT], 'anchor': [0,0],
        'anims': {'body': {'row': 0, 'frames': 2, 'fps': 1}},
        'notes': 'Native PixelLab slate candidates0/3; minimum-error32px overlaps extend each face to704px, exact source pixels without resizing. Normals follow identical provenance. Upright OpenGL mineral normals registered1:1. Sources/receipts: art/pixellab/2026-11-02-ravine. Scoped exporter: tools/pixellab/glasswater_ravine.py.'}
    (out / f'{NAME}.json').write_text(json.dumps(manifest, indent=2) + '\n')
    return {'sources': {str(p.relative_to(ROOT)): digest(p) for p in selected + [SOURCE/'raw/normal-pro.png']},
        'outputs': {str(p): digest(p) for p in [out/f'{NAME}.png', out/f'{NAME}.json', normals_out/f'{NAME}.png']},
        'dimensions': list(atlas.size), 'frames': 2, 'alpha_values': [255],
        'source_pixels_unchanged': True, 'quilt': {'patch_height': 128, 'overlap': 32, 'cut': 'minimum squared RGB error, connected native pixels'},
        'normal_maximum_tilt_degrees': round(tilt,3)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--normals-out', type=Path)
    parser.add_argument('--receipt', type=Path)
    args = parser.parse_args()
    result = export(args.out, args.normals_out or args.out.parent/'normals')
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
