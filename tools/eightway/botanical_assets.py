#!/usr/bin/env python3
"""Native articulated quarter-view botanical poses; never rotate/rescale a bitmap.

Original cardinal albedo, normals, metadata and receipts are immutable inputs.
Diagonals redraw body planes, masks, leaves and joint chains on the native grid.
The shared motion rig retains exact frame durations and hop support heights.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "art/eightway/botanical"
NAMES = ("sproutling", "sproutling_v1", "sproutling_v2", "thorn_imp", "thorn_imp_v1", "thorn_imp_v2")
BASES = ("idle", "move", "windup", "attack", "hurt")
DIRS = ("up", "up_right", "right", "down_right", "down", "down_left", "left", "up_left")
INK = "#303b4d"
SAGE = ("#3b535b", "#465b61", "#647c77", "#829b8e")
ROSE = ("#655f78", "#927c91", "#bf9da9", "#dbb9bd", "#dfdac1")
SEED_COLORS = {(163,163,143,255), (189,187,161,255)}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sprite_grid(rows, pal):
    out = Image.new("RGBA", (max(map(len, rows)), len(rows)))
    for y, row in enumerate(rows):
        for x, c in enumerate(row):
            if c in pal:
                out.putpixel((x,y), tuple(bytes.fromhex(pal[c].lstrip("#")))+(255,))
    return out


def outlined(image):
    """One native, four-connected structural outline, never a blurred alpha edge."""
    out = Image.new("RGBA", image.size)
    d = ImageDraw.Draw(out)
    for y in range(image.height):
        for x in range(image.width):
            if image.getpixel((x,y))[3]:
                for p in ((x-1,y),(x+1,y),(x,y-1),(x,y+1)):
                    d.point(p, fill=INK)
    out.alpha_composite(image)
    return out


def line(d, pts, color):
    if len(pts)>1:
        d.line(pts, fill=color, width=1)


def sprout(name, direction, base, index, original):
    """Body is a new asymmetric quarter-volume, with two depth-separated root feet."""
    up = direction == "up_right"
    variant = int(name[-1]) if "_v" in name else 0
    image = Image.new("RGBA", (24,24)); d=ImageDraw.Draw(image)
    crouch = (base == "idle" and index == 2) or base == "move" and index in (0,5) or base == "hurt" and index == 1
    rear = base == "windup" and index>0 or base == "hurt" and index==0
    lunge = base == "attack" and index<2
    impact = base == "attack" and index==2
    hot = base == "windup" and index>0 or base == "attack" and index<3
    blink = base == "idle" and index==4
    dx = (1+index if lunge else -1 if rear else 0) + (int(index%2==1) if base=="windup" and index>2 else 0)
    by = 15 + int(crouch) - int(rear)*2 - int(base=="move" and index in (1,4)) + int(impact)
    if lunge:by=18
    # The far root inserts above the near root; feet point along the diagonal.
    near = [(12+dx,by+1),(13+dx,18),(12+dx,20),(14+dx,20)]
    far = [(9+dx,by),(8+dx,17),(7+dx,18),(9+dx,18)]
    line(d,far,SAGE[0]); line(d,near,SAGE[2])
    if base=="move" and index in (2,3):
        image=Image.new("RGBA",(24,24));d=ImageDraw.Draw(image)
        line(d,[(9+dx,by),(8+dx,by+3),(6+dx,by+3)],SAGE[0])
        line(d,[(12+dx,by+1),(14+dx,by+3),(15+dx,by+3)],SAGE[2])
    # Facing determines the snout's depth plane, not a rotation of the source.
    if up:
        outline=[(6+dx,by-4),(9+dx,by-7),(14+dx,by-7),(17+dx,by-5),(17+dx,by-2),(13+dx,by+1),(8+dx,by)]
        lit=[(7+dx,by-4),(10+dx,by-6),(13+dx,by-6),(11+dx,by-2),(8+dx,by-1)]
        nose=[(14+dx,by-7),(17+dx,by-7),(18+dx,by-5),(16+dx,by-3),(14+dx,by-3)]
    else:
        outline=[(6+dx,by-5),(10+dx,by-7),(14+dx,by-5),(17+dx,by-2),(16+dx,by+1),(12+dx,by+2),(8+dx,by)]
        lit=[(7+dx,by-5),(10+dx,by-6),(12+dx,by-4),(11+dx,by),(8+dx,by-1)]
        nose=[(13+dx,by-3),(16+dx,by-2),(18+dx,by),(15+dx,by+2),(12+dx,by)]
    if lunge:
        # The ram's entire husk lies low along the diagonal strike line. These
        # are separately authored launch polygons, not an upright idle pasted
        # over moving legs and not a transformed raster.
        if up:
            outline=[(2+dx,by),(7+dx,by-3),(13+dx,by-5),(17+dx,by-6),(18+dx,by-4),(14+dx,by-1),(8+dx,by+2),(3+dx,by+2)]
            lit=[(3+dx,by),(8+dx,by-3),(12+dx,by-4),(10+dx,by-1),(5+dx,by+1)]
            nose=[(15+dx,by-6),(18+dx,by-6),(19+dx,by-5),(17+dx,by-3),(15+dx,by-3)]
        else:
            outline=[(2+dx,by-5),(7+dx,by-6),(12+dx,by-5),(16+dx,by-3),(18+dx,by-2),(18+dx,by),(14+dx,by+1),(9+dx,by-1),(4+dx,by-1)]
            lit=[(3+dx,by-5),(8+dx,by-5),(12+dx,by-3),(10+dx,by-2),(5+dx,by-2)]
            nose=[(15+dx,by-3),(18+dx,by-2),(19+dx,by),(17+dx,by+1),(14+dx,by-1)]
        if index==1:
            outline=[(x-1 if x<7+dx else x,y) for x,y in outline]
    d.polygon(outline,fill=ROSE[0]);d.polygon(lit,fill=ROSE[2]);d.line(lit[:3],fill=ROSE[3])
    if variant==1:
        # Oatcap: broad shallow ivory cap, recessed gills under its near lip.
        cy=by-3; cap=[(4+dx,cy),(6+dx,cy-2),(10+dx,cy-3),(15+dx,cy-3),(18+dx,cy-1),(19+dx,cy+1),(17+dx,cy+2),(6+dx,cy+2)]
        if lunge:cap=[(x,round(cy+(y-cy)*0.65)) for x,y in cap]
        d.polygon(cap,fill="#bf9da9");d.polygon([(4+dx,cy),(7+dx,cy-1),(11+dx,cy-2),(15+dx,cy-2),(18+dx,cy)],fill="#dbb9bd")
        d.line([(4+dx,cy+1),(10+dx,cy+2),(18+dx,cy+1)],fill="#655f78")
        for x in (7,10,13,16):d.point((x+dx,cy+2),fill="#303b4d")
        d.line([(8+dx,cy-1),(10+dx,cy-1)],fill="#dfdac1")
        d.point((14+dx,cy),fill="#927c91")
    elif variant==2:
        # Camellia: five individually drawn overlapping petal planes and sepals.
        cx=11+dx;cy=by-3
        for pts,c in [([(cx-5,cy-3),(cx-3,cy-6),(cx,cy-3),(cx-1,cy)],ROSE[1]), ([(cx-1,cy-4),(cx+2,cy-6),(cx+4,cy-2),(cx+1,cy)],ROSE[2]), ([(cx+2,cy-2),(cx+6,cy-2),(cx+6,cy+1),(cx+1,cy+2)],ROSE[3]), ([(cx,cy+1),(cx+3,cy+4),(cx-1,cy+3),(cx-3,cy)],ROSE[1]), ([(cx-4,cy-1),(cx-1,cy-2),(cx+2,cy),(cx,cy+2),(cx-3,cy+1)],ROSE[2])]:
            if lunge:pts=[(x,round(cy+(y-cy)*0.55)) for x,y in pts]
            d.polygon(pts,fill=c)
        d.line([(cx-4,cy+1),(cx-6,cy+2),(cx-5,cy-1)],fill=SAGE[2])
        d.line([(cx,cy+3),(cx+2,cy+5),(cx+3,cy+2)],fill=SAGE[1])
        nose=[(x-1,y) for x,y in nose]
    d.polygon(nose,fill="#c2c1b3" if up else "#dfdac1")
    if up:
        d.line([(15+dx,by-4),(16+dx,by-5)],fill="#bd7a85" if hot else INK)
        d.point((17+dx,by-6),fill="#dfdac1")
    else:
        eye="#c2c1b3" if base=="hurt" else "#bd7a85" if hot else INK
        d.point((13+dx,by-2),fill=eye);d.line([(15+dx,by-1),(16+dx,by-1)],fill=eye)
        if blink:d.line([(13+dx,by-2),(16+dx,by-1)],fill=INK)
        d.point((15+dx,by+1),fill="#c2c1b3")
    # Crown roots in the back shoulder; separate planar leaves rake behind aim.
    root=(8+dx,by-5)
    if lunge:root=(6+dx,by-3)
    flare=base=="windup" and index>0
    sway=int(base=="idle" and index in (1,2))
    leaves=[[(root[0],root[1]),(root[0]-4+sway,root[1]-(5 if flare else 3)),(root[0]-3,root[1]),root], [root,(root[0]+1+sway,root[1]-5),(root[0]+3+sway,root[1]-4),(root[0]+1,root[1]-1)]]
    if variant==1:leaves=[]  # The oatcap's crown is its broad cap, never antlers.
    if variant==2:leaves=leaves[:1]  # Camellia sepals sit behind the petal planes.
    if lunge:leaves=[[(x,root[1]+round((y-root[1])*0.4)) for x,y in leaf] for leaf in leaves]
    for j,leaf in enumerate(leaves):
        d.polygon(leaf,fill=SAGE[j]);d.line(leaf[:2],fill=SAGE[2])
    image=outlined(image)
    # Keep the authored per-frame airborne height exactly; feet never invent lift.
    bottom=original.getbbox()[3]; own=image.getbbox()[3]
    shifted=Image.new("RGBA",image.size);shifted.alpha_composite(image,(0,bottom-own))
    return shifted


IMP_MASKS={
  (0,False):["..QO...",".QQON..","QQOONM.","QOoONM.","QooOoM.","QOoONM.",".OONMM.","..ONM..","..NM...","...M..."],
  (0,True):["..NOM..",".NyxwM.","NyxxwM.","OyxxwM.","NxxwwM.",".xwwMM.","..wwM..","...M..."],
  (1,False):[".Q...Q.",".QQ.QO.","QQQQONM","QOoOONM","QOooONM","QOoONMM",".OONMM.",".ONMMM.","..NM...","..NM...","...M..."],
  (1,True):[".N...M.",".NN.MM.","NNyxwMM","NyxxwMM","NxxxwMM",".xxwwM.","..wwMM.","...MM..","...M..."],
  (2,False):["...xz...",".zzQQO..","zzQQQON.","zQOoONMM",".QOooNMM","..OONMM.","...xww..","....w..."],
  (2,True):["...xz...",".zzNyw..","zzNyxww.","zNyxxwwM",".NxxxwwM","..xxwwM.","...wwM..","....M..."]}


def imp_poses():
    sys.path.insert(0,str(ROOT/"tools/bloomwild_enemies"))
    spec=importlib.util.spec_from_file_location("botanical_imp_rig",ROOT/"tools/bloomwild_enemies/imp.py")
    mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
    mod.fig=lambda pose:pose
    return {name[:-6]:frames for name,frames,meta in mod.anims_right()},mod


def imp(name,direction,base,index,original,poses,rig,hand_source=None,side_hand=False,mirror_body=False):
    """Reproject native joint chains, then draw separately authored quarter masks."""
    up=direction=="up_right"; variant=int(name[-1]) if "_v" in name else 0
    p=copy.deepcopy(poses[base][index]); image=Image.new("RGBA",(32,32));d=ImageDraw.Draw(image)
    def project(q,depth=0):
        x,y=q
        return (round(16+(x-10)*0.72+depth),round(y+9+(-1 if up else 1)*(x-10)*0.20))
    throwing=base=="windup" and index>0 or base=="attack" and index<2
    for part,color,depth in [("far_leg",SAGE[0],-1),("far_arm",SAGE[0],-1),("spine",SAGE[1],0),("near_leg",SAGE[2],1),("near_arm",SAGE[2],1)]:
        if part=="near_arm" and throwing:continue
        pts=[project(q,depth) for q in p[part]]
        line(d,pts,color)
        if part=="spine":line(d,[(x,y+1) for x,y in pts],SAGE[0]);line(d,[(x,y-1) for x,y in pts],SAGE[2])
        elif part in ("near_leg","near_arm") and len(pts)>2:line(d,[(x-1,y) for x,y in pts[:2]],SAGE[1])
    for x,y,*c in p["thorns"]:
        q=project((x,y));d.point(q,fill="#bf9da9");d.point((q[0],q[1]+1),fill="#927c91")
    mx,my=p["mask"][1:];head=project((mx,my))
    mask=IMP_MASKS[variant,up];pal={"Q":"#dfdac1","O":"#c2c1b3","N":"#a1a4a5","M":"#747d8d","o":INK,"w":SAGE[0],"x":SAGE[1],"y":SAGE[2],"z":SAGE[3]}
    if base=="hurt":pal["o"]="#c2c1b3"
    if base=="windup" and index>0:pal["o"]="#8c7786"
    # Lichen/raffia mantle behind the mask preserves each appearance's anatomy.
    if variant:
        hx,hy=head; shade="#bf9da9" if variant==1 else "#c5b493"
        d.polygon([(hx-3,hy+3),(hx-5,hy+6),(hx-2,hy+8),(hx+1,hy+6)],fill="#927c91" if variant==1 else "#a68b6b")
        line(d,[(hx-3,hy+3),(hx-4,hy+6),(hx-2,hy+6)],shade)
        if variant==1:
            # The forked birch crown remains visible from the back quarter.
            for branch in [[(hx,hy+2),(hx-2,hy-1),(hx-3,hy-4)],[(hx+2,hy+1),(hx+5,hy-1),(hx+6,hy-4)]]:
                line(d,branch,SAGE[2]);d.point(branch[-1],fill="#bf9da9")
        else:
            # Broad bracken fan and folded raffia shoulder returns keep their
            # width across yaw rather than collapsing into a generic hood.
            d.polygon([(hx-3,hy+2),(hx,hy-2),(hx+3,hy-1),(hx+6,hy+2),(hx+5,hy+5),(hx+1,hy+5)],fill=SAGE[1])
            line(d,[(hx-3,hy+2),(hx,hy),(hx+3,hy+1),(hx+5,hy+3)],SAGE[3])
            line(d,[(hx-4,hy+6),(hx-1,hy+4),(hx+2,hy+6)],"#c5b493")
    image.alpha_composite(sprite_grid(mask,pal),(head[0]-2,head[1]))
    image=outlined(image)
    # Preserve source support height. Per-joint depth gives two real native feet.
    bottom=original.getbbox()[3];own=image.getbbox()[3]
    shifted=Image.new("RGBA",image.size);shifted.alpha_composite(image,(0,bottom-own));image=shifted
    if mirror_body:image=mirror_native(image,16)
    if throwing and hand_source is not None:
        # Seed and release sites are exact native gameplay data. Redraw the
        # articulated arm to that site; the quarter-facing body never changes.
        seed=[(x,y) for y in range(32) for x in range(32) if hand_source.getpixel((x,y)) in SEED_COLORS]
        if seed:
            sx=sum(q[0] for q in seed)/len(seed);sy=sum(q[1] for q in seed)/len(seed)
            target=(round(sx),round(sy)+1)
        else:
            target=(25,20) if side_hand else (17,8) if up else (19,28)
        shoulder=project(p["spine"][-1]);shoulder=(shoulder[0],shoulder[1]+bottom-own+2)
        if mirror_body:shoulder=(32-shoulder[0],shoulder[1])
        elbow=(round((shoulder[0]+target[0])/2)-2,round((shoulder[1]+target[1])/2)-1)
        arm=Image.new("RGBA",image.size);ad=ImageDraw.Draw(arm);line(ad,[shoulder,elbow,target],SAGE[2]);image.alpha_composite(outlined(arm))
        if seed:
            x0=min(x for x,y in seed)-1;y0=min(y for x,y in seed)-1;x1=max(x for x,y in seed)+2;y1=max(y for x,y in seed)+2
            # One-pixel native seed outline and reserved material mask survive
            # exactly, so hand() reads the original centroid in every skin.
            crop=hand_source.crop((x0,y0,x1,y1));image.paste(crop,(x0,y0))
    return image


def mirror_native(image, anchor_x):
    """Reflect about the ground anchor, not the half-pixel canvas midpoint."""
    out=Image.new("RGBA",image.size)
    for y in range(image.height):
        for x in range(image.width):
            dst=2*anchor_x-x
            if 0<=dst<image.width:out.putpixel((dst,y),image.getpixel((x,y)))
    return out


def mirror_normal(image, anchor_x):
    out=mirror_native(image,anchor_x)
    out.putdata([(255-r,g,b,a) if a else (0,0,0,0) for r,g,b,a in out.getdata()])
    return out


def normal_for(image):
    """Empty staging backing; complete production normal receipts are mandatory."""
    return Image.new("RGBA",image.size)


def registered_normals(name, albedo, normal, meta):
    """Install only new PixelLab native rows, stabilize holds, reflect native X."""
    raw_path=ART/f"{name}-normal-raw.png";packing_path=ART/f"{name}-normal-packing.json"
    if not raw_path.exists() or not packing_path.exists():
        raise FileNotFoundError(f"Missing registered native normal source for {name}; no production fallback")
    if raw_path.exists() and packing_path.exists():
        packing=json.loads(packing_path.read_text());raw=Image.open(raw_path).convert("RGBA")
        source=Image.open(ART/f"{name}-normal-input.png").convert("RGBA")
        assert raw.size==source.size
        cw,ch=meta["cell"];cols=packing["columns"]
        expected={(key,i) for key,a in meta["anims"].items() if a["row"]>=15 and (key.endswith(("up_right","down_right")) or name.startswith("thorn_imp") and key.startswith(("windup_","attack_")) and "_side_" not in key) for i in range(a["frames"])}
        actual={(f["animation"],f["frame"]) for f in packing["frames"]}
        assert expected==actual and len(actual)==len(packing["frames"]),f"Incomplete native normal coverage for {name}"
        for i,f in enumerate(packing["frames"]):
            assert f["row"]==meta["anims"][f["animation"]]["row"],f"Stale native normal row for {name} {f}"
            x,y=i%cols*cw,i//cols*ch;dst=(f["frame"]*cw,f["row"]*ch)
            art=albedo.crop((dst[0],dst[1],dst[0]+cw,dst[1]+ch))
            guide=source.crop((x,y,x+cw,y+ch));n=raw.crop((x,y,x+cw,y+ch))
            assert art.tobytes()==guide.tobytes(),f"Stale native normal source for {name} {f}"
            n.putalpha(art.getchannel("A"));normal.paste(n,dst)
        spec=importlib.util.spec_from_file_location("botanical_normal_stability",ROOT/"tools/pixellab/round7_normal_assets.py")
        mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
        scoped={"cell":meta["cell"],"anims":{k:a for k,a in meta["anims"].items() if a["row"]>=15}}
        normal,poses,pixels=mod.stabilize_near_poses(albedo,normal,scoped)
        for name_left,a in meta["anims"].items():
            if a["row"]<15 or not name_left.endswith(("up_left","down_left")):continue
            right=meta["anims"][name_left.replace("_left","_right")]
            for i in range(a["frames"]):
                c=normal.crop((i*cw,right["row"]*ch,(i+1)*cw,(right["row"]+1)*ch))
                reflected=mirror_normal(c,meta["anchor"][0])
                if name.startswith("thorn_imp") and name_left.startswith(("windup_","attack_")) and "_side_" not in name_left:
                    # The fixed north/south launch arm is asymmetric. Unchanged
                    # mirrored3x3 body material keeps the reflected native
                    # volume; only its newly articulated hand/arm uses its own
                    # registered PixelLab geometry.
                    donor=mirror_native(albedo.crop((i*cw,right["row"]*ch,(i+1)*cw,(right["row"]+1)*ch)),meta["anchor"][0])
                    target=albedo.crop((i*cw,a["row"]*ch,(i+1)*cw,(a["row"]+1)*ch))
                    for yy in range(1,ch-1):
                        for xx in range(1,cw-1):
                            if target.getpixel((xx,yy))[3] and all(target.getpixel((xx+dx,yy+dy))==donor.getpixel((xx+dx,yy+dy)) for dy in (-1,0,1) for dx in (-1,0,1)):
                                normal.putpixel((i*cw+xx,a["row"]*ch+yy),reflected.getpixel((xx,yy)))
                else:normal.paste(reflected,(i*cw,a["row"]*ch))
    # Exact repeated poses, including unused transparent cells, share exact data.
    cw,ch=meta["cell"];canonical={}
    for y in range(0,albedo.height,ch):
        for x in range(0,albedo.width,cw):
            box=(x,y,x+cw,y+ch);key=albedo.crop(box).tobytes()
            if key in canonical and y>=15*ch:normal.paste(canonical[key],(x,y))
            else:canonical[key]=normal.crop(box)
    normal.putalpha(albedo.getchannel("A"))
    return normal


def build(out_dir):
    sprites=out_dir/"sprites";normals=out_dir/"normals";audit=out_dir/"audit"
    for d in (sprites,normals,audit):d.mkdir(parents=True,exist_ok=True)
    poses,rig=imp_poses();coverage={"method":"authored articulated native quarter planes; original cardinal rows immutable", "actors":[]}
    for name in NAMES:
        meta=json.loads((ART/f"{name}-original-sprites.json").read_text());old=Image.open(ART/f"{name}-original-sprites.png").convert("RGBA");oldn=Image.open(ART/f"{name}-original-normals.png").convert("RGBA")
        is_imp=name.startswith("thorn_imp")
        cw,ch=meta["cell"];new=Image.new("RGBA",(old.width,old.height+(28 if is_imp else 20)*ch));newn=Image.new("RGBA",new.size);new.paste(old,(0,0));newn.paste(oldn,(0,0));rows=old.height//ch
        artframes={};normalframes={};entries=[]
        for direction in ("up_right","down_right"):
            for base in BASES:
                source=meta["anims"][f"{base}_right"]
                frames=[];nframes=[]
                for index in range(source["frames"]):
                    original=old.crop((index*cw,source["row"]*ch,(index+1)*cw,(source["row"]+1)*ch))
                    if name.startswith("sprout"):
                        frame=sprout(name,direction,base,index,original)
                    else:
                        handanim=meta["anims"][f"{base}_{'up' if direction=='up_right' else 'down'}"]
                        handsource=old.crop((index*cw,handanim["row"]*ch,(index+1)*cw,(handanim["row"]+1)*ch))
                        frame=imp(name,direction,base,index,original,poses,rig,handsource)
                    frames.append(frame);nframes.append(normal_for(frame))
                artframes[base,direction]=frames;normalframes[base,direction]=nframes
                if is_imp and base in ("windup","attack"):
                    sideframes=[]
                    for index in range(source["frames"]):
                        original=old.crop((index*cw,source["row"]*ch,(index+1)*cw,(source["row"]+1)*ch))
                        sideframes.append(imp(name,direction,base,index,original,poses,rig,original,True))
                    artframes[base+"_side",direction]=sideframes;normalframes[base+"_side",direction]=list(map(normal_for,sideframes))
        for direction in ("up_right","down_right","down_left","up_left"):
            right=direction.replace("left","right");flipped=direction.endswith("left")
            for base in BASES:
                source=copy.deepcopy(meta["anims"][f"{base}_right"]);source["row"]=rows
                source["notes"]=source.get("notes","")+" Native articulated quarter view; world timing and launch sites retained."
                if "release_offset" in source:
                    source["release_offset"]=copy.deepcopy(meta["anims"][f"{base}_{'up' if direction.startswith('up') else 'down'}"]["release_offset"])
                meta["anims"][f"{base}_{direction}"]=source
                for index,(frame,nframe) in enumerate(zip(artframes[base,right],normalframes[base,right])):
                    if flipped:
                        frame=mirror_native(frame,meta["anchor"][0]);nframe=mirror_normal(nframe,meta["anchor"][0])
                        if is_imp and base in ("windup","attack"):
                            original_anim=meta["anims"][f"{base}_right"]
                            original=old.crop((index*cw,original_anim["row"]*ch,(index+1)*cw,(original_anim["row"]+1)*ch))
                            handanim=meta["anims"][f"{base}_{'up' if direction.startswith('up') else 'down'}"]
                            handsource=old.crop((index*cw,handanim["row"]*ch,(index+1)*cw,(handanim["row"]+1)*ch))
                            frame=imp(name,right,base,index,original,poses,rig,handsource,mirror_body=True);nframe=normal_for(frame)
                    new.paste(frame,(index*cw,rows*ch));newn.paste(nframe,(index*cw,rows*ch))
                    entries.append({"animation":base,"direction":direction,"frame":index,"row":rows,"support_y":frame.getbbox()[3]-1,"albedo_nonempty":True})
                rows+=1
        if is_imp:
            for direction in ("up_right","down_right","down_left","up_left"):
                right=direction.replace("left","right");flipped=direction.endswith("left")
                for base in ("windup","attack"):
                    source=copy.deepcopy(meta["anims"][f"{base}_right"]);source["row"]=rows
                    source["notes"]="Same quarter-facing body with an articulated throwing arm meeting the immutable side-biased gameplay launch site."
                    if flipped and "release_offset" in source:source["release_offset"][0]*=-1
                    meta["anims"][f"{base}_side_{direction}"]=source
                    for index,(frame,nframe) in enumerate(zip(artframes[base+"_side",right],normalframes[base+"_side",right])):
                        if flipped:frame=mirror_native(frame,meta["anchor"][0]);nframe=mirror_normal(nframe,meta["anchor"][0])
                        new.paste(frame,(index*cw,rows*ch));newn.paste(nframe,(index*cw,rows*ch))
                        entries.append({"animation":base+"_side","direction":direction,"frame":index,"row":rows,"support_y":frame.getbbox()[3]-1,"albedo_nonempty":True})
                    rows+=1
        meta["eightway"]={"directions":list(DIRS),"method":"authored joint chains, native volume planes, masks and separately anchored leaves","original_rows":old.height//ch}
        newn=registered_normals(name,new,newn,meta)
        new.save(sprites/f"{name}.png");newn.save(normals/f"{name}.png");(sprites/f"{name}.json").write_text(json.dumps(meta,indent=1)+"\n")
        # Readable all-state/all-direction contact sheet, plus true60Hz frame loops.
        audit_bases=BASES+(("windup_side","attack_side") if is_imp else ())
        max_frames=max(a["frames"] for a in meta["anims"].values());sheet=Image.new("RGBA",(max_frames*cw*8,len(audit_bases)*ch),"#181f2a")
        for bi,base in enumerate(audit_bases):
            for di,direction in enumerate(DIRS):
                srcdir="right" if direction=="left" else direction;anim=meta["anims"].get(f"{base}_{srcdir}")
                if anim is None:continue
                for i in range(anim["frames"]):
                    frame=new.crop((i*cw,anim["row"]*ch,(i+1)*cw,(anim["row"]+1)*ch))
                    if direction=="left":frame=ImageOps.mirror(frame)
                    sheet.alpha_composite(frame,((di*max_frames+i)*cw,bi*ch))
        sheet.save(audit/f"{name}-all-frames-native.png");sheet.resize((sheet.width*3,sheet.height*3),Image.Resampling.NEAREST).save(audit/f"{name}-all-frames-3x.png")
        # Direction sweep holds each sector through a complete authored animation.
        for base in audit_bases:
            loop=[]
            for direction in DIRS:
                srcdir="right" if direction=="left" else direction;anim=meta["anims"].get(f"{base}_{srcdir}")
                if anim is None:continue
                for i,duration in enumerate(anim["durations"]):
                    frame=new.crop((i*cw,anim["row"]*ch,(i+1)*cw,(anim["row"]+1)*ch))
                    if direction=="left":frame=ImageOps.mirror(frame)
                    bg=Image.new("RGBA",(cw+8,ch+8),"#181f2a");bg.alpha_composite(frame,(4,4));bg=bg.resize((bg.width*5,bg.height*5),Image.Resampling.NEAREST).convert("RGB")
                    loop.extend([bg]*duration)
            loop[0].save(audit/f"{name}-{base}-sweep.gif",save_all=True,append_images=loop[1:],duration=17,loop=0,optimize=False)
        assert new.crop((0,0,old.width,old.height)).tobytes()==old.tobytes()
        assert newn.crop((0,0,old.width,old.height)).tobytes()==oldn.tobytes()
        assert set(new.getchannel("A").getdata())<={0,255}
        assert new.getchannel("A").tobytes()==newn.getchannel("A").tobytes()
        coverage["actors"].append({"name":name,"directions":list(DIRS),"animations":list(BASES),"new_frames":entries,"albedo_sha256":digest(sprites/f"{name}.png"),"normal_sha256":digest(normals/f"{name}.png"),"original_rows_unchanged":True,"motion":f"audit/{name}-*-sweep.gif","contact_sheet":f"audit/{name}-all-frames-native.png"})
    (audit/"coverage.json").write_text(json.dumps(coverage,indent=2)+"\n")
    print(json.dumps({"actors":len(coverage["actors"]),"frames":sum(len(a["new_frames"]) for a in coverage["actors"]),"out":str(out_dir)}))


if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--out",required=True,type=Path);args=parser.parse_args();build(args.out)
