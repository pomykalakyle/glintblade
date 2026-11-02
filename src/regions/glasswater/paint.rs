//! Paints Glasswater Hollow's pre-rendered ground once at load: layered cave-floor terraces,
//! broad slate masses with 3/4-view faces, broken wet shores, the lakebed under
//! the water, and ledges dropping into the abyss. Quiet angular planes cross tile seams.

use super::art::{CAP, DEEP_DARK, ROCK, WALL, WATER};
use crate::gfx::{Rgb, rgb, sheet};
use crate::util::{bayer4, fbm, hash2};
use crate::world::{HOLE, Tile, Zone};

pub const FLOOR: u8 = 0;
pub const WALL_TOP: u8 = 1;
pub const FACE: u8 = 2;
pub const SHALLOW: u8 = 3;
pub const DEEP: u8 = 4;
pub const VOID: u8 = 5;

/// Quiet submerged terraces between the shallow slate and the lake's dark center.
/// These four native value masses soften the depth break without moving its physical shore.
pub const DEEP_WATER: [Rgb; 4] = [rgb(0x264e61), rgb(0x23495c), rgb(0x1f3f52), WATER[0]];

/// Pixel-level classification of the zone.
pub struct PixMap {
    pub w: i32,
    pub h: i32,
    pub cls: Vec<u8>,
    /// FACE: rows below the lip. VOID: rows below the ledge edge (255 = far below).
    pub depth: Vec<u8>,
    /// WALL_TOP/FACE: the raised wall height in this column. VOID: the hanging ledge depth.
    pub span: Vec<u8>,
    /// VOID: 0 = floor ledge, 1 = wall cliff continuing down, 2 = under water (waterfall).
    pub ledge: Vec<u8>,
}

impl PixMap {
    #[inline]
    pub fn at(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            WALL_TOP
        } else {
            self.cls[(y * self.w + x) as usize]
        }
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
            let t = [
                tile_c(z, tx, ty),
                tile_c(z, tx + 1, ty),
                tile_c(z, tx, ty + 1),
                tile_c(z, tx + 1, ty + 1),
            ];
            let bil = |p: &dyn Fn(Tile) -> bool| -> f32 {
                let v = |t: Tile| if p(t) { 1.0 } else { 0.0 };
                (v(t[0]) * (1.0 - ax) + v(t[1]) * ax) * (1.0 - ay)
                    + (v(t[2]) * (1.0 - ax) + v(t[3]) * ax) * ay
            };
            let nz = fbm(x as f32 * 0.07, y as f32 * 0.07, 2, 5) - 0.5;
            let c = if bil(&|t| t == Tile::Wall) + nz * 0.25 > 0.5 {
                WALL_TOP
            } else if bil(&|t| t == Tile::Void) + nz * 0.3 > 0.5 {
                VOID
            } else {
                let n2 = fbm(x as f32 * 0.09, y as f32 * 0.09, 2, 9) - 0.5;
                if bil(&|t| matches!(t, Tile::Shallow | Tile::Deep)) + n2 * 0.45 > 0.5 {
                    if bil(&|t| t == Tile::Deep) + n2 * 0.6 > 0.5 {
                        DEEP
                    } else {
                        SHALLOW
                    }
                } else {
                    FLOOR
                }
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
                for (dx, dy) in [
                    (-1, 0),
                    (1, 0),
                    (0, -1),
                    (0, 1),
                    (-1, -1),
                    (1, -1),
                    (-1, 1),
                    (1, 1),
                ] {
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

    // The old causeway is built masonry, not a smoothed tongue of cave floor.
    // Keep the authored two-tile crossing straight right up to its visible curb.
    for y in 15 * 16..(17 * 16).min(h) {
        for x in 47 * 16..(59 * 16).min(w) {
            cls[(y * w + x) as usize] = FLOOR;
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
            let run = ye - ys;
            let f = if run >= fh + 4 {
                fh
            } else {
                (run * 3 / 4).max(run.min(6))
            };
            // The cap and its exposed face belong to one rock mass. Retain its
            // altitude even in quiet cap interiors that have no native atlas
            // sample, and for masses whose front is beyond the map boundary.
            for yy in ys..ye {
                span[(yy * w + x) as usize] = f as u8;
            }
            if ye >= h {
                continue;
            }
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
    PixMap {
        w,
        h,
        cls,
        depth,
        span,
        ledge,
    }
}

/// Bonded slate over the rift, with real landings and an arch-shaped stone soffit.
/// A distinct deck separates this crossing from natural shelves elsewhere in the cave.
pub fn bridge(z: &mut Zone, m: &PixMap) {
    const X0: i32 = 47 * 16;
    const X1: i32 = 59 * 16;
    const Y0: i32 = 15 * 16;
    const Y1: i32 = 17 * 16;
    for y in Y0..Y1 {
        for x in X0..X1 {
            if m.at(x, y) != FLOOR {
                continue;
            }
            let ly = y - Y0;
            let course = ly / 8;
            let bond = (x - X0 + course * 11).rem_euclid(23);
            let stone = hash2((x - X0 + course * 11).div_euclid(23), course, 870);
            let mut color = if stone < 0.35 { ROCK[2] } else { WALL[2] };
            if ly % 8 == 7 || bond == 22 {
                color = ROCK[1];
            }
            // Two low stone curbs, rather than a railing that hides actors.
            if ly < 2 {
                color = if ly == 0 { WALL[3] } else { WALL[1] };
            }
            if ly >= 29 {
                color = if ly == 29 { WALL[3] } else { WALL[1] };
            }
            // A pair of broad threshold blocks makes each landing visibly load-bearing.
            if x < X0 + 9 || x >= X1 - 9 {
                color = if (y - Y0).rem_euclid(11) == 10 {
                    ROCK[1]
                } else {
                    WALL[2]
                };
                if x == X0 || x == X1 - 9 {
                    color = WALL[3];
                }
            }
            z.gset(x, y, color);
            let (normal, height) = bridge_deck_surface(x - X0, ly);
            z.gset_plane_surface(x, y, normal, height);
        }
    }
    for x in X0..X1 {
        let across = (x - (X0 + X1) / 2).abs();
        // Taller haunches enter the receiving banks; the center is a shallow keystone.
        let depth = 7 + across * across / 800;
        for j in 0..depth {
            let y = Y1 + j;
            if m.at(x, y) != VOID {
                continue;
            }
            let mortar = (x - X0 + (j / 5) * 12).rem_euclid(25) == 24;
            let color = if j == 0 {
                WALL[2]
            } else if j == depth - 1 || mortar || j % 5 == 4 {
                ROCK[0]
            } else if j < 4 {
                WALL[1]
            } else {
                ROCK[1]
            };
            z.gset(x, y, color);
            // This masonry hangs below the crossing; its visible face cannot
            // inherit the raised natural-cliff metadata painted here earlier.
            z.gset_plane_surface(x, y, if j == 0 { [0.0, 0.7, 0.7] } else { [0.0, 1.0, 0.0] }, 0.0);
        }
        // Clear the old 38px natural ledge below the new, shallower bridge arch.
        for j in depth..38 {
            let y = Y1 + j;
            if m.at(x, y) == VOID && m.ledge[(y * m.w + x) as usize] == 0 {
                z.gset(x, y, HOLE);
            }
        }
    }
}

/// Native deck bonds and landing blocks use the same coordinates as their albedo.
/// Mortar remains a quiet recess; only the stone's one-pixel bevel changes normal.
fn bridge_deck_surface(x: i32, y: i32) -> ([f32; 3], f32) {
    if x < 9 || x >= 12 * 16 - 9 {
        let row = y.rem_euclid(11);
        if row == 10 { return ([0.0, 0.0, 1.0], 0.0); }
        let nx = if x == 0 || x == 12 * 16 - 9 { -0.45 } else { 0.0 };
        let ny = if row == 0 { -0.35 } else if row == 9 { 0.35 } else { 0.0 };
        return ([nx, ny, 1.0], 0.0);
    }
    match y {
        0 | 29 => return ([0.0, 0.0, 1.0], 2.0),
        1 => return ([0.0, 1.0, 0.0], 1.0),
        30 | 31 => return ([0.0, 1.0, 0.0], (31 - y) as f32),
        _ => {}
    }
    let row = y.rem_euclid(8);
    let bond = (x + y / 8 * 11).rem_euclid(23);
    if row == 7 || bond == 22 { return ([0.0, 0.0, 1.0], 0.0); }
    let nx = if bond == 0 { -0.4 } else if bond == 21 { 0.4 } else { 0.0 };
    let ny = if row == 0 { -0.35 } else if row == 6 { 0.35 } else { 0.0 };
    ([nx, ny, 1.0], 0.0)
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
    if r > 0.84 && skew_x.rem_euclid(112) < 84 && skew_y.rem_euclid(76) < 48 {
        2
    } else {
        1
    }
}

pub struct Fields {
    pub d_rock: Vec<u8>,
    pub d_water: Vec<u8>,
    pub d_land: Vec<u8>,
    pub d_open: Vec<u8>,
    pub d_void: Vec<u8>,
    pub d_shallow: Vec<u8>,
}

pub fn fields(m: &PixMap) -> Fields {
    let (w, h) = (m.w, m.h);
    Fields {
        d_rock: distance(w, h, |i| matches!(m.cls[i], WALL_TOP | FACE)),
        d_water: distance(w, h, |i| matches!(m.cls[i], SHALLOW | DEEP)),
        d_land: distance(w, h, |i| !matches!(m.cls[i], SHALLOW | DEEP)),
        d_open: distance(w, h, |i| m.cls[i] != WALL_TOP),
        d_void: distance(w, h, |i| m.cls[i] == VOID),
        d_shallow: distance(w, h, |i| m.cls[i] == SHALLOW),
    }
}

/// Paint `z.ground` from the pixel map.
pub fn paint(z: &mut Zone, m: &PixMap, fl: &Fields) {
    // Painted wall caps represent solid rock down to the walkable datum.
    // Their top and front sheets alone leave the interior empty: a low ray
    // entering from the north can otherwise pass through the entire mass.
    let cap: Vec<_> = m.cls.iter().zip(&m.span)
        .map(|(&kind, &height)| (kind == WALL_TOP && height > 0).then_some(height))
        .collect();
    z.terrain_volumes.clear();
    z.add_cap_volumes(&cap);
    let (w, h) = (m.w, m.h);
    let ramps: [[Rgb; 6]; 4] = [floor_ramp(0), floor_ramp(1), floor_ramp(2), floor_ramp(3)];
    // terrace map, smoothed so no level forms a sliver
    let mut lev = vec![0i8; (w * h) as usize];
    let generated = super::materials::available();
    for y in 0..h {
        for x in 0..w {
            lev[(y * w + x) as usize] = if generated { 1 } else { terrace(x, y) as i8 };
        }
    }
    let lv = |x: i32, y: i32| -> i32 {
        if x < 0 || y < 0 || x >= w || y >= h {
            1
        } else {
            lev[(y * w + x) as usize] as i32
        }
    };

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
            // The painted geometry is authoritative before optional native
            // art is applied. A missing atlas must not flatten raised caps or
            // remove their upright face from local-light occlusion.
            let surface = match m.cls[i] {
                WALL_TOP => Some(([0.0, 0.0, 1.0], f32::from(m.span[i]))),
                FACE => Some(([0.0, 1.0, 0.0], f32::from(m.span[i] - m.depth[i]))),
                // These faces descend below the walkable floor; the compact
                // unsigned terrain format represents them at its lower bound.
                // They must never become above-floor shadow casters.
                VOID if c.r >= 0.0 && m.ledge[i] != 2 => Some(([0.0, 1.0, 0.0], 0.0)),
                _ => None,
            };
            if let Some((normal, height)) = surface {
                z.gset_plane_surface(x, y, normal, height);
            }
        }
    }
}

/// Complete the procedural shore margins without replacing the exact native
/// material normals in the room centers. Water is a horizontal receiving plane;
/// its animated ripples remain owned by the existing water renderer.
pub fn walkable_surfaces(z: &mut Zone, m: &PixMap, fl: &Fields) {
    let generated = super::materials::available();
    z.fill_walkable_surfaces(|x, y| {
        if m.at(x, y) == SHALLOW { return ([0.0, 0.0, 1.0], 0.0); }
        let relief = |xx, yy| {
            if m.at(xx, yy) != FLOOR { return 0.0; }
            let i = (yy * m.w + xx) as usize;
            let (shelf, joint) = floor_bedding(fl, xx, yy, i);
            let slab = if !generated && terrace(xx, yy) == 2 { 0.7 } else { 0.0 };
            slab + if joint { -0.35 } else if shelf { 0.5 } else { 0.0 }
        };
        (relief_normal(x, y, relief), 0.0)
    });
}

/// Slab relief is authored from the painter's masks, never its light or palette.
fn relief_normal(x: i32, y: i32, relief: impl Fn(i32, i32) -> f32) -> [f32; 3] {
    [
        (relief(x - 1, y) - relief(x + 1, y)) * 0.65,
        (relief(x, y - 1) - relief(x, y + 1)) * 0.65,
        1.0,
    ]
}

fn floor_bedding(fl: &Fields, x: i32, y: i32, i: usize) -> (bool, bool) {
    let (joint, width, bed) = plane(x - y / 2, y);
    let shelf = bed.rem_euclid(3) == 0 && joint > 3 && joint < width - 5;
    let dry_bank = fl.d_water[i] >= 3 && fl.d_water[i] < (6 + joint.min(width - joint) / 3) as u8;
    let run = (y + bed * 13).rem_euclid(59);
    let seam = bed.rem_euclid(4) == 1
        && joint <= i32::from((16..28).contains(&run))
        && (12..34).contains(&run);
    (shelf && dry_bank, seam)
}

fn lit_bank_depth(x: i32, y: i32) -> Option<i32> {
    let (joint, width, bed) = plane(x - y / 2, y);
    (bed.rem_euclid(4) == 1 && joint > 4 && joint < width - 5)
        .then_some(4 + (joint.min(width - joint) / 4).min(7))
}

/// Wet mineral shelves beside permanent crystals. Their broken faces follow
/// real receiving banks and the cave's bedding, rather than drawing a circular
/// aura into the floor. Open room centers and every physical wet pixel stay intact.
pub fn lit_banks(z: &mut Zone, m: &PixMap, fl: &Fields, decor: &[super::Decor]) {
    for source in decor.iter().filter(|d| d.kind == super::Kind::Crystal) {
        let (cx, cy) = (source.x.floor() as i32, source.y.floor() as i32);
        let reach = if source.cell.h >= 60 { 76 } else { 45 };
        for y in (cy - reach).max(0)..(cy + reach).min(m.h) {
            for x in (cx - reach).max(0)..(cx + reach).min(m.w) {
                let i = (y * m.w + x) as usize;
                if m.cls[i] != FLOOR || (x - cx).pow(2) + (y - cy).pow(2) > reach * reach {
                    continue;
                }
                let d = fl.d_water[i] as i32;
                if !(2..=13).contains(&d) || fl.d_rock[i] < 3 {
                    continue;
                }
                let (joint, width, _) = plane(x - y / 2, y);
                // Unequal low ledges follow selected natural bedding runs, with
                // long untouched banks between them. A bevel is just one pixel.
                let Some(depth) = lit_bank_depth(x, y) else { continue };
                if d > depth {
                    continue;
                }
                let old = z.ground[i];
                // Preserve the existing physical rock contact shadows and
                // authored cracks; the wet body uses close slate/teal values.
                if old.luma() < ROCK[1].luma() + 0.012 {
                    continue;
                }
                let color = if d == depth && joint > 9 && joint < width - 9 {
                    ROCK[2].lerp(ROCK[3], 0.30)
                } else if d <= 3 {
                    ROCK[1].lerp(WATER[1], 0.20)
                } else {
                    ROCK[2].lerp(WATER[2], 0.22)
                };
                z.ground[i] = color;
                // This wet shelf has replaced the original material pixel.
                // Keep its bevel attached to the same broken bedding contour.
                let relief = |xx, yy| {
                    if m.at(xx, yy) != FLOOR { return 0.0; }
                    let ii = (yy * m.w + xx) as usize;
                    let dd = i32::from(fl.d_water[ii]);
                    if (2..=13).contains(&dd) && fl.d_rock[ii] >= 3
                        && lit_bank_depth(xx, yy).is_some_and(|depth| dd <= depth) { 0.5 } else { 0.0 }
                };
                z.gset_plane_surface(x, y, relief_normal(x, y, relief), 0.0);
            }
        }
    }
}

fn floor_px(
    m: &PixMap,
    fl: &Fields,
    ramps: &[[Rgb; 6]; 4],
    lv: &dyn Fn(i32, i32) -> i32,
    x: i32,
    y: i32,
    i: usize,
) -> Rgb {
    let r = &ramps[area_at(x, y)];
    let raised = lv(x, y) == 2;
    let mut idx = 2;
    // Exposed bedrock is attached to the shore, with broad breaks between shelves.
    // These are material planes, not painted pools of light around unseen emitters.
    let (shelf, joint) = floor_bedding(fl, x, y, i);
    if shelf {
        idx = 3;
    }
    // Short tapered bedding joints cross the dry gallery and lake approach in connected runs.
    if joint {
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
    // A fixed down-right shadow contradicts the local crystals and moving
    // emitters. The two-pixel attached contact above remains part of the stone.
    let cast = crate::gfx::lighting::legacy_mode() && (3..=10).any(|k| m.rock(x - k, y - k));
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
    WALL[if seed.rem_euclid(4) == 1 && d < 42 {
        2
    } else {
        1
    }]
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
        // Underwater terrace fragments carry the eye into the depth instead of cutting one
        // enormous ink disk out of the lake. The dark center and physical Deep mask remain.
        return if d < 8 && m.rock(x, y - d as i32 - 2) {
            WATER[1]
        } else {
            DEEP_WATER[deep_water_level(fl, x, y, i)]
        };
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

/// Three unequal submerged contour intervals, offset by the lakebed's broad stone planes.
/// This is baked once and shared by the still bed and the animated surface; no shimmer,
/// ordered noise, smooth gradient or geometry change is involved.
pub fn deep_water_level(fl: &Fields, x: i32, y: i32, i: usize) -> usize {
    let (_, _, bed) = plane(x - y / 3, y / 2);
    let d = fl.d_shallow[i] as i32 + bed.rem_euclid(7) - 3;
    if d < 6 {
        0
    } else if d < 16 {
        1
    } else if d < 31 {
        2
    } else {
        3
    }
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
            z.ground[i] = if g.luma() < ROCK[1].luma() + 0.005 {
                ROCK[0].lerp(CAP[0], 0.5)
            } else {
                c
            };
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
        s.cells
            .get(xy.get(1)?.as_u64()? as usize)?
            .get(xy.get(0)?.as_u64()? as usize)
    };
    let names = [
        "decal_pebbles_a",
        "decal_pebbles_b",
        "decal_stone",
        "decal_crack_a",
        "decal_crack_b",
    ];
    for ty in 1..z.th - 1 {
        for tx in 1..z.tw - 1 {
            let r = hash2(tx, ty, 1967);
            if r < 0.92 {
                continue;
            }
            let Some(cell) =
                tile(names[(hash2(tx, ty, 1968) * names.len() as f32) as usize % names.len()])
            else {
                continue;
            };
            for y in 0..16 {
                for x in 0..16 {
                    let (wx, wy) = (tx * 16 + x, ty * 16 + y);
                    let i = (wy * m.w + wx) as usize;
                    let p = cell.at(x, y);
                    if !p.a
                        || m.cls[i] != FLOOR
                        || fl.d_rock[i] < 9
                        || fl.d_water[i] < 8
                        || fl.d_void[i] < 6
                    {
                        continue;
                    }
                    z.ground[i] = p.c;
                    z.gset_surface(wx, wy, cell, x, y, false, 0.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod normal_tests {
    use super::*;
    use crate::gfx::lighting::TerrainSurface;
    use crate::world::ZoneId;

    /// An actual accessible receiver west of the bridge was lit through the
    /// middle of the 30px-high wall to its east. Top/front sheets do not close
    /// that mass; the cap extrusion must, while retaining open and above-cap rays.
    #[test]
    fn raised_rock_is_solid_to_low_light_without_sealing_open_floor_or_its_top() {
        use crate::gfx::{Frame, Rgb, W, lighting, surface};
        let (mut z, region) = super::super::build();
        assert!(!z.blocked(644.0, 248.0, 4.0));
        assert!(!z.blocked(731.0, 248.0, 4.0));
        assert!(!z.blocked(731.0, 260.0, 4.0), "the projected emitter at Y248/Z12 stands on accessible ground Y260");
        assert!(!z.blocked(620.0, 248.0, 4.0), "the open-floor control is an accessible source position");
        assert!(z.terrain_volumes.len() > 100, "exercise the native cliff masses");
        let areas: Vec<_> = z.terrain_volumes.iter()
            .map(|v| (v.max[0] - v.min[0]) * (v.max[1] - v.min[1])).collect();
        eprintln!("Glasswater solid caps: {} rectangles, mean area {:.2}px, max area {:.0}px",
            areas.len(), areas.iter().sum::<f32>() / areas.len() as f32,
            areas.iter().copied().fold(0.0, f32::max));
        let response = |z: &Zone, target: (i32, i32), source: (f32, f32, f32)| {
            let mut f = Frame::new();
            f.cam_x = 480;
            f.cam_y = 160;
            f.begin(Rgb::BLACK, Rgb::BLACK);
            f.surface_lighting = true;
            lighting::prepare(z, region.as_ref());
            let index = ((target.1 - f.cam_y) as usize) * W + (target.0 - f.cam_x) as usize;
            // Isolate world geometry from both microscopic incidence and
            // viewport surface shadows. A wall must also occlude when its
            // relevant cap pixels lie outside the rendered viewport.
            let height = z.ground_surfaces[(target.1 * z.pw + target.0) as usize].height;
            f.surfaces[index] = crate::gfx::frame::Surface {
                normal: [0.0, 0.0, 1.0], height: f32::from(height), owner: 0,
                kind: if height > 0 { 2 } else { 1 },
            };
            f.point_lights.push(crate::gfx::frame::PointLight {
                x: source.0, y: source.1, height: source.2, radius: 140.0,
                color: Rgb::WHITE, intensity: 1.0,
            });
            surface::apply_world(z, &mut f);
            f.light[index].r
        };
        assert_eq!(response(&z, (644, 248), (731.0, 248.0, 12.0)), 0.0,
            "the source reaches floor through solid raised rock");
        assert!(response(&z, (644, 248), (620.0, 236.0, 12.0)) > 0.1,
            "the mass must not seal adjacent open floor");
        assert!(response(&z, (680, 218), (680.5, 248.5, 50.0)) > 0.1,
            "a source above the cap must still light its top");
        z.terrain_volumes.clear();
        z.invalidate_lighting();
        let sheets = response(&z, (644, 248), (731.0, 248.0, 12.0));
        assert!(sheets > 0.03,
            "this regression must exercise the empty-interior failure of native sheets alone: {sheets}");
    }

    #[test]
    fn raised_wall_caps_and_faces_share_their_authored_altitude() {
        let (mut z, _) = super::super::build();
        let m = classify(&z);
        let fl = fields(&m);
        let built = z.ground_surfaces.clone();
        let mut counts = [0usize; 3];
        for i in 0..m.cls.len() {
            let expected = match m.cls[i] {
                WALL_TOP => {
                    counts[0] += 1;
                    assert!(m.span[i] > 0, "raised cap lacks its wall height at {i}");
                    m.span[i]
                }
                FACE => {
                    counts[1] += 1;
                    m.span[i] - m.depth[i]
                }
                VOID if z.ground[i].r >= 0.0 && m.ledge[i] != 2 => {
                    counts[2] += 1;
                    0
                }
                _ => continue,
            };
            assert_eq!(built[i].height, expected, "wrong native terrain elevation at {i}");
        }
        assert!(counts.iter().all(|&n| n > 1_000), "exercise real caps, faces and hanging ledges: {counts:?}");

        // Re-run the authored painter alone, without native material maps. Its
        // structural heights must agree with the fully composed atlas result.
        z.ground_surfaces.fill(TerrainSurface::default());
        paint(&mut z, &m, &fl);
        for i in 0..m.cls.len() {
            if matches!(m.cls[i], WALL_TOP | FACE)
                || m.cls[i] == VOID && z.ground[i].r >= 0.0 && m.ledge[i] != 2
            {
                assert!(z.ground_surfaces[i].present(), "procedural wall lacks geometry at {i}");
                assert_eq!(z.ground_surfaces[i].height, built[i].height,
                    "native artwork changed the authored wall geometry at {i}");
            }
            if m.cls[i] == FACE && m.depth[i] == 0 {
                let above = i - m.w as usize;
                if m.cls[above] == WALL_TOP {
                    assert_eq!(z.ground_surfaces[above].height, z.ground_surfaces[i].height,
                        "cap/front lip has a discontinuity at {i}");
                }
            }
        }
    }

    #[test]
    fn low_blade_light_does_not_light_the_entry_south_shelf_but_can_light_a_wall_front() {
        use crate::gfx::{Frame, Rgb, W, lighting, surface};

        let (z, region) = super::super::build();
        let m = classify(&z);
        let (cap_x, cap_y) = (42, 468);
        let cap_i = (cap_y * z.pw + cap_x) as usize;
        assert_eq!(m.cls[cap_i], WALL_TOP, "exercise the actual raised shelf south of the entry");
        assert_eq!(z.tile_at(42.5, 460.5), Tile::Floor, "the blade source stands on the entry floor");
        assert!(z.ground_surfaces[cap_i].height > 12);

        let mut f = Frame::new();
        f.begin(Rgb::BLACK, Rgb::BLACK);
        f.surface_lighting = true;
        f.cam_y = 400;
        lighting::ground_surface(&z, region.as_ref(), &mut f);
        f.add_light_3d_w(42.5, 460.5, 12.0, 80.0, Rgb::WHITE, 1.0);
        surface::apply_world(&z, &mut f);
        let cap_screen = (cap_y - f.cam_y) as usize * W + cap_x as usize;
        let cap_light = f.light[cap_screen];

        // A vertical face toward a source on its open floor remains eligible.
        // Do not fix the shelf by suppressing light on all blocked terrain.
        let (face_x, face_y) = (0..m.cls.len()).find_map(|i| {
            if m.cls[i] != FACE || m.span[i] - m.depth[i] != 8 { return None; }
            let (x, y) = (i as i32 % m.w, i as i32 / m.w);
            if x < 16 || x >= 300 || y < 300 || y >= 500 { return None; }
            (z.tile_at(x as f32 + 0.5, y as f32 + 16.5) == Tile::Floor).then_some((x, y))
        }).expect("entry has a visible eight-pixel-high wall face beside open floor");
        let face_i = (face_y * m.w + face_x) as usize;
        assert_eq!(z.ground_surfaces[face_i].height, 8);
        let mut side = Frame::new();
        side.begin(Rgb::BLACK, Rgb::BLACK);
        side.surface_lighting = true;
        side.cam_y = face_y - 100;
        lighting::ground_surface(&z, region.as_ref(), &mut side);
        side.add_light_3d_w(face_x as f32 + 0.5, face_y as f32 + 16.5,
            12.0, 80.0, Rgb::WHITE, 1.0);
        surface::apply_world(&z, &mut side);
        let face_screen = 100 * W + face_x as usize;
        assert!(side.light[face_screen].r > 0.1,
            "a facing vertical wall must still receive the nearby blade: ({face_x}, {face_y}), {:?}",
            side.light[face_screen]);
        assert_eq!(cap_light, Rgb::BLACK,
            "a low blade on the north floor lights the top of the higher south shelf");
    }

    #[test]
    fn upright_native_facets_keep_the_wall_front_macro_orientation() {
        use crate::gfx::{Frame, Rgb, W, lighting, surface};

        let (z, region) = super::super::build();
        let m = classify(&z);
        // Both native wall facets tilt upward enough that their micro normal
        // alone looks like a cap. Their authored source basis is still upright.
        for (x, y) in [(95, 303), (129, 302)] {
            let i = (y * z.pw + x) as usize;
            let native = z.ground_surfaces[i];
            assert_eq!(m.cls[i], FACE);
            assert!(native.upright);
            assert!(native.normal[2] > 63, "exercise an upward-tilting front facet at ({x}, {y})");
            let source_y = y as f32 + f32::from(native.height) + 8.5;
            assert_eq!(z.tile_at(x as f32 + 0.5, source_y), Tile::Floor);

            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::BLACK);
            f.surface_lighting = true;
            f.cam_y = 200;
            lighting::ground_surface(&z, region.as_ref(), &mut f);
            let screen_i = (y - f.cam_y) as usize * W + x as usize;
            assert_eq!(f.surfaces[screen_i].kind, 5, "retain the authored front instead of classifying its local normal");
            f.add_light_3d_w(x as f32 + 0.5, source_y, 12.0, 80.0, Rgb::WHITE, 1.0);
            surface::apply_world(&z, &mut f);
            assert!(f.light[screen_i].r > 0.1,
                "a lower light facing the front must reach its upward-tilting native facet at ({x}, {y}): {:?}", f.light[screen_i]);
        }
    }

    #[test]
    fn procedural_margins_preserve_native_material_normals_and_painted_ground() {
        let (mut z, _) = super::super::build();
        let m = classify(&z);
        let fl = fields(&m);
        // Recreate the material stage so the unfilled banks and water are real
        // authored margins, alongside actual atlas normals at exact source UVs.
        z.ground_surfaces.fill(TerrainSurface::default());
        super::super::materials::paint(&mut z, &m, &fl);
        let source = z.ground_surfaces.clone();
        let albedo = z.ground.clone();
        let collision = z.surface_tiles.clone();
        walkable_surfaces(&mut z, &m, &fl);
        let (mut native, mut procedural, mut bevels, mut shallow) = (0, 0, 0, 0);
        for i in 0..m.cls.len() {
            if source[i].present() {
                native += 1;
                assert_eq!(z.ground_surfaces[i], source[i], "native source normal changed at {i}");
            } else if matches!(m.cls[i], FLOOR | SHALLOW) {
                procedural += 1;
                let surface = z.ground_surfaces[i];
                assert!(surface.present(), "walkable margin lacks a normal at {i}");
                if m.cls[i] == SHALLOW {
                    shallow += 1;
                    assert_eq!(surface.normal, [0, 0, 127]);
                } else if surface.normal[0] != 0 || surface.normal[1] != 0 {
                    bevels += 1;
                }
            }
        }
        assert!(native > 100_000, "exercise the real native material maps");
        assert!(procedural > 10_000 && shallow > 10_000 && bevels > 100,
            "exercise dry bank bevels and the walkable water: {procedural}/{shallow}/{bevels}");
        assert_eq!(z.ground, albedo, "normal completion must preserve painted albedo");
        assert_eq!(z.surface_tiles, collision, "normal completion must preserve movement");
    }

    #[test]
    fn bridge_replaces_old_floor_and_cliff_normals_at_its_native_bonds() {
        let mut z = Zone::new(ZoneId::Glasswater, 60, 20);
        let n = (z.pw * z.ph) as usize;
        let mut m = PixMap { w: z.pw, h: z.ph, cls: vec![FLOOR; n], depth: vec![0; n],
            span: vec![38; n], ledge: vec![0; n] };
        for y in 272..m.h {
            m.cls[(y * m.w) as usize..((y + 1) * m.w) as usize].fill(VOID);
        }
        let old = TerrainSurface::new([1.0, 0.0, 0.0], 55.0);
        z.ground_surfaces.fill(old);
        bridge(&mut z, &m);
        let at = |x, y| z.ground_surfaces[(y * z.pw + x) as usize];
        assert_eq!(at(800, 244).normal, [0, 0, 127], "stone body faces up");
        assert!(at(775, 244).normal[0] < 0, "west bond bevel follows the painted bond");
        assert!(at(796, 244).normal[0] > 0, "east bond bevel follows the painted bond");
        assert_eq!(at(800, 247).normal, [0, 0, 127], "mortar is a recessed plane");
        assert_eq!(at(800, 240), TerrainSurface::new([0.0, 0.0, 1.0], 2.0));
        assert_eq!(at(800, 241), TerrainSurface::new([0.0, 1.0, 0.0], 1.0));
        assert_eq!(at(756, 241), TerrainSurface::new([0.0, 0.0, 1.0], 0.0),
            "landing blocks override the curbs in both art and normals");
        assert_eq!(at(800, 273), TerrainSurface::new([0.0, 1.0, 0.0], 0.0),
            "the arch hangs below the deck rather than rising above it");
        assert!(!at(800, 290).present(), "cleared arch space retains no stale cliff normal");
        assert_eq!(at(751, 244), old, "nearby original material remains untouched");
    }

    #[test]
    fn bank_relief_ignores_albedo_and_lighting_values() {
        let mut a = Zone::new(ZoneId::Glasswater, 12, 8);
        let mut b = Zone::new(ZoneId::Glasswater, 12, 8);
        for y in 5..8 {
            for x in 0..12 {
                a.set_tile(x, y, Tile::Shallow);
                b.set_tile(x, y, Tile::Shallow);
            }
        }
        a.ground.fill(ROCK[0]);
        for (i, color) in b.ground.iter_mut().enumerate() {
            *color = ROCK[i % ROCK.len()];
        }
        let m = classify(&a);
        let fl = fields(&m);
        let tiles: Vec<_> = m.cls.iter().map(|&c| if c == SHALLOW { Tile::Shallow } else { Tile::Floor }).collect();
        a.set_surface_tiles(tiles.clone());
        b.set_surface_tiles(tiles);
        walkable_surfaces(&mut a, &m, &fl);
        walkable_surfaces(&mut b, &m, &fl);
        assert_eq!(a.ground_surfaces, b.ground_surfaces, "palette or painted light cannot become relief");
        assert!(a.ground_surfaces.iter().any(|s| s.normal[0] != 0 || s.normal[1] != 0),
            "the actual shore bedding has geometric bevels");
    }
}
