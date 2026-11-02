//! Paints the Summit's ground once at load: walkable stone tops, masonry rim walls in 3/4 view,
//! rocky undersides hanging into the void, the stair, the dais, inlays and joint-bound wear.
//! Everything is built from designed shapes (ellipse courses, broad stone courses and broken buttresses), never noise.

use super::{ARENA, ARENA_FACE, BRIDGE, DAIS, Ellipse, LANDING, STAIR, TERRACE, pal::*};
use crate::gfx::{Rgb, sheet};
use crate::util::{hash1, hash2};
use crate::world::{HOLE, TILE, Tile, Zone};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

const L_ARENA: u8 = 1;
const L_STAIR: u8 = 2;
const L_TERRACE: u8 = 3;
const L_BRIDGE: u8 = 4;
const L_LANDING: u8 = 5;
const ARENA_FLOOR_BOUNDS: &[f32] = &[53.0, 101.0, 148.0, 192.0, 214.0];
const STAIR_TOP: i32 = 662;

struct Canvas<'a> {
    z: &'a mut Zone,
    lab: Vec<u8>,
}

impl Canvas<'_> {
    #[inline]
    fn lab(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.z.pw || y >= self.z.ph {
            0
        } else {
            self.lab[(y * self.z.pw + x) as usize]
        }
    }
    #[inline]
    fn set(&mut self, x: i32, y: i32, c: Rgb) {
        self.z.gset(x, y, c);
    }
    #[inline]
    fn get(&self, x: i32, y: i32) -> Rgb {
        self.z.gget(x, y)
    }
}

fn in_rect(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && x < r.2 && y >= r.1 && y < r.3
}

/// Paint all platform tops and undersides, then build the walkable collision grid.
pub fn paint(z: &mut Zone) {
    let mut c = paint_geometry(z);
    walkable_surfaces(&mut c);
    material_surfaces(&mut c);
}

/// Keep the authored color/collision pass independent of its final receiver metadata.
fn paint_geometry(z: &mut Zone) -> Canvas<'_> {
    let (pw, ph) = (z.pw, z.ph);
    z.ground.fill(HOLE);
    let mut lab = vec![0u8; (pw * ph) as usize];
    for y in 0..ph {
        for x in 0..pw {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let l = if ARENA.contains(fx, fy) {
                L_ARENA
            } else if in_rect(STAIR, x, y) {
                L_STAIR
            } else if TERRACE.contains(fx, fy) {
                L_TERRACE
            } else if LANDING.contains(fx, fy) {
                L_LANDING
            } else if in_rect(BRIDGE, x, y) {
                L_BRIDGE
            } else {
                0
            };
            lab[(y * pw + x) as usize] = l;
        }
    }
    let mut c = Canvas { z, lab };

    faces(&mut c);
    bridge_parapet(&mut c);
    arena_top(&mut c);
    observatory_inlay(&mut c);
    seal(&mut c);
    small_top(&mut c, &TERRACE, L_TERRACE, &[50.0], 7.0, 11);
    small_top(&mut c, &LANDING, L_LANDING, &[56.0], 7.0, 23);
    bridge_deck(&mut c);
    stair(&mut c);
    dais(&mut c);
    cracks(&mut c);
    floor_wear(&mut c);
    native_floor(&mut c);

    // Collision: a tile is floor if its center is on a walkable top.
    let (tw, th) = (c.z.tw, c.z.th);
    for ty in 0..th {
        for tx in 0..tw {
            let l = c.lab(tx * TILE + TILE / 2, ty * TILE + TILE / 2);
            c.z.set_tile(tx, ty, if l != 0 { Tile::Floor } else { Tile::Void });
        }
    }
    // Install after coarse setters: set_tile deliberately updates any existing precise mask.
    // Exact native top pixels own feet contact; masonry/undersides remain non-walkable.
    let surface: Vec<Tile> = c
        .lab
        .iter()
        .map(|&l| if l != 0 { Tile::Floor } else { Tile::Void })
        .collect();
    c.z.set_surface_tiles(surface);
    c
}

// ------------------------------------------------------------------------------------------
// Rim walls and undersides

fn face_height(l: u8) -> i32 {
    match l {
        L_ARENA => ARENA_FACE,
        L_TERRACE => 30,
        L_LANDING => 26,
        L_BRIDGE => 14,
        _ => 0,
    }
}

fn platform_of(l: u8) -> Option<Ellipse> {
    match l {
        L_ARENA => Some(ARENA),
        L_TERRACE => Some(TERRACE),
        L_LANDING => Some(LANDING),
        _ => None,
    }
}

/// Depth of the rock hanging under a platform's rim wall at column x.
fn under_depth(l: u8, x: i32) -> i32 {
    let xf = x as f32 + 0.5;
    if l == L_BRIDGE {
        return bridge_under_depth(x);
    }
    let Some(e) = platform_of(l) else { return 0 };
    let u = ((xf - e.cx) / e.rx).clamp(-1.0, 1.0);
    let base = match l {
        L_ARENA => 150.0,
        L_TERRACE => 70.0,
        _ => 84.0,
    };
    let cone = base * (1.0 - u * u).max(0.0).powf(1.15);
    // Unequal joined buttresses, with broad ledges and only two deep broken ends.
    let profile = [
        (-1.0, 0.12),
        (-0.82, 0.42),
        (-0.64, 0.35),
        (-0.44, 0.87),
        (-0.28, 0.95),
        (-0.16, 0.65),
        (0.12, 0.58),
        (0.26, 1.0),
        (0.40, 0.92),
        (0.53, 0.42),
        (0.76, 0.47),
        (1.0, 0.06),
    ];
    for pair in profile.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        if u >= ax && u <= bx {
            let t = (u - ax) / (bx - ax);
            return (cone * (ay + (by - ay) * t) + 5.0) as i32;
        }
    }
    4
}

fn bridge_under_depth(x: i32) -> i32 {
    // One shallow load-bearing arch springs from the two actual island foundations.
    // The previous arcade had vertical piers terminating in empty sky between islands.
    let (left, right) = bridge_springlines();
    let u = ((x as f32 + 0.5 - (left + right) * 0.5) / ((right - left) * 0.5)).clamp(-1.0, 1.0);
    // Its 18px spring stays inside the adjacent 26px rim and rock shoulder. A tall
    // arch would simply move the unsupported tips to the ends of the span.
    (7.0 + 11.0 * (1.0 - (1.0 - u * u).sqrt())) as i32
}

/// Intersections of the deck's visible south edge with its real supporting platforms.
fn bridge_springlines() -> (f32, f32) {
    let y = BRIDGE.3 as f32 - 0.5;
    let half = |e: Ellipse| e.rx * (1.0 - ((y - e.cy) / e.ry).powi(2)).max(0.0).sqrt();
    (LANDING.cx + half(LANDING), TERRACE.cx - half(TERRACE))
}

/// Broad west-facing facets separated by unequal structural clefts.
fn tooth_side(l: u8, x: i32) -> (bool, f32) {
    let Some(e) = platform_of(l) else {
        return (false, 1.0);
    };
    let u = (x as f32 - e.cx) / e.rx;
    let bounds = [-1.0, -0.77, -0.42, -0.19, 0.18, 0.48, 0.80, 1.0];
    for pair in bounds.windows(2) {
        if u >= pair[0] && u <= pair[1] {
            let t = (u - pair[0]) / (pair[1] - pair[0]);
            return (t < 0.38, t);
        }
    }
    (false, 1.0)
}

fn faces(c: &mut Canvas) {
    let (pw, ph) = (c.z.pw, c.z.ph);
    for x in 0..pw {
        let mut y = 0;
        while y < ph - 1 {
            let l = c.lab(x, y);
            if l == 0 || c.lab(x, y + 1) != 0 || face_height(l) == 0 {
                y += 1;
                continue;
            }
            let lip = y;
            let hf = face_height(l);
            let ud = under_depth(l, x);
            let mut yy = y + 1;
            while yy < ph && yy <= lip + hf + ud && c.lab(x, yy) == 0 {
                let v = yy - lip - 1;
                let col = if v < hf {
                    face_px(l, x, lip, v, hf)
                } else {
                    under_px(l, x, v - hf, ud)
                };
                c.set(x, yy, col);
                let nx = platform_of(l).map_or(0.0, |e| ((x as f32 + 0.5 - e.cx) / e.rx).clamp(-0.98, 0.98));
                let front = [nx, (1.0 - nx * nx).sqrt()];
                let height = (hf + ud - v).max(0) as f32;
                c.z.gset_plane_surface(x, yy, if v == 0 { [0.0, 0.0, 1.0] } else { [front[0], front[1], 0.0] }, height);
                if v >= 2 && v < hf {
                    if let Some(s) = sheet("summit_masonry") && let Some(a) = s.anim("wall") {
                        let cell = s.cell(a, 0);
                        let sx = (rim_u(l, x, lip).floor() as i32).rem_euclid(cell.w);
                        let sy = (v + if l == L_ARENA { 0 } else { 32 }).rem_euclid(cell.h);
                        if cell.at(sx, sy).a {
                            c.z.gset_surface(x, yy, cell, sx, sy, true, height);
                            c.z.gorient_surface(x, yy, front);
                        }
                    }
                } else if v >= hf && l != L_BRIDGE {
                    if let Some(s) = sheet("summit_foundation") && let Some(a) = s.anim("rock") {
                        let cell = s.cell(a, 0);
                        let sx = (x + l as i32 * 43).rem_euclid(cell.w);
                        let sy = (v - hf).min(cell.h - 1);
                        if cell.at(sx, sy).a {
                            c.z.gset_surface(x, yy, cell, sx, sy, true, height);
                            c.z.gorient_surface(x, yy, front);
                        }
                    }
                }
                yy += 1;
            }
            y = yy;
        }
    }
}

/// Horizontal coordinate along a platform rim (so masonry blocks keep their size as it curves).
fn rim_u(l: u8, x: i32, lip: i32) -> f32 {
    match platform_of(l) {
        Some(e) => {
            let a = ((lip as f32 + 0.5 - e.cy) / e.ry).atan2((x as f32 + 0.5 - e.cx) / e.rx);
            a * (e.rx + e.ry) * 0.5
        }
        None => x as f32,
    }
}

/// Masonry rim wall. v = row below the lip (0 = cornice).
fn face_px(l: u8, x: i32, lip: i32, v: i32, hf: i32) -> Rgb {
    // How much this part of the wall faces the light (upper left): west-facing is lit.
    let nx = match platform_of(l) {
        Some(e) => ((x as f32 + 0.5 - e.cx) / e.rx).clamp(-1.0, 1.0),
        None => 0.0,
    };
    if v == 0 {
        if l == L_LANDING || l == L_TERRACE {
            return if nx < -0.55 { F3 } else { F2 };
        }
        return if nx < 0.5 { F4 } else { F3 };
    }
    if v == 1 {
        return F0;
    }
    // The generated stone is a native material, not a stretched picture. Its courses
    // follow the curved wall in rim-space while the original cornice and geometry stay.
    if let Some(s) = sheet("summit_masonry")
        && let Some(a) = s.anim("wall")
    {
        let cell = s.cell(a, 0);
        let u = rim_u(l, x, lip).floor() as i32;
        let p = cell.at(
            u.rem_euclid(cell.w),
            (v + if l == L_ARENA { 0 } else { 32 }).rem_euclid(cell.h),
        );
        if p.a {
            let facing = if nx < -0.45 {
                1.0
            } else if nx < 0.2 {
                0.88
            } else if nx < 0.7 {
                0.73
            } else {
                0.62
            };
            let depth = if v > hf - 7 { 0.72 } else { 1.0 };
            return p.c * (facing * depth);
        }
    }
    let course_h = if l == L_ARENA { 24 } else { 15 };
    let vv = v - 2;
    let course = vv / course_h;
    let r = vv % course_h;
    let u = rim_u(l, x, lip);
    // block boundaries along u, staggered per course
    let blen = if l == L_ARENA { 72.0 } else { 48.0 };
    let off = hash1(course, 7 + l as u32) * blen + if course % 2 == 1 { blen * 0.5 } else { 0.0 };
    let uu = u + off;
    let bi = (uu / blen).floor() as i32;
    let local = uu - bi as f32 * blen;
    // vary block length a little by merging/splitting via hash of index
    let mortar_v = r == course_h - 1;
    let mortar_u = local < 1.0;
    if mortar_v || mortar_u {
        return F0;
    }
    // tone per block from facing + a little per-block variety
    let h = hash2(bi, course, 13 + l as u32);
    let mut lvl = if nx < -0.45 {
        3
    } else if nx < 0.2 {
        2
    } else if nx < 0.7 {
        1
    } else {
        0
    };
    if h < 0.18 {
        lvl -= 1;
    } else if h > 0.9 {
        lvl += 1;
    }
    // lower courses sit in the shadow of the floor above them... no: of the overhang below
    let depth = vv as f32 / (hf - 2).max(1) as f32;
    if depth > 0.72 {
        lvl -= 1;
    }
    let ramp = [F0, F1, F2, F2B, F3];
    let lvl = lvl.clamp(0, 3) as usize;
    let mut col = ramp[lvl];
    // lit top edge and left edge of each block
    if r == 0 || (local < 2.0 && nx < 0.3) {
        col = ramp[(lvl + 1).min(4)];
    }
    col
}

/// Heavy rock planes below the masonry, with no glowing contour or fine striations.
fn under_px(l: u8, x: i32, v: i32, ud: i32) -> Rgb {
    if l == L_BRIDGE {
        return bridge_under_px(x, v, ud);
    }
    let (lit, d) = tooth_side(l, x);
    let k = v as f32 / ud.max(1) as f32;
    if let Some(s) = sheet("summit_foundation")
        && let Some(a) = s.anim("rock")
    {
        let cell = s.cell(a, 0);
        let p = cell.at((x + l as i32 * 43).rem_euclid(cell.w), v.min(cell.h - 1));
        if p.a {
            let plane = if lit { 0.88 } else { 0.65 };
            let depth = if k > 0.88 {
                0.56
            } else if k > 0.68 {
                0.76
            } else {
                1.0
            };
            return p.c * (plane * depth);
        }
    }
    if v < 3 || d > 0.94 || k > 0.88 {
        return F0;
    }
    if lit && k < 0.64 {
        F2
    } else if k < 0.72 {
        S1
    } else {
        F1
    }
}

fn bridge_under_px(x: i32, v: i32, ud: i32) -> Rgb {
    let (left, right) = bridge_springlines();
    let u = (x as f32 + 0.5 - (left + right) * 0.5) / ((right - left) * 0.5);
    // Wedge joints follow the arch, rather than the upright courses of nonexistent piers.
    let angle = ((v as f32 - 18.0) / 11.0).atan2(u);
    let wedge = ((angle + PI) * 17.0 / PI).rem_euclid(1.0);
    if v >= ud - 7 {
        if wedge < 0.04 || v == ud - 1 {
            return F0;
        }
        return if v == ud - 7 {
            F2B
        } else if v >= ud - 3 {
            F1
        } else {
            F2
        };
    }
    // The spandrel bonds into the same dark masonry as the adjacent island walls.
    if v % 11 == 10 || (x + v / 11 * 19).rem_euclid(35) == 0 {
        F0
    } else {
        F1
    }
}

fn bridge_parapet(c: &mut Canvas) {
    // A low wall along the deck's north edge rises 8 px above it.
    let (x0, y0, x1, _) = BRIDGE;
    for x in x0..x1 {
        if c.lab(x, y0) != L_BRIDGE {
            continue;
        }
        // broken off in one place
        if (448..470).contains(&x) {
            continue;
        }
        for dy in 1..=8 {
            let y = y0 - dy;
            if c.lab(x, y) != 0 {
                continue;
            }
            let col = if dy >= 7 {
                if dy == 8 { F3 } else { F4 }
            } else if dy == 1 {
                F0
            } else if (x - x0).rem_euclid(48) == 0 {
                F0
            } else if (x - x0).rem_euclid(48) == 1 {
                F2B
            } else {
                F2
            };
            c.set(x, y, col);
        }
    }
    // the broken stub: a low jagged edge
    for x in 448..470 {
        let hgt = 2 + ((x - 448) as f32 * 0.7).sin().abs() as i32 * 2 + (hash1(x, 5) * 2.0) as i32;
        for dy in 1..=hgt {
            let y = y0 - dy;
            c.set(
                x,
                y,
                if dy == hgt {
                    F3
                } else if dy == 1 {
                    F0
                } else {
                    F1
                },
            );
        }
    }
}

// ------------------------------------------------------------------------------------------
// Stone courses

/// Ring-course stones on an ellipse. Returns (joint, stone id, course index) or None outside.
/// `bounds` are course boundaries in "ry pixels" (r = rho * ry); the last course runs to ry.
fn ring_stone(
    e: &Ellipse,
    bounds: &[f32],
    stone_len: f32,
    seed: u32,
    x: f32,
    y: f32,
) -> Option<(bool, u32, usize)> {
    let u = (x - e.cx) / e.rx;
    let v = (y - e.cy) / e.ry;
    let rho2 = u * u + v * v;
    let rho = rho2.sqrt();
    if rho > 1.0 {
        return None;
    }
    let r = rho * e.ry;
    // gradient magnitude of r (px of r per screen px)
    let gx = (x - e.cx) / (e.rx * e.rx);
    let gy = (y - e.cy) / (e.ry * e.ry);
    let gr = if rho > 1e-4 {
        e.ry * (gx * gx + gy * gy).sqrt() / rho
    } else {
        1.0
    };
    let mut course = bounds.len();
    for (i, b) in bounds.iter().enumerate() {
        if r < *b {
            course = i;
            break;
        }
    }
    let mut joint = false;
    for b in bounds {
        // joint on the inner side of each boundary, ~1 px thick
        let d = (b - r) / gr.max(1e-3);
        if (0.0..1.0).contains(&d) {
            joint = true;
        }
    }
    // radial joints
    let (lo, hi) = (
        if course == 0 { 0.0 } else { bounds[course - 1] },
        if course < bounds.len() {
            bounds[course]
        } else {
            e.ry
        },
    );
    let mid = (lo + hi) * 0.5;
    let circ = TAU * (mid / e.ry) * ((e.rx + e.ry) * 0.5);
    let n = (circ / stone_len)
        .round()
        .max(if course == 0 { 1.0 } else { 3.0 });
    let th = v.atan2(u); // -PI..PI
    let off = hash1(course as i32, seed) + if course % 2 == 1 { 0.5 } else { 0.0 };
    let g = (th + PI) / TAU * n + off;
    let gi = g.floor();
    let frac = g - gi;
    // |grad g|
    let denom = (u * u + v * v).max(1e-6);
    let gth = ((v / e.rx).powi(2) + (u / e.ry).powi(2)).sqrt() / denom;
    let gg = n / TAU * gth;
    if course > 0 && frac / gg.max(1e-4) < 1.0 {
        joint = true;
    }
    let seg = (gi as i32).rem_euclid(n as i32);
    let sid = (course as u32) * 4096 + seg as u32 + seed * 131;
    Some((joint, sid, course))
}

struct Tones {
    joint: Rgb,
    base: [Rgb; 3],
    hi: [Rgb; 3],
}

// Combat ground stays quiet; the brighter ramp is confined to the worn perimeter.
const FLOOR_TONES: Tones = Tones {
    joint: S1,
    base: [F2, F2, F2],
    hi: [F2, F2, F2],
};
const CURB_TONES: Tones = Tones {
    joint: F2,
    base: [F3, F3, F2],
    hi: [F4, F3, F3],
};

fn tone_pick(sid: u32, _t: &Tones) -> usize {
    let h = hash1(sid as i32, 999);
    if h < 0.62 {
        0
    } else if h < 0.9 {
        1
    } else {
        2
    }
}

/// Displaced slabs span the arena; a single worn rim retains its physical boundary.
fn arena_top(c: &mut Canvas) {
    let e = ARENA;
    for y in (e.cy - e.ry) as i32..=(e.cy + e.ry) as i32 {
        for x in (e.cx - e.rx) as i32..=(e.cx + e.rx) as i32 {
            if c.lab(x, y) != L_ARENA {
                continue;
            }
            let r = e.rho(x as f32 + 0.5, y as f32 + 0.5);
            // The floor was laid around the old prism instrument, so its broad
            // wedge stones share the platform's perspective instead of forming an
            // unrelated rectangular grid. Different stone bodies remain close in
            // value; the Warden's pale telegraphs still own the fighting floor.
            let stone = ring_stone(&e, ARENA_FLOOR_BOUNDS,
                74.0, 17, x as f32 + 0.5, y as f32 + 0.5);
            let col = if r > 0.958 {
                let Some((joint, sid, _)) =
                    ring_stone(&e, &[214.0], 76.0, 3, x as f32 + 0.5, y as f32 + 0.5)
                else {
                    continue;
                };
                if joint {
                    F2
                } else if y < 286 {
                    // Recess the far course behind the head; only the west end catches cold fill.
                    if r > 0.992 && (506..538).contains(&x) {
                        F3
                    } else {
                        F2
                    }
                } else if r > 0.992 && y < e.cy as i32 && (x < 454 || (988..1012).contains(&x)) {
                    F3
                } else {
                    CURB_TONES.base[tone_pick(sid, &CURB_TONES)]
                }
            } else {
                let Some((joint, sid, course)) = stone else { continue };
                let gain = match tone_pick(sid, &FLOOR_TONES) {
                    0 => 1.0,
                    1 => 0.965,
                    _ => 1.035,
                };
                // Worn joints disappear for short connected lengths, never
                // break into freestanding grit or high-contrast radial spokes.
                let worn = ((x + y / 2 + course as i32 * 13).div_euclid(9)
                    + sid as i32).rem_euclid(7) == 0;
                if joint && !worn {
                    F2.lerp(F1, if course == 0 { 0.15 } else { 0.27 })
                } else {
                    F2.scale(gain)
                }
            };
            c.set(x, y, col);
        }
    }
}

/// The ruined instrument's stone and tarnished-metal scale. It is static,
/// non-emissive masonry: two interrupted arcs and a few rooted index marks,
/// with no free particles or complete target-like circle beneath the player.
fn observatory_inlay(c: &mut Canvas) {
    let e = ARENA;
    let groove = F2.lerp(F1, 0.34);
    let metal = F2.lerp(S3, 0.23).lerp(G1, 0.10);
    let worn_metal = F2.lerp(S2, 0.45);
    for y in 278..610 {
        for x in 452..989 {
            if c.lab(x, y) != L_ARENA { continue }
            let u = (x as f32 + 0.5 - e.cx) / 1.5;
            let v = y as f32 + 0.5 - e.cy;
            let r = (u * u + v * v).sqrt();
            let angle = v.atan2(u);
            let sector = ((angle + PI) / (TAU / 12.0)).floor() as i32;
            let local = (angle + PI).rem_euclid(TAU / 12.0);
            // Three large missing sections expose the ordinary wedge masonry.
            let retained = !matches!(sector, 1 | 5 | 8)
                && !(sector == 10 && local > 0.24);
            if !retained { continue }
            let ring = if (137.0..140.0).contains(&r) {
                Some((137.0, 140.0))
            } else if (158.0..160.0).contains(&r) && sector.rem_euclid(3) != 0 {
                Some((158.0, 160.0))
            } else { None };
            if let Some((inner, outer)) = ring {
                c.set(x, y, if r < inner + 0.8 { groove }
                    else if r > outer - 0.8 { metal } else { worn_metal });
            }
            // Indexes belong to the surviving arc and share its weathered metal.
            let ray_distance = r * local.min(TAU / 12.0 - local);
            if (140.0..147.0).contains(&r) && ray_distance < 0.8 {
                c.set(x, y, worn_metal);
            }
        }
    }
}

/// Stamp the artist's cold, cracked seal at native resolution into the fighting floor.
fn seal(c: &mut Canvas) {
    let Some(s) = sheet("summit_tiles") else {
        return;
    };
    let Some(block) = s.json.get("blocks").and_then(|b| b.get("seal_dim")) else {
        return;
    };
    let (Some(at), Some(size)) = (block.get("at"), block.get("size")) else {
        return;
    };
    let (Some(tx), Some(ty), Some(tw), Some(th)) = (
        at[0].as_i64(),
        at[1].as_i64(),
        size[0].as_i64(),
        size[1].as_i64(),
    ) else {
        return;
    };
    let (w, h) = (tw as i32 * s.cell_w, th as i32 * s.cell_h);
    let (x0, y0) = (ARENA.cx as i32 - w / 2, ARENA.cy as i32 - h / 2);
    for by in 0..th as usize {
        for bx in 0..tw as usize {
            let Some(cell) = s
                .cells
                .get(ty as usize + by)
                .and_then(|row| row.get(tx as usize + bx))
            else {
                continue;
            };
            for y in 0..cell.h {
                for x in 0..cell.w {
                    let px = cell.at(x, y);
                    let (wx, wy) = (x0 + bx as i32 * s.cell_w + x, y0 + by as i32 * s.cell_h + y);
                    if px.a && c.lab(wx, wy) == L_ARENA {
                        c.set(wx, wy, px.c);
                        c.z.gset_surface(wx, wy, cell, x, y, false, 0.0);
                    }
                }
            }
        }
    }
}

/// The broad center slabs share their displaced joints with their normal geometry.
fn small_center_stone(e: &Ellipse, l: u8, x: i32, y: i32) -> (bool, u32) {
    let cy = e.cy as i32;
    let row = if y < cy - 18 { 0 } else if y < cy + 20 { 1 } else { 2 };
    let seams = if l == L_LANDING { [222, 264, 194] } else { [701, 746, 683] };
    let sx = seams[row] + if row == 1 && y >= cy + 3 { 2 } else { 0 };
    let joint = y == cy - 18 || y == cy + 20 || x == sx
        || (row == 1 && y == cy + 3 && (sx - 2..=sx).contains(&x));
    (joint, l as u32 * 4096 + row as u32 * 2 + u32::from(x > sx))
}

/// Terrace and landing: a few rings and a curb.
fn small_top(c: &mut Canvas, e: &Ellipse, l: u8, bounds: &[f32], _curb_w: f32, seed: u32) {
    let x0 = (e.cx - e.rx) as i32 - 1;
    let x1 = (e.cx + e.rx) as i32 + 1;
    let y0 = (e.cy - e.ry) as i32 - 1;
    let y1 = (e.cy + e.ry) as i32 + 1;
    let curb = bounds.len();
    let get = |x: i32, y: i32| -> Option<(bool, u32, usize)> {
        let ll = if x < 0 || y < 0 { 0 } else { c.lab(x, y) };
        if ll != l {
            return None;
        }
        let course_guess = ring_stone(e, bounds, 30.0, seed, x as f32 + 0.5, y as f32 + 0.5)?;
        if course_guess.2 == curb {
            return ring_stone(e, bounds, 22.0, seed, x as f32 + 0.5, y as f32 + 0.5);
        }
        Some(course_guess)
    };
    let mut centers: HashMap<u32, (f32, f32, f32)> = HashMap::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if let Some((_, sid, course)) = get(x, y)
                && course > 0
            {
                let c = centers.entry(sid).or_insert((0.0, 0.0, 0.0));
                c.0 += x as f32 + 0.5;
                c.1 += y as f32 + 0.5;
                c.2 += 1.0;
            }
        }
    }
    let mut buf = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let Some((joint, sid, course)) = get(x, y) else {
                continue;
            };
            let t = if course == curb {
                &CURB_TONES
            } else {
                &FLOOR_TONES
            };
            let col = if course == 0 {
                // Broad displaced courses carry the approach; interiors stay unmarked.
                if small_center_stone(e, l, x, y).0 {
                    S2
                } else {
                    F2
                }
            } else if joint {
                t.joint
            } else {
                let k = tone_pick(sid, t);
                let edge = |o: Option<(bool, u32, usize)>| {
                    o.map(|(j, s, _)| j || s != sid).unwrap_or(true)
                };
                // Cold fill catches a few upper-left stone lips, not a luminous circular rail.
                let u = (x as f32 - e.cx) / e.rx;
                let v = (y as f32 - e.cy) / e.ry;
                let &(sx, sy, count) = centers.get(&sid).unwrap();
                let cu = (sx / count - e.cx) / e.rx;
                let cv = (sy / count - e.cy) / e.ry;
                let catch = (cu < -0.3 || cv < -0.45) && hash1(sid as i32, 1901) < 0.45;
                if catch && u < 0.15 && v < -0.45 && (edge(get(x - 1, y)) || edge(get(x, y - 1))) {
                    F4
                } else if (cu < -0.5 && cv < 0.35) || catch {
                    t.base[k]
                } else {
                    if k == 2 { F2 } else { S2 }
                }
            };
            buf.push((x, y, col));
        }
    }
    for (x, y, col) in buf {
        c.set(x, y, col);
    }
}

/// The deck painter and its receiver normals share the same staggered joints.
fn bridge_stone(x: i32, y: i32) -> Option<(bool, i32)> {
    if !in_rect(BRIDGE, x, y) {
        return None;
    }
    let (x0, y0, _, _) = BRIDGE;
    let row_h = 16;
    let row = (y - y0) / row_h;
    let r = (y - y0) % row_h;
    let len = if row == 2 { 56 } else { 44 };
    let off = (hash1(row, 41) * len as f32) as i32 + row * 11;
    let xx = x - x0 + off;
    let bi = xx.div_euclid(len);
    let lx = xx.rem_euclid(len);
    let joint = r == row_h - 1 || lx == 0;
    Some((joint, row * 1000 + bi))
}

fn bridge_deck(c: &mut Canvas) {
    let (x0, y0, x1, y1) = BRIDGE;
    let row_h = 16;
    let mut buf = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if c.lab(x, y) != L_BRIDGE {
                continue;
            }
            let Some((joint, sid)) = bridge_stone(x, y) else {
                continue;
            };
            let curb = (y - y0) / row_h == 2;
            let t = if curb { &CURB_TONES } else { &FLOOR_TONES };
            let col = if joint {
                t.joint
            } else {
                let k = tone_pick(sid as u32 + 77, t);
                let edge = |o: Option<(bool, i32)>| o.map(|(j, s)| j || s != sid).unwrap_or(true);
                if edge(bridge_stone(x - 1, y)) || edge(bridge_stone(x, y - 1)) {
                    t.hi[k]
                } else {
                    t.base[k]
                }
            };
            buf.push((x, y, col));
        }
    }
    for (x, y, col) in buf {
        c.set(x, y, col);
    }
}

fn stair(c: &mut Canvas) {
    let (x0, _, x1, y1) = STAIR;
    // the top step starts at the arena lip line in the stair's middle
    let top = STAIR_TOP;
    for x in x0 - 8..x1 + 8 {
        for y in top - 10..y1 {
            let l = c.lab(x, y);
            let in_stair_x = x >= x0 && x < x1;
            if in_stair_x {
                if l == L_ARENA {
                    // break the curb for the stair opening: plain landing stones
                    let r = ARENA.rho(x as f32 + 0.5, y as f32 + 0.5) * ARENA.ry;
                    if r > 207.0 {
                        let col = if (x - x0) % 16 == 0 || y == top - 1 {
                            F1
                        } else if y == top - 10 + 1 {
                            F3
                        } else {
                            F3
                        };
                        c.set(x, y, col);
                    }
                    continue;
                }
                if l != L_STAIR {
                    continue;
                }
                let yr = y - top;
                if yr < 0 {
                    continue;
                }
                let r = yr % 8;
                // wear: the middle of each tread is a touch lighter
                let mid = (x - (x0 + x1) / 2).abs() < 14;
                let col = match r {
                    0 => F3B,
                    1..=3 => {
                        if mid {
                            F3B
                        } else {
                            F3
                        }
                    }
                    4 => F4,
                    5 => F0,
                    6 => F1,
                    _ => F2,
                };
                // chipped nosings here and there
                let chipped = [(1, 711, 719), (4, 726, 736), (6, 707, 713)]
                    .iter()
                    .any(|&(step, left, right)| yr / 8 == step && (left..right).contains(&x));
                let col = if r == 4 && chipped { F2 } else { col };
                c.set(x, y, col);
            } else {
                // cheek walls: 8 px each side, running from the arena lip down to the terrace
                let left = x < x0;
                let wx = if left { x - (x0 - 8) } else { x - x1 };
                if !(0..8).contains(&wx) {
                    continue;
                }
                if l == L_ARENA && ARENA.rho(x as f32 + 0.5, y as f32 + 0.5) * ARENA.ry < 214.0 {
                    continue;
                }
                if l == L_TERRACE && TERRACE.rho(x as f32 + 0.5, y as f32 + 0.5) < 0.84 {
                    continue;
                }
                let yr = y - top + 10;
                let col = if left {
                    match wx {
                        0 => F0,
                        1..=5 => {
                            if yr % 16 == 0 {
                                F1
                            } else if wx == 1 {
                                F4
                            } else {
                                F3
                            }
                        }
                        6 => F2,
                        _ => F0,
                    }
                } else {
                    match wx {
                        0 => F2B,
                        1 => F1,
                        2..=6 => {
                            if yr % 16 == 0 {
                                F1
                            } else if wx == 2 {
                                F4
                            } else {
                                F3
                            }
                        }
                        _ => F0,
                    }
                };
                c.set(x, y, col);
            }
        }
    }
}

/// Two-tier round plinth under the Prism. Tops are shifted up by their height (3/4 view).
fn dais(c: &mut Canvas) {
    if let Some(s) = sheet("summit_dais")
        && let Some(a) = s.anim("dais")
    {
        let cell = s.cell(a, 0);
        let (ax, ay) = s.anchor_of(a);
        for y in 0..cell.h {
            for x in 0..cell.w {
                let (wx, wy) = (DAIS.cx as i32 - ax + x, DAIS.cy as i32 + 14 - ay + y);
                let p = cell.at(x, y);
                if p.a && c.lab(wx, wy) == L_ARENA {
                    c.set(wx, wy, p.c);
                    c.z.gset_surface(wx, wy, cell, x, y, true, (cell.h - y - 1).min(12).max(0) as f32);
                }
            }
        }
        return;
    }
    let tiers = [
        (DAIS, 6.0f32, F2),
        (
            Ellipse {
                cx: DAIS.cx,
                cy: DAIS.cy - 6.0,
                rx: DAIS.rx * 0.68,
                ry: DAIS.ry * 0.68,
            },
            6.0,
            F2,
        ),
    ];
    for (e, hgt, top) in tiers {
        let te = Ellipse {
            cx: e.cx,
            cy: e.cy - hgt,
            ..e
        };
        let x0 = (e.cx - e.rx) as i32 - 1;
        let x1 = (e.cx + e.rx) as i32 + 1;
        let y0 = (te.cy - e.ry) as i32 - 1;
        let y1 = (e.cy + e.ry) as i32 + 1;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let in_top = te.contains(fx, fy);
                let in_base = e.contains(fx, fy);
                // face: between the top's south outline and the base's south outline
                let u = (fx - e.cx) / e.rx;
                if u.abs() > 1.0 {
                    continue;
                }
                let half = e.ry * (1.0 - u * u).sqrt();
                let ts = te.cy + half; // top outline (south)
                let bs = e.cy + half; // base outline (south)
                if in_top {
                    let rho = te.rho(fx, fy);
                    let col = if rho > 0.93 && fy < te.cy && u < -0.55 && !(232..=239).contains(&y)
                    {
                        F3
                    } else {
                        top
                    };
                    c.set(x, y, col);
                } else if fy > ts && fy <= bs + 0.5 && (in_base || fy <= bs) {
                    // Retain the central step, fading both face ends through three flat tones.
                    let base = F1.lerp(top, 0.5);
                    let fade = (((u.abs() - 0.32) / 0.40).clamp(0.0, 1.0) * 3.0).round() / 3.0;
                    c.set(x, y, base.lerp(top, fade));
                }
            }
        }
    }
    // A shallow socket keeps its rim without becoming a dark cap behind the Keeper.
    let e = Ellipse {
        cx: DAIS.cx,
        cy: DAIS.cy - 12.0,
        rx: DAIS.rx * 0.40,
        ry: DAIS.ry * 0.40,
    };
    for y in (e.cy - e.ry) as i32 - 2..=(e.cy + e.ry) as i32 + 2 {
        for x in (e.cx - e.rx) as i32 - 2..=(e.cx + e.rx) as i32 + 2 {
            let rho = e.rho(x as f32 + 0.5, y as f32 + 0.5);
            if (rho - 1.0).abs() < 0.07 {
                let far_cut =
                    (y as f32) < e.cy && (x as f32) < e.cx && x.div_euclid(5).rem_euclid(3) != 0;
                c.set(x, y, if far_cut { F1 } else { F2 });
            } else if rho < 0.93 {
                c.set(x, y, F2);
            }
        }
    }
}

/// Three structural cracks collect at the loaded perimeter, leaving the fighting floor calm.
fn cracks(c: &mut Canvas) {
    let starts: [(f32, f32, f32, i32, u32); 3] = [
        (470.0, 520.0, 2.6, 34, 5),
        (968.0, 330.0, 0.6, 29, 6),
        (560.0, 300.0, -2.3, 23, 7),
    ];
    for (sx, sy, a0, len, seed) in starts {
        crack(c, sx, sy, a0, len, seed, true);
    }
}

fn crack(c: &mut Canvas, sx: f32, sy: f32, a0: f32, len: i32, seed: u32, branch: bool) {
    let (mut x, mut y) = (sx, sy);
    let mut a = a0;
    for i in 0..len {
        // wander in deliberate bends: change direction every few px, not every px
        if i % 5 == 0 {
            a += (hash1(i, seed) - 0.5) * 0.9;
        }
        let (dx, dy) = (a.cos(), a.sin() * 0.7);
        x += dx;
        y += dy;
        let (xi, yi) = (x.round() as i32, y.round() as i32);
        if c.lab(xi, yi) != L_ARENA {
            break;
        }
        c.set(xi, yi, F0);
        // the far lip of the crack catches the light
        if hash1(i, seed + 9) < 0.5
            && c.lab(xi + 1, yi + 1) == L_ARENA
            && c.get(xi + 1, yi + 1) != F0
        {
            let b = c.get(xi + 1, yi + 1);
            if b == F2 || b == F2B {
                c.set(xi + 1, yi + 1, F1);
            }
        }
        if branch && i > 12 && i % 23 == 0 {
            crack(
                c,
                x,
                y,
                a + if hash1(i, seed + 3) < 0.5 { 0.8 } else { -0.8 },
                len / 3,
                seed + 100 + i as u32,
                false,
            );
        }
    }
}

/// Fill an authored stone fragment on walkable tops without changing the hole mask.
fn floor_fragment(c: &mut Canvas, points: &[(i32, i32)], col: Rgb) {
    let x0 = points.iter().map(|p| p.0).min().unwrap();
    let x1 = points.iter().map(|p| p.0).max().unwrap();
    let y0 = points.iter().map(|p| p.1).min().unwrap();
    let y1 = points.iter().map(|p| p.1).max().unwrap();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if c.lab(x, y) == 0 || c.lab(x, y) == L_STAIR {
                continue;
            }
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut inside = false;
            for i in 0..points.len() {
                let (ax, ay) = points[i];
                let (bx, by) = points[(i + 1) % points.len()];
                if (ay as f32 > py) != (by as f32 > py) {
                    let edge = ax as f32 + (py - ay as f32) * (bx - ax) as f32 / (by - ay) as f32;
                    if px < edge {
                        inside = !inside;
                    }
                }
            }
            if inside {
                c.set(x, y, col);
            }
        }
    }
}

/// Connected chips and short abraded lips belong to rims, displaced joints and thresholds.
fn floor_wear(c: &mut Canvas) {
    // Shallow curb cuts open onto a joint or the edge, rather than floating over a slab.
    for (cut, facet) in [
        (
            [(122, 778), (139, 778), (146, 781), (138, 783), (122, 781)],
            [(123, 781), (138, 782), (146, 781), (139, 784), (123, 782)],
        ),
        (
            [(216, 832), (221, 831), (224, 834), (222, 837), (217, 836)],
            [(217, 835), (221, 835), (224, 834), (223, 838), (218, 837)],
        ),
        (
            [(396, 510), (406, 510), (412, 514), (407, 516), (398, 514)],
            [(399, 514), (407, 515), (412, 514), (408, 517), (399, 516)],
        ),
        (
            [(623, 657), (629, 652), (635, 649), (639, 651), (634, 656)],
            [(627, 656), (634, 652), (639, 651), (635, 657), (630, 659)],
        ),
        (
            [(1003, 550), (997, 546), (995, 543), (999, 544), (1007, 549)],
            [
                (998, 548),
                (997, 545),
                (1000, 546),
                (1008, 550),
                (1005, 551),
            ],
        ),
        (
            [(943, 274), (950, 277), (949, 280), (943, 279), (940, 277)],
            [(942, 278), (946, 278), (950, 280), (948, 282), (944, 281)],
        ),
        (
            [(807, 829), (803, 825), (802, 822), (806, 823), (811, 828)],
            [(804, 826), (804, 824), (807, 825), (812, 829), (809, 831)],
        ),
    ] {
        floor_fragment(c, &cut, S1);
        floor_fragment(c, &facet, F2);
    }

    // Two loaded landing joints retain a short lit lip at their clipped corners.
    floor_fragment(
        c,
        &[(258, 758), (267, 755), (269, 758), (264, 762), (258, 761)],
        S1,
    );
    floor_fragment(
        c,
        &[(258, 761), (264, 761), (269, 758), (267, 762), (258, 763)],
        F3,
    );
    floor_fragment(
        c,
        &[(194, 796), (194, 792), (200, 796), (198, 799), (195, 799)],
        S1,
    );
    floor_fragment(
        c,
        &[(194, 798), (198, 798), (200, 796), (199, 800), (194, 799)],
        F2,
    );

    // Shallow abrasion stays on the footpath's actual bridge and stair thresholds.
    for (x, y, width) in [
        (326, 773, 13),
        (622, 776, 11),
        (714, 804, 14),
        (709, 651, 15),
    ] {
        floor_fragment(
            c,
            &[
                (x, y),
                (x + width, y),
                (x + width - 4, y + 2),
                (x + 2, y + 2),
            ],
            S2,
        );
        floor_fragment(
            c,
            &[
                (x + 2, y),
                (x + width - 3, y),
                (x + width - 5, y + 1),
                (x + 2, y + 1),
            ],
            F3,
        );
    }

    // Only two loaded arena joints lose corners; the central fighting floor stays broad.
    for (x, y) in [(503, 529), (876, 336)] {
        floor_fragment(c, &[(x, y), (x + 10, y), (x + 5, y + 4), (x, y + 3)], S1);
        floor_fragment(
            c,
            &[
                (x + 1, y + 3),
                (x + 5, y + 3),
                (x + 10, y),
                (x + 8, y + 3),
                (x + 2, y + 4),
            ],
            F2,
        );
    }
}

/// The fighting floor remains large calm slabs. A sparse set of native 32px
/// abrasions adds material to the approach and loaded rim, never a checkerboard.
/// Authored seams, curb highlights, steps, seal, dais and fractures are protected.
fn native_floor(c: &mut Canvas) {
    let Some(s) = sheet("summit_floor") else {
        return;
    };
    let Some(a) = s.anim("stone") else { return };
    if s.cell_w != 32 || s.cell_h != 32 || a.frames == 0 {
        return;
    }
    for ty in 0..(c.z.ph + 31) / 32 {
        for tx in 0..(c.z.pw + 31) / 32 {
            let (x0, y0) = (tx * 32, ty * 32);
            let l = c.lab(x0 + 16, y0 + 16);
            if !matches!(l, L_ARENA | L_LANDING | L_TERRACE | L_BRIDGE) {
                continue;
            }
            let rho = ARENA.rho((x0 + 16) as f32, (y0 + 16) as f32);
            let coverage = if l == L_ARENA {
                if rho < 0.64 { 0.16 } else { 0.44 }
            } else {
                0.7
            };
            if hash2(tx, ty, 940) > coverage {
                continue;
            }
            let variant = ((tx.rem_euclid(a.frames as i32) + ty * (a.frames as i32 / 2 + 1))
                .rem_euclid(a.frames as i32)) as usize;
            let cell = s.cell(a, variant);
            for y in 0..32 {
                for x in 0..32 {
                    let (wx, wy) = (x0 + x, y0 + y);
                    if c.lab(wx, wy) != l || c.get(wx, wy) != F2 {
                        continue;
                    }
                    // Keep the dais and its precise empty socket free of incidental wear.
                    if DAIS.rho(wx as f32, wy as f32) < 1.15 {
                        continue;
                    }
                    let p = cell.at(x, y);
                    if p.a {
                        c.set(wx, wy, p.c);
                        c.z.gset_surface(wx, wy, cell, x, y, false, 0.0);
                    }
                }
            }
        }
    }
}

/// Reuse the masonry's authored boundaries, including its deliberately worn joints.
/// A painted highlight or dark mineral is never interpreted as a height change.
fn floor_stone(l: u8, x: i32, y: i32) -> Option<(bool, u32)> {
    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
    match l {
        L_ARENA => {
            let r = ARENA.rho(fx, fy);
            if (STAIR.0..STAIR.2).contains(&x) && (STAIR_TOP - 10..STAIR_TOP).contains(&y)
                && r * ARENA.ry > 207.0
            {
                return Some(((x - STAIR.0) % 16 == 0 || y == STAIR_TOP - 1,
                    29000 + (x - STAIR.0) as u32 / 16));
            }
            if r > 0.958 {
                ring_stone(&ARENA, &[214.0], 76.0, 3, fx, fy)
                    .map(|(joint, sid, _)| (joint, sid))
            } else {
                ring_stone(&ARENA, ARENA_FLOOR_BOUNDS, 74.0, 17, fx, fy)
                    .map(|(joint, sid, course)| {
                        let worn = ((x + y / 2 + course as i32 * 13).div_euclid(9)
                            + sid as i32).rem_euclid(7) == 0;
                        (joint && !worn, sid)
                    })
            }
        }
        L_TERRACE | L_LANDING => {
            let (e, bound, seed) = if l == L_TERRACE { (TERRACE, 50.0, 11) }
                else { (LANDING, 56.0, 23) };
            let (_, _, course) = ring_stone(&e, &[bound], 30.0, seed, fx, fy)?;
            if course == 0 {
                Some(small_center_stone(&e, l, x, y))
            } else {
                ring_stone(&e, &[bound], 22.0, seed, fx, fy)
                    .map(|(joint, sid, _)| (joint, sid))
            }
        }
        L_BRIDGE => bridge_stone(x, y).map(|(joint, sid)| (joint, sid as u32)),
        _ => None,
    }
}

/// Finish only missing receivers after all native art and procedural overlays.
/// Slabs get shallow joint bevels and a quiet, consistent tilt per authored stone;
/// the grand stair's five tread rows face up and three riser rows face south.
fn walkable_surfaces(c: &mut Canvas) {
    let pw = c.z.pw;
    let ph = c.z.ph;
    let labels = &c.lab;
    let label_at = |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= pw || y >= ph { 0 }
        else { labels[(y * pw + x) as usize] }
    };
    c.z.fill_walkable_surfaces(|x, y| {
        let l = label_at(x, y);
        if l == L_STAIR && y >= STAIR_TOP {
            let row = (y - STAIR_TOP) % 8;
            return if row <= 4 { ([0.0, 0.0, 1.0], 0.0) }
                else { ([0.0, 1.0, 0.0], (7 - row) as f32) };
        }
        let Some((joint, sid)) = floor_stone(l, x, y) else {
            return ([0.0, 0.0, 1.0], 0.0);
        };
        let depth = |joint| if joint { -0.35 } else { 0.0 };
        let relief = |sx, sy| {
            if label_at(sx, sy) != l { return depth(joint); }
            floor_stone(l, sx, sy).map_or(depth(joint), |(j, _)| depth(j))
        };
        let dx = (relief(x + 1, y) - relief(x - 1, y)) * 0.5;
        let dy = (relief(x, y + 1) - relief(x, y - 1)) * 0.5;
        let tilt_x = (hash1(sid as i32, 3112) - 0.5) * 0.04;
        let tilt_y = (hash1(sid as i32, 3113) - 0.5) * 0.04;
        ([-dx + tilt_x, -dy + tilt_y, 1.0], 0.0)
    });
}

/// Stone remains a material across the whole platform, including native floor
/// pixels whose authored map is perfectly flat. Compose world-space grain with
/// the retained slab bevels instead of limiting relief to the sparse abrasions.
/// Tall masonry, the dais faces and stair risers keep their exact native planes.
fn material_surfaces(c: &mut Canvas) {
    use crate::gfx::lighting::TerrainSurface;
    use crate::gfx::terrain_detail::{self, Material};

    for y in 0..c.z.ph {
        for x in 0..c.z.pw {
            let i = (y * c.z.pw + x) as usize;
            let surface = c.z.ground_surfaces[i];
            if c.lab[i] == 0 || c.z.ground[i].r < 0.0 || !surface.present()
                || surface.normal[2] <= 82
            {
                continue;
            }
            let base = surface.normal.map(|v| v as f32 / 127.0);
            let normal = terrain_detail::normal(Material::Stone, x, y, base);
            c.z.ground_surfaces[i] = TerrainSurface::new(normal, surface.height as f32);
        }
    }
    c.z.invalidate_lighting();
}

#[cfg(test)]
mod walkable_normal_tests {
    use super::*;
    use crate::world::ZoneId;

    #[test]
    fn complete_walkable_normals_preserve_native_art_collision_and_architecture() {
        let mut z = Zone::new(ZoneId::Summit, super::super::TW, super::super::TH);
        let mut c = paint_geometry(&mut z);
        let (albedo, emission) = (c.z.ground.clone(), c.z.ground_emit.clone());
        let (tiles, physical) = (c.z.tiles.clone(), c.z.surface_tiles.clone());
        let retained = c.z.ground_surfaces.clone();
        walkable_surfaces(&mut c);

        assert_eq!(c.z.ground, albedo, "normal completion cannot repaint stone or holes");
        assert_eq!(c.z.ground_emit, emission);
        assert_eq!(c.z.tiles, tiles);
        assert_eq!(c.z.surface_tiles, physical, "native platform contact remains exact");
        let mut native = 0;
        let mut filled = [0usize; 6];
        for (i, before) in retained.iter().enumerate() {
            let after = c.z.ground_surfaces[i];
            if before.present() {
                native += 1;
                assert_eq!(after, *before, "native UV normal and altitude at pixel {i}");
            } else if c.z.ground[i].r >= 0.0 && !physical.as_ref().unwrap()[i].solid() {
                assert!(after.present(), "missing normal at native floor pixel {i}");
                filled[c.lab[i] as usize] += 1;
            } else {
                assert_eq!(after, *before, "outside the opaque walkable surface at pixel {i}");
            }
        }
        assert!(native > 10000, "exercise actual floor, seal, dais and architecture normal art");
        for (l, name) in [(L_ARENA, "arena"), (L_STAIR, "stairs"), (L_TERRACE, "terrace"),
            (L_BRIDGE, "bridge"), (L_LANDING, "landing")]
        {
            assert!(filled[l as usize] > 100, "complete the real {name} surface");
        }
        let finished = c.z.ground_surfaces.clone();
        walkable_surfaces(&mut c);
        assert_eq!(c.z.ground_surfaces, finished, "revisiting finished materials is idempotent");
    }

    #[test]
    fn grand_stair_treads_face_up_and_short_risers_face_south() {
        let mut z = Zone::new(ZoneId::Summit, super::super::TW, super::super::TH);
        paint(&mut z);
        let mut tread_rows = 0;
        let mut riser_rows = 0;
        for y in STAIR_TOP + 2..STAIR.3 {
            let s = z.ground_surfaces[(y * z.pw + 720) as usize];
            let row = (y - STAIR_TOP) % 8;
            if row <= 4 {
                tread_rows += 1;
                assert!(s.normal[2] > 112, "tread row {y} retains its upward macro plane");
                assert_eq!(s.height, 0, "treads retain their walkable receiver plane");
            } else {
                riser_rows += 1;
                assert_eq!(s.normal, [0, 127, 0], "riser row {y} faces the terrace");
                assert_eq!(s.height, (7 - row) as u8, "short visible riser at {y}");
            }
        }
        assert!(tread_rows > 30 && riser_rows > 20, "exercise the complete painted flight");
    }

    #[test]
    fn deck_joint_bevels_follow_the_visible_staggered_masonry() {
        let mut z = Zone::new(ZoneId::Summit, super::super::TW, super::super::TH);
        let mut c = paint_geometry(&mut z);
        let retained = c.z.ground_surfaces.clone();
        walkable_surfaces(&mut c);
        let mut joints = 0;
        for y in BRIDGE.1 + 2..BRIDGE.3 - 2 {
            for x in BRIDGE.0 + 2..BRIDGE.2 - 2 {
                let index = |x| (y * c.z.pw + x) as usize;
                if [x - 1, x, x + 1].iter().any(|&xx| c.lab(xx, y) != L_BRIDGE
                    || retained[index(xx)].present()) { continue; }
                if !bridge_stone(x, y).unwrap().0 || bridge_stone(x - 1, y).unwrap().0
                    || bridge_stone(x + 1, y).unwrap().0 { continue; }
                let left = c.z.ground_surfaces[index(x - 1)];
                let right = c.z.ground_surfaces[index(x + 1)];
                assert!(left.normal[0] > 12 && right.normal[0] < -12,
                    "both stone lips must descend into the authored joint at {x},{y}");
                assert!(left.normal[2] > 120 && right.normal[2] > 120,
                    "shallow floor joints cannot become vertical walls");
                joints += 1;
            }
        }
        assert!(joints > 5, "exercise actual deck seams outside retained native abrasions");
    }

    #[test]
    fn material_relief_preserves_opaque_shapes_heights_and_steep_authored_faces() {
        let mut z = Zone::new(ZoneId::Summit, super::super::TW, super::super::TH);
        let mut c = paint_geometry(&mut z);
        walkable_surfaces(&mut c);
        let (albedo, emission) = (c.z.ground.clone(), c.z.ground_emit.clone());
        let (tiles, physical) = (c.z.tiles.clone(), c.z.surface_tiles.clone());
        let before = c.z.ground_surfaces.clone();
        material_surfaces(&mut c);

        assert_eq!(c.z.ground, albedo);
        assert_eq!(c.z.ground_emit, emission);
        assert_eq!(c.z.tiles, tiles);
        assert_eq!(c.z.surface_tiles, physical);
        let (mut flat_native, mut enriched_native) = (0, 0);
        for (i, &original) in before.iter().enumerate() {
            let after = c.z.ground_surfaces[i];
            assert_eq!(after.height, original.height, "no material grain becomes a caster at {i}");
            assert_eq!(after.present(), original.present());
            if c.lab[i] == 0 || albedo[i].r < 0.0 || original.normal[2] <= 82 {
                assert_eq!(after, original, "keep authored architecture at {i}");
            } else if original.normal == [0, 0, 127] {
                flat_native += 1;
                enriched_native += usize::from(after.normal != original.normal);
            }
        }
        assert!(flat_native > 1000, "exercise flat native and tread material interiors");
        assert!(enriched_native * 100 > flat_native * 95,
            "flat source UVs also need material relief: {enriched_native}/{flat_native}");
    }

    #[test]
    fn stone_material_has_visible_light_response_on_every_platform() {
        let mut z = Zone::new(ZoneId::Summit, super::super::TW, super::super::TH);
        let mut c = paint_geometry(&mut z);
        walkable_surfaces(&mut c);
        material_surfaces(&mut c);

        // Matched grazing keys from opposite sides reveal a real material
        // response; merely attaching [0,0,127] passes coverage but fails this.
        let mut responses = [const { Vec::<f32>::new() }; 6];
        let mut tilted = [0usize; 6];
        for (i, &l) in c.lab.iter().enumerate() {
            if l == 0 { continue; }
            let s = c.z.ground_surfaces[i];
            if s.height > 0 || s.normal[2] <= 82 { continue; }
            let n = s.normal.map(|v| v as f32 / 127.0);
            let lateral = n[0].hypot(n[1]);
            tilted[l as usize] += usize::from(lateral > 0.05);
            let left = (n[0] * -0.8 + n[1] * -0.35 + n[2] * 0.49).max(0.0);
            let right = (n[0] * 0.8 + n[1] * 0.35 + n[2] * 0.49).max(0.0);
            responses[l as usize].push((left - right).abs());
        }
        for (l, name) in [(L_ARENA, "arena"), (L_STAIR, "stairs"), (L_TERRACE, "terrace"),
            (L_BRIDGE, "bridge"), (L_LANDING, "landing")]
        {
            let samples = &mut responses[l as usize];
            assert!(samples.len() > 1000, "sample actual {name} pixels");
            samples.sort_by(f32::total_cmp);
            let median = samples[samples.len() / 2];
            let p90 = samples[samples.len() * 9 / 10];
            assert!(tilted[l as usize] * 100 > samples.len() * 75,
                "{name} remains visibly flat over too much of its material");
            assert!(median > 0.06 && p90 > 0.16,
                "{name} needs visible lighting response, median={median}, p90={p90}");
        }
    }
}

#[cfg(test)]
mod bridge_support_tests {
    use super::*;

    #[test]
    fn one_shallow_arch_seats_in_the_actual_island_foundations() {
        let (left, right) = bridge_springlines();
        assert!((329.0..332.0).contains(&left));
        assert!((594.0..598.0).contains(&right));
        let center = ((left + right) * 0.5) as i32;
        let middle = bridge_under_depth(center);
        assert_eq!(middle, 7);
        // One uninterrupted opening: no periodic vertical piers into empty sky.
        let mut previous = bridge_under_depth(left.ceil() as i32);
        for x in left.ceil() as i32 + 1..=center {
            let depth = bridge_under_depth(x);
            assert!(
                depth <= previous,
                "the arch rises continuously toward its crown"
            );
            previous = depth;
        }
        for x in center + 1..right.floor() as i32 {
            let depth = bridge_under_depth(x);
            assert!(
                depth >= previous,
                "the arch descends continuously into the other foundation"
            );
            previous = depth;
        }
        for (x, platform, label) in [
            (left.floor() as i32, LANDING, L_LANDING),
            (right.ceil() as i32, TERRACE, L_TERRACE),
        ] {
            let shoulder = face_height(label) + under_depth(label, x);
            let spring = face_height(L_BRIDGE) + bridge_under_depth(x);
            let platform_lip = platform.cy
                + platform.ry
                    * (1.0 - ((x as f32 + 0.5 - platform.cx) / platform.rx).powi(2))
                        .max(0.0)
                        .sqrt();
            assert!(
                BRIDGE.3 as f32 + spring as f32 <= platform_lip + shoulder as f32 + 2.0,
                "the spring at {x} must be seated in real platform masonry/rock, not hang below it"
            );
        }
    }
}
