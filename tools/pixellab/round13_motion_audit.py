#!/usr/bin/env python3
"""Rank temporal review candidates in actual native headless gameplay captures.

Requires Pillow and numpy. This is a review aid, not a visual pass/fail oracle.
Adjacent frames find brief flashes; 15-tick audit samples use exact recorded
camera coordinates to distinguish world motion from ordinary camera scrolling.
No camera interpolation, image resampling, browser, window, or game mutation.

Example:
  uv run --with pillow --with numpy python tools/pixellab/round13_motion_audit.py \
      --capture out/pixellab/recordings/round13-campaign --out out/round13-motion
"""

import argparse
import bisect
import csv
import hashlib
import io
import json
from pathlib import Path
import re

import numpy as np
from PIL import Image, ImageDraw

WIDTH, HEIGHT = 384, 216
SAFE_TOP, SAFE_BOTTOM = 24, 196
FRAME_NAME = re.compile(r"f(\d+)\.png$")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_frame(path):
    data = path.read_bytes()
    with Image.open(io.BytesIO(data)) as im:
        if im.size != (WIDTH, HEIGHT):
            raise ValueError(f"Expected native {WIDTH}x{HEIGHT}, found {im.size}: {path}")
        rgb = np.asarray(im.convert("RGB"), dtype=np.uint8).copy()
    return rgb, hashlib.sha256(data).hexdigest()


def delta(a, b):
    return np.max(np.abs(a.astype(np.int16) - b.astype(np.int16)), axis=2)


def isolated_flash(before, current, after, threshold=24, return_threshold=8):
    return ((delta(before, current) >= threshold)
            & (delta(current, after) >= threshold)
            & (delta(before, after) <= return_threshold))


def registered_pair(before, after, camera_before, camera_after, roi=None):
    """Return identically world-registered crops, with HUD excluded on both."""
    ax, ay = map(int, camera_before)
    bx, by = map(int, camera_after)
    x0, y0 = max(ax, bx), max(ay + SAFE_TOP, by + SAFE_TOP)
    x1, y1 = min(ax + WIDTH, bx + WIDTH), min(ay + SAFE_BOTTOM, by + SAFE_BOTTOM)
    if roi is not None:
        rx, ry, rw, rh = roi
        x0, y0 = max(x0, rx), max(y0, ry)
        x1, y1 = min(x1, rx + rw), min(y1, ry + rh)
    if x1 <= x0 or y1 <= y0:
        return None
    a = before[y0-ay:y1-ay, x0-ax:x1-ax]
    b = after[y0-by:y1-by, x0-bx:x1-bx]
    return a, b, [x0, y0, x1-x0, y1-y0]


def choose(rows, key, top, separation=30):
    selected = []
    for row in sorted(rows, key=lambda r: r[key], reverse=True):
        if row[key] <= 0:
            continue
        if all(abs(row["tick"] - prior["tick"]) >= separation for prior in selected):
            selected.append(row)
        if len(selected) >= top:
            break
    return selected


def write_csv(path, rows):
    if not rows:
        path.write_text("")
        return
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def fit_native(image):
    """Center a smaller crop; never scale native pixels."""
    tile = Image.new("RGB", (WIDTH, HEIGHT), (10, 13, 19))
    tile.paste(image, ((WIDTH-image.width)//2, (HEIGHT-image.height)//2))
    return tile


def heatmap(differences):
    rgb = np.zeros((*differences.shape, 3), dtype=np.uint8)
    rgb[:, :, 0] = np.minimum(differences * 3, 255)
    rgb[:, :, 1] = np.minimum(differences, 255)
    return Image.fromarray(rgb)


def contact_sheet(path, cards):
    if not cards:
        return
    columns = max(len(items) for _, items in cards)
    row_height = HEIGHT + 48
    sheet = Image.new("RGB", (columns * WIDTH, len(cards) * row_height), (14, 19, 26))
    draw = ImageDraw.Draw(sheet)
    for row_index, (label, items) in enumerate(cards):
        y = row_index * row_height
        draw.text((8, y + 4), label, fill=(235, 229, 209))
        for col, (caption, im) in enumerate(items):
            draw.text((col * WIDTH + 8, y + 20), caption, fill=(180, 196, 211))
            sheet.paste(fit_native(im), (col * WIDTH, y + 44))
    sheet.save(path)


def self_test():
    # A moving camera must produce zero registered change for fixed world art.
    world = np.random.default_rng(7).integers(0, 256, (280, 460, 3), dtype=np.uint8)
    first = world[0:HEIGHT, 0:WIDTH]
    moved = world[3:3+HEIGHT, 5:5+WIDTH]
    a, b, bounds = registered_pair(first, moved, (0, 0), (5, 3))
    assert not delta(a, b).any() and bounds == [5, 27, 379, 169]
    a, b, bounds = registered_pair(first, moved, (0, 0), (5, 3), (30, 40, 40, 50))
    assert not delta(a, b).any() and bounds == [30, 40, 40, 50]
    assert registered_pair(first, moved, (0, 0), (1000, 0)) is None
    blank = np.zeros((HEIGHT, WIDTH, 3), dtype=np.uint8)
    flash = blank.copy()
    flash[100:110, 120:140] = 255
    assert int(isolated_flash(blank, flash, blank).sum()) == 200
    assert not isolated_flash(blank, flash, flash).any()
    assert len(choose([{"tick": 10, "score": 2}, {"tick": 11, "score": 3},
                       {"tick": 60, "score": 1}], "score", 8)) == 2
    print("Self-test passed: camera registration, ROI, isolated flash, candidate spacing")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, help="record.py root or direct shot output directory")
    parser.add_argument("--report", type=Path, help="defaults to CAPTURE/report.json")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--zone", choices=["bloomwild", "glasswater", "cinder", "summit"])
    parser.add_argument("--world-roi", help="x,y,width,height; applies only to exact-camera pairs")
    parser.add_argument("--top", type=int, default=8)
    parser.add_argument("--threshold", type=int, default=24)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not args.capture or not args.out:
        parser.error("--capture and --out are required")
    if args.top < 1 or not 1 <= args.threshold <= 255:
        parser.error("--top must be positive and --threshold must be 1..255")
    roi = None
    if args.world_roi:
        try:
            roi = tuple(int(value) for value in args.world_roi.split(","))
            assert len(roi) == 4 and roi[2] > 0 and roi[3] > 0
        except (ValueError, AssertionError):
            parser.error("--world-roi must be integer x,y,positive_width,positive_height")
    source = args.capture.resolve()
    report_path = (args.report or source / "report.json").resolve()
    report = json.loads(report_path.read_text())
    if report.get("scale") != 1 or report.get("simulation_hz") != 60:
        raise ValueError("Requires scale=1, simulation_hz=60 shot --audit report")
    frame_dir = source / "frames" if (source / "frames").is_dir() else source
    frames = {}
    for path in frame_dir.glob("f*.png"):
        match = FRAME_NAME.fullmatch(path.name)
        if match:
            tick = int(match[1])
            if tick in frames:
                raise ValueError(f"Duplicate captured tick {tick}: {frames[tick]} and {path}")
            frames[tick] = path
    ticks = sorted(frames)
    if len(ticks) < 3:
        raise ValueError("At least three actual frames are required")
    if len(ticks) != report.get("frames"):
        raise ValueError(f"Frame count {len(ticks)} disagrees with shot report {report.get('frames')}")
    trace = {int(row["tick"]): row for row in report.get("trace", [])}
    if not trace:
        raise ValueError("No exact-camera trace: capture with shot --audit")
    for tick, row in trace.items():
        camera = row.get("cam", [])
        if len(camera) != 2 or any(int(value) != value for value in camera):
            raise ValueError(f"Expected exact integer-pixel camera at trace tick {tick}")
    trace_ticks = sorted(trace)

    def zone_at(tick):
        # Zone is known only between two samples in the same zone. Never infer
        # camera coordinates between trace rows (they may contain shake/rounding).
        at = bisect.bisect_right(trace_ticks, tick)
        if tick in trace:
            return trace[tick]["zone"]
        if 0 < at < len(trace_ticks):
            a, b = trace[trace_ticks[at-1]], trace[trace_ticks[at]]
            if a["zone"] == b["zone"]:
                return a["zone"]
        return None

    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    adjacent, registered, frame_hashes = [], [], hashlib.sha256()
    previous, current = None, None
    for index, tick in enumerate(ticks):
        following, digest = load_frame(frames[tick])
        frame_hashes.update(f"{frames[tick].name} {digest}\n".encode())
        if previous is not None:
            prev_tick, a = previous
            current_tick, b = current
            if current_tick-prev_tick == 1 and tick-current_tick == 1:
                zone = zone_at(current_tick)
                if args.zone is None or zone == args.zone:
                    aa, bb, cc = (im[SAFE_TOP:SAFE_BOTTOM] for im in (a, b, following))
                    differences = delta(aa, bb)
                    flashes = isolated_flash(aa, bb, cc, args.threshold)
                    adjacent.append({"tick": current_tick, "zone": zone,
                        "changed_fraction": float((differences >= args.threshold).mean()),
                        "mean_max_channel_delta": float(differences.mean()),
                        "one_frame_reversal_fraction": float(flashes.mean())})
        previous, current = current, (tick, following)
        if index and index % 3000 == 0:
            print(f"Scanned {index}/{len(ticks)} native frames", flush=True)
    captured_trace = [tick for tick in trace_ticks if tick in frames]
    for before_tick, tick in zip(captured_trace, captured_trace[1:]):
        a_row, b_row = trace[before_tick], trace[tick]
        if tick-before_tick != 15 or a_row["zone"] != b_row["zone"]:
            continue
        if args.zone is not None and b_row["zone"] != args.zone:
            continue
        a, _ = load_frame(frames[before_tick])
        b, _ = load_frame(frames[tick])
        result = registered_pair(a, b, a_row["cam"], b_row["cam"], roi)
        if result is None:
            continue
        aa, bb, bounds = result
        differences = delta(aa, bb)
        registered.append({"tick": tick, "previous_tick": before_tick, "zone": b_row["zone"],
            "camera_dx": b_row["cam"][0]-a_row["cam"][0],
            "camera_dy": b_row["cam"][1]-a_row["cam"][1],
            "world_bounds": bounds, "compared_pixels": int(differences.size),
            "changed_fraction": float((differences >= args.threshold).mean()),
            "mean_max_channel_delta": float(differences.mean())})
    flash_candidates = choose(adjacent, "one_frame_reversal_fraction", args.top)
    change_candidates = choose(registered, "changed_fraction", args.top)
    flash_cards = []
    for row in flash_candidates:
        tick = row["tick"]
        images = [(f"tick {t}", Image.open(frames[t]).convert("RGB")) for t in [tick-1, tick, tick+1]]
        a, _ = load_frame(frames[tick-1])
        b, _ = load_frame(frames[tick])
        c, _ = load_frame(frames[tick+1])
        mask = isolated_flash(a, b, c, args.threshold)
        mask[:SAFE_TOP] = False
        mask[SAFE_BOTTOM:] = False
        images.append(("1-frame reversal pixels", Image.fromarray(np.repeat(mask[:, :, None], 3, 2).astype(np.uint8)*255)))
        flash_cards.append((f"REVIEW {row['zone']} tick {tick}: reversal {row['one_frame_reversal_fraction']:.4%}; screen coordinates", images))
    contact_sheet(out / "adjacent-reversals.png", flash_cards)
    registered_cards = []
    for row in change_candidates:
        before_tick, tick = row["previous_tick"], row["tick"]
        a, _ = load_frame(frames[before_tick])
        b, _ = load_frame(frames[tick])
        aa, bb, bounds = registered_pair(a, b, trace[before_tick]["cam"], trace[tick]["cam"], roi)
        registered_cards.append((f"REVIEW {row['zone']} {before_tick}->{tick}: changed {row['changed_fraction']:.2%}; world {bounds}",
            [(f"tick {before_tick}, registered crop", Image.fromarray(aa)),
             (f"tick {tick}, registered crop", Image.fromarray(bb)),
             ("difference: red=max RGB delta", heatmap(delta(aa, bb)))]))
    contact_sheet(out / "registered-changes.png", registered_cards)
    write_csv(out / "adjacent.csv", adjacent)
    write_csv(out / "registered.csv", registered)
    gaps = [{"before": a, "after": b, "missing_ticks": b-a-1}
            for a, b in zip(ticks, ticks[1:]) if b-a != 1]
    manifest_path = source / "manifest.json"
    result = {"schema": "glintblade.round13.motion-review.v1", "verdict": "candidates_require_visual_review",
        "source": str(source), "report": str(report_path), "report_sha256": sha(report_path),
        "source_manifest_sha256": sha(manifest_path) if manifest_path.is_file() else None,
        "frame_inventory_sha256": frame_hashes.hexdigest(), "frame_inventory_recipe": "sha256 of ascending-tick filename + space + file_sha256 + newline",
        "native_pixels": [WIDTH, HEIGHT], "simulation_hz": 60, "frame_count": len(ticks),
        "first_tick": ticks[0], "last_tick": ticks[-1], "every_tick_present": not gaps,
        "gaps": gaps, "zone_filter": args.zone, "world_roi": roi,
        "max_channel_threshold": args.threshold, "excluded_hud_rows": [0, SAFE_TOP, SAFE_BOTTOM, HEIGHT],
        "adjacent_triples_measured": len(adjacent), "exact_camera_pairs_measured": len(registered),
        "flash_candidates": flash_candidates, "registered_change_candidates": change_candidates,
        "limitations": ["Candidates include expected attacks, particles, grass sway, water, fades, trauma and daylight.",
            "Adjacent reversal scores use screen coordinates; moving camera and shake can create false candidates.",
            "Exact world registration uses recorded 15-tick camera samples only; it cannot resolve a 1-frame shadow drift.",
            "Low temporal change cannot prove correct contact, silhouette, weapon visibility, terrain collision or performance.",
            "No ordinary-player difficulty, browser, fullscreen, GPU, audio or runtime FPS conclusion is made."]}
    (out / "review.json").write_text(json.dumps(result, indent=2) + "\n")
    (out / "README.md").write_text("# Temporal candidates require review\n\n"
        "Inspect `adjacent-reversals.png` for transient flashes and `registered-changes.png` for world-aligned pops. "
        "Expected combat effects and moving scenery rank too. The CSV files retain all measured scores; "
        "`review.json` records provenance, frame gaps, thresholds and limitations. No score is a visual correctness verdict.\n")
    print(json.dumps({"out": str(out), "frames": len(ticks), "every_tick_present": not gaps,
                      "adjacent_triples": len(adjacent), "exact_camera_pairs": len(registered)}))


if __name__ == "__main__":
    main()
