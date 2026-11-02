#!/usr/bin/env python3
"""Pack four unscaled native PixelLab floor studies; all pixels retain provenance."""
from pathlib import Path
from PIL import Image
import argparse, hashlib, json
ROOT=Path(__file__).resolve().parents[2]
p=argparse.ArgumentParser();p.add_argument('--out',type=Path,default=ROOT/'assets/sprites');a=p.parse_args()
sources=ROOT/'art/pixellab/2026-11-01-round6/bloom-floor'
canvas=Image.new('RGBA',(128,256));names=['fluted_chips','broken_mosaic','damp_apron','ivy']
for i,name in enumerate(names):
 image=Image.open(sources/f'raw-{i}.png').convert('RGBA')
 assert image.size==(128,64) and set(image.getchannel('A').get_flattened_data())=={0,255}
 canvas.paste(image,(0,64*i))
a.out.mkdir(parents=True,exist_ok=True);canvas.save(a.out/'bloomwild_floor_details.png')
data={'cell':[128,64],'anchor':[64,32],'emissive':[], 'anims':{name:{'row':i,'frames':1,'fps':1} for i,name in enumerate(names)},'notes':'Four native unscaled PixelLab ground studies. Flat walkable low-relief fragments; clipped to existing dry receiving ground. No collider, light, transform, resampling or palette alteration. Job 9bc76122-d753-493e-9d69-8f949ef4db89; exact source pixels and prompt archived in art/pixellab/2026-11-01-round6/bloom-floor.'}
(a.out/'bloomwild_floor_details.json').write_text(json.dumps(data,indent=2)+'\n')
print(json.dumps({p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in a.out.glob('bloomwild_floor_details.*')},indent=2))
