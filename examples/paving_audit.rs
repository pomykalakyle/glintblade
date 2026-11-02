//! Inspect the actual scene receivers, albedo, and moving-sun render together.
//! cargo run --release --example paving_audit -- out/paving-audit
use glintblade::{assets::write_png, fx::Fx, gfx::{Frame, W, H, post}, regions,
    view::render_world, world::{RegionCtx, ZoneId}};
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or("out/paving-audit".into()));
    std::fs::create_dir_all(&out).unwrap();
    let mut records = Vec::new();
    for (name, pos) in [("altar", (408.0, 176.0)), ("entry", (258.0, 640.0)),
        ("temple", (1056.0, 464.0)), ("upper-bridge", (632.0, 274.0)),
        ("lower-bridge", (840.0, 590.0))] {
        let (mut zone, mut region) = regions::build(ZoneId::Bloomwild);
        let cam = ((pos.0 as i32 - W as i32 / 2).clamp(0, zone.pw - W as i32),
            (pos.1 as i32 - H as i32 / 2).clamp(0, zone.ph - H as i32));
        let mut fx = Fx::new(1);
        let mut f = Frame::new();
        let mut first_normals = Vec::new();
        for t in 0..=2250 {
            region.update(&mut zone, &RegionCtx { t, player: pos, movers: &[], cam }, &mut fx);
            fx.particles.update(&zone);
            for p in &mut zone.props { p.tick(); }
            fx.tick();
            if [90, 630, 1170, 1710, 2250].contains(&t) {
                f.cam_x = cam.0;
                f.cam_y = cam.1;
                render_world(&zone, region.as_ref(), &fx, &mut f, t, &[], &mut |_, _| {}, None);
                assert!(f.normal_sun && f.surface_lighting);
                for s in &f.surfaces {
                    assert!(s.normal.iter().all(|v| v.is_finite()));
                    if s.kind != 0 {
                        let length = s.normal.iter().map(|v| v * v).sum::<f32>().sqrt();
                        assert!((0.97..1.03).contains(&length));
                    }
                }
                if t == 90 {
                    let normal: Vec<u8> = f.surfaces.iter().flat_map(|s| if s.kind == 0 {
                        [0, 0, 0, 255]
                    } else {
                        [(s.normal[0] * 127.0 + 128.0) as u8,
                         (s.normal[1] * 127.0 + 128.0) as u8,
                         (s.normal[2] * 127.0 + 128.0) as u8, 255]
                    }).collect();
                    write_png(&out.join(format!("{name}-normals.png")), W as u32, H as u32, &normal).unwrap();
                    let albedo: Vec<u8> = f.albedo.iter().flat_map(|c| { let [r, g, b] = c.to_u8(); [r, g, b, 255] }).collect();
                    write_png(&out.join(format!("{name}-albedo.png")), W as u32, H as u32, &albedo).unwrap();
                    first_normals = f.surfaces.iter().zip(&f.water)
                        .map(|(s, &water)| (s.normal, s.height, s.kind, water)).collect();
                }
                if t == 2250 {
                    // Exactly one sun period later, dry floor geometry is still
                    // fixed in world coordinates; it must not swim with light.
                    for (i, (before, after)) in first_normals.iter().zip(&f.surfaces).enumerate() {
                        if before.2 == 1 && after.kind == 1 && before.3 == 0 && f.water[i] == 0 {
                            assert_eq!(before.0, after.normal, "{name} receiver {},{} water {}",
                                i as i32 % W as i32 + cam.0, i as i32 / W as i32 + cam.1, f.water[i]);
                            assert_eq!(before.1, after.height);
                        }
                    }
                }
                post::finish(&mut f, &zone.grade);
                let rgba: Vec<u8> = f.out.iter().flatten().copied().collect();
                let file = format!("{name}-sun-{t:05}.png");
                write_png(&out.join(&file), W as u32, H as u32, &rgba).unwrap();
                records.push(serde_json::json!({"scene": name, "tick": t, "file": file,
                    "normal_lighting": true, "camera": [cam.0, cam.1]}));
            }
            fx.lights.clear();
        }
    }
    std::fs::write(out.join("report.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "normal_channels": ["world X", "world Y", "world Z"],
        "kind": "Actual engine surface buffers and 36-second sun cycle, no synthetic relighting",
        "stability_check": "Dry ground receivers only; live water waves intentionally change their normals",
        "captures": records
    })).unwrap()).unwrap();
    println!("{}", out.display());
}
