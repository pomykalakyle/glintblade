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
    round_coast(z, m, &mut mask);
    cliffs(z, m, &mut mask);
    if let Some(s) = sheet("bloomwild_terrace") {
        terrace_art(z, m, s);
    }
    details(z, &mask);
    sheltered_turf(z, &mask);
    sunlit_lawn(z, &mask);
    trampled_verge(z, &mask);
    // Optional native material art replaces only floor interiors. Protected banks, stairs,
    // altar geometry and the final island rim keep their existing authored shapes.
    if let Some(s) = sheet("bloomwild_materials") {
        material_art(z, &mask, s);
    }
    floor_details(z, m, &mask);
    opening_steps(z, &mut mask);
    if m.k.contains(&K::Altar) {
        altar(z);
    }
    outer_rims(z, m, &mask);
    if let Some(s) = sheet("bloomwild_cliff") {
        perimeter_art(z, m, &mask, s);
    }
    coast_side_returns(z, &mut mask);
    // Native images supply the rock and masonry, while the actual map owns their contacts.
    // Apply these last: a facade must not erase the coping, turf or its own foundation.
    banks(z, &mask);
    terrain_contacts(z, m, &mask);
    terrace_endcaps(z, m, &mut mask);
    mask
}

/// Singular remnants explain the old garden: a broken medallion at its shrine,
/// fluting from the temple collapse and low damp stones at two bridge approaches.
/// These are flush surface details, clipped to real dry ground before architecture.
fn floor_details(z: &mut Zone, m: &Map, mask: &[u8]) {
    if m.marks.is_empty() { return; }
    let Some(s) = sheet("bloomwild_floor_details") else { return };
    for (name, x, y) in [
        ("broken_mosaic", 408, 176),
        ("fluted_chips", 1131, 539),
        ("ivy", 1003, 433),
        ("damp_apron", 642, 274),
        ("ivy", 918, 592),
    ] {
        let Some(a) = s.anim(name) else { continue };
        let cell = s.cell(a, 0);
        let anchor = s.anchor_of(a);
        for py in 0..cell.h {
            for px in 0..cell.w {
                let (wx, wy) = (x + px - anchor.0, y + py - anchor.1);
                if wx < 0 || wy < 0 || wx >= z.pw || wy >= z.ph { continue; }
                let i = (wy * z.pw + wx) as usize;
                if !matches!(mask[i], mat::GRASS | mat::PATH | mat::SLAB) || z.ground[i] == HOLE { continue; }
                let p = cell.at(px, py);
                if p.a {
                    z.ground[i] = p.c;
                    z.gset_surface(wx, wy, cell, px, py, false, 0.0);
                }
            }
        }
    }
}

/// The painted shore owns movement at sub-tile corners. Decorative rock below the
/// floating island remains void; a cut in the actual top silhouette remains void too.
pub(super) fn physical_surface(z: &Zone, mask: &[u8]) -> Vec<crate::world::Tile> {
    use crate::world::Tile;
    mask.iter()
        .enumerate()
        .map(|(i, &g)| match g {
            mat::GRASS | mat::PATH | mat::SLAB | mat::BRIDGE | mat::STAIRS => {
                if z.ground[i] == HOLE {
                    Tile::Void
                } else {
                    Tile::Floor
                }
            }
            mat::DEEP => Tile::Deep,
            mat::SHALLOW => Tile::Shallow,
            mat::CLIFF | mat::FALLS => Tile::Wall,
            _ => Tile::Void,
        })
        .collect()
}

/// Native masonry follows the authored terrace facade while keeping all openings intact.
fn terrace_art(z: &mut Zone, m: &Map, s: &crate::gfx::Sheet) {
    if (s.cell_w, s.cell_h) != (128, 32) {
        return;
    }
    let Some(anim) = s.anim("body") else { return };
    // The source's 4px left / 6px right end caps are standalone module borders. Repeating those
    // as part of a long facade made enormous pale posts every 128px. Join only its
    // native inner courses; real corners and stair jambs are drawn from the map below.
    for x in 4_i32 * 16..81 * 16 {
        let segment = (x - 64).div_euclid(118);
        let variant = (hash2(segment, 0, 708) * anim.frames as f32) as usize % anim.frames;
        let cell = s.cell(anim, variant);
        for py in 0..32 {
            let y = 20 * 16 + py;
            // The original stair, glyph niches and falling-water outlets stay exposed.
            if m.at(x / 16, y / 16) != K::Cliff {
                continue;
            }
            let pixel = cell.at((x - 64).rem_euclid(118) + 4, py);
            if pixel.a {
                z.gset(x, y, pixel.c);
                z.gset_surface(x, y, cell, (x - 64).rem_euclid(118) + 4, py, true, (32 - py) as f32);
            }
        }
    }
}

/// Round native land/air contacts around the authored tile silhouette. The four
/// neighboring tile centers define a compact 4–7px corner, rather than a staircase.
/// Wet/structural materials retain their exact source contacts.
fn round_coast(z: &mut Zone, m: &Map, mask: &mut [u8]) {
    for y in 0..z.ph {
        for x in 0..z.pw {
            let i = (y * z.pw + x) as usize;
            if !matches!(mask[i], mat::VOID | mat::GRASS | mat::PATH | mat::SLAB) {
                continue;
            }
            let (gx, gy) = ((x as f32 + 0.5 - 8.0) / 16.0, (y as f32 + 0.5 - 8.0) / 16.0);
            let (ix, iy) = (gx.floor() as i32, gy.floor() as i32);
            let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
            let mut land = 0.0;
            let mut air = 0.0;
            let mut protected = false;
            for (dx, dy, weight) in [
                (0, 0, (1.0 - fx) * (1.0 - fy)),
                (1, 0, fx * (1.0 - fy)),
                (0, 1, (1.0 - fx) * fy),
                (1, 1, fx * fy),
            ] {
                match m.at(ix + dx, iy + dy) {
                    K::Grass | K::Tall | K::Path | K::Slab | K::Altar => land += weight,
                    K::Void => air += weight,
                    _ => protected = true,
                }
            }
            if protected || land == 0.0 || air == 0.0 {
                continue;
            }
            if land < 0.5 {
                mask[i] = mat::VOID;
                z.ground[i] = HOLE;
            } else if mask[i] == mat::VOID {
                mask[i] = mat::GRASS;
                z.ground[i] = pal::GRASS[2];
            }
        }
    }
}

/// Native top silhouettes own the facade geometry. Rounded corners, not tile bounds,
/// determine where each rock column starts; vertical faces are a separate void material.
fn raster_face_runs(z: &Zone, mask: &[u8]) -> Vec<(i32, i32, i32)> {
    let mut runs: Vec<(i32, i32, i32)> = Vec::new();
    for y in 0..z.ph - 1 {
        for x in 0..z.pw {
            let i = (y * z.pw + x) as usize;
            if !matches!(mask[i], mat::GRASS | mat::PATH | mat::SLAB)
                || z.ground[i] == HOLE
                || !matches!(mask[i + z.pw as usize], mat::VOID | mat::EDGE)
            {
                continue;
            }
            if let Some(last) = runs.last_mut()
                && last.1 == x
                && last.2 == y + 1
            {
                last.1 = x + 1;
            } else {
                runs.push((x, x + 1, y + 1));
            }
        }
    }
    runs
}

/// Every exposed south boundary, including ledges above recessed coastline.
/// Looking only for the last occupied tile in a column misses those upper shelves.
fn exposed_face_runs(m: &Map, dry_only: bool) -> Vec<(i32, i32, i32)> {
    let mut runs: Vec<(i32, i32, i32)> = Vec::new();
    for ty in 0..m.th {
        for tx in 0..m.tw {
            let k = m.at(tx, ty);
            if k == K::Void
                || m.at(tx, ty + 1) != K::Void
                || dry_only && matches!(k, K::Deep | K::Shallow | K::Falls)
            {
                continue;
            }
            let (left, right, edge) = (tx * 16, (tx + 1) * 16, (ty + 1) * 16);
            if let Some(last) = runs.last_mut()
                && last.1 == left
                && last.2 == edge
            {
                last.1 = right;
            } else {
                runs.push((left, right, edge));
            }
        }
    }
    runs
}

#[cfg(test)]
fn south_face_runs(m: &Map) -> Vec<(i32, i32, i32)> {
    exposed_face_runs(m, true)
}

/// Keep one geological coordinate field across tile-height changes. Native source pixels
/// retain their size; only their integer origin follows the nearby connected shore.
fn connected_face_lip(runs: &[(i32, i32, i32)], x: i32, edge: i32) -> i32 {
    let mut total = 0;
    let mut weight = 0;
    for &(left, right, other_edge) in runs {
        if (other_edge - edge).abs() > 48 {
            continue;
        }
        for xx in (x - 20).max(left)..(x + 21).min(right) {
            let w = 21 - (xx - x).abs();
            total += other_edge * w;
            weight += w;
        }
    }
    if weight == 0 {
        edge
    } else {
        (total + weight / 2) / weight
    }
}

/// A connected shoulder can include many one-pixel rounded lip runs. Its end taper
/// follows their shared outer contour, rather than making each tiny run a hanging tooth.
fn connected_face_limits(
    runs: &[(i32, i32, i32)],
    left: i32,
    right: i32,
    edge: i32,
) -> ((i32, i32), (i32, i32)) {
    let (mut l, mut le, mut r, mut re) = (left, edge, right, edge);
    loop {
        let next = runs
            .iter()
            .filter(|&&(a, b, e)| a < l && b >= l && (e - le).abs() <= 48)
            .min_by_key(|&&(a, _, e)| ((e - le).abs(), a));
        let Some(&(a, _, e)) = next else { break };
        l = a;
        le = e;
    }
    loop {
        let next = runs
            .iter()
            .filter(|&&(a, b, e)| a <= r && b > r && (e - re).abs() <= 48)
            .min_by_key(|&&(_, b, e)| ((e - re).abs(), -b));
        let Some(&(_, b, e)) = next else { break };
        r = b;
        re = e;
    }
    ((l, le), (r, re))
}

/// Overlapping native crags share world-space phases, so a coastline corner no longer
/// restarts a different rock asset or cuts a perfectly vertical module seam.
fn perimeter_art(z: &mut Zone, _m: &Map, mask: &[u8], s: &crate::gfx::Sheet) {
    if (s.cell_w, s.cell_h) != (128, 96) {
        return;
    }
    let Some(anim) = s.anim("body") else { return };
    let (_, ay) = s.anchor_of(anim);
    let runs = raster_face_runs(z, mask);
    // Clear all dry backing, including columns whose top was subsequently eroded.
    // Clearing only the new face runs leaves skinny old rock teeth under absent turf.
    // Waterfall columns retain their original backing and every true surface stays intact.
    for x in 0..z.pw {
        let mut wet_support = false;
        for y in 0..z.ph {
            let i = (y * z.pw + x) as usize;
            if mask[i] == mat::EDGE {
                if !wet_support {
                    z.ground[i] = HOLE;
                }
            } else if mask[i] != mat::VOID {
                wet_support = mat::water(mask[i]);
            }
        }
    }
    let mut origin = -16;
    let mut variant = 0;
    while origin < z.pw {
        let cell = s.cell(anim, variant % anim.frames);
        for &(left, right, edge) in &runs {
            for x in left.max(origin)..right.min(origin + cell.w) {
                let lip = connected_face_lip(&runs, x, edge);
                let ((outer_left, left_edge), (outer_right, right_edge)) =
                    connected_face_limits(&runs, left, right, edge);
                // A narrow projecting shoulder cannot carry the same deep crag as a
                // broad headland. Share a shallow lower contour across its adjacent
                // rounded lips, rather than hanging a new dark tooth at every tile step.
                let support: i32 = runs
                    .iter()
                    .filter(|&&(_, _, e)| (e - edge).abs() <= 6)
                    .map(|&(l, r, _)| ((x + 49).min(r) - (x - 48).max(l)).max(0))
                    .sum();
                let depth = (18 + support * support / 100).min(96);
                let lower_contour = lip + depth - (x.rem_euclid(53) - 24).abs() / 8;
                for y in edge..(edge + 96).min(z.ph) {
                    let i = (y * z.pw + x) as usize;
                    if !matches!(mask[i], mat::EDGE | mat::VOID) {
                        break;
                    }
                    let py = y - lip + ay;
                    if py < 0 || py >= cell.h {
                        continue;
                    }
                    if mask[i] != mat::EDGE {
                        continue;
                    }
                    // At a genuinely open end, bevel the last few columns into a rock
                    // shoulder. A vertical rectangular cut used to expose the source crop.
                    if y > left_edge + 10 + (x - outer_left) * 2
                        || y > right_edge + 10 + (outer_right - 1 - x) * 2
                        || y > lower_contour
                    {
                        continue;
                    }
                    let pixel = cell.at(x - origin, py);
                    if pixel.a {
                        z.ground[i] = pixel.c;
                        z.gset_surface(x, y, cell, x - origin, py, true, (lower_contour - y).max(0) as f32);
                    }
                }
            }
        }
        origin += [105, 113, 99, 109][variant % 4];
        variant += 1;
    }
    cliff_corner_returns(z, &runs, mask);
}

/// Narrow oblique rock shoulders give vertical shorelines the same depth as their
/// south-facing facades. The top silhouette owns the attachment; every return remains
/// non-walkable EDGE, including where a rounded corner cuts into an original floor tile.
fn coast_side_returns(z: &mut Zone, mask: &mut [u8]) {
    let (pw, ph) = (z.pw, z.ph);
    let top: Vec<bool> = mask
        .iter()
        .enumerate()
        .map(|(i, &g)| matches!(g, mat::GRASS | mat::PATH | mat::SLAB) && z.ground[i] != HOLE)
        .collect();
    let original = mask.to_vec();
    let original_opaque: Vec<bool> = z.ground.iter().map(|&c| c != HOLE).collect();
    let mut nearest = vec![u8::MAX; mask.len()];
    for y in 0..ph {
        for x in 1..pw - 1 {
            let i = (y * pw + x) as usize;
            if !top[i] {
                continue;
            }
            for side in [-1, 1] {
                let neighbor = (y * pw + x + side) as usize;
                if top[neighbor]
                    || !matches!(original[neighbor], mat::VOID | mat::EDGE | mat::GRASS)
                {
                    continue;
                }
                // Soil follows the real lawn edge instead of outlining every coast in pale stone.
                if original[i] == mat::GRASS {
                    z.ground[i] = z.ground[i].lerp(
                        crate::gfx::rgb(0x65715e),
                        if side < 0 { 0.19 } else { 0.32 },
                    );
                }
                let width = 10;
                'projection: for depth in 1..=width {
                    let rx = x + side * depth;
                    if rx < 0 || rx >= pw {
                        break;
                    }
                    // Fill the whole side volume between the real shore and its oblique
                    // projection. A single offset pixel per column made detached bars
                    // below short ledges, and left air between the wall cap and its foot.
                    for vertical in 0..=1 + depth * 3 / 2 {
                        let ry = y + vertical;
                        if ry >= ph {
                            break;
                        }
                        let r = (ry * pw + rx) as usize;
                        if top[r]
                            || matches!(
                                original[r],
                                mat::DEEP
                                    | mat::SHALLOW
                                    | mat::FALLS
                                    | mat::BIGFALLS
                                    | mat::CLIFF
                                    | mat::BRIDGE
                                    | mat::STAIRS
                            )
                        {
                            break 'projection;
                        }
                        if !matches!(original[r], mat::VOID | mat::GRASS | mat::EDGE)
                            || original_opaque[r]
                            || nearest[r] <= depth as u8
                        {
                            continue;
                        }
                        // Existing opaque native facets stay intact. The solid return
                        // may sit behind transparent native pixels only inside its actual
                        // narrow volume; source gaps elsewhere remain open sky.
                        nearest[r] = depth as u8;
                        let along = ry.rem_euclid(37);
                        let lit_width = 1 + along.min(36 - along) / 6;
                        let plane = if depth <= lit_width {
                            0
                        } else if depth <= 6 {
                            1
                        } else {
                            2
                        };
                        let west = [0x806876, 0x645867, 0x303c48];
                        let east = [0x645867, 0x303c48, 0x303c48];
                        let mut color =
                            crate::gfx::rgb(if side < 0 { west[plane] } else { east[plane] });
                        if side > 0 && depth > 6 {
                            color = color.scale(0.81);
                        }
                        if (ry + ry.div_euclid(37) * 7).rem_euclid(41) == 0 && depth > 2 {
                            color = color.scale(0.76);
                        }
                        z.ground[r] = color;
                        mask[r] = mat::EDGE;
                    }
                }
            }
        }
    }
}

/// An exposed masonry end is a real oblique face, not the front wall's rectangular
/// texture crop. Courses and coping wrap down the 10px return into its rock shoulder.
fn terrace_endcaps(z: &mut Zone, m: &Map, mask: &mut [u8]) {
    for tx in 0..m.tw {
        if !matches!(m.at(tx, 20), K::Cliff | K::Niche) {
            continue;
        }
        for side in [-1, 1] {
            if m.at(tx + side, 20) != K::Void {
                continue;
            }
            let end = if side < 0 { tx * 16 } else { (tx + 1) * 16 - 1 };
            for depth in 1..=10 {
                let x = end + side * depth;
                if x < 0 || x >= z.pw {
                    continue;
                }
                let offset = (depth * 4 + 2) / 5;
                for row in 0..32 {
                    let y = 320 + offset + row;
                    if y >= z.ph {
                        continue;
                    }
                    let i = (y * z.pw + x) as usize;
                    if !matches!(mask[i], mat::VOID | mat::EDGE) {
                        continue;
                    }
                    let source = sheet("bloomwild_terrace")
                        .and_then(|s| {
                            s.anim("body").map(|a| {
                                let c = s.cell(a, 0);
                                let p = c.at(4 + depth, row);
                                if p.a { p.c } else { pal::STONE[1] }
                            })
                        })
                        .unwrap_or(pal::STONE[1]);
                    let mut color = match row {
                        0 => pal::STONE[4],
                        1 => pal::STONE[3],
                        2 => pal::STONE[1],
                        29..=31 => pal::STONE[0],
                        _ => source.scale(if side < 0 { 1.08 } else { 0.73 }),
                    };
                    if depth == 10 && row > 2 {
                        color = color.scale(0.77);
                    }
                    if row > 28 && hash2(depth / 3, row / 2, 1372) > 0.52 {
                        color = pal::TEAL[0];
                    }
                    z.ground[i] = color;
                    mask[i] = mat::EDGE;
                }
            }
        }
    }
}

/// Short return planes connect two different-height rims. They stay entirely on the
/// non-walkable face, and never fill a native transparent gap or an eroded turf cut.
fn cliff_corner_returns(z: &mut Zone, runs: &[(i32, i32, i32)], mask: &[u8]) {
    for &(left, right, edge) in runs {
        for (end, side) in [(left, 1), (right - 1, -1)] {
            let outside = end - side;
            let Some(&(_, _, next_edge)) = runs
                .iter()
                .find(|&&(l, r, e)| outside >= l && outside < r && e >= edge + 4 && e <= edge + 48)
            else {
                continue;
            };
            for d in 0..6 {
                let x = end + side * d;
                if x < left || x >= right {
                    continue;
                }
                for y in edge..(next_edge + 13).min(z.ph) {
                    let i = (y * z.pw + x) as usize;
                    if mask[i] != mat::EDGE || z.ground[i] == HOLE {
                        continue;
                    }
                    let shade = if side > 0 {
                        0.86 + d as f32 * 0.018
                    } else {
                        0.70 + d as f32 * 0.035
                    };
                    z.ground[i] = z.ground[i].scale(shade);
                }
            }
        }
    }
}

/// Seat vertical surfaces below their own ground plane, never paint rock over walkable turf.
fn terrain_contacts(z: &mut Zone, m: &Map, mask: &[u8]) {
    let (pw, ph) = (z.pw, z.ph);
    let dry = |x: i32, y: i32| {
        x >= 0
            && x < pw
            && y >= 0
            && y < ph
            && matches!(
                mask[(y * pw + x) as usize],
                mat::GRASS | mat::PATH | mat::SLAB
            )
    };
    // A continuous warm coping is the top plane of the retaining wall. Its irregular
    // grass fringe connects to the upper lawn; the lower shadow attaches to the meadow.
    for x in 64..1296 {
        let k = m.at(x / 16, 20);
        if !matches!(k, K::Cliff | K::Niche) {
            continue;
        }
        let overhang = 1 + (hash2(x / 9, 0, 733) * 2.0) as i32;
        for dy in 0..5 {
            z.gset(
                x,
                320 + dy,
                match dy {
                    0 => {
                        if x.rem_euclid(37) < 2 {
                            pal::STONE[2]
                        } else {
                            pal::STONE[4]
                        }
                    }
                    1 => pal::STONE[3],
                    2 => pal::STONE[1],
                    _ => pal::STONE[0],
                },
            );
        }
        if m.at(x / 16, 19) != K::Void {
            for dy in 0..overhang {
                if dry(x, 319 - dy) {
                    z.gset(x, 319 - dy, pal::GRASS[1]);
                }
            }
        }
        for dy in 0..7 {
            let y = 352 + dy;
            if !dry(x, y) {
                continue;
            }
            let shade = [0.59, 0.69, 0.78, 0.86, 0.91, 0.95, 0.98][dy as usize];
            z.gset(x, y, z.gget(x, y).scale(shade));
            if dy < 2 && hash2(x / 7, 1, 734) > 0.44 {
                z.gset(x, y, pal::TEAL[0]);
            }
        }
    }
    // The two stair mouths are open passages, with the same pale sill as the coping.
    // Their walls terminate beside the actual traversable 48px stair, not across it.
    for x in 64..1296 {
        if m.at(x / 16, 20) != K::Stairs {
            continue;
        }
        for (y, c) in [
            (319, pal::STONE[4]),
            (320, pal::STONE[5]),
            (350, pal::STONE[3]),
            (351, pal::STONE[4]),
            (352, pal::STONE[3]),
        ] {
            z.gset(x, y, c);
        }
    }
    natural_cliff_contacts(z, m, mask);
    bridge_decks(z, mask);
}

/// Break the meadow/rock contact into rooted turf drapes and exposed native facets.
/// A continuous painted strip flattens the irregular source into a concrete slab.
fn natural_cliff_contacts(z: &mut Zone, _m: &Map, mask: &[u8]) {
    for (left, right, edge) in raster_face_runs(z, mask) {
        for x in left..right {
            let top = edge - 2;
            let source = z.gget(x, top);
            if top < 0 || source == HOLE || mask[(top * z.pw + x) as usize] != mat::GRASS {
                continue;
            }
            let patch = x.div_euclid(31);
            if hash2(patch, edge, 735) < 0.32 {
                continue;
            }
            let center = 8 + (hash2(patch, edge, 740) * 15.0) as i32;
            let half = 5 + (hash2(patch, edge, 741) * 9.0) as i32;
            let reach = (half - (x.rem_euclid(31) - center).abs()).max(0);
            let height = (reach * (3 + (hash2(patch, edge, 742) * 3.0) as i32) / half).min(5);
            for dy in 0..height {
                let y = edge + dy;
                if y >= z.ph || mask[(y * z.pw + x) as usize] != mat::EDGE || z.gget(x, y) == HOLE {
                    continue;
                }
                let tip = dy == height - 1;
                let earth = tip && hash2(patch, edge, 743) > 0.68;
                z.gset(
                    x,
                    y,
                    if earth {
                        crate::gfx::rgb(0x746452)
                    } else {
                        source
                            .lerp(pal::GRASS[1], 0.28)
                            .scale(if tip { 0.72 } else { 0.87 })
                    },
                );
            }
        }
        // Sparse short roots cling to existing rock; transparent source gaps stay sky.
        for group in left.div_euclid(53)..right.div_euclid(53) {
            if hash2(group, edge, 744) < 0.68 {
                continue;
            }
            let root_x = group * 53 + 11 + (hash2(group, edge, 745) * 27.0) as i32;
            if root_x < left + 4 || root_x >= right - 4 {
                continue;
            }
            let length = 4 + (hash2(group, edge, 746) * 5.0) as i32;
            for dy in 0..length {
                let x = root_x + dy / 4;
                let y = edge + dy;
                if y >= z.ph || mask[(y * z.pw + x) as usize] != mat::EDGE || z.gget(x, y) == HOLE {
                    continue;
                }
                z.gset(
                    x,
                    y,
                    crate::gfx::rgb(if dy < 2 { 0x64725c } else { 0x66584b }),
                );
            }
        }
    }
}

/// Stone slabs span from bank to bank; end courses form broad, level abutments.
fn bridge_decks(z: &mut Zone, mask: &[u8]) {
    for (bx, by) in [(41 * 16, 16 * 16), (49 * 16, 36 * 16)] {
        for y in by..by + 32 {
            for x in bx..bx + 112 {
                if mask[(y * z.pw + x) as usize] != mat::BRIDGE {
                    continue;
                }
                let (lx, ly) = (x - bx, y - by);
                let row = ly / 10;
                let joint = (lx + row * 17).rem_euclid(35);
                let edge = lx < 14 || lx >= 98;
                let c = if ly == 0 {
                    pal::STONE[5]
                } else if ly < 3 {
                    pal::STONE[3]
                } else if ly == 29 {
                    pal::STONE[5]
                } else if ly > 29 {
                    pal::STONE[2]
                } else if joint == 0 || ly == 12 || ly == 22 {
                    pal::STONE[2]
                } else if joint == 1 {
                    pal::STONE[3]
                } else if edge && (ly == 7 || ly == 24) {
                    pal::STONE[3]
                } else {
                    pal::STONE[4]
                };
                z.gset(x, y, c);
            }
        }
        // Worn approach stones taper into the existing path rather than ending at a
        // bright rectangular seam. Only land receives them; every water byte stays wet.
        for side in [-1, 1] {
            for d in 0..9 {
                let x = if side < 0 { bx - d - 1 } else { bx + 112 + d };
                for ly in 3 + d / 3..29 - d / 3 {
                    let y = by + ly;
                    let i = (y * z.pw + x) as usize;
                    if !matches!(mask[i], mat::GRASS | mat::PATH | mat::SLAB) {
                        continue;
                    }
                    if d < 3 || hash2(x / 3, y / 4, 736) > d as f32 / 11.0 {
                        z.gset(
                            x,
                            y,
                            if ly == 3 + d / 3 {
                                pal::STONE[3]
                            } else {
                                pal::STONE[4]
                            },
                        );
                    }
                }
            }
        }
    }
}

/// Stable world-space variants without repeating the immediately adjacent tile. Unlike per-frame
/// random selection this also matches across camera boundaries and headless stitched overviews.
fn material_variants(w: usize, h: usize, frames: usize, salt: u32) -> Vec<usize> {
    let mut chosen = vec![0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut v = (hash2(x as i32, y as i32, salt) * frames as f32) as usize % frames;
            if frames == 2 {
                v = (x + y) % 2;
            } else if frames > 2 {
                while (x > 0 && chosen[y * w + x - 1] == v)
                    || (y > 0 && chosen[(y - 1) * w + x] == v)
                {
                    v = (v + 1) % frames;
                }
            }
            chosen[y * w + x] = v;
        }
    }
    chosen
}

/// Stamp transparent or opaque native 32px material modules, clipped to their physical floor.
/// Each anim can set `coverage: 0..1` to leave quiet gaps; source transparency leaves individual
/// pixels of the previous floor intact. Colors are copied exactly, without filtering or blending.
fn material_art(z: &mut Zone, mask: &[u8], s: &crate::gfx::Sheet) {
    if (s.cell_w, s.cell_h) != (32, 32) {
        eprintln!("warning: bloomwild_materials needs native 32x32 cells");
        return;
    }
    let (tw, th) = ((z.pw as usize).div_ceil(32), (z.ph as usize).div_ceil(32));
    let preserve_fields = s
        .json
        .get("preserve_fields")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let meadow_light = preserve_fields.then(|| meadow_illumination(z, mask));
    for (name, material, salt) in [
        ("grass", mat::GRASS, 181),
        ("slab", mat::SLAB, 182),
        ("path", mat::PATH, 183),
    ] {
        let Some(anim) = s.anim(name) else { continue };
        let coverage = anim
            .json
            .get("coverage")
            .or_else(|| s.json.get("coverage"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0) as f32;
        let variants = material_variants(tw, th, anim.frames, salt);
        for ty in 0..th {
            for tx in 0..tw {
                if coverage == 0.0 || hash2(tx as i32, ty as i32, salt + 7) > coverage {
                    continue;
                }
                let cell = s.cell(anim, variants[ty * tw + tx]);
                for py in 0..32 {
                    for px in 0..32 {
                        let (x, y) = (tx as i32 * 32 + px, ty as i32 * 32 + py);
                        if x < 3 || y < 3 || x >= z.pw - 3 || y >= z.ph - 3 {
                            continue;
                        }
                        let i = (y * z.pw + x) as usize;
                        if mask[i] != material {
                            continue;
                        }
                        // Keep authored shore lips and terrace contacts continuous. Grass/path/slab
                        // boundaries may meet directly; only non-floor materials reserve a border.
                        if (1..=3).any(|d| {
                            [(-d, 0), (d, 0), (0, -d), (0, d)].iter().any(|&(dx, dy)| {
                                !matches!(
                                    mask[((y + dy) * z.pw + x + dx) as usize],
                                    mat::GRASS | mat::PATH | mat::SLAB
                                )
                            })
                        }) {
                            continue;
                        }
                        let p = cell.at(px, py);
                        if p.a {
                            z.ground[i] = if preserve_fields && material == mat::GRASS {
                                // Native texture stays crisp. Only the broad illumination plan
                                // has a soft, low-contrast penumbra instead of polygon cut-outs.
                                p.c.scale(meadow_light.as_ref().unwrap()[i])
                            } else {
                                p.c
                            };
                            z.gset_surface(x, y, cell, px, py, false, 0.0);
                        }
                    }
                }
            }
        }
    }
}

/// Filter the authored light fields, never the native albedo pixels. Grass-only
/// weights keep a pale road or a dark cliff out of the meadow's illumination.
fn meadow_illumination(z: &Zone, mask: &[u8]) -> Vec<f32> {
    let (w, h) = (z.pw as usize, z.ph as usize);
    let stride = w + 1;
    let mut sums = vec![0.0_f32; stride * (h + 1)];
    let mut counts = vec![0_u32; stride * (h + 1)];
    for y in 0..h {
        let (mut sum, mut count) = (0.0, 0);
        for x in 0..w {
            let i = y * w + x;
            if mask[i] == mat::GRASS {
                sum += (z.ground[i].luma() / pal::GRASS[2].luma()).clamp(0.72, 1.16);
                count += 1;
            }
            sums[(y + 1) * stride + x + 1] = sums[y * stride + x + 1] + sum;
            counts[(y + 1) * stride + x + 1] = counts[y * stride + x + 1] + count;
        }
    }
    let mut light = vec![1.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if mask[i] != mat::GRASS { continue; }
            let (l, r, t, b) = (x.saturating_sub(9), (x + 10).min(w), y.saturating_sub(9), (y + 10).min(h));
            let n = counts[b * stride + r] + counts[t * stride + l] - counts[t * stride + r] - counts[b * stride + l];
            if n == 0 { continue; }
            let sum = sums[b * stride + r] + sums[t * stride + l] - sums[t * stride + r] - sums[b * stride + l];
            // A handful of value steps belongs to the same pixel-art palette.
            light[i] = ((1.0 + (sum / n as f32 - 1.0) * 0.43) * 48.0).round() / 48.0;
        }
    }
    light
}

const SHELTERED_TURF: &[&[(i32, i32)]] = &[
    &[
        (30, 478),
        (86, 481),
        (92, 497),
        (95, 515),
        (96, 535),
        (98, 555),
        (101, 575),
        (104, 592),
        (103, 608),
        (97, 622),
        (86, 633),
        (30, 642),
    ],
    &[
        (172, 612),
        (188, 613),
        (206, 619),
        (226, 627),
        (248, 634),
        (262, 634),
        (272, 632),
        (292, 633),
        (307, 651),
        (305, 671),
        (285, 688),
        (250, 681),
        (228, 667),
        (194, 664),
        (176, 646),
        (172, 628),
    ],
    &[
        (366, 712),
        (380, 704),
        (398, 703),
        (418, 708),
        (432, 716),
        (428, 723),
        (378, 723),
    ],
    &[
        (168, 746),
        (178, 736),
        (200, 731),
        (226, 733),
        (250, 731),
        (270, 735),
        (286, 741),
        (286, 754),
        (168, 754),
    ],
    &[
        (414, 726),
        (424, 721),
        (446, 722),
        (458, 717),
        (489, 716),
        (497, 712),
        (531, 713),
        (546, 716),
        (571, 715),
        (580, 711),
        (612, 710),
        (626, 714),
        (638, 721),
        (648, 729),
        (656, 740),
        (414, 740),
    ],
];

/// Keep the island lip clear while repainting only open meadow pixels.
fn clear_meadow(z: &Zone, x: i32, y: i32) -> bool {
    if z.tile_at(x as f32, y as f32) != crate::world::Tile::Floor {
        return false;
    }
    let void =
        |xx: i32, yy: i32| z.tile(xx.div_euclid(16), yy.div_euclid(16)) == crate::world::Tile::Void;
    !(1..=3).any(|d| void(x - d, y) || void(x + d, y) || void(x, y - d) || void(x, y + d))
}

/// Match the painter's integer half-open scanline rule at one world pixel.
fn in_plane(points: &[(i32, i32)], x: i32, y: i32) -> bool {
    let mut inside = false;
    for i in 0..points.len() {
        let (ax, ay) = points[i];
        let (bx, by) = points[(i + 1) % points.len()];
        if ((ay <= y && by > y) || (by <= y && ay > y))
            && ax + (y - ay) * (bx - ax) / (by - ay) <= x
        {
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
                if mask[i] == mat::GRASS
                    && pal::GRASS[1..=3].contains(&z.ground[i])
                    && in_plane(points, x, y)
                    && clear_meadow(z, x, y)
                {
                    z.gset(x, y, pal::GRASS[1]);
                }
            }
        }
    }
}

/// Open landing turf clear of the tree and fin shadows, leading from the path to the broken pier.
const SUNLIT_LAWN: &[(i32, i32)] = &[
    (138, 668),
    (158, 670),
    (182, 679),
    (204, 684),
    (226, 694),
    (250, 700),
    (272, 712),
    (276, 726),
    (262, 735),
    (238, 740),
    (214, 737),
    (190, 741),
    (164, 738),
    (138, 742),
];

/// Lift the open landing lawn one step where daylight reaches it past the tree and ruin.
fn sunlit_lawn(z: &mut Zone, mask: &[u8]) {
    for y in 660..744 {
        for x in 130..284 {
            let i = (y * z.pw + x) as usize;
            if mask[i] == mat::GRASS
                && z.ground[i] == pal::GRASS[2]
                && in_plane(SUNLIT_LAWN, x, y)
                && clear_meadow(z, x, y)
            {
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
            if mask[i] != mat::GRASS || z.tile_at(x as f32, y as f32) != crate::world::Tile::Floor {
                continue;
            }
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
                z.gset(
                    x,
                    y,
                    if nd == nc + 1 {
                        pal::STONE[3]
                    } else {
                        pal::STONE[2]
                    },
                );
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

/// Paint dry banks against the actual curved water mask. Stone appears as pool coping or
/// broken remnants at the outlet; the stream has an eroded earth/turf bank, not nested pipes.
fn banks(z: &mut Zone, mask: &[u8]) {
    for y in 6..z.ph - 6 {
        for x in 6..z.pw - 6 {
            let i = (y * z.pw + x) as usize;
            let g = mask[i];
            if g != mat::GRASS {
                continue;
            }
            let wet = |xx: i32, yy: i32| mat::water(mask[(yy * z.pw + xx) as usize]);
            let Some(d) = (1..=6).find(|&d| {
                [(-d, 0), (d, 0), (0, -d), (0, d)]
                    .iter()
                    .any(|&(dx, dy)| wet(x + dx, y + dy))
            }) else {
                continue;
            };
            let pool = y < 254 && (584..816).contains(&x);
            let remnant = y >= 608 && hash2(y / 17, x / 24, 738) > 0.43;
            let edge = if wet(x + d, y) || wet(x, y + d) {
                pal::GRASS[1]
            } else {
                pal::TEAL[0]
            };
            let c = if pool && d <= 4 {
                [pal::STONE[1], pal::STONE[2], pal::STONE[4], pal::GRASS[1]][d as usize - 1]
            } else if remnant && d < 4 {
                if d == 1 {
                    pal::STONE[1]
                } else if wet(x - d, y) {
                    pal::STONE[3]
                } else {
                    pal::STONE[2]
                }
            } else if d == 1 {
                edge
            } else if d == 2 {
                pal::GRASS[1]
            } else if d < 5 && hash2(x / 5, y / 7, 739) > d as f32 * 0.17 {
                pal::EARTH[2].lerp(pal::GRASS[2], 0.45)
            } else {
                continue;
            };
            z.gset(x, y, c);
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
    let mut exposed = raster_face_runs(z, mask);
    exposed.extend(
        exposed_face_runs(m, false)
            .into_iter()
            .filter(|&(l, _, e)| {
                matches!(m.at(l / 16, e / 16 - 1), K::Deep | K::Shallow | K::Falls)
            }),
    );
    for (run_left, run_right, top) in exposed {
        for x in run_left..run_right {
            let west_depth = ((run_right - run_left) * 2 / 3).clamp(10, 48);
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
                    if mask[i] == mat::EDGE {
                        continue;
                    }
                    break;
                }
                if landing
                    && dy > h - 8
                    && (footprint_x < (dy - h + 8) * 2
                        || footprint_x > footprint_width - (dy - h + 8))
                {
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
                }
                .min(3);
                let hidden = dy > tip.min(west_tip) || lx < inset || lx >= width - inset;
                // The shadowed east plane leans back as it rises, so each face reads as a chamfered mass.
                let side = lx > width * 3 / 4 - dy / 4;
                let shelf = ledge + (lx / 13).min(3) - (mass as i32 % 3);
                // Turf spills over the lit lip in a few broad, low overhangs, not a dashed edge.
                let seg = x.div_euclid(29);
                let along = x.rem_euclid(29);
                let drape = if hash2(seg, mass as i32, 31) > 0.7 {
                    2 + (along.min(28 - along) / 4).min(2)
                } else {
                    2
                };
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
        let Some(bottom) = (0..m.th).rev().find(|&y| m.at(cx / 16, y) != K::Void) else {
            continue;
        };
        let top = (bottom + 1) * 16;
        for dy in 1..length {
            let x = cx + dy.max(0) / 8;
            let y = top + dy;
            if y >= z.ph {
                break;
            }
            for dx in 0..if dy > length - 10 { 1 } else { 2 } {
                let i = (y * z.pw + x + dx) as usize;
                if mask[i] == mat::EDGE {
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
    // Niches are carved stone, with an unlit prism motif shared by the altar.
    for tx in [18, 32] {
        if m.at(tx, 21) != K::Niche {
            continue;
        }
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
        if !matches!(m.at(cx / 16, 20), K::Cliff | K::Niche) {
            continue;
        }
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

#[cfg(test)]
mod material_tests {
    use super::*;
    use crate::gfx::{Rgb, Sheet};
    use crate::world::ZoneId;

    #[test]
    fn bounded_and_empty_gardens_do_not_paint_orphan_level_details_into_sky() {
        for bounded in [false, true] {
            let mut m = Map::parse();
            m.k.fill(K::Void);
            m.level.fill(0);
            m.marks.clear();
            if bounded {
                for (y, left, right) in [(4, 17, 19), (5, 15, 21), (6, 14, 22), (7, 15, 21)] {
                    for x in left..=right {
                        m.k[(y * m.tw + x) as usize] = K::Grass;
                    }
                }
            }
            let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
            for y in 0..m.th {
                for x in 0..m.tw {
                    z.set_tile(x, y, super::super::tile_of(m.at(x, y)));
                }
            }
            let mask = paint(&mut z, &m);
            for (i, &g) in mask.iter().enumerate() {
                if g == mat::VOID {
                    assert_eq!(
                        z.ground[i],
                        HOLE,
                        "no orphan altar, niche, ivy or root in sky at {},{}",
                        i as i32 % z.pw,
                        i as i32 / z.pw
                    );
                }
            }
            assert_eq!(mask.iter().any(|&g| g == mat::GRASS), bounded);
            assert_eq!(mask.iter().any(|&g| g == mat::EDGE), bounded);
        }
    }

    #[test]
    fn natural_cliff_contacts_leave_broken_native_rock_and_preserve_the_entire_surface() {
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        let mask = paint(&mut z, &m);
        let marker = Rgb::from_u8([40, 30, 50]);
        for (i, &g) in mask.iter().enumerate() {
            if g == mat::EDGE && z.ground[i] != HOLE {
                z.ground[i] = marker;
            }
        }
        let before = z.ground.clone();
        let surface = physical_surface(&z, &mask);
        natural_cliff_contacts(&mut z, &m, &mask);
        assert_eq!(
            physical_surface(&z, &mask),
            surface,
            "turf drapes never change physical ground or hazards"
        );
        for (i, &g) in mask.iter().enumerate() {
            if g != mat::EDGE || before[i] == HOLE {
                assert_eq!(
                    z.ground[i], before[i],
                    "all ground tops, water and transparent rock gaps stay intact"
                );
            }
        }
        for (left, right, edge) in south_face_runs(&m) {
            if right - left < 128 {
                continue;
            }
            let mut exposed = 0;
            let mut grass = 0;
            for x in left..right {
                let i = (edge * z.pw + x) as usize;
                if before[i] != marker {
                    continue;
                }
                if z.ground[i] == marker {
                    exposed += 1;
                } else {
                    grass += 1;
                }
            }
            assert!(
                exposed > (right - left) / 3,
                "native facets must break the lip across {left}..{right}"
            );
            assert!(
                grass > (right - left) / 30,
                "the meadow must connect through irregular turf drapes"
            );
        }
    }

    #[test]
    fn exposed_shelves_and_recessed_corners_share_a_continuous_native_origin() {
        let mut m = Map::parse();
        m.k.fill(K::Void);
        // A projecting ledge, then an inset shelf, then lower land in the same columns.
        for (y, left, right) in [
            (4, 14, 20),
            (5, 14, 20),
            (6, 15, 20),
            (7, 16, 20),
            (10, 14, 20),
        ] {
            for x in left..=right {
                m.k[(y * m.tw + x) as usize] = K::Grass;
            }
        }
        let runs = south_face_runs(&m);
        assert!(
            runs.contains(&(224, 240, 96)),
            "the projecting upper ledge must not disappear behind the bottommost land"
        );
        assert!(runs.contains(&(240, 256, 112)));
        assert!(runs.contains(&(256, 336, 128)));
        assert!(runs.contains(&(224, 336, 176)));
        let upper = connected_face_lip(&runs, 239, 96);
        let lower = connected_face_lip(&runs, 240, 112);
        assert!(
            (upper - lower).abs() <= 2,
            "one rock field crosses the step instead of restarting a source frame: {upper}/{lower}"
        );
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        let mask = paint(&mut z, &m);
        for y in 164..172 {
            for x in 232..328 {
                assert_eq!(
                    mask[(y * z.pw + x) as usize],
                    mat::GRASS,
                    "an upper cliff cannot overwrite the lower shelf"
                );
            }
        }
        assert_eq!(
            physical_surface(&z, &mask)[(97 * z.pw + 230) as usize],
            crate::world::Tile::Void,
            "visible upper foundation stays non-walkable"
        );
    }

    #[test]
    fn rounded_coast_separates_traversable_tops_from_attached_vertical_rock() {
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        let mask = paint(&mut z, &m);
        let surface = physical_surface(&z, &mask);
        let mut rounded_out = 0;
        let mut rounded_in = 0;
        for y in 0..z.ph {
            for x in 0..z.pw {
                let i = (y * z.pw + x) as usize;
                let original = group(m.at(x / 16, y / 16));
                if original == mat::VOID && mask[i] == mat::GRASS {
                    rounded_out += 1;
                    assert_eq!(surface[i], crate::world::Tile::Floor);
                    assert_ne!(
                        z.ground[i], HOLE,
                        "a rounded concave shore is visible wherever it becomes walkable"
                    );
                }
                if original == mat::GRASS && matches!(mask[i], mat::VOID | mat::EDGE) {
                    rounded_in += 1;
                    assert_eq!(
                        surface[i],
                        crate::world::Tile::Void,
                        "rounded-away top and its attached vertical face cannot be walked on"
                    );
                }
                if mask[i] == mat::VOID {
                    assert_eq!(z.ground[i], HOLE);
                }
            }
        }
        assert!(
            rounded_in > 100 && rounded_out > 100,
            "exercise real convex and concave corners ({rounded_in}/{rounded_out})"
        );
    }

    #[test]
    fn side_returns_attach_to_native_turf_without_changing_any_contact_or_water() {
        let mut z = Zone::new(ZoneId::Bloomwild, 5, 5);
        z.ground.fill(HOLE);
        let mut mask = vec![mat::VOID; (z.pw * z.ph) as usize];
        for (first, last, left) in [(16, 24, 24), (24, 32, 40), (32, 40, 16), (40, 48, 32)] {
            for y in first..last {
                for x in left..48 {
                    let i = (y * z.pw + x) as usize;
                    mask[i] = mat::GRASS;
                    z.ground[i] = pal::GRASS[2];
                }
            }
        }
        for y in 24..42 {
            for x in 48..56 {
                let i = (y * z.pw + x) as usize;
                mask[i] = mat::DEEP;
                z.ground[i] = pal::WATER[1];
            }
        }
        let surface = physical_surface(&z, &mask);
        let before = z.ground.clone();
        coast_side_returns(&mut z, &mut mask);
        assert_eq!(
            physical_surface(&z, &mask),
            surface,
            "a visible side plane never extends traversable land or changes a water hazard"
        );
        let mut returns = 0;
        for y in 0..z.ph {
            for x in 0..z.pw {
                let i = (y * z.pw + x) as usize;
                if mask[i] == mat::DEEP {
                    assert_eq!(z.ground[i], before[i]);
                }
                if mask[i] != mat::EDGE {
                    continue;
                }
                returns += 1;
                assert_ne!(z.ground[i], HOLE);
                assert!(
                    ((y - 17).max(0)..=y).any(|ty| ((x - 11).max(0)..=(x + 11).min(z.pw - 1))
                        .any(|tx| surface[(ty * z.pw + tx) as usize] == crate::world::Tile::Floor)),
                    "every visible side plane remains attached to actual native land"
                );
            }
        }
        assert!(returns > 100);
        let mut reached = vec![false; mask.len()];
        let mut queue = std::collections::VecDeque::new();
        for (i, &s) in surface.iter().enumerate() {
            if s == crate::world::Tile::Floor {
                reached[i] = true;
                queue.push_back(i);
            }
        }
        while let Some(i) = queue.pop_front() {
            let (x, y) = (i as i32 % z.pw, i as i32 / z.pw);
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx < 0 || ny < 0 || nx >= z.pw || ny >= z.ph {
                    continue;
                }
                let n = (ny * z.pw + nx) as usize;
                if !reached[n] && mask[n] == mat::EDGE && z.ground[n] != HOLE {
                    reached[n] = true;
                    queue.push_back(n);
                }
            }
        }
        for (i, &g) in mask.iter().enumerate() {
            if g == mat::EDGE {
                assert!(
                    reached[i],
                    "no visible side-face component can float away from real native land at {},{}",
                    i as i32 % z.pw,
                    i as i32 / z.pw
                );
            }
        }
    }

    #[test]
    fn complete_terrace_endcap_sits_on_connected_rock_instead_of_air() {
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        let mask = paint(&mut z, &m);
        let surface = physical_surface(&z, &mask);
        for y in 359..368 {
            let i = (y * z.pw + 54) as usize;
            assert_eq!(mask[i], mat::EDGE);
            assert_ne!(
                z.ground[i], HOLE,
                "the west wall cap needs a solid attached rock foot at54,{y}"
            );
            assert_eq!(surface[i], crate::world::Tile::Void);
        }
    }

    #[test]
    fn terrace_endcaps_wrap_only_exposed_wall_ends_and_preserve_every_surface() {
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        let mut mask = Vec::with_capacity((z.pw * z.ph) as usize);
        for y in 0..z.ph {
            for x in 0..z.pw {
                let g = group(m.at(x / 16, y / 16));
                mask.push(g);
                if g != mat::VOID {
                    z.gset(x, y, pal::STONE[1]);
                }
            }
        }
        let surface = physical_surface(&z, &mask);
        let before = z.ground.clone();
        terrace_endcaps(&mut z, &m, &mut mask);
        assert_eq!(
            physical_surface(&z, &mask),
            surface,
            "visible oblique masonry is a vertical face, never a new traversable ledge"
        );
        assert_eq!(mask[(334 * z.pw + 58) as usize], mat::EDGE);
        assert_ne!(
            z.gget(58, 334),
            HOLE,
            "the west termination has an actual face rather than a 1px outline"
        );
        for (i, &g) in mask.iter().enumerate() {
            if g != mat::EDGE {
                assert_eq!(
                    z.ground[i], before[i],
                    "coping returns cannot cover actual land, front masonry, stairs, niches or water"
                );
            }
        }
        for x in 64..1296 {
            if matches!(m.at(x / 16, 20), K::Stairs | K::Falls) {
                assert_ne!(
                    mask[(334 * z.pw + x) as usize],
                    mat::EDGE,
                    "internal passages are not exposed terminal wall ends"
                );
            }
        }
    }

    #[test]
    fn native_facade_drops_standalone_end_posts_and_keeps_real_stairs_and_outlets_open() {
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        z.ground.fill(pal::GRASS[2]);
        let mut rgba = Vec::new();
        for _y in 0..32 {
            for x in 0..128 {
                rgba.extend_from_slice(if !(4..122).contains(&x) {
                    &[240, 20, 220, 255]
                } else {
                    &[40, 60, 90, 255]
                });
            }
        }
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 128, 32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&rgba)
                .unwrap();
        }
        let s = Sheet::parse(
            "bloomwild_terrace",
            &png,
            r#"{"cell":[128,32],"anims":{"body":{"row":0,"frames":1}}}"#,
        )
        .unwrap();
        terrace_art(&mut z, &m, &s);
        for y in 320..352 {
            for x in 64..1296 {
                assert_eq!(
                    z.gget(x, y),
                    if m.at(x / 16, y / 16) == K::Cliff {
                        Rgb::from_u8([40, 60, 90])
                    } else {
                        pal::GRASS[2]
                    },
                    "no repeated module end post and no covered opening at {x},{y}"
                );
            }
        }
    }

    #[test]
    fn rounded_shores_and_sky_cuts_have_the_same_physical_surface_as_the_art() {
        use crate::world::Tile;
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        let mask = paint(&mut z, &m);
        let surface = physical_surface(&z, &mask);
        let mut rounded = 0;
        let mut skyline_cuts = 0;
        for y in 0..z.ph {
            for x in 0..z.pw {
                let i = (y * z.pw + x) as usize;
                let expected = super::super::tile_of(m.at(x / 16, y / 16));
                if surface[i] != expected && matches!(mask[i], mat::DEEP | mat::SHALLOW) {
                    rounded += 1;
                    assert!(matches!(surface[i], Tile::Deep | Tile::Shallow));
                }
                if mask[i] == mat::GRASS && z.ground[i] == HOLE {
                    skyline_cuts += 1;
                    assert_eq!(surface[i], Tile::Void);
                }
                if matches!(mask[i], mat::EDGE | mat::VOID) {
                    assert_eq!(
                        surface[i],
                        Tile::Void,
                        "hanging decorative cliff is not walkable"
                    );
                }
                if matches!(mask[i], mat::BRIDGE | mat::STAIRS) {
                    assert_eq!(surface[i], Tile::Floor, "all visible crossings stay open");
                }
            }
        }
        assert!(
            rounded > 1000,
            "exercise real curved shore pixels, not only tile centers"
        );
        assert!(skyline_cuts > 100, "exercise the eroded top silhouette");
        z.set_surface_tiles(surface.clone());
        for (x, y) in [(688, 304), (704, 255), (816, 610), (856, 733)] {
            assert_eq!(
                z.tile_at(x as f32, y as f32),
                surface[(y * z.pw + x) as usize]
            );
        }
    }

    #[test]
    fn terrace_contacts_preserve_openings_and_all_wet_pixels() {
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        let mask = paint(&mut z, &m);
        let before = z.ground.clone();
        terrain_contacts(&mut z, &m, &mask);
        for (i, &g) in mask.iter().enumerate() {
            if mat::water(g) {
                assert_eq!(
                    z.ground[i], before[i],
                    "abutments and coping never become a water floor"
                );
            }
        }
        for x in 64..1296 {
            if m.at(x / 16, 20) == K::Stairs {
                assert_eq!(mask[(336 * z.pw + x) as usize], mat::STAIRS);
                assert_ne!(
                    z.gget(x, 336),
                    pal::STONE[0],
                    "no facade crosses the stair route"
                );
            } else if m.at(x / 16, 20) == K::Cliff {
                assert_eq!(
                    z.gget(x, 322),
                    pal::STONE[1],
                    "coping seats above the native wall"
                );
            }
        }
    }

    #[test]
    fn native_perimeter_covers_every_land_face_and_preserves_outlet_support() {
        let m = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
        for ty in 0..m.th {
            for tx in 0..m.tw {
                z.set_tile(tx, ty, super::super::tile_of(m.at(tx, ty)));
            }
        }
        let mask = paint(&mut z, &m);
        let before = z.ground.clone();
        let collision = z.tiles.clone();
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 128, 96);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[240, 20, 220, 255].repeat(128 * 96))
                .unwrap();
        }
        let s = Sheet::parse(
            "bloomwild_cliff",
            &png,
            r#"{"cell":[128,96],"anchor":[64,26],"anims":{"body":{"row":0,"frames":1}}}"#,
        )
        .unwrap();
        perimeter_art(&mut z, &m, &mask, &s);
        let marker = Rgb::from_u8([240, 20, 220]);
        let runs = raster_face_runs(&z, &mask);
        let coarse_runs = south_face_runs(&m);
        let mut checked_land_pixels = 0;
        for ty in 0..m.th {
            for tx in 0..m.tw {
                let k = m.at(tx, ty);
                let exposed = k != K::Void
                    && m.at(tx, ty + 1) == K::Void
                    && !matches!(k, K::Deep | K::Shallow | K::Falls);
                let owners = coarse_runs
                    .iter()
                    .filter(|&&(l, r, e)| tx * 16 >= l && tx * 16 < r && e == (ty + 1) * 16)
                    .count();
                assert_eq!(
                    owners,
                    usize::from(exposed),
                    "each exposed dry ledge is covered exactly once"
                );
            }
        }
        for (i, &g) in mask.iter().enumerate() {
            if g != mat::EDGE {
                assert_eq!(
                    z.ground[i], before[i],
                    "all top surfaces, water, erosion cuts and open sky are protected"
                );
            } else if z.ground[i] != before[i] {
                checked_land_pixels += 1;
                let x = i as i32 % z.pw;
                let y = i as i32 / z.pw;
                assert!(
                    z.ground[i] == HOLE
                        || runs
                            .iter()
                            .any(|&(l, r, e)| x >= l && x < r && y >= e && y < e + 96),
                    "visible native rock belongs to an actual exposed dry ledge; orphan old backing is removed at {x},{y}"
                );
                let pixel = z.ground[i];
                assert!(
                    pixel == HOLE
                        || pixel == marker
                        || pixel.r <= marker.r && pixel.g <= marker.g && pixel.b <= marker.b,
                    "only native pixels, attached corner shading and transparent source gaps are emitted"
                );
            }
        }
        assert!(checked_land_pixels > 40_000);
        assert_eq!(
            physical_surface(&z, &mask),
            {
                let mut original = Zone::new(ZoneId::Bloomwild, m.tw, m.th);
                original.ground = before;
                physical_surface(&original, &mask)
            },
            "decorative rock never changes exact ground, erosion or hazard contacts"
        );
        assert_eq!(
            z.tiles, collision,
            "native cliff art never changes world collision"
        );
    }

    #[test]
    fn candidate_materials_clip_water_preserve_transparency_and_use_matching_material() {
        let mut rgba = vec![0; 32 * 64 * 4];
        for y in 0..64 {
            for x in 0..32 {
                let i = (y * 32 + x) * 4;
                rgba[i..i + 4].copy_from_slice(if y < 32 {
                    &[255, 0, 0, 255]
                } else {
                    &[0, 0, 255, 255]
                });
            }
        }
        rgba[(15 * 32 + 15) * 4 + 3] = 0;
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 32, 64);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&rgba)
                .unwrap();
        }
        let manifest = r#"{"cell":[32,32],"anims":{"grass":{"row":0,"frames":1},"slab":{"row":1,"frames":1}}}"#;
        let mut s = Sheet::parse("bloomwild_materials", &png, manifest).unwrap();
        let mut z = Zone::new(ZoneId::Bloomwild, 4, 4);
        let original = Rgb::from_u8([40, 50, 60]);
        z.ground.fill(original);
        let mut mask = vec![mat::GRASS; 64 * 64];
        mask[10 * 64 + 10] = mat::SLAB;
        mask[20 * 64 + 20] = mat::DEEP;
        material_art(&mut z, &mask, &s);
        assert_eq!(z.ground[8 * 64 + 8], Rgb::from_u8([255, 0, 0]));
        assert_eq!(z.ground[10 * 64 + 10], Rgb::from_u8([0, 0, 255]));
        assert_eq!(
            z.ground[15 * 64 + 15],
            original,
            "transparent source retains previous native floor"
        );
        assert_eq!(
            z.ground[20 * 64 + 20],
            original,
            "water never receives floor art"
        );
        assert_eq!(
            z.ground[20 * 64 + 17],
            original,
            "bank contact keeps its authored border"
        );
        s.json["preserve_fields"] = serde_json::json!(true);
        z.ground.fill(original);
        material_art(&mut z, &mask, &s);
        assert_eq!(z.ground[8 * 64 + 8], Rgb::from_u8([255, 0, 0]).scale(42.0 / 48.0));
        assert_eq!(
            z.ground[10 * 64 + 10],
            Rgb::from_u8([0, 0, 255]),
            "field lighting applies to grass, not stone"
        );
        assert_eq!(
            z.ground[15 * 64 + 15],
            original,
            "lighting never fills transparent source pixels"
        );
    }

    #[test]
    fn material_variants_are_deterministic_and_do_not_repeat_neighbors() {
        for frames in [2, 3, 8] {
            let chosen = material_variants(44, 27, frames, 181);
            assert_eq!(chosen, material_variants(44, 27, frames, 181));
            for y in 0..27 {
                for x in 0..44 {
                    let v = chosen[y * 44 + x];
                    assert!(v < frames);
                    if x > 0 {
                        assert_ne!(v, chosen[y * 44 + x - 1]);
                    }
                    if y > 0 {
                        assert_ne!(v, chosen[(y - 1) * 44 + x]);
                    }
                }
            }
        }
    }
}
