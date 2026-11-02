"""Native eight-facing Cinder part rigs; scoped append-only atlas exporter.

uv run --offline --with pillow python tools/eightway/cinder.py --out PATH

Historical rows and their normal data are immutable receipts. New views use
source-native PixelLab anatomy with articulated authored body/limb pieces and
hand-authored basalt planes, never rotated or resized sprite bitmaps. Every
edit and registration stays on the native integer grid with binary alpha.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import shutil
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "art/eightway/cinder"
DIRS = ["up", "up_right", "right", "down_right", "down", "down_left", "left", "up_left"]
VECTORS = [(0,-1),(.70710678,-.70710678),(1,0),(.70710678,.70710678),(0,1),(-.70710678,.70710678),(-1,0),(-.70710678,-.70710678)]
EMIT = {(212,134,112), (239,182,138)}
EMPTY = (0,0,0,0)


def rgba(s):
    return tuple(bytes.fromhex(s.lstrip("#"))) + (255,)


def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()








def native_normal(im):
    """Authored broad part volumes, ignoring albedo brightness/material marks."""
    nm=Image.new("RGBA",im.size)
    ink=im.getchannel("A")
    for y in range(im.height):
        runs=[];start=None
        for x in range(im.width+1):
            a=x<im.width and ink.getpixel((x,y))
            if a and start is None:start=x
            if not a and start is not None:runs.append((start,x-1));start=None
        for lo,hi in runs:
            for x in range(lo,hi+1):
                nx=(x-(lo+hi)*.5)/max((hi-lo+1)*.5,1)*.6
                ny=.38 if not y or not ink.getpixel((x,y-1)) else .10
                nz=math.sqrt(max(.01,1-nx*nx-ny*ny))
                nm.putpixel((x,y),(round((nx+1)*127.5),round((ny+1)*127.5),round((nz+1)*127.5),255))
    return nm


def native_hound_seed(direction,skin):
    """Register unscaled PixelLab anatomy, shortening only distal leg segments.

    The service returned square padding and excessively long shins. The complete
    head/shoulder plate stays byte-exact. A cut across the four narrow lower-leg
    segments removes their extra native rows; no torso/armor pixel is scaled.
    """
    label="ivory" if skin==1 else "jasper"
    facing={"up_right":"north-east","down_right":"south-east","down_left":"south-west","up_left":"north-west"}[direction]
    raw=Image.open(ART/f"pixellab/{label}-{facing}.png").convert("RGBA")
    bbox=raw.getbbox();top,bottom=bbox[1],bbox[3]
    remove=max(0,(bottom-top)-19)
    cut=bottom-5-remove
    trimmed=Image.new("RGBA",raw.size)
    trimmed.paste(raw.crop((0,0,40,cut)),(0,0))
    trimmed.paste(raw.crop((0,cut+remove,40,40)),(0,cut))
    # All foot extremes now fit inside the original row21 contact and gutters.
    actual=trimmed.getbbox();dy=22-actual[3]
    result=Image.new("RGBA",(40,24));result.paste(trimmed,(0,dy))
    if skin==0:
        # This anatomy carries native slate legs and a low canine muzzle.
        # Replace the jasper material with ash and cut its shell into the
        # original hound's dorsal plate ridge, keeping the real shoulder mass.
        for y in range(24):
            for x in range(40):
                p=result.getpixel((x,y))
                if not p[3]:continue
                v=sum(p[:3])/3
                if p[0]>p[1]+22 and p[0]>p[2]+10:
                    c="7c7780" if v>105 else "5b5966" if v>65 else "3f414f"
                else:c="292c3c" if v<47 else "393746" if v<65 else "5b5966" if v<99 else "7c7780"
                result.putpixel((x,y),rgba(c))
        d=ImageDraw.Draw(result)
        # Native angular dorsal caps follow the oblique spine; no round dome.
        leftward=direction in ["up_left","down_left"]
        caps=[(19,5),(21,7),(23,9)] if "up_" in direction else [(16,5),(19,6),(22,7)]
        for x,y in caps:
            if leftward:x=39-x
            d.polygon([(x-1,y+3),(x,y),(x+2,y+4)],fill=rgba("5b5966"))
            d.line([(x,y),(x+1,y+2)],fill=rgba("7c7780"))
    return result


def hound(direction,anim,frame,skin):
    assert direction in ["up_right","down_right","down_left","up_left"], direction
    seed=native_hound_seed(direction,skin)
    out=Image.new("RGBA",seed.size)
    # The rigid shoulder is kept intact across every frame. Lower joint/foot
    # pieces articulate independently, avoiding the prior shrinking carapace.
    cut=17 if "up_" in direction else 16
    upper=seed.crop((0,0,40,cut))
    body_dx=0;body_dy=0
    if anim=="idle":body_dy=1 if frame==2 else 0
    elif anim=="move":body_dy=[0,0,1,0,0,-1][frame]
    elif anim=="windup":body_dy=1 if frame else 0;body_dx=(-1 if "right" in direction else 1) if frame==2 else 0
    elif anim=="attack":body_dy=[0,-2,-1,1][frame];body_dx=[0,1,1,0][frame]*(1 if "right" in direction else -1)
    elif anim=="hurt":body_dx=(-1 if "right" in direction else 1) if frame==0 else 0
    out.alpha_composite(upper,(body_dx,body_dy))
    # Split lower native anatomy into individual two-/three-column leg chains.
    # Root rows overlap the original torso by one native row on compression.
    runs=[];left=None
    ink=seed.getchannel("A")
    for x in range(41):
        a=x<40 and any(ink.getpixel((x,y)) for y in range(17,22))
        if a and left is None:left=x
        if not a and left is not None:runs.append((left,x));left=None
    for index,(lo,hi) in enumerate(runs):
        leg=seed.crop((lo,cut,hi,24));dx=dy=0
        if anim=="move":
            phase=(frame+index*3)%6;dx=[-2,0,2,2,0,-2][phase];dy=[-1,0,0,-1,-2,-1][phase]
        elif anim=="attack" and frame in [1,2]:
            dx=(-2 if index%2==0 else 2)*(1 if "right" in direction else -1);dy=-2 if frame==1 else -1
        out.alpha_composite(leg,(lo+dx,cut+dy))
    # Registration keeps at least a one-native-pixel gutter even in leap/hurt.
    for x in range(40):out.putpixel((x,0),EMPTY);out.putpixel((x,23),EMPTY)
    for y in range(24):out.putpixel((0,y),EMPTY);out.putpixel((39,y),EMPTY)
    right="right" in direction
    # Cues refer to the actual native head plate or visible dorsal seam. North
    # facings deliberately put the eye at the visible outer brow, never rump.
    eye={"up_right":(26,8),"down_right":(31,10),"up_left":(10,8),"down_left":(13,9)}[direction]
    muzzle={"up_right":(28,10),"down_right":(33,12),"up_left":(8,10),"down_left":(11,11)}[direction]
    eye=(eye[0]+body_dx,eye[1]+body_dy);muzzle=(muzzle[0]+body_dx,muzzle[1]+body_dy)
    candidates=[(x,y) for y in range(max(1,eye[1]-2),min(23,eye[1]+3)) for x in range(max(1,eye[0]-2),min(39,eye[0]+3)) if out.getpixel((x,y))[3]]
    if candidates:eye=min(candidates,key=lambda p:(p[0]-eye[0])**2+(p[1]-eye[1])**2)
    if anim=="attack" and frame in [1,2]:
        # The jaw is a separately authored local part: gape on the bite key,
        # followed by a shut snapping lip. The shoulder never changes size.
        mx,my=muzzle;dd=ImageDraw.Draw(out);toward=1 if right else -1
        dd.line([(mx-2*toward,my-1),(mx,my-1)],fill=rgba("292c3c"))
        if frame==1:
            dd.line([(mx-2*toward,my),(mx,my)],fill=rgba("d48670"))
            dd.line([(mx-2*toward,my+1),(mx,my+1)],fill=rgba("393746"))
    color=rgba("efb68a" if anim in ["windup","attack","hurt"] else "d48670")
    out.putpixel(eye,color)
    # Original quiet seam palette; no invented free particles or huge heat block.
    return out,native_normal(out),{"eye":eye,"muzzle":muzzle}




def native_totem_seed(direction,skin):
    label="copper" if skin==1 else "ceramic"
    facing={"up_right":"north-east","right":"east","down_right":"south-east","down_left":"south-west","left":"west","up_left":"north-west"}[direction]
    raw=Image.open(ART/f"pixellab/{label}-{facing}.png").convert("RGBA")
    # Crop transparent square padding only. Native furnace, cavities and flues
    # stay their original pixel dimensions; lower belly shortens at a stone seam.
    raw=raw.crop((4,0,32,36))
    top=raw.getbbox()[1];shift=max(0,3-top)
    result=Image.new("RGBA",(28,36));result.paste(raw.crop((0,0,28,27-shift)),(0,shift));result.paste(raw.crop((0,27,28,36)),(0,27))
    return result


def totem(direction,anim,frame,skin,source):
    if skin==0:
        # The original distributor is a thin slotted basalt fin. Native
        # quarter-view planes retain its real long aperture and broken crown.
        im=source.copy();d=ImageDraw.Draw(im);d.rectangle((0,0,27,17),fill=EMPTY)
        right=direction in ["up_right","right","down_right"]
        back=direction.startswith("up")
        if direction in ["right","left"]:
            d.polygon([(13,3),(16,5),(17,8),(17,17),(11,17),(11,7)],fill=rgba("292c3c"))
            d.polygon([(13,4),(15,6),(15,17),(12,17),(12,7)],fill=rgba("79707a"))
            d.polygon([(15,6),(16,8),(16,17),(15,17)],fill=rgba("56505d"))
            d.line([(13,8),(13,16)],fill=rgba("3f414f"))
        else:
            d.polygon([(12,3),(15,4),(18,8),(18,17),(9,17),(9,7)],fill=rgba("292c3c"))
            d.polygon([(12,4),(14,5),(14,17),(10,17),(10,7)],fill=rgba("79707a" if not back else "56505d"))
            d.polygon([(15,6),(17,9),(17,17),(15,17)],fill=rgba("56505d" if not back else "79707a"))
            d.polygon([(12,7),(13,8),(13,17),(11,17),(11,9)],fill=rgba("292c3c"))
            d.line([(10,7),(12,4),(14,5)],fill=rgba("9a8e92"))
        if not right:
            upper=ImageOps.mirror(im.crop((0,0,28,18)));im.paste(upper,(0,0))
        return im,native_normal(im),{"maw":[13,21],"aim":list(VECTORS[DIRS.index(direction)])}
    im=native_totem_seed(direction,skin)
    # The actual native plinth stays registered and stationary at every turn.
    im.paste(source.crop((0,31,28,36)),(0,31))
    d=ImageDraw.Draw(im)
    # Generated ports appeared on arbitrary side faces. A fixed inset firing
    # pivot remains in the historical physical position; the surrounding upper
    # furnace shell/vents articulate around it. Quiet the obsolete side glow.
    for y in range(36):
        for x in range(28):
            p=im.getpixel((x,y))
            if not p[3]:continue
            if skin==2 and p[0]>p[1]+30 and p[0]>130:
                im.putpixel((x,y),rgba("363947"))
    # Native common outlet/backplate from the source binds all variants and
    # directions to exactly the same lob point and frame-by-frame combat cue.
    # The panel is a distinct physical part, not a replacement of the chassis.
    panel=source.crop((8,18,19,24))
    im.paste(panel,(8,18))
    # Late windup visibly parts its two lips and recoil lifts only the block.
    if anim=="attack" and frame==0:
        im.paste(source.crop((8,17,19,24)),(8,17))
    if anim=="hurt":
        # Keep the old appearance's exact material response on both hurt keys,
        # including the copper furnace's deliberately dark first recoil.
        palette={p for p in source.get_flattened_data() if p[3] and p[:3] not in EMIT}
        for y in range(36):
            for x in range(28):
                p=im.getpixel((x,y))
                if p[3] and p[:3] not in EMIT:
                    im.putpixel((x,y),min(palette,key=lambda q:sum((p[i]-q[i])**2 for i in range(3))))
    nm=native_normal(im)
    # Restore baseline exact-emissive set: alternate shells cannot introduce a
    # new firing glow behind the fixed native aperture.
    for y in range(36):
        for x in range(28):
            p=im.getpixel((x,y))
            if p[:3] in EMIT and not (8<=x<19 and 17<=y<24):im.putpixel((x,y),rgba("a36059"))
    return im,nm,{"maw":[13,21],"aim":list(VECTORS[DIRS.index(direction)])}


def mirror_normals(im):
    im=ImageOps.mirror(im)
    return Image.merge("RGBA",(ImageOps.invert(im.getchannel("R")),im.getchannel("G"),im.getchannel("B"),im.getchannel("A")))


def support_receipt(cell, depth):
    """Actual full-width nonemissive sole band used by production drawing."""
    opaque=lambda x,y: cell.getpixel((x,y))[3] and cell.getpixel((x,y))[:3] not in EMIT
    bottom=max(y for y in range(cell.height) for x in range(cell.width) if opaque(x,y))
    top=max(0,bottom-depth+1);columns=[]
    for x in range(cell.width):
        ys=[y for y in range(top,bottom+1) if opaque(x,y)]
        columns.append([min(ys),max(ys)+1] if ys else [-1,-1])
    return {"depth":depth,"native_band":[top,bottom+1],"columns":columns,"anchor_space":"source-native pixels; fixed sheet anchor"}


def snapshot():
    for sub in ["source/sprites","source/normals","evidence"]:(ART/sub).mkdir(parents=True,exist_ok=True)
    names=[species+suffix for species in ["ember_hound","spitting_totem"] for suffix in ["","_v1","_v2"]]
    for name in names:
        for ext in ["png","json"]:
            target=ART/f"source/sprites/{name}.{ext}"
            if not target.exists():shutil.copy2(ROOT/f"assets/sprites/{name}.{ext}",target)
        target=ART/f"source/normals/{name}.png"
        if not target.exists():shutil.copy2(ROOT/f"assets/normals/{name}.png",target)
    return names


def pack(out):
    names=snapshot();reports=[]
    (out/"sprites").mkdir(parents=True,exist_ok=True);(out/"normals").mkdir(parents=True,exist_ok=True)
    cues={};pose_points={}
    for name in names:
        species=name.split("_v")[0];skin=0 if "_v" not in name else int(name[-1])
        src=ART/f"source/sprites/{name}.png";meta=json.loads((ART/f"source/sprites/{name}.json").read_text())
        original=Image.open(src).convert("RGBA");oldnorm=Image.open(ART/f"source/normals/{name}.png").convert("RGBA")
        cw,ch=meta["cell"];states=["idle","move","windup","attack","hurt"] if species=="ember_hound" else ["idle","windup","attack","hurt"]
        missing=[d for d in DIRS if f"idle_{d}" not in meta["anims"]]
        rows=original.height//ch;atlas=Image.new("RGBA",(original.width,(rows+len(missing)*len(states))*ch));normal=Image.new("RGBA",atlas.size)
        atlas.paste(original,(0,0));normal.paste(oldnorm,(0,0))
        coverage=[]
        for direction in missing:
            for state in states:
                source_anim=meta["anims"][f"{state}_right" if species=="ember_hound" else f"{state}_down"]
                a=copy.deepcopy(source_anim);a["row"]=rows;a["native_eightway"]=True
                points={};frame_reports=[]
                for i in range(a["frames"]):
                    if species=="ember_hound" and direction=="left":
                        old=source_anim["row"]
                        cell=ImageOps.mirror(original.crop((i*cw,old*ch,(i+1)*cw,(old+1)*ch)))
                        nm=mirror_normals(oldnorm.crop((i*cw,old*ch,(i+1)*cw,(old+1)*ch)))
                        pts={}
                    elif species=="ember_hound":cell,nm,pts=hound(direction,state,i,skin)
                    else:
                        old=source_anim["row"]
                        source=original.crop((i*cw,old*ch,(i+1)*cw,(old+1)*ch))
                        cell,nm,pts=totem(direction,state,i,skin,source)
                    key=(species,direction,state,i)
                    if skin==0:
                        cues[key]={(x,y):cell.getpixel((x,y)) for y in range(ch) for x in range(cw) if cell.getpixel((x,y))[:3] in EMIT and cell.getpixel((x,y))[3]}
                        pose_points[key]=pts
                    else:
                        # Every appearance retains identical authored combat cue masks.
                        for x,y in [(x,y) for y in range(ch) for x in range(cw) if cell.getpixel((x,y))[:3] in EMIT]:
                            cell.putpixel((x,y),rgba("a36059"))
                        for p,c in cues[key].items():cell.putpixel(p,c);nm.putpixel(p,(128,150,251,255))
                        pts=pose_points[key]
                    for k,v in pts.items():points.setdefault(k,[]).append(v)
                    atlas.paste(cell,(i*cw,rows*ch));normal.paste(nm,(i*cw,rows*ch))
                    assert cell.getbbox(),(name,state,direction,i)
                    assert cell.getchannel("A").tobytes()==nm.getchannel("A").tobytes(),(name,state,direction,i,"normal alpha")
                    assert set(cell.getchannel("A").get_flattened_data())<={0,255}
                    frame_reports.append({"frame":i,"bbox":cell.getbbox(),"opaque":sum(v>0 for v in cell.getchannel("A").get_flattened_data()),"emission":len(cues[key]),"points":pts,"support":support_receipt(cell,4 if species=="ember_hound" else 5)})
                a.update(points);meta["anims"][f"{state}_{direction}"]=a
                coverage.append({"animation":f"{state}_{direction}","row":rows,"durations":a["durations"],"frames":frame_reports});rows+=1
        meta["eightway"]={"directions":DIRS,"recipe":"tools/eightway/cinder.py","native_parts":True,"legacy_rows_preserved":original.height//ch,"death":"selected pose debris; legacy reference stream retained","fixed_chassis":species=="spitting_totem"}
        meta["notes"]+=" Native eight-way extension: projected articulated anatomy and native facet normals. Historical rows retained. Fixed totem lower plinth and gameplay projectile launch unchanged; furnace vanes turn around fixed firing pivot."
        atlas.save(out/f"sprites/{name}.png");normal.save(out/f"normals/{name}.png");(out/f"sprites/{name}.json").write_text(json.dumps(meta,indent=2)+"\n")
        assert atlas.crop((0,0,*original.size)).tobytes()==original.tobytes()
        assert normal.crop((0,0,*oldnorm.size)).tobytes()==oldnorm.tobytes()
        full_coverage=[]
        for dr in DIRS:
            for state in states:
                key=f"{state}_{dr}";entry=meta["anims"][key]
                full_coverage.append({"animation":key,"direction":dr,"row":entry["row"],"frames":entry["frames"],"durations":entry["durations"],"lookup":"exact authored row; no fallback","normal":"same atlas cell and exact alpha"})
        reports.append({"name":name,"cell":meta["cell"],"anchor":meta["anchor"],"old_rows":original.height//ch,"total_rows":rows,"added":coverage,"coverage":full_coverage,"png_sha256":digest(out/f"sprites/{name}.png"),"normal_sha256":digest(out/f"normals/{name}.png"),"json_sha256":digest(out/f"sprites/{name}.json")})
    return reports


def evidence(out,reports):
    for report in reports:
        name=report["name"];meta=json.loads((out/f"sprites/{name}.json").read_text());im=Image.open(out/f"sprites/{name}.png").convert("RGBA");cw,ch=meta["cell"]
        states=["idle","move","windup","attack","hurt"] if "hound" in name else ["idle","windup","attack","hurt"]
        for state in states:
            small=Image.new("RGB",(80+6*cw,8*(ch+7)),(35,36,45));sd=ImageDraw.Draw(small)
            for di,dr in enumerate(DIRS):
                a=meta["anims"][f"{state}_{dr}"];y=di*(ch+7);sd.text((2,y+5),dr,fill=(215,210,205))
                for i in range(a["frames"]):
                    c=im.crop((i*cw,a["row"]*ch,(i+1)*cw,(a["row"]+1)*ch));small.paste(c,(80+i*cw,y),c)
            small.save(ART/f"evidence/{name}-{state}-1x.png");small.resize((small.width*3,small.height*3),Image.Resampling.NEAREST).save(ART/f"evidence/{name}-{state}-3x.png")
        # Every actual frame, direction columns with six native cells per column.
        sheet=Image.new("RGB",(80+8*6*cw,len(states)*(ch+16)+20),(35,36,45));d=ImageDraw.Draw(sheet)
        for di,dr in enumerate(DIRS):
            d.text((80+di*6*cw,2),dr,fill=(220,215,210))
            for si,state in enumerate(states):
                a=meta["anims"][f"{state}_{dr}"];y=20+si*(ch+16)
                if di==0:d.text((2,y),state,fill=(220,215,210))
                for frame in range(a["frames"]):
                    c=im.crop((frame*cw,a["row"]*ch,(frame+1)*cw,(a["row"]+1)*ch));sheet.paste(c,(80+di*6*cw+frame*cw,y),c)
        sheet.save(ART/f"evidence/{name}-all-frames-1x.png");sheet.resize((sheet.width*3,sheet.height*3),Image.Resampling.NEAREST).save(ART/f"evidence/{name}-all-frames-3x.png")
        compact=Image.new("RGB",(8*(cw+8),len(states)*(ch+14)),(35,36,45));d=ImageDraw.Draw(compact)
        for di,dr in enumerate(DIRS):
            for si,state in enumerate(states):
                a=meta["anims"][f"{state}_{dr}"];c=im.crop((0,a["row"]*ch,cw,(a["row"]+1)*ch));x=di*(cw+8);y=si*(ch+14)
                compact.paste(c,(x,y+12),c);d.text((x,y),dr if si==0 else state,fill=(200,200,200))
        compact.resize((compact.width*4,compact.height*4),Image.Resampling.NEAREST).save(ART/f"evidence/{name}-directions-4x.png")
        frames=[]
        for state in states:
            for dr in DIRS+["up_left","up","up_right","right","right","up_right"]:
                a=meta["anims"][f"{state}_{dr}"]
                for i in range(a["frames"]):
                    c=im.crop((i*cw,a["row"]*ch,(i+1)*cw,(a["row"]+1)*ch));f=Image.new("RGB",(80,64),(35,36,45));fd=ImageDraw.Draw(f);fd.text((2,1),state+" "+dr,fill="white");f.paste(c,((80-cw)//2,20),c)
                    frames.extend([f.resize((320,256),Image.Resampling.NEAREST)]*max(1,a["durations"][i]//2))
        frames[0].save(ART/f"evidence/{name}-sweep.gif",save_all=True,append_images=frames[1:],duration=33,loop=0,optimize=False)
    (ART/"coverage.json").write_text(json.dumps({"directions":DIRS,"skins":reports,"directionless":["ember_glob freely flies and spins on its existing continuous projectile vector","hound scorch/lane/particles continuous world vectors","totem splash and breakup are procedural"],"fixed_parts":["totem native lower plinth stays planted for all facing sectors"],"physics":"All eightway decisions are visual; exact aim, attack milestones, movement and historical lob launch remain unchanged."},indent=2)+"\n")


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("--out",type=Path,required=True);a=p.parse_args()
    reports=pack(a.out);evidence(a.out,reports)
    print(json.dumps({r["name"]:{"old_rows":r["old_rows"],"total_rows":r["total_rows"],"added_frames":sum(len(x["frames"]) for x in r["added"])} for r in reports},indent=2))


if __name__=="__main__":main()
