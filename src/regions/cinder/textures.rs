//! Native PixelLab mineral surfaces, clipped to the existing physical material masks.
//! Lava, banks, bridges and stair treads keep their authored pixels. Raised shrine
//! masonry shares its native silhouette with collision around a clear processional lane.

use super::field::Grid;
use super::paint::{K_ISLEFACE, K_PAVE, K_ROCKFACE};
use crate::gfx::{Sheet, sheet};
use crate::util::hash2;
use crate::world::{HOLE, Tile, Zone};

/// Every material and masonry cell stays on its native integer grid.
pub fn apply(z: &mut Zone, kinds: &Grid<u8>) {
    if let Some(s) = sheet("cinder_floor") {
        floor(z, kinds, s);
    }
    if let Some(s) = sheet("cinder_cliff") {
        cliff(z, kinds, s);
    }
    if let Some(s) = sheet("cinder_shrine") {
        shrine(z, kinds, s);
    }
    if let Some(s) = sheet("cinder_mineral") {
        mineral(z, kinds, s);
    }
    workyards(z, kinds);
    cooled_flows(z, kinds);
}

/// The surviving presses belong to work floors, rather than standing as isolated ornaments.
/// Everything here is flush albedo: old copper sockets, wheel scars and worn receiving slabs.
/// Preserve the native cracks underneath, and leave the middle of the court quiet for combat.
fn workyards(z: &mut Zone, kinds: &Grid<u8>) {
    use super::art::{C2, C3, O3, O4};
    for (cx, cy, rx, ry, cut) in [(621, 501, 64, 46, 12), (600, 709, 67, 25, 10)] {
        let upper = cy == 501;
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                if !kinds.inside(x, y) || kinds.at(x, y) != K_PAVE {
                    continue;
                }
                let (dx, dy) = ((x - cx).abs(), (y - cy).abs());
                // Chamfered receiving apron. These are broad flat slabs, with no fake shadow
                // or raised lip; the old running-bond joints remain visible through the wear.
                if dx > rx || dy > ry || dx + dy > rx + ry - cut {
                    continue;
                }
                let c = z.gget(x, y);
                let edge = (rx - dx).min(ry - dy).min(rx + ry - cut - dx - dy);
                let worn = if upper {
                    // The pressure bed is rubbed in front of the upper press, away from the
                    // two clear side lanes. Long scars belong to the machine's working axis.
                    (x - cx).abs() < 27 && (cy - 40..cy + 21).contains(&y)
                } else {
                    (y - cy).abs() < 9 && (cx - 44..cx + 39).contains(&x)
                };
                if worn {
                    // A rubbed copper receiving bed is a distinct material, not another
                    // dark rectangle on dark stone. Keep native mortar and cracks visible.
                    let copper = upper && (x - cx).abs() + (y - (cy - 4)).abs() / 4 < 26;
                    let intact = c.luma() > O3.luma() * 0.60;
                    z.gset(
                        x,
                        y,
                        if copper && intact {
                            c.lerp(C2, 0.58)
                        } else {
                            c.lerp(O3, 0.24)
                        },
                    );
                } else if edge > 5 {
                    z.gset(x, y, c.lerp(O4, 0.08));
                }
                // Broken copper stays cool and subordinate to every actual flame. Its
                // deliberate long gaps also avoid outlining an apparent solid obstacle.
                let segment = if upper {
                    (x < cx - 48 && y > cy - 27 && y < cy + 15)
                        || (x > cx + 47 && y > cy - 15 && y < cy + 31)
                        || (y > cy + 36 && x > cx - 23 && x < cx + 37)
                        || (y < cy - 36 && x > cx - 33 && x < cx + 16)
                } else {
                    (y < cy - 17 && x > cx - 48 && x < cx - 6)
                        || (y > cy + 17 && x > cx + 5 && x < cx + 44)
                        || (x > cx + 59 && (y - cy).abs() < 8)
                };
                if segment && edge == 3 {
                    z.gset(x, y, c.lerp(C2, 0.63));
                } else if segment && edge == 4 && (x + 2 * y).rem_euclid(19) < 7 {
                    z.gset(x, y, c.lerp(C3, 0.31));
                }
                // Paired flush wheel grooves run from the standing press into its receiving
                // apron. Their staggered ends read as wear, not new collision geometry.
                if upper
                    && (x == cx - 14 || x == cx + 13)
                    && (cy - 44..cy + 22).contains(&y)
                    && !((cy - 10..cy - 5).contains(&y) || (cy + 7..cy + 12).contains(&y))
                {
                    z.gset(x, y, c.lerp(O3, 0.47));
                    z.gset_plane_surface(x, y, [0.0, 0.0, 1.0], 0.0);
                    // The tiny inset replaces the substrate at its own footprint.
                    // Its flanks face into the geometric groove, never toward an
                    // inferred dark/bright direction in the underlying mineral art.
                    for (dx, nx) in [(-1, 0.24), (1, -0.24)] {
                        if kinds.inside(x + dx, y) && kinds.at(x + dx, y) == K_PAVE {
                            z.gset_plane_surface(x + dx, y, [nx, 0.0, 0.97], 0.0);
                        }
                    }
                }
            }
        }
    }
    // Four paired recessed hold-down sockets frame the upper machine bed. Each is a tiny
    // two-tone inset with no drop shadow; none occupies a new physical footprint.
    for (x, y) in [(584, 462), (653, 462), (584, 534), (653, 534)] {
        for dy in 0..3 {
            for dx in 0..5 {
                let (wx, wy) = (x + dx, y + dy);
                if kinds.inside(wx, wy) && kinds.at(wx, wy) == K_PAVE {
                    let old = z.gget(wx, wy);
                    let ink = if dy == 2 { C2 } else { O3 };
                    z.gset(wx, wy, old.lerp(ink, 0.58));
                    let ny = if dy == 0 { 0.25 } else if dy == 2 { -0.25 } else { 0.0 };
                    z.gset_plane_surface(wx, wy, [0.0, ny, 0.97], 0.0);
                }
            }
        }
    }
    press_material_accents(z, kinds);
}

/// Old pressure marks and two offset material spills belong to this workshop, rather than
/// another generic ruined court. All three are flush pigment on existing paving; no outline,
/// cast shadow, raised object, new collision or emissive orange is introduced.
fn press_material_accents(z: &mut Zone, kinds: &Grid<u8>) {
    use super::art::{C2, C3, O2, O3, O4};
    for y in 471..534 {
        for x in 604..639 {
            if !kinds.inside(x, y) || kinds.at(x, y) != K_PAVE {
                continue;
            }
            let c = z.gget(x, y);
            if c.luma() < O3.luma() * 0.62 {
                continue;
            }
            let local_x = x - 604;
            // Long, broad burnished patches break across the old slab joints. Three unequal
            // widths and staggered ends avoid a pair of railway-like lines through combat.
            let polished = (3..9).contains(&local_x) && (480..511).contains(&y)
                || (15..24).contains(&local_x) && (473..498).contains(&y)
                || (26..30).contains(&local_x) && (501..529).contains(&y);
            if polished && y.rem_euclid(23) != 22 {
                z.gset(x, y, c.lerp(C3, 0.28));
            }
        }
    }
    // A broken fan of ground-in copper scale left of the bed, with stone dust opposite it.
    // These use broad attached clusters and a few lit upper edges, never scattered noise.
    for (cx, cy, copper) in [(571, 526, true), (659, 547, false)] {
        for dy in 0..10 {
            for dx in 0..25 {
                let (x, y) = (cx + dx, cy + dy);
                if !kinds.inside(x, y) || kinds.at(x, y) != K_PAVE {
                    continue;
                }
                let first = dy < 6 && (2 + dy / 2..18 - dy).contains(&dx);
                let second = (4..9).contains(&dy) && (14..24 - (dy - 4) * 2).contains(&dx);
                if !(first || second) {
                    continue;
                }
                let c = z.gget(x, y);
                if c.luma() < O3.luma() * 0.60 {
                    continue;
                }
                let ink = if copper {
                    if dy == 0 && (4..12).contains(&dx) {
                        C3
                    } else {
                        C2
                    }
                } else if dy < 2 && dx < 15 {
                    O4
                } else if dy >= 7 {
                    O2
                } else {
                    O3
                };
                z.gset(x, y, c.lerp(ink, if copper { 0.61 } else { 0.48 }));
            }
        }
    }
}

/// A few large cooling fans tell the old lava's direction across the ash. These are shallow
/// sediment bands, not rocks: only dry ash receives them, with long clean intervals between.
fn cooled_flows(z: &mut Zone, kinds: &Grid<u8>) {
    use super::art::{A1, A3};
    for (cx, cy, length, bend, flip) in [
        (845, 335, 126, 10, false),
        (267, 659, 72, 14, true),
        (1038, 441, 88, 8, false),
    ] {
        for band in 0..3 {
            let span = length - band * 18;
            for x in -span / 2..=span / 2 {
                // Broad, stepped curves share a geological direction; no random speckles.
                let t = x as f32 / (length as f32 * 0.5);
                let curve = (t * t * bend as f32) as i32;
                let yy = cy + band * 5 + if flip { -curve } else { curve };
                for dy in 0..3 {
                    let (wx, wy) = (cx + x, yy + dy);
                    if !kinds.inside(wx, wy) || kinds.at(wx, wy) != super::paint::K_ASH {
                        continue;
                    }
                    // Keep a little air around every end. The seam is part of an old cooled
                    // sheet, with a subdued lower lip instead of an emissive orange stripe.
                    if x.abs() > span / 2 - 3 && dy != 1 {
                        continue;
                    }
                    let c = z.gget(wx, wy);
                    z.gset(
                        wx,
                        wy,
                        c.lerp(
                            if dy == 2 { A3 } else { A1 },
                            if dy == 2 { 0.19 } else { 0.30 },
                        ),
                    );
                    // A shallow cooled seam and its lower lip follow this exact
                    // painted curve; broad ash shading never enters the geometry.
                    let slope = 2.0 * t * bend as f32 / (length as f32 * 0.5)
                        * if flip { -1.0 } else { 1.0 };
                    let ny = if dy == 0 { 0.20 } else if dy == 2 { -0.20 } else { 0.0 };
                    z.gset_plane_surface(wx, wy, [-slope * ny, ny, 0.98], 0.0);
                }
            }
        }
    }
}

/// Copy slab interiors, retaining the chipped perimeter from the authored paving field.
fn floor(z: &mut Zone, kinds: &Grid<u8>, s: &Sheet) {
    let Some(frames) = s.cells.first() else {
        return;
    };
    if frames.is_empty() {
        return;
    }
    let (w, h) = (s.cell_w, s.cell_h);
    for y in 0..z.ph {
        for x in 0..z.pw {
            if kinds.at(x, y) != K_PAVE {
                continue;
            }
            // Align to the original running bond. Tiles contain quiet broad planes rather than
            // a second grid of outlines; the original underpainting remains at exposed chips.
            let row = y.div_euclid(h);
            let off = if row.rem_euclid(2) == 0 { 0 } else { w / 2 };
            let col = (x + off).div_euclid(w);
            // Damage is a subordinate accent, not a crack on every single paving unit.
            if hash2(col, row, 305) > 0.34 {
                continue;
            }
            let frame =
                ((hash2(col, row, 307) * frames.len() as f32) as usize).min(frames.len() - 1);
            let p = frames[frame].at((x + off).rem_euclid(w), y.rem_euclid(h));
            if p.a {
                z.gset(x, y, p.c);
                z.gset_surface(x, y, &frames[frame], (x + off).rem_euclid(w), y.rem_euclid(h), false, 0.0);
            }
        }
    }
}

/// Native irregular fractured faces replace the old identical broad cuboids. The generated
/// base silhouette may fall into backdrop but never over the walkable ground or hot river.
fn cliff(z: &mut Zone, kinds: &Grid<u8>, s: &Sheet) {
    let Some(frames) = s.cells.first() else {
        return;
    };
    if frames.is_empty() {
        return;
    }
    let (w, _) = (s.cell_w, s.cell_h);
    let face_top = s
        .json
        .get("face_top")
        .and_then(|v| v.as_i64())
        .unwrap_or(24) as i32;
    let old = z.ground.clone();
    // Keep the already sculpted mesa tops and bank lips. Only the two cold cliff materials
    // receive the replacement; no visual art is permitted to bridge a lava channel.
    for x in 0..z.pw {
        let mut y = 0;
        while y < z.ph {
            let k = kinds.at(x, y);
            if !matches!(k, K_ISLEFACE | K_ROCKFACE) {
                y += 1;
                continue;
            }
            let top = y;
            while y < z.ph && kinds.at(x, y) == k {
                y += 1;
            }
            let end = y;
            let margin = if w >= 16 { 4 } else { 0 };
            let span = (w - 2 * margin).min(80).max(1);
            let region = x.div_euclid(span);
            let frame = ((hash2(region, if k == K_ISLEFACE { 1 } else { 2 }, 311)
                * frames.len() as f32) as usize)
                .min(frames.len() - 1);
            let c = &frames[frame];
            // Native subregions vary width/phase instead of repeating a whole128px stamp.
            let offset = margin
                + (hash2(region, if k == K_ISLEFACE { 1 } else { 2 }, 313)
                    * (w - span - 2 * margin).max(0) as f32) as i32;
            let sx = x.rem_euclid(span) + offset;
            let depth = end - top;
            // At the island boundary the source's own broken lower silhouette is visible.
            // It may extend only into old backdrop; hot falls and islands below are protected.
            let draw_end = if k == K_ISLEFACE {
                (top + c.h - face_top).min(z.ph)
            } else {
                end
            };
            let draw_start = if k == K_ROCKFACE {
                (top - face_top).max(0)
            } else {
                top
            };
            for yy in draw_start..draw_end {
                if yy < top && kinds.at(x, yy) != super::paint::K_ROCKTOP {
                    continue;
                }
                if yy >= end && kinds.at(x, yy) != super::paint::K_VOID {
                    break;
                }
                let sy = face_top + yy - top;
                let p = c.at(sx, sy);
                if p.a {
                    let retained = z.ground_surfaces[(yy * z.pw + x) as usize];
                    z.gset(x, yy, p.c);
                    // Source UVs retain the material normal; its painter owns the
                    // macro altitude. Hanging island art is below the floor, while
                    // an inaccessible mesa cap stays above its real visible face.
                    let height = if k == K_ISLEFACE { 0.0 }
                        else if yy < top { retained.height as f32 }
                        else { (end - yy - 1).max(0) as f32 };
                    z.gset_surface(x, yy, c, sx, sy, yy >= top, height);
                } else if k == K_ISLEFACE && yy - top > depth * 2 / 3 {
                    z.gset(x, yy, HOLE);
                } else if yy >= end {
                    // Transparent source pixels stay transparent beyond the old face.
                } else {
                    // Transparent top chips reveal the original continuous cliff backing.
                    z.gset(x, yy, old[(yy * z.pw + x) as usize]);
                }
            }
        }
    }
}

/// Broken ceremonial masonry frames a clear north-south route through the shrine.
/// The native raised stone is solid; its two open ends expose the original dais
/// paving so the approach to the shard and onward gate agrees with the artwork.
fn shrine(z: &mut Zone, kinds: &Grid<u8>, s: &Sheet) {
    let Some(c) = s.cells.first().and_then(|r| r.first()) else {
        return;
    };
    let (ox, oy) = (1144 - s.anchor.0, 182 - s.anchor.1);
    let stone = |x, y| {
        let p = c.at(x, y);
        p.a && p.c != crate::gfx::rgb(0x101017)
    };
    // A 24-pixel causeway keeps the shard and gate on the same visible floor.
    // Neither source albedo nor its normal may cover this opening.
    let lane = 1132..1156;
    let bottom: Vec<_> = (0..c.w)
        .map(|x| (0..c.h).rev().find(|&y| stone(x, y)))
        .collect();
    for y in 0..c.h {
        for x in 0..c.w {
            let (wx, wy) = (ox + x, oy + y);
            if lane.contains(&wx) || kinds.at(wx, wy) != super::paint::K_DAIS {
                continue;
            }
            let p = c.at(x, y);
            // The generator's staging fill is not a hole in the walkable dais.
            if stone(x, y) {
                z.gset(wx, wy, p.c);
                let from_bottom = bottom[x as usize].unwrap() - y;
                let upright = from_bottom < 8;
                z.gset_surface(wx, wy, c, x, y, upright, from_bottom.min(8) as f32);
                if let Some(tiles) = z.surface_tiles.as_mut() {
                    tiles[(wy * z.pw + wx) as usize] = Tile::Wall;
                }
            }
        }
    }
}

/// Low cooled mineral shelves break the broad mesa caps with deliberate sparse clusters.
/// They are embedded in the rock mask and cannot read as walkable stepping stones over lava.
fn mineral(z: &mut Zone, kinds: &Grid<u8>, s: &Sheet) {
    let Some(frames) = s.cells.first() else {
        return;
    };
    if frames.is_empty() {
        return;
    }
    for gy in (0..z.ph).step_by(64) {
        for gx in (0..z.pw).step_by(80) {
            if hash2(gx, gy, 317) > 0.28 {
                continue;
            }
            let c = &frames[(hash2(gx, gy, 319) * frames.len() as f32) as usize % frames.len()];
            let (ox, oy) = (gx + 4, gy + 6);
            let fits = (0..c.h).all(|y| {
                (0..c.w)
                    .all(|x| !c.at(x, y).a || kinds.at(ox + x, oy + y) == super::paint::K_ROCKTOP)
            });
            if !fits {
                continue;
            }
            for y in 0..c.h {
                for x in 0..c.w {
                    let p = c.at(x, y);
                    if p.a {
                        z.gset(ox + x, oy + y, p.c);
                        z.gset_surface(ox + x, oy + y, c, x, y, true, (c.h - y - 1).max(0) as f32);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::paint::{K_BANK, K_DAIS, K_DECK, K_FALL, K_LAVA, K_PATH, K_VOID, K_WALLFACE};
    use super::*;
    use crate::gfx::{Cell, Px, Rgb};
    use crate::world::ZoneId;
    use std::collections::HashMap;

    #[test]
    fn broken_shrine_masonry_is_solid_around_an_untouched_floor_causeway() {
        let s = sheet("cinder_shrine").expect("embedded native shrine artwork");
        let c = &s.cells[0][0];
        let (ox, oy) = (1144 - s.anchor.0, 182 - s.anchor.1);
        let mut z = Zone::new(ZoneId::Cinder, 88, 16);
        let base = Rgb::new(0.1, 0.2, 0.3);
        z.ground.fill(base);
        z.set_surface_tiles(vec![Tile::Floor; z.ground.len()]);
        z.fill_walkable_surfaces(|_, _| ([0.0, 0.0, 1.0], 0.0));
        let floor = z.ground_surfaces[0];
        let mut kinds = Grid::new(z.pw, z.ph, K_DAIS);
        // Even native stone cannot replace the actual lava next to the platform.
        kinds.set(ox + 13, oy + 42, K_LAVA);
        shrine(&mut z, &kinds, s);

        let mut walls = 0;
        let mut open_stone = 0;
        let mut roofs = 0;
        let mut faces = 0;
        for y in 0..c.h {
            for x in 0..c.w {
                let (wx, wy) = (ox + x, oy + y);
                let i = (wy * z.pw + wx) as usize;
                let p = c.at(x, y);
                let stone = p.a && p.c != crate::gfx::rgb(0x101017);
                if stone && !(1132..1156).contains(&wx) && kinds.at(wx, wy) == K_DAIS {
                    walls += 1;
                    assert_eq!(z.ground[i], p.c, "native stone changed at {wx},{wy}");
                    assert_eq!(z.tile_at(wx as f32, wy as f32), Tile::Wall,
                        "raised stone is passable at {wx},{wy}");
                    let bottom = (0..c.h).rev().find(|&sy| {
                        let p = c.at(x, sy);
                        p.a && p.c != crate::gfx::rgb(0x101017)
                    }).unwrap();
                    let from_bottom = bottom - y;
                    let surface = z.ground_surfaces[i];
                    assert_eq!(surface.height, from_bottom.min(8) as u8);
                    assert_eq!(surface.upright, from_bottom < 8);
                    if surface.upright { faces += 1; } else { roofs += 1; }
                } else {
                    if stone && (1132..1156).contains(&wx) { open_stone += 1; }
                    assert_eq!(z.ground[i], base, "protected floor changed at {wx},{wy}");
                    assert_eq!(z.ground_surfaces[i], floor,
                        "protected floor normal changed at {wx},{wy}");
                    assert_eq!(z.tile_at(wx as f32, wy as f32), Tile::Floor,
                        "shrine sealed its floor opening at {wx},{wy}");
                }
            }
        }
        assert!(walls > 2000 && open_stone > 400, "exercise real shrine and both openings");
        assert!(roofs > 1500 && faces > 300, "raised masonry needs roof and front planes");
    }

    /// Cover the most dangerous integration boundary: decorative native faces may extend into
    /// backdrop but cannot cover a flowing lavafall, a bank, bridge or usable path.
    #[test]
    fn native_mineral_art_preserves_hot_surfaces_and_crossings() {
        let mut z = Zone::new(ZoneId::Cinder, 2, 2);
        let base = Rgb::new(0.1, 0.2, 0.3);
        let stone = Rgb::new(0.6, 0.5, 0.4);
        z.ground.fill(base);
        let tiles = z.tiles.clone();
        let mut kinds = Grid::new(z.pw, z.ph, K_VOID);
        for (y, kind) in [
            (2, K_PAVE),
            (4, K_ROCKFACE),
            (8, K_ISLEFACE),
            (9, K_LAVA),
            (10, K_BANK),
            (11, K_DECK),
            (12, K_FALL),
            (13, K_PATH),
            (14, K_DAIS),
            (15, K_WALLFACE),
        ] {
            for x in 0..z.pw {
                kinds.set(x, y, kind);
            }
        }
        let sheet = Sheet {
            name: "test".into(),
            cell_w: 8,
            cell_h: 8,
            anchor: (4, 0),
            cells: vec![vec![Cell {
                w: 8,
                h: 8,
                surface: Default::default(),
                px: vec![
                    Px {
                        c: stone,
                        a: true,
                        emit: false
                    };
                    64
                ],
            }]],
            anims: HashMap::new(),
            json: serde_json::json!({"face_top":0}),
        };
        floor(&mut z, &kinds, &sheet);
        cliff(&mut z, &kinds, &sheet);
        for y in 0..z.ph {
            for x in 0..z.pw {
                let c = z.ground[(y * z.pw + x) as usize];
                if matches!(kinds.at(x, y), K_PAVE | K_ROCKFACE | K_ISLEFACE) {
                    if kinds.at(x, y) != K_PAVE
                        || hash2(x.div_euclid(8), y.div_euclid(8), 305) <= 0.34
                    {
                        assert_eq!(c, stone, "cold surface was not replaced at {x},{y}");
                    }
                } else if kinds.at(x, y) != K_VOID {
                    assert_eq!(c, base, "protected material changed at {x},{y}");
                }
            }
        }
        assert_eq!(z.tiles, tiles, "native artwork changed collision");
    }
}
