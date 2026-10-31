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
        3,
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
