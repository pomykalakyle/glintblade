//! Style V2 Cinder: soot-black graft over the ash, obsidian and salmon ramps in palettes_v2.json.
//! The extra soot shades hold the dark value structure of the approved Cinder C mock.

use crate::gfx::{Rgb, rgb};

// ash: the steppe floor
pub const A0: Rgb = rgb(0x16151d);
pub const A1: Rgb = rgb(0x24232d);
pub const A2: Rgb = rgb(0x363340);
pub const A3: Rgb = rgb(0x514854);
pub const A4: Rgb = rgb(0x6f616b);
pub const A5: Rgb = rgb(0x8b7c81);
// scorched grass and burnt soil
pub const G0: Rgb = rgb(0x292c3c);
pub const G1: Rgb = rgb(0x393746);
pub const G2: Rgb = rgb(0x514854);
pub const G3: Rgb = rgb(0x6f616b);
pub const G4: Rgb = rgb(0x79707a);
// obsidian: the ruins
pub const O0: Rgb = rgb(0x101017);
pub const O1: Rgb = rgb(0x1a1923);
pub const O2: Rgb = rgb(0x292c3c);
pub const O3: Rgb = rgb(0x3f414f);
pub const O4: Rgb = rgb(0x5b5966);
pub const O5: Rgb = rgb(0x7c7780);
// basalt: rock masses and cliffs
pub const B0: Rgb = rgb(0x1a1923);
pub const B1: Rgb = rgb(0x292c3c);
pub const B2: Rgb = rgb(0x393746);
pub const B3: Rgb = rgb(0x56505d);
pub const B4: Rgb = rgb(0x79707a);
// lava (L3..L5 are emissive)
pub const L0: Rgb = rgb(0x24232d);
pub const L1: Rgb = rgb(0x613039);
pub const L2: Rgb = rgb(0xa64c3b);
pub const L3: Rgb = rgb(0xd8784d);
pub const L4: Rgb = rgb(0xf2ae66);
pub const L5: Rgb = rgb(0xffdda0);
// dusk sky
pub const S0: Rgb = rgb(0x16151d);
pub const S1: Rgb = rgb(0x24232d);
pub const S2: Rgb = rgb(0x363340);
pub const S3: Rgb = rgb(0x393746);
pub const S4: Rgb = rgb(0x514854);
pub const S5: Rgb = rgb(0x606170);
// copper inlays
pub const C0: Rgb = rgb(0x292c3c);
pub const C1: Rgb = rgb(0x393746);
pub const C2: Rgb = rgb(0x63515a);
pub const C3: Rgb = rgb(0x93766f);
pub const C4: Rgb = rgb(0xb69b85);

pub const ASH_R: [Rgb; 6] = [A0, A1, A2, A3, A4, A5];
pub const GRASS_R: [Rgb; 5] = [G0, G1, G2, G3, G4];
pub const OBS_R: [Rgb; 6] = [O0, O1, O2, O3, O4, O5];
pub const BAS_R: [Rgb; 5] = [B0, B1, B2, B3, B4];
pub const LAVA_R: [Rgb; 6] = [L0, L1, L2, L3, L4, L5];
pub const COP_R: [Rgb; 5] = [C0, C1, C2, C3, C4];
pub const SKY_R: [Rgb; 6] = [S0, S1, S2, S3, S4, S5];

/// Ramp ids for the palette-index ground buffer.
pub const RA: u8 = 0;
pub const RG: u8 = 1;
pub const RO: u8 = 2;
pub const RB: u8 = 3;
pub const RL: u8 = 4;
pub const RC: u8 = 5;
pub const RS: u8 = 6;

/// Select the material color ramp.
pub fn ramp(r: u8) -> &'static [Rgb] {
    match r {
        RA => &ASH_R,
        RG => &GRASS_R,
        RO => &OBS_R,
        RB => &BAS_R,
        RL => &LAVA_R,
        RC => &COP_R,
        _ => &SKY_R,
    }
}

/// Color at `level` of ramp `r`, clamped to the ramp.
pub fn rc(r: u8, level: i32) -> Rgb {
    let rp = ramp(r);
    rp[level.clamp(0, rp.len() as i32 - 1) as usize]
}

pub const LAVA_LIGHT: Rgb = rgb(0xf2ae66);
pub const EMBER: [Rgb; 2] = [rgb(0xd8784d), rgb(0xf2ae66)];

/// A small stamp: rows of 'h' (light), 'm' (mid), 'd' (dark), '.' (none).
pub type Stamp = &'static [&'static str];

pub const PEBBLES: [Stamp; 4] = [
    &["hm", "md"],
    &["hmm", "mmd"],
    &[".hm", "hmd", "dd."],
    &["hm.", "mmd", ".dd"],
];

pub const STONES: [Stamp; 4] = [
    &[".hm.", "hmmd", ".dd."],
    &[".hmm.", "hmmmd", "mmmdd", ".ddd."],
    &["..hm..", ".hmmm.", "hmmmmd", "mmmmdd", ".mddd."],
    &[".hhm.", "hmmmd", "mmmdd", ".dd.."],
];

/// Scorched grass tufts: '1' darkest .. '4' brightest tip. Light from the upper left, so the
/// left blades carry the light tips. The bottom row sits on the ground.
pub const TUFTS: [Stamp; 6] = [
    &[
        "...4...", ".3..3.3", "..3.32.", "3.232.2", ".23221.", "..1211.",
    ],
    &[".4.3.", "3.3.2", ".323.", "22321", ".121."],
    &["3..3..", ".3.2.2", "232221", ".1211."],
    &["..4..3", "3.3.2.", ".3232.", "232221", ".1211."],
    &["3.3", ".32", "221", "11."],
    &[".3...", "3.3.3", ".32.2", "22221", "1211."],
];

/// Return the native pixel dimensions of a hand-authored stamp.
pub fn stamp_size(s: Stamp) -> (i32, i32) {
    (
        s.iter().map(|r| r.len()).max().unwrap_or(0) as i32,
        s.len() as i32,
    )
}
