"""Native 384x216 lit preview of the assembled Warden on summit paving, through the engine's
lighting path (stepped, dithered light and on-grid glow; port in tools/glasswater_enemies).
Light sources: the summit ambient and the Warden's own heart light. Enlarged afterwards with
nearest-neighbor only.

usage (from tools/warden): uv run --no-project --with pillow --with numpy python3 lit_preview.py
"""

import importlib.util
import json
import random
from pathlib import Path

import numpy as np
from PIL import Image

import export

ROOT = Path(__file__).resolve().parents[2]
SPR = ROOT / "assets" / "sprites"
PRE = ROOT / "assets" / "previews"

_spec = importlib.util.spec_from_file_location("engine_light", ROOT / "tools" / "glasswater_enemies" / "engine_light.py")
engine_light = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(engine_light)

PAL = json.loads((ROOT / "assets" / "concept" / "palettes.json").read_text())["regions"]["warden"]
AMBIENT = (PAL["ambient"]["color"], PAL["ambient"]["intensity"])
HORIZON = 40


def paving():
    """Summit paving rows from summit_tiles course16 blocks (running bond), sky above."""
    tiles = Image.open(SPR / "summit_tiles.png").convert("RGBA")
    man = json.loads((SPR / "summit_tiles.json").read_text())
    blocks = [man["blocks"][n] for n in man["groups"]["course16"]]
    img = Image.new("RGBA", (384, 216), (0x09, 0x0a, 0x1e, 255))
    rng = random.Random(7)
    for row, y in enumerate(range(HORIZON, 216, 16)):
        x = -32 if row % 2 else 0
        while x < 384:
            b = rng.choice(blocks)
            bx, by = b["at"]
            bw, bh = b["size"]
            piece = tiles.crop((bx * 16, by * 16, (bx + bw) * 16, (by + bh) * 16))
            if x < 0:
                piece = piece.crop((-x, 0, piece.width, piece.height))
            img.alpha_composite(piece, (max(0, x), y))
            x += bw * 16
    return img


def emissive_all():
    hexes = set()
    for name in export.SHEETS:
        hexes |= set(json.loads((SPR / f"{name}.json").read_text())["emissive"])
    hero = json.loads((SPR / "hero.json").read_text())
    return hexes, set(hero.get("emissive", []))


def scene(pose, moves=None, wx=214, wy=146):
    em, hero_em = emissive_all()
    fr = engine_light.NativeFrame(384, 216, ambient=AMBIENT)
    ground = np.array(paving())
    fr.put(ground, 0, 0)
    fr.shadow(wx, wy, 32, 6, 0.5)
    w = np.array(export.assemble(pose, fist_moves=moves))
    fr.put(w, wx - 75, wy - 132, em)
    g, ga = export.glint()
    if g is not None:
        fr.shadow(132, 188, 6, 2, 0.5)
        fr.put(np.array(g), 132 - ga[0], 188 - ga[1], hero_em)
    # the heart is the Warden's light (warden_heart.json "light")
    hl = json.loads((SPR / "warden_heart.json").read_text())["light"]
    fr.add_light(wx + 0.5, wy - 60 + 0.5, hl["radius"], hl["color"], 0.8)
    # Glint's blade glow, as in the engine (small)
    fr.add_light(140, 182, 24, "#5ff2ff", 0.5)
    return fr.render()


def main():
    shots = [
        ("idle", export.POSES["idle"], None),
        ("slam", export.POSES["slam"], export.POSE_FIST_MOVES.get("slam")),
        ("last_stand", export.POSES["last stand"], export.POSE_FIST_MOVES.get("last stand")),
    ]
    outs = []
    for name, pose, mv in shots:
        im = Image.fromarray(scene(pose, mv))
        im.save(PRE / f"warden_lit_{name}_1x.png")
        outs.append(im)
    strip = Image.new("RGB", (384, 216 * len(outs) + 4 * (len(outs) - 1)), (0, 0, 0))
    for i, im in enumerate(outs):
        strip.paste(im, (0, i * 220))
    strip.resize((strip.width * 3, strip.height * 3), Image.NEAREST).save(PRE / "warden_lit_3x.png")


if __name__ == "__main__":
    main()
