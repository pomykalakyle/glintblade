//! Broad indirect illumination for retained terrain normals. The room/sky fill
//! has a diffuse upper-hemisphere bias, rather than depending on a nearby actor
//! or point source to reveal material relief. No extra energy or shadow caster
//! is introduced: a horizontal receiver retains its authored fill exactly.

use super::Frame;
use crate::world::{Zone, ZoneId};

#[derive(Clone, Copy)]
struct Hemisphere {
    direction: [f32; 3],
    isotropic: f32,
    directional: f32,
    min: f32,
    max: f32,
}

impl Hemisphere {
    const SKY: Self = Self {
        direction: [-0.45, -0.35, 0.82],
        isotropic: 0.45,
        directional: 0.55,
        min: 0.82,
        max: 1.16,
    };

    // The cave ceiling, ash haze and night sky provide broad indirect fill.
    // Its off-axis component lets shallow mineral facets read independently
    // of a practical light. The isotropic part keeps this a diffuse lobe;
    // it has neither a hard terminator nor the sun's moving cast shadows.
    const INDIRECT: Self = Self {
        direction: [-0.60, -0.45, 0.66],
        isotropic: 0.65,
        directional: 0.35,
        min: 0.86,
        max: 1.14,
    };

    #[inline]
    fn response(self, normal: [f32; 3]) -> f32 {
        let dot = (self.direction[0] * normal[0]
            + self.direction[1] * normal[1]
            + self.direction[2] * normal[2])
            .max(0.0);
        ((self.isotropic + self.directional * dot)
            / (self.isotropic + self.directional * self.direction[2]))
            .clamp(self.min, self.max)
    }
}

/// Shape only the existing terrain fill, before source-local direct lights.
/// Owner zero includes raised terrain caps as well as the ground plane; sprite
/// owners, actors, preserved ink, vertical walls and the backdrop keep their
/// existing lighting. In particular a bright Bloomwild night does not turn on
/// a daylight proxy merely because its ambient color happens to be bright.
pub(crate) fn apply(zone: Option<&Zone>, frame: &mut Frame) {
    if !frame.surface_lighting {
        return;
    }
    let hemisphere = if frame.normal_sun {
        Hemisphere::SKY
    } else if zone.is_some_and(|zone| {
        matches!(
            zone.id,
            ZoneId::Glasswater | ZoneId::Cinder | ZoneId::Summit
        )
    }) {
        Hemisphere::INDIRECT
    } else {
        return;
    };
    for (surface, light) in frame.surfaces.iter().zip(&mut frame.light) {
        if surface.owner == 0 && matches!(surface.kind, 1 | 2) && surface.normal[2] > 0.5 {
            *light = light.scale(hemisphere.response(surface.normal));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{H, Rgb, W, frame::Surface, lighting, post};
    use crate::world::Tile;

    #[test]
    fn indirect_fill_reveals_facets_without_changing_flat_exposure_or_nonterrain() {
        for id in [ZoneId::Glasswater, ZoneId::Cinder, ZoneId::Summit] {
            let zone = Zone::new(id, 2, 2);
            let mut frame = Frame::new();
            let fill = zone.lighting_settings.fill * 0.82;
            frame.begin(Rgb::gray(0.3), fill);
            frame.surface_lighting = true;
            for (i, normal) in [
                [-0.25, 0.0, 0.9375_f32.sqrt()],
                [0.25, 0.0, 0.9375_f32.sqrt()],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0],
            ]
            .into_iter()
            .enumerate()
            {
                frame.surfaces[i] = Surface {
                    normal,
                    kind: 1,
                    ..Surface::default()
                };
            }
            // Raised caps are terrain even though they share scenery's kind.
            frame.surfaces[4] = Surface {
                normal: frame.surfaces[0].normal,
                kind: 2,
                height: 30.0,
                ..Surface::default()
            };
            for (i, kind, owner) in [(5, 2, 1), (6, 3, 0), (7, 4, 0), (8, 0, 0)] {
                frame.surfaces[i] = Surface {
                    normal: frame.surfaces[0].normal,
                    kind,
                    owner,
                    ..Surface::default()
                };
            }
            let albedo = frame.albedo.clone();
            let emissive = frame.emissive.clone();
            let surfaces = frame
                .surfaces
                .iter()
                .map(|s| (s.normal, s.height, s.owner, s.kind))
                .collect::<Vec<_>>();
            apply(Some(&zone), &mut frame);
            assert!(
                frame.light[0].luma() - frame.light[1].luma() > 0.025,
                "{id:?}: indirect material response must survive native light quantization"
            );
            assert_eq!(frame.light[2], fill, "flat exposure changes in {id:?}");
            assert_eq!(frame.light[3], fill, "existing wall form changes in {id:?}");
            assert_eq!(
                frame.light[4], frame.light[0],
                "raised terrain loses its relief in {id:?}"
            );
            assert!(frame.light[5..=8].iter().all(|light| *light == fill));
            assert!(frame.point_lights.is_empty() && !frame.normal_sun);
            assert_eq!(frame.albedo, albedo);
            assert_eq!(frame.emissive, emissive);
            assert_eq!(
                frame
                    .surfaces
                    .iter()
                    .map(|s| (s.normal, s.height, s.owner, s.kind))
                    .collect::<Vec<_>>(),
                surfaces
            );
        }
    }

    #[test]
    fn day_fill_retains_existing_response_and_night_does_not_invent_a_sun() {
        let mut frame = Frame::new();
        frame.begin(Rgb::WHITE, Rgb::gray(0.5));
        frame.surface_lighting = true;
        frame.surfaces[0] = Surface {
            normal: [-0.25, 0.0, 0.9375_f32.sqrt()],
            kind: 1,
            ..Surface::default()
        };
        let zone = Zone::new(ZoneId::Bloomwild, 1, 1);
        apply(Some(&zone), &mut frame);
        assert_eq!(frame.light[0], Rgb::gray(0.5));
        apply(None, &mut frame);
        assert_eq!(frame.light[0], Rgb::gray(0.5));
        frame.normal_sun = true;
        let n = frame.surfaces[0].normal;
        let old_sky = ((0.45 + 0.55 * (-0.45 * n[0] - 0.35 * n[1] + 0.82 * n[2]).max(0.0))
            / (0.45 + 0.55 * 0.82))
            .clamp(0.82, 1.16);
        apply(Some(&zone), &mut frame);
        assert_eq!(frame.light[0], Rgb::gray(0.5).scale(old_sky));
    }

    #[test]
    fn real_glasswater_floor_has_visible_relief_with_every_point_emitter_absent() {
        let (zone, region) = crate::regions::build(ZoneId::Glasswater);
        let mut frame = Frame::new();
        frame.begin(zone.bg, zone.ambient);
        frame.surface_lighting = true;
        // The gallery is far from the entry/hero. Use its actual painted
        // materials, normals, cached contact shade, production fill and grade;
        // no hero, scenery emitter, crystal or temporary light is submitted.
        frame.cam_x = 480;
        frame.cam_y = 480;
        zone.draw_ground(&mut frame);
        region.draw_ground(&zone, &mut frame, 90);
        lighting::ground_surface(&zone, region.as_ref(), &mut frame);
        lighting::apply(&zone, region.as_ref(), &mut frame, 90);
        assert!(!frame.normal_sun && frame.point_lights.is_empty());
        let original_light = frame.light.clone();
        let mut grade = zone.grade;
        grade.vignette = 0.0;
        post::compose(&mut frame, &grade);
        let original_out = frame.out.clone();
        apply(Some(&zone), &mut frame);
        post::compose(&mut frame, &grade);
        let (mut pixels, mut visible, mut delta, mut old_exposure, mut new_exposure) =
            (0, 0, 0.0, 0.0, 0.0);
        let mut ratios = Vec::new();
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                let surface = frame.surfaces[i];
                if surface.kind != 1
                    || surface.normal[2] <= 0.5
                    || zone.tile_at(
                        (x as i32 + frame.cam_x) as f32,
                        (y as i32 + frame.cam_y) as f32,
                    ) != Tile::Floor
                {
                    continue;
                }
                pixels += 1;
                let change = (0..3)
                    .map(|c| (i32::from(frame.out[i][c]) - i32::from(original_out[i][c])).abs())
                    .max()
                    .unwrap();
                visible += usize::from(change >= 1);
                delta += change as f32;
                old_exposure += original_light[i].luma();
                new_exposure += frame.light[i].luma();
                ratios.push(frame.light[i].luma() / original_light[i].luma());
            }
        }
        ratios.sort_by(f32::total_cmp);
        let mean_delta = delta / pixels as f32;
        let exposure_ratio = new_exposure / old_exposure;
        let contrast = ratios[ratios.len() * 95 / 100] - ratios[ratios.len() * 5 / 100];
        println!(
            "UNLIT_GLASSWATER pixels={pixels}, RGB8-visible={visible}, mean max-channel delta={mean_delta:.3}, mean fill ratio={exposure_ratio:.4}, p95-p05 response={contrast:.4}"
        );
        assert!(pixels > 10_000, "exercise a substantial real unlit floor");
        assert!(
            visible * 100 > pixels * 35 && mean_delta > 0.75,
            "terrain relief still disappears in the actual final RGB8 output"
        );
        assert!(
            contrast > 0.07,
            "indirect response is too flat across real mineral facets"
        );
        assert!(
            (0.95..=1.02).contains(&exposure_ratio),
            "indirect relief changes the room exposure"
        );
        if let Ok(directory) = std::env::var("GLINT_AMBIENT_PROOF") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            for (name, pixels) in [
                ("glasswater-no-emitters-before.png", &original_out),
                ("glasswater-no-emitters-after.png", &frame.out),
            ] {
                let rgba: Vec<_> = pixels.iter().flatten().copied().collect();
                crate::assets::write_png(&directory.join(name), W as u32, H as u32, &rgba).unwrap();
            }
        }
    }
}
