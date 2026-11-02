"""Build a native 60 Hz, authored-duration atlas inspector without a server."""
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
ART=ROOT/'art/pixellab/2026-11-01-round6/enemies/fire-spirit'

def main():
    families=['ember_hound','wisp','spitting_totem']
    data={n:json.loads((ART/f'source-{n}.json').read_text()) for n in families}
    html=['<!doctype html><meta charset="utf-8"><style>body{margin:0;color:var(--text-primary,#d8d8d0);font:14px system-ui}p{margin:8px 0}.row{display:grid;grid-template-columns:repeat(3,192px);gap:8px;margin:8px 0 16px}canvas{width:192px;height:168px;image-rendering:pixelated;background:#282a32}label{display:block;font-size:12px;margin-top:4px}select,button{font:inherit;color:inherit;background:var(--bg-secondary,#343640);border:1px solid var(--border-color,#666);border-radius:4px;padding:4px}img{display:none}</style><p>Native enemy variants · original / alternate 1 / alternate 2</p><button id="pause">Pause</button> <button id="step">Step one tick</button> <label style="display:inline"><input id="flip" type="checkbox">Mirror</label>']
    for family in families:
        options=''.join(f'<option>{a}</option>' for a in data[family]['anims'])
        html.append(f'<p>{family.replace("_"," ")} <select id="{family}-anim">{options}</select> <span id="{family}-frame"></span></p><div class="row">')
        for i in range(3):
            name=family if i==0 else f'{family}_v{i}'
            path=ART/f'source-{family}.png' if i==0 else ROOT/f'assets/sprites/{name}.png'
            html.append(f'<div><canvas width="64" height="56" id="{name}"></canvas><label>{["Original","Alternate 1","Alternate 2"][i]}</label><img id="{name}-source" src="{path}"></div>')
        html.append('</div>')
    html.append('<script>const data='+json.dumps(data)+';const names='+json.dumps(families)+';')
    html.append('''let tick=0,running=true,last=performance.now();const start={};
document.getElementById('pause').onclick=()=>{running=!running;document.getElementById('pause').textContent=running?'Pause':'Play'};
document.getElementById('step').onclick=()=>{running=false;document.getElementById('pause').textContent='Play';tick=Math.floor(tick)+1;render()};
for(const name of names){start[name]=0;document.getElementById(name+'-anim').onchange=()=>{start[name]=tick;render()}}
function render(){for(const name of names){const d=data[name],anim=document.getElementById(name+'-anim').value,a=d.anims[anim],ds=a.durations||Array(a.frames).fill(60/(a.fps||1));let t=(tick-start[name])%ds.reduce((x,y)=>x+y,0),f=0;while(f<ds.length-1&&t>=ds[f]){t-=ds[f];f++}document.getElementById(name+'-frame').textContent='frame '+(f+1)+'/'+a.frames+' · '+ds[f]+' ticks';
for(let v=0;v<3;v++){const n=name+(v?'_v'+v:''),im=document.getElementById(n+'-source'),c=document.getElementById(n),ctx=c.getContext('2d');ctx.clearRect(0,0,64,56);ctx.imageSmoothingEnabled=false;ctx.fillStyle='#1e2028';ctx.fillRect(26,44,12,2);if(!im.complete||!im.naturalWidth)continue;ctx.save();if(document.getElementById('flip').checked){ctx.translate(64,0);ctx.scale(-1,1)}ctx.drawImage(im,f*d.cell[0],a.row*d.cell[1],d.cell[0],d.cell[1],32-d.anchor[0],43-d.anchor[1]-(name==='wisp'?7:0),d.cell[0],d.cell[1]);ctx.restore()}}}
function frame(now){if(running)tick+=(now-last)*60/1000;last=now;render();requestAnimationFrame(frame)}requestAnimationFrame(frame);
</script>''')
    path=ART/'animation-inspector.html';path.write_text(''.join(html));print(path)

if __name__=='__main__':main()
