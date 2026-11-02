"""Pack selected native PixelLab floor modules without resampling or repainting."""
import argparse
import json
from pathlib import Path
from PIL import Image


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--sources', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    materials = {
        'grass': [a.sources / 'grass-focus.tiles' / f'{i:02}.png' for i in range(16)],
        'slab': [a.sources / 'stone-focus.tiles' / f'{i:02}.png' for i in [0, 1, 2, 3, 4, 6, 7, 10, 12, 14, 15]],
    }
    sheet = Image.new('RGBA', (512, 64))
    anims = {}
    for row, (name, paths) in enumerate(materials.items()):
        for col, path in enumerate(paths):
            tile = Image.open(path).convert('RGBA')
            assert tile.size == (32, 32), path
            assert tile.getchannel('A').getextrema() == (255, 255), path
            sheet.paste(tile, (col * 32, row * 32))
        anims[name] = {'row': row, 'frames': len(paths), 'fps': 1, 'coverage': 1.0}
    sheet.save(a.out / 'bloomwild_materials.png')
    metadata = {'cell': [32, 32], 'anchor': [0, 0], 'anims': anims,
                'notes': 'Native PixelLab 32px modules, curated from grass-focus16 and stone-focus16. '
                         'Ground-only material clipping; no collision change, resampling, recoloring or baked actor shadows. '
                         'Existing path remains authored; variants are selected in stable world space.'}
    (a.out / 'bloomwild_materials.json').write_text(json.dumps(metadata, indent=2))
    print('Packed', {k: len(v) for k, v in materials.items()})


if __name__ == '__main__':
    main()
