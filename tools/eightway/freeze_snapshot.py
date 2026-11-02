#!/usr/bin/env python3
"""Freeze matched native/WASM build, actor-authoring and asset inputs for audit."""
import argparse
import hashlib
import json
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def inputs():
    files = {}
    for name in ("src", "assets", "tools/eightway", "tools/hero", "art/eightway"):
        for path in (ROOT / name).rglob("*"):
            if path.is_file() and "__pycache__" not in path.parts:
                files[str(path.relative_to(ROOT))] = path
    for name in ("AGENTS.md", "Cargo.toml", "Cargo.lock", "build.rs", ".cargo/config.toml", "examples/eightway_regression.rs"):
        path = ROOT / name
        if path.is_file():
            files[name] = path
    return dict(sorted(files.items()))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    assert not args.out.exists(), "use a fresh immutable snapshot directory"
    files = inputs()
    hashes = {name: digest(path) for name, path in files.items()}
    for name, path in files.items():
        dest = args.out / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, dest)
    assert hashes == {name: digest(path) for name, path in inputs().items()}, "source/assets changed during freeze"
    assert hashes == {name: digest(args.out / name) for name in files}, "snapshot bytes differ"
    manifest = {"created_utc": datetime.now(timezone.utc).isoformat(), "source": str(ROOT),
                "snapshot": str(args.out.resolve()), "files_sha256": hashes,
                "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "git_status": subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True),
                "relation": "build and renders must use this frozen source and assets"}
    (args.out / "frozen-inputs.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"files": len(files), "snapshot": str(args.out.resolve())}))


if __name__ == "__main__":
    main()
