//! Spitting totem: a carved basalt pillar with a lava maw. It never moves.
//!
//! Loop:
//! 1. Idle: the maw smolders, the odd wisp of smoke curls up.
//! 2. Charge (the telegraph, 46 ticks): embers escape the furnace, the maw and its light
//!    swell from dull red to white-hot, and in the last 12 ticks the whole pillar trembles.
//! 3. Spit: it lobs an ember glob in a high arc at where the player is heading. The landing
//!    spot is marked on the ground for the whole flight: a ring that tightens onto the splash
//!    radius while its dithered fill thickens. The glob splashes in a burst of lava droplets
//!    that keep glowing on the ground as they cool. Every third volley is three quick globs.
//! 4. Cooldown, then again.
//!
//! Hitting it chips stone off, rocks it, and snuffs a charge in a puff of smoke. As it takes
//! damage, glowing cracks open across its body. Destroying it topples the stones into rubble,
//! the maw erupts one last time, and any globs it still has in the air fizzle out.
//! A glob can be slashed out of the air.
//!
//! Art: `spitting_totem` (idle / windup / attack / hurt) and `ember_glob` (its fly anim).
//! Without the sheets, hand-placed fallbacks are drawn.

use super::EnemyCtx;
use crate::combat::Hit;
use crate::entities::{Body, Projectile};
use crate::fx::{Debris, Fx, Particle};
use crate::gfx::{Anim, Cell, Dir, DrawOpts, Frame, Rgb, Sheet, rgb, sheet};
use crate::util::{approach, bayer4, hash2, norm};
use std::sync::OnceLock;

const HP: i32 = 7;
const RADIUS: f32 = 6.0;

/// Starts a volley when the player is this close.
const RANGE: f32 = 200.0;
const CHARGE_FIRST: u32 = 46;
const CHARGE_NEXT: u32 = 22;
/// The last ticks of a charge where the pillar trembles.
const TREMBLE: u32 = 12;
const RECOIL: u32 = 28;
const GRAV: f32 = 0.11;
/// Projectile radius; the engine's landing hit is a circle of r + 6.
const GLOB_R: f32 = 6.0;
const SPLASH_R: f32 = GLOB_R + 6.0;
/// Ring radius drawn at the moment of impact (splash radius + about the player's half-width).
const MARK_R: f32 = 13.0;

// Palette: cool basalt planes and source-bound salmon heat.
const ST0: Rgb = rgb(0x393746);
const ST2: Rgb = rgb(0x56505d);
const ST3: Rgb = rgb(0x79707a);
const ST4: Rgb = rgb(0x9a8e92);
const CU2: Rgb = rgb(0x79707a);
const HOT: Rgb = rgb(0xefb68a);
const YEL: Rgb = rgb(0xefb68a);
const ORA: Rgb = rgb(0xd48670);
const RED: Rgb = rgb(0xa36059);
const DRED: Rgb = rgb(0x6f454b);
const LAVA0: Rgb = rgb(0x6f454b);
const ASH: Rgb = rgb(0x6f616b);
const ASH_D: Rgb = rgb(0x514854);
const SCORCH: Rgb = rgb(0x292c3c);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Charge,
    Recoil,
    Cooldown,
}

#[derive(Clone, Copy, Debug)]
struct Glob {
    id: u32,
    /// Landing spot (exact: the flight is simulated at launch).
    tx: f32,
    ty: f32,
    t: u32,
    total: u32,
    /// Has the impact splash already been emitted?
    splashed: bool,
    /// Last seen (x, y, z, vz).
    last: (f32, f32, f32, f32),
}

#[derive(Clone, Debug)]
pub struct State {
    pub phase: Phase,
    pub pt: u32,
    pub t: u32,
    home: (f32, f32),
    charge_len: u32,
    cooldown: u32,
    volley: u32,
    shots_left: u32,
    globs: Vec<Glob>,
    /// Maw heat: ~0.15 smolder, 1 white-hot, >1 the flare right after a spit.
    heat: f32,
    /// Ticks of rocking after a hit, and which way.
    wobble: u32,
    wobble_dir: i32,
    prev_player: (f32, f32),
    pvel: (f32, f32),
    kill_dir: (f32, f32),
    seed: u32,
    appearance: u8,
    noticed: bool,
}

pub fn spawn(x: f32, y: f32) -> (Body, State) {
    let seed = (hash2(x as i32, y as i32, 41) * 1_000_000.0) as u32;
    let s = State {
        phase: Phase::Cooldown,
        pt: 0,
        t: (hash2(seed as i32, 1, 3) * 100.0) as u32,
        home: (x, y),
        charge_len: CHARGE_FIRST,
        cooldown: 30 + (hash2(seed as i32, 2, 3) * 50.0) as u32,
        volley: 0,
        shots_left: 0,
        globs: Vec::new(),
        heat: 0.15,
        wobble: 0,
        wobble_dir: 1,
        prev_player: (x, y),
        pvel: (0.0, 0.0),
        kill_dir: (0.0, 1.0),
        seed,
        appearance: super::variation::appearance(x, y),
        noticed: false,
    };
    (Body::new(x, y, RADIUS, HP), s)
}

fn set_phase(s: &mut State, p: Phase) {
    s.phase = p;
    s.pt = 0;
}

pub fn update(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    s.t += 1;
    s.pt += 1;
    s.wobble = s.wobble.saturating_sub(1);
    // it's a pillar: nothing moves it
    b.x = s.home.0;
    b.y = s.home.1;
    b.vx = 0.0;
    b.vy = 0.0;
    b.kx = 0.0;
    b.ky = 0.0;
    let (px, py) = ctx.player;
    let pv = (px - s.prev_player.0, py - s.prev_player.1);
    if pv.0.abs() + pv.1.abs() < 8.0 {
        s.pvel = (approach(s.pvel.0, pv.0, 0.2), approach(s.pvel.1, pv.1, 0.2));
    }
    s.prev_player = (px, py);
    let d = b.dist_to(px, py);
    b.face = b.dir_to(px, py);

    if !s.noticed {
        // it wakes: the maw flares once
        s.noticed = true;
        s.heat = 0.9;
        ctx.fx.sound_vol("enemy_alert", 0.5);
    }
    track_globs(s, ctx);

    let stunned = b.stun > 0;
    match s.phase {
        Phase::Idle => {
            s.heat = approach(s.heat, 0.15, 0.05);
            if !stunned && ctx.player_alive && d < RANGE && s.pt > 20 && ctx.zone.line_clear(b.x, b.y + 1.0, px, py) {
                s.volley += 1;
                s.shots_left = if s.volley % 3 == 0 { 3 } else { 1 };
                s.charge_len = CHARGE_FIRST;
                set_phase(s, Phase::Charge);
                ctx.fx.sound_vol("lava_bubble_1", 0.7);
            }
        }
        Phase::Charge => {
            if stunned {
                s.pt -= 1;
            }
            let k = (s.pt as f32 / s.charge_len as f32).min(1.0);
            s.heat = approach(s.heat, 0.2 + k * k * 0.9, 0.25);
            inhale(b, s, ctx.fx, k);
            // the generic attack tell is 24 ticks long and ends where the spit happens
            if s.pt == s.charge_len.saturating_sub(24).max(1) {
                ctx.fx.sound_vol("enemy_windup", 0.75);
            }
            if s.charge_len - s.pt.min(s.charge_len) < TREMBLE && s.pt % 4 == 0 {
                let fx = &mut *ctx.fx;
                let ox = fx.rng.signed() * 5.0;
                fx.particles.dust(&mut fx.rng, b.x + ox, b.y + 1.0, 1, ASH_D);
            }
            if s.pt >= s.charge_len {
                if spit(b, s, ctx) {
                    s.shots_left = s.shots_left.saturating_sub(1);
                    set_phase(s, Phase::Recoil);
                } else {
                    // the player ducked behind cover: it gives up with a cough of smoke
                    snuff(b, s, ctx.fx, 40);
                }
            }
        }
        Phase::Recoil => {
            s.heat = approach(s.heat, 0.5, 0.12);
            if s.pt >= RECOIL {
                if s.shots_left > 0 && ctx.player_alive {
                    s.charge_len = CHARGE_NEXT;
                    set_phase(s, Phase::Charge);
                } else {
                    s.cooldown = 100 + ctx.fx.rng.irange(0, 50) as u32;
                    set_phase(s, Phase::Cooldown);
                }
            }
        }
        Phase::Cooldown => {
            s.heat = approach(s.heat, 0.15, 0.04);
            if s.pt >= s.cooldown {
                set_phase(s, Phase::Idle);
            }
        }
    }

    // smolder: an occasional curl of smoke from the maw
    if s.t % 37 == 0 && s.phase != Phase::Charge {
        let (mx, mz) = maw();
        let fx = &mut *ctx.fx;
        fx.particles.add(
            Particle::new(b.x + mx + fx.rng.signed(), b.y + 1.0)
                .z(mz + 2.0)
                .vel(fx.rng.signed() * 0.08, 0.0, 0.3)
                .color(ASH)
                .fade_to(ASH_D)
                .grav(0.0)
                .drag(0.985)
                .life(fx.rng.range(40.0, 60.0)),
        );
    }
    b.integrate(ctx.zone);
    b.x = s.home.0;
    b.y = s.home.1;
}

/// A few embers escape from the furnace as the charge heats up.
fn inhale(b: &Body, s: &State, fx: &mut Fx, k: f32) {
    let every = if k < 0.5 { 8 } else { 4 };
    if s.pt % every != 0 {
        return;
    }
    let (mx, mz) = maw();
    fx.particles.add(
        Particle::new(b.x + mx + fx.rng.signed() * 2.0, b.y + 1.0)
            .z(mz)
            .vel(fx.rng.signed() * 0.2, 0.0, fx.rng.range(0.3, 0.6))
            .color(YEL)
            .fade_to(RED)
            .glow(1.0)
            .grav(0.0)
            .drag(0.96)
            .life(16.0),
    );
}

/// Launch a glob at where the player is heading. Returns false if there's no clear line.
fn spit(b: &Body, s: &mut State, ctx: &mut EnemyCtx) -> bool {
    let (px, py) = ctx.player;
    let (mx, mz) = maw();
    let (x0, y0) = (b.x + mx, b.y + 1.0);
    let dist = ((px - x0).powi(2) + (py - y0).powi(2)).sqrt();
    let t_est = (34.0 + dist * 0.16).clamp(36.0, 64.0);
    // lead the target a little (less on follow-up shots, which already spread by re-aiming)
    let lead = if s.shots_left >= 3 { 0.35 } else { 0.2 };
    let (lx, ly) = (s.pvel.0 * t_est * lead, s.pvel.1 * t_est * lead);
    let ll = (lx * lx + ly * ly).sqrt();
    let (lx, ly) = if ll > 30.0 { (lx / ll * 30.0, ly / ll * 30.0) } else { (lx, ly) };
    let (tx, ty) = (px + lx, py + ly);
    if !ctx.zone.line_clear(x0, y0, tx, ty) || ctx.zone.solid_tile_at(tx, ty) {
        return false;
    }
    // pick vz so it lands at about t_est, then simulate to get the exact landing tick
    let n = t_est.round();
    let z0 = mz;
    let vz0 = (GRAV * n * (n + 1.0) * 0.5 - z0) / n - 0.02;
    let (mut z, mut vz, mut ticks) = (z0, vz0, 0u32);
    while ticks < 200 {
        vz -= GRAV;
        z += vz;
        ticks += 1;
        if z <= 0.0 {
            break;
        }
    }
    let (vx, vy) = ((tx - x0) / ticks as f32, (ty - y0) / ticks as f32);
    let fx = &mut *ctx.fx;
    let mut p = Projectile::new(x0, y0, vx, vy, fx).z(z0, vz0, GRAV).color(ORA);
    p.r = GLOB_R;
    p.life = ticks + 30;
    p.light = 22.0;
    if let Some(a) = glob_anim() {
        p = p.look("ember_glob", a);
    }
    let g = Glob { id: p.id, tx, ty, t: 1, total: ticks, splashed: false, last: (x0, y0, z0, vz0) };
    mark(fx, &g);
    s.globs.push(g);
    ctx.projectiles.push(p);

    // the spit itself: flare, recoil, sparks and a smoke puff out of the maw
    s.heat = 1.25;
    fx.sound("ember_spit");
    fx.shake(0.05);
    let (dx, dy) = norm(tx - x0, ty - y0);
    for _ in 0..7 {
        let a = dy.atan2(dx) + fx.rng.signed() * 0.7;
        let sp = fx.rng.range(0.5, 1.6);
        fx.particles.add(
            Particle::new(x0, y0)
                .z(mz)
                .vel(a.cos() * sp, a.sin() * sp * 0.5, fx.rng.range(0.6, 1.8))
                .color(HOT)
                .fade_to(ORA)
                .glow(1.0)
                .grav(0.1)
                .drag(0.9)
                .life(fx.rng.range(10.0, 18.0))
                .streak(true),
        );
    }
    for _ in 0..4 {
        fx.particles.add(
            Particle::new(x0 + fx.rng.signed() * 2.0, y0)
                .z(mz + fx.rng.range(0.0, 3.0))
                .vel(dx * 0.3 + fx.rng.signed() * 0.15, 0.0, fx.rng.range(0.25, 0.5))
                .color(ASH)
                .fade_to(ASH_D)
                .grav(0.0)
                .drag(0.95)
                .life(fx.rng.range(24.0, 40.0)),
        );
    }
    true
}

/// Follow our globs in the engine's projectile list: mark their landing spots while they fly,
/// splash where they land, and crackle when the player slashes one out of the air.
fn track_globs(s: &mut State, ctx: &mut EnemyCtx) {
    let fx = &mut *ctx.fx;
    let mut i = 0;
    while i < s.globs.len() {
        let g = &mut s.globs[i];
        if let Some(p) = ctx.projectiles.iter_mut().find(|p| p.id == g.id) {
            if p.alive {
                g.last = (p.x, p.y, p.z, p.vz);
                // Enemies update before projectiles: show the splash on the damage tick.
                if p.z + p.vz - p.grav <= 0.0 {
                    splash(fx, g.tx, g.ty);
                    g.splashed = true;
                } else {
                    g.t = p.t + 1;
                    mark(fx, g);
                }
                i += 1;
                continue;
            }
            // A slashed glob must not land later in this same projectile update.
            p.life = 0;
        }
        let g = s.globs.swap_remove(i);
        if !g.splashed {
            let (x, y, z, _) = g.last;
            fx.sound_vol("deflect", 0.6);
            droplets(fx, x, y, z, 6, 0.8);
        }
    }
}

/// One-frame ground decal pixel (see ember_hound.rs: life 2 -> 1 before it's drawn).
fn flash_px(fx: &mut Fx, x: i32, y: i32, c: Rgb, glow: f32) {
    fx.particles.debris.push(Debris { x, y, c, glow, life: 2, max_life: 1 });
}

fn ground_px(fx: &mut Fx, x: i32, y: i32, c: Rgb, glow: f32, life: u32) {
    fx.particles.debris.push(Debris { x, y, c, glow, life, max_life: life });
}

/// Midpoint circle: calls `f` for each pixel of a 1-px ring.
fn ring(r: i32, mut f: impl FnMut(i32, i32)) {
    let (mut x, mut y, mut d) = (r, 0, 1 - r);
    while x >= y {
        for (a, b) in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)] {
            f(a, b);
        }
        y += 1;
        if d < 0 {
            d += 2 * y + 1;
        } else {
            x -= 1;
            d += 2 * (y - x) + 1;
        }
    }
}

/// The landing marker: a ring closing in on the splash radius, a dithered fill that thickens,
/// four reticle ticks and a center point. Dull red early, hot orange at the end.
fn mark(fx: &mut Fx, g: &Glob) {
    let p = (g.t as f32 / g.total.max(1) as f32).min(1.0);
    let (cx, cy) = (g.tx.round() as i32, g.ty.round() as i32);
    let r = (MARK_R + 9.0 * (1.0 - p) * (1.0 - p)).round() as i32;
    let (rc, rg) = if p > 0.8 {
        (ORA, 0.75)
    } else if p > 0.5 {
        (RED, 0.5)
    } else {
        (DRED, 0.35)
    };
    ring(r, |x, y| flash_px(fx, cx + x, cy + y, rc, rg));
    // fill: ordered dither, denser as it falls (unlit, so it reads as a dark-red stain)
    let fill_r = SPLASH_R as i32;
    let dens = 0.1 + 0.32 * p;
    for y in -fill_r..=fill_r {
        for x in -fill_r..=fill_r {
            if x * x + y * y <= fill_r * fill_r && bayer4(cx + x, cy + y) < dens {
                flash_px(fx, cx + x, cy + y, if p > 0.8 { DRED } else { LAVA0 }, 0.0);
            }
        }
    }
    // reticle ticks pointing in, outside the ring
    for (ux, uy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        for k in 2..4 {
            flash_px(fx, cx + ux * (r + k), cy + uy * (r + k), rc, rg * 0.8);
        }
    }
    flash_px(fx, cx, cy, if p > 0.8 { YEL } else { ORA }, 0.6);
}

/// Lava droplets thrown up from a point; they land and keep glowing as they cool.
fn droplets(fx: &mut Fx, x: f32, y: f32, z: f32, n: usize, power: f32) {
    for _ in 0..n {
        let (dx, dy) = fx.rng.dir();
        let sp = fx.rng.range(0.4, 1.4) * power;
        fx.particles.add(
            Particle::new(x, y)
                .z(z)
                .vel(dx * sp, dy * sp * 0.7, fx.rng.range(1.0, 2.6) * power)
                .color(if fx.rng.chance(0.3) { HOT } else { YEL })
                .fade_to(RED)
                .glow(1.0)
                .grav(0.16)
                .drag(0.96)
                .bounce(0.2)
                .life(fx.rng.range(70.0, 130.0))
                .settle(true),
        );
    }
}

fn splash(fx: &mut Fx, x: f32, y: f32) {
    fx.shake(0.1);
    fx.sound_vol("lava_hiss_2", 0.8);
    fx.sound_vol("lava_bubble_2", 0.5);
    // a low ring of embers skidding outward
    for i in 0..18 {
        let a = i as f32 / 18.0 * std::f32::consts::TAU + fx.rng.signed() * 0.15;
        let sp = fx.rng.range(1.4, 2.6);
        fx.particles.add(
            Particle::new(x, y)
                .z(fx.rng.range(0.5, 2.5))
                .vel(a.cos() * sp, a.sin() * sp, fx.rng.range(0.2, 0.7))
                .color(HOT)
                .fade_to(RED)
                .glow(1.0)
                .grav(0.08)
                .drag(0.86)
                .life(fx.rng.range(14.0, 22.0))
                .streak(true),
        );
    }
    droplets(fx, x, y, 2.0, 12, 1.0);
    // a gout of smoke
    for _ in 0..5 {
        fx.particles.add(
            Particle::new(x + fx.rng.signed() * 4.0, y + fx.rng.signed() * 2.0)
                .z(fx.rng.range(2.0, 6.0))
                .vel(fx.rng.signed() * 0.2, 0.0, fx.rng.range(0.3, 0.6))
                .color(ASH)
                .fade_to(ASH_D)
                .grav(0.0)
                .drag(0.95)
                .life(fx.rng.range(30.0, 48.0)),
        );
    }
    // the flash
    fx.particles.add(Particle::new(x, y).z(3.0).color(HOT).fade_to(ORA).glow(1.0).grav(0.0).life(12.0).light(44.0));
    // the splat: a glowing pool edge that cools, on a dark scorch
    let (cx, cy) = (x.floor() as i32, y.floor() as i32);
    for yy in -7..=7 {
        for xx in -9..=9 {
            let e = (xx * xx) as f32 / 81.0 + (yy * yy) as f32 / 49.0;
            if e > 1.0 {
                continue;
            }
            let n = hash2(cx + xx, cy + yy, 17);
            if e < 0.3 && n < 0.85 {
                ground_px(fx, cx + xx, cy + yy, if n < 0.2 { YEL } else { ORA }, 0.9, 60 + (n * 70.0) as u32);
            } else if e < 0.55 && n < 0.6 {
                ground_px(fx, cx + xx, cy + yy, RED, 0.8, 50 + (n * 60.0) as u32);
            } else if n < 0.55 - e * 0.3 {
                ground_px(fx, cx + xx, cy + yy, SCORCH, 0.0, 240 + (n * 120.0) as u32);
            }
        }
    }
}

pub fn on_hit(b: &mut Body, s: &mut State, hit: &Hit, ctx: &mut EnemyCtx) {
    b.kx = 0.0;
    b.ky = 0.0;
    let (dx, dy) = hit.push_dir(b.x, b.y);
    s.kill_dir = (dx, dy);
    s.wobble = if hit.heavy { 14 } else { 9 };
    s.wobble_dir = if dx >= 0.0 { 1 } else { -1 };
    let fx = &mut *ctx.fx;
    fx.sound_vol("pot_break", 0.35);
    // stone chips knocked off the far side
    let n = if hit.heavy { 9 } else { 5 };
    for _ in 0..n {
        let a = dy.atan2(dx) + fx.rng.signed() * 0.9;
        let sp = fx.rng.range(0.7, 1.9);
        let c = *fx.rng.pick(&[ST2, ST3, ST4, ST3, CU2]);
        fx.particles.add(
            Particle::new(b.x + dx * 4.0, b.y)
                .z(fx.rng.range(6.0, 22.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.7, fx.rng.range(0.6, 1.8))
                .color(c)
                .grav(0.17)
                .drag(0.96)
                .bounce(0.35)
                .life(fx.rng.range(200.0, 360.0))
                .settle(true),
        );
    }
    // a charge gets snuffed: the maw coughs smoke and cools
    if s.phase == Phase::Charge {
        snuff(b, s, fx, 70);
    }
}

/// Cancel a charge: the maw coughs smoke and a few drops, and cools off for `cooldown` ticks.
fn snuff(b: &Body, s: &mut State, fx: &mut Fx, cooldown: u32) {
    let (mx, mz) = maw();
    for _ in 0..6 {
        fx.particles.add(
            Particle::new(b.x + mx + fx.rng.signed() * 2.0, b.y + 1.0)
                .z(mz + fx.rng.signed() * 2.0)
                .vel(fx.rng.signed() * 0.4, 0.0, fx.rng.range(0.2, 0.6))
                .color(ASH)
                .fade_to(ASH_D)
                .grav(0.0)
                .drag(0.93)
                .life(fx.rng.range(24.0, 40.0)),
        );
    }
    droplets(fx, b.x + mx, b.y + 1.0, mz, 3, 0.6);
    fx.sound_vol("proj_fizzle", 0.7);
    s.heat = 0.0;
    s.shots_left = 0;
    s.cooldown = cooldown;
    set_phase(s, Phase::Cooldown);
}

pub fn on_death(b: &Body, s: &State, ctx: &mut EnemyCtx) {
    let (kx, ky) = s.kill_dir;
    // its globs still in the air fizzle out
    let mut fizzled = false;
    for g in &s.globs {
        if let Some(p) = ctx.projectiles.iter_mut().find(|p| p.id == g.id && p.alive) {
            p.alive = false;
            p.life = 0;
            let fx = &mut *ctx.fx;
            for _ in 0..4 {
                fx.particles.add(
                    Particle::new(p.x + fx.rng.signed() * 2.0, p.y)
                        .z(p.z)
                        .vel(fx.rng.signed() * 0.3, 0.0, fx.rng.range(0.1, 0.4))
                        .color(ASH)
                        .fade_to(ASH_D)
                        .grav(0.0)
                        .drag(0.95)
                        .life(30.0),
                );
            }
            droplets(fx, p.x, p.y, p.z, 3, 0.4);
            fizzled = true;
        }
    }
    let fx = &mut *ctx.fx;
    if fizzled {
        fx.sound_vol("proj_fizzle", 0.8);
    }
    fx.shake(0.32);
    fx.sound("pot_break");
    fx.sound_vol("lava_bubble_2", 0.9);

    // 1. the stones topple and crumble along the killing blow; upper stones fall farther
    let (cell, anchor) = current_cell(b, s);
    let first = fx.particles.list.len();
    fx.particles.burst_cell(&mut fx.rng, cell, anchor, false, b.x, b.y, 0.7, 650.0);
    let hmax = anchor.1.max(1) as f32;
    for p in &mut fx.particles.list[first..] {
        let up = (p.z / hmax).clamp(0.0, 1.0);
        let topple = 0.3 + up * 1.6;
        p.vx += kx * topple * fx.rng.range(0.5, 1.1);
        p.vy += ky * topple * 0.6;
        p.vz = fx.rng.range(0.1, 0.9) + up * 0.8;
        p.grav = 0.15;
        p.bounce = 0.25;
        if p.glow > 0.0 {
            p.c1 = RED;
            p.life = fx.rng.range(90.0, 170.0);
            p.max_life = p.life;
        }
    }
    // 2. the maw erupts one last time
    let (mx, mz) = maw();
    for _ in 0..26 {
        let a = fx.rng.f32() * std::f32::consts::TAU;
        let sp = fx.rng.range(0.2, 1.2);
        fx.particles.add(
            Particle::new(b.x + mx, b.y + 1.0)
                .z(mz)
                .vel(a.cos() * sp, a.sin() * sp * 0.6, fx.rng.range(1.6, 3.6))
                .color(HOT)
                .fade_to(ORA)
                .glow(1.0)
                .grav(0.1)
                .drag(0.95)
                .life(fx.rng.range(20.0, 40.0))
                .streak(true),
        );
    }
    droplets(fx, b.x + mx, b.y + 1.0, mz, 14, 1.2);
    for i in 0..6 {
        fx.particles.add(
            Particle::new(b.x + fx.rng.signed() * 5.0, b.y + fx.rng.signed() * 2.0)
                .z(mz + fx.rng.range(-6.0, 4.0))
                .vel(fx.rng.signed() * 0.12, 0.0, fx.rng.range(0.25, 0.5))
                .color(if i % 2 == 0 { HOT } else { YEL })
                .fade_to(DRED)
                .glow(1.0)
                .grav(0.0)
                .drag(0.985)
                .life(fx.rng.range(80.0, 130.0))
                .light(10.0),
        );
    }
    fx.particles.add(Particle::new(b.x, b.y + 1.0).z(mz).color(HOT).fade_to(ORA).glow(1.0).grav(0.0).life(14.0).light(44.0));
    // 3. dust rolling out from the base
    for i in 0..16 {
        let a = i as f32 / 16.0 * std::f32::consts::TAU;
        let sp = fx.rng.range(0.6, 1.3);
        fx.particles.add(
            Particle::new(b.x + a.cos() * 4.0, b.y + a.sin() * 2.0)
                .z(fx.rng.range(0.0, 3.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.6, fx.rng.range(0.1, 0.4))
                .color(ASH)
                .fade_to(ASH_D)
                .grav(0.02)
                .drag(0.9)
                .life(fx.rng.range(24.0, 40.0)),
        );
    }
    // 4. a lava pool where it stood, cooling
    let (cx, cy) = (b.x.floor() as i32, b.y.floor() as i32);
    for yy in -5..=5 {
        for xx in -8..=8 {
            let e = (xx * xx) as f32 / 64.0 + (yy * yy) as f32 / 25.0;
            if e > 1.0 {
                continue;
            }
            let n = hash2(cx + xx, cy + yy, 29);
            if e < 0.35 && n < 0.8 {
                ground_px(fx, cx + xx, cy + yy, if n < 0.25 { YEL } else { ORA }, 0.9, 120 + (n * 90.0) as u32);
            } else if e < 0.7 && n < 0.6 {
                ground_px(fx, cx + xx, cy + yy, RED, 0.7, 100 + (n * 80.0) as u32);
            } else if n < 0.7 {
                ground_px(fx, cx + xx, cy + yy, SCORCH, 0.0, 600);
            }
        }
    }
}

// ------------------------------------------------------------------ drawing

fn pick<'a>(sh: &'a Sheet, names: &[&str], dir: Dir) -> Option<(&'a Anim, bool)> {
    names.iter().find_map(|n| sh.dir_anim(n, dir))
}

/// The cell to draw now (cell, anchor).
fn current_cell(b: &Body, s: &State) -> (&'static Cell, (i32, i32)) {
    if let Some(sh) = sheet("spitting_totem") {
        let dir = if b.face.1 < -0.35 { Dir::Up } else { Dir::Down };
        let hurt = b.stun > 6 && sh.dir_anim("hurt", dir).is_some();
        let found = if hurt {
            pick(sh, &["hurt"], dir).map(|(a, _)| (a, ((1.0 - b.flash) / 0.12).round().max(0.0) as u32, false))
        } else {
            match s.phase {
                Phase::Charge => pick(sh, &["windup", "charge", "glow", "idle"], dir).map(|(a, _)| {
                    // stretch the windup over the charge so its last frame lands on the spit
                    let tt = if a.name.starts_with("idle") { s.t } else { s.pt * a.total_ticks() / s.charge_len.max(1) };
                    (a, tt, a.name.starts_with("idle"))
                }),
                Phase::Recoil => pick(sh, &["attack", "spit", "idle"], dir).map(|(a, _)| (a, s.pt, a.name.starts_with("idle"))),
                _ => pick(sh, &["idle"], dir).map(|(a, _)| (a, s.t, true)),
            }
        };
        if let Some((a, tt, looping)) = found.or_else(|| pick(sh, &["idle"], dir).map(|(a, _)| (a, s.t, true))) {
            return (super::variation::cell("spitting_totem", sh.cell(a, a.frame_at(tt, looping)), s.appearance), sh.anchor_of(a));
        }
    }
    let cells = fallback();
    let open = matches!(s.phase, Phase::Recoil) || (s.phase == Phase::Charge && s.pt * 2 > s.charge_len);
    let c = &cells[if open { 1 } else { 0 }];
    (c, (14, 33))
}

/// The maw position relative to the feet: (x offset, height). Taken from the centroid of the
/// lowest emissive cluster of the idle frame (the lava maw), so it follows the art.
fn maw() -> (f32, f32) {
    static MAW: OnceLock<(f32, f32)> = OnceLock::new();
    *MAW.get_or_init(|| {
        let (cell, anchor) = match sheet("spitting_totem").and_then(|sh| pick(sh, &["idle"], Dir::Down).map(|(a, _)| (sh.cell(a, 0), sh.anchor_of(a)))) {
            Some(v) => v,
            None => {
                let c = &fallback()[0];
                (c, (14, 33))
            }
        };
        if let Some(sh) = sheet("spitting_totem") {
            if let Some((a, _)) = pick(sh, &["idle"], Dir::Down) {
                if let Some((x, y)) = a.point("maw", 0) {
                    return ((x - anchor.0) as f32, (anchor.1 - y) as f32);
                }
            }
        }
        let lowest = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).emit)).unwrap_or(cell.h / 2);
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
        for y in (lowest - 4).max(0)..=lowest {
            for x in 0..cell.w {
                let p = cell.at(x, y);
                if p.a && p.emit {
                    sx += x as f32;
                    sy += y as f32;
                    n += 1.0;
                }
            }
        }
        if n == 0.0 {
            return (0.0, anchor.1 as f32 * 0.62);
        }
        (sx / n - anchor.0 as f32, anchor.1 as f32 - sy / n)
    })
}

/// The ember_glob anim to fly with, if the sheet exists.
fn glob_anim() -> Option<&'static str> {
    let sh = sheet("ember_glob")?;
    for n in ["fly", "spin", "loop", "idle", "glob"] {
        if let Some((k, _)) = sh.anims.get_key_value(n) {
            return Some(k.as_str());
        }
    }
    sh.anims.keys().min().map(|k| k.as_str())
}

/// Glowing cracks that open as it takes damage: short jagged walks over interior pixels,
/// placed deterministically per totem.
fn draw_cracks(f: &mut Frame, cell: &Cell, anchor: (i32, i32), sx: i32, sy: i32, s: &State, dmg: i32, pulse: f32) {
    let interior = |x: i32, y: i32| -> bool {
        let p = cell.at(x, y);
        p.a && !p.emit && cell.at(x - 1, y).a && cell.at(x + 1, y).a && cell.at(x, y - 1).a && cell.at(x, y + 1).a && p.c.luma() > 0.06
    };
    let ox = sx - anchor.0;
    let oy = sy - anchor.1;
    for k in 0..dmg.min(5) {
        // find a start pixel
        let mut start = None;
        for tries in 0..40 {
            let x = (hash2(s.seed as i32, k * 97 + tries, 1) * cell.w as f32) as i32;
            let y = (hash2(s.seed as i32, k * 97 + tries, 2) * cell.h as f32 * 0.8) as i32 + 1;
            if interior(x, y) {
                start = Some((x, y));
                break;
            }
        }
        let Some((mut x, mut y)) = start else {
            continue;
        };
        let steps = 4 + (hash2(s.seed as i32, k, 3) * 4.0) as i32;
        let mut dir = if hash2(s.seed as i32, k, 4) < 0.5 { -1 } else { 1 };
        for st in 0..steps {
            let c = if st == 0 || st == steps - 1 { DRED.lerp(RED, pulse) } else { RED.lerp(ORA, pulse) };
            f.put_glow(ox + x, oy + y, c.scale(0.8 + 0.2 * pulse));
            // a dark lip under the crack for depth
            if cell.at(x, y + 1).a && !cell.at(x, y + 1).emit {
                f.put(ox + x, oy + y + 1, ST0);
            }
            let r = hash2(s.seed as i32, k * 31 + st, 5);
            if r < 0.35 {
                x += dir;
            } else if r < 0.45 {
                dir = -dir;
            }
            y += 1;
            if !interior(x, y) {
                break;
            }
        }
    }
}

pub fn draw(b: &Body, s: &State, f: &mut Frame) {
    f.shadow(f.sx(b.x), f.sy(b.y), 7, 2, 0.45);
    let (cell, anchor) = current_cell(b, s);
    let trembling = s.phase == Phase::Charge && s.charge_len.saturating_sub(s.pt) < TREMBLE;
    let mut jx = 0;
    if trembling && (s.pt / 2) % 2 == 0 {
        jx = 1;
    }
    if s.wobble > 0 {
        jx = if (s.wobble / 2) % 2 == 0 { s.wobble_dir } else { 0 };
    }
    let flick = 0.06 * ((s.t as f32 * 0.37).sin() + (s.t as f32 * 0.91).sin() * 0.5);
    let heat = (s.heat + flick).max(0.0);
    let gain = 0.75 + heat * 0.85;
    let (x, y) = (b.x + jx as f32, b.y);
    let o = DrawOpts { actor: true, foot_depth: 5, foot_half_w: b.r, ground_y: Some(b.y.floor()), elevation: b.z, flash: snappy(b.flash) * 0.7, emit_gain: gain, ..Default::default() };
    f.sprite_w(cell, anchor, x, y, &o);
    let (sx, sy) = (f.sx(x), f.sy(y));

    // damage cracks
    let dmg = b.max_hp - b.hp;
    if dmg > 0 {
        let pulse = 0.5 + 0.5 * (s.t as f32 * 0.12).sin();
        draw_cracks(f, cell, anchor, sx, sy, s, (dmg + 1) / 2, pulse.max(heat.min(1.0)));
    }

    // white-hot core in the maw at the end of a charge and on the spit
    let (mx, mz) = maw();
    let (cx, cy) = (sx + mx.round() as i32, sy - mz.round() as i32);
    if heat > 0.75 {
        f.put_glow(cx, cy, HOT.lerp(YEL, ((heat - 0.75) * 2.0).min(1.0)));
        if heat > 1.0 {
            f.put_glow(cx - 1, cy, HOT);
            f.put_glow(cx + 1, cy, HOT);
        }
    }

    // The furnace mouth becomes a readable light on nearby stone throughout
    // its real charge/spit/cooldown. The source sits in this pose's glowing maw,
    // rather than inside the opaque lower plinth.
    if f.surface_lighting {
        if let Some((lx, ly)) = maw_source(cell, anchor) {
            let r = 40.0 + heat.min(1.6) * 18.0;
            let i = 0.28 + heat.min(1.6) * 0.68;
            let color = ORA.lerp(YEL, ((heat - 0.6) / 0.9).clamp(0.0, 1.0));
            f.add_light_3d_w(x.floor() + lx, y.floor(), -ly, r, color, i);
        }
    } else {
        let r = 12.0 + heat.min(1.6) * 18.0;
        let i = 0.12 + heat.min(1.6) * 0.26;
        f.add_light_3d_w(x + mx, y, mz * 0.5, r, ORA, i);
        if heat > 0.6 {
            f.add_light_3d_w(x + mx, y, mz - 2.0, 9.0, YEL, (heat - 0.6) * 0.35);
        }
    }
}

/// Brightest actual mouth pixel, sampled from the current appearance and pose.
fn maw_source(cell: &Cell, anchor: (i32, i32)) -> Option<(f32, f32)> {
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

// ------------------------------------------------------------------ fallback art

fn fallback() -> &'static [Cell; 2] {
    static CELLS: OnceLock<[Cell; 2]> = OnceLock::new();
    CELLS.get_or_init(|| {
        let pal = [
            ('#', rgb(0x292c3c)),
            ('B', rgb(0x3f414f)),
            ('C', rgb(0x56505d)),
            ('D', rgb(0x79707a)),
            ('E', rgb(0x9a8e92)),
            ('O', rgb(0xd48670)),
            ('R', rgb(0xa36059)),
            ('W', rgb(0xefb68a)),
            ('Y', rgb(0xefb68a)),
            ('r', rgb(0x6f454b)),
        ];
        let em = ['O', 'Y', 'W'];
        let pose_0 = Cell::from_ascii(
            &[
                "............................",
                "............................",
                "............................",
                "...........##...............",
                "..........#DD#..............",
                ".........#DEDC#.............",
                "........#DEDDCC#............",
                "........#DCDDCC#............",
                ".......#DCC##CCC#...........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB##.........",
                "......#DCCBBBBBCCCB#........",
                "......#DCB#######CB#........",
                "......#DCBO#####OCB#........",
                "......#DCBOOOOOOOCB#........",
                "......#DCCBBBBBBCCB#........",
                ".......#DCCCCCCCB##.........",
                ".......#DCBrrrCCB#..........",
                ".......#DCBrrRCCB#..........",
                ".......#DCBrrrCCB#..........",
                ".......#DCBrrrCCB#..........",
                ".......#DCCBBCCCB#..........",
                ".......#DCCBCCCCB#..........",
                ".......#DCCBCCCCB#..........",
                "......#DCCCCCCCCCB#.........",
                ".....#DDCCCCCCCCCCB#........",
                ".....#CCCCCCBBBBBBB#........",
                "......#############.........",
                "............................",
            ],
            &pal,
            &em,
        );
        let pose_1 = Cell::from_ascii(
            &[
                "............................",
                "............................",
                "............................",
                "...........##...............",
                "..........#DD#..............",
                ".........#DEDC#.............",
                "........#DEDDCC#............",
                "........#DCDDCC#............",
                ".......#DCC##CCC#...........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB#..........",
                ".......#DCB##CCCB##.........",
                "......#DCCBBBBBCCCB#........",
                ".....#DDCB#rRRRr#CCB#.......",
                ".....#DCB#ROYYYOR#CB#.......",
                ".....#DCCBOYWWWYOBCB#.......",
                "......#DCCBBBBBBCCB#........",
                ".......#DCCCCCCCB##.........",
                ".......#DCBrrOCCB#..........",
                ".......#DCBrrRCCB#..........",
                ".......#DCBrrrCCB#..........",
                ".......#DCBrrrCCB#..........",
                ".......#DCCBBCCCB#..........",
                ".......#DCCBCCCCB#..........",
                ".......#DCCBCCCCB#..........",
                "......#DCCCCCCCCCB#.........",
                ".....#DDCCCCCCCCCCB#........",
                ".....#CCCCCCBBBBBBB#........",
                "......#############.........",
                "............................",
            ],
            &pal,
            &em,
        );
        [pose_0, pose_1]
    })
}

/// The body's hit flash (1 -> 0 over ~8 ticks) made snappier: full white on the hit frames, gone
/// after about five ticks, so the hit reads as a pop instead of a lingering white blob.
fn snappy(flash: f32) -> f32 {
    ((flash - 0.35) / 0.65).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The last idle pose deliberately closes its glowing outlet. A dark
    /// authored frame must extinguish its source, then relight with real ink.
    #[test]
    fn native_dark_idle_does_not_invent_a_light_and_luminous_idle_relights() {
        use crate::gfx::frame::Surface;
        use crate::gfx::W;
        let sh = sheet("spitting_totem").unwrap();
        for appearance in 0..3 {
            for direction in ["idle_down", "idle_up"] {
                let (mut b, mut s) = spawn(192.73, 120.81);
                b.face = (0.0, if direction == "idle_up" { -1.0 } else { 1.0 });
                s.appearance = appearance;
                s.phase = Phase::Cooldown;
                let a = sh.anim(direction).unwrap();
                s.t = a.durations[..3].iter().sum();
                let (cell, anchor) = current_cell(&b, &s);
                assert!(std::ptr::eq(cell, super::super::variation::cell("spitting_totem", sh.cell(a, 3), appearance)));
                assert!(maw_source(cell, anchor).is_none(), "fixture must use the real dark pose");
                let mut f = Frame::new();
                f.begin(Rgb::gray(0.5), Rgb::BLACK);
                f.surface_lighting = true;
                draw(&b, &s, &mut f);
                assert!(f.point_lights.is_empty(), "dark {direction} appearance{appearance} invented a source");
                s.t = 0;
                f.begin(Rgb::gray(0.5), Rgb::BLACK);
                f.surface_lighting = true;
                f.surfaces.fill(Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 });
                draw(&b, &s, &mut f);
                assert_eq!(f.point_lights.len(), 1);
                let light = f.point_lights[0];
                assert!(f.emissive[light.y.floor() as usize * W + light.x.floor() as usize].max_c() > 0.0,
                    "relit source must remain on fractional {direction} appearance{appearance} raster ink");
                assert!(crate::gfx::lighting::point_visibility(&f, light).iter().all(|&v| v == 255),
                    "relit {direction} appearance{appearance} blocked its own source");
            }
        }
    }

    /// Fallback outlet pixels retain the same launch cluster and charge pose as the real sheet.
    #[test]
    fn fallback_outlet_matches_authored_sheet() {
        let sh = sheet("spitting_totem").expect("production totem sheet");
        for (i, name, frame) in [(0, "idle_down", 0), (1, "windup_down", 2)] {
            let expected = sh.cell(sh.anim(name).expect("totem animation"), frame);
            let actual = &fallback()[i];
            assert_eq!((actual.w, actual.h), (expected.w, expected.h));
            for y in 0..actual.h {
                for x in 0..actual.w {
                    let (a, b) = (actual.at(x, y), expected.at(x, y));
                    assert_eq!((a.a, a.emit, a.c), (b.a, b.emit, b.c), "{name} pixel {x},{y}");
                }
            }
        }
    }
    use crate::world::{Zone, ZoneId};

    /// A lob lands on its marked center and produces its splash on the damage tick.
    #[test]
    fn glob_marker_matches_actual_landing() {
        let zone = Zone::new(ZoneId::Cinder, 32, 32);
        let mut fx = Fx::new(1);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx { player: (320.0, 270.0), player_alive: true, zone: &zone, fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 0 };
        let (b, mut s) = spawn(256.0, 256.0);
        assert!(spit(&b, &mut s, &mut ctx));
        let marker = s.globs[0];
        assert!(!ctx.fx.particles.debris.is_empty(), "Mark the target on launch");
        ctx.fx.sounds.clear();
        for tick in 1..=marker.total {
            if tick > 1 {
                track_globs(&mut s, &mut ctx);
            }
            let h = ctx.projectiles[0].update(&zone, ctx.fx, ctx.player);
            if tick < marker.total {
                assert!(h.is_none());
                assert!(!s.globs[0].splashed);
            } else {
                let hit = h.expect("The lob should deal damage on the predicted tick");
                let (x, y) = hit.shape.center();
                assert!((x - marker.tx).abs() < 0.001 && (y - marker.ty).abs() < 0.001);
                assert!(s.globs[0].splashed);
                assert!(ctx.fx.sounds.iter().any(|s| s.name == "lava_hiss_2"));
            }
        }
    }

    /// A slash on the last airborne tick neutralizes the impending landing hit.
    #[test]
    fn slashed_glob_cannot_splash_damage() {
        let zone = Zone::new(ZoneId::Cinder, 32, 32);
        let mut fx = Fx::new(1);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx { player: (320.0, 270.0), player_alive: true, zone: &zone, fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 0 };
        let (b, mut s) = spawn(256.0, 256.0);
        assert!(spit(&b, &mut s, &mut ctx));
        let p = &mut ctx.projectiles[0];
        p.z = 0.1;
        p.vz = -1.0;
        p.deflect(ctx.fx);
        track_globs(&mut s, &mut ctx);
        assert!(ctx.projectiles[0].update(&zone, ctx.fx, ctx.player).is_none());
        assert!(s.globs.is_empty());
    }

    /// Breaking the totem safely extinguishes even a glob about to land.
    #[test]
    fn death_cancels_pending_glob_damage() {
        let zone = Zone::new(ZoneId::Cinder, 32, 32);
        let mut fx = Fx::new(1);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx { player: (320.0, 270.0), player_alive: true, zone: &zone, fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 0 };
        let (b, mut s) = spawn(256.0, 256.0);
        assert!(spit(&b, &mut s, &mut ctx));
        ctx.projectiles[0].z = 0.1;
        ctx.projectiles[0].vz = -1.0;
        on_death(&b, &s, &mut ctx);
        assert!(!ctx.projectiles[0].alive);
        assert!(ctx.projectiles[0].update(&zone, ctx.fx, ctx.player).is_none());
    }
}
