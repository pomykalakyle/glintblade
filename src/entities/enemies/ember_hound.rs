//! Ember hound: an obsidian dog with lava running in its cracks.
//!
//! Behavior, as a loop:
//! 1. Prowl: circles the player at a preferred distance (direction flips off walls), shedding
//!    the odd ember from its back.
//! 2. Crouch (the telegraph, 34 ticks): plants, sinks, its cracks heat up and a chevron lane
//!    appears on the ground. It tracks the player for the first 22 ticks, then the aim LOCKS:
//!    an eye glint pings, the lane brightens and the body shivers. 12 ticks later it goes.
//! 3. Lunge: a fast leap along the locked lane that overshoots the player a little, leaving
//!    afterimages and a trail of scorched, glowing embers on the ground.
//! 4. Skid, then Recover: it lands, skids, and pants with its cracks cooled and dull (the
//!    punish window). Slamming into a wall mid-lunge dazes it for longer.
//!
//! Any hit staggers it and cancels a crouch. Death bursts it into pixels along the killing
//! blow, with the hot crack pixels flying as embers, a few slow rising cinders and a burn mark.
//!
//! Art: the `ember_hound` sheet (idle/move/windup/attack/hurt, `_right` mirrored for left;
//! `_down`/`_up` used when present). Without the sheet, a hand-placed fallback is drawn.

use super::EnemyCtx;
use crate::combat::{Hit, Shape, Team};
use crate::entities::Body;
use crate::fx::{Debris, Fx, Particle};
use crate::gfx::{Anim, Cell, Dir, DrawOpts, Frame, Rgb, Sheet, rgb, sheet};
use crate::util::{angle_diff, approach, hash2, norm};
use std::collections::HashMap;
use std::sync::OnceLock;

const HP: i32 = 4;
const RADIUS: f32 = 6.0;

const PROWL_SPEED: f32 = 1.05;
const CHASE_SPEED: f32 = 1.35;
const CROUCH_TICKS: u32 = 34;
/// Aim stops tracking at this tick of the crouch.
const LOCK_AT: u32 = 22;
const LUNGE_SPEED: f32 = 5.0;
const SKID_TICKS: u32 = 14;
const RECOVER_TICKS: u32 = 36;
const BONK_TICKS: u32 = 56;
/// Lunge hit capsule radius (the player's own hurt radius adds 4).
const BITE_R: f32 = 5.0;

// Palette (cinder ramps: obsidian body, lava cracks).
const HOT: Rgb = rgb(0xefb68a);
const YEL: Rgb = rgb(0xefb68a);
const ORA: Rgb = rgb(0xd48670);
const RED: Rgb = rgb(0xa36059);
const DRED: Rgb = rgb(0x6f454b);
const ASH: Rgb = rgb(0x6f616b);
const ASH_PALE: Rgb = rgb(0xa79a98);
const SCORCH: Rgb = rgb(0x292c3c);
const SCORCH_2: Rgb = rgb(0x363340);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Prowl,
    Crouch,
    Lunge,
    Skid,
    Recover,
    Stagger,
}

/// One short, dithered echo of the hound's airborne pose.
#[derive(Clone, Debug)]
struct Ghost {
    x: f32,
    y: f32,
    cell: &'static Cell,
    anchor: (i32, i32),
    flip: bool,
    life: f32,
}

#[derive(Clone, Debug)]
pub struct State {
    pub phase: Phase,
    /// Ticks in the current phase.
    pub pt: u32,
    pub t: u32,
    /// +1 / -1: which way it circles.
    spin: f32,
    /// Preferred circling distance.
    radius: f32,
    /// Prowl ticks before the next crouch.
    next_attack: u32,
    /// Lunge direction (unit) and length.
    aim: (f32, f32),
    lunge_len: f32,
    lunge_ticks: u32,
    recover_ticks: u32,
    hit_id: u32,
    /// Facing left (for side-view sheets).
    left: bool,
    prev: (f32, f32),
    /// Crack heat: 0 normal, 1 white-hot (telegraph), negative = cooled after a lunge.
    heat: f32,
    /// Ticks of the eye glint (counts down).
    glint: u32,
    /// Direction of the last hit (death burst follows it).
    kill_dir: (f32, f32),
    /// Afterimages retain the exact launch pose and direction.
    ghosts: Vec<Ghost>,
    flip_cd: u32,
    seed: u32,
    appearance: u8,
    /// Has it reacted to spotting the player yet?
    noticed: bool,
}

pub fn spawn(x: f32, y: f32) -> (Body, State) {
    let seed = (hash2(x as i32, y as i32, 77) * 1_000_000.0) as u32;
    let h = |k: u32| hash2(seed as i32, k as i32, 91);
    let s = State {
        phase: Phase::Prowl,
        pt: 0,
        t: (h(1) * 60.0) as u32,
        spin: if h(2) < 0.5 { 1.0 } else { -1.0 },
        radius: 58.0 + h(3) * 18.0,
        next_attack: 70 + (h(4) * 50.0) as u32,
        aim: (1.0, 0.0),
        lunge_len: 60.0,
        lunge_ticks: 12,
        recover_ticks: RECOVER_TICKS,
        hit_id: 0,
        left: h(5) < 0.5,
        prev: (x, y),
        heat: 0.0,
        glint: 0,
        kill_dir: (0.0, 1.0),
        ghosts: Vec::new(),
        flip_cd: 0,
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
    s.flip_cd = s.flip_cd.saturating_sub(1);
    s.glint = s.glint.saturating_sub(1);
    for g in s.ghosts.iter_mut() {
        g.life -= 0.2;
    }
    s.ghosts.retain(|g| g.life > 0.0);
    let (px, py) = ctx.player;
    let d = b.dist_to(px, py);
    let to = b.dir_to(px, py);
    s.prev = (b.x, b.y);
    if !s.noticed {
        // ears up: a little hop and a yip when it first spots Glint
        s.noticed = true;
        b.vz = 1.7;
        b.z = 0.1;
        b.face = to;
        s.left = to.0 < 0.0;
        s.heat = 0.5;
        s.next_attack += 20;
        ctx.fx.sound_vol("enemy_alert", 0.7);
        back_ember(b, s, ctx.fx);
        back_ember(b, s, ctx.fx);
    }

    if b.stun > 0 && s.phase != Phase::Stagger {
        set_phase(s, Phase::Stagger);
    }

    let mut blocked = false;
    match s.phase {
        Phase::Prowl => {
            s.heat = approach(s.heat, 0.0, 0.05);
            let away = if d > 0.01 { ((b.x - px) / d, (b.y - py) / d) } else { (1.0, 0.0) };
            let tang = (-away.1 * s.spin, away.0 * s.spin);
            let err = ((s.radius - d) / s.radius).clamp(-1.0, 1.0);
            let far = d > s.radius + 46.0;
            let (wx, wy) = if far { (-away.0, -away.1) } else { norm(tang.0 + away.0 * err * 2.2, tang.1 + away.1 * err * 2.2) };
            let sp = if far { CHASE_SPEED } else { PROWL_SPEED };
            let want = if ctx.player_alive { (wx * sp, wy * sp) } else { (0.0, 0.0) };
            b.vx = approach(b.vx, want.0, 0.14);
            b.vy = approach(b.vy, want.1, 0.14);
            let v = (b.vx * b.vx + b.vy * b.vy).sqrt();
            b.face = if v > 0.25 { norm(b.vx, b.vy) } else { to };
            // the odd ember off its back while it trots
            if v > 0.4 && s.t % 11 == 0 {
                back_ember(b, s, ctx.fx);
            }
            if s.pt >= s.next_attack && ctx.player_alive && (30.0..118.0).contains(&d) && ctx.zone.line_clear(b.x, b.y - 3.0, px, py - 3.0) {
                set_phase(s, Phase::Crouch);
                s.aim = to;
                b.face = to;
                ctx.fx.sound_vol("lava_bubble_2", 0.55);
            }
        }
        Phase::Crouch => {
            b.vx *= 0.7;
            b.vy *= 0.7;
            let k = s.pt as f32 / CROUCH_TICKS as f32;
            s.heat = approach(s.heat, 0.25 + k * 0.9, 0.2);
            if s.pt < LOCK_AT {
                // track the player with a limited turn rate
                let cur = s.aim.1.atan2(s.aim.0);
                let want = to.1.atan2(to.0);
                let a = cur + angle_diff(cur, want).clamp(-0.12, 0.12);
                s.aim = (a.cos(), a.sin());
                s.lunge_len = (d + 18.0).clamp(48.0, 104.0);
            } else if s.pt == LOCK_AT {
                s.glint = 10;
            }
            // the generic attack tell is 24 ticks long and ends where the lunge starts
            if s.pt == CROUCH_TICKS - 24 {
                ctx.fx.sound_vol("enemy_windup", 0.8);
            }
            b.face = s.aim;
            // scrabbling hind paws
            if s.pt % 5 == 0 {
                let fx = &mut *ctx.fx;
                fx.particles.dust(&mut fx.rng, b.x - s.aim.0 * 7.0, b.y + 1.0, 1, ASH);
            }
            if s.pt % 4 == 0 && s.pt > 8 {
                back_ember(b, s, ctx.fx);
            }
            lane_decal(b, s, ctx.fx);
            if s.pt >= CROUCH_TICKS {
                launch(b, s, ctx.fx);
            }
        }
        Phase::Lunge => {
            b.vx = s.aim.0 * LUNGE_SPEED;
            b.vy = s.aim.1 * LUNGE_SPEED;
            b.face = s.aim;
            s.heat = approach(s.heat, 1.0, 0.3);
            if s.pt % 3 == 1 {
                let (cell, anchor, flip) = current_cell(b, s);
                s.ghosts.push(Ghost { x: b.x, y: b.y - b.z, cell, anchor, flip, life: 0.8 });
            }
            lunge_embers(b, s, ctx.fx);
            if s.pt >= s.lunge_ticks {
                set_phase(s, Phase::Skid);
                b.vx = s.aim.0 * 2.6;
                b.vy = s.aim.1 * 2.6;
                land(b, s, ctx.fx);
            }
        }
        Phase::Skid => {
            b.vx *= 0.82;
            b.vy *= 0.82;
            s.heat = approach(s.heat, -0.4, 0.08);
            if s.pt % 2 == 0 && b.vx.abs() + b.vy.abs() > 0.4 {
                let fx = &mut *ctx.fx;
                fx.particles.dust(&mut fx.rng, b.x + s.aim.0 * 5.0, b.y, 1, ASH);
                scorch(fx, b.x + s.aim.0 * 4.0, b.y, 1, 70);
            }
            if s.pt >= SKID_TICKS {
                set_phase(s, Phase::Recover);
                s.recover_ticks = RECOVER_TICKS;
            }
        }
        Phase::Recover => {
            b.vx *= 0.7;
            b.vy *= 0.7;
            s.heat = approach(s.heat, -0.55, 0.06);
            // panting: ash puffs from the muzzle
            if s.pt % 12 == 3 {
                let fx = &mut *ctx.fx;
                let mx = b.x + if s.left { -9.0 } else { 9.0 };
                fx.particles.add(
                    Particle::new(mx, b.y)
                        .z(8.0)
                        .vel(if s.left { -0.25 } else { 0.25 }, 0.0, 0.35)
                        .color(ASH_PALE)
                        .fade_to(ASH)
                        .grav(0.0)
                        .drag(0.94)
                        .life(26.0),
                );
            }
            if s.pt >= s.recover_ticks {
                set_phase(s, Phase::Prowl);
                s.next_attack = 50 + ctx.fx.rng.irange(0, 50) as u32;
                if ctx.fx.rng.chance(0.4) {
                    s.spin = -s.spin;
                }
            }
        }
        Phase::Stagger => {
            b.vx *= 0.6;
            b.vy *= 0.6;
            s.heat = approach(s.heat, 0.1, 0.1);
            if b.stun == 0 {
                set_phase(s, Phase::Prowl);
                s.spin = -s.spin;
                s.next_attack = 34 + ctx.fx.rng.irange(0, 36) as u32;
            }
        }
    }

    let moving_fast = s.phase == Phase::Lunge;
    if b.integrate(ctx.zone) {
        blocked = true;
    }
    if blocked {
        match s.phase {
            Phase::Lunge if s.pt > 1 => bonk(b, s, ctx.fx),
            Phase::Prowl if s.flip_cd == 0 => {
                s.spin = -s.spin;
                s.flip_cd = 40;
            }
            _ => {}
        }
    }
    if moving_fast && s.phase == Phase::Lunge {
        ctx.hits.push(
            Hit::new(Shape::Capsule { x0: s.prev.0, y0: s.prev.1 - 2.0, x1: b.x, y1: b.y - 2.0, r: BITE_R }, Team::Enemy, 1, s.hit_id)
                .knock(3.2)
                .dir(s.aim.0, s.aim.1),
        );
    }
    if b.face.0.abs() > 0.2 {
        s.left = b.face.0 < 0.0;
    }
}

fn launch(b: &mut Body, s: &mut State, fx: &mut Fx) {
    set_phase(s, Phase::Lunge);
    s.hit_id = fx.hit_id();
    s.lunge_ticks = ((s.lunge_len / LUNGE_SPEED).round() as u32).clamp(8, 22);
    // a leap whose landing lines up with the end of the lunge
    b.vz = 0.25 * s.lunge_ticks as f32 * 0.5;
    b.z = 0.01;
    s.heat = 1.0;
    fx.sound("hound_lunge");
    fx.shake(0.06);
    // kick-off: dust and a puff of embers behind
    let (bx, by) = (b.x - s.aim.0 * 6.0, b.y - s.aim.1 * 6.0);
    fx.particles.dust(&mut fx.rng, bx, by, 6, ASH);
    for _ in 0..6 {
        let a = (-s.aim.1).atan2(-s.aim.0) + fx.rng.signed() * 0.8;
        let sp = fx.rng.range(0.8, 1.8);
        fx.particles.add(
            Particle::new(bx, by)
                .z(fx.rng.range(2.0, 7.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.6, fx.rng.range(0.3, 1.2))
                .color(HOT)
                .fade_to(RED)
                .glow(1.0)
                .grav(0.06)
                .drag(0.9)
                .life(fx.rng.range(14.0, 24.0))
                .streak(true),
        );
    }
    scorch(fx, bx, by, 3, 90);
}

fn land(b: &mut Body, s: &mut State, fx: &mut Fx) {
    b.z = 0.0;
    b.vz = 0.0;
    fx.particles.dust(&mut fx.rng, b.x, b.y, 5, ASH);
    fx.sound_vol("step_stone_2", 0.7);
    for _ in 0..5 {
        let side = if fx.rng.chance(0.5) { 1.0 } else { -1.0 };
        let (nx, ny) = (-s.aim.1 * side, s.aim.0 * side);
        let sp = fx.rng.range(0.6, 1.4);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(1.0)
                .vel(nx * sp + s.aim.0 * 0.8, ny * sp + s.aim.1 * 0.8, fx.rng.range(0.4, 1.0))
                .color(YEL)
                .fade_to(RED)
                .glow(1.0)
                .grav(0.1)
                .drag(0.9)
                .life(fx.rng.range(12.0, 20.0))
                .streak(true),
        );
    }
}

fn bonk(b: &mut Body, s: &mut State, fx: &mut Fx) {
    set_phase(s, Phase::Recover);
    s.recover_ticks = BONK_TICKS;
    b.kx = -s.aim.0 * 1.8;
    b.ky = -s.aim.1 * 1.8;
    b.vx = 0.0;
    b.vy = 0.0;
    b.vz = 1.4;
    b.z = b.z.max(0.5);
    b.flash = 0.6;
    s.heat = -0.2;
    fx.shake(0.18);
    fx.sound("hit_2");
    fx.particles.dust(&mut fx.rng, b.x + s.aim.0 * 6.0, b.y + s.aim.1 * 6.0, 8, ASH);
    fx.particles.sparks(&mut fx.rng, b.x + s.aim.0 * 6.0, b.y + s.aim.1 * 6.0, 6.0, -s.aim.0, -s.aim.1, 7, ORA, 2.0);
}

/// One ember shed from the hound's back: rises, drifts back, cools.
fn back_ember(b: &Body, s: &mut State, fx: &mut Fx) {
    let back = if s.left { 1.0 } else { -1.0 };
    let x = b.x + back * fx.rng.range(1.0, 7.0);
    fx.particles.add(
        Particle::new(x, b.y)
            .z(b.z + fx.rng.range(7.0, 10.0))
            .vel(back * fx.rng.range(0.05, 0.3), fx.rng.signed() * 0.05, fx.rng.range(0.2, 0.45))
            .color(YEL)
            .fade_to(DRED)
            .glow(1.0)
            .grav(0.0)
            .drag(0.97)
            .life(fx.rng.range(26.0, 44.0)),
    );
}

/// Lunge trail: embers spraying off the body and a scorched, glowing streak on the ground.
fn lunge_embers(b: &Body, s: &mut State, fx: &mut Fx) {
    for _ in 0..2 {
        let a = (-s.aim.1).atan2(-s.aim.0) + fx.rng.signed() * 0.6;
        let sp = fx.rng.range(0.4, 1.2);
        let hot = fx.rng.chance(0.4);
        fx.particles.add(
            Particle::new(b.x + fx.rng.signed() * 3.0, b.y)
                .z(b.z + fx.rng.range(4.0, 10.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.6, fx.rng.range(0.1, 0.6))
                .color(if hot { HOT } else { YEL })
                .fade_to(DRED)
                .glow(1.0)
                .grav(0.03)
                .drag(0.93)
                .life(fx.rng.range(16.0, 30.0)),
        );
    }
    // the ground streak (two paw tracks), cooling over ~2 s
    let (nx, ny) = (-s.aim.1, s.aim.0);
    for side in [-2.0f32, 2.0] {
        let (x, y) = (b.x + nx * side, b.y + ny * side);
        if fx.rng.chance(0.75) {
            let c = if fx.rng.chance(0.3) { ORA } else { RED };
            let life = 110 + fx.rng.irange(0, 50) as u32;
            ember_px(fx, x.floor() as i32, y.floor() as i32, c, 0.9, life);
        }
    }
}

/// A glowing ground pixel that dims and dither-fades.
fn ember_px(fx: &mut Fx, x: i32, y: i32, c: Rgb, glow: f32, life: u32) {
    fx.particles.debris.push(Debris { x, y, c, glow, life, max_life: life });
}

/// A dark scorch mark (unlit ground decal).
fn scorch(fx: &mut Fx, x: f32, y: f32, r: i32, life: u32) {
    let (cx, cy) = (x.floor() as i32, y.floor() as i32);
    for dy in -r..=r {
        for dx in -r - 1..=r + 1 {
            let e = (dx * dx) as f32 / ((r + 1) * (r + 1)) as f32 + (dy * dy) as f32 / (r * r).max(1) as f32;
            if e <= 1.0 && fx.rng.chance(0.8 - e * 0.4) {
                let c = if e < 0.4 { SCORCH } else { SCORCH_2 };
                fx.particles.debris.push(Debris { x: cx + dx, y: cy + dy, c, glow: 0.0, life, max_life: life });
            }
        }
    }
}

/// A ground decal that shows for exactly one frame (pushed during update; the particle update
/// later this tick takes life 2 -> 1, so it draws at full strength and is gone next tick).
fn flash_px(fx: &mut Fx, x: i32, y: i32, c: Rgb, glow: f32) {
    fx.particles.debris.push(Debris { x, y, c, glow, life: 2, max_life: 1 });
}

/// The crouch telegraph on the ground: chevrons pointing along the lunge, scrolling forward.
/// Faint while it's still aiming; bright and solid once the aim locks.
fn lane_decal(b: &Body, s: &State, fx: &mut Fx) {
    if s.pt < 6 {
        return;
    }
    let locked = s.pt >= LOCK_AT;
    let (ax, ay) = s.aim;
    let (nx, ny) = (-ay, ax);
    let len = if locked { s.lunge_len } else { s.lunge_len * (s.pt as f32 / LOCK_AT as f32).min(1.0) };
    let spacing = 9.0;
    let scroll = (s.pt as f32 * if locked { 0.9 } else { 0.4 }) % spacing;
    let last = CROUCH_TICKS - s.pt < 5;
    let (c, glow) = if last {
        (YEL, 0.7)
    } else if locked {
        (ORA, 0.5)
    } else {
        (DRED, 0.25)
    };
    let mut dist = 10.0 + scroll;
    while dist < len {
        let (tx, ty) = (b.x + ax * dist, b.y + ay * dist);
        let arm = 3.0;
        for k in 0..=3 {
            let kf = k as f32;
            for side in [-1.0f32, 1.0] {
                if k == 0 && side > 0.0 {
                    continue;
                }
                let x = tx - ax * kf + nx * kf * side;
                let y = ty - ay * kf + ny * kf * side;
                if kf <= arm {
                    flash_px(fx, x.round() as i32, y.round() as i32, c, glow);
                }
            }
        }
        dist += spacing;
    }
    // a faint end mark where it will land
    if locked {
        let (ex, ey) = (b.x + ax * s.lunge_len, b.y + ay * s.lunge_len);
        for (dx, dy) in [(-2, 0), (2, 0), (0, -1), (0, 1)] {
            flash_px(fx, ex.round() as i32 + dx, ey.round() as i32 + dy, DRED, 0.3);
        }
    }
}

pub fn on_hit(b: &mut Body, s: &mut State, hit: &Hit, ctx: &mut EnemyCtx) {
    let (dx, dy) = hit.push_dir(b.x, b.y);
    s.kill_dir = (dx, dy);
    set_phase(s, Phase::Stagger);
    s.ghosts.clear();
    s.heat = 0.45;
    // a yelp hop
    b.vz = if hit.heavy { 2.2 } else { 1.4 };
    b.z = b.z.max(0.2);
    // the cracks bleed embers out of the far side
    let fx = &mut *ctx.fx;
    for _ in 0..if hit.heavy { 12 } else { 7 } {
        let a = dy.atan2(dx) + fx.rng.signed() * 0.7;
        let sp = fx.rng.range(0.8, 2.2);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(fx.rng.range(4.0, 9.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.7, fx.rng.range(0.4, 1.6))
                .color(HOT)
                .fade_to(RED)
                .glow(1.0)
                .grav(0.12)
                .drag(0.92)
                .life(fx.rng.range(14.0, 26.0))
                .streak(true),
        );
    }
    // a few obsidian chips
    for _ in 0..3 {
        let a = dy.atan2(dx) + fx.rng.signed() * 1.0;
        let sp = fx.rng.range(0.6, 1.5);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(fx.rng.range(3.0, 8.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.7, fx.rng.range(0.8, 1.8))
                .color(rgb(0x5b5966))
                .grav(0.16)
                .drag(0.95)
                .bounce(0.3)
                .life(fx.rng.range(80.0, 140.0))
                .settle(true),
        );
    }
}

pub fn on_death(b: &Body, s: &State, ctx: &mut EnemyCtx) {
    let fx = &mut *ctx.fx;
    let (kx, ky) = s.kill_dir;
    fx.shake(0.22);
    fx.sound_vol("proj_fizzle", 0.8);
    // 1. the body bursts into pixels, flung along the killing blow
    let (cell, anchor, flip) = current_cell(b, s);
    let first = fx.particles.list.len();
    fx.particles.burst_cell(&mut fx.rng, cell, anchor, flip, b.x, b.y, 1.4, 330.0);
    for p in &mut fx.particles.list[first..] {
        p.z += b.z;
        let push = fx.rng.range(0.3, 1.4);
        p.vx += kx * push;
        p.vy += ky * push * 0.7;
        if p.glow > 0.0 {
            // The lava seams cool faster than the obsidian rubble.
            p.c1 = RED;
            p.life = fx.rng.range(90.0, 160.0);
            p.max_life = p.life;
        }
    }
    // 2. a heat ring pulsing out along the ground
    for i in 0..20 {
        let a = i as f32 / 20.0 * std::f32::consts::TAU;
        fx.particles
            .add(Particle::new(b.x, b.y).z(0.5).vel(a.cos() * 2.6, a.sin() * 1.7, 0.0).color(HOT).fade_to(DRED).glow(1.0).grav(0.0).drag(0.86).life(16.0));
    }
    // 3. a gout of sparks up and along the blow
    for _ in 0..12 {
        let a = ky.atan2(kx) + fx.rng.signed() * 1.1;
        let sp = fx.rng.range(1.0, 3.2);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(6.0)
                .vel(a.cos() * sp, a.sin() * sp * 0.6, fx.rng.range(0.8, 2.8))
                .color(HOT)
                .fade_to(ORA)
                .glow(1.0)
                .grav(0.1)
                .drag(0.91)
                .life(fx.rng.range(14.0, 28.0))
                .streak(true),
        );
    }
    // 4. slow cinders rising out of the wreck, each a small moving light
    for i in 0..5 {
        fx.particles.add(
            Particle::new(b.x + fx.rng.signed() * 6.0, b.y + fx.rng.signed() * 2.0)
                .z(fx.rng.range(2.0, 8.0))
                .vel(fx.rng.signed() * 0.12, 0.0, fx.rng.range(0.25, 0.5))
                .color(if i % 2 == 0 { HOT } else { YEL })
                .fade_to(DRED)
                .glow(1.0)
                .grav(0.0)
                .drag(0.985)
                .life(fx.rng.range(70.0, 120.0))
                .light(10.0),
        );
    }
    // 5. the flash: one white-hot core with a big short light
    fx.particles.add(Particle::new(b.x, b.y).z(6.0).color(HOT).fade_to(ORA).glow(1.0).grav(0.0).life(12.0).light(36.0));
    // 6. a burn mark: dark scorch with a glowing rim that cools
    scorch(fx, b.x, b.y, 5, 420);
    let (cx, cy) = (b.x.floor() as i32, b.y.floor() as i32);
    for a in 0..28 {
        let ang = a as f32 / 28.0 * std::f32::consts::TAU;
        let r = 6.0 + hash2(a, s.seed as i32, 5) * 2.0;
        let (x, y) = (cx + (ang.cos() * r * 1.2).round() as i32, cy + (ang.sin() * r * 0.7).round() as i32);
        if hash2(x, y, 9) < 0.7 {
            ember_px(fx, x, y, if a % 3 == 0 { ORA } else { RED }, 0.9, 100 + (hash2(x, y, 3) * 80.0) as u32);
        }
    }
}

// ------------------------------------------------------------------ drawing

/// First anim in `names` that the sheet has, resolved for direction.
fn pick<'a>(sh: &'a Sheet, names: &[&str], dir: Dir) -> Option<(&'a Anim, bool)> {
    names.iter().find_map(|n| sh.dir_anim(n, dir))
}

fn has_vertical(sh: &Sheet) -> bool {
    sh.anims.keys().any(|k| k.ends_with("_down"))
}

/// The anim names to try for the current phase, and whether it loops.
fn anim_names(b: &Body, s: &State) -> (&'static [&'static str], bool) {
    match s.phase {
        Phase::Prowl => {
            if b.vx.abs() + b.vy.abs() > 0.3 {
                (&["move", "run", "walk", "idle"], true)
            } else {
                (&["idle"], true)
            }
        }
        Phase::Crouch => (&["windup", "crouch", "telegraph", "idle"], false),
        Phase::Lunge => (&["attack", "lunge", "leap", "move"], false),
        Phase::Skid => (&["land", "skid", "attack", "idle"], false),
        Phase::Recover => (&["recover", "pant", "idle"], true),
        Phase::Stagger => (&["hurt", "idle"], false),
    }
}

/// Ticks into the anim for the current phase. Skid uses the attack anim's tail if it has no
/// own anim (holds the follow-through frame).
fn anim_ticks(s: &State, a: &Anim) -> u32 {
    match s.phase {
        Phase::Skid if a.name.starts_with("attack") => a.total_ticks() - 1,
        Phase::Lunge => {
            // stretch the attack anim over the lunge, leaving the last frame for the skid
            let total = a.total_ticks();
            let pre = total.saturating_sub(*a.durations.last().unwrap_or(&1)).max(1);
            (s.pt * pre / s.lunge_ticks.max(1)).min(pre - 1)
        }
        Phase::Prowl => s.t,
        _ => s.pt,
    }
}

/// The cell to draw now: (cell, anchor, flip).
fn current_cell(b: &Body, s: &State) -> (&'static Cell, (i32, i32), bool) {
    if let Some(sh) = sheet("ember_hound") {
        let dir = if has_vertical(sh) {
            Dir::from_vec(b.face.0, b.face.1)
        } else if s.left {
            Dir::Left
        } else {
            Dir::Right
        };
        let (names, looping) = anim_names(b, s);
        if let Some((a, flip)) = pick(sh, names, dir).or_else(|| pick(sh, &["idle", "move"], dir)) {
            let f = a.frame_at(anim_ticks(s, a), looping);
            return (leg_contour(super::variation::cell("ember_hound", sh.cell(a, f), s.appearance)), sh.anchor_of(a), flip);
        }
    }
    let cells = fallback();
    let i = match s.phase {
        Phase::Prowl => {
            if b.vx.abs() + b.vy.abs() > 0.3 {
                ((s.t / 6) % 2) as usize
            } else {
                0
            }
        }
        Phase::Crouch => 2,
        Phase::Lunge => 3,
        Phase::Skid | Phase::Stagger => 3,
        Phase::Recover => ((s.pt / 14) % 2) as usize,
    };
    let c = &cells[i];
    (leg_contour(c), (20, 20), s.left)
}

/// The base obsidian skin needs a little reflected ash on its lit shin edges.
/// Keep it inside the ink and on the authored light side: no halo, emission or
/// new coverage. Cached albedo retains the pose's original normal map, so lava
/// still determines the actual illumination. The already readable skins pass through.
fn leg_contour(cell: &'static Cell) -> &'static Cell {
    static CELLS: OnceLock<HashMap<usize, Cell>> = OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let sources = sheet("ember_hound").into_iter().flat_map(|sh| sh.cells.iter().flatten())
            .chain(fallback().iter());
        sources.map(|source| {
            let mut material = source.clone();
            let occupied = |y| (0..source.w).any(|x| source.at(x, y).a);
            if let (Some(top), Some(bottom)) = ((0..source.h).find(|&y| occupied(y)), (0..source.h).rfind(|&y| occupied(y))) {
                let shins = top + (bottom - top + 1) * 2 / 3;
                for y in shins..=bottom {
                    for x in 1..source.w {
                        let p = source.at(x, y);
                        let edge = source.at(x - 1, y);
                        if !p.a || p.emit || !edge.a || edge.c != SCORCH { continue; }
                        let c = match p.c.key() {
                            0x3f414f => ASH,
                            0x5b5966 => rgb(0x7c7780),
                            _ => continue,
                        };
                        material.px[(y * source.w + x) as usize].c = c;
                    }
                }
            }
            (source as *const Cell as usize, material)
        }).collect()
    });
    cells.get(&(cell as *const Cell as usize)).unwrap_or(cell)
}

/// Where the eye is, relative to the anchor, for the glint. Uses an `eye` point from the sheet if
/// it has one, else the emissive pixel farthest toward the front-top of the (right-facing) cell:
/// the head, not the seams on the flank or the tail tip.
fn eye_offset(cell: &Cell, anchor: (i32, i32), flip: bool, point: Option<(i32, i32)>) -> (i32, i32) {
    let mut best: Option<(i32, i32)> = point;
    if best.is_none() {
        let mut score = f32::MIN;
        for y in 0..cell.h {
            for x in 0..cell.w {
                let p = cell.at(x, y);
                let sc = x as f32 - y as f32 * 1.2;
                if p.a && p.emit && sc > score {
                    best = Some((x, y));
                    score = sc;
                }
            }
        }
    }
    let (x, y) = best.unwrap_or((cell.w * 3 / 4, cell.h / 3));
    let lx = if flip { cell.w - 1 - x } else { x };
    let ax = if flip { cell.w - 1 - anchor.0 } else { anchor.0 };
    (lx - ax, y - anchor.1)
}

pub fn draw(b: &Body, s: &State, f: &mut Frame) {
    let air = b.z.max(0.0);
    let srx = (7.0 - air * 0.25).max(4.0) as i32;
    f.shadow(f.sx(b.x), f.sy(b.y), srx, 2, if air > 1.0 { 0.32 } else { 0.42 });

    // afterimages of the lunge
    for g in &s.ghosts {
        let o =
            DrawOpts { flip: g.flip, alpha: g.life * 0.6, silhouette: Some(DRED.lerp(RED, g.life).scale(0.55)), silhouette_glow: true, ..Default::default() };
        f.sprite_w(g.cell, g.anchor, g.x, g.y, &o);
    }

    let (cell, anchor, flip) = current_cell(b, s);
    let shiver = if s.phase == Phase::Crouch && s.pt >= LOCK_AT && (s.pt / 2) % 2 == 0 { if s.left { 1.0 } else { -1.0 } } else { 0.0 };
    let (x, y) = (b.x + shiver, b.y - air);
    let gain = (1.0 + s.heat * 0.9).max(0.35);
    let o = DrawOpts { actor: true, foot_depth: 4, foot_half_w: b.r, ground_y: Some(b.y.floor()), elevation: b.z, flip, flash: snappy(b.flash), emit_gain: gain, ..Default::default() };
    f.sprite_w(cell, anchor, x, y, &o);

    // eye glint at aim lock: a four-point star that pops and shrinks
    if s.glint > 0 {
        let point = sheet("ember_hound").and_then(|sh| {
            let dir = if s.left { Dir::Left } else { Dir::Right };
            let (a, _) = pick(sh, anim_names(b, s).0, dir)?;
            a.point("eye", 0)
        });
        let (ex, ey) = eye_offset(cell, anchor, flip, point);
        let (sx, sy) = (f.sx(x) + ex, f.sy(y) + ey);
        let arm = match s.glint {
            9..=10 => 2,
            6..=8 => 4,
            3..=5 => 2,
            _ => 1,
        };
        f.put_glow(sx, sy, HOT);
        for k in 1..=arm {
            let c = if k == arm { YEL } else { HOT };
            f.put_glow(sx + k, sy, c);
            f.put_glow(sx - k, sy, c);
            if k <= arm / 2 + 1 {
                f.put_glow(sx, sy + k, c);
                f.put_glow(sx, sy - k, c);
            }
        }
        f.add_light_3d_w(x + ex as f32, b.y, air - ey as f32, 8.0, YEL, 0.25);
    }

    // Its own glow: the eye and hot seams carry a travelling warm source.
    let h = s.heat.max(-0.6);
    if f.surface_lighting {
        if let Some((lx, ly)) = ember_source(cell, anchor, flip) {
            let r = 34.0 + h.max(0.0) * 14.0;
            let i = (0.48 + h * 0.48).max(0.12);
            f.add_light_3d_w(x + lx, b.y, air - ly, r, ORA, i);
        }
    } else {
        let r = 9.0 + h.max(0.0) * 6.0;
        let i = (0.10 + h * 0.12).max(0.04);
        f.add_light_3d_w(b.x, b.y, air + 6.0, r, ORA, i);
    }
}

/// Put the source inside the hottest visible ember, respecting the current pose
/// and mirror. It rises with the real leap instead of staying behind on the floor.
fn ember_source(cell: &Cell, anchor: (i32, i32), flip: bool) -> Option<(f32, f32)> {
    let mut best = None;
    let mut brightness = -1.0;
    for y in 0..cell.h {
        for x in 0..cell.w {
            let p = cell.at(x, y);
            if p.a && p.emit && p.c.luma() > brightness {
                brightness = p.c.luma();
                // Mirroring the source and anchor cancels the cell-width term.
                let lx = x as f32 - anchor.0 as f32;
                best = Some((if flip { -lx + 0.5 } else { lx + 0.5 }, y as f32 + 0.5 - anchor.1 as f32));
            }
        }
    }
    best
}

// ------------------------------------------------------------------ fallback art

fn fallback() -> &'static [Cell; 4] {
    static CELLS: OnceLock<[Cell; 4]> = OnceLock::new();
    CELLS.get_or_init(|| {
        let pal = [
            ('#', rgb(0x292c3c)),
            ('O', rgb(0xd48670)),
            ('R', rgb(0xa36059)),
            ('W', rgb(0xefb68a)),
            ('Y', rgb(0xefb68a)),
            ('b', rgb(0x393746)),
            ('c', rgb(0x3f414f)),
            ('d', rgb(0x5b5966)),
            ('e', rgb(0x7c7780)),
            ('r', rgb(0x6f454b)),
            ('z', rgb(0x393746)),
        ];
        let em = ['O', 'Y', 'W'];
        let pose_0 = Cell::from_ascii(
            &[
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "....................#...................",
                ".................#.#e#.......#..........",
                "................#e##ed##....#e#.........",
                "..............##ed#edd#e#..#ed#.........",
                "...........###eeddddddded##eddd##.......",
                "..........#eeeddddddcccddeedddWdd#......",
                ".........#edddccccccccdddddddddddd#.....",
                "........#eddcbbbbbbbbcdddddc#crRc#......",
                "........#dddc#######bcddddc#.####.......",
                ".......#c#ddcz......zcRddc#.............",
                ".......#R#dddz......z#cddc#.............",
                "........##ddc#z.....z.#dc#..............",
                "..........#dc#z.....z.#dc#..............",
                ".........#dc#.z.....z.#dc#..............",
                ".........#cc#.zz...zz.#cc#..............",
                "..........##...........##...............",
                "........................................",
                "........................................",
            ],
            &pal,
            &em,
        );
        let pose_1 = Cell::from_ascii(
            &[
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "..................#.....................",
                ".................#e##.....#.............",
                "...............##ed#e##..#e#............",
                "............###eeddeddd##edd###.........",
                "..........##eeeddddddddddeddWdd#........",
                ".........#eedddddcccdddddddddddd#.......",
                "........#edddddcccccccddddcrRc##........",
                "......##ddddcbbbbbbbbcddddc###..........",
                ".....#Rc##dddc#z####bcdcdc#.............",
                "......##..#dc#z.....##zdc#..............",
                ".........##dc#z........#dc#.............",
                "........#ccczz..........#czz............",
                ".........###............#cc#............",
                ".........................##.............",
                "........................................",
                "........................................",
            ],
            &pal,
            &em,
        );
        let pose_2 = Cell::from_ascii(
            &[
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "....................#...................",
                ".................#.#e#..#...............",
                "................#e##ed##e#..............",
                "...............#ed#edd#ed#..............",
                "............###eddeddddedd#....#........",
                "..........##eeedddddddddddd####e#.......",
                ".........#eeddddddcccccddddeeeedd#......",
                "........#eddddccccccccdddddddddWdd#.....",
                "........#dddcbbbbbbbbcdddddddddddd#.....",
                ".......#cddcc#######bcdddddc#cOYc#......",
                ".......#Y#ddcz.....zcYdddc##.####.......",
                "........##ddz.....z.#cddc#..............",
                "........#ddc#z....z.#cdc#...............",
                "........#ccc#zz..zz#ccc#c#..............",
                ".........###........###.#...............",
                "........................................",
                "........................................",
            ],
            &pal,
            &em,
        );
        let pose_3 = Cell::from_ascii(
            &[
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "...................#....................",
                "..................#e##.....#............",
                "................##ed#e##..#e#...........",
                ".............###eeddeddd##edd#####......",
                ".......#...##eeeddddddddddeddWdddd#.....",
                "......#O###eedddddcccdddddddc#####......",
                ".......#deedddddcccccccdddddc#YWWY##....",
                "........#edddcbbbbbbbbcddddc##bOYYOb#...",
                ".........##ddcc#z####bcddddc##bccccb#...",
                "........##dcc#zz.....####zzddcc#####....",
                ".......#ccc##z.............z###ccc#.....",
                "........##zzz...............zzz###......",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
                "........................................",
            ],
            &pal,
            &em,
        );
        [pose_0, pose_1, pose_2, pose_3]
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
    use crate::world::{Zone, ZoneId};

    /// The final crouch locks its lane, and a hit cancels the pending bite.
    #[test]
    fn locked_lunge_can_be_dodged_or_interrupted() {
        let zone = Zone::new(ZoneId::Cinder, 32, 32);
        let mut fx = Fx::new(1);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx { player: (320.0, 256.0), player_alive: true, zone: &zone, fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 0 };
        let (mut b, mut s) = spawn(256.0, 256.0);
        s.noticed = true;
        s.phase = Phase::Crouch;
        s.pt = LOCK_AT - 2;
        update(&mut b, &mut s, &mut ctx);
        update(&mut b, &mut s, &mut ctx);
        let locked = s.aim;
        assert_eq!(s.glint, 10);
        ctx.player = (256.0, 320.0);
        for _ in 0..5 {
            update(&mut b, &mut s, &mut ctx);
            assert_eq!(s.aim, locked);
            assert!(ctx.hits.is_empty());
        }
        let hit = Hit::new(Shape::Circle { x: b.x - 10.0, y: b.y, r: 16.0 }, Team::Player, 1, ctx.fx.hit_id()).knock(1.0);
        assert!(b.take(&hit));
        on_hit(&mut b, &mut s, &hit, &mut ctx);
        for _ in 0..20 {
            update(&mut b, &mut s, &mut ctx);
        }
        assert_eq!(s.phase, Phase::Prowl);
        assert!(ctx.hits.is_empty(), "An interrupted crouch must not bite");
    }
}
