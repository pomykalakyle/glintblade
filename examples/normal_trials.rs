//! Headless trials of the actual native renderer (no game window or synthetic relighting).
use glintblade::gfx::frame::Surface;
use glintblade::gfx::{self, DrawOpts, Frame, Grade, H, Rgb, W, rgb};
use std::path::Path;
fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "out/pixellab/round7/trial-render".into());
    std::fs::create_dir_all(&dir).unwrap();
    let thin = std::env::args().nth(2).is_some_and(|s| s == "sheet");
    let s = gfx::sheet("bloomwild_ruins").unwrap();
    let a = s.anim("pillar").unwrap();
    let column = s.cell(a, 0);
    let hero = gfx::sheet("hero").unwrap();
    let ha = hero
        .anim("idle_down")
        .or_else(|| hero.anim("idle_right"))
        .unwrap();
    for t in 0..240 {
        let mut f = Frame::new();
        f.begin(rgb(0x161f2d), Rgb::new(0.42, 0.46, 0.55));
        f.surface_lighting = true;
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                if y >= 112 {
                    let c = if (x / 24 + y / 12) % 2 == 0 {
                        rgb(0x566255)
                    } else {
                        rgb(0x505b50)
                    };
                    f.albedo[i] = c;
                    f.surfaces[i] = Surface {
                        normal: [0., 0., 1.],
                        height: 0.,
                        owner: 0,
                        kind: 1,
                    };
                }
            }
        }
        f.sprite(column, s.anchor_of(a), 192, 150, &DrawOpts::default());
        // A second identical instance demonstrates independent shadow ownership.
        f.sprite(column, s.anchor_of(a), 259, 158, &DrawOpts::default());
        let angle = t as f32 / 240. * std::f32::consts::TAU;
        let (lx, gy, h) = (192. + angle.cos() * 83., 152. + angle.sin() * 24., 86.);
        let hx = 103. + angle.sin() * 15.;
        let hy = 155. + angle.cos() * 8.;
        f.sprite(
            hero.cell(ha, 0),
            hero.anchor_of(ha),
            hx as i32,
            hy as i32,
            &DrawOpts {
                actor: true,
                ..Default::default()
            },
        );
        f.add_light_3d_w(lx, gy, h, 160., rgb(0xffe5b3), 0.82);
        if thin {
            for caster in &mut f.surface_casters {
                caster.foot = std::sync::Arc::from([]);
            }
        }
        gfx::surface::apply(&mut f);
        f.circle_glow(lx as i32, (gy - h) as i32, 2, rgb(0xffedb7));
        let grade = Grade {
            vignette: 0.,
            bloom: 0.12,
            light_steps: 32.,
            ..Default::default()
        };
        gfx::post::compose(&mut f, &grade);
        gfx::post::finish(&mut f, &grade);
        let rgba: Vec<u8> = f.out.iter().flatten().copied().collect();
        glintblade::assets::write_png(
            &Path::new(&dir).join(format!("{t:03}.png")),
            W as u32,
            H as u32,
            &rgba,
        )
        .unwrap();
        if [0, 60, 120, 180].contains(&t) {
            let normal: Vec<u8> = f
                .surfaces
                .iter()
                .flat_map(|s| {
                    if s.kind == 0 {
                        [0, 0, 0, 255]
                    } else {
                        [
                            (s.normal[0] * 127. + 128.) as u8,
                            (s.normal[2] * 127. + 128.) as u8,
                            (s.normal[1] * 127. + 128.) as u8,
                            255,
                        ]
                    }
                })
                .collect();
            glintblade::assets::write_png(
                &Path::new(&dir).join(format!("normal-{t:03}.png")),
                W as u32,
                H as u32,
                &normal,
            )
            .unwrap();
        }
    }
}
