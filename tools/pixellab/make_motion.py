#!/usr/bin/env python3
"""Make an integer-scale 60fps sample reel and a portable inline player.

Uses actual frames/audio from record.py. Never retimes footage or covers the HUD.
The compact WebM is a preview; the MP4 retains the recording's quality settings.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SUBJECTS = [('Bloomwild', (712, 274)), ('Glasswater', (656, 160)),
            ('Cinder', (414, 480)), ('Summit', (480, 776))]
FRAMES = 120
FPS = 60


def run(command):
    subprocess.run(command, check=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--recording', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    args = p.parse_args()
    source = args.recording.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((source / 'manifest.json').read_text())
    assert manifest['fps'] == FPS and manifest['simulation_hz'] == FPS
    assert manifest['native_pixels'] == [384, 216] and manifest['integer_scale'] == 2
    # Route timings change when collision geometry changes. Select subjects from this
    # recording's audit rather than reusing tick offsets from an older executable.
    report = json.loads((source / manifest['report']).read_text())
    trace = report['trace']
    clips, selections = [], []
    for name, target in SUBJECTS:
        rows = [r for r in trace if r['zone'] == name.lower()]
        assert rows, f'Recording never visits {name}'
        first, last = min(r['tick'] for r in rows), max(r['tick'] for r in rows)
        candidates = [r for r in rows if first + FRAMES // 2 <= r['tick'] <= last - FRAMES // 2]
        assert candidates, f'Not enough uninterrupted footage in {name}'
        nearest = min(candidates, key=lambda r: sum((a-b)**2 for a,b in zip(r['pos'], target)))
        tick = nearest['tick'] - FRAMES // 2
        assert (source / f'frames/f{tick:05}.png').is_file()
        clips.append((name, tick))
        selections.append({'name': name, 'first_tick': tick, 'subject_target': target,
                           'nearest_tick': nearest['tick'], 'nearest_position': nearest['pos']})
    inputs, filters, joins = [], [], []
    for i, (_, tick) in enumerate(clips):
        inputs += ['-ss', str((tick - 1) / FPS), '-t', str(FRAMES / FPS), '-i', str(source / 'gameplay.mp4')]
        filters += [f'[{i}:v]setpts=PTS-STARTPTS[v{i}]',
                    f'[{i}:a]asetpts=PTS-STARTPTS,afade=t=in:d=0.02,afade=t=out:st={FRAMES / FPS - .02}:d=0.02[a{i}]']
        joins += [f'[v{i}][a{i}]']
    filters.append(''.join(joins) + f'concat=n={len(clips)}:v=1:a=1[v][a]')
    run(['ffmpeg', '-v', 'error', '-y', *inputs, '-filter_complex', ';'.join(filters),
         '-map', '[v]', '-map', '[a]', '-c:v', 'libx264', '-crf', '17', '-preset', 'slow',
         '-pix_fmt', 'yuv420p', '-r', str(FPS), '-threads', '4', '-c:a', 'aac', '-b:a', '192k',
         '-movflags', '+faststart', str(out / 'samples.mp4')])
    # Keep the same frame rate and exact 2x dimensions even in the small inline preview.
    for crf in [34, 38, 42]:
        run(['ffmpeg', '-v', 'error', '-y', '-i', str(out / 'samples.mp4'), '-c:v', 'libvpx-vp9',
             '-b:v', '0', '-crf', str(crf), '-row-mt', '1', '-threads', '4', '-c:a', 'libopus',
             '-b:a', '40k', str(out / 'samples.webm')])
        if (out / 'samples.webm').stat().st_size < 335000:
            break
    video = base64.b64encode((out / 'samples.webm').read_bytes()).decode()
    poster = base64.b64encode((source / f'frames/f{clips[0][1]:05}.png').read_bytes()).decode()
    buttons = ''.join(f'<button class="seek" data-time="{i * FRAMES / FPS}">{name}</button>'
                      for i, (name, _) in enumerate(clips))
    doc = '''<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Glintblade motion samples</title><style>
body{margin:0;color:var(--foreground);font:14px var(--font-sans,system-ui)}.screen{overflow:auto;background:var(--code-background);padding:16px;border-radius:var(--radius,8px)}video{display:block;width:384px;height:216px;margin:auto;image-rendering:pixelated;max-width:none}.row{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-top:12px}button{padding:7px 10px;font:inherit;border-radius:var(--radius,8px);border:1px solid var(--border);color:var(--foreground);background:var(--secondary);cursor:pointer}button:focus-visible,input:focus-visible{outline:2px solid var(--ring);outline-offset:2px}input{min-width:100px;flex:1;accent-color:var(--primary)}p{color:var(--muted-foreground);line-height:1.5;margin:10px 0 0}.time{font:12px var(--font-mono,monospace)}
</style></head><body><div class="screen"><video id="film" playsinline muted preload="metadata" poster="data:image/png;base64,POSTER" src="data:video/webm;base64,VIDEO"></video></div><div class="row"><button id="play">Play</button><button id="sound">Sound off</button><input id="scrub" aria-label="Playback position" type="range" min="0" max="8" step="0.0166667" value="0"><span id="time" class="time">0:00 / 0:08</span><button id="zoom">2× pixels</button></div><div class="row">BUTTONS</div><p id="place">Bloomwild · actual gameplay · 60fps</p><p>Four short cuts at original speed. Controls stay below the picture. This automated QA route uses invulnerability; the complete walkthrough includes all shards and the ending.</p><script>
const v=document.getElementById('film'),play=document.getElementById('play'),sound=document.getElementById('sound'),scrub=document.getElementById('scrub'),names=['Bloomwild','Glasswater','Cinder','Summit'];let large=false;
play.onclick=()=>v.paused?v.play():v.pause();v.onplay=()=>play.textContent='Pause';v.onpause=()=>play.textContent='Play';sound.onclick=()=>{v.muted=!v.muted;sound.textContent=v.muted?'Sound off':'Sound on'};scrub.oninput=()=>v.currentTime=Number(scrub.value);v.ontimeupdate=()=>{scrub.value=v.currentTime;document.getElementById('time').textContent='0:'+String(Math.floor(v.currentTime)).padStart(2,'0')+' / 0:08';document.getElementById('place').textContent=names[Math.min(3,Math.floor(v.currentTime/2))]+' · actual gameplay · 60fps'};document.getElementById('zoom').onclick=e=>{large=!large;v.style.width=(large?768:384)+'px';v.style.height=(large?432:216)+'px';e.target.textContent=large?'1× pixels':'2× pixels'};for(const b of document.querySelectorAll('.seek'))b.onclick=()=>{v.currentTime=Number(b.dataset.time);v.play()};
</script></body></html>'''.replace('POSTER', poster).replace('VIDEO', video).replace('BUTTONS', buttons)
    assert len(doc) <= 512000, f'Inline preview too large: {len(doc)} characters'
    (out / 'motion.html').write_text(doc)
    (out / 'manifest.json').write_text(json.dumps({
        'source_recording': str(source), 'source_binary_sha256': manifest['binary_sha256'],
        'source_video_sha256': manifest['video_sha256'], 'fps': FPS, 'retimed': False,
        'god': manifest['god'], 'frames_per_clip': FRAMES,
        'clips': selections,
        'compact_preview_crf': crf,
        'preview_sha256': hashlib.sha256((out / 'samples.webm').read_bytes()).hexdigest(),
        'html_characters': len(doc)}, indent=2) + '\n')
    print(out / 'motion.html')
    print(f'Preview: {len(doc)} HTML characters, {len(clips) * FRAMES} original frames at 60fps')


if __name__ == '__main__':
    main()
