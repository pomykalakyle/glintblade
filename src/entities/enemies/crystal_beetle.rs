//! Crystal beetle (Glasswater Hollow): a squat armored beetle with a crest of pale iris crystals.
//!
//! Loop: it stalks Glint, turning slowly to keep its armored front toward them. When it has a
//! clear run it plants its feet and aims: two marching rails on the floor show exactly the lane
//! it will run down and where it will stop. The rails lock and brighten a beat before it
//! launches, so a sidestep or a dash always gets you out. It charges in a straight line and
//! either skids to a stop or slams into whatever is at the end of the lane, which leaves it
//! dazed on the spot with its crystals dark.
//!
//! Its front shrugs hits off (a crystalline clank, sparks thrown back at you, no damage), and
//! poking it head-on makes it retaliate with a quicker charge. Hit it from the side or behind,
//! or while it's dazed. A charged spin slash to the face cracks its guard and dazes it too.
//! On death it shatters: the body bursts into pixels and the crest into glowing shards that
//! light the floor as they scatter, then settle as fading glints.

use super::EnemyCtx;
use crate::combat::{Hit, Shape, Team};
use crate::entities::Body;
use crate::fx::{Fx, Particle};
use crate::gfx::{Cell, Dir, DrawOpts, Frame, Rgb, rgb, sheet};
use crate::util::{angle_diff, hash2, norm};
use crate::world::Zone;
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

const SHEET: &str = "crystal_beetle";
const HP: i32 = 6;
/// Feet box half-width and hurt radius.
const R: f32 = 6.5;
const WALK: f32 = 0.5;
/// Max turn per tick while stalking (radians). Slow enough that a dash gets you to its flank.
const TURN: f32 = 0.055;
/// It starts a charge when Glint is this close and in sight.
const SIGHT: f32 = 170.0;
/// Plant: ticks spent aiming (tracks Glint), then ticks locked before launch.
const AIM: u32 = 30;
const LOCK: u32 = 18;
const TOP_SPEED: f32 = 4.0;
/// Longest charge lane.
const RANGE: f32 = 230.0;
/// Distance a skid covers from top speed (the lane ends where it actually stops).
const SKID_DIST: f32 = 21.0;
const DAZE: u32 = 120;
const DAZE_GUARD_BREAK: u32 = 80;
const RECOVER: u32 = 34;
/// Half-width of the drawn lane. The real hit reaches a little less far, so standing just
/// outside the rails is always safe.
const LANE: f32 = 12.0;
const HIT_R: f32 = 5.0;
/// Hits from within this cone in front of it (cosine of the half-angle) glance off.
const FRONT_COS: f32 = 0.34;

// Glasswater palette (assets/concept/palettes_v2.json: crystal_beetle + rock).
const CRYSTAL: [Rgb; 4] = [rgb(0x4d4866), rgb(0x796c91), rgb(0xa195b8), rgb(0xcdc3db)];
const CRYSTAL_LIGHT: Rgb = rgb(0xcdc3db);
const EYE: Rgb = rgb(0xa195b8);
const EYE_HOT: Rgb = rgb(0xe2d8e9);
const SHELL: [Rgb; 4] = [rgb(0x29313f), rgb(0x364657), rgb(0x4c6070), rgb(0x73858b)];
const ROCK: [Rgb; 4] = [rgb(0x29313f), rgb(0x364657), rgb(0x4c6070), rgb(0x73858b)];
const DUST: Rgb = rgb(0x73858b);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Stalk,
    Plant,
    Charge,
    Skid,
    Dazed,
    Recover,
}

#[derive(Clone, Debug)]
pub struct State {
    pub mode: Mode,
    pub mode_t: u32,
    pub t: u32,
    /// Facing angle (radians, +y down). `Body::face` mirrors it.
    pub ang: f32,
    /// Charge direction (locked at the end of the aim).
    pub dir: (f32, f32),
    /// Length of the charge lane and whether a wall ends it (slam) or it just runs out (skid).
    pub lane: f32,
    pub lane_wall: bool,
    /// Distance covered in the current charge.
    pub run: f32,
    pub speed: f32,
    pub hit_id: u32,
    /// Ticks before it may plant again.
    pub cooldown: u32,
    /// Crystals flare after a glancing hit or on waking (0..1).
    pub flare: f32,
    pub daze_len: u32,
    pub moving: bool,
    pub walk_t: u32,
    /// Visual sector only; armor, turning and charge keep their continuous heading.
    visual_dir: Dir,
    seed: u32,
    appearance: u8,
}

impl State {
    /// Enter a behavior and restart its pose clock.
    fn set(&mut self, m: Mode) {
        self.mode = m;
        self.mode_t = 0;
    }
    /// Return the unit heading used by both the armor and charge.
    fn facing(&self) -> (f32, f32) {
        (self.ang.cos(), self.ang.sin())
    }
    /// Turn toward a heading at a limited angular speed.
    fn turn_to(&mut self, want: f32, rate: f32) {
        self.ang += angle_diff(self.ang, want).clamp(-rate, rate);
    }
    /// 0..1, a different value per beetle (from its spawn spot), for variety.
    fn vary(&self, k: u32) -> f32 {
        hash2(self.seed as i32, k as i32, 77)
    }
}

/// Spawn a beetle with staggered attack timing.
pub fn spawn(x: f32, y: f32) -> (Body, State) {
    let seed = (x as i32 as u32).wrapping_mul(31).wrapping_add(y as i32 as u32);
    let mut s = State {
        mode: Mode::Stalk,
        mode_t: 0,
        t: 0,
        ang: PI,
        dir: (-1.0, 0.0),
        lane: 0.0,
        lane_wall: false,
        run: 0.0,
        speed: 0.0,
        hit_id: 0,
        cooldown: 0,
        flare: 0.0,
        daze_len: DAZE,
        moving: false,
        walk_t: 0,
        visual_dir: Dir::Left,
        seed,
        appearance: super::variation::appearance(x, y),
    };
    s.cooldown = 30 + (s.vary(1) * 50.0) as u32;
    let mut b = Body::new(x, y, R, HP);
    b.face = s.facing();
    s.visual_dir = s.visual_dir.stable_from_vec(b.face.0, b.face.1);
    (b, s)
}

/// Advance the beetle by one tick.
pub fn update(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    s.t += 1;
    s.mode_t += 1;
    s.flare = (s.flare - 0.06).max(0.0);
    s.cooldown = s.cooldown.saturating_sub(1);
    s.moving = false;
    let (px, py) = ctx.player;
    let d = b.dist_to(px, py);
    let want = (py - b.y).atan2(px - b.x);

    // Flinch from a hit that landed on its flank: a hit mid-aim cancels the charge.
    if b.stun > 0 && matches!(s.mode, Mode::Stalk | Mode::Plant | Mode::Recover) {
        if s.mode == Mode::Plant {
            s.set(Mode::Recover);
            s.cooldown = s.cooldown.max(60);
        }
        b.vx = 0.0;
        b.vy = 0.0;
        b.integrate(ctx.zone);
        return;
    }

    match s.mode {
        Mode::Stalk => stalk(b, s, ctx, d, want),
        Mode::Plant => plant(b, s, ctx, want),
        Mode::Charge => charge(b, s, ctx),
        Mode::Skid => skid(b, s, ctx),
        Mode::Dazed => dazed(b, s, ctx),
        Mode::Recover => {
            s.turn_to(want, TURN * 0.6);
            b.vx = 0.0;
            b.vy = 0.0;
            b.integrate(ctx.zone);
            if s.mode_t >= RECOVER {
                s.set(Mode::Stalk);
                s.cooldown = s.cooldown.max(45 + (s.vary(s.t) * 45.0) as u32);
            }
        }
    }
    b.face = s.facing();
    s.visual_dir = s.visual_dir.stable_from_vec(b.face.0, b.face.1);
}

/// Approach slowly and choose a clear charge lane.
fn stalk(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx, d: f32, want: f32) {
    let (px, py) = ctx.player;
    s.turn_to(want, TURN);
    let err = angle_diff(s.ang, want).abs();
    let seen = ctx.player_alive && d < SIGHT && ctx.zone.line_clear(b.x, b.y - 4.0, px, py - 4.0);
    if seen && s.cooldown == 0 && err < 0.3 {
        let (len, wall) = cast(ctx.zone, b.x, b.y, s.facing());
        // Only charge if the lane reaches Glint (no prop in the way) and is worth running.
        if len >= (d - 8.0).max(34.0) {
            begin_plant(s, len, wall, 0);
            b.vx = 0.0;
            b.vy = 0.0;
            b.integrate(ctx.zone);
            return;
        }
    }
    // Lumber closer, front first; back off a step when crowded.
    let speed = if !ctx.player_alive {
        0.0
    } else if d > 64.0 {
        WALK
    } else if d < 30.0 {
        -WALK * 0.6
    } else {
        0.0
    };
    let speed = speed * (1.0 - err / PI).max(0.0);
    let (fx_, fy_) = s.facing();
    b.vx = fx_ * speed;
    b.vy = fy_ * speed;
    if speed.abs() > 0.05 {
        s.moving = true;
        s.walk_t += 1;
        if s.walk_t % 14 == 0 {
            let fx = &mut *ctx.fx;
            fx.particles.dust(&mut fx.rng, b.x - fx_ * 5.0, b.y, 1, DUST);
        }
    }
    b.integrate(ctx.zone);
}

/// Plant the feet and remember the projected charge lane.
fn begin_plant(s: &mut State, len: f32, wall: bool, skip: u32) {
    s.set(Mode::Plant);
    s.mode_t = skip;
    s.lane = len;
    s.lane_wall = wall;
    s.dir = s.facing();
}

/// Track briefly, then hold a locked direction before launching.
fn plant(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx, want: f32) {
    let t = s.mode_t;
    if !ctx.player_alive {
        s.set(Mode::Recover);
        b.vx = 0.0;
        b.vy = 0.0;
        b.integrate(ctx.zone);
        return;
    }
    b.vx = 0.0;
    b.vy = 0.0;
    if t <= AIM {
        // Aiming: track Glint, the lane follows.
        s.turn_to(want, 0.07);
        s.dir = s.facing();
        let (len, wall) = cast(ctx.zone, b.x, b.y, s.dir);
        s.lane = len;
        s.lane_wall = wall;
    }
    let fx = &mut *ctx.fx;
    if t == AIM + 1 {
        // The lock: crystals flash once so the moment reads even without looking at the floor.
        s.flare = s.flare.max(0.7);
    }
    // The rumble builds for ~0.5 s and peaks at launch.
    if t == AIM + LOCK - 30 {
        fx.sound("beetle_charge");
    }
    // Pawing: kick grit back from the rear feet.
    if t % 8 == 3 {
        let (dx, dy) = s.dir;
        let (bx, by) = (b.x - dx * 7.0, b.y - dy * 3.0);
        for _ in 0..3 {
            fx.particles.add(
                Particle::new(bx + fx.rng.signed() * 2.0, by)
                    .z(1.0)
                    .vel(
                        -dx * fx.rng.range(0.6, 1.4) + fx.rng.signed() * 0.3,
                        -dy * fx.rng.range(0.4, 1.0) + fx.rng.signed() * 0.2,
                        fx.rng.range(0.8, 1.6),
                    )
                    .color(*fx.rng.pick(&ROCK[1..]))
                    .life(fx.rng.range(30.0, 50.0))
                    .grav(0.14)
                    .settle(true),
            );
        }
    }
    b.integrate(ctx.zone);
    if t >= AIM + LOCK {
        s.set(Mode::Charge);
        s.hit_id = fx.hit_id();
        s.speed = 1.6;
        s.run = 0.0;
        fx.shake(0.12);
        let (dx, dy) = s.dir;
        fx.particles.dust(&mut fx.rng, b.x - dx * 7.0, b.y - dy * 3.0, 8, DUST);
    }
}

/// How far can the beetle run from (x, y) along `d` before its feet box hits something?
fn cast(zone: &Zone, x: f32, y: f32, d: (f32, f32)) -> (f32, bool) {
    let mut l = 0.0;
    while l < RANGE {
        let nl = l + 2.0;
        if zone.blocked(x + d.0 * nl, y + d.1 * nl, R) {
            return (l, true);
        }
        l = nl;
    }
    (RANGE, false)
}

/// The charge hit: the beetle's front half. Knocks Glint diagonally out of the lane.
fn push_hit(b: &Body, s: &State, ctx: &mut EnemyCtx) {
    let (dx, dy) = s.dir;
    let (px, py) = ctx.player;
    let side = if dx * (py - b.y) - dy * (px - b.x) >= 0.0 { 1.0 } else { -1.0 };
    let (kx, ky) = norm(dx - dy * side * 0.9, dy + dx * side * 0.9);
    let y = b.y - 2.0;
    ctx.hits.push(
        Hit::new(
            Shape::Capsule {
                x0: b.x - dx * 3.0,
                y0: y - dy * 3.0,
                x1: b.x + dx * 6.0,
                y1: y + dy * 6.0,
                r: HIT_R,
            },
            Team::Enemy,
            1,
            s.hit_id,
        )
        .knock(4.2)
        .dir(kx, ky)
        .heavy(true),
    );
}

/// Rush along the locked lane until a wall or the braking distance.
fn charge(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    s.speed = (s.speed + 0.45).min(TOP_SPEED);
    let (dx, dy) = s.dir;
    b.vx = dx * s.speed;
    b.vy = dy * s.speed;
    b.kx *= 0.5;
    b.ky *= 0.5;
    let (x0, y0) = (b.x, b.y);
    let blocked = b.integrate(ctx.zone);
    let prog = (b.x - x0) * dx + (b.y - y0) * dy;
    s.run += prog.max(0.0);
    push_hit(b, s, ctx);
    let fx = &mut *ctx.fx;
    if fx.rng.chance(0.6) {
        let back = fx.rng.range(4.0, 8.0);
        fx.particles.dust(&mut fx.rng, b.x - dx * back, b.y - dy * back * 0.5, 1, DUST);
    }
    // A few glints shed from the crest: the speed reads, the crystals stay the focus.
    if s.mode_t % 7 == 3 {
        fx.particles.add(
            Particle::new(b.x + fx.rng.signed() * 4.0, b.y)
                .z(fx.rng.range(10.0, 15.0))
                .vel(-dx * 0.6, -dy * 0.6, 0.15)
                .color(CRYSTAL[3])
                .fade_to(CRYSTAL[0])
                .glow(1.0)
                .grav(0.0)
                .drag(0.9)
                .life(fx.rng.range(8.0, 14.0)),
        );
    }
    if blocked || (s.lane_wall && s.run >= s.lane) {
        // Ordinary movement slides along walls; a charge ends at its first obstruction.
        b.x = x0 + dx * prog.max(0.0);
        b.y = y0 + dy * prog.max(0.0);
        if ctx.zone.blocked(b.x, b.y, R) {
            b.x = x0;
            b.y = y0;
        }
        slam(b, s, ctx);
    } else if (!s.lane_wall && s.run >= s.lane - SKID_DIST) || s.run > RANGE + 30.0 {
        s.set(Mode::Skid);
    }
}

/// Brake on open ground and leave short claw scrapes.
fn skid(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    s.speed *= 0.82;
    let (dx, dy) = s.dir;
    b.vx = dx * s.speed;
    b.vy = dy * s.speed;
    b.integrate(ctx.zone);
    if s.speed > 1.6 {
        push_hit(b, s, ctx);
    }
    let fx = &mut *ctx.fx;
    // Grit sprays sideways off the dug-in feet, and the claws leave two fading scrape lines.
    let (nx, ny) = (-dy, dx);
    for side in [-1.0f32, 1.0] {
        let (fx0, fy0) = (b.x + dx * 4.0 + nx * side * 4.0, b.y + dy * 2.0 + ny * side * 2.0);
        if s.speed > 0.8 {
            fx.particles.add(
                Particle::new(fx0, fy0)
                    .z(0.5)
                    .vel(
                        nx * side * fx.rng.range(0.4, 1.2) + dx * s.speed * 0.3,
                        ny * side * fx.rng.range(0.3, 0.8) + dy * s.speed * 0.3,
                        fx.rng.range(0.5, 1.3),
                    )
                    .color(*fx.rng.pick(&ROCK[1..]))
                    .life(fx.rng.range(30.0, 50.0))
                    .grav(0.14)
                    .settle(true),
            );
        }
        fx.particles.add(Particle::new(fx0, fy0).color(ROCK[0]).life(160.0).settle(true).fade(false));
    }
    if s.mode_t % 3 == 0 {
        fx.particles.dust(&mut fx.rng, b.x + dx * 6.0, b.y, 1, DUST);
    }
    if s.speed < 0.25 {
        s.set(Mode::Recover);
    }
}

/// Rebound from a wall and expose the unguarded shell.
fn slam(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    s.set(Mode::Dazed);
    s.daze_len = DAZE;
    let (dx, dy) = s.dir;
    b.kx = -dx * 2.4;
    b.ky = -dy * 2.4;
    b.vz = 2.0;
    let fx = &mut *ctx.fx;
    fx.shake(0.42);
    fx.freeze(5);
    fx.sound_vol("crystal_shatter_2", 0.75);
    fx.sound_var("hit", 3);
    let (ix, iy) = (b.x + dx * (R + 2.0), b.y + dy * (R + 2.0));
    // Rock chips burst back out of the wall.
    for _ in 0..16 {
        let a = (-dy).atan2(-dx) + fx.rng.signed() * 1.1;
        let sp = fx.rng.range(0.6, 2.2);
        fx.particles.add(
            Particle::new(ix, iy)
                .z(fx.rng.range(2.0, 10.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.8, fx.rng.range(0.6, 2.4))
                .color(*fx.rng.pick(&ROCK[1..]))
                .life(fx.rng.range(160.0, 260.0))
                .grav(0.16)
                .bounce(0.35)
                .settle(true),
        );
    }
    fx.particles.sparks(&mut fx.rng, ix, iy, 6.0, -dx, -dy, 12, CRYSTAL[3], 2.6);
    fx.particles.dust(&mut fx.rng, ix, iy, 10, DUST);
    chips(fx, b.x, b.y, (-dx, -dy), 7);
}

/// Hold a generous punish window before relighting the crest.
fn dazed(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    b.vx = 0.0;
    b.vy = 0.0;
    b.integrate(ctx.zone);
    let fx = &mut *ctx.fx;
    if s.mode_t == 8 {
        // landing thump after the bounce
        fx.particles.dust(&mut fx.rng, b.x, b.y, 6, DUST);
    }
    if s.mode_t >= s.daze_len {
        // Shake it off: the crystals relight with a pulse.
        s.set(Mode::Recover);
        s.flare = 1.0;
        fx.particles.sparks(&mut fx.rng, b.x, b.y, 12.0, 0.0, -1.0, 6, CRYSTAL[2], 1.4);
        fx.particles.dust(&mut fx.rng, b.x, b.y, 4, DUST);
    }
}

/// Glowing crystal chips knocked off the crest; they settle as glints on the floor.
fn chips(fx: &mut Fx, x: f32, y: f32, dir: (f32, f32), n: usize) {
    for _ in 0..n {
        let a = dir.1.atan2(dir.0) + fx.rng.signed() * 1.0;
        let sp = fx.rng.range(0.5, 1.7);
        fx.particles.add(
            Particle::new(x + fx.rng.signed() * 3.0, y)
                .z(fx.rng.range(9.0, 15.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.8, fx.rng.range(1.0, 2.4))
                .color(CRYSTAL[3])
                .fade_to(CRYSTAL[1])
                .glow(1.0)
                .life(fx.rng.range(200.0, 320.0))
                .grav(0.16)
                .bounce(0.4)
                .settle(true),
        );
    }
}

/// Deflect frontal strikes or react to an exposed flank hit.
pub fn on_hit(b: &mut Body, s: &mut State, hit: &Hit, ctx: &mut EnemyCtx) {
    let (cx, cy) = hit.shape.center();
    let to_att = norm(cx - b.x, cy - b.y);
    let (fx_, fy_) = s.facing();
    let from_front = to_att.0 * fx_ + to_att.1 * fy_ > FRONT_COS;
    let fx = &mut *ctx.fx;
    if from_front && s.mode != Mode::Dazed {
        // Glance off the armored front: undo the damage and the stagger.
        b.hp += hit.dmg;
        b.flash = 0.0;
        b.stun = 0;
        b.kx *= 0.25;
        b.ky *= 0.25;
        s.flare = 1.0;
        let (ix, iy) = (b.x + fx_ * (R + 1.0), b.y + fy_ * (R + 1.0));
        fx.particles.sparks(&mut fx.rng, ix, iy, 7.0, to_att.0, to_att.1, 10, CRYSTAL[3], 3.0);
        fx.sound("deflect");
        fx.freeze(4);
        let guard_break = hit.heavy && matches!(hit.shape, Shape::Circle { .. });
        if guard_break {
            // A charged spin to the face cracks the guard: knocked onto its back.
            s.set(Mode::Dazed);
            s.daze_len = DAZE_GUARD_BREAK;
            b.vz = 1.8;
            b.kx = -fx_ * 2.0;
            b.ky = -fy_ * 2.0;
            fx.shake(0.3);
            fx.sound_vol("crystal_shatter_2", 0.6);
            chips(fx, b.x, b.y, to_att, 5);
        } else if s.mode == Mode::Stalk && ctx.player_alive {
            // Poking it head-on makes it angry: a quicker charge, straight back at you.
            let (len, wall) = cast(ctx.zone, b.x, b.y, s.facing());
            if len > 24.0 {
                begin_plant(s, len, wall, AIM / 2);
            }
        }
        return;
    }
    match s.mode {
        // Momentum: a flank hit mid-charge lands but doesn't stop it.
        Mode::Charge | Mode::Skid => {
            b.stun = 0;
            b.kx *= 0.3;
            b.ky *= 0.3;
        }
        // Lying dazed: it takes the hits where it lies, so a full combo connects.
        Mode::Dazed => {
            b.stun = 0;
            b.kx *= 0.35;
            b.ky *= 0.35;
        }
        // Heavy: a flinch, not a tumble, so the rest of the combo still reaches it.
        _ => {
            b.kx *= 0.5;
            b.ky *= 0.5;
        }
    }
    let (dx, dy) = hit.push_dir(b.x, b.y);
    chips(fx, b.x, b.y, (dx, dy), if hit.heavy { 5 } else { 3 });
}

/// Shatter the current sprite into bouncing crystal and shell pixels.
pub fn on_death(b: &Body, s: &State, ctx: &mut EnemyCtx) {
    let fx = &mut *ctx.fx;
    let (cell, anchor, flip) = current_cell(b, s, ctx.t as u32);
    fx.particles.burst_cell(&mut fx.rng, cell, anchor, flip, b.x, b.y, 1.4, 420.0);
    // The crest breaks into big shards that light the floor while they fly.
    let n = 6;
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU + fx.rng.signed() * 0.4;
        let sp = fx.rng.range(0.7, 1.8);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(12.0)
                .vel(a.cos() * sp, a.sin() * sp * 0.75, fx.rng.range(1.6, 3.0))
                .color(CRYSTAL[3])
                .fade_to(CRYSTAL[2])
                .glow(1.0)
                .size(2)
                .life(fx.rng.range(260.0, 380.0))
                .grav(0.16)
                .bounce(0.35)
                .drag(0.96)
                .settle(true)
                .shadow(true)
                .light(13.0),
        );
    }
    // A ring of sparks and one bright instant of light.
    for i in 0..12 {
        let a = i as f32 / 12.0 * TAU;
        let sp = fx.rng.range(2.0, 3.2);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(9.0)
                .vel(a.cos() * sp, a.sin() * sp * 0.7, fx.rng.range(0.0, 0.8))
                .color(EYE_HOT)
                .fade_to(CRYSTAL[2])
                .glow(1.0)
                .life(fx.rng.range(10.0, 18.0))
                .grav(0.05)
                .drag(0.86)
                .streak(true),
        );
    }
    fx.particles
        .add(Particle::new(b.x, b.y).z(9.0).color(CRYSTAL[3]).glow(1.0).grav(0.0).life(9.0).fade(false).light(80.0));
    fx.shake(0.32);
    fx.sound("crystal_shatter");
}

// ------------------------------------------------------------------------------------ drawing

/// Which anim names to try for the current state (first one the sheet has wins), and its clock.
/// `t` is the scene clock (keeps idle breathing while the beetle hasn't noticed Glint yet).
fn anim_for(b: &Body, s: &State, t: u32) -> (&'static [&'static str], u32, bool) {
    if b.stun > 0 && matches!(s.mode, Mode::Stalk | Mode::Recover) {
        return (&["hurt", "idle"], 12u32.saturating_sub(b.stun), false);
    }
    match s.mode {
        Mode::Stalk if s.moving => (&["walk", "move", "run", "idle"], s.walk_t, true),
        Mode::Stalk | Mode::Recover => (&["idle", "walk"], t, true),
        Mode::Plant => (&["windup", "plant", "telegraph", "aim", "idle"], s.mode_t, false),
        Mode::Charge | Mode::Skid => (&["charge", "run", "dash", "attack", "walk"], s.t, true),
        Mode::Dazed => (&["dazed", "stunned", "stun", "flipped", "hurt", "idle"], s.mode_t, true),
    }
}

/// The cell showing right now (sheet if it exists, else the built-in fallback).
fn current_cell(b: &Body, s: &State, t: u32) -> (&'static Cell, (i32, i32), bool) {
    let dir = if super::variation::reference_pose() {
        super::variation::legacy_dir(b.face.0, b.face.1)
    } else {
        s.visual_dir.stable_from_vec(b.face.0, b.face.1)
    };
    if let Some(sh) = sheet(SHEET) {
        let (names, ticks, looping) = anim_for(b, s, t);
        if let Some((a, flip)) = names.iter().find_map(|n| sh.dir_anim(n, dir)) {
            let looping = a.json.get("loop").and_then(|v| v.as_bool()).unwrap_or(looping);
            // The windup is stretched over the whole plant so its last (crouched) frame lands
            // on the launch, whatever its authored length.
            let ticks = if s.mode == Mode::Plant { ticks * a.total_ticks() / (AIM + LOCK) } else { ticks };
            return (super::variation::cell(SHEET, sh.cell(a, a.frame_at(ticks, looping)), s.appearance), sh.anchor_of(a), flip);
        }
    }
    let (cell, flip) = fallback(dir);
    (cell, (cell.w / 2, cell.h - 1), flip)
}

/// Audit the actual selected geometry without duplicating animation selection.
pub(crate) fn audit_pose(b: &Body, s: &State, t: u64) -> (&'static Cell, (i32, i32), bool) {
    current_cell(b, s, t as u32)
}

/// Render the sprite, its crystal light, and its current telegraph.
pub fn draw(b: &Body, s: &State, f: &mut Frame) {
    if s.mode == Mode::Plant {
        draw_lane(b, s, f);
    }
    f.shadow(f.sx(b.x), f.sy(b.y), 9, 2, 0.45);

    // Walk rows already bob the shell above planted native feet. Additional
    // whole-cell walk bob would lift those feet away from the contact shadow.
    // Aim tremble/pull-back and dazed wobble remain intentional whole-body motion.
    let (mut ox, mut oy) = (0.0f32, 0.0f32);
    let mut glow = 1.0;
    let mut light = 0.3;
    match s.mode {
        Mode::Plant => {
            let k = (s.mode_t as f32 / (AIM + LOCK) as f32).min(1.0);
            glow = 1.0 + 0.35 * k;
            light = 0.3 + 0.45 * k;
            if s.mode_t > AIM {
                ox += if (s.mode_t / 2) % 2 == 0 { 1.0 } else { 0.0 };
                if s.mode_t + 6 >= AIM + LOCK {
                    ox -= s.dir.0 * 2.0;
                    oy -= s.dir.1 * 1.0;
                }
                glow += if (s.mode_t / 2) % 2 == 0 { 0.2 } else { 0.0 };
            }
        }
        Mode::Charge => {
            glow = 1.3;
            light = 0.7;
        }
        Mode::Skid => {
            glow = 1.15;
            light = 0.5;
        }
        Mode::Dazed => {
            // Guard down: the crystals sputter at half strength, catching now and then like a
            // loose bulb, and come back steadily as it gathers itself.
            let k = s.mode_t as f32 / s.daze_len as f32;
            let catch = hash2(s.mode_t as i32 / 3, s.seed as i32, 5) < 0.18 + 0.3 * k;
            glow = if catch { 0.85 } else { 0.45 };
            light = if catch { 0.24 } else { 0.14 };
            let wob = (s.mode_t as f32 * 0.45).sin() * (1.0 - s.mode_t as f32 / s.daze_len as f32);
            ox += (wob * 1.6).round();
        }
        _ => {}
    }
    glow += s.flare * 0.8;
    light += s.flare * 0.5;

    let (cell, anchor, flip) = current_cell(b, s, f.t as u32);
    f.sprite_w(
        cell,
        anchor,
        b.x + ox,
        b.y - b.z + oy,
        &DrawOpts {
            actor: true,
            foot_depth: 8,
            foot_full_width: true,
            foot_half_w: b.r,
            ground_y: Some(b.y.floor()),
            elevation: b.z - oy,
            flip,
            flash: b.flash,
            emit_gain: glow,
            ..Default::default()
        },
    );
    if f.surface_lighting {
        if let Some((lx, ly)) = crest_source(cell, anchor, flip) {
            // The actual crystal cap now lights adjacent masonry as it stalks or
            // charges. Dazed crystals still visibly gutter through `light`.
            let radius = 40.0 + 10.0 * s.flare;
            f.add_light_3d_w(b.x + ox + lx, b.y, b.z - oy - ly, radius, CRYSTAL_LIGHT, light * 1.8);
        }
    } else {
        f.add_light_3d_w(b.x, b.y, 10.0, 14.0 + 6.0 * s.flare, CRYSTAL_LIGHT, light * 0.35);
    }

    if s.mode == Mode::Dazed {
        draw_dizzy(b, s, f);
    }
}

/// The crest's highest emissive pixel belongs to the drawn crystal geometry.
/// Choosing actual ink avoids hiding the source inside the armored shell.
fn crest_source(cell: &Cell, anchor: (i32, i32), flip: bool) -> Option<(f32, f32)> {
    for y in 0..cell.h {
        let mut best = None;
        let mut brightness = -1.0;
        for x in 0..cell.w {
            let p = cell.at(x, y);
            if p.a && p.emit && p.c.luma() > brightness {
                brightness = p.c.luma();
                let lx = x as f32 - anchor.0 as f32;
                best = Some((if flip { -lx + 0.5 } else { lx + 0.5 }, y as f32 + 0.5 - anchor.1 as f32));
            }
        }
        if best.is_some() { return best; }
    }
    None
}

/// The telegraph: two rails on the floor along the exact lane, dashes marching outward while
/// it aims; once the direction locks they brighten, flicker and march fast. They fade with
/// distance so the near end (where the danger starts) reads first. If a wall ends the lane, a
/// bar marks where it will slam.
fn draw_lane(b: &Body, s: &State, f: &mut Frame) {
    let t = s.mode_t;
    let locked = t > AIM;
    let (dx, dy) = s.dir;
    let (nx, ny) = (-dy, dx);
    let start = R + 6.0;
    let full = (s.lane + R).max(start);
    let grow = (t as f32 / 10.0).min(1.0);
    let end = start + (full - start) * grow;
    let (march, on, period) = if locked { (s.t as i32, 4, 6) } else { ((s.t / 3) as i32, 3, 6) };
    let (c, k) = if locked {
        (if (t / 2) % 2 == 0 { EYE_HOT } else { EYE }, 0.68)
    } else {
        (EYE, 0.32 + 0.20 * t as f32 / AIM as f32)
    };
    let len_px = (end - start).max(1.0);
    let mut pts: Vec<(i32, i32, f32)> = Vec::with_capacity(512);
    for side in [-1.0f32, 1.0] {
        let (x0, y0) = (b.x + dx * start + nx * LANE * side, b.y + dy * start + ny * LANE * side);
        let (x1, y1) = (b.x + dx * end + nx * LANE * side, b.y + dy * end + ny * LANE * side);
        let (sx0, sy0, sx1, sy1) = (f.sx(x0), f.sy(y0), f.sx(x1), f.sy(y1));
        let steps = (sx1 - sx0).abs().max((sy1 - sy0).abs()).max(1) as f32;
        let mut i = 0i32;
        Frame::line_with(sx0, sy0, sx1, sy1, |x, y| {
            if (i - march).rem_euclid(period) < on {
                let u = i as f32 / steps;
                pts.push((x, y, 1.0 - 0.6 * u));
            }
            i += 1;
        });
    }
    for (x, y, fade) in pts.drain(..) {
        f.add_emit(x, y, c.scale(k * fade));
    }
    // The slam bar at a wall (only once the rails reach it).
    if grow >= 1.0 && s.lane_wall {
        let (ex, ey) = (b.x + dx * end, b.y + dy * end);
        // a touch brighter than the rails' faded far end, so the stop point reads
        let bar = c.scale(k * if len_px < 60.0 { 0.9 } else { 0.6 });
        let mut bpts = Vec::with_capacity(24);
        Frame::line_with(f.sx(ex - nx * LANE), f.sy(ey - ny * LANE), f.sx(ex + nx * LANE), f.sy(ey + ny * LANE), |x, y| {
            bpts.push((x, y))
        });
        for (x, y) in bpts {
            f.add_emit(x, y, bar);
        }
    }
}

/// Dazed: three crystal motes circle above its head, each with a one-pixel tail.
fn draw_dizzy(b: &Body, s: &State, f: &mut Frame) {
    let (cx, cy) = (b.x, b.y - 22.0);
    for i in 0..3 {
        let a = s.mode_t as f32 * 0.15 + i as f32 * TAU / 3.0;
        let front = a.sin() > 0.0;
        let tail = a - 0.35;
        let (tx, ty) = (f.sx(cx + tail.cos() * 8.0), f.sy(cy + tail.sin() * 2.5));
        let (x, y) = (f.sx(cx + a.cos() * 8.0), f.sy(cy + a.sin() * 2.5));
        if (tx, ty) != (x, y) {
            f.put_glow(tx, ty, CRYSTAL[0]);
        }
        f.put_glow(x, y, if front { CRYSTAL[3] } else { CRYSTAL[2] });
    }
}

// ----------------------------------------------------------------------------------- fallback

/// Built-in frames used until `crystal_beetle.png` exists (the Glasswater artist's WIP design).
fn fallback(dir: Dir) -> (&'static Cell, bool) {
    static CELLS: OnceLock<[Cell; 5]> = OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let pal = [
            ('O', rgb(0x222d3d)),
            ('k', rgb(0x29313f)),
            ('1', SHELL[0]),
            ('2', SHELL[1]),
            ('3', SHELL[2]),
            ('4', SHELL[3]),
            ('5', rgb(0xa6b4ad)),
            ('a', rgb(0x4d4866)),
            ('b', CRYSTAL[0]),
            ('c', CRYSTAL[1]),
            ('d', CRYSTAL[2]),
            ('e', CRYSTAL[3]),
            ('m', EYE),
        ];
        let em = ['c', 'd', 'e', 'm'];
        let right = Cell::from_ascii(
            &[
                "......O...............",
                ".....OeO..............",
                ".....OedO.............",
                "...O.OedcO............",
                "..OeOOedcO.O..........",
                "..OedOedcOOeO.........",
                ".OOdcOedcOedO.........",
                ".O4OdObdcOdcO......OO.",
                "O345ObbdbOdO4O....OeO.",
                "O3455544bbb433O..OedO.",
                "O34555444443332OOdcO..",
                "O34454444433322O4dcO..",
                "O33444444333222O433O..",
                "O23333333332221O4322O.",
                "O22222222222111O222mO.",
                ".O1111111111kkO.O111O.",
                "..OOOOOOOOOOOOO.OOOO..",
                "..O....O...O...O...O..",
                ".O.....O....O...O...O.",
            ],
            &pal,
            &em,
        );
        let down = Cell::from_ascii(
            &[
                "........O..........",
                ".......OeO.........",
                ".......OedO........",
                "....O..OedcO.......",
                "...OeO.OedcO.O.....",
                "...OedOOedcOOeO....",
                "....OedOedcOedO....",
                "...OOOdObdcOdcOO...",
                "..O44OdObdbOdO33O..",
                ".O455OObbbbbOO332O.",
                ".O4555444244433322O",
                "O34554444244433322O",
                "O34444444244333222O",
                "O33444OOOOOOO32221O",
                "O2333O4dO44dO22211O",
                "O2222O3eO33eO21111O",
                ".O111Om33c3mO1111O.",
                "..OOO.O2222O.OOOO..",
                "...O..OO11OO..O....",
                "..O....OOOO....O...",
            ],
            &pal,
            &em,
        );
        let up = Cell::from_ascii(
            &[
                "........O..........",
                ".......OeO.........",
                ".......OedO........",
                "....O..OedcO.......",
                "...OeO.OedcO.O.....",
                "...OedOOedcOOeO....",
                "....OedOedcOedO....",
                "...OOOdObdcOdcOO...",
                "..O44OdObdbOdO33O..",
                ".O455OObbbbbOO332O.",
                ".O4555444244433322O",
                "O34554444244433322O",
                "O34444444244433322O",
                "O33444444244333222O",
                "O23333333233332221O",
                "O22222222222222111O",
                "O22222222222221111O",
                ".O111111111111111O.",
                "..OOOOOOOOOOOOOOO..",
                "..O..O.......O..O..",
                ".O..O.........O..O.",
            ],
            &pal,
            &em,
        );
        // Native diagonal silhouettes have a pitched head and distinct near/far
        // legs. These only serve builds that intentionally omit artist sheets.
        let down_right = Cell::from_ascii(&[
            "........O................",
            ".......OeO...............",
            "....O..OedO..............",
            "...OeO.OedcO.............",
            "...OedOOedcO.............",
            "....OedObdcO...O.........",
            "...OO4bObbbOO.OeO........",
            "..O4555544443OOedO.......",
            ".O455554444333ObdcO......",
            "O45554444433322O4dcO.....",
            "O344444443332211O33mO....",
            ".O33344333322211O322O....",
            "..O22222221111OOO11OO....",
            "..OO11111111OO.O.OOO.O...",
            "...OOOOOOOOO...O.....O...",
            "..O...O.....O...O.....O..",
            ".O.....O.....O...........",
        ], &pal, &em);
        let up_right = Cell::from_ascii(&[
            "..........O..............",
            ".........OeO.............",
            ".....O...OedO............",
            "....OeO..OedcO.....OO....",
            "....OedOOObdcO....OedO...",
            "...OOedO4bbbbOO..O4dcO...",
            "..O455554444433OO3m2O....",
            ".O455554444433322O22O....",
            "O45554442443332211OO.....",
            "O344444424333222111O.....",
            ".O3334442333222111O......",
            "..O2222222221111OO.......",
            "...O11111111111OO........",
            "...OOOOOOOOOOOO.O........",
            "..O...O....O.....O.......",
            ".O.....O....O.....O......",
            ".............O...........",
        ], &pal, &em);
        [right, down, up, down_right, up_right]
    });
    match dir {
        Dir::Right => (&cells[0], false),
        Dir::Left => (&cells[0], true),
        Dir::Down => (&cells[1], false),
        Dir::Up => (&cells[2], false),
        Dir::DownRight => (&cells[3], false),
        Dir::DownLeft => (&cells[3], true),
        Dir::UpRight => (&cells[4], false),
        Dir::UpLeft => (&cells[4], true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::enemies::{Brain, Enemy, EnemyKind};
    use crate::world::{Tile, ZoneId};

    #[test]
    fn native_eightway_states_cover_every_appearance_without_cardinal_substitution() {
        let sh = sheet(SHEET).unwrap();
        for dir in Dir::ALL {
            let (x, y) = dir.vec();
            for (mode, moving, base) in [
                (Mode::Stalk, false, "idle"), (Mode::Stalk, true, "move"),
                (Mode::Plant, false, "windup"), (Mode::Charge, false, "attack"),
                (Mode::Skid, false, "attack"), (Mode::Dazed, false, "dazed"),
                (Mode::Recover, false, "idle"),
            ] {
                let (mut b, mut s) = spawn(100.0, 100.0);
                b.face = (x, y); s.visual_dir = dir; s.mode = mode; s.moving = moving;
                let (a, flip) = sh.dir_anim(base, dir).expect("complete native pose");
                assert!(a.name.ends_with(dir.suffix()) || (dir == Dir::Left && flip));
                for appearance in 0..3 {
                    s.appearance = appearance;
                    let (cell, anchor, actual_flip) = current_cell(&b, &s, 0);
                    assert!(std::ptr::eq(cell, super::super::variation::cell(SHEET, sh.cell(a, 0), appearance)));
                    assert_eq!((anchor, actual_flip), (sh.anchor_of(a), flip));
                    assert!(!cell.is_empty());
                }
            }
        }
    }

    #[test]
    fn visual_sector_stability_never_changes_continuous_armor_or_charge_heading() {
        let (mut b, mut s) = spawn(100.0, 100.0);
        s.visual_dir = Dir::UpRight;
        s.ang = -0.38;
        b.face = s.facing();
        s.dir = (0.932_327, -0.361_615);
        let (angle, face, charge) = (s.ang, b.face, s.dir);
        let _ = current_cell(&b, &s, 10);
        assert_eq!((s.ang, b.face, s.dir), (angle, face, charge));
        assert_eq!(s.visual_dir.stable_from_vec(b.face.0, b.face.1), Dir::UpRight);
    }

    #[test]
    fn authored_walk_keeps_native_feet_on_the_ground_in_every_sector() {
        for dir in Dir::ALL {
            let (mut b, mut s) = spawn(192.73, 120.81);
            b.face = dir.vec(); s.visual_dir = dir; s.moving = true;
            for appearance in 0..3 {
                s.appearance = appearance;
                for tick in 0..32 {
                    s.walk_t = tick;
                    let mut f = Frame::new();
                    f.surface_lighting = true;
                    draw(&b, &s, &mut f);
                    let caster = &f.surface_casters[0];
                    assert_eq!(caster.y, b.y.floor(), "{dir:?} look{appearance} tick{tick}: extra bob lifted native soles");
                    assert_eq!(caster.min_height, 0.0);
                }
            }
        }
    }

    #[test]
    fn crest_light_matches_current_native_cap_under_mirroring() {
        let sh = sheet(SHEET).unwrap();
        for a in sh.anims.values() {
            let anchor = sh.anchor_of(a);
            for i in 0..a.frames {
                for appearance in 0..3 {
                    let cell = super::super::variation::cell(SHEET, sh.cell(a, i), appearance);
                    for flip in [false, true] {
                        if let Some((x, y)) = crest_source(cell, anchor, flip) {
                            let sx = if flip { anchor.0 as f32 - x + 0.5 } else { anchor.0 as f32 + x - 0.5 };
                            let sy = anchor.1 as f32 + y - 0.5;
                            let p = cell.at(sx.round() as i32, sy.round() as i32);
                            assert!(p.a && p.emit, "{} pose{i} appearance{appearance} mirror{flip}", a.name);
                        }
                    }
                }
            }
        }
    }

    /// Exercise armor through the real dispatcher, including its lethal-hit bookkeeping.
    #[test]
    fn armor_deflects_lethal_front_but_accepts_flank_and_dazed_hits() {
        let zone = Zone::new(ZoneId::Glasswater, 30, 20);
        let mut fx = Fx::new(7);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx {
            player: (80.0, 100.0),
            player_alive: true,
            zone: &zone,
            fx: &mut fx,
            hits: &mut hits,
            projectiles: &mut projectiles,
            t: 1,
        };
        let mut enemy = Enemy::spawn(EnemyKind::CrystalBeetle, 100.0, 100.0);
        let front = Hit::new(Shape::Circle { x: 80.0, y: 100.0, r: 24.0 }, Team::Player, HP, 1);
        assert!(enemy.on_hit(&front, &mut ctx));
        assert_eq!(enemy.body.hp, HP);
        assert!(enemy.body.alive);
        assert_eq!(enemy.body.stun, 0);
        assert!(!enemy.on_hit(&front, &mut ctx), "one swing must only clank once");
        let flank = Hit::new(Shape::Circle { x: 100.0, y: 120.0, r: 24.0 }, Team::Player, 2, 2);
        assert!(enemy.on_hit(&flank, &mut ctx));
        assert_eq!(enemy.body.hp, HP - 2);
        if let Brain::CrystalBeetle(s) = &mut enemy.brain {
            s.set(Mode::Dazed);
        }
        let finish = Hit { id: 3, ..front };
        assert!(enemy.on_hit(&finish, &mut ctx));
        assert!(!enemy.body.alive);
        assert!(!ctx.fx.particles.list.is_empty());
    }

    /// The last beat commits to a heading even if Glint moves to the opposite flank.
    #[test]
    fn locked_charge_keeps_direction_and_stuns_on_a_glancing_wall() {
        let mut zone = Zone::new(ZoneId::Glasswater, 30, 20);
        for y in 0..20 {
            zone.set_tile(10, y, Tile::Wall);
        }
        let mut fx = Fx::new(7);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx {
            player: (80.0, 200.0),
            player_alive: true,
            zone: &zone,
            fx: &mut fx,
            hits: &mut hits,
            projectiles: &mut projectiles,
            t: 1,
        };
        let (mut b, mut s) = spawn(145.0, 100.0);
        s.ang = 1.2;
        let (len, wall) = cast(&zone, b.x, b.y, s.facing());
        begin_plant(&mut s, len, wall, AIM);
        let heading = s.dir;
        for _ in 0..LOCK {
            update(&mut b, &mut s, &mut ctx);
        }
        assert_eq!(s.mode, Mode::Charge);
        assert_eq!(s.dir, heading);
        for _ in 0..80 {
            update(&mut b, &mut s, &mut ctx);
            if s.mode == Mode::Dazed {
                break;
            }
        }
        assert_eq!(s.mode, Mode::Dazed);
        assert!(!zone.blocked(b.x, b.y, R));
        assert!(((b.x - 145.0) * heading.1 - (b.y - 100.0) * heading.0).abs() < 0.01, "wall sliding must not bend a charge");
        assert_eq!(s.daze_len, DAZE);
    }
}
