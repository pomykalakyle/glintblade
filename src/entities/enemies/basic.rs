//! Shared placeholder enemy behavior, used by each kind until its real AI is written.
//! Walks toward the player, telegraphs, lunges; draws from the kind's sprite sheet if it exists,
//! otherwise a colored blob.

use super::EnemyCtx;
use crate::combat::{Hit, Shape, Team};
use crate::entities::Body;
use crate::gfx::{Dir, DrawOpts, Frame, Rgb, sheet};

#[derive(Clone, Debug, Default)]
pub struct Basic {
    pub t: u32,
    /// 0 approach, 1 windup, 2 attack, 3 recover
    pub phase: u8,
    pub phase_t: u32,
    pub dir: (f32, f32),
    pub hit_id: u32,
    /// Visual facing has its own hysteresis; attack direction stays precise.
    pub visual_dir: Option<Dir>,
}

pub struct Params {
    pub sheet: &'static str,
    pub speed: f32,
    pub reach: f32,
    pub windup: u32,
    pub lunge: f32,
    pub color: Rgb,
    pub size: i32,
}

pub fn update(b: &mut Body, s: &mut Basic, ctx: &mut EnemyCtx, p: &Params) {
    s.t += 1;
    s.phase_t += 1;
    let (px, py) = ctx.player;
    let d = b.dist_to(px, py);
    let to = b.dir_to(px, py);
    if b.stun > 0 {
        b.vx = 0.0;
        b.vy = 0.0;
        b.integrate(ctx.zone);
        return;
    }
    match s.phase {
        0 => {
            b.face = to;
            b.vx = to.0 * p.speed;
            b.vy = to.1 * p.speed;
            if d < p.reach && ctx.player_alive {
                s.phase = 1;
                s.phase_t = 0;
                s.dir = to;
            }
        }
        1 => {
            b.vx = 0.0;
            b.vy = 0.0;
            if s.phase_t >= p.windup {
                s.phase = 2;
                s.phase_t = 0;
                s.hit_id = ctx.fx.hit_id();
            }
        }
        2 => {
            b.vx = s.dir.0 * p.lunge;
            b.vy = s.dir.1 * p.lunge;
            ctx.hits.push(Hit::new(Shape::Circle { x: b.x, y: b.y - 4.0, r: b.r + 3.0 }, Team::Enemy, 1, s.hit_id).dir(s.dir.0, s.dir.1));
            if s.phase_t >= 10 {
                s.phase = 3;
                s.phase_t = 0;
            }
        }
        _ => {
            b.vx *= 0.8;
            b.vy *= 0.8;
            if s.phase_t >= 40 {
                s.phase = 0;
                s.phase_t = 0;
            }
        }
    }
    b.integrate(ctx.zone);
    let previous = s.visual_dir.unwrap_or(Dir::from_vec(b.face.0, b.face.1));
    s.visual_dir = Some(previous.stable_from_vec(b.face.0, b.face.1));
}

pub fn anim_name(s: &Basic) -> &'static str {
    match s.phase {
        0 => "move",
        1 => "windup",
        2 => "attack",
        _ => "idle",
    }
}

pub fn draw(b: &Body, s: &Basic, f: &mut Frame, p: &Params) {
    f.shadow(f.sx(b.x), f.sy(b.y), p.size / 2 + 1, 2, 0.4);
    let dir = s.visual_dir.unwrap_or_else(|| Dir::from_vec(b.face.0, b.face.1));
    let name = if b.stun > 0 { "hurt" } else { anim_name(s) };
    if let Some(sh) = sheet(p.sheet)
        && let Some((a, flip)) = sh.dir_anim(name, dir).or_else(|| sh.dir_anim("idle", dir))
    {
        let cell = sh.cell(a, a.frame_at(s.phase_t, true));
        f.sprite_w(cell, sh.anchor_of(a), b.x, b.y - b.z, &DrawOpts { flip, flash: b.flash, ..Default::default() });
        return;
    }
    let shake = if s.phase == 1 { (s.phase_t % 2) as i32 } else { 0 };
    let (sx, sy) = (f.sx(b.x) + shake, f.sy(b.y - b.z) - p.size / 2);
    let c = if b.flash > 0.0 { p.color.lerp(Rgb::WHITE, b.flash) } else { p.color };
    f.circle(sx, sy, p.size / 2, c);
    f.put_glow(sx - 1, sy - 1, Rgb::WHITE);
    f.put_glow(sx + 1, sy - 1, Rgb::WHITE);
}

/// Burst the current frame into pixels.
pub fn burst(b: &Body, s: &Basic, ctx: &mut EnemyCtx, p: &Params) {
    let dir = s.visual_dir.unwrap_or_else(|| Dir::from_vec(b.face.0, b.face.1));
    if let Some(sh) = sheet(p.sheet)
        && let Some((a, flip)) = sh.dir_anim("hurt", dir).or_else(|| sh.dir_anim("idle", dir))
    {
        let cell = sh.cell(a, a.frame_at(s.phase_t, true));
        let fx = &mut *ctx.fx;
        fx.particles.burst_cell(&mut fx.rng, cell, sh.anchor_of(a), flip, b.x, b.y, 1.8, 400.0);
        return;
    }
    let fx = &mut *ctx.fx;
    for _ in 0..(p.size * p.size) {
        let (dx, dy) = fx.rng.dir();
        let sp = fx.rng.range(0.5, 2.0);
        fx.particles.add(
            crate::fx::Particle::new(b.x, b.y)
                .z(fx.rng.range(0.0, p.size as f32))
                .vel(dx * sp, dy * sp, fx.rng.range(0.5, 2.5))
                .color(p.color)
                .life(300.0)
                .settle(true),
        );
    }
}
