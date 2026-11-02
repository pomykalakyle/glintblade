//! Compare simulation receipts against a frozen pre-eightway source-and-asset build.
//! Visual facing, pixels and particles are deliberately excluded; combat and RNG are exact.
use glintblade::combat::{Hit, Shape, Team};
use glintblade::entities::player::PlayerCtx;
use glintblade::entities::{Body, Enemy, EnemyCtx, EnemyKind, Player, Projectile};
use glintblade::fx::Fx;
use glintblade::input::{Input, Raw};
use glintblade::world::{Zone, ZoneId};
use serde_json::json;
use std::io::{BufWriter, Write};

fn receipt(body: &Body, fx: &Fx, hits: &[Hit], shots: &[Projectile]) -> String {
    format!(
        "{body:?}|{:?}|{hits:?}|{shots:?}|{:?}|{}|{}|{}",
        fx.rng,
        fx.sounds
            .iter()
            .map(|s| (&s.name, s.volume.to_bits()))
            .collect::<Vec<_>>(),
        fx.hitstop,
        fx.trauma.to_bits(),
        fx.flash.to_bits()
    )
}

fn appearance(x: f32, y: f32) -> u8 {
    (glintblade::util::hash2(x as i32, y as i32, 941) * 3.0) as u8 % 3
}

fn main() {
    let path = std::env::args().nth(1).expect("output JSONL path");
    let mut out = BufWriter::new(std::fs::File::create(path).unwrap());
    let zone = Zone::new(ZoneId::Bloomwild, 192, 192);
    let directions = [
        (0., -1.),
        (1., -1.),
        (1., 0.),
        (1., 1.),
        (0., 1.),
        (-1., 1.),
        (-1., 0.),
        (-1., -1.),
    ];
    for (d, mv) in directions.into_iter().enumerate() {
        let mut p = Player::new(1024., 1024.);
        let mut fx = Fx::new(739);
        let mut input = Input::default();
        for t in 0..540u32 {
            let moving = t < 60 || (160..220).contains(&t) || (270..300).contains(&t);
            input.step(Raw {
                mv: if moving { mv } else { (0., 0.) },
                attack: [80, 100, 120].contains(&t) || (160..220).contains(&t),
                dash: t == 270 || t == 331,
                ..Raw::default()
            });
            let mut hits = Vec::new();
            fx.sounds.clear();
            fx.tick();
            if t == 320 || t == 420 {
                p.body.iframes = 0;
                let hit = Hit::new(
                    Shape::Circle {
                        x: p.body.x,
                        y: p.body.y,
                        r: 5.,
                    },
                    Team::Enemy,
                    if t == 420 { 99 } else { 1 },
                    9000 + t,
                )
                .dir(mv.0, mv.1);
                p.on_hit(&hit, &mut fx);
            }
            p.update(
                &input,
                &mut PlayerCtx {
                    zone: &zone,
                    fx: &mut fx,
                    hits: &mut hits,
                },
            );
            writeln!(
                out,
                "{}",
                json!({"actor":"hero","direction":d,"tick":t,
                "state":format!("{:?}",p.state),"simulation":receipt(&p.body,&fx,&hits,&[]),
                "respawn":p.wants_respawn})
            )
            .unwrap();
        }
    }
    let kinds = [
        EnemyKind::Sproutling,
        EnemyKind::ThornImp,
        EnemyKind::CrystalBeetle,
        EnemyKind::Wisp,
        EnemyKind::EmberHound,
        EnemyKind::SpittingTotem,
    ];
    for kind in kinds {
        for skin in 0..3 {
            for (d, mv) in directions.into_iter().enumerate() {
                let x = (1000..1100)
                    .map(|x| x as f32)
                    .find(|&x| appearance(x, 1024.) == skin)
                    .unwrap();
                let mut enemy = Enemy::spawn(kind, x, 1024.);
                enemy.awake = true;
                let mut fx = Fx::new(739);
                let mut shots = Vec::new();
                for t in 0..480u32 {
                    fx.sounds.clear();
                    fx.tick();
                    let mut hits = Vec::new();
                    let distance = if t < 120 {
                        72.
                    } else if t < 280 {
                        28.
                    } else {
                        58.
                    };
                    let aim = glintblade::util::norm(mv.0, mv.1);
                    let target = (
                        enemy.body.x + aim.0 * distance,
                        enemy.body.y + aim.1 * distance,
                    );
                    let mut ctx = EnemyCtx {
                        player: target,
                        player_alive: true,
                        zone: &zone,
                        fx: &mut fx,
                        hits: &mut hits,
                        projectiles: &mut shots,
                        t: t as u64,
                    };
                    if t == 320 || t == 420 {
                        enemy.body.iframes = 0;
                        let hit = Hit::new(
                            Shape::Circle {
                                x: enemy.body.x,
                                y: enemy.body.y,
                                r: 5.,
                            },
                            Team::Player,
                            if t == 420 { 999 } else { 1 },
                            8000 + t,
                        )
                        .dir(aim.0, aim.1)
                        .heavy(t == 320);
                        enemy.on_hit(&hit, &mut ctx);
                    }
                    enemy.update(&mut ctx);
                    for shot in &mut shots {
                        if shot.alive {
                            if let Some(hit) = shot.update(&zone, &mut fx, target) {
                                hits.push(hit);
                            }
                        }
                    }
                    shots.retain(|p| p.alive);
                    writeln!(
                        out,
                        "{}",
                        json!({"actor":format!("{kind:?}"),"skin":skin,"direction":d,"tick":t,
                "simulation":receipt(&enemy.body,&fx,&hits,&shots)})
                    )
                    .unwrap();
                }
            }
        }
    }
}
