//! Complete geometric receivers, then enrich their material planes once at load.
//! Relief follows world-space turf and stone, never color or baked illumination.

use super::{K, Map, mat};
use crate::gfx::{lighting::TerrainSurface, terrain_detail::{self, Material}};
use crate::util::hash2;
use crate::world::Zone;

const UP: [f32; 3] = [0.0, 0.0, 1.0];

pub(super) fn apply(z: &mut Zone, m: &Map, mask: &[u8]) {
    let pw = z.pw;
    z.fill_walkable_surfaces(|x, y| {
        let normal = match mask[(y * pw + x) as usize] {
            mat::PATH => path(x, y),
            mat::BRIDGE => bridge(x, y),
            // Slab fringes outside a source slab tile are painted as 16px inset
            // stones by slabs(). Interior source cells retain their exact atlas UV.
            mat::SLAB if !matches!(m.at(x / 16, y / 16), K::Slab | K::Altar) => {
                let (lx, ly) = (x.rem_euclid(16), y.rem_euclid(16));
                if lx == 0 || lx == 15 || ly == 0 || ly == 15 { UP }
                else { bevel(lx == 1, lx == 14, ly == 1, ly == 14) }
            }
            // Macro geometry stays independent of the material relief below.
            _ => UP,
        };
        (normal, 0.0)
    });
    // This is a one-shot builder pass, immediately after painting and installing
    // the physical mask. In particular, flat native atlas samples need the same
    // continuous turf/material relief as the procedural margins around them.
    for y in 0..z.ph {
        for x in 0..z.pw {
            let i = (y * pw + x) as usize;
            if z.ground[i].r < 0.0 { continue; }
            let Some(material) = material(mask, pw, z.ph, x, y) else { continue };
            let source = z.ground_surfaces[i];
            let base = if source.present() {
                source.normal.map(|v| v as f32 / 127.0)
            } else {
                // Hidden floor under a solid prop is still real terrain and
                // becomes visible when the prop breaks. Do not infer missing
                // cliff altitude or replace a hanging face with an upward plane.
                if matches!(mask[i], mat::CLIFF | mat::EDGE) { continue; }
                UP
            };
            let detail = terrain_detail::normal(material, x, y, base);
            let mut surface = TerrainSurface::new(detail, source.height as f32);
            if source.present() { surface.upright = source.upright; }
            z.ground_surfaces[i] = surface;
        }
    }
    z.invalidate_lighting();
}

fn material(mask: &[u8], pw: i32, ph: i32, x: i32, y: i32) -> Option<Material> {
    Some(match mask[(y * pw + x) as usize] {
        mat::GRASS => {
            // Same authored pool-coping and broken outlet stones as banks().
            let stone_bank = y < 254 && (584..816).contains(&x)
                || y >= 608 && hash2(y / 17, x / 24, 738) > 0.43;
            let near_water = stone_bank && (1..=4).any(|d| {
                [(-d, 0), (d, 0), (0, -d), (0, d)].iter().any(|&(dx, dy)| {
                    let (xx, yy) = (x + dx, y + dy);
                    xx >= 0 && yy >= 0 && xx < pw && yy < ph
                        && mat::water(mask[(yy * pw + xx) as usize])
                })
            });
            if near_water { Material::WetStone } else { Material::Grass }
        }
        mat::PATH => if path_stone(x, y) { Material::Stone } else { Material::Earth },
        mat::SLAB | mat::BRIDGE | mat::STAIRS => Material::Stone,
        mat::CLIFF | mat::EDGE => Material::Rock,
        mat::DIRT | mat::BANK => Material::Earth,
        _ => return None,
    })
}

fn bevel(left: bool, right: bool, back: bool, front: bool) -> [f32; 3] {
    [(i32::from(right) - i32::from(left)) as f32 * 0.4,
     (i32::from(front) - i32::from(back)) as f32 * 0.4, 1.0]
}

/// Same staggered 27x15 stepping stones and chipped corners as paint::trails.
fn path(x: i32, y: i32) -> [f32; 3] {
    let row = y.div_euclid(15);
    let xx = x + row.rem_euclid(2) * 13;
    let (lx, ly) = (xx.rem_euclid(27), y.rem_euclid(15));
    if !path_stone(x, y) { return UP; }
    bevel(lx == 3, lx == 26, ly == 3, ly == 14)
}

fn path_stone(x: i32, y: i32) -> bool {
    let row = y.div_euclid(15);
    let xx = x + row.rem_euclid(2) * 13;
    let (bx, by) = (xx.div_euclid(27), row);
    let (lx, ly) = (xx.rem_euclid(27), y.rem_euclid(15));
    let chipped = hash2(bx, by, 113) > 0.6 && lx + ly < 5;
    lx > 2 && ly > 2 && !chipped && hash2(bx, by, 110) > 0.1
}

/// bridge_decks paints two 112x32 decks, with 35px bonds and seams at rows 12/22.
fn bridge(x: i32, y: i32) -> [f32; 3] {
    let Some((bx, by)) = [(41 * 16, 16 * 16), (49 * 16, 36 * 16)].into_iter()
        .find(|&(bx, by)| (bx..bx + 112).contains(&x) && (by..by + 32).contains(&y))
    else { return UP; };
    let (lx, ly) = (x - bx, y - by);
    if ly > 29 { return [0.0, 1.0, 0.0]; }
    if ly < 3 { return if ly == 0 { UP } else { [0.0, 1.0, 0.0] }; }
    let joint = (lx + ly / 10 * 17).rem_euclid(35);
    if joint == 0 || ly == 12 || ly == 22 { return UP; }
    bevel(joint == 1, joint == 34, ly == 13 || ly == 23, ly == 11 || ly == 21 || ly == 29)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::ZoneId;

    #[test]
    fn material_relief_covers_the_route_without_changing_art_collision_or_macrofaces() {
        let map = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, map.tw, map.th);
        let mask = super::super::paint::paint(&mut z, &map);
        z.set_surface_tiles(super::super::paint::physical_surface(&z, &mask));
        let art = z.ground.clone();
        let physical = z.surface_tiles.clone();
        let native = z.ground_surfaces.clone();
        apply(&mut z, &map, &mask);
        assert_eq!(z.ground, art);
        assert_eq!(z.surface_tiles, physical);
        let (mut floor, mut shallow, mut filled, mut retained, mut relief) = (0, 0, 0, 0, 0);
        for y in 0..z.ph {
            for x in 0..z.pw {
                let i = (y * z.pw + x) as usize;
                if native[i].present() {
                    assert_eq!(z.ground_surfaces[i].height, native[i].height,
                        "microgeometry changed architectural altitude at {x},{y}");
                    if native[i].normal[2] <= 31 || material(&mask, z.pw, z.ph, x, y).is_none() {
                        assert_eq!(z.ground_surfaces[i], native[i], "steep macroface or water changed at {x},{y}");
                    }
                    retained += 1;
                }
                if z.ground[i].r < 0.0 || z.tile_at(x as f32, y as f32).solid() { continue; }
                let surface = z.ground_surfaces[i];
                assert!(surface.present(), "walkable pixel lacks a normal at {x},{y}");
                if mask[i] == mat::SHALLOW { shallow += 1; } else { floor += 1; }
                if !native[i].present() {
                    filled += 1;
                    if surface.normal[0] != 0 || surface.normal[1] != 0 { relief += 1; }
                }
            }
        }
        assert!(floor > 100_000 && shallow > 1000 && retained > 10_000 && filled > 1000 && relief > 100,
            "exercise actual dry/wet routes, source art and geometric seams: {floor}/{shallow}/{retained}/{filled}/{relief}");
        for (bx, by) in [(41 * 16, 16 * 16), (49 * 16, 36 * 16)] {
            let at = |x, y| z.ground_surfaces[(y * z.pw + x) as usize];
            assert!(at(bx + 20, by).normal[2] > 120, "worn curb coping still faces upward");
            assert_eq!(at(bx + 20, by + 1).normal, [0, 127, 0], "north curb's exposed front faces south");
            assert_eq!(at(bx + 20, by + 31).normal, [0, 127, 0], "bridge front faces south");
            assert!(at(bx + 20, by + 8).normal[2] > 100, "walkable deck faces upward");
        }
    }

    #[test]
    fn actual_native_grass_receives_visible_relief_once_and_keeps_its_pixels() {
        let map = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, map.tw, map.th);
        let mask = super::super::paint::paint(&mut z, &map);
        z.set_surface_tiles(super::super::paint::physical_surface(&z, &mask));
        let old = z.ground_surfaces.clone();
        let pixels = z.ground.clone();
        apply(&mut z, &map, &mask);
        assert_eq!(z.ground, pixels, "normal relief must not repaint baked color or lighting");
        let mut slopes = Vec::new();
        let mut facing = Vec::new();
        let mut native_flat_enriched = 0;
        for y in 0..z.ph { for x in 0..z.pw {
            let i = (y * z.pw + x) as usize;
            if z.ground[i].r < 0.0 || mask[i] != mat::GRASS
                || material(&mask, z.pw, z.ph, x, y) != Some(Material::Grass)
                || z.tile_at(x as f32, y as f32).solid() { continue; }
            let n = z.ground_surfaces[i].normal.map(|v| v as f32 / 127.0);
            slopes.push(n[0].hypot(n[1]));
            // Opposite oblique lights must reveal different sides of real turf.
            facing.push((n[0] * 1.12 + n[1] * 0.64).abs());
            if old[i].normal == [0, 0, 127] {
                native_flat_enriched += usize::from(n[0].hypot(n[1]) > 0.05);
            }
        }}
        slopes.sort_by(f32::total_cmp);
        facing.sort_by(f32::total_cmp);
        let size = slopes.len();
        let covered = slopes.iter().filter(|&&s| s > 0.05).count();
        println!("BLOOMWILD_GRASS_RELIEF: {size} pixels, {} flat native pixels enriched, median {:.4}, >.05 {:.2}%, opposing-light median {:.4}",
            native_flat_enriched, slopes[size / 2], covered as f32 * 100.0 / size as f32, facing[size / 2]);
        assert!(size > 100_000 && native_flat_enriched > 100_000);
        assert!(covered * 100 > size * 90 && slopes[size / 2] > 0.15);
        assert!(facing[size / 2] > 0.12, "grass still has no directional lighting response");

        // The two real constructors must compose exactly one material layer.
        // This catches accidental repeated passes without storing runtime markers.
        let (built, _) = super::super::build();
        for (x, y) in [(92, 686), (106, 694), (450, 670)] {
            let i = (y * z.pw + x) as usize;
            assert_eq!(built.ground_surfaces[i], z.ground_surfaces[i], "build applied relief twice at {x},{y}");
        }
    }

    #[test]
    fn title_garden_retains_normals_on_the_complete_headland() {
        let (z, _) = super::super::title_garden();
        let mut count = 0;
        for y in 0..z.ph {
            for x in 0..z.pw {
                let i = (y * z.pw + x) as usize;
                if z.ground[i].r < 0.0 || z.tile_at(x as f32, y as f32).solid() { continue; }
                assert!(z.ground_surfaces[i].present(), "title ground lacks a normal at {x},{y}");
                count += 1;
            }
        }
        assert!(count > 100_000);
    }
}
