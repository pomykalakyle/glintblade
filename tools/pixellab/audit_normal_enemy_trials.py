#!/usr/bin/env python3
"""Audit actual normal_enemy_trials frames; never synthesizes shaded pixels."""
from pathlib import Path
import argparse
import hashlib
import json

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument("study", type=Path)
args = parser.parse_args()
study = args.study.resolve()
manifest = json.loads((study / "manifest.json").read_text())
timeline = json.loads((study / "timeline.json").read_text())
assert manifest["fps"] == manifest["simulation_hz"] == 60
assert manifest["native_pixels"] == [384,336]
assert manifest["integer_scale"] == 1 and manifest["retimed"] is False
assert manifest["light_mode"] == "fixed", "Use the fixed-light study for temporal auditing"
assert len(manifest["cases"]) == 18
assert len(timeline) == manifest["frame_count"]
cases = manifest["cases"]
frames = []
for tick,event in enumerate(timeline):
    assert event["tick"] == tick
    path = study / "frames" / event["frame"]
    im = Image.open(path).convert("RGBA")
    assert im.size == (384,336)
    frames.append(np.asarray(im))


def authored_frame(durations,tick):
    tick %= sum(durations)
    for index,d in enumerate(durations):
        if tick < d:
            return index
        tick -= d
    raise AssertionError("invalid pose clock")


reports = []
for index,case in enumerate(cases):
    source = ROOT / "assets/sprites" / (case["sheet"]+".png")
    meta = json.loads(source.with_suffix(".json").read_text())
    anim = meta["anims"][case["animation"]]
    cw,ch = meta["cell"]
    durations = anim.get("durations") or [max(1,int(60/anim.get("fps",8)+.5))]*anim["frames"]
    assert durations == case["durations_60hz"]
    assert anim["frames"] == case["frames"]
    original = Image.open(source).convert("RGBA")
    poses = [np.asarray(original.crop((p*cw,anim["row"]*ch,(p+1)*cw,(anim["row"]+1)*ch))) for p in range(anim["frames"])]
    tx,ty,tw,th = case["crop_on_board"]
    # Constants are the frozen example's source Frame geometry. Every body
    # pixel must fit the native tile without cropping or scale conversion.
    ox = 192-case["anchor"][0]-128
    oy = 124-int(case["elevation"])-case["anchor"][1]-84
    for pose in poses:
        yy,xx = np.where(pose[...,3]>=128)
        assert np.all((xx+ox>=0)&(xx+ox<tw)&(yy+oy>=7)&(yy+oy<th))
    emit_colors = [int(x.lstrip('#'),16) for x in meta.get("emissive",[])]
    seen = {}
    repeats = transitions = 0
    deltas = []
    for tick,event in enumerate(timeline):
        pose_index = authored_frame(durations,tick)
        assert event["pose_indices"][index] == pose_index
        # Remove only the diagnostic UI header. Actual body, glow, ground and
        # alpha-shadow pixels remain byte-for-byte renderer output.
        tile = frames[tick][ty+7:ty+th,tx:tx+tw]
        pose_key = poses[pose_index].tobytes()
        if pose_key in seen:
            assert np.array_equal(tile,seen[pose_key]), (case["sheet"],"held/exact-source pose changes under a fixed light",tick)
            repeats += 1
        else:
            seen[pose_key] = tile.copy()
        if tick==0 or pose_index==timeline[tick-1]["pose_indices"][index]:
            continue
        previous = timeline[tick-1]["pose_indices"][index]
        aa,ab = poses[previous],poses[pose_index]
        same = np.all(aa==ab,axis=2)&(aa[...,3]>=128)
        keys = (aa[...,0].astype(np.uint32)<<16)|(aa[...,1].astype(np.uint32)<<8)|aa[...,2]
        same &= ~np.isin(keys,emit_colors)
        interior = same.copy()
        interior[1:] &= same[:-1]
        interior[:-1] &= same[1:]
        interior[:,1:] &= same[:,:-1]
        interior[:,:-1] &= same[:,1:]
        yy,xx = np.where(interior)
        if len(xx)>=8:
            before = frames[tick-1][ty+oy+yy,tx+ox+xx,:3].astype(np.int16)
            after = frames[tick][ty+oy+yy,tx+ox+xx,:3].astype(np.int16)
            delta = np.max(np.abs(after-before),axis=1)
            deltas.append({"tick":tick,"poses":[previous,pose_index],"unchanged_nonemissive_interior_pixels":len(xx),
                           "source_changed_fraction":1-float(np.all(aa==ab,axis=2)[aa[...,3]>=128].mean()),
                           "mean_rgb8_change":float(delta.mean()),"p95_rgb8_change":float(np.percentile(delta,95)),
                           "max_rgb8_change":int(delta.max())})
        transitions += 1
    reports.append({"sheet":case["sheet"],"animation":case["animation"],"distinct_authored_poses_seen":len(set(e["pose_indices"][index] for e in timeline)),
                    "held_or_exact_source_pose_repeats_checked":repeats,"pose_transitions_checked":transitions,"transitions":deltas})

review = study / "review"
review.mkdir(exist_ok=True)
for species in dict.fromkeys(c["species"] for c in cases):
    group = [i for i,c in enumerate(cases) if c["species"]==species]
    base = cases[group[0]]
    durs = base["durations_60hz"]
    times = [durs[0],sum(durs[:2])] if species=="thorn_imp" else [0,durs[0]] if species=="ember_hound" else [0,sum(durs[:2])]
    times = [t for t in times if t<len(frames)]
    board = Image.new("RGB",(720,225*len(times)),(29,31,39))
    draw = ImageDraw.Draw(board)
    for row,tick in enumerate(times):
        for column,i in enumerate(group):
            case = cases[i]
            tx,ty,_,_ = case["crop_on_board"]
            # Exact source-renderer crop, integer nearest-neighbor enlargement.
            crop = Image.fromarray(frames[tick][ty+7:ty+48,tx+40:tx+88]).resize((240,205),Image.Resampling.NEAREST)
            board.paste(crop,(column*240,row*225+20))
            label = case["sheet"]+" t"+str(tick)+" f"+str(timeline[tick]["pose_indices"][i])
            draw.text((column*240+3,row*225+3),label,fill=(218,223,230))
    board.save(review/(species+"-fixed-5x.png"))

result = {"study":str(study),"renderer_binary":manifest.get("renderer_binary"),"frame_count":len(frames),
          "native_pixels":manifest["native_pixels"],"fps":60,"retimed":False,
          "case_count":len(reports),"all_original_duration_clocks_match":True,"all_opaque_source_pixels_inside_native_crop":True,
          "held_or_exact_source_pose_repeats_checked":sum(r["held_or_exact_source_pose_repeats_checked"] for r in reports),
          "checks":"Actual rendered pixels. Larger authored pose deformations can legitimately change surface response; RGB deltas are reported for visual review, not automatically classified as defects.",
          "manifest_sha256":hashlib.sha256((study/"manifest.json").read_bytes()).hexdigest(),"cases":reports}
(study/"motion-audit.json").write_text(json.dumps(result,indent=2)+"\n")
print(json.dumps({k:v for k,v in result.items() if k not in {'cases','checks'}},indent=2))
