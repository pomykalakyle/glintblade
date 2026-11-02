"""Pack native PixelLab full-animation fire/spirit skins and prove their invariants.

uv run --offline --with pillow python tools/pixellab/round6_fire_variants.py
The raw generations stay untouched. Integer pose registration, native cutout
assembly, removal of invented sparks, and authored combat cue restoration occur.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / 'art/pixellab/2026-11-01-round6/enemies/fire-spirit'
EMPTY = (0, 0, 0, 0)
DESIGNS = {
    'ember_hound_v1': 'Ivory rib armor, a broad ceramic shoulder, bone brow and dark flexible joints.',
    'ember_hound_v2': 'Glassback: a large faceted jasper shoulder shell, low slate muzzle and ridged tail.',
    'wisp_v1': 'Moon moth: paired broad angular wings that fold tightly around its charging heart.',
    'wisp_v2': 'Hanging lantern: faceted spectral cage, crescent side horns and two tapering streamers.',
    'spitting_totem_v1': 'Twin-flue copper furnace with broad shoulders and layered beveled plinth.',
    'spitting_totem_v2': 'Ash ceramic kiln with broad arched ribs, dark backplate and inset fixed firing outlet.',
}

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def points(im):
    return {(x,y) for y in range(im.height) for x in range(im.width) if im.getpixel((x,y))[3]}

def largest_component(im):
    """Reject generator-invented loose sparks; original authored release sparks return below."""
    left = points(im)
    groups=[]
    while left:
        p=left.pop(); todo=[p]; group={p}
        while todo:
            x,y=todo.pop()
            for dx,dy in ((-1,-1),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)):
                q=(x+dx,y+dy)
                if q in left: left.remove(q); group.add(q); todo.append(q)
        groups.append(group)
    if not groups: raise AssertionError('empty generated pose')
    keep=max(groups,key=len)
    out=Image.new('RGBA',im.size)
    for p in keep: out.putpixel(p,im.getpixel(p))
    return out,len(points(im))-len(keep)

def translated(im,dx,dy):
    out=Image.new('RGBA',im.size);out.paste(im,(dx,dy));return out

def original_emission(source,colors):
    return {p:source.getpixel(p) for p in points(source) if source.getpixel(p)[:3] in colors}

def nearest_nonemissive(pixel,palette):
    return min(palette,key=lambda p:sum((a-b)**2 for a,b in zip(p[:3],pixel[:3])))

def rigid_glass_plate(raw):
    """One native generated jasper plate, carried without changing its pixel dimensions."""
    first=translated(raw.crop((0,0,40,24)),0,-1)
    warm={p for p in points(first) if p[0]>=19 and (lambda c:c[0]>c[1]+20 and c[0]>c[2]+20 and c[0]<210)(first.getpixel(p))}
    rim={(x+dx,y+dy) for x,y in warm for dx in [-1,0,1] for dy in [-1,0,1]}
    plate=Image.new('RGBA',first.size)
    for p in rim & points(first):plate.putpixel(p,first.getpixel(p))
    return plate

def rigid_ivory_armor(raw):
    """Carry the generated rib armor through running and airborne side poses.

    PixelLab drew the defining shoulder only in planted poses. This is one
    unchanged native cutout of that shoulder, including its outline; feet,
    muzzle and moving legs continue to use each authored/generated pose.
    """
    first=translated(raw.crop((0,0,40,24)),0,-1)
    ivory={p for p in points(first) if 15<=p[0]<=25 and 5<=p[1]<=15
           and (lambda c:c[0]>130 and c[1]>100 and c[2]>80)(first.getpixel(p))}
    rim={(x+dx,y+dy) for x,y in ivory for dx in [-1,0,1] for dy in [-1,0,1]}
    plate=Image.new('RGBA',first.size)
    for p in rim & points(first):
        if 14<=p[0]<=26 and 4<=p[1]<=16:plate.putpixel(p,first.getpixel(p))
    return plate

def moth_fold(cell,raw,meta,frame):
    """Two native wing positions bridge open idle to the tightly folded charge."""
    # The generated upper tips touched the cell's top border. Lower this upper
    # wing/antenna piece one native pixel, leaving the firing heart in place.
    upper=cell.crop((3,0,13,9))
    for y in range(9):
        for x in range(3,13):cell.putpixel((x,y),EMPTY)
    cell.alpha_composite(upper,(3,1))
    if frame<2:
        idle=meta['anims']['idle'];i=idle['frames']-1;cw,ch=meta['cell']
        open_wings=raw.crop((i*cw,idle['row']*ch,(i+1)*cw,(idle['row']+1)*ch))
        # Only the outer native wing lobes move. Their inward/upward movement
        # reads as folding around the unchanged luminous heart.
        for x,y in points(open_wings):
            if 6<=y<=12 and (x<=4 or x>=10):
                xx=x+(frame if x<=4 else -frame)
                cell.putpixel((xx,y-frame),open_wings.getpixel((x,y)))
    return cell

def pack(name,out):
    species=name.rsplit('_v',1)[0]
    original_path=ART/f'source-{species}.png'
    original=Image.open(original_path).convert('RGBA')
    meta=json.loads((ART/f'source-{species}.json').read_text())
    raw_path=ART/f'{name}-raw.png'
    raw=Image.open(raw_path).convert('RGBA')
    assert raw.size==original.size,(name,raw.size,original.size)
    assert set(raw.getchannel('A').get_flattened_data())<={0,255},name
    cw,ch=meta['cell'];atlas=Image.new('RGBA',original.size)
    emit={tuple(bytes.fromhex(c.lstrip('#'))) for c in meta.get('emissive',[])}
    palette={p for p in raw.get_flattened_data() if p[3] and p[:3] not in emit}
    plate=rigid_glass_plate(raw) if name=='ember_hound_v2' else None
    ivory=rigid_ivory_armor(raw) if name=='ember_hound_v1' else None
    canonical=None
    if species=='spitting_totem':
        canonical,_=largest_component(raw.crop((0,0,cw,ch)))
        canonical=points(translated(canonical,0,2 if name.endswith('v1') else 1))
    frames=[]
    for anim,a in meta['anims'].items():
        row=a['row']
        for i in range(a['frames']):
            box=(i*cw,row*ch,(i+1)*cw,(row+1)*ch)
            source=original.crop(box);cell=raw.crop(box)
            removed=0
            if species=='spitting_totem':
                cell,removed=largest_component(cell)
                dy=2 if name.endswith('v1') else 1
            elif species=='ember_hound':
                # PixelLab retained every source pose but registered its ink one row lower.
                dy=source.getbbox()[3]-cell.getbbox()[3]
                assert abs(dy)<=2,(name,anim,i,dy)
            else: dy=0
            cell=translated(cell,0,dy)
            upper_inset=0
            if species=='ember_hound' and any(cell.getpixel((x,0))[3] for x in range(cw)):
                # Two generated vertical leap poses put their upper armor tip
                # on the cell edge after foot registration. Keep the feet and
                # muzzle fixed; inset only the native upper cap by one row.
                upper=cell.crop((0,0,cw,6))
                for y in range(6):
                    for x in range(cw):cell.putpixel((x,y),EMPTY)
                cell.alpha_composite(upper,(0,1));upper_inset=1
            if canonical:
                # A furnace is a solid, fixed chassis. Generated heat sparks outside
                # that chassis are removed; only authored launch sparks return below.
                for p in points(cell)-canonical:cell.putpixel(p,EMPTY);removed+=1
            plate_registration=None
            active_plate=None
            if plate is not None and anim.endswith('_right'):
                old_plate={p for p in points(cell) if p[0]>=12 and (lambda c:c[0]>c[1]+20 and c[0]>c[2]+20 and c[0]<210)(cell.getpixel(p))}
                old_rim={(x+dx,y+dy) for x,y in old_plate for dx in [-1,0,1] for dy in [-1,0,1]}
                for p in old_rim:
                    if 0<=p[0]<cw and 0<=p[1]<ch:cell.putpixel(p,source.getpixel(p))
                cue=original_emission(source,emit)
                eye=min((p for p in cue if p[0]>20),key=lambda p:(p[1],-p[0]))
                # The airborne lunge raises the shoulder farther than the
                # planted pose; keep its complete native cap inside the cell.
                plate_registration=(eye[0]-31,max(eye[1]-11,1-plate.getbbox()[1]))
                cell.alpha_composite(plate,plate_registration)
                active_plate=plate
            if ivory is not None and anim in ('move_right','attack_right'):
                cue=original_emission(source,emit)
                eye=min((p for p in cue if p[0]>20),key=lambda p:(p[1],-p[0]))
                plate_registration=(eye[0]-30,eye[1]-11)
                cell.alpha_composite(ivory,plate_registration)
                active_plate=ivory
            if species in ('ember_hound','spitting_totem'):
                # The original exact emissive mask is the gameplay/lighting contract.
                for p in points(cell):
                    color=cell.getpixel(p)
                    if color[:3] in emit: cell.putpixel(p,nearest_nonemissive(color,palette))
                if species=='spitting_totem':
                    # Native original outlet/backplate joins both furnace walls and fixes
                    # the glow centroid, flare shape and frame-by-frame heat progression.
                    patch=(8,18,19,24)
                    cell.paste(source.crop(patch),(patch[0],patch[1]))
                for p,color in original_emission(source,emit).items():cell.putpixel(p,color)
                assert original_emission(cell,emit)==original_emission(source,emit),(name,anim,i)
                if active_plate is not None:
                    cues=original_emission(source,emit)
                    for p in points(active_plate):
                        q=(p[0]+plate_registration[0],p[1]+plate_registration[1])
                        assert q in cues or cell.getpixel(q)==active_plate.getpixel(p),(name,anim,i,'rigid plate')
            else:
                if name=='wisp_v1' and anim=='windup':cell=moth_fold(cell,raw,meta,i)
                # Preserve the native luminous heart and tiny dark face slit. The outer
                # glow and silhouette can change so this is a new spirit anatomy.
                core=[]
                for y in range(9,16):
                    for x in range(4,11):
                        color=source.getpixel((x,y))
                        if color[:3] in {(205,195,219),(226,216,233),(34,45,61)} and color[3]:
                            cell.putpixel((x,y),color);core.append((x,y))
                assert core
                # A one-pixel native gutter avoids an atlas-edge wing reading as cropped.
                for y in range(ch):
                    cell.putpixel((0,y),EMPTY);cell.putpixel((cw-1,y),EMPTY)
                if name=='wisp_v1' and anim=='windup':
                    assert not any(cell.getpixel((x,0))[3] for x in range(cw)),('moth upper gutter',i)
            assert cell.getbbox(),(name,anim,i)
            if species in ('ember_hound','spitting_totem'):
                assert cell.getbbox()[3]==source.getbbox()[3],(name,anim,i,'feet')
            assert not any(cell.getpixel((x,ch-1))[3] for x in range(cw)),(name,anim,i,'bottom gutter')
            if species=='ember_hound':
                assert not any(cell.getpixel((x,0))[3] for x in range(cw)),(name,anim,i,'top gutter')
            atlas.paste(cell,(box[0],box[1]))
            old=points(source);new=points(cell)
            changed=sum(source.getpixel((x,y))!=cell.getpixel((x,y)) for y in range(ch) for x in range(cw))
            frames.append({'animation':anim,'frame':i,'registration':[0,dy],
                'rigid_plate_registration':plate_registration,
                'upper_cap_inset_rows':upper_inset,
                'moth_fold_stage':i if name=='wisp_v1' and anim=='windup' and i<2 else None,
                'removed_invented_spark_pixels':removed,'source_opaque':len(old),'variant_opaque':len(new),
                'silhouette_changed':len(old^new),'changed_pixels':changed,'bbox':cell.getbbox(),
                'emission_pixels':len(original_emission(cell,emit)) if species!='wisp' else None})
    assert len(frames)==sum(a['frames'] for a in meta['anims'].values())
    out.mkdir(parents=True,exist_ok=True)
    atlas.save(out/f'{name}.png')
    result=copy.deepcopy(meta)
    result['notes']=f'Round 6 full-animation native PixelLab alternate skin. {DESIGNS[name]} '
    result['notes']+='All original animation rows/counts/durations, cell/anchor, behavior and exact authored combat cue positions retained. See art/pixellab/2026-11-01-round6/enemies/fire-spirit for raw generation, prompts, receipts and native assembly recipe.'
    result['variant_source']=species
    result['variant_design']=DESIGNS[name]
    (out/f'{name}.json').write_text(json.dumps(result,indent=2)+'\n')
    assert {k:result[k] for k in ['cell','anchor','anims','emissive']}=={k:meta[k] for k in ['cell','anchor','anims','emissive']}
    return {'name':name,'raw_sha256':sha(raw_path),'source_sha256':sha(original_path),
        'source_json_sha256':sha(ART/f'source-{species}.json'),
        'png_sha256':sha(out/f'{name}.png'),'json_sha256':sha(out/f'{name}.json'),
        'frames':frames,'total_frames':len(frames),
        'total_silhouette_changes':sum(f['silhouette_changed'] for f in frames),
        'total_changed_pixels':sum(f['changed_pixels'] for f in frames)}

def contacts(out,reports):
    for species in ['ember_hound','wisp','spitting_totem']:
        meta=json.loads((out/f'{species}_v1.json').read_text());cw,ch=meta['cell']
        names=[species,species+'_v1',species+'_v2']
        anims=list(meta['anims'])
        sheet=Image.new('RGB',(3*6*cw+100,len(anims)*(ch+10)+24),(39,41,49));draw=ImageDraw.Draw(sheet)
        for j,name in enumerate(names):
            image=Image.open(ART/f'source-{species}.png' if j==0 else out/f'{name}.png').convert('RGBA')
            draw.text((100+j*6*cw,3),name,fill=(222,218,209))
            for k,anim in enumerate(anims):
                a=meta['anims'][anim];y=24+k*(ch+10)
                if j==0:draw.text((2,y),anim,fill=(190,188,185))
                for i in range(a['frames']):
                    box=(i*cw,a['row']*ch,(i+1)*cw,(a['row']+1)*ch)
                    cell=image.crop(box);sheet.paste(cell,(100+j*6*cw+i*cw,y),cell)
        sheet.save(ART/f'{species}-all-poses-1x.png')
        sheet.resize((sheet.width*3,sheet.height*3),Image.Resampling.NEAREST).save(ART/f'{species}-all-poses-3x.png')
    # A compact native comparison is useful for scene-independent first-pass review.
    sheet=Image.new('RGB',(3*110,3*60),(39,41,49));draw=ImageDraw.Draw(sheet)
    for r,species in enumerate(['ember_hound','wisp','spitting_totem']):
        for col,name in enumerate([species,species+'_v1',species+'_v2']):
            meta=json.loads((ART/f'source-{species}.json').read_text());w,h=meta['cell']
            im=Image.open(ART/f'source-{species}.png' if col==0 else out/f'{name}.png').convert('RGBA').crop((0,0,w,h))
            sheet.paste(im,(col*110+(110-w)//2,r*60+10),im);draw.text((col*110+4,r*60+45),name.replace('spitting_totem','totem').replace('ember_hound','hound'),fill=(210,210,200))
    sheet.save(ART/'first-looks-1x.png')
    sheet.resize((sheet.width*3,sheet.height*3),Image.Resampling.NEAREST).save(ART/'first-looks-3x.png')

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,default=ROOT/'assets/sprites');a=p.parse_args()
    reports=[pack(n,a.out) for n in DESIGNS]
    contacts(a.out,reports)
    (ART/'coverage.json').write_text(json.dumps({'recipe':'tools/pixellab/round6_fire_variants.py','skins':reports,'total_frames':sum(r['total_frames'] for r in reports),'resampling':'none; native pixels plus integer pose registration only'},indent=2)+'\n')
    print(json.dumps({r['name']:{k:r[k] for k in ['total_frames','total_silhouette_changes','total_changed_pixels']} for r in reports},indent=2))

if __name__=='__main__':main()
