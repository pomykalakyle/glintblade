#!/usr/bin/env python3
"""Offline release evidence; never rebuilds assets, runs the game, or calls an API.

Examples (run from the repository root):
  uv run --offline --with pillow python tools/pixellab/verify_lighting_release.py normals --out art/pixellab/2026-11-01-round8/normal-audit.json
  python tools/pixellab/verify_lighting_release.py snapshot --out out/pixellab/round8/selected/sources.json
  python tools/pixellab/verify_lighting_release.py verify --sources out/pixellab/round8/selected/sources.json

All output destinations are explicit. Existing evidence is never overwritten.
Normal audits also return nonzero on mismatches, even when a failure report is saved.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
FROZEN = ROOT / "art/pixellab/2026-11-01-round7/normals"
FROZEN_MANIFEST_SHA = "2486f71e5edba3fd6a56320044dea4b7b17d881e7161daee68c39235d25edfa6"
FROZEN_LIST_SHA = "646c096e1fc3341119fa7a5a2da6d9ecf358e4a4a17ecccac5d1c921812f0953"
SHA_PATTERN = re.compile(r"[0-9a-f]{64}\Z")


class AuditError(Exception):
    pass


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def relative_file(name: str) -> Path:
    """Reject noncanonical paths, aliases outside the checkout, and missing files."""
    if not isinstance(name, str):
        raise AuditError("Input paths must be strings")
    path = Path(name)
    if path.is_absolute() or ".." in path.parts or path.as_posix() != name:
        raise AuditError(f"Noncanonical input path: {name!r}")
    full = ROOT / path
    if not full.resolve().is_relative_to(ROOT) or not full.is_file():
        raise AuditError(f"Missing or external input: {name}")
    return full


def write_json(destination: Path, value: object) -> None:
    destination = destination.resolve()
    protected = [ROOT / "assets", ROOT / "src", ROOT / "art/pixellab/2026-11-01-round7"]
    if destination.suffix != ".json" or any(destination.is_relative_to(p) for p in protected):
        raise AuditError(f"Evidence destination must be a new JSON file outside source/assets/R7: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive creation prevents silently replacing evidence from another build.
    with destination.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(value, indent=2, sort_keys=True) + "\n")


def audit_normals() -> dict:
    # Import lazily so snapshot/verify need only the Python standard library.
    from PIL import Image

    manifest = FROZEN / "selected.json"
    checksum_list = FROZEN / "selected.sha256"
    if digest(manifest) != FROZEN_MANIFEST_SHA or digest(checksum_list) != FROZEN_LIST_SHA:
        raise AuditError("Frozen R7 normal manifest/checksum-list changed; refusing a new baseline")
    entries = json.loads(manifest.read_text())
    if not isinstance(entries, list) or len(entries) != 78:
        raise AuditError("Expected exactly 78 frozen normal families")
    pinned = {}
    for line in checksum_list.read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (assets/normals/[^/]+\.png)", line)
        if not match or match[2] in pinned:
            raise AuditError("Malformed or duplicate frozen checksum entry")
        pinned[match[2]] = match[1]
    if len(pinned) != 78:
        raise AuditError("Expected exactly 78 frozen normal checksums")

    errors, pairs, names = [], [], set()
    for entry in entries:
        sheet = entry["sheet"]
        source_name, normal_name = entry["source"], entry["normal"]
        if sheet in names or source_name != f"assets/sprites/{sheet}.png" or normal_name != f"assets/normals/{sheet}.png":
            raise AuditError(f"Unexpected or duplicate frozen family: {sheet}")
        names.add(sheet)
        if pinned.get(normal_name) != entry["normal_sha256"]:
            raise AuditError(f"Frozen manifest/checksum disagreement: {normal_name}")
        pair = {"sheet": sheet, "source": source_name, "normal": normal_name, "issues": []}
        try:
            source, normal = relative_file(source_name), relative_file(normal_name)
            pair["source_sha256"], pair["normal_sha256"] = digest(source), digest(normal)
            for role in ["source", "normal"]:
                if pair[f"{role}_sha256"] != entry[f"{role}_sha256"]:
                    pair["issues"].append(f"{role} SHA256 differs from frozen R7")
            with Image.open(source) as image:
                art = image.convert("RGBA")
            with Image.open(normal) as image:
                normals = image.convert("RGBA")
            pair["source_size"], pair["normal_size"] = list(art.size), list(normals.size)
            if art.size != normals.size or list(art.size) != entry["size"]:
                pair["issues"].append("Source/normal/frozen dimensions differ")
            art_alpha, normal_alpha = art.getchannel("A").tobytes(), normals.getchannel("A").tobytes()
            if art.size == normals.size:
                pair["alpha_mismatches"] = sum(a != b for a, b in zip(art_alpha, normal_alpha))
                if pair["alpha_mismatches"]:
                    pair["issues"].append("Source and normal alpha differ")
            else:
                pair["alpha_mismatches"] = None
            pair["opaque_pixels"] = sum(a >= 128 for a in art_alpha)
        except (AuditError, OSError, ValueError) as error:
            pair["issues"].append(str(error))
        errors.extend(f"{sheet}: {issue}" for issue in pair["issues"])
        pairs.append(pair)

    actual = {p.relative_to(ROOT).as_posix() for p in (ROOT / "assets/normals").rglob("*.png")}
    expected = set(pinned)
    missing, extra = sorted(expected - actual), sorted(actual - expected)
    errors.extend(f"Missing normal family: {name}" for name in missing)
    errors.extend(f"Unexpected normal family: {name}" for name in extra)
    return {"status": "verified" if not errors else "failed", "checked_at_utc": datetime.now(timezone.utc).isoformat(),
            "scope": "Frozen R7 SHA256, all 78 albedo/normal pairs, dimensions, exact alpha, complete normal PNG inventory. No generation or rendering.",
            "frozen_manifest_sha256": FROZEN_MANIFEST_SHA, "frozen_checksum_list_sha256": FROZEN_LIST_SHA,
            "normal_pairs_checked": len(pairs), "failed_pairs": sum(bool(p["issues"]) for p in pairs),
            "missing_normals": missing, "extra_normals": extra, "errors": errors, "pairs": pairs}


def tree_files(folder: Path, extensions: set[str] | None):
    if folder.is_symlink():
        raise AuditError(f"Symlink build directory needs an explicit provenance policy: {folder}")
    for directory, subdirs, filenames in os.walk(folder, followlinks=False):
        here = Path(directory)
        for name in subdirs + filenames:
            if (here / name).is_symlink():
                raise AuditError(f"Symlink build input needs an explicit provenance policy: {here / name}")
        for name in filenames:
            path = here / name
            if extensions is None or path.suffix[1:] in extensions:
                yield path


def source_inventory() -> dict[str, str]:
    if os.environ.get("GLINT_ASSETS"):
        raise AuditError("Unset GLINT_ASSETS: this inventory covers the checkout's embedded assets")
    build_source = (ROOT / "build.rs").read_text()
    # Mirror this repository's actual build.rs rules, and fail if its structure changes.
    extension_match = re.search(r'matches!\(ext,\s*((?:"[^"\\]+"\s*\|\s*)*"[^"\\]+")\s*\)', build_source)
    subdir_match = re.search(r'for sub in\s*\[((?:\s*"[^"\\]+"\s*,?)+)\]\s*\{', build_source)
    if not extension_match or not subdir_match:
        raise AuditError("Cannot recognize build.rs asset traversal; update the inventory helper")
    extensions = set(re.findall(r'"([^"\\]+)"', extension_match[1]))
    subdirs = re.findall(r'"([^"\\]+)"', subdir_match[1])
    if not extensions or not subdirs or any(Path(s).name != s or s in {".", ".."} for s in subdirs):
        raise AuditError("Unsupported build.rs asset rules")
    files = {relative_file(name) for name in ["build.rs", "Cargo.toml", "Cargo.lock"]}
    if not (ROOT / "src").is_dir():
        raise AuditError("Missing Rust src directory")
    for folder in ["src", "examples", "tests", "benches"]:
        # Include non-Rust compile-time data too: Glasswater directly include_str!s
        # src/regions/glasswater/map.txt, outside build.rs's asset registry.
        files.update(tree_files(ROOT / folder, None))
    for folder in subdirs:
        files.update(tree_files(ROOT / "assets" / folder, extensions))
    # Local toolchain/config files affect compilation even though they aren't Rust code.
    for name in ["rust-toolchain", "rust-toolchain.toml", ".cargo/config", ".cargo/config.toml"]:
        if (ROOT / name).exists():
            files.add(relative_file(name))
    return {path.relative_to(ROOT).as_posix(): digest(path) for path in sorted(files)}


def verify_sources(source_file: Path) -> dict:
    expected = json.loads(source_file.read_text())
    if not isinstance(expected, dict) or not expected:
        raise AuditError("Sources JSON must be a nonempty path-to-SHA256 object")
    for name, sha in expected.items():
        if not isinstance(name, str) or not isinstance(sha, str) or not SHA_PATTERN.fullmatch(sha):
            raise AuditError("Sources JSON contains a malformed path or SHA256")
        path = Path(name)
        if path.is_absolute() or ".." in path.parts or path.as_posix() != name:
            raise AuditError(f"Sources JSON contains a noncanonical path: {name!r}")
    actual = source_inventory()
    added, removed = sorted(actual.keys() - expected.keys()), sorted(expected.keys() - actual.keys())
    modified = sorted(name for name in actual.keys() & expected.keys() if actual[name] != expected[name])
    return {"status": "verified" if not (added or removed or modified) else "failed",
            "sources_file": str(source_file.resolve()), "sources_sha256": digest(source_file),
            "files_checked": len(actual), "added": added, "removed": removed, "modified": modified}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    normals = commands.add_parser("normals", help="Check frozen maps and save a new report; requires Pillow")
    normals.add_argument("--out", required=True, type=Path)
    snapshot = commands.add_parser("snapshot", help="Pin the complete current Rust/embedded-asset inventory")
    snapshot.add_argument("--out", required=True, type=Path, help="New selected/sources.json destination")
    verify = commands.add_parser("verify", help="Compare complete current inventory to a saved snapshot")
    verify.add_argument("--sources", required=True, type=Path)
    verify.add_argument("--out", type=Path, help="Optional new JSON report; otherwise only print the result")
    args = parser.parse_args()
    try:
        if args.command == "snapshot":
            pins = source_inventory()
            write_json(args.out, pins)
            result = {"status": "snapshot_saved", "files_pinned": len(pins), "out": str(args.out.resolve()), "sha256": digest(args.out)}
        else:
            result = audit_normals() if args.command == "normals" else verify_sources(args.sources)
            if args.out:
                write_json(args.out, result)
            result = {key: value for key, value in result.items() if key != "pairs"}
            if args.out:
                result["out"] = str(args.out.resolve())
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result["status"] in {"verified", "snapshot_saved"} else 1
    except (AuditError, OSError, ValueError, KeyError, ImportError) as error:
        print(f"Release audit failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
