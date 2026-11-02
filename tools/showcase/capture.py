#!/usr/bin/env python3
"""Capture an honest, silent gallery through the release binary's real headless game loop."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "target/release/glintblade"
SDK = "/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk"
ASSET_DIRS = ("sprites", "normals", "music", "sfx", "fonts", "data")
ASSET_SUFFIXES = (".png", ".json", ".ogg", ".wav", ".txt")
GUARD = '''#!/bin/sh
# Retained renderer: never enter the windowed frontend by accident.
case "$1" in
  shot|zoneview) exec "$0.real" "$@";;
  *) echo "blocked: only shot and zoneview may run" >&2; exit 2;;
esac
'''


def run(command, cwd=ROOT, env=None):
    """Run a checked local command without opening any media or window."""
    print("+", " ".join(map(str, command)), flush=True)
    subprocess.run(list(map(str, command)), cwd=cwd, env=env, check=True)


def sha256(path):
    """Hash the exact bytes of one source, renderer or artifact."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_output(directory, *args):
    """Read optional Git metadata without treating an enclosing repository as the source tree."""
    try:
        result = subprocess.run(["git", *args], cwd=directory, text=True, capture_output=True)
    except OSError:
        return None
    return result.stdout.rstrip("\r\n") if result.returncode == 0 else None


def git_state(directory, paths, exact_root=False):
    """Describe the actual repository scope, leaving identity unknown for unversioned snapshots."""
    directory = directory.resolve()
    top = git_output(directory, "rev-parse", "--show-toplevel")
    if top is None:
        return {"head": None, "dirty_files": None, "repository_root": None,
                "git_context": "not_in_a_repository"}
    repository = Path(top).resolve()
    if exact_root and repository != directory:
        return {"head": None, "dirty_files": None, "repository_root": str(repository),
                "git_context": "enclosing_repository_is_not_source"}
    scope = [str(path.absolute().relative_to(repository)) for path in paths]
    if not exact_root and not git_output(repository, "ls-files", "--", *scope):
        return {"head": None, "dirty_files": None, "repository_root": str(repository),
                "git_context": "asset_scope_is_untracked", "scope": scope}
    dirty = git_output(repository, "status", "--porcelain=v1", "--untracked-files=all", "--", *scope)
    return {"head": git_output(repository, "rev-parse", "HEAD"),
            "dirty_files": None if dirty is None else dirty.splitlines(), "repository_root": str(repository),
            "git_context": "source_repository" if exact_root else "asset_repository", "scope": scope}


def source_state(asset_root=None):
    """Record source and actual asset-root Git states without borrowing a parent snapshot's HEAD."""
    asset_root = (asset_root or Path(os.environ.get("GLINT_ASSETS") or ROOT / "assets")).resolve()
    names = ("src", "build.rs", "Cargo.toml", "Cargo.lock", ".cargo", "rust-toolchain",
             "rust-toolchain.toml", "tools/showcase")
    paths = [ROOT / name for name in names]
    if asset_root == (ROOT / "assets").resolve():
        paths.extend(asset_root / name for name in ASSET_DIRS)
    return {**git_state(ROOT, paths, exact_root=True), "asset_root": str(asset_root),
            "asset_git": git_state(asset_root, [asset_root / name for name in ASSET_DIRS])}


def input_files(asset_root):
    """Enumerate build sources and the exact asset subset embedded by build.rs."""
    files = {str(p.relative_to(ROOT)): p for p in (ROOT / "src").rglob("*") if p.is_file()}
    for name in ("Cargo.toml", "Cargo.lock", "build.rs", ".cargo/config.toml",
                 "rust-toolchain", "rust-toolchain.toml", "tools/showcase/capture.py",
                 "tools/showcase/README.md"):
        if (ROOT / name).is_file():
            files[name] = ROOT / name
    for name in ASSET_DIRS:
        for p in (asset_root / name).rglob("*"):
            if p.is_file() and p.suffix in ASSET_SUFFIXES:
                files[f"assets/{p.relative_to(asset_root).as_posix()}"] = p
    return dict(sorted(files.items()))


def freeze_sources(dest):
    """Freeze and hash build inputs, rejecting a checkout or asset change during the copy."""
    asset_root = Path(os.environ.get("GLINT_ASSETS") or ROOT / "assets").resolve()
    before = source_state(asset_root)
    files = input_files(asset_root)
    hashes = {name: sha256(p) for name, p in files.items()}
    for name, p in files.items():
        output = dest / name
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, output)
    assert hashes == {name: sha256(p) for name, p in input_files(asset_root).items()}, \
        "runtime inputs changed while freezing; rerun after edits settle"
    assert hashes == {name: sha256(dest / name) for name in hashes}, "source copy changed"
    assert before == source_state(asset_root), "checkout changed while freezing; rerun after edits settle"
    return {**before, "asset_root": str(asset_root), "files_sha256": hashes,
            "snapshot": str(dest), "binary_input_relation": "built_from_frozen_inputs"}


def replay_files(out, binary, captures):
    """Write relocatable shot-only replay recipes while preserving the original command records."""
    lines = ["#!/bin/sh", "set -eu", 'cd "$(dirname "$0")"', "mkdir -p replay"]
    for capture in captures:
        command = ["./" + binary.relative_to(out).as_posix(), *capture["command"][1:]]
        command[command.index("--out") + 1] = f"replay/{capture['name']}"
        command[command.index("--report") + 1] = f"replay/{capture['name']}.json"
        capture["replay_command"] = command
        capture["replay_cwd"] = ". (bundle directory)"
        lines.append(shlex.join(command))
    (out / "REPLAY.sh").write_text("\n".join(lines) + "\n")
    (out / "REPLAY.md").write_text(
        "# Replay this final showcase\n\n"
        "Written " + datetime.now(timezone.utc).date().isoformat() + ".\n"
        "Keep this directory together; run `sh REPLAY.sh` from any directory on a compatible Mac.\n"
        "It uses only explicit headless `shot` invocations and writes fresh sequences under `replay/`.\n"
        "The renderer embeds its assets and needs no checkout or Cargo build to replay.\n"
        "The guarded renderer accepts only `shot` or `zoneview`; never invoke its `.real` companion directly.\n"
        "The manifest preserves the actual original commands and separate relocatable replay commands.\n"
        "Its `stills` entries identify each source capture and frame, plus native and 3x SHA-256 hashes.\n"
        "Native PNGs are original `--scale 1` engine files, not downsampled gallery images.\n"
        "Recreate 3x stills with the manifest's still recipe and videos with its video recipe.\n"
        "Both recipes use paths relative to this directory and require the existing ffmpeg/ffprobe tools.\n"
        "Frozen source/build inputs and their hashes are retained beside the renderer.\n"
        "Cargo dependencies and the platform toolchain are not vendored; rendering replay needs neither.\n"
    )


def probe(movie, expected_frames):
    """Require H.264 video at the intended frame rate, size and duration with no audio stream."""
    result = subprocess.run(
        ["ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", str(movie)],
        check=True, capture_output=True, text=True,
    )
    data = json.loads(result.stdout)
    streams = data["streams"]
    assert len(streams) == 1 and streams[0]["codec_type"] == "video", "audio or extra stream found"
    video = streams[0]
    assert video["codec_name"] == "h264" and video["pix_fmt"] == "yuv420p"
    assert (video["width"], video["height"]) == (1152, 648)
    assert video["avg_frame_rate"] == "15/1"
    assert int(video["nb_frames"]) == expected_frames
    assert abs(float(data["format"]["duration"]) - expected_frames / 15) < 0.1
    return data


def movie(frames, output):
    """Encode the chronological 15 fps sequence with nearest-neighbor scaling and no audio."""
    pngs = sorted(frames.glob("f*.png"))
    assert pngs, f"no frames in {frames}"
    ticks = [int(p.stem[1:]) for p in pngs]
    assert all(b - a == 4 for a, b in zip(ticks, ticks[1:])), "nonuniform frame sequence"
    run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-framerate", "15",
         "-pattern_type", "glob", "-i", frames / "f*.png", "-an",
         "-vf", "scale=1152:648:flags=neighbor", "-c:v", "libx264", "-crf", "17",
         "-pix_fmt", "yuv420p", "-movflags", "+faststart", output])
    return probe(output, len(pngs))


def still(source, output):
    """Preserve an engine PNG, scaling native captures only by exact nearest-neighbor pixels."""
    run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", source,
         "-vf", "scale=1152:648:flags=neighbor", "-frames:v", "1", "-update", "1", output])


def main():
    """Build optionally, capture title/regions/combat/the complete fight, and write a linked gallery."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "out/samples/capture_tooling")
    parser.add_argument("--build", action="store_true", help="build current embedded assets in a private target")
    parser.add_argument("--final", action="store_true", help="freeze/build/retain a reproducible final bundle")
    args = parser.parse_args()
    for tool in ("ffmpeg", "ffprobe"):
        if not shutil.which(tool):
            parser.error(f"{tool} must already be installed")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    prefix = "" if args.final else "WIP_"
    for name in ("manifest.json", "GALLERY.md", "REPLAY.sh", "REPLAY.md",
                 "14_warden_shatter.png", "14_warden_shatter_native1x.png"):
        (out / f"{prefix}{name}").unlink(missing_ok=True)
    if not (args.build or args.final) and not BIN.is_file():
        parser.error("release binary missing; pass --build")
    manifest = {"label": "Final" if args.final else "WIP",
                "captured_utc": datetime.now(timezone.utc).isoformat(),
                "invocation": [sys.executable, *sys.argv], "cwd": str(ROOT),
                "captures": [], "stills": [], "videos": {}, "video_recipes": {}}
    gallery = [f"# GLINTBLADE {'final showcase' if args.final else 'WIP capture tooling samples'}", "",
               "Actual engine frames from deterministic 60 Hz simulation.",
               "Videos are 15 fps, 3x nearest-neighbor pixels and contain no audio stream.",
               "The Warden pilot uses ordinary movement and sword inputs with god mode enabled.",
               "Region views start at selected coordinates with enemies enabled.", ""]
    with tempfile.TemporaryDirectory(prefix=".showcase-", dir=out) as tmp_name:
        tmp = Path(tmp_name)
        renderer_dir = Path(tempfile.mkdtemp(prefix="renderer-", dir=out)) if args.final else tmp
        if args.final:
            renderer_dir.chmod(0o755)
        build_root = ROOT
        if args.final:
            build_root = renderer_dir / "source"
            manifest["source"] = freeze_sources(build_root)
            manifest["source"]["snapshot"] = build_root.relative_to(out).as_posix()
        else:
            manifest["source"] = {**source_state(), "binary_input_relation":
                                  "live_checkout_build" if args.build else "unverified_existing_binary"}
        source_binary = BIN
        if args.build or args.final:
            env = {**os.environ, "SDKROOT": SDK, "CARGO_TARGET_DIR": str(tmp / "target")}
            if args.final:
                env["GLINT_ASSETS"] = str(build_root / "assets")
            command = ["cargo", "build", "--release", "--locked"]
            manifest["build"] = {"command": command, "cwd": str(build_root),
                                 "env": {k: env[k] for k in ("SDKROOT", "CARGO_TARGET_DIR", "GLINT_ASSETS") if k in env},
                                 "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
                                 "cargo": subprocess.check_output(["cargo", "-V"], text=True).strip()}
            run(command, cwd=build_root, env=env)
            source_binary = tmp / "target/release/glintblade"
        binary = renderer_dir / "glintblade"
        real_binary = binary.with_suffix(".real")
        shutil.copy2(source_binary, real_binary)
        binary.write_text(GUARD)
        binary.chmod(0o755)
        manifest["binary_sha256"] = sha256(real_binary)
        if args.final:
            manifest["renderer"] = {"path": binary.relative_to(out).as_posix(),
                                    "real_path": real_binary.relative_to(out).as_posix(),
                                    "wrapper_sha256": sha256(binary)}
            (renderer_dir / "SHA256SUMS").write_text(
                f"{sha256(real_binary)}  glintblade.real\n{sha256(binary)}  glintblade\n")

        def shot(name, options):
            """Capture into a fresh directory and retain the exact command and engine report."""
            dest = tmp / name
            report = tmp / f"{name}.json"
            command = [str(binary), "shot", "--seed", "7", *options,
                       "--out", str(dest), "--report", str(report)]
            run(command)
            data = json.loads(report.read_text())
            assert data["frames"] > 0, f"empty capture {name}"
            manifest["captures"].append({"name": name, "command": command, "cwd": str(ROOT), "report": data})
            return dest, data

        def image(name, source):
            """Save one unretouched gameplay frame and add it to the gallery."""
            output = out / f"{prefix}{name}.png"
            still(source, output)
            native = out / f"{prefix}{name}_native1x.png"
            shutil.copy2(source, native)
            manifest["stills"].append({"name": name, "capture": source.parent.name,
                                       "frame": source.name, "native1x": native.name,
                                       "native1x_sha256": sha256(native), "display3x": output.name,
                                       "display3x_sha256": sha256(output),
                                       "recipe": ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
                                                  "-i", f"replay/{source.parent.name}/{source.name}",
                                                  "-vf", "scale=1152:648:flags=neighbor", "-frames:v", "1",
                                                  "-update", "1", f"replay/{output.name}"]})
            gallery.extend([f"## {name.replace('_', ' ')}", "", f"![{name}]({output.name})", "",
                            f"[Native 384x216 PNG]({native.name})", ""])

        title, _ = shot("title", ["--ticks", "180", "--scale", "1"])
        image("01_title", title / "f00180.png")
        views = [
            ("bloomwild", "entry", None),
            ("bloomwild", "falls", "780,700"),
            ("glasswater", "lake", "360,416"),
            ("glasswater", "gallery", "540,118"),
            ("cinder", "arrival", None),
            ("cinder", "ruins", "720,420"),
            ("summit", "approach", None),
            ("summit", "arena", "620,364"),
        ]
        for i, (zone, name, pos) in enumerate(views, 2):
            # The arena's earlier pose keeps Glint clear of the falling foreground fist.
            tick = 280 if (zone, name) == ("summit", "arena") else 300
            options = ["--zone", zone, "--god", "--ticks", str(tick), "--scale", "1"]
            if pos:
                options += ["--pos", pos]
            frames, _ = shot(f"{zone}_{name}", options)
            image(f"{i:02}_{zone}_{name}", frames / f"f{tick:05d}.png")
        combat, combat_report = shot("combat", [
            "--zone", "bloomwild", "--pos", "400,644", "--god", "--scale", "1",
            "--script", "W300 D12 A1 W8 DA1 W8 DA1 W24 A60 W20 DX1 W25 U20 A1 W15 A1 W30",
            "--ticks", "600", "--from", "300", "--every", "4",
        ])
        assert combat_report["enemies_defeated"] > 0, "combat script did not defeat an enemy"
        image("10_combat", combat / "f00348.png")
        fight, report = shot("warden", [
            "--pilot", "warden", "--god", "--until-victory", "--ticks", "24000",
            "--pos", "720,520", "--pilot-delay", "300", "--from", "300",
            "--scale", "1", "--every", "4",
        ])
        assert report["scene"] == "victory" and report["victory_ready"]
        milestones = report["milestones"]
        for name in ("warden_awake", "phase_2_damage", "phase_2_heart_open", "phase_3_damage", "warden_dying"):
            assert name in milestones, f"Warden capture did not reach required milestone: {name}"
        assert milestones["warden_awake"] > 300, "the delayed pilot must awaken after the banner"
        phase_one = (milestones["warden_awake"] + 283) // 4 * 4
        assert phase_one < milestones["phase_2_damage"], "phase-one still occurs after phase-two damage"
        last = report["ticks"] // 4 * 4
        for name, tick in [
            ("11_warden_phase_1", phase_one),
            ("12_warden_phase_2", milestones["phase_2_heart_open"] + 52),
            ("13_warden_phase_3", milestones["phase_3_damage"] + 24),
            ("14_warden_overload", milestones["warden_dying"] + 192),
            ("15_victory", last),
        ]:
            tick = min(last, (tick + 3) // 4 * 4)
            image(name, fight / f"f{tick:05}.png")
        for name, frames in [("combat", combat), ("warden_fight_and_victory", fight)]:
            output = out / f"{prefix}{name}_muted.mp4"
            manifest["videos"][output.name] = movie(frames, output)
            manifest["video_recipes"][output.name] = [
                "ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-framerate", "15",
                "-pattern_type", "glob", "-i", f"replay/{frames.name}/f*.png", "-an",
                "-vf", "scale=1152:648:flags=neighbor", "-c:v", "libx264", "-crf", "17",
                "-pix_fmt", "yuv420p", "-movflags", "+faststart", f"replay/{output.name}"]
            gallery.extend([f"## {name.replace('_', ' ')}", "", f"[Muted MP4]({output.name})", ""])
        assert sha256(real_binary) == manifest["binary_sha256"], "private renderer changed during capture"
        if args.final:
            assert sha256(binary) == manifest["renderer"]["wrapper_sha256"], "renderer guard changed"
            hashes = manifest["source"]["files_sha256"]
            assert hashes == {name: sha256(build_root / name) for name in hashes}, "frozen inputs changed"
            replay_files(out, binary, manifest["captures"])
            manifest["artifacts_sha256"] = {
                p.name: sha256(p) for p in sorted(out.iterdir())
                if p.is_file() and (p.suffix in (".png", ".mp4") or p.name in ("REPLAY.sh", "REPLAY.md"))
            }
            gallery.extend(["## Reproduce these captures", "", "[Replay instructions](REPLAY.md)", "",
                            f"[Guarded renderer]({binary.relative_to(out).as_posix()})", ""])
    (out / f"{prefix}manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (out / f"{prefix}GALLERY.md").write_text("\n".join(gallery))
    print(f"Silent gallery: {out / (prefix + 'GALLERY.md')}", flush=True)


if __name__ == "__main__":
    main()
