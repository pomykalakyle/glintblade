"""Independent, native-resolution inventory for the final round-six enemy audit.

This reads only shipped PNG/JSON files. It neither imports the art packers nor
modifies an atlas. Contact-sheet review findings are recorded separately.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from PIL import Image, ImageDraw

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'out/pixellab/round6/enemy-independent'
ART=ROOT/'art/pixellab/2026-11-01-round6/enemies'
FAMILIES=('sproutling','thorn_imp','crystal_beetle','wisp','ember_hound','spitting_totem')

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def mask(im, colors):
    return {(x,y):im.getpixel((x,y)) for y in range(im.height) for x in range(im.width)
            if im.getpixel((x,y))[3] and im.getpixel((x,y))[:3] in colors}

def run():
    OUT.mkdir(parents=True,exist_ok=True)
    report={'scope':'Independent shipped-file scan of all six base atlases and twelve alternate atlases. '
                    'Direct visual review covered every authored pose in native/integer contact sheets. '
                    'Integrated captures, runtime timing and gameplay require their own evidence.',
            'base_frames':0,'alternate_frames':0,'files':[],'frames':[],'issues':[],
            'visual_findings':[
                {'issue':'Ivory shoulder armor vanished during side running and attack',
                 'status':'repaired in ten dynamic poses by one unchanged native armor cutout'},
                {'issue':'Moth spread-to-folded charge snapped and folded upper tips touched the atlas top',
                 'status':'repaired by two native intermediate wing positions and a one-row upper inset'},
                {'issue':'Three airborne hound poses introduced opaque pixels on the upper cell border',
                 'status':'repaired by native upper-cap inset and gutter-constrained rigid shoulder placement'},
                {'issue':'Skin-dependent death pixel counts advanced the shared gameplay RNG differently',
                 'status':'root isolated reference death RNG; implementation reviewed; root owns executable verification'},
            ]}
    report['aesthetic_review']={
        'sproutling':'The low oatcap and upright petal/sepals form different heads and bodies, retained across hopping, stretch, impact and recovery poses.',
        'thorn_imp':'Birch forked mask/rose lichen and broad bracken/raffia profiles read as distinct materials; the same held object remains legible through each throw.',
        'crystal_beetle':'Broken quartz bowl and ribbed fossil valves replace the tall spike body with two coherent shells. Both retain a violet cleft and charge-to-dazed brightness progression.',
        'ember_hound':'Ivory rib armor and a rigid jasper shoulder distinguish both skins through all poses; the repaired side movement no longer loses the ivory shoulder.',
        'wisp':'Moon-moth wings and a hanging lantern cage differ in anatomy, retaining the shared small bright firing heart. Intermediate native wing positions clarify the fold.',
        'spitting_totem':'Twin copper flues and an ivory ceramic arch have clear silhouettes and grounded plinths; the inset fixed firing outlet stays visibly joined to each chassis.',
    }
    for family in FAMILIES:
        base_path=ROOT/f'assets/sprites/{family}.png'
        base=Image.open(base_path).convert('RGBA')
        data=json.loads((ROOT/f'assets/sprites/{family}.json').read_text())
        w,h=data['cell'];emit={tuple(bytes.fromhex(c[1:])) for c in data.get('emissive',[])}
        for variant in range(3):
            name=family+(f'_v{variant}' if variant else '')
            png=ROOT/f'assets/sprites/{name}.png';meta=ROOT/f'assets/sprites/{name}.json'
            image=Image.open(png).convert('RGBA');d=json.loads(meta.read_text())
            assert image.size==base.size,(name,'atlas dimensions')
            assert {k:d[k] for k in ('cell','anchor','anims')}=={k:data[k] for k in ('cell','anchor','anims')},(name,'pose contract')
            assert set(image.getchannel('A').get_flattened_data())<={0,255},(name,'nonbinary alpha')
            report['files'] += [{'path':str(p.relative_to(ROOT)),'sha256':sha(p)} for p in (png,meta)]
            for anim,a in data['anims'].items():
                for frame in range(a['frames']):
                    box=(frame*w,a['row']*h,(frame+1)*w,(a['row']+1)*h)
                    source=base.crop(box);cell=image.crop(box);bbox=cell.getbbox()
                    assert bbox,(name,anim,frame,'empty')
                    report['alternate_frames' if variant else 'base_frames']+=1
                    if variant and family!='wisp':
                        assert bbox[3]==source.getbbox()[3],(name,anim,frame,'foot baseline')
                    if variant and family not in ('wisp','crystal_beetle'):
                        assert mask(cell,emit)==mask(source,emit),(name,anim,frame,'exact emissive cue')
                    if variant and family=='thorn_imp':
                        seed={(163,163,143),(189,187,161)}
                        assert mask(cell,seed)==mask(source,seed),(name,anim,frame,'held seed centroid')
                    edges={(x,y) for y in range(h) for x in range(w) if x in (0,w-1) or y in (0,h-1)}
                    new_edges=[(x,y) for x,y in edges if cell.getpixel((x,y))[3] and not source.getpixel((x,y))[3]]
                    if variant and new_edges:report['issues'].append({'name':name,'anim':anim,'frame':frame,'introduced_edge_pixels':new_edges})
                    pixels=[(x,y) for y in range(h) for x in range(w) if cell.getpixel((x,y))[3]]
                    report['frames'].append({'name':name,'anim':anim,'frame':frame,'row':a['row'],
                        'duration':a.get('durations',[round(60/a.get('fps',1))]*a['frames'])[frame],
                        'rgba_sha256':hashlib.sha256(cell.tobytes()).hexdigest(),'bbox':bbox,'opaque_pixels':len(pixels),
                        'new_border_pixels':new_edges})
    assert report['base_frames']==305,report['base_frames']
    assert report['alternate_frames']==610,report['alternate_frames']
    assert not report['issues'],report['issues']
    report['independent_script_sha256']=sha(Path(__file__))
    path=ART/'independent-review.json';path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'base_frames':report['base_frames'],'alternate_frames':report['alternate_frames'],
        'files':len(report['files']),'introduced_border_issues':len(report['issues']),'report':str(path.relative_to(ROOT))}))

if __name__=='__main__':run()
