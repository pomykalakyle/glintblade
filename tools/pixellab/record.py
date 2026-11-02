#!/usr/bin/env python3
"""Record real game ticks at 60 fps, with synchronized emitted sound and music.

Run with the PixelLab venv's Python (numpy), or uv run --with numpy python.
Example: record.py --tag campaign --pilot tour --god --until-victory --ticks 36000
No window, server, generation or purchase is involved. --god is recorded in the manifest.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import wave

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
RATE = 48000


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def decode(path):
    data = subprocess.check_output(['ffmpeg', '-v', 'error', '-i', str(path), '-f', 'f32le',
                                    '-ac', '2', '-ar', str(RATE), 'pipe:1'])
    return np.frombuffer(data, dtype='<f4').reshape(-1, 2)


def mix(report, first, count, fps, out):
    """Offline frontend mix: original cues, game tick timestamps, same gains/debounce."""
    start = (first - 1) / 60
    duration = count / fps
    audio = np.zeros((round(duration * RATE), 2), dtype=np.float32)
    cache, last = {}, {}
    missing = set()
    cues = report['audio_events']
    music = [e for e in cues if 'music' in e]
    for i, event in enumerate(music):
        name = event['music']
        path = ROOT / 'assets/music' / f'{name}.ogg'
        if not name or not path.is_file():
            continue
        samples = decode(path)
        began = (event['tick'] - 1) / 60
        following = (music[i + 1]['tick'] - 1) / 60 if i + 1 < len(music) else start + duration
        # Frontend increases/decreases gain by .008 per 60 Hz frame, max .55.
        fade = .55 / (.008 * 60)
        lo, hi = max(start, began), min(start + duration, following + fade)
        for offset in range(round((lo - start) * RATE), round((hi - start) * RATE), RATE):
            n = min(RATE, len(audio) - offset, round((hi - start) * RATE) - offset)
            if n <= 0:
                break
            now = start + (offset + np.arange(n)) / RATE
            gain = np.minimum(.55, np.maximum(0, (now - began) * .008 * 60))
            gain = np.minimum(gain, np.maximum(0, .55 - (now - following) * .008 * 60))
            indices = np.floor((now - began) * RATE).astype(np.int64)
            if name == 'victory':
                gain *= indices < len(samples)
            audio[offset:offset + n] += samples[indices % len(samples)] * gain[:, None]
    for event in cues:
        if 'sound' not in event:
            continue
        name = event['sound']
        path = ROOT / 'assets/sfx' / f'{name}.wav'
        if not path.is_file():
            stem, _, tail = name.rpartition('_')
            fallback = ROOT / 'assets/sfx' / f'{stem}.wav'
            if tail.isdigit() and fallback.is_file():
                name, path = stem, fallback
            else:
                name += '_1'
                path = ROOT / 'assets/sfx' / f'{name}.wav'
        if not path.is_file():
            missing.add(event['sound'])
            continue
        when = (event['tick'] - 1) / 60
        if when - last.get(name, -100) < .035:
            continue
        last[name] = when
        if name not in cache:
            cache[name] = decode(path)
        samples = cache[name]
        at = round((when - start) * RATE)
        src, dst = max(0, -at), max(0, at)
        n = min(len(samples) - src, len(audio) - dst)
        if n > 0:
            audio[dst:dst + n] += samples[src:src + n] * min(1, max(0, event['volume'] * .8))
    peak = float(np.max(np.abs(audio))) if len(audio) else 0
    gain = min(1, .98 / max(peak, .0001))
    with wave.open(str(out), 'wb') as stream:
        stream.setparams((2, 2, RATE, len(audio), 'NONE', 'not compressed'))
        stream.writeframes((np.clip(audio * gain, -1, 1) * 32767).astype('<i2').tobytes())
    return {'events': len(cues), 'missing_sounds': sorted(missing), 'unscaled_peak': peak,
            'gain_to_avoid_clipping': gain, 'mix': 'Original assets, emitted 60 Hz events, frontend gains/debounce; offline mix'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/glintblade')
    parser.add_argument('--zone', choices=['bloomwild', 'glasswater', 'cinder', 'summit'])
    parser.add_argument('--pos')
    parser.add_argument('--script')
    parser.add_argument('--pilot', choices=['tour', 'warden'])
    parser.add_argument('--ticks', type=int, default=36000)
    parser.add_argument('--from-tick', type=int, default=1)
    parser.add_argument('--fps', type=int, choices=[30, 60], default=60)
    parser.add_argument('--god', action='store_true')
    parser.add_argument('--peaceful', action='store_true')
    parser.add_argument('--until-victory', action='store_true')
    args = parser.parse_args()
    if not args.tag or Path(args.tag).name != args.tag or args.tag in ['.', '..']:
        parser.error('--tag must be one directory name')
    out = ROOT / 'out/pixellab/recordings' / args.tag
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    before = sha(binary)
    command = [str(binary), 'shot', '--ticks', str(args.ticks), '--from', str(args.from_tick),
               '--every', str(60 // args.fps), '--scale', '1', '--seed', '7', '--audit',
               '--out', str(out / 'frames'), '--report', str(out / 'report.json')]
    for key in ['zone', 'pos', 'script', 'pilot']:
        value = getattr(args, key)
        if value:
            command += ['--' + key, value]
    for key in ['god', 'peaceful', 'until_victory']:
        if getattr(args, key):
            command += ['--' + key.replace('_', '-')]
    env = os.environ.copy()
    env.pop('GLINT_SPRITES', None)
    subprocess.run(command, cwd=ROOT, env=env, check=True)
    assert sha(binary) == before, 'Binary changed during recording'
    report = json.loads((out / 'report.json').read_text())
    frames = sorted((out / 'frames').glob('f*.png'))
    assert len(frames) == report['frames'] and frames
    first = int(frames[0].stem[1:])
    audit = mix(report, first, len(frames), args.fps, out / 'audio.wav')
    # 2x before 4:2:0 encoding preserves a full native pixel of chroma resolution.
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-framerate', str(args.fps), '-pattern_type', 'glob',
                    '-i', str(out / 'frames/*.png'), '-i', str(out / 'audio.wav'), '-vf', 'scale=768:432:flags=neighbor',
                    '-c:v', 'libx264', '-preset', 'slow', '-crf', '17', '-pix_fmt', 'yuv420p', '-threads', '4',
                    '-c:a', 'aac', '-b:a', '192k', '-movflags', '+faststart', '-shortest', str(out / 'gameplay.mp4')], check=True)
    (out / 'manifest.json').write_text(json.dumps({'command': command, 'cwd': str(ROOT), 'binary_sha256': before,
        'GLINT_SPRITES': None, 'fps': args.fps, 'simulation_hz': 60, 'frames': len(frames),
        'duration_seconds': len(frames) / args.fps, 'native_pixels': [384, 216], 'encoded_pixels': [768, 432],
        'integer_scale': 2, 'god': args.god, 'audio': audit, 'report': 'report.json',
        'video_sha256': sha(out / 'gameplay.mp4')}, indent=2) + '\n')
    print(out / 'gameplay.mp4', flush=True)


if __name__ == '__main__':
    main()
