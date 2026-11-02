#!/usr/bin/env python3
"""Refresh README media from one verified world capture, recording and motion reel.

Run with the PixelLab venv's Python (Pillow), or uv run --with pillow python.
Example: refresh_media.py --world-manifest out/pixellab/world/final/manifest.json \
    --recording out/pixellab/recordings/final --motion out/pixellab/review/final-motion \
    --work out/pixellab/review/final-media

Existing dirty docs/media files are backed up before installation. Source PNGs
stay on their native grid; clips use this recording's native frames and timed WAV.
--plan-only writes the provenance/selection plan without touching docs/media.
"""
import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
FPS = 60
NATIVE = (384, 216)
ENCODED = (768, 432)
SCREENS = {
    '02_bloomwild_entry.png': ('bloomwild', 'entry'),
    '03_bloomwild_falls.png': ('bloomwild', 'falls'),
    '04_glasswater_causeway.png': ('glasswater', 'causeway'),
    '05_glasswater_gallery.png': ('glasswater', 'gallery'),
    '06_cinder_arrival.png': ('cinder', 'approach'),
    '07_cinder_ruins.png': ('cinder', 'press'),
    '08_cinder_lava.png': ('cinder', 'caldera-bridge'),
    '09_summit_arena.png': ('summit', 'arena'),
}
COLLAGE = [('bloomwild', 'entry'), ('glasswater', 'gallery'),
           ('cinder', 'caldera-bridge'), ('summit', 'stair')]


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text())


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n')


def probe(path):
    result = subprocess.check_output(['ffprobe', '-v', 'error', '-show_streams',
                                      '-show_format', '-of', 'json', str(path)])
    raw = json.loads(result)
    keys = ['codec_type', 'codec_name', 'width', 'height', 'r_frame_rate',
            'avg_frame_rate', 'nb_frames', 'duration', 'sample_rate', 'channels']
    return {'streams': [{k: stream[k] for k in keys if k in stream}
                        for stream in raw['streams']],
            'duration': raw.get('format', {}).get('duration')}


def verify_video(path, frames=None):
    data = probe(path)
    video = next(s for s in data['streams'] if s['codec_type'] == 'video')
    assert (video['width'], video['height']) == ENCODED, (path, 'wrong pixel dimensions')
    assert Fraction(video['r_frame_rate']) == FPS, (path, 'video must remain 60fps')
    assert any(s['codec_type'] == 'audio' for s in data['streams']), (path, 'missing original audio')
    if frames is not None:
        assert int(video['nb_frames']) == frames, (path, 'wrong frame count')
        assert abs(float(video['duration']) - frames / FPS) <= 1 / FPS + 0.00001
    return data


def source_file(value, base):
    path = Path(value)
    if path.is_absolute():
        return path
    # Capture tools normally write absolute paths. Also accept their repo-relative paths.
    return (base / path).resolve() if (base / path).is_file() else (ROOT / path).resolve()


def native_image(path):
    image = Image.open(path).convert('RGB')
    assert image.size == NATIVE, (path, image.size, 'source must be native 384x216')
    return image


def make_collage(captures, manifest_base, destination):
    collage = Image.new('RGB', (NATIVE[0] * 2, NATIVE[1] * 2))
    sources = []
    for index, subject in enumerate(COLLAGE):
        capture = captures[subject]
        source = source_file(capture['file'], manifest_base)
        assert sha(source) == capture['sha256']
        collage.paste(native_image(source), ((index % 2) * NATIVE[0], (index // 2) * NATIVE[1]))
        sources.append({'source': str(source), 'sha256': sha(source), 'subject': subject})
    collage.resize((1536, 864), Image.Resampling.NEAREST).save(destination)
    return {'file': 'highlights.png', 'pixels': [1536, 864], 'native_scale': 2,
            'resampling': 'nearest', 'sources': sources}


def frame_path(recording, tick):
    path = recording / 'frames' / f'f{tick:05}.png'
    assert path.is_file(), f'Missing actual recorded tick {tick}: {path}'
    return path


def select_cinder(report, first_tick, final_tick):
    rows = [r for r in report['trace'] if r['zone'] == 'cinder']
    assert rows, 'Recording never visits Cinder'
    lo, hi = max(first_tick, min(r['tick'] for r in rows)), min(final_tick, max(r['tick'] for r in rows))
    count, half = 12 * FPS, 6 * FPS
    eligible = [r for r in rows if lo <= r['tick'] - half and r['tick'] + half - 1 <= hi]
    assert eligible, 'Need twelve uninterrupted seconds inside Cinder'
    fighting = [r for r in eligible if r['enemies_alive'] > 0]
    assert fighting, 'No Cinder combat remains in the usable recording window'
    target = (621, 510)
    center = min(fighting, key=lambda r: sum((a - b) ** 2 for a, b in zip(r['pos'], target)))
    first = center['tick'] - half
    last = first + count - 1
    attacks = [e for e in report['audio_events'] if first <= e['tick'] + 2 <= last
               and e.get('sound', '').startswith('slash_')]
    assert attacks, 'Selected Cinder combat window contains no actual sword attacks'
    # Select a real active sword frame near the foundry, rather than a static landscape.
    def screenshot_score(event):
        row = min(rows, key=lambda r: abs(r['tick'] - event['tick']))
        return (sum((a - b) ** 2 for a, b in zip(row['pos'], target)), abs(event['tick'] - center['tick']))
    strike = min(attacks, key=screenshot_score)
    return {'first_tick': first, 'frames': count, 'last_tick': last,
            'selection': 'Twelve seconds around the closest live encounter to the foundry press',
            'subject_target': target, 'nearest_trace': center,
            'combat_image_tick': strike['tick'] + 2, 'combat_image_event': strike}


def clip_native(recording, first_recorded, first_tick, frames, destination):
    duration = frames / FPS
    command = ['ffmpeg', '-v', 'error', '-y', '-framerate', str(FPS),
               '-start_number', str(first_tick), '-i', str(recording / 'frames/f%05d.png'),
               '-ss', str((first_tick - first_recorded) / FPS), '-i', str(recording / 'audio.wav'),
               '-map', '0:v:0', '-map', '1:a:0', '-frames:v', str(frames), '-t', str(duration),
               '-vf', 'scale=768:432:flags=neighbor', '-c:v', 'libx264', '-preset', 'slow',
               '-crf', '17', '-pix_fmt', 'yuv420p', '-threads', '4', '-c:a', 'aac',
               '-b:a', '192k', '-movflags', '+faststart', str(destination)]
    subprocess.run(command, check=True)
    return {'first_tick': first_tick, 'frames': frames, 'fps': FPS,
            'source_audio_offset_seconds': (first_tick - first_recorded) / FPS,
            'retimed': False, 'command': command, 'ffprobe': verify_video(destination, frames)}


def file_inventory(directory):
    return [{'file': str(p.relative_to(directory)), 'bytes': p.stat().st_size, 'sha256': sha(p)}
            for p in sorted(directory.rglob('*')) if p.is_file()]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--world-manifest', required=True, type=Path)
    parser.add_argument('--recording', required=True, type=Path)
    parser.add_argument('--motion', required=True, type=Path, help='make_motion.py output directory')
    parser.add_argument('--work', required=True, type=Path, help='New audit/backup/staging directory')
    parser.add_argument('--media', type=Path, default=ROOT / 'docs/media')
    parser.add_argument('--plan-only', action='store_true', help='Validate sources and write selections without changing docs/media')
    args = parser.parse_args()
    world_path, recording, motion = args.world_manifest.resolve(), args.recording.resolve(), args.motion.resolve()
    world, rec, reel = read_json(world_path), read_json(recording / 'manifest.json'), read_json(motion / 'manifest.json')
    report_path = source_file(rec['report'], recording)
    report = read_json(report_path)
    assert rec['fps'] == rec['simulation_hz'] == reel['fps'] == FPS
    assert rec['native_pixels'] == list(NATIVE) and rec['encoded_pixels'] == list(ENCODED)
    assert rec['integer_scale'] == 2 and reel['retimed'] is False
    assert world.get('GLINT_SPRITES') is None and rec.get('GLINT_SPRITES') is None, 'Use the embedded final assets, without overrides'
    assert world['binary_sha256'] == rec['binary_sha256'] == reel['source_binary_sha256'], 'Source binary hashes differ'
    assert rec['video_sha256'] == reel['source_video_sha256'] == sha(recording / 'gameplay.mp4'), 'Recording hashes differ'
    assert Path(reel['source_recording']).resolve() == recording, 'Motion reel belongs to another recording'
    assert not rec['audio']['missing_sounds'], 'Recording has missing audio assets'
    assert (recording / 'audio.wav').is_file(), 'Need the original timed WAV, not a second AAC transcode'
    verify_video(recording / 'gameplay.mp4', rec['frames'])
    verify_video(motion / 'samples.mp4', len(reel['clips']) * reel['frames_per_clip'])
    frames = sorted((recording / 'frames').glob('f*.png'))
    first_tick, final_tick = int(frames[0].stem[1:]), int(frames[-1].stem[1:])
    assert len(frames) == rec['frames'] == final_tick - first_tick + 1, 'Recording frames must be contiguous at 60Hz'
    captures = {(c['zone'], c['name']): c for c in world['captures']}
    shots = {}
    for destination, subject in SCREENS.items():
        capture = captures[subject]
        source = source_file(capture['file'], world_path.parent)
        assert sha(source) == capture['sha256'], f'World capture changed: {source}'
        shots[destination] = {'source': str(source), 'source_sha256': sha(source),
                              'zone': subject[0], 'scene': subject[1], 'kind': 'scene study'}
    title = source_file(world['title'], world_path.parent)
    shots['01_title.png'] = {'source': str(title), 'source_sha256': sha(title), 'kind': 'actual title screen'}
    cinder = select_cinder(report, first_tick, final_tick)
    milestones = report['milestones']
    phase_name = 'phase_2_heart_open' if 'phase_2_heart_open' in milestones else 'phase_2_damage'
    for destination, tick, meaning in [
        ('10_combat.png', cinder['combat_image_tick'], 'Cinder sword attack + 2 ticks'),
        ('12_warden_phase_2.png', milestones[phase_name], phase_name),
    ]:
        source = frame_path(recording, tick)
        shots[destination] = {'source': str(source), 'source_sha256': sha(source),
                              'tick': tick, 'kind': 'actual gameplay', 'selection': meaning}
    awake = milestones['warden_awake']
    warden_first = max(first_tick, awake - 55)
    warden_frames = final_tick - warden_first + 1
    assert report['victory_ready'] and milestones['victory'] >= awake
    plan = {'world_manifest': str(world_path), 'world_manifest_sha256': sha(world_path),
            'recording': str(recording), 'recording_manifest_sha256': sha(recording / 'manifest.json'),
            'report_sha256': sha(report_path), 'source_video_sha256': rec['video_sha256'],
            'source_audio_sha256': sha(recording / 'audio.wav'), 'binary_sha256': rec['binary_sha256'],
            'motion': str(motion), 'motion_manifest_sha256': sha(motion / 'manifest.json'),
            'samples_sha256': sha(motion / 'samples.mp4'), 'god': rec['god'],
            'fps': FPS, 'simulation_hz': FPS, 'source_tick_range': [first_tick, final_tick],
            'screenshots': shots, 'screenshot_scale': 3, 'screenshot_pixels': [1152, 648],
            'collage': COLLAGE, 'collage_native_scale': 2, 'cinder_combat': cinder,
            'warden_fight': {'milestone': 'warden_awake', 'milestone_tick': awake, 'lead_in_ticks': awake - warden_first,
                             'first_tick': warden_first, 'frames': warden_frames, 'last_tick': final_tick},
            'media_destination': str(args.media.resolve()), 'plan_only': args.plan_only}
    # Validate every screenshot before any destination/backup mutation.
    for spec in shots.values():
        native_image(spec['source'])
    work, media = args.work.resolve(), args.media.resolve()
    assert not work.is_relative_to(media) and not media.is_relative_to(work), 'Audit/backup and media directories must be separate'
    work.mkdir(parents=True, exist_ok=False)
    write_json(work / 'plan.json', plan)
    if args.plan_only:
        print(work / 'plan.json')
        return

    staged = work / 'rendered-media'
    staged.mkdir()
    outputs = []
    for name, spec in shots.items():
        native_image(spec['source']).resize((1152, 648), Image.Resampling.NEAREST).save(staged / name)
        outputs.append({'file': name, **spec, 'pixels': [1152, 648], 'scale': 3, 'resampling': 'nearest'})
    outputs.append(make_collage(captures, world_path.parent, staged / 'highlights.png'))
    shutil.copy2(motion / 'samples.mp4', staged / 'highlights.mp4')
    outputs.append({'file': 'highlights.mp4', 'source': str(motion / 'samples.mp4'),
                    'source_sha256': sha(motion / 'samples.mp4'), 'copied_byte_for_byte': True,
                    'ffprobe': verify_video(staged / 'highlights.mp4', len(reel['clips']) * reel['frames_per_clip'])})
    outputs.append({'file': 'combat.mp4', **clip_native(recording, first_tick, cinder['first_tick'], cinder['frames'], staged / 'combat.mp4')})
    outputs.append({'file': 'warden_fight.mp4', **clip_native(recording, first_tick, warden_first, warden_frames, staged / 'warden_fight.mp4')})
    for item in outputs:
        path = staged / item['file']
        item.update(sha256=sha(path), bytes=path.stat().st_size)

    # Preserve the user's current dirty media, including files not referenced by README.
    backup = work / 'original-docs-media'
    if media.exists():
        shutil.copytree(media, backup)
    else:
        backup.mkdir()
    backup_inventory = file_inventory(backup)
    media.mkdir(parents=True, exist_ok=True)
    for item in outputs:
        shutil.copy2(staged / item['file'], media / item['file'])
    # Keep older unlinked media in place; the README's named outputs are the refresh scope.
    output_names = {item['file'] for item in outputs}
    inventory = {'plan': plan, 'outputs': outputs, 'backup': str(backup),
                 'backup_inventory': backup_inventory,
                 'preserved_unmapped_files': [item for item in backup_inventory if item['file'] not in output_names],
                 'installed_inventory': file_inventory(media)}
    assert all(sha(media / item['file']) == item['sha256'] for item in outputs)
    write_json(work / 'inventory.json', inventory)
    print(f'Refreshed {len(outputs)} files from binary {rec["binary_sha256"]}.')
    print(f'Backup: {backup}')
    print(f'Inventory: {work / "inventory.json"}')


if __name__ == '__main__':
    main()
