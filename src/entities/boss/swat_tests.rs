use super::*;
use crate::combat::{Shape, Team};
use crate::game::{Game, Options, Scene};
use crate::input::Input;
use crate::world::{Zone, ZoneId};

#[test]
fn normal_front_lane_movement_triggers_and_takes_a_swat_without_crossing_sides() {
    let mut game = Game::new(Options {
        start: Some(ZoneId::Summit),
        ..Options::default()
    });
    if let Scene::Play(p) = &mut game.scene {
        p.player.respawn(720.0, 520.0);
        p.checkpoint = (720.0, 520.0);
    }
    // Stop 76 px in front, outside the old 54 px gate; no attacks, dashes or god mode.
    let mut steps = crate::tools::parse_script("W300 U65 W475")
        .into_iter()
        .flat_map(|(raw, n)| std::iter::repeat_n(raw, n as usize));
    let mut input = Input::default();
    let (mut selected, mut swept, mut hurt_by_sweep) = (false, false, false);
    let mut previous_hp = 6;
    for _ in 0..840 {
        input.step(steps.next().unwrap_or_default());
        game.tick(&input);
        let Scene::Play(p) = &game.scene else {
            panic!("left the fight")
        };
        let b = p.boss.as_ref().unwrap();
        assert!(!p.god && p.deaths == 0);
        assert!(p.player.body.x >= b.body.x, "never crosses to the idle side");
        assert!(
            p.player.body.y > b.body.y + 54.0,
            "never enters the old gate"
        );
        if let Some(Attack::Swat { side }) = b.attack {
            selected = true;
            assert_eq!(
                side, 0,
                "the idle left hand sweeps while Glint remains on the right"
            );
        }
        let active = b.fists.iter().any(|f| matches!(f.st, St::SwatSweep(_)));
        swept |= active;
        hurt_by_sweep |= active && p.player.body.hp < previous_hp;
        previous_hp = p.player.body.hp;
        game.fx.sounds.clear();
        game.fx.lights.clear();
    }
    assert!(
        selected && swept && hurt_by_sweep,
        "the enlarged trigger must produce a reachable hit"
    );
}

#[test]
fn swat_retains_lane_width_and_requires_player_still_in_front() {
    let zone = Zone::new(ZoneId::Summit, 60, 40);
    for (offset, expected) in [
        ((65.0, 99.0), true),
        ((66.1, 80.0), false),
        ((0.0, 100.1), false),
        ((0.0, -10.1), false),
    ] {
        let mut b = Boss::new(480.0, 240.0);
        b.mode = Mode::Fight;
        b.hug = 150; // Includes a player who just left after building the counter.
        for f in &mut b.fists {
            f.st = St::Rest;
        }
        let (mut fx, mut hits, mut projectiles) = (Fx::new(7), Vec::new(), Vec::new());
        b.choose(&mut EnemyCtx {
            player: (480.0 + offset.0, 240.0 + offset.1),
            player_alive: true,
            zone: &zone,
            fx: &mut fx,
            hits: &mut hits,
            projectiles: &mut projectiles,
            t: 0,
        });
        assert_eq!(
            matches!(b.attack, Some(Attack::Swat { .. })),
            expected,
            "offset {offset:?}"
        );
    }
}

#[test]
fn swats_leave_stuck_fists_available_and_cannot_replace_every_planned_attack() {
    let zone = Zone::new(ZoneId::Summit, 60, 40);
    for phase in 0..3 {
        let mut b = Boss::new(480.0, 240.0);
        b.mode = Mode::Fight;
        b.phase = phase;
        b.hug = 150;
        b.fists[0].st = St::Rest;
        b.fists[1].st = St::Stuck(20, 86);
        let (mut fx, mut hits, mut projectiles) = (Fx::new(7), Vec::new(), Vec::new());
        let mut ctx = EnemyCtx {
            player: (500.0, 320.0),
            player_alive: true,
            zone: &zone,
            fx: &mut fx,
            hits: &mut hits,
            projectiles: &mut projectiles,
            t: 0,
        };
        b.choose(&mut ctx);
        assert!(matches!(b.attack, Some(Attack::Swat { side: 0 })));
        assert_eq!(
            b.fists[1].st,
            St::Stuck(20, 86),
            "preserve the crystal punish window"
        );
        assert_eq!(b.plan_i, 0, "counter must not skip a planned attack");
        // At the following decision an idle hand and the dwell counter are ready again.
        b.attack = None;
        b.hug = 150;
        b.fists[1].st = St::Rest;
        b.choose(&mut ctx);
        assert!(!matches!(b.attack, None | Some(Attack::Swat { .. })));
        assert_eq!(
            b.plan_i, 1,
            "regular phase attacks must progress between counters"
        );
    }
}

#[test]
fn near_and_far_swipes_hit_their_warned_thin_arc_without_homing_or_gaps() {
    let base = (480.0, 240.0);
    let zone = Zone::new(ZoneId::Summit, 60, 40);
    for side in 0..2 {
        for offset in [(0.0, 54.0), (0.0, 90.0), (65.0, 99.0)] {
            let target = (base.0 + offset.0, base.1 + offset.1);
            let mut f = Fist::new(side, base.0, base.1);
            let mut fx = Fx::new(7);
            f.swat(base, target, &mut fx);
            let (mut covered, mut previous) = (false, None::<(f32, f32)>);
            let (mut damaging_ticks, mut swing_id) = (0, None);
            for _ in 0..100 {
                fx.particles.clear();
                let mut hits = Vec::new();
                f.update(&mut fist::Ctx {
                    base,
                    rest: (base.0, base.1, 0.0),
                    player: (base.0 + 200.0, base.1 + 200.0), // Move after the radius locks.
                    tempo: TEMPO[0],
                    fx: &mut fx,
                    hits: &mut hits,
                    zone: &zone,
                });
                for h in hits {
                    let Shape::Circle { x, y, r } = h.shape else {
                        panic!("swipe shape changed")
                    };
                    assert_eq!(r, 13.0, "keep the original damage thickness");
                    assert_eq!((h.team, h.dmg), (Team::Enemy, 1));
                    assert_eq!(
                        *swing_id.get_or_insert(h.id),
                        h.id,
                        "one damage event per swipe"
                    );
                    if let Some((px, py)) = previous {
                        assert!((x - px).hypot(y - py) < 26.0, "no gaps between hit circles");
                    }
                    previous = Some((x, y));
                    let radius = (x - base.0).hypot((y - base.1 - 8.0) / 0.8);
                    assert!((58.0 - 0.01..=128.0 + 0.01).contains(&radius));
                    // Warning pixels describe the same thin band as the fist and its hit.
                    let warning = fx.particles.debris.iter().filter(|p| p.glow > 0.0);
                    assert!(
                        warning
                            .clone()
                            .any(|p| (p.x as f32 - x).hypot(p.y as f32 - y) < 14.0)
                    );
                    assert!(
                        warning.into_iter().all(|p| {
                            let d = (p.x as f32 - base.0).hypot((p.y as f32 - base.1 - 8.0) / 0.8);
                            (d - radius).abs() <= 12.51
                        }),
                        "telegraph must not grow into a wider filled attack"
                    );
                    covered |= h.shape.touches(target.0, target.1, 5.0);
                    let safe = (
                        base.0 + (x - base.0) * 0.55,
                        base.1 + 8.0 + (y - base.1 - 8.0) * 0.55,
                    );
                    assert!(
                        !h.shape.touches(safe.0, safe.1, 5.0),
                        "the inside of the arc remains safe"
                    );
                    damaging_ticks += 1;
                }
                if matches!(f.st, St::SwatRest(_)) {
                    break;
                }
            }
            assert!(
                covered,
                "side {side}, target {offset:?} must remain reachable after radius locks"
            );
            assert!(damaging_ticks >= 14);
        }
    }
}
