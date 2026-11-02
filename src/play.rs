//! One zone being played: the player, enemies, projectiles, the Warden, the shard and the gate.
//! `update` runs one 60 Hz tick; `render` draws the world (the caller draws UI and finishes).

use crate::camera::Camera;
use crate::combat::{Hit, Team};
use crate::entities::boss::Boss;
use crate::entities::player::PlayerCtx;
use crate::entities::{Enemy, EnemyCtx, Player, Projectile};
use crate::fx::{Fx, Particle};
use crate::gfx::{DrawOpts, Frame, Rgb, rgb, sheet};
use crate::input::Input;
use crate::regions;
use crate::view::render_world_focused;
use crate::world::{Region, RegionCtx, Zone, ZoneId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gate {
    Closed,
    Opening(u32),
    Open,
}

/// A small floating heal pickup dropped by urns and grass.
#[derive(Clone, Copy, Debug)]
pub struct Mote {
    pub x: f32,
    pub y: f32,
    pub t: u32,
    pub alive: bool,
}

pub struct PlayOpts {
    pub god: bool,
    pub peaceful: bool,
}

pub struct Play {
    pub id: ZoneId,
    pub zone: Zone,
    pub region: Box<dyn Region>,
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub boss: Option<Boss>,
    pub cam: Camera,
    /// Bypass zone bounds and shake for the title postcard only.
    pub free_camera: bool,
    pub t: u64,
    pub shard_taken: bool,
    pub gate: Gate,
    pub motes: Vec<Mote>,
    /// Ticks since arriving (banner timing).
    pub arrive_t: u32,
    /// Player stepped through the open gate.
    pub exit: bool,
    /// The Warden's death sequence finished.
    pub victory: bool,
    pub checkpoint: (f32, f32),
    pub god: bool,
    /// Boss fight has started (player entered the arena).
    pub boss_awake: bool,
    pub deaths: u32,
    /// Don't draw the player (title screen backdrop).
    pub hide_player: bool,
}

const ID_PLAYER: u32 = 0;
const ID_ENEMY: u32 = 1_000;
const ID_BOSS: u32 = 10_000;
const ID_PROJ: u32 = 20_000;
const ID_SHARD: u32 = 30_000;
const ID_GATE: u32 = 30_001;
const ID_MOTE: u32 = 40_000;

impl Play {
    pub fn new(id: ZoneId, opts: &PlayOpts) -> Play {
        let (zone, region) = regions::build(id);
        let (px, py) = zone.player_start;
        let enemies = if opts.peaceful { Vec::new() } else { zone.enemies.iter().map(|&(k, x, y)| Enemy::spawn(k, x, y)).collect() };
        let boss = if zone.boss && !opts.peaceful { zone.boss_pos.map(|(x, y)| Boss::new(x, y)) } else { None };
        let gate = if zone.shard.is_some() { Gate::Closed } else { Gate::Open };
        Play {
            id,
            region,
            player: Player::new(px, py),
            enemies,
            projectiles: Vec::new(),
            boss,
            cam: Camera::new(px, py),
            free_camera: false,
            t: 0,
            shard_taken: false,
            gate,
            motes: Vec::new(),
            arrive_t: 0,
            exit: false,
            victory: false,
            checkpoint: (px, py),
            god: opts.god,
            boss_awake: false,
            deaths: 0,
            hide_player: false,
            zone,
        }
    }

    pub fn update(&mut self, input: &Input, fx: &mut Fx) {
        self.arrive_t += 1;
        let music = if self.boss_awake { "warden".to_string() } else { self.zone.music.clone() };
        fx.music(&music);
        // Keep taps that happen entirely during hit-stop.
        self.player.buffer_input(input);
        if fx.hitstop > 0 {
            fx.hitstop -= 1;
            return;
        }
        self.t += 1;
        let t = self.t;

        // --- player
        let mut player_hits: Vec<Hit> = Vec::new();
        {
            let mut ctx = PlayerCtx { zone: &self.zone, fx, hits: &mut player_hits };
            self.player.update(input, &mut ctx);
        }
        if self.god {
            self.player.body.hp = self.player.body.max_hp;
        }

        // --- player attacks vs the world (props, region, projectiles)
        for h in &player_hits {
            self.region.slash(&mut self.zone, h, fx);
            for p in self.zone.props.iter_mut() {
                if p.breakable && p.alive {
                    let (cx, cy) = (p.x, p.y - 5.0);
                    if h.shape.touches(cx, cy, 5.0) && p.hit(h.dmg, fx, t) {
                        if fx.rng.chance(0.35) {
                            self.motes.push(Mote { x: p.x, y: p.y, t: 0, alive: true });
                        }
                        fx.sound("pot_break");
                    }
                }
            }
            for pr in self.projectiles.iter_mut() {
                if pr.alive && pr.team == Team::Enemy && h.shape.touches(pr.x, pr.y, pr.r + 2.0) {
                    pr.deflect(fx);
                }
            }
        }

        // --- enemies and boss
        let mut enemy_hits: Vec<Hit> = Vec::new();
        let player_pos = (self.player.body.x, self.player.body.y);
        let player_alive = self.player.alive();
        {
            let mut ctx = EnemyCtx { player: player_pos, player_alive, zone: &self.zone, fx, hits: &mut enemy_hits, projectiles: &mut self.projectiles, t };
            for e in self.enemies.iter_mut() {
                e.update(&mut ctx);
            }
            if let Some(b) = self.boss.as_mut() {
                if !self.boss_awake && b.body.dist_to(player_pos.0, player_pos.1) < 150.0 {
                    self.boss_awake = true;
                    ctx.fx.sound("boss_roar");
                    ctx.fx.shake(0.6);
                }
                if self.boss_awake {
                    b.update(&mut ctx);
                }
            }
            // player hits land on enemies and the boss
            for h in &player_hits {
                for e in self.enemies.iter_mut() {
                    if e.body.alive && e.body.touched_by(h) {
                        let was_alive = e.body.alive;
                        e.on_hit(h, &mut ctx);
                        if was_alive && !e.body.alive && ctx.fx.rng.chance(0.3) {
                            self.motes.push(Mote { x: e.body.x, y: e.body.y, t: 0, alive: true });
                        }
                    }
                }
                if let Some(b) = self.boss.as_mut() {
                    for (x, y, r, part) in b.hurtboxes() {
                        if h.shape.touches(x, y, r) {
                            b.on_hit(h, part, &mut ctx);
                            break;
                        }
                    }
                }
            }
        }
        // separation so enemies don't stack
        let n = self.enemies.len();
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = self.enemies.split_at_mut(j);
                let (ea, eb) = (&mut a[i].body, &mut b[0].body);
                if !ea.alive || !eb.alive {
                    continue;
                }
                let (dx, dy) = (eb.x - ea.x, eb.y - ea.y);
                let d = (dx * dx + dy * dy).sqrt();
                let min = ea.r + eb.r;
                if d < min && d > 0.01 {
                    let push = (min - d) * 0.25;
                    ea.kx -= dx / d * push;
                    ea.ky -= dy / d * push;
                    eb.kx += dx / d * push;
                    eb.ky += dy / d * push;
                }
            }
        }

        // --- projectiles
        for pr in self.projectiles.iter_mut() {
            if let Some(h) = pr.update(&self.zone, fx, player_pos) {
                enemy_hits.push(h);
            }
        }
        self.projectiles.retain(|p| p.alive);

        // --- enemy hits land on the player
        for h in &enemy_hits {
            if h.team == Team::Enemy && self.player.body.touched_by(h) && self.player.on_hit(h, fx) {
                for pr in self.projectiles.iter_mut() {
                    if pr.id == h.id {
                        pr.alive = false;
                    }
                }
            }
        }

        // --- respawn
        if self.player.wants_respawn {
            self.deaths += 1;
            let (x, y) = self.checkpoint;
            self.player.respawn(x, y);
            self.cam.snap(x, y);
            fx.particles.list.clear();
        }

        // --- shard and gate
        if let Some((sx, sy)) = self.zone.shard
            && !self.shard_taken
            && self.player.alive()
            && self.player.body.dist_to(sx, sy) < 12.0
        {
            self.shard_taken = true;
            self.gate = Gate::Opening(0);
            fx.sound("shard_pickup");
            fx.flash(0.35);
            fx.shake(0.3);
            for i in 0..60 {
                let a = i as f32 / 60.0 * std::f32::consts::TAU;
                let sp = fx.rng.range(1.0, 3.2);
                fx.particles.add(
                    Particle::new(sx, sy)
                        .z(10.0)
                        .vel(a.cos() * sp, a.sin() * sp * 0.7, fx.rng.range(0.5, 2.5))
                        .color(Rgb::WHITE)
                        .fade_to(rgb(0xbaf5ec))
                        .glow(1.0)
                        .life(fx.rng.range(30.0, 60.0))
                        .grav(0.05)
                        .drag(0.93)
                        .streak(true),
                );
            }
            self.player.heal(self.player.body.max_hp);
        }
        match self.gate {
            Gate::Opening(n) => {
                if n == 30 {
                    fx.sound("gate_open");
                    fx.shake(0.25);
                }
                self.gate = if n > 90 { Gate::Open } else { Gate::Opening(n + 1) };
            }
            Gate::Open => {
                if let Some((gx, gy)) = self.zone.gate
                    && self.player.alive()
                    && (self.player.body.x - gx).abs() < 12.0
                    && (self.player.body.y - gy).abs() < 10.0
                {
                    self.exit = true;
                }
            }
            Gate::Closed => {}
        }
        if let Some(b) = &self.boss
            && b.finished()
        {
            self.victory = true;
        }

        // --- heal motes
        for m in self.motes.iter_mut() {
            m.t += 1;
            let d = self.player.body.dist_to(m.x, m.y);
            if m.t > 30 && d < 40.0 {
                let (dx, dy) = self.player.body.dir_to(m.x, m.y);
                let pull = (1.0 - d / 40.0) * 3.0 + 0.4;
                m.x -= dx * pull;
                m.y -= dy * pull;
            }
            if m.t > 30 && d < 6.0 && self.player.body.hp < self.player.body.max_hp {
                m.alive = false;
                self.player.heal(1);
                fx.sound("heal");
            }
            if m.t > 1200 {
                m.alive = false;
            }
            if m.alive && m.t % 4 == 0 {
                fx.particles.add(Particle::new(m.x, m.y).z(6.0 + (m.t as f32 * 0.1).sin() * 2.0).vel(0.0, 0.0, 0.25).color(rgb(0xd8b0ab)).grav(0.0).life(14.0));
            }
        }
        self.motes.retain(|m| m.alive);

        // --- region life, particles, props
        let mut movers = vec![(self.player.body.x, self.player.body.y, (self.player.body.vx.powi(2) + self.player.body.vy.powi(2)).sqrt())];
        for e in self.enemies.iter().filter(|e| e.body.alive) {
            movers.push((e.body.x, e.body.y, (e.body.vx.powi(2) + e.body.vy.powi(2)).sqrt()));
        }
        let cam = self.cam.view(self.zone.pw, self.zone.ph, 0.0, t);
        let rctx = RegionCtx { t, player: player_pos, movers: &movers, cam };
        self.region.update(&mut self.zone, &rctx, fx);
        fx.particles.update(&self.zone);
        for p in self.zone.props.iter_mut() {
            p.tick();
        }
        let speed = (self.player.body.vx.powi(2) + self.player.body.vy.powi(2)).sqrt();
        // Nearby, frame both Glint and the Warden (its head stands far above its feet).
        let (mut tx, mut ty) = (self.player.body.x, self.player.body.y);
        let mut boss_top = None;
        if self.boss_awake
            && let Some(b) = &self.boss
            && !b.finished()
        {
            // Ease back to Glint on the retreat/approach; awakening persists across checkpoint retries.
            let near = ((300.0 - b.body.dist_to(tx, ty)) / 120.0).clamp(0.0, 1.0);
            tx += (b.body.x - tx) * 0.4 * near;
            if let Some(top) = b.framing_top() {
                // Center the actual assembly and Glint, compensating follow's eight-pixel lift.
                let bottom = self.player.body.y.max(b.body.y + 10.0);
                ty += ((top + bottom) * 0.5 + 8.0 - ty) * near;
                if near >= 0.99 { boss_top = Some(top); }
            }
        }
        self.cam.follow(tx, ty, self.player.body.face, speed);
        if let Some(top) = boss_top {
            // Correct look-ahead/smoothing only when the assembly fits above Glint's safe anchor.
            let highest_center = top + crate::gfx::H as f32 / 2.0 - 16.0;
            let lowest_center = self.player.body.y - self.player.body.z - crate::gfx::H as f32 / 2.0 + 32.0;
            if lowest_center <= highest_center {
                self.cam.y = self.cam.y.min(highest_center);
            }
        }
        if self.boss_awake {
            // Bound the smoothed offset too, including look-ahead and the airborne respawn.
            self.cam.keep_visible(self.player.body.x, self.player.body.y - self.player.body.z);
            if self.player.alive() {
                self.cam.prepare_weapon_margin(self.player.body.y - self.player.body.z);
            }
            if self.player.alive() && let Some(b) = &self.boss && b.body.dist_to(self.player.body.x, self.player.body.y) < 300.0 {
                let player_y = self.player.body.y - self.player.body.z;
                // Establish the full fitting interval during the lead-in, not at beam onset.
                if let Some(top) = boss_top
                    && self.player.body.y - self.player.body.z - top <= crate::gfx::H as f32 - 48.0
                {
                    self.cam.keep_fitting_assembly(top, player_y);
                }
                self.cam.frame_attack_sources((self.player.body.x, player_y), b.attack_sources());
            }
        }
    }

    /// Select the same camera path for ambient region updates and world rendering.
    fn camera_view(&self, trauma: f32, t: u64) -> (i32, i32) {
        if self.free_camera { self.cam.view_free() } else { self.cam.view(self.zone.pw, self.zone.ph, trauma, t) }
    }

    /// Only the region's ambient life (title screen backdrop): no player, enemies or camera follow.
    pub fn update_ambient(&mut self, fx: &mut Fx) {
        self.t += 1;
        let cam = self.camera_view(0.0, self.t);
        let center = (self.cam.x, self.cam.y + 40.0);
        let movers = [(center.0, center.1, 0.0)];
        let rctx = RegionCtx { t: self.t, player: center, movers: &movers, cam };
        self.region.update(&mut self.zone, &rctx, fx);
        fx.particles.update(&self.zone);
        for p in self.zone.props.iter_mut() {
            p.tick();
        }
    }

    pub fn render(&mut self, f: &mut Frame, fx: &Fx) {
        let (cx, cy) = self.camera_view(fx.trauma, fx.t);
        f.cam_x = cx;
        f.cam_y = cy;
        // Shadow casters for the lighting pass (feet position, half-width, height).
        f.casters.clear();
        if !self.hide_player && self.player.alive() {
            f.casters.push(crate::gfx::Caster { x: self.player.body.x, y: self.player.body.y, half_w: 4.0, height: 16.0 });
        }
        for e in self.enemies.iter().filter(|e| e.body.alive) {
            f.casters.push(crate::gfx::Caster { x: e.body.x, y: e.body.y, half_w: e.body.r, height: e.body.r * 2.5 });
        }
        if let Some(b) = &self.boss && b.solid().is_some() {
            f.casters.push(crate::gfx::Caster { x: b.body.x, y: b.body.y, half_w: 28.0, height: 110.0 });
        }
        let mut ext: Vec<(f32, u32)> = Vec::new();
        if !self.hide_player {
            ext.push((self.player.body.y, ID_PLAYER));
        }
        for (i, e) in self.enemies.iter().enumerate() {
            if e.body.alive {
                ext.push((e.body.y, ID_ENEMY + i as u32));
            }
        }
        if let Some(b) = &self.boss {
            ext.push((b.sort_y(), ID_BOSS));
        }
        for (i, p) in self.projectiles.iter().enumerate() {
            ext.push((p.y, ID_PROJ + i as u32));
        }
        if let Some((_, sy)) = self.zone.shard
            && !self.shard_taken
        {
            ext.push((sy, ID_SHARD));
        }
        if let Some((_, gy)) = self.zone.gate {
            ext.push((gy, ID_GATE));
        }
        for (i, m) in self.motes.iter().enumerate() {
            ext.push((m.y, ID_MOTE + i as u32));
        }
        let t = self.t;
        let me = &*self;
        let mut draw = |f: &mut Frame, id: u32| match id {
            ID_PLAYER => me.player.draw(f),
            ID_BOSS => {
                if let Some(b) = &me.boss {
                    b.draw(f)
                }
            }
            ID_SHARD => me.draw_shard(f, t),
            ID_GATE => me.draw_gate(f, t),
            i if i >= ID_MOTE => {
                let m = me.motes[(i - ID_MOTE) as usize];
                let (sx, sy) = (f.sx(m.x), f.sy(m.y - 6.0 - (m.t as f32 * 0.1).sin() * 2.0));
                f.shadow(f.sx(m.x), f.sy(m.y), 2, 1, 0.3);
                f.put_glow(sx, sy, rgb(0xdce0ce));
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    f.put(sx + dx, sy + dy, rgb(0xb98087));
                }
            }
            i if i >= ID_PROJ => me.projectiles[(i - ID_PROJ) as usize].draw(f),
            i if i >= ID_ENEMY => me.enemies[(i - ID_ENEMY) as usize].draw(f),
            _ => {}
        };
        let sprite_focus = if self.hide_player { None } else {
            self.player.current_cell().map(|(cell, anchor, flip)| crate::gfx::sprite::SpriteFocus {
                cell, anchor, flip, x: self.player.body.x, y: self.player.body.y - self.player.body.z,
                ground_y: self.player.body.y,
            })
        };
        render_world_focused(&self.zone, self.region.as_ref(), fx, f, t, &ext, &mut draw, Some((self.player.body.x, self.player.body.y)), sprite_focus);
        if !self.hide_player && self.player.alive() && self.boss_awake && let Some(b) = &self.boss {
            b.draw_attack_cues(f);
        }
    }

    fn draw_shard(&self, f: &mut Frame, t: u64) {
        let Some((x, y)) = self.zone.shard else { return };
        let bob = ((t as f32) * 0.06).sin() * 2.0;
        f.shadow(f.sx(x), f.sy(y), 4, 1, 0.35);
        if f.anim_w("shard", "spin", t as u32, x, y - 8.0 + bob, &DrawOpts::default()) || f.anim_w("shard", "idle", t as u32, x, y - 8.0 + bob, &DrawOpts::default()) {
            return;
        }
        let (sx, sy) = (f.sx(x), f.sy(y - 12.0 + bob));
        let w = ((t as f32 * 0.08).sin().abs() * 3.0).round() as i32;
        for dy in -5i32..=5 {
            let half = (5 - dy.abs()) * w / 5;
            for dx in -half..=half {
                let c = if dx == 0 { rgb(0xedf9ef) } else { rgb(0xbaf5ec) };
                f.put_glow(sx + dx, sy + dy, c);
            }
        }
    }

    fn draw_gate(&self, f: &mut Frame, t: u64) {
        let Some((x, y)) = self.zone.gate else { return };
        let anim = match self.gate {
            Gate::Closed => "closed",
            Gate::Opening(_) => "opening",
            Gate::Open => "open",
        };
        let ticks = match self.gate {
            Gate::Opening(n) => n,
            _ => t as u32,
        };
        if sheet("gate").and_then(|s| s.anim(anim)).is_some() {
            f.anim_w("gate", anim, ticks, x, y, &DrawOpts::default());
            return;
        }
        let (sx, sy) = (f.sx(x), f.sy(y));
        let stone = rgb(0xc2c1b3);
        f.rect(sx - 14, sy - 30, 4, 30, stone);
        f.rect(sx + 10, sy - 30, 4, 30, stone);
        f.rect(sx - 14, sy - 34, 28, 4, stone);
        if self.gate != Gate::Closed {
            for dy in 0..26 {
                for dx in -9..10 {
                    let w = ((dy as f32 * 0.4 + t as f32 * 0.1).sin() * 0.5 + 0.5) * 0.6 + 0.2;
                    f.put_glow(sx + dx, sy - 29 + dy, rgb(0xb7add0).scale(w));
                }
            }
        }
    }

    /// Lights for the shard and gate (call before render; they're per-frame lights).
    pub fn add_lights(&self, fx: &mut Fx) {
        if let Some((x, y)) = self.zone.shard
            && !self.shard_taken
        {
            fx.light(x, y - 10.0, 28.0, rgb(0xbaf5ec), 0.6);
        }
        if let Some((x, y)) = self.zone.gate
            && self.gate != Gate::Closed
        {
            fx.light(x, y - 16.0, 28.0, rgb(0xe2d8e9), 0.6);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Options, Scene};
    use crate::gfx::{H, W};
    use crate::tools::parse_script;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Reach legal ground behind the foreground tree using ordinary movement. The old
    /// direct rightward leg stopped at the entry pier once its native base became solid.
    #[test]
    fn bloomwild_foreground_wood_reveals_actual_hero_pixels_at_motion_probe() {
        let mut game = Game::new(Options { start: Some(ZoneId::Bloomwild), ..Options::default() });
        let mut input = Input::default();
        let mut tick = 0;
        'route: for (raw, ticks) in parse_script("W360 U82 R59 D20") {
            for _ in 0..ticks {
                input.step(raw); game.tick(&input); tick += 1;
                if tick == 521 { break 'route; }
            }
        }
        let Scene::Play(p) = &game.scene else { panic!("probe must remain in Bloomwild") };
        assert!(!p.zone.blocked(p.player.body.x, p.player.body.y, p.player.body.r),
            "foreground visibility must be checked from legal ground");
        let (cell, anchor, flip) = p.player.current_cell().unwrap();
        let focus = crate::gfx::sprite::SpriteFocus { cell, anchor, flip, x: p.player.body.x, y: p.player.body.y - p.player.body.z, ground_y: p.player.body.y };
        let (cx, cy) = p.cam.view(p.zone.pw, p.zone.ph, 0.0, tick);
        let mut opaque = Frame::new(); let mut revealed = Frame::new();
        for f in [&mut opaque, &mut revealed] {
            f.cam_x = cx; f.cam_y = cy; f.begin(Rgb::BLACK, Rgb::WHITE); p.player.draw(f);
        }
        for tree in p.zone.props.iter().filter(|tree| tree.alive && tree.sheet == "bloomwild_trees" && tree.sort_y() > p.player.body.y) {
            tree.draw(&mut opaque, tick, 1.0);
            tree.draw_revealed(&mut revealed, tick, 1.0, Some(focus));
        }
        let mut hidden = 0; let mut recovered = 0;
        let ox = p.player.body.x.floor() as i32 - if flip { cell.w - 1 - anchor.0 } else { anchor.0 };
        let oy = focus.y.floor() as i32 - anchor.1;
        for y in 0..cell.h { for x in 0..cell.w {
            if !cell.at(x, y).a { continue; }
            let wx = ox + if flip { cell.w - 1 - x } else { x };
            let (sx, sy) = (wx - cx, oy + y - cy);
            if sx < 0 || sy < 0 || sx >= W as i32 || sy >= H as i32 { continue; }
            let i = sy as usize * W + sx as usize;
            if !opaque.actor_pixels[i] { hidden += 1; recovered += usize::from(revealed.actor_pixels[i]); }
        }}
        assert!(hidden >= 8, "the fixture must actually exercise foreground wood: {hidden} hidden pixels");
        assert!(recovered * 3 >= hidden * 2, "Glint must retain >=2/3 of the previously hidden body pixels: {recovered}/{hidden}");
        for y in 0..H { for x in 0..W {
            if !focus.covers(x as i32 + cx, y as i32 + cy) {
                assert_eq!(opaque.albedo[y * W + x], revealed.albedo[y * W + x], "roots/scenery outside the actual pose must not fade");
            }
        }}
    }

    /// Observe the actual ambient RegionCtx and rendered origin through both camera modes.
    #[test]
    fn ambient_and_render_share_camera_mode() {
        struct CameraProbe(Rc<Cell<(i32, i32)>>);
        impl Region for CameraProbe {
            /// Record the camera coordinates delivered to the region's ambient update.
            fn update(&mut self, _zone: &mut Zone, ctx: &RegionCtx, _fx: &mut Fx) {
                self.0.set(ctx.cam);
            }
        }
        let mut play = Play::new(ZoneId::Bloomwild, &PlayOpts { god: true, peaceful: true });
        assert!(!play.free_camera);
        let observed = Rc::new(Cell::new((0, 0)));
        play.region = Box::new(CameraProbe(observed.clone()));
        play.cam.snap(-10.5, -30.5);
        let mut fx = Fx::new(7);
        fx.trauma = 0.8;
        fx.t = 600;
        let mut frame = Frame::new();
        for free in [false, true] {
            play.free_camera = free;
            play.update_ambient(&mut fx);
            let ambient = if free { play.cam.view_free() } else { play.cam.view(play.zone.pw, play.zone.ph, 0.0, play.t) };
            let rendered = if free { play.cam.view_free() } else { play.cam.view(play.zone.pw, play.zone.ph, fx.trauma, fx.t) };
            assert_eq!(observed.get(), ambient);
            play.render(&mut frame, &fx);
            assert_eq!((frame.cam_x, frame.cam_y), rendered);
        }
    }

    /// Keep Glint visible on the normal Summit retry, return approach and a living arena retreat.
    #[test]
    fn summit_camera_keeps_player_visible_on_retry_and_retreat() {
        for (god, script) in [
            (false, "R350 U260 W2200 R350 U260 W60"),
            (true, "R350 U260 W60 D260 L350 W60 R350 U260 W60"),
        ] {
            let mut game = Game::new(Options { start: Some(ZoneId::Summit), god, ..Options::default() });
            let mut input = Input::default();
            let mut saw_retry = false;
            let mut saw_retreat = false;
            for (raw, ticks) in parse_script(script) {
                for _ in 0..ticks {
                    input.step(raw);
                    game.tick(&input);
                    game.fx.sounds.clear();
                    game.fx.lights.clear();
                    let Scene::Play(p) = &game.scene else { panic!("Summit route left the play scene") };
                    assert_eq!(p.checkpoint, p.zone.player_start, "the normal entrance checkpoint must stay in use");
                    if !p.boss_awake || !p.player.alive() {
                        continue;
                    }
                    let (cx, cy) = p.cam.view(p.zone.pw, p.zone.ph, game.fx.trauma, game.fx.t);
                    let (sx, sy) = (p.player.body.x - cx as f32, p.player.body.y - p.player.body.z - cy as f32);
                    assert!((24.0..=W as f32 - 24.0).contains(&sx) && (36.0..=H as f32 - 24.0).contains(&sy),
                        "god={god}, tick {}: Glint anchor ({sx}, {sy}) outside usable view, world ({}, {}), deaths {}",
                        game.t, p.player.body.x, p.player.body.y, p.deaths);
                    let b = p.boss.as_ref().unwrap();
                    if b.body.dist_to(p.player.body.x, p.player.body.y) <= 180.0
                        && let Some(top) = b.framing_top()
                        && p.player.body.y - p.player.body.z - top <= H as f32 - 48.0
                    {
                        assert!(top - cy as f32 >= 10.0, "tick {}: assembly fits but its top is clipped", game.t);
                    }
                    if (!god && game.t == 2810) || (god && game.t == 1340) {
                        assert!((sx - W as f32 / 2.0).abs() < 32.0 && (sy - H as f32 / 2.0).abs() < 32.0,
                            "distant Warden must release the landing/bridge framing");
                    }
                    saw_retry |= p.deaths > 0 && p.player.body.dist_to(p.checkpoint.0, p.checkpoint.1) < 2.0;
                    saw_retreat |= god && game.t > 670 && p.player.body.y > 700.0;
                    if game.t == 600 {
                        let b = p.boss.as_ref().unwrap();
                        assert!(b.body.y - 120.0 - cy as f32 > -16.0, "nearby framing must retain the Warden's upper silhouette");
                    }
                }
            }
            let Scene::Play(p) = &game.scene else { unreachable!() };
            assert!(p.boss_awake, "the route must enter the actual encounter");
            assert!(p.player.body.y < 500.0, "the route must return from the entrance to the arena");
            if god {
                assert!(saw_retreat, "the living retreat must leave the arena");
                assert_eq!(p.deaths, 0);
            } else {
                assert!(saw_retry, "the normal-damage route must respawn at the entrance");
            }
        }
    }
}
