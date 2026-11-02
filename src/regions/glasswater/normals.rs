//! Mineral relief across the complete dry cave, including the quiet native
//! atlas interiors. The painted rock masks, never palette values, select the
//! material; retained macro normals and structural heights remain authoritative.

use super::paint::{FACE, FLOOR, Fields, PixMap, VOID, WALL_TOP};
use crate::gfx::lighting::TerrainSurface;
use crate::gfx::terrain_detail::{self, Material};
use crate::world::Zone;

pub(super) fn apply(zone: &mut Zone, map: &PixMap, fields: &Fields) {
    for y in 0..map.h {
        for x in 0..map.w {
            let i = (y * map.w + x) as usize;
            // Real holes include the space cleared below the shallow bridge
            // arch. Neither that space nor the animated water is stone.
            if zone.ground[i].r < 0.0 {
                continue;
            }
            let Some(material) = material_at(map, fields, x, y) else {
                continue;
            };
            let source = zone.ground_surfaces[i];
            let base = if source.present() {
                source.normal.map(|n| n as f32 / 127.0)
            } else if matches!(map.cls[i], FACE | VOID) {
                [0.0, 1.0, 0.0]
            } else {
                [0.0, 0.0, 1.0]
            };
            let normal = if material == Material::Stone {
                let (stone, mortar) = bridge_stone(x, y);
                terrain_detail::masonry(x, y, base, stone, mortar)
            } else { terrain_detail::normal(material, x, y, base) };
            let mut surface = TerrainSurface::new(normal, source.height as f32);
            if source.present() { surface.upright = source.upright; }
            zone.ground_surfaces[i] = surface;
        }
    }
    zone.invalidate_lighting();
}

/// Same running bonds and nine-pixel landing blocks as paint::bridge.
fn bridge_stone(x: i32, y: i32) -> (u32, bool) {
    let (lx, ly) = (x - 47 * 16, y - 15 * 16);
    if lx < 9 || lx >= 12 * 16 - 9 {
        let row = ly.div_euclid(11);
        return (crate::util::hash2(lx.div_euclid(9), row, 9421).to_bits(), ly.rem_euclid(11) == 10);
    }
    let row = ly.div_euclid(8);
    let bond = lx + row * 11;
    (crate::util::hash2(bond.div_euclid(23), row, 9422).to_bits(),
        ly.rem_euclid(8) == 7 || bond.rem_euclid(23) == 22 || ly == 0 || ly == 29)
}

fn material_at(map: &PixMap, fields: &Fields, x: i32, y: i32) -> Option<Material> {
    let i = (y * map.w + x) as usize;
    let dry = match map.cls[i] {
        FLOOR | WALL_TOP | FACE => true,
        VOID => map.depth[i] < map.span[i] && map.ledge[i] != 2,
        _ => false,
    };
    if !dry {
        return None;
    }
    // The bridge's deck, thresholds, curbs and visible soffit all share built
    // masonry. The original painter still owns every bond bevel and height.
    if (47 * 16..59 * 16).contains(&x) && (15 * 16..17 * 16 + 38).contains(&y) {
        return Some(Material::Stone);
    }
    Some(if map.cls[i] == FLOOR && fields.d_water[i] <= 13 {
        Material::WetStone
    } else {
        Material::Rock
    })
}

#[cfg(test)]
mod tests {
    use super::super::paint::{self, DEEP, SHALLOW};
    use super::*;
    use crate::gfx::rgb;
    use crate::world::{HOLE, ZoneId};

    #[test]
    fn all_dry_materials_have_light_reactive_relief_without_changing_geometry() {
        let (mut zone, _) = super::super::build();
        let map = paint::classify(&zone);
        let fields = paint::fields(&map);
        let mut count = [0usize; 3];
        let mut relief = [0usize; 3];
        let mut response = [0.0f32; 3];
        for y in 0..map.h {
            for x in 0..map.w {
                let i = (y * map.w + x) as usize;
                if map.cls[i] != FLOOR {
                    continue;
                }
                let bucket = match material_at(&map, &fields, x, y) {
                    Some(Material::Rock) => 0,
                    Some(Material::WetStone) => 1,
                    Some(Material::Stone) => 2,
                    _ => continue,
                };
                let n = zone.ground_surfaces[i].normal.map(|v| v as f32 / 127.0);
                count[bucket] += 1;
                let threshold = match bucket { 0 => 0.06, 1 => 0.025, _ => 0.015 };
                if n[0].hypot(n[1]) > threshold {
                    relief[bucket] += 1;
                }
                // Two opposite physical light positions over the same rock
                // must reveal different facets without touching its albedo.
                response[bucket] += (n[0] * 1.6).powi(2);
            }
        }
        for bucket in 0..3 {
            assert!(
                count[bucket] > 1_000,
                "exercise real dry/wet/bridge masks: {count:?}"
            );
            assert!(
                relief[bucket] * 100 > count[bucket] * 60,
                "material {bucket} still looks like a flat receiving plane: {relief:?}/{count:?}"
            );
            let rms = (response[bucket] / count[bucket] as f32).sqrt();
            assert!(
                rms > if bucket == 0 { 0.055 } else { 0.025 },
                "material {bucket} has imperceptible directional response: {rms}"
            );
        }
        assert!(response[1] / (count[1] as f32) < response[0] / (count[0] as f32) * 0.35,
            "water-worn receiving banks must remain smoother than dry cave rock");

        let albedo = zone.ground.clone();
        let collision = zone.surface_tiles.clone();
        let heights: Vec<_> = zone.ground_surfaces.iter().map(|s| s.height).collect();
        let protected = zone.ground_surfaces.clone();
        // Give every eligible cap a neutral native plane: merely possessing
        // source metadata must never opt a quiet atlas interior out of detail.
        for i in 0..map.cls.len() {
            if matches!(map.cls[i], FLOOR | WALL_TOP) && zone.ground[i].r >= 0.0 {
                zone.ground_surfaces[i].normal = [0, 0, 127];
            }
        }
        apply(&mut zone, &map, &fields);
        assert_eq!(
            zone.ground, albedo,
            "material normals alter no painted pixels"
        );
        assert_eq!(
            zone.surface_tiles, collision,
            "material normals alter no movement"
        );
        for i in 0..map.cls.len() {
            assert_eq!(
                zone.ground_surfaces[i].height, heights[i],
                "structural height changed at {i}"
            );
            if matches!(map.cls[i], SHALLOW | DEEP) || albedo[i] == HOLE {
                assert_eq!(
                    zone.ground_surfaces[i], protected[i],
                    "water/hole metadata changed at {i}"
                );
            }
        }
    }

    #[test]
    fn actual_bridge_mortar_retains_its_authored_planes_after_material_wear() {
        let (mut z, _) = super::super::build();
        let map = paint::classify(&z);
        let fields = paint::fields(&map);
        paint::bridge(&mut z, &map);
        let before = z.ground_surfaces.clone();
        apply(&mut z, &map, &fields);
        let mut mortar = 0;
        for y in 242..269 { for x in 762..934 {
            if !bridge_stone(x, y).1 { continue; }
            let i = (y * z.pw + x) as usize;
            assert_eq!(z.ground_surfaces[i], before[i], "wear crossed the authored deck mortar at {x},{y}");
            mortar += 1;
        }}
        assert!(mortar > 500, "sample the physical bridge's horizontal and staggered joints");
    }

    #[test]
    fn light_response_is_independent_of_painted_color_and_preserves_native_faces() {
        let mut a = Zone::new(ZoneId::Glasswater, 8, 8);
        let mut b = Zone::new(ZoneId::Glasswater, 8, 8);
        a.ground.fill(rgb(0x536875));
        for (i, color) in b.ground.iter_mut().enumerate() {
            *color = if i % 2 == 0 {
                rgb(0x132130)
            } else {
                rgb(0xa7c4c8)
            };
        }
        let n = a.ground.len();
        let mut map = PixMap {
            w: a.pw,
            h: a.ph,
            cls: vec![FLOOR; n],
            depth: vec![0; n],
            span: vec![0; n],
            ledge: vec![0; n],
        };
        let native = TerrainSurface::new([0.45, 0.89, 0.0], 23.0);
        for y in 8..12 {
            for x in 0..map.w {
                let i = (y * map.w + x) as usize;
                map.cls[i] = FACE;
                a.ground_surfaces[i] = native;
                b.ground_surfaces[i] = native;
            }
        }
        let fields = paint::fields(&map);
        let colors_a = a.ground.clone();
        let colors_b = b.ground.clone();
        apply(&mut a, &map, &fields);
        apply(&mut b, &map, &fields);
        assert_eq!(a.ground_surfaces, b.ground_surfaces);
        assert_eq!(a.ground, colors_a);
        assert_eq!(b.ground, colors_b);
        for i in 0..n {
            if map.cls[i] == FACE {
                assert_eq!(a.ground_surfaces[i], native);
            }
        }
        let moving_light_response = a
            .ground_surfaces
            .iter()
            .filter(|s| s.height == 0)
            .map(|s| (s.normal[0] as f32 / 127.0 * 1.6).powi(2))
            .sum::<f32>()
            / (n - 4 * map.w as usize) as f32;
        assert!(
            moving_light_response.sqrt() > 0.055,
            "a moving light must reveal relief even on a solid-color floor"
        );
    }
}
