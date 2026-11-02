//! Wisp (Glasswater Hollow): a little pale iris flame spirit with a chalk-bright core.
//!
//! It floats at a distance and circles Glint, keeping out of sword reach (it darts away if you
//! close in, and flips direction when it bumps a wall or gets hit). Every few seconds it stops,
//! rises a little and gathers itself: its core brightens and sparks gather along its locked aim, then
//! it spits a slow glowing bolt (sometimes a fan of three) and kicks back from the recoil.
//! Bolts are slow enough to walk around and can be cut out of the air.
//!
//! It lights the dark cave around itself with a flickering pale iris light, trails a curling
//! tail, gutters when struck, and dies in a puff of embers that drift upward and fade, leaving
//! a few ash flecks on the floor.

use super::EnemyCtx;
use crate::combat::Hit;
use crate::entities::{Body, Projectile};
use crate::fx::Particle;
use crate::gfx::{Cell, Dir, DrawOpts, Frame, Rgb, rgb, sheet};
use crate::util::{hash2, norm};
use std::f32::consts::TAU;
use std::sync::OnceLock;

const SHEET: &str = "wisp";
const BOLT_SHEET: &str = "wisp_bolt";
const HP: i32 = 3;
const R: f32 = 4.5;
/// Hover height of the flame's base above its shadow.
const HOVER: f32 = 7.0;
const ORBIT_SPEED: f32 = 0.95;
const FLEE_SPEED: f32 = 1.5;
/// Closer than this and it darts away.
const PERSONAL: f32 = 44.0;
const SIGHT: f32 = 190.0;
const WINDUP: u32 = 38;
const RECOIL: u32 = 32;
/// The final fifth of a second is committed, so walking sideways avoids the shot.
const AIM_LOCK: u32 = 12;
const BOLT_SPEED: f32 = 1.2;
const TAIL: usize = 8;

// Glasswater wisp flame ramp (assets/concept/palettes_v2.json).
const FLAME: [Rgb; 5] = [rgb(0x4d4866), rgb(0x796c91), rgb(0xa195b8), rgb(0xcdc3db), rgb(0xe2d8e9)];
const ASH: Rgb = rgb(0x424956);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Drift,
    Windup,
    Recoil,
}

#[derive(Clone, Debug)]
pub struct State {
    pub mode: Mode,
    pub mode_t: u32,
    pub t: u32,
    /// Orbit direction: +1 counter-clockwise on screen, -1 clockwise.
    pub spin: f32,
    pub radius: f32,
    pub cooldown: u32,
    /// Ticks before it may flip orbit direction again.
    pub flip_cd: u32,
    pub volley: u32,
    /// Aim locked at the end of the windup.
    pub aim: (f32, f32),
    /// Past positions (x, y, hover) for the curling tail, newest first.
    pub trail: [(f32, f32, f32); TAIL],
    seed: u32,
    appearance: u8,
}

impl State {
    /// Enter a behavior and restart its pose clock.
    fn set(&mut self, m: Mode) {
        self.mode = m;
        self.mode_t = 0;
    }
    /// Return stable variation from this wisp spawn.
    fn vary(&self, k: u32) -> f32 {
        hash2(self.seed as i32, k as i32, 91)
    }
}

/// Height of the flame's base above the ground at tick `t` (bob, plus the windup rise).
fn hover(s: &State, t: u64) -> f32 {
    let bob = ((t as f32 + s.seed as f32 * 13.0) * 0.07).sin() * 1.5;
    let rise = if s.mode == Mode::Windup { (s.mode_t as f32 / 12.0).min(1.0) * 3.0 } else { 0.0 };
    HOVER + bob + rise
}

/// Spawn a floating wisp with staggered orbit and shot timing.
pub fn spawn(x: f32, y: f32) -> (Body, State) {
    let seed = (x as i32 as u32).wrapping_mul(17).wrapping_add((y as i32 as u32).wrapping_mul(5));
    let mut s = State {
        mode: Mode::Drift,
        mode_t: 0,
        t: 0,
        spin: 1.0,
        radius: 0.0,
        cooldown: 0,
        flip_cd: 0,
        volley: 0,
        aim: (0.0, 1.0),
        trail: [(x, y, HOVER); TAIL],
        seed,
        appearance: super::variation::appearance(x, y),
    };
    s.spin = if s.vary(1) < 0.5 { -1.0 } else { 1.0 };
    s.radius = 60.0 + s.vary(2) * 20.0;
    s.cooldown = 70 + (s.vary(3) * 90.0) as u32;
    (Body::new(x, y, R, HP), s)
}

/// Advance orbiting, the committed firing cue, or its recoil.
pub fn update(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    s.t += 1;
    s.mode_t += 1;
    s.cooldown = s.cooldown.saturating_sub(1);
    s.flip_cd = s.flip_cd.saturating_sub(1);
    let h = hover(s, ctx.t);
    if s.t % 2 == 0 {
        s.trail.rotate_right(1);
        s.trail[0] = (b.x, b.y, h);
    }
    let (px, py) = ctx.player;
    let d = b.dist_to(px, py);

    // Struck: tumble with the knockback, no attacking.
    if b.stun > 0 {
        if s.mode == Mode::Windup {
            s.set(Mode::Drift);
            s.cooldown = s.cooldown.max(60);
        }
        b.vx *= 0.8;
        b.vy *= 0.8;
        b.integrate(ctx.zone);
        return;
    }

    match s.mode {
        Mode::Drift => {
            let seen = ctx.player_alive && d < SIGHT && ctx.zone.line_clear(b.x, b.y - 4.0, px, py - 4.0);
            steer(b, s, ctx, d, seen);
            if seen && s.cooldown == 0 && d > PERSONAL * 0.8 {
                s.aim = b.dir_to(px, py);
                s.set(Mode::Windup);
            }
        }
        Mode::Windup => {
            if !ctx.player_alive || !ctx.zone.line_clear(b.x, b.y - 4.0, px, py - 4.0) {
                s.set(Mode::Drift);
                s.cooldown = 45;
                b.integrate(ctx.zone);
                return;
            }
            if s.mode_t <= WINDUP - AIM_LOCK {
                s.aim = b.dir_to(px, py);
            }
            b.vx *= 0.85;
            b.vy *= 0.85;
            let fx = &mut *ctx.fx;
            // Sparks drawn into the core.
            if s.mode_t % 3 == 0 && s.mode_t + 6 < WINDUP {
                let r = fx.rng.range(8.0, 12.0);
                let side = fx.rng.signed() * 3.0;
                let (ox, oy) = (s.aim.0 * r - s.aim.1 * side, s.aim.1 * r + s.aim.0 * side);
                fx.particles.add(
                    Particle::new(b.x + ox, b.y + oy)
                        .z(h + 5.0)
                        .vel(-ox * 0.09, -oy * 0.09, 0.0)
                        .color(FLAME[3])
                        .fade_to(FLAME[2])
                        .glow(1.0)
                        .grav(0.0)
                        .drag(0.97)
                        .life(11.0),
                );
            }
            b.integrate(ctx.zone);
            if s.mode_t >= WINDUP {
                fire(b, s, ctx, h);
            }
        }
        Mode::Recoil => {
            b.vx *= 0.9;
            b.vy *= 0.9;
            b.integrate(ctx.zone);
            if s.mode_t >= RECOIL {
                s.set(Mode::Drift);
            }
        }
    }
}

/// Circle Glint at the orbit radius (an ellipse squashed for the 3/4 view); dart away when
/// crowded; head straight for Glint when out of sight.
fn steer(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx, d: f32, seen: bool) {
    let (px, py) = ctx.player;
    let (tx, ty, max) = if !ctx.player_alive {
        (b.x, b.y, 0.0)
    } else if !seen {
        (px, py, ORBIT_SPEED)
    } else {
        let away = if d > 0.5 { norm(b.x - px, (b.y - py) / 0.8) } else { (1.0, 0.0) };
        let a = away.1.atan2(away.0) + s.spin * 0.4;
        let r = if d < PERSONAL { s.radius + 30.0 } else { s.radius };
        (px + a.cos() * r, py + a.sin() * r * 0.8, if d < PERSONAL { FLEE_SPEED } else { ORBIT_SPEED })
    };
    let (dx, dy) = (tx - b.x, ty - b.y);
    let dist = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = norm(dx, dy);
    let sp = max * (dist / 16.0).min(1.0);
    b.vx += (ux * sp - b.vx) * 0.06;
    b.vy += (uy * sp - b.vy) * 0.06;
    let blocked = b.integrate(ctx.zone);
    // Bump a wall (or now and then on a whim): circle the other way.
    let whim = s.t % 240 == 0 && s.vary(s.t) < 0.35;
    if (blocked || whim) && s.flip_cd == 0 {
        s.spin = -s.spin;
        s.flip_cd = 50;
    }
}

/// Launch a slow, slashable bolt along the telegraphed direction.
fn fire(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx, h: f32) {
    let aim = if s.aim == (0.0, 0.0) { (0.0, 1.0) } else { s.aim };
    let fan = s.volley % 3 == 2;
    s.volley += 1;
    let base = aim.1.atan2(aim.0);
    let angles: &[f32] = if fan { &[-0.34, 0.0, 0.34] } else { &[0.0] };
    let anim = bolt_anim();
    let fx = &mut *ctx.fx;
    for &da in angles {
        let a = base + da;
        let (vx, vy) = (a.cos() * BOLT_SPEED, a.sin() * BOLT_SPEED);
        let mut p = Projectile::new(b.x + a.cos() * 4.0, b.y + a.sin() * 4.0, vx, vy, fx).color(FLAME[2]).z(h + 4.0, 0.0, 0.0);
        if let Some(anim) = anim {
            p = p.look(BOLT_SHEET, anim);
        }
        p.r = 3.0;
        p.life = 190;
        p.light = 12.0;
        p.homing = 0.0;
        ctx.projectiles.push(p);
    }
    fx.sound("wisp_shot");
    fx.particles
        .sparks(&mut fx.rng, b.x + aim.0 * 4.0, b.y + aim.1 * 4.0, h + 4.0, aim.0, aim.1, 5, FLAME[2], 1.5);
    b.kx = -aim.0 * 1.7;
    b.ky = -aim.1 * 1.7;
    s.set(Mode::Recoil);
    s.cooldown = 130 + (s.vary(s.volley) * 80.0) as u32;
}

/// The bolt anim to use, if the bolt sheet exists: a known name, else its first anim.
fn bolt_anim() -> Option<&'static str> {
    static ANIM: OnceLock<Option<&'static str>> = OnceLock::new();
    *ANIM.get_or_init(|| {
        let sh = sheet(BOLT_SHEET)?;
        const NAMES: [&str; 6] = ["fly", "idle", "loop", "spin", "bolt", "default"];
        NAMES.iter().copied().find(|n| sh.has(n)).or_else(|| sh.anims.keys().min().map(|k| k.as_str()))
    })
}

/// Scatter a few embers and let the strike interrupt spellcasting.
pub fn on_hit(b: &mut Body, s: &mut State, hit: &Hit, ctx: &mut EnemyCtx) {
    // Light as a breath: struck wisps fly further and flee round the other way.
    b.kx *= 1.5;
    b.ky *= 1.5;
    if s.flip_cd < 20 {
        s.spin = -s.spin;
        s.flip_cd = 40;
    }
    s.cooldown = s.cooldown.max(50);
    let fx = &mut *ctx.fx;
    let (dx, dy) = hit.push_dir(b.x, b.y);
    let h = hover(s, ctx.t) + 5.0;
    fx.particles.sparks(&mut fx.rng, b.x, b.y, h, dx, dy, 7, FLAME[2], 2.0);
    // A few embers knocked loose.
    for _ in 0..4 {
        fx.particles.add(
            Particle::new(b.x + fx.rng.signed() * 3.0, b.y)
                .z(h + fx.rng.signed() * 3.0)
                .vel(dx * fx.rng.range(0.3, 1.0) + fx.rng.signed() * 0.3, dy * fx.rng.range(0.3, 1.0), fx.rng.range(0.2, 0.6))
                .color(FLAME[3])
                .fade_to(FLAME[0])
                .glow(1.0)
                .grav(-0.01)
                .drag(0.93)
                .life(fx.rng.range(18.0, 30.0)),
        );
    }
}

/// Extinguish the flame into rising sprite pixels and falling ash.
pub fn on_death(b: &Body, s: &State, ctx: &mut EnemyCtx) {
    let h = hover(s, ctx.t) + 5.0;
    let fx = &mut *ctx.fx;
    fx.sound_vol("proj_fizzle", 0.8);
    // A soft pop of light from the core.
    fx.particles
        .add(Particle::new(b.x, b.y).z(h).color(FLAME[4]).glow(1.0).grav(0.0).life(8.0).fade(false).light(64.0));
    // Embers: a ring bursting out, then drifting up and fading. Some carry light.
    for i in 0..22 {
        let a = i as f32 / 22.0 * TAU + fx.rng.signed() * 0.2;
        let sp = fx.rng.range(0.8, 2.0);
        let hot = i % 4 == 0;
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(h)
                .vel(a.cos() * sp, a.sin() * sp * 0.75, fx.rng.range(-0.3, 0.6))
                .color(if hot { FLAME[4] } else { FLAME[3] })
                .fade_to(FLAME[0])
                .glow(1.0)
                .grav(-0.018)
                .drag(0.92)
                .life(fx.rng.range(30.0, 60.0))
                .light(if hot { 12.0 } else { 0.0 }),
        );
    }
    // The core's last sparks.
    fx.particles.sparks(&mut fx.rng, b.x, b.y, h, 0.0, -1.0, 8, FLAME[3], 1.6);
    // Ash flecks fall and settle.
    for _ in 0..10 {
        let (dx, dy) = fx.rng.dir();
        fx.particles.add(
            Particle::new(b.x + dx * 2.0, b.y + dy * 2.0)
                .z(h)
                .vel(dx * 0.5, dy * 0.4, fx.rng.range(0.2, 0.8))
                .color(ASH)
                .grav(0.06)
                .drag(0.95)
                .life(fx.rng.range(200.0, 300.0))
                .settle(true),
        );
    }
    // Scatter the actual silhouette, including when the hand-drawn fallback is in use.
    let (cell, anchor) = current_cell(b, s, ctx.t);
    let start = fx.particles.list.len();
    fx.particles.burst_cell(&mut fx.rng, cell, anchor, false, b.x, b.y, 0.9, 42.0);
    for p in &mut fx.particles.list[start..] {
        p.z += h - 5.0;
        p.grav = -0.012;
        p.drag = 0.94;
        p.settle = false;
        p.c1 = FLAME[0];
    }
}

// ------------------------------------------------------------------------------------ drawing

/// Pick pose names and playback timing for the current behavior.
fn anim_for(b: &Body, s: &State) -> (&'static [&'static str], u32, bool) {
    if b.stun > 0 {
        return (&["hurt", "idle", "float"], 12u32.saturating_sub(b.stun), false);
    }
    match s.mode {
        Mode::Drift => (&["idle", "float", "fly", "move"], s.t, true),
        Mode::Windup => (&["windup", "charge", "telegraph", "idle", "float"], s.mode_t, false),
        Mode::Recoil => (&["shoot", "attack", "fire", "idle", "float"], s.mode_t, false),
    }
}

/// Sample the artist sheet when it has a compatible animation.
fn sheet_cell(b: &Body, s: &State, t: u64) -> Option<(&'static Cell, (i32, i32))> {
    let sh = sheet(SHEET)?;
    let (names, ticks, looping) = anim_for(b, s);
    let ticks = if s.mode == Mode::Drift && b.stun == 0 { t as u32 } else { ticks };
    let a = names.iter().find_map(|n| sh.anim(n).or_else(|| sh.dir_anim(n, Dir::Down).map(|(a, _)| a)))?;
    let idle = ["idle", "float", "fly", "move"]
        .iter()
        .any(|base| a.name == *base || a.name.starts_with(&format!("{base}_")));
    let looping = a.json.get("loop").and_then(|v| v.as_bool()).unwrap_or(looping || idle);
    let ticks = if s.mode == Mode::Windup && !idle { ticks * a.total_ticks() / WINDUP } else { ticks };
    Some((super::variation::cell(SHEET, sh.cell(a, a.frame_at(ticks, looping)), s.appearance), sh.anchor_of(a)))
}

/// Choose the authored sprite or a native-grid fallback with the same anchor contract.
fn current_cell(b: &Body, s: &State, t: u64) -> (&'static Cell, (i32, i32)) {
    if let Some(frame) = sheet_cell(b, s, t) {
        return frame;
    }
    let frame = ((t as u32 + s.seed) / 6) as usize % 4;
    let gathering = s.mode == Mode::Windup || (s.mode == Mode::Recoil && s.mode_t < 10);
    let cell = &fallback()[if gathering { 4 } else { frame }];
    (cell, (cell.w / 2, cell.h - 1))
}

/// Draw the flame body, a restrained tail, and its motivated light.
pub fn draw(b: &Body, s: &State, f: &mut Frame) {
    let h = hover(s, f.t);
    let (gx, gy) = (b.x, b.y);
    let (cell, anchor) = current_cell(b, s, f.t);
    let jx = if b.stun > 0 { ((b.stun % 2) as f32) * 2.0 - 1.0 } else { 0.0 };
    // Shadow: small and soft, it's floating.
    f.shadow(f.sx(gx), f.sy(gy), 3, 1, 0.3);

    let charge = if s.mode == Mode::Windup { (s.mode_t as f32 / WINDUP as f32).min(1.0) } else { 0.0 };
    let recoil = if s.mode == Mode::Recoil { 1.0 - (s.mode_t as f32 / 10.0).min(1.0) } else { 0.0 };
    // Flicker from two incommensurate waves: alive, never a regular pulse.
    let ft = f.t as f32 + s.seed as f32;
    let flicker = 0.92 + 0.05 * (ft * 0.31).sin() + 0.03 * (ft * 0.77).sin();
    let gutter = (b.stun as f32 / 12.0).min(1.0);
    if !f.surface_lighting {
        let light_r = (16.0 + 12.0 * charge + 4.0 * recoil) * flicker * (1.0 - 0.35 * gutter);
        f.add_light_3d_w(gx, gy, h + 5.0, light_r, FLAME[2], 0.24 + 0.14 * charge);
    }

    // Authored sheets own their complete silhouette, including any flame tail.
    if sheet_cell(b, s, f.t).is_none() {
        draw_tail(b, s, f, h);
    }

    // Hurt jitter, and the core going chalk-bright through the last third of the windup.
    let hot = ((charge - 0.6) / 0.4).clamp(0.0, 1.0);
    let opts = DrawOpts {
        actor: true,
        foot_depth: 2,
        foot_half_w: b.r,
        ground_y: Some(gy.floor()),
        elevation: h,
        flash: b.flash.max(hot * 0.35),
        emit_gain: 1.0 + 0.25 * charge + 0.2 * recoil,
        ..Default::default()
    };
    let caster_index = f.surface_casters.len();
    f.sprite_w(cell, anchor, gx + jx, gy - h, &opts);
    if f.surface_lighting
        && let Some(caster) = f.surface_casters.get(caster_index)
        && let Some((x, ground_y, height)) = flame_light_position(cell, anchor, caster)
    {
        // The artist's tail extends below its nominal flame anchor. Use this
        // actual pose's foot depth so the core belongs to its native housing,
        // while retaining the exact glowing screen pixel through hover/jitter.
        let light_r = (52.0 + 16.0 * charge + 4.0 * recoil) * flicker;
        let intensity = (0.90 + 0.48 * charge + 0.12 * recoil) * (1.0 - 0.5 * gutter);
        f.add_light_3d_w(x, ground_y, height, light_r, FLAME[3], intensity);
    }

    if s.mode == Mode::Windup && s.mode_t >= WINDUP - AIM_LOCK {
        // A small luminous tongue points down the locked shot direction.
        let x = f.sx(gx + s.aim.0 * 7.0);
        let y = f.sy(gy - h - 5.0 + s.aim.1 * 7.0);
        f.put_glow(x, y, FLAME[3]);
        f.put_glow(x - s.aim.0.round() as i32, y - s.aim.1.round() as i32, FLAME[2]);
    }
}

/// The brightest authored core pixel, relative to this pose's flame anchor.
/// A deterministic tie keeps a held pose's source perfectly still.
fn flame_core(cell: &Cell, anchor: (i32, i32)) -> Option<(f32, f32)> {
    let mut best = None;
    let mut brightness = -1.0;
    for y in 0..cell.h {
        for x in 0..cell.w {
            let p = cell.at(x, y);
            if p.a && p.emit && p.c.luma() > brightness {
                brightness = p.c.luma();
                best = Some((x as f32 + 0.5 - anchor.0 as f32, y as f32 + 0.5 - anchor.1 as f32));
            }
        }
    }
    best
}

/// Share the exact native source column and foot interval with the caster.
/// Moving Y and Z together changes depth without moving the projected flame.
fn flame_light_position(cell: &Cell, anchor: (i32, i32), caster: &crate::gfx::frame::SurfaceCaster) -> Option<(f32, f32, f32)> {
    let (lx, ly) = flame_core(cell, anchor)?;
    let source_x = (lx + anchor.0 as f32 - 0.5).round() as usize;
    let span = caster.foot.get(source_x)?;
    let ground_y = span[0] + (span[1] - span[0]) * 0.5;
    let projected_y = caster.y.floor() + ly;
    let x = caster.x.floor() + if caster.flip { 1.0 - lx } else { lx };
    Some((x, ground_y, ground_y - projected_y))
}

/// The curling tail hangs from the flame's base: it lags sideways behind the motion (from the
/// position history), sinks, and a travelling wave curls it.
fn draw_tail(b: &Body, s: &State, f: &mut Frame, h: f32) {
    let ft = f.t as f32 * 0.16 + s.seed as f32;
    let base_y = b.y - h + 1.0;
    let mut prev: Option<(i32, i32)> = None;
    let mut pts: Vec<((i32, i32), Rgb)> = Vec::with_capacity(48);
    for i in 0..TAIL {
        let k = i as f32;
        let (hx, hy, hh) = s.trail[i];
        let lag_x = ((hx - b.x) * 0.7).clamp(-k * 1.4, k * 1.4);
        let lag_y = ((hy - hh) - (b.y - h)).clamp(0.0, k * 0.8);
        let wave = (ft - k * 0.8).sin() * (0.2 + k * 0.28);
        let p = (f.sx(b.x + lag_x + wave), f.sy(base_y + k * 1.1 + lag_y));
        let c = match i {
            0..=2 => FLAME[2],
            3..=5 => FLAME[1],
            _ => FLAME[0],
        };
        if let Some(q) = prev {
            Frame::line_with(q.0, q.1, p.0, p.1, |x, y| pts.push(((x, y), c)));
        }
        prev = Some(p);
    }
    for ((x, y), c) in pts {
        f.put_glow(x, y, c);
    }
}

// ----------------------------------------------------------------------------------- fallback

/// Built-in frames used until `wisp.png` exists: four flicker frames and a gathering frame.
fn fallback() -> &'static [Cell; 5] {
    static CELLS: OnceLock<[Cell; 5]> = OnceLock::new();
    CELLS.get_or_init(|| {
        let pal = [('1', FLAME[0]), ('2', FLAME[1]), ('3', FLAME[2]), ('4', FLAME[3]), ('5', FLAME[4]), ('k', rgb(0x222d3d))];
        let em = ['1', '2', '3', '4', '5'];
        // Pale iris mass, a small chalk-bright core between the eyes, deep violet underside.
        let body = [
            "..123321..",
            ".12344321.",
            "1234554321",
            "123k55k321",
            "1233443321",
            "1223333221",
            ".11222211.",
            "..111111..",
        ];
        let tips: [[&str; 4]; 4] = [
            [".....1....", ".....21...", "....232...", "...12321.."],
            ["....1.....", "....12....", "....232...", "...12321.."],
            ["...1......", "...12.....", "...1232...", "...12321.."],
            ["......1...", ".....12...", "....1232..", "...12321.."],
        ];
        let mk = |tip: &[&str; 4], body: &[&str]| {
            let rows: Vec<&str> = tip.iter().copied().chain(body.iter().copied()).collect();
            Cell::from_ascii(&rows, &pal, &em)
        };
        // Gathering: the core swells and the flame stands up straight.
        let gather_body = [
            "..134431..",
            ".13455431.",
            "1345555431",
            "134k55k431",
            "1345555431",
            "1234554321",
            ".12344321.",
            "..112211..",
        ];
        [
            mk(&tips[0], &body),
            mk(&tips[1], &body),
            mk(&tips[2], &body),
            mk(&tips[3], &body),
            mk(&["....11....", "....22....", "...1331...", "..123321.."], &gather_body),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::Fx;
    use crate::world::{Zone, ZoneId};

    /// All luminous archetypes stop submitting both their source and caster on
    /// a real lethal hit; death-particle flashes remain separate transient FX.
    #[test]
    fn luminous_enemy_death_extinguishes_live_source_and_housing() {
        use crate::entities::enemies::{Brain, Enemy, EnemyKind};
        use crate::combat::{Shape, Team};
        let zone = Zone::new(ZoneId::Glasswater, 20, 20);
        for kind in [EnemyKind::Wisp, EnemyKind::CrystalBeetle, EnemyKind::EmberHound, EnemyKind::SpittingTotem] {
            for appearance in 0..3 {
                super::super::variation::with_skin(appearance, || {
                    let mut enemy = Enemy::spawn(kind, 100.0, 100.0);
                    if let Brain::SpittingTotem(state) = &mut enemy.brain {
                        // The authored final idle pose is dark. Test extinction from
                        // the actual luminous first idle pose, rather than inventing
                        // a source in whichever cooldown pose the spawn clock selects.
                        state.t = 0;
                    }
                    let mut f = Frame::new();
                    f.surface_lighting = true;
                    enemy.draw(&mut f);
                    assert!(!f.point_lights.is_empty(), "{kind:?} appearance{appearance} luminous pose needs a source");
                    assert!(f.point_lights.iter().any(|l| l.radius >= 30.0));
                    let mut fx = Fx::new(17);
                    let mut hits = Vec::new();
                    let mut projectiles = Vec::new();
                    let mut ctx = EnemyCtx { player: (120.0, 100.0), player_alive: true, zone: &zone,
                        fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 0 };
                    // Behind the beetle's initial leftward guard.
                    let hit = Hit::new(Shape::Circle { x: 120.0, y: 100.0, r: 24.0 }, Team::Player, 99, 1);
                    enemy.on_hit(&hit, &mut ctx);
                    assert!(!enemy.body.alive, "{kind:?} lethal hit");
                    f.begin(Rgb::BLACK, Rgb::WHITE);
                    f.surface_lighting = true;
                    enemy.draw(&mut f);
                    assert!(f.point_lights.is_empty(), "{kind:?} left a persistent light after death");
                    assert!(f.surface_casters.is_empty(), "{kind:?} left a persistent shadow housing");
                });
            }
        }
    }

    /// Source-column depth is indexed before mirroring, while the visible
    /// source follows the renderer's reflected anchor and fractional hover raster.
    #[test]
    fn native_flame_source_preserves_mirrored_and_hovering_attachment() {
        use crate::gfx::frame::Surface;
        use crate::gfx::W;
        let sh = sheet(SHEET).unwrap();
        let anim = sh.anim("idle").unwrap();
        for appearance in 0..3 {
            for flip in [false, true] {
                let cell = super::super::variation::cell(SHEET, sh.cell(anim, 0), appearance);
                let anchor = sh.anchor_of(anim);
                let mut f = Frame::new();
                f.begin(Rgb::BLACK, Rgb::BLACK);
                f.surface_lighting = true;
                f.surfaces.fill(Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 });
                f.sprite_w(cell, anchor, 192.73, 120.81 - 7.3,
                    &DrawOpts { actor: true, flip, elevation: 7.3, ground_y: Some(120.0), foot_depth: 2, foot_half_w: R, ..Default::default() });
                let (x, ground, height) = flame_light_position(cell, anchor, &f.surface_casters[0]).unwrap();
                f.add_light_3d_w(x, ground, height, 52.0, FLAME[3], 0.9);
                let light = f.point_lights[0];
                assert!(f.emissive[light.y.floor() as usize * W + light.x.floor() as usize].max_c() > 0.0);
                assert!(crate::gfx::lighting::point_visibility(&f, light).iter().all(|&v| v == 255));
            }
        }
    }

    /// Every real flame pose keeps its source inside the native volume and on
    /// emitting raster ink, including fractional motion, hovering and hurt jitter.
    #[test]
    fn native_flame_lights_its_nearby_floor_without_self_occluding() {
        use crate::gfx::frame::Surface;
        use crate::gfx::W;
        let sh = sheet(SHEET).unwrap();
        let mut checked = 0;
        for name in ["idle", "windup", "attack", "hurt"] {
            let a = sh.anim(name).unwrap();
            let mut elapsed = 0;
            for pose in 0..a.frames {
                for appearance in 0..3 {
                    let (mut b, mut s) = spawn(192.73, 120.81);
                    s.appearance = appearance;
                    s.mode = match name { "windup" => Mode::Windup, "attack" => Mode::Recoil, _ => Mode::Drift };
                    s.mode_t = if name == "windup" { (elapsed * WINDUP).div_ceil(a.total_ticks()) } else { elapsed };
                    s.t = elapsed;
                    if name == "hurt" { b.stun = 12 - elapsed; }
                    let mut f = Frame::new();
                    f.begin(Rgb::gray(0.5), Rgb::BLACK);
                    f.surface_lighting = true;
                    f.t = if name == "idle" { u64::from(elapsed) } else { u64::from(71 + elapsed) };
                    f.surfaces.fill(Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 });
                    let (cell, _) = current_cell(&b, &s, f.t);
                    assert!(std::ptr::eq(cell, super::super::variation::cell(SHEET, sh.cell(a, pose), appearance)),
                        "fixture must draw {name} pose{pose} appearance{appearance}");
                    draw(&b, &s, &mut f);
                    assert_eq!(f.point_lights.len(), 1);
                    let light = f.point_lights[0];
                    let core = light.y.floor() as usize * W + light.x.floor() as usize;
                    assert!(f.emissive[core].max_c() > 0.0,
                        "source left emitting ink in {name} pose{pose} appearance{appearance}");
                    assert!(crate::gfx::lighting::point_visibility(&f, light).iter().all(|&v| v == 255),
                        "flame blocked itself in {name} pose{pose} appearance{appearance}");
                    let floor = (b.y.floor() as usize + 12) * W + b.x.floor() as usize + 20;
                    assert_eq!((f.surfaces[floor].owner, f.surfaces[floor].height), (0, 0.0));
                    crate::gfx::surface::apply(&mut f);
                    assert!(f.light[floor].luma() > 0.03,
                        "flame failed to light nearby floor in {name} pose{pose} appearance{appearance}");
                    checked += 1;
                }
                elapsed += a.durations[pose];
            }
        }
        assert_eq!(checked, 54);
    }

    #[test]
    fn flame_core_tracks_actual_native_emission_for_every_pose_and_appearance() {
        let sh = sheet(SHEET).unwrap();
        for a in sh.anims.values() {
            for i in 0..a.frames {
                for appearance in 0..3 {
                    let cell = super::super::variation::cell(SHEET, sh.cell(a, i), appearance);
                    if let Some((x, y)) = flame_core(cell, sh.anchor_of(a)) {
                        let (ax, ay) = sh.anchor_of(a);
                        let p = cell.at((x + ax as f32).floor() as i32, (y + ay as f32).floor() as i32);
                        assert!(p.a && p.emit, "{} pose{i} appearance{appearance}", a.name);
                    }
                }
            }
        }
    }

    /// A late sidestep avoids a committed, straight bolt instead of triggering new aim.
    #[test]
    fn firing_holds_its_last_aim_and_gives_the_player_recovery_time() {
        let zone = Zone::new(ZoneId::Glasswater, 30, 20);
        let mut fx = Fx::new(7);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx {
            player: (100.0, 180.0),
            player_alive: true,
            zone: &zone,
            fx: &mut fx,
            hits: &mut hits,
            projectiles: &mut projectiles,
            t: 1,
        };
        let (mut b, mut s) = spawn(100.0, 100.0);
        s.set(Mode::Windup);
        s.mode_t = WINDUP - AIM_LOCK;
        s.aim = (1.0, 0.0);
        for _ in 0..AIM_LOCK {
            update(&mut b, &mut s, &mut ctx);
        }
        assert_eq!(s.mode, Mode::Recoil);
        assert_eq!(ctx.projectiles.len(), 1);
        let p = &ctx.projectiles[0];
        assert!((p.vx - BOLT_SPEED).abs() < 0.001 && p.vy.abs() < 0.001);
        assert_eq!(p.homing, 0.0);
        assert!(s.cooldown >= 130);
        assert_eq!(p.dmg, 1);
    }

    /// A landed strike interrupts even the last frame of a spell windup.
    #[test]
    fn a_hit_interrupts_a_ready_spell_and_death_extinguishes_sprite_pixels() {
        let zone = Zone::new(ZoneId::Glasswater, 30, 20);
        let mut fx = Fx::new(7);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx {
            player: (180.0, 100.0),
            player_alive: true,
            zone: &zone,
            fx: &mut fx,
            hits: &mut hits,
            projectiles: &mut projectiles,
            t: 1,
        };
        let (mut b, mut s) = spawn(100.0, 100.0);
        s.set(Mode::Windup);
        s.mode_t = WINDUP - 1;
        b.stun = 12;
        update(&mut b, &mut s, &mut ctx);
        assert_eq!(s.mode, Mode::Drift);
        assert!(ctx.projectiles.is_empty());
        assert!(s.cooldown >= 60);
        on_death(&b, &s, &mut ctx);
        assert!(
            ctx.fx.particles.list.iter().filter(|p| p.grav < 0.0 && !p.settle).count() > 50,
            "the actual sprite should dissolve into rising pixels"
        );
    }
}
