#!/usr/bin/env python3
"""Pack reviewed PixelLab native pose edits without moving/rescaling any pixel.

The raw sheet is art, not a new rig. Restore the authored foot-support rows
and the thrower's exact held-seed mask; map the new mineral seams to the
existing charge ramp, with the original quiet/charged/dazed progression.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from collections import Counter, deque
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "art/pixellab/2026-11-01-round6/enemies/botanical"
OUT = ROOT / "out/pixellab/round6/botanical"
SPECIES = ("sproutling", "thorn_imp", "crystal_beetle")
TRANSPARENT = (0, 0, 0, 0)


def rgba(value: str) -> tuple[int, int, int, int]:
    return tuple(bytes.fromhex(value.removeprefix("#"))) + (255,)


SEED = {rgba("a3a38f"), rgba("bdbba1")}
CRYSTAL = [rgba(c) for c in ("4d4866", "796c91", "a195b8", "cdc3db", "e2d8e9")]
PALETTES = {
    "sproutling": [rgba(c) for c in (
        "263244", "303b4d", "3b535b", "587278", "829b8e", "a2af9a",
        "655f78", "927c91", "bf9da9", "dbb9bd", "bd7a85", "9e716d",
        "b28b7a", "a39c89", "c2c1b3", "dfdac1", "e7dfcb", "afa58e",
    )],
    "thorn_imp": [rgba(c) for c in (
        "263244", "303b4d", "3b535b", "465b61", "647c77", "829b8e",
        "a2af9a", "747d8d", "a1a4a5", "c2c1b3", "dfdac1", "927c91",
        "bf9da9", "8c5c5d", "b58278", "c5b493", "d4c4a4", "a68b6b",
        "747678", "4d505f",
    )],
    "crystal_beetle": [rgba(c) for c in (
        "222d3d", "29313f", "364657", "4c6070", "73858b", "a6b4ad",
        "9da2aa", "b3b4ad", "c5c7b8", "dfdac1", "665b78", "908f94",
    )],
}
LOOKS = {
    ("sproutling", 1): "oatcap: broad ivory fungus with dark gills and rose lichen",
    ("sproutling", 2): "camellia: open dusty-rose petal mask and sage sepals",
    ("thorn_imp", 1): "birch: forked pale bark mask and rust-rose lichen mantle",
    ("thorn_imp", 2): "bracken: broad layered fern mask and pale raffia shoulders",
    ("crystal_beetle", 1): "geode: low broken quartz carapace with hollow violet seam",
    ("crystal_beetle", 2): "scallop: ribbed fossil valves and violet mineral cleft",
}


def nearest(p, palette):
    return min(palette, key=lambda q: sum((p[i] - q[i]) ** 2 for i in range(3)))


def color(p, species: str, frame: dict):
    if not p[3]:
        return TRANSPARENT
    if species == "crystal_beetle" and p[0] >= 80 and p[2] - p[1] > 12 and p[0] - p[1] > -8:
        # Match the new geode/scallop crystal material to the existing shader
        # emission colors. Ivory shell remains matte. The new anatomy keeps its
        # own crystal shape; the old tall spikes are deliberately not pasted on.
        luma = (p[0] + p[1] + p[2]) / 3
        level = 0 if luma < 100 else 1 if luma < 140 else 2 if luma < 185 else 3
        anim, index = frame["anim"], frame["frame"]
        if anim.startswith("dazed"):
            level = min(level, 1)
        elif anim.startswith("hurt"):
            level = min(level, 2)
        elif anim.startswith("attack") or anim.startswith("windup") and index >= 3:
            level = min(4, level + 1)
        elif anim.startswith("windup") and index >= 1:
            level = min(3, level + 1)
        return CRYSTAL[level]
    return nearest(p, PALETTES[species])


def source_seed_mask(source: Image.Image) -> set[tuple[int, int]]:
    w, h = source.size
    seed = {(x, y) for y in range(h) for x in range(w) if source.getpixel((x, y)) in SEED}
    # Keep the entire authored 5x5 seed (dark outline included), along with the
    # same tiny transparent surround. This preserves the release-hand centroid
    # the simulation reads, and the visual held-to-released timing.
    return {(x + dx, y + dy) for x, y in seed for dx in range(-2, 3) for dy in range(-2, 3)
            if 0 <= x + dx < w and 0 <= y + dy < h}


def components(im: Image.Image):
    w, h = im.size
    unvisited = {(x, y) for y in range(h) for x in range(w) if im.getpixel((x, y))[3]}
    result = []
    while unvisited:
        first = min(unvisited, key=lambda p: (p[1], p[0]))
        unvisited.remove(first)
        queue, group = deque([first]), [first]
        while queue:
            x, y = queue.popleft()
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    p = x + dx, y + dy
                    if p in unvisited:
                        unvisited.remove(p)
                        queue.append(p)
                        group.append(p)
        result.append(group)
    return result


def make_cell(raw: Image.Image, source: Image.Image, species: str, frame: dict):
    w, h = source.size
    out = Image.new("RGBA", (w, h))
    for y in range(h):
        for x in range(w):
            out.putpixel((x, y), color(raw.getpixel((x, y)), species, frame))
    protected = set()
    foot_cut = max(0, source.getbbox()[3] - (4 if species == "thorn_imp" else 3))
    if species == "crystal_beetle":
        # The shell is redesigned, but its articulated legs/front head and
        # authored charge-impact reach continue on the exact old support rig.
        foot_cut = min(17, foot_cut)
    for y in range(foot_cut, h):
        for x in range(w):
            protected.add((x, y))
    if species == "thorn_imp":
        protected |= source_seed_mask(source)
    for p in protected:
        out.putpixel(p, source.getpixel(p))
    if species == "crystal_beetle" and (frame["anim"].startswith("attack") or frame["anim"].startswith("windup") and frame["frame"] >= 3):
        # Charge tips hit the authored hot-iris tier on the last locked aim and
        # every charge pose. Their positions follow this skin's own drawn seam,
        # rather than reinstating the original tall crystal-spike anatomy.
        seam = [(x, y) for y in range(h) for x in range(w)
                if (x, y) not in protected and out.getpixel((x, y)) in CRYSTAL[1:]]
        seam.sort(key=lambda p: sum(raw.getpixel(p)[:3]), reverse=True)
        for p in seam[:2]:
            out.putpixel(p, CRYSTAL[4])
    # One-pixel white flecks in raw generations are not intended limbs, lights,
    # or new particles. Keep authored detached dust/cue pixels, remove only new
    # detached flecks. Main pose clusters are never moved or rescaled.
    removed = 0
    for group in components(out):
        if len(group) <= 3 and not all(out.getpixel(p) == source.getpixel(p) for p in group):
            for p in group:
                if p not in protected:
                    out.putpixel(p, TRANSPARENT)
                    removed += 1
    # Reserved seed colors occur only on the authored held object, never the
    # new bark/raffia/leaf artwork.
    if species == "thorn_imp":
        assert {(x, y) for y in range(h) for x in range(w) if out.getpixel((x, y)) in SEED} == {
            (x, y) for y in range(h) for x in range(w) if source.getpixel((x, y)) in SEED}
    if species in ("sproutling", "crystal_beetle"):
        # Three crouched/lunging mushroom caps and several flat mineral-shell
        # poses end exactly at the left canvas border. Inset only their new
        # outermost edge pixels by one native column, retaining the authored
        # snout/attack reach and support rows.
        for x, y in {(0, y) for y in range(h)} | {(w - 1, y) for y in range(h)}:
            if (x, y) not in protected and not source.getpixel((x, y))[3]:
                out.putpixel((x, y), TRANSPARENT)
    if species == "crystal_beetle":
        # The new mineral body has a connected pale rim. Stray one-pixel white
        # albedo flecks beside that rim read as unmotivated particles; shade
        # those back into the adjacent matte slate limb/body. Emission tips and
        # original support/impact particles stay untouched.
        bright = Image.new("RGBA", (w, h))
        for y in range(h):
            for x in range(w):
                p = out.getpixel((x, y))
                if p[3] and p not in CRYSTAL and sum(p[:3]) > 570:
                    bright.putpixel((x, y), p)
        for group in components(bright):
            if len(group) == 1 and group[0] not in protected:
                out.putpixel(group[0], rgba("73858b"))
    for p in protected:
        assert out.getpixel(p) == source.getpixel(p)
    return out, protected, removed, foot_cut


def sha(path: Path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative(path: Path):
    return os.path.relpath(path, ROOT)


def build(out_dir: Path):
    out_dir.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {"method": "native PixelLab full-pose sheet edits; exact crop/unpack; curated material palette; exact authored support and held-seed masks", "resampling": False, "variants": []}
    for species in SPECIES:
        packing = json.loads((ART / f"{species}-packing.json").read_text())
        metadata = json.loads((ART / f"{species}-original.json").read_text())
        source_atlas = Image.open(ART / f"{species}-original.png").convert("RGBA")
        source_packed = Image.open(ART / f"{species}-packed-input.png").convert("RGBA")
        w, h = metadata["cell"]
        for variant in (1, 2):
            raw_path = ART / f"{species}_v{variant}-raw.png"
            if species == "sproutling" and variant == 1 and (ART / "sproutling_v1b-raw.png").exists():
                raw_path = ART / "sproutling_v1b-raw.png"
            if species == "crystal_beetle" and (ART / f"crystal_beetle_v{variant}b-raw.png").exists():
                raw_path = ART / f"crystal_beetle_v{variant}b-raw.png"
            if not raw_path.exists():
                print("PENDING", raw_path.name)
                continue
            raw = Image.open(raw_path).convert("RGBA")
            padded_beetle = species == "crystal_beetle" and raw.size == (256, 288)
            assert raw.size == source_packed.size or padded_beetle
            atlas = Image.new("RGBA", source_atlas.size)
            packed = Image.new("RGBA", source_packed.size)
            stat = {"species": species, "variant": variant, "look": LOOKS[species, variant], "raw": str(raw_path.relative_to(ROOT)), "raw_sha256": sha(raw_path), "frames": [], "anatomy_changes": 0, "union_opaque": 0, "changed_pixels": 0, "removed_new_speckles": 0}
            for i, frame in enumerate(packing["frames"]):
                x, y = i % packing["columns"] * w, i // packing["columns"] * h
                box = x, y, x + w, y + h
                source = source_packed.crop(box)
                # The new domes use the padded headroom. Recover their complete
                # upper contour by native cropping (1px lower for geode, 3px
                # lower for scallop); the authored support rig is restored
                # independently below. No interpolation or pixel rescaling.
                raw_x, raw_y = (x, i // packing["columns"] * 32 + (2 if variant == 1 else 0)) if padded_beetle else (x, y)
                raw_cell = raw.crop((raw_x, raw_y, raw_x + w, raw_y + h))
                seam_repair = []
                if species == "crystal_beetle" and variant == 2 and not padded_beetle and frame["anim"] == "windup_right" and frame["frame"] == 4:
                    # PixelLab omitted the violet cleft in this single held
                    # charge pose. The preceding authored brace has the same
                    # native shell position; carry only its five drawn seam
                    # pixels onto occupied shell pixels, without shifting any
                    # other pose geometry or borrowing the old spiked crest.
                    previous = raw.crop((x - w, y, x, y + h))
                    for py in range(h):
                        for px in range(w):
                            p = previous.getpixel((px, py))
                            if p[3] and p[0] >= 80 and p[2] - p[1] > 12 and p[0] - p[1] > -8 and raw_cell.getpixel((px, py))[3]:
                                raw_cell.putpixel((px, py), p)
                                seam_repair.append([px, py])
                cell, protected, removed, foot_cut = make_cell(raw_cell, source, species, frame)
                atlas.alpha_composite(cell, (frame["frame"] * w, frame["row"] * h))
                packed.alpha_composite(cell, (x, y))
                pairs = list(zip(source.get_flattened_data(), cell.get_flattened_data()))
                changed = sum(a != b for a, b in pairs)
                silhouette = sum(bool(a[3]) != bool(b[3]) for a, b in pairs)
                union = sum(bool(a[3] or b[3]) for a, b in pairs)
                emis = Counter("#%02x%02x%02x" % p[:3] for p in cell.get_flattened_data() if p in CRYSTAL[1:])
                stat["frames"].append({**frame, "changed_pixels": changed, "silhouette_pixels": silhouette, "protected_support_pixels": sum(source.getpixel(p)[3] > 0 for p in protected), "foot_cut_y": foot_cut, "native_seam_repair": seam_repair, "emissive": dict(emis) if species == "crystal_beetle" else {}})
                stat["anatomy_changes"] += silhouette
                stat["union_opaque"] += union
                stat["changed_pixels"] += changed
                stat["removed_new_speckles"] += removed
            meta = {**metadata, "colors_used": len(set(atlas.get_flattened_data())) - 1, "notes": f"Round6 {LOOKS[species, variant]}. Complete native PixelLab pose edit, no scaling. Original anchor, support rows, frame layout/timing and held-object mask retained. " + metadata.get("notes", "")}
            png_path, json_path = out_dir / f"{species}_v{variant}.png", out_dir / f"{species}_v{variant}.json"
            atlas.save(png_path)
            json_path.write_text(json.dumps(meta, indent=2) + "\n")
            packed.save(OUT / f"{species}_v{variant}-packed.png")
            for key in ("cell", "anchor", "anims", "emissive", "light"):
                assert meta.get(key) == metadata.get(key)
            stat.update({"frame_count": len(stat["frames"]), "atlas_size": list(atlas.size), "changed_fraction": stat["changed_pixels"] / stat["union_opaque"], "silhouette_fraction": stat["anatomy_changes"] / stat["union_opaque"], "png": relative(png_path), "png_sha256": sha(png_path), "json_sha256": sha(json_path), "palette_colors": len(set(atlas.get_flattened_data())) - 1})
            assert stat["changed_fraction"] > .10
            assert stat["silhouette_fraction"] > .01
            manifest["variants"].append(stat)
            print(species, variant, stat["frame_count"], "frames", round(stat["changed_fraction"] * 100, 1), "% changed", round(stat["silhouette_fraction"] * 100, 1), "% silhouette")
    (OUT / "build-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


def preview(manifest):
    # Deliberately static strips for exact native review. Enlargements use an
    # integer only; they are preview artifacts, never source artwork.
    looks = ("original", "v1", "v2")
    for scale in (1, 6):
        width = max(620, 3 * (32 * scale + 50))
        height = sum(42 + (24 if s == "sproutling" else 32 if s == "thorn_imp" else 26) * scale for s in SPECIES)
        out = Image.new("RGBA", (width, height), (28, 31, 39, 255))
        draw = ImageDraw.Draw(out)
        y = 5
        for species in SPECIES:
            w, h = json.loads((ART / f"{species}-original.json").read_text())["cell"]
            draw.text((8, y), "REVIEW CANDIDATE / " + species, fill="white")
            y += 16
            for i, look in enumerate(looks):
                p = ART / f"{species}-packed-input.png" if look == "original" else OUT / f"{species}_{look}-packed.png"
                if not p.exists():
                    continue
                im = Image.open(p).convert("RGBA").crop((0, 0, w, h))
                x = 15 + i * (width // 3)
                out.alpha_composite(im.resize((w * scale, h * scale), Image.Resampling.NEAREST), (x, y))
                draw.text((x, y + h * scale + 3), look, fill="white")
            y += h * scale + 26
        out.save(OUT / f"prepared-first-pose-{scale}x.png")
    for species in SPECIES:
        packing = json.loads((ART / f"{species}-packing.json").read_text())
        w, h = packing["cell"]
        rows = len(packing["frames"])
        meta = json.loads((ART / f"{species}-original.json").read_text())
        atlas_images = [Image.open(ART / f"{species}-original.png").convert("RGBA")]
        atlas_images += [Image.open(ROOT / "assets/sprites" / f"{species}_v{i}.png").convert("RGBA") for i in (1, 2)]
        for direction in ("right", "down", "up"):
            anims = [(a, data) for a, data in meta["anims"].items() if a.endswith(direction)]
            col_w, row_h = w * 4 + 6, h * 4 + 18
            contact = Image.new("RGBA", (6 * col_w + 145, len(anims) * 3 * row_h + 12), (28, 31, 39, 255))
            draw = ImageDraw.Draw(contact)
            for ai, (anim, a) in enumerate(anims):
                for v, im in enumerate(atlas_images):
                    y = (ai * 3 + v) * row_h + 8
                    draw.text((5, y), f"{anim} {v}", fill="white")
                    for f in range(a["frames"]):
                        cell = im.crop((f * w, a["row"] * h, (f + 1) * w, (a["row"] + 1) * h))
                        contact.alpha_composite(cell.resize((w * 4, h * 4), Image.Resampling.NEAREST), (145 + f * col_w, y))
            contact.save(OUT / f"{species}-{direction}-all-poses-4x.png")
        # Every authored pose, row-labelled and side by side, native source plus
        # both variants. Human review uses 3x, native row below is also archived.
        for scale in (1, 3):
            row_h = h * scale + 4
            out = Image.new("RGBA", (180 + w * scale * 3 + 24, row_h * rows + 20), (28, 31, 39, 255))
            draw = ImageDraw.Draw(out)
            ims = [Image.open(ART / f"{species}-packed-input.png").convert("RGBA")]
            ims += [Image.open(OUT / f"{species}_v{i}-packed.png").convert("RGBA") for i in (1, 2)]
            for i, f in enumerate(packing["frames"]):
                x0, y0 = i % 8 * w, i // 8 * h
                draw.text((3, 8 + i * row_h), f["anim"] + " " + str(f["frame"]), fill="white")
                for j, im in enumerate(ims):
                    cell = im.crop((x0, y0, x0 + w, y0 + h))
                    out.alpha_composite(cell.resize((w * scale, h * scale), Image.Resampling.NEAREST), (175 + j * (w * scale + 8), 5 + i * row_h))
            out.save(OUT / f"{species}-all-poses-{scale}x.png")
        # Native animated candidate sheets expose inter-frame identity shifts.
        for anim, a in json.loads((ART / f"{species}-original.json").read_text())["anims"].items():
            indices = [i for i, f in enumerate(packing["frames"]) if f["anim"] == anim]
            frames = []
            ims = [Image.open(ART / f"{species}-packed-input.png").convert("RGBA")]
            ims += [Image.open(OUT / f"{species}_v{i}-packed.png").convert("RGBA") for i in (1, 2)]
            for index in indices:
                out = Image.new("RGBA", ((w + 8) * 3 * 4, (h + 8) * 4), (28, 31, 39, 255))
                x0, y0 = index % 8 * w, index // 8 * h
                for j, im in enumerate(ims):
                    out.alpha_composite(im.crop((x0, y0, x0 + w, y0 + h)).resize((w * 4, h * 4), Image.Resampling.NEAREST), ((j * (w + 8) + 4) * 4, 4 * 4))
                frames.append(out.convert("RGB"))
            durations = a.get("durations", [round(60 / a["fps"])] * a["frames"])
            frames[0].save(OUT / f"{species}-{anim}-4x.gif", save_all=True, append_images=frames[1:], duration=[round(t * 1000 / 60) for t in durations], loop=0, disposal=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, default=ROOT / "assets/sprites")
    args = parser.parse_args()
    manifest = build(args.out.resolve())
    if len(manifest["variants"]) == 6:
        preview(manifest)
