#!/usr/bin/env python3
"""Publish a WASM build with a verifiable, content-addressed browser URL.

The manifest is replaced last, after both the immutable file and the legacy
glintblade.wasm URL are ready. This script only reads Git metadata; it never
changes the checkout or contacts a remote.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def build_label() -> str:
    root = Path(__file__).resolve().parents[2]
    try:
        commit = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        dirty = subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return "local"
    return f"{commit}-dirty" if dirty else commit


def atomic_write(path: Path, data: bytes) -> None:
    """Replace one complete file, without exposing a partially written file."""
    if path.exists() and path.read_bytes() == data:
        return
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary_path = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
            os.fchmod(output.fileno(), 0o644)
        os.replace(temporary_path, path)
    finally:
        temporary_path.unlink(missing_ok=True)


def prepare(wasm_path: Path, out_dir: Path, controls_revision: int) -> dict:
    wasm = wasm_path.read_bytes()
    if len(wasm) < 8 or wasm[:8] != b"\x00asm\x01\x00\x00\x00":
        raise ValueError(f"{wasm_path} is not a WebAssembly version 1 module")
    if controls_revision < 1:
        raise ValueError("controls revision must be positive")

    digest = hashlib.sha256(wasm).hexdigest()
    filename = f"glintblade.{digest[:16]}.wasm"
    manifest = {
        "schema": 1,
        "build": build_label(),
        "wasm": filename,
        "sha256": digest,
        "controls_revision": controls_revision,
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    immutable_path = out_dir / filename
    if immutable_path.exists() and immutable_path.read_bytes() != wasm:
        raise ValueError(f"refusing to replace different bytes at {immutable_path}")

    atomic_write(immutable_path, wasm)
    # Existing cached player scripts still request this compatibility URL.
    atomic_write(out_dir / "glintblade.wasm", wasm)
    atomic_write(
        out_dir / "build-info.json",
        (json.dumps(manifest, indent=2) + "\n").encode("utf-8"),
    )
    return manifest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wasm", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, default=Path("web"))
    parser.add_argument("--controls-revision", type=int, default=13)
    args = parser.parse_args()
    try:
        manifest = prepare(args.wasm, args.out_dir, args.controls_revision)
    except (OSError, ValueError) as error:
        parser.exit(1, f"prepare_web_build: {error}\n")
    print(json.dumps(manifest, sort_keys=True))


if __name__ == "__main__":
    main()
