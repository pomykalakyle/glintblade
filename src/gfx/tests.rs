//! Headless renderer contract tests and a small, reproducible lighting study.

use super::{Cell, DrawOpts, Frame, Grade, H, Rgb, W, post, rgb};

/// Actor coverage follows visible sprite pixels and is removed by opaque foreground scenery.
#[test]
fn actor_coverage_tracks_opaque_sprite_and_foreground() {
    let cell = Cell::from_ascii(&["aaa", "a.a", "aaa"], &[('a', rgb(0x414c64))], &[]);
    let mut frame = Frame::new();
    frame.begin(Rgb::BLACK, Rgb::WHITE);
    frame.sprite(&cell, (1, 2), 100, 100, &DrawOpts { actor: true, ..Default::default() });
    assert_eq!(frame.actor_pixels.iter().filter(|&&v| v).count(), 8);
    assert!(!frame.actor_pixels[99 * W + 100]);
    frame.rect(99, 98, 3, 3, rgb(0x4c6070));
    assert!(!frame.actor_pixels.iter().any(|&v| v));
    frame.sprite(&cell, (1, 2), 100, 100, &DrawOpts { actor: true, ..Default::default() });
    frame.begin(Rgb::BLACK, Rgb::WHITE);
    assert!(!frame.actor_pixels.iter().any(|&v| v));
}

/// A real flashing hero keeps its perimeter readable without making cloth emit light.
#[test]
fn hit_flash_preserves_actor_edges_and_material_emission() {
    let sheet = super::sheet("hero").unwrap();
    let anim = sheet.anim("idle_down").unwrap();
    let cell = sheet.cell(anim, 0);
    let anchor = sheet.anchor_of(anim);
    let mut normal = Frame::new();
    let mut flashed = Frame::new();
    normal.begin(Rgb::BLACK, Rgb::WHITE);
    flashed.begin(Rgb::BLACK, Rgb::WHITE);
    normal.sprite(cell, anchor, 100, 100, &DrawOpts::default());
    flashed.sprite(cell, anchor, 100, 100, &DrawOpts { flash: 1.0, ..Default::default() });
    let (mut perimeter, mut brightened) = (0, 0);
    for y in 0..cell.h {
        for x in 0..cell.w {
            let p = cell.at(x, y);
            if !p.a { continue; }
            let i = ((100 - anchor.1 + y) as usize) * W + (100 - anchor.0 + x) as usize;
            let edge = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .iter().any(|&(nx, ny)| !cell.at(nx, ny).a);
            if edge {
                assert_eq!(flashed.albedo[i], normal.albedo[i], "perimeter {x},{y}");
                perimeter += 1;
            } else if flashed.albedo[i].luma() > normal.albedo[i].luma() {
                brightened += 1;
            }
            if !p.emit { assert_eq!(flashed.emissive[i], Rgb::BLACK, "material {x},{y}"); }
        }
    }
    assert!(perimeter > 20 && brightened > 20);
}

/// Paint a restrained stone bank, a pool, and two visible crystal light sources.
fn study(f: &mut Frame) {
    f.begin(rgb(0x252438), rgb(0xa0a4c0));
    f.t = 90;
    for y in 18..198 {
        let c = if y < 102 { rgb(0x625b76) } else { rgb(0x1d3453) };
        f.rect(18, y, 348, 1, c);
    }
    for (x, y, w) in [(26, 39, 74), (109, 39, 80), (198, 39, 71), (279, 39, 77), (26, 74, 93), (129, 74, 85), (224, 74, 132)] {
        f.rect(x, y, w, 2, rgb(0x464358));
    }
    f.rect(18, 98, 348, 3, rgb(0x9694a6));
    f.rect(18, 101, 348, 3, rgb(0x343747));
    f.rect(32, 178, 320, 2, rgb(0x2c4f6c));
    // The same mask is exercised before and after automatic reflections are enabled.
    f.water_any = true;
    for y in 104..198 {
        for x in 18..366 {
            f.water[y * W + x] = 210;
        }
    }
    let crystal = Cell::from_ascii(
        &["....a....", "...abb...", "..aabbc..", ".aaabbbc.", "aaaabbbcc", "aaaabbbcc", ".aaabbbc.", "..aabbc..", "...abc..."],
        &[('a', rgb(0x328aaf)), ('b', rgb(0x71d9df)), ('c', rgb(0xd2faf2))],
        &['b', 'c'],
    );
    for (x, color) in [(86, rgb(0x4cd8ec)), (286, rgb(0xad79eb))] {
        f.shadow(x + 2, 96, 12, 3, 0.38);
        f.sprite(&crystal, (4, 8), x, 97, &DrawOpts::default());
        f.add_light(x as f32, 92.0, 72.0, color, 0.8);
    }
    let wanderer = Cell::from_ascii(
        &["...aaa....", "..abbaa...", ".abbbbaa..", ".abddbaa..", "..abbba...", "..beeba...", ".abbbbaa..", "abbbbbbaf.", "abbbbbaf..", ".abbbba...", "..acca...."],
        &[('a', rgb(0x20253e)), ('b', rgb(0x485c95)), ('c', rgb(0x7384ad)), ('d', rgb(0x182333)), ('e', rgb(0x87f3ff)), ('f', rgb(0xd7faff))],
        &['e', 'f'],
    );
    f.shadow(186, 132, 8, 3, 0.45);
    f.sprite(&wanderer, (4, 10), 184, 130, &DrawOpts::default());
    f.add_light(190.0, 126.0, 25.0, rgb(0x6bd8ed), 0.38);
    // Bright palette chips must remain authored colors in a neutral-lit patch.
    for (j, c) in [0xf4eee3, 0xc8d7e1, 0x6096aa, 0xb69dc5, 0x495477].into_iter().enumerate() {
        f.rect(142 + j as i32 * 20, 55, 16, 13, rgb(c));
    }
}

/// Save the fixed lighting study when explicitly requested by an environment variable.
#[test]
fn renderer_study() {
    let Some(dest) = std::env::var_os("GLINT_RENDER_STUDY") else { return };
    let mut f = Frame::new();
    study(&mut f);
    let g = Grade { vignette: 0.0, ..Grade::default() };
    post::compose(&mut f, &g);
    post::finish(&mut f, &g);
    crate::assets::write_png(std::path::Path::new(&dest), (W * 3) as u32, (H * 3) as u32, &post::composite_scaled(&f, 3)).unwrap();
}

/// Neutral light preserves all channel values, including bright palette colors, after finish.
#[test]
fn neutral_palette_is_exact() {
    let mut f = Frame::new();
    f.begin(Rgb::BLACK, Rgb::WHITE);
    for i in 0..W * H {
        f.albedo[i] = Rgb::from_u8([(i % 256) as u8, ((i * 73) % 256) as u8, ((i * 157) % 256) as u8]);
    }
    let g = Grade { vignette: 0.0, ..Grade::default() };
    post::compose(&mut f, &g);
    post::finish(&mut f, &g);
    for i in 0..W * H {
        let u = f.albedo[i].to_u8();
        assert_eq!(f.out[i], [u[0], u[1], u[2], 255], "palette changed at {i}");
    }
}

/// A flat colored ambient is a single solid color, and point-light bands keep their hue.
#[test]
fn ambient_is_flat_and_light_bands_keep_hue() {
    let mut f = Frame::new();
    f.begin(rgb(0x8796ac), rgb(0x717bad));
    let g = Grade { vignette: 0.0, bloom: 0.0, ..Grade::default() };
    post::compose(&mut f, &g);
    assert!(f.out.iter().all(|p| *p == f.out[0]));
    f.begin(Rgb::WHITE, Rgb::BLACK);
    f.look.light_scatter = 0.0;
    f.add_light(192.0, 108.0, 80.0, Rgb::new(0.2, 0.6, 1.0), 0.7);
    post::compose(&mut f, &g);
    let mut colors = std::collections::HashSet::new();
    for p in &f.out {
        if p[2] > 0 {
            assert!((p[0] as f32 - p[2] as f32 * 0.2).abs() < 1.0);
            assert!((p[1] as f32 - p[2] as f32 * 0.6).abs() < 1.0);
        }
        colors.insert(*p);
    }
    assert!(colors.len() <= 9, "{} shades for a seven-step light", colors.len());
}

/// Moving the camera by an odd number of pixels keeps world-space lighting and halos stable.
#[test]
fn light_and_glow_are_world_anchored() {
    let mut a = Frame::new();
    let mut b = Frame::new();
    b.cam_x = 7;
    b.cam_y = 3;
    let g = Grade { vignette: 0.0, ..Grade::default() };
    for f in [&mut a, &mut b] {
        f.begin(rgb(0x24304a), rgb(0x9399b1));
        f.light_source_w(191.0, 107.0, 3, 60.0, rgb(0x58c9e5), 0.8);
        post::compose(f, &g);
        post::finish(f, &g);
    }
    for y in 30..180 {
        for x in 30..350 {
            assert_eq!(a.out[y * W + x], b.out[(y - 3) * W + x - 7], "unstable world pixel {x},{y}");
        }
    }
    assert!(a.bloom.iter().all(|p| *p == [0, 0, 0, 255]));
    let scaled = post::composite_scaled(&a, 3);
    for y in 0..H * 3 {
        for x in 0..W * 3 {
            assert_eq!(&scaled[(y * W * 3 + x) * 4..(y * W * 3 + x) * 4 + 4], a.out[(y / 3) * W + x / 3]);
        }
    }
}

/// A full fade includes native glow and UI; a full flash includes every halo.
#[test]
fn fades_and_flashes_cover_glow_and_ui() {
    let mut f = Frame::new();
    for (fade, flash, expected) in [(1.0, 0.0, [0, 0, 0, 255]), (0.0, 1.0, [255, 255, 255, 255])] {
        f.begin(rgb(0x333c59), Rgb::WHITE);
        f.circle_glow(192, 108, 8, rgb(0x74dbe8));
        let g = Grade::default();
        post::compose(&mut f, &g);
        f.ui_glow(40, 40, Rgb::WHITE, 1.0);
        f.fade = fade;
        f.flash = flash;
        post::finish(&mut f, &g);
        assert!(f.out.iter().all(|p| *p == expected));
        assert!(f.bloom.iter().all(|p| *p == [0, 0, 0, 255]));
    }
}

/// Reflections only touch masked water, and opaque sprites hide previously drawn emission.
#[test]
fn reflections_are_masked_and_sprite_occlusion_is_opaque() {
    let mut f = Frame::new();
    f.begin(rgb(0x26344e), Rgb::WHITE);
    f.look.water_wave = 0.0;
    f.look.water_alpha = 1.0;
    let cell = Cell::from_ascii(&["aaa", "a.a", "aaa"], &[('a', rgb(0x83d8df))], &['a']);
    f.water_rect(98, 102, 2, 6, 1.0);
    f.sprite(&cell, (1, 2), 100, 100, &DrawOpts::default());
    for y in 101..108 {
        for x in 96..106 {
            let i = y * W + x;
            if !(98..100).contains(&x) || y < 102 {
                assert_eq!(f.emissive[i], Rgb::BLACK);
            }
        }
    }
    assert!(f.emissive[102 * W + 99].max_c() > 0.0);
    let opaque = Cell::from_ascii(&["aaa", "aaa", "aaa"], &[('a', rgb(0x475a76))], &[]);
    f.sprite(&opaque, (1, 2), 100, 100, &DrawOpts::default());
    assert_eq!(f.emissive[99 * W + 99], Rgb::BLACK);
    f.begin(Rgb::BLACK, Rgb::WHITE);
    assert!(!f.water_any);
    assert!(f.water.iter().all(|p| *p == 0));
}

/// Measure real game renders without PNG encoding or scaling, with a high particle load too.
#[test]
#[ignore = "explicit release-mode render timing and visual output"]
fn renderer_performance() {
    use crate::game::{Game, Options};
    use crate::input::Input;
    use crate::world::ZoneId;
    use std::hint::black_box;
    use std::time::Instant;
    for zone in [ZoneId::Bloomwild, ZoneId::Glasswater, ZoneId::Cinder, ZoneId::Summit] {
        let mut game = Game::new(Options { start: Some(zone), god: true, peaceful: false, seed: 7 });
        for _ in 0..90 {
            game.tick(&Input::default());
        }
        let mut timings = Vec::new();
        for _ in 0..120 {
            let start = Instant::now();
            game.render();
            black_box(&game.frame.out);
            timings.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        timings.sort_by(f64::total_cmp);
        println!("{zone:?}: render median {:.3} ms, p95 {:.3} ms", timings[60], timings[114]);
        assert!(timings[114] < 8.0);
    }
    let mut f = Frame::new();
    let mut particles = crate::fx::particles::Particles::default();
    let mut rng = crate::util::Rng::new(9);
    for _ in 0..8000 {
        let x = rng.range(0.0, W as f32);
        let y = rng.range(0.0, H as f32);
        particles.add(crate::fx::particles::Particle::new(x, y).glow(1.0));
    }
    let start = Instant::now();
    for _ in 0..120 {
        study(&mut f);
        particles.draw(&mut f);
        post::compose(&mut f, &Grade::default());
        post::finish(&mut f, &Grade::default());
        black_box(&f.out);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / 120.0;
    println!("Lighting study + 8000 emissive particles: {ms:.3} ms average");
    assert!(ms < 8.0);
}
