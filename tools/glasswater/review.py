"""Render the current Glasswater review set from a release build, using only headless tools."""
import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    """Capture five composed views, an overview, and two real play frames."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", type=Path)
    parser.add_argument("--samples", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    args.out.mkdir(parents=True, exist_ok=True)
    views = {
        "start": "130,420",
        "lake": "360,416",
        "gallery": "540,118",
        "bridge": "840,250",
        "grove": "1220,270",
    }
    (root / "out").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="glasswater-review-", dir=root / "out") as tmp:
        tmp = Path(tmp)
        for name, pos in views.items():
            dest = tmp / name
            cmd = [str(root / "target/release/glintblade"), "zoneview", "glasswater",
                   "--at", pos, "--scale", "3", "--out", str(dest)]
            if name == "lake":
                cmd += ["--overview", "--walk", "0.25,0", "--ticks", "60", "--every", "60"]
            subprocess.run(cmd, cwd=root, check=True)
            image = sorted(dest.glob("glasswater_t*.png"))[-1]
            shutil.copyfile(image, args.out / f"{name}.png")
            if name == "lake":
                shutil.copyfile(dest / "glasswater_overview.png", args.out / "overview.png")
        for name, pos, script, tick in [
            ("combat", "1160,220", "W250 R40 A1 W10 A1 W12 A1 W20", 295),
            ("heart", "1240,644", "W280", 270),
        ]:
            dest = tmp / name
            subprocess.run([str(root / "target/release/glintblade"), "shot", "--zone", "glasswater",
                            "--pos", pos, "--script", script, "--at", str(tick), "--ticks", str(tick + 1),
                            "--god", "--scale", "3", "--out", str(dest)], cwd=root, check=True)
            shutil.copyfile(dest / f"f{tick:05}.png", args.out / f"{name}.png")
    if args.samples:
        args.samples.mkdir(parents=True, exist_ok=True)
        for name in ["lake", "gallery", "bridge", "combat"]:
            shutil.copyfile(args.out / f"{name}.png", args.samples / f"region_glasswater_{name}.png")
    print(f"Review set: {args.out.resolve()}")


if __name__ == "__main__":
    main()
