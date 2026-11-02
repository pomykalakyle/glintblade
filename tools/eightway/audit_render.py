#!/usr/bin/env python3
"""Join strict asset coverage to production-render evidence and make motion clips.

No visual pass is inferred from a numerical metric. The report distinguishes raw
native-pose grids, real Player/Enemy updates, and human-reviewed evidence.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw


def make_clip(paths, out, fps, scale=3):
    if not paths:
        return None
    ffmpeg=shutil.which("ffmpeg")
    if ffmpeg:
        process=subprocess.Popen([ffmpeg,"-hide_banner","-loglevel","error","-y","-f","rawvideo","-pixel_format","rgb24",
                                  "-video_size","384x216","-framerate",str(fps),"-i","-","-vf",f"scale=iw*{scale}:ih*{scale}:flags=neighbor",
                                  "-c:v","libx264","-preset","fast","-crf","17","-pix_fmt","yuv420p","-r","60","-movflags","+faststart",str(out)],stdin=subprocess.PIPE)
        try:
            for path in paths:
                process.stdin.write(Image.open(path).convert("RGB").tobytes())
        finally:
            process.stdin.close()
        if process.wait():
            raise RuntimeError("ffmpeg failed")
        return str(out)
    frames=[]
    for path in paths:
        image=Image.open(path).convert("RGB")
        frames.append(image.resize((image.width*scale,image.height*scale),Image.Resampling.NEAREST))
    out=out.with_suffix(".gif")
    frames[0].save(out,save_all=True,append_images=frames[1:],duration=round(1000/fps),loop=0,disposal=2)
    return str(out)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage",type=Path,required=True)
    parser.add_argument("--renders",nargs="+",type=Path,required=True)
    parser.add_argument("--out",type=Path,required=True)
    parser.add_argument("--movies",action="store_true")
    parser.add_argument("--binary",type=Path)
    args=parser.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    coverage=json.loads(args.coverage.read_text())
    records=[]
    receipts=[]
    for path in args.renders:
        data=json.loads(path.read_text())
        records.extend(data["captures"])
        receipts.append({"path":str(path.resolve()),"sha256":hashlib.sha256(path.read_bytes()).hexdigest(),"mode":data["mode"],"conditions":data.get("conditions")})
    grids=collections.defaultdict(list)
    observations=collections.defaultdict(list)
    motion=collections.defaultdict(list)
    errors=list(coverage["errors"])
    for record in records:
        path=Path(record["path"])
        if not path.is_file():
            errors.append(f"missing rendered evidence {path}")
        if record["kind"]=="production-renderer-native-pose-grid":
            for lookup in record["lookups"]:
                for frame in range(record["first_frame"],min(lookup["frames"],record["first_frame"]+record["frame_rows"])):
                    grids[(record["atlas"],record["base"],lookup["direction"],frame)].append({"path":str(path),"region":record["region"],"animation":lookup["animation"],"flip":lookup["flip"],"native_frame_row":frame-record["first_frame"]})
        else:
            atlas=record.get("atlas","hero")
            if record["kind"]!="moving-production-sword-light":
                pose=record.get("pose") or {}
                matches=pose.get("exact_matches",[])
                identities=[m for m in matches if m["match"]=="immutable-cell-identity"]
                for match in identities or matches:
                    observations[(atlas,match["animation"],match["frame"])].append({"path":str(path),"tick":record.get("tick",record.get("local_tick")),"state":record["state"],"flip":pose.get("flip"),"region":record.get("region")})
            group=((atlas,record["region"],"sword-sweep") if record["kind"]=="moving-production-sword-light" else
                   ("hero",record["region"],record["direction_case"],record.get("sequence","walk-stop-turn-combo-charge-spin-dash-hurt")) if atlas=="hero" else (atlas,))
            motion[group].append(record)
    results=[]
    for actor in coverage["actors"]:
        entries=[]
        for base,directions in actor["animations"].items():
            for direction,anim in directions.items():
                for frame in anim["frames"]:
                    key=(actor["atlas"],base,direction,frame["frame"])
                    evidence=grids[key]
                    if not evidence:
                        errors.append(f"no production-renderer native frame {actor['atlas']}/{base}_{direction}[{frame['frame']}]")
                    for image in evidence:
                        if image["animation"]!=anim["animation"] or image["flip"]!=anim["flip"]:
                            errors.append(f"runtime lookup mismatch {key}: {image['animation']} flip={image['flip']}")
                    entries.append({"base":base,"direction":direction,"frame":frame["frame"],"asset_animation":anim["animation"],"asset_row":anim["row"],
                                    "flip":anim["flip"],"duration":frame["duration"],"native_render_evidence":evidence,
                                    "actual_update_observations":observations[(actor["atlas"],anim["animation"],frame["frame"])][:8]})
        results.append({"atlas":actor["atlas"],"family":actor["family"],"appearance":actor["appearance"],"pose_count":len(entries),"poses":entries})
    clips=[]
    for group,frames in motion.items():
        tick="local_tick" if group[0]=="hero" else "tick"
        frames.sort(key=lambda r:r[tick])
        deltas=[b[tick]-a[tick] for a,b in zip(frames,frames[1:])]
        cadence=min((d for d in deltas if d>0),default=1)
        path=args.out/("-".join(group)+".mp4")
        clip=make_clip([r["path"] for r in frames],path,60/cadence) if args.movies else None
        sheet=Image.new("RGB",(384*4,216*3),(30,31,40))
        draw=ImageDraw.Draw(sheet)
        indices=sorted(set(round(i*(len(frames)-1)/11) for i in range(12)))
        for i,index in enumerate(indices):
            image=Image.open(frames[index]["path"]).convert("RGB")
            sheet.paste(image,((i%4)*384,(i//4)*216))
            draw.text(((i%4)*384+4,(i//4)*216+4),f"tick{frames[index][tick]} {frames[index]['state']}",fill="white",stroke_width=1,stroke_fill="black")
        filmstrip=args.out/("-".join(group)+"-filmstrip.png")
        sheet.save(filmstrip)
        clips.append({"group":group,"frames":len(frames),"capture_tick_cadence":cadence,"native_frame_rate":60/cadence,"movie":clip,"filmstrip":str(filmstrip.resolve()),
                      "states":sorted(set(r["state"].split(':')[0] for r in frames))})
    report={"schema":1,"passed":not errors,"errors":errors,"atlases":results,"render_receipts":receipts,"motion":clips,
            "binary":None if not args.binary else {"path":str(args.binary.resolve()),"sha256":hashlib.sha256(args.binary.read_bytes()).hexdigest()},
            "review_limit":"Native frame evidence is complete only if every requested entry has a grid. Actual update observations are reported separately; numerical coverage is not visual approval."}
    (args.out/"render-coverage.json").write_text(json.dumps(report,indent=2)+"\n")
    gallery=["<!doctype html><meta charset=utf-8><title>Eightway actual render audit</title><style>body{background:#1e1f28;color:#ddd;font:14px system-ui}img,video{image-rendering:pixelated;max-width:100%}video{width:1152px}a{color:#bcf}</style>",f"<h1>Actual actor renders: {len(records)} frames, {len(errors)} coverage gaps</h1>"]
    for clip in clips:
        gallery.append(f"<h2>{' / '.join(clip['group'])}</h2><p>{clip['frames']} frames; {clip['native_frame_rate']:g} native captures per second.</p>")
        if clip["movie"]:
            path=Path(clip["movie"]).name
            gallery.append(f"<video controls loop muted preload='metadata' src='{path}'></video>")
        gallery.append(f"<p><img src='{Path(clip['filmstrip']).name}'></p>")
    (args.out/"index.html").write_text("\n".join(gallery))
    print(json.dumps({"captures":len(records),"atlases":len(results),"passed":not errors,"errors":errors[:10],"report":str(args.out/"render-coverage.json")}))
    raise SystemExit(bool(errors))


if __name__=="__main__":
    main()
