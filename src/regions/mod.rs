//! One file per region. Each exposes `pub fn build() -> (Zone, Box<dyn Region>)`: it lays out
//! the tile grid, paints the pre-rendered ground, places props, lights and enemy spawns, and
//! returns the region's live behavior (grass, water, lava, weather...).

pub mod bloomwild;
pub mod cinder;
pub mod glasswater;
pub mod summit;

use crate::gfx::{Rgb, rgb};
use crate::util::{bayer4, fbm};
use crate::world::{Region, TILE, Tile, Zone, ZoneId};

pub fn build(id: ZoneId) -> (Zone, Box<dyn Region>) {
    let (zone, region) = match id {
        ZoneId::Bloomwild => bloomwild::build(),
        ZoneId::Glasswater => glasswater::build(),
        ZoneId::Cinder => cinder::build(),
        ZoneId::Summit => summit::build(),
    };
    crate::gfx::lighting::prepare(&zone, region.as_ref());
    (zone, region)
}

/// A plain walled rectangle with a noisy two-tone floor. Used by the placeholder regions.
pub fn placeholder(id: ZoneId, title: &str, floor: (u32, u32), wall: u32, ambient: Rgb, music: &str) -> Zone {
    let (tw, th) = (60, 34);
    let mut z = Zone::new(id, tw, th);
    z.title = title.to_string();
    z.music = music.to_string();
    z.ambient = ambient;
    z.bg = rgb(0x05040c);
    for ty in 0..th {
        for tx in 0..tw {
            let edge = tx == 0 || ty == 0 || tx == tw - 1 || ty == th - 1;
            let rock = fbm(tx as f32 * 0.25, ty as f32 * 0.25, 3, 7) > 0.68;
            z.set_tile(tx, ty, if edge || rock { Tile::Wall } else { Tile::Floor });
        }
    }
    let (a, b) = (rgb(floor.0), rgb(floor.1));
    let w = rgb(wall);
    for y in 0..z.ph {
        for x in 0..z.pw {
            let t = z.tile(x / TILE, y / TILE);
            let c = if t == Tile::Wall {
                w.scale(0.7 + 0.3 * ((y % TILE) as f32 / TILE as f32))
            } else {
                let n = fbm(x as f32 * 0.05, y as f32 * 0.05, 3, 1);
                if n > bayer4(x, y) * 0.3 + 0.45 { a } else { b }
            };
            z.gset(x, y, c);
        }
    }
    z.player_start = (5.0 * TILE as f32, (th / 2) as f32 * TILE as f32);
    z.gate = Some(((tw - 4) as f32 * TILE as f32, (th / 2) as f32 * TILE as f32));
    z.shard = Some(((tw - 8) as f32 * TILE as f32, (th / 2) as f32 * TILE as f32));
    z
}

#[cfg(test)]
mod normal_coverage_tests {
    use super::*;

    /// Audit the finished native movement mask, including painter-created margins
    /// and crossings that cannot be checked through the coarse navigation grid.
    #[test]
    fn every_walkable_pixel_has_a_retained_normal_in_every_region() {
        for id in [ZoneId::Bloomwild, ZoneId::Glasswater, ZoneId::Cinder, ZoneId::Summit] {
            let (zone, _) = build(id);
            let mut walkable = 0;
            let mut missing = 0;
            let mut first_missing = None;
            for y in 0..zone.ph {
                for x in 0..zone.pw {
                    let i = (y * zone.pw + x) as usize;
                    if zone.ground[i] == crate::world::HOLE || zone.tile_at(x as f32, y as f32).solid() {
                        continue;
                    }
                    walkable += 1;
                    let surface = zone.ground_surfaces[i];
                    if !surface.present() {
                        missing += 1;
                        first_missing.get_or_insert((x, y));
                        continue;
                    }
                    let length = surface.normal.iter().map(|&n| (n as f32 / 127.0).powi(2)).sum::<f32>().sqrt();
                    assert!((length - 1.0).abs() < 0.015, "{id:?} invalid normal at ({x},{y}): {:?}", surface.normal);
                }
            }
            println!("WALKABLE_NORMALS {id:?}: {walkable} pixels, {missing} missing");
            assert!(walkable > 10_000, "audit must cover real region geometry");
            assert_eq!(missing, 0, "{id:?} missing walkable normals, first at {first_missing:?}");
        }
    }

    /// A populated UP vector is still flat. Check finished, native-quantized
    /// material normals after every overlay, not just the presence of metadata.
    #[test]
    fn dry_terrain_has_material_relief_after_all_region_builders() {
        for id in [ZoneId::Bloomwild, ZoneId::Glasswater, ZoneId::Cinder, ZoneId::Summit] {
            let (zone, _) = build(id);
            let mut slopes = Vec::new();
            let mut incidence = Vec::new();
            for y in 0..zone.ph {
                for x in 0..zone.pw {
                    let i = (y * zone.pw + x) as usize;
                    if zone.ground[i].r < 0.0 || zone.tile_at(x as f32, y as f32) != Tile::Floor {
                        continue;
                    }
                    let n = zone.ground_surfaces[i].normal.map(|n| n as f32 / 127.0);
                    if n[2] < 0.6 { continue; } // Authored upright risers are a separate plane.
                    slopes.push(n[0].hypot(n[1]));
                    incidence.push(n[0] * -0.56 + n[1] * -0.32 + n[2] * 0.76);
                }
            }
            slopes.sort_by(f32::total_cmp);
            incidence.sort_by(f32::total_cmp);
            let count = slopes.len();
            assert!(count > 100_000, "audit must sample the real region");
            // Coverage must detect quiet wear as well as rough rock. Two native
            // normal steps are visible geometry; requiring every material to
            // exceed rough-stone amplitudes fights intentionally shallow masonry.
            // Material-specific contrast bounds live in terrain_detail and the
            // region normal tests; this check catches missing/flat final overlays.
            let textured = slopes.iter().filter(|&&v| v >= 2.0 / 127.0).count();
            let median = slopes[count / 2];
            let span = incidence[count * 95 / 100] - incidence[count * 5 / 100];
            println!("TERRAIN_RELIEF {id:?}: {count} dry top pixels, {:.2}% textured, median {median:.4}, directional span {span:.4}", textured as f32 * 100.0 / count as f32);
            assert!(textured * 100 > count * 70, "{id:?} still has broad flat ground");
            assert!(median > 0.025 && span > 0.06, "{id:?} has ineffective material relief");
        }
    }
}
