//! Paints the Cinder Steppes ground once at load, pixel by pixel, into a palette-index buffer
//! (ramp + level) that becomes `Zone::ground`. Shapes come from the tile map, rounded into
//! organic outlines with soft fields; raised things (basalt mesas, obsidian walls, the dais,
//! the island itself) get 3/4-view faces; lava areas are left for the animated lava pass.

use super::art::*;
use super::field::{Grid, edt, soften};
use super::map::MAP;
use crate::util::{fbm, hash2, noise2};
use crate::world::{HOLE, TILE, Tile, Zone};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mat {
    Void,
    Ash,
    Grass,
    Path,
    Paving,
    Rock,
    Wall,
    Lava,
    Canal,
    Bridge,
    Arch,
    Dais,
    Stairs,
}

/// Look up a logical material in the authored tile map.
pub fn mat(tx: i32, ty: i32) -> Mat {
    if ty < 0 || ty >= MAP.len() as i32 {
        return Mat::Void;
    }
    let row = MAP[ty as usize].as_bytes();
    if tx < 0 || tx >= row.len() as i32 {
        return Mat::Void;
    }
    match row[tx as usize] {
        b'.' => Mat::Ash,
        b',' => Mat::Grass,
        b'-' => Mat::Path,
        b':' => Mat::Paving,
        b'#' => Mat::Rock,
        b'O' => Mat::Wall,
        b'~' => Mat::Lava,
        b'c' => Mat::Canal,
        b'=' => Mat::Bridge,
        b'b' => Mat::Arch,
        b'^' => Mat::Dais,
        b's' => Mat::Stairs,
        _ => Mat::Void,
    }
}

/// Return the authored map dimensions in tiles.
pub fn map_size() -> (i32, i32) {
    (MAP[0].len() as i32, MAP.len() as i32)
}

// Pixel kinds, kept for the live systems (lava, grass, embers...).
pub const K_VOID: u8 = 0;
pub const K_ASH: u8 = 1;
pub const K_GRASS: u8 = 2;
pub const K_PATH: u8 = 3;
pub const K_PAVE: u8 = 4;
pub const K_ROCKTOP: u8 = 5;
pub const K_ROCKFACE: u8 = 6;
pub const K_WALLTOP: u8 = 7;
pub const K_WALLFACE: u8 = 8;
pub const K_LAVA: u8 = 9;
pub const K_DECK: u8 = 10;
pub const K_DAIS: u8 = 11;
pub const K_ISLEFACE: u8 = 12;
pub const K_BANK: u8 = 13;
pub const K_FALL: u8 = 14;

/// Identify pixel surfaces that accept ground details.
pub fn walkable_kind(k: u8) -> bool {
    matches!(k, K_ASH | K_GRASS | K_PATH | K_PAVE | K_DECK | K_DAIS)
}

/// A palette index: ramp id + level. `r == HOLE_R` means the backdrop shows through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct P {
    pub r: u8,
    pub l: i8,
}
pub const HOLE_R: u8 = 255;
/// Encode a material ramp and shade as one palette index.
const fn p(r: u8, l: i8) -> P {
    P { r, l }
}

/// Everything the painter learned that the live region needs.
pub struct Painted {
    pub kind: Grid<u8>,
    /// Lava including what runs under bridges and over the island edge (for flow coordinates).
    pub lava_full: Grid<bool>,
    /// Lava pixels actually drawn each frame.
    pub lava_draw: Grid<bool>,
    /// Lavafall pixels (subset of lava_full).
    pub fall: Grid<bool>,
}

struct Ctx {
    pw: i32,
    ph: i32,
    wob: Grid<f32>,
    wob2: Grid<f32>,
}

/// Expand a logical material selection to a native pixel mask.
fn tile_mask(pw: i32, ph: i32, f: impl Fn(Mat) -> bool) -> Grid<f32> {
    let (tw, th) = map_size();
    let mut tm = vec![false; (tw * th) as usize];
    for ty in 0..th {
        for tx in 0..tw {
            tm[(ty * tw + tx) as usize] = f(mat(tx, ty));
        }
    }
    let mut g = Grid::new(pw, ph, 0.0f32);
    for y in 0..ph {
        let ty = y / TILE;
        for x in 0..pw {
            let tx = x / TILE;
            if tx < tw && ty < th && tm[(ty * tw + tx) as usize] {
                g.v[(y * pw + x) as usize] = 1.0;
            }
        }
    }
    g
}

/// Cut an organic silhouette from a smoothed tile mask.
fn threshold(c: &Ctx, f: &Grid<f32>, base: f32, wob: f32, wob2: f32) -> Grid<bool> {
    let mut m = Grid::new(c.pw, c.ph, false);
    for i in 0..m.v.len() {
        m.v[i] = f.v[i] > base + c.wob.v[i] * wob + c.wob2.v[i] * wob2;
    }
    m
}

/// Per-column vertical runs of a mask: calls f(x, y_top, y_bottom) for each run.
fn runs(m: &Grid<bool>, mut f: impl FnMut(i32, i32, i32)) {
    for x in 0..m.w {
        let mut y = 0;
        while y < m.h {
            if m.v[(y * m.w + x) as usize] {
                let y0 = y;
                while y < m.h && m.v[(y * m.w + x) as usize] {
                    y += 1;
                }
                f(x, y0, y - 1);
            } else {
                y += 1;
            }
        }
    }
}

/// Column bands for basalt columns: returns (band index, x offset inside band, band width).
fn band(x: i32, seed: u32) -> (i32, i32, i32) {
    // Unequal broad basalt facets, rather than a comb of narrow ribs.
    let block = x.div_euclid(48);
    let split = 18 + (hash2(block, 0, seed) * 12.0) as i32;
    let offset = x.rem_euclid(48);
    if offset < split {
        (block * 2, offset, split)
    } else {
        (block * 2 + 1, offset - split, 48 - split)
    }
}

/// Paint the island, cliffs, masonry, ash, and banks at native resolution.
pub fn paint(z: &mut Zone) -> Painted {
    let (tw, th) = map_size();
    assert_eq!((tw, th), (z.tw, z.th));
    let (pw, ph) = (z.pw, z.ph);
    for ty in 0..th {
        for tx in 0..tw {
            let t = match mat(tx, ty) {
                Mat::Void => Tile::Void,
                Mat::Rock | Mat::Wall => Tile::Wall,
                Mat::Lava | Mat::Canal => Tile::Lava,
                _ => Tile::Floor,
            };
            z.set_tile(tx, ty, t);
        }
    }

    let mut wob = Grid::new(pw, ph, 0.0f32);
    let mut wob2 = Grid::new(pw, ph, 0.0f32);
    for y in 0..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            wob.v[i] = fbm(x as f32 / 21.0, y as f32 / 21.0, 2, 101) - 0.5;
            wob2.v[i] = noise2(x as f32 / 70.0, y as f32 / 70.0, 103) - 0.5;
        }
    }
    let c = Ctx { pw, ph, wob, wob2 };

    // ------------------------------------------------------------ shapes
    let island = threshold(
        &c,
        &soften(&tile_mask(pw, ph, |m| m != Mat::Void), 14),
        0.5,
        0.35,
        0.3,
    );
    let lava_t = tile_mask(pw, ph, |m| matches!(m, Mat::Lava | Mat::Bridge | Mat::Arch));
    let river = threshold(&c, &soften(&lava_t, 12), 0.56, 0.25, 0.2);
    let canal_t = {
        // canal tiles + bridges whose neighbors are canal: straight, lined, inset 4 px top/bottom
        let mut g = Grid::new(pw, ph, false);
        for ty in 0..th {
            for tx in 0..tw {
                let m = mat(tx, ty);
                let is = m == Mat::Canal
                    || (m == Mat::Bridge
                        && (mat(tx - 1, ty) == Mat::Canal || mat(tx + 1, ty) == Mat::Canal));
                if !is {
                    continue;
                }
                let top = mat(tx, ty - 1) != Mat::Canal && mat(tx, ty - 1) != Mat::Bridge;
                let bot = mat(tx, ty + 1) != Mat::Canal && mat(tx, ty + 1) != Mat::Bridge;
                for y in 0..TILE {
                    if (top && y < 4) || (bot && y >= TILE - 4) {
                        continue;
                    }
                    for x in 0..TILE {
                        g.set(tx * TILE + x, ty * TILE + y, true);
                    }
                }
            }
        }
        g
    };
    let rock = {
        let mut m = threshold(
            &c,
            &soften(&tile_mask(pw, ph, |m| m == Mat::Rock), 10),
            0.48,
            0.32,
            0.25,
        );
        for i in 0..m.v.len() {
            m.v[i] &= island.v[i];
        }
        m
    };
    let grass = threshold(
        &c,
        &soften(&tile_mask(pw, ph, |m| m == Mat::Grass), 16),
        0.5,
        0.45,
        0.5,
    );
    let path = threshold(
        &c,
        &soften(
            &tile_mask(pw, ph, |m| matches!(m, Mat::Path | Mat::Arch)),
            8,
        ),
        0.5,
        0.25,
        0.1,
    );
    let dais = threshold(
        &c,
        &soften(
            &tile_mask(pw, ph, |m| matches!(m, Mat::Dais | Mat::Stairs)),
            8,
        ),
        0.5,
        0.1,
        0.0,
    );
    let wall = {
        let t = tile_mask(pw, ph, |m| m == Mat::Wall);
        Grid {
            w: pw,
            h: ph,
            v: t.v.iter().map(|&v| v > 0.5).collect(),
        }
    };
    let deck = {
        let t = tile_mask(pw, ph, |m| m == Mat::Bridge);
        Grid {
            w: pw,
            h: ph,
            v: t.v.iter().map(|&v| v > 0.5).collect::<Vec<bool>>(),
        }
    };
    let arch = {
        let t = tile_mask(pw, ph, |m| m == Mat::Arch);
        Grid {
            w: pw,
            h: ph,
            v: t.v.iter().map(|&v| v > 0.5).collect::<Vec<bool>>(),
        }
    };

    let mut lava_full = Grid::new(pw, ph, false);
    for i in 0..lava_full.v.len() {
        lava_full.v[i] =
            island.v[i] && (river.v[i] || canal_t.v[i]) && !rock.v[i] && !wall.v[i] && !dais.v[i];
    }

    let mut px = Grid::new(pw, ph, p(HOLE_R, 0));
    let mut kind = Grid::new(pw, ph, K_VOID);

    // ------------------------------------------------------------ ground: ash with calm fields
    let lava_dist = edt(&lava_full);
    let rock_dist = {
        let mut r = rock.clone();
        for i in 0..r.v.len() {
            r.v[i] |= wall.v[i];
        }
        edt(&r)
    };
    for y in 0..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            if !island.v[i] {
                continue;
            }
            let (fx, fy) = (x as f32, y as f32);
            // big, calm tonal fields
            let n = fbm(fx / 120.0, fy / 95.0, 3, 11);
            let mut l = 2;
            if n > 0.62 {
                l = 3;
            } else if n < 0.30 {
                l = 1;
            }
            // scorched band along lava banks
            let ld = lava_dist.v[i];
            if ld < 9.0 + c.wob.v[i] * 10.0 {
                l = l.min(1);
            }
            // darker at the foot of rock (occlusion), irregular
            if rock_dist.v[i] < 3.0 + c.wob.v[i] * 4.0 {
                l = l.min(1);
            }
            px.v[i] = p(RA, l);
            kind.v[i] = K_ASH;
        }
    }

    // ------------------------------------------------------------ scorched grass soil
    for i in 0..px.v.len() {
        if island.v[i] && grass.v[i] && kind.v[i] == K_ASH && lava_dist.v[i] > 6.0 {
            px.v[i] = p(RG, 0);
            kind.v[i] = K_GRASS;
        }
    }

    // ------------------------------------------------------------ paths: lighter worn ash
    let path_f = soften(&path.to_f32(), 6);
    for y in 0..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            if !(island.v[i] && path.v[i]) || lava_full.v[i] && !arch.v[i] {
                continue;
            }
            let center = path_f.v[i] > 0.8 + c.wob.v[i] * 0.3;
            px.v[i] = p(RA, if center { 3 } else { 2 });
            kind.v[i] = K_PATH;
        }
    }

    // ------------------------------------------------------------ obsidian paving (courtyard, terrace)
    paint_paving(&c, &mut px, &mut kind, &island);

    // ------------------------------------------------------------ lava areas (placeholder colour; drawn live)
    for i in 0..px.v.len() {
        if lava_full.v[i] {
            px.v[i] = p(RL, 1);
            kind.v[i] = K_LAVA;
        }
    }
    // bank rim: ground touching lava gets a dark cooled crust line
    for y in 1..ph - 1 {
        for x in 1..pw - 1 {
            let i = (y * pw + x) as usize;
            if lava_full.v[i] || !island.v[i] {
                continue;
            }
            let near = lava_full.at(x + 1, y)
                || lava_full.at(x - 1, y)
                || lava_full.at(x, y + 1)
                || lava_full.at(x, y - 1);
            if near && walkable_kind(kind.v[i]) {
                px.v[i] = p(RB, 0);
            }
        }
    }

    // ------------------------------------------------------------ bridges
    // obsidian decks over the lake and the canal
    for y in 0..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            if !deck.v[i] {
                continue;
            }
            let curb_w = !deck.at(x - 1, y) || !deck.at(x - 2, y);
            let curb_e = !deck.at(x + 1, y) || !deck.at(x + 2, y);
            let lx = x.rem_euclid(16);
            let ly = y.rem_euclid(8);
            let mut l = 2;
            if ly == 7 {
                l = 1;
            } else if ly == 0 {
                l = 3;
            }
            if lx == 15 && !curb_e {
                l = 1;
            }
            if curb_w {
                l = if !deck.at(x - 1, y) { 4 } else { 3 };
            }
            if curb_e {
                l = if !deck.at(x + 1, y) { 0 } else { 1 };
            }
            px.v[i] = p(RO, l);
            kind.v[i] = K_DECK;
        }
    }
    // natural basalt arch: path surface already painted; give it rocky edges and a face over the lava
    for y in 0..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            if !arch.v[i] {
                continue;
            }
            // A worn ash road runs over a single basalt span, with broken stone curbs.
            let rim = 3 + (hash2(x / 13, 0, 42) * 3.0) as i32;
            let curb = !arch.at(x, y - rim) || !arch.at(x, y + rim);
            px.v[i] = if curb { p(RB, 2) } else { p(RA, 3) };
            // A few dry fractures follow the old crossing's supporting stone.
            let seam = (x + y / 3).rem_euclid(37);
            if !curb && seam == 0 && y.rem_euclid(48) > 14 && y.rem_euclid(48) < 28 {
                px.v[i] = p(RA, 2);
            }
            kind.v[i] = K_DECK;
            if !arch.at(x, y - 1) {
                px.v[i] = p(RB, 4);
            }
        }
    }

    // ------------------------------------------------------------ the dais (raised obsidian in the lake)
    paint_dais(&c, &mut px, &mut kind, &dais);

    // ------------------------------------------------------------ obsidian walls
    paint_walls(&c, &mut px, &mut kind, &wall);

    // ------------------------------------------------------------ basalt mesas
    paint_rocks(&c, &mut px, &mut kind, &rock);

    // ------------------------------------------------------------ lava banks: little faces where ground overhangs lava
    let mut lava_draw = lava_full.clone();
    for x in 0..pw {
        let mut y = 1;
        while y < ph {
            let i = (y * pw + x) as usize;
            let above = ((y - 1) * pw + x) as usize;
            if lava_full.v[i] && !lava_full.v[above] && island.v[above] {
                let k = kind.v[above];
                let (r, face_h) = match k {
                    K_DECK => (RB, 9 + (hash2(x / 3, 0, 43) * 3.0) as i32),
                    K_DAIS => (RO, 16),
                    K_WALLFACE | K_WALLTOP => (RO, 3),
                    _ => (
                        RB,
                        3 + (noise2(x as f32 / 9.0, y as f32 / 9.0, 47) * 3.0) as i32,
                    ),
                };
                for j in 0..face_h {
                    let yy = y + j;
                    if yy >= ph || !lava_full.at(x, yy) {
                        break;
                    }
                    let ii = (yy * pw + x) as usize;
                    lava_draw.v[ii] = false;
                    kind.v[ii] = K_BANK;
                    let lv = if r == RO {
                        // dais face: masonry courses
                        let course = j / 5;
                        let bx = (x + course * 7).rem_euclid(14);
                        if j == 0 {
                            4
                        } else if j % 5 == 0 || bx == 0 {
                            1
                        } else if j % 5 == 1 || bx == 1 {
                            3
                        } else {
                            2
                        }
                    } else if j == 0 {
                        3
                    } else if j < face_h / 2 {
                        2
                    } else if j < face_h - 1 {
                        1
                    } else {
                        0
                    };
                    px.v[ii] = p(r, lv as i8);
                }
                y += face_h.max(1);
                continue;
            }
            y += 1;
        }
    }
    // The natural bridge has a visible south face over the river beneath it.
    runs(&arch, |x, _y0, y1| {
        let (column, bx, bw) = band(x, 45);
        let depth = 9 + (hash2(column, y1, 46) * 5.0) as i32;
        for j in 1..=depth {
            let y = y1 + j;
            if !lava_full.inside(x, y) || !lava_full.at(x, y) {
                break;
            }
            let i = (y * pw + x) as usize;
            lava_draw.v[i] = false;
            kind.v[i] = K_BANK;
            let l = if j == 1 {
                4
            } else if bx == bw - 1 {
                0
            } else if bx == 0 {
                3
            } else if j < depth / 2 {
                2
            } else {
                1
            };
            px.v[i] = p(RB, l);
        }
    });
    // the dais stairs: steps down to the bridge
    for ty in 0..th {
        for tx in 0..tw {
            if mat(tx, ty) != Mat::Stairs {
                continue;
            }
            for y in ty * TILE..ty * TILE + TILE {
                for x in tx * TILE..tx * TILE + TILE {
                    let i = (y * pw + x) as usize;
                    let sy = (y - ty * TILE) % 4;
                    let edge = x == tx * TILE && mat(tx - 1, ty) != Mat::Stairs;
                    let edge_e = x == tx * TILE + TILE - 1 && mat(tx + 1, ty) != Mat::Stairs;
                    px.v[i] = p(
                        RO,
                        if sy == 0 {
                            4
                        } else if sy == 3 {
                            1
                        } else {
                            2
                        },
                    );
                    if edge {
                        px.v[i] = p(RO, 3);
                    }
                    if edge_e {
                        px.v[i] = p(RO, 0);
                    }
                    kind.v[i] = K_DAIS;
                    lava_draw.v[i] = false;
                }
            }
        }
    }
    // under the decks the lava runs on, unseen
    for i in 0..lava_draw.v.len() {
        if deck.v[i] || arch.v[i] {
            lava_draw.v[i] = false;
        }
    }

    // ------------------------------------------------------------ the island's hanging faces and lavafalls
    let mut fall = Grid::new(pw, ph, false);
    for x in 0..pw {
        let mut y = 1;
        while y < ph {
            let i = (y * pw + x) as usize;
            let above = ((y - 1) * pw + x) as usize;
            if !island.v[i] && island.v[above] {
                let lava_above = lava_full.v[above];
                let (b, bx, bw) = band(x, 51);
                let base_len = 44.0 + (noise2(x as f32 / 40.0, 0.5, 53) - 0.5) * 22.0;
                let col_len = base_len as i32 + (hash2(b, 0, 55) * 14.0) as i32 - 4;
                let len = if lava_above { col_len + 70 } else { col_len };
                let mut j = 0;
                while j < len {
                    let yy = y + j;
                    if yy >= ph || island.at(x, yy) {
                        break;
                    }
                    let ii = (yy * pw + x) as usize;
                    if lava_above {
                        fall.v[ii] = true;
                        lava_full.v[ii] = true;
                        lava_draw.v[ii] = true;
                        kind.v[ii] = K_FALL;
                        px.v[ii] = p(RL, 1);
                    } else {
                        // basalt columns, darker with depth, the lit lip on top
                        let depth = j as f32 / len as f32;
                        let mut l: i32 = if j == 0 {
                            3
                        } else if depth < 0.28 {
                            2
                        } else if depth < 0.72 {
                            1
                        } else {
                            0
                        };
                        if bx == 0 && j > 0 {
                            l = (l + 1).min(3);
                        }
                        if bx == bw - 1 {
                            l = (l - 1).max(0);
                        }
                        // horizontal fracture in the column
                        let fy = 8 + (hash2(b, 1, 57) * 24.0) as i32;
                        if j == fy {
                            l = 0;
                        } else if j == fy + 1 {
                            l = (l + 1).min(3);
                        }
                        px.v[ii] = if depth > 0.9 {
                            p(RS, 1)
                        } else {
                            p(RB, l as i8)
                        };
                        kind.v[ii] = K_ISLEFACE;
                    }
                    j += 1;
                }
                y += j.max(1);
                continue;
            }
            y += 1;
        }
    }
    // north rims of the island catch the sky light
    for y in 1..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            if island.v[i] && !island.v[i - pw as usize] && !lava_full.v[i] {
                if matches!(
                    kind.v[i],
                    K_ASH | K_GRASS | K_PATH | K_ROCKTOP | K_PAVE | K_DAIS | K_WALLTOP
                ) {
                    let q = px.v[i];
                    px.v[i] = P { r: q.r, l: q.l + 1 };
                }
            }
        }
    }

    // ------------------------------------------------------------ small details: pebbles, stones, cracks
    paint_details(&c, &mut px, &kind, &rock_dist, &lava_dist);

    // ------------------------------------------------------------ grass tufts
    let mut tufts = Vec::new();
    {
        let cell = 11;
        let mut gy = 0;
        while gy < ph {
            let mut gx = 0;
            while gx < pw {
                let x = gx + (hash2(gx, gy, 61) * cell as f32) as i32;
                let y = gy + (hash2(gx, gy, 62) * cell as f32) as i32;
                if x < pw && y < ph {
                    let i = (y * pw + x) as usize;
                    let k = kind.v[i];
                    let dense = k == K_GRASS && grass.v[i];
                    let stray = (k == K_ASH || k == K_PATH)
                        && hash2(x, y, 63) < 0.05
                        && rock_dist.v[i] < 30.0
                        && lava_dist.v[i] > 14.0;
                    let ok = (dense && hash2(x, y, 64) < 0.36) || stray;
                    if ok {
                        let s = (hash2(x, y, 65) * TUFTS.len() as f32) as u8;
                        let st = TUFTS[s as usize];
                        // keep the whole tuft on walkable ground
                        let (w, h) = stamp_size(st);
                        let fits = (0..w).all(|dx| {
                            walkable_kind(kind.at(x - w / 2 + dx, y))
                                && walkable_kind(kind.at(x - w / 2 + dx, y - h + 1))
                        });
                        if fits {
                            tufts.push((x, y, s, hash2(x, y, 66) < 0.5));
                        }
                    }
                }
                gx += cell;
            }
            gy += cell;
        }
    }

    // Burned tufts are dry scenery, not another animated signature effect.
    for &(x, y, id, flip) in &tufts {
        let st = TUFTS[id as usize];
        let (w, h) = stamp_size(st);
        for (dy, row) in st.iter().enumerate() {
            for (dx, ch) in row.bytes().enumerate() {
                if ch == b'.' {
                    continue;
                }
                let xx = x - w / 2 + if flip { w - 1 - dx as i32 } else { dx as i32 };
                px.set(xx, y - h + 1 + dy as i32, p(RG, (ch - b'0').min(3) as i8));
            }
        }
    }

    // ------------------------------------------------------------ write the zone ground
    for i in 0..px.v.len() {
        let q = px.v[i];
        z.ground[i] = if q.r == HOLE_R {
            HOLE
        } else {
            rc(q.r, q.l as i32)
        };
    }

    Painted {
        kind,
        lava_full,
        lava_draw,
        fall,
    }
}

/// Lay weathered obsidian slabs in staggered courses.
fn paint_paving(c: &Ctx, px: &mut Grid<P>, kind: &mut Grid<u8>, island: &Grid<bool>) {
    let (pw, ph) = (c.pw, c.ph);
    // Slabs 32x24, running bond. A slab exists where its center is on a paving tile; slabs on the
    // edge of the paved area are sometimes missing (rubble), so the border reads as worn.
    let (sw, sh) = (32, 24);
    for y in 0..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            if !island.v[i] {
                continue;
            }
            let row = y.div_euclid(sh);
            let off = if row % 2 == 0 { 0 } else { sw / 2 };
            let col = (x + off).div_euclid(sw);
            let (lx, ly) = ((x + off).rem_euclid(sw), y.rem_euclid(sh));
            let cx = col * sw - off + sw / 2;
            let cy = row * sh + sh / 2;
            let m = mat(cx.div_euclid(TILE), cy.div_euclid(TILE));
            if m != Mat::Paving {
                continue;
            }
            let edge = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                mat(
                    (cx + dx * sw).div_euclid(TILE),
                    (cy + dy * sh).div_euclid(TILE),
                ) != Mat::Paving
            });
            let hs = hash2(col, row, 71);
            if edge && hs < 0.4 {
                continue; // missing slab: ash shows
            }
            // A rubble gap exposes its neighbor too, breaking aligned gaps into worn stone edges.
            let gone = |dx: i32| {
                let (nx, nc) = (cx + dx * sw, col + dx);
                mat(nx.div_euclid(TILE), cy.div_euclid(TILE)) == Mat::Paving
                    && [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(ex, ey)| {
                        mat((nx + ex * sw).div_euclid(TILE), (cy + ey * sh).div_euclid(TILE)) != Mat::Paving
                    })
                    && hash2(nc, row, 71) < 0.4
            };
            if edge || gone(-1) || gone(1) {
                let exposed = |dx: i32, dy: i32| (dy == 0 && gone(dx)) || mat((cx + dx * sw).div_euclid(TILE), (cy + dy * sh).div_euclid(TILE)) != Mat::Paving;
                let bite_x = [3, 6, 4, 7][((ly / 5 + col).rem_euclid(4)) as usize];
                let bite_y = [4, 2, 6, 3][((lx / 6 + row).rem_euclid(4)) as usize];
                let vertical = (6..sh - 3).contains(&ly)
                    && ((exposed(-1, 0) && lx < bite_x) || (exposed(1, 0) && lx >= sw - bite_x));
                let horizontal = (7..sw - 4).contains(&lx)
                    && ((exposed(0, -1) && ly < bite_y) || (exposed(0, 1) && ly >= sh - bite_y));
                if vertical || horizontal {
                    continue; // short, stepped ash tongues interrupt the court perimeter
                }
            }
            // A readable cool midtone stays calm beneath the lit stone; joints stay subordinate.
            let tone = 4;
            let mut l = tone;
            if (ly == sh - 1 && lx > 5) || (lx == sw - 1 && ly > 7) {
                l = 3; // shallow grout (lower right), rather than black slab outlines
            } else if ly == 0 && lx < 7 && edge {
                l = tone + 1; // a chipped edge, not a bevel on every slab
            }
            // a few cracked slabs
            let hc = hash2(col, row, 73);
            if edge && hc < 0.12 && l != 2 {
                let d = if hc < 0.09 {
                    lx - ly * 4 / 3
                } else {
                    (sw - lx) - ly * 4 / 3
                };
                if d == 2 {
                    l = 2;
                } else if d == 3 {
                    l = tone + 1;
                }
            }
            px.v[i] = p(RO, l as i8);
            kind.v[i] = K_PAVE;
        }
    }
}

/// Draw radial obsidian slabs and a copper shard setting.
fn paint_dais(c: &Ctx, px: &mut Grid<P>, kind: &mut Grid<u8>, dais: &Grid<bool>) {
    let (pw, ph) = (c.pw, c.ph);
    // dais center (from the map: the ellipse at tile 71, 11.4)
    let (cx, cy) = (71.0 * TILE as f32 + 8.0, 11.4 * TILE as f32);
    for y in 0..ph {
        for x in 0..pw {
            let i = (y * pw + x) as usize;
            if !dais.v[i] {
                continue;
            }
            let dx = x as f32 + 0.5 - cx;
            let dy = (y as f32 + 0.5 - cy) * 1.25; // 3/4 view: circles are squashed
            let r = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            // rings of slabs
            let ring = (r / 13.0) as i32;
            let segs = 6 + ring * 6;
            let seg = ((a / std::f32::consts::TAU + 0.5) * segs as f32) as i32;
            let fr = r - ring as f32 * 13.0;
            let fa = ((a / std::f32::consts::TAU + 0.5) * segs as f32).fract();
            let mut l = if hash2(ring, seg, 81) > 0.8 { 3 } else { 2 };
            if fr < 1.0 {
                l = 1;
            } else if fr < 2.0 {
                l += 1;
            }
            if fa < 0.04 * (1.0 + 4.0 / (r + 1.0)) && r > 4.0 {
                l = 1;
            }
            let mut q = p(RO, l as i8);
            // copper inlay circle around the shard
            if (r - 27.5).abs() < 0.8 {
                q = p(RC, 2);
            } else if (r - 26.5).abs() < 0.5 && dy < 0.0 {
                q = p(RC, 3);
            }
            // lit outer lip
            if !dais.at(x, y - 1) || !dais.at(x - 1, y) {
                q = p(RO, 4);
            } else if !dais.at(x + 1, y) {
                q = p(RO, 1);
            }
            px.v[i] = q;
            kind.v[i] = K_DAIS;
        }
    }
}

/// Shade broken obsidian masonry with distinct top and front planes.
fn paint_walls(c: &Ctx, px: &mut Grid<P>, kind: &mut Grid<u8>, wall: &Grid<bool>) {
    let pw = c.pw;
    const FULL: i32 = 21;
    runs(wall, |x, y0, y1| {
        let len = y1 - y0 + 1;
        // broken height per 8 px segment
        let seg = x.div_euclid(8);
        let hs = hash2(seg, y1 / TILE, 91);
        let h = if hs < 0.45 {
            FULL
        } else if hs < 0.75 {
            FULL - 5
        } else {
            FULL - 10
        }
        .min(len - 4);
        let top_t = len - FULL; // top surface thickness for a full wall
        let top_y1 = y1 - h; // last top row
        let top_y0 = top_y1 - top_t + 1;
        for y in y0..=y1 {
            let i = (y * pw + x) as usize;
            if y < top_y0 {
                // behind a broken section: rubble on ash
                let r = hash2(x / 2, y / 2, 93);
                px.v[i] = if r < 0.35 {
                    p(RO, 2)
                } else if r < 0.5 {
                    p(RO, 3)
                } else {
                    p(RA, 1)
                };
                kind.v[i] = K_WALLTOP;
                continue;
            }
            if y <= top_y1 {
                // top surface
                let mut l = 3;
                if y == top_y0 || !wall.at(x - 1, y) {
                    l = 4;
                }
                if !wall.at(x + 1, y) {
                    l = 2;
                }
                if y == top_y1 {
                    l = 4; // front lip
                }
                // where a lower section meets a taller one, the taller one's side shows
                let hl = {
                    let s2 = (x - 1).div_euclid(8);
                    if s2 != seg {
                        Some(hash2(s2, y1 / TILE, 91))
                    } else {
                        None
                    }
                };
                if let Some(h2) = hl {
                    let _ = h2;
                }
                px.v[i] = p(RO, l);
                kind.v[i] = K_WALLTOP;
                continue;
            }
            // face: masonry courses of 5 px, blocks 9..13 wide, staggered
            let j = y - top_y1 - 1; // 0 at top of face
            let course = j / 5;
            let cj = j % 5;
            let shift = (hash2(course, x / 64, 95) * 13.0) as i32 + course * 6;
            let bw = 11;
            let bx = (x + shift).rem_euclid(bw);
            let mut l = 4;
            if cj == 4 || bx == bw - 1 {
                l = 3; // mortar (lower/right)
            } else if (cj == 0 || bx == 0) && hash2((x + shift).div_euclid(bw), course, 96) < 0.3 {
                l = 5; // occasional chipped upper-left block edge, not a bevel on every brick
            }
            if j == 0 {
                l = 5;
            }
            if y >= y1 - 1 {
                l = (l - 1).max(0); // occluded foot
            }
            if !wall.at(x + 1, y) {
                l = 1;
            }
            px.v[i] = p(RO, l as i8);
            kind.v[i] = K_WALLFACE;
        }
    });
}

/// Shape basalt mesas into stepped column faces and broad top planes.
fn paint_rocks(c: &Ctx, px: &mut Grid<P>, kind: &mut Grid<u8>, rock: &Grid<bool>) {
    let pw = c.pw;
    runs(rock, |x, y0, y1| {
        let len = y1 - y0 + 1;
        let (b, bx, bw) = band(x, 101);
        let fh_base = 28.0 + (noise2(x as f32 / 46.0, y1 as f32 / 46.0, 103) - 0.5) * 12.0;
        // each basalt column ends at its own height, so the lip is stepped, not a smooth curve
        let fh = (fh_base as i32 + (hash2(b, y1 / 32, 105) * 5.0) as i32 - 2)
            .min(len - 3)
            .max(0);
        let lip = y1 - fh; // last top row
        for y in y0..=y1 {
            let i = (y * pw + x) as usize;
            if y <= lip {
                // mesa top: calm basalt with lighter shelves, lit north/west rims
                let n = fbm(x as f32 / 40.0, y as f32 / 30.0, 2, 107);
                let mut l = if n > 0.58 { 3 } else { 2 };
                if !rock.at(x, y - 1) || !rock.at(x - 1, y) {
                    l = 4;
                } else if !rock.at(x, y - 2) || !rock.at(x - 2, y) {
                    l = 3;
                }
                if !rock.at(x + 1, y) {
                    l = 1;
                }
                if y == lip {
                    l = 4; // front edge catches the light
                } else if y == lip - 1 {
                    l = l.max(3);
                }
                px.v[i] = p(RB, l);
                kind.v[i] = K_ROCKTOP;
            } else {
                let j = y - lip - 1;
                let depth = j as f32 / fh.max(1) as f32;
                let split = 0.35 + hash2(b, 3, 109) * 0.2;
                let mut l: i32 = if depth < split { 2 } else { 1 };
                if bx == 0 {
                    l += 1;
                }
                if bx == bw - 1 {
                    l -= 1;
                }
                let fy = 6 + (hash2(b, 4, 111) * 20.0) as i32;
                if j == fy && fy < fh - 3 {
                    l = 0;
                } else if j == fy + 1 && fy < fh - 3 {
                    l += 1;
                }
                if y >= y1 - 1 {
                    l = 0; // dark foot
                }
                if !rock.at(x + 1, y) {
                    l = l.min(1);
                }
                px.v[i] = p(RB, l.clamp(0, 3) as i8);
                kind.v[i] = K_ROCKFACE;
            }
        }
    });
}

/// Place a small stone cluster only when its entire silhouette fits.
fn stamp(
    px: &mut Grid<P>,
    kind: &Grid<u8>,
    x: i32,
    y: i32,
    s: Stamp,
    r: u8,
    lv: [i8; 3],
    allowed: impl Fn(u8) -> bool,
) -> bool {
    let (w, h) = stamp_size(s);
    for (dy, row) in s.iter().enumerate() {
        for (dx, ch) in row.bytes().enumerate() {
            if ch != b'.' && !allowed(kind.at(x + dx as i32, y + dy as i32)) {
                return false;
            }
        }
    }
    let _ = (w, h);
    for (dy, row) in s.iter().enumerate() {
        for (dx, ch) in row.bytes().enumerate() {
            let l = match ch {
                b'h' => lv[0],
                b'm' => lv[1],
                b'd' => lv[2],
                _ => continue,
            };
            px.set(x + dx as i32, y + dy as i32, P { r, l });
        }
    }
    true
}

/// Gather scree near cliffs and sparse dry fissures on the ash fields.
fn paint_details(
    c: &Ctx,
    px: &mut Grid<P>,
    kind: &Grid<u8>,
    rock_dist: &Grid<f32>,
    lava_dist: &Grid<f32>,
) {
    let (pw, ph) = (c.pw, c.ph);
    let ground = |k: u8| matches!(k, K_ASH | K_PATH | K_GRASS);
    // pebbles and stones: scree gathers at the foot of the mesas, sparse elsewhere
    let cell = 12;
    let mut gy = 0;
    while gy < ph {
        let mut gx = 0;
        while gx < pw {
            let x = gx + (hash2(gx, gy, 121) * cell as f32) as i32;
            let y = gy + (hash2(gx, gy, 122) * cell as f32) as i32;
            if x < pw && y < ph {
                let i = (y * pw + x) as usize;
                let rd = rock_dist.v[i];
                let dens = 0.015
                    + 0.32 * (-rd / 12.0).exp()
                    + if lava_dist.v[i] < 16.0 { 0.1 } else { 0.0 };
                let h = hash2(x, y, 123);
                if h < dens && ground(kind.v[i]) {
                    let big = hash2(x, y, 124) < 0.25 + 0.3 * (-rd / 10.0).exp();
                    if big {
                        let s = STONES[(hash2(x, y, 125) * 4.0) as usize % 4];
                        stamp(px, kind, x, y, s, RB, [3, 2, 1], ground);
                    } else {
                        let s = PEBBLES[(hash2(x, y, 126) * 4.0) as usize % 4];
                        stamp(px, kind, x, y, s, RB, [3, 2, 1], ground);
                    }
                }
            }
            gx += cell;
        }
        gy += cell;
    }
    // cracks in ash: short branching dark lines with a lit lower lip
    let cell = 40;
    let mut gy = 0;
    while gy < ph {
        let mut gx = 0;
        while gx < pw {
            if hash2(gx, gy, 131) < 0.45 {
                let mut x = gx + (hash2(gx, gy, 132) * cell as f32) as i32;
                let mut y = gy + (hash2(gx, gy, 133) * cell as f32) as i32;
                let len = 5 + (hash2(gx, gy, 134) * 9.0) as i32;
                let mut dir = if hash2(gx, gy, 135) < 0.5 { 1 } else { -1 };
                for s in 0..len {
                    let i = (y.clamp(0, ph - 1) * pw + x.clamp(0, pw - 1)) as usize;
                    if !(0..pw).contains(&x) || !(0..ph).contains(&y) || kind.v[i] != K_ASH {
                        break;
                    }
                    let q = px.v[i];
                    if q.r == RA {
                        px.v[i] = p(RA, (q.l - 1).max(0));
                    }
                    x += dir;
                    if hash2(x, y + s, 136) < 0.45 {
                        y += 1;
                    }
                    if hash2(x, y, 137) < 0.12 {
                        dir = -dir;
                        y += 1;
                    }
                }
            }
            gx += cell;
        }
        gy += cell;
    }
}
