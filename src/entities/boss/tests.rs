//! Headless regression checks using the actual player, sword, collisions and scene loop.

use super::*;
use crate::input::{Input, Raw};
use crate::play::{Play, PlayOpts};
use crate::world::{NoRegion, Tile, Zone, ZoneId};

/// Build a walkable test arena while retaining the real scene and combat path.
fn arena() -> Play {
    let mut play = Play::new(
        ZoneId::Summit,
        &PlayOpts {
            god: true,
            peaceful: false,
        },
    );
    play.zone = Zone::new(ZoneId::Summit, 60, 40);
    for y in 0..40 {
        for x in 0..60 {
            play.zone.set_tile(x, y, Tile::Floor);
        }
    }
    play.region = Box::new(NoRegion);
    play.enemies.clear();
    play.boss = Some(Boss::new(480.0, 240.0));
    play.player.respawn(480.0, 288.0);
    play.checkpoint = (480.0, 288.0);
    play
}

/// Walk toward a visible weak point and tap the actual sword, without injecting damage.
fn swordsman(play: &Play, tick: u32) -> Raw {
    let b = play.boss.as_ref().unwrap();
    let (px, py) = (play.player.body.x, play.player.body.y);
    // Dodge only hazards already telegraphed on screen, using the player's real dash.
    for r in b.rings.iter().filter(|r| !r.harmless) {
        let d = (px - r.x).hypot(py - r.y);
        if (d - r.r).abs() < 18.0 {
            return Raw {
                mv: crate::util::norm(r.x - px, r.y - py),
                dash: tick % 2 == 0,
                ..Raw::default()
            };
        }
    }
    for beam in b.beams.iter().filter(|beam| beam.firing()) {
        let (dx, dy) = (beam.a.cos(), beam.a.sin());
        let (qx, qy) = (px - beam.ox, py - 8.0 - beam.oy);
        let perp = qx * -dy + qy * dx;
        if qx * dx + qy * dy > 0.0 && perp.abs() < 22.0 {
            let dir = if perp < 0.0 { -1.0 } else { 1.0 };
            return Raw {
                mv: (-dy * dir, dx * dir),
                dash: tick % 2 == 0,
                ..Raw::default()
            };
        }
    }
    for s in &b.shards {
        if s.t >= s.delay && (px - s.x).hypot(py - s.y) < s.radius() + 18.0 {
            let (dx, dy) = crate::util::norm(px - s.x + 0.1, py - s.y + 0.1);
            return Raw {
                mv: (dx, dy),
                ..Raw::default()
            };
        }
    }
    let mut target = None;
    if matches!(b.mode, Mode::Stagger(_)) {
        target = Some(b.heart_pos());
    } else if b.phase == 0 {
        target = b
            .fists
            .iter()
            .enumerate()
            .filter(|(_, f)| f.low() && f.crystal_alive())
            .map(|(i, _)| b.fist_crystal_center(i))
            .min_by(|a, c| ((a.0 - px).hypot(a.1 - py)).total_cmp(&(c.0 - px).hypot(c.1 - py)));
    } else if b.phase == 1 {
        target = b
            .pylons
            .iter()
            .filter(|p| p.alive)
            .map(|p| (p.x, p.y - 9.0))
            .min_by(|a, c| ((a.0 - px).hypot(a.1 - py)).total_cmp(&(c.0 - px).hypot(c.1 - py)));
    }
    if let Some((tx, ty)) = target {
        let (dx, dy) = (tx - px, ty + 5.0 - py);
        let dist = dx.hypot(dy);
        return Raw {
            mv: if dist > 20.0 || tick % 8 == 0 {
                crate::util::norm(dx, dy)
            } else {
                (0.0, 0.0)
            },
            attack: dist < 34.0 && tick % 8 == 0,
            ..Raw::default()
        };
    }
    // Stay close enough to reach openings, and step out of locked slam targets.
    let locked = b
        .fists
        .iter()
        .find(|f| matches!(f.st, St::Lock(_) | St::Drop(_)));
    let target = if let Some(f) = locked {
        (f.tx + if px < b.body.x { -35.0 } else { 35.0 }, f.ty - 14.0)
    } else {
        (b.body.x, b.body.y + 48.0)
    };
    let (dx, dy) = (target.0 - px, target.1 - py);
    Raw {
        mv: if dx.hypot(dy) > 5.0 {
            crate::util::norm(dx, dy)
        } else {
            (0.0, 0.0)
        },
        ..Raw::default()
    }
}

/// Prove all three phases, shield breaks, real sword openings and the finale can complete.
#[test]
fn sword_playthrough_reaches_victory() {
    let mut play = Play::new(
        ZoneId::Summit,
        &PlayOpts {
            god: false,
            peaceful: false,
        },
    );
    let (bx, by) = play.zone.boss_pos.expect("Summit must place the Warden");
    play.player.respawn(bx, by + 48.0);
    play.checkpoint = (bx, by + 48.0);
    let mut fx = Fx::new(7);
    let mut input = Input::default();
    let mut phases = [false; 3];
    let mut max_pixels = 0;
    for tick in 0..24_000 {
        input.step(swordsman(&play, tick));
        play.update(&input, &mut fx);
        phases[play.boss.as_ref().unwrap().phase] = true;
        max_pixels = max_pixels.max(fx.particles.list.len());
        fx.sounds.clear();
        fx.lights.clear();
        fx.tick();
        if play.victory {
            assert!(phases.into_iter().all(|p| p));
            assert!(
                play.deaths <= 2,
                "readable attacks should allow victory within three tries"
            );
            assert!(
                max_pixels > 2_000,
                "finale must shatter into thousands of pixels"
            );
            println!(
                "Warden defeated with real sword inputs in {tick} frames; peak {max_pixels} particles; {} deaths",
                play.deaths
            );

            return;
        }
    }
    let b = play.boss.as_ref().unwrap();
    panic!(
        "fight stalled: phase {}, hp {}, mode {:?}, attack {:?}",
        b.phase, b.hp, b.mode, b.attack
    );
}

/// Every charge/sweep has a readable native source or an explicit edge cue, including twins.
/// This deliberately includes the non-fitting source/hero geometry from the real 1200 replay.
#[test]
fn beam_warnings_remain_readable_across_prism_and_cracked_phases() {
    let mut play = arena();
    let mut fx = Fx::new(7);
    let mut hits = Vec::new();
    let mut b = play.boss.take().unwrap();
    let mut ctx = EnemyCtx {
        player: (480.0, 452.0),
        player_alive: true,
        zone: &play.zone,
        fx: &mut fx,
        hits: &mut hits,
        projectiles: &mut play.projectiles,
        t: 0,
    };
    let mut saw = [[false; 2]; 2];
    for (pi, phase) in [1, 2].into_iter().enumerate() {
        b.phase = phase;
        b.mode = Mode::Fight;
        b.beams.clear();
        b.fire_beams(phase == 2, &mut ctx);
        for py in [280.0, 355.0, 452.0] {
            for t in 0..350 {
                for beam in &mut b.beams {
                    beam.t = t;
                }
                let mut cam = crate::camera::Camera::new(480.0, py - 8.0);
                for _ in 0..90 {
                    cam.frame_attack_sources((480.0, py), b.attack_sources());
                }
                let (cx, cy) = cam.view(960, 640, 0.8, t as u64);
                let mut f = Frame::new();
                f.cam_x = cx;
                f.cam_y = cy;
                b.draw_attack_cues(&mut f);
                for beam in b
                    .beams
                    .iter()
                    .filter(|beam| beam.charging() || beam.firing())
                {
                    saw[pi][usize::from(beam.firing())] = true;
                    let source = (beam.ox - cx as f32, beam.oy - cy as f32);
                    if let Some((sx, sy, dx, dy)) = attack_cue(source) {
                        assert!((16..=368).contains(&sx) && (28..=178).contains(&sy));
                        let i = sy as usize * crate::gfx::W + sx as usize;
                        assert_ne!(
                            f.out[i],
                            [0, 0, 0, 255],
                            "offscreen warning must actually draw"
                        );
                        let (ax, ay) = (sx + dx * 9, sy + dy * 9);
                        assert!(
                            ay >= 19 && ay < 196 && ax >= 7 && ax < 377,
                            "cue must clear both HUD bands"
                        );
                    } else {
                        assert!(
                            (16.0..=368.0).contains(&source.0)
                                && (28.0..=178.0).contains(&source.1)
                        );
                    }
                }
            }
        }
    }
    assert!(
        saw.into_iter().flatten().all(|s| s),
        "exercise charge and live sweep in both beam phases"
    );
}

/// Verify death during an attack clears hazards and restarts at the current phase checkpoint.
#[test]
fn retry_and_heart_opening_clear_hostile_hazards() {
    let mut play = arena();
    let mut fx = Fx::new(7);
    let mut hits = Vec::new();
    let b = play.boss.as_mut().unwrap();
    let mut ctx = EnemyCtx {
        player: (480.0, 288.0),
        player_alive: false,
        zone: &play.zone,
        fx: &mut fx,
        hits: &mut hits,
        projectiles: &mut play.projectiles,
        t: 0,
    };
    b.phase = 1;
    b.hp = 25;
    b.mode = Mode::Fight;
    b.fire_beams(false, &mut ctx);
    b.rain(&mut ctx);
    b.rings
        .push(Ring::new(480.0, 260.0, 12.0, 2.0, 200.0, 100, false));
    b.update(&mut ctx);
    assert_eq!(b.hp, 40);
    assert!(matches!(b.mode, Mode::Rest(_)));
    assert!(b.beams.is_empty() && b.shards.is_empty() && b.rings.is_empty());
    assert!(b.hurtboxes().is_empty() && ctx.hits.is_empty());
    ctx.player_alive = true;
    for _ in 0..200 {
        b.update(&mut ctx);
    }
    assert_eq!(b.mode, Mode::Fight);
    assert!(
        !b.shards.is_empty() || !b.pylons.is_empty(),
        "retry must restore phase-two shields"
    );
    b.fire_beams(false, &mut ctx);
    b.rain(&mut ctx);
    b.start_stagger(&mut ctx);
    assert!(b.beams.is_empty() && b.shards.is_empty() && b.rings.is_empty());
}

/// Verify an aimed slam locks its target before landing and leaves a reachable weak point.
#[test]
fn slam_has_a_fixed_dodge_window_and_punish_window() {
    let mut play = arena();
    let mut fx = Fx::new(7);
    let mut hits = Vec::new();
    let mut c = fist::Ctx {
        base: (480.0, 240.0),
        rest: (430.0, 242.0, 20.0),
        player: (480.0, 288.0),
        tempo: TEMPO[0],
        fx: &mut fx,
        hits: &mut hits,
        zone: &play.zone,
    };
    let f = &mut play.boss.as_mut().unwrap().fists[0];
    f.slam(c.player, c.fx);
    for _ in 0..TEMPO[0].seek {
        f.update(&mut c);
    }
    let locked = (f.tx, f.ty);
    c.player = (580.0, 350.0);
    for _ in 0..TEMPO[0].lock + 4 {
        f.update(&mut c);
    }
    assert_eq!((f.tx, f.ty), locked);
    assert!(c.hits.is_empty());
    f.update(&mut c);
    assert_eq!(c.hits.len(), 1);
    assert!(f.stuck() && f.low() && f.crystal_alive());
    assert!(c.hits[0].shape.touches(locked.0, locked.1, 0.0));
    assert!(!c.hits[0].shape.touches(c.player.0, c.player.1, 4.0));
}

/// Verify slam poses follow the sheet (wind-up, fall, impact at contact, recover) without
/// stacking the sheet's baked lift on top of the fist's runtime height.
#[test]
fn slam_poses_match_contact_without_double_lift() {
    let mut play = arena();
    let mut fx = Fx::new(7);
    let mut hits = Vec::new();
    let mut c = fist::Ctx {
        base: (480.0, 240.0),
        rest: (430.0, 242.0, 20.0),
        player: (480.0, 288.0),
        tempo: TEMPO[0],
        fx: &mut fx,
        hits: &mut hits,
        zone: &play.zone,
    };
    let f = &mut play.boss.as_mut().unwrap().fists[0];
    let rest = spr_frame(art::FIST, &["slam"], 4).unwrap();
    let mut seen = Vec::new();
    f.slam(c.player, c.fx);
    for _ in 0..TEMPO[0].seek + TEMPO[0].lock + 5 + 20 {
        f.update(&mut c);
        let frame = slam_frame(f.st);
        let (s, lift, _) = fist_pose(f);
        if matches!(frame, Some(0..=2)) {
            // the hand's bottom lands on the runtime anchor, so only `z` lifts it
            assert_eq!(art::bottom(s) + lift, art::bottom(rest), "{:?}", f.st);
        }
        if seen.last() != Some(&frame) {
            seen.push(frame);
        }
        if f.impact {
            assert_eq!(frame, Some(3), "impact pose on the contact tick");
        }
    }
    assert_eq!(seen, [Some(0), None, Some(1), Some(2), Some(3), Some(4)]);
}

/// Guard approved baked motion and attachment contours across Warden sheet restyles.
#[test]
fn part_contours_preserve_approved_attachments() {
    let rest = spr_frame(art::FIST, &["slam"], 4).unwrap();
    for (i, (lift, back)) in [(9, 0), (23, -3), (3, -5), (0, 6), (0, 0)]
        .into_iter()
        .enumerate()
    {
        let s = spr_frame(art::FIST, &["slam"], i).unwrap();
        let measured_lift = art::baked_lift(s, rest);
        assert_eq!(
            measured_lift, lift,
            "slam {i}: coordinate changed baked motion with the artist"
        );
        assert_eq!(
            art::solid_top(s) + measured_lift - art::solid_top(rest),
            back,
            "slam {i}: coordinate changed crystal attachment with the artist"
        );
    }
    let idle = spr(art::HEAD, &["idle"], 0, true);
    assert_eq!(
        art::peak_lift(art::HEAD, "roar", idle),
        if pose::rig().integrated_head { 0 } else { 3 },
        "coordinate changed roar motion with the artist"
    );
}

/// Verify the return sweep fades at its actual endpoint instead of snapping across the arena.
#[test]
fn returning_beam_keeps_its_angle_during_fade() {
    let mut play = arena();
    let mut fx = Fx::new(7);
    let mut hits = Vec::new();
    let mut ctx = EnemyCtx {
        player: (480.0, 288.0),
        player_alive: true,
        zone: &play.zone,
        fx: &mut fx,
        hits: &mut hits,
        projectiles: &mut play.projectiles,
        t: 0,
    };
    let mut b = Beam {
        ox: 440.0,
        oy: 180.0,
        a: 0.5,
        a0: 0.5,
        a1: 2.4,
        t: 0,
        charge: 20,
        sweep: 30,
        back: true,
        id: 1,
        side: 0,
    };
    for _ in 0..80 {
        b.update(&mut ctx, 440.0, 180.0);
    }
    assert!((b.a - b.a0).abs() < 0.001);
    assert!(!b.firing());
    b.update(&mut ctx, 440.0, 180.0);
    assert!((b.a - b.a0).abs() < 0.001);
}

/// The reported legal route ends behind the Warden, without a teleport through its feet.
/// Compare actual native hero colors: both combatants stamp `actor_pixels`.
#[test]
fn summit_rear_walk_reveals_native_glint_and_surrounding_floor_in_the_play_scene() {
    for (start_x, ticks, movement) in [
        (640.0, 50, (1.0, 0.0)), // ordinary R50 route from the audit
        (720.0, 15, (0.0, 0.0)), // exact clean15 native screenshot fixture
    ] {
        let mut play = Play::new(ZoneId::Summit, &PlayOpts { god: true, peaceful: false });
        play.player = crate::entities::Player::new(start_x, 320.0);
        play.enemies.clear();
        let mut input = Input::default();
        let mut fx = Fx::new(15);
        for _ in 0..ticks {
            input.step(Raw { mv: movement, ..Raw::default() });
            play.update(&input, &mut fx);
            fx.tick();
        }
        assert!(play.player.alive() && play.deaths == 0);
        if movement.0 > 0.0 {
            assert!((705.0..=716.0).contains(&play.player.body.x),
                "ordinary R50 must reach the reported rear floor: {}", play.player.body.x);
        } else {
            assert!((play.player.body.x - 720.0).abs() < 0.1);
        }
        assert!((play.player.body.y - 320.0).abs() < 0.1);
        assert!(!play.zone.blocked(play.player.body.x, play.player.body.y, play.player.body.r));
        let focus = play.player.foreground_focus().expect("living native hero pose");
        let boss = play.boss.take().expect("actual Summit Warden");
        assert!(boss.sort_y() > focus.ground_y);

        let mut background = Frame::new();
        play.render(&mut background, &fx);
        play.boss = Some(boss);
        let mut revealed = Frame::new();
        play.render(&mut revealed, &fx);
        assert_eq!((background.cam_x, background.cam_y), (revealed.cam_x, revealed.cam_y));
        // Draw the old opaque boss over the identical underlying scene; lighting does not
        // alter albedo, so this preserves the exact native player/floor comparison.
        let mut opaque = Frame::new();
        opaque.cam_x = revealed.cam_x;
        opaque.cam_y = revealed.cam_y;
        opaque.albedo.clone_from(&background.albedo);
        opaque.surface_lighting = revealed.surface_lighting;
        play.boss.as_ref().unwrap().draw(&mut opaque);
        let (ox, oy) = art::origin(focus.cell, focus.anchor,
            revealed.sx(focus.x), revealed.sy(focus.y), focus.flip);
        let (mut hidden, mut recovered) = (0, 0);
        for y in 0..focus.cell.h {
            for x in 0..focus.cell.w {
                let source = focus.cell.at(x, y);
                if !source.a { continue; }
                let px = ox + if focus.flip { focus.cell.w - 1 - x } else { x };
                let Some(i) = Frame::idx(px, oy + y) else { continue };
                // Scarves, blade glints and material flashes may cover some source pixels.
                if background.albedo[i] != source.c || opaque.albedo[i] == source.c { continue; }
                hidden += 1;
                if revealed.albedo[i] == source.c { recovered += 1; }
            }
        }
        assert!(hidden > 20, "the real route must exercise native hero occlusion: {hidden}");
        assert!(recovered > 20, "native Glint pixels remain hidden by the Warden: {recovered}/{hidden}");
        let mut surrounding_floor = 0;
        for y in 0..crate::gfx::HI {
            for x in 0..crate::gfx::WI {
                let i = y as usize * crate::gfx::W + x as usize;
                let wx = x + revealed.cam_x;
                if (wx - focus.x.floor() as i32).abs() < 16 { continue; }
                if background.surfaces[i].kind == 1 && opaque.albedo[i] != background.albedo[i]
                    && revealed.albedo[i] == background.albedo[i]
                {
                    surrounding_floor += 1;
                }
            }
        }
        assert!(surrounding_floor > 30,
            "the real scene needs nearby floor beyond Glint's body, got {surrounding_floor}");
    }
}

/// Moving stone, its fractures and its physical shadow share one local opening.
/// Crystals, the heart, charge/live beams and offscreen warnings retain their pixels.
#[test]
fn warden_stone_reveal_preserves_native_casters_and_combat_sources_across_poses() {
    use crate::gfx::{rgb, sprite::SpriteFocus};
    const BACKGROUND: Rgb = rgb(0xf400b9);
    let hero = crate::gfx::sheet("hero").unwrap();
    let anim = hero.anim("idle_down").unwrap();
    let make_frame = || {
        let mut f = Frame::new();
        f.begin(BACKGROUND, Rgb::WHITE);
        f.surface_lighting = true;
        f
    };
    let mut protected_near = 0;
    let mut crack_pixels_removed = 0;
    for (phase, kneel, state, height) in [
        (0, 0.0, St::Rest, 20.0),
        (2, 0.0, St::Drop(2), 24.0),
        (2, kneel_depth(), St::Stuck(10, 80), 0.0),
    ] {
        let mut b = Boss::new(192.0, 180.0);
        b.mode = Mode::Fight;
        b.phase = phase;
        b.body_dy = kneel;
        b.crack_lit = b.cracks.len() as f32;
        b.eyes = 1.0;
        for fi in &mut b.fists {
            fi.st = state;
            fi.z = height;
            fi.trail = [(fi.x, fi.y, fi.z); 3];
        }
        let (head, hx, hy) = b.head_part();
        let (fist, lift, _) = fist_pose(&b.fists[0]);
        // Use the actual source bounds; this also works with an older art override.
        let head_mid = (art::top(head) + art::bottom(head)) / 2;
        let fist_mid = (art::top(fist) + art::bottom(fist)) / 2;
        let targets = [
            (b.body.x, b.body.y - 32.0),
            (hx, hy + head_mid as f32 + 15.0),
            (b.fists[0].x, b.fists[0].y - height + (lift + fist_mid) as f32 + 15.0),
            (b.heart_pos().0, b.heart_pos().1 + 15.0),
        ];
        for (px, py) in targets {
            let focus = SpriteFocus { cell: hero.cell(anim, 0), anchor: hero.anchor_of(anim),
                flip: false, x: px, y: py, ground_y: py };
            let mut opaque = make_frame();
            let mut revealed = make_frame();
            b.draw(&mut opaque);
            b.draw_revealed(&mut revealed, Some(focus));
            let changed = opaque.albedo.iter().zip(&revealed.albedo)
                .filter(|(a, c)| a != c).count();
            assert!(changed > 20, "phase {phase}, kneel {kneel}, target {px},{py}: no stone opening");
            let mut opened_background = 0;
            for (i, (before, after)) in opaque.albedo.iter().zip(&revealed.albedo).enumerate() {
                if before == after { continue; }
                let (x, y) = ((i % crate::gfx::W) as i32, (i / crate::gfx::W) as i32);
                assert!(focus.covers(x, y), "a local stone reveal changed distant boss art");
                // An opened foreground part can expose a different-depth stone part,
                // or a native combat emitter whose dark source remains protected.
                if *after == BACKGROUND { opened_background += 1; }
            }
            assert!(opened_background > 20, "stone opening exposed no useful surroundings");

            // Check fracture masking on the actual native torso, independently of
            // the protected emitters and restored parts in the assembled boss.
            let body = b.body_sprite();
            let body_look = Look { ground_y: Some(b.body.y + 1.0),
                clip_y: if kneel > 0.5 { b.body.y as i32 } else { i32::MAX },
                ..Default::default() };
            let fold = if kneel > 0.0 {
                spr_frame(art::BODY, &[b.body_fold_anim()], 0)
                    .map(|s| (s, kneel as i32))
            } else { None };
            let mut bare_cracks = make_frame();
            let mut revealed_cracks = make_frame();
            for (f, look) in [(&mut bare_cracks, body_look),
                (&mut revealed_cracks, Look { reveal: Some(focus), ..body_look })]
            {
                art::draw_cracks(f, body, b.body.x as i32, (b.body.y + kneel) as i32,
                    &look, &b.cracks, b.cracks.len(), P2, PW, fold);
            }
            for (i, before) in bare_cracks.albedo.iter().enumerate() {
                if *before == BACKGROUND { continue; }
                let (x, y) = ((i % crate::gfx::W) as i32, (i / crate::gfx::W) as i32);
                if !focus.allows_scenery_at_depth(x, y, b.body.y + 1.0) {
                    assert_eq!(revealed_cracks.albedo[i], BACKGROUND,
                        "native fracture painted back into opened torso at {x},{y}");
                    crack_pixels_removed += 1;
                } else {
                    assert_eq!(revealed_cracks.albedo[i], *before,
                        "native fracture outside the opening changed");
                }
            }
            assert_eq!(opaque.surface_casters.len(), revealed.surface_casters.len());
            for (before, after) in opaque.surface_casters.iter().zip(&revealed.surface_casters) {
                assert_eq!((before.w, before.h, before.anchor, before.x, before.y, before.base,
                    before.flip, before.opacity, before.min_height),
                    (after.w, after.h, after.anchor, after.x, after.y, after.base,
                    after.flip, after.opacity, after.min_height));
                assert_eq!(before.opacity, 1.0, "presentation fade weakened physical stone");
                assert_eq!(before.foot.as_ref(), after.foot.as_ref());
                assert_eq!(before.pixels.len(), after.pixels.len());
                for (a, c) in before.pixels.iter().zip(after.pixels.iter()) {
                    assert_eq!((a.n, a.a, a.emit), (c.n, c.a, c.emit),
                        "opening changed the native physical silhouette");
                }
            }
            // Draw each protected crystal at its real attachment and compare source RGB,
            // discarding only pixels legitimately covered by a later opaque part.
            let cs = spr(art::CRYSTAL, &["intact", "idle"], b.t, true);
            let mut crystal_anchors = vec![(b.shoulder_pos(0), false), (b.shoulder_pos(1), true)];
            for fi in &b.fists {
                let (_, _, back) = fist_pose(fi);
                crystal_anchors.push(((fi.x + fi.jit as f32,
                    fi.y - fi.z.max(0.0) - 22.0 + back as f32), fi.side == 0));
            }
            for ((cx, cy), flip) in crystal_anchors {
                let (ox, oy) = art::origin(cs.cell, cs.anchor, cx.floor() as i32, cy.floor() as i32, flip);
                for y in 0..cs.cell.h {
                    for x in 0..cs.cell.w {
                        let source = cs.cell.at(x, y);
                        if !source.a { continue; }
                        let wx = ox + if flip { cs.cell.w - 1 - x } else { x };
                        let wy = oy + y;
                        let Some(i) = Frame::idx(wx, wy) else { continue };
                        if opaque.albedo[i] != source.c { continue; }
                        assert_eq!(revealed.albedo[i], source.c, "reveal erased a combat crystal");
                        if focus.covers(wx, wy) { protected_near += 1; }
                    }
                }
            }
            let mut heart_only = make_frame();
            b.draw_heart(&mut heart_only);
            for (i, source) in heart_only.albedo.iter().enumerate() {
                if *source == BACKGROUND || opaque.albedo[i] != *source { continue; }
                assert_eq!(revealed.albedo[i], *source, "reveal erased the heart weak point");
                if focus.covers((i % crate::gfx::W) as i32, (i / crate::gfx::W) as i32) {
                    protected_near += 1;
                }
            }
        }

        for beam_t in [20, 45] {
            let source = b.beam_src(0);
            b.beams = vec![Beam { ox: source.0, oy: source.1, a: 0.7, a0: 0.7, a1: 1.5,
                t: beam_t, charge: 30, sweep: 80, back: false, id: 41, side: 0 }];
            let focus = SpriteFocus { cell: hero.cell(anim, 0), anchor: hero.anchor_of(anim),
                flip: false, x: source.0, y: source.1 + 15.0, ground_y: source.1 + 15.0 };
            let mut opaque = make_frame();
            let mut revealed = make_frame();
            let mut beam_only = make_frame();
            b.draw(&mut opaque);
            b.draw_revealed(&mut revealed, Some(focus));
            b.beams[0].draw_at_height(&mut beam_only, (b.body.y + 1.0 - source.1).max(0.0));
            let mut beam_pixels = 0;
            for (i, color) in beam_only.albedo.iter().enumerate() {
                if *color == BACKGROUND { continue; }
                assert_eq!(revealed.albedo[i], opaque.albedo[i], "charge/live beam pixel was faded");
                beam_pixels += 1;
            }
            assert!(beam_pixels > 20, "charge/live source fixture drew no attack");
            let lights = |f: &Frame| f.point_lights.iter().map(|p|
                (p.x, p.y, p.height, p.radius, p.color, p.intensity)).collect::<Vec<_>>();
            assert_eq!(lights(&revealed), lights(&opaque), "reveal altered attack/heart light output");
            opaque.cam_y = 200;
            revealed.cam_y = 200;
            b.draw_attack_cues(&mut opaque);
            b.draw_attack_cues(&mut revealed);
            assert_eq!(revealed.out, opaque.out, "offscreen attack warning changed");
            assert!(revealed.out.iter().filter(|p| **p != [0, 0, 0, 255]).count() > 20,
                "the offscreen attack warning fixture must actually draw");
        }
    }
    assert!(protected_near > 40, "the opening never overlapped protected weak points");
    assert!(crack_pixels_removed > 10, "native fractures were never tested inside the opening");
}

/// Capture real player combat, or a selected phase, without exposing debug hooks to the game.
#[test]
#[ignore = "manual visual capture; GLINT_WARDEN_CAPTURE chooses output directory"]
fn capture_warden_fight() {
    let out =
        std::path::PathBuf::from(std::env::var("GLINT_WARDEN_CAPTURE").expect("capture directory"));
    std::fs::create_dir_all(&out).unwrap();
    let mut play = Play::new(
        ZoneId::Summit,
        &PlayOpts {
            god: true,
            peaceful: false,
        },
    );
    let (bx, by) = play.zone.boss_pos.unwrap();
    play.player.respawn(bx, by + 44.0);
    play.boss_awake = true;
    play.arrive_t = 10_000;
    let phase: usize = std::env::var("GLINT_WARDEN_PHASE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let b = play.boss.as_mut().unwrap();
    if phase > 0 {
        b.phase = phase.min(2);
        b.hp = b.top_hp();
        b.mode = Mode::Fight;
        b.eyes = 1.0;
        b.fists.iter_mut().for_each(Fist::go_home);
        if phase == 1 {
            b.attack = Some(Attack::Summon { t: 0 });
        }
        if phase == 3 {
            b.hp = 0;
            b.body.hp = 0;
            b.mode = Mode::Dying(0);
            b.fists.iter_mut().for_each(Fist::throes);
        }
    }
    let ticks: u32 = std::env::var("GLINT_WARDEN_TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(900);
    let mut fx = Fx::new(7);
    let mut input = Input::default();
    let mut frame = Frame::new();
    for tick in 0..ticks {
        input.step(swordsman(&play, tick));
        play.update(&input, &mut fx);
        if tick % 4 == 0 {
            // A fixed arena camera keeps the 120px guardian and the sword in frame.
            play.cam.snap(bx, by - 22.0);
            play.render(&mut frame, &fx);
            crate::ui::hud(&mut frame, &play, &crate::ui::Hud::new());
            crate::gfx::post::finish(&mut frame, &play.zone.grade);
            let img = crate::gfx::post::composite_scaled(&frame, 3);
            crate::assets::write_png(&out.join(format!("f{tick:05}.png")), 1152, 648, &img)
                .unwrap();
        }
        fx.sounds.clear();
        fx.lights.clear();
        fx.tick();
    }
}
