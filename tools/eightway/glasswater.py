#!/usr/bin/env python3
"""Author native Glasswater eight-way anatomy and registered geometry normals.

The frozen input rows are receipts, copied unchanged. New frames are rasterized
from articulated native parts: no bitmap rotation, resizing or cardinal relabel.
Run: uv run --offline --with pillow --with numpy python tools/eightway/glasswater.py --out PATH
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "art/eightway/glasswater"
NAMES = ("crystal_beetle", "crystal_beetle_v1", "crystal_beetle_v2", "wisp", "wisp_v1", "wisp_v2")
DIRS = {"up": (0,-1), "up_right": (2**-.5,-2**-.5), "right": (1,0), "down_right": (2**-.5,2**-.5),
        "down": (0,1), "down_left": (-2**-.5,2**-.5), "left": (-1,0), "up_left": (-2**-.5,-2**-.5)}
OUTLINE = (34,45,61,255)
SLATE = [(41,49,63,255),(54,70,87,255),(76,96,112,255),(115,133,139,255),(166,180,173,255)]
QUARTZ = [(77,72,102,255),(121,108,145,255),(161,149,184,255),(205,195,219,255),(226,216,233,255)]
IVORY = [(102,91,120,255),(144,143,148,255),(179,180,173,255),(197,199,184,255),(223,218,193,255)]


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def unit(n):
    n=np.array(n,dtype=float); return n/max(float(np.linalg.norm(n)),.00001)


def encoded(n):
    # Native world normals -> OpenGL upright tangent (right, up, towards camera).
    x,y,z=unit(n); return tuple(int(round((v+1)*127.5)) for v in (x,z,y))+(255,)


class Anatomy:
    def __init__(self,size,anchor,direction,ground=.55):
        self.size=size;self.anchor=anchor;self.f=DIRS[direction];self.ground=ground;self.parts=[]

    def world(self,p):
        a,b,z=p;fx,fy=self.f;return np.array((fx*a-fy*b,fy*a+fx*b,z),dtype=float)

    def project(self,p):
        x,y,z=self.world(p);return (round(self.anchor[0]+x),round(self.anchor[1]+self.ground*y-z))

    def poly(self,points,color,normal=None,outline=True):
        w=[self.world(p) for p in points]
        if normal is None:
            normal=unit(np.cross(w[1]-w[0],w[2]-w[0]))
        else:normal=self.world(normal)
        if normal[1]+self.ground*normal[2]<0:normal=-normal
        depths=[p[1]+p[2]*self.ground for p in w]
        self.parts.append((sum(depths)/len(w),[self.project(p) for p in points],color,encoded(normal),outline,depths))

    def line(self,a,b,color=OUTLINE,width=1):
        wa,wb=self.world(a),self.world(b);depth=(wa[1]+wb[1]+self.ground*(wa[2]+wb[2]))*.5
        self.parts.append((depth,[self.project(a),self.project(b)],color,encoded((0,.6,.8)),width,[wa[1]+wa[2]*self.ground,wb[1]+wb[2]*self.ground]))

    def dome(self,center,radii,palette,seg=10,rings=3):
        a,b,z=center;ra,rb,rz=radii
        vertices=[]
        for ring in range(rings+1):
            angle=(ring/rings)*math.pi*.5
            vertices.append([(a+math.cos(t*math.tau/seg)*ra*math.cos(angle),
                              b+math.sin(t*math.tau/seg)*rb*math.cos(angle),z+rz*math.sin(angle)) for t in range(seg)])
        for r in range(rings):
            for s in range(seg):
                pts=[vertices[r][s],vertices[r][(s+1)%seg],vertices[r+1][(s+1)%seg],vertices[r+1][s]]
                w=[self.world(p) for p in pts];n=unit(np.cross(w[1]-w[0],w[2]-w[0]))
                if n[1]+self.ground*n[2] <= -.01:continue
                light=float(np.dot(n,unit((-.6,-.25,1))))
                c=palette[min(len(palette)-1,max(0,int((light+1)*.5*(len(palette)-1))))]
                self.poly(pts,c,outline=False)
        # A dark lower lip carries shell weight without a pillow-shaded border.
        rim=[(a+math.cos(t*math.tau/seg)*ra,b+math.sin(t*math.tau/seg)*rb,z) for t in range(seg)]
        for s in range(seg):
            p,q=rim[s],rim[(s+1)%seg]
            if self.world(p)[1]>0:self.line(p,q,OUTLINE)

    def crystal(self,a,b,z,height,width,palette):
        apex=(a-2,b-.2,z+height);left=(a-width,b,z+1);right=(a+width,b,z+1);front=(a,b+width,z)
        self.poly([left,apex,front],palette[-2],outline=False)
        self.poly([front,apex,right],palette[1],outline=False)
        self.line(left,apex);self.line(apex,right)

    def render(self):
        im=Image.new("RGBA",self.size);nm=Image.new("RGBA",self.size);depth=np.full((self.size[1],self.size[0]),-np.inf)
        def put(x,y,z,color,normal):
            if 0<=x<self.size[0] and 0<=y<self.size[1] and z>=depth[y,x]-.001:
                im.putpixel((x,y),color);nm.putpixel((x,y),normal);depth[y,x]=z
        def line(a,b,za,zb,color,normal,width=1):
            steps=max(abs(a[0]-b[0]),abs(a[1]-b[1]),1)
            for i in range(steps+1):
                t=i/steps;x=round(a[0]+(b[0]-a[0])*t);y=round(a[1]+(b[1]-a[1])*t)
                for dx in range(width):put(x+dx,y,za+(zb-za)*t+.02,color,normal)
        for _,pts,color,normal,edge,zs in self.parts:
            if len(pts)==2:line(*pts,*zs,color,normal,int(edge));continue
            for k in range(1,len(pts)-1):
                p0,p1,p2=pts[0],pts[k],pts[k+1];z0,z1,z2=zs[0],zs[k],zs[k+1]
                den=(p1[1]-p2[1])*(p0[0]-p2[0])+(p2[0]-p1[0])*(p0[1]-p2[1])
                if den==0:continue
                for y in range(max(0,min(p0[1],p1[1],p2[1])),min(self.size[1]-1,max(p0[1],p1[1],p2[1]))+1):
                    for x in range(max(0,min(p0[0],p1[0],p2[0])),min(self.size[0]-1,max(p0[0],p1[0],p2[0]))+1):
                        a=((p1[1]-p2[1])*(x-p2[0])+(p2[0]-p1[0])*(y-p2[1]))/den
                        b=((p2[1]-p0[1])*(x-p2[0])+(p0[0]-p2[0])*(y-p2[1]))/den;c=1-a-b
                        if min(a,b,c)>=-.001:put(x,y,a*z0+b*z1+c*z2,color,normal)
            if edge:
                for k,p in enumerate(pts):line(p,pts[(k+1)%len(pts)],zs[k],zs[(k+1)%len(pts)],OUTLINE,normal)
        nm.putalpha(im.getchannel("A"));return im,nm


def energy(palette,anim,i):
    if anim=="dazed":return [palette[0],palette[0],palette[1],palette[1],palette[2]]
    if anim=="hurt":return [palette[0],palette[0],palette[1],palette[2],palette[3]]
    if anim in ("windup","attack") and (anim=="attack" or i>=3):return [palette[1],palette[2],palette[3],palette[4],palette[4]]
    if anim=="idle" and i in (1,2):return [palette[0],palette[1],palette[2],palette[3],palette[4]]
    return palette


def beetle(name,direction,anim,i):
    variant=0 if name=="crystal_beetle" else int(name[-1]);rig=Anatomy((32,26),(16,20),direction,.4)
    loaded=anim in ("windup","attack");bob=(i%2 if anim in ("move","attack") else 0)
    z=3-bob-(1 if loaded and i>0 else 0);shift=-1 if anim=="hurt" and i==0 else 0
    cp=energy(QUARTZ,anim,i)
    # Tripod gait: support contacts stay on their native ground plane. The
    # alternate tripod advances in body space; only the lifted feet leave it.
    for side in (-1,1):
        for leg,a in enumerate((-6,-1,4)):
            phase=(i+leg+(side==1))%2
            step=(1 if phase else -1) if anim in ("move","attack") else 0
            spread=8 if loaded else 7
            foot=(a+step,side*spread,1 if anim=="move" and phase and i%2==0 else 0)
            knee=(a-1,side*6,max(1,z-1))
            rig.line((a+shift,side*4,z),knee);rig.line(knee,foot)
            rig.line((foot[0]-.5,foot[1],foot[2]),foot,SLATE[1])
    rig.dome((-3+shift,0,z),(10 if loaded else 9,7,5 if loaded else 7),SLATE,12,3)
    # A separate pitched head and lowered lance visibly establish the diagonal.
    rig.dome((6+shift,0,z+1),(4,3.5,3),SLATE,8,2)
    horn_z=z+(2 if loaded else 4)
    rig.poly([(8+shift,-1,horn_z),(12+shift,0,horn_z+(1 if loaded else 5)),(8+shift,1,horn_z)],cp[-2],normal=(.3,0,.95))
    rig.line((8+shift,-1,horn_z),(12+shift,0,horn_z+(1 if loaded else 5)))
    for side in (-1,1):
        eye=(8+shift,side*2.7,z+3);rig.line(eye,(eye[0]+1,eye[1],eye[2]),cp[0] if anim=="dazed" else cp[-2])
        rig.line((9+shift,side*2,z),(11+shift,side*3,z-1))
    if variant==0:
        for a,b,h,w in ((-4,-2,8,2),(-1,0,11,2.5),(2,2,7,2)):
            rig.crystal(a+shift,b,z+4,h-(3 if loaded else 0),w,cp)
    elif variant==1:
        # A broken hollow quartz bowl, with independent far/near broken rims.
        shell=IVORY[:];height=6 if loaded else 9
        cavity=[(-3+shift+5.3*math.cos(k*math.tau/12),3.8*math.sin(k*math.tau/12),z+height-.5) for k in range(12)]
        rig.poly(cavity,cp[0],normal=(0,0,1),outline=False)
        outer=[];inner=[];bottom=[]
        for k in range(10):
            t=k*math.tau/10;h=height+(1 if k in (1,4,7) else -1 if k in (2,6) else 0)
            outer.append((-3+shift+7*math.cos(t),5*math.sin(t),z+h))
            inner.append((-3+shift+5.3*math.cos(t),3.8*math.sin(t),z+height-.75))
            bottom.append((-3+shift+7*math.cos(t),5*math.sin(t),z+height-2))
        for k in range(10):
            q=(k+1)%10
            rig.poly([bottom[k],bottom[q],outer[q],outer[k]],shell[3 if k>=5 else 1],outline=False)
            rig.poly([outer[k],outer[q],inner[q],inner[k]],shell[4 if k>=5 else 2],outline=False)
            rig.line(outer[k],outer[q],shell[4])
        # An exposed violet fracture belongs inside the hollow, not above it.
        rig.line((-6+shift,0,z+height-.3),(1+shift,0,z+height-.3),cp[2])
        rig.crystal(-5+shift,-2,z+height-1,3 if loaded else 5,1.8,shell)
    else:
        # Ribbed fossil valve: the cleft and radial ribs share the body bearing.
        height=6 if loaded else 9
        rig.dome((-3+shift,0,z+2),(9,6.5,height),IVORY,12,4)
        cleft=[(a+shift,0,z+2+height*math.sqrt(max(.01,1-((a+3)/9)**2))+1.5) for a in range(-10,5)]
        for p,q in zip(cleft,cleft[1:]):rig.line(p,q,cp[-2])
    im,nm=rig.render()
    if variant==2:
        # The fossil's three continuous painted folds follow the native valve
        # surface. They are pigmentation on its geometric normal field, never
        # little floating depth lines that can break into isolated speckles.
        mask=Image.new("1",im.size);draw=ImageDraw.Draw(mask)
        for k in range(8):
            phi=k*math.tau/8;u,v=math.cos(phi),math.sin(phi)
            if rig.world((u,v,0))[1]<.3:continue
            pts=[rig.project((-3+shift+9*u*math.sin(t*math.pi/20),6.5*v*math.sin(t*math.pi/20),
                              z+2+height*math.cos(t*math.pi/20)+.3)) for t in range(11)]
            draw.line(pts,fill=1,width=1)
        original=im.copy()
        for y in range(im.height):
            for x in range(im.width):
                if mask.getpixel((x,y)) and original.getpixel((x,y)) in IVORY:
                    im.putpixel((x,y),IVORY[1])
                    if x>0 and original.getpixel((x-1,y)) in IVORY and not mask.getpixel((x-1,y)):
                        im.putpixel((x-1,y),IVORY[4])
    return im,nm


def wisp(name,direction,anim,i,source):
    variant=0 if name=="wisp" else int(name[-1]);rig=Anatomy((16,24),(8,15),direction,.55)
    fx,fy=DIRS[direction];charge=anim=="windup";recoil=anim=="attack";hurt=anim=="hurt"
    cp=energy(QUARTZ,anim,i);z=5
    if recoil:
        cp=([QUARTZ[1],QUARTZ[2],QUARTZ[3],QUARTZ[4],QUARTZ[4]] if i==0 else
            QUARTZ if i<3 else [QUARTZ[0],QUARTZ[0],QUARTZ[1],QUARTZ[2],QUARTZ[3]])
    fold=(min(1,i/3) if charge else (.7 if i==0 else .3) if hurt else (-.15,0,.15,.05)[i] if recoil else 0)
    if variant==1:
        # Wings are separate hinged surfaces. A side view hides its far wing;
        # diagonal views foreshorten each lobe independently around the heart.
        span=5.5-fold*2.5;top=9.5+(1 if i%2 else 0)
        for side in (-1,1):
            outer=side*span
            pts=[(-1,side*1,5),(-2,outer,top),(1,side*(span-.5),8),(2,side*(span-.8),2),(0,side*1,2)]
            rig.poly(pts,cp[1],outline=False);rig.poly([pts[0],pts[1],pts[2]],cp[3],normal=(-.3,side*.4,.85),outline=False)
            rig.line((-1,side*1,5),(-2,outer,top),cp[2])
            rig.line((0,side*.7,8),(.3,side*2,12),cp[2])
            rig.line((-1,side*1,2),(-3+i*.5 if recoil else -2,side*2,-2),cp[1])
    elif variant==2:
        # Lantern planes surround the same heart; front/back uprights switch
        # correctly as the housing turns instead of mirroring a flat front.
        for side in (-1,1):
            rig.line((-2,side*3,4),(-2,side*3,10),cp[1])
            rig.line((2,side*3,4),(2,side*3,10),cp[3])
            rig.line((-2,side*3,10),(2,side*3,10),cp[3])
            rig.line((-2,side*3,4),(2,side*3,4),cp[2])
            rig.line((0,side*3,10),(-1,side*5,11.5),cp[-2])
            tail=-3+i*.5 if recoil else -1
            rig.line((-1,side*1,3),(-2,side*2,0),cp[1]);rig.line((-2,side*2,0),(tail,side*1,-3),cp[2])
    # The flame is a local volume, not the direction of the orbit velocity.
    # Aim rotates this body in place; the source remains in its native housing.
    width=2.5 if charge else (4,3.5,3,3)[i] if recoil else (2 if i==0 else 2.7) if hurt else 3
    rig.dome((0,0,4),(width,2.5,4 if charge else 2 if hurt and i==0 else 3),cp,8,2)
    if variant==0:
        lean=(2 if recoil else -1 if charge else 1)
        tip=(9 if i==0 else 11) if hurt else 12+(i%2)
        rig.poly([(-3,-1,5),(-2,1,5),(lean,0,tip),(1,0,7)],cp[2])
        rig.poly([(-2,0,4),(-4,0,0),(-2,0,-2),(0,0,1)],cp[1])
    # Reuse the exact tiny authored core color sequence, mounted in newly
    # projected anatomy rather than copying/rotating an entire character.
    core_colors=[source.getpixel((x,y)) for y in range(8,16) for x in range(4,11)
                 if source.getpixel((x,y))[3] and source.getpixel((x,y))[:3] in {(205,195,219),(226,216,233)}]
    core=core_colors[len(core_colors)//2] if core_colors else cp[-2]
    rig.line((1,0,6),(1,0,5),core,width=2)
    # Face gap/outlet moves onto the front plane; its back is intentionally
    # occluded when facing north. All headings preserve the luminous source.
    if fy>-.5:rig.line((2.4,-.6,5),(2.4,.6,5),OUTLINE)
    if hurt:rig.line((-1,0,4),(0,0,3),cp[0])
    return rig.render()


def snapshot():
    source=ART/"source";source.mkdir(parents=True,exist_ok=True)
    for name in NAMES:
        for folder,suffix in (("sprites",".png"),("sprites",".json"),("normals",".png")):
            dest=source/folder/(name+suffix);dest.parent.mkdir(parents=True,exist_ok=True)
            if not dest.exists():dest.write_bytes((ROOT/"assets"/folder/(name+suffix)).read_bytes())


def pack(name,out):
    base=ART/"source";old=Image.open(base/"sprites"/f"{name}.png").convert("RGBA")
    oldn=Image.open(base/"normals"/f"{name}.png").convert("RGBA")
    meta=json.loads((base/"sprites"/f"{name}.json").read_text());result=copy.deepcopy(meta)
    beet=name.startswith("crystal");cw,ch=meta["cell"];oldrows=old.height//ch
    bases=("idle","move","windup","attack","hurt","dazed") if beet else ("idle","windup","attack","hurt")
    headings=("up_right","down_right","down_left","up_left") if beet else tuple(DIRS)
    atlas=Image.new("RGBA",(old.width,(oldrows+len(bases)*len(headings))*ch));normal=Image.new("RGBA",atlas.size)
    atlas.paste(old,(0,0));normal.paste(oldn,(0,0));row=oldrows;frames=[]
    for direction in headings:
        for anim in bases:
            source_anim=meta["anims"][anim+"_right" if beet else anim]
            a=copy.deepcopy(source_anim);a["row"]=row
            result["anims"][f"{anim}_{direction}"]=a
            for i in range(a["frames"]):
                src=old.crop((i*cw,source_anim["row"]*ch,(i+1)*cw,(source_anim["row"]+1)*ch))
                im,nm=beetle(name,direction,anim,i) if beet else wisp(name,direction,anim,i,src)
                assert im.getbbox(),(name,direction,anim,i)
                # Keep a full native gutter; meshes are authored to fit, never cropped.
                assert not any(im.getpixel((x,0))[3] for x in range(cw)),(name,direction,anim,i,"top")
                assert not any(im.getpixel((x,ch-1))[3] for x in range(cw)),(name,direction,anim,i,"bottom")
                assert not any(im.getpixel((x,y))[3] for x in (0,cw-1) for y in range(ch)),(name,direction,anim,i,"side")
                vectors=np.asarray(nm,dtype=float)[:,:,:3]/127.5-1
                mask=np.asarray(im)[:,:,3]>0;length=np.linalg.norm(vectors[mask],axis=1)
                facing=vectors[:,:,2]+(.4 if beet else .55)*vectors[:,:,1]
                assert np.all((length>.99)&(length<1.01)),(name,direction,anim,i,"normal unit length")
                assert np.min(facing[mask])>-.02,(name,direction,anim,i,"normal faces actual camera")
                atlas.paste(im,(i*cw,row*ch));normal.paste(nm,(i*cw,row*ch))
                frames.append({"direction":direction,"animation":anim,"frame":i,"row":row,"bbox":im.getbbox(),"opaque":sum(p[3]>0 for p in im.get_flattened_data()),"normal_alpha_registered":im.getchannel("A").tobytes()==nm.getchannel("A").tobytes()})
            row+=1
    # Quantized native geometry can yield the exact same drawing at adjacent
    # poses. A held drawing owns one held lighting field; legacy-first matching
    # also preserves the approved source normals whenever its ink is identical.
    canonical={};deduplicated=[]
    for rr in range(row):
        for cc in range(old.width//cw):
            box=(cc*cw,rr*ch,(cc+1)*cw,(rr+1)*ch);cell=atlas.crop(box)
            if not cell.getbbox():continue
            key=cell.tobytes();norm=normal.crop(box)
            if key in canonical and rr>=oldrows:
                held=canonical[key]
                if norm.tobytes()!=held.tobytes():deduplicated.append([rr,cc])
                normal.paste(held,(cc*cw,rr*ch))
            else:canonical.setdefault(key,norm)
    data=np.asarray(normal,dtype=float);opaque=np.asarray(atlas)[:,:,3]>0
    novel=opaque.copy();novel[:oldrows*ch]=False
    vectors=data[:,:,:3]/127.5-1;length=np.linalg.norm(vectors[novel],axis=1)
    assert np.all((length>.99)&(length<1.01)),(name,"canonical normal unit length")
    assert np.min((vectors[:,:,2]+(.4 if beet else .55)*vectors[:,:,1])[novel])>-.02,(name,"canonical normals actual camera")
    result["eightway"]={"order":list(DIRS),"native_anatomy":True,"preserved_source_rows":oldrows,
                         "normal_source":"native articulated geometry; exact same raster as albedo; legacy-first exact-art canonical fields",
                         "directionless_effects":["death particles","floating tail","projectile continuous heading"] if not beet else ["crystal debris physics"]}
    result["notes"]+=" Eight-way extension: native projected articulated anatomy, unchanged original rows, exact original per-state pose timing and anchors. Source receipts and reproducible geometry tools: art/eightway/glasswater and tools/eightway/glasswater.py."
    for folder in ("sprites","normals"):(out/folder).mkdir(parents=True,exist_ok=True)
    atlas.save(out/"sprites"/f"{name}.png");normal.save(out/"normals"/f"{name}.png")
    (out/"sprites"/f"{name}.json").write_text(json.dumps(result,indent=2)+"\n")
    assert atlas.crop((0,0,*old.size)).tobytes()==old.tobytes()
    assert normal.crop((0,0,*oldn.size)).tobytes()==oldn.tobytes()
    return {"name":name,"source_png_sha256":sha(base/"sprites"/f"{name}.png"),"staged_png_sha256":sha(out/"sprites"/f"{name}.png"),"old_rows_preserved":oldrows,"canonical_normal_cells":deduplicated,"new_frames":frames,"all_directions":list(DIRS),"animation_bases":list(bases)}


def evidence(name,out):
    im=Image.open(out/"sprites"/f"{name}.png").convert("RGBA");meta=json.loads((out/"sprites"/f"{name}.json").read_text());cw,ch=meta["cell"]
    beet=name.startswith("crystal");bases=("idle","move","windup","attack","hurt","dazed") if beet else ("idle","windup","attack","hurt")
    def resolve(base,direction):
        key=base+"_"+direction
        if key in meta["anims"]:return meta["anims"][key],False
        assert beet and direction=="left";return meta["anims"][base+"_right"],True
    def cell(base,direction,frame):
        a,flip=resolve(base,direction);p=im.crop((frame*cw,a["row"]*ch,(frame+1)*cw,(a["row"]+1)*ch))
        return ImageOps.mirror(p) if flip else p
    sheet=Image.new("RGB",(95+8*6*cw,20+len(bases)*(ch+13)),(31,35,44));draw=ImageDraw.Draw(sheet)
    for j,d in enumerate(DIRS):draw.text((95+j*6*cw,3),d,fill="white")
    for y,base in enumerate(bases):
        yy=20+y*(ch+13);draw.text((1,yy),base,fill=(211,211,211))
        for j,d in enumerate(DIRS):
            a,_=resolve(base,d)
            for f in range(a["frames"]):
                p=cell(base,d,f);sheet.paste(p,(95+(j*6+f)*cw,yy),p)
    audit=out/"audit";audit.mkdir(exist_ok=True)
    sheet.save(audit/f"{name}-allframes-native.png");sheet.resize((sheet.width*3,sheet.height*3),Image.Resampling.NEAREST).save(audit/f"{name}-allframes-3x.png")
    animation=[]
    for d in DIRS:
        for t in range(24):
            frame=Image.new("RGB",(180,128),(31,35,44));dr=ImageDraw.Draw(frame);dr.text((4,3),name+" "+d,fill="white")
            for j,b in enumerate(bases):
                a,_=resolve(b,d);p=cell(b,d,(t//4)%a["frames"]);x=8+(j%3)*55;y=28+(j//3)*48
                dr.line((x,y+ch-3,x+cw,y+ch-3),fill=(40,46,56));frame.paste(p,(x,y),p)
            animation.append(frame.resize((540,384),Image.Resampling.NEAREST))
    animation[0].save(audit/f"{name}-direction-sweep.gif",save_all=True,append_images=animation[1:],duration=60,loop=0,optimize=False)


def main():
    p=argparse.ArgumentParser();p.add_argument("--out",required=True,type=Path);a=p.parse_args();snapshot();a.out.mkdir(parents=True,exist_ok=True)
    reports=[]
    for name in NAMES:
        reports.append(pack(name,a.out));evidence(name,a.out)
    manifest={"contract":"N, NE, E, SE, S, SW, W, NW; exact diagonal anatomy; stable runtime visual selection independent of gameplay aim","actors":reports,"total_new_frames":sum(len(r["new_frames"]) for r in reports)}
    (a.out/"coverage.json").write_text(json.dumps(manifest,indent=2)+"\n")
    (ART/"coverage.json").write_text(json.dumps(manifest,indent=2)+"\n")
    print(json.dumps({"actors":len(reports),"new_frames":manifest["total_new_frames"],"staged":str(a.out)}))


if __name__=="__main__":main()
