//! Developer tools that run without a window.
//!
//! `glintblade zoneview <zone> [options]` renders a zone (with its region effects, props,
//! lights and grading, and Glint standing there for scale) to PNG files:
//!   --at X,Y        camera center / Glint position in world pixels (default: player start)
//!   --walk VX,VY    move Glint by this many px per tick (grass, water and dust react)
//!   --warm N        ticks to simulate before the first capture (default 90)
//!   --ticks N       ticks to simulate after warm-up (default 0 = one capture)
//!   --every K       capture every K ticks during --ticks (default: only the first)
//!   --scale S       upscale factor with bloom composite (default 4)
//!   --out DIR       output folder (default out/zoneview)
//!   --no-hero       don't draw Glint
//!   --overview      also write the whole zone stitched at 1x (layout check)

use crate::assets::write_png;
use crate::entities::Player;
use crate::fx::Fx;
use crate::gfx::{Frame, H, W, post};
use crate::regions;
use crate::view::{render_world, render_world_focused};
use crate::world::{RegionCtx, ZoneId};
use std::path::PathBuf;

mod tour;

fn arg<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

fn pair(s: &str) -> Option<(f32, f32)> {
    let mut it = s.split(',');
    Some((it.next()?.trim().parse().ok()?, it.next()?.trim().parse().ok()?))
}

pub fn zoneview(args: &[String]) {
    let Some(zone_name) = args.first() else {
        eprintln!("usage: glintblade zoneview <bloomwild|glasswater|cinder|summit> [options]");
        return;
    };
    let Some(id) = ZoneId::parse(zone_name) else {
        eprintln!("unknown zone {zone_name}");
        return;
    };
    let (mut zone, mut region) = regions::build(id);
    let mut pos = arg(args, "--at").and_then(pair).unwrap_or(zone.player_start);
    let walk = arg(args, "--walk").and_then(pair).unwrap_or((0.0, 0.0));
    let warm: u64 = arg(args, "--warm").and_then(|s| s.parse().ok()).unwrap_or(90);
    let ticks: u64 = arg(args, "--ticks").and_then(|s| s.parse().ok()).unwrap_or(0);
    let every: u64 = arg(args, "--every").and_then(|s| s.parse().ok()).unwrap_or(0);
    let scale: usize = arg(args, "--scale").and_then(|s| s.parse().ok()).unwrap_or(4);
    let out = PathBuf::from(arg(args, "--out").unwrap_or("out/zoneview"));
    let hero = !args.iter().any(|a| a == "--no-hero");
    std::fs::create_dir_all(&out).ok();

    let mut fx = Fx::new(1);
    let mut frame = Frame::new();
    let mut player = Player::new(pos.0, pos.1);
    let total = warm + ticks;
    let mut written = 0;
    for t in 0..=total {
        // simulate
        if t >= warm {
            pos = (pos.0 + walk.0, pos.1 + walk.1);
        }
        player.body.x = pos.0;
        player.body.y = pos.1;
        let speed = (walk.0 * walk.0 + walk.1 * walk.1).sqrt();
        let movers = [(pos.0, pos.1, speed)];
        let cam = ((pos.0 as i32 - W as i32 / 2).clamp(0, (zone.pw - W as i32).max(0)), (pos.1 as i32 - H as i32 / 2).clamp(0, (zone.ph - H as i32).max(0)));
        let ctx = RegionCtx { t, player: pos, movers: &movers, cam };
        region.update(&mut zone, &ctx, &mut fx);
        fx.particles.update(&zone);
        for p in zone.props.iter_mut() {
            p.tick();
        }
        fx.tick();

        let capture = t == warm || (every > 0 && t > warm && (t - warm) % every == 0);
        if capture {
            frame.cam_x = cam.0;
            frame.cam_y = cam.1;
            let ext = if hero { vec![(player.body.y, 0u32)] } else { vec![] };
            let pl = &player;
            let focus = if hero { player.current_cell().map(|(cell, anchor, flip)| crate::gfx::sprite::SpriteFocus {
                cell, anchor, flip, x: player.body.x, y: player.body.y - player.body.z, ground_y: player.body.y,
            }) } else { None };
            render_world_focused(&zone, region.as_ref(), &fx, &mut frame, t, &ext, &mut |f, _| pl.draw(f), Some(pos), focus);
            post::finish(&mut frame, &zone.grade);
            let img = post::composite_scaled(&frame, scale);
            let path = out.join(format!("{}_t{:05}.png", zone_name, t));
            write_png(&path, (W * scale) as u32, (H * scale) as u32, &img).expect("write png");
            written += 1;
        }
        fx.lights.clear();
    }
    println!("wrote {written} frame(s) to {}", out.display());

    if args.iter().any(|a| a == "--overview") {
        let (pw, ph) = (zone.pw as usize, zone.ph as usize);
        let mut big = vec![0u8; pw * ph * 4];
        let mut cy = 0;
        while cy < ph {
            let mut cx = 0;
            while cx < pw {
                frame.cam_x = cx as i32;
                frame.cam_y = cy as i32;
                let ext: Vec<(f32, u32)> = vec![];
                render_world(&zone, region.as_ref(), &fx, &mut frame, total, &ext, &mut |_, _| {}, None);
                post::finish(&mut frame, &zone.grade);
                for y in 0..H {
                    for x in 0..W {
                        let (gx, gy) = (cx + x, cy + y);
                        if gx < pw && gy < ph {
                            let o = (gy * pw + gx) * 4;
                            big[o..o + 4].copy_from_slice(&frame.out[y * W + x]);
                        }
                    }
                }
                cx += W;
            }
            cy += H;
        }
        let path = out.join(format!("{}_overview.png", zone_name));
        write_png(&path, pw as u32, ph as u32, &big).expect("write png");
        println!("wrote {}", path.display());
    }
}

/// Parse a play script: space-separated tokens of input letters then a tick count.
/// Letters: U D L R (move), A (attack held), X (dash held), C (confirm), P (pause), W (nothing).
/// "R60 A2 W10 A2 W10 A2 W30 DRX2 A50" = walk right 1 s, three taps, a dash down-right, a 50-tick charge.
pub fn parse_script(s: &str) -> Vec<(crate::input::Raw, u32)> {
    let mut out = Vec::new();
    for tok in s.split_whitespace() {
        let letters: String = tok.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        let n: u32 = tok[letters.len()..].parse().unwrap_or(1);
        let mut raw = crate::input::Raw::default();
        for c in letters.to_ascii_uppercase().chars() {
            match c {
                'U' => raw.mv.1 -= 1.0,
                'D' => raw.mv.1 += 1.0,
                'L' => raw.mv.0 -= 1.0,
                'R' => raw.mv.0 += 1.0,
                'A' => raw.attack = true,
                'X' => raw.dash = true,
                'C' => raw.confirm = true,
                'P' => raw.pause = true,
                _ => {}
            }
        }
        out.push((raw, n));
    }
    out
}

/// `glintblade shot [options]`: play the real game headless with a scripted input and save frames.
///   --zone Z        start in a zone (default: title screen)
///   --pos X,Y       warp the player here at the start
///   --script "..."  input script (see parse_script)
///   --ticks N       total ticks to run (default: script length + 30)
///   --every K       save every K ticks (default: only the last tick); --at a,b,c saves those ticks
///   --scale S       output scale (default 3)
///   --out DIR       output folder (default out/shot)
///   --god, --peaceful, --seed N
///   --pilot warden  fight via real inputs, starting near the Summit boss unless --pos is given
///   --pilot tour    traverse each region, collect shards, fight and reach the Warden via real inputs
///   --audit         include timed sound/music events and 15-tick gameplay traces in --report
///   --pilot-delay N wait N ticks before piloting (default 0; use --pos outside awakening range)
///   --until-victory stop once the real victory scene has shown its return prompt
///   --report FILE   write capture settings, milestone ticks and ending scene as JSON
///   --from N        simulate normally but start saving at this tick (default 1)
pub fn shot(args: &[String]) {
    use crate::game::{Game, Options, Scene};
    use crate::input::{Input, Raw};
    let pilot_kind = arg(args, "--pilot");
    assert!(pilot_kind.is_none() || matches!(pilot_kind, Some("warden" | "tour")), "--pilot must be warden or tour");
    let pilot = pilot_kind == Some("warden");
    let touring = pilot_kind == Some("tour");
    let audit = args.iter().any(|a| a == "--audit");
    let pilot_delay: u32 = arg(args, "--pilot-delay").map(|s| s.parse().expect("invalid --pilot-delay")).unwrap_or(0);
    assert!(pilot || pilot_delay == 0, "--pilot-delay needs --pilot warden");
    let start = arg(args, "--zone").map(|s| ZoneId::parse(s).expect("unknown --zone"));
    assert!(!pilot || start.is_none() || start == Some(ZoneId::Summit), "warden pilot needs --zone summit");
    assert!(!pilot || !args.iter().any(|a| a == "--peaceful"), "warden pilot needs a live boss");
    let until_victory = args.iter().any(|a| a == "--until-victory");
    let opts = Options {
        start: start.or(if pilot { Some(ZoneId::Summit) } else if touring { Some(ZoneId::Bloomwild) } else { None }),
        god: args.iter().any(|a| a == "--god"),
        peaceful: args.iter().any(|a| a == "--peaceful"),
        seed: arg(args, "--seed").and_then(|s| s.parse().ok()).unwrap_or(7),
    };
    assert!(!(pilot || touring) || arg(args, "--script").is_none(), "choose --pilot or --script");
    let seed = opts.seed;
    let script = parse_script(arg(args, "--script").unwrap_or(""));
    let script_len: u32 = script.iter().map(|s| s.1).sum();
    let ticks: u32 = arg(args, "--ticks").and_then(|s| s.parse().ok()).unwrap_or(if touring { 36_000 } else if pilot { 24_000 } else { script_len + 30 });
    let every: u32 = arg(args, "--every").and_then(|s| s.parse().ok()).unwrap_or(0);
    let from: u32 = arg(args, "--from").and_then(|s| s.parse().ok()).unwrap_or(1);
    let at: Vec<u32> = arg(args, "--at").map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
    let scale: usize = arg(args, "--scale").and_then(|s| s.parse().ok()).unwrap_or(3);
    let out = PathBuf::from(arg(args, "--out").unwrap_or("out/shot"));
    assert!(ticks > 0 && (1..=8).contains(&scale), "shot needs --ticks > 0 and --scale 1..8");
    std::fs::create_dir_all(&out).expect("create shot output directory");
    let mut game = Game::new(opts);
    let pos = arg(args, "--pos").and_then(pair).or_else(|| {
        if pilot && let Scene::Play(p) = &game.scene {
            p.zone.boss_pos.map(|(x, y)| (x, y + 48.0))
        } else {
            None
        }
    });
    if let Some((x, y)) = pos
        && let Scene::Play(p) = &mut game.scene
    {
        p.player.body.x = x;
        p.player.body.y = y;
        p.checkpoint = (x, y);
        p.cam.snap(x, y);
    }
    let mut input = Input::default();
    let mut steps = script.into_iter().flat_map(|(raw, n)| std::iter::repeat_n(raw, n as usize));
    let mut written = 0;
    let started = std::time::Instant::now();
    let mut milestones = serde_json::Map::new();
    let mut last_tick = 0;
    let mut victory_ready = false;
    let mut tour = tour::Tour::default();
    let mut audio = Vec::new();
    let mut trace = Vec::new();
    let mut last_music = None;
    for t in 1..=ticks {
        let raw = if pilot { warden_input(&game, t, pilot_delay) } else if touring { tour.input(&game, t) } else { steps.next().unwrap_or(Raw::default()) };
        input.step(raw);
        game.tick(&input);
        if audit {
            if game.fx.music != last_music {
                audio.push(serde_json::json!({"tick":t,"music":game.fx.music}));
                last_music = game.fx.music.clone();
            }
            for sound in &game.fx.sounds {
                audio.push(serde_json::json!({"tick":t,"sound":sound.name,"volume":sound.volume}));
            }
            if t % 15 == 0 && let Scene::Play(p) = &game.scene {
                let cam = p.cam.view(p.zone.pw, p.zone.ph, game.fx.trauma, game.fx.t);
                trace.push(serde_json::json!({"tick":t,"zone":format!("{:?}",p.id).to_lowercase(),
                    "pos":[p.player.body.x,p.player.body.y],"cam":[cam.0,cam.1],"hp":p.player.body.hp,
                    "shard_taken":p.shard_taken,"enemies_alive":p.enemies.iter().filter(|e|e.body.alive).count(),
                    "boss_hp":p.boss.as_ref().map(|b|b.body.hp),"deaths":p.deaths}));
            }
        }
        game.fx.sounds.clear();
        last_tick = t;
        if let Scene::Play(p) = &game.scene && p.shard_taken {
            milestones.entry(format!("{:?}_shard", p.id).to_lowercase()).or_insert(serde_json::json!(t));
        }
        if let Scene::Play(p) = &game.scene && let Some(b) = &p.boss {
            for (name, reached) in [
                ("warden_awake", p.boss_awake),
                // Strictly below a phase floor proves damage in the next phase, not its transition.
                ("phase_2_damage", b.body.hp < 40),
                ("phase_3_damage", b.body.hp < 20),
                ("phase_2_heart_open", b.body.hp < 40 && b.body.hp > 20 && b.hurtboxes().iter().any(|h| h.3 == 20)),
                ("warden_dying", b.dying > 0),
            ] {
                if reached { milestones.entry(name).or_insert(serde_json::json!(t)); }
            }
        }
        if let Scene::Victory { t: vt, .. } = &game.scene {
            milestones.entry("victory").or_insert(serde_json::json!(t));
            victory_ready = *vt >= crate::ui::VICTORY_PROMPT + 30;
        }
        let save = (every > 0 && t % every == 0) || at.contains(&t) || (every == 0 && at.is_empty() && (t == ticks || (until_victory && victory_ready)));
        if save && t >= from {
            game.render();
            let img = post::composite_scaled(&game.frame, scale);
            write_png(&out.join(format!("f{t:05}.png")), (W * scale) as u32, (H * scale) as u32, &img).expect("png");
            written += 1;
        } else {
            game.fx.lights.clear();
        }
        if until_victory && victory_ready { break; }
    }
    if arg(args, "--report").is_some() {
        let scene = match &game.scene {
            Scene::Title { .. } => "title",
            Scene::Play(_) => "play",
            Scene::Victory { .. } => "victory",
        };
        let enemies_defeated = match &game.scene {
            Scene::Play(p) => p.enemies.iter().filter(|e| !e.body.alive).count(),
            _ => 0,
        };
        let end_pos = match &game.scene {
            Scene::Play(p) => Some([p.player.body.x, p.player.body.y]),
            _ => None,
        };
        let report = serde_json::json!({
            "seed": seed, "pilot": pilot_kind,
            "pilot_delay": pilot_delay, "start_pos": pos,
            "god": args.iter().any(|a| a == "--god"), "ticks": last_tick,
            "every": every, "from": from, "scale": scale, "frames": written, "scene": scene,
            "victory_ready": victory_ready, "milestones": milestones, "enemies_defeated": enemies_defeated,
            "end_pos": end_pos,
            "simulation_hz": 60, "capture_fps": if every > 0 { Some(60.0 / every as f32) } else { None },
            "visited_zones": tour.visited, "audio_events": audio, "trace": trace,
        });
        std::fs::write(arg(args, "--report").unwrap(), serde_json::to_vec_pretty(&report).expect("serialize shot report")).expect("write shot report");
    }
    println!("wrote {written} frame(s) to {} in {:.1}s", out.display(), started.elapsed().as_secs_f32());
    assert!(!until_victory || victory_ready, "victory prompt not reached within --ticks {ticks}");
}

/// Pilot the real sword toward exposed Warden weak points without warping or injecting damage.
fn warden_input(game: &crate::game::Game, tick: u32, delay: u32) -> crate::input::Raw {
    use crate::game::Scene;
    use crate::input::Raw;
    if tick <= delay { return Raw::default(); }
    let Scene::Play(p) = &game.scene else { return Raw::default() };
    let Some(b) = &p.boss else { return Raw::default() };
    if b.dying > 0 { return Raw::default(); }
    let (px, py) = (p.player.body.x, p.player.body.y);
    // Boss::hurtboxes documents weak points before armor. Part IDs below 30 are
    // fist crystals (0..2), pylons (10..18), and the exposed heart (20).
    let target = b.hurtboxes().into_iter().filter(|h| h.3 < 30)
        .min_by(|a, c| (a.0 - px).hypot(a.1 - py).total_cmp(&(c.0 - px).hypot(c.1 - py)));
    let (tx, ty, attacking) = target.map(|(x, y, _, _)| (x, y + 5.0, true))
        .unwrap_or((b.body.x, b.body.y + 48.0, false));
    let (dx, dy) = (tx - px, ty - py);
    let dist = dx.hypot(dy);
    Raw {
        mv: if dist > (if attacking { 20.0 } else { 5.0 }) || (attacking && tick % 8 == 0) {
            crate::util::norm(dx, dy)
        } else { (0.0, 0.0) },
        attack: attacking && dist < 34.0 && tick % 8 == 0,
        ..Raw::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercise all Warden health bands, the real death sequence and victory via ordinary inputs.
    #[test]
    fn warden_pilot_reaches_real_victory() {
        use crate::game::{Game, Options, Scene};
        let mut game = Game::new(Options { start: Some(ZoneId::Summit), god: true, ..Options::default() });
        if let Scene::Play(p) = &mut game.scene {
            let (x, y) = p.zone.boss_pos.unwrap();
            p.player.respawn(x, y + 48.0);
            p.checkpoint = (x, y + 48.0);
        }
        let mut input = crate::input::Input::default();
        let mut health_bands = [false; 3];
        let mut fitted_frames = 0;
        let mut saw_tight_beam_fit = false;
        let mut last_lead_in_view: Option<(u32, i32, i32)> = None;
        let mut last_attack_view: Option<(i32, i32, bool)> = None;
        let mut saw_nonfitting_charge_onset = false;
        for t in 1..=24_000 {
            input.step(warden_input(&game, t, 0));
            game.tick(&input);
            game.fx.sounds.clear();
            game.fx.lights.clear();
            if let Scene::Play(p) = &game.scene && let Some(b) = &p.boss {
                let (_, rendered_y) = p.cam.view(p.zone.pw, p.zone.ph, game.fx.trauma, game.fx.t);
                let (_, unshaken_y) = p.cam.view(p.zone.pw, p.zone.ph, 0.0, game.fx.t);
                let active_source = b.attack_sources().next().is_some();
                if let Some((prev_y, prev_unshaken, was_active)) = last_attack_view {
                    if active_source && !was_active {
                        assert!((unshaken_y - prev_unshaken).abs() <= 2, "tick {t}: charge onset switched camera margin abruptly");
                        assert!((rendered_y - prev_y).abs() <= 3, "tick {t}: charge onset moved the viewport abruptly");
                        if t == 1137 {
                            saw_nonfitting_charge_onset = true;
                            assert!(p.player.body.y - p.player.body.z - b.framing_top().unwrap() > H as f32 - 48.0);
                            assert!(p.player.body.y - p.player.body.z - rendered_y as f32 <= 178.0, "non-fitting view must retain weapon clearance");
                        }
                    }
                    if t == 2612 {
                        assert!((unshaken_y - prev_unshaken).abs() <= 2, "twin firing switched fit policy abruptly");
                        assert!((rendered_y - prev_y).abs() <= 3, "twin firing snapped the actual viewport");
                    }
                    if was_active && !active_source {
                        assert!((unshaken_y - prev_unshaken).abs() <= 3, "tick {t}: beam release snapped the camera");
                    }
                }
                last_attack_view = Some((rendered_y, unshaken_y, active_source));
                if active_source {
                    let (cell, anchor, _) = p.player.current_cell().unwrap();
                    let bottom = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).unwrap();
                    let weapon_bottom = (p.player.body.y - p.player.body.z).floor() as i32 + bottom - anchor.1 - rendered_y;
                    assert!(weapon_bottom < H as i32 - 20, "tick {t}: actual authored blade overlaps boss HUD at row {weapon_bottom}");
                }
                if (1098..=1140).contains(&t) || (2490..=2540).contains(&t) {
                    let (_, cy) = p.cam.view(p.zone.pw, p.zone.ph, game.fx.trauma, game.fx.t);
                    let (_, unshaken) = p.cam.view(p.zone.pw, p.zone.ph, 0.0, game.fx.t);
                    if let Some((prev_t, prev_y, prev_unshaken)) = last_lead_in_view {
                        if prev_t + 1 == t {
                            assert!((unshaken - prev_unshaken).abs() <= 2, "tick {t}: camera snapped during beam lead-in: {prev_unshaken}->{unshaken}");
                            assert!((cy - prev_y).abs() <= 3, "tick {t}: rendered view snapped during beam lead-in: {prev_y}->{cy}");
                        }
                        if t == 2523 {
                            assert!((cy - prev_y).abs() <= 1, "actual2522->2523 beam-onset snap returned");
                        }
                    }
                    last_lead_in_view = Some((t, cy, unshaken));
                }
                health_bands[0] |= b.body.hp > 40;
                health_bands[1] |= b.body.hp < 40 && b.body.hp > 20;
                health_bands[2] |= b.body.hp < 20 && b.body.hp > 0;
                if let Some(top) = b.framing_top()
                    && b.body.dist_to(p.player.body.x, p.player.body.y) <= 180.0
                    && p.player.body.y - p.player.body.z - top <= H as f32 - 48.0
                {
                    let (_, cy) = p.cam.view(p.zone.pw, p.zone.ph, game.fx.trauma, game.fx.t);
                    assert!(top - cy as f32 >= 10.0, "tick {t}: a fitting boss/core was cropped");
                    if b.attack_sources().next().is_some() {
                        if t == 2529 {
                            saw_tight_beam_fit = true;
                            assert!(p.player.body.y - p.player.body.z - top > 150.0, "regression must exercise the tight full-assembly composition");
                        }
                    }
                    fitted_frames += 1;
                }
            }
            if let Scene::Victory { t: vt, .. } = &game.scene && *vt >= crate::ui::VICTORY_PROMPT + 30 {
                assert!(health_bands.into_iter().all(|b| b));
                assert!(fitted_frames > 500, "exercise framing throughout actual combat");
                assert!(saw_tight_beam_fit, "exercise the actual tick2529 beam/crown/weapon regression");
                assert!(saw_nonfitting_charge_onset, "exercise the actual1137 non-fitting onset");
                println!("real Warden pilot reached victory in {t} ticks");
                return;
            }
        }
        panic!("Warden pilot did not finish within 24,000 ticks");
    }

    /// Wait outside awakening, enter with ordinary input, and retain a caster only while stone exists.
    #[test]
    fn delayed_warden_pilot_reveals_after_banner_and_finishes() {
        use crate::game::{Game, Options, Scene};
        let mut game = Game::new(Options { start: Some(ZoneId::Summit), god: true, ..Options::default() });
        if let Scene::Play(p) = &mut game.scene {
            p.player.respawn(720.0, 520.0);
            p.checkpoint = (720.0, 520.0);
            p.cam.snap(720.0, 520.0);
        }
        let mut input = crate::input::Input::default();
        let mut awake = 0;
        let mut saw_remnant = false;
        for t in 1..=24_000 {
            input.step(warden_input(&game, t, 300));
            game.tick(&input);
            game.fx.sounds.clear();
            game.fx.lights.clear();
            if let Scene::Play(p) = &mut game.scene {
                if t <= 300 { assert!(!p.boss_awake, "waiting must not start the reveal"); }
                if p.boss_awake && awake == 0 { awake = t; }
                if let Some(b) = &p.boss && b.dying == 250 {
                    let (bx, by) = (b.body.x, b.body.y);
                    let mut frame = Frame::new();
                    p.render(&mut frame, &game.fx);
                    assert!(!frame.casters.iter().any(|c| c.x == bx && c.y == by));
                    saw_remnant = true;
                }
            }
            if let Scene::Victory { t: vt, .. } = &game.scene && *vt >= crate::ui::VICTORY_PROMPT + 30 {
                assert!(awake > 300 && saw_remnant);
                println!("delayed pilot awakened at {awake}, victory prompt at {t}");
                return;
            }
        }
        panic!("delayed Warden pilot did not finish");
    }
}
