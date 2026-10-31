//! Cached terrain: quiet meadow fields, laid limestone, cut banks and a layered island face.

use super::{K, Map, mat, pal};
use crate::gfx::{Cell, sheet};
use crate::util::hash2;
use crate::world::{HOLE, Zone};

pub const ALTAR: (f32, f32) = (408.0, 176.0);

/// Copy a named terrain cell into the cached ground without resampling its pixels.
fn tile(z: &mut Zone, name: &str, x: i32, y: i32) {
    let Some(s) = sheet("bloomwild_tiles") else {
        return;
    };
    let Some(at) = s.json.get("tiles").and_then(|v| v.get(name)) else {
        return;
    };
    let (Some(cx), Some(cy)) = (at[0].as_u64(), at[1].as_u64()) else {
        return;
    };
    let Some(c) = s
        .cells
        .get(cy as usize)
        .and_then(|row| row.get(cx as usize))
    else {
        return;
    };
    blit(z, c, x, y);
}

/// Blit opaque sprite pixels into the ground cache.
fn blit(z: &mut Zone, c: &Cell, x: i32, y: i32) {
    for py in 0..c.h {
        for px in 0..c.w {
            let p = c.at(px, py);
            if p.a {
                z.gset(x + px, y + py, p.c);
            }
        }
    }
}

/// Classify floor materials before rounding the boundaries between them.
fn group(k: K) -> u8 {
    match k {
        K::Slab | K::Altar => mat::SLAB,
        K::Path => mat::PATH,
        K::Deep => mat::DEEP,
        K::Shallow => mat::SHALLOW,
        K::Bridge => mat::BRIDGE,
        K::Cliff | K::Niche => mat::CLIFF,
        K::Falls => mat::FALLS,
        K::Stairs => mat::STAIRS,
        K::Void => mat::VOID,
        _ => mat::GRASS,
    }
}

/// Smooth only floor boundaries, keeping cliff collision and bridge crossings exact.
fn material(m: &Map, x: i32, y: i32) -> u8 {
    let here = group(m.at(x / 16, y / 16));
    if matches!(
        here,
        mat::VOID | mat::CLIFF | mat::FALLS | mat::STAIRS | mat::BRIDGE | mat::DEEP
    ) {
        return here;
    }
    let gx = (x as f32 - 8.0) / 16.0;
    let gy = (y as f32 - 8.0) / 16.0;
    let (ix, iy) = (gx.floor() as i32, gy.floor() as i32);
    let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
    let mut weights = [0.0f32; 16];
    for (dx, dy, w) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let g = group(m.at(ix + dx, iy + dy));
        weights[g as usize] += w;
    }
    let wet = weights[mat::DEEP as usize] + weights[mat::SHALLOW as usize];
    if wet > 0.40 {
        return if weights[mat::DEEP as usize] > 0.48 {
            mat::DEEP
        } else {
            mat::SHALLOW
        };
    }
    let rough = (hash2(x.div_euclid(4), y.div_euclid(3), 62) - 0.5) * 0.12;
    if weights[mat::SLAB as usize] > 0.44 + rough {
        return mat::SLAB;
    }
    if weights[mat::PATH as usize] > 0.38 + rough {
        // The wide landing is worn only along its quiet northbound approach.
        // This changes the painted material, not the source map or walkable footprint.
        if x < 224 && y >= 624 {
            let center = 151 + (704 - y).clamp(-32, 64) / 8;
            let half_width = 17 + (y - 624).clamp(0, 96) / 24;
            if (x - center).abs() > half_width + (rough * 24.0) as i32 {
                return mat::GRASS;
            }
        }
        return mat::PATH;
    }
    mat::GRASS
}

/// Paint all static pixels once and return their physical material mask.
pub fn paint(z: &mut Zone, m: &Map) -> Vec<u8> {
    z.ground.fill(HOLE);
    let mut mask = vec![mat::VOID; (z.pw * z.ph) as usize];
    for y in 0..z.ph {
        for x in 0..z.pw {
            let g = material(m, x, y);
            mask[(y * z.pw + x) as usize] = g;
            let c = match g {
                mat::VOID => continue,
                mat::GRASS => {
                    // One meadow base crosses the former x=432/y=608 fill boundary.
                    // Damp and sheltered planes are the authored garden_fields below.
                    pal::GRASS[2]
                }
                mat::PATH => pal::EARTH[3],
                mat::SLAB => pal::STONE[4],
                mat::DEEP => pal::WATER[1],
                mat::SHALLOW => pal::WATER[2],
                mat::BRIDGE => pal::STONE[4],
                mat::CLIFF => pal::STONE[1],
                mat::FALLS => pal::WATER[3],
                mat::STAIRS => pal::STONE[3],
                _ => pal::GRASS[2],
            };
            z.gset(x, y, c);
        }
    }
    landing_fields(z, &mask);
    garden_fields(z, &mask);
    slabs(z, m, &mask);
    trails(z, &mask);
    weathering(z, &mask);
    banks(z, &mask);
    cliffs(z, m, &mut mask);
    details(z, &mask);
    sheltered_turf(z, &mask);
    sunlit_lawn(z, &mask);
    trampled_verge(z, &mask);
    opening_steps(z, &mut mask);
    altar(z);
    outer_rims(z, m, &mask);
    mask
}

const SHELTERED_TURF: &[&[(i32, i32)]] = &[
    &[(30, 478), (86, 481), (92, 497), (95, 515), (96, 535), (98, 555), (101, 575), (104, 592), (103, 608), (97, 622), (86, 633), (30, 642)],
    &[(172, 612), (188, 613), (206, 619), (226, 627), (248, 634), (262, 634), (272, 632), (292, 633), (307, 651), (305, 671), (285, 688), (250, 681), (228, 667), (194, 664), (176, 646), (172, 628)],
    &[(366, 712), (380, 704), (398, 703), (418, 708), (432, 716), (428, 723), (378, 723)],
    &[(168, 746), (178, 736), (200, 731), (226, 733), (250, 731), (270, 735), (286, 741), (286, 754), (168, 754)],
    &[(414, 726), (424, 721), (446, 722), (458, 717), (489, 716), (497, 712), (531, 713), (546, 716), (571, 715), (580, 711), (612, 710), (626, 714), (638, 721), (648, 729), (656, 740), (414, 740)],
];

/// Keep the island lip clear while repainting only open meadow pixels.
fn clear_meadow(z: &Zone, x: i32, y: i32) -> bool {
    if z.tile_at(x as f32, y as f32) != crate::world::Tile::Floor { return false; }
    let void = |xx: i32, yy: i32| z.tile(xx.div_euclid(16), yy.div_euclid(16)) == crate::world::Tile::Void;
    !(1..=3).any(|d| void(x - d, y) || void(x + d, y) || void(x, y - d) || void(x, y + d))
}

/// Match the painter's integer half-open scanline rule at one world pixel.
fn in_plane(points: &[(i32, i32)], x: i32, y: i32) -> bool {
    let mut inside = false;
    for i in 0..points.len() {
        let (ax, ay) = points[i];
        let (bx, by) = points[(i + 1) % points.len()];
        if ((ay <= y && by > y) || (by <= y && ay > y))
            && ax + (y - ay) * (bx - ax) / (by - ay) <= x {
            inside = !inside;
        }
    }
    inside
}

/// Reveal projected tree and stone shade inside the selected landing turf masses.
pub(super) fn sheltered_shadow(z: &Zone, x: i32, y: i32) -> bool {
    let landing = (60..156).contains(&x) && (624..744).contains(&y);
    let fins = (226..426).contains(&x) && (680..744).contains(&y);
    (landing || fins || SHELTERED_TURF.iter().any(|p| in_plane(p, x, y))) && clear_meadow(z, x, y)
}

/// Join sheltered turf below the arch, fins and island lips while retaining the lit approach.
fn sheltered_turf(z: &mut Zone, mask: &[u8]) {
    for points in SHELTERED_TURF {
        let left = points.iter().map(|p| p.0).min().unwrap();
        let right = points.iter().map(|p| p.0).max().unwrap();
        let top = points.iter().map(|p| p.1).min().unwrap();
        let bottom = points.iter().map(|p| p.1).max().unwrap();
        for y in top..bottom {
            for x in left..right {
                let i = (y * z.pw + x) as usize;
                if mask[i] == mat::GRASS && pal::GRASS[1..=3].contains(&z.ground[i])
                    && in_plane(points, x, y) && clear_meadow(z, x, y) {
                    z.gset(x, y, pal::GRASS[1]);
                }
            }
        }
    }
}

/// Open landing turf clear of the tree and fin shadows, leading from the path to the broken pier.
const SUNLIT_LAWN: &[(i32, i32)] = &[
    (138, 668), (158, 670), (182, 679), (204, 684), (226, 694), (250, 700), (272, 712), (276, 726),
    (262, 735), (238, 740), (214, 737), (190, 741), (164, 738), (138, 742),
];

/// Lift the open landing lawn one step where daylight reaches it past the tree and ruin.
fn sunlit_lawn(z: &mut Zone, mask: &[u8]) {
    for y in 660..744 {
        for x in 130..284 {
            let i = (y * z.pw + x) as usize;
            if mask[i] == mat::GRASS && z.ground[i] == pal::GRASS[2]
                && in_plane(SUNLIT_LAWN, x, y) && clear_meadow(z, x, y) {
                z.gset(x, y, pal::GRASS[3]);
            }
        }
    }
}

/// Wear the landing turf back to bare earth where feet step off the stepping-stone path.
/// Only colors change: the verge keeps its grass material, so tufts and collision are untouched.
fn trampled_verge(z: &mut Zone, mask: &[u8]) {
    let worn = |m: u8| m == mat::PATH;
    for y in 560..760 {
        for x in 8..380 {
            let i = (y * z.pw + x) as usize;
            if mask[i] != mat::GRASS || !clear_meadow(z, x, y) {
                continue;
            }
            // Feet scuff wider at a few bends and almost nothing elsewhere.
            let width = 2 + (hash2(x.div_euclid(9), y.div_euclid(7), 520) * 4.6) as i32;
            let Some(d) = (1..=width).find(|&d| {
                [(-d, 0), (d, 0), (0, -d), (0, d)]
                    .iter()
                    .any(|&(dx, dy)| worn(mask[((y + dy) * z.pw + x + dx) as usize]))
            }) else {
                continue;
            };
            // Shaded turf stays green; in the sun the dust packs pale against the stones.
            if z.ground[i] == pal::GRASS[1] || z.ground[i] == pal::GRASS[0] {
                continue;
            }
            z.gset(x, y, if d == 1 { pal::EARTH[3] } else { pal::EARTH[2] });
        }
    }
}

/// Erode the raw north and west tile edges into short stone lips without changing collision.
fn outer_rims(z: &mut Zone, m: &Map, mask: &[u8]) {
    const CUT: [i32; 12] = [1, 2, 3, 4, 3, 2, 1, 1, 0, 1, 3, 4];
    for y in 0..z.ph {
        for x in 0..z.pw {
            let i = (y * z.pw + x) as usize;
            if mask[i] != mat::GRASS || z.tile_at(x as f32, y as f32) != crate::world::Tile::Floor { continue; }
            let (tx, ty) = (x / 16, y / 16);
            let north = m.at(tx, ty - 1) == K::Void;
            let west = m.at(tx - 1, ty) == K::Void;
            let nd = if north { y.rem_euclid(16) + 1 } else { 99 };
            let wd = if west { x.rem_euclid(16) + 1 } else { 99 };
            let nc = CUT[x.div_euclid(9).rem_euclid(12) as usize];
            let wc = CUT[(y.div_euclid(9) + 5).rem_euclid(12) as usize];
            if nd <= nc || wd <= wc {
                z.gset(x, y, crate::gfx::Rgb::new(-1.0, -1.0, -1.0));
            } else if nd == nc + 1 || wd == wc + 1 {
                z.gset(x, y, if nd == nc + 1 { pal::STONE[3] } else { pal::STONE[2] });
            } else if nd == nc + 2 || wd == wc + 2 {
                z.gset(x, y, pal::GRASS[1]);
            }
        }
    }
}

/// Anchor broad damp turf, worn earth and stone fragments to the waterworks and grove.
fn garden_fields(z: &mut Zone, mask: &[u8]) {
    let patches: &[(&[(i32, i32)], crate::gfx::Rgb)] = &[
        (
            &[
                (620, 624),
                (652, 615),
                (676, 628),
                (698, 664),
                (708, 688),
                (743, 710),
                (790, 714),
                (799, 735),
                (731, 735),
                (706, 720),
                (685, 679),
                (651, 661),
            ],
            pal::GRASS[1],
        ),
        (
            &[
                (885, 637),
                (911, 635),
                (949, 653),
                (978, 684),
                (976, 719),
                (998, 735),
                (918, 735),
                (913, 713),
                (893, 697),
                (879, 660),
            ],
            pal::TEAL[1],
        ),
        (
            &[
                (613, 636),
                (640, 634),
                (663, 648),
                (674, 671),
                (692, 687),
                (680, 692),
                (655, 678),
                (643, 655),
                (616, 650),
            ],
            pal::EARTH[2],
        ),
        (
            &[
                (888, 612),
                (915, 613),
                (924, 626),
                (912, 638),
                (896, 636),
                (882, 623),
            ],
            pal::STONE[2],
        ),
        (
            &[
                (666, 710),
                (686, 716),
                (704, 729),
                (731, 732),
                (736, 735),
                (682, 735),
                (666, 728),
            ],
            pal::STONE[2],
        ),
        (
            &[
                (627, 352),
                (652, 352),
                (674, 370),
                (674, 393),
                (653, 415),
                (615, 420),
                (602, 400),
                (619, 383),
            ],
            pal::GRASS[1],
        ),
        (
            &[
                (758, 352),
                (813, 352),
                (841, 370),
                (832, 402),
                (810, 417),
                (781, 408),
                (765, 382),
            ],
            pal::TEAL[1],
        ),
        (
            &[
                (869, 159),
                (900, 147),
                (944, 155),
                (971, 174),
                (960, 197),
                (928, 211),
                (887, 197),
                (857, 186),
            ],
            pal::GRASS[1],
        ),
        (
            &[
                (319, 493),
                (345, 501),
                (387, 505),
                (418, 499),
                (435, 513),
                (410, 532),
                (367, 535),
                (337, 523),
                (313, 515),
            ],
            pal::EARTH[2],
        ),
    ];
    for &(points, color) in patches {
        let top = points.iter().map(|p| p.1).min().unwrap();
        let bottom = points.iter().map(|p| p.1).max().unwrap();
        for y in top..bottom {
            let mut cuts = Vec::new();
            for i in 0..points.len() {
                let (ax, ay) = points[i];
                let (bx, by) = points[(i + 1) % points.len()];
                if (ay <= y && by > y) || (by <= y && ay > y) {
                    cuts.push(ax + (y - ay) * (bx - ax) / (by - ay));
                }
            }
            cuts.sort_unstable();
            for pair in cuts.as_chunks::<2>().0 {
                for x in pair[0]..pair[1] {
                    if mask[(y * z.pw + x) as usize] == mat::GRASS {
                        if color == pal::STONE[2] {
                            let chip = 1 + (hash2(y.div_euclid(3), top, 181) * 4.0) as i32;
                            if x < pair[0] + chip || x >= pair[1] - chip {
                                continue;
                            }
                            let edge = x == pair[1] - chip - 1;
                            z.gset(x, y, if edge { pal::STONE[0] } else { pal::STONE[1] });
                        } else {
                            z.gset(x, y, color);
                        }
                    }
                }
            }
        }
    }
}

/// Group the landing meadow into flat sage planes rooted at the tree, ruin and lit clearing.
fn landing_fields(z: &mut Zone, mask: &[u8]) {
    let fields: &[(&[(i32, i32)], usize)] = &[
        (
            &[
                (64, 644),
                (84, 628),
                (114, 634),
                (123, 657),
                (144, 678),
                (150, 697),
                (128, 707),
                (119, 731),
                (96, 739),
                (64, 727),
            ],
            1,
        ),
        (
            &[
                (230, 694),
                (246, 683),
                (285, 685),
                (309, 699),
                (351, 701),
                (377, 719),
                (422, 721),
                (422, 739),
                (286, 739),
                (267, 729),
                (248, 728),
            ],
            1,
        ),
        (
            &[
                (189, 612),
                (249, 614),
                (270, 631),
                (291, 632),
                (304, 651),
                (302, 670),
                (281, 684),
                (242, 677),
                (225, 663),
                (195, 660),
                (179, 644),
            ],
            3,
        ),
    ];
    for &(points, shade) in fields {
        let top = points.iter().map(|p| p.1).min().unwrap();
        let bottom = points.iter().map(|p| p.1).max().unwrap();
        for y in top..bottom {
            let mut cuts = Vec::new();
            for i in 0..points.len() {
                let (ax, ay) = points[i];
                let (bx, by) = points[(i + 1) % points.len()];
                if (ay <= y && by > y) || (by <= y && ay > y) {
                    cuts.push(ax + (y - ay) * (bx - ax) / (by - ay));
                }
            }
            cuts.sort_unstable();
            for pair in cuts.chunks_exact(2) {
                for x in pair[0]..pair[1] {
                    let i = (y * z.pw + x) as usize;
                    if mask[i] == mat::GRASS {
                        z.gset(x, y, pal::GRASS[shade]);
                    }
                }
            }
        }
    }
}

/// Seat the landing distributor on shallow worn steps above the island's south lip.
fn opening_steps(z: &mut Zone, mask: &mut [u8]) {
    for y in 694..728 {
        let inset = if y < 703 {
            7
        } else if y < 714 {
            3
        } else {
            0
        };
        for x in 283 + inset..378 - inset {
            let i = (y * z.pw + x) as usize;
            if mask[i] != mat::GRASS {
                continue;
            }
            let c = if [702, 713, 724].contains(&y) {
                pal::STONE[2]
            } else if [703, 714, 725].contains(&y) {
                pal::STONE[4]
            } else if y > 724 {
                pal::STONE[2]
            } else {
                pal::STONE[3]
            };
            z.gset(x, y, c);
            mask[i] = mat::SLAB;
        }
    }
}

/// Pack the plaza and temple with the artist's large multi-tile limestone blocks.
fn slabs(z: &mut Zone, m: &Map, mask: &[u8]) {
    let mut occupied = vec![false; (m.tw * m.th) as usize];
    for ty in 0..m.th {
        for tx in 0..m.tw {
            let idx = (ty * m.tw + tx) as usize;
            if occupied[idx] || !matches!(m.at(tx, ty), K::Slab | K::Altar) {
                continue;
            }
            let variants = [
                ('a', 3, 3),
                ('b', 3, 3),
                ('c', 3, 3),
                ('d', 3, 2),
                ('e', 3, 2),
                ('f', 2, 3),
                ('g', 2, 3),
                ('h', 2, 2),
                ('i', 2, 2),
            ];
            let start = (hash2(tx, ty, 4) * 9.0) as usize;
            let choice = (0..9)
                .map(|i| variants[(i + start) % 9])
                .find(|&(_, w, h)| {
                    (ty..ty + h).all(|y| {
                        (tx..tx + w).all(|x| {
                            x < m.tw
                                && y < m.th
                                && !occupied[(y * m.tw + x) as usize]
                                && matches!(m.at(x, y), K::Slab | K::Altar)
                        })
                    })
                });
            if let Some((name, w, h)) = choice {
                for dy in 0..h {
                    for dx in 0..w {
                        tile(
                            z,
                            &format!("slab_{name}_{dx}{dy}"),
                            (tx + dx) * 16,
                            (ty + dy) * 16,
                        );
                        occupied[((ty + dy) * m.tw + tx + dx) as usize] = true;
                    }
                }
            } else {
                let variant = (hash2(tx, ty, 8) * 8.0) as u8;
                tile(z, &format!("ruin_{variant}"), tx * 16, ty * 16);
                occupied[idx] = true;
            }
        }
    }
    // The small fringe cells are eroded edges, not cut square tile silhouettes.
    for y in 0..z.ph {
        for x in 0..z.pw {
            let i = (y * z.pw + x) as usize;
            if mask[i] == mat::SLAB && !matches!(m.at(x / 16, y / 16), K::Slab | K::Altar) {
                let inset = (x % 16).min(15 - x % 16).min((y % 16).min(15 - y % 16));
                z.gset(
                    x,
                    y,
                    if inset < 1 {
                        pal::STONE[2]
                    } else {
                        pal::STONE[3]
                    },
                );
            }
        }
    }
}

/// Lay staggered stepping stones through the route, with broken corners and worn seams.
fn trails(z: &mut Zone, mask: &[u8]) {
    for y in 0..z.ph {
        for x in 0..z.pw {
            if mask[(y * z.pw + x) as usize] != mat::PATH {
                continue;
            }
            let row = y.div_euclid(15);
            let xx = x + (row % 2) * 13;
            let (bx, by) = (xx.div_euclid(27), row);
            let (lx, ly) = (xx.rem_euclid(27), y.rem_euclid(15));
            let chipped = hash2(bx, by, 113) > 0.6 && lx + ly < 5;
            if lx > 2 && ly > 2 && !chipped && hash2(bx, by, 110) > 0.1 {
                let c = if ly == 3 && lx < 15 && hash2(bx, by, 118) > 0.6 {
                    pal::STONE[4]
                } else if (ly == 14 || lx == 26) && hash2(bx, by, 116) > 0.65 {
                    pal::STONE[2]
                } else {
                    pal::STONE[3]
                };
                z.gset(x, y, c);
            }
        }
    }
}

/// Break a few slabs and grow moss from their seams, keeping large floor fields quiet.
fn weathering(z: &mut Zone, mask: &[u8]) {
    for gy in 1..z.ph / 37 {
        for gx in 1..z.pw / 41 {
            if hash2(gx, gy, 160) < 0.88 {
                continue;
            }
            let x = gx * 41 + (hash2(gx, gy, 161) * 18.0) as i32;
            let y = gy * 37 + (hash2(gx, gy, 162) * 16.0) as i32;
            if !matches!(mask[(y * z.pw + x) as usize], mat::SLAB | mat::PATH) {
                continue;
            }
            for step in 0..9 {
                let xx = x + step / 3;
                let yy = y + step;
                if matches!(mask[(yy * z.pw + xx) as usize], mat::SLAB | mat::PATH) {
                    z.gset(xx, yy, pal::STONE[2]);
                    if step > 4 {
                        z.gset(xx - 1, yy, pal::GRASS[1]);
                    }
                }
            }
        }
    }
}

/// Paint shallow shelves, undercut banks and the grass fringe at the waterline.
fn banks(z: &mut Zone, mask: &[u8]) {
    for y in 6..z.ph - 6 {
        for x in 6..z.pw - 6 {
            let i = (y * z.pw + x) as usize;
            let g = mask[i];
            if !matches!(g, mat::GRASS | mat::SHALLOW) {
                continue;
            }
            let wet = |xx: i32, yy: i32| mat::water(mask[(yy * z.pw + xx) as usize]);
            // Cut limestone shoulders follow the water mask without concealing deep tiles.
            let basin = !(256..608).contains(&y) || (352..418).contains(&y);
            if g == mat::GRASS && basin {
                let near = (1..=6).find(|&d| {
                    [(-d, 0), (d, 0), (0, -d), (0, d)]
                        .iter()
                        .any(|&(dx, dy)| wet(x + dx, y + dy))
                });
                if let Some(d) = near {
                    let joint = (y + (x / 24) * 7).rem_euclid(31) == 0;
                    let c = if d == 1 {
                        pal::STONE[0]
                    } else if d < 3 || joint {
                        pal::STONE[1]
                    } else if (768..912).contains(&x) && (608..736).contains(&y) {
                        if wet(x + d, y) {
                            if y < 650 || (662..700).contains(&y) || y > 718 { pal::STONE[1] } else { pal::GRASS[1] }
                        } else if wet(x - d, y) {
                            if (614..643).contains(&y) || (654..691).contains(&y) || (710..733).contains(&y) { pal::STONE[4] }
                            else { pal::GRASS[1] }
                        } else {
                            pal::STONE[2]
                        }
                    } else if wet(x + d, y) || wet(x, y + d) {
                        pal::STONE[4]
                    } else {
                        pal::STONE[3]
                    };
                    z.gset(x, y, c);
                    continue;
                }
            }
            if g == mat::GRASS
                && [(-2, 0), (2, 0), (0, -2), (0, 2)]
                    .iter()
                    .any(|&(dx, dy)| wet(x + dx, y + dy))
            {
                z.gset(
                    x,
                    y,
                    if wet(x, y + 1) {
                        pal::TEAL[0]
                    } else {
                        pal::TEAL[2]
                    },
                );
            } else if g == mat::SHALLOW && !wet(x, y - 2) {
                z.gset(x, y, pal::WATER[1]);
            }
        }
    }
}

/// Give both the terrace and island a visible upper lip and south-facing depth.
fn cliffs(z: &mut Zone, m: &Map, mask: &mut [u8]) {
    // The terrace is two tiles high; large masonry courses keep it distinct from raw rock.
    for y in 20 * 16..22 * 16 {
        for x in 4 * 16..81 * 16 {
            let k = m.at(x / 16, y / 16);
            if matches!(k, K::Cliff | K::Niche) {
                let ly = y - 20 * 16;
                let course = ly / 12;
                let joint = (x + course * 19).rem_euclid(43);
                let c = if ly < 3 {
                    pal::GRASS[1]
                } else if ly < 5 {
                    pal::STONE[4]
                } else if ly % 12 == 5 || joint == 0 {
                    pal::STONE[0]
                } else if ly % 12 == 6 || joint == 1 {
                    pal::STONE[2]
                } else {
                    pal::STONE[1]
                };
                z.gset(x, y, c);
                if ly > 25 {
                    z.gset(x, y, pal::TEAL[0]);
                }
            } else if k == K::Stairs {
                let ly = (y - 20 * 16) % 5;
                z.gset(
                    x,
                    y,
                    if ly == 0 {
                        pal::STONE[5]
                    } else if ly > 2 {
                        pal::STONE[2]
                    } else {
                        pal::STONE[4]
                    },
                );
                if x % 16 < 2 {
                    z.gset(x, y, pal::STONE[2]);
                }
            }
        }
    }
    // A narrow moss seam roots the retaining wall into the meadow.
    for x in 4 * 16..81 * 16 {
        if m.at(x / 16, 21) == K::Cliff {
            let h = 2 + (hash2(x / 5, 0, 16) * 4.0) as i32;
            for dy in 0..h {
                z.gset(x, 22 * 16 + dy, pal::GRASS[1]);
            }
        }
    }
    // Unequal buttressed masses keep natural cliffs distinct from laid retaining walls.
    let masses = [
        0, 61, 143, 196, 289, 347, 422, 531, 599, 671, 751, 814, 866, 949, 1017, 1129, 1191, 1287,
        1344,
    ];
    let faces = [0, 61, 196, 347, 422, 599, 751, 866, 1017, 1129, 1287, 1344];
    for x in 0..z.pw {
        let tx = x / 16;
        let Some(bottom) = (0..m.th).rev().find(|&y| m.at(tx, y) != K::Void) else {
            continue;
        };
        let top = (bottom + 1) * 16;
        // Narrow west-facing tile steps need a shallow capped skirt, not a
        // tall isolated slab hanging against the sky in the opening frame.
        let run_left = (0..=tx)
            .rev()
            .take_while(|&xx| (0..m.th).rev().find(|&yy| m.at(xx, yy) != K::Void) == Some(bottom))
            .count() as i32;
        let run_right = (tx + 1..m.tw)
            .take_while(|&xx| (0..m.th).rev().find(|&yy| m.at(xx, yy) != K::Void) == Some(bottom))
            .count() as i32;
        let west_depth = ((run_left + run_right) * 16 * 2 / 3).clamp(10, 48);
        let mass = masses
            .windows(2)
            .position(|s| x >= s[0] && x < s[1])
            .unwrap_or(0);
        let footprint_x = x - masses[mass];
        let footprint_width = masses[mass + 1] - masses[mass];
        let face = faces.windows(2).find(|s| x >= s[0] && x < s[1]).unwrap();
        let lx = x - face[0];
        let width = face[1] - face[0];
        let ledge = [17, 28, 21, 34, 19][mass % 5];
        // The landing exposes the bottom of its rock shelves against the cloud sea.
        let landing = mass <= 5 && top >= 704;
        let h = if landing {
            [40, 53, 43, 55, 44][mass % 5]
        } else {
            [58, 72, 51, 64, 78, 55, 69][mass % 7]
        };
        for dy in 0..h {
            let y = top + dy;
            if y >= z.ph {
                break;
            }
            let i = (y * z.pw + x) as usize;
            if mask[i] != mat::VOID {
                continue;
            }
            if landing && dy > h - 8 && (footprint_x < (dy - h + 8) * 2 || footprint_x > footprint_width - (dy - h + 8)) {
                continue;
            }
            // Keep the existing decorative material footprint, but carve its visible
            // rock into sloping shoulders and unequal, attached lower shelves.
            let tip = match mass % 3 {
                0 => h - 5,
                1 => h - 4 - lx * 4 / width,
                _ => h - 8 + lx * 4 / width,
            };
            let west_tip = if x < 112 && top < 704 {
                west_depth - (x % 16 - 7).abs() / 3
            } else {
                h
            };
            let shoulder = (dy - (tip.min(west_tip) - 12)).max(0) / 4;
            let inset = if lx < width / 2 {
                shoulder / 2
            } else {
                shoulder
            }.min(3);
            let hidden = dy > tip.min(west_tip) || lx < inset || lx >= width - inset;
            // The shadowed east plane leans back as it rises, so each face reads as a chamfered mass.
            let side = lx > width * 3 / 4 - dy / 4;
            let shelf = ledge + (lx / 13).min(3) - (mass as i32 % 3);
            // Turf spills over the lit lip in a few broad, low overhangs, not a dashed edge.
            let seg = x.div_euclid(29);
            let along = x.rem_euclid(29);
            let drape = if hash2(seg, mass as i32, 31) > 0.7 { 2 + (along.min(28 - along) / 4).min(2) } else { 2 };
            // The upper-left plane of each mass faces the light as one broad wedge, one step lighter.
            let facet = dy < shelf && lx * 2 + dy < width / 2 + 8;
            let c = if hidden {
                HOLE
            } else if dy < drape {
                pal::GRASS[1]
            } else if dy < 4 {
                pal::STONE[4]
            } else if dy < 7 {
                pal::EARTH[2]
            } else if side {
                pal::STONE[0]
            } else if facet {
                pal::STONE[2]
            } else if dy < shelf {
                pal::STONE[1]
            } else if dy < shelf + 2 && lx < width * 2 / 3 {
                pal::STONE[0]
            } else if dy < shelf + 5 && lx < width * 2 / 3 {
                pal::STONE[2]
            } else {
                pal::STONE[1]
            };
            // A displaced fissure terminates inside the mass, never a full-height rib.
            let fracture =
                !hidden && dy > shelf + 9 && dy < h - 11 && lx == width / 3 + (dy - shelf) / 8;
            z.gset(x, y, if fracture { pal::STONE[0] } else { c });
            mask[i] = mat::EDGE;
        }
    }
    // Sparse roots grip the lit lip and follow the rock, with small branching tips.
    for (cx, length) in [
        (103, 61),
        (183, 15),
        (314, 24),
        (508, 18),
        (643, 27),
        (786, 80),
        (904, 23),
        (1062, 19),
        (1217, 26),
    ] {
        let bottom = (0..m.th)
            .rev()
            .find(|&y| m.at(cx / 16, y) != K::Void)
            .unwrap();
        let top = (bottom + 1) * 16;
        let hanging = cx == 103 || cx == 786;
        for dy in 1..length {
            let x = cx + dy.max(0) / 8;
            let y = top + dy;
            if y >= z.ph {
                break;
            }
            for dx in 0..if dy > length - 10 { 1 } else { 2 } {
                let i = (y * z.pw + x + dx) as usize;
                if mask[i] == mat::EDGE || hanging && mask[i] == mat::VOID {
                    z.gset(
                        x + dx,
                        y,
                        if dx == 0 { pal::TEAL[0] } else { pal::EARTH[1] },
                    );
                }
            }
            if dy == length / 2 {
                for branch in 1..5 {
                    let (bx, by) = (x - branch, y + branch / 2);
                    if by < z.ph && mask[(by * z.pw + bx) as usize] == mat::EDGE {
                        z.gset(bx, by, pal::TEAL[0]);
                    }
                }
            }
        }
    }
    // Bridge deck courses run across the river; the cap and front face give them depth.
    for (bx, by) in [(41 * 16, 16 * 16), (49 * 16, 36 * 16)] {
        for y in by..by + 32 {
            for x in bx..bx + 112 {
                let lx = x - bx;
                let ly = y - by;
                let c = if ly < 3 {
                    pal::STONE[5]
                } else if ly > 27 {
                    pal::STONE[2]
                } else if (lx + (ly / 14) * 14) % 28 == 0 || ly == 15 {
                    pal::STONE[2]
                } else {
                    pal::STONE[4]
                };
                z.gset(x, y, c);
            }
        }
        for x in bx + 6..bx + 106 {
            if (x - bx) / 22 == 2 {
                continue;
            }
            z.gset(x, by - 2, pal::STONE[4]);
            z.gset(x, by - 1, pal::STONE[2]);
        }
    }
    // Niches are carved stone, with an unlit prism motif shared by the altar.
    for tx in [18, 32] {
        let (cx, cy) = (tx * 16 + 8, 21 * 16 + 5);
        for dy in -10i32..=9 {
            for dx in -9i32..=9 {
                if dx.abs() + (-dy - 5).max(0) > 9 {
                    continue;
                }
                let c = if dx.abs() > 6 || dy == 9 {
                    pal::STONE[2]
                } else {
                    pal::STONE[0]
                };
                z.gset(cx + dx, cy + dy, c);
            }
        }
        for dy in -7i32..=6 {
            for dx in [-4, 0, 4] {
                z.gset(cx + dx, cy + dy, pal::STONE[3]);
            }
        }
    }
    // Ivy descends from the terrace rim in a few rooted groups.
    for (cx, length) in [(123, 17), (173, 10), (535, 23), (877, 15), (1241, 20)] {
        for dy in 0..length {
            let x = cx + dy / 7;
            z.gset(x, 320 + dy, pal::TEAL[0]);
            if dy % 4 < 2 {
                for dx in -2..=2 {
                    z.gset(x + dx, 320 + dy, pal::GRASS[2]);
                }
            }
        }
    }
    // Thin visible side rims, without drawing disconnected rocks in the sky.
    for y in 0..z.ph {
        for x in 1..z.pw - 1 {
            if mask[(y * z.pw + x) as usize] != mat::GRASS {
                continue;
            }
            if m.at((x - 2).div_euclid(16), y / 16) == K::Void {
                z.gset(x, y, pal::GRASS[3]);
            }
            if m.at((x + 2) / 16, y / 16) == K::Void {
                z.gset(x, y, pal::GRASS[0]);
            }
        }
    }
}

/// Add sparse, clustered ground marks with plenty of uninterrupted meadow between them.
fn details(z: &mut Zone, mask: &[u8]) {
    for gy in 0..z.ph / 13 {
        for gx in 0..z.pw / 19 {
            if hash2(gx, gy, 70) < 0.88 {
                continue;
            }
            let x = gx * 19 + 3 + (hash2(gx, gy, 71) * 11.0) as i32;
            let y = gy * 13 + 3 + (hash2(gx, gy, 72) * 7.0) as i32;
            if mask[(y * z.pw + x) as usize] != mat::GRASS {
                continue;
            }
            for (dx, dy, c) in [
                (-2, 0, pal::GRASS[1]),
                (-1, 0, pal::GRASS[1]),
                (0, 0, pal::GRASS[1]),
                (0, -1, pal::GRASS[3]),
                (1, -1, pal::GRASS[3]),
            ] {
                if mask[((y + dy) * z.pw + x + dx) as usize] == mat::GRASS {
                    z.gset(x + dx, y + dy, c);
                }
            }
        }
    }
}

/// Set the prism on a raised octagonal plinth with an inlaid compass, not an emissive floor.
fn altar(z: &mut Zone) {
    let (cx, cy) = (ALTAR.0 as i32, ALTAR.1 as i32);
    for y in -24i32..=25 {
        for x in -32i32..=32 {
            let d = x
                .abs()
                .max(y.abs() + 8)
                .max((x.abs() + y.abs() + 8) * 2 / 3);
            if d > 32 {
                continue;
            }
            let c = if y > 19 || d > 29 {
                pal::STONE[2]
            } else if d > 26 {
                pal::STONE[5]
            } else {
                pal::STONE[3]
            };
            z.gset(cx + x, cy + y, c);
        }
    }
    for y in -15i32..=15 {
        for x in -20i32..=20 {
            let d = x.abs() + y.abs() * 4 / 3;
            if (18..=20).contains(&d) {
                z.gset(cx + x, cy + y - 2, pal::STONE[1]);
            }
            if d < 5 {
                z.gset(cx + x, cy + y - 2, pal::STONE[5]);
            }
        }
    }
}
