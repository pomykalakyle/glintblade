//! The Warden's art: the part sheets (`warden_body`, `warden_head`, `warden_fist`,
//! `warden_heart`, `warden_crystal`), where the parts sit on the body (`warden_layout.json`),
//! and a blitter with the extras the fight needs: clipping at the ground line (kneeling),
//! hit flash, dimmed emissives (dormant eyes) and glowing crack overlays.
//!
//! Every lookup falls back to a small code-drawn part, so the fight works before the art lands.

use super::hazards::{P0, P1, P2, PW};
use crate::gfx::{Anim, Cell, Frame, Px, Rgb, Sheet, rgb, sheet};
use crate::util::{Rng, bayer4};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const BODY: &str = "warden_body";
pub const HEAD: &str = "warden_head";
pub const FIST: &str = "warden_fist";
pub const HEART: &str = "warden_heart";
pub const CRYSTAL: &str = "warden_crystal";

/// Where the parts attach, as offsets in pixels from the body anchor (the ground point under
/// the feet), +y down. Read from `warden_layout.json` when it exists.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Head anchor (bottom middle of the hood).
    pub neck: (i32, i32),
    /// Heart sprite anchor (its center) in the chest socket.
    pub heart: (i32, i32),
    /// Crystal bases on the left and right pauldrons.
    pub crystals: [(i32, i32); 2],
    /// Fist anchor at rest, relative to the body anchor (left, right).
    pub fists: [(i32, i32); 2],
    /// Body cell size (for the burst and the crack overlay bounds).
    pub body_size: (i32, i32),
}

impl Default for Layout {
    /// Return safe authored defaults while art is unavailable.
    fn default() -> Self {
        // Anchor-relative defaults match the finished warden_layout.json.
        Layout {
            neck: (0, -75),
            heart: (0, -60),
            crystals: [(-36, -100), (36, -100)],
            fists: [(-40, -11), (40, -11)],
            body_size: (120, 112),
        }
    }
}

/// Read the artist's anchor-relative part offsets, keeping fallbacks for unfinished sheets.
pub fn layout() -> &'static Layout {
    static L: OnceLock<Layout> = OnceLock::new();
    L.get_or_init(|| {
        let mut l = Layout::default();
        if let Some(s) = sheet(BODY) {
            l.body_size = (s.cell_w, s.cell_h);
        }
        let Some(js) = crate::assets::get_str("sprites/warden_layout.json") else {
            return l;
        };
        let Ok(v) = serde_json::from_str::<Value>(js) else {
            eprintln!("warning: could not parse warden_layout.json");
            return l;
        };
        // `hurtboxes` contains centers, not attachment anchors. Only `parts` places sprites.
        let offset = |name: &str| -> Option<(i32, i32)> {
            let a = v.get("parts")?.get(name)?.get("offset")?.as_array()?;
            Some((a.first()?.as_i64()? as i32, a.get(1)?.as_i64()? as i32))
        };
        l.neck = offset("head").unwrap_or(l.neck);
        l.heart = offset("heart").unwrap_or(l.heart);
        l.crystals = [
            offset("crystal_left").unwrap_or(l.crystals[0]),
            offset("crystal_right").unwrap_or(l.crystals[1]),
        ];
        l.fists = [
            offset("fist_left").unwrap_or(l.fists[0]),
            offset("fist_right").unwrap_or(l.fists[1]),
        ];
        l
    })
}

/// One frame of a part: the cell and its anchor.
#[derive(Clone, Copy)]
pub struct Spr {
    pub cell: &'static Cell,
    pub anchor: (i32, i32),
}

/// Find the first supported animation in preference order.
fn first_anim<'a>(s: &'a Sheet, names: &[&str]) -> Option<&'a Anim> {
    names.iter().find_map(|n| s.anim(n))
}

/// The frame of the first anim in `names` that the sheet has (or the sheet's first anim, or a
/// code-drawn fallback). `ticks` into the anim; `looping` wraps, otherwise holds the last frame.
pub fn spr(sheet_name: &str, names: &[&str], ticks: u32, looping: bool) -> Spr {
    if let Some(s) = sheet(sheet_name) {
        let a = first_anim(s, names).or_else(|| {
            let mut v: Vec<&Anim> = s.anims.values().collect();
            v.sort_by_key(|a| a.row);
            v.into_iter().next()
        });
        if let Some(a) = a {
            let cell = s.cell(a, a.frame_at(ticks, looping));
            if !cell.is_empty() {
                return Spr {
                    cell,
                    anchor: s.anchor_of(a),
                };
            }
        }
    }
    let c = fallback(sheet_name, names.first().copied().unwrap_or(""));
    let anchor = match sheet_name {
        HEART => (c.w / 2, c.h / 2),
        FIST => (18, 33),
        BODY => (60, 109),
        HEAD => (20, 41),
        _ => (c.w / 2, c.h - 1),
    };
    Spr { cell: c, anchor }
}

/// Frame `i` of the first matching anim (clamped), for staged things like crack levels.
pub fn spr_frame(sheet_name: &str, names: &[&str], i: usize) -> Option<Spr> {
    let s = sheet(sheet_name)?;
    let a = first_anim(s, names)?;
    let cell = s.cell(a, i.min(a.frames.saturating_sub(1)));
    if cell.is_empty() {
        return None;
    }
    Some(Spr {
        cell,
        anchor: s.anchor_of(a),
    })
}

/// Lowest opaque row of a frame relative to its anchor (+ is below the anchor).
pub fn bottom(s: Spr) -> i32 {
    let c = s.cell;
    (0..c.h)
        .rev()
        .find(|&y| (0..c.w).any(|x| c.at(x, y).a))
        .map_or(0, |y| y - s.anchor.1)
}

/// Highest opaque row relative to the part's anchor, including its silhouette edges.
pub fn top(s: Spr) -> i32 {
    (0..s.cell.h)
        .find(|&y| (0..s.cell.w).any(|x| s.cell.at(x, y).a))
        .map_or(0, |y| y - s.anchor.1)
}

/// Top of the solid mass over the anchor column relative to the anchor, skipping 1 px speed lines.
pub fn solid_top(s: Spr) -> i32 {
    let c = s.cell;
    let ax = s.anchor.0;
    (0..c.h)
        .find(|&y| (ax - 2..=ax + 2).all(|x| c.at(x, y).a))
        .map_or(0, |y| y - s.anchor.1)
}

/// How many pixels above its rest pose a frame's art is drawn (a lift baked into the sheet).
pub fn baked_lift(s: Spr, rest: Spr) -> i32 {
    (bottom(rest) - bottom(s)).max(0)
}

/// Highest lift baked into any frame of an anim, relative to a rest frame.
pub fn peak_lift(sheet_name: &str, anim: &str, rest: Spr) -> i32 {
    (0..sheet(sheet_name)
        .and_then(|s| s.anim(anim))
        .map_or(0, |a| a.frames))
        .filter_map(|i| spr_frame(sheet_name, &[anim], i))
        .map(|s| baked_lift(s, rest))
        .max()
        .unwrap_or(0)
}

/// Ticks the sheet holds frame `i` of an anim, or `fallback` when the sheet lacks it.
pub fn frame_ticks(sheet_name: &str, anim: &str, i: usize, fallback: u32) -> u32 {
    sheet(sheet_name)
        .and_then(|s| s.anim(anim))
        .and_then(|a| a.durations.get(i).copied())
        .unwrap_or(fallback)
}

/// How to draw a part.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub flip: bool,
    /// 0..1 material flash, preserving the perimeter and authored emission mask.
    pub flash: f32,
    /// Interior heart energy during the death overload; ordinary material hits remain matte.
    pub energy_flash: f32,
    /// Emissive pixels: 1 = full glow, 0 = drawn as dull unlit iris stone (dormant), between = dim.
    pub emit: f32,
    /// Blend every color toward this.
    pub tint: Option<(Rgb, f32)>,
    /// Skip pixels below this screen row (the part sinks below the ground line when kneeling).
    pub clip_y: i32,
    /// Dithered opacity.
    pub alpha: f32,
    /// Solid color for every pixel (smears, afterimages); glows.
    pub silhouette: Option<Rgb>,
}

impl Default for Look {
    /// Return safe authored defaults while art is unavailable.
    fn default() -> Self {
        Look {
            flip: false,
            flash: 0.0,
            energy_flash: 0.0,
            emit: 1.0,
            tint: None,
            clip_y: i32::MAX,
            alpha: 1.0,
            silhouette: None,
        }
    }
}

/// Top-left screen position of a cell drawn with `anchor` at screen (sx, sy).
pub fn origin(cell: &Cell, anchor: (i32, i32), sx: i32, sy: i32, flip: bool) -> (i32, i32) {
    (
        sx - if flip {
            cell.w - 1 - anchor.0
        } else {
            anchor.0
        },
        sy - anchor.1,
    )
}

/// Draw `cell` with its `anchor` at screen (sx, sy).
pub fn blit(f: &mut Frame, s: Spr, sx: i32, sy: i32, l: &Look) {
    let cell = s.cell;
    let (ox, oy) = origin(cell, s.anchor, sx, sy, l.flip);
    if ox > crate::gfx::WI || oy > crate::gfx::HI || ox + cell.w < 0 || oy + cell.h < 0 {
        return;
    }
    for y in 0..cell.h {
        let py = oy + y;
        if !(0..crate::gfx::HI).contains(&py) || py > l.clip_y {
            continue;
        }
        for x in 0..cell.w {
            let p = cell.px[(y * cell.w + x) as usize];
            if !p.a {
                continue;
            }
            let px = ox + if l.flip { cell.w - 1 - x } else { x };
            if !(0..crate::gfx::WI).contains(&px) {
                continue;
            }
            if l.alpha < 1.0 && l.alpha <= bayer4(px + f.cam_x, py + f.cam_y) {
                continue;
            }
            let c = cell.flash_color(x, y, p.c, l.flash);
            put_px(f, px, py, Px { c, ..p }, l);
            if l.energy_flash > 0.0 && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(dx, dy)| solid_at(cell, x + dx, y + dy)) {
                f.add_emit(px, py, PW.scale(l.energy_flash * 0.25));
            }
        }
    }
}

/// Draw one opaque source pixel with hit flash and emissive control.
#[inline]
fn put_px(f: &mut Frame, px: i32, py: i32, p: Px, l: &Look) {
    f.emissive[py as usize * crate::gfx::W + px as usize] = Rgb::BLACK;
    if let Some(s) = l.silhouette {
        f.put_glow(px, py, s);
        return;
    }
    let mut c = p.c;
    if let Some((t, k)) = l.tint {
        c = c.lerp(t, k);
    }
    if p.emit {
        if l.emit >= 0.999 {
            f.put_glow(px, py, c);
        } else {
            // dull: an engraved, unlit version of the glow color, plus whatever glow is left
            let dull = c.lerp(rgb(0x333b4c), 0.72);
            f.put(px, py, dull);
            if l.emit > 0.0 {
                f.add_emit(px, py, c.scale(l.emit));
            }
        }
    } else {
        f.put(px, py, c);
    }
    f.mark_actor(px, py);
}

/// Is the cell pixel at (x, y) solid stone (not transparent, not the outline, not glowing)?
pub fn solid_at(cell: &Cell, x: i32, y: i32) -> bool {
    let p = cell.at(x, y);
    p.a && !p.emit && p.c.luma() > 0.06
}

/// Glowing cracks over a part: `cracks` are cell pixels in growth order; the first `n` are lit.
/// Only drawn where the part is solid stone, so the outline stays clean.
pub fn draw_cracks(
    f: &mut Frame,
    s: Spr,
    sx: i32,
    sy: i32,
    l: &Look,
    cracks: &[(i32, i32, u8)],
    n: usize,
    color: Rgb,
    core: Rgb,
) {
    let (ox, oy) = origin(s.cell, s.anchor, sx, sy, l.flip);
    for &(x, y, w) in cracks.iter().take(n) {
        if !solid_at(s.cell, x, y) {
            continue;
        }
        let px = ox + if l.flip { s.cell.w - 1 - x } else { x };
        let py = oy + y;
        if py > l.clip_y {
            continue;
        }
        f.put_glow(px, py, if w == 0 { core } else { color });
        f.mark_actor(px, py);
    }
}

/// Crack paths through a part: branching random walks from `seeds` (cell coords), mostly
/// outward from `from`. Returns pixels in growth order with a weight (0 = trunk, 1 = branch).
pub fn crack_paths(
    cell: &Cell,
    seeds: &[(i32, i32)],
    from: (i32, i32),
    seed: u64,
    total: usize,
) -> Vec<(i32, i32, u8)> {
    let mut rng = Rng::new(seed);
    let mut out: Vec<(i32, i32, u8)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // walkers: (x, y, dx, dy, weight, steps left)
    let mut walkers: Vec<(f32, f32, f32, f32, u8, i32)> = Vec::new();
    for (i, &(sx, sy)) in seeds.iter().enumerate() {
        let a = (i as f32 / seeds.len().max(1) as f32) * std::f32::consts::TAU + rng.signed() * 0.4;
        let (mut dx, mut dy) = (a.cos(), a.sin());
        let (fx, fy) = ((sx - from.0) as f32, (sy - from.1) as f32);
        let l = (fx * fx + fy * fy).sqrt();
        if l > 2.0 {
            dx = fx / l;
            dy = fy / l;
        }
        walkers.push((sx as f32, sy as f32, dx, dy, 0, 40 + rng.irange(0, 30)));
    }
    let mut guard = 0;
    while out.len() < total && !walkers.is_empty() && guard < 20_000 {
        guard += 1;
        let i = (guard as usize) % walkers.len();
        let (x, y, dx, dy, w, left) = walkers[i];
        if left <= 0 {
            walkers.swap_remove(i);
            continue;
        }
        // wander: mostly straight runs with occasional kinks (reads as cracked stone, not noise)
        let (mut ndx, mut ndy) = (dx, dy);
        if rng.chance(0.22) {
            let a = ndy.atan2(ndx) + rng.signed() * 0.9;
            ndx = a.cos();
            ndy = a.sin();
        }
        let (nx, ny) = (x + ndx, y + ndy);
        let (ix, iy) = (nx.round() as i32, ny.round() as i32);
        if !solid_at(cell, ix, iy) {
            walkers[i].5 = 0;
            continue;
        }
        if seen.insert((ix, iy)) {
            out.push((ix, iy, w));
        }
        walkers[i] = (nx, ny, ndx, ndy, w, left - 1);
        if rng.chance(0.045) && walkers.len() < 24 {
            let a = ndy.atan2(ndx) + if rng.chance(0.5) { 0.9 } else { -0.9 };
            walkers.push((nx, ny, a.cos(), a.sin(), 1, 10 + rng.irange(0, 16)));
        }
    }
    out
}

// ------------------------------------------------------------------ fallback parts
// Simple code-drawn parts in the summit's warden_stone ramp, used only until the sheets exist.

const S: [Rgb; 5] = [
    rgb(0x333b4c),
    rgb(0x555d70),
    rgb(0x7f8797),
    rgb(0xb0b7bb),
    rgb(0xd9dacc),
];
const OL: Rgb = rgb(0x1f2838);

struct Canvas {
    w: i32,
    h: i32,
    px: Vec<Px>,
}

impl Canvas {
    /// Create a transparent native-resolution fallback canvas.
    fn new(w: i32, h: i32) -> Canvas {
        Canvas {
            w,
            h,
            px: vec![Px::default(); (w * h) as usize],
        }
    }
    /// Place one fully opaque pixel inside the fallback canvas.
    fn put(&mut self, x: i32, y: i32, c: Rgb, emit: bool) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = Px { c, a: true, emit };
        }
    }
    /// Read a canvas pixel, treating its exterior as transparent.
    fn get(&self, x: i32, y: i32) -> Px {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize]
        } else {
            Px::default()
        }
    }
    /// Draw a clipped stone slab with broad upper-left planes.
    fn slab(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, base: usize) {
        for y in 0..self.h {
            for x in 0..self.w {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx.abs() <= 1.0 && dy.abs() <= 1.0 && dx.abs() + dy.abs() <= 1.6 {
                    let light = -dx * 0.6 - dy * 0.8;
                    let tone = if light > 0.55 {
                        base + 2
                    } else if light > 0.05 {
                        base + 1
                    } else if light < -0.6 {
                        base.saturating_sub(1)
                    } else {
                        base
                    };
                    self.put(x, y, S[tone.min(4)], false);
                }
            }
        }
    }
    /// Add a one-pixel dark contour around the deliberate silhouette.
    fn outline(&mut self) {
        let mut add = Vec::new();
        for y in 0..self.h {
            for x in 0..self.w {
                if self.get(x, y).a {
                    continue;
                }
                if [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let p = self.get(x + dx, y + dy);
                    p.a && p.c != OL
                }) {
                    add.push((x, y));
                }
            }
        }
        for (x, y) in add {
            self.put(x, y, OL, false);
        }
    }
    /// Finish the fallback canvas as a sprite cell.
    fn cell(self) -> Cell {
        Cell {
            w: self.w,
            h: self.h,
            px: self.px,
        }
    }
}

/// Build simple native-grid parts only when an artist’s sheet is unavailable.
fn fallback(sheet_name: &str, anim: &str) -> &'static Cell {
    static CELLS: OnceLock<HashMap<&'static str, Cell>> = OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let mut m = HashMap::new();
        // body: 120x112, anchor (60, 111)
        let mut c = Canvas::new(120, 112);
        c.slab(44.0, 104.0, 13.0, 8.0, 2); // feet
        c.slab(76.0, 104.0, 13.0, 8.0, 1);
        c.slab(46.0, 88.0, 10.0, 16.0, 2); // legs
        c.slab(74.0, 88.0, 10.0, 16.0, 1);
        c.slab(60.0, 72.0, 26.0, 10.0, 2); // belt
        c.slab(60.0, 50.0, 32.0, 24.0, 2); // chest
        c.slab(14.0, 58.0, 9.0, 14.0, 2); // upper arms
        c.slab(106.0, 58.0, 9.0, 14.0, 1);
        c.slab(22.0, 28.0, 21.0, 17.0, 3); // pauldrons
        c.slab(98.0, 28.0, 21.0, 17.0, 2);
        for dy in -8i32..=8 {
            for dx in -8i32..=8 {
                if dx.abs() + dy.abs() <= 8 {
                    c.put(60 + dx, 49 + dy, OL, false); // heart socket
                }
            }
        }
        c.outline();
        m.insert(BODY, c.cell());
        // head: 40x44
        let mut c = Canvas::new(40, 44);
        c.slab(20.0, 26.0, 15.0, 17.0, 2);
        for y in 18..34 {
            for x in 13..27 {
                c.put(x, y, OL, false);
            }
        }
        for x in [15, 16, 17, 22, 23, 24] {
            c.put(x, 24, P2, true);
        }
        c.outline();
        m.insert(HEAD, c.cell());
        // fist: 36x36
        let mut c = Canvas::new(36, 36);
        c.slab(18.0, 20.0, 15.0, 13.0, 2);
        c.slab(18.0, 8.0, 7.0, 6.0, 2);
        c.outline();
        m.insert(FIST, c.cell());
        // heart: 18x22 diamond prism
        let mut c = Canvas::new(18, 22);
        for y in 0..22i32 {
            let half = if y < 11 {
                y * 8 / 11
            } else {
                (21 - y) * 8 / 10
            };
            for x in (9 - half)..=(9 + half) {
                let col = if x < 9 && y < 11 {
                    PW
                } else if x < 9 {
                    P2
                } else if y < 11 {
                    P1
                } else {
                    P0
                };
                c.put(x, y, col, col == PW);
            }
        }
        c.outline();
        m.insert(HEART, c.cell());
        // crystal: 10x16 shard
        let mut c = Canvas::new(10, 16);
        for y in 0..16i32 {
            let half = if y < 5 { y * 4 / 5 } else { 4 - (y - 5) / 6 };
            for x in (5 - half)..=(5 + half) {
                let col = if x < 5 { PW } else { P2 };
                c.put(x, y, col, col == PW);
            }
        }
        c.outline();
        m.insert(CRYSTAL, c.cell());
        m
    });
    let _ = anim;
    cells
        .get(sheet_name)
        .unwrap_or_else(|| cells.get(CRYSTAL).unwrap())
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Warden material flashes keep the perimeter and never turn matte stone into an emitter.
    #[test]
    fn material_flash_preserves_ink_and_native_emission() {
        let cell = Cell::from_ascii(&["aaaaa", "abbba", "abeba", "abbba", "aaaaa"],
            &[('a', rgb(0x1f2838)), ('b', rgb(0x7f8797)), ('e', PW)], &['e']);
        let mut frame = Frame::new();
        // Cover an earlier emitter to prove opaque stone also clears underlying emission.
        frame.put_glow(10, 10, PW);
        let s = Spr { cell: Box::leak(Box::new(cell)), anchor: (0, 0) };
        blit(&mut frame, s, 10, 10, &Look { flash: 1.0, ..Default::default() });
        assert_eq!(frame.albedo[10 * crate::gfx::W + 10], rgb(0x1f2838));
        assert_ne!(frame.albedo[11 * crate::gfx::W + 11], rgb(0x7f8797));
        for y in 0..5 {
            for x in 0..5 {
                let emit = frame.emissive[(10 + y) * crate::gfx::W + 10 + x];
                assert_eq!(emit != Rgb::BLACK, x == 2 && y == 2);
            }
        }
    }

    /// The death overload floods the stone interior while keeping its outer ink intact.
    #[test]
    fn death_overload_lights_only_the_interior() {
        let cell = Cell::from_ascii(&["aaaaa", "abbba", "abbba", "abbba", "aaaaa"],
            &[('a', rgb(0x1f2838)), ('b', rgb(0x7f8797))], &[]);
        let mut frame = Frame::new();
        let s = Spr { cell: Box::leak(Box::new(cell)), anchor: (0, 0) };
        blit(&mut frame, s, 10, 10, &Look { flash: 1.0, energy_flash: 1.0, ..Default::default() });
        for y in 0..5 {
            for x in 0..5 {
                let emit = frame.emissive[(10 + y) * crate::gfx::W + 10 + x];
                assert_eq!(emit.max_c() > 0.20, (1..4).contains(&x) && (1..4).contains(&y));
            }
        }
        assert_eq!(frame.albedo[10 * crate::gfx::W + 10], rgb(0x1f2838));
    }
}
