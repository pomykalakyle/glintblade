//! Deterministic native actor coverage in the production sprite/world renderer.
//! `glintblade eightway-audit --out DIR --mode atlas|runtime|player|all`
//! This developer command never runs in normal play and changes no game data.

use crate::combat::{Hit, Shape, Team};
use crate::entities::enemies::Brain;
use crate::entities::player::PlayerCtx;
use crate::entities::{Enemy, EnemyCtx, EnemyKind, Player};
use crate::fx::Fx;
use crate::gfx::{Cell, Dir, DrawOpts, Frame, H, W, post, rgb, sheet};
use crate::input::{Input, Raw};
use crate::world::{Region, RegionCtx, Zone, ZoneId};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const FAMILIES: [&str; 7] = [
    "hero",
    "sproutling",
    "thorn_imp",
    "crystal_beetle",
    "wisp",
    "ember_hound",
    "spitting_totem",
];

fn option<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn bases(family: &str) -> &'static [&'static str] {
    match family {
        "hero" => &[
            "idle",
            "run",
            "attack1",
            "attack2",
            "attack3",
            "dash",
            "charge",
            "charge_walk",
            "spin",
            "hurt",
            "collapse",
            "fall",
            "land",
        ],
        "crystal_beetle" => &["idle", "move", "windup", "attack", "hurt", "dazed"],
        "thorn_imp" => &[
            "idle",
            "move",
            "windup",
            "attack",
            "hurt",
            "windup_side",
            "attack_side",
        ],
        "wisp" | "spitting_totem" => &["idle", "windup", "attack", "hurt"],
        _ => &["idle", "move", "windup", "attack", "hurt"],
    }
}

fn ground(family: &str) -> (f32, i32, f32) {
    match family {
        "hero" => (4.0, 4, 0.0),
        "sproutling" => (4.0, 3, 0.0),
        "thorn_imp" => (4.5, 3, 0.0),
        "crystal_beetle" => (8.0, 8, 0.0),
        "wisp" => (4.5, 2, 7.0),
        "ember_hound" => (6.0, 4, 0.0),
        _ => (6.0, 5, 0.0),
    }
}

fn write_frame(path: &Path, f: &Frame) {
    let bytes = post::composite_scaled(f, 1);
    crate::assets::write_png(path, W as u32, H as u32, &bytes).expect("write audit frame");
}

fn describe_frame(f: &Frame) -> Value {
    let actors = f.surfaces.iter().filter(|p| p.kind == 3).count();
    let actor_owners = f
        .surfaces
        .iter()
        .filter(|p| p.kind == 3)
        .map(|p| p.owner)
        .collect::<std::collections::HashSet<_>>();
    let valid = f
        .surfaces
        .iter()
        .filter(|p| {
            p.kind == 3
                && p.normal.iter().all(|v| v.is_finite())
                && p.normal.iter().map(|v| v * v).sum::<f32>() > 0.5
        })
        .count();
    json!({"actor_surface_pixels":actors,"valid_actor_normal_pixels":valid,
        "surface_casters":f.surface_casters.len(),"lights":f.point_lights.len(),
        "point_lights":f.point_lights.iter().map(|p|json!({"position":[p.x,p.y,p.height],"radius":p.radius,"intensity":p.intensity,"color":p.color.to_u8()})).collect::<Vec<_>>(),
        "casters":f.surface_casters.iter().map(|c|json!({"owner":c.owner,"actor":actor_owners.contains(&c.owner),"position":[c.x,c.y],"base":c.base,
            "anchor":[c.anchor.0,c.anchor.1],"flip":c.flip,"min_height":c.min_height,"opacity":c.opacity,
            "support_columns":c.foot.iter().filter(|v|v[0].is_finite()&&v[1]>=v[0]).count()})).collect::<Vec<_>>()})
}

fn warm(zone: &mut Zone, region: &mut dyn Region, fx: &mut Fx, pos: (f32, f32), t: u64) {
    let movers = [(pos.0, pos.1, 0.0)];
    let ctx = RegionCtx {
        t,
        player: pos,
        movers: &movers,
        cam: (
            (pos.0 - W as f32 / 2.0) as i32,
            (pos.1 - H as f32 / 2.0) as i32,
        ),
    };
    region.update(zone, &ctx, fx);
    fx.particles.update(zone);
    fx.tick();
}

fn expected_name(base: &str, dir: Dir, actual: &str, flip: bool) -> bool {
    if actual == format!("{base}_{}", dir.suffix()) && !flip {
        return true;
    }
    let source = match dir {
        Dir::Left => Some(Dir::Right),
        Dir::UpLeft => Some(Dir::UpRight),
        Dir::DownLeft => Some(Dir::DownRight),
        _ => None,
    };
    source.is_some_and(|d| flip && actual == format!("{base}_{}", d.suffix()))
}

fn diagnostic_zone(source: &Zone) -> Zone {
    let mut stage = Zone::new(source.id, 24, 14);
    stage.ambient = source.ambient;
    stage.bg = source.bg;
    stage.grade = source.grade;
    stage.lighting_settings = source.lighting_settings;
    let (x, y) = source.player_start;
    let floor = source.ground[(y as i32).clamp(0, source.ph - 1) as usize * source.pw as usize
        + (x as i32).clamp(0, source.pw - 1) as usize];
    stage.ground.fill(floor);
    stage
}

fn atlas(
    out: &Path,
    filter: Option<&str>,
    region_filter: Option<&str>,
    diagnostic: bool,
) -> Vec<Value> {
    let mut records = Vec::new();
    for id in [
        ZoneId::Bloomwild,
        ZoneId::Glasswater,
        ZoneId::Cinder,
        ZoneId::Summit,
    ] {
        let region_name = format!("{id:?}").to_lowercase();
        if region_filter.is_some_and(|r| r != "all" && r != region_name) {
            continue;
        }
        let (mut zone, mut region) = crate::regions::build(id);
        if diagnostic {
            // A separate flat diagnostic floor puts every column on real
            // receiving material. The unmodified actual-region grids remain
            // available separately; this stage never changes a game region.
            let mut stage = diagnostic_zone(&zone);
            let source = match id {
                ZoneId::Glasswater => {
                    Some(("glasswater_crystals_native", "body", rgb(0xa195b8), 36.0))
                }
                ZoneId::Cinder => Some(("cinder_brazier", "brazier_lit", rgb(0xefb68a), 20.0)),
                ZoneId::Summit => Some(("summit_brazier", "brazier", rgb(0xded0ad), 20.0)),
                _ => None,
            };
            if let Some((name, anim, color, height)) = source {
                for x in [-16.0, 400.0] {
                    let mut p = crate::world::Prop::new(name, anim, x, 128.0);
                    // Fixed diagnostic reach makes the signed normal response
                    // legible across the384px board; ambient/fill/grade and
                    // production point-light/cast/contact math are unchanged.
                    p.light = Some((color, 260.0, 0.9, 0.0, -height));
                    stage.props.push(p);
                }
            }
            zone = stage;
            region = Box::new(crate::world::NoRegion);
        }
        let pos = zone.player_start;
        let mut fx = Fx::new(739);
        for t in 0..90 {
            warm(&mut zone, region.as_mut(), &mut fx, pos, t);
            fx.lights.clear();
        }
        let cam = (
            (pos.0 - W as f32 / 2.0) as i32,
            (pos.1 - H as f32 / 2.0) as i32,
        );
        for family in FAMILIES {
            for variant in 0..if family == "hero" { 1 } else { 3 } {
                let name = if variant == 0 {
                    family.to_owned()
                } else {
                    format!("{family}_v{variant}")
                };
                if filter.is_some_and(|s| s != name) {
                    continue;
                }
                let sh = sheet(&name).expect("audit actor sheet");
                let directory = out
                    .join(if diagnostic { "matrix" } else { "atlas" })
                    .join(&region_name)
                    .join(&name);
                std::fs::create_dir_all(&directory).unwrap();
                for base in bases(family) {
                    // A strict lookup rejects silent cardinal or unsuffixed fallbacks.
                    let poses = Dir::ALL.map(|dir| {
                        if base.ends_with("_side")
                            && matches!(dir, Dir::Up | Dir::Right | Dir::Down | Dir::Left)
                        {
                            return None;
                        }
                        let (a, flip) = sh
                            .dir_anim(base, dir)
                            .unwrap_or_else(|| panic!("missing {name}/{base}_{}", dir.suffix()));
                        assert!(
                            expected_name(base, dir, &a.name, flip),
                            "{name}/{base}_{} silently resolved {}",
                            dir.suffix(),
                            a.name
                        );
                        Some((dir, a, flip))
                    });
                    let frames = poses
                        .iter()
                        .flatten()
                        .map(|(_, a, _)| a.frames)
                        .max()
                        .unwrap();
                    let pitch_y = sh.cell_h.max(24) + 8;
                    let rows = (H as i32 / pitch_y).max(1) as usize;
                    for page in 0..frames.div_ceil(rows) {
                        let first = page * rows;
                        let count = (frames - first).min(rows);
                        let mut f = Frame::new();
                        f.cam_x = cam.0;
                        f.cam_y = cam.1;
                        let ext = (0..count)
                            .flat_map(|row| {
                                (0..8).map(move |d| {
                                    (
                                        (cam.1 + pitch_y * (row as i32 + 1) - 5) as f32,
                                        (row * 8 + d) as u32,
                                    )
                                })
                            })
                            .collect::<Vec<_>>();
                        let (half, depth, elevation) = ground(family);
                        crate::view::render_world(
                            &zone,
                            region.as_ref(),
                            &fx,
                            &mut f,
                            120,
                            &ext,
                            &mut |f, index| {
                                let row = index as usize / 8;
                                let column = index as usize % 8;
                                let Some((_, a, flip)) = poses[column] else {
                                    return;
                                };
                                if first + row >= a.frames {
                                    return;
                                }
                                let cell = sh.cell(a, first + row);
                                let x = (cam.0 + 24 + column as i32 * 48) as f32;
                                let y = (cam.1 + pitch_y * (row as i32 + 1) - 5) as f32;
                                f.shadow(f.sx(x), f.sy(y), half as i32, 2, 0.35);
                                f.sprite_w(
                                    cell,
                                    sh.anchor_of(a),
                                    x,
                                    y - elevation,
                                    &DrawOpts {
                                        actor: true,
                                        foot_half_w: half,
                                        foot_depth: depth,
                                        elevation,
                                        ground_y: Some(y.floor()),
                                        flip,
                                        foot_full_width: matches!(
                                            family,
                                            "crystal_beetle" | "ember_hound" | "spitting_totem"
                                        ),
                                        native_support: if family == "hero" {
                                            crate::entities::player::audit_support(a, first + row)
                                        } else {
                                            None
                                        },
                                        ..Default::default()
                                    },
                                );
                            },
                            None,
                        );
                        post::finish(&mut f, &zone.grade);
                        let file = directory.join(format!("{base}-frames-{first:02}.png"));
                        write_frame(&file, &f);
                        records.push(json!({"kind":"production-renderer-native-pose-grid","fixture":if diagnostic {"flat-native-floor-regional-profile"}else{"unmodified-region"},"region":region_name,"atlas":name,"base":base,
                            "path":file,"first_frame":first,"frame_rows":count,"direction_columns":Dir::ALL.iter().map(|d|d.suffix()).collect::<Vec<_>>(),
                            "lookups":poses.iter().flatten().map(|(d,a,flip)|json!({"direction":d.suffix(),"animation":a.name,"flip":flip,"frames":a.frames,"durations":a.durations,"anchor":sh.anchor_of(a)})).collect::<Vec<_>>(),
                            "render":describe_frame(&f)}));
                    }
                }
            }
        }
    }
    records
}

fn spawn_for_skin(kind: EnemyKind, skin: u8, center: (f32, f32)) -> Enemy {
    // Select a real production spawn near a clear start, using its unchanged
    // appearance hash. Keeping the original spawn also preserves totem.home.
    let appearance = |x: i32, y: i32| match (x, y) {
        (392, 648) | (1144, 472) | (661, 535) => 1,
        (544, 243) => 2,
        _ => (crate::util::hash2(x, y, 941) * 3.0) as u8 % 3,
    };
    let pos = (0..24)
        .flat_map(|r| {
            (-r..=r).map(move |x| (center.0.floor() as i32 + x, center.1.floor() as i32 + r))
        })
        .find(|&(x, y)| appearance(x, y) == skin)
        .unwrap();
    let pos = (pos.0 as f32, pos.1 as f32);
    Enemy::spawn(kind, pos.0, pos.1)
}

fn enemy_state(e: &Enemy) -> String {
    match &e.brain {
        Brain::Sproutling(s) => {
            let (name, t, _) = crate::entities::enemies::sproutling::audit_state(s);
            format!("{name}:{t}")
        }
        Brain::ThornImp(s) => {
            let (name, t, _) = crate::entities::enemies::thorn_imp::audit_state(s);
            format!("{name}:{t}")
        }
        Brain::CrystalBeetle(s) => format!("{:?}:{}", s.mode, s.mode_t),
        Brain::Wisp(s) => format!("{:?}:{}", s.mode, s.mode_t),
        Brain::EmberHound(s) => format!("{:?}:{}", s.phase, s.pt),
        Brain::SpittingTotem(s) => format!("{:?}:{}", s.phase, s.pt),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn enemy_pose(e: &Enemy, t: u64) -> Option<(&'static Cell, (i32, i32), bool)> {
    use crate::entities::enemies::*;
    Some(match &e.brain {
        Brain::Sproutling(s) => sproutling::audit_pose(&e.body, s, t),
        Brain::ThornImp(s) => thorn_imp::audit_pose(&e.body, s, t),
        Brain::CrystalBeetle(s) => crystal_beetle::audit_pose(&e.body, s, t),
        Brain::Wisp(s) => wisp::audit_pose(&e.body, s, t),
        Brain::EmberHound(s) => ember_hound::audit_pose(&e.body, s, t),
        Brain::SpittingTotem(s) => spitting_totem::audit_pose(&e.body, s, t),
    })
}

#[cfg(target_arch = "wasm32")]
fn enemy_pose(_e: &Enemy, _t: u64) -> Option<(&'static Cell, (i32, i32), bool)> {
    None
}

fn pose_receipt(atlas: &str, pose: Option<(&Cell, (i32, i32), bool)>) -> Value {
    let Some((cell, anchor, flip)) = pose else {
        return Value::Null;
    };
    let sh = sheet(atlas).unwrap();
    let mut matches = Vec::new();
    let mut best: Option<(usize, usize, usize)> = None;
    for a in sh.anims.values() {
        for index in 0..a.frames {
            let source = sh.cell(a, index);
            if std::ptr::eq(source, cell) {
                matches.push(json!({"animation":a.name,"frame":index,"row":a.row,"match":"immutable-cell-identity"}));
            } else if source.w == cell.w && source.h == cell.h {
                let changes = source
                    .px
                    .iter()
                    .zip(&cell.px)
                    .filter(|(a, b)| a.a != b.a || a.a && (a.c != b.c || a.emit != b.emit))
                    .count();
                if best.is_none_or(|(old, _, _)| changes < old) {
                    best = Some((changes, a.row, index));
                }
                if changes == 0 {
                    matches.push(json!({"animation":a.name,"frame":index,"row":a.row,"match":"identical-native-pixels"}));
                }
            }
        }
    }
    json!({"anchor":anchor,"flip":flip,"cell":[cell.w,cell.h],"exact_matches":matches,
        "derived_contour_nearest":best.map(|(changes,row,index)|json!({"row":row,"frame":index,"changed_pixels":changes})),
        "opaque_pixels":cell.px.iter().filter(|p|p.a).count(),"emissive_pixels":cell.px.iter().filter(|p|p.a&&p.emit).count()})
}

fn runtime(out: &Path, filter: Option<&str>, every: u32) -> Vec<Value> {
    let mut records = Vec::new();
    for family in FAMILIES.into_iter().skip(1) {
        let kind = EnemyKind::parse(family).unwrap();
        let id = match family {
            "sproutling" | "thorn_imp" => ZoneId::Bloomwild,
            "wisp" | "crystal_beetle" => ZoneId::Glasswater,
            _ => ZoneId::Cinder,
        };
        for skin in 0..3 {
            let name = if skin == 0 {
                family.to_owned()
            } else {
                format!("{family}_v{skin}")
            };
            if filter.is_some_and(|s| s != name) {
                continue;
            }
            let (mut zone, mut region) = crate::regions::build(id);
            let pos = zone.player_start;
            let mut e = spawn_for_skin(kind, skin, pos);
            e.awake = true;
            let mut fx = Fx::new(739);
            let mut projectiles = Vec::new();
            let mut hits = Vec::new();
            let directory = out.join("runtime").join(&name);
            std::fs::create_dir_all(&directory).unwrap();
            for t in 1..=640u64 {
                fx.lights.clear();
                fx.sounds.clear();
                hits.clear();
                let dir = Dir::ALL[((t - 1) / 80) as usize % 8];
                let v = dir.vec();
                let range = match family {
                    "sproutling" => 30.0,
                    "thorn_imp" => 110.0,
                    "wisp" => 65.0,
                    "crystal_beetle" => 85.0,
                    "ember_hound" => 65.0,
                    _ => 80.0,
                };
                let target = (e.body.x + v.0 * range, e.body.y + v.1 * range);
                let mut ctx = EnemyCtx {
                    player: target,
                    player_alive: true,
                    zone: &zone,
                    fx: &mut fx,
                    hits: &mut hits,
                    projectiles: &mut projectiles,
                    t,
                };
                if t == 574 {
                    e.body.iframes = 0;
                    e.on_hit(
                        &Hit::new(
                            Shape::Circle {
                                x: e.body.x,
                                y: e.body.y,
                                r: 20.0,
                            },
                            Team::Player,
                            1,
                            90_001,
                        )
                        .dir(-v.0, -v.1),
                        &mut ctx,
                    );
                }
                if t == 620 {
                    e.body.iframes = 0;
                    e.on_hit(
                        &Hit::new(
                            Shape::Circle {
                                x: e.body.x,
                                y: e.body.y,
                                r: 20.0,
                            },
                            Team::Player,
                            99,
                            90_002,
                        )
                        .dir(-v.0, -v.1),
                        &mut ctx,
                    );
                }
                e.update(&mut ctx);
                for p in &mut projectiles {
                    p.update(&zone, &mut fx, target);
                }
                projectiles.retain(|p| p.alive);
                let center = (e.body.x, e.body.y);
                warm(&mut zone, region.as_mut(), &mut fx, center, t);
                if t % every as u64 == 0 {
                    let mut f = Frame::new();
                    f.cam_x = (center.0 - W as f32 / 2.0) as i32;
                    f.cam_y = (center.1 - H as f32 / 2.0) as i32;
                    let ext = std::iter::once((e.body.y, 0))
                        .chain(
                            projectiles
                                .iter()
                                .enumerate()
                                .map(|(i, p)| (p.y, (i + 1) as u32)),
                        )
                        .collect::<Vec<_>>();
                    crate::view::render_world(
                        &zone,
                        region.as_ref(),
                        &fx,
                        &mut f,
                        t,
                        &ext,
                        &mut |f, i| {
                            if i == 0 {
                                e.draw(f)
                            } else {
                                projectiles[i as usize - 1].draw(f)
                            }
                        },
                        None,
                    );
                    post::finish(&mut f, &zone.grade);
                    let file = directory.join(format!("frame-{t:04}.png"));
                    write_frame(&file, &f);
                    records.push(json!({"kind":"production-enemy-update-and-draw","atlas":name,"tick":t,"path":file,"target_sector":dir.suffix(),
                        "state":enemy_state(&e),"body":{"position":[e.body.x,e.body.y,e.body.z],"face":[e.body.face.0,e.body.face.1],"hp":e.body.hp,"alive":e.body.alive},
                        "pose":pose_receipt(&name,enemy_pose(&e,t)),"render":describe_frame(&f)}));
                }
            }
        }
    }
    records
}

fn player(out: &Path, region_filter: Option<&str>, every: u32, transitions: bool) -> Vec<Value> {
    let mut records = Vec::new();
    for id in [
        ZoneId::Bloomwild,
        ZoneId::Glasswater,
        ZoneId::Cinder,
        ZoneId::Summit,
    ] {
        let region_name = format!("{id:?}").to_lowercase();
        if region_filter.is_some_and(|r| r != "all" && r != region_name) {
            continue;
        }
        let (mut zone, mut region) = crate::regions::build(id);
        let pos = zone.player_start;
        let mut fx = Fx::new(739);
        let mut hits = Vec::new();
        let directory = out
            .join(if transitions {
                "player-transitions"
            } else {
                "player"
            })
            .join(&region_name);
        std::fs::create_dir_all(&directory).unwrap();
        // All8directions: walk -> stop -> turn -> attack/combo -> charge/spin -> dash.
        // The authored input sequence runs through production Player::update; only
        // the starting position is reset between independent direction cases.
        for direction in Dir::ALL {
            let mut p = Player::new(pos.0, pos.1);
            let mut input = Input::default();
            let d = direction.vec();
            let ticks = if transitions { 440 } else { 220 };
            for local in 0..ticks {
                let t = direction_index(direction) as u64 * ticks as u64 + local as u64;
                fx.lights.clear();
                fx.sounds.clear();
                hits.clear();
                let mut raw = Raw::default();
                raw.mv = if transitions {
                    match local {
                        0..=15 | 40..=55 | 120..=140 | 190..=192 => d,
                        16..=31 => (-d.0, -d.1),
                        _ => (0.0, 0.0),
                    }
                } else {
                    match local {
                        0..=15 => d,
                        22..=25 => Dir::ALL[(direction_index(direction) + 1) % 8].vec(),
                        26..=27 => d,
                        170..=172 => d,
                        _ => (0.0, 0.0),
                    }
                };
                raw.attack = if transitions {
                    matches!(local, 60 | 80..=150)
                } else {
                    matches!(local, 30 | 43 | 58 | 80..=138)
                };
                raw.dash = local == if transitions { 190 } else { 170 };
                input.step(raw);
                if local == if transitions { 210 } else { 190 } || transitions && local == 230 {
                    p.body.iframes = 0;
                    p.on_hit(
                        &Hit::new(
                            Shape::Circle {
                                x: p.body.x,
                                y: p.body.y,
                                r: 20.0,
                            },
                            Team::Enemy,
                            if transitions && local == 230 { 99 } else { 1 },
                            100_000 + t as u32,
                        )
                        .dir(-d.0, -d.1),
                        &mut fx,
                    );
                }
                p.update(
                    &input,
                    &mut PlayerCtx {
                        zone: &zone,
                        fx: &mut fx,
                        hits: &mut hits,
                    },
                );
                // Match Play's checkpoint handoff without changing the Player
                // death clock or its intentionally south-facing respawn.
                let checkpoint_respawn = transitions && p.wants_respawn;
                if checkpoint_respawn {
                    p.respawn(pos.0, pos.1);
                }
                warm(&mut zone, region.as_mut(), &mut fx, (p.body.x, p.body.y), t);
                if local % every == 0 {
                    let mut f = Frame::new();
                    let center = if transitions {
                        (p.body.x, p.body.y)
                    } else {
                        pos
                    };
                    f.cam_x = (center.0 - W as f32 / 2.0) as i32;
                    f.cam_y = (center.1 - H as f32 / 2.0) as i32;
                    crate::view::render_world_focused(
                        &zone,
                        region.as_ref(),
                        &fx,
                        &mut f,
                        t,
                        &[(p.body.y, 0)],
                        &mut |f, _| p.draw(f),
                        Some((p.body.x, p.body.y)),
                        p.foreground_focus(),
                    );
                    post::finish(&mut f, &zone.grade);
                    let file = directory.join(format!("{}-{local:03}.png", direction.suffix()));
                    write_frame(&file, &f);
                    records.push(json!({"kind":"production-player-update-and-draw","region":region_name,"direction_case":direction.suffix(),"local_tick":local,"path":file,
                        "sequence":if transitions {"reversal-chargewalk-death-respawn"}else{"walk-stop-turn-combo-charge-spin-dash-hurt"},"checkpoint_respawn":checkpoint_respawn,
                        "state":format!("{:?}",p.state),"visual_direction":p.dir.suffix(),"precise_face":[p.body.face.0,p.body.face.1],"position":[p.body.x,p.body.y,p.body.z],
                        "pose":pose_receipt("hero",p.current_cell()),"render":describe_frame(&f)}));
                }
            }
        }
    }
    records
}

fn direction_index(d: Dir) -> usize {
    Dir::ALL.iter().position(|v| *v == d).unwrap()
}

fn sword_sweep(
    out: &Path,
    filter: Option<&str>,
    region_filter: Option<&str>,
    every: u32,
) -> Vec<Value> {
    let mut records = Vec::new();
    for id in [
        ZoneId::Bloomwild,
        ZoneId::Glasswater,
        ZoneId::Cinder,
        ZoneId::Summit,
    ] {
        let region_name = format!("{id:?}").to_lowercase();
        if region_filter.is_some_and(|r| r != "all" && r != region_name) {
            continue;
        }
        let (reference, _) = crate::regions::build(id);
        for family in FAMILIES.into_iter().skip(1) {
            for variant in 0..3 {
                let name = if variant == 0 {
                    family.to_owned()
                } else {
                    format!("{family}_v{variant}")
                };
                if filter.is_some_and(|s| s != name) {
                    continue;
                }
                let mut zone = diagnostic_zone(&reference);
                let mut region = crate::world::NoRegion;
                let center = (192.0, 128.0);
                let mut hero = Player::new(center.0 + 32.0, center.1);
                let mut input = Input::default();
                let mut fx = Fx::new(739);
                let mut hits = Vec::new();
                let sh = sheet(&name).unwrap();
                let directory = out.join("sword-sweep").join(&region_name).join(&name);
                std::fs::create_dir_all(&directory).unwrap();
                for t in 0..96u32 {
                    fx.lights.clear();
                    fx.sounds.clear();
                    hits.clear();
                    let angle = t as f32 / 96.0 * std::f32::consts::TAU;
                    let goal = (center.0 + 32.0 * angle.cos(), center.1 + 24.0 * angle.sin());
                    input.step(Raw {
                        mv: crate::util::norm(goal.0 - hero.body.x, goal.1 - hero.body.y),
                        attack: matches!(t, 18 | 40 | 66),
                        ..Default::default()
                    });
                    hero.update(
                        &input,
                        &mut PlayerCtx {
                            zone: &zone,
                            fx: &mut fx,
                            hits: &mut hits,
                        },
                    );
                    warm(
                        &mut zone,
                        &mut region,
                        &mut fx,
                        (hero.body.x, hero.body.y),
                        t as u64,
                    );
                    if t % every != 0 {
                        continue;
                    }
                    let direction = Dir::ALL[((t % 32) / 4) as usize];
                    let requested = match t / 32 {
                        0 => "idle",
                        1 => "move",
                        _ => "attack",
                    };
                    // Flame drift and a planted furnace have no locomotion row.
                    let base = if requested == "move" && matches!(family, "wisp" | "spitting_totem")
                    {
                        "idle"
                    } else {
                        requested
                    };
                    let (anim, flip) = sh.dir_anim(base, direction).unwrap();
                    assert!(expected_name(base, direction, &anim.name, flip));
                    let index = anim.frame_at(t, true);
                    let cell = sh.cell(anim, index);
                    let anchor = sh.anchor_of(anim);
                    let (half, depth, elevation) = ground(family);
                    let mut f = Frame::new();
                    f.cam_y = 4;
                    let mut receptor_owner = 0;
                    crate::view::render_world_focused(
                        &zone,
                        &region,
                        &fx,
                        &mut f,
                        t as u64,
                        &[(center.1, 0), (hero.body.y, 1)],
                        &mut |f, key| {
                            if key == 1 {
                                hero.draw(f);
                                return;
                            }
                            f.shadow(f.sx(center.0), f.sy(center.1), half as i32, 2, 0.35);
                            let before = f.surface_casters.len();
                            f.sprite_w(
                                cell,
                                anchor,
                                center.0,
                                center.1 - elevation,
                                &DrawOpts {
                                    actor: true,
                                    flip,
                                    elevation,
                                    ground_y: Some(center.1),
                                    foot_half_w: half,
                                    foot_depth: depth,
                                    foot_full_width: matches!(
                                        family,
                                        "crystal_beetle" | "ember_hound" | "spitting_totem"
                                    ),
                                    ..Default::default()
                                },
                            );
                            if f.surface_casters.len() > before {
                                receptor_owner = f.surface_casters[before].owner;
                            }
                        },
                        Some((hero.body.x, hero.body.y)),
                        hero.foreground_focus(),
                    );
                    post::finish(&mut f, &zone.grade);
                    let path = directory.join(format!("frame-{t:03}.png"));
                    write_frame(&path, &f);
                    let receiver_pixels = f
                        .surfaces
                        .iter()
                        .filter(|p| p.owner == receptor_owner && p.kind == 3)
                        .count();
                    records.push(json!({"kind":"moving-production-sword-light","fixture":"flat-native-floor-regional-profile","atlas":name,"region":region_name,"tick":t,"path":path,
                        "state":format!("{base}_{direction:?}"),"animation":anim.name,"animation_frame":index,"direction":direction.suffix(),"flip":flip,
                        "receptor_owner":receptor_owner,"receptor_normal_pixels":receiver_pixels,"receptor_pose":pose_receipt(&name,Some((cell,anchor,flip))),
                        "source_player":{"position":[hero.body.x,hero.body.y,hero.body.z],"state":format!("{:?}",hero.state),"direction":hero.dir.suffix(),"precise_face":[hero.body.face.0,hero.body.face.1],"pose":pose_receipt("hero",hero.current_cell())},
                        "render":describe_frame(&f)}));
                }
            }
        }
    }
    records
}

pub fn run(args: &[String]) {
    let directory = PathBuf::from(option(args, "--out").unwrap_or("out/eightway-audit"));
    assert!(
        !directory.join("render-manifest.json").exists(),
        "audit output is immutable; use a fresh --out tag"
    );
    std::fs::create_dir_all(&directory).expect("create audit directory");
    let mode = option(args, "--mode").unwrap_or("all");
    assert!(
        [
            "atlas",
            "matrix",
            "runtime",
            "player",
            "player-transitions",
            "sword-sweep",
            "all"
        ]
        .contains(&mode),
        "invalid audit mode"
    );
    let filter = option(args, "--atlas");
    let region = option(args, "--region");
    let every = option(args, "--every")
        .map(|v| v.parse::<u32>().expect("invalid --every"))
        .unwrap_or(if mode == "sword-sweep" { 2 } else { 1 })
        .max(1);
    let mut records = Vec::new();
    if mode == "atlas" || mode == "all" {
        records.extend(atlas(&directory, filter, region, false));
    }
    if mode == "matrix" {
        records.extend(atlas(&directory, filter, region, true));
    }
    if mode == "runtime" || mode == "all" {
        records.extend(runtime(&directory, filter, every));
    }
    if mode == "player" || mode == "all" {
        records.extend(player(&directory, region, every, false));
    }
    if mode == "player-transitions" {
        records.extend(player(&directory, region, every, true));
    }
    if mode == "sword-sweep" {
        records.extend(sword_sweep(&directory, filter, region, every));
    }
    let report = json!({"schema":1,"native_size":[W,H],"mode":mode,"captures":records,
        "source_assets":std::env::var("GLINT_ASSETS").ok(),"sprite_overrides":std::env::var("GLINT_SPRITES").ok(),
        "conditions":["Production world/sprite/normal/light/shadow renderer, region grade and postprocessing.",
            "Atlas pages stage native frames in8direction columns; no animation or bitmap rotation.",
            "Runtime enemies use production updates, pose choices, projectiles, hit/death effects and real draw paths.",
            "Player sequences use production input normalization/update/draw including scarf, blade lights, ghosts and foreground reveal.",
            "Normal maps are embedded; build with matching GLINT_ASSETS. GLINT_SPRITES alone cannot validate appended native normals."]});
    std::fs::write(
        directory.join("render-manifest.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "eightway-audit wrote {} captures to {}",
        report["captures"].as_array().unwrap().len(),
        directory.display()
    );
}
