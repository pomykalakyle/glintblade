use super::*;
use crate::gfx::{frame::Surface, surface::NormalPixel};

fn fixture(actor: Option<Rgb>, actor_x: i32, camera: (i32, i32)) -> (Frame, u32) {
    let mut f = Frame::new();
    f.begin(Rgb::new(0.08, 0.10, 0.14), Rgb::WHITE);
    f.surface_lighting = true;
    (f.cam_x, f.cam_y) = camera;
    f.surfaces.fill(Surface { normal: [0.0, 0.0, 1.0], kind: 1, ..Surface::default() });
    let mut crystal = Cell::from_ascii(&["xxxxxxxxxxxxxxxxxxxxxxxx"; 48], &[('x', Rgb::new(0.48, 0.40, 0.58))], &[]);
    crystal.surface = vec![NormalPixel { n: [0, 0, 127], a: true, emit: false }; crystal.px.len()].into();
    f.sprite_w(&crystal, (12, 47), 100.0, 100.0, &DrawOpts { reflectivity: 0.22, ..Default::default() });
    let owner = f.surface_casters.last().unwrap().owner;
    f.capture_reflection_sources = true;
    if let Some(color) = actor {
        let actor = Cell::from_ascii(&["..ttt..", "..ttt..", ".bbbbb.", ".bbbbb.", "bbbbbbb", "bbbbbbb", ".bbbbb.", ".bbbbb.", ".b...b.", ".b...b.", ".b...b."],
            &[('t', Rgb::new(0.1, 0.9, 0.2)), ('b', color)], &['t']);
        f.sprite_w(&actor, (3, 10), actor_x as f32, 120.0,
            &DrawOpts { actor: true, foot_depth: 1, foot_full_width: true, ground_y: Some(121.0), ..Default::default() });
    }
    (f, owner)
}

fn compose(f: &mut Frame) {
    let grade = Grade { bloom: 0.0, light_steps: 0.0, ..Default::default() };
    super::super::post::compose(f, &grade);
    apply(None, f, &grade);
}

#[test]
fn reflection_direction_uses_the_native_face() {
    let v = std::f32::consts::FRAC_1_SQRT_2;
    assert_eq!(reflected_ray([0.0, 1.0, 0.0]), Some([0.0, v, -v]));
    assert_eq!(reflected_ray([0.0, 0.0, 0.0]), None);
    assert_eq!(reflected_ray([0.0, -1.0, 0.0]), None);
}

#[test]
fn actual_actor_art_and_emissive_head_are_upright_in_the_crystal() {
    let (mut red, owner) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (0, 0));
    let (mut blue, _) = fixture(Some(Rgb::new(0.1, 0.1, 0.9)), 100, (0, 0));
    let original_albedo = red.albedo.clone();
    let original_emission = red.emissive.clone();
    compose(&mut red);
    compose(&mut blue);
    let mut reflected_body = Vec::new();
    let mut reflected_head = Vec::new();
    for i in 0..W * H {
        if red.surfaces[i].owner == owner {
            if red.out[i] != blue.out[i] { reflected_body.push(i / W); }
            let p = [(i % W) as f32 + 0.5, (i / W) as f32 + 0.5 + red.surfaces[i].height, red.surfaces[i].height];
            if let Some(Hit { donor: Donor::Native(id, pixel, _), .. }) = native_hit(&red, 0, p, reflected_ray(red.surfaces[i].normal).unwrap(), RANGE)
                && red.reflection_sources[id].pixels[pixel].emit
            { reflected_head.push(i / W); }
        }
    }
    assert!(reflected_body.len() >= 20, "actual source color must change a recognizable body region, got {}", reflected_body.len());
    assert!(!reflected_head.is_empty(), "emissive native pixels need reflection geometry too");
    assert!(reflected_head.iter().sum::<usize>() / reflected_head.len() < reflected_body.iter().sum::<usize>() / reflected_body.len(),
        "a vertical mirror keeps the reflected character upright");
    assert_eq!(red.albedo, original_albedo);
    assert_eq!(red.emissive, original_emission);
    assert!(red.point_lights.is_empty(), "reflected imagery cannot create a light");
}

#[test]
fn source_motion_removal_and_world_camera_registration_are_live() {
    let (mut a, owner) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (0, 0));
    let (mut b, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 106, (0, 0));
    let (mut absent, _) = fixture(None, 100, (0, 0));
    compose(&mut a); compose(&mut b); compose(&mut absent);
    let changes = |f: &Frame| (0..W * H).filter(|&i| a.surfaces[i].owner == owner && f.out[i] != absent.out[i]).map(|i| i % W).collect::<Vec<_>>();
    let left = changes(&a); let right = changes(&b);
    assert!(!left.is_empty() && !right.is_empty());
    assert!(right.iter().sum::<usize>() as f32 / right.len() as f32 > left.iter().sum::<usize>() as f32 / left.len() as f32 + 3.0);
    let (mut scrolled, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (80, 60));
    compose(&mut scrolled);
    for y in 60..101 { for x in 88..112 {
        assert_eq!(a.out[y * W + x], scrolled.out[(y - 60) * W + x - 80], "camera moved a world reflection at {x},{y}");
    } }
}

#[test]
fn retained_actor_survives_direct_occlusion_and_viewport_edges() {
    let (f, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (80, 100));
    // The projected source is outside a camera moved above it; retained native
    // source rays remain valid independent of the direct view's pixel buffers.
    let p = [100.5, 100.5, 30.0];
    let ray = reflected_ray([0.0, 1.0, 0.0]).unwrap();
    let first = native_hit(&f, 0, p, ray, RANGE).expect("nearby character intersects reflected ray");
    let mut hidden = f;
    hidden.surfaces.fill(Surface::default());
    hidden.actor_pixels.fill(false);
    hidden.hdr.fill(Rgb::BLACK);
    hidden.cam_y = -120;
    let second = native_hit(&hidden, 0, p, ray, RANGE).expect("off-screen/hidden native art is still present");
    assert_eq!(first.distance, second.distance);
    let color = source_color(None, &hidden, second.donor, &Grade::default());
    assert!(color.max_c() > 0.1);
}

#[test]
fn nearest_blocker_alpha_holes_and_distance_are_geometric() {
    let (mut f, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (0, 0));
    let p = [100.5, 100.5, 30.0];
    let ray = reflected_ray([0.0, 1.0, 0.0]).unwrap();
    let hit = native_hit(&f, 0, p, ray, RANGE).unwrap();
    assert!(native_hit(&f, 0, p, ray, hit.distance - 0.01).is_none());
    assert!(native_hit(&f, 0, [150.5, p[1], p[2]], ray, RANGE).is_none());
    let blocker = Cell::from_ascii(&["xxxxx"; 30], &[('x', Rgb::new(0.2, 0.2, 0.2))], &[]);
    f.sprite_w(&blocker, (2, 29), 100.0, 110.0, &DrawOpts::default());
    let blocked = scene_hit(&f, p, ray, 1, hit.distance).expect("nearer scenery blocks the actor reflection");
    assert!(blocked.distance < hit.distance);
    assert!(native_hit(&f, 0, [p[0], p[1] - 200.0, p[2]], ray, RANGE).is_none());
}

#[test]
fn scene_boundaries_and_zero_materials_do_not_change_or_panic() {
    let (mut f, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (0, 0));
    for c in &mut f.surface_casters { c.reflectivity = 0.0; }
    compose(&mut f);
    let before = f.out.clone(); let hdr = f.hdr.clone();
    apply(None, &mut f, &Grade::default());
    assert_eq!(f.out, before); assert_eq!(f.hdr, hdr);
    for camera in [(0, 0), (-384, -216), (704, 216), (1152, 648)] {
        (f.cam_x, f.cam_y) = camera;
        for ray in [[-0.3, 0.7, -0.6], [0.3, -0.7, 0.6], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for p in [[0.5, 30.5, 30.0], [383.5, 245.5, 30.0], [848.5, 280.5, 30.0], [1152.0, 700.0, 52.0]] {
                let _ = scene_hit(&f, p, ray, 99, RANGE);
            }
        }
    }
}

#[test]
fn offscreen_native_blocker_still_wins_over_a_retained_character() {
    let (mut f, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (0, -136));
    let blocker = Cell::from_ascii(&["xxxxx"; 30], &[('x', Rgb::new(0.2, 0.2, 0.2))], &[]);
    f.sprite_w(&blocker, (2, 29), 100.0, 110.0, &DrawOpts::default());
    let p = [100.5, 100.5, 30.0];
    let ray = reflected_ray([0.0, 1.0, 0.0]).unwrap();
    let actor = native_hit(&f, 0, p, ray, RANGE).unwrap();
    assert!(scene_hit(&f, p, ray, 1, actor.distance).is_none(), "blocker's direct view is outside the viewport");
    let hit = native_hit(&f, 1, p, ray, actor.distance).expect("retained scene geometry must block the reflection anyway");
    assert!(hit.distance < actor.distance);
    if let Donor::Native(id, _, _) = hit.donor { assert!(!f.reflection_sources[id].actor); }
    else { panic!("expected actual blocker art"); }
}

#[test]
fn hidden_source_lighting_respects_native_blockers_and_scenery_is_not_given_actor_fill() {
    use crate::gfx::frame::PointLight;
    let (mut f, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (0, 0));
    f.surfaces.fill(Surface::default());
    f.actor_pixels.fill(false);
    let donor = Donor::Native(0, 4 * 7 + 3, [100.5, 120.0, 6.0]);
    f.point_lights.push(PointLight { x: 100.5, y: 130.0, height: 10.0, radius: 60.0,
        color: Rgb::WHITE, intensity: 1.0 });
    let open = source_color(None, &f, donor, &Grade::default());
    let blocker = Cell::from_ascii(&["xxxxx"; 30], &[('x', Rgb::new(0.2, 0.2, 0.2))], &[]);
    f.sprite_w(&blocker, (2, 29), 100.0, 130.0, &DrawOpts::default());
    let shaded = source_color(None, &f, donor, &Grade::default());
    assert!(open.r > shaded.r + 0.1, "hidden sources need real light visibility");
    f.ambient = Rgb::gray(0.1);
    f.point_lights.clear();
    let scenery = source_color(None, &f, Donor::Native(1, 2, [100.5, 130.0, 10.0]), &Grade::default());
    assert!(scenery.max_c() < 0.03, "dark rock cannot inherit the hero's readability floor");
}

#[test]
fn native_glasswater_faces_reflect_actual_hero_beetle_and_wisp_poses() {
    use crate::{entities::{Player, Enemy, EnemyKind}, fx::Fx, regions, world::ZoneId};
    let (zone, region) = regions::build(ZoneId::Glasswater);
    for species in ["hero", "beetle", "wisp"] {
    let mut counts = Vec::new();
    for x in [516.0, 524.0, 532.0, 540.0, 548.0] {
        let player = Player::new(x, 82.0);
        let enemy = Enemy::spawn(if species == "wisp" { EnemyKind::Wisp } else { EnemyKind::CrystalBeetle }, x, 82.0);
        let mut frame = Frame::new();
        frame.cam_x = 340;
        crate::view::render_world(&zone, region.as_ref(), &Fx::new(7), &mut frame, 90,
            &[(82.0, 0)], &mut |f, _| if species == "hero" { player.draw(f); } else { enemy.draw(f); }, None);
        let mut body = std::collections::HashSet::new();
        let mut pixels = 0;
        let mut clear = 0;
        for (i, s) in frame.surfaces.iter().enumerate() {
            if s.kind != 2 || s.owner == 0 || frame.surface_casters[s.owner as usize - 1].reflectivity == 0.0 { continue; }
            let Some(ray) = reflected_ray(s.normal) else { continue; };
            let p = [(i % W) as f32 + frame.cam_x as f32 + 0.5,
                (i / W) as f32 + 0.5 + s.height, s.height];
            let mut closest = None;
            for id in 0..frame.reflection_sources.len() {
                if frame.reflection_sources[id].owner == s.owner { continue; }
                if let Some(hit) = native_hit(&frame, id, p, ray, closest.map_or(RANGE, |h: Hit| h.distance)) { closest = Some(hit); }
            }
            if let Some(Hit { donor: Donor::Native(id, pixel, _), distance }) = closest
                && frame.reflection_sources[id].actor
            {
                pixels += 1; body.insert(pixel);
                if !crate::gfx::lighting::reflection_blocked(&zone, p, std::array::from_fn(|a| p[a] + distance * ray[a])) { clear += 1; }
            }
        }
        counts.push((x, pixels, body.len(), clear));
    }
    println!("NATIVE_REFLECTION {species}: {counts:?}");
    assert!(counts.iter().any(|r| r.3 >= 8), "native faces must contain actual {species} imagery");
    }
}

#[test]
fn retained_local_lights_keep_offscreen_reflected_art_camera_independent() {
    let (mut visible, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (0, 0));
    let (mut outside, _) = fixture(Some(Rgb::new(0.9, 0.1, 0.1)), 100, (-340, 0));
    for f in [&mut visible, &mut outside] {
        f.surfaces.fill(Surface::default()); f.actor_pixels.fill(false);
        f.add_light_3d_w(100.5, 134.0, 8.0, 18.0, Rgb::WHITE, 1.0);
    }
    assert!(!visible.point_lights.is_empty());
    assert!(outside.point_lights.is_empty(), "direct lighting still culls the off-screen source");
    assert_eq!(outside.reflection_lights.len(), 1);
    assert_eq!(outside.reflection_lights[0].height, 8.0);
    let donor = Donor::Native(0, 4 * 7 + 3, [100.5, 120.0, 6.0]);
    let a = source_color(None, &visible, donor, &Grade::default());
    let b = source_color(None, &outside, donor, &Grade::default());
    assert_eq!(a, b);
    assert!(a.r > 0.42 * 0.9, "the retained light actually illuminates the reflected art");
    outside.begin(Rgb::BLACK, Rgb::WHITE);
    assert!(outside.reflection_lights.is_empty() && outside.reflection_sources.is_empty());
}
