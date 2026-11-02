#!/usr/bin/env python3
"""Native-resolution PixelLab normal trials and reproducible atlas packing.

PixelLab RGB is data (OpenGL +Y up), never sRGB-decoded or filtered.  The
albedo alpha remains authoritative; normals cannot grow or shrink a sprite.
Generation receipts are kept separately from this offline-only tool.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import io
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from PIL import ImageOps

ROOT = Path(__file__).resolve().parents[2]
TRIALS = ROOT / "out/pixellab/round7/normal-trials"
ARCHIVE = ROOT / "art/pixellab/2026-11-01-round7/normals"
ENEMIES = ("sproutling", "thorn_imp", "crystal_beetle", "wisp", "ember_hound", "spitting_totem")
FACET_SHEETS = {"glasswater_crystals_big", "glasswater_crystals_native",
                "glasswater_crystals_small", "warden_crystal"}


def prepare() -> None:
    """Prepare a native-pixel queue. Large sheets split on cell rows, never scale."""
    input_dir = ARCHIVE / "inputs"
    input_dir.mkdir(parents=True, exist_ok=True)
    queue = []
    for source in sorted((ROOT / "assets/sprites").glob("*.png")):
        name = source.stem
        if name.startswith(("font", "hud", "logo")) or name in {"seed", "wisp_bolt", "ember_glob"}:
            continue
        meta_path = source.with_suffix(".json")
        if not meta_path.exists():
            continue
        meta = json.loads(meta_path.read_text())
        cell_w, cell_h = meta["cell"]
        image = Image.open(source).convert("RGBA")
        character = name == "hero" or name.startswith(ENEMIES + ("warden_",))
        floor = any(k in name for k in ("_tiles", "_floor", "_materials", "_walltop", "_terrace")) or name == "cinder_lava_surface"
        kind = "character" if character else "topdown" if floor else "object"
        params = {"kind": kind, "mode": "lite", "convention": "opengl",
                  "strength": 0.35 if floor else 0.65,
                  "outline_rounding": 0.0 if floor else 0.7,
                  "part_shapes": 0.0 if floor else 0.9,
                  "surface_detail": 0.08 if floor else 0.1,
                  "detail_softness": 1.5 if floor else 1.0}
        if not character and not floor:
            params["mode"] = "pro"
        if name in {"bloomwild_ruins", "glasswater_crystals_big", "ember_hound"}:
            params["mode"] = "pro"
        if name in {"bloomwild_ruins", "glasswater_crystals_big"}:
            params["surface_detail"] = 0.15
        if name == "bloomwild_trees":
            # Two original layers share one volume field. Source column 1 is
            # retained too, even where current animation metadata uses one frame.
            pairs = image.height // (2 * cell_h)
            cols = image.width // cell_w
            composite = Image.new("RGBA", (pairs * cell_w, cols * cell_h))
            mappings = []
            for c in range(cols):
                for pair in range(pairs):
                    trunk = image.crop((c*cell_w, pair*2*cell_h, (c+1)*cell_w, (pair*2+1)*cell_h))
                    canopy = image.crop((c*cell_w, (pair*2+1)*cell_h, (c+1)*cell_w, (pair*2+2)*cell_h))
                    trunk.alpha_composite(canopy)
                    composite.paste(trunk, (pair*cell_w, c*cell_h))
                    for layer in range(2):
                        mappings.append({"from": [pair*cell_w, c*cell_h, cell_w, cell_h],
                                         "to": [c*cell_w, (pair*2+layer)*cell_h, cell_w, cell_h]})
            step = (512 // cell_w) * cell_w
            for start_x in range(0, composite.width, step):
                end_x = min(start_x+step, composite.width)
                dest = input_dir / f"{name}-composite-part-{start_x}.png"
                composite.crop((start_x,0,end_x,composite.height)).save(dest)
                subset = []
                for mapping in mappings:
                    sx,sy,w,h = mapping["from"]
                    if start_x <= sx and sx+w <= end_x:
                        subset.append({"from": [sx-start_x,sy,w,h], "to": mapping["to"]})
                queue.append({"name": name, "id": dest.stem, "input": str(dest.relative_to(ROOT)),
                              "source": str(source.relative_to(ROOT)), "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                              "size": image.size, "cell": meta["cell"], "params": params, "mappings": subset})
            continue
        # Crop only when required by the normal tool's 512px side limit. Cell-aligned
        # boundaries keep pose anatomy complete; giant single-cell liquid fields
        # are planar and have outline/part shapes disabled.
        chunk_w = max(1, 512 // cell_w) * cell_w if cell_w <= 512 else 512
        chunk_h = max(1, 512 // cell_h) * cell_h if cell_h <= 512 else 512
        for y in range(0, image.height, chunk_h):
            for x in range(0, image.width, chunk_w):
                w, h = min(chunk_w, image.width-x), min(chunk_h, image.height-y)
                suffix = "" if (w,h) == image.size else f"-part-{x}-{y}"
                dest = input_dir / f"{name}{suffix}.png"
                image.crop((x,y,x+w,y+h)).save(dest)
                queue.append({"name": name, "id": name+suffix, "input": str(dest.relative_to(ROOT)),
                              "source": str(source.relative_to(ROOT)), "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                              "size": image.size, "cell": meta["cell"], "params": params,
                              "mappings": [{"from": [0,0,w,h], "to": [x,y,w,h]}]})
    (ARCHIVE / "queue.json").write_text(json.dumps(queue, indent=2) + "\n")
    print(f"Prepared {len(queue)} native inputs for {len(set(q['name'] for q in queue))} sheets")


def facet_normals(albedo: Image.Image, normal: Image.Image, min_area: int = 8) -> Image.Image:
    """Planar authored color patches receive their mean *geometry* normal.

    Unlike luminance-to-height conversion this never invents ridges at dark eyes
    or highlights. A constant-color crystal face is a plane; disconnected equal
    colors remain independent. Its perimeter retains the generated bevel.
    """
    art = np.asarray(albedo.convert("RGBA"))
    data = np.asarray(normal.convert("RGBA")).copy()
    source_n = vectors(normal)
    visited = np.zeros(art.shape[:2], dtype=bool)
    h, w = visited.shape
    for y in range(h):
        for x in range(w):
            if visited[y,x] or art[y,x,3] == 0:
                continue
            visited[y,x] = True
            color = art[y,x,:3]
            todo = deque([(x,y)])
            patch = []
            while todo:
                xx, yy = todo.popleft()
                patch.append((xx,yy))
                for nx,ny in ((xx-1,yy),(xx+1,yy),(xx,yy-1),(xx,yy+1)):
                    if 0<=nx<w and 0<=ny<h and not visited[ny,nx] and art[ny,nx,3]>0 and np.array_equal(art[ny,nx,:3],color):
                        visited[ny,nx] = True
                        todo.append((nx,ny))
            if len(patch) < min_area:
                continue
            mean = np.mean([source_n[yy,xx] for xx,yy in patch],axis=0)
            mean /= max(float(np.linalg.norm(mean)),1e-6)
            encoded = np.rint((mean+1)*127.5).astype(np.uint8)
            for xx,yy in patch:
                if all(0<=nx<w and 0<=ny<h and art[ny,nx,3]>0 for nx,ny in ((xx-1,yy),(xx+1,yy),(xx,yy-1),(xx,yy+1))):
                    data[yy,xx,:3] = encoded
    return Image.fromarray(data)


def stabilize_near_poses(albedo: Image.Image, normal: Image.Image, meta: dict) -> tuple[Image.Image, int, int]:
    """Keep unchanged anatomy coherent without freezing authored deformation.

    If ≥90% of a pose is the same native art under a ±2px translation, its
    matching pixels inherit reference geometry. For larger authored movement,
    reuse only exact 3x3 material patches, with ≥55% whole-pose correspondence.
    New/moved parts and their contour keep their own generated normals. This
    fixes an eye pulse changing the shape of a motionless hound's whole skull.
    """
    art = np.asarray(albedo.convert("RGBA"))
    data = np.asarray(normal.convert("RGBA")).copy()
    cw,ch = meta["cell"]
    stabilized = copied = 0
    seen_rows = set()
    for anim in meta.get("anims",{}).values():
        row,frames = anim.get("row",0),anim.get("frames",1)
        if frames <= 1 or row in seen_rows:
            continue
        seen_rows.add(row)
        y = row*ch
        reference = art[y:y+ch,:cw]
        reference_n = data[y:y+ch,:cw].copy()
        if not (reference[...,3]>0).any():
            continue
        for frame in range(1,frames):
            x = frame*cw
            current = art[y:y+ch,x:x+cw]
            best = None
            for dy in range(-2,3):
                for dx in range(-2,3):
                    shifted = np.zeros_like(reference)
                    xs,xe,ys,ye = max(dx,0),min(cw,cw+dx),max(dy,0),min(ch,ch+dy)
                    shifted[ys:ye,xs:xe] = reference[ys-dy:ye-dy,xs-dx:xe-dx]
                    equal = np.all(current==shifted,axis=2)&(current[...,3]>0)
                    score = float(equal.sum()) / max(int((current[...,3]>0).sum()),int((shifted[...,3]>0).sum()),1)
                    rank = (score,-abs(dx)-abs(dy))
                    if best is None or rank > best[0]:
                        best = (rank,dx,dy,equal)
            if best[0][0] < 0.55:
                continue
            _,dx,dy,equal = best
            if best[0][0] < 0.9:
                # Local context must match too. A coincident single pixel in a
                # newly bent limb is insufficient evidence of rigid anatomy.
                padded = np.pad(equal,1,constant_values=False)
                local = equal.copy()
                for yy in range(3):
                    for xx in range(3):
                        local &= padded[yy:yy+ch,xx:xx+cw]
                equal = local
            if not equal.any():
                continue
            shifted_n = np.zeros_like(reference_n)
            xs,xe,ys,ye = max(dx,0),min(cw,cw+dx),max(dy,0),min(ch,ch+dy)
            shifted_n[ys:ye,xs:xe] = reference_n[ys-dy:ye-dy,xs-dx:xe-dx]
            target = data[y:y+ch,x:x+cw]
            target[equal] = shifted_n[equal]
            stabilized += 1
            copied += int(equal.sum())
    return Image.fromarray(data),stabilized,copied


def pack() -> None:
    queue = json.loads((ARCHIVE / "queue.json").read_text())
    report = []
    out = ROOT / "assets/normals"
    out.mkdir(exist_ok=True)
    for name in sorted(set(q["name"] for q in queue)):
        parts = [q for q in queue if q["name"] == name]
        if any(not (ARCHIVE / "raw" / (q["id"]+".png")).exists() for q in parts):
            continue
        source = ROOT / parts[0]["source"]
        assert hashlib.sha256(source.read_bytes()).hexdigest() == parts[0]["source_sha256"], f"source changed: {name}"
        albedo = Image.open(source).convert("RGBA")
        normal = Image.new("RGBA", albedo.size, (128,128,255,0))
        for part in parts:
            native = Image.open(ARCHIVE / "raw" / (part["id"]+".png")).convert("RGBA")
            expected = Image.open(ROOT / part["input"]).size
            assert native.size == expected, (part["id"], native.size, expected)
            for mapping in part["mappings"]:
                sx,sy,w,h = mapping["from"]
                dx,dy,_,_ = mapping["to"]
                normal.paste(native.crop((sx,sy,sx+w,sy+h)), (dx,dy))
        normal.putalpha(albedo.getchannel("A"))
        if name in FACET_SHEETS:
            normal = facet_normals(albedo, normal)
        meta = json.loads(source.with_suffix(".json").read_text())
        normal,stabilized_poses,transported_pixels = stabilize_near_poses(albedo,normal,meta)
        # The exact same authored pose must have the exact same normal field.
        # This prevents global AI planning noise from breathing during a hold.
        cw,ch = parts[0]["cell"]
        canonical = {}
        duplicate_cells = 0
        mirrored_cells = 0
        for y in range(0,albedo.height,ch):
            for x in range(0,albedo.width,cw):
                box = (x,y,x+cw,y+ch)
                cell = albedo.crop(box)
                key = cell.tobytes()
                if key in canonical:
                    normal.paste(canonical[key],(x,y))
                    duplicate_cells += 1
                else:
                    mirror_key = ImageOps.mirror(cell).tobytes()
                    if mirror_key in canonical:
                        mirrored = np.asarray(ImageOps.mirror(canonical[mirror_key])).copy()
                        mirrored[...,0] = 255-mirrored[...,0]
                        normal.paste(Image.fromarray(mirrored),(x,y))
                        mirrored_cells += 1
                    canonical[key] = normal.crop(box)
        dest = out / f"{name}.png"
        encoded = io.BytesIO()
        normal.save(encoded, format="PNG")
        blob = encoded.getvalue()
        if not dest.exists() or dest.read_bytes() != blob:
            temporary = dest.with_suffix(".pending.png")
            temporary.write_bytes(blob)
            temporary.replace(dest)
        a,n = np.asarray(albedo),np.asarray(normal)
        assert np.array_equal(a[...,3],n[...,3]), name
        opaque = a[...,3] >= 128
        assert np.all(n[...,2][opaque] >= 128), f"backfacing generated normal: {name}"
        report.append({"sheet": name,"source": parts[0]["source"],"source_sha256": parts[0]["source_sha256"],
                       "normal": str(dest.relative_to(ROOT)),"normal_sha256": hashlib.sha256(dest.read_bytes()).hexdigest(),
                       "size": albedo.size,"opaque_pixels": int(opaque.sum()),"alpha_mismatches": 0,
                       "duplicate_cells_canonicalized": duplicate_cells,"mirrored_cells_canonicalized":mirrored_cells,
                       "faceted_authored_color_patches": name in FACET_SHEETS,
                       "near_poses_stabilized":stabilized_poses,"normal_pixels_transported":transported_pixels,
                       "generation_parts": [p["id"] for p in parts]})
    (ARCHIVE / "selected.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps({"selected_sheets":len(report),"opaque_pixels":sum(r["opaque_pixels"] for r in report),"sheets":[r["sheet"] for r in report]},indent=2))


def audit() -> None:
    """Audit actual selected PNG data, independently of generation receipts."""
    reports = []
    for path in sorted((ROOT / "assets/normals").glob("*.png")):
        source = ROOT / "assets/sprites" / path.name
        art = Image.open(source).convert("RGBA")
        normal = Image.open(path).convert("RGBA")
        assert art.size == normal.size, path.name
        a,n = np.asarray(art),np.asarray(normal)
        assert np.array_equal(a[...,3],n[...,3]), path.name
        opaque = a[...,3] >= 128
        encoded = n[...,:3].astype(np.float32) / 127.5 - 1
        lengths = np.linalg.norm(encoded[opaque],axis=1)
        assert np.all(lengths > 0.98) and np.all(lengths < 1.02), (path.name,float(lengths.min()),float(lengths.max()))
        assert np.all(encoded[...,2][opaque] >= 0), path.name
        meta = json.loads(source.with_suffix(".json").read_text())
        cw,ch = meta["cell"]
        source_cells = {}
        duplicate = 0
        mirrors = 0
        for y in range(0,art.height,ch):
            for x in range(0,art.width,cw):
                box = (x,y,x+cw,y+ch)
                key = art.crop(box).tobytes()
                candidate = normal.crop(box).tobytes()
                if key in source_cells:
                    assert candidate == source_cells[key], (path.name,"duplicate pose normal drift",x,y)
                    duplicate += 1
                else:
                    mirror_key = ImageOps.mirror(art.crop(box)).tobytes()
                    if mirror_key in source_cells:
                        reference = np.asarray(Image.frombytes("RGBA",(cw,ch),source_cells[mirror_key]))
                        reflected = np.flip(reference,axis=1).copy()
                        reflected[...,0] = 255-reflected[...,0]
                        assert candidate == reflected.tobytes(), (path.name,"mirrored pose normal drift",x,y)
                        mirrors += 1
                    source_cells[key] = candidate
        nv = vectors(normal)
        light = np.array([0.72,0.34,0.61],np.float32)
        light /= np.linalg.norm(light)
        mirrored = np.flip(nv,axis=1).copy()
        mirrored[...,0] *= -1
        mirrored_light = light.copy()
        mirrored_light[0] *= -1
        left = np.einsum("ijk,k->ij",nv,light)
        right = np.flip(np.einsum("ijk,k->ij",mirrored,mirrored_light),axis=1)
        assert np.allclose(left,right,atol=1e-6), path.name
        reports.append({"sheet":path.stem,"size":normal.size,"opaque_pixels":int(opaque.sum()),
                        "alpha_mismatches":0,"min_encoded_vector_length":float(lengths.min()),
                        "max_encoded_vector_length":float(lengths.max()),"duplicate_pose_cells_checked":duplicate,
                        "mirrored_pose_cells_checked":mirrors,
                        "mirror_light_equivariance_max_error":float(np.max(np.abs(left-right))),
                        "normal_sha256":hashlib.sha256(path.read_bytes()).hexdigest()})
    result = {"sheet_count":len(reports),"opaque_pixels":sum(r["opaque_pixels"] for r in reports),
              "duplicate_pose_cells_checked":sum(r["duplicate_pose_cells_checked"] for r in reports),"sheets":reports}
    result["mirrored_pose_cells_checked"] = sum(r["mirrored_pose_cells_checked"] for r in reports)
    (ARCHIVE / "asset-audit.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({k:v for k,v in result.items() if k!="sheets"},indent=2))


def coherence() -> None:
    """Measure generated versus selected idle stability on unchanged interiors."""
    queue = json.loads((ARCHIVE / "queue.json").read_text())
    lights = np.array([[-.75,.4,.6],[.75,.4,.6]],np.float32)
    lights /= np.linalg.norm(lights,axis=1,keepdims=True)
    reports = []
    for path in sorted((ROOT / "assets/normals").glob("*.png")):
        name = path.stem
        if not (name == "hero" or name.startswith(ENEMIES)):
            continue
        parts = [q for q in queue if q["name"] == name]
        source = ROOT / "assets/sprites" / path.name
        meta = json.loads(source.with_suffix(".json").read_text())
        cw,ch = meta["cell"]
        art = Image.open(source).convert("RGBA")
        raw = Image.new("RGBA",art.size)
        for part in parts:
            native = Image.open(ARCHIVE / "raw" / (part["id"]+".png")).convert("RGBA")
            for mapping in part["mappings"]:
                sx,sy,w,h = mapping["from"]
                dx,dy,_,_ = mapping["to"]
                raw.paste(native.crop((sx,sy,sx+w,sy+h)),(dx,dy))
        selected = Image.open(path).convert("RGBA")
        for an,anim in meta.get("anims",{}).items():
            if not an.startswith("idle"):
                continue
            y = anim["row"]*ch
            for f in range(anim["frames"]-1):
                b0,b1 = (f*cw,y,(f+1)*cw,y+ch),((f+1)*cw,y,(f+2)*cw,y+ch)
                a,b = np.asarray(art.crop(b0)),np.asarray(art.crop(b1))
                same = np.all(a==b,axis=2)&(a[...,3]>0)
                inside = same.copy()
                inside[1:] &= same[:-1]
                inside[:-1] &= same[1:]
                inside[:,1:] &= same[:,:-1]
                inside[:,:-1] &= same[:,1:]
                if inside.sum() < 8:
                    continue
                row = {"sheet":name,"anim":an,"frames":[f,f+1],"unchanged_interior_pixels":int(inside.sum()),
                       "source_changed_fraction":1-float(same.sum())/max(int((a[...,3]>0).sum()),int((b[...,3]>0).sum()),1)}
                for label,normal in [("raw",raw),("selected",selected)]:
                    na,nb = vectors(normal.crop(b0)),vectors(normal.crop(b1))
                    delta = np.max(np.abs(np.einsum("ijk,lk->ijl",na-nb,lights)),axis=2)[inside]
                    row[label+"_mean_diffuse_change"] = float(delta.mean())
                    row[label+"_p95_diffuse_change"] = float(np.percentile(delta,95))
                reports.append(row)
    reports.sort(key=lambda r:r["raw_p95_diffuse_change"],reverse=True)
    near_static = [r for r in reports if r["source_changed_fraction"] <= .1]
    result = {"meaning":"N dot L difference under fixed left/right lights on unchanged source-interior pixels. Larger authored pose changes can legitimately alter surface orientation.",
              "comparisons":len(reports),"near_static_comparisons":len(near_static),
              "near_static_max_p95_raw":max(r["raw_p95_diffuse_change"] for r in near_static),
              "near_static_max_p95_selected":max(r["selected_p95_diffuse_change"] for r in near_static),"rows":reports}
    (ARCHIVE / "idle-coherence.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({k:v for k,v in result.items() if k!="rows"},indent=2))
    print(json.dumps(reports[:6],indent=2))


def vectors(image: Image.Image) -> np.ndarray:
    rgb = np.asarray(image.convert("RGBA"), dtype=np.float32)[..., :3]
    normal = rgb / 127.5 - 1.0
    return normal / np.maximum(np.linalg.norm(normal, axis=2, keepdims=True), 1e-6)


def relight(albedo: Image.Image, normal: Image.Image, angle: float) -> Image.Image:
    """Diagnostic Lambert light; only direction changes, geometry stays fixed."""
    src = np.asarray(albedo.convert("RGBA"), dtype=np.float32)
    light = np.array([math.sin(angle) * 0.82, 0.32, math.cos(angle) * 0.82])
    light /= np.linalg.norm(light)
    diffuse = np.maximum(np.einsum("ijk,k->ij", vectors(normal), light), 0)
    gain = 0.38 + 0.85 * diffuse
    src[..., :3] = np.clip(src[..., :3] * gain[..., None], 0, 255)
    return Image.fromarray(np.rint(src).astype(np.uint8))


def compare() -> None:
    names = ["bloomwild_ruins", "glasswater_crystals_big", "hero",
             "ember_hound", "spitting_totem", "bloomwild_canopy",
             "bloomwild_composite"]
    rows = []
    metrics = []
    for name in names:
        source = TRIALS / f"{name}-cell.png"
        if not source.exists():
            continue
        albedo = Image.open(source).convert("RGBA")
        for mode in ["lite", "pro"]:
            path = TRIALS / f"{name}-{mode}.png"
            if not path.exists():
                continue
            normal = Image.open(path).convert("RGBA")
            assert normal.size == albedo.size, (name, normal.size, albedo.size)
            alpha = np.asarray(albedo)[..., 3]
            generated_alpha = np.asarray(normal)[..., 3]
            n = vectors(normal)
            metrics.append({"name": name, "mode": mode, "size": albedo.size,
                            "opaque_pixels": int((alpha > 0).sum()),
                            "alpha_mismatches": int((alpha != generated_alpha).sum()),
                            "left_mean_x": float(n[:, :n.shape[1]//2, 0][alpha[:, :n.shape[1]//2] > 0].mean()),
                            "right_mean_x": float(n[:, n.shape[1]//2:, 0][alpha[:, n.shape[1]//2:] > 0].mean()),
                            "min_z": float(n[..., 2][alpha > 0].min()),
                            "normal_sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
            # Exact source alpha removes any normal-generator fringe.
            normal.putalpha(albedo.getchannel("A"))
            panels = [albedo, normal] + [relight(albedo, normal, a) for a in [-1.1, 0, 1.1]]
            rows.append((f"{name} / {mode}", panels))
    scale = 3
    cell_w, cell_h = 242, 306
    board = Image.new("RGB", (5 * cell_w, len(rows) * cell_h), (29, 31, 39))
    draw = ImageDraw.Draw(board)
    headers = ["Original albedo", "Normal map (+Y up)", "Light left", "Light front", "Light right"]
    for row, (label, panels) in enumerate(rows):
        for col, panel in enumerate(panels):
            draw.text((col * cell_w + 5, row * cell_h + 4), label if col == 0 else headers[col], fill=(214, 218, 226))
            p = panel.resize((panel.width * scale, panel.height * scale), Image.Resampling.NEAREST)
            board.paste(p, (col * cell_w + (cell_w - p.width) // 2, row * cell_h + 20), p)
    board.save(TRIALS / "normal-trial-contact.png")
    (TRIALS / "normal-trial-metrics.json").write_text(json.dumps(metrics, indent=2) + "\n")
    print(json.dumps(metrics, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["compare", "prepare", "pack", "audit", "coherence"])
    args = parser.parse_args()
    if args.command == "compare":
        compare()
    elif args.command == "prepare":
        prepare()
    elif args.command == "pack":
        pack()
    elif args.command == "audit":
        audit()
    elif args.command == "coherence":
        coherence()
