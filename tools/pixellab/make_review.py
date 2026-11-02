#!/usr/bin/env python3
"""Build a portable before/after viewer from two capture_world.py runs."""
import argparse
import base64
import html
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[2]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--before', default='before-round3')
    p.add_argument('--after', default='final-v3')
    p.add_argument('--out', type=Path, default=ROOT / 'out/pixellab/review-round3')
    args = p.parse_args()
    manifests = {phase: json.loads((ROOT / 'out/pixellab/world' / tag / 'manifest.json').read_text())
                 for phase, tag in [('before', args.before), ('after', args.after)]}
    zones = {'title': ['menu']}
    for c in manifests['after']['captures']:
        zones.setdefault(c['zone'], []).append(c['name'])
    args.out.mkdir(parents=True, exist_ok=True)
    descriptions = {
        'title': 'One coherent island view anchors Glint. The native wordmark and menus occupy quiet sky.',
        'bloomwild': 'Grass caps and soil contacts join native cliffs. Trees and ruins block at visible bases; bridge decks connect to banks.',
        'glasswater': 'Distinct shallows and deep water, a bonded slate causeway, exact object supports and live actor reflections.',
        'cinder': 'Molten currents, clustered crust and land-seated bridge abutments. All three crossings and object bases are checked with real inputs.',
        'summit': 'Curved top pixels own collision; masonry faces and hanging rock stay below walkable platforms.',
    }
    for portable, filename in [(False, 'gallery-host.html'), (True, 'gallery.html')]:
        def source(path):
            path = Path(path)
            return 'data:image/png;base64,' + base64.b64encode(path.read_bytes()).decode() if portable else str(path)
        images = []
        for phase, data in manifests.items():
            images.append(f'<img class="frame" data-zone="title" data-scene="menu" data-phase="{phase}" ' + ('' if phase == 'after' else 'hidden ') + f'src="{source(data["title"])}" width="384" height="216" alt="Title screen {phase}">')
            for c in data['captures']:
                visible = False
                images.append(f'<img class="frame" data-zone="{c["zone"]}" data-scene="{c["name"]}" data-phase="{phase}" '
                    f'{"" if visible else "hidden "}src="{source(c["file"])}" width="384" height="216" '
                    f'alt="{html.escape(c["zone"]+" "+c["name"]+" "+phase)}">')
            for zone in zones:
                if zone == 'title':
                    continue
                first = next(c for c in data['captures'] if c['zone'] == zone)
                path = Path(first['file']).parent / f'{zone}_overview.png'
                width, height = struct.unpack('>II', path.read_bytes()[16:24])
                images.append(f'<img class="overview" hidden data-zone="{zone}" data-phase="{phase}" '
                    f'src="{source(path)}" width="{width//2}" height="{height//2}" alt="{zone} full overview {phase}">')
        doc = '''<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Glintblade · four-region review</title><style>
body{margin:0;font:14px var(--font-sans,system-ui);color:var(--foreground)}.row{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:12px}button,select{font:inherit;color:var(--foreground);background:var(--secondary);border:1px solid var(--border);border-radius:var(--radius,8px);padding:7px 11px}button{cursor:pointer}button[aria-pressed=true]{background:var(--primary);color:var(--primary-foreground)}button:focus-visible,select:focus-visible{outline:2px solid var(--ring);outline-offset:2px}.screen{overflow:auto;max-height:560px;background:var(--code-background);padding:16px;border-radius:var(--radius,8px)}img{display:block;max-width:none;margin:auto;image-rendering:pixelated}img[hidden]{display:none}.meta{font:12px var(--font-mono,monospace);color:var(--muted-foreground);margin:0 0 10px}p{margin:10px 0 0;color:var(--muted-foreground);line-height:1.5}
</style></head><body><div class="row"><select id="region" aria-label="Region"><option selected value="title">Title</option><option value="bloomwild">Bloomwild</option><option value="glasswater">Glasswater</option><option value="cinder">Cinder</option><option value="summit">Summit</option></select><select id="scene" aria-label="Scene"></select><button id="map" aria-pressed="false">Whole map</button></div><div class="row"><button id="before" aria-pressed="false">Before this round</button><button id="after" aria-pressed="true">Current game</button><button id="zoom">2× pixels</button></div><div class="screen"><div id="label" class="meta">CURRENT GAME · TITLE · MENU · 384 × 216 NATIVE</div>IMAGES</div><p id="note">Molten currents, clustered crust and land-seated bridge abutments. All three crossings and object bases are checked with real inputs.</p><p>VIEWCOUNT native studies plus the title and four full maps. Before is the selected prior build; matching scenes use identical capture positions.</p><script>
const scenes=SCENES,notes=NOTES;let zone='title',scene='menu',phase='after',zoom=1,map=false;
function options(){const s=document.getElementById('scene');s.replaceChildren();for(const name of scenes[zone]){const o=document.createElement('option');o.value=name;o.textContent=name.replaceAll('-',' ');o.selected=name===scene;s.append(o)}}
function update(){for(const f of document.querySelectorAll('.frame')){f.hidden=map||f.dataset.zone!==zone||f.dataset.scene!==scene||f.dataset.phase!==phase;f.style.width=384*zoom+'px';f.style.height=216*zoom+'px'}for(const f of document.querySelectorAll('.overview'))f.hidden=!map||f.dataset.zone!==zone||f.dataset.phase!==phase;document.getElementById('before').setAttribute('aria-pressed',phase==='before');document.getElementById('after').setAttribute('aria-pressed',phase==='after');document.getElementById('map').setAttribute('aria-pressed',map);document.getElementById('map').disabled=zone==='title';document.getElementById('scene').disabled=map;document.getElementById('zoom').disabled=map;document.getElementById('zoom').textContent=zoom===1?'2× pixels':'1× pixels';document.getElementById('label').textContent=(phase==='before'?'BEFORE THIS ROUND':'CURRENT GAME')+' · '+zone.toUpperCase()+' · '+(map?'WHOLE MAP AT ½ SCALE':scene.toUpperCase()+' · 384 × 216 NATIVE');document.getElementById('note').textContent=notes[zone]}
document.getElementById('region').onchange=e=>{zone=e.target.value;map=false;scene=scenes[zone][0];options();update()};document.getElementById('scene').onchange=e=>{scene=e.target.value;update()};document.getElementById('before').onclick=()=>{phase='before';update()};document.getElementById('after').onclick=()=>{phase='after';update()};document.getElementById('zoom').onclick=()=>{zoom=3-zoom;update()};document.getElementById('map').onclick=()=>{map=!map;update()};options();update();
</script></body></html>'''.replace('IMAGES', ''.join(images)).replace('SCENES', json.dumps(zones)).replace('NOTES', json.dumps(descriptions)).replace('VIEWCOUNT', str(len(manifests['after']['captures'])))
        (args.out / filename).write_text(doc)
        print(args.out / filename)


if __name__ == '__main__':
    main()
