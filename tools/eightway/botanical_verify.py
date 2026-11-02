#!/usr/bin/env python3
"""Independent offline botanical actor contract verification (no art mutation)."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

ROOT=Path(__file__).resolve().parents[2]
ART=ROOT/"art/eightway/botanical"
NAMES=("sproutling","sproutling_v1","sproutling_v2","thorn_imp","thorn_imp_v1","thorn_imp_v2")
BASES=("idle","move","windup","attack","hurt")
DIRECTIONS=("up","up_right","right","down_right","down","down_left","left","up_left")


def verify(asset_root):
    result={"actors":[],"gates": ["unchanged original rows and metadata", "complete eight-way state coverage", "matching skin rows/clocks", "native hard alpha and transparent cell margins", "unit OpenGL normals", "identical-pose normal stability", "exact hop support height", "anchor-centered anatomical reflection", "held seed material registration"]}
    for name in NAMES:
        path=asset_root/"sprites"/f"{name}.png";normal_path=asset_root/"normals"/f"{name}.png"
        meta=json.loads(path.with_suffix(".json").read_text());oldmeta=json.loads((ART/f"{name}-original-sprites.json").read_text())
        art=Image.open(path).convert("RGBA");norm=Image.open(normal_path).convert("RGBA");old=Image.open(ART/f"{name}-original-sprites.png").convert("RGBA");oldnorm=Image.open(ART/f"{name}-original-normals.png").convert("RGBA")
        assert art.size==norm.size
        assert art.crop((0,0,*old.size)).tobytes()==old.tobytes(),name
        assert norm.crop((0,0,*oldnorm.size)).tobytes()==oldnorm.tobytes(),name
        for k,a in oldmeta["anims"].items():assert a==meta["anims"][k],(name,k)
        w,h=meta["cell"];anchor=meta["anchor"];canonical={};count=0
        array=np.asarray(art);narray=np.asarray(norm);mask=array[...,3]>0
        assert set(np.unique(array[...,3]))<={0,255}
        assert np.array_equal(array[...,3],narray[...,3]),name
        vectors=narray[...,:3].astype(float)/127.5-1;lengths=np.linalg.norm(vectors[mask],axis=1)
        assert lengths.min()>.98 and lengths.max()<1.02,(name,lengths.min(),lengths.max())
        assert np.all(vectors[...,2][mask]>=0),name
        for base in BASES:
            for direction in DIRECTIONS:
                a=meta["anims"].get(f"{base}_{'right' if direction=='left' else direction}")
                assert a is not None,(name,base,direction)
                assert a["durations"]==meta["anims"][f"{base}_right"]["durations"],(name,base,direction)
        for anim,a in meta["anims"].items():
            for i in range(a["frames"]):
                box=(i*w,a["row"]*h,(i+1)*w,(a["row"]+1)*h);cell=art.crop(box);n=norm.crop(box)
                assert cell.getbbox(),(name,anim,i)
                key=cell.tobytes()
                if key in canonical:assert n.tobytes()==canonical[key],(name,anim,i,"normal hold discontinuity")
                canonical[key]=n.tobytes()
                if a["row"]<15:continue
                count+=1;b=cell.getbbox();assert 0<b[0]<b[2]<w and 0<b[1]<b[3]<h,(name,anim,i,"cell contour clipped")
                base=next(b for b in BASES if anim.startswith(b+"_"));source=oldmeta["anims"][base+"_right"]
                original=old.crop((i*w,source["row"]*h,(i+1)*w,(source["row"]+1)*h))
                assert cell.getbbox()[3]==original.getbbox()[3],(name,anim,i,"hop/support height changed")
                if name.startswith("thorn_imp") and (base=="windup" and i>0 or base=="attack" and i==0):
                    side="_side_" in anim;up=anim.endswith(("up_right","up_left"));source_direction="right" if side else "up" if up else "down"
                    oldanim=oldmeta["anims"][f"{base}_{source_direction}"];held=old.crop((i*w,oldanim["row"]*h,(i+1)*w,(oldanim["row"]+1)*h))
                    seed_colors={(163,163,143,255),(189,187,161,255)}
                    expected={(x,y,p) for y in range(h) for x in range(w) if (p:=held.getpixel((x,y))) in seed_colors}
                    if side and anim.endswith(("up_left","down_left")):expected={(2*anchor[0]-x,y,p) for x,y,p in expected}
                    actual={(x,y,p) for y in range(h) for x in range(w) if (p:=cell.getpixel((x,y))) in seed_colors}
                    assert actual==expected,(name,anim,i,"seed material or release-hand centroid changed")
                if anim.endswith(("up_left","down_left")) and not(name.startswith("thorn_imp") and base in ("windup","attack") and "_side_" not in anim):
                    right=meta["anims"][anim.replace("_left","_right")];donor=art.crop((i*w,right["row"]*h,(i+1)*w,(right["row"]+1)*h))
                    for y in range(h):
                        for x in range(w):
                            rx=2*anchor[0]-x
                            assert cell.getpixel((x,y))==(donor.getpixel((rx,y)) if 0<=rx<w else (0,0,0,0)),(name,anim,i,"mirror anchor drift")
        base_name=name.split("_v")[0];base_meta=json.loads((asset_root/"sprites"/f"{base_name}.json").read_text())
        assert meta["anims"]==base_meta["anims"],(name,"appearance row/clock mismatch")
        result["actors"].append({"actor":name,"new_frames":count,"directions":list(DIRECTIONS),"animations":list(BASES),"old_albedo_and_normal_bytes_preserved":True,"normal_vector_length":[float(lengths.min()),float(lengths.max())],"albedo_sha256":hashlib.sha256(path.read_bytes()).hexdigest(),"normal_sha256":hashlib.sha256(normal_path.read_bytes()).hexdigest()})
    return result


if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--assets",required=True,type=Path);parser.add_argument("--out",required=True,type=Path);args=parser.parse_args()
    report=verify(args.assets);args.out.write_text(json.dumps(report,indent=2)+"\n");print(json.dumps({"actors":len(report["actors"]),"new_frames":sum(a["new_frames"] for a in report["actors"]),"gates":len(report["gates"]),"result":"pass"}))
