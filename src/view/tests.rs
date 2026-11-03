//! Exercise the real world-rendering entry point, rather than only the sprite primitive.
//! Native cells and known reachable placements make missed scenery paths observable.

use super::*;
use crate::gfx::{DrawOpts, H, Rgb, W, rgb, sheet, sprite::SpriteFocus};
use crate::util::bayer4;
use crate::world::{NoRegion, Prop, ZoneId};
use std::cell::RefCell;

const HERO_INK: Rgb = rgb(0xff00d7);

fn hero(x: f32, y: f32) -> SpriteFocus {
    let sheet = sheet("hero").unwrap();
    let anim = sheet.anim("idle_down").unwrap();
    SpriteFocus { cell: sheet.cell(anim, 0), anchor: sheet.anchor_of(anim),
        flip: false, x, y, ground_y: y }
}

fn fixture(id: ZoneId, props: Vec<Prop>) -> Zone {
    let mut zone = Zone::new(id, 24, 14);
    zone.props = props;
    zone
}

/// A distinctive color lets us count actual native hero pixels independently of lighting.
/// The source cell, its silhouette, its facing and its elevation remain the real pose.
fn render(
    zone: &Zone, region: &dyn Region, actor: Option<SpriteFocus>,
    reveal: Option<SpriteFocus>, point: Option<(f32, f32)>, camera: (i32, i32),
) -> Frame {
    let mut frame = Frame::new();
    frame.cam_x = camera.0;
    frame.cam_y = camera.1;
    let ext: Vec<_> = actor.into_iter().map(|a| (a.ground_y, 0)).collect();
    render_world_focused(zone, region, &Fx::new(17), &mut frame, 0, &ext,
        &mut |f, _| {
            let actor = actor.unwrap();
            f.sprite_w(actor.cell, actor.anchor, actor.x, actor.y, &DrawOpts {
                flip: actor.flip, actor: true, elevation: actor.ground_y - actor.y,
                ground_y: Some(actor.ground_y), silhouette: Some(HERO_INK),
                ..Default::default()
            });
        }, point, reveal);
    frame
}

fn hero_pixels(pose: SpriteFocus) -> Vec<(i32, i32)> {
    let ox = pose.x.floor() as i32 - if pose.flip {
        pose.cell.w - 1 - pose.anchor.0
    } else { pose.anchor.0 };
    let oy = pose.y.floor() as i32 - pose.anchor.1;
    let mut pixels = Vec::new();
    for y in 0..pose.cell.h {
        for x in 0..pose.cell.w {
            if pose.cell.at(x, y).a {
                pixels.push((ox + if pose.flip { pose.cell.w - 1 - x } else { x }, oy + y));
            }
        }
    }
    pixels
}

fn at(frame: &Frame, wx: i32, wy: i32) -> Rgb {
    frame.get(wx - frame.cam_x, wy - frame.cam_y)
}

/// Check every changed screen pixel and every actual hidden body pixel. The contextual
/// opening is local, shares one world-space dither across layers, and retains physical shade.
fn assert_local_reveal(opaque: &Frame, revealed: &Frame, pose: SpriteFocus, label: &str) {
    assert_eq!((opaque.cam_x, opaque.cam_y), (revealed.cam_x, revealed.cam_y));
    let mut changed = 0;
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let i = y as usize * W + x as usize;
            if opaque.albedo[i] != revealed.albedo[i] {
                let (wx, wy) = (x + opaque.cam_x, y + opaque.cam_y);
                assert!(pose.covers(wx, wy), "{label}: reveal escaped local context at {wx},{wy}");
                assert!(bayer4(wx, wy) >= pose.scenery_alpha(wx, wy),
                    "{label}: incorrect reveal phase at {wx},{wy}");
                assert!(!pose.allows_scenery(wx, wy),
                    "{label}: retained scenery pixel changed at {wx},{wy}");
                changed += 1;
            }
        }
    }
    let mut hidden = 0;
    let mut recovered = 0;
    let mut retained = 0;
    for (x, y) in hero_pixels(pose) {
        if Frame::idx(x - opaque.cam_x, y - opaque.cam_y).is_none() { continue; }
        if at(opaque, x, y) == HERO_INK { continue; }
        hidden += 1;
        if !pose.allows_scenery(x, y) {
            assert_eq!(at(revealed, x, y), HERO_INK, "{label}: hero still hidden at {x},{y}");
            recovered += 1;
        } else {
            assert_eq!(at(revealed, x, y), at(opaque, x, y), "{label}: opaque dither pixel changed");
            retained += 1;
        }
    }
    assert!(hidden > 40, "{label}: fixture must actually obscure the hero, got {hidden}");
    assert!(recovered > 25 && changed >= recovered, "{label}: no useful foreground reveal");
    assert!(retained * 4 >= hidden, "{label}: too little occluding scenery retained: {retained}/{hidden}");
    assert_physical_casters_unchanged(opaque, revealed, label);
}

fn assert_physical_casters_unchanged(opaque: &Frame, revealed: &Frame, label: &str) {
    assert_eq!(opaque.surface_casters.len(), revealed.surface_casters.len(), "{label}: caster removed");
    for (before, after) in opaque.surface_casters.iter().zip(&revealed.surface_casters) {
        assert_eq!(before.opacity, after.opacity, "{label}: visibility fade weakened physical shadow");
        assert_eq!((before.x, before.y, before.base), (after.x, after.y, after.base));
        assert!(std::sync::Arc::ptr_eq(&before.pixels, &after.pixels), "{label}: caster geometry changed");
    }
}

#[test]
fn foreground_native_scenery_reveals_the_hero_in_every_land_region() {
    // Offsets reproduce legal, reachable hero/prop pairs in the shipped regions.
    let cases = [
        (ZoneId::Bloomwild, "bloomwild_growth", "body", -9.0, -11.0),
        (ZoneId::Bloomwild, "bloomwild_ruins", "pillar_broken_tall", 0.0, -16.0),
        (ZoneId::Bloomwild, "bloomwild_aqueduct", "left_pier", -7.0, -22.0),
        (ZoneId::Cinder, "cinder_foundry", "press", -5.0, -18.0),
        (ZoneId::Cinder, "cinder_foundry", "broken_press", -4.0, -18.0),
        (ZoneId::Cinder, "cinder_markers", "pillar_broken_tall", 2.0, -21.0),
        (ZoneId::Summit, "summit_stones", "monolith_1", -4.0, -44.0),
        (ZoneId::Summit, "summit_pillars", "pillar_0", -4.0, -48.0),
        (ZoneId::Summit, "summit_banners", "banner", 8.0, -24.0),
    ];
    for (id, sheet, anim, dx, dy) in cases {
        let prop = Prop::new(sheet, anim, 180.0, 170.0);
        assert!(!prop.canopy, "{sheet}/{anim} must exercise the y-sorted prop path");
        let pose = hero(prop.x + dx, prop.y + dy);
        assert!(prop.sort_y() > pose.ground_y);
        let zone = fixture(id, vec![prop]);
        let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
        let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), None, (0, 0));
        assert_local_reveal(&opaque, &revealed, pose, &format!("{sheet}/{anim}"));
    }
}

#[test]
fn native_foreground_opens_visible_ground_well_beyond_the_hero_body() {
    for (id, sheet, anim, prop_y, hero_y) in [
        (ZoneId::Bloomwild, "bloomwild_trees", "a_canopy", 180.0, 140.0),
        (ZoneId::Cinder, "cinder_foundry", "press", 170.0, 152.0),
    ] {
        let zone = fixture(id, vec![Prop::new(sheet, anim, 180.0, prop_y)]);
        let pose = hero(180.0, hero_y);
        let ground = render(&fixture(id, Vec::new()), &NoRegion, Some(pose), None, None, (0, 0));
        let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
        let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), None, (0, 0));
        let body = hero_pixels(pose);
        let (mut contextual_ground, mut left, mut right, mut exterior) = (0, 0, 0, 0);
        let mut furthest_ground = 0;
        for y in 70..190 {
            for x in 125..235 {
                // Inspect pixels actually hidden by native scenery, not empty parts of its cell.
                if at(&opaque, x, y) == at(&ground, x, y) { continue; }
                if !pose.covers(x, y) {
                    assert_eq!(at(&revealed, x, y), at(&opaque, x, y),
                        "{sheet}/{anim}: exterior native ink changed at {x},{y}");
                    exterior += 1;
                }
                let gap = body.iter().map(|&(hx, hy)| (x - hx).abs().max((y - hy).abs())).min().unwrap();
                if gap < 8 || at(&revealed, x, y) != at(&ground, x, y) { continue; }
                contextual_ground += 1;
                furthest_ground = furthest_ground.max(gap);
                if gap <= 16 {
                    if x < pose.x as i32 { left += 1; } else { right += 1; }
                }
            }
        }
        assert!(contextual_ground >= 60,
            "{sheet}/{anim}: only {contextual_ground} actual ground pixels opened at least 8 px beyond the body");
        assert!(left >= 4 && right >= 4,
            "{sheet}/{anim}: context must expose both sides of the player, got {left}/{right}");
        assert!(furthest_ground >= 12, "{sheet}/{anim}: opening is still a tight body outline");
        if sheet == "bloomwild_trees" {
            assert!(exterior >= 20, "the broad canopy must also exercise preserved exterior scenery");
        }
    }
}

#[test]
fn contextual_opening_keeps_its_width_across_facing_and_action_poses() {
    let s = sheet("hero").unwrap();
    let base = hero(180.75, 150.25);
    let width = |pose: SpriteFocus, y: i32| {
        (100..260).filter(|&x| pose.covers(x, y)).count() as i32
    };
    let rows = [126, 134, 142, 150];
    for name in ["idle_down", "idle_up", "idle_right", "run_right", "attack1_right", "dash_right"] {
        let a = s.anim(name).unwrap();
        for i in 0..a.frames {
            for flip in [false, true] {
                let pose = SpriteFocus { cell: s.cell(a, i), anchor: s.anchor_of(a), flip, ..base };
                for y in rows {
                    let (before, after) = (width(base, y), width(pose, y));
                    assert!(after >= 48,
                        "{name} frame {i}: context collapsed to a body-sized opening ({after} px at y={y})");
                    assert!((after - before).abs() <= 4,
                        "{name} frame {i}: scenery opening pumps with the animation ({before} -> {after} px)");
                }
            }
        }
    }
}

#[test]
fn native_press_cap_and_upright_remain_partially_visible() {
    let zone = fixture(ZoneId::Cinder, vec![
        Prop::new("cinder_foundry", "press", 621.0, 450.0),
    ]);
    let pose = hero(616.0, 432.0);
    let camera = (424, 324);
    let bare = render(&fixture(ZoneId::Cinder, Vec::new()), &NoRegion, None, None, None, camera);
    let opaque = render(&zone, &NoRegion, None, None, None, camera);
    let revealed = render(&zone, &NoRegion, None, Some(pose), None, camera);
    let (mut native_cap, mut retained_cap, mut exposed_cap) = (0, 0, 0);
    // Native cap coordinates from the frozen beauty probe. The cap and shaft
    // now retain scenery pixels throughout the opening.
    for y in 61..85 { for x in 194..212 {
        let i = y * W + x;
        if opaque.albedo[i] == bare.albedo[i] { continue; }
        native_cap += 1;
        retained_cap += usize::from(revealed.albedo[i] == opaque.albedo[i]);
        exposed_cap += usize::from(revealed.albedo[i] == bare.albedo[i]);
    } }
    assert!(native_cap >= 40, "fixture must exercise the native press cap");
    assert!(retained_cap * 4 >= native_cap && exposed_cap > 10,
        "cap must remain visible through the opening: {retained_cap}/{native_cap}");
}

#[test]
fn background_pillar_stays_intact_after_glint_has_passed_its_foot() {
    // Regression for the aqueduct route: the pillar rooted at (360,616)
    // lost its lower shaft when Glint reached (349,630.91), leaving a cap.
    let zone = fixture(ZoneId::Bloomwild, vec![
        Prop::new("bloomwild_ruins", "pillar", 360.0, 616.0),
    ]);
    let pose = hero(349.0, 630.91);
    let camera = (169, 506);
    let opaque = render(&zone, &NoRegion, Some(pose), None, None, camera);
    let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), None, camera);
    assert_eq!(revealed.albedo, opaque.albedo,
        "a pillar behind Glint must keep its complete shaft and cap");
    assert_eq!(revealed.emissive, opaque.emissive);

    // Real overhead foliage still opens when its root is behind the hero.
    let canopy = fixture(ZoneId::Bloomwild, vec![
        Prop::new("bloomwild_trees", "a_canopy", 360.0, 616.0),
    ]);
    let opaque = render(&canopy, &NoRegion, Some(pose), None, None, camera);
    let revealed = render(&canopy, &NoRegion, Some(pose), Some(pose), None, camera);
    assert!(revealed.albedo.iter().zip(&opaque.albedo).any(|(a,b)| a != b),
        "overhead canopies must not inherit ordinary background-prop restoration");
}

#[test]
fn scenery_restores_before_glint_crosses_its_foot_in_every_land_region() {
    for (id, name, anim) in [
        (ZoneId::Bloomwild, "bloomwild_ruins", "pillar"),
        (ZoneId::Cinder, "cinder_foundry", "press"),
        (ZoneId::Cinder, "cinder_foundry", "broken_press"),
        (ZoneId::Summit, "summit_pillars", "pillar_0"),
    ] {
        let prop = Prop::new(name, anim, 180.0, 170.0);
        let depth = prop.sort_y();
        let zone = fixture(id, vec![prop]);
        for delta in [-12.0, -6.0, -1.0, 0.0, 0.1, 1.0, 6.0, 12.0] {
            let pose = hero(180.0, depth + delta);
            let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
            let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), None, (0, 0));
            if delta >= 0.0 {
                assert_eq!(revealed.albedo, opaque.albedo, "{name}/{anim}: fading at foot delta {delta}");
                assert_eq!(revealed.emissive, opaque.emissive);
            } else if delta <= -6.0 {
                assert_ne!(revealed.albedo, opaque.albedo, "{name}/{anim}: foreground must still reveal Glint");
            }
            assert_physical_casters_unchanged(&opaque, &revealed, name);
        }
    }
}

#[test]
fn nearby_scenery_without_actual_hero_overlap_stays_opaque() {
    let zone = fixture(ZoneId::Cinder, vec![Prop::new("cinder_foundry", "press", 180.0, 170.0)]);
    // Find a native pose whose context reaches real press ink, but whose body does not.
    let prop = &zone.props[0];
    let s = prop.sheet().unwrap();
    let anim = s.anim(&prop.anim).unwrap();
    let cell = s.cell(anim, anim.frame_at(prop.t0, true));
    let anchor = s.anchor_of(anim);
    let source_at = |x, y| cell.at(x - 180 + anchor.0, y - 170 + anchor.1).a;
    let pose = (190..270).map(|x| hero(x as f32, 150.0)).find(|pose| {
        hero_pixels(*pose).iter().all(|&(x,y)| !source_at(x,y))
            && (100..230).any(|y| (100..270).any(|x| source_at(x,y) && pose.covers(x,y)))
    }).expect("real scenery inside the context radius without actual actor overlap");
    let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
    let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), None, (0, 0));
    assert_eq!(revealed.albedo, opaque.albedo);
    assert_eq!(revealed.emissive, opaque.emissive);
}

#[test]
fn stacked_scenery_keeps_one_world_aligned_reveal_through_camera_facing_and_airborne_changes() {
    let zone = fixture(ZoneId::Cinder, vec![
        Prop::new("cinder_foundry", "press", 180.0, 170.0),
        Prop::new("cinder_foundry", "broken_press", 182.0, 174.0).flipped(true),
    ]);
    let ordinary = fixture(ZoneId::Cinder, vec![zone.props[0].clone()]);
    for flip in [false, true] {
        for elevation in [0.0, 12.0] {
            let pose = SpriteFocus { flip, y: 152.0 - elevation, ..hero(177.75, 152.0) };
            let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
            let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), None, (0, 0));
            let ordinary_opaque = render(&ordinary, &NoRegion, Some(pose), None, None, (0, 0));
            let ordinary_revealed = render(&ordinary, &NoRegion, Some(pose), Some(pose), None, (0, 0));
            assert_local_reveal(&ordinary_opaque, &ordinary_revealed, pose, "underlying foundry sprite");
            assert_local_reveal(&opaque, &revealed, pose, "stacked foundry sprites");
            assert_physical_casters_unchanged(&opaque, &revealed, "stacked foundry sprites");
            let shifted = render(&zone, &NoRegion, Some(pose), Some(pose), None, (7, 5));
            for y in 70..185 {
                for x in 125..235 {
                    assert_eq!(at(&revealed, x, y), at(&shifted, x, y), "camera changed reveal at {x},{y}");
                }
            }
        }
    }
}

#[test]
fn zone_canopies_use_local_context_but_keep_point_only_callers_working() {
    let canopy = Prop::new("bloomwild_trees", "a_canopy", 180.0, 180.0);
    assert!(canopy.canopy);
    let zone = fixture(ZoneId::Bloomwild, vec![canopy]);
    let pose = hero(180.0, 140.0);
    let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
    let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), Some((180.0, 140.0)), (0, 0));
    assert_local_reveal(&opaque, &revealed, pose, "zone-owned canopy");
    let fallback = render(&zone, &NoRegion, Some(pose), None, Some((180.0, 140.0)), (0, 0));
    assert!(opaque.albedo.iter().zip(&fallback.albedo).enumerate().any(|(i, (a, b))| {
        a != b && !pose.covers((i % W) as i32, (i / W) as i32)
    }), "point-only callers retain the existing canopy fade");
    for caster in &fallback.surface_casters { assert_eq!(caster.opacity, 1.0); }
    let absent = render(&zone, &NoRegion, None, None, None, (0, 0));
    let mut manual = Frame::new();
    manual.begin(Rgb::BLACK, Rgb::WHITE);
    zone.props[0].draw(&mut manual, 0, 1.0);
    assert_eq!(absent.albedo, manual.albedo, "without a hero or focus the canopy stays completely opaque");
}

#[derive(Default)]
struct HookProbe {
    calls: RefCell<Vec<(u32, bool)>>,
    overlay: RefCell<Option<(Option<(f32, f32)>, bool)>>,
}

impl Region for HookProbe {
    fn sorted_items(&self, _zone: &Zone, _f: &Frame, out: &mut Vec<(f32, u32)>) {
        out.extend([(110.0, 1), (130.0, 2), (120.0, 3)]);
    }

    fn draw_item_revealed(&self, _zone: &Zone, _f: &mut Frame, id: u32, _t: u64, reveal: Option<SpriteFocus>) {
        self.calls.borrow_mut().push((id, reveal.is_some()));
    }

    fn draw_overlay_revealed(&self, _zone: &Zone, _f: &mut Frame, _t: u64,
        focus: Option<(f32, f32)>, reveal: Option<SpriteFocus>) {
        self.calls.borrow_mut().push((100, reveal.is_some()));
        *self.overlay.borrow_mut() = Some((focus, reveal.is_some()));
    }
}

#[test]
fn region_hooks_receive_context_pose_on_both_sides_of_actor_depth_in_render_order() {
    let zone = fixture(ZoneId::Cinder, Vec::new());
    let region = HookProbe::default();
    let pose = hero(160.0, 120.0);
    let mut frame = Frame::new();
    render_world_focused(&zone, &region, &Fx::new(5), &mut frame, 0, &[(120.0, 90)],
        &mut |_, id| region.calls.borrow_mut().push((id, false)),
        Some((160.0, 120.0)), Some(pose));
    assert_eq!(*region.calls.borrow(), [(1, true), (3, true), (90, false), (2, true), (100, true)]);
    assert_eq!(*region.overlay.borrow(), Some((Some((160.0, 120.0)), true)));
    region.calls.borrow_mut().clear();
    render_world(&zone, &region, &Fx::new(5), &mut frame, 0, &[], &mut |_, _| {}, None);
    assert!(region.calls.borrow().iter().all(|(_, reveal)| !reveal));
    assert_eq!(*region.overlay.borrow(), Some((None, false)), "a following title/no-hero frame must not retain a pose");
}

#[test]
fn new_reveal_hooks_preserve_regions_with_only_the_original_draw_hooks() {
    #[derive(Default)]
    struct OriginalHooks(RefCell<Vec<u32>>);
    impl Region for OriginalHooks {
        fn sorted_items(&self, _zone: &Zone, _f: &Frame, out: &mut Vec<(f32, u32)>) { out.push((150.0, 7)); }
        fn draw_item(&self, _zone: &Zone, _f: &mut Frame, id: u32, _t: u64) { self.0.borrow_mut().push(id); }
        fn draw_overlay(&self, _zone: &Zone, _f: &mut Frame, _t: u64) { self.0.borrow_mut().push(99); }
    }
    let region = OriginalHooks::default();
    let pose = hero(160.0, 120.0);
    let zone = fixture(ZoneId::Cinder, Vec::new());
    // The lighting bake legitimately invokes the original scenery hook as well.
    // Warm it first so this assertion isolates the live renderer's forwarding.
    crate::gfx::lighting::prepare(&zone, &region);
    region.0.borrow_mut().clear();
    render(&zone, &region, Some(pose), Some(pose), None, (0, 0));
    assert_eq!(*region.0.borrow(), [7, 99]);
}

#[test]
fn actual_region_renderers_reveal_reachable_foreground_scenery() {
    for (id, x, y) in [
        (ZoneId::Bloomwild, 764.0, 656.0),
        (ZoneId::Cinder, 616.0, 432.0),
        (ZoneId::Summit, 192.0, 712.0),
    ] {
        let (zone, region) = match id {
            ZoneId::Bloomwild => crate::regions::bloomwild::build(),
            ZoneId::Cinder => crate::regions::cinder::build(),
            ZoneId::Summit => crate::regions::summit::build(),
            ZoneId::Glasswater => unreachable!(),
        };
        assert!(!zone.blocked(x, y, 4.0), "{id:?}: hero probe must be legal in the shipped map");
        let pose = hero(x, y);
        let camera = (x as i32 - 180, y as i32 - 140);
        let opaque = render(&zone, region.as_ref(), Some(pose), None, None, camera);
        let revealed = render(&zone, region.as_ref(), Some(pose), Some(pose), Some((x, y)), camera);
        let visible = |frame: &Frame| hero_pixels(pose).into_iter()
            .filter(|&(x, y)| at(frame, x, y) == HERO_INK).count();
        assert!(visible(&revealed) >= visible(&opaque) + 100,
            "{id:?}: native region scenery still hides the reachable hero ({} -> {})",
            visible(&opaque), visible(&revealed));
    }
}
