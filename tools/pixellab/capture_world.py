#!/usr/bin/env python3
"""Capture comparable native scene studies and full overviews across all four regions.

python3 tools/pixellab/capture_world.py --tag final
Use --binary PATH to repeat the same studies with a preserved baseline executable.
These are scene studies; record.py captures real combat, camera motion and HUD.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCENES = {
    'bloomwild': [('landing', (258,696)), ('entry', (258,640)), ('courtyard', (456,536)),
                  ('temple', (1056,464)), ('terrace', (664,340)), ('altar', (408,176)),
                  ('falls', (780,700)), ('grove', (934,188))],
    'glasswater': [('lake', (300,416)), ('island', (500,440)), ('gallery', (550,102)),
                   ('bridge', (850,250)), ('grove', (1200,230)), ('heart', (1170,700))],
    'cinder': [('approach', (136,752)), ('crossing', (414,480)), ('foundry', (621,510)),
               ('press', (621,460)), ('upper-bank', (880,330)), ('shrine', (1144,200))],
    'summit': [('arrival', (196,782)), ('bridge', (480,776)), ('stair', (720,696)),
               ('arena', (720,520)), ('dais', (720,280)), ('west', (490,510))],
}

DETAIL_SCENES = {
    'bloomwild': [('upper-bridge', (712,274)), ('lower-bridge', (840,590)),
                  ('stair-contact', (1128,340)), ('west-terrace', (290,344)),
                  ('cliff-contact', (420,718)), ('falls-impact', (712,356)),
                  ('outlet-bank', (812,698))],
    'glasswater': [('channel', (656,160)), ('causeway', (848,250)),
                   ('shallow-grove', (1120,280))],
    'cinder': [('canal-bridge', (880,616)), ('caldera-bridge', (1144,280)),
               ('river-join', (440,616))],
    'summit': [('curved-edge', (470,320))],
}

# These fixed camera centers frame a wall, deep pool or shore edge. A scene study
# may center on scenery, but must not stage Glint on physically blocked ground.
# Use the same omission in both matched builds so their camera framing stays exact.
SCENERY_ONLY = {('bloomwild', 'terrace'), ('bloomwild', 'west-terrace'),
                ('bloomwild', 'falls-impact'), ('glasswater', 'island'),
                ('glasswater', 'channel')}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/glintblade')
    parser.add_argument('--detail', action='store_true', help='Include shore, bridge and cliff contact studies')
    args = parser.parse_args()
    if not args.tag or Path(args.tag).name != args.tag or args.tag in ['.', '..']:
        parser.error('--tag must be one directory name')
    out = ROOT / 'out/pixellab/world' / args.tag
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    before = sha(binary)
    env = os.environ.copy()
    env.pop('GLINT_SPRITES', None)
    manifest = {'binary': str(binary), 'binary_sha256': before, 'GLINT_SPRITES': None,
                'native_resolution': [384,216], 'kind': 'scene studies, not gameplay', 'captures': []}
    for zone, base_scenes in SCENES.items():
        scenes = base_scenes + (DETAIL_SCENES[zone] if args.detail else [])
        for index, (name, pos) in enumerate(scenes):
            folder = out / zone / name
            command = [str(binary), 'zoneview', zone, '--at', ','.join(map(str, pos)),
                       '--warm', '90', '--scale', '1', '--out', str(folder)]
            hero = (zone, name) not in SCENERY_ONLY
            if not hero:
                command += ['--no-hero']
            if index == 0:
                command += ['--overview']
            subprocess.run(command, cwd=ROOT, env=env, check=True, capture_output=True)
            image = folder / f'{zone}_t00090.png'
            manifest['captures'].append({'zone': zone, 'name': name, 'position': pos,
                'hero_visible': hero, 'file': str(image), 'sha256': sha(image), 'command': command})
        print(f'Captured {zone}: {len(scenes)} native views + full overview', flush=True)
    command = [str(binary), 'shot', '--ticks', '240', '--scale', '1', '--out', str(out / 'title')]
    subprocess.run(command, cwd=ROOT, env=env, check=True, capture_output=True)
    manifest['title'] = str(out / 'title/f00240.png')
    assert sha(binary) == before, 'Binary changed during captures'
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(out / 'manifest.json')


if __name__ == '__main__':
    main()
