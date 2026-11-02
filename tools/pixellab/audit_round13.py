#!/usr/bin/env python3
"""Independent, offline Round13 export/evidence audit. Never builds or runs the game.

uv run --with pillow python tools/pixellab/audit_round13.py \
  --root . --baseline-root out/beauty-pass-2026-11-02/baseline/renderer/source \
  --out out/beauty-pass-2026-11-02/harness/candidate.json

Attach actual runtime evidence using --capture-manifest, --campaign-report,
--scenario-plan and --test-log. All outputs are exclusively created. Exit 1 means
a measured contract failed; exit 0 still leaves explicitly listed review gates.
PNG statistics cannot establish beauty, real collision or shadow correctness.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import fnmatch
import hashlib
import json
import math
from pathlib import Path
import re
import sys

from PIL import Image, ImageOps

SCHEMA = "glintblade.round13.audit.v1"
REGIONS = {"bloomwild", "glasswater", "cinder", "summit"}
SHARD_REGIONS = REGIONS - {"summit"}
STRICT_SHEETS = ["round13_landmarks*", "bloomwild_altar", "warden_body"]
NORMAL_SHEETS = ["round13_landmarks_garden", "round13_landmarks_distributor",
                 "round13_landmarks_press", "round13_landmarks_summit",
                 "bloomwild_altar", "warden_body"]
ROUND13_SHEETS = NORMAL_SHEETS
REQUIRED_MILESTONES = [*(f"{r}_shard" for r in sorted(SHARD_REGIONS)),
                       "warden_awake", "phase_2_damage", "phase_3_damage",
                       "phase_2_heart_open", "warden_dying", "victory"]
REVIEW_GATES = [
    "A human must judge beauty at native 1x, integer zoom, full-map and 60fps motion; no aesthetic score is inferred.",
    "Actual collision/route validity requires the complete real-input campaign plus contact probes; atlas alpha is not world collision.",
    "Foreground reveal must be exercised with Glint behind each new solid, stacked props and grouped vegetation, including dash/airborne/mirrored poses.",
    "Sky-shadow/receiver correctness needs the renderer exclusion tests and actual edge/bridge/sky frames; composited PNGs do not expose receiver masks.",
    "Warden heart/neck/crystal attachment, anatomy and later-phase readability require assembled poses and actual active attacks; fixed atlas coordinates alone are insufficient.",
    "The full-source altar extends 4px farther south than its predecessor; all opaque terrain pixels need dry SLAB receiver material and zero terrain height in the finished region.",
    "A source snapshot and a binary hash do not prove that binary was built from those inputs; preserve a controlled build receipt.",
]


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def issue(issues: list, severity: str, code: str, subject: str, detail: object) -> None:
    issues.append({"severity": severity, "code": code, "subject": subject, "detail": detail})


def matches(name: str, patterns: list[str]) -> bool:
    return any(fnmatch.fnmatchcase(name, p) for p in patterns)


def rgba(path: Path) -> Image.Image:
    with Image.open(path) as im:
        if im.format != "PNG":
            raise ValueError("Expected an actual PNG")
        return im.convert("RGBA")


def pair(value: object, positive: bool = False) -> bool:
    return (isinstance(value, list) and len(value) == 2
            and all(type(n) is int and (not positive or n > 0) for n in value))


def visible_key(im: Image.Image) -> bytes:
    # Transparent RGB is not renderer-visible and must not hide a duplicated pose.
    return bytes(channel for px in pixels(im) for channel in (px if px[3] else (0, 0, 0, 0)))


def pixels(im: Image.Image):
    return im.get_flattened_data() if hasattr(im, "get_flattened_data") else im.getdata()


def normal_metrics(art: Image.Image, normal: Image.Image, meta: dict, subject: str, issues: list) -> dict:
    result = {"size": list(normal.size), "expected_size": list(art.size)}
    if normal.size != art.size:
        issue(issues, "error", "normal_dimensions", subject, result)
        return result
    a = art.getchannel("A").tobytes()
    n = normal.getchannel("A").tobytes()
    mismatch = sum(x != y for x, y in zip(a, n))
    lengths, backfacing = [], 0
    for source_alpha, px in zip(a, pixels(normal)):
        if source_alpha != 255:
            continue
        vector = [(v / 127.5) - 1.0 for v in px[:3]]
        lengths.append(math.sqrt(sum(v * v for v in vector)))
        backfacing += vector[2] < 0
    invalid = sum(not .98 <= v <= 1.02 for v in lengths)
    result.update(alpha_mismatches=mismatch, opaque_pixels=len(lengths),
                  min_encoded_vector_length=min(lengths, default=None),
                  max_encoded_vector_length=max(lengths, default=None),
                  nonunit_vectors=invalid, backfacing_vectors=backfacing)
    for count, code in [(mismatch, "normal_alpha"), (invalid, "normal_unit_length"),
                        (backfacing, "normal_backfacing")]:
        if count:
            issue(issues, "error", code, subject, {"pixels": count})
    cw, ch = meta["cell"]
    canonical, duplicates, mirrors = {}, 0, 0
    # Check every stored cell, including unused padding. This detects row shifts
    # when identical visible poses have moved between animation states.
    for y in range(0, art.height, ch):
        for x in range(0, art.width, cw):
            box = (x, y, x + cw, y + ch)
            ac, nc = art.crop(box), normal.crop(box)
            if not ac.getchannel("A").getbbox():
                continue
            key, val = visible_key(ac), visible_key(nc)
            if key in canonical:
                duplicates += 1
                if val != canonical[key]:
                    issue(issues, "error", "duplicate_pose_normal_drift", subject, {"cell": [x // cw, y // ch]})
            else:
                mirrored = visible_key(ImageOps.mirror(ac))
                if mirrored in canonical:
                    mirrors += 1
                    ref = Image.frombytes("RGBA", (cw, ch), canonical[mirrored])
                    transformed = ImageOps.mirror(ref)
                    transformed.putdata([(255-r, g, b, a) if a else (0,0,0,0)
                                         for r,g,b,a in pixels(transformed)])
                    if val != visible_key(transformed):
                        issue(issues, "error", "mirrored_pose_normal_drift", subject, {"cell": [x // cw, y // ch]})
                canonical[key] = val
    result.update(duplicate_visible_poses_checked=duplicates, mirrored_visible_poses_checked=mirrors)
    return result


def audit_sheet(png: Path, normal_dir: Path, strict: bool, require_normal: bool,
                issues: list) -> dict:
    subject = png.stem
    result = {"sheet": subject, "png_sha256": digest(png), "strict_free_standing_gutters": strict}
    image = rgba(png)
    result["size"] = list(image.size)
    alpha = image.getchannel("A")
    hist = alpha.histogram()
    result["nonbinary_alpha_pixels"] = sum(hist[1:255])
    if result["nonbinary_alpha_pixels"]:
        issue(issues, "error", "binary_alpha", subject, result["nonbinary_alpha_pixels"])
    meta_path = png.with_suffix(".json")
    if not meta_path.is_file():
        issue(issues, "error", "missing_manifest", subject, str(meta_path))
        return result
    meta = json.loads(meta_path.read_text())
    result["json_sha256"] = digest(meta_path)
    if not isinstance(meta, dict) or not pair(meta.get("cell"), positive=True):
        issue(issues, "error", "manifest_cell", subject, meta)
        return result
    cw, ch = meta["cell"]
    anchor = meta.get("anchor", [cw // 2, ch - 1])
    result.update(cell=[cw, ch], anchor=anchor)
    if image.width % cw or image.height % ch:
        issue(issues, "error", "sheet_grid", subject, {"image": image.size, "cell": [cw, ch]})
        return result
    if not pair(anchor) or not (0 <= anchor[0] < cw and 0 <= anchor[1] < ch):
        issue(issues, "error", "manifest_anchor", subject, anchor)
        return result
    anims = meta.get("anims")
    direct_layout = (not anims and not strict and ("tiles" in meta or "glyphs" in meta))
    if direct_layout:
        anims = {}
        result["direct_layout"] = "glyphs" if "glyphs" in meta else "tiles"
        for key, coord in meta.get("tiles", {}).items():
            if not pair(coord) or not (0 <= coord[0] < image.width//cw and 0 <= coord[1] < image.height//ch):
                issue(issues, "error", "tile_cell_bounds", f"{subject}/{key}", coord)
        for key, rect in meta.get("glyphs", {}).items():
            if (not isinstance(rect, list) or len(rect) != 3 or any(type(v) is not int for v in rect)
                or not (0 <= rect[0] < image.width and 0 <= rect[1] < image.height
                        and 0 < rect[2] <= image.width-rect[0] and rect[1]+ch <= image.height)):
                issue(issues, "error", "glyph_pixel_bounds", f"{subject}/{key}", rect)
    elif not isinstance(anims, dict) or not anims:
        issue(issues, "error", "manifest_anims", subject, anims)
        return result
    if subject == "warden_body":
        if [cw, ch] != [120, 112] or anchor != [60, 109]:
            issue(issues, "error", "warden_fixed_canvas", subject, {"cell": [cw,ch], "anchor": anchor})
        missing = sorted({"idle", "awake", "damaged"} - anims.keys())
        if missing:
            issue(issues, "error", "warden_pose_keys", subject, missing)
    if subject == "bloomwild_altar":
        if [cw, ch] != [72, 56] or anchor != [36, 25]:
            issue(issues, "error", "altar_native_canvas", subject, {"cell": [cw,ch], "anchor": anchor})
        result["altar_approved_source_bbox_exclusive"] = [6,4,66,54]
    result["frames"] = frames = []
    referenced = set()
    for name, anim in sorted(anims.items()):
        label = f"{subject}/{name}"
        if not isinstance(anim, dict):
            issue(issues, "error", "animation_object", label, anim)
            continue
        row, count = anim.get("row", 0), anim.get("frames", 1)
        if type(row) is not int or type(count) is not int or row < 0 or count < 1 or row >= image.height // ch or count > image.width // cw:
            issue(issues, "error", "animation_bounds", label, {"row": row, "frames": count})
            continue
        duration = anim.get("durations")
        if duration is not None and (not isinstance(duration, list) or len(duration) != count
                                   or any(type(d) is not int or d <= 0 for d in duration)):
            issue(issues, "error", "animation_durations", label, duration)
        fps = anim.get("fps", 8)
        if type(fps) not in (int, float) or not math.isfinite(fps) or fps <= 0:
            issue(issues, "error", "animation_fps", label, fps)
        aa = anim.get("anchor", anchor)
        if not pair(aa) or not (0 <= aa[0] < cw and 0 <= aa[1] < ch):
            issue(issues, "error", "animation_anchor", label, aa)
            continue
        rects = anim.get("collision_rects", meta.get("collision_rects", []))
        if not isinstance(rects, list):
            issue(issues, "error", "collision_rects_type", label, rects)
            rects = []
        for index in range(count):
            referenced.add((row, index))
            cell = image.crop((index*cw, row*ch, (index+1)*cw, (row+1)*ch))
            ca = cell.getchannel("A")
            bbox = ca.getbbox()
            rec = {"anim": name, "frame": index, "row": row, "anchor": aa,
                   "bbox_exclusive": list(bbox) if bbox else None,
                   "opaque_pixels": ca.histogram()[255], "visible_sha256": hashlib.sha256(visible_key(cell)).hexdigest()}
            frames.append(rec)
            if bbox is None:
                issue(issues, "error" if strict else "warning", "empty_referenced_frame", label, index)
                continue
            gutters = [bbox[0], bbox[1], cw-bbox[2], ch-bbox[3]]
            rec.update(gutters_ltrb=gutters, bottom_minus_anchor=(bbox[3]-1)-aa[1],
                       foot_columns=[max((y for y in range(ch) if ca.getpixel((x,y))), default=-1) for x in range(cw)])
            if strict and min(gutters) < 1:
                issue(issues, "error", "transparent_gutter", label, {"frame": index, "gutters_ltrb": gutters})
            if subject == "warden_body":
                # The existing world torso sinks by body_dy. A fold frame is
                # selected by that depth, so planted feet must cancel it exactly.
                world_dy = index if name.endswith("_fold") else 0
                rec["warden_runtime_body_dy"] = world_dy
                rec["warden_world_foot_delta"] = rec["bottom_minus_anchor"] + world_dy
                if rec["warden_world_foot_delta"] != 0:
                    issue(issues, "error", "warden_foot_anchor", label,
                          {"frame": index, "world_foot_delta": rec["warden_world_foot_delta"]})
                reference_name = "damaged" if name.startswith("damaged") else "awake"
                reference = anims.get(reference_name)
                if isinstance(reference, dict) and type(reference.get("row")) is int:
                    ry = reference["row"] * ch
                    fixed = image.crop((0,ry,cw,ry+30))
                    moving = cell.crop((0,0,cw,30))
                    changed = sum(a != b for a,b in zip(pixels(fixed),pixels(moving)))
                    rec["warden_upper_attachment_band_changed_pixels"] = changed
                    if changed:
                        issue(issues, "error", "warden_attachment_band_drift", label,
                              {"frame": index, "reference": reference_name, "changed_pixels": changed})
            if subject == "bloomwild_altar" and list(bbox) != [6,4,66,54]:
                issue(issues, "error", "altar_complete_source_bounds", label,
                      {"actual": list(bbox), "approved": [6,4,66,54], "reason": "Approved source8 must stay complete and uncut"})
            support_pixels, transparent_support = set(), set()
            for rect in rects:
                if (not isinstance(rect, list) or len(rect) != 4 or any(type(n) is not int for n in rect)
                    or not (0 <= rect[0] < rect[2] <= cw and 0 <= rect[1] < rect[3] <= ch)):
                    issue(issues, "error", "collision_rect_bounds", label, rect)
                    continue
                area = [(x,y) for y in range(rect[1],rect[3]) for x in range(rect[0],rect[2])]
                supported = [p for p in area if ca.getpixel(p) == 255]
                support_pixels.update(supported)
                transparent_support.update(p for p in area if ca.getpixel(p) == 0)
                if not supported:
                    issue(issues, "error", "collision_rect_without_visible_support", label, {"frame": index, "rect": rect})
            rec.update(collision_opaque_pixels=len(support_pixels), collision_transparent_pixels=len(transparent_support))
            if transparent_support:
                issue(issues, "warning", "collision_spans_transparency", label, {"frame": index, "pixels": len(transparent_support)})
    unused = []
    for row in range(0 if direct_layout else image.height // ch):
        for col in range(image.width // cw):
            if (row,col) not in referenced and alpha.crop((col*cw,row*ch,(col+1)*cw,(row+1)*ch)).getbbox():
                unused.append([row,col])
    result["unused_opaque_cells"] = unused
    if unused:
        issue(issues, "warning", "unreferenced_opaque_cells", subject, unused)
    normal_path = normal_dir / png.name
    result["normal_present"] = normal_path.is_file()
    if normal_path.is_file():
        result["normal_sha256"] = digest(normal_path)
        result["normal"] = normal_metrics(image, rgba(normal_path), meta, subject, issues)
    elif require_normal:
        issue(issues, "error", "required_normal_missing", subject, str(normal_path))
    return result


def inventory(root: Path) -> dict[str, str]:
    files = list((root / "src").rglob("*"))
    files += list((root / "assets").rglob("*"))
    files += [root / n for n in ("build.rs", "Cargo.toml", "Cargo.lock", ".cargo/config.toml", "rust-toolchain.toml")]
    return {p.relative_to(root).as_posix(): digest(p) for p in sorted(files) if p.is_file()}


def embedding_metrics(root: Path, baseline_root: Path | None, issues: list) -> dict:
    build = root / "build.rs"
    if not build.is_file():
        issue(issues, "error", "build_rules_missing", str(root), "build.rs missing")
        return {}
    source = build.read_text()
    submatch = re.search(r'for sub in\s*\[((?:\s*"[^"\\]+"\s*,?)+)\]', source)
    extmatch = re.search(r'matches!\(ext,\s*((?:"[^"\\]+"\s*\|\s*)*"[^"\\]+")\s*\)', source)
    if not submatch or not extmatch:
        issue(issues, "error", "unrecognized_embedding_rules", "build.rs", "Independent inventory needs review after build-rule changes")
        return {}
    subdirs, extensions = re.findall(r'"([^"\\]+)"', submatch[1]), set(re.findall(r'"([^"\\]+)"', extmatch[1]))
    accepted, suspicious, total = [], [], 0
    # The build traverses recursively. A pretty preview in assets/sprites would
    # silently ship even if no game code names it.
    pattern = re.compile(r"(?:^|[_./-])(raw|preview|contact|trial|candidate|debug|thumbnail|reference|backup|source)(?:[_./-]|$)", re.I)
    for sub in subdirs:
        for p in sorted((root / "assets" / sub).rglob("*")):
            if not p.is_file() or p.suffix[1:] not in extensions:
                continue
            rel = p.relative_to(root).as_posix()
            size = p.stat().st_size
            accepted.append({"file": rel, "bytes": size, "sha256": digest(p)})
            total += size
            old = baseline_root / rel if baseline_root else None
            newly_embedded = old is not None and not old.is_file()
            if p.is_symlink() or pattern.search(p.relative_to(root / "assets").as_posix()):
                suspicious.append(rel)
                issue(issues, "error" if newly_embedded or p.is_symlink() else "warning", "suspicious_embedded_asset", rel,
                      "Recursive build includes this file; review its intent")
    return {"subdirectories": subdirs, "extensions": sorted(extensions), "file_count": len(accepted),
            "bytes": total, "suspicious": suspicious, "files": accepted,
            "limitation": "Checks source build rules; verify actual build receipt and GLINT_ASSETS override separately."}


def asset_metrics(root: Path, baseline_root: Path | None, strict_patterns: list[str],
                  required_normal_patterns: list[str], issues: list, baseline: bool = False) -> dict:
    folder = root / "assets/sprites"
    if not folder.is_dir():
        issue(issues, "error", "sprite_directory_missing", str(folder), "Cannot audit this root")
    result = []
    for path in sorted(folder.glob("*.png")):
        try:
            result.append(audit_sheet(path, root / "assets/normals", matches(path.stem, strict_patterns),
                                      not baseline and matches(path.stem, required_normal_patterns), issues))
        except (OSError, ValueError, TypeError, KeyError, OverflowError) as exc:
            issue(issues, "error", "invalid_export", path.name, str(exc))
    orphan_manifests = [p.name for p in sorted(folder.glob("*.json")) if not p.with_suffix(".png").is_file()
                        and p.name != "warden_layout.json"]
    if orphan_manifests:
        issue(issues, "warning", "manifests_without_png", str(folder), orphan_manifests)
    orphan_normals = [p.name for p in sorted((root / "assets/normals").glob("*.png")) if not (folder / p.name).is_file()]
    if orphan_normals:
        issue(issues, "error", "normals_without_albedo", str(root), orphan_normals)
    if baseline and not (root / "assets/normals").is_dir():
        issue(issues, "warning", "baseline_normals_not_frozen", str(root), "Baseline source snapshot excludes normals; do not substitute current maps")
    return {"sheet_count": len(result), "normal_pair_count": sum(s.get("normal_present", False) for s in result),
            "orphan_manifests": orphan_manifests, "sheets": result}


def resolve_evidence_file(name: str, manifest: Path, root: Path) -> Path:
    p = Path(name)
    if p.is_absolute():
        return p
    # Existing tools use both repository-relative and manifest-relative names.
    for candidate in [manifest.parent / p, root / p]:
        if candidate.is_file():
            return candidate
    return manifest.parent / p


def render_command_settings(command: object) -> list | None:
    if not isinstance(command, list):
        return None
    result, index = [], 1
    while index < len(command):
        if command[index] in ("--out", "--report"):
            index += 2
        else:
            result.append(command[index])
            index += 1
    return result


def capture_metrics(manifests: list[Path], root: Path, issues: list) -> dict:
    records, observed = [], set()
    for path in manifests:
        doc = json.loads(path.read_text())
        entries = doc.get("captures", [])
        if not isinstance(entries, list):
            issue(issues, "error", "capture_manifest_entries", str(path), "Expected captures list")
            continue
        group = {"manifest": str(path), "sha256": digest(path), "binary_sha256": doc.get("binary_sha256"),
                 "captures": [], "kind": doc.get("kind")}
        records.append(group)
        if not entries:
            issue(issues, "warning", "empty_capture_manifest", str(path), "No captured image evidence")
        for rec in entries:
            filename = rec.get("file", rec.get("path"))
            label = f"{rec.get('zone', 'unknown')}/{rec.get('name', filename)}"
            if label in observed:
                issue(issues, "error", "duplicate_capture_identity", str(path), label)
            observed.add(label)
            if not isinstance(filename, str):
                issue(issues, "error", "capture_file_missing", label, rec)
                continue
            file = resolve_evidence_file(filename, path, root)
            if not file.is_file():
                issue(issues, "error", "capture_file_missing", label, str(file))
                continue
            im = rgba(file)
            actual_sha = digest(file)
            expected = rec.get("sha256")
            if expected is None:
                issue(issues, "warning", "capture_unpinned", label, str(file))
            elif actual_sha != expected:
                issue(issues, "error", "capture_hash", label, {"expected": expected, "actual": actual_sha})
            if im.size != (384, 216):
                issue(issues, "error", "capture_native_grid", label, list(im.size))
            lum = list(pixels(im.convert("L")))
            group["captures"].append({"key": label, "file": str(file), "sha256": actual_sha,
                                      "size": list(im.size), "hero_visible": rec.get("hero_visible"),
                                      "binary_sha256": doc.get("binary_sha256"),
                                      "render_settings": {"position": rec.get("position"),
                                                          "hero_visible": rec.get("hero_visible"),
                                                          "command": render_command_settings(rec.get("command")),
                                                          "GLINT_SPRITES": doc.get("GLINT_SPRITES"),
                                                          "GLINT_LIGHT_TRIAL": doc.get("GLINT_LIGHT_TRIAL")},
                                      "mean_luma": sum(lum)/len(lum), "distinct_rgb_colors": len(set(pixels(im.convert("RGB"))))})
    return {"manifests": records,
            "limitation": "Color/luma counts are descriptive; no threshold is interpreted as beauty or correctness."}


def capture_comparison(before: dict, after: dict, issues: list, exact_replay: bool = False) -> dict:
    def flatten(doc):
        return {c["key"]: c for m in doc["manifests"] for c in m["captures"]}
    old, new = flatten(before), flatten(after)
    result = {"matched": [], "only_before": sorted(old.keys()-new.keys()), "only_after": sorted(new.keys()-old.keys()),
              "mode": "exact deterministic replay" if exact_replay else "descriptive before/after"}
    for key in sorted(old.keys() & new.keys()):
        if exact_replay:
            old_sha, new_sha = old[key].get("binary_sha256"), new[key].get("binary_sha256")
            if not old_sha or old_sha != new_sha:
                issue(issues, "error", "replay_renderer_identity", key, {"before": old_sha, "after": new_sha})
            if old[key].get("render_settings") != new[key].get("render_settings"):
                issue(issues, "error", "replay_settings_mismatch", key,
                      {"before": old[key].get("render_settings"), "after": new[key].get("render_settings")})
        a, b = rgba(Path(old[key]["file"])), rgba(Path(new[key]["file"]))
        if a.size != b.size:
            result["matched"].append({"key": key, "different_dimensions": [list(a.size),list(b.size)]})
            if exact_replay:
                issue(issues, "error", "replay_dimensions", key, result["matched"][-1])
            continue
        aa, bb = list(pixels(a)), list(pixels(b))
        changed = sum(x != y for x,y in zip(aa,bb))
        result["matched"].append({"key": key, "changed_pixels": changed, "changed_fraction": changed/len(aa),
                                   "rgb_abs_difference_mean": sum(sum(abs(x[i]-y[i]) for i in range(3)) for x,y in zip(aa,bb))/(len(aa)*3)})
        if exact_replay and changed:
            issue(issues, "error", "replay_pixel_difference", key, changed)
    if exact_replay and (result["only_before"] or result["only_after"]):
        issue(issues, "error", "replay_scenario_mismatch", "capture sets", {k:v for k,v in result.items() if k.startswith("only_")})
    if not result["matched"]:
        issue(issues, "warning", "no_matched_captures", result["mode"], "No paired frames were compared")
    return result


def campaign_metrics(path: Path, issues: list) -> dict:
    doc = json.loads(path.read_text())
    subject = str(path)
    trace, milestones = doc.get("trace", []), doc.get("milestones", {})
    if not isinstance(trace, list) or not isinstance(milestones, dict):
        issue(issues, "error", "campaign_schema", subject, "Missing audit trace/milestones")
        return {"sha256": digest(path)}
    zones = sorted({str(row.get("zone")).lower() for row in trace})
    missing_zones = sorted(REGIONS-set(zones))
    missing_milestones = sorted(set(REQUIRED_MILESTONES)-milestones.keys())
    if missing_zones:
        issue(issues, "error", "campaign_regions", subject, missing_zones)
    if missing_milestones:
        issue(issues, "error", "campaign_milestones", subject, missing_milestones)
    invalid_milestones = {k: milestones[k] for k in REQUIRED_MILESTONES if k in milestones
                          and (type(milestones[k]) is not int or milestones[k] <= 0
                               or (type(doc.get("ticks")) is int and milestones[k] > doc["ticks"]))}
    if invalid_milestones:
        issue(issues, "error", "campaign_milestone_timestamps", subject, invalid_milestones)
    if doc.get("scene") != "victory" or doc.get("victory_ready") is not True:
        issue(issues, "error", "campaign_ending", subject, {"scene": doc.get("scene"), "victory_ready": doc.get("victory_ready")})
    if doc.get("pilot") != "tour":
        issue(issues, "error", "campaign_real_input_tour", subject, doc.get("pilot"))
    if doc.get("simulation_hz") != 60:
        issue(issues, "error", "campaign_simulation_rate", subject, doc.get("simulation_hz"))
    region_sequence = []
    for row in trace:
        z = str(row.get("zone", "")).lower()
        if not region_sequence or z != region_sequence[-1]:
            region_sequence.append(z)
    if region_sequence != ["bloomwild","glasswater","cinder","summit"]:
        issue(issues, "error", "campaign_region_order", subject, region_sequence)
    ticks = [row.get("tick") for row in trace]
    if not ticks or any(type(t) is not int for t in ticks) or any(b <= a for a,b in zip(ticks,ticks[1:])):
        issue(issues, "error", "campaign_trace_order", subject, "Trace absent or timestamps not strictly increasing")
    malformed = []
    for row in trace:
        pos = row.get("pos")
        if not isinstance(pos, list) or len(pos) != 2 or any(type(v) not in (int,float) or not math.isfinite(v) for v in pos):
            malformed.append(row.get("tick"))
    if malformed:
        issue(issues, "error", "campaign_trace_positions", subject, malformed)
    shard_trace = {z: any(r.get("zone") == z and r.get("shard_taken") is True for r in trace) for z in sorted(SHARD_REGIONS)}
    if not all(shard_trace.values()):
        issue(issues, "error", "campaign_shard_trace", subject, shard_trace)
    # Milestones should reflect real encounter progression, not merely a final
    # boolean. A separate phase_2_heart_open can occur after phase_2_damage.
    sequence = ["warden_awake", "phase_2_damage", "phase_3_damage", "warden_dying", "victory"]
    vals = [milestones.get(k) for k in sequence]
    if all(type(v) is int for v in vals) and any(b < a for a,b in zip(vals,vals[1:])):
        issue(issues, "error", "campaign_phase_order", subject, dict(zip(sequence, vals)))
    return {"file": subject, "sha256": digest(path), "pilot": doc.get("pilot"), "god": doc.get("god"),
            "ticks": doc.get("ticks"), "capture_fps": doc.get("capture_fps"), "trace_rows": len(trace),
            "regions_in_trace": zones, "shard_trace": shard_trace, "milestones": milestones,
            "region_sequence": region_sequence,
            "scene": doc.get("scene"), "victory_ready": doc.get("victory_ready"),
            "max_recorded_deaths": max((r.get("deaths",0) for r in trace), default=0),
            "limitation": "God mode verifies reachable inputs/progression, not normal difficulty. A 15tick trace is not 60fps visual coverage."}


def test_log_metrics(paths: list[Path], issues: list) -> list:
    result = []
    for path in paths:
        text = path.read_text(errors="replace")
        passed = re.findall(r"^test (\S+) \.\.\. ok$", text, re.M)
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", text, re.M)
        summary = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
        if failed or any(s[0] != "ok" or int(s[2]) for s in summary):
            issue(issues, "error", "runtime_test_failures", str(path), failed or summary)
        if not summary or not passed:
            issue(issues, "warning", "runtime_test_log_inconclusive", str(path), "No individual passing tests and final result found")
        result.append({"file": str(path), "sha256": digest(path), "passed_tests": passed,
                       "failed_tests": failed, "summaries": summary,
                       "limitation": "A log alone does not prove source/binary correspondence; bind it to the controlled build receipt."})
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--baseline-root", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--strict-sheet", action="append", default=[], help="Additional fnmatch pattern for >=1px transparent gutters")
    parser.add_argument("--require-normal", action="append", default=[])
    parser.add_argument("--require-round13", action="store_true", help="Require the assembled pass's six expected atlas families")
    parser.add_argument("--baseline-only", action="store_true", help="Audit an older frozen snapshot without requiring new assets or missing unfrozen normals")
    parser.add_argument("--capture-manifest", type=Path, action="append", default=[])
    parser.add_argument("--baseline-capture-manifest", type=Path, action="append", default=[])
    parser.add_argument("--replay-manifest", type=Path, action="append", default=[])
    parser.add_argument("--campaign-report", type=Path)
    parser.add_argument("--scenario-plan", type=Path)
    parser.add_argument("--test-log", type=Path, action="append", default=[])
    args = parser.parse_args(argv)
    root = args.root.resolve()
    baseline = args.baseline_root.resolve() if args.baseline_root else None
    destination = args.out.resolve()
    if destination.exists():
        parser.error("--out already exists; use a fresh evidence filename")
    if destination.suffix != ".json" or any(destination.is_relative_to(root / p) for p in ("src", "assets")):
        parser.error("--out must be a new JSON evidence file outside src/assets")
    issues = []
    before_inputs = inventory(root)
    measurements = {"assets": asset_metrics(root, baseline, STRICT_SHEETS+args.strict_sheet, NORMAL_SHEETS+args.require_normal, issues, baseline=args.baseline_only),
                    "embedding": embedding_metrics(root, baseline, issues)}
    if args.require_round13:
        names = {s["sheet"] for s in measurements["assets"]["sheets"]}
        missing = sorted(set(ROUND13_SHEETS)-names)
        if missing:
            issue(issues, "error", "round13_family_missing", "assets/sprites", missing)
        warden = next((s for s in measurements["assets"]["sheets"] if s["sheet"] == "warden_body"), {})
        expected = {k:1 for k in ("idle","awake","damaged","brace","damaged_brace","opening","damaged_opening")}
        expected.update({k:17 for k in ("awake_fold","opening_fold","damaged_fold","damaged_opening_fold")})
        declared = Counter(f["anim"] for f in warden.get("frames", []))
        wrong = {k: {"expected":v, "actual":declared[k]} for k,v in expected.items() if declared[k] != v}
        if wrong:
            issue(issues, "error", "round13_warden_pose_coverage", "warden_body", wrong)
    if baseline:
        baseline_issues = []
        measurements["baseline"] = {"root": str(baseline), "input_inventory": inventory(baseline),
                                    "assets": asset_metrics(baseline, None, STRICT_SHEETS+args.strict_sheet, [], baseline_issues, baseline=True),
                                    "issues": baseline_issues}
        old = measurements["baseline"]["input_inventory"]
        measurements["input_changes"] = {"added": sorted(before_inputs.keys()-old.keys()), "removed": sorted(old.keys()-before_inputs.keys()),
                                          "modified": sorted(p for p in old.keys() & before_inputs.keys() if old[p] != before_inputs[p])}
    captured = capture_metrics(args.capture_manifest, root, issues)
    measurements["captures"] = captured
    if args.baseline_capture_manifest:
        old = capture_metrics(args.baseline_capture_manifest, baseline or root, issues)
        measurements["baseline_captures"] = old
        measurements["visual_delta"] = capture_comparison(old, captured, issues)
    if args.replay_manifest:
        replay = capture_metrics(args.replay_manifest, root, issues)
        measurements["replay"] = replay
        measurements["determinism"] = capture_comparison(captured, replay, issues, exact_replay=True)
    if args.campaign_report:
        measurements["campaign"] = campaign_metrics(args.campaign_report, issues)
    else:
        issue(issues, "warning", "campaign_not_supplied", "runtime", "Routes, three regional shards and ending remain unverified")
    measurements["runtime_test_logs"] = test_log_metrics(args.test_log, issues)
    if args.scenario_plan:
        measurements["scenario_plan"] = {"file": str(args.scenario_plan.resolve()), "sha256": digest(args.scenario_plan),
                                         "status": "planned scenarios; execution not implied"}
    after_inputs = inventory(root)
    if before_inputs != after_inputs:
        issue(issues, "error", "input_changed_during_audit", str(root), sorted(k for k in before_inputs.keys() | after_inputs.keys() if before_inputs.get(k) != after_inputs.get(k)))
    failures = sum(i["severity"] == "error" for i in issues)
    report = {"schema": SCHEMA, "created_utc": datetime.now(timezone.utc).isoformat(),
              "root": str(root), "tool_sha256": digest(Path(__file__)),
              "verdict": "failed" if failures else "measured_with_unverified_gates",
              "contract_failures": failures, "issue_counts": dict(Counter(i["severity"] for i in issues)),
              "issues": issues, "measurements": measurements, "input_inventory": before_inputs,
              "unverified_gates": REVIEW_GATES, "aesthetic_verdict": "not assessed by this automated harness"}
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open("x") as stream:
        json.dump(report, stream, indent=2, sort_keys=True)
        stream.write("\n")
    print(json.dumps({"report": str(destination), "verdict": report["verdict"], "contract_failures": failures,
                      "sheets": measurements["assets"]["sheet_count"], "issues": report["issue_counts"]}))
    return bool(failures)


if __name__ == "__main__":
    sys.exit(main())
