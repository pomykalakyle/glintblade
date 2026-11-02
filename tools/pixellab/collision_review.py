#!/usr/bin/env python3
"""Reproduce visible prop contacts with real game inputs in their authored map positions."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
# Every probe starts on the authored map and uses the ordinary movement input path.
# W300 lets the region title fade. Enemies are disabled to isolate physical scenery.
PROBES = {
    'tree-b-left': ((190, 627), 'R90'),
    'tree-b-right': ((295, 627), 'L90'),
    'tree-b-above': ((240, 590), 'D90'),
    'tree-b-below': ((240, 663), 'U90'),
    'broken-pillar-left': ((505, 614), 'R90'),
    'broken-pillar-right': ((600, 614), 'L90'),
    'broken-pillar-above': ((552, 575), 'D90'),
    'broken-pillar-below': ((552, 650), 'U90'),
    'entry-pier-right-cross': ((152, 598), 'R90'),
    'aqueduct-pier-right': ((425, 698), 'L90'),
    'aqueduct-passage': ((334, 728), 'U90'),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--tag', required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    binary_sha = hashlib.sha256(binary.read_bytes()).hexdigest()
    out = ROOT / 'out/pixellab/round4/collisions' / args.tag
    out.mkdir(parents=True, exist_ok=False)
    results = []
    for name, (start, movement) in PROBES.items():
        directory = out / name
        command = [str(binary), 'shot', '--zone', 'bloomwild', '--peaceful', '--god',
                   '--pos', ','.join(map(str, start)), '--script', 'W300 ' + movement,
                   '--ticks', '390', '--at', '300,330,360,390', '--scale', '1',
                   '--audit', '--out', str(directory), '--report', str(directory / 'report.json')]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=True)
        report = json.loads((directory / 'report.json').read_text())
        results.append({'name': name, 'start': start, 'movement': movement,
                        'end': report.get('end_pos'), 'command': command,
                        'frames': sorted(str(p.relative_to(ROOT)) for p in directory.glob('*.png')),
                        'stdout': result.stdout.strip()})
    assert binary_sha == hashlib.sha256(binary.read_bytes()).hexdigest()
    (out / 'manifest.json').write_text(json.dumps({'binary': str(binary), 'sha256': binary_sha,
                                                'probes': results}, indent=2) + '\n')
    print(out)


if __name__ == '__main__':
    main()
