use super::*;
use crate::{
    fx::Fx,
    game::{Game, Options, Scene},
    input::Input,
    regions,
    view::render_world,
    world::{NoRegion, Prop},
};
use std::time::Instant;

#[test]
fn fractional_native_light_planes_match_the_drawn_sprite_raster() {
    for x in [100.2,100.8] { for flip in [false,true] {
        let mut z=small(ZoneId::Glasswater);
        let mut p=Prop::new("bloomwild_ruins","pillar",x,140.8); p.flip=flip;
        let s=p.sheet().unwrap(); let a=s.anim(&p.anim).unwrap();
        let cell=s.cell(a,0); let anchor=s.anchor_of(a);
        let ox=x.floor() as i32-if flip{cell.w-1-anchor.0}else{anchor.0};
        let oy=p.y.floor() as i32-anchor.1;
        z.props.push(p);
        let field=Field::bake(&z,&NoRegion,geometry_key(&z));
        for y in 0..cell.h {for sx in 0..cell.w {
            let wx=ox+if flip{cell.w-1-sx}else{sx};
            let i=field.index(wx,oy+y).unwrap();
            assert_eq!(field.raised[i],cell.at(sx,y).a,"raster mismatch at anchor{x} flipped{flip} source{sx},{y}");
        }}
    }}
}

/// Real crystal keys survive native base refinement; dimensions do not encode light identity.
#[test]
fn glasswater_native_crystal_keys_survive_collision_refinement() {
    let (mut zone, region) = regions::build(ZoneId::Glasswater);
    let crystals: Vec<_> = zone.props.iter().enumerate().filter(|(_, p)|
        p.sheet == "glasswater_blocker" && p.alive && p.light.is_some()
        && !p.collision_rects().is_empty()).map(|(i, _)| i).collect();
    assert!(crystals.len() >= 6, "native crystals need explicit motivated light sources");
    let before = Field::bake(&zone, region.as_ref(), geometry_key(&zone));
    assert!(crystals.iter().any(|&i| zone.props[i].light.unwrap().3.abs() > 0.5),
        "the native crest attachment must exercise a horizontal offset");
    for &i in &crystals {
        let p = &mut zone.props[i];
        let (color, radius, _, ox, oy) = p.light.unwrap();
        assert!(before.sources.iter().any(|s| (s.x - (p.x + ox)).abs() < 0.01
            && (s.y - p.y).abs() < 0.01 && (s.z - (-oy).max(20.0)).abs() < 0.01
            && s.radius == radius.max(56.0) && s.color == color),
            "the native light must follow its actual crest, not the support anchor");
        let (w, _) = p.footprint.unwrap();
        p.footprint = Some((w, 11.0));
    }
    let after = Field::bake(&zone, region.as_ref(), geometry_key(&zone));
    let sources = |f: &Field| f.sources.iter().map(|s| (s.x, s.y, s.z, s.radius, s.color)).collect::<Vec<_>>();
    assert_eq!(sources(&before), sources(&after), "changing ground support must not remove a crystal key");
}

/// Cinder rims illuminate material inside the contour instead of weakening the outer ink edge.
#[test]
fn source_rim_preserves_outer_actor_contour() {
    let mut zone = small(ZoneId::Cinder);
    zone.lights.push(crate::fx::Light { x: 140.0, y: 100.0, radius: 100.0, color: rgb(0xefb68a), intensity: 1.0 });
    let field = Field::bake(&zone, &NoRegion, geometry_key(&zone));
    let mut frame = Frame::new();
    frame.begin(Rgb::BLACK, zone.ambient);
    frame.casters.push(Caster { x: 100.0, y: 100.0, half_w: 4.0, height: 16.0 });
    for y in 90..=100 { for x in 97..=103 { frame.mark_actor(x, y); } }
    assert_eq!(actor_rim(&field, &frame, 103, 95), Rgb::BLACK);
    assert!(actor_rim(&field, &frame, 102, 95).r > 0.0);
}

/// A bridge or cliff-edge actor keeps the same light when its silhouette projects over a hole.
#[test]
fn actor_light_is_independent_of_ground_holes() {
    let mut zone = small(ZoneId::Glasswater);
    let cell = Cell::from_ascii(&["aaa", "aaa", "aaa"], &[('a', rgb(0x414c64))], &[]);
    let render = |z: &Zone| {
        let mut frame = Frame::new();
        frame.begin(Rgb::BLACK, z.ambient);
        z.draw_ground(&mut frame);
        frame.sprite(&cell, (1, 1), 100, 90, &super::super::DrawOpts { actor: true, ..Default::default() });
        apply(z, &NoRegion, &mut frame, 360);
        frame.light[90 * W + 100]
    };
    let floor = render(&zone);
    for y in 89..=91 { for x in 99..=101 { zone.gset(x, y, crate::world::HOLE); } }
    assert_eq!(render(&zone), floor);
}

/// Animated scenery inside a caster's old bounding box never becomes an actor surface.
#[test]
fn moving_floor_pixels_do_not_receive_actor_fill() {
    let z = small(ZoneId::Glasswater);
    let mut control = Frame::new();
    control.begin(Rgb::BLACK, z.ambient);
    z.draw_ground(&mut control);
    control.casters.push(Caster { x: 100.0, y: 100.0, half_w: 4.0, height: 16.0 });
    apply(&z, &NoRegion, &mut control, 360);
    let index = 90 * W + 100;
    for color in [rgb(0x3e5665), rgb(0xefb68a), rgb(0x647c77)] {
        let mut moving = Frame::new();
        moving.begin(Rgb::BLACK, z.ambient);
        z.draw_ground(&mut moving);
        moving.casters = control.casters.clone();
        moving.put(100, 90, color);
        apply(&z, &NoRegion, &mut moving, 360);
        assert_eq!(moving.light[index], control.light[index]);
        assert!(!moving.actor_pixels[index]);
    }
}

/// Sprite companions cannot introduce a second recovered pillar plane above their contact footprint.
#[test]
fn summit_collision_companions_do_not_add_planes() {
    let (mut zone, region) = regions::build(ZoneId::Summit);
    let with = Field::bake(&zone, region.as_ref(), geometry_key(&zone));
    let duplicates: Vec<_> = zone.props.iter().filter(|p| sprite_companion(&zone, p))
        .map(|p| (p.sheet, p.x, p.y)).collect();
    assert!(!duplicates.is_empty());
    zone.props.retain(|p| !duplicates.iter().any(|&(s, x, y)| p.sheet == s && p.x == x && p.y == y));
    let without = Field::bake(&zone, region.as_ref(), geometry_key(&zone));
    assert_eq!(with.light, without.light);
    assert_eq!(with.raised, without.raised);
    assert_eq!(with.dark, without.dark, "physics-only support spans never stamp shadows into the sky");
}

/// Painted cave and arena terrain does not acquire a box plane from an invisible collision tile.
#[test]
fn authored_terrain_ignores_collision_grid_planes() {
    for id in [ZoneId::Glasswater, ZoneId::Summit] {
        let mut zone = small(id);
        let before = Field::bake(&zone, &NoRegion, geometry_key(&zone));
        zone.set_tile(8, 7, Tile::Wall);
        let after = Field::bake(&zone, &NoRegion, geometry_key(&zone));
        assert_eq!(before.light, after.light);
        assert_eq!(before.raised, after.raised);
    }
}

/// Construct a receiving field without any platform, audio or sprite dependency.
fn small(id: ZoneId) -> Zone {
    let mut z = Zone::new(id, 32, 20);
    z.ground.fill(rgb(0x929b90));
    z
}

/// Apply the real default hook to an initialized test frame.
fn sample(z: &Zone, cam: (i32, i32)) -> Frame {
    let mut f = Frame::new();
    f.cam_x = cam.0;
    f.cam_y = cam.1;
    f.begin(Rgb::BLACK, z.ambient);
    z.draw_ground(&mut f);
    apply(z, &NoRegion, &mut f, 360);
    f
}

/// Distinct equal-id zones, broken props and bulk/individual geometry edits must never share stale light.
#[test]
fn zone_owned_cache_invalidates_geometry_and_breakage() {
    let mut a = small(ZoneId::Bloomwild);
    let mut p = Prop::new("__fixture", "none", 150.0, 130.0);
    p.footprint = Some((18.0, 5.0));
    a.props.push(p);
    let first = sample(&a, (0, 0));
    let ptr = a.lighting.borrow().as_ref().unwrap().light.as_ptr();
    assert_eq!(sample(&a, (0, 0)).light, first.light);
    assert_eq!(
        a.lighting.borrow().as_ref().unwrap().light.as_ptr(),
        ptr,
        "warm frame rebaked"
    );
    a.props[0].alive = false;
    let broken = sample(&a, (0, 0));
    assert!(first.light[130 * W + 150].luma() < broken.light[130 * W + 150].luma());
    let b = small(ZoneId::Bloomwild);
    assert_eq!(broken.light, sample(&b, (0, 0)).light);
    a.set_tile(8, 7, Tile::Wall);
    assert!(a.lighting.borrow().is_none());
    assert_ne!(sample(&a, (0, 0)).light, broken.light);
    a.gset(2, 2, crate::world::HOLE);
    assert!(a.lighting.borrow().is_none());
    sample(&a, (0, 0));
    a.ground[123].g += 0.1;
    a.invalidate_lighting();
    assert!(a.lighting.borrow().is_none());
    a.eset(2, 2, Rgb::WHITE);
    assert!(a.lighting.borrow().is_none());
}

/// Require exact silhouette gaps, nonstacking projections and C's broad lit/shadow planes.
#[test]
fn alpha_projection_and_planes_are_native_and_nonstacking() {
    let z = small(ZoneId::Bloomwild);
    let mut field = Field::bake(&z, &NoRegion, geometry_key(&z));
    let cell = Cell::from_ascii(&["1...1", "1...1", "1...1"], &[('1', Rgb::WHITE)], &[]);
    let p = Prop::new("__stone_fixture", "none", 80.0, 60.0);
    let mut planes = Planes::new(Rect::zone(&z));
    field.sprite(&z, &mut planes, &cell, (2, 2), &p);
    let at = |x, y| (y * z.pw + x) as usize;
    assert!(field.raised[at(78, 58)]);
    assert!(!field.raised[at(80, 58)]);
    assert!(field.dark[at(79, 62)] > 0);
    assert_eq!(field.dark[at(80, 62)], 0);
    let first = field.dark.clone();
    field.sprite(&z, &mut planes, &cell, (2, 2), &p);
    assert_eq!(first, field.dark);
    let left = plane(2, 10, 0, 15, 0, 20);
    let right = plane(14, 10, 0, 15, 0, 20);
    assert!(
        field.form(z.id, left, 10.0, 10.0).luma()
            > field.form(z.id, right, 10.0, 10.0).luma() + 0.15
    );
}

/// Lava banks must follow actual source geometry and source direction rather than ZoneId alone.
#[test]
fn lava_field_and_local_keys_follow_sources() {
    let mut z = small(ZoneId::Cinder);
    z.set_tile(8, 8, Tile::Lava);
    let f = Field::bake(&z, &NoRegion, geometry_key(&z));
    let a = f.index(150, 140).unwrap();
    let b = f.index(300, 140).unwrap();
    assert!(f.light[a].r > f.light[b].r + 0.2);
    let s = f.source(180.0, 140.0).unwrap();
    let d = f.direction(z.id, 180.0, 140.0, None).unwrap();
    assert!((180.0 - s.x) * d.0 + (140.0 - s.y) * d.1 > 0.0);
    assert!(
        Field::bake(&small(ZoneId::Cinder), &NoRegion, 0)
            .source(180.0, 140.0)
            .is_none()
    );
}

/// Preserve albedo/emission and min-combine repeated sprite contacts without darkening the actor body.
#[test]
fn contacts_materials_emission_and_actor_readability() {
    let z = small(ZoneId::Bloomwild);
    let mut f = Frame::new();
    f.begin(rgb(0x929b90), z.ambient);
    f.shadow(150, 130, 5, 2, 0.45);
    let shade = f.shade.clone();
    f.shadow(150, 130, 5, 2, 0.45);
    assert_eq!(shade, f.shade);
    assert!(f.albedo.iter().all(|&a| a == rgb(0x929b90)));
    f.casters.push(Caster {
        x: 150.0,
        y: 130.0,
        half_w: 4.0,
        height: 16.0,
    });
    f.albedo[120 * W + 150] = rgb(0x7c9c95);
    f.emissive[120 * W + 150] = rgb(0xbaf5ec);
    f.mark_actor(150, 120);
    let (art, emit) = (f.albedo.clone(), f.emissive.clone());
    apply(&z, &NoRegion, &mut f, 0);
    assert_eq!(art, f.albedo);
    assert_eq!(emit, f.emissive);
    assert!(f.light[120 * W + 150].r >= 0.78);
    assert!(f.shade.iter().all(|&s| s == 1.0));
    assert!(f.light[130 * W + 150].luma() > 0.5, "foot contacts stacked");
}

/// Compare real region lighting across repeated renders and odd camera shifts, including static procedural props.
#[test]
fn real_regions_repeat_and_stay_world_anchored() {
    for (id, cam) in [
        (ZoneId::Bloomwild, (0, 588)),
        (ZoneId::Glasswater, (348, 10)),
        (ZoneId::Cinder, (178, 347)),
        (ZoneId::Summit, (428, 256)),
    ] {
        let (z, r) = regions::build(id);
        let mut frames = Vec::new();
        for (dx, dy) in [(0, 0), (0, 0), (7, 3)] {
            let mut f = Frame::new();
            f.cam_x = cam.0 + dx;
            f.cam_y = cam.1 + dy;
            f.casters.push(Caster {
                x: (cam.0 + 190) as f32,
                y: (cam.1 + 120) as f32,
                half_w: 4.0,
                height: 16.0,
            });
            render_world(
                &z,
                r.as_ref(),
                &Fx::new(7),
                &mut f,
                360,
                &[],
                &mut |_, _| {},
                None,
            );
            frames.push(f);
        }
        assert_eq!(frames[0].out, frames[1].out, "{id:?} repeat");
        let mut compared = 0;
        for y in 24..H - 24 {
            for x in 24..W - 24 {
                let (i, j) = (y * W + x, (y - 3) * W + x - 7);
                if frames[0].albedo[i] != frames[2].albedo[j] {
                    continue;
                }
                let (a, b) = (frames[0].light[i], frames[2].light[j]);
                assert!(
                    (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs() < 0.001,
                    "{id:?} world {},{}",
                    cam.0 + x as i32,
                    cam.1 + y as i32
                );
                compared += 1;
            }
        }
        assert!(compared > 1000);
        println!("ANCHOR {id:?}: {compared} equal-art world pixels, zero changes");
    }
}

/// Measure complete real Game::render separately from a cold bake and PNG encoding in release mode.
#[test]
fn full_game_release_timing() {
    for (id, pos) in [
        (ZoneId::Bloomwild, None),
        (ZoneId::Glasswater, Some((540.0, 118.0))),
        (ZoneId::Cinder, Some((370.0, 455.0))),
        (ZoneId::Summit, Some((620.0, 364.0))),
    ] {
        let mut game = Game::new(Options {
            start: Some(id),
            peaceful: false,
            god: true,
            seed: 7,
        });
        if let Some((x, y)) = pos
            && let Scene::Play(p) = &mut game.scene
        {
            p.player.body.x = x;
            p.player.body.y = y;
            p.checkpoint = (x, y);
            p.cam.snap(x, y);
        }
        for _ in 0..360 {
            game.tick(&Input::default());
            game.fx.lights.clear();
        }
        let t = Instant::now();
        game.render();
        let cold = t.elapsed().as_secs_f64() * 1000.0;
        let mut times = Vec::new();
        for _ in 0..120 {
            let t = Instant::now();
            game.render();
            std::hint::black_box(&game.frame.out);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        times.sort_by(f64::total_cmp);
        let p95 = times[114];
        println!(
            "DEFAULT_RENDER {id:?}: cold_full={cold:.3}ms warm_mean={mean:.3}ms p95={p95:.3}ms max={:.3}ms",
            times[119]
        );
        if let Scene::Play(p) = &game.scene {
            let t = Instant::now();
            let bake = Field::bake(&p.zone, p.region.as_ref(), geometry_key(&p.zone));
            println!(
                "DEFAULT_BAKE {id:?}: {:.3}ms retained={:.2}MiB",
                t.elapsed().as_secs_f64() * 1000.0,
                (bake.light.len() * 12
                    + bake.art.len() * 12
                    + bake.dark.len()
                    + bake.raised.len()
                    + bake.receiving.len()
                    + bake.lava.len() * std::mem::size_of::<(i32, i32, u16)>())
                    as f64
                    / 1048576.0
            );
        }
        if !cfg!(debug_assertions) {
            assert!(mean < 6.0 && p95 < 8.0, "{id:?} rendering exceeds budget");
        }
    }
}

/// Compare every retained lighting buffer with an independent fresh bake after a bounded update.
fn equals_fresh(z: &Zone, r: &dyn Region) {
    let fresh = Field::bake(z, r, geometry_key(z));
    let cache = z.lighting.borrow();
    let field = cache.as_ref().unwrap();
    assert_eq!(field.art, fresh.art, "canonical art");
    assert_eq!(field.raised, fresh.raised, "stale raised mask");
    assert_eq!(
        field.dark, fresh.dark,
        "stale or lost overlapping occlusion"
    );
    assert_eq!(field.light, fresh.light, "stale form plane");
    assert_eq!(field.receiving, fresh.receiving);
    assert_eq!(field.lava, fresh.lava);
}

/// Remove real authored breakables without replacing any full-zone storage or losing overlapping shadows.
#[test]
fn breakable_patch_matches_fresh_bakes_and_preserves_overlaps() {
    let template = Prop::new("bloomwild_props", "urn_small", 150.0, 130.0);
    assert!(
        incremental(&template),
        "fixture must be an actual authored breakable"
    );
    let mut z = small(ZoneId::Bloomwild);
    let mut permanent = template.clone();
    permanent.breakable = false;
    permanent.x += 4.0;
    z.props.extend([template, permanent]);
    prepare(&z, &NoRegion);
    let (ptr, before) = {
        let cache = z.lighting.borrow();
        let f = cache.as_ref().unwrap();
        (f.light.as_ptr(), f.dark.clone())
    };
    assert!(z.props[0].hit(100, &mut Fx::new(7), 360));
    prepare(&z, &NoRegion);
    equals_fresh(&z, &NoRegion);
    let cache = z.lighting.borrow();
    let f = cache.as_ref().unwrap();
    assert_eq!(ptr, f.light.as_ptr(), "whole field replaced on break");
    assert!(f.patched_pixels > 0 && f.patched_pixels < f.light.len() / 4);
    assert!(
        before.iter().zip(&f.dark).any(|(&a, &b)| a > b),
        "no shadow removed"
    );
    assert!(
        before
            .iter()
            .zip(&f.dark)
            .any(|(&a, &b)| a == 186 && b == 186),
        "overlap lost"
    );
    assert!(f.raised.iter().any(|&a| a));
    drop(cache);
    // Resurrection uses the same bounded path and must restore exact canonical geometry.
    z.props[0].alive = true;
    prepare(&z, &NoRegion);
    equals_fresh(&z, &NoRegion);
}

/// Exercise every real region's sprite breakables sequentially, including procedural neighbors and camera seams.
#[test]
fn real_breakables_match_fresh_fields() {
    for id in [
        ZoneId::Bloomwild,
        ZoneId::Glasswater,
        ZoneId::Cinder,
        ZoneId::Summit,
    ] {
        let (mut z, r) = regions::build(id);
        let indices: Vec<_> = z
            .props
            .iter()
            .enumerate()
            .filter(|(_, p)| incremental(p))
            .map(|(i, _)| i)
            .collect();
        let ptr = z.lighting.borrow().as_ref().unwrap().light.as_ptr();
        for &i in &indices {
            let (before_dark, before_raised) = {
                let cache = z.lighting.borrow();
                let f = cache.as_ref().unwrap();
                (f.dark.clone(), f.raised.clone())
            };
            z.props[i].alive = false;
            prepare(&z, r.as_ref());
            assert_eq!(ptr, z.lighting.borrow().as_ref().unwrap().light.as_ptr());
            equals_fresh(&z, r.as_ref());
            let cache = z.lighting.borrow();
            let f = cache.as_ref().unwrap();
            let shadows_removed = before_dark
                .iter()
                .zip(&f.dark)
                .filter(|(a, b)| a > b)
                .count();
            let raised_removed = before_raised
                .iter()
                .zip(&f.raised)
                .filter(|(a, b)| **a && !**b)
                .count();
            assert!(
                shadows_removed > 0 && raised_removed > 0,
                "authored prop left stale occlusion"
            );
            println!(
                "REMOVED {id:?} prop={i}: shadow_pixels={shadows_removed} raised_pixels={raised_removed}"
            );
        }
        println!(
            "BREAKABLE_EQUIVALENCE {id:?}: {} authored props, all buffers equal fresh bakes",
            indices.len()
        );
    }
}

/// Measure every complete live Game::render through a normal sword-triggered pot break, including the invalidation frame.
#[test]
fn actual_sword_break_full_render_timing() {
    let (zone, _) = regions::build(ZoneId::Bloomwild);
    let pots: Vec<_> = zone
        .props
        .iter()
        .enumerate()
        .filter(|(_, p)| incremental(p))
        .map(|(i, _)| i)
        .collect();
    for index in pots {
        let id = ZoneId::Bloomwild;
        let load_start = Instant::now();
        let mut game = Game::new(Options {
            start: Some(id),
            peaceful: false,
            god: true,
            seed: 7,
        });
        let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;
        let Scene::Play(p) = &mut game.scene else {
            panic!("play")
        };
        let (x, y) = (p.zone.props[index].x, p.zone.props[index].y);
        p.player.respawn(x, y + 18.0);
        p.checkpoint = (x, y + 18.0);
        p.cam.snap(x, y + 18.0);
        let ptr = p.zone.lighting.borrow().as_ref().unwrap().light.as_ptr();
        let mut input = Input::default();
        let mut times = Vec::new();
        let mut break_time = None;
        for tick in 0..180 {
            input.step(crate::input::Raw {
                mv: if tick == 30 { (0.0, -1.0) } else { (0.0, 0.0) },
                attack: tick == 30,
                ..Default::default()
            });
            game.tick(&input);
            let Scene::Play(p) = &game.scene else {
                panic!("play")
            };
            let just_broken = !p.zone.props[index].alive && break_time.is_none();
            let t = Instant::now();
            game.render();
            std::hint::black_box(&game.frame.out);
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            if just_broken {
                break_time = Some((tick, ms));
            }
            times.push(ms);
            game.fx.sounds.clear();
        }
        let (tick, break_ms) = break_time.expect("real sword must break authored pot");
        let Scene::Play(p) = &game.scene else {
            panic!("play")
        };
        let cache = p.zone.lighting.borrow();
        let field = cache.as_ref().unwrap();
        assert_eq!(ptr, field.light.as_ptr(), "sword break rebaked whole zone");
        assert!(field.patched_pixels > 0 && field.patched_pixels < field.light.len());
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        times.sort_by(f64::total_cmp);
        println!(
            "BREAK_RENDER {id:?} prop={index}: load_including_bake={load_ms:.3}ms sword_tick={tick} break_frame={break_ms:.3}ms mean={mean:.3}ms p95={:.3}ms max={:.3}ms patched_pixels={} zone_pixels={} storage_unchanged=true",
            times[171],
            times[179],
            field.patched_pixels,
            field.light.len()
        );
        if !cfg!(debug_assertions) {
            assert!(
                break_ms < 8.0 && times[179] < 8.0,
                "complete render exceeds budget"
            );
        }
    }
}

/// Keep geometry/source changes on full invalidation and isolate distinct geometry with the same ZoneId.
#[test]
fn rare_geometry_and_source_changes_still_rebake() {
    let mut z = small(ZoneId::Bloomwild);
    let prop = Prop::new("bloomwild_props", "urn_small", 150.0, 130.0);
    z.props.push(prop.clone());
    prepare(&z, &NoRegion);
    let old = z.lighting.borrow().as_ref().unwrap().light.as_ptr();
    z.props[0].x += 50.0;
    prepare(&z, &NoRegion);
    assert_ne!(old, z.lighting.borrow().as_ref().unwrap().light.as_ptr());
    equals_fresh(&z, &NoRegion);
    let mut other = small(z.id);
    other.props.push(prop);
    prepare(&other, &NoRegion);
    assert_ne!(
        z.lighting.borrow().as_ref().unwrap().raised,
        other.lighting.borrow().as_ref().unwrap().raised
    );
    z.props[0].light = Some((Rgb::WHITE, 80.0, 0.8, 0.0, -20.0));
    prepare(&z, &NoRegion);
    let old = z.lighting.borrow().as_ref().unwrap().light.as_ptr();
    z.props[0].alive = false;
    prepare(&z, &NoRegion);
    assert_ne!(old, z.lighting.borrow().as_ref().unwrap().light.as_ptr());
    assert!(z.lighting.borrow().as_ref().unwrap().sources.is_empty());
    equals_fresh(&z, &NoRegion);
}

/// Include procedural readers, outside wall casts, flipped seam-crossing sprites and simultaneous state changes.
#[test]
fn dependent_procedural_and_camera_seam_patches_match_fresh() {
    for id in [
        ZoneId::Bloomwild,
        ZoneId::Glasswater,
        ZoneId::Cinder,
        ZoneId::Summit,
    ] {
        let mut z = small(id);
        let p = Prop::new("bloomwild_props", "urn_large", 383.6, 216.4).flipped(true);
        let mut neighbor = p.clone();
        neighbor.x -= 4.0;
        neighbor.breakable = false;
        let mut procedural = Prop::new("__procedural", "none", 380.0, 236.0);
        procedural.footprint = Some((18.0, 7.0));
        z.props.extend([p, neighbor, procedural]);
        z.set_tile(22, 13, Tile::Wall);
        if id == ZoneId::Cinder {
            z.set_tile(21, 13, Tile::Lava);
        }
        prepare(&z, &NoRegion);
        z.props[0].alive = false;
        prepare(&z, &NoRegion);
        equals_fresh(&z, &NoRegion);
        let a = sample(&z, (200, 110));
        let b = sample(&z, (207, 113));
        for y in 3..H {
            for x in 7..W {
                assert_eq!(a.light[y * W + x], b.light[(y - 3) * W + x - 7]);
            }
        }
        z.props[0].alive = true;
        z.props[1].breakable = true;
        prepare(&z, &NoRegion);
        z.props[0].alive = false;
        z.props[1].alive = false;
        prepare(&z, &NoRegion);
        equals_fresh(&z, &NoRegion);
    }
}

/// Shaped court reception warms the floor while an actor over the same point keeps its own fill.
#[test]
fn cinder_court_reception_does_not_leak_into_actors() {
    let (z, region) = regions::build(ZoneId::Cinder);
    let mut floor = Frame::new();
    floor.begin(Rgb::BLACK, z.ambient);
    floor.cam_x = 646 - 192;
    floor.cam_y = 468 - 108;
    let mut actor = Frame::new();
    actor.begin(Rgb::BLACK, z.ambient);
    actor.cam_x = floor.cam_x;
    actor.cam_y = floor.cam_y;
    actor.mark_actor(192, 108);
    apply(&z, region.as_ref(), &mut floor, 300);
    apply(&z, region.as_ref(), &mut actor, 300);
    let i = 108 * W + 192;
    assert!(floor.light[i].r > floor.light[i].b, "the apron should receive the visible fire");
    assert!(actor.light[i].r < actor.light[i].b, "the actor must retain its independent cool fill");
}
