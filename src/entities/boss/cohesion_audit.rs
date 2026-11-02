//! Manual, deterministic visual coverage of the assembled Warden in the real Summit.
//! No production state, combat timing, art, lighting or renderer switches are changed.
//!
//! GLINT_WARDEN_AUDIT=/tmp/warden-audit cargo test --release --lib
//!     capture_warden_cohesion -- --ignored --nocapture --test-threads=1
//! For paired old/new art captures, set GLINT_WARDEN_AUDIT_DIAGNOSTIC_CAMERA_Y
//! to the same world-space camera center in both runs (the manifest records it).

use super::*;
use crate::gfx::{H, W};
use crate::input::{Input, Raw};
use crate::play::{Play, PlayOpts};
use crate::world::ZoneId;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

struct Scene {
    play: Play,
    fx: Fx,
    input: Input,
    hud: crate::ui::Hud,
    camera_mode: CameraMode,
    diagnostic_camera_y: f32,
    hold_next_attack: bool,
    last_input: Raw,
    update_count: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    /// Art registration fixture, intentionally independent of the gameplay camera.
    Diagnostic,
    /// Play::update alone owns camera position, bounds, shake and attack framing.
    Game,
}

impl CameraMode {
    fn label(self) -> &'static str {
        match self {
            Self::Diagnostic => "diagnostic-art-registration",
            Self::Game => "game-camera",
        }
    }
}

impl Scene {
    fn new(phase: usize) -> Self {
        let mut play = Play::new(
            ZoneId::Summit,
            &PlayOpts {
                god: true,
                peaceful: false,
            },
        );
        let (x, y) = play.zone.boss_pos.unwrap();
        // Stand beside the giant so the complete native sculpture and Glint fit.
        // This exact hero fixture is shared by old/new registration captures.
        play.player = crate::entities::Player::new(x + 85.0, y + 4.0);
        play.enemies.clear();
        play.boss_awake = true;
        play.arrive_t = 10_000;
        play.t = 120;
        let b = play.boss.as_mut().unwrap();
        b.phase = phase;
        b.hp = b.top_hp();
        b.body.hp = b.hp;
        b.mode = Mode::Fight;
        b.cool = 10_000;
        b.t = 120;
        b.body_dy = 0.0;
        b.head_dy = 0.0;
        b.eyes = 1.0;
        b.shoulder = [if phase == 0 { 0.45 } else { 1.0 }; 2];
        b.crack_lit = [0.0, 130.0, 360.0][phase];
        b.player = (play.player.body.x, play.player.body.y);
        for side in 0..2 {
            let (x, y, z) = b.rest_pos(side);
            let f = &mut b.fists[side];
            (f.x, f.y, f.z) = (x, y, z);
            f.st = St::Rest;
            f.t = 120;
            f.trail = [(x, y, z); 3];
        }
        let diagnostic_camera_y = std::env::var("GLINT_WARDEN_AUDIT_DIAGNOSTIC_CAMERA_Y")
            .map(|v| v.parse::<f32>().expect("finite diagnostic camera Y"))
            .unwrap_or_else(|_| (b.framing_top().unwrap_or(y) + y + 10.0) * 0.5);
        assert!(diagnostic_camera_y.is_finite());
        let mut fx = Fx::new(7);
        fx.t = 120;
        let mut hud = crate::ui::Hud::new();
        for _ in 0..100 {
            hud.update(&play, 3, &mut fx);
        }
        fx.sounds.clear();
        Self {
            play,
            fx,
            input: Input::default(),
            hud,
            camera_mode: CameraMode::Diagnostic,
            diagnostic_camera_y,
            hold_next_attack: true,
            last_input: Raw::default(),
            update_count: 0,
        }
    }

    /// Ordinary entrance/checkpoint state: no fixed fixture or manual camera snap.
    fn gameplay(god: bool) -> Self {
        let play = Play::new(
            ZoneId::Summit,
            &PlayOpts {
                god,
                peaceful: false,
            },
        );
        Self {
            play,
            fx: Fx::new(7),
            input: Input::default(),
            hud: crate::ui::Hud::new(),
            camera_mode: CameraMode::Game,
            diagnostic_camera_y: 0.0,
            hold_next_attack: false,
            last_input: Raw::default(),
            update_count: 0,
        }
    }

    /// A controlled combat state with normal following from its first Play update.
    /// Only the initial actor placement is synthetic; rendering never snaps the camera.
    fn game_probe(phase: usize, offset: (f32, f32)) -> Self {
        let mut scene = Self::new(phase);
        let (x, y) = scene.play.zone.boss_pos.unwrap();
        scene.play.player = crate::entities::Player::new(x + offset.0, y + offset.1);
        scene.play.cam =
            crate::camera::Camera::new(scene.play.player.body.x, scene.play.player.body.y);
        scene.camera_mode = CameraMode::Game;
        for _ in 0..120 {
            scene.tick();
        }
        scene
    }

    fn boss(&self) -> &Boss {
        self.play.boss.as_ref().unwrap()
    }
    fn boss_mut(&mut self) -> &mut Boss {
        self.play.boss.as_mut().unwrap()
    }

    /// Trigger an existing production transition, then let normal Play updates run it.
    fn action(&mut self, action: impl FnOnce(&mut Boss, &mut EnemyCtx)) {
        let play = &mut self.play;
        let mut hits = Vec::new();
        let mut ctx = EnemyCtx {
            player: (play.player.body.x, play.player.body.y),
            player_alive: true,
            zone: &play.zone,
            fx: &mut self.fx,
            hits: &mut hits,
            projectiles: &mut play.projectiles,
            t: play.t,
        };
        action(play.boss.as_mut().unwrap(), &mut ctx);
    }

    fn tick(&mut self) {
        self.tick_input(Raw::default());
    }

    fn tick_input(&mut self, raw: Raw) {
        self.fx.sounds.clear();
        self.fx.lights.clear();
        // Hold the next attack while auditing the current move's complete recovery.
        // This changes no move clock and never runs in the shipped game.
        if self.hold_next_attack && self.boss().attack.is_none() {
            self.boss_mut().cool = 10_000;
        }
        self.last_input = raw;
        self.input.step(raw);
        self.play.update(&self.input, &mut self.fx);
        self.hud.update(&self.play, 3, &mut self.fx);
        self.fx.tick();
        self.update_count += 1;
    }

    fn until(&mut self, description: &str, state: impl Fn(&Boss) -> bool) {
        for _ in 0..2_000 {
            if state(self.boss()) {
                return;
            }
            self.tick();
        }
        panic!(
            "audit never reached {description}: mode {:?}, attack {:?}, fists {:?}",
            self.boss().mode,
            self.boss().attack,
            self.boss().fists.iter().map(|f| f.st).collect::<Vec<_>>()
        );
    }

    /// Match Game::render: normal world lighting, post glow, crisp HUD, final screen flash.
    /// The art fixture and ordinary game camera are separate and explicitly tagged.
    /// No zoom, lighting or shadow debug mode is enabled.
    fn render(&mut self, include_flash: bool) -> Frame {
        if self.camera_mode == CameraMode::Diagnostic {
            let (x, _) = self.play.zone.boss_pos.unwrap();
            self.play.cam.snap(x, self.diagnostic_camera_y);
        }
        self.play.add_lights(&mut self.fx);
        let mut f = Frame::new();
        self.play.render(&mut f, &self.fx);
        f.fade = 0.0;
        f.flash = 0.0;
        crate::gfx::post::finish(&mut f, &self.play.zone.grade);
        crate::ui::hud(&mut f, &self.play, &self.hud);
        if include_flash {
            f.flash = self.fx.flash.clamp(0.0, 1.0);
            for p in &mut f.out {
                for c in p.iter_mut().take(3) {
                    *c = (*c as f32 + (255.0 - *c as f32) * f.flash) as u8;
                }
            }
        }
        f
    }
}

struct Capture {
    name: String,
    group: String,
    pixels: Vec<[u8; 4]>,
    state: Value,
}

/// Keep the tight beam regression independent of the pilot's route: a larger
/// guardian naturally makes its ordinary frontal approach unable to fit.
#[test]
fn native_beam_camera_retains_crown_and_authored_blade_at_full_fit() {
    for phase in [1, 2] {
        let seed = Scene::new(phase);
        let height = seed.boss().body.y - seed.boss().framing_top().unwrap();
        let mut scene = Scene::game_probe(phase, (85.0, H as f32 - 48.0 - height));
        scene.boss_mut().attack = Some(Attack::Beam {
            t: 0,
            twin: phase == 2,
        });
        let mut seen = [false; 2];
        let mut checked = 0;
        for _ in 0..400 {
            scene.tick();
            let b = scene.boss();
            seen[0] |= b.beams.iter().any(|beam| beam.charging());
            seen[1] |= b.beams.iter().any(|beam| beam.firing());
            if b.attack_sources().next().is_none() {
                continue;
            }
            let player = &scene.play.player;
            let top = b.framing_top().unwrap();
            let span = player.body.y - player.body.z - top;
            let (_, cy) = scene.play.cam.view(
                scene.play.zone.pw,
                scene.play.zone.ph,
                scene.fx.trauma,
                scene.fx.t,
            );
            let (cell, anchor, _) = player.current_cell().unwrap();
            let bottom = (0..cell.h)
                .rev()
                .find(|&y| (0..cell.w).any(|x| cell.at(x, y).a))
                .unwrap();
            let ink_bottom =
                (player.body.y - player.body.z).floor() as i32 + bottom - anchor.1 - cy;
            assert!(
                ink_bottom < H as i32 - 20,
                "phase {} blade at HUD row {ink_bottom}",
                phase + 1
            );
            // Beam knockback can move even god-mode Glint out of the fitting
            // interval. Still check the weapon there; require the crown whenever
            // the full assembly and Glint can physically fit.
            if span <= H as f32 - 48.0 {
                assert!(span >= 150.0, "tight native fit: {span}");
                assert!(top - cy as f32 >= 10.0, "phase {} crown clipped", phase + 1);
                checked += 1;
            }
        }
        assert!(
            seen.into_iter().all(|v| v) && checked > 30,
            "exercise real charge, fire and sweep: phase {} checked {checked}",
            phase + 1
        );
    }
}

struct Audit {
    directory: PathBuf,
    captures: Vec<Capture>,
}

fn write_pixels(path: &Path, width: usize, height: usize, pixels: &[[u8; 4]], scale: usize) {
    let mut rgba = Vec::with_capacity(width * height * scale * scale * 4);
    for row in pixels.chunks_exact(width) {
        for _ in 0..scale {
            for pixel in row {
                for _ in 0..scale {
                    rgba.extend_from_slice(pixel);
                }
            }
        }
    }
    crate::assets::write_png(path, (width * scale) as u32, (height * scale) as u32, &rgba).unwrap();
}

impl Audit {
    fn save(&mut self, scene: &mut Scene, group: &str, name: &str) {
        let source = if scene.camera_mode == CameraMode::Game {
            "game-camera-live-play-updates"
        } else {
            "diagnostic-fixture-live-updates"
        };
        self.save_as(scene, group, name, true, source);
    }

    fn save_as(&mut self, scene: &mut Scene, group: &str, name: &str, flash: bool, source: &str) {
        let frame = scene.render(flash);
        let b = scene.boss();
        let body = b.body_sprite();
        let (head, head_x, head_y) = b.head_part();
        let crown_y = (head_y + b.breath() + art::top(head) as f32)
            .min(b.body.y + b.body_dy.round() + art::top(body) as f32);
        let crown_present = !matches!(b.mode, Mode::Dying(t) if t > death::BURST);
        let hero_screen = [
            scene.play.player.body.x - frame.cam_x as f32,
            scene.play.player.body.y - scene.play.player.body.z - frame.cam_y as f32,
        ];
        let crown_screen = [head_x - frame.cam_x as f32, crown_y - frame.cam_y as f32];
        // Use the exact native ink, including the held blade, and the renderer's
        // floor convention. Floating anchors alone overstate fractional overflow.
        let hero_ink = scene.play.player.current_cell().map(|(cell, anchor, _)| {
            let (mut left, mut top, mut right, mut bottom) = (cell.w, cell.h, -1, -1);
            for y in 0..cell.h {
                for x in 0..cell.w {
                    if cell.at(x, y).a {
                        left = left.min(x);
                        top = top.min(y);
                        right = right.max(x);
                        bottom = bottom.max(y);
                    }
                }
            }
            let x = scene.play.player.body.x.floor() as i32 - anchor.0 - frame.cam_x;
            let y = (scene.play.player.body.y - scene.play.player.body.z).floor() as i32
                - anchor.1
                - frame.cam_y;
            [left + x, top + y, right + x, bottom + y]
        });
        let on_screen =
            |p: [f32; 2]| (0.0..W as f32).contains(&p[0]) && (0.0..H as f32).contains(&p[1]);
        let mut state = json!({
            "name": name, "group": group, "source": source,
            "camera_mode": scene.camera_mode.label(),
            "diagnostic_camera_center_y": (scene.camera_mode == CameraMode::Diagnostic)
                .then_some(scene.diagnostic_camera_y),
            "camera_updated_by_play": scene.camera_mode == CameraMode::Game,
            "fixture_holds_followup_attacks": scene.hold_next_attack,
            "play_updates": scene.update_count,
            "input": {"move": [scene.last_input.mv.0, scene.last_input.mv.1],
                "attack": scene.last_input.attack, "dash": scene.last_input.dash},
            "native": format!("native/{name}.png"), "nearest_3x": format!("3x/{name}.png"),
            "mode": format!("{:?}", b.mode), "attack": format!("{:?}", b.attack),
            "phase": b.phase + 1, "hp": b.hp, "boss_tick": b.t,
            "scene_tick": scene.play.t, "effect_tick": scene.fx.t,
            "body_dy": b.body_dy, "head_dy": b.head_dy,
            "breath": b.breath(), "tremble": b.tremble,
            "heart_out": b.heart.out, "heart_position": b.heart_pos(),
            "body_fold_animation": b.body_fold_anim(),
            "support_contacts": b.support_contacts(),
        });
        state.as_object_mut().unwrap().extend(json!({
            "body_cell": [body.cell.w, body.cell.h],
            "body_anchor": body.anchor,
            "integrated_head_rig": pose::rig().integrated_head,
            "head_cell": [head.cell.w, head.cell.h], "head_anchor": head.anchor,
            "head_world_anchor": [head_x, head_y],
            "body_bottom_relative_to_ground": art::bottom(body) + b.body_dy.round() as i32,
            "fists": b.fists.iter().map(|f| json!({"side":f.side,"state":format!("{:?}",f.st),
                "x":f.x,"y":f.y,"z":f.z,"crystal_hp":f.crystal_hp})).collect::<Vec<_>>(),
            "camera": [frame.cam_x, frame.cam_y],
            "player": [scene.play.player.body.x, scene.play.player.body.y, scene.play.player.body.z],
            "player_state": format!("{:?}", scene.play.player.state),
            "player_alive": scene.play.player.alive(), "deaths": scene.play.deaths,
            "boss_awake": scene.play.boss_awake,
            "hero_screen_anchor": hero_screen, "hero_on_screen": on_screen(hero_screen),
            "hero_anchor_in_safe_view": (24.0..=W as f32-24.0).contains(&hero_screen[0])
                && (36.0..=H as f32-24.0).contains(&hero_screen[1]),
            "hero_native_ink_bounds": hero_ink,
            "hero_weapon_envelope_clears_hud": !scene.play.player.alive()
                || hero_ink.is_some_and(|[_, top, _, bottom]| top >= 0 && bottom < H as i32-20),
            "crown_present": crown_present, "crown_world_y": crown_y,
            "crown_screen": crown_screen,
            "crown_on_screen": crown_present && on_screen(crown_screen),
            "assembly_top_world_y": b.framing_top(),
            "assembly_top_screen_y": b.framing_top().map(|y| y-frame.cam_y as f32),
            "assembly_and_hero_can_fit": b.framing_top().is_some_and(|y|
                scene.play.player.body.y-scene.play.player.body.z-y <= H as f32-48.0),
            "surface_lighting": frame.surface_lighting,
            "native_surface_casters": frame.surface_casters.len(),
            "point_lights": frame.point_lights.iter().map(|p| json!({
                "x":p.x,"y":p.y,"height":p.height,"radius":p.radius,
                "intensity":p.intensity})).collect::<Vec<_>>(),
            "beams": b.beams.len(), "pylons": b.pylons.iter().filter(|p| p.alive).count(),
            "particles": scene.fx.particles.list.len(),
            "effect_flash": scene.fx.flash, "included_flash": flash,
        }).as_object().unwrap().clone());
        state["sounds"] = json!(scene.fx.sounds.iter().map(|s| &s.name).collect::<Vec<_>>());
        state["hitstop"] = json!(scene.fx.hitstop);
        write_pixels(
            &self.directory.join("native").join(format!("{name}.png")),
            W,
            H,
            &frame.out,
            1,
        );
        write_pixels(
            &self.directory.join("3x").join(format!("{name}.png")),
            W,
            H,
            &frame.out,
            3,
        );
        self.captures.push(Capture {
            name: name.into(),
            group: group.into(),
            pixels: frame.out,
            state,
        });
    }

    fn finish(self) {
        let mut groups = Vec::<String>::new();
        for c in &self.captures {
            if !groups.contains(&c.group) {
                groups.push(c.group.clone());
            }
        }
        let mut sheets = Vec::new();
        for group in groups {
            let captures = self
                .captures
                .iter()
                .filter(|c| c.group == group)
                .collect::<Vec<_>>();
            for (page, chunk) in captures.chunks(12).enumerate() {
                let columns = chunk.len().min(3);
                let rows = chunk.len().div_ceil(columns);
                let (width, height) = (columns * (W + 4) + 4, rows * (H + 16) + 4);
                let mut pixels = vec![[14, 18, 28, 255]; width * height];
                for (index, capture) in chunk.iter().enumerate() {
                    let (x, y) = (
                        4 + index % columns * (W + 4),
                        4 + index / columns * (H + 16),
                    );
                    for row in 0..H {
                        pixels[(y + row) * width + x..(y + row) * width + x + W]
                            .copy_from_slice(&capture.pixels[row * W..(row + 1) * W]);
                    }
                    let mut label = Frame::new();
                    label.out.fill([14, 18, 28, 255]);
                    crate::gfx::font::text(
                        &mut label,
                        0,
                        2,
                        &capture.name.replace('_', "-"),
                        Rgb::WHITE,
                        1,
                        0.0,
                    );
                    for row in 0..10 {
                        pixels[(y + H + row) * width + x..(y + H + row) * width + x + W]
                            .copy_from_slice(&label.out[row * W..(row + 1) * W]);
                    }
                }
                let name = format!("{group}-{:02}", page + 1);
                for scale in [1, 3] {
                    write_pixels(
                        &self
                            .directory
                            .join("sheets")
                            .join(format!("{name}-{scale}x.png")),
                        width,
                        height,
                        &pixels,
                        scale,
                    );
                }
                sheets.push(json!({"group":group,"page":page+1,
                    "native":format!("sheets/{name}-1x.png"),
                    "nearest_3x":format!("sheets/{name}-3x.png"),
                    "captures":chunk.iter().map(|c| &c.name).collect::<Vec<_>>()}));
            }
        }
        let diagnostic_count = self
            .captures
            .iter()
            .filter(|c| c.state["camera_mode"] == "diagnostic-art-registration")
            .count();
        let manifest = json!({"version":2,"resolution":[W,H],"seed":7,
            "render":"actual Summit Play::render + normal surface lighting + post::finish + HUD + FX flash",
            "framing":{
                "diagnostic-art-registration":"Fixed native framing for art registration ONLY; hero bx+85/by+4. Center Y derives from resting native assembly top and by+10, or the explicitly recorded GLINT_WARDEN_AUDIT_DIAGNOSTIC_CAMERA_Y override for identical old/new fixtures. This is not the normal game camera.",
                "game-camera":"Play::update owns follow, native viewport bounds, attack-source framing and shake. Rendering never snaps this camera. Controlled game_* probes seed actor states/positions then run ordinary updates; route_* probes begin at the ordinary entrance and use only real inputs.",
                "native_zoom":"No zoom, lighting or shadow overrides in either mode."},
            "coverage_counts":{"diagnostic":diagnostic_count,"game_camera":self.captures.len()-diagnostic_count,
                "total":self.captures.len()},
            "synthetic_coverage":"Explicit fold_* captures set pose depth directly. Other combat fixtures seed a production state or start an existing transition, then all motion uses ordinary Play updates. Game-camera route_* captures use only entrance state plus real inputs.",
            "contact_sheets":sheets,"captures":self.captures.iter().map(|c| &c.state).collect::<Vec<_>>()});
        std::fs::write(
            self.directory.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        println!(
            "WARDEN_COHESION_AUDIT {} captures: {}",
            self.captures.len(),
            self.directory.display()
        );
    }
}

#[test]
#[ignore = "manual full-cadence vulnerable kneel audit"]
fn capture_warden_kneel() {
    let directory =
        PathBuf::from(std::env::var("GLINT_WARDEN_AUDIT").expect("private audit directory"));
    for sub in ["native", "3x", "sheets"] {
        std::fs::create_dir_all(directory.join(sub)).unwrap();
    }
    let mut audit = Audit {
        directory,
        captures: Vec::new(),
    };
    for phase in 0..3 {
        let mut scene = Scene::new(phase);
        scene.action(|b, ctx| {
            if phase == 0 {
                for f in &mut b.fists {
                    f.crystal_hp = 0;
                }
            }
            b.start_stagger(ctx);
        });
        let group = format!("p{}_fall", phase + 1);
        // Capture wall ticks, including every held impact and reaction frame.
        for tick in 0..80 {
            audit.save(&mut scene, &group, &format!("{group}_{tick:03}"));
            scene.tick();
        }
        let end = 38 + TEMPO[phase].window;
        scene.until("closing lead-in", |b| b.mode == Mode::Stagger(end - 6));
        let group = format!("p{}_rise", phase + 1);
        for tick in 0..50 {
            audit.save(&mut scene, &group, &format!("{group}_{tick:03}"));
            scene.tick();
        }
        if phase < 2 {
            let mut scene = Scene::new(phase);
            scene.action(|b, ctx| b.start_stagger(ctx));
            scene.until("naturally settled phase-clear pose", |b| {
                b.mode == Mode::Stagger(50)
            });
            scene.action(|b, ctx| {
                let _ = ctx;
                b.hp = b.floor_hp();
                b.body.hp = b.hp;
                b.chunk_done = true;
            });
            let group = format!("p{}_clear", phase + 1);
            for tick in 0..60 {
                audit.save(&mut scene, &group, &format!("{group}_{tick:03}"));
                scene.tick();
            }
        }
    }
    audit.finish();
}

#[test]
#[ignore = "manual native Summit visual audit; writes PNGs and manifest to GLINT_WARDEN_AUDIT"]
fn capture_warden_cohesion() {
    let directory = PathBuf::from(
        std::env::var("GLINT_WARDEN_AUDIT").unwrap_or_else(|_| "/tmp/glint-warden-cohesion".into()),
    );
    for sub in ["native", "3x", "sheets"] {
        std::fs::create_dir_all(directory.join(sub)).unwrap();
    }
    let mut audit = Audit {
        directory,
        captures: Vec::new(),
    };

    let mut scene = Scene::new(0);
    let (x, y) = scene.play.zone.boss_pos.unwrap();
    scene.play.boss = Some(Boss::new(x, y));
    scene.play.boss_awake = false;
    audit.save(&mut scene, "lifecycle", "dormant");
    scene.play.boss_awake = true;
    scene.boss_mut().mode = Mode::Intro(0);
    for tick in [0, 6, 14, 24, 40, 80, 128, 158, 171] {
        scene.until("intro", |b| b.mode == Mode::Intro(tick));
        audit.save(&mut scene, "lifecycle", &format!("intro_{tick:03}"));
    }
    scene.until("fight", |b| b.mode == Mode::Fight);
    audit.save(&mut scene, "lifecycle", "fight_after_intro");

    let mut scene = Scene::new(0);
    scene.action(|b, ctx| b.enter_rest(ctx));
    for tick in [0, 30, 60] {
        scene.until("checkpoint rest", |b| b.mode == Mode::Rest(tick));
        audit.save(&mut scene, "lifecycle", &format!("rest_{tick:03}"));
    }
    for tick in [0, 8, 40, 79] {
        scene.until("checkpoint rouse", |b| b.mode == Mode::Rouse(tick));
        audit.save(&mut scene, "lifecycle", &format!("rouse_{tick:03}"));
    }

    for phase in 0..3 {
        let group = format!("phase{}_motion", phase + 1);
        let mut scene = Scene::new(phase);
        audit.save(&mut scene, &group, &format!("p{}_fight_rest", phase + 1));
        for side in 0..2 {
            let mut scene = Scene::new(phase);
            scene.action(|b, ctx| {
                b.fists[side].slam(ctx.player, ctx.fx);
                b.attack = Some(Attack::Slam { side });
            });
            let states = [
                St::Seek(0),
                St::Seek(24),
                St::Lock(0),
                St::Lock(TEMPO[phase].lock - 1),
                St::Drop(0),
                St::Drop(3),
                St::Stuck(0, TEMPO[phase].stuck),
                St::Pull(0),
                St::Return(16),
            ];
            for (index, state) in states.into_iter().enumerate() {
                scene.until("slam pose", |b| b.fists[side].st == state);
                audit.save(
                    &mut scene,
                    &group,
                    &format!("p{}_slam{}_{}_{state:?}", phase + 1, side, index)
                        .replace(['(', ')', ',', ' '], "_")
                        .to_ascii_lowercase(),
                );
            }
        }
        let mut scene = Scene::new(phase);
        scene.action(|b, ctx| {
            b.fists[0].swat(ctx.fx);
            b.attack = Some(Attack::Swat { side: 0 });
        });
        for (index, state) in [
            St::SwatWind(0),
            St::SwatHold(0),
            St::SwatSweep(0),
            St::SwatSweep(7),
            St::SwatRest(0),
        ]
        .into_iter()
        .enumerate()
        {
            scene.until("sweep pose", |b| b.fists[0].st == state);
            audit.save(&mut scene, &group, &format!("p{}_sweep_{index}", phase + 1));
        }

        // The highest native hands and crystal seats must be reviewed in every
        // phase, including the last overhead tick before the falling pose.
        let mut scene = Scene::new(phase);
        scene.action(|b, ctx| {
            for fist in &mut b.fists {
                fist.raise(ctx.fx);
            }
            b.attack = Some(Attack::Double {
                landed: false,
                rings_left: TEMPO[phase].rings,
                ring_t: 0,
            });
        });
        for (index, state) in [
            St::Raise(0),
            St::Raise(18),
            St::Raise(35),
            St::Overhead(0),
            St::Overhead(TEMPO[phase].overhead / 2),
            St::Overhead(TEMPO[phase].overhead - 1),
            St::Slam2(0),
            St::Slam2(3),
            St::Stuck(0, TEMPO[phase].stuck2),
            St::Pull(0),
            St::Return(16),
            St::Rest,
        ]
        .into_iter()
        .enumerate()
        {
            scene.until("double raise/overhead", |b| {
                b.fists.iter().all(|fist| fist.st == state)
            });
            audit.save(
                &mut scene,
                &group,
                &format!("p{}_double_{index:02}", phase + 1),
            );
        }

        if phase > 0 {
            for side in 0..2 {
                let mut scene = Scene::new(phase);
                scene.action(|b, ctx| {
                    b.fists[side].sky();
                    ctx.fx.sound_vol("boss_telegraph", 0.7);
                    b.attack = Some(Attack::Rain { side, t: 0 });
                });
                for (index, state) in [
                    St::Sky(0),
                    St::Sky(13),
                    St::Sky(25),
                    St::Sky(26),
                    St::Sky(27),
                    St::Sky(40),
                    St::Sky(64),
                    St::Sky(71),
                    St::Return(0),
                    St::Return(16),
                    St::Rest,
                ]
                .into_iter()
                .enumerate()
                {
                    scene.until("rain sky/recovery", |b| b.fists[side].st == state);
                    audit.save(
                        &mut scene,
                        &group,
                        &format!("p{}_rain{side}_{index:02}", phase + 1),
                    );
                }
            }
        }

        let group = format!("phase{}_opening", phase + 1);
        let mut scene = Scene::new(phase);
        scene.action(|b, ctx| {
            if phase == 0 {
                for fist in &mut b.fists {
                    fist.crystal_hp = 0;
                }
            }
            b.start_stagger(ctx);
        });
        let end = 38 + TEMPO[phase].window;
        for tick in [
            0,
            4,
            8,
            12,
            16,
            17,
            18,
            20,
            24,
            30,
            37,
            38,
            50,
            end - 1,
            end + 6,
            end + 13,
            end + 14,
            end + 20,
            end + 28,
            end + 33,
        ] {
            scene.until("opening/recovery", |b| b.mode == Mode::Stagger(tick));
            audit.save(
                &mut scene,
                &group,
                &format!("p{}_opening_{tick:03}", phase + 1),
            );
        }
        scene.until("recovered fight", |b| b.mode == Mode::Fight);
        audit.save(
            &mut scene,
            &group,
            &format!("p{}_opening_recovered", phase + 1),
        );

        if phase < 2 {
            let mut scene = Scene::new(phase);
            scene.action(|b, ctx| b.start_stagger(ctx));
            scene.until("naturally settled phase transition", |b| {
                b.mode == Mode::Stagger(50)
            });
            scene.action(|b, ctx| {
                let _ = ctx;
                b.hp = b.floor_hp();
                b.body.hp = b.hp;
                b.chunk_done = true;
            });
            let group = format!("transition_{}_to_{}", phase + 1, phase + 2);
            for tick in [0, 6, 14, 26, 27, 50, 70, 110, 155] {
                scene.until("phase transition", |b| b.mode == Mode::Transition(tick));
                audit.save(&mut scene, &group, &format!("{group}_{tick:03}"));
            }
            scene.until("next phase fight", |b| b.mode == Mode::Fight);
            audit.save(&mut scene, &group, &format!("{group}_complete"));
        }
    }

    for (phase, twin) in [(1, false), (2, true)] {
        let mut scene = Scene::new(phase);
        if phase == 1 {
            scene.boss_mut().attack = Some(Attack::Summon { t: 0 });
            scene.until("three real pylons", |b| {
                b.pylons.iter().filter(|p| p.alive).count() == 3
            });
            audit.save(&mut scene, "prism", "p2_pylon_shield");
        }
        scene.boss_mut().attack = Some(Attack::Beam { t: 0, twin });
        scene.until("beam charge", |b| {
            b.beams.iter().any(|beam| beam.charging())
        });
        audit.save(&mut scene, "prism", &format!("p{}_beam_charge", phase + 1));
        scene.until("beam fire", |b| b.beams.iter().any(|beam| beam.firing()));
        audit.save(&mut scene, "prism", &format!("p{}_beam_fire", phase + 1));
        for _ in 0..24 {
            scene.tick();
        }
        audit.save(&mut scene, "prism", &format!("p{}_beam_sweep", phase + 1));
    }

    // Native one-pixel folds are deliberately isolated from effects and time changes.
    for phase in [0, 2] {
        for opening in [false, true] {
            let mut scene = Scene::new(phase);
            let group = format!(
                "fold_p{}_{}",
                phase + 1,
                if opening { "open" } else { "closed" }
            );
            for depth in 0..=pose::rig().fold_max {
                let b = scene.boss_mut();
                b.mode = Mode::Stagger(50);
                b.body_dy = depth as f32;
                b.head_dy = 2.0 * depth as f32 / kneel_depth();
                b.heart.out = if opening { 1.0 } else { 0.0 };
                for side in 0..2 {
                    let (x, y, _) = b.rest_pos(side);
                    let fist = &mut b.fists[side];
                    (fist.x, fist.y, fist.z) = (x, y + 9.0, 0.0);
                    fist.st = St::Limp(32);
                }
                audit.save_as(
                    &mut scene,
                    &group,
                    &format!("{group}_{depth:02}"),
                    false,
                    "synthetic-fold-pose",
                );
            }
        }
    }

    let mut scene = Scene::new(2);
    scene.action(|b, ctx| {
        b.hp = 0;
        b.body.hp = 0;
        b.start_dying(ctx);
    });
    for tick in [
        0, 14, 62, 100, 112, 113, 134, 135, 164, 194, 205, 206, 207, 216, 240, 262, 300, 335,
    ] {
        scene.until("death", |b| b.mode == Mode::Dying(tick));
        audit.save(&mut scene, "death", &format!("death_{tick:03}"));
        if scene.fx.flash > 0.3 {
            audit.save_as(
                &mut scene,
                "death",
                &format!("death_{tick:03}_unflashed"),
                false,
                "live-updates-diagnostic-no-screen-flash",
            );
        }
    }
    capture_game_cameras(&mut audit);
    audit.finish();
}

/// Independent native camera coverage. Static setups are declared fixtures;
/// movement, attack clocks, camera interpolation and rendering are production.
fn capture_game_cameras(audit: &mut Audit) {
    for (name, offset) in [
        ("approach", (0.0, 240.0)),
        ("front", (0.0, 72.0)),
        ("close_front", (0.0, 4.0)),
        ("close_front_left", (-32.0, 8.0)),
        ("side_right", (85.0, 4.0)),
        ("side_left", (-85.0, 4.0)),
        ("behind", (0.0, -32.0)),
        ("retreat", (0.0, 300.0)),
    ] {
        let mut scene = Scene::game_probe(0, offset);
        audit.save(&mut scene, "game_positions", &format!("game_{name}_idle"));
        for tick in 1..=48 {
            scene.tick_input(Raw {
                attack: true,
                ..Raw::default()
            });
            if [1, 12, 30, 48].contains(&tick) {
                audit.save(
                    &mut scene,
                    "game_positions",
                    &format!("game_{name}_hero_attack_{tick:02}"),
                );
            }
        }
        scene.tick();
        audit.save(
            &mut scene,
            "game_positions",
            &format!("game_{name}_hero_release"),
        );
    }

    for (name, offset, movement, ticks) in [
        ("approaching", (0.0, 240.0), (0.0, -1.0), 120),
        ("retreating", (85.0, 4.0), (0.0, 1.0), 180),
        ("passing_behind", (-90.0, -32.0), (1.0, 0.0), 120),
    ] {
        let mut scene = Scene::game_probe(0, offset);
        audit.save(&mut scene, "game_motion", &format!("game_{name}_000"));
        for tick in 1..=ticks {
            scene.tick_input(Raw {
                mv: movement,
                ..Raw::default()
            });
            if tick % 20 == 0 {
                audit.save(&mut scene, "game_motion", &format!("game_{name}_{tick:03}"));
            }
        }
    }

    for phase in [1, 2] {
        for (distance, offset) in [("close", (85.0, 4.0)), ("distant", (0.0, 220.0))] {
            let mut scene = Scene::game_probe(phase, offset);
            audit.save(
                &mut scene,
                "game_prism",
                &format!("game_p{}_beam_{distance}_idle", phase + 1),
            );
            scene.boss_mut().attack = Some(Attack::Beam {
                t: 0,
                twin: phase == 2,
            });
            scene.until("game camera beam charge", |b| {
                b.beams.iter().any(|beam| beam.charging())
            });
            audit.save(
                &mut scene,
                "game_prism",
                &format!("game_p{}_beam_{distance}_charge", phase + 1),
            );
            scene.until("game camera beam firing", |b| {
                b.beams.iter().any(|beam| beam.firing())
            });
            audit.save(
                &mut scene,
                "game_prism",
                &format!("game_p{}_beam_{distance}_fire", phase + 1),
            );
            for _ in 0..24 {
                scene.tick();
            }
            audit.save(
                &mut scene,
                "game_prism",
                &format!("game_p{}_beam_{distance}_sweep", phase + 1),
            );
        }

        let mut scene = Scene::game_probe(phase, (85.0, 4.0));
        scene.action(|b, ctx| {
            for fist in &mut b.fists {
                fist.raise(ctx.fx);
            }
            b.attack = Some(Attack::Double {
                landed: false,
                rings_left: TEMPO[phase].rings,
                ring_t: 0,
            });
        });
        for (name, state) in [
            ("raise", St::Raise(18)),
            ("overhead", St::Overhead(TEMPO[phase].overhead - 1)),
            ("fall", St::Slam2(3)),
            ("impact", St::Stuck(0, TEMPO[phase].stuck2)),
        ] {
            scene.until("game camera overhead", |b| b.fists[0].st == state);
            audit.save(
                &mut scene,
                "game_overhead",
                &format!("game_p{}_double_{name}", phase + 1),
            );
        }
    }

    // These routes preserve the ordinary entrance checkpoint, actual awakening,
    // actual damage/respawn and the living retreat. No boss/player/camera warps.
    for (name, god, script) in [
        ("retry", false, "R350 U260 W2200 R350 U260 W60"),
        ("retreat", true, "R350 U260 W60 D260 L350 W60 R350 U260 W60"),
    ] {
        let mut scene = Scene::gameplay(god);
        let mut tick = 0u32;
        let mut prior_awake = false;
        let mut prior_deaths = 0;
        audit.save(
            &mut scene,
            "game_routes",
            &format!("route_{name}_{tick:04}"),
        );
        for (raw, ticks) in crate::tools::parse_script(script) {
            for _ in 0..ticks {
                scene.tick_input(raw);
                tick += 1;
                let awakening = !prior_awake && scene.play.boss_awake;
                let retry = prior_deaths != scene.play.deaths;
                if tick % 120 == 0 || awakening || retry {
                    audit.save(
                        &mut scene,
                        "game_routes",
                        &format!("route_{name}_{tick:04}"),
                    );
                }
                prior_awake = scene.play.boss_awake;
                prior_deaths = scene.play.deaths;
            }
        }
        assert!(
            scene.play.boss_awake,
            "{name} route must reach the encounter"
        );
        if !god {
            assert!(
                scene.play.deaths > 0,
                "retry route must exercise an actual respawn"
            );
        }
        audit.save(&mut scene, "game_routes", &format!("route_{name}_complete"));
    }
}
