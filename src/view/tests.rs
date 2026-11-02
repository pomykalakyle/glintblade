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

/// Check every changed screen pixel and every actual hidden body pixel. A whole-billboard
/// fade, a screen-space dither, or one unrevealed layer in a stack all violate this contract.
fn assert_local_reveal(opaque: &Frame, revealed: &Frame, pose: SpriteFocus, label: &str) {
    assert_eq!((opaque.cam_x, opaque.cam_y), (revealed.cam_x, revealed.cam_y));
    let mut changed = 0;
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let i = y as usize * W + x as usize;
            if opaque.albedo[i] != revealed.albedo[i] {
                let (wx, wy) = (x + opaque.cam_x, y + opaque.cam_y);
                assert!(pose.covers(wx, wy), "{label}: reveal escaped hero at {wx},{wy}");
                assert!(bayer4(wx, wy) >= 0.25, "{label}: incorrect reveal phase at {wx},{wy}");
                changed += 1;
            }
        }
    }
    let mut hidden = 0;
    let mut recovered = 0;
    for (x, y) in hero_pixels(pose) {
        if Frame::idx(x - opaque.cam_x, y - opaque.cam_y).is_none() { continue; }
        if at(opaque, x, y) == HERO_INK { continue; }
        hidden += 1;
        if bayer4(x, y) >= 0.25 {
            assert_eq!(at(revealed, x, y), HERO_INK, "{label}: hero still hidden at {x},{y}");
            recovered += 1;
        } else {
            assert_eq!(at(revealed, x, y), at(opaque, x, y), "{label}: opaque dither pixel changed");
        }
    }
    assert!(hidden > 40, "{label}: fixture must actually obscure the hero, got {hidden}");
    assert!(recovered > 25 && changed >= recovered, "{label}: no useful foreground reveal");
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
fn scenery_at_or_behind_the_hero_keeps_its_native_ink_even_when_the_hero_is_airborne() {
    let prop = Prop::new("cinder_foundry", "press", 180.0, 170.0);
    let depth = prop.sort_y();
    let zone = fixture(ZoneId::Cinder, vec![prop]);
    for ground_y in [depth, depth + 20.0] {
        let pose = SpriteFocus { ground_y, ..hero(175.0, 152.0) };
        let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
        let focused = render(&zone, &NoRegion, Some(pose), Some(pose), None, (0, 0));
        assert_eq!(opaque.albedo, focused.albedo, "reveal must use ground depth, not airborne sprite y");
    }
}

#[test]
fn stacked_scenery_keeps_one_world_aligned_reveal_through_camera_facing_and_airborne_changes() {
    let zone = fixture(ZoneId::Cinder, vec![
        Prop::new("cinder_foundry", "press", 180.0, 170.0),
        Prop::new("cinder_foundry", "broken_press", 182.0, 174.0).flipped(true),
    ]);
    for flip in [false, true] {
        for elevation in [0.0, 12.0] {
            let pose = SpriteFocus { flip, y: 152.0 - elevation, ..hero(177.75, 152.0) };
            let opaque = render(&zone, &NoRegion, Some(pose), None, None, (0, 0));
            let revealed = render(&zone, &NoRegion, Some(pose), Some(pose), None, (0, 0));
            assert_local_reveal(&opaque, &revealed, pose, "stacked foundry sprites");
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
fn zone_canopies_use_the_exact_pose_but_keep_point_only_callers_working() {
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
fn region_hooks_receive_depth_gated_pose_and_overlay_focus_in_render_order() {
    let zone = fixture(ZoneId::Cinder, Vec::new());
    let region = HookProbe::default();
    let pose = hero(160.0, 120.0);
    let mut frame = Frame::new();
    render_world_focused(&zone, &region, &Fx::new(5), &mut frame, 0, &[(120.0, 90)],
        &mut |_, id| region.calls.borrow_mut().push((id, false)),
        Some((160.0, 120.0)), Some(pose));
    assert_eq!(*region.calls.borrow(), [(1, false), (3, false), (90, false), (2, true), (100, true)]);
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
