//! Walkable material geometry retained alongside the procedural painter.
//! Relief changes shading only: it never raises collision or invents shadow casters.

use super::field::Grid;
use super::paint::{self, K_DAIS, Mat, walkable_kind};
use crate::gfx::lighting::TerrainSurface;
use crate::gfx::terrain_detail::{self, Material};
use crate::world::{TILE, Zone};

/// Turn the authored relief into floor normals, retaining every native atlas sample.
pub(super) fn fill(
    z: &mut Zone,
    kind: &Grid<u8>,
    relief: &Grid<f32>,
    crust: Option<&super::geology::Crust>,
) {
    z.fill_walkable_surfaces(|x, y| {
        let mut normal = sample(kind, relief, x, y);
        if let Some(crust) = crust {
            let slope = crust.normal(kind, x, y);
            normal[0] += slope[0];
            normal[1] += slope[1];
        }
        (normal, 0.0)
    });
}

/// Add material-scale relief after every native overlay has installed its own
/// geometry. Missing-only filling leaves both the ash and near-flat atlas
/// interiors smooth; this final pass retains their macroplanes and adds the
/// ash grains, scorched blades, worn earth or stone facets at world coordinates.
/// Native altitude is copied verbatim: detail cannot become a new shadow caster.
#[cfg(test)]
pub(super) fn detail(z: &mut Zone, kind: &Grid<u8>) {
    detail_with_crust(z, kind, None);
}

pub(super) fn detail_with_crust(
    z: &mut Zone,
    kind: &Grid<u8>,
    crust: Option<&super::geology::Crust>,
) {
    for y in 0..z.ph {
        for x in 0..z.pw {
            let i = (y * z.pw + x) as usize;
            if z.ground[i].r < 0.0 {
                continue;
            }
            let Some((material, fallback)) = material(kind.at(x, y)) else {
                continue;
            };
            let surface = z.ground_surfaces[i];
            // Missing non-walkable metadata is reconstructed by Field::geometry
            // with its actual broad face and height. Stamping a zero-height
            // normal here would replace that existing receiving geometry.
            if !surface.present() && !walkable_kind(kind.at(x, y)) {
                continue;
            }
            let base = if surface.present() {
                surface.normal.map(|n| n as f32 / 127.0)
            } else {
                fallback
            };
            let normal = if super::geology::dry(kind.at(x, y)) && crust.is_some() {
                crust.unwrap().finish_normal(x, y, base)
            } else if kind.at(x, y) == super::paint::K_PAVE {
                paving_normal(x, y, base)
            } else {
                terrain_detail::normal(material, x, y, base)
            };
            let mut detailed = TerrainSurface::new(normal, surface.height as f32);
            if surface.present() {
                detailed.upright = surface.upright;
            }
            z.ground_surfaces[i] = detailed;
        }
    }
    z.invalidate_lighting();
}

/// The exact 32x24 running bond used by paint_paving. Recessed grout and
/// authored cracks keep their existing source geometry, without new pores.
fn paving_normal(x: i32, y: i32, base: [f32; 3]) -> [f32; 3] {
    let row = y.div_euclid(24);
    let xx = x + row.rem_euclid(2) * 16;
    let (lx, ly) = (xx.rem_euclid(32), y.rem_euclid(24));
    let mortar = (ly == 23 && lx > 5) || (lx == 31 && ly > 7);
    let stone = crate::util::hash2(xx.div_euclid(32), row, 9423).to_bits();
    terrain_detail::masonry(x, y, base, stone, mortar)
}

fn material(kind: u8) -> Option<(Material, [f32; 3])> {
    use super::paint::*;
    let up = [0.0, 0.0, 1.0];
    let front = [0.0, 1.0, 0.0];
    Some(match kind {
        K_ASH => (Material::Ash, up),
        K_GRASS => (Material::Ash, up),
        K_PATH => (Material::Ash, up),
        K_PAVE | K_DECK | K_DAIS | K_WALLTOP => (Material::Stone, up),
        K_ROCKTOP => (Material::Rock, up),
        K_ROCKFACE | K_WALLFACE | K_ISLEFACE | K_BANK => (Material::Rock, front),
        // Only native hanging-cliff art can extend opaque pixels beyond the
        // old island silhouette. Backdrop pixels remain HOLE and are skipped.
        K_VOID => (Material::Rock, front),
        K_LAVA | K_FALL => return None,
        _ => return None,
    })
}

fn sample(kind: &Grid<u8>, relief: &Grid<f32>, x: i32, y: i32) -> [f32; 3] {
    // The one-pixel south riser belongs to the stairs' exact four-row painter.
    // Tread interiors stay horizontal, and every step rises toward the northern dais.
    if kind.at(x, y) == K_DAIS && paint::mat(x.div_euclid(TILE), y.div_euclid(TILE)) == Mat::Stairs
    {
        return if y.rem_euclid(4) == 3 {
            [0.0, 0.75, 0.66]
        } else {
            [0.0, 0.0, 1.0]
        };
    }
    let center = relief.at(x, y);
    let neighbor = |nx, ny| {
        // Never bend a floor toward the unrelated cliff, molten bank, or backdrop
        // beside it. Material relief is continuous only across its own footprint.
        if relief.inside(nx, ny)
            && walkable_kind(kind.at(nx, ny))
            && kind.at(nx, ny) == kind.at(x, y)
        {
            relief.at(nx, ny)
        } else {
            center
        }
    };
    let dx = (neighbor(x + 1, y) - neighbor(x - 1, y)) * 0.5;
    let dy = (neighbor(x, y + 1) - neighbor(x, y - 1)) * 0.5;
    [-dx, -dy, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{Rgb, lighting::TerrainSurface};
    use crate::regions::cinder::paint::{K_ASH, K_LAVA};
    use crate::world::{Tile, ZoneId};

    #[test]
    fn every_walkable_cinder_material_has_normals_and_native_uvs_survive() {
        use crate::regions::cinder::paint::{K_DECK, K_GRASS, K_PATH, K_PAVE};
        let (tw, th) = paint::map_size();
        let mut z = Zone::new(ZoneId::Cinder, tw, th);
        let painted = paint::paint(&mut z);
        for kind in [K_ASH, K_GRASS, K_PATH, K_PAVE, K_DECK, K_DAIS] {
            let mut count = 0;
            let mut tilted = 0;
            for (i, &k) in painted.kind.v.iter().enumerate() {
                if k != kind
                    || z.tile_at((i as i32 % z.pw) as f32, (i as i32 / z.pw) as f32) != Tile::Floor
                {
                    continue;
                }
                let surface = z.ground_surfaces[i];
                assert!(
                    surface.present(),
                    "missing material {kind} at {},{}",
                    i as i32 % z.pw,
                    i as i32 / z.pw
                );
                assert_eq!(surface.height, 0, "relief cannot lift the gameplay floor");
                count += 1;
                tilted += usize::from(surface.normal[0] != 0 || surface.normal[1] != 0);
            }
            assert!(
                count > 100 && tilted > 20,
                "material {kind} is only flat fallback: {tilted}/{count}"
            );
        }
        // Clean northern court lies outside every workshop overlay: retain the
        // authored source normal at the exact staggered UV used for albedo.
        let sheet = crate::gfx::sheet("cinder_floor").expect("embedded Cinder floor atlas");
        let frames = &sheet.cells[0];
        let mut checked = 0;
        for y in 400..436 {
            for x in 450..740 {
                if painted.kind.at(x, y) != K_PAVE {
                    continue;
                }
                let row = y.div_euclid(sheet.cell_h);
                let off = if row.rem_euclid(2) == 0 {
                    0
                } else {
                    sheet.cell_w / 2
                };
                let col = (x + off).div_euclid(sheet.cell_w);
                if crate::util::hash2(col, row, 305) > 0.34 {
                    continue;
                }
                let frame = ((crate::util::hash2(col, row, 307) * frames.len() as f32) as usize)
                    .min(frames.len() - 1);
                let (sx, sy) = (
                    (x + off).rem_euclid(sheet.cell_w),
                    y.rem_euclid(sheet.cell_h),
                );
                let cell = &frames[frame];
                if !cell.at(sx, sy).a {
                    continue;
                }
                let source = cell.surface[(sy * cell.w + sx) as usize];
                if !source.a {
                    continue;
                }
                let [nx, ny, nz] = source.n.map(|n| n as f32 / 127.0);
                assert_eq!(
                    z.ground_surfaces[(y * z.pw + x) as usize],
                    {
                        let base = TerrainSurface::new([nx, -ny, nz], 0.0);
                        TerrainSurface::new(
                            paving_normal(x, y, base.normal.map(|n| n as f32 / 127.0)),
                            0.0,
                        )
                    },
                    "native floor UV or material detail changed at {x},{y}"
                );
                checked += 1;
            }
        }
        assert!(
            checked > 100,
            "native floor preservation test missed the real atlas"
        );
        for y in 240..256 {
            let surface = z.ground_surfaces[(y * z.pw + 1144) as usize];
            if y.rem_euclid(4) == 3 {
                assert!(
                    surface.normal[1] > 80 && surface.normal[2] > 70,
                    "south riser turned away from the ascending approach"
                );
            } else {
                assert!(
                    surface.normal[2] > 110,
                    "material-scale tread wear must retain the horizontal macroplane"
                );
            }
        }
    }

    #[test]
    fn relief_follows_material_geometry_and_preserves_native_normals() {
        let mut z = Zone::new(ZoneId::Cinder, 2, 2);
        z.tiles.fill(Tile::Floor);
        z.ground.fill(Rgb::new(0.3, 0.2, 0.1));
        let mut kind = Grid::new(z.pw, z.ph, K_ASH);
        let mut relief = Grid::new(z.pw, z.ph, 0.0);
        // A geometric ridge faces away from its raised center, irrespective of albedo.
        for y in 0..z.ph {
            relief.set(16, y, 0.8);
        }
        let native = TerrainSurface::new([0.4, -0.1, 0.9], 0.0);
        z.ground_surfaces[(8 * z.pw + 8) as usize] = native;
        kind.set(3, 3, K_LAVA);
        z.set_tile(0, 0, Tile::Lava);
        let mut tiles = vec![Tile::Floor; (z.pw * z.ph) as usize];
        tiles[(3 * z.pw + 3) as usize] = Tile::Lava;
        z.set_surface_tiles(tiles);
        let albedo = z.ground.clone();
        let collision = z.surface_tiles.clone();
        fill(&mut z, &kind, &relief, None);
        assert!(z.ground_surfaces[(10 * z.pw + 15) as usize].normal[0] < -30);
        assert!(z.ground_surfaces[(10 * z.pw + 17) as usize].normal[0] > 30);
        assert_eq!(z.ground_surfaces[(8 * z.pw + 8) as usize], native);
        assert!(!z.ground_surfaces[(3 * z.pw + 3) as usize].present());
        assert_eq!(z.ground, albedo);
        assert_eq!(z.surface_tiles, collision);
        assert!(z.ground_surfaces.iter().all(|p| p.height == 0));
    }

    #[test]
    fn real_material_interiors_have_visible_relief_across_the_whole_cinder_map() {
        use crate::regions::cinder::paint::{K_DECK, K_GRASS, K_PATH, K_PAVE};
        let (tw, th) = paint::map_size();
        let mut z = Zone::new(ZoneId::Cinder, tw, th);
        let painted = paint::paint(&mut z);
        let mut medians = Vec::new();
        for k in [K_ASH, K_GRASS, K_PATH, K_PAVE, K_DECK, K_DAIS] {
            let mut slopes: Vec<_> = painted
                .kind
                .v
                .iter()
                .enumerate()
                .filter_map(|(i, &actual)| {
                    if actual != k
                        || z.ground[i].r < 0.0
                        || z.tile_at((i as i32 % z.pw) as f32, (i as i32 / z.pw) as f32)
                            != Tile::Floor
                    {
                        return None;
                    }
                    let [nx, ny, nz] = z.ground_surfaces[i].normal.map(|n| n as f32 / 127.0);
                    assert!(
                        z.ground_surfaces[i].present() && nx.is_finite() && ny.is_finite(),
                        "invalid material normal {k} at {i}"
                    );
                    assert!(nz > 0.0, "floor material {k} turned upside down at {i}");
                    Some(nx.hypot(ny))
                })
                .collect();
            assert!(slopes.len() > 100, "real material {k} missing");
            slopes.sort_by(f32::total_cmp);
            let median = slopes[slopes.len() / 2];
            let laid_stone = matches!(k, K_PAVE | K_DECK | K_DAIS);
            let threshold = if laid_stone { 0.015 } else { 0.06 };
            let visible =
                slopes.iter().filter(|&&s| s > threshold).count() as f32 / slopes.len() as f32;
            println!(
                "Cinder material {k}: pixels {}, median lateral {median:.4}, visibly tilted {visible:.3}",
                slopes.len()
            );
            assert!(
                median > if laid_stone { 0.02 } else { 0.065 } && visible > 0.52,
                "material {k} still has visually flat interiors: median {median:.4}, visible {visible:.3}"
            );
            if laid_stone {
                assert!(
                    median < 0.09,
                    "worn stone became as rough as ash or grass: {median}"
                );
            }
            medians.push(median);
        }
        assert!(
            medians[1] > medians[3] * 1.12,
            "scorched grass should have more blade relief than worn paving: {medians:?}"
        );
    }

    #[test]
    fn final_detail_preserves_opaque_masks_collision_and_all_cast_heights() {
        use crate::regions::cinder::paint::*;
        let mut z = Zone::new(ZoneId::Cinder, 2, 2);
        let mut kinds = Grid::new(z.pw, z.ph, K_ASH);
        let samples = [
            K_ASH, K_GRASS, K_PATH, K_PAVE, K_DECK, K_DAIS, K_ROCKTOP, K_ROCKFACE, K_WALLTOP,
            K_WALLFACE, K_ISLEFACE, K_BANK, K_VOID, K_LAVA, K_FALL,
        ];
        for (i, &k) in samples.iter().enumerate() {
            kinds.v[i] = k;
            z.ground_surfaces[i] = TerrainSurface::new(
                if matches!(k, K_ROCKFACE | K_WALLFACE | K_ISLEFACE | K_BANK) {
                    [0.0, 1.0, 0.0]
                } else {
                    [0.0, 0.0, 1.0]
                },
                (i * 7) as f32,
            );
        }
        z.ground[32] = crate::world::HOLE;
        for (offset, &k) in samples.iter().enumerate() {
            kinds.v[48 + offset] = k;
        }
        let opaque = z.ground.clone();
        let collision = z.tiles.clone();
        let before = z.ground_surfaces.clone();
        detail(&mut z, &kinds);
        assert_eq!(z.ground, opaque, "normal detail must not repaint terrain");
        assert_eq!(
            z.tiles, collision,
            "normal detail must not change collision"
        );
        assert!(
            before
                .iter()
                .zip(&z.ground_surfaces)
                .all(|(a, b)| a.height == b.height),
            "normal detail changed native cast geometry"
        );
        for (i, &k) in samples.iter().enumerate() {
            if matches!(
                k,
                K_ROCKFACE | K_WALLFACE | K_ISLEFACE | K_BANK | K_LAVA | K_FALL
            ) {
                assert_eq!(
                    z.ground_surfaces[i], before[i],
                    "macroface or molten surface {k} changed"
                );
            }
            if !walkable_kind(k) {
                assert_eq!(
                    z.ground_surfaces[48 + i],
                    before[48 + i],
                    "missing non-walkable {k} must retain its runtime macrogeometry"
                );
            }
        }
        assert_eq!(
            z.ground_surfaces[32], before[32],
            "transparent pixels gained normals"
        );
    }

    /// Exercise the actual normal renderer at identical exposure, albedo, light,
    /// camera and macrogeometry; nonzero metadata alone cannot pass this test.
    #[test]
    fn fixed_exposure_light_reveals_each_cinder_material_without_repainting_it() {
        use crate::gfx::{Frame, Grade, H, W, lighting, lighting_hook, post};
        use crate::regions::cinder::paint::{K_GRASS, K_PATH, K_PAVE};
        use crate::world::NoRegion;
        let mut z = Zone::new(ZoneId::Cinder, 24, 14);
        z.ground.fill(Rgb::gray(0.45));
        z.ambient = Rgb::gray(0.30);
        z.ground_surfaces
            .fill(TerrainSurface::new([0.0, 0.0, 1.0], 0.0));
        let mut kinds = Grid::new(z.pw, z.ph, K_ASH);
        for y in 0..z.ph {
            for x in 0..z.pw {
                kinds.set(x, y, [K_ASH, K_GRASS, K_PATH, K_PAVE][(x / 96) as usize]);
            }
        }
        let render = |zone: &Zone| {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, zone.ambient);
            f.surface_lighting = true;
            zone.draw_ground(&mut f);
            lighting::ground_surface(zone, &NoRegion, &mut f);
            f.add_light_3d_w(192.0, 108.0, 60.0, 300.0, Rgb::WHITE, 1.2);
            lighting_hook::apply(zone, &NoRegion, &mut f, 90);
            post::compose(
                &mut f,
                &Grade {
                    exposure: 1.0,
                    vignette: 0.0,
                    bloom: 0.0,
                    light_steps: 0.0,
                    ..Grade::default()
                },
            );
            f
        };
        let before = render(&z);
        detail(&mut z, &kinds);
        let after = render(&z);
        assert_eq!(before.albedo, after.albedo);
        for (stripe, name) in ["ash", "scorched grass", "worn earth", "obsidian paving"]
            .iter()
            .enumerate()
        {
            let mut delta = 0.0;
            let mut visible = 0;
            let mut count = 0;
            for y in 16..H - 16 {
                for x in stripe * 96 + 8..(stripe + 1) * 96 - 8 {
                    let i = y * W + x;
                    let d = (after.light[i].luma() - before.light[i].luma()).abs();
                    delta += d;
                    visible += usize::from(after.out[i][0].abs_diff(before.out[i][0]) >= 2);
                    count += 1;
                }
            }
            let mean = delta / count as f32;
            let fraction = visible as f32 / count as f32;
            println!(
                "Cinder {name} fixed-exposure relief: light delta {mean:.5}, >=2-byte pixels {fraction:.3}"
            );
            assert!(
                mean > 0.008 && fraction > 0.30,
                "{name} normal metadata did not produce visible relief: {mean:.5}, {fraction:.3}"
            );
        }
        if let Ok(directory) = std::env::var("GLINT_CINDER_DETAIL_CAPTURE") {
            let directory = std::path::Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            for (name, f) in [("before-flat", before), ("after-material-relief", after)] {
                crate::assets::write_png(
                    &directory.join(format!("cinder-{name}.png")),
                    (W * 3) as u32,
                    (H * 3) as u32,
                    &post::composite_scaled(&f, 3),
                )
                .unwrap();
            }
        }
    }
}
