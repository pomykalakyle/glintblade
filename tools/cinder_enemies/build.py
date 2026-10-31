"""Rebuild the owned V2 Cinder enemy sheets and silent source samples."""

import sys
import re

import ember_glob
import glob_pal
import hound
import hound_pal
import pix
import totem
import totem_pal

pix.PREVIEWS = pix.ROOT / "out/samples/enemies_cinder_v2style/source"


def outlined(frames, no_outline, rim=None):
    """Add tinted outer contours without thickening far-side limbs."""
    out = []
    for cv in frames:
        c = cv.copy()
        c.outline(skip=no_outline)
        if rim:
            rim(c)
        out.append(c)
    return out


def build_hound():
    """Build all hound directions with the original cell, anchor and playback timings."""
    anims = [(an, outlined(frames, hound_pal.NO_OUTLINE, hound.finish), meta)
             for an, frames, meta in hound.all_anims()]
    extra = {
        "notes": "Style V2: lean ash/obsidian hound with a deep loaded chest, straight planted forelegs, a ridge of obsidian plates over high withers, a pointed ear and tapered muzzle, one hot eye and restrained salmon seams. Front keys turn the head three-quarters to the hound's right: a low projecting muzzle, one small far ear, the head joined to a loaded shoulder with the rump behind it, and a dark jaw slot with no bright mouth block. Back views are 3/4 standing poses that keep the ears and plate ridge. Palette: palettes_v2.json Cinder obsidian/basalt/lava. Original 40x24 cells, (20,20) anchor, animation rows and durations retained. Vertical windup folds the hindquarters; leap extends the spine. ember_hound.rs maps the flight frames through Lunge and holds the landing on Skid.",
        "light": {"color": "#d48670", "radius": 14, "offset": [2, -9]},
    }
    pix.write_sheet("ember_hound", (hound.CW, hound.CH), hound.ANCHOR, anims,
                    hound_pal.PAL, hound_pal.EMISSIVE, extra)


def build_totem():
    """Build a slotted basalt fin with an animated furnace outlet at the original maw origin."""
    anims = [(an, outlined(frames, totem_pal.NO_OUTLINE), meta)
             for an, frames, meta in totem.all_anims()]
    extra = {
        "notes": "Style V2: fractured basalt distributor fin with one long dark slot and a dark outlet slot lit from within (heat lines its floor and inner ends); no face, crown ornaments or copper bands. Palette: palettes_v2.json Cinder basalt/obsidian/lava. Original 28x36 cells, (14,33) anchor, facing sets, durations and footprint retained. The idle emissive centroid remains (13,20.7777778), preserving the source-derived projectile origin. Only the outlet block recoils; heat and droplets are source-bound.",
        "light": {"color": "#d48670", "radius": 28, "offset": [0, -13]},
        "footprint": [12, 5],
    }
    pix.write_sheet("spitting_totem", (totem.CW, totem.CH), totem.ANCHOR, anims,
                    totem_pal.PAL, totem_pal.EMISSIVE, extra)


def build_glob():
    """Build the folding two-lobe salmon melt and its original four-stage splash."""
    fly, pop = ember_glob.all_anims()
    anims = [(fly[0], outlined(fly[1], glob_pal.NO_OUTLINE), fly[2]), pop]
    extra = {"notes": "Style V2: source-bound salmon melt in two unequal lobes joined by a hot neck, one dark crust plate and a core deep in the big lobe; palettes_v2.json Cinder lava ramp. Original fly/pop API, 12x12 cell, (6,6) anchor and all durations retained. The engine supplies flight, trail, light and splash timing."}
    pix.write_sheet("ember_glob", (ember_glob.CW, ember_glob.CH), ember_glob.ANCHOR, anims,
                    glob_pal.PAL, glob_pal.EMISSIVE, extra)



def sync_fallback(name, poses, pal, no_outline, finish=None):
    """Keep owned Rust fallback grids byte-for-byte aligned with the authored key poses."""
    frames = outlined(poses, no_outline, finish)
    symbols = sorted({ch for cv in frames for row in cv.c for ch in row} - {"."})
    lines = [f"fn fallback() -> &'static [Cell; {len(frames)}] {{",
             f"    static CELLS: OnceLock<[Cell; {len(frames)}]> = OnceLock::new();",
             "    CELLS.get_or_init(|| {", "        let pal = ["]
    for ch in symbols:
        lines.append(f"            ('{ch}', rgb(0x{pal[ch][1:]})),")
    lines += ["        ];", "        let em = ['O', 'Y', 'W'];"]
    for i, cv in enumerate(frames):
        lines += [f"        let pose_{i} = Cell::from_ascii(", "            &["]
        lines += [f'                "{row}",' for row in cv.text().splitlines()]
        lines += ["            ],", "            &pal,", "            &em,", "        );"]
    lines += ["        [" + ", ".join(f"pose_{i}" for i in range(len(frames))) + "]",
              "    })", "}"]
    source = pix.ROOT / "src/entities/enemies" / f"{name}.rs"
    code = source.read_text()
    code = re.sub(r"fn fallback\(\).*?\n}\n", "\n".join(lines) + "\n", code, count=1, flags=re.S)
    anchor = "(20, 20)" if name == "ember_hound" else "(14, 33)"
    code = code.replace("(c.w / 2, c.h - 1)", anchor)
    # Constants formerly used only by the removed copper bands/face have no remaining callers.
    code = re.sub(r"^const (\w+): Rgb = rgb\(0x[0-9a-f]+\);\n",
                  lambda m: "" if len(re.findall(r"\b" + m[1] + r"\b", code)) == 1 else m[0],
                  code, flags=re.M)
    source.write_text(code)


if __name__ == "__main__":
    for name in sys.argv[1:] or ["hound", "totem", "glob"]:
        {"hound": build_hound, "totem": build_totem, "glob": build_glob}[name]()
        if name == "hound":
            sync_fallback("ember_hound", [hound.idle()[0][0], hound.move()[0][1],
                          hound.windup()[0][-1], hound.attack()[0][1]],
                          hound_pal.PAL, hound_pal.NO_OUTLINE, hound.finish)
        elif name == "totem":
            poses = totem.anims("down")
            sync_fallback("spitting_totem", [poses[0][1][0], poses[1][1][2]],
                          totem_pal.PAL, totem_pal.NO_OUTLINE)
