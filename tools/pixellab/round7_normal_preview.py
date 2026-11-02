#!/usr/bin/env python3
"""Build a small browser-only normal asset diagnostic; no server required."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "out/pixellab/round7/normal-trials/normal-assets.html"
specs = [
    ("bloomwild_ruins", "Limestone", [48,56], [0,0], None),
    ("glasswater_crystals_big", "Crystal facets", [48,64], [0,0], None),
    ("bloomwild_trees", "Blossom tree", [80,96], [0,0], [0,96]),
    ("hero", "Hero", [48,48], [0,0], None),
    ("ember_hound_v1", "Bone hound", [40,24], [0,0], None),
    ("spitting_totem_v2", "Kiln", [28,36], [0,0], None),
]
images = []
data = []
cards = []
for i,(name,label,cell,origin,overlay) in enumerate(specs):
    meta = json.loads((ROOT / "assets/sprites" / (name+".json")).read_text())
    images.extend([f'<img hidden id="a{i}" src="{ROOT}/assets/sprites/{name}.png">',
                   f'<img hidden id="n{i}" src="{ROOT}/assets/normals/{name}.png">'])
    data.append({"cell":cell,"origin":origin,"overlay":overlay,
                 "emit":[int(c.lstrip('#'),16) for c in meta.get("emissive",[])]})
    cards.append(f'<div><div class="label">{label}</div><canvas id="c{i}" width="80" height="100"></canvas></div>')

html = '''<!doctype html><html><head><meta charset="utf-8"><style>
*{box-sizing:border-box}body{margin:0;color:var(--text-primary,#d9dde4);font:14px system-ui;background:transparent}p{margin:6px 0 14px;color:var(--text-secondary,#a1a9b8)}
.controls{display:flex;align-items:center;gap:10px;flex-wrap:wrap;margin:12px 0}.controls input{width:200px}.grid{display:grid;grid-template-columns:repeat(3,240px);max-width:720px;gap:14px 0}.label{height:22px;color:var(--text-secondary,#abb6c8)}canvas{display:block;width:240px;height:300px;image-rendering:pixelated;background:#1d1f27}button{color:inherit;background:var(--background-secondary,#303442);border:1px solid var(--border-primary,#4a5060);border-radius:5px;padding:5px 9px;cursor:pointer}@media(max-width:600px){.grid{grid-template-columns:repeat(2,160px)}canvas{width:160px;height:200px}}
</style></head><body><div>Same pixels. Changing surface light.</div><p>Normal-map asset trial. The sprites stay still; each pixel responds to the light direction.</p><div class="controls"><button id="play">Pause light</button><label>Direction <input id="angle" type="range" min="-65" max="65" value="-35"></label><button id="map">Show normals</button></div><div class="grid">CARDS</div>IMAGES<script>
const specs=DATA, assets=[],slider=document.getElementById('angle');let playing=true,showNormals=false;
document.getElementById('play').onclick=function(){playing=!playing;this.textContent=playing?'Pause light':'Animate light'};slider.oninput=()=>{playing=false;document.getElementById('play').textContent='Animate light'};
document.getElementById('map').onclick=function(){showNormals=!showNormals;this.textContent=showNormals?'Show shaded art':'Show normals'};
function extract(img,spec){const c=document.createElement('canvas');c.width=spec.cell[0];c.height=spec.cell[1];const x=c.getContext('2d',{willReadFrequently:true});x.drawImage(img,-spec.origin[0],-spec.origin[1]);if(spec.overlay)x.drawImage(img,-spec.overlay[0],-spec.overlay[1]);return x.getImageData(0,0,c.width,c.height)}
Promise.all([...document.querySelectorAll('img')].map(im=>im.decode())).then(()=>{specs.forEach((s,i)=>{assets.push({spec:s,art:extract(document.getElementById('a'+i),s),normal:extract(document.getElementById('n'+i),s),context:document.getElementById('c'+i).getContext('2d')})});requestAnimationFrame(draw)});
function draw(ms){const deg=playing?Math.sin(ms/1700)*65:Number(slider.value);if(playing)slider.value=deg;const angle=deg*Math.PI/180;let lx=Math.sin(angle)*.82,ly=.40,lz=Math.cos(angle)*.82;const ll=Math.hypot(lx,ly,lz);lx/=ll;ly/=ll;lz/=ll;
for(const a of assets){const src=a.art.data,n=a.normal.data,out=new ImageData(a.art.width,a.art.height);for(let j=0;j<src.length;j+=4){if(src[j+3]===0)continue;let nx=n[j]/127.5-1,ny=n[j+1]/127.5-1,nz=n[j+2]/127.5-1;const norm=Math.hypot(nx,ny,nz)||1;const dot=Math.max(0,(nx*lx+ny*ly+nz*lz)/norm);const key=(src[j]<<16)|(src[j+1]<<8)|src[j+2],gain=a.spec.emit.includes(key)?1:.48+.74*dot;for(let k=0;k<3;k++)out.data[j+k]=showNormals?n[j+k]:Math.min(255,Math.round(src[j+k]*gain));out.data[j+3]=src[j+3]}a.context.clearRect(0,0,80,100);a.context.putImageData(out,Math.floor((80-a.art.width)/2),100-a.art.height-3)}requestAnimationFrame(draw)}
</script></body></html>'''
html=html.replace('CARDS',''.join(cards)).replace('IMAGES',''.join(images)).replace('DATA',json.dumps(data))
OUT.parent.mkdir(parents=True,exist_ok=True)
OUT.write_text(html)
print(OUT)
