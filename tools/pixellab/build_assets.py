"""Rebuild the selected PixelLab environment from preserved native sources.

uv run --with pillow python tools/pixellab/build_assets.py
Use --out out/pixellab/candidates/rebuilt/sprites for a reversible preview export.
No network, credentials or paid generation are involved.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys

from PIL import Image
from native_collision import add_collision

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).resolve().parent
SOURCES = ROOT / 'art/pixellab/2026-11-01'


def manifest(out, name, cell, anchor, frames=1, notes=''):
    data = {'cell': cell, 'anchor': anchor,
            'anims': {'body': {'row': 0, 'frames': frames, 'fps': 1, 'footprint': [0, 0]}},
            'notes': notes}
    (out / f'{name}.json').write_text(json.dumps(data, indent=2))


def arch(out):
    raw = Image.open(SOURCES / 'arch/entry-arch.png').convert('RGBA')
    assert raw.size == (64, 72)
    assert set(raw.getchannel('A').tobytes()) <= {0, 255}
    left, right, span = [Image.new('RGBA', (64, 72)) for _ in range(3)]
    for y in range(72):
        for x in range(64):
            pixel = raw.getpixel((x, y))
            if x < 23:
                left.putpixel((x + 17, y), pixel)
            elif x >= 42:
                right.putpixel((x - 17, y), pixel)
            else:
                span.putpixel((x, y), pixel)
    combined = Image.new('RGBA', (64, 72))
    combined.alpha_composite(left, (-17, 0))
    combined.alpha_composite(right, (17, 0))
    combined.alpha_composite(span)
    expected = Image.new('RGBA', raw.size)
    expected.alpha_composite(raw)
    assert combined.tobytes() == expected.tobytes()
    atlas = Image.new('RGBA', (64, 216))
    for i, image in enumerate([left, span, right]):
        atlas.paste(image, (0, i * 72))
    atlas.save(out / 'bloomwild_arch.png')
    data = {'cell': [64, 72], 'anchor': [32, 69], 'anims': {
        'arch_post_left': {'row': 0, 'frames': 1, 'fps': 1, 'footprint': [12, 6]},
        'arch_post': {'row': 0, 'frames': 1, 'fps': 1, 'footprint': [12, 6]},
        'arch_span': {'row': 1, 'frames': 1, 'fps': 1, 'layer': 'canopy'},
        'arch_post_right': {'row': 2, 'frames': 1, 'fps': 1, 'footprint': [12, 6]},
    }, 'notes': 'PixelLab native arch, exact visible source pixels split into supported piers '
                'and overhead passage. Original anchor and post world points retained; collision '
                'matches each actual opaque native foot, including the shorter right pier.'}
    for name, image in [('arch_post_left', left), ('arch_post', left), ('arch_post_right', right)]:
        add_collision(data['anims'][name], image, 8)
    (out / 'bloomwild_arch.json').write_text(json.dumps(data, indent=2))


def legacy_prop_collision(out):
    # Keep original authored art and breakability; only replace the approximate old boxes.
    source = ROOT / 'art/pixellab/2026-11-01-round4/legacy'
    image = Image.open(source / 'bloomwild_props.png').convert('RGBA')
    data = json.loads((source / 'bloomwild_props.json').read_text())
    cw, ch = data['cell']
    for anim in data['anims'].values():
        if anim.get('footprint'):
            cell = image.crop((0, anim['row'] * ch, cw, (anim['row'] + 1) * ch))
            add_collision(anim, cell, int(anim['footprint'][1]))
    data['notes'] = ('Original authored foliage, ceramics and mineral boulders retained byte-for-byte. '
                     'Opaque structural base rows supply collision and contact shading. '
                     'Original urn breakability and walk-through flowers retained.')
    shutil.copyfile(source / 'bloomwild_props.png', out / 'bloomwild_props.png')
    (out / 'bloomwild_props.json').write_text(json.dumps(data, indent=2) + '\n')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out', type=Path, default=ROOT / 'assets/sprites')
    p.add_argument('--normals-out', type=Path,
                   help='Round 13 normal exports; defaults to a normals directory beside --out')
    a = p.parse_args()
    out = a.out.resolve()
    normals_out = a.normals_out.resolve() if a.normals_out else out.parent / 'normals'
    out.mkdir(parents=True, exist_ok=True)
    subprocess.run([sys.executable, str(TOOLS / 'pack_trees.py'), '--sources', str(SOURCES / 'trees'),
                    '--base', str(SOURCES / 'legacy'), '--out', str(out)], check=True)
    subprocess.run([sys.executable, str(TOOLS / 'pack_materials.py'), '--sources', str(SOURCES / 'terrain'),
                    '--out', str(out)], check=True)
    path = out / 'bloomwild_materials.json'
    data = json.loads(path.read_text())
    data.update(preserve_fields=True, live_grass_density=0.18)
    path.write_text(json.dumps(data, indent=2))
    arch(out)
    legacy_prop_collision(out)
    for name, source, cell, anchor in [
        ('bloomwild_aqueduct', 'props/aqueduct.png', [128, 96], [64, 83]),
        ('bloomwild_growth', 'props/planting.png', [64, 80], [32, 74]),
        ('bloomwild_growth_low', 'props/planting-low.png', [48, 32], [24, 30]),
        ('bloomwild_grass', 'grass/cuttable-selected-strip.png', [32, 32], [16, 28]),
    ]:
        source_path = SOURCES / source
        image = Image.open(source_path)
        assert image.height == cell[1] and image.width % cell[0] == 0
        shutil.copyfile(source_path, out / f'{name}.png')
        manifest(out, name, cell, anchor, image.width // cell[0],
                 'Selected native PixelLab source; no resampling, recoloring or mirroring. '
                 'See art/pixellab/2026-11-01/manifest.json for provenance.')
    atlas = Image.new('RGBA', (512, 96))
    for i, name in enumerate(['cliff-refined', 'cliff-variant-1', 'cliff-variant-2', 'cliff-variant-3']):
        raw = Image.open(SOURCES / f'props/{name}.png').convert('RGBA')
        assert raw.size == (128, 96)
        atlas.paste(raw, (i * 128, 0))
    atlas.save(out / 'bloomwild_cliff.png')
    manifest(out, 'bloomwild_cliff', [128, 96], [64, 26], 4,
             'Four native organic cliff variants, clipped to decorative island faces. '
             'Original refined frame plus selected Pro variants; upper-left lighting retained.')
    for script in ['build_bloomwild_structures.py', 'build_glasswater.py', 'build_cinder.py', 'build_summit.py',
                   'round6_bloom_floor.py', 'round6_botanical_variants.py', 'round6_fire_variants.py']:
        subprocess.run([sys.executable, str(TOOLS / script), '--out', str(out)], check=True)
    for script in ['round13_landmarks.py', 'round13_altar.py', 'round13_warden.py', 'audit_sapling.py']:
        subprocess.run([sys.executable, str(TOOLS / script), '--out', str(out),
                        '--normals-out', str(normals_out)], check=True)
    print('Exported the selected native environment sheets for all four regions:', out)


if __name__ == '__main__':
    main()
