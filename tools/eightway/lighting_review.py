#!/usr/bin/env python3
"""Read-only actor normal/contact diagnostics; shaded evidence comes from the game.

This checks source data independently of export receipts and writes native and
nearest-neighbor diagnostic boards. It never generates/replaces game assets.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SPECIES = ("sproutling", "thorn_imp", "crystal_beetle", "wisp", "ember_hound", "spitting_totem")
SHEETS = ["hero"] + [s + v for s in SPECIES for v in ("", "_v1", "_v2")]
DIRECTIONS = ("up", "up_right", "right", "down_right", "down", "down_left", "left", "up_left")
DIRECTION_LABELS = ("N", "NE", "E", "SE", "S", "SW", "W", "NW")
MIRROR = {"left": "right", "up_left": "up_right", "down_left": "down_right"}
# Same finite support-volume selection as current DrawOpts; gameplay radii stay untouched.
CONTACT = {"hero": (4.0, 4), "sproutling": (4.0, 3), "thorn_imp": (4.5, 3),
           "crystal_beetle": (None, 8), "wisp": (4.5, 2),
           "ember_hound": (None, 4), "spitting_totem": (None, 5)}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def base_name(name: str) -> str:
    return next((s for s in SPECIES if name == s or name.startswith(s + "_v")), name)


def split_direction(name: str) -> tuple[str, str | None]:
    for direction in sorted(DIRECTIONS, key=len, reverse=True):
        if name.endswith("_" + direction):
            return name[:-len(direction)-1], direction
    return name, None


def resolve(anims: dict, base: str, direction: str) -> tuple[str | None, bool]:
    key = f"{base}_{direction}"
    if key in anims:
        return key, False
    if direction in MIRROR and f"{base}_{MIRROR[direction]}" in anims:
        return f"{base}_{MIRROR[direction]}", True
    if not any(f"{base}_{d}" in anims for d in DIRECTIONS) and base in anims:
        return base, False
    return None, False


def support(cell: np.ndarray, emission: np.ndarray, anchor: list[int], radius: float | None, depth: int) -> dict:
    """Unprojected source support only; actual caster placement is audited in Rust renders."""
    h, w = cell.shape[:2]
    mask = (cell[..., 3] >= 128) & ~emission
    left, right = (0, w) if radius is None else (max(0, anchor[0]-int(np.ceil(radius))), min(w, anchor[0]+int(np.ceil(radius))+1))
    lower, upper = max(0, anchor[1]-14), min(h-1, anchor[1]+8)
    found = [y for y in range(lower, upper+1) if mask[y, left:right].any()]
    if found:
        bottom = max(found)
    else:
        yy, _ = np.where(cell[..., 3] >= 128)
        bottom = int(yy.max()) if len(yy) else anchor[1]
    top = max(0, bottom-depth+1)
    columns = []
    for x in range(left, right):
        yy = np.where(mask[top:bottom+1, x])[0]
        if len(yy):
            columns.append([x-anchor[0], int(yy.min()+top-anchor[1]), int(yy.max()+top+1-anchor[1])])
    return {"source_bottom_from_anchor": int(bottom-anchor[1]), "native_columns": columns,
            "support_is_nonemissive": bool(found), "radius": radius, "depth": depth, "full_width": radius is None}


def authored_support(anim: dict, frame: int, cell: np.ndarray, anchor: list[int], radius: float | None, depth: int) -> dict | None:
    """Validate optional native support spans independently of runtime parsing."""
    if "support_spans" not in anim:
        return None
    h, w = cell.shape[:2]
    all_spans = anim["support_spans"]
    if len(all_spans) != anim["frames"]:
        return {"errors": ["support_spans frame count mismatch"]}
    spans = all_spans[frame]
    if len(spans) != w:
        return {"errors": [f"support_spans columns {len(spans)} != native width {w}"]}
    errors, native = [], []
    for x, span in enumerate(spans):
        if span is None:
            continue
        if not isinstance(span, list) or len(span) != 2 or any(type(v) is not int for v in span):
            errors.append(f"invalid support span in column {x}")
        elif not 0 <= span[0] < span[1] <= h:
            errors.append(f"support outside native cell in column {x}: {span}")
        else:
            native.append([x-anchor[0], span[0]-anchor[1], span[1]-anchor[1]])
    if not native:
        errors.append("authored support has no finite native column")
    r = int(np.ceil(radius)) if radius is not None else w
    centered = [p for p in native if radius is None or -r <= p[0] <= r]
    if not centered:
        errors.append("authored support has no finite body-center column")
    bottom = max((p[2] for p in centered), default=None)
    selected = [[p[0], max(p[1], bottom-depth), p[2]] for p in centered if bottom is not None and p[2] > bottom-depth]
    return {"errors": errors, "native_columns": native,
            "source_bottom_from_anchor": max((p[2]-1 for p in native), default=None),
            "selected_source_bottom_from_anchor": bottom-1 if bottom is not None else None,
            "selected_native_columns": selected}


def inspect_sheet(name: str, assets: Path, baseline: Path | None, out: Path) -> dict:
    source, normals = assets / "sprites" / f"{name}.png", assets / "normals" / f"{name}.png"
    metadata = assets / "sprites" / f"{name}.json"
    art_image, normal_image = Image.open(source).convert("RGBA"), Image.open(normals).convert("RGBA")
    art, normal = np.asarray(art_image), np.asarray(normal_image)
    meta = json.loads(metadata.read_text())
    errors: list[str] = []
    if art.shape != normal.shape:
        return {"sheet": name, "errors": [f"atlas dimensions differ: {art.shape} vs {normal.shape}"]}
    opaque = art[..., 3] >= 128
    alpha_mismatches = int(np.count_nonzero(art[..., 3] != normal[..., 3]))
    if alpha_mismatches:
        errors.append(f"normal alpha mismatches: {alpha_mismatches}")
    if not np.isin(art[..., 3], (0, 255)).all():
        errors.append("albedo has partial alpha")
    vectors = normal[..., :3].astype(np.float32) / 127.5 - 1.0
    lengths = np.linalg.norm(vectors[opaque], axis=1)
    if len(lengths) and (lengths.min() < .98 or lengths.max() > 1.02):
        errors.append("encoded vectors are not unit length within 2%")
    negative_blue = int((normal[..., 2][opaque] < 128).sum())
    geometric_normals = bool(meta.get("eightway", {}).get("native_anatomy"))
    # Native projected geometry may expose a north-facing top facet: world Y
    # (PNG blue) can be negative while its positive world Z remains visible.
    # The older PixelLab maps are front-hemisphere tangent maps by contract.
    if negative_blue and not geometric_normals:
        errors.append("back-facing OpenGL Z normal in front-hemisphere tangent map")
    geometry_view = None
    if geometric_normals:
        ground = .4 if base_name(name) == "crystal_beetle" else .55
        first_new_y = int(meta["eightway"].get("preserved_source_rows", 0))*meta["cell"][1]
        selected = opaque.copy()
        selected[:first_new_y] = False
        facing = vectors[..., 2] + ground*vectors[..., 1]
        if selected.any():
            geometry_view = {"projection": f"screenY={ground}*Y-Z", "min_encoded_camera_dot": float(facing[selected].min()),
                             "backfacing_pixels_beyond_quantization": int((facing[selected] < -.015).sum())}
            if geometry_view["backfacing_pixels_beyond_quantization"]:
                errors.append(f"visible geometry has {geometry_view['backfacing_pixels_beyond_quantization']} normals facing away from its actual camera")
    cw, ch = meta["cell"]
    emit_keys = {int(c.lstrip("#"), 16) for c in meta.get("emissive", [])}
    rgb_keys = (art[..., 0].astype(np.uint32) << 16) | (art[..., 1].astype(np.uint32) << 8) | art[..., 2]
    emission = np.isin(rgb_keys, list(emit_keys)) & opaque
    animations = []
    seen: dict[bytes, np.ndarray] = {}
    duplicate_checks = duplicate_drift = mirror_checks = mirror_drift = 0
    held_material_transitions = []
    for anim_name, anim in meta["anims"].items():
        row, frames = anim["row"], anim["frames"]
        anchor = anim.get("anchor", meta["anchor"])
        frame_report = []
        previous_cell = previous_field = previous_emits = None
        for frame in range(frames):
            sy, sx = row*ch, frame*cw
            cell = art[sy:sy+ch, sx:sx+cw]
            field = normal[sy:sy+ch, sx:sx+cw]
            emits = emission[sy:sy+ch, sx:sx+cw]
            if cell.shape != (ch, cw, 4) or not (cell[..., 3] >= 128).any():
                errors.append(f"{anim_name} frame {frame}: empty/outside atlas")
                continue
            key = cell.tobytes()
            if key in seen:
                duplicate_checks += 1
                if not np.array_equal(field, seen[key]):
                    duplicate_drift += 1
                    errors.append(f"{anim_name} frame {frame}: identical albedo has different normals")
            else:
                reflected_key = cell[:, ::-1].tobytes()
                if reflected_key in seen:
                    mirror_checks += 1
                    expected = seen[reflected_key][:, ::-1].copy()
                    expected[..., 0] = 255-expected[..., 0]
                    shaded = (cell[..., 3] >= 128) & ~emits
                    if not np.array_equal(field[shaded], expected[shaded]):
                        mirror_drift += 1
                        errors.append(f"{anim_name} frame {frame}: mirrored albedo has different reflected normals")
                seen[key] = field.copy()
            if previous_cell is not None:
                same = np.all(cell == previous_cell, axis=2) & (cell[..., 3] >= 128) & ~emits & ~previous_emits
                interior = same.copy()
                interior[1:] &= same[:-1]
                interior[:-1] &= same[1:]
                interior[:, 1:] &= same[:, :-1]
                interior[:, :-1] &= same[:, 1:]
                count = int(interior.sum())
                if count >= 8:
                    before = previous_field[..., :3].astype(np.float32)/127.5-1
                    after = field[..., :3].astype(np.float32)/127.5-1
                    before /= np.maximum(np.linalg.norm(before, axis=2, keepdims=True), 1e-6)
                    after /= np.maximum(np.linalg.norm(after, axis=2, keepdims=True), 1e-6)
                    angles = np.degrees(np.arccos(np.clip(np.sum(before*after, axis=2)[interior], -1, 1)))
                    held_material_transitions.append({"animation": anim_name, "from_frame": frame-1, "to_frame": frame,
                                                      "held_interior_pixels": count, "mean_normal_angle_degrees": float(angles.mean()),
                                                      "p95_normal_angle_degrees": float(np.percentile(angles, 95)), "max_normal_angle_degrees": float(angles.max())})
            previous_cell, previous_field, previous_emits = cell, field, emits
            yy, xx = np.where(cell[..., 3] >= 128)
            ey, ex = np.where(emits)
            native_support = support(cell, emits, anchor, *CONTACT[base_name(name)])
            explicit_support = authored_support(anim, frame, cell, anchor, *CONTACT[base_name(name)])
            if explicit_support:
                errors.extend(f"{anim_name} frame {frame}: {error}" for error in explicit_support["errors"])
            frame_report.append({"frame": frame, "opaque_pixels": len(xx),
                                 "bounds": [int(xx.min()), int(yy.min()), int(xx.max()+1), int(yy.max()+1)],
                                 "emissive_pixels": len(ex), "support": native_support, "authored_support": explicit_support,
                                 "emissive_centroid_from_anchor": [float(ex.mean()+.5-anchor[0]), float(ey.mean()+.5-anchor[1])] if len(ex) else None})
        animations.append({"name": anim_name, "row": row, "frames": frames, "anchor": anchor, "poses": frame_report})
    bases = sorted({split_direction(n)[0] for n in meta["anims"]})
    coverage = []
    for base in bases:
        entries = {d: {"animation": resolve(meta["anims"], base, d)[0], "flip": resolve(meta["anims"], base, d)[1]} for d in DIRECTIONS}
        coverage.append({"base": base, "directionless": not any(split_direction(n) == (base, d) for n in meta["anims"] for d in DIRECTIONS),
                         "directions": entries, "missing": [d for d, item in entries.items() if item["animation"] is None]})
    preserved = None
    if baseline is not None and (baseline / "sprites" / f"{name}.png").exists():
        previous = np.asarray(Image.open(baseline / "sprites" / f"{name}.png").convert("RGBA"))
        previous_n = np.asarray(Image.open(baseline / "normals" / f"{name}.png").convert("RGBA"))
        h, w = previous.shape[:2]
        if h <= art.shape[0] and w <= art.shape[1]:
            preserved = {"previous_size": [w, h], "changed_albedo_pixels_in_old_rectangle": int(np.any(previous != art[:h, :w], axis=2).sum()),
                         "changed_normal_pixels_in_old_rectangle": int(np.any(previous_n != normal[:h, :w], axis=2).sum())}
    # Native source diagnostic: one first-frame sample per available animation.
    board = Image.new("RGBA", (175+8*(cw+2), 20+len(bases)*(2*ch+17)), (28, 31, 39, 255))
    draw = ImageDraw.Draw(board)
    for column, direction in enumerate(DIRECTION_LABELS):
        draw.text((175+column*(cw+2), 3), direction, fill=(220, 220, 222))
    for row_index, base in enumerate(bases):
        y = 20+row_index*(2*ch+17)
        draw.text((3, y+ch), base, fill=(220, 220, 222))
        for column, direction in enumerate(DIRECTIONS):
            key, flip = resolve(meta["anims"], base, direction)
            if key is None:
                continue
            anim = meta["anims"][key]
            box = (0, anim["row"]*ch, cw, (anim["row"]+1)*ch)
            a, n = art_image.crop(box), normal_image.crop(box)
            if flip:
                a = a.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
                data = np.asarray(n.transpose(Image.Transpose.FLIP_LEFT_RIGHT)).copy()
                data[..., 0] = 255-data[..., 0]
                n = Image.fromarray(data)
            x = 175+column*(cw+2)
            board.alpha_composite(a, (x, y))
            board.alpha_composite(n, (x, y+ch+2))
    board.save(out / f"{name}-source-normal-native.png")
    board.resize((board.width*4, board.height*4), Image.Resampling.NEAREST).save(out / f"{name}-source-normal-4x.png")
    return {"sheet": name, "size": list(art_image.size), "cell": [cw, ch], "anchor": meta["anchor"],
            "hashes": {"albedo": digest(source), "normal": digest(normals), "metadata": digest(metadata)},
            "opaque_pixels": int(opaque.sum()), "alpha_mismatches": alpha_mismatches,
            "encoded_vector_length": [float(lengths.min()), float(lengths.max())] if len(lengths) else None,
            "negative_blue_pixels": negative_blue, "projected_geometry_normals": geometric_normals,
            "geometry_view_basis": geometry_view,
            "duplicate_pose_checks": duplicate_checks, "duplicate_normal_drift": duplicate_drift,
            "mirrored_pose_checks": mirror_checks, "mirrored_normal_drift": mirror_drift,
            "held_material_transitions": sorted(held_material_transitions, key=lambda item: item["p95_normal_angle_degrees"], reverse=True),
            "preserved_rectangle": preserved, "coverage": coverage, "animations": animations, "errors": errors}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, default=ROOT / "assets")
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--sheets", nargs="+", default=SHEETS)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    reports = [inspect_sheet(name, args.assets, args.baseline, args.out) for name in args.sheets]
    summary = {"asset_root": str(args.assets.resolve()), "sheet_count": len(reports),
               "errors": sum(len(s["errors"]) for s in reports), "sheets": reports,
               "limits": ["Source boards are diagnostic assets, not shaded game renders.",
                          "Support columns are source coordinates; live support/caster registration needs runtime evidence.",
                          "Missing directions are reported even for intentional mixed legacy death metadata; runtime policy must be explicit."]}
    (args.out / "source-normal-contact-audit.json").write_text(json.dumps(summary, indent=2)+"\n")
    print(json.dumps({"sheets": len(reports), "errors": summary["errors"], "out": str(args.out.resolve())}))
    if summary["errors"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
