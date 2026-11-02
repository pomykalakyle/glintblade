"""Pack native PixelLab trees into the existing trunk/canopy atlas contract.

No pixels are resampled or recolored. Leaf-colored clusters are separated for
canopy occlusion. Entire crowns stay rooted: translating every leaf pixel one
pixel sideways looked like the tree was sliding rather than blowing in wind.
The combined first frame is checked against each untouched source.
"""
import argparse
import json
from pathlib import Path

from PIL import Image
from native_collision import add_collision


def layers(source, ivory=False):
    pixels = source.load()
    rose = set()
    for y in range(source.height):
        for x in range(source.width):
            r, g, b, a = pixels[x, y]
            if a and ((r > g and b > g) or (ivory and y < 61 and r > g > b)):
                rose.add((x, y))
    leaf = set(rose)
    for x, y in rose:
        for dy in range(-2, 3):
            for dx in range(-2, 3):
                xx, yy = x + dx, y + dy
                if not (0 <= xx < source.width and 0 <= yy < source.height):
                    continue
                r, g, b, a = pixels[xx, yy]
                if a and b >= r and max(r, g, b) < 145:
                    leaf.add((xx, yy))
    trunk = Image.new('RGBA', source.size)
    canopy = Image.new('RGBA', source.size)
    for y in range(source.height):
        for x in range(source.width):
            if pixels[x, y][3]:
                (canopy if (x, y) in leaf else trunk).putpixel((x, y), pixels[x, y])
    merged = trunk.copy()
    merged.alpha_composite(canopy)
    assert merged.tobytes() == source.tobytes()
    return trunk, canopy


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--sources', type=Path, required=True)
    p.add_argument('--base', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    sheet = Image.open(a.base / 'bloomwild_trees.png').convert('RGBA')
    metadata = json.loads((a.base / 'bloomwild_trees.json').read_text())
    # Preserve the fifth original sapling, then append two genuinely upright
    # silhouettes. Their raw PixelLab sources have complete roots and crown.
    sheet = Image.new('RGBA', (160, 96 * 14))
    sheet.paste(Image.open(a.base / 'bloomwild_trees.png').convert('RGBA'), (0, 0))
    sources = [(v, a.sources / f'trees-pro.result-{i}.png', False)
               for v, i in zip('abcd', [1, 4, 2, 3])]
    new_sources = Path(__file__).resolve().parents[2] / 'art/pixellab/2026-11-01-round5/variety/trees'
    sources += [('f', new_sources / 'result-2.png', True),
                ('g', new_sources / 'result-3.png', True)]
    for variant, source_path, ivory in sources:
        raw = Image.open(source_path).convert('RGBA')
        assert raw.size == (80, 96)
        native = Image.new('RGBA', (80, 96))
        native.alpha_composite(raw, (0, 92 - raw.getbbox()[3] if ivory else 8))
        assert native.getbbox()[3] <= 92
        trunk, canopy = layers(native, ivory)
        if ivory:
            row = 10 if variant == 'f' else 12
            metadata['anims'][f'{variant}_trunk'] = {'row': row, 'frames': 1, 'fps': 1, 'footprint': [36, 12]}
            metadata['anims'][f'{variant}_canopy'] = {'row': row + 1, 'frames': 1, 'fps': 1, 'layer': 'canopy'}
        trunk_row = metadata['anims'][f'{variant}_trunk']['row']
        canopy_row = metadata['anims'][f'{variant}_canopy']['row']
        for frame in range(sheet.width // 80):
            sheet.paste((0, 0, 0, 0), (frame * 80, trunk_row * 96, (frame + 1) * 80, (trunk_row + 1) * 96))
            sheet.paste((0, 0, 0, 0), (frame * 80, canopy_row * 96, (frame + 1) * 80, (canopy_row + 1) * 96))
        sheet.paste(trunk, (0, trunk_row * 96))
        sheet.paste(canopy, (0, canopy_row * 96))
        sheet.paste(canopy, (80, canopy_row * 96))
        metadata['anims'][f'{variant}_canopy'].update(frames=1, fps=1, durations=[96])
        add_collision(metadata['anims'][f'{variant}_trunk'], trunk, 12)
        print(f'{variant}: {source_path.name}, fixed crown and exact native root support')
    young_anim = metadata['anims']['e_trunk']
    young = sheet.crop((0, young_anim['row'] * 96, 80, (young_anim['row'] + 1) * 96))
    add_collision(young_anim, young, 8)
    young_canopy = metadata['anims']['e_canopy']
    y = young_canopy['row'] * 96
    sheet.paste(sheet.crop((0, y, 80, y + 96)), (80, y))
    young_canopy.update(frames=1, fps=1, durations=[96])
    metadata['notes'] = ('PixelLab Pro native tree family (job cf9df804-7455-4102-a346-af9ad6d189a0), '
                         'four candidates packed without resizing/recoloring; original young tree retained. '
                         'Two upright ivory-blossom companions from round5 job '
                         'f3c0a77c-39e0-4585-87f6-ccdaf85a7b9a supplement the rose family. '
                         'Native cells and original anchors retained. Collision follows the '
                         'opaque native root rows, preserving asymmetry and gaps. Crown geometry '
                         'is static; wind lives in grass, detached petals and ambient leaves. '
                         'Each trunk/canopy composite matches its aligned native source exactly.')
    sheet.save(a.out / 'bloomwild_trees.png')
    (a.out / 'bloomwild_trees.json').write_text(json.dumps(metadata, indent=2))


if __name__ == '__main__':
    main()
