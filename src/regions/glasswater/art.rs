//! Native PixelLab and code-drawn sprites for Glasswater Hollow: crystals, shelf fungi and rock
//! spires. Generated families retain native geometry with grounded roots and small emissive caps.
//! Every pixel lands on the native grid with a color from the region ramps
//! (`assets/concept/palettes_v2.json`), light from the upper left. No blending, no soft alpha.
//! Stone and water ramps are shifted to slate; violet sprigs and fungi are desaturated so
//! the few artist-authored landmark crystals carry the region's color.

use crate::gfx::{Cell, Px, Rgb, rgb, sheet};
use crate::util::hash2;

// Style v2 ramps keep the index contracts used by the painter; repeated end stops
// deliberately avoid inventing intermediate colors for the five-swatch stone ramps.
pub const ROCK: [Rgb; 6] = [rgb(0x222d3d), rgb(0x364657), rgb(0x4c6070), rgb(0x73858b), rgb(0xa6b4ad), rgb(0xa6b4ad)];
pub const WALL: [Rgb; 6] = [rgb(0x29313f), rgb(0x424956), rgb(0x626a74), rgb(0x929797), rgb(0xbbc0b4), rgb(0xbbc0b4)];
pub const VIOLET: [Rgb; 5] = [rgb(0x4d4866), rgb(0x4d4866), rgb(0x796c91), rgb(0xa195b8), rgb(0xcdc3db)];
pub const WATER: [Rgb; 6] = [rgb(0x1c3548), rgb(0x285365), rgb(0x467d8a), rgb(0x88adb2), rgb(0xc2d7cf), rgb(0xc2d7cf)];
pub const CAP: [Rgb; 5] = [rgb(0x414858), rgb(0x66677b), rgb(0x9091a0), rgb(0xbcb9c0), rgb(0xbcb9c0)];
pub const STEM: [Rgb; 4] = [rgb(0x454f60), rgb(0x454f60), rgb(0x7c8991), rgb(0xb9c4ba)];
/// Unlit recesses use the region outline, rather than swallowing the wall's top plane.
pub const DEEP_DARK: Rgb = rgb(0x222d3d);

/// A pixel canvas that becomes a `Cell`.
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub px: Vec<Px>,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        Canvas { w, h, px: vec![Px::default(); (w * h) as usize] }
    }
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: Rgb, emit: bool) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = Px { c, a: true, emit };
        }
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Px {
        if x >= 0 && y >= 0 && x < self.w && y < self.h { self.px[(y * self.w + x) as usize] } else { Px::default() }
    }
    /// Crop to the opaque bounding box; returns the cell and the anchor moved with it.
    pub fn trimmed(self, anchor: (i32, i32)) -> (Cell, (i32, i32)) {
        let (mut x0, mut y0, mut x1, mut y1) = (self.w, self.h, -1, -1);
        for y in 0..self.h {
            for x in 0..self.w {
                if self.px[(y * self.w + x) as usize].a {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        if x1 < 0 {
            return (Cell::new(1, 1, vec![Px::default()]), (0, 0));
        }
        // keep the anchor inside the cell
        x0 = x0.min(anchor.0);
        y1 = y1.max(anchor.1);
        x1 = x1.max(anchor.0);
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut px = vec![Px::default(); (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                px[(y * w + x) as usize] = self.get(x + x0, y + y0);
            }
        }
        (Cell::new(w, h, px), (anchor.0 - x0, anchor.1 - y0))
    }
}

/// One crystal prism: base center (bx, by), height h, half-width hw, lean = (num, den) px per row,
/// chisel = slanted top facing the light instead of a point.
#[derive(Clone, Copy)]
struct Shard {
    bx: i32,
    by: i32,
    h: i32,
    hw: i32,
    lean: (i32, i32),
    chisel: bool,
}

/// Rasterize one shard. Faces: lit left (ramp 2), ridge (3 low, 4 high, glowing), shadow right
/// (ramp 1) with a dark outline (ramp 0); the tip facet catches the light (ramp 4).
fn draw_shard(cv: &mut Canvas, ramp: &[Rgb; 5], s: Shard, outline_left: bool) {
    let w = 2 * s.hw + 1;
    let mut drawn: Vec<(i32, i32)> = Vec::new();
    for r in 0..s.h {
        let y = s.by - r;
        let shift = (r * s.lean.0).div_euclid(s.lean.1.max(1));
        let left = s.bx + shift - s.hw;
        for u in 0..w {
            // top boundary
            let cut = if s.hw == 0 {
                0
            } else if s.chisel {
                // high on the right, low on the left: the top facet faces up-left (lit)
                ((w - 1 - u) * (s.hw + 1)) / (w - 1).max(1)
            } else {
                // pointed: two rows per column toward the tip
                (u - s.hw).abs() * 2
            };
            let top = s.h - 1 - cut;
            if r > top {
                continue;
            }
            let from_top = top - r;
            let frac = r as f32 / s.h as f32;
            let (idx, emit) = if s.hw == 0 {
                if from_top < 2 { (4, true) } else { (3, false) }
            } else if u == s.hw {
                if from_top < 2 || frac > 0.78 { (4, true) } else { (3, false) }
            } else if u < s.hw {
                if from_top < 1 + (s.hw > 1) as i32 {
                    (3, false)
                } else if u == 0 && frac > 0.2 {
                    (3, false)
                } else {
                    (2, false)
                }
            } else if u == w - 1 {
                (0, false)
            } else if from_top < 1 {
                (2, false)
            } else {
                (1, false)
            };
            // ground contact: the bottom row sits in shadow
            let (idx, emit) = if r == 0 && !(u == s.hw && s.hw == 0) { ((idx as i32 - 1).max(0) as usize, false) } else { (idx, emit) };
            cv.set(left + u, y, ramp[idx], emit);
            if u == 0 {
                drawn.push((left - 1, y));
            }
        }
    }
    if outline_left {
        // separate this shard from shards behind it with a dark edge on its left
        for (x, y) in drawn {
            if cv.get(x, y).a {
                cv.set(x, y, ramp[0], false);
            }
        }
    }
}

/// A rubble mound under a cluster so it sits on the ground.
fn mound(cv: &mut Canvas, cx: i32, by: i32, rx: i32, seed: u32) {
    for dx in -rx..=rx {
        let t = 1.0 - (dx as f32 / (rx as f32 + 0.5)).powi(2);
        let hgt = (t * 3.0 + hash2(dx, 1, seed) * 1.2).round() as i32;
        for k in 0..hgt.max(1) {
            let y = by - k;
            let c = if k == hgt - 1 && dx < 0 {
                ROCK[4]
            } else if k == hgt - 1 {
                ROCK[3]
            } else if dx > rx / 3 {
                ROCK[1]
            } else {
                ROCK[2]
            };
            cv.set(cx + dx, y, c, false);
        }
    }
}

/// Native generated cells keep their source geometry and one-pixel density. Only the tiny
/// highest crystal cap emits: broad light-facing planes remain actual receiving material.
const CRYSTAL_CREST_ROWS: i32 = 8;
const CRYSTAL_CREST_PIXELS: usize = 8;

/// Select a real, connected highlight at the prism's highest cap. The generated
/// families use several palettes: a missing literal violet must not create an
/// invisible emitter at an invented size-based height. Brightness only chooses
/// existing cap material; it never supplies surface height or changes its RGB.
fn mark_crystal_crest(cell: &mut Cell) -> usize {
    let Some(first) = cell.px.iter().position(|p| p.a) else { return 0 };
    let top = first as i32 / cell.w;
    let cap: Vec<_> = cell.px.iter().enumerate()
        .filter(|(i, p)| p.a && (*i as i32 / cell.w) < top + CRYSTAL_CREST_ROWS)
        .map(|(i, _)| i).collect();
    let mut candidates = vec![false; cell.px.len()];
    // Sheet JSON has already been decoded into Px.emit. Honor an authored
    // native cap before considering the older palette-specific convention.
    for &i in &cap { candidates[i] = cell.px[i].emit; }
    if !candidates.iter().any(|&candidate| candidate) {
        for &i in &cap { candidates[i] = cell.px[i].c.key() == rgb(0xcdc3db).key(); }
    }
    if !candidates.iter().any(|&candidate| candidate) {
        let mut brightest = cap[0];
        for &i in &cap[1..] {
            if cell.px[i].c.luma() > cell.px[brightest].c.luma() { brightest = i; }
        }
        let color = cell.px[brightest].c.key();
        for &i in &cap { candidates[i] = cell.px[i].c.key() == color; }
    }
    for p in &mut cell.px { p.emit = false; }
    let Some(seed) = candidates.iter().position(|&candidate| candidate) else { return 0 };
    let mut queue = std::collections::VecDeque::from([seed]);
    candidates[seed] = false;
    let mut count = 0;
    while let Some(i) = queue.pop_front() {
        cell.px[i].emit = true;
        count += 1;
        if count == CRYSTAL_CREST_PIXELS { break; }
        let (x, y) = (i as i32 % cell.w, i as i32 / cell.w);
        for dy in -1..=1 { for dx in -1..=1 {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= cell.w || ny >= cell.h { continue; }
            let next = (ny * cell.w + nx) as usize;
            if candidates[next] {
                candidates[next] = false;
                queue.push_back(next);
            }
        }}
    }
    count
}

fn native(name: &str, seed: u32, crystal: bool) -> Option<(Cell, (i32, i32))> {
    let s = sheet(name)?;
    let a = s.anim("body")?;
    let mut c = s.cell(a, seed as usize % a.frames.max(1)).clone();
    if crystal {
        mark_crystal_crest(&mut c);
    }
    c.refresh_surface_mask();
    Some((c, s.anchor_of(a)))
}

/// A crystal cluster about `size` px tall. Returns (cell, anchor at the base center).
pub fn crystal_cluster(seed: u32, ramp: &[Rgb; 5], size: i32) -> (Cell, (i32, i32)) {
    let family = if size >= 38 { "glasswater_crystals_native" } else { "glasswater_crystals_small" };
    if let Some(cell) = native(family, seed, true) { return cell; }
    // Landmark sprites stay at their authored native size. Small clusters use the same
    // faceted construction below until their own sheet is delivered.
    if size >= 40 {
        let name = match seed % 4 {
            0 => "aqua_b",
            1 => "aqua_a",
            2 => "violet_b",
            _ => "violet_a",
        };
        if let Some(s) = sheet("glasswater_crystals_big")
            && let Some(a) = s.anim(name)
        {
            return (s.cell(a, 0).clone(), s.anchor_of(a));
        }
    }
    let cw = size * 2 + 24;
    let chh = size + 10;
    let mut cv = Canvas::new(cw, chh);
    let bx = cw / 2;
    let by = chh - 2;
    let rnd = |i: i32| hash2(seed as i32, i, 9137);
    let main_hw = (size / 6).clamp(2, 6);
    mound(&mut cv, bx, by + 1, main_hw * 2 + 4 + size / 8, seed);

    // back shards: lean outward, shorter
    let n_side = if size >= 30 { 2 } else { 1 };
    let mut back = Vec::new();
    for (si, side) in [-1i32, 1].iter().enumerate() {
        for k in 0..n_side {
            let i = si as i32 * 10 + k;
            let h = ((size as f32) * (0.42 + 0.26 * rnd(i) - 0.12 * k as f32)) as i32;
            let hw = (h / 9).clamp(1, 3);
            let dx = side * (main_hw + hw - 1 + k * (hw + 2) + (rnd(i + 3) * 2.0) as i32);
            let den = 3 + k + (rnd(i + 5) * 3.0) as i32;
            back.push(Shard { bx: bx + dx, by: by - 1, h: h.max(4), hw, lean: (*side, den), chisel: rnd(i + 7) > 0.5 });
        }
    }
    for s in &back {
        draw_shard(&mut cv, ramp, *s, false);
    }
    // the main prism, nearly upright
    let lean = if rnd(40) > 0.5 { (1, 9 + (rnd(41) * 6.0) as i32) } else { (-1, 9 + (rnd(42) * 6.0) as i32) };
    draw_shard(&mut cv, ramp, Shard { bx, by, h: size, hw: main_hw, lean, chisel: size >= 20 && rnd(43) > 0.4 }, true);
    // A few steep, broad front shards overlap the main mineral's root.
    let n_front = if size >= 20 { 2 } else { 1 };
    for k in 0..n_front {
        let side = if k % 2 == 0 { -1 } else { 1 };
        let h = ((size as f32) * (0.16 + 0.16 * rnd(60 + k))) as i32 + 2;
        let hw = if h >= 5 { 1 } else { 0 };
        let dx = side * (main_hw + k * 2);
        let den = 3 + (rnd(90 + k) * 3.0) as i32;
        draw_shard(&mut cv, ramp, Shard { bx: bx + dx, by: by + 1, h, hw, lean: (side, den), chisel: false }, true);
    }
    cv.trimmed((bx, by + 1))
}

/// A lone needle or small group for floors and ledges (4-12 px).
pub fn crystal_sprig(seed: u32, ramp: &[Rgb; 5], size: i32) -> (Cell, (i32, i32)) {
    let mut cv = Canvas::new(size * 2 + 8, size + 4);
    let bx = size + 4;
    let by = size + 2;
    let rnd = |i: i32| hash2(seed as i32, i, 4411);
    let hw = if size >= 8 { 1 } else { 0 };
    draw_shard(&mut cv, ramp, Shard { bx, by, h: size, hw, lean: (if rnd(1) > 0.5 { 1 } else { -1 }, 4 + (rnd(2) * 4.0) as i32), chisel: false }, false);
    let side = if rnd(3) > 0.5 { 1 } else { -1 };
    draw_shard(&mut cv, ramp, Shard { bx: bx + side * (hw + 1), by: by + 1, h: (size * 5 / 9).max(2), hw: 0, lean: (side, 2), chisel: false }, true);
    if size >= 8 {
        draw_shard(&mut cv, ramp, Shard { bx: bx - side * (hw + 2), by: by + 1, h: (size / 3).max(2), hw: 0, lean: (-side, 1), chisel: false }, true);
    }
    cv.trimmed((bx, by + 1))
}

/// A rock spire (stalagmite) in the wall ramp; no glow.
pub fn stalagmite(seed: u32, size: i32) -> (Cell, (i32, i32)) {
    let mut cv = Canvas::new(size + 16, size + 6);
    let bx = size / 2 + 8;
    let by = size + 3;
    let rnd = |i: i32| hash2(seed as i32, i, 2203);
    let hw = (size / 6).clamp(1, 4);
    let ramp = [WALL[0], WALL[1], WALL[2], WALL[3], WALL[4]];
    let lean = (if rnd(1) > 0.5 { 1 } else { -1 }, 6 + (rnd(2) * 6.0) as i32);
    // body: like a shard, but rock colors and a rounded, wider base
    for r in 0..size {
        let t = r as f32 / size as f32;
        let half = ((hw as f32 + 1.5) * (1.0 - t).powf(0.7)).round() as i32;
        let cx = bx + (r * lean.0).div_euclid(lean.1);
        for u in -half..=half {
            let idx = if u == -half {
                if t > 0.15 { 3 } else { 2 }
            } else if u < 0 {
                2
            } else if u == half {
                0
            } else if u == 0 && t > 0.3 {
                2
            } else {
                1
            };
            let idx = if r == size - 1 { 3 } else { idx };
            cv.set(cx + u, by - r, ramp[idx], false);
        }
    }
    mound(&mut cv, bx, by + 1, hw + 4, seed);
    cv.trimmed((bx, by + 1))
}

/// A clipped slate boulder with three broad upper-left-lit planes.
pub fn boulder(seed: u32, rx: i32, ry: i32) -> (Cell, (i32, i32)) {
    let mut cv = Canvas::new(rx * 2 + 3, ry * 2 + 3);
    let (cx, cy) = (rx + 1, ry + 1);
    let skew = (hash2(seed as i32, 0, 31) * 3.0) as i32 - 1;
    for dy in -ry..=ry / 2 + 1 {
        for dx in -rx..=rx {
            if dx.abs() + (dy + skew).abs() / 2 > rx || dy < -ry + (dx + rx) / 4 {
                continue;
            }
            let k = if dy < -ry / 3 && dx < rx / 2 {
                3
            } else if dx < rx / 3 - dy / 2 {
                2
            } else {
                1
            };
            cv.set(cx + dx, cy + dy, WALL[k], false);
        }
    }
    cv.trimmed((cx, cy + ry / 2 + 1))
}

/// A broken distributor fin with a recessed water channel and no new collision.
pub fn sluice_fin(seed: u32, height: i32) -> (Cell, (i32, i32)) {
    if let Some(cell) = native("glasswater_waterworks", seed, false) { return cell; }
    let mut cv = Canvas::new(40, height + 8);
    let base = height + 4;
    let broken = seed % 2 == 0;
    for y in 3..=base {
        let foot = y > base - 7;
        let left = if foot { 3 } else { 10 };
        let right = if foot { 36 } else { 29 };
        for x in left..=right {
            let top = 3 + (x - 10).max(0) / 4 + if broken && x > 19 { 8 } else { 0 };
            if y < top {
                continue;
            }
            let c = if y < top + 3 || foot && y < base - 4 {
                WALL[3]
            } else if x < left + 4 {
                WALL[2]
            } else if x > right - 5 {
                WALL[0]
            } else {
                WALL[1]
            };
            let slot = !foot && x >= 17 && x <= 21 && y > top + 10;
            cv.set(x, y, if slot { DEEP_DARK } else { c }, false);
            if slot && x == 17 {
                cv.set(x, y, WALL[2], false);
            }
        }
    }
    // The exposed channel ends at a broken projecting spout, never a glowing rune.
    for y in base - 15..base - 9 {
        for x in 14..34 {
            cv.set(x, y, if y == base - 15 { WALL[3] } else { WALL[1] }, false);
        }
    }
    cv.trimmed((20, base + 1))
}

/// A quiet shelf fungus with non-emissive gills and muted cap markings.
/// `cap` = cap half-width; `stem` = stem height; `lean` bends the stem (px at the top).
pub fn mushroom(seed: u32, cap: i32, stem: i32, lean: i32) -> (Cell, (i32, i32)) {
    let capy = (cap as f32 * 0.62).round().max(2.0) as i32;
    let w = cap * 2 + lean.abs() * 2 + 8;
    let h = stem + capy + 6;
    let mut cv = Canvas::new(w, h);
    let bx = w / 2;
    let by = h - 2;
    let rnd = |i: i32| hash2(seed as i32, i, 7777);
    let sw = (cap / 4).clamp(1, 4);
    // stem
    let mut top_x = bx;
    for r in 0..stem {
        let t = r as f32 / stem as f32;
        let cx = bx + (lean as f32 * t * t).round() as i32;
        top_x = cx;
        let flare = if r < 2 && sw >= 2 { 1 } else { 0 };
        for u in -(sw + flare)..=(sw - 1 + flare).max(0) {
            let idx: usize = if u == -(sw + flare) {
                3
            } else if u < 0 {
                2
            } else if u == (sw - 1 + flare).max(0) && sw > 1 {
                0
            } else {
                1
            };
            let idx = if r < 1 { idx.saturating_sub(1) } else { idx };
            cv.set(cx + u, by - r, STEM[idx], false);
        }
    }
    // cap: a dome over (top_x, cy)
    let cy = by - stem + 1;
    // Gills remain stone-gray, without a second emissive material family.
    for dx in -(cap - 1)..=(cap - 1) {
        let glow = (dx + seed as i32).rem_euclid(2) == 0 || dx.abs() <= sw;
        cv.set(top_x + dx, cy, if glow { CAP[2] } else { CAP[1] }, false);
    }
    for dy in 0..=capy {
        for dx in -cap..=cap {
            let nx = (dx as f32 + 0.5) / (cap as f32 + 0.5);
            let ny = dy as f32 / (capy as f32 + 0.5);
            let d = nx * nx + ny * ny;
            if d > 1.0 {
                continue;
            }
            let nz = (1.0 - d).max(0.0).sqrt();
            let lum = -0.5 * nx + 0.55 * ny * 0.0 + 0.62 * nz + 0.25 * ny;
            let rim = dy == 0 || (d > 0.78 && nx > 0.2);
            let idx = if rim {
                if nx < -0.3 { 2 } else { 1 }
            } else if lum > 0.62 {
                3
            } else if lum > 0.38 {
                2
            } else {
                1
            };
            cv.set(top_x + dx, cy - 1 - dy, CAP[idx], false);
        }
    }
    // Quiet cap markings stay grouped and non-emissive.
    let spots = (cap / 2).clamp(1, 5);
    for k in 0..spots {
        let a = rnd(k) * 2.0 - 1.0;
        let sx = (a * (cap as f32 - 2.0)).round() as i32;
        let sy = 1 + (rnd(k + 20) * (capy as f32 - 1.5)).round() as i32;
        let nx = sx as f32 / cap as f32;
        let ny = sy as f32 / capy as f32;
        if nx * nx + ny * ny < 0.7 {
            cv.set(top_x + sx, cy - 1 - sy, CAP[2], false);
            if cap >= 6 && rnd(k + 40) > 0.5 {
                cv.set(top_x + sx + 1, cy - 1 - sy, CAP[2], false);
            }
        }
    }
    cv.trimmed((bx, by + 1))
}

/// A few small mushrooms growing together.
pub fn mushroom_patch(seed: u32, n: i32) -> (Cell, (i32, i32)) {
    if let Some(cell) = native("glasswater_fungi", seed, false) { return cell; }
    let mut cv = Canvas::new(40, 24);
    let (bx, by) = (20, 22);
    let rnd = |i: i32| hash2(seed as i32, i, 1301);
    let mut items: Vec<(i32, i32, i32, i32, i32)> = Vec::new();
    for k in 0..n {
        let cap = 2 + (rnd(k) * 3.0) as i32;
        let stem = 3 + (rnd(k + 10) * 6.0) as i32;
        let dx = ((k as f32 - (n - 1) as f32 * 0.5) * 6.0 + (rnd(k + 20) - 0.5) * 3.0).round() as i32;
        let dy = (rnd(k + 30) * 3.0) as i32 - 1;
        let lean = ((rnd(k + 40) - 0.5) * 4.0).round() as i32;
        items.push((dx, dy, cap, stem, lean));
    }
    // back to front
    items.sort_by_key(|i| i.1);
    for (k, (dx, dy, cap, stem, lean)) in items.iter().enumerate() {
        let (c, a) = mushroom(seed.wrapping_add(k as u32 * 17), *cap, *stem, *lean);
        let ox = bx + dx - a.0;
        let oy = by + dy - a.1;
        for y in 0..c.h {
            for x in 0..c.w {
                let p = c.at(x, y);
                if p.a {
                    cv.set(ox + x, oy + y, p.c, p.emit);
                }
            }
        }
    }
    cv.trimmed((bx, by + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_crystal_caps_cover_every_palette_without_repainting_geometry() {
        for name in ["glasswater_crystals_native", "glasswater_crystals_small"] {
            let source = sheet(name).expect("native crystal family");
            let anim = source.anim("body").expect("native crystal variants");
            for variant in 0..anim.frames {
                let original = source.cell(anim, variant);
                let (cell, anchor) = native(name, variant as u32, true).unwrap();
                assert_eq!((cell.w, cell.h, anchor), (original.w, original.h, source.anchor_of(anim)));
                let top = original.px.iter().position(|p| p.a).unwrap() as i32 / original.w;
                let emitted: Vec<_> = cell.px.iter().enumerate()
                    .filter(|(_, p)| p.emit).map(|(i, _)| i).collect();
                assert!(!emitted.is_empty(), "{name} variant {variant} has no visible source housing");
                assert!(emitted.len() <= CRYSTAL_CREST_PIXELS, "{name} variant {variant} bleaches a broad face");
                for (i, (p, q)) in original.px.iter().zip(&cell.px).enumerate() {
                    assert_eq!((p.c, p.a), (q.c, q.a), "{name} variant {variant} albedo/alpha {i}");
                    assert_eq!(original.surface[i].n, cell.surface[i].n, "crest selection must preserve normals");
                    assert_eq!((cell.surface[i].a, cell.surface[i].emit), (q.a, q.emit));
                    if q.emit {
                        let y = i as i32 / cell.w;
                        assert!(q.a && y >= top && y < top + CRYSTAL_CREST_ROWS);
                    }
                }
                // A centroid between disconnected high tips would not describe
                // the actual housing of either tip. Require one native patch.
                let mut reached = std::collections::HashSet::from([emitted[0]]);
                let mut todo = vec![emitted[0]];
                while let Some(i) = todo.pop() {
                    let (x, y) = (i as i32 % cell.w, i as i32 / cell.w);
                    for &next in &emitted {
                        let (nx, ny) = (next as i32 % cell.w, next as i32 / cell.w);
                        if (x - nx).abs() <= 1 && (y - ny).abs() <= 1 && reached.insert(next) {
                            todo.push(next);
                        }
                    }
                }
                assert_eq!(reached.len(), emitted.len(), "{name} variant {variant} split source housing");
            }
        }
    }

    #[test]
    fn authored_cap_emission_precedes_a_brighter_legacy_palette_color() {
        let mut cell = Cell::from_ascii(&["ab", "bb"],
            &[('a', rgb(0x73858b)), ('b', rgb(0xcdc3db))], &['a']);
        assert_eq!(mark_crystal_crest(&mut cell), 1);
        assert!(cell.at(0, 0).emit);
        assert!(!cell.at(1, 0).emit && !cell.at(0, 1).emit && !cell.at(1, 1).emit);
    }
}
