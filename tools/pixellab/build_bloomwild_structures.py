#!/usr/bin/env python3
"""Repack the native Bloomwild structure family from archived selections, offline."""
import argparse
import json
from pathlib import Path
from PIL import Image
from native_collision import add_collision

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / 'art/pixellab/2026-11-01-round3/bloomwild'
REPAIRS = ROOT / 'art/pixellab/2026-11-01-round5/columns'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out', type=Path, default=ROOT / 'assets/sprites')
    a = p.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    selection = json.loads((SOURCES / 'selection.json').read_text())
    repairs = json.loads((REPAIRS / 'selection.json').read_text())
    metadata = json.loads((SOURCES / 'legacy/bloomwild_ruins.json').read_text())
    legacy = Image.open(SOURCES / 'legacy/bloomwild_ruins.png').convert('RGBA')
    names = list(metadata['anims'])
    names += [n for n in selection['anims'] if n not in names]
    atlas = Image.new('RGBA', (48, 56 * len(names)))
    for row, name in enumerate(names):
        spec = selection['anims'].get(name)
        repair = repairs['anims'].get(name)
        if repair:
            image = Image.open(REPAIRS / repair['source']).convert('RGBA')
            original = Image.open(REPAIRS / repair['original']).convert('RGBA')
            assert image.size == original.size == (48, 56), repair['source']
            assert set(image.getchannel('A').tobytes()) <= {0, 255}
            # Keep the established native shaft untouched. Its repaired foot must
            # finish inside the cell rather than hit the old guillotine edge.
            keep = repair['preserved_rows']
            assert image.crop((0, 0, 48, keep)).tobytes() == original.crop((0, 0, 48, keep)).tobytes()
            raw = Image.open(REPAIRS / repair['edit_source']).convert('RGBA')
            repaired = Image.new('RGBA', (48, 56))
            repaired.paste(original.crop((0, 0, 48, keep)), (0, 0))
            repaired.paste(raw.crop(tuple(repair['source_rect'])), (0, keep))
            assert repaired.tobytes() == image.tobytes(), 'Selected repair must reproduce from exact native source pixels'
            assert image.getchannel('A').crop((0, 54, 48, 56)).getbbox() is None, repair['source']
            anim = {**metadata['anims'].get(name, {}), 'row': row, 'frames': 1, 'fps': 1}
        elif spec:
            image = Image.open(SOURCES / spec['source']).convert('RGBA')
            assert image.size == (48, 56), spec['source']
            assert set(image.getchannel('A').tobytes()) <= {0, 255}
            anim = {**metadata['anims'].get(name, {}), 'row': row, 'frames': 1, 'fps': 1,
                    'footprint': spec['footprint']}
        else:
            anim = metadata['anims'][name].copy()
            image = legacy.crop((0, anim['row'] * 56, 48, (anim['row'] + 1) * 56))
            anim['row'] = row
        add_collision(anim, image, 10 if name.startswith('pillar') else 8)
        metadata['anims'][name] = anim
        atlas.paste(image, (0, row * 56))
    atlas.save(a.out / 'bloomwild_ruins.png')
    metadata['notes'] = ('PixelLab native structure family: ' + '; '.join(selection['notes'] + repairs['notes'])
                         + '; exact opaque native base collision spans.')
    (a.out / 'bloomwild_ruins.json').write_text(json.dumps(metadata, indent=2) + '\n')
    if 'terrace' in selection:
        spec = selection['terrace']
        atlas = Image.new('RGBA', (128 * len(spec['sources']), 32))
        for i, name in enumerate(spec['sources']):
            image = Image.open(SOURCES / name).convert('RGBA')
            assert image.size == (128, 32)
            atlas.paste(image, (128 * i, 0))
        atlas.save(a.out / 'bloomwild_terrace.png')
        data = {'cell': [128, 32], 'anchor': [0, 0], 'anims': {'body': {'row': 0, 'frames': len(spec['sources']), 'fps': 1}},
                'notes': 'Native masonry strips clipped to the existing terrace face; stairs, niches, water and collision protected.'}
        (a.out / 'bloomwild_terrace.json').write_text(json.dumps(data, indent=2) + '\n')
    print('Exported Bloomwild native structures to', a.out)


if __name__ == '__main__':
    main()
