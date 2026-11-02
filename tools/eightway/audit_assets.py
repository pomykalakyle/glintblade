#!/usr/bin/env python3
"""Audit every named actor pose, export native/nearest-neighbor sheets and motion.

The manifest records exact source rows and deliberate same-diagonal mirroring.
It never accepts a cardinal/unsuffixed fallback for a directional actor.
Run against an immutable copied asset root before a visual sign-off.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
DIRECTIONS = ("up", "up_right", "right", "down_right", "down", "down_left", "left", "up_left")
MIRROR = {"left": "right", "up_left": "up_right", "down_left": "down_right"}
BACK = (35, 36, 48, 255)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def timing(anim):
    if "durations" in anim:
        return anim["durations"]
    return [max(1, int(math.floor(60 / anim.get("fps", 8) + 0.5)))] * anim["frames"]


def resolve(anims, base, direction):
    name = f"{base}_{direction}"
    if name in anims:
        return name, False
    if direction in MIRROR:
        name = f"{base}_{MIRROR[direction]}"
        if name in anims:
            return name, True
    raise KeyError(f"missing exact directional pose: {base}_{direction}")


def frame_image(image, size, anim, index, flip=False):
    w, h = size
    cell = image.crop((index*w, anim["row"]*h, (index+1)*w, (anim["row"]+1)*h))
    return cell.transpose(Image.Transpose.FLIP_LEFT_RIGHT) if flip else cell


def frame_at(durations, tick):
    tick %= sum(durations)
    for frame, duration in enumerate(durations):
        if tick < duration:
            return frame
        tick -= duration
    raise AssertionError("invalid animation clock")


def save_motion(path, frames):
    # GIF stores centiseconds: error diffusion gives 50ms per 3 ticks instead of
    # flooring 17ms to 10ms. MP4 captures retain the exact production 60Hz clock.
    durations = [(20, 10, 20)[i % 3] for i in range(len(frames))]
    frames[0].save(path, save_all=True, append_images=frames[1:], duration=durations,
                   loop=0, disposal=2, optimize=False)


def audit_actor(actor, assets, out, errors, baseline=True):
    name = actor["atlas"]
    paths = {kind: assets/kind/f"{name}.{ext}" for kind, ext in
             (("sprites", "json"), ("normals", "png"))}
    png = assets/"sprites"/f"{name}.png"
    spec = json.loads(paths["sprites"].read_text())
    image = Image.open(png).convert("RGBA")
    normal = Image.open(paths["normals"]).convert("RGBA")
    size = tuple(spec["cell"])
    w, h = size
    target = out/name
    target.mkdir(parents=True, exist_ok=True)
    result = {"atlas": name, "family": actor["family"], "appearance": actor["appearance"],
              "files": {"albedo": str(png), "normal": str(paths["normals"]), "metadata": str(paths["sprites"])},
              "sha256": {"albedo": digest(png), "normal": digest(paths["normals"]), "metadata": digest(paths["sprites"])},
              "dimensions": list(image.size), "cell": list(size), "anchor": spec["anchor"], "animations": {}}
    if image.size != normal.size:
        errors.append(f"{name}: normal dimensions {normal.size} != albedo {image.size}")
    elif ImageChops.difference(image.getchannel("A"), normal.getchannel("A")).getbbox():
        errors.append(f"{name}: normal alpha is not registered with albedo")
    if set(image.getchannel("A").tobytes()) - {0, 255}:
        errors.append(f"{name}: semi-transparent albedo pixels")
    if tuple(spec["anchor"]) != tuple(actor["anchor"]):
        errors.append(f"{name}: changed sheet ground anchor")
    before = actor["source_before"]["animations"]
    if baseline:
        for key, old in before.items():
            current = spec["anims"].get(key)
            if current is None:
                errors.append(f"{name}: deleted original animation {key}")
            elif any(current.get(k) != old.get(k) for k in ("row", "frames", "durations", "fps", "anchor", "release_frame", "release_offset", "scarf_anchor", "blade_tip")):
                errors.append(f"{name}: changed original pose clock/row/anchor {key}")
    requirements = {base: DIRECTIONS for base in actor["required_directional_bases"]}
    requirements.update(actor.get("conditional_directional_bases", {}))
    for base, requested_directions in requirements.items():
        records = {}
        for direction in requested_directions:
            try:
                key, flip = resolve(spec["anims"], base, direction)
            except KeyError as exc:
                errors.append(f"{name}: {exc.args[0]}")
                continue
            anim = spec["anims"][key]
            durations = timing(anim)
            if len(durations) != anim["frames"] or any(not isinstance(d, int) or d <= 0 for d in durations):
                errors.append(f"{name}/{key}: invalid frame durations")
                continue
            if anim["frames"]*w > image.width or (anim["row"]+1)*h > image.height:
                errors.append(f"{name}/{key}: frame rectangle outside sheet")
                continue
            anchor = anim.get("anchor", spec["anchor"])
            frames = []
            hashes = []
            for index in range(anim["frames"]):
                cell = frame_image(image, size, anim, index, flip)
                alpha = cell.getchannel("A")
                bbox = alpha.getbbox()
                if bbox is None:
                    errors.append(f"{name}/{key}[{index}]: empty native pose")
                # Feet are source pixels around the registered anchor; held swords
                # or tails may extend below it, so this records rather than guesses.
                support = []
                ax = w-1-anchor[0] if flip else anchor[0]
                for x in range(max(0, ax-8), min(w, ax+9)):
                    ys = [y for y in range(max(0, anchor[1]-4), min(h, anchor[1]+5)) if alpha.getpixel((x,y))]
                    if ys:
                        support.append([x-ax, min(ys)-anchor[1], max(ys)-anchor[1]])
                for point_key in ("scarf_anchor", "blade_tip", "release_offset", "mouth_offset"):
                    points = anim.get(point_key)
                    if isinstance(points, list) and points and isinstance(points[0], list) and len(points) != anim["frames"]:
                        errors.append(f"{name}/{key}: {point_key} has {len(points)} keys for {anim['frames']} frames")
                hashes.append(hashlib.sha256(cell.tobytes()).hexdigest())
                frames.append({"frame": index, "duration": durations[index], "bbox": bbox,
                               "opaque_pixels": sum(v != 0 for v in alpha.tobytes()), "support_columns": support})
            records[direction] = {"animation": key, "row": anim["row"], "flip": flip, "anchor": anchor,
                                  "frames": frames, "durations": durations, "pixel_hashes": hashes}
        result["animations"][base] = records
        if len(records) != len(requested_directions):
            continue
        count = max(len(r["frames"]) for r in records.values())
        gutter = 3
        board = Image.new("RGBA", (60 + count*(w+gutter), 14+len(requested_directions)*(h+gutter)), BACK)
        draw = ImageDraw.Draw(board)
        draw.text((2, 1), name+" / "+base, fill=(220, 217, 218))
        for row, direction in enumerate(requested_directions):
            record = records[direction]
            anim = spec["anims"][record["animation"]]
            y = 14+row*(h+gutter)
            draw.text((1,y+max(0,h//2-4)), direction, fill=(190,187,193))
            for index in range(len(record["frames"])):
                cell = frame_image(image, size, anim, index, record["flip"])
                board.alpha_composite(cell, (60+index*(w+gutter),y))
        native = target/f"{base}-native.png"
        enlarged = target/f"{base}-4x.png"
        board.save(native)
        board.resize((board.width*4,board.height*4),Image.Resampling.NEAREST).save(enlarged)
        # All8directions loop together on their exact native pose clocks.
        total = max(sum(r["durations"]) for r in records.values())
        motion = []
        for tick in range(min(total,180)):
            canvas = Image.new("RGBA", (len(requested_directions)*(w+6),h+12), BACK)
            for column,direction in enumerate(requested_directions):
                record=records[direction]
                anim=spec["anims"][record["animation"]]
                index=frame_at(record["durations"],tick)
                canvas.alpha_composite(frame_image(image,size,anim,index,record["flip"]),(column*(w+6)+3,4))
            motion.append(canvas.resize((canvas.width*4,canvas.height*4),Image.Resampling.NEAREST).convert("RGB"))
        loop = target/f"{base}-all-directions.gif"
        save_motion(loop,motion)
        # A single actor turns through8sectors while continuing its animation clock.
        sweep = []
        for tick in range(96):
            direction=requested_directions[(tick//12)%len(requested_directions)]
            record=records[direction]
            anim=spec["anims"][record["animation"]]
            canvas=Image.new("RGBA",(w+16,h+16),BACK)
            canvas.alpha_composite(frame_image(image,size,anim,frame_at(record["durations"],tick),record["flip"]),(8,8))
            sweep.append(canvas.resize((canvas.width*6,canvas.height*6),Image.Resampling.NEAREST).convert("RGB"))
        sweep_path=target/f"{base}-direction-sweep.gif"
        save_motion(sweep_path,sweep)
        result.setdefault("evidence",{})[base]={"native":str(native),"enlarged":str(enlarged),"animation":str(loop),"direction_sweep":str(sweep_path)}
        # Exact cardinal duplicates do not count as a new diagonal body pose.
        for diagonal, adjacent in (("up_right",("up","right")),("down_right",("down","right")),("down_left",("down","left")),("up_left",("up","left"))):
            if diagonal in records and any(cardinal in records and records[diagonal]["pixel_hashes"] == records[cardinal]["pixel_hashes"] for cardinal in adjacent):
                errors.append(f"{name}/{base}_{diagonal}: identical whole animation to cardinal")
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets",type=Path,default=ROOT/"assets")
    parser.add_argument("--inventory",type=Path,default=ROOT/"art/eightway/audit/inventory.json")
    parser.add_argument("--out",type=Path,required=True)
    parser.add_argument("--only",help="comma-separated atlas names")
    parser.add_argument("--no-baseline-check",action="store_true")
    args=parser.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    inventory=json.loads(args.inventory.read_text())
    selected=set(args.only.split(',')) if args.only else None
    errors=[]
    results=[]
    for actor in inventory["actors"]:
        if selected and actor["atlas"] not in selected:
            continue
        try:
            results.append(audit_actor(actor,args.assets.resolve(),args.out.resolve(),errors,not args.no_baseline_check))
        except (FileNotFoundError,ValueError,KeyError) as exc:
            errors.append(f"{actor['atlas']}: {exc}")
    report={"schema":1,"inventory":str(args.inventory.resolve()),"assets":str(args.assets.resolve()),
            "actor_atlas_count":len(results),"errors":errors,"passed":not errors,"actors":results,
            "review_limit":"Structural/image-data validation only. Native/enlarged animation and actual regional draws still require direct visual inspection."}
    (args.out/"coverage.json").write_text(json.dumps(report,indent=2)+"\n")
    gallery=["<!doctype html><meta charset=utf-8><title>Eight direction actor audit</title><style>body{background:#20212b;color:#ddd;font:14px system-ui}img{image-rendering:pixelated;max-width:100%}section{margin:25px 0}a{color:#bdf}summary{cursor:pointer}</style>",f"<h1>{len(results)} actor atlases</h1><p>{len(errors)} contract errors. Human visual review remains required.</p>"]
    for result in results:
        gallery.append(f"<h2>{result['atlas']}</h2>")
        for base,evidence in result.get("evidence",{}).items():
            rel=lambda k: str(Path(evidence[k]).relative_to(args.out.resolve()))
            gallery.append(f"<details><summary>{base}</summary><img src='{rel('enlarged')}'><p><img src='{rel('animation')}'><img src='{rel('direction_sweep')}'></p><a href='{rel('native')}'>native sheet</a></details>")
    (args.out/"index.html").write_text("\n".join(gallery))
    print(json.dumps({"atlases":len(results),"passed":not errors,"error_count":len(errors),"report":str(args.out/"coverage.json"),"first_errors":errors[:12]}))
    raise SystemExit(bool(errors))


if __name__=="__main__":
    main()
