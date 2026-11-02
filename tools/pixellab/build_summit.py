"""Pack curated Summit sources at their native pixel size; no network or generation.

uv run --with pillow python tools/pixellab/build_summit.py
"""
import argparse
import json
import hashlib
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / 'art/pixellab/2026-11-01-round3/summit'


def raw(path, size):
    im = Image.open(SOURCES / path).convert('RGBA')
    assert im.size == size, (path, im.size, size)
    assert set(im.getchannel('A').tobytes()) <= {0, 255}, path
    return im


def save(out, name, image, cell, anchor, anims, notes):
    image.save(out / f'{name}.png')
    data = {'cell': cell, 'anchor': anchor, 'emissive': [], 'anims': anims,
            'notes': notes + ' Native source pixels only; no resampling, mirroring or recoloring. '
                     'Rebuild: tools/pixellab/build_summit.py. Receipts: art/pixellab/2026-11-01-round3/summit/.'}
    (out / f'{name}.json').write_text(json.dumps(data, indent=2) + '\n')


def support_rects(image, first_row):
    """Exact opaque native footprint, merged only across identical consecutive spans.

    Bounds use source-cell coordinates [x0,y0,x1,y1], with exclusive right/bottom.
    Split pier feet and diagonal front corners retain their real gaps; a generous
    enclosing rectangle would invent invisible walls at the corners.
    """
    rects, active = [], {}
    for y in range(first_row, image.height + 1):
        spans, start = [], None
        for x in range(image.width + 1):
            opaque = y < image.height and x < image.width and image.getpixel((x, y))[3] != 0
            if opaque and start is None:
                start = x
            if not opaque and start is not None:
                spans.append((start, x))
                start = None
        for span in list(active):
            if span not in spans:
                rects.append(active.pop(span))
        for x0, x1 in spans:
            if (x0, x1) in active:
                active[(x0, x1)][3] = y + 1
            else:
                active[(x0, x1)] = [x0, y, x1, y + 1]
    expected = {(x, y) for y in range(first_row, image.height) for x in range(image.width)
                if image.getpixel((x, y))[3]}
    actual = {(x, y) for x0, y0, x1, y1 in rects for y in range(y0, y1) for x in range(x0, x1)}
    assert actual == expected
    return sorted(rects, key=lambda r: (r[1], r[0]))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out', type=Path, default=ROOT / 'assets/sprites')
    a = p.parse_args()
    out = a.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    selected = [0, 1, 4, 5, 14, 15]
    atlas = Image.new('RGBA', (32 * len(selected), 32))
    for col, n in enumerate(selected):
        tile = raw(f'floor/{n:02}.png', (32, 32))
        # The tiles tool adds a two-pixel dark staging gutter around flat squares.
        # Exclude that canvas gutter; retain all 28x28 interior pixels at native size.
        # The transparent margin reveals the existing slab base, so there is no grid.
        canvas = Image.new('RGBA', (32, 32))
        canvas.paste(tile.crop((2, 2, 30, 30)), (2, 2))
        atlas.paste(canvas, (col * 32, 0))
    save(out, 'summit_floor', atlas, [32, 32], [0, 0],
         {'stone': {'row': 0, 'frames': len(selected), 'fps': 1}},
         'Six quiet abrasion modules curated from sixteen, with the tool staging gutter masked transparent. '
         'Edge-reaching fractures, hole-like marks and conspicuous light lichen rejected. '
         'Sparse material clipped to original floor interiors, with combat-center coverage only 16%.')
    for name, path, size, anim in [
        ('summit_masonry', 'wall.result-1.png', (128, 96), 'wall'),
        ('summit_foundation', 'foundation.result-1.png', (128, 128), 'rock'),
        ('summit_dais', 'dais.result-1.png', (128, 80), 'dais'),
    ]:
        image = raw(path, size)
        anchor = [64, 52] if name == 'summit_dais' else [size[0] // 2, size[1] - 1]
        animation = {'row': 0, 'frames': 1, 'fps': 1}
        if name == 'summit_dais':
            animation['footprint'] = [89, 45]
            animation['collision_rects'] = support_rects(image, image.getbbox()[1])
        save(out, name, image, list(size), anchor,
             {anim: animation},
             'Static Summit architecture material; platform domain retained, dais collider fitted to visible native base.')
    for name, variants, anim_prefix in [
        ('summit_pillars', [4, 2, 1], 'pillar'),
        ('summit_stones', [1, 3], 'monolith'),
    ]:
        atlas = Image.new('RGBA', (64, 112 * len(variants)))
        anims = {}
        for row, n in enumerate(variants):
            image = raw(f'pier.result-{n}.png', (64, 112))
            atlas.paste(image, (0, row * 112))
            anchor_y = image.getbbox()[3] - 3
            base_row = {1: 79, 2: 79, 3: 76, 4: 78}[n]
            rects = support_rects(image, base_row)
            x0 = min(r[0] for r in rects)
            x1 = max(r[2] for r in rects)
            y1 = max(r[3] for r in rects)
            anims[f'{anim_prefix}_{row}'] = {'row': row, 'frames': 1, 'fps': 1,
                                           'anchor': [32, anchor_y],
                                           'footprint': [x1 - x0, y1 - base_row],
                                           'collision_rects': rects, 'support_first_row': base_row}
        save(out, name, atlas, [64, 112], [32, 99], anims,
             'Fractured slotted engineering piers replace geometric placeholders. Source-specific native foot anchors '
             'and exact opaque base collision rectangles, including asymmetric footings and gaps.')
    hashes = {}
    for f in sorted(SOURCES.rglob('*.png')):
        hashes[str(f.relative_to(SOURCES))] = hashlib.sha256(f.read_bytes()).hexdigest()
    manifest = {'date': '2026-11-01', 'generator': 'PixelLab MCP, Kyle subscription',
                'selected_floor': selected, 'selected_pillars': [4, 2, 1], 'selected_monoliths': [1, 3],
                'native_pixel_policy': 'No resampling, mirroring or recoloring during packing. Region applies the existing lighting model.',
                'source_sha256': hashes}
    (SOURCES / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print('Exported six native Summit sheets to', out)


if __name__ == '__main__':
    main()
