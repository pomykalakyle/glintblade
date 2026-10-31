//! Paints Glasswater Hollow's pre-rendered ground once at load: layered cave-floor terraces,
//! broad slate masses with 3/4-view faces, broken wet shores, the lakebed under
//! the water, and ledges dropping into the abyss. Quiet angular planes cross tile seams.

use super::art::{CAP, DEEP_DARK, ROCK, WALL, WATER};
use crate::gfx::{Rgb, sheet};
use crate::util::{bayer4, fbm, hash2};
use crate::world::{HOLE, Tile, Zone};

pub const FLOOR: u8 = 0;
pub const WALL_TOP: u8 = 1;
pub const FACE: u8 = 2;
pub const SHALLOW: u8 = 3;
pub const DEEP: u8 = 4;
pub const VOID: u8 = 5;

/// Pixel-level classification of the zone.
pub struct PixMap {
    pub w: i32,
    pub h: i32,
    pub cls: Vec<u8>,
    /// FACE: rows below the lip. VOID: rows below the ledge edge (255 = far below).
    pub depth: Vec<u8>,
    /// FACE: the face's height in this column. VOID: the ledge face height.
    pub span: Vec<u8>,
    /// VOID: 0 = floor ledge, 1 = wall cliff continuing down, 2 = under water (waterfall).
    pub ledge: Vec<u8>,
}

impl PixMap {
    #[inline]
    pub fn at(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h { WALL_TOP } else { self.cls[(y * self.w + x) as usize] }
    }
    #[inline]
    pub fn rock(&self, x: i32, y: i32) -> bool {
        matches!(self.at(x, y), WALL_TOP | FACE)
    }
}

fn tile_c(z: &Zone, tx: i32, ty: i32) -> Tile {
    z.tile(tx.clamp(0, z.tw - 1), ty.clamp(0, z.th - 1))
}

/// Broad unequal wall masses vary their height without a narrow ribbed rhythm.
fn face_height(x: i32) -> i32 {
    let (_, _, seed) = plane(x, 0);
    22 + (seed.rem_euclid(3) * 4)
}

pub fn classify(z: &Zone) -> PixMap {
    let (w, h) = (z.pw, z.ph);
    let n = (w * h) as usize;
    let mut cls = vec![FLOOR; n];
    for y in 0..h {
        let fy = (y as f32 + 0.5) / 16.0 - 0.5;
        let ty = fy.floor() as i32;
        let ay = fy - ty as f32;
        for x in 0..w {
            let fx = (x as f32 + 0.5) / 16.0 - 0.5;
            let tx = fx.floor() as i32;
            let ax = fx - tx as f32;
            let t = [tile_c(z, tx, ty), tile_c(z, tx + 1, ty), tile_c(z, tx, ty + 1), tile_c(z, tx + 1, ty + 1)];
            let bil = |p: &dyn Fn(Tile) -> bool| -> f32 {
                let v = |t: Tile| if p(t) { 1.0 } else { 0.0 };
                (v(t[0]) * (1.0 - ax) + v(t[1]) * ax) * (1.0 - ay) + (v(t[2]) * (1.0 - ax) + v(t[3]) * ax) * ay
            };
            let nz = fbm(x as f32 * 0.07, y as f32 * 0.07, 2, 5) - 0.5;
            let c = if bil(&|t| t == Tile::Wall) + nz * 0.25 > 0.5 {
                WALL_TOP
            } else if bil(&|t| t == Tile::Void) + nz * 0.3 > 0.5 {
                VOID
            } else {
                let n2 = fbm(x as f32 * 0.09, y as f32 * 0.09, 2, 9) - 0.5;
                if bil(&|t| matches!(t, Tile::Shallow | Tile::Deep)) + n2 * 0.45 > 0.5 { if bil(&|t| t == Tile::Deep) + n2 * 0.6 > 0.5 { DEEP } else { SHALLOW } } else { FLOOR }
            };
            cls[(y * w + x) as usize] = c;
        }
    }
    // remove lone pixels and 1 px notches
    for _ in 0..2 {
        let src = cls.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = (y * w + x) as usize;
                let me = src[i];
                let mut same = 0;
                let mut counts = [0u8; 6];
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let c = src[((y + dy) * w + x + dx) as usize];
                    counts[c as usize] += 1;
                    if c == me {
                        same += 1;
                    }
                }
                if same < 3 {
                    let best = (0..6).max_by_key(|&k| counts[k]).unwrap();
                    cls[i] = best as u8;
                }
            }
        }
    }

    let mut depth = vec![0u8; n];
    let mut span = vec![0u8; n];
    let mut ledge = vec![0u8; n];

    // Cliff faces: the bottom of every rock run that has open ground below it.
    for x in 0..w {
        let fh = face_height(x);
        let mut y = 0;
        while y < h {
            if cls[(y * w + x) as usize] != WALL_TOP {
                y += 1;
                continue;
            }
            let ys = y;
            while y < h && cls[(y * w + x) as usize] == WALL_TOP {
                y += 1;
            }
            let ye = y;
            if ye >= h {
                continue;
            }
            let run = ye - ys;
            let f = if run >= fh + 4 { fh } else { (run * 3 / 4).max(run.min(6)) };
            for yy in (ye - f)..ye {
                let i = (yy * w + x) as usize;
                cls[i] = FACE;
                depth[i] = (yy - (ye - f)) as u8;
                span[i] = f as u8;
            }
        }
    }

    // Ledges: the visible cliff under the north edge of every void run.
    for x in 0..w {
        let mut y = 0;
        while y < h {
            if cls[(y * w + x) as usize] != VOID {
                y += 1;
                continue;
            }
            let ys = y;
            while y < h && cls[(y * w + x) as usize] == VOID {
                y += 1;
            }
            if ys == 0 {
                continue;
            }
            let above = cls[((ys - 1) * w + x) as usize];
            // a bridge: a short strip of floor with void above it too
            let mut k = ys - 1;
            while k > 0 && cls[(k * w + x) as usize] == FLOOR {
                k -= 1;
            }
            let bridge = above == FLOOR && ys - 1 - k <= 40 && cls[(k * w + x) as usize] == VOID;
            let kind = match above {
                SHALLOW | DEEP => 2,
                WALL_TOP | FACE => 1,
                _ => 0,
            };
            let lh = if bridge { 9 } else { 38 };
            for yy in ys..y {
                let i = (yy * w + x) as usize;
                depth[i] = (yy - ys).min(255) as u8;
                span[i] = lh as u8;
                ledge[i] = kind;
            }
        }
    }
    PixMap { w, h, cls, depth, span, ledge }
}

/// Two-pass chamfer distance (in px) to the nearest pixel where `src` is true, capped at 255.
pub fn distance(w: i32, h: i32, src: impl Fn(usize) -> bool) -> Vec<u8> {
    let n = (w * h) as usize;
    let big = 255 * 3;
    let mut d: Vec<u16> = (0..n).map(|i| if src(i) { 0 } else { big }).collect();
    let (wu, hu) = (w as usize, h as usize);
    for y in 0..hu {
        for x in 0..wu {
            let i = y * wu + x;
            let mut v = d[i];
            if x > 0 {
                v = v.min(d[i - 1] + 3);
            }
            if y > 0 {
                v = v.min(d[i - wu] + 3);
                if x > 0 {
                    v = v.min(d[i - wu - 1] + 4);
                }
                if x + 1 < wu {
                    v = v.min(d[i - wu + 1] + 4);
                }
            }
            d[i] = v;
        }
    }
    for y in (0..hu).rev() {
        for x in (0..wu).rev() {
            let i = y * wu + x;
            let mut v = d[i];
            if x + 1 < wu {
                v = v.min(d[i + 1] + 3);
            }
            if y + 1 < hu {
                v = v.min(d[i + wu] + 3);
                if x + 1 < wu {
                    v = v.min(d[i + wu + 1] + 4);
                }
                if x > 0 {
                    v = v.min(d[i + wu - 1] + 4);
                }
            }
            d[i] = v;
        }
    }
    d.iter().map(|v| (v / 3).min(255) as u8).collect()
}

/// All rooms share slate stone; the iris belongs to actual mineral sources.
fn floor_ramp(_area: usize) -> [Rgb; 6] {
    ROCK
}

/// Find an unequal broad rock plane with a slanted joint, independent of tile seams.
fn plane(x: i32, depth: i32) -> (i32, i32, i32) {
    const WIDTHS: [i32; 7] = [39, 62, 27, 47, 71, 32, 58];
    let period: i32 = WIDTHS.iter().sum();
    let fx = x + depth / 4;
    let mut local = fx.rem_euclid(period);
    for (k, width) in WIDTHS.iter().enumerate() {
        if local < *width {
            return (local, *width, fx.div_euclid(period) * 7 + k as i32);
        }
        local -= width;
    }
    unreachable!()
}

/// Which part of the hollow a world pixel belongs to (0 caves, 1 gallery, 2 grove, 3 heart),
/// with an ordered-dither seam between parts.
fn area_at(x: i32, y: i32) -> usize {
    let j = (bayer4(x, y) - 0.5) * 20.0;
    let tx = (x as f32 + j) / 16.0;
    let ty = (y as f32 - j * 0.7) / 16.0;
    if ty < 11.5 && tx > 19.0 && tx < 58.0 {
        1
    } else if tx > 44.0 && tx < 52.0 && ty < 17.0 {
        1
    } else if tx > 57.0 && ty < 30.5 {
        2
    } else if tx > 55.0 && ty >= 30.5 {
        3
    } else {
        0
    }
}

/// Terrace level of the cave floor: 0 low, 1 mid, 2 high. Smooth, wide shapes.
fn terrace(x: i32, y: i32) -> i32 {
    // Large clipped slabs, with most receiving floor left as one calm plane.
    let skew_x = x + y / 3;
    let skew_y = y - x / 7;
    let (cx, cy) = (skew_x.div_euclid(112), skew_y.div_euclid(76));
    let r = hash2(cx, cy, 41);
    if r > 0.84 && skew_x.rem_euclid(112) < 84 && skew_y.rem_euclid(76) < 48 { 2 } else { 1 }
}

pub struct Fields {
    pub d_rock: Vec<u8>,
    pub d_water: Vec<u8>,
    pub d_land: Vec<u8>,
    pub d_open: Vec<u8>,
    pub d_void: Vec<u8>,
}

pub fn fields(m: &PixMap) -> Fields {
    let (w, h) = (m.w, m.h);
    Fields {
        d_rock: distance(w, h, |i| matches!(m.cls[i], WALL_TOP | FACE)),
        d_water: distance(w, h, |i| matches!(m.cls[i], SHALLOW | DEEP)),
        d_land: distance(w, h, |i| !matches!(m.cls[i], SHALLOW | DEEP)),
        d_open: distance(w, h, |i| m.cls[i] != WALL_TOP),
        d_void: distance(w, h, |i| m.cls[i] == VOID),
    }
}

/// Paint `z.ground` from the pixel map.
pub fn paint(z: &mut Zone, m: &PixMap, fl: &Fields) {
    let (w, h) = (m.w, m.h);
    let ramps: [[Rgb; 6]; 4] = [floor_ramp(0), floor_ramp(1), floor_ramp(2), floor_ramp(3)];
    // terrace map, smoothed so no level forms a sliver
    let mut lev = vec![0i8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            lev[(y * w + x) as usize] = terrace(x, y) as i8;
        }
    }
    let lv = |x: i32, y: i32| -> i32 { if x < 0 || y < 0 || x >= w || y >= h { 1 } else { lev[(y * w + x) as usize] as i32 } };

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let c = match m.cls[i] {
                FLOOR => floor_px(m, fl, &ramps, &lv, x, y, i),
                WALL_TOP => wall_top_px(m, fl, x, y, i),
                FACE => face_px(m, x, y, i),
                SHALLOW | DEEP => water_bed_px(m, fl, x, y, i),
                _ => ledge_px(m, x, y, i),
            };
            z.ground[i] = c;
        }
    }
}


/// Lift existing shore shelves beside permanent crystals without brightening the open room.
pub fn lit_banks(z: &mut Zone, m: &PixMap, fl: &Fields, decor: &[super::Decor]) {
    for source in decor.iter().filter(|d| d.kind == super::Kind::Crystal) {
        let (cx, cy) = (source.x as i32, source.y as i32);
        for y in (cy - 64).max(0)..(cy + 64).min(m.h) {
            for x in (cx - 64).max(0)..(cx + 64).min(m.w) {
                let i = (y * m.w + x) as usize;
                if m.cls[i] != FLOOR || (x - cx).pow(2) + (y - cy).pow(2) > 64 * 64 {
                    continue;
                }
                let (joint, width, bed) = plane(x - y / 2, y);
                let shelf = bed.rem_euclid(3) == 0 && joint > 3 && joint < width - 5;
                let bank = fl.d_water[i] >= 3 && fl.d_water[i] < (6 + joint.min(width - joint) / 3) as u8;
                if shelf && bank && z.ground[i] == ROCK[3] {
                    z.ground[i] = ROCK[4];
                }
            }
        }
    }
}

fn floor_px(m: &PixMap, fl: &Fields, ramps: &[[Rgb; 6]; 4], lv: &dyn Fn(i32, i32) -> i32, x: i32, y: i32, i: usize) -> Rgb {
    let r = &ramps[area_at(x, y)];
    let raised = lv(x, y) == 2;
    let mut idx = 2;
    let (joint, width, bed) = plane(x - y / 2, y);
    // Exposed bedrock is attached to the shore, with broad breaks between shelves.
    // These are material planes, not painted pools of light around unseen emitters.
    let shelf = bed.rem_euclid(3) == 0 && joint > 3 && joint < width - 5;
    let dry_bank = fl.d_water[i] >= 3 && fl.d_water[i] < (6 + joint.min(width - joint) / 3) as u8;
    if shelf && dry_bank {
        idx = 3;
    }
    // Short tapered bedding joints cross the dry gallery and lake approach in connected runs.
    let run = (y + bed * 13).rem_euclid(59);
    if bed.rem_euclid(4) == 1 && joint <= i32::from((16..28).contains(&run)) && (12..34).contains(&run) {
        idx = 1;
    }
    // Raised slabs share the floor's quiet body value; only their physical edges change plane.
    if raised && (lv(x - 1, y) < 2 || lv(x, y - 1) < 2) {
        idx = 3;
    }
    if raised && (lv(x + 2, y) < 2 || lv(x, y + 2) < 2) {
        idx = 1;
    }
    // Only a short cap catches light at the abyss, never a complete outlined island.
    if fl.d_void[i] <= 1 && m.at(x, y + 1) == VOID {
        return r[3];
    }
    // Attached, crisp contact and source-directed shadows, without checker fringes.
    if fl.d_rock[i] <= 2 {
        return r[0];
    }
    let cast = (3..=10).any(|k| m.rock(x - k, y - k));
    if cast {
        idx = 1;
    }
    // Wet stone occupies a narrow, broken band, preserving the dry bank value.
    if fl.d_water[i] <= 2 && (x + y / 3).rem_euclid(37) < 24 {
        idx = idx.min(1);
    }
    r[idx as usize]
}

fn wall_top_px(m: &PixMap, fl: &Fields, x: i32, y: i32, i: usize) -> Rgb {
    let d = fl.d_open[i];
    let (pos, width, seed) = plane(x, y / 2);
    if d <= 2 {
        let lit = !m.rock(x - 3, y) || !m.rock(x, y - 3);
        return WALL[if lit && pos < width * 2 / 3 { 3 } else { 1 }];
    }
    // Broad connected top faces remain solid at 1x, with one occasional lost joint.
    if pos <= 1 && seed.rem_euclid(3) == 0 && d < 25 {
        return WALL[0];
    }
    WALL[if seed.rem_euclid(4) == 1 && d < 42 { 2 } else { 1 }]
}

fn face_px(m: &PixMap, x: i32, _y: i32, i: usize) -> Rgb {
    let d = m.depth[i] as i32;
    let low = m.span[i] as i32 - d;
    let (pos, width, seed) = plane(x, d);
    let lit = seed.rem_euclid(4) != 2;
    if d < 2 && pos < width - 4 {
        return WALL[if lit { 3 } else { 2 }];
    }
    if low <= 2 || pos >= width - 2 {
        return WALL[0];
    }
    if d > 9 && pos > width * 3 / 4 + d / 5 {
        return WALL[0];
    }
    WALL[if lit && pos < width / 3 - d / 8 { 2 } else { 1 }]
}

fn water_bed_px(m: &PixMap, fl: &Fields, x: i32, y: i32, i: usize) -> Rgb {
    let d = fl.d_land[i];
    if m.cls[i] == DEEP {
        // A submerged foot below an island's stone face gives the focal mineral a solid bank.
        // The open lake remains a dark depth mass, distinct from traversable shallows.
        return WATER[usize::from(d < 8 && m.rock(x, y - d as i32 - 2))];
    }
    if d <= 1 && m.at(x, y - 2) == FACE {
        return WATER[0];
    }
    // The same fractured banks continue under the shallows in a few broad connected shelves.
    // Keep the open body calm instead of outlining the entire lake.
    let (joint, width, bed) = plane(x - y / 2, y);
    let shelf = bed.rem_euclid(3) == 0 && joint > 3 && joint < width - 5;
    if shelf && d < (4 + joint.min(width - joint) / 3) as u8 {
        return WATER[2];
    }
    // Submerged bank fragments sit along selected south-facing banks, not concentric rings.
    if d <= 2 && !matches!(m.at(x, y + 3), SHALLOW | DEEP) && (x + y / 3).rem_euclid(41) < 23 {
        return WATER[2];
    }
    WATER[1]
}

fn ledge_px(m: &PixMap, x: i32, _y: i32, i: usize) -> Rgb {
    let d = m.depth[i] as i32;
    let lh = m.span[i] as i32;
    let ramp: [Rgb; 6] = if m.ledge[i] == 1 { WALL } else { ROCK };
    if d >= lh {
        return HOLE;
    }
    let (pos, width, seed) = plane(x, d);
    if d == 0 && pos < width - 6 {
        return ramp[3];
    }
    if d > lh * 3 / 4 {
        return DEEP_DARK;
    }
    ramp[if pos > width - 3 || seed.rem_euclid(3) == 0 {
        0
    } else if d < lh / 3 {
        2
    } else {
        1
    }]
}

/// Moss over the grove floor: dark, calm patches with lit upper-left edges.
pub fn moss(z: &mut Zone, m: &PixMap, fl: &Fields) {
    let w = m.w;
    for y in 0..m.h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if m.cls[i] != FLOOR || area_at(x, y) != 2 {
                continue;
            }
            let v = fbm(x as f32 * 0.035, y as f32 * 0.045, 3, 303);
            let near = fl.d_water[i] < 12 || fl.d_rock[i] < 10;
            if !near {
                continue;
            }
            let th = 0.50;
            if v < th {
                continue;
            }
            let edge = fbm((x - 1) as f32 * 0.035, (y - 1) as f32 * 0.045, 3, 303) < th;
            let c = if edge {
                CAP[1]
            } else if v > th + 0.12 && bayer4(x, y) < 0.25 {
                CAP[1]
            } else {
                CAP[0]
            };
            // keep shadows: moss in shadow uses the darkest rock
            let g = z.ground[i];
            z.ground[i] = if g.luma() < ROCK[1].luma() + 0.005 { ROCK[0].lerp(CAP[0], 0.5) } else { c };
        }
    }
}

/// Stamp sparse artist-authored fissures and stones into calm floor away from shore edges.
pub fn details(z: &mut Zone, m: &PixMap, fl: &Fields) {
    let Some(s) = sheet("glasswater_tiles") else {
        return;
    };
    let tile = |name: &str| {
        let xy = s.json.get("tiles")?.get(name)?;
        s.cells.get(xy.get(1)?.as_u64()? as usize)?.get(xy.get(0)?.as_u64()? as usize)
    };
    let names = ["decal_pebbles_a", "decal_pebbles_b", "decal_stone", "decal_crack_a", "decal_crack_b"];
    for ty in 1..z.th - 1 {
        for tx in 1..z.tw - 1 {
            let r = hash2(tx, ty, 1967);
            if r < 0.92 {
                continue;
            }
            let Some(cell) = tile(names[(hash2(tx, ty, 1968) * names.len() as f32) as usize % names.len()]) else {
                continue;
            };
            for y in 0..16 {
                for x in 0..16 {
                    let (wx, wy) = (tx * 16 + x, ty * 16 + y);
                    let i = (wy * m.w + wx) as usize;
                    let p = cell.at(x, y);
                    if !p.a || m.cls[i] != FLOOR || fl.d_rock[i] < 9 || fl.d_water[i] < 8 || fl.d_void[i] < 6 {
                        continue;
                    }
                    z.ground[i] = p.c;
                }
            }
        }
    }
}
