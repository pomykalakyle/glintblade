//! Water for Glasswater Hollow: a ripple height field that anything wading disturbs, drawn
//! over the painted lakebed with crisp palette steps (no blending), plus shore lapping,
//! wobbling reflections of the crystals and mushrooms on the shore.

use super::art::{VIOLET, WATER};
use super::paint::{DEEP, DEEP_WATER, Fields, PixMap, SHALLOW, deep_water_level};
use super::{Decor, Kind};
use crate::gfx::{Cell, Frame, H, Rgb, W, rgb};
use crate::util::{bayer4, hash2};

/// Ripple grid cell size in world pixels. Heights are sampled bilinearly per pixel, so ring
/// edges stay 1 px crisp even though the simulation runs at half resolution.
const CELL: i32 = 2;
/// Wave speed squared (cells per tick, squared). Must stay below 0.5 for stability.
const C2: f32 = 0.30;
const DAMP: f32 = 0.972;
/// The open deep surface inside a lake interval: slate between WATER[0] and WATER[1].
const OPEN_DEEP: Rgb = rgb(0x234255);
/// Broad, unequal horizontal intervals on the lake's open deep surface, in world pixels:
/// (y0, y1, x0, x1, left end step per row, right end step per row, drift phase).
/// Heights and gaps grow toward the near shore so the dark body reads as one level plane.
const LAKE_INTERVALS: [(i32, i32, i32, i32, i32, i32, u32); 7] = [
    (357, 359, 426, 512, -1, 0, 0),
    (366, 369, 468, 584, 1, -2, 97),
    (381, 385, 394, 470, -2, -1, 171),
    (409, 413, 552, 602, 1, 0, 233),
    (440, 446, 418, 498, -1, 1, 59),
    (476, 481, 452, 566, 2, -1, 310),
    (499, 502, 510, 560, 1, -2, 140),
];
/// Short paired catches beside the lake's crystals: (y, x0, x1, drift phase). Inside a
/// crystal's reach they take WATER[2]; elsewhere they step one ramp colour up.
const LAKE_CATCHES: [(i32, i32, i32, u32); 7] = [
    (339, 361, 380, 0),
    (341, 367, 373, 61),
    (412, 471, 490, 90),
    (414, 477, 484, 151),
    (427, 549, 573, 260),
    (430, 556, 563, 311),
    (423, 652, 670, 140),
];
/// Ticks per drift cycle of the lake intervals and catches (7 s at 60 Hz).
const LAKE_PERIOD: f32 = 420.0;

/// A crest coordinate baked from a physical crystal and the receiving water mask.
#[derive(Clone, Copy)]
struct SourceCrest {
    radius: u8,
    phase: u8,
    glint: bool,
}

/// An island's column waterlines and reflected rock support, built from the painted bank.
struct BankMirror {
    source: (i32, i32),
    left: i32,
    lines: Vec<i32>,
    pixels: Vec<(i32, i32, usize)>,
}

pub struct Water {
    pw: i32,
    ph: i32,
    /// Per world pixel: 255 = no water; else the lakebed's WATER ramp index (0..=3), +8 if deep.
    bed: Vec<u8>,
    /// Baked underwater terrace shade (0..=3), independent of ripple/shore cue indices.
    depth_tone: Vec<u8>,
    /// Per world pixel: distance to land (px), capped.
    dland: Vec<u8>,
    /// Selected shore segments share a world-anchored phase, never a screen-space pattern.
    shore: Vec<u8>,
    source: Vec<Option<SourceCrest>>,
    /// Sparse horizontal surface catches, baked into the actual receiving water.
    currents: Vec<u8>,
    mirrors: Vec<BankMirror>,
    gw: i32,
    gh: i32,
    cur: Vec<f32>,
    prev: Vec<f32>,
    wet: Vec<bool>,
    active: Vec<usize>,
}

impl Water {
    /// Bake the wet mask, shore segments and motivated reflection geometry once at load.
    pub fn new(m: &PixMap, fl: &Fields, ground: &[Rgb], decor: &[Decor]) -> Water {
        let (pw, ph) = (m.w, m.h);
        let mut bed = vec![255u8; (pw * ph) as usize];
        let mut depth_tone = vec![0u8; bed.len()];
        for i in 0..bed.len() {
            let c = m.cls[i];
            if c == SHALLOW || c == DEEP {
                let g = ground[i];
                // match the painted lakebed back to its ramp index
                let mut best = 0;
                let mut bd = f32::MAX;
                for (k, w) in WATER.iter().enumerate() {
                    let d = (w.r - g.r).abs() + (w.g - g.g).abs() + (w.b - g.b).abs();
                    if d < bd {
                        bd = d;
                        best = k;
                    }
                }
                if c == DEEP {
                    let (x, y) = (i as i32 % pw, i as i32 / pw);
                    let d = fl.d_land[i];
                    bed[i] = 8 + u8::from(d < 8 && m.rock(x, y - d as i32 - 2));
                    depth_tone[i] = deep_water_level(fl, x, y, i) as u8;
                } else {
                    bed[i] = best.min(3) as u8;
                }
            }
        }
        // Propagate nearest dry coordinates only through the receiving shore and its quiet outgoing wave band.
        // This lets a lap follow the actual ragged bank instead of a world-grid rectangle.
        let mut nearest = vec![(0i32, 0i32); bed.len()];
        let mut bands = vec![Vec::new(); 96];
        for y in 0..ph {
            for x in 0..pw {
                let i = (y * pw + x) as usize;
                if bed[i] == 255 {
                    nearest[i] = (x, y);
                } else if fl.d_land[i] < 96 {
                    bands[fl.d_land[i] as usize].push(i);
                }
            }
        }
        let mut shore = vec![255; bed.len()];
        for band in bands.iter().skip(1) {
            for &i in band {
                let (x, y) = (i as i32 % pw, i as i32 / pw);
                for (nx, ny) in [
                    (x, y - 1),
                    (x - 1, y),
                    (x + 1, y),
                    (x, y + 1),
                    (x - 1, y - 1),
                    (x + 1, y - 1),
                    (x - 1, y + 1),
                    (x + 1, y + 1),
                ] {
                    if nx < 0 || ny < 0 || nx >= pw || ny >= ph {
                        continue;
                    }
                    let j = (ny * pw + nx) as usize;
                    if fl.d_land[j] < fl.d_land[i] {
                        nearest[i] = nearest[j];
                        break;
                    }
                }
                let (bx, by) = nearest[i];
                let seed = hash2(bx.div_euclid(28), by.div_euclid(22), 731);
                if seed > 0.80 {
                    shore[i] = (hash2(bx.div_euclid(28), by.div_euclid(22), 741) * 128.0) as u8;
                }
            }
        }
        let mut source = vec![None; bed.len()];
        let mut currents = vec![0; bed.len()];
        for y in 0..ph {
            let row = y.div_euclid(19);
            let line = 3 + (hash2(row, 0, 877) * 10.0) as i32;
            if y.rem_euclid(19) != line {
                continue;
            }
            for x in 0..pw {
                let i = (y * pw + x) as usize;
                if bed[i] == 255 || fl.d_land[i] < 5 {
                    continue;
                }
                let col = (x + row * 29).div_euclid(79);
                let start = 7 + (hash2(col, row, 879) * 24.0) as i32;
                let length = 8 + (hash2(col, row, 881) * 16.0) as i32;
                let local = (x + row * 29).rem_euclid(79);
                if (start..start + length).contains(&local) {
                    currents[i] = 1;
                }
            }
        }
        let mut mirrors = Vec::new();
        for d in decor {
            if d.kind != Kind::Crystal {
                continue;
            }
            let Some((_, radius, _, _)) = d.light else {
                continue;
            };
            let (cx, cy) = (d.x.floor() as i32, d.y.floor() as i32);
            // Glints stay inside the physical source's reach, with no ambient sparkle field.
            let reach = radius.ceil() as i32;
            for y in cy.max(0)..(cy + reach).min(ph) {
                for x in (cx - reach).max(0)..(cx + reach).min(pw) {
                    let i = (y * pw + x) as usize;
                    let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
                    if bed[i] == 255 || dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    let r = (dx * dx * 0.25 + dy * dy).sqrt();
                    if r < 5.0 || r > 42.0 || dy < 2.0 || dx.abs() < dy * 0.25 {
                        continue;
                    }
                    let crest = SourceCrest {
                        radius: (r * 4.0).round() as u8,
                        phase: ((cx + cy).rem_euclid(96)) as u8,
                        glint: dx.abs() < 16.0 && fl.d_land[i] > 2,
                    };
                    if source[i].is_none_or(|old: SourceCrest| crest.radius < old.radius) {
                        source[i] = Some(crest);
                    }
                }
            }
            if let Some(mirror) = d.mirror {
                let ax = if d.flip {
                    d.cell.w - 1 - d.anchor.0
                } else {
                    d.anchor.0
                };
                let left = cx - ax;
                let mut lines = Vec::new();
                let mut pixels = Vec::new();
                for x in left..left + d.cell.w {
                    let line = (cy..mirror.ceil() as i32 + 8)
                        .find(|&y| {
                            x >= 0
                                && x < pw
                                && y >= 0
                                && y < ph
                                && bed[(y * pw + x) as usize] != 255
                        })
                        .unwrap_or(mirror.floor() as i32);
                    lines.push(line);
                    for y in cy..line {
                        let wy = 2 * line - 1 - y;
                        if x < 0
                            || x >= pw
                            || y < 0
                            || wy >= ph
                            || bed[(wy * pw + x) as usize] == 255
                        {
                            continue;
                        }
                        let l = ground[(y * pw + x) as usize].luma();
                        pixels.push((
                            x,
                            wy,
                            if l > 0.70 && x < cx {
                                2
                            } else if l > 0.28 {
                                1
                            } else {
                                0
                            },
                        ));
                    }
                }
                mirrors.push(BankMirror {
                    source: (cx, cy),
                    left,
                    lines,
                    pixels,
                });
            }
        }
        let gw = pw / CELL + 1;
        let gh = ph / CELL + 1;
        let mut wet = vec![false; (gw * gh) as usize];
        for gy in 0..gh {
            for gx in 0..gw {
                let (x, y) = ((gx * CELL).min(pw - 1), (gy * CELL).min(ph - 1));
                wet[(gy * gw + gx) as usize] = bed[(y * pw + x) as usize] != 255;
            }
        }
        // The outermost samples stay dry so the stencil can access all four neighbours.
        let mut active = Vec::new();
        for gy in 0..gh {
            for gx in 0..gw {
                let i = (gy * gw + gx) as usize;
                if gx == 0 || gy == 0 || gx == gw - 1 || gy == gh - 1 {
                    wet[i] = false;
                }
                if wet[i] {
                    active.push(i);
                }
            }
        }
        Water {
            pw,
            ph,
            bed,
            depth_tone,
            dland: fl.d_land.clone(),
            shore,
            source,
            currents,
            mirrors,
            gw,
            gh,
            cur: vec![0.0; (gw * gh) as usize],
            prev: vec![0.0; (gw * gh) as usize],
            wet,
            active,
        }
    }

    #[inline]
    pub fn is_water(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && x < self.pw
            && y < self.ph
            && self.bed[(y * self.pw + x) as usize] != 255
    }

    /// Push the surface down in a small disc at world (x, y).
    pub fn poke(&mut self, x: f32, y: f32, r: f32, amount: f32) {
        let (cx, cy) = (x / CELL as f32, y / CELL as f32);
        let rr = (r / CELL as f32).max(0.6);
        let ri = rr.ceil() as i32;
        for gy in (cy as i32 - ri)..=(cy as i32 + ri) {
            for gx in (cx as i32 - ri)..=(cx as i32 + ri) {
                if gx < 1 || gy < 1 || gx >= self.gw - 1 || gy >= self.gh - 1 {
                    continue;
                }
                let d =
                    ((gx as f32 + 0.5 - cx).powi(2) + (gy as f32 + 0.5 - cy).powi(2)).sqrt() / rr;
                if d < 1.0 {
                    let i = (gy * self.gw + gx) as usize;
                    if self.wet[i] {
                        self.cur[i] -= amount * (1.0 - d * d);
                    }
                }
            }
        }
    }

    /// Advance every wet sample so waves continue and decay while the camera is elsewhere.
    pub fn step(&mut self, _cam: (i32, i32)) {
        let gw = self.gw as usize;
        for &i in &self.active {
            let c = self.cur[i];
            // Dry neighbours reflect the wave instead of draining the shoreline.
            let n = |j: usize| if self.wet[j] { self.cur[j] } else { c };
            let lap = n(i - 1) + n(i + 1) + n(i - gw) + n(i + gw) - 4.0 * c;
            self.prev[i] = (2.0 * c - self.prev[i] + C2 * lap) * DAMP;
        }
        std::mem::swap(&mut self.cur, &mut self.prev);
    }

    #[inline]
    fn h(&self, wx: f32, wy: f32) -> f32 {
        let fx = wx / CELL as f32 - 0.5;
        let fy = wy / CELL as f32 - 0.5;
        let x0 = fx.floor() as i32;
        let y0 = fy.floor() as i32;
        if x0 < 0 || y0 < 0 || x0 >= self.gw - 1 || y0 >= self.gh - 1 {
            return 0.0;
        }
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let i = (y0 * self.gw + x0) as usize;
        let g = self.gw as usize;
        let a = self.cur[i] + (self.cur[i + 1] - self.cur[i]) * tx;
        let b = self.cur[i + g] + (self.cur[i + g + 1] - self.cur[i + g]) * tx;
        a + (b - a) * ty
    }

    /// Surface slope toward the light (upper left) at a world pixel; > 0 = lit side of a ripple.
    #[inline]
    pub fn slope(&self, wx: i32, wy: i32) -> (f32, f32) {
        let x = wx as f32 + 0.5;
        let y = wy as f32 + 0.5;
        let h0 = self.h(x, y);
        (self.h(x + 1.0, y) - h0, self.h(x, y + 1.0) - h0)
    }

    /// Draw the water surface over the painted lakebed.
    pub fn draw(&self, f: &mut Frame, t: u64) {
        for sy in 0..H as i32 {
            let wy = sy + f.cam_y;
            if wy < 0 || wy >= self.ph {
                continue;
            }
            let offset = ((t as f32 / 240.0 + wy as f32 * 0.04).sin()).round() as i32;
            for sx in 0..W as i32 {
                let wx = sx + f.cam_x;
                if wx < 0 || wx >= self.pw {
                    continue;
                }
                let i = (wy * self.pw + wx) as usize;
                let b = self.bed[i];
                if b == 255 {
                    continue;
                }
                let deep = b >= 8;
                let (dx, dy) = self.slope(wx, wy);
                let s = dx + dy;
                // refraction: the bed wobbles under the ripples
                let ox = (dx * 5.0).round().clamp(-2.0, 2.0) as i32;
                let oy = (dy * 5.0).round().clamp(-2.0, 2.0) as i32;
                let mut idx = if ox != 0 || oy != 0 {
                    let j = ((wy + oy).clamp(0, self.ph - 1) * self.pw
                        + (wx + ox).clamp(0, self.pw - 1)) as usize;
                    let bb = self.bed[j];
                    if bb == 255 { b & 7 } else { bb & 7 }
                } else {
                    b & 7
                } as i32;
                let dl = self.dland[i] as i32;
                // Unequal quiet intervals between short laps; never trace the whole shore.
                let phase = self.shore[i];
                if phase != 255 {
                    let beat = (t / 5 + phase as u64) % 64;
                    let reach = 1 + (beat / 8) as i32;
                    if beat < 40 && dl == reach && (!deep || dl <= 5) {
                        idx = 2;
                    }
                }
                // Flattened, broken arcs belong only to receiving water near a crystal.
                let mut glint = false;
                if let Some(crest) = self.source[i] {
                    let travel = ((t / 3 + crest.phase as u64) % 192) as i32;
                    let offset = (crest.radius as i32 - (20 + travel)).abs();
                    if offset <= if crest.glint { 3 } else { 1 } {
                        idx = 2;
                        glint = crest.glint && offset <= 1;
                    }
                }
                // Short horizontal catches lie on the plane across shallow and deep alike.
                let open = deep && b & 7 == 0;
                let mx = (wx + offset).clamp(0, self.pw - 1);
                let caught = lake_catch(wx, wy, t)
                    || self.currents[(wy * self.pw + mx) as usize] != 0 && dl > 4;
                if caught {
                    idx = if self.source[i].is_some() {
                        2
                    } else {
                        (idx + 1).min(2)
                    };
                }
                // ripple shading in crisp steps
                let th = 0.085 + bayer4(wx, wy) * 0.008;
                let mut dark = false;
                if s > th {
                    idx = (idx + 1).min(2);
                } else if s < -th * 1.6 {
                    idx = (idx - 1).max(0);
                    dark = true;
                }
                let surface = if open && idx == 0 && !dark && !caught {
                    let base = DEEP_WATER[self.depth_tone[i] as usize];
                    if lake_interval(wx, wy, t) {
                        base.lerp(OPEN_DEEP, 0.48)
                    } else {
                        base
                    }
                } else {
                    WATER[idx.clamp(0, 2) as usize]
                };
                let screen = sy as usize * W + sx as usize;
                f.albedo[screen] = if glint { VIOLET[2] } else { surface };
                // Actors and gates share the real water plane. The landmarks use their
                // column-specific reflection below, so region scenery opts out of this pass.
                f.water[screen] = 255;
                f.water_any = true;
            }
        }
    }

    /// A wobbling, darkened mirror image of a sprite standing at world (x, y), drawn only on water.
    /// Emissive pixels reflect as dimmer glow.
    #[allow(clippy::too_many_arguments)]
    pub fn reflect(
        &self,
        f: &mut Frame,
        cell: &Cell,
        anchor: (i32, i32),
        x: f32,
        y: f32,
        flip: bool,
        emit_gain: f32,
        t: u64,
    ) {
        let ax = if flip {
            cell.w - 1 - anchor.0
        } else {
            anchor.0
        };
        let base_x = x.floor() as i32 - ax;
        let base_y = y.floor() as i32;
        let tf = t as f32;
        let mirror = self
            .mirrors
            .iter()
            .find(|m| m.source == (x.floor() as i32, base_y));
        if let Some(m) = mirror {
            for &(wx, wy, idx) in &m.pixels {
                let depth = wy - m.lines[(wx - m.left) as usize];
                if (8..=10).contains(&depth)
                    || (18..=20).contains(&depth)
                    || depth > 22 && depth % 4 >= 2
                {
                    continue;
                }
                // Keep contact fixed; only the lower reflected bank shears with the surface.
                let wob = if depth <= 2 {
                    0
                } else {
                    ((wy as f32 * 0.15 + tf * 0.025).sin() * 0.7).round() as i32
                };
                let (sx, sy) = (wx + wob - f.cam_x, wy - f.cam_y);
                if self.is_water(wx + wob, wy)
                    && (0..W as i32).contains(&sx)
                    && (0..H as i32).contains(&sy)
                {
                    f.albedo[sy as usize * W + sx as usize] = WATER[idx];
                }
            }
        }
        for cy in (0..cell.h).rev() {
            let hgt = anchor.1 - cy; // height above the base line
            if hgt < 0 {
                continue;
            }
            let wy = base_y + hgt;
            // Broad unequal water intervals break the reflected vertical mass into a surface.
            let broken = if mirror.is_some() {
                (9..=11).contains(&hgt)
                    || (24..=26).contains(&hgt)
                    || (39..=42).contains(&hgt)
                    || hgt > 29 && hgt % 4 >= 2
            } else {
                matches!(hgt, 12 | 27 | 43)
            };
            if broken {
                continue;
            }
            let (rdx, _) = self.slope(base_x + cell.w / 2, wy);
            let wob = ((wy as f32 * 0.15 + tf * 0.025).sin() * 0.7 + rdx * 6.0).round() as i32;
            for cx in 0..cell.w {
                let p = cell.px[(cy * cell.w + cx) as usize];
                if !p.a {
                    continue;
                }
                let lx = if flip { cell.w - 1 - cx } else { cx };
                let wx = base_x + lx + wob;
                let line = mirror
                    .map(|m| m.lines[(base_x + lx - m.left) as usize])
                    .unwrap_or(base_y);
                let wy = 2 * line - base_y + hgt;
                let sy = wy - f.cam_y;
                if !(0..H as i32).contains(&sy) || !self.is_water(wx, wy) {
                    continue;
                }
                let sx = wx - f.cam_x;
                if !(0..W as i32).contains(&sx) {
                    continue;
                }
                let k = sy as usize * W + sx as usize;
                let fade = (hgt as f32 / 70.0).min(0.6);
                if p.emit {
                    f.albedo[k] = WATER[2];
                    f.emissive[k] += p.c.scale(emit_gain * (0.32 - fade * 0.25));
                } else {
                    // Keep non-emissive reflected facets on the darker water ramp near bright sources.
                    let l = p.c.luma();
                    let idx = if l > 0.38 {
                        2
                    } else if l > 0.20 {
                        1
                    } else {
                        0
                    };
                    f.albedo[k] = if mirror.is_some() && hgt < 18 && idx > 0 {
                        VIOLET[idx - 1]
                    } else {
                        WATER[if mirror.is_some() && hgt >= 18 {
                            idx.min(1)
                        } else {
                            idx
                        }]
                    };
                }
            }
        }
    }
}

/// Sideways drift in px for a lake mark with the given phase at tick t.
fn lake_drift(t: u64, phase: u32) -> (i32, f32) {
    let a = (t as f32 + phase as f32) / LAKE_PERIOD * std::f32::consts::TAU;
    (a.sin().round() as i32, a)
}

/// True when a broad lake interval covers world (wx, wy) at tick t.
fn lake_interval(wx: i32, wy: i32, t: u64) -> bool {
    LAKE_INTERVALS
        .iter()
        .any(|&(y0, y1, x0, x1, sl, sr, phase)| {
            if wy < y0 || wy >= y1 {
                return false;
            }
            let (off, _) = lake_drift(t, phase);
            let r = wy - y0;
            (x0 + sl * r + off..x1 + sr * r + off).contains(&wx)
        })
}

/// True when a lake catch covers world (wx, wy) at tick t; alternate ends breathe by up to 2 px.
fn lake_catch(wx: i32, wy: i32, t: u64) -> bool {
    LAKE_CATCHES.iter().any(|&(y, x0, x1, phase)| {
        if wy != y {
            return false;
        }
        let (off, a) = lake_drift(t, phase);
        let trim = (((a * 0.5 + 1.3).sin() * 0.5 + 0.5) * 2.0).round() as i32;
        let (lo, hi) = if phase % 2 == 0 {
            (x0 + off + trim, x1 + off)
        } else {
            (x0 + off, x1 + off - trim)
        };
        (lo..hi).contains(&wx)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Tile, Zone, ZoneId};

    /// Check wading excites a stable field that decays even when its pool is off camera.
    #[test]
    fn ripples_decay_offscreen() {
        let mut z = Zone::new(ZoneId::Glasswater, 8, 8);
        z.tiles.fill(Tile::Shallow);
        let m = super::super::paint::classify(&z);
        let fl = super::super::paint::fields(&m);
        z.ground.fill(WATER[1]);
        let mut water = Water::new(&m, &fl, &z.ground, &[]);
        water.poke(64.0, 64.0, 4.0, 2.0);
        let before: f32 = water.cur.iter().map(|v| v.abs()).sum();
        assert!(before > 0.0);
        for _ in 0..900 {
            water.step((2000, 2000));
        }
        assert!(water.cur.iter().all(|v| v.is_finite()));
        let after = water.cur.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(after < 0.01, "residual height {after}");
    }
    /// Regress the missing island support and prove idle crests move without touching dry pixels.
    #[test]
    fn idle_surface_keeps_the_island_mirror_on_its_bank() {
        let (mut z, _) = super::super::build();
        let m = super::super::paint::classify(&z);
        let fl = super::super::paint::fields(&m);
        let mut decor = Vec::new();
        super::super::place::populate(&mut z, &mut decor, &m);
        let water = Water::new(&m, &fl, &z.ground, &decor);
        let mirror = &water.mirrors[0];
        assert_eq!(mirror.source, (520, 404));
        for x in 504..=536 {
            let line = mirror.lines[(x - mirror.left) as usize];
            assert!(
                (431..=433).contains(&line),
                "unexpected physical shore at {x},{line}"
            );
            assert!(
                mirror
                    .pixels
                    .iter()
                    .any(|&(px, py, _)| px == x && py == line),
                "detached support at {x}"
            );
        }
        assert!(
            mirror
                .pixels
                .iter()
                .all(|&(x, y, idx)| water.is_water(x, y) && idx <= 2)
        );
        let mut a = Frame::new();
        a.cam_x = 228;
        a.cam_y = 314;
        let mut b = Frame::new();
        b.cam_x = a.cam_x;
        b.cam_y = a.cam_y;
        water.draw(&mut a, 300);
        water.draw(&mut b, 308);
        let changed = a
            .albedo
            .iter()
            .zip(&b.albedo)
            .filter(|(a, b)| a.r != b.r || a.g != b.g || a.b != b.b)
            .count();
        assert!(changed > 50, "idle surface only changed {changed} pixels");
        assert!(changed < 1500, "idle surface too busy: {changed} pixels");
        // Ambient movement is analytical, so the wading field remains undisturbed at rest.
        assert!(water.cur.iter().all(|&h| h == 0.0));
        for y in 0..H as i32 {
            for x in 0..W as i32 {
                let i = y as usize * W + x as usize;
                if !water.is_water(x + a.cam_x, y + a.cam_y) {
                    assert_eq!(a.albedo[i].luma(), 0.0);
                    assert_eq!(b.albedo[i].luma(), 0.0);
                }
            }
        }
        // The same world pixels agree when viewed by an overlapping camera.
        b.cam_x += 10;
        water.draw(&mut b, 300);
        for y in 0..H {
            for x in 10..W {
                let p = a.albedo[y * W + x];
                let q = b.albedo[y * W + x - 10];
                if water.is_water(x as i32 + a.cam_x, y as i32 + a.cam_y) {
                    assert_eq!((p.r, p.g, p.b), (q.r, q.g, q.b));
                }
            }
        }
    }
}
