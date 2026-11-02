//! Sproutling: a hopping bulb creature, Bloomwild's fodder enemy. It bounces toward Glint in
//! little hops, fans out around Glint with the rest of its group, and the group takes turns
//! headbutting:
//!
//! 1. Windup (32 ticks): it rears back, the leaves flare, the eyes ignite red. The aim follows
//!    Glint until 10 ticks before the lunge, then locks; at the lock it flashes and shivers.
//! 2. Headbutt (10 ticks): a fast lunge along the locked aim. If it reaches Glint it boings
//!    off; if it smacks a wall it gets dizzy.
//! 3. Recover / dizzy: a free window to hit it back.
//!
//! On death the bulb bursts into pixels toward the hit direction, juice splashes, and the leaf
//! sprig pops off in one piece and lands flat on the grass.
//!
//! `kit` at the bottom holds helpers shared with `thorn_imp.rs` (the other Bloomwild enemy).

use super::EnemyCtx;
use crate::combat::{Hit, Shape, Team};
use crate::entities::Body;
use crate::fx::Particle;
use crate::gfx::{Anim, Cell, Dir, DrawOpts, Frame, Rgb, rgb, sheet};
use crate::util::{Rng, norm};
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;

const SHEET: &str = "sproutling";
const HP: i32 = 3;

// Timings in 60 Hz ticks. The hop and the headbutt poses follow the sheet's own durations.
const WINDUP: u32 = 32;
/// The aim stops following Glint here: the last WINDUP - AIM_LOCK ticks are committed.
const AIM_LOCK: u32 = 22;
const BUTT: u32 = 10;
const RECOVER: u32 = 30;
const DIZZY: u32 = 70;
/// Only one sproutling in the whole zone may start a windup per this many ticks.
const TURN_GAP: u64 = 40;
/// Start a headbutt when this close (the lunge covers about 31 px plus the hit radius).
const ENGAGE: f32 = 38.0;

// Palette bits for code-drawn touches (the sheet's own colors).
const OUTLINE: Rgb = rgb(0x303b4d);
/// The sage thorn sprig and hooked limbs in the style v2 sheet.
const LEAF: [u32; 6] = [0x3b535b, 0x587278, 0x829b8e, 0x3b535b, 0x587278, 0x829b8e];
/// Crown and belly colors: juice, flecks.
const JUICE: [u32; 4] = [0xbf9da9, 0xdbb9bd, 0x927c91, 0x655f78];
/// What outline pixels turn into when the bulb bursts.
const DEEP: Rgb = rgb(0x655f78);
/// Dried-rose shadow color for the lunge afterimages.
const PALE: Rgb = rgb(0x927c91);
/// The code-drawn fallback's bulb ramp.
const BULB: [u32; 5] = [0x655f78, 0x927c91, 0xbf9da9, 0xdbb9bd, 0xdfdac1];
const DANGER: Rgb = rgb(0xbd7a85);
const STAR: Rgb = rgb(0xc2c1b3);
const DUST: Rgb = rgb(0x92a594);

/// One turn token shared by every sproutling (see `kit::claim`).
static TURN: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub(super) fn reset_turn() { TURN.store(0, std::sync::atomic::Ordering::Relaxed); }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Just woke up: a startled hop in place after a short random delay.
    Notice,
    Rest,
    Hop,
    Windup,
    Butt,
    Recover,
    Dizzy,
    Hurt,
}

pub struct State {
    phase: Phase,
    /// Ticks in the current phase.
    pt: u32,
    /// Phase length chosen when it started (rest length, notice delay).
    len: u32,
    /// Hop velocity, applied on the hop's airborne frames only.
    hop: (f32, f32),
    aim: (f32, f32),
    hit_id: u32,
    rng: Rng,
    /// +1 / -1: which way it circles Glint while waiting for its turn.
    orbit: f32,
    /// Ticks before it may headbutt again.
    cool: u32,
    /// Danger-pink pulse at the moment the aim locks (decays).
    commit: f32,
    /// Afterimages of the lunge: (x, y, life).
    trail: Vec<(f32, f32, f32)>,
    /// Attack frame to show during the lunge (jumps to the bounce pose on contact).
    butt_frame: Option<usize>,
    started: bool,
    /// Idle animation offset so a group doesn't breathe in sync.
    idle_off: u32,
    appearance: u8,
}

impl State {
    /// Enter a behavior phase and restart its local clock.
    fn set(&mut self, p: Phase, len: u32) {
        self.phase = p;
        self.pt = 0;
        self.len = len;
    }
}

/// Create an independently paced Bloomwild enemy at its ground position.
pub fn spawn(x: f32, y: f32) -> (Body, State) {
    let mut rng = Rng::new((x as u64).wrapping_mul(73_856_093) ^ (y as u64).wrapping_mul(19_349_663) ^ 0x5eed);
    let orbit = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let cool = rng.irange(20, 70) as u32;
    let idle_off = rng.irange(0, 40) as u32;
    let s = State { phase: Phase::Notice, pt: 0, len: 0, hop: (0.0, 0.0), aim: (0.0, 1.0), hit_id: 0, rng, orbit, cool, commit: 0.0, trail: Vec::new(), butt_frame: None, started: false, idle_off, appearance: super::variation::appearance(x, y) };
    (Body::new(x, y, 4.0, HP), s)
}

// ------------------------------------------------------------------------------- behavior

/// Advance behavior, telegraphs, and body physics by one simulation tick.
pub fn update(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    if !s.started {
        s.started = true;
        let delay = s.rng.irange(1, 19) as u32;
        s.set(Phase::Notice, delay);
    }
    s.pt += 1;
    s.cool = s.cool.saturating_sub(1);
    s.commit = (s.commit - 0.1).max(0.0);
    for t in s.trail.iter_mut() {
        t.2 -= 0.09;
    }
    s.trail.retain(|t| t.2 > 0.0);

    let (px, py) = ctx.player;
    let d = b.dist_to(px, py);
    let to = b.dir_to(px, py);
    let vol = hear(d);

    if b.stun > 0 && s.phase != Phase::Hurt {
        s.set(Phase::Hurt, 0);
    }

    match s.phase {
        Phase::Notice => {
            stop(b);
            b.face = to;
            if s.pt == s.len {
                // a startled little jump
                b.vz = 1.8;
                ctx.fx.sound_vol("sprout_hop", 0.5 * vol);
            }
            if s.pt > s.len + 2 && b.z == 0.0 {
                let fx = &mut *ctx.fx;
                fx.particles.dust(&mut fx.rng, b.x, b.y, 3, DUST);
                let rest = s.rng.irange(4, 16) as u32;
                s.set(Phase::Rest, rest);
            }
        }
        Phase::Rest => {
            stop(b);
            if ctx.player_alive {
                b.face = to;
            }
            if s.pt >= s.len {
                decide(b, s, ctx, d, to);
            }
        }
        Phase::Hop => {
            let hop = hop_timing();
            let (air, takeoff, landing) = hop.at(s.pt - 1);
            if air {
                b.vx = s.hop.0;
                b.vy = s.hop.1;
            } else {
                stop(b);
            }
            if ctx.player_alive {
                b.face = to;
            }
            if takeoff {
                ctx.fx.sound_vol("sprout_hop", 0.4 * vol);
                let fx = &mut *ctx.fx;
                fx.particles.dust(&mut fx.rng, b.x, b.y, 2, DUST);
            }
            if landing {
                let fx = &mut *ctx.fx;
                fx.particles.dust(&mut fx.rng, b.x, b.y, 3, DUST);
            }
            if s.pt >= hop.total {
                let rest = if d > 110.0 { s.rng.irange(0, 6) } else { s.rng.irange(6, 22) } as u32;
                s.set(Phase::Rest, rest);
            }
        }
        Phase::Windup => {
            stop(b);
            if s.pt < AIM_LOCK && ctx.player_alive {
                s.aim = to;
            }
            b.face = s.aim;
            // pawing the ground: dust kicked out behind it
            if s.pt % 7 == 3 && s.pt < AIM_LOCK {
                let fx = &mut *ctx.fx;
                let (bx, by) = (b.x - s.aim.0 * 5.0, b.y - s.aim.1 * 2.0);
                for _ in 0..2 {
                    let sp = fx.rng.range(0.4, 0.9);
                    fx.particles.add(
                        Particle::new(bx, by)
                            .z(1.0)
                            .vel(-s.aim.0 * sp + fx.rng.signed() * 0.2, -s.aim.1 * sp * 0.6 + fx.rng.signed() * 0.15, fx.rng.range(0.5, 1.1))
                            .color(DUST)
                            .life(fx.rng.range(14.0, 22.0))
                            .grav(0.1)
                            .drag(0.9),
                    );
                }
            }
            if s.pt == AIM_LOCK {
                s.commit = 1.0;
            }
            if s.pt >= WINDUP {
                s.set(Phase::Butt, BUTT);
                s.hit_id = ctx.fx.hit_id();
                s.butt_frame = None;
                s.trail.clear();
                ctx.fx.sound_vol("sprout_hop", 0.9 * vol.max(0.5));
                let fx = &mut *ctx.fx;
                for _ in 0..5 {
                    let sp = fx.rng.range(0.5, 1.4);
                    fx.particles.add(
                        Particle::new(b.x - s.aim.0 * 3.0, b.y)
                            .z(0.5)
                            .vel(-s.aim.0 * sp + fx.rng.signed() * 0.4, -s.aim.1 * sp * 0.6 + fx.rng.signed() * 0.3, fx.rng.range(0.3, 1.0))
                            .color(DUST)
                            .life(fx.rng.range(16.0, 26.0))
                            .grav(0.08)
                            .drag(0.88),
                    );
                }
            }
        }
        Phase::Butt => {
            // fast for the two stretched frames, then braking through the bounce pose
            let sp = if s.pt <= 5 { 5.2 - s.pt as f32 * 0.2 } else { (3.0 - (s.pt - 6) as f32 * 0.65).max(0.3) };
            b.vx = s.aim.0 * sp;
            b.vy = s.aim.1 * sp;
            b.face = s.aim;
            let (hx, hy) = (b.x + s.aim.0 * 4.0, b.y - 3.0 + s.aim.1 * 2.0);
            ctx.hits.push(Hit::new(Shape::Circle { x: hx, y: hy, r: b.r + 2.0 }, Team::Enemy, 1, s.hit_id).knock(2.8).dir(s.aim.0, s.aim.1));
            if s.pt % 2 == 1 && s.pt < 7 {
                s.trail.push((b.x, b.y, 0.55));
            }
            // reached Glint: boing off (whether or not the hit landed through i-frames)
            let contact = ctx.player_alive && ((hx - px).powi(2) + (hy - (py - 2.0)).powi(2)).sqrt() < b.r + 6.0;
            if contact {
                b.kx = -s.aim.0 * 2.2;
                b.ky = -s.aim.1 * 2.2;
                stop(b);
                b.vz = 1.6;
                s.butt_frame = Some(2);
                s.set(Phase::Recover, RECOVER);
                s.cool = s.rng.irange(90, 150) as u32;
            } else if s.pt >= BUTT {
                s.set(Phase::Recover, RECOVER);
                s.cool = s.rng.irange(90, 150) as u32;
            }
        }
        Phase::Recover => {
            b.vx *= 0.7;
            b.vy *= 0.7;
            if s.pt >= s.len {
                s.butt_frame = None;
                let rest = s.rng.irange(10, 24) as u32;
                s.set(Phase::Rest, rest);
            }
        }
        Phase::Dizzy => {
            b.vx *= 0.7;
            b.vy *= 0.7;
            if s.pt >= s.len {
                let rest = s.rng.irange(4, 12) as u32;
                s.set(Phase::Rest, rest);
            }
        }
        Phase::Hurt => {
            stop(b);
            if b.stun == 0 && b.z == 0.0 {
                let rest = s.rng.irange(6, 16) as u32;
                s.set(Phase::Rest, rest);
            }
        }
    }

    let blocked = b.integrate(ctx.zone);

    // Smacked into a wall mid-lunge: bonk, bounce back, see stars.
    if s.phase == Phase::Butt && blocked && s.pt > 1 && s.pt < 7 {
        b.kx = -s.aim.0 * 2.4;
        b.ky = -s.aim.1 * 2.4;
        stop(b);
        b.vz = 2.0;
        s.set(Phase::Dizzy, DIZZY);
        s.cool = s.rng.irange(100, 160) as u32;
        let fx = &mut *ctx.fx;
        fx.shake(0.12);
        fx.sound_vol("pot_break", 0.25 * vol);
        let (wx, wy) = (b.x + s.aim.0 * 5.0, b.y + s.aim.1 * 2.0);
        fx.particles.chips(&mut fx.rng, wx, wy, 5.0, -s.aim.0, -s.aim.1, 5, STAR, 1.6);
        fx.particles.dust(&mut fx.rng, wx, wy, 4, DUST);
    }
}

/// Stop voluntary motion while preserving knockback.
fn stop(b: &mut Body) {
    b.vx = 0.0;
    b.vy = 0.0;
}

/// Volume falloff so a group off screen doesn't fill the mix.
fn hear(d: f32) -> f32 {
    (1.0 - (d - 120.0) / 160.0).clamp(0.0, 1.0)
}

/// Choose the next action from a rest: a headbutt if it's this sproutling's turn, otherwise a
/// hop that closes in, or circles Glint at a waiting distance.
fn decide(b: &Body, s: &mut State, ctx: &mut EnemyCtx, d: f32, to: (f32, f32)) {
    let (px, py) = ctx.player;
    if ctx.player_alive && s.cool == 0 && d < ENGAGE && ctx.zone.line_clear(b.x, b.y - 4.0, px, py - 4.0) && kit::claim(&TURN, ctx.t, TURN_GAP) {
        s.aim = to;
        s.set(Phase::Windup, WINDUP);
        ctx.fx.sound_vol("enemy_windup", 0.35 * hear(d));
        return;
    }
    if s.rng.chance(0.12) {
        s.orbit = -s.orbit;
    }
    let (dir, dist) = if !ctx.player_alive {
        (s.rng.dir(), s.rng.range(8.0, 16.0))
    } else {
        // Waiting ring: just outside the lunge range, so a group circles instead of piling in.
        let ring = if s.cool == 0 { ENGAGE - 8.0 } else { 54.0 };
        let radial = ((d - ring) / 22.0).clamp(-1.0, 1.0);
        let tang = if d < 100.0 { 0.8 } else { 0.25 };
        let perp = (-to.1 * s.orbit, to.0 * s.orbit);
        let j = s.rng.signed() * 0.25;
        let dir = norm(to.0 * radial + perp.0 * tang + j * perp.0, to.1 * radial + perp.1 * tang + j * perp.1);
        let dist = if d > 100.0 { s.rng.range(22.0, 28.0) } else { s.rng.range(14.0, 22.0) };
        (dir, dist)
    };
    // Don't hop into walls or off the island: try a few rotations, else hop in place.
    let mut pick = None;
    for rot in [0.0f32, 0.6, -0.6, 1.2, -1.2, 2.2, -2.2] {
        let (c, sn) = (rot.cos(), rot.sin());
        let r = (dir.0 * c - dir.1 * sn, dir.0 * sn + dir.1 * c);
        let ok = (1..=3).all(|i| {
            let k = dist * i as f32 / 3.0;
            !ctx.zone.blocked(b.x + r.0 * k, b.y + r.1 * k, b.r)
        });
        if ok {
            pick = Some(r);
            break;
        }
    }
    let air = hop_timing().air.max(1) as f32;
    let hop = match pick {
        Some(r) => (r.0 * dist / air, r.1 * dist / air),
        None => {
            s.orbit = -s.orbit;
            (0.0, 0.0)
        }
    };
    s.hop = hop;
    s.set(Phase::Hop, 0);
}

/// One hop, timed by the sheet's `move` anim: crouch, airborne frames (the only ticks the body
/// moves, so the base never slides), landing squash.
struct HopTiming {
    /// Airborne frame range in ticks: [start, end).
    start: u32,
    end: u32,
    air: u32,
    total: u32,
}

impl HopTiming {
    /// For tick `t` of the hop: (airborne, takeoff tick, landing tick).
    fn at(&self, t: u32) -> (bool, bool, bool) {
        ((self.start..self.end).contains(&t), t == self.start, t == self.end)
    }
}

/// Read the authored takeoff and landing ticks so the bulb never slides on the ground.
fn hop_timing() -> HopTiming {
    if let Some(a) = sheet(SHEET).and_then(|sh| sh.anim("move_right")) {
        let air = air_frames(a);
        let (mut start, mut end, mut t) = (u32::MAX, 0, 0);
        for (i, d) in a.durations.iter().enumerate() {
            if air.contains(&i) {
                start = start.min(t);
                end = t + d;
            }
            t += d;
        }
        if start < end {
            return HopTiming { start, end, air: end - start, total: a.total_ticks() };
        }
    }
    HopTiming { start: 5, end: 21, air: 16, total: 28 }
}

/// Read the sprite manifest's airborne frame indices, with an older-sheet fallback.
fn air_frames(a: &Anim) -> Vec<usize> {
    a.json.get("airborne").and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_u64()).map(|x| x as usize).collect()).unwrap_or_else(|| vec![1, 2, 3, 4])
}

// ------------------------------------------------------------------------------- reactions

/// Interrupt the current action and shed a few pixels along the hit direction.
pub fn on_hit(b: &mut Body, s: &mut State, hit: &Hit, ctx: &mut EnemyCtx) {
    // Any hit interrupts a windup or lunge: aggression is rewarded.
    s.set(Phase::Hurt, 0);
    s.trail.clear();
    s.butt_frame = None;
    s.cool = s.cool.max(40);
    // knocked up off the grass a little
    if b.z < 2.0 {
        b.vz = if hit.heavy { 2.2 } else { 1.4 };
    }
    let (dx, dy) = hit.push_dir(b.x, b.y);
    let fx = &mut *ctx.fx;
    // a couple of leaf flecks and a drop of juice fly off the far side
    for i in 0..3 {
        let c = if i == 2 { rgb(JUICE[fx.rng.irange(0, 3) as usize]) } else { rgb(LEAF[fx.rng.irange(0, 3) as usize]) };
        let sp = fx.rng.range(0.8, 1.8);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(fx.rng.range(6.0, 11.0))
                .vel(dx * sp + fx.rng.signed() * 0.5, dy * sp * 0.7 + fx.rng.signed() * 0.4, fx.rng.range(0.8, 1.8))
                .color(c)
                .life(fx.rng.range(90.0, 160.0))
                .grav(0.14)
                .bounce(0.25)
                .settle(true),
        );
    }
}

/// Break the body into settling pixels while preserving its signature piece.
pub fn on_death(b: &Body, s: &State, ctx: &mut EnemyCtx) {
    let (cell, anchor, flip) = current_cell(b, s, 0);
    let dir = norm(b.kx, b.ky);
    let fx = &mut *ctx.fx;
    // The leaf sprig (and its outline) flies off as one piece and lands flat on the grass.
    let is_leaf = |c: &Cell, x: i32, y: i32| kit::part_of(c, x, y, &LEAF);
    let up = (dir.0 * 0.7 + fx.rng.signed() * 0.4, dir.1 * 0.5 + fx.rng.signed() * 0.3, 2.6);
    let o = kit::Burst { power: 1.0, life: 420.0, dir, outline: (OUTLINE, DEEP), chunk: &is_leaf, chunk_v: up, chunk_life: 140.0 };
    kit::burst(fx, cell, anchor, flip, b.x, b.y, b.z, &o);
    // juice splash
    for _ in 0..12 {
        let c = rgb(JUICE[fx.rng.irange(0, 4) as usize]);
        let a = dir.1.atan2(dir.0) + fx.rng.signed() * 1.1;
        let sp = fx.rng.range(0.6, 2.2);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(fx.rng.range(3.0, 8.0))
                .vel(a.cos() * sp, a.sin() * sp * 0.7, fx.rng.range(0.6, 2.0))
                .color(c)
                .life(fx.rng.range(260.0, 380.0))
                .grav(0.18)
                .bounce(0.15)
                .settle(true),
        );
    }
    fx.particles.dust(&mut fx.rng, b.x, b.y, 5, DUST);
    fx.sound_vol("grass_cut", 0.5);
}

// ------------------------------------------------------------------------------- drawing

/// The sprite cell for the current pose: (cell, anchor, flip). `t` animates idle poses.
fn current_cell(b: &Body, s: &State, t: u64) -> (&'static Cell, (i32, i32), bool) {
    let dir = Dir::from_vec(b.face.0, b.face.1);
    if let Some(sh) = sheet(SHEET) {
        let (names, frame): (&[&str], Frame_) = match s.phase {
            Phase::Notice if b.z > 0.0 => (&["move"], Frame_::Fixed(1)),
            Phase::Notice | Phase::Rest => (&["idle"], Frame_::Loop(t as u32 + s.idle_off)),
            Phase::Hop => (&["move", "idle"], Frame_::Once(s.pt.saturating_sub(1))),
            Phase::Windup => (&["windup", "idle"], Frame_::Shiver(s.pt)),
            Phase::Butt => match s.butt_frame {
                Some(i) => (&["attack", "idle"], Frame_::Fixed(i)),
                None => (&["attack", "idle"], Frame_::Once(s.pt)),
            },
            Phase::Recover => {
                if s.pt < 8 {
                    (&["attack", "idle"], Frame_::Fixed(if s.butt_frame.is_some() && s.pt < 3 { 2 } else { 3 }))
                } else {
                    (&["idle"], Frame_::Loop(s.pt))
                }
            }
            Phase::Dizzy => (&["hurt", "idle"], Frame_::Fixed(1)),
            Phase::Hurt => (&["hurt", "idle"], Frame_::Once(s.pt)),
        };
        if let Some((a, flip)) = kit::find(sh, names, dir) {
            let i = match frame {
                Frame_::Loop(k) => a.frame_at(k, true),
                Frame_::Once(k) => a.frame_at(k, false),
                Frame_::Fixed(i) => i.min(a.frames - 1),
                Frame_::Shiver(k) => {
                    // play through once, then alternate the last two frames (the shiver)
                    let n = a.frames;
                    let lead: u32 = a.durations[..n.saturating_sub(2)].iter().sum();
                    if n < 3 || k < lead { a.frame_at(k, false) } else { n - 2 + ((k - lead) / 3) as usize % 2 }
                }
            };
            return (super::variation::cell(SHEET, sh.cell(a, i), s.appearance), sh.anchor_of(a), flip);
        }
    }
    let c = fallback_cell();
    (c, (c.w / 2, c.h - 1), dir == Dir::Left)
}

enum Frame_ {
    Loop(u32),
    Once(u32),
    Fixed(usize),
    /// One-shot that keeps shivering on its last two frames.
    Shiver(u32),
}

/// Draw the directional pose and its restrained combat cues.
pub fn draw(b: &Body, s: &State, f: &mut Frame) {
    let (cell, anchor, flip) = current_cell(b, s, f.t);
    // Shadow shrinks as the bulb leaves the ground (the hop arc is drawn into the cells).
    let lift = kit::lift(cell, anchor) as f32 + b.z;
    let k = (lift / 9.0).min(1.0);
    let rx = (5.0 - k * 2.0).round() as i32;
    f.shadow(f.sx(b.x), f.sy(b.y), rx, if k > 0.5 { 1 } else { 2 }, 0.42 - 0.14 * k);

    // Committed: a 1 px tremble on top of the sheet's own shiver frames.
    let ox = if s.phase == Phase::Windup && s.pt >= AIM_LOCK && (s.pt / 2) % 2 == 0 { 1.0 } else { 0.0 };
    // Lunge smear: dithered afterimages of the stretched frame in pale belly color.
    if !s.trail.is_empty()
        && let Some(sh) = sheet(SHEET)
        && let Some((a, fl)) = kit::find(sh, &["attack"], Dir::from_vec(s.aim.0, s.aim.1))
    {
        let c = super::variation::cell(SHEET, sh.cell(a, 0), s.appearance);
        for &(tx, ty, life) in &s.trail {
            let o = DrawOpts { flip: fl, alpha: life, silhouette: Some(PALE), ..Default::default() };
            f.sprite_w(c, sh.anchor_of(a), tx, ty, &o);
        }
    }
    // The aim lock pulses danger pink (white is reserved for taking a hit).
    let tint = (s.commit > 0.0).then_some((DANGER, s.commit * 0.6));
    let o = DrawOpts { actor: true, foot_depth: 3, foot_half_w: b.r, ground_y: Some(b.y.floor()), elevation: b.z, flip, flash: b.flash, tint, ..Default::default() };
    f.sprite_w(cell, anchor, b.x + ox, b.y - b.z, &o);

    // Dizzy: two short, matte husk marks circling over its head.
    if s.phase == Phase::Dizzy {
        let top = f.sy(b.y - b.z) - 15;
        let cx = f.sx(b.x);
        for i in 0..2 {
            let a = s.pt as f32 * 0.16 + i as f32 * std::f32::consts::PI;
            let (x, y) = (cx + (a.cos() * 5.0).round() as i32, top + (a.sin() * 1.6).round() as i32);
            let c = if a.sin() > 0.0 { STAR } else { STAR.scale(0.6) };
            f.put(x, y, c);
            f.put(x + 1, y, c);
        }
    }
}

/// Code-drawn stand-in used only if the sheet is missing (hand-placed, same palette).
fn fallback_cell() -> &'static Cell {
    static C: OnceLock<Cell> = OnceLock::new();
    C.get_or_init(|| {
        let rows = [
            ".dd.....bc.",
            "dccd...bbba",
            ".cccd.bbba.",
            "..ccccbba..",
            "....443....",
            "..4444433..",
            ".4454o333o.",
            "445545o3o53",
            "44443553552",
            "444333333o2",
            "33333oooo22",
            ".333333221.",
            "..2222211..",
        ];
        let pal = [
            ('o', OUTLINE),
            ('1', rgb(BULB[0])),
            ('2', rgb(BULB[1])),
            ('3', rgb(BULB[2])),
            ('4', rgb(BULB[3])),
            ('5', rgb(BULB[4])),
            ('a', rgb(LEAF[0])),
            ('b', rgb(LEAF[1])),
            ('c', rgb(LEAF[2])),
            ('d', rgb(LEAF[3])),
        ];
        kit::outlined(&rows, &pal, OUTLINE)
    })
}

/// Helpers shared by the Bloomwild enemies (`sproutling.rs` and `thorn_imp.rs`).
pub(super) mod kit {
    use crate::fx::{Debris, Fx, Particle};
    use crate::gfx::{Anim, Cell, Dir, Px, Rgb, Sheet};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// The first anim in `names` the sheet has (directional variants resolved), with its flip.
    pub fn find<'a>(sh: &'a Sheet, names: &[&str], dir: Dir) -> Option<(&'a Anim, bool)> {
        names.iter().find_map(|n| sh.dir_anim(n, dir))
    }

    /// How far the lowest opaque pixel sits above the anchor row (hop arcs drawn into cells).
    pub fn lift(cell: &Cell, anchor: (i32, i32)) -> i32 {
        for y in (0..cell.h).rev() {
            if (0..cell.w).any(|x| cell.at(x, y).a) {
                return (anchor.1 - y).max(0);
            }
        }
        0
    }

    /// Group pacing: one turn token per enemy kind. Returns true (and takes the token) if no
    /// other enemy of the kind took it in the last `gap` ticks. A new zone restarts `t`.
    pub fn claim(slot: &AtomicU64, t: u64, gap: u64) -> bool {
        // Store t + 1 so tick zero is distinct from an unused token.
        let stamp = slot.load(Ordering::Relaxed);
        if stamp != 0 {
            let last = stamp - 1;
            if t >= last && t - last < gap {
                return false;
            }
        }
        slot.store(t.saturating_add(1), Ordering::Relaxed);
        true
    }

    /// A ground-layer pixel (drawn under characters) for this tick only: a 2-tick debris pixel.
    pub fn ground_px(fx: &mut Fx, x: i32, y: i32, c: Rgb, glow: f32) {
        if fx.particles.debris.len() < 15_500 {
            fx.particles.debris.push(Debris { x, y, c, glow, life: 2, max_life: 2 });
        }
    }

    /// Is pixel (x, y) part of the piece made of `ramp` colors? Outline pixels count when they
    /// touch the piece and nothing else.
    pub fn part_of(c: &Cell, x: i32, y: i32, ramp: &[u32]) -> bool {
        let p = c.at(x, y);
        if !p.a {
            return false;
        }
        let key = p.c.key();
        if ramp.contains(&key) {
            return true;
        }
        let n = [(1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dy)| c.at(x + dx, y + dy));
        let touches_piece = n.iter().any(|q| q.a && ramp.contains(&q.c.key()));
        let touches_other = n.iter().any(|q| q.a && !ramp.contains(&q.c.key()) && q.c.key() != key);
        touches_piece && !touches_other
    }

    /// How a sprite bursts on death (see `burst`).
    pub struct Burst<'a> {
        /// Speed of the spray; 1 = a tight splat around the body.
        pub power: f32,
        /// Ticks the settled pixels lie on the ground.
        pub life: f32,
        /// Direction of the killing blow: the spray leans this way.
        pub dir: (f32, f32),
        /// Outline pixels become the second color (and half are dropped), so the splat isn't
        /// peppered with near-black specks.
        pub outline: (Rgb, Rgb),
        /// Pixels that fly off together as one rigid piece (a leaf, a mask)...
        pub chunk: &'a dyn Fn(&Cell, i32, i32) -> bool,
        /// ...with this velocity (x, y, z)...
        pub chunk_v: (f32, f32, f32),
        /// ...and lie flat on the ground this long once they land, as a decal.
        pub chunk_life: f32,
    }

    /// Burst a cell into pixels like `Particles::burst_cell`, drawn as if anchored at ground
    /// (wx, wy) and lifted by `z`. The spray is tighter than the engine's and leans along the
    /// killing blow; the chunk pixels keep their shape and land flat.
    #[allow(clippy::too_many_arguments)]
    pub fn burst(fx: &mut Fx, cell: &Cell, anchor: (i32, i32), flip: bool, wx: f32, wy: f32, z: f32, o: &Burst) {
        // the chunk keeps its shape: every pixel shares one height (its bottom) and velocity
        let mut chunk_base = i32::MAX;
        for y in 0..cell.h {
            for x in 0..cell.w {
                if (o.chunk)(cell, x, y) {
                    chunk_base = chunk_base.min(anchor.1 - y);
                }
            }
        }
        let chunk_z = chunk_base.max(0) as f32 + z;
        let mut spray = cell.clone();
        for y in 0..cell.h {
            for x in 0..cell.w {
                let p = cell.at(x, y);
                if !p.a {
                    continue;
                }
                let dst = &mut spray.px[(y * cell.w + x) as usize];
                if chunk_base != i32::MAX && (o.chunk)(cell, x, y) {
                    // Keep the leaf or mask intact; the remaining body uses the engine burst.
                    dst.a = false;
                    let lx = if flip { cell.w - 1 - x } else { x };
                    let ax = if flip { cell.w - 1 - anchor.0 } else { anchor.0 };
                    let h = (anchor.1 - y) as f32;
                    fx.particles.add(
                        Particle::new(wx + (lx - ax) as f32, wy - (h + z - chunk_z))
                            .z(chunk_z)
                            .vel(o.chunk_v.0, o.chunk_v.1, o.chunk_v.2)
                            .color(p.c)
                            .glow(if p.emit { 1.0 } else { 0.0 })
                            .life(o.chunk_life)
                            .grav(0.16)
                            .drag(0.96)
                            .bounce(0.3)
                            .settle(true),
                    );
                } else if p.c.key() == o.outline.0.key() {
                    dst.a = !fx.rng.chance(0.5);
                    dst.c = o.outline.1;
                }
            }
        }
        let start = fx.particles.list.len();
        fx.particles.burst_cell(&mut fx.rng, &spray, anchor, flip, wx, wy, o.power, o.life);
        for p in &mut fx.particles.list[start..] {
            p.z += z;
            // A tight directional splat, rather than a cloud covering the next enemy.
            let push = o.power * fx.rng.range(0.2, 0.7);
            p.vx = p.vx * 0.65 + o.dir.0 * push;
            p.vy = p.vy * 0.65 + o.dir.1 * push * 0.8;
            p.drag = 0.92;
            if p.glow > 0.0 {
                p.life *= 0.3;
                p.max_life = p.life;
            }
        }
    }

    /// Build a cell from hand-placed ASCII rows and add a 1 px outline around the silhouette.
    pub fn outlined(rows: &[&str], pal: &[(char, Rgb)], outline: Rgb) -> Cell {
        let inner = Cell::from_ascii(rows, pal, &[]);
        let (w, h) = (inner.w + 2, inner.h + 2);
        let mut px = vec![Px::default(); (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let p = inner.at(x - 1, y - 1);
                if p.a {
                    px[(y * w + x) as usize] = p;
                } else if [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| inner.at(x - 1 + dx, y - 1 + dy).a) {
                    px[(y * w + x) as usize] = Px { c: outline, a: true, emit: false };
                }
            }
        }
        Cell::new(w, h, px)
    }

    /// Same as `outlined`, with some characters marked emissive.
    pub fn outlined_emit(rows: &[&str], pal: &[(char, Rgb)], emissive: &[char], outline: Rgb) -> Cell {
        let mut c = outlined(rows, pal, outline);
        let src = Cell::from_ascii(rows, pal, emissive);
        for y in 0..src.h {
            for x in 0..src.w {
                if src.at(x, y).emit {
                    c.px[((y + 1) * c.w + x + 1) as usize].emit = true;
                }
            }
        }
        c.refresh_surface_mask();
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simultaneous attackers share one token, including tick zero and a restarted zone clock.
    #[test]
    fn group_turns_include_tick_zero() {
        let slot = AtomicU64::new(0);
        assert!(kit::claim(&slot, 0, 40));
        assert!(!kit::claim(&slot, 0, 40));
        assert!(!kit::claim(&slot, 39, 40));
        assert!(kit::claim(&slot, 40, 40));
        assert!(kit::claim(&slot, 1, 40));
        assert!(!kit::claim(&slot, 1, 40));
    }

    /// A sword hit cancels an active headbutt before it can emit another damaging hit.
    #[test]
    fn hit_interrupts_lunge() {
        let zone = crate::world::Zone::new(crate::world::ZoneId::Bloomwild, 30, 20);
        let (mut b, mut s) = spawn(120.0, 120.0);
        let mut fx = crate::fx::Fx::new(7);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx { player: (140.0, 120.0), player_alive: true, zone: &zone, fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 100 };
        s.started = true;
        s.set(Phase::Butt, BUTT);
        let hit = Hit::new(Shape::Circle { x: 100.0, y: 120.0, r: 30.0 }, Team::Player, 1, ctx.fx.hit_id());
        assert!(b.take(&hit));
        on_hit(&mut b, &mut s, &hit, &mut ctx);
        update(&mut b, &mut s, &mut ctx);
        assert_eq!(s.phase, Phase::Hurt);
        assert!(ctx.hits.is_empty());
        assert!(b.x > 120.0, "hit knockback must survive the interrupted movement");
    }
}
