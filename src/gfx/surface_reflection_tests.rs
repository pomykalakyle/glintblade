use super::*;
use crate::gfx::{Rgb, sprite::DrawOpts};

fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = v.iter().map(|n| n * n).sum::<f32>().sqrt();
    v.map(|n| n / length)
}

fn fixture(reflectivity: f32) -> Frame {
    let mut f = Frame::new();
    f.begin(Rgb::BLACK, Rgb::BLACK);
    f.surface_lighting = true;
    let cell = Cell::from_ascii(&["xxe"], &[('x', Rgb::new(0.4, 0.3, 0.5)), ('e', Rgb::WHITE)], &['e']);
    f.sprite(&cell, (0, 0), 80, 100, &DrawOpts { reflectivity, ..Default::default() });
    let view = std::f32::consts::FRAC_1_SQRT_2;
    for x in 80..82 {
        f.surfaces[100 * W + x].normal = unit([0.6, 0.8 + view, view]);
    }
    f
}

#[test]
fn reflection_follows_native_face_and_source_without_a_time_or_camera_glint() {
    let mut f = fixture(0.22);
    let face = f.surfaces[100 * W + 80];
    let left = Surface { normal: [-face.normal[0], face.normal[1], face.normal[2]], ..face };
    let right_light = [24.0, 32.0, 0.0];
    let left_light = [-24.0, 32.0, 0.0];
    let bright = point_reflection(&f, face, right_light);
    assert!(bright > 0.15 && bright <= 0.22);
    assert!(point_reflection(&f, left, right_light) < bright * 0.01);
    assert!((point_reflection(&f, left, left_light) - bright).abs() < 0.00001);
    assert_eq!(point_reflection(&f, face, right_light.map(|v| -v)), 0.0);
    assert!((point_reflection(&f, face, [0.06, 0.08, 0.0]) - bright).abs() < 0.00001,
        "a subpixel source retains its direction");
    f.cam_x = 1307;
    f.cam_y = 761;
    f.t = 9876;
    assert_eq!(point_reflection(&f, face, right_light), bright);
    for kind in [0, 1, 3, 4, 5] {
        assert_eq!(point_reflection(&f, Surface { kind, ..face }, right_light), 0.0);
    }
    assert_eq!(point_reflection(&f, Surface { owner: 0, ..face }, right_light), 0.0);
    f.surface_casters[0].reflectivity = 0.0;
    assert_eq!(point_reflection(&f, face, right_light), 0.0);
}

#[test]
fn reflection_needs_reaching_light_and_preserves_art_emission_and_other_materials() {
    use crate::gfx::frame::PointLight;
    for count in [0, 1, 7] {
        let mut matte = fixture(0.0);
        let mut crystal = fixture(0.22);
        for f in [&mut matte, &mut crystal] {
            if count > 0 {
                f.point_lights.push(PointLight { x: 104.5, y: 132.5, height: 0.5,
                    radius: 80.0, color: Rgb::new(0.7, 0.8, 1.0), intensity: 0.8 });
            }
            for _ in 1..count {
                f.point_lights.push(PointLight { x: -1000.0, y: -1000.0, height: 12.0,
                    radius: 2.0, color: Rgb::WHITE, intensity: 1.0 });
            }
            apply_points(None, f);
        }
        assert_eq!(matte.albedo, crystal.albedo);
        assert_eq!(matte.emissive, crystal.emissive);
        let sample = 100 * W + 80;
        if count == 0 { assert_eq!(matte.light, crystal.light); }
        else {
            let gain = crystal.light[sample].b - matte.light[sample].b;
            assert!(gain > 0.07 && gain < 0.10, "bounded local highlight: {gain}");
        }
        for i in 0..W * H {
            if i != sample && i != sample + 1 {
                assert_eq!(matte.light[i], crystal.light[i], "reflection escaped visible receiving faces at {i}");
            }
        }
    }
}

#[test]
fn opaque_native_shadow_blocks_reflection_with_the_diffuse_light() {
    use crate::gfx::frame::PointLight;
    let mut f = fixture(0.22);
    let blocker = Cell::from_ascii(&["xxxxxxxx"; 8], &[('x', Rgb::WHITE)], &[]);
    f.sprite(&blocker, (3, 7), 100, 100, &DrawOpts::default());
    let sample_cell = Cell::from_ascii(&["x"], &[('x', Rgb::WHITE)], &[]);
    f.sprite(&sample_cell, (0, 0), 112, 100, &DrawOpts { reflectivity: 0.22, ..Default::default() });
    let sample = 100 * W + 112;
    let view = std::f32::consts::FRAC_1_SQRT_2;
    let l = unit([-26.0, -0.5, 11.5]);
    f.surfaces[sample].normal = unit([l[0], l[1] + view, l[2] + view]);
    let light = PointLight { x: 86.5, y: 88.5, height: 12.0, radius: 60.0, color: Rgb::WHITE, intensity: 1.0 };
    assert_eq!(super::super::lighting::point_visibility(&f, light)[sample], 0);
    assert!(point_reflection(&f, f.surfaces[sample], [-26.0, -0.5, 11.5]) > 0.1);
    f.point_lights.push(light);
    apply_points(None, &mut f);
    assert_eq!(f.light[sample], Rgb::BLACK);
}
