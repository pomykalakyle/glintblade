"""Remove authored ground-facing ink from otherwise unchanged static scenery.

This is a pure RGB post-export pass. Coordinates are reviewed native support
contours, not a lowest-opaque-pixel rule. Slots, arch undersides, raised tiers,
canopies, side ink, alpha, normals, anchors and emitters are left alone.

Glasswater's scoped packer calls ``correct`` before saving its four sheets.
For the other independent exporters, post-process only their named output::

    uv run --with pillow python tools/pixellab/scenery_other_contacts.py \
        --sprites assets/sprites --out /tmp/contact-sprites \
        --families cinder_brazier summit_dais --report /tmp/contacts.json

Do not run the Cinder or Summit umbrella exporters to apply this correction.
"""
import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
INK = {(24, 35, 49), (34, 45, 61)}
ROCK = {(54, 70, 87), (66, 73, 86), (76, 96, 112), (98, 106, 116),
        (115, 133, 139), (146, 151, 151)}
PURPLE = {(77, 72, 102), (121, 108, 145), (161, 149, 184)}
FUNGUS = ROCK | {(41, 49, 63)}

# Each entry is (cell column, cell row, native cell dimensions, material,
# [(y, inclusive x0, inclusive x1), ...]). Only INK-colored pixels in these
# deliberate spans change. Extra tuples retain separate front rubble material.
CONTACTS = {
    'glasswater_crystals_native': [
        (0, 0, (48, 64), 'rock', [
            (53, 4, 4), (53, 43, 43), (54, 5, 6), (54, 41, 42),
            (55, 7, 8), (55, 24, 26), (55, 30, 32), (55, 37, 40),
            (56, 9, 10), (56, 22, 23), (56, 33, 36),
            (57, 11, 12), (57, 19, 21), (58, 13, 18)]),
        (2, 0, (48, 64), 'rock', [
            (54, 7, 8), (54, 37, 40), (55, 9, 11), (55, 36, 38),
            (56, 33, 35), (57, 15, 16), (57, 29, 32),
            (58, 16, 17), (58, 26, 29), (59, 18, 25)]),
        (5, 0, (48, 64), 'rock', [
            (54, 5, 5), (54, 30, 32), (55, 6, 6), (55, 28, 29),
            (56, 7, 7), (56, 23, 25), (57, 8, 9), (57, 21, 22),
            (58, 10, 13), (58, 18, 20), (59, 14, 17)]),
        (7, 0, (48, 64), 'rock', [
            (53, 6, 6), (53, 41, 42), (54, 7, 8), (54, 38, 41),
            (55, 9, 12), (55, 35, 37), (56, 12, 18),
            (56, 28, 30), (57, 23, 27)]),
        (7, 0, (48, 64), 'purple', [(59, 18, 21), (58, 31, 33)]),
    ],
    'glasswater_crystals_small': [
        # All seven currently placed variants have a single authored support row.
        (c, 0, (32, 40), 'rock', [(37, x0, x1)])
        for c, x0, x1 in [(0, 7, 24), (1, 6, 24), (2, 7, 25),
                           (3, 7, 25), (4, 6, 24), (6, 7, 25), (7, 7, 24)]
    ],
    'glasswater_fungi': [
        # Frame 0's broad black stump recess is intentional. Caps/gills and
        # the split between frame 2's two rooted rocks remain fully intact.
        (2, 0, (48, 48), 'fungus', [(42, 12, 12), (42, 42, 43),
                                     (43, 13, 29), (43, 34, 41)]),
        (3, 0, (48, 48), 'fungus', [(41, 14, 14), (41, 35, 35),
                                     (42, 15, 16), (42, 32, 34), (43, 17, 31)]),
    ],
    'glasswater_waterworks': [
        # Both live non-distributor fins select frame 0. Three distinct native
        # feet are kept distinct; the overhead slot/spout is outside this mask.
        (0, 0, (96, 96), 'rock', [
            (85, 35, 36), (86, 25, 25), (86, 33, 34),
            (87, 26, 27), (87, 31, 32), (88, 28, 30),
            (85, 39, 40), (85, 47, 49), (86, 41, 42), (86, 45, 46),
            (87, 43, 44), (86, 50, 51), (86, 67, 68),
            (87, 52, 53), (87, 65, 66), (88, 54, 55),
            (88, 63, 64), (89, 56, 62)]),
    ],
    'cinder_brazier': [
        # Bottom outline was explicitly added by base().outline('K'). Copy
        # the neighboring pedestal face into that line, with flame RGB intact.
        (c, r, (16, 32), 'cinder', [(31, 3, 12)])
        for c, r in [(c, 0) for c in range(6)] + [(0, 1)]
    ],
    'summit_dais': [
        # Only the straight support contour; the socket, tier boundaries,
        # diagonal side outlines and all original volume remain unchanged.
        (0, 0, (128, 80), 'summit', [(62, 41, 83)]),
    ],
}
MATERIALS = {
    'rock': ROCK, 'purple': PURPLE, 'fungus': FUNGUS,
    'cinder': {(26, 25, 35), (41, 44, 60), (63, 65, 79)},
    'summit': {(43, 53, 72), (51, 59, 76), (85, 93, 112)},
}
INK_BY_MATERIAL = {'cinder': {(16, 16, 23)}, 'summit': {(31, 40, 56)}}


def authored_mask(name):
    return {(cx*cw+x, cy*ch+y)
            for cx, cy, (cw, ch), _, spans in CONTACTS.get(name, [])
            for y, x0, x1 in spans for x in range(x0, x1+1)}


def correct(name, image):
    """Return corrected RGBA plus an exact old/new/donor pixel receipt.

    Material donors are unchanged adjacent original pixels in the same cell.
    Prefer the nearest interior row, then the nearest lateral material pixel.
    Excluding all authored mask pixels makes repeated exports idempotent.
    """
    original = image.convert('RGBA')
    result = original.copy()
    mask = authored_mask(name)
    changes = []
    for cx, cy, (cw, ch), material, spans in CONTACTS.get(name, []):
        allowed = MATERIALS[material]
        ink = INK_BY_MATERIAL.get(material, INK)
        for y, x0, x1 in spans:
            for x in range(x0, x1+1):
                px, py = cx*cw+x, cy*ch+y
                old = original.getpixel((px, py))
                if old[3] != 255 or old[:3] not in ink:
                    continue
                donors = []
                for dy in range(0, 9):
                    for dx in range(-6, 7):
                        nx, ny = x+dx, y-dy
                        if not 0 <= nx < cw or not 0 <= ny < ch:
                            continue
                        ax, ay = cx*cw+nx, cy*ch+ny
                        candidate = original.getpixel((ax, ay))
                        if (ax, ay) in mask or candidate[3] != 255 or candidate[:3] not in allowed:
                            continue
                        # Same-column face continuation, not a distant lighter top.
                        distance = dy*dy + dx*dx*2 + (dy == 0)
                        donors.append((distance, abs(dx), dy, ax, ay, candidate))
                if not donors:
                    raise ValueError(f'{name}: no local {material} donor for {px},{py}')
                _, _, _, ax, ay, new = min(donors)
                result.putpixel((px, py), new)
                changes.append({'pixel': [px, py], 'old': list(old), 'new': list(new),
                                'donor': [ax, ay], 'material': material})
    assert result.size == original.size
    assert result.getchannel('A').tobytes() == original.getchannel('A').tobytes()
    changed = {(i % original.width, i // original.width)
               for i, (a, b) in enumerate(zip(original.getdata(), result.getdata())) if a != b}
    assert changed <= mask and len(changed) == len(changes)
    return result, changes


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--sprites', type=Path, default=ROOT/'assets/sprites')
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--families', nargs='+', choices=sorted(CONTACTS), default=sorted(CONTACTS))
    ap.add_argument('--report', type=Path)
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    report = {}
    for name in a.families:
        source = a.sprites/f'{name}.png'
        old = Image.open(source).convert('RGBA')
        new, changes = correct(name, old)
        # Idempotency and exact emission color membership, including unchanged
        # nonopaque RGB. No JSON/masks/normals are written by this exporter.
        repeated, repeated_changes = correct(name, new)
        assert not repeated_changes and repeated.tobytes() == new.tobytes()
        metadata = a.sprites/f'{name}.json'
        data = json.loads(metadata.read_text())
        emit = {tuple(bytes.fromhex(c.lstrip('#'))) for c in data.get('emissive', [])}
        assert all((tuple(c['old'][:3]) in emit) == (tuple(c['new'][:3]) in emit) for c in changes)
        target = a.out/f'{name}.png'
        new.save(target)
        report[name] = {'source': str(source.resolve()), 'source_sha256': sha(source),
                        'output': str(target.resolve()), 'output_sha256': sha(target),
                        'metadata_sha256': sha(metadata), 'changed_pixels': len(changes),
                        'invariants': {'canvas': True, 'alpha': True, 'metadata_untouched': True,
                            'normals_untouched': True, 'emission_membership': True,
                            'rgb_outside_authored_contacts': True, 'idempotent': True},
                        'changes': changes}
    if a.report:
        a.report.parent.mkdir(parents=True, exist_ok=True)
        a.report.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k: v['changed_pixels'] for k, v in report.items()}, sort_keys=True))


if __name__ == '__main__':
    main()
