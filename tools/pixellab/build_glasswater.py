"""Pack the reviewed Glasswater PixelLab art at native scale; no network or paid calls.

uv run --with pillow python tools/pixellab/build_glasswater.py
Use --out out/pixellab/round3/glasswater/sprites for a reversible candidate export.
"""
import argparse
import json
from pathlib import Path
from PIL import Image
from scenery_other_contacts import correct as correct_contacts

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / 'art/pixellab/2026-11-01-round3/glasswater'


def load(path, size):
    image = Image.open(SOURCES / path).convert('RGBA')
    image.load()
    assert image.size == tuple(size), (path, image.size, size)
    assert set(image.getchannel('A').tobytes()) <= {0, 255}, path
    return image


def pack(out, name, files, size, anchor, anim='body', notes=''):
    atlas = Image.new('RGBA', (size[0] * len(files), size[1]))
    for i, path in enumerate(files):
        atlas.paste(load(path, size), (i * size[0], 0))
    # Reviewed ground-facing RGB only; native alpha, metadata, emitters and
    # normal atlases remain paired with the original solid source geometry.
    atlas, _ = correct_contacts(name, atlas)
    atlas.save(out / (name + '.png'))
    data = {'cell': size, 'anchor': anchor,
            'anims': {anim: {'row': 0, 'frames': len(files), 'fps': 1}},
            'notes': notes or 'Native PixelLab sources, no resizing or recolouring. See art/pixellab/2026-11-01-round3/glasswater/selection.json.'}
    (out / (name + '.json')).write_text(json.dumps(data, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, default=ROOT / 'assets/sprites')
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    selection = json.loads((SOURCES / 'selection.json').read_text())
    for entry in selection['sheets']:
        pack(out, entry['name'], entry['files'], entry['size'], entry['anchor'], entry.get('anim', 'body'), entry.get('notes', ''))
    print('Exported', len(selection['sheets']), 'native Glasswater sheets to', out)


if __name__ == '__main__':
    main()
