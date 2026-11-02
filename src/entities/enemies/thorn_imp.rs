//! Thorn imp: a wiry bramble creature in a carved bone mask. It keeps its distance, circling
//! Glint at about 80 px, and throws seeds:
//!
//! - Straight seed: it raises the seed overhead (34 ticks). The seed and the mask's eyes warm
//!   up; the aim locks 8 ticks before release with a single bright glint on the seed.
//! - Volley (every third throw, at range): three seeds lobbed high. A pink ring on the grass
//!   marks exactly where each one lands (the ring is the real hit radius) and tightens as the
//!   seed comes down.
//!
//! Chase it and it backs off; corner it and it panics, scurrying away sideways (sweating), then
//! throws again soon after. Imps take turns across the zone so a group doesn't fire at once.
//! On death the body bursts into pixels and the mask pops off whole, its eyes go dark, and it
//! lies on the grass for a while.

use super::EnemyCtx;
use super::sproutling::kit;
use crate::combat::Hit;
use crate::entities::{Body, Projectile};
use crate::fx::Particle;
use crate::gfx::{Cell, Dir, DrawOpts, Frame, Rgb, rgb, sheet};
use crate::util::{Rng, norm};
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;

const SHEET: &str = "thorn_imp";
const SEED_SHEET: &str = "seed";
const HP: i32 = 4;

const WINDUP: u32 = 34;
const WINDUP_VOLLEY: u32 = 44;
/// Ticks before release when the aim locks and the seed glints.
const LOCK_BEFORE: u32 = 8;
const THROW: u32 = 22;
const FLEE: u32 = 32;
const TURN_GAP: u64 = 50;
const SEED_SPEED: f32 = 2.1;
const LOB_GRAV: f32 = 0.14;
/// Seeds are thrown from about this height.
const HAND_Z: f32 = 12.0;

const OUTLINE: Rgb = rgb(0x303b4d);
const BRAMBLE: [u32; 4] = [0x3b535b, 0x465b61, 0x647c77, 0x829b8e];
const MASK: [u32; 6] = [0x747d8d, 0xa1a4a5, 0xc2c1b3, 0xdfdac1, 0x8c7786, 0x8c7786];
const PAINT: u32 = 0x8c7786;
const SEED_COLORS: [u32; 4] = [0x4d505f, 0x747678, 0xa3a38f, 0xbdbba1];
const SEED_GLOW: Rgb = rgb(SEED_COLORS[1]);
const EYE: Rgb = rgb(0x303b4d);
const DANGER: Rgb = rgb(0xbd7a85);
const DANGER_DARK: Rgb = rgb(0x724b59);
const SWEAT: Rgb = rgb(0xc2c1b3);
const DUST: Rgb = rgb(0x92a594);

static TURN: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub(super) fn reset_turn() { TURN.store(0, std::sync::atomic::Ordering::Relaxed); }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Notice,
    Skulk,
    Windup,
    Throw,
    Flee,
    Hurt,
}

pub struct State {
    phase: Phase,
    pt: u32,
    len: u32,
    rng: Rng,
    /// Preferred distance from Glint.
    range: f32,
    orbit: f32,
    orbit_t: u32,
    cool: u32,
    throws: u32,
    volley: bool,
    aim: (f32, f32),
    target: (f32, f32),
    flee: (f32, f32),
    flee_cd: u32,
    stuck_t: u32,
    close_t: u32,
    /// Ticks it has been ready to throw without a clear line to Glint.
    blind_t: u32,
    /// Lobbed seeds in the air: landing point, landing tick, flight ticks.
    marks: Vec<Landing>,
    moving: bool,
    started: bool,
    idle_off: u32,
    appearance: u8,
}

/// An active lob and its ground warning, kept together until impact or cancellation.
struct Landing {
    x: f32,
    y: f32,
    tick: u64,
    flight: u32,
    id: u32,
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
    let mut rng = Rng::new((x as u64).wrapping_mul(83_492_791) ^ (y as u64).wrapping_mul(2_654_435_761) ^ 0x1ac);
    let s = State {
        phase: Phase::Notice,
        pt: 0,
        len: 0,
        range: rng.range(72.0, 92.0),
        orbit: if rng.chance(0.5) { 1.0 } else { -1.0 },
        orbit_t: rng.irange(80, 160) as u32,
        cool: rng.irange(30, 70) as u32,
        throws: rng.irange(0, 2) as u32,
        volley: false,
        aim: (0.0, 1.0),
        target: (x, y),
        flee: (0.0, 0.0),
        flee_cd: 0,
        stuck_t: 0,
        close_t: 0,
        blind_t: 0,
        marks: Vec::new(),
        moving: false,
        started: false,
        appearance: super::variation::appearance(x, y),
        idle_off: rng.irange(0, 40) as u32,
        rng,
    };
    (Body::new(x, y, 4.5, HP), s)
}

// ------------------------------------------------------------------------------- behavior

/// Advance behavior, telegraphs, and body physics by one simulation tick.
pub fn update(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx) {
    if !s.started {
        s.started = true;
        let delay = s.rng.irange(1, 21) as u32;
        s.set(Phase::Notice, delay);
    }
    s.pt += 1;
    s.cool = s.cool.saturating_sub(1);
    s.flee_cd = s.flee_cd.saturating_sub(1);
    s.moving = false;
    let (px, py) = ctx.player;
    let d = b.dist_to(px, py);
    let to = b.dir_to(px, py);

    if b.stun > 0 && s.phase != Phase::Hurt {
        s.set(Phase::Hurt, 0);
    }
    let mut retreating = false;

    match s.phase {
        Phase::Notice => {
            stop(b);
            b.face = to;
            if s.pt == s.len {
                b.vz = 2.0; // startled hop
            }
            if s.pt > s.len + 2 && b.z == 0.0 {
                let fx = &mut *ctx.fx;
                fx.particles.dust(&mut fx.rng, b.x, b.y, 3, DUST);
                s.set(Phase::Skulk, 0);
            }
        }
        Phase::Skulk => {
            if ctx.player_alive {
                b.face = to;
            }
            s.orbit_t = s.orbit_t.saturating_sub(1);
            if s.orbit_t == 0 {
                s.orbit = -s.orbit;
                s.orbit_t = s.rng.irange(90, 180) as u32;
            }
            let (want, speed) = if !ctx.player_alive {
                ((0.0, 0.0), 0.0)
            } else {
                let radial = if d < s.range - 12.0 {
                    retreating = true;
                    -1.0
                } else if d > s.range + 12.0 {
                    1.0
                } else {
                    0.0
                };
                let perp = (-to.1 * s.orbit, to.0 * s.orbit);
                let w = norm(to.0 * radial + perp.0 * 0.6, to.1 * radial + perp.1 * 0.6);
                (w, if d < 50.0 { 1.05 } else { 0.72 })
            };
            // smooth, creaturely acceleration
            b.vx += (want.0 * speed - b.vx) * 0.18;
            b.vy += (want.1 * speed - b.vy) * 0.18;
            s.moving = speed > 0.0;
            s.close_t = if d < 30.0 { s.close_t + 1 } else { s.close_t.saturating_sub(2) };

            let can_see = ctx.player_alive && ctx.zone.line_clear(b.x, b.y - 8.0, px, py - 6.0);
            // Every throw needs sight: seeds still collide with solid walls in flight.
            let volley = s.throws % 3 == 2 && d > 64.0;
            let clear = can_see;
            // ready but blocked from view: circle the other way to find an opening
            s.blind_t = if s.cool == 0 && !clear { s.blind_t + 1 } else { 0 };
            if s.blind_t > 30 {
                s.orbit = -s.orbit;
                s.orbit_t = s.rng.irange(90, 180) as u32;
                s.blind_t = 0;
            }
            if s.cool == 0 && clear && d > 34.0 && d < 170.0 && kit::claim(&TURN, ctx.t, TURN_GAP) {
                s.volley = volley;
                s.aim = to;
                s.target = ctx.player;
                s.set(Phase::Windup, if s.volley { WINDUP_VOLLEY } else { WINDUP });
                ctx.fx.sound_vol("enemy_windup", 0.4 * (1.0 - (d - 120.0) / 160.0).clamp(0.0, 1.0));
            } else if s.close_t > 36 {
                start_flee(b, s, ctx, to);
            }
        }
        Phase::Windup => {
            stop(b);
            let lock = s.len - LOCK_BEFORE;
            if s.pt < lock && ctx.player_alive {
                s.aim = to;
                s.target = ctx.player;
            }
            b.face = s.aim;
            if s.pt >= s.len {
                let (_, duration) = throw_timing(b);
                s.set(Phase::Throw, duration);
            }
        }
        Phase::Throw => {
            let (release_tick, _) = throw_timing(b);
            if s.pt == release_tick && ctx.player_alive {
                release(b, s, ctx, d);
            }
            b.vx *= 0.8;
            b.vy *= 0.8;
            if s.pt >= s.len {
                s.throws += 1;
                s.cool = s.rng.irange(100, 160) as u32;
                s.set(Phase::Skulk, 0);
            }
        }
        Phase::Flee => {
            let k = if s.pt < 20 { 1.0 } else { 1.0 - (s.pt - 20) as f32 / 14.0 };
            b.vx = s.flee.0 * 2.2 * k.max(0.2);
            b.vy = s.flee.1 * 2.2 * k.max(0.2);
            b.face = s.flee;
            s.moving = true;
            let fx = &mut *ctx.fx;
            if s.pt % 9 == 1 && b.z == 0.0 {
                b.vz = 1.1; // scampering
                fx.particles.dust(&mut fx.rng, b.x, b.y, 2, DUST);
            }
            if s.pt % 6 == 2 {
                // a drop of panic sweat flung off the mask
                let side = if fx.rng.chance(0.5) { 1.0 } else { -1.0 };
                fx.particles.add(
                    Particle::new(b.x, b.y).z(b.z + 18.0).vel(-s.flee.0 * 0.4 + side * 0.5, -s.flee.1 * 0.3, fx.rng.range(0.9, 1.4)).color(SWEAT).life(fx.rng.range(18.0, 26.0)).grav(0.12).drag(0.96),
                );
            }
            if s.pt >= s.len {
                s.cool = s.cool.min(28);
                s.close_t = 0;
                s.stuck_t = 0;
                s.set(Phase::Skulk, 0);
            }
        }
        Phase::Hurt => {
            stop(b);
            if b.stun == 0 && b.z == 0.0 {
                // skittish: a hurt imp often bolts
                if b.hp <= 2 && s.flee_cd == 0 && s.rng.chance(0.6) {
                    start_flee(b, s, ctx, to);
                } else {
                    s.set(Phase::Skulk, 0);
                }
            }
        }
    }

    let (ox, oy) = (b.x, b.y);
    let blocked = b.integrate(ctx.zone);
    let stalled = blocked && (b.x - ox).abs() + (b.y - oy).abs() < 0.4;
    // Backing into a wall or a ledge: cornered.
    if retreating && stalled {
        s.stuck_t += 1;
        if s.stuck_t > 8 {
            start_flee(b, s, ctx, to);
        }
    } else if s.phase == Phase::Skulk {
        s.stuck_t = s.stuck_t.saturating_sub(1);
        // circling into a rock: go round the other way
        if stalled && s.orbit_t < 150 {
            s.orbit = -s.orbit;
            s.orbit_t = 200;
        }
    }

    // Landing rings for lobbed seeds, drawn on the ground layer under everyone.
    let now = ctx.t;
    s.marks.retain(|m| m.tick > now && ctx.projectiles.iter().any(|p| p.id == m.id && p.alive));
    for m in &s.marks {
        let left = (m.tick - now) as f32;
        let k = 1.0 - left / m.flight as f32;
        ring(ctx, m.x, m.y, k, now);
    }
}

/// Stop voluntary motion while preserving knockback.
fn stop(b: &mut Body) {
    b.vx = 0.0;
    b.vy = 0.0;
}

/// Where the held seed is (world x, ground y, height), read from the seed pixels in the frame.
fn hand(b: &Body, s: &State) -> (f32, f32, f32) {
    let (cell, anchor, flip) = current_cell(b, s, 0);
    if let Some((cx, cy)) = seed_px(cell) {
        let lx = if flip { cell.w as f32 - 1.0 - cx } else { cx };
        let ax = if flip { (cell.w - 1 - anchor.0) as f32 } else { anchor.0 as f32 };
        return (b.x + lx - ax, b.y, anchor.1 as f32 - cy + b.z);
    }
    (b.x + b.face.0 * 3.0, b.y, HAND_Z + 6.0 + b.z)
}

/// Centroid of the seed-colored pixels in a cell, if any.
fn seed_px(cell: &Cell) -> Option<(f32, f32)> {
    let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
    for y in 0..cell.h {
        for x in 0..cell.w {
            let p = cell.at(x, y);
            if p.a && SEED_COLORS[2..].contains(&p.c.key()) {
                sx += x as f32;
                sy += y as f32;
                n += 1.0;
            }
        }
    }
    (n > 0.0).then(|| (sx / n, sy / n))
}

/// Read the release tick and follow-through length from the directional throw animation.
fn throw_timing(b: &Body) -> (u32, u32) {
    if let Some(sh) = sheet(SHEET)
        && let Some((a, _)) = kit::find(sh, &["throw", "attack"], Dir::from_vec(b.face.0, b.face.1))
    {
        let frame = a.json.get("release_frame").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let tick = a.durations.iter().take(frame.min(a.frames - 1)).sum::<u32>().max(1);
        return (tick, a.total_ticks().max(tick + 1));
    }
    (3, THROW)
}

/// Release position authored by the artist, mirrored for left-facing throws.
fn release_hand(b: &Body, s: &State) -> (f32, f32, f32) {
    if let Some(sh) = sheet(SHEET)
        && let Some((a, flip)) = kit::find(sh, &["throw", "attack"], Dir::from_vec(b.face.0, b.face.1))
        && let Some((x, y)) = a.point("release_offset", a.frame_at(s.pt, false))
    {
        return (b.x + if flip { -x } else { x } as f32, b.y, -y as f32 + b.z);
    }
    hand(b, s)
}

/// Let go of the seed(s).
fn release(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx, d: f32) {
    let (hx, hy, hz) = release_hand(b, s);
    let fx = &mut *ctx.fx;
    fx.sound_vol("seed_throw", (1.0 - (d - 120.0) / 160.0).clamp(0.4, 1.0));
    b.kx -= s.aim.0 * 0.9;
    b.ky -= s.aim.1 * 0.9;
    if !s.volley {
        // Preserve the drawn release position while flying at the player's chest height.
        let z = 7.0;
        let y = hy - hz + z;
        let aim = norm(s.target.0 - hx, s.target.1 - 2.0 - y);
        let mut p = Projectile::new(hx, y, aim.0 * SEED_SPEED, aim.1 * SEED_SPEED, fx).color(SEED_GLOW).z(z, 0.0, 0.0);
        dress_seed(&mut p, false);
        p.life = 150;
        ctx.projectiles.push(p);
        return;
    }
    // Volley: one seed on Glint, one to each side, landing together.
    let (px, py) = s.target;
    let flight = (d / 1.8).clamp(34.0, 54.0).round() as u32;
    let perp = (-s.aim.1, s.aim.0);
    for off in [-20.0f32, 0.0, 20.0] {
        let (tx, ty) = (px + perp.0 * off, py + perp.1 * off * 0.8);
        let n = flight as f32;
        let vz = (LOB_GRAV * n * (n + 1.0) * 0.5 - hz) / n;
        let mut p = Projectile::new(hx, hy, (tx - hx) / n, (ty - hy) / n, fx).color(SEED_GLOW).z(hz, vz, LOB_GRAV);
        dress_seed(&mut p, true);
        p.r = 3.0;
        p.life = flight + 10;
        ctx.projectiles.push(p);
        s.marks.push(Landing { x: tx, y: ty, tick: ctx.t + flight as u64 - 1, flight, id: ctx.projectiles.last().unwrap().id });
    }
}

/// Seed look: the artist's `seed` sheet if it exists (else the engine's glowing dot).
fn dress_seed(p: &mut Projectile, _lobbed: bool) {
    p.light = 0.0;
    p.trail = false;
    if let Some(sh) = sheet(SEED_SHEET) {
        let name = ["spin", "fly", "idle", "seed"].into_iter().find(|n| sh.has(n)).or_else(|| sh.anims.keys().next().map(|k| k.as_str()));
        if let Some(n) = name
            && let Some(a) = sh.anims.get_key_value(n)
        {
            p.sheet = SEED_SHEET;
            p.anim = a.0.as_str();
        }
    }
}

/// A clean 1 px ring of ground pixels where a lobbed seed will land. `k` goes 0 -> 1 over the
/// flight: the ring starts wide and dim, tightens onto the real hit radius and brightens.
fn ring(ctx: &mut EnemyCtx, x: f32, y: f32, k: f32, now: u64) {
    let hit_r = 3.0 + 6.0; // projectile r + the engine's landing splash
    let r = (hit_r + (1.0 - k) * 4.0).round() as i32;
    let last = k > 0.85;
    // a quick double-blink right before impact
    let blink = last && (now / 3) % 2 == 0;
    let c = if blink { rgb(0xe6b4ac) } else { DANGER_DARK.lerp(DANGER, k) };
    // Keep the warning flat and bright: emissive debris is darkened by the shared renderer.
    let fx = &mut *ctx.fx;
    let (cx, cy) = (x.round() as i32, y.round() as i32);
    // midpoint circle: an even, hand-drawn-looking pixel ring
    let (mut dx, mut dy, mut err) = (r, 0, 1 - r);
    while dx >= dy {
        for (px, py) in [(dx, dy), (dy, dx), (-dy, dx), (-dx, dy), (-dx, -dy), (-dy, -dx), (dy, -dx), (dx, -dy)] {
            kit::ground_px(fx, cx + px, cy + py, c, 0.0);
        }
        dy += 1;
        if err < 0 {
            err += 2 * dy + 1;
        } else {
            dx -= 1;
            err += 2 * (dy - dx) + 1;
        }
    }
    kit::ground_px(fx, cx, cy, c, 0.0);
}

/// Pick the clearest sideways escape route and limit repeated panic escapes.
fn start_flee(b: &Body, s: &mut State, ctx: &mut EnemyCtx, to: (f32, f32)) {
    if s.flee_cd > 0 {
        // Cornered again too soon: stand and throw point blank instead of running forever.
        if s.cool > 20 {
            s.cool = 20;
        }
        s.close_t = 0;
        s.stuck_t = 0;
        return;
    }
    let perp = (-to.1, to.0);
    // Sideways and away first; scurrying past Glint only if nothing else is open.
    let mut cands: Vec<(f32, f32)> = Vec::new();
    for (pk, ak) in [(1.0, -0.5), (-1.0, -0.5), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0), (1.0, 0.45), (-1.0, 0.45)] {
        cands.push(norm(perp.0 * pk + to.0 * ak, perp.1 * pk + to.1 * ak));
    }
    let free = |dir: (f32, f32)| -> u32 {
        let mut n = 0;
        for i in 1..=6 {
            let k = i as f32 * 8.0;
            if ctx.zone.blocked(b.x + dir.0 * k, b.y + dir.1 * k, b.r) {
                break;
            }
            n += 1;
        }
        n
    };
    let mut best = cands[0];
    let mut best_n = 0;
    for (i, &c) in cands.iter().enumerate() {
        // small random preference so two imps don't mirror each other
        let n = free(c) * 2 + if i < 2 && s.rng.chance(0.5) { 1 } else { 0 };
        if n > best_n {
            best_n = n;
            best = c;
        }
    }
    s.flee = best;
    s.flee_cd = 150;
    s.close_t = 0;
    s.stuck_t = 0;
    s.set(Phase::Flee, FLEE);
    let fx = &mut *ctx.fx;
    fx.particles.dust(&mut fx.rng, b.x, b.y, 4, DUST);
}

// ------------------------------------------------------------------------------- reactions

/// Interrupt the current action and shed a few pixels along the hit direction.
pub fn on_hit(b: &mut Body, s: &mut State, hit: &Hit, ctx: &mut EnemyCtx) {
    let was_winding = s.phase == Phase::Windup;
    s.set(Phase::Hurt, 0);
    s.cool = s.cool.max(50);
    if b.z < 2.0 {
        b.vz = if hit.heavy { 1.8 } else { 1.0 };
    }
    let (dx, dy) = hit.push_dir(b.x, b.y);
    let fx = &mut *ctx.fx;
    // bramble flecks and a thorn snap off
    for i in 0..4 {
        let c = if i == 0 { rgb(PAINT) } else { rgb(BRAMBLE[fx.rng.irange(2, 4) as usize]) };
        let sp = fx.rng.range(0.8, 1.9);
        fx.particles.add(
            Particle::new(b.x, b.y)
                .z(fx.rng.range(8.0, 16.0))
                .vel(dx * sp + fx.rng.signed() * 0.5, dy * sp * 0.7 + fx.rng.signed() * 0.4, fx.rng.range(0.6, 1.6))
                .color(c)
                .life(fx.rng.range(90.0, 160.0))
                .grav(0.14)
                .bounce(0.25)
                .settle(true),
        );
    }
    if was_winding {
        drop_seed(b, ctx.fx);
    }
}

/// An interrupted throw: the seed tumbles out of its hand.
fn drop_seed(b: &Body, fx: &mut crate::fx::Fx) {
    for (i, c) in [SEED_COLORS[3], SEED_COLORS[2], SEED_COLORS[1]].into_iter().enumerate() {
        fx.particles.add(
            Particle::new(b.x + b.face.0 * 3.0, b.y + i as f32 * 0.3)
                .z(HAND_Z + 4.0 - i as f32)
                .vel(-b.face.0 * 0.5, 0.0, 1.2)
                .color(rgb(c))
                .life(200.0)
                .grav(0.16)
                .drag(0.97)
                .bounce(0.4)
                .settle(true),
        );
    }
}

/// Break the body into settling pixels while preserving its signature piece.
pub fn on_death(b: &Body, s: &State, ctx: &mut EnemyCtx) {
    // A dead imp no longer updates its landing warnings, so remove those same lobbed seeds.
    // Removal matters: the shared projectile loop also updates entries marked dead this tick.
    ctx.projectiles.retain(|p| !s.marks.iter().any(|m| m.id == p.id));
    let (cell, anchor, flip) = current_cell(b, s, 0);
    let dir = norm(b.kx, b.ky);
    let fx = &mut *ctx.fx;
    let is_mask = |c: &Cell, x: i32, y: i32| mask_px(c, x, y);
    let pop = (dir.0 * 0.9 + fx.rng.signed() * 0.3, dir.1 * 0.6 + fx.rng.signed() * 0.2, 3.0);
    let before = fx.particles.list.len();
    let o = kit::Burst { power: 1.0, life: 420.0, dir, outline: (OUTLINE, rgb(BRAMBLE[0])), chunk: &is_mask, chunk_v: pop, chunk_life: 600.0 };
    kit::burst(fx, cell, anchor, flip, b.x, b.y, b.z, &o);
    // the light goes out of the mask's eyes once it's off
    for p in fx.particles.list[before..].iter_mut() {
        if p.max_life == 600.0 && p.glow > 0.0 {
            p.glow = 0.0;
            p.c0 = OUTLINE;
            p.c1 = OUTLINE;
        }
    }
    fx.sound_vol("grass_cut", 0.4);
}

/// Mask pixels (bone, paint touching bone, eyes, and outline that hugs only the mask).
fn mask_px(c: &Cell, x: i32, y: i32) -> bool {
    let p = c.at(x, y);
    if !p.a {
        return false;
    }
    let k = p.c.key();
    let near_mask = |x: i32, y: i32| [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| MASK[..4].contains(&c.at(x + dx, y + dy).c.key()) && c.at(x + dx, y + dy).a);
    if MASK.contains(&k) {
        return true;
    }
    if k == PAINT {
        return near_mask(x, y);
    }
    kit::part_of(c, x, y, &MASK)
}

// ------------------------------------------------------------------------------- drawing

/// Resolve the sheet pose, anchor, and mirror for the current behavior.
fn current_cell(b: &Body, s: &State, t: u64) -> (&'static Cell, (i32, i32), bool) {
    let dir = Dir::from_vec(b.face.0, b.face.1);
    if let Some(sh) = sheet(SHEET) {
        let progress = |name: &str, pt: u32, len: u32| -> usize {
            let n = sh.anim(&format!("{name}_right")).or_else(|| sh.anim(name)).map(|a| a.frames).unwrap_or(1);
            ((pt as f32 / len.max(1) as f32 * n as f32) as usize).min(n - 1)
        };
        let (names, sel): (&[&str], Sel) = match s.phase {
            Phase::Notice => (&["idle"], Sel::Loop(t as u32 + s.idle_off)),
            Phase::Skulk => {
                if s.moving && (b.vx.abs() + b.vy.abs()) > 0.25 {
                    (&["move", "walk", "run", "idle"], Sel::Loop(t as u32))
                } else {
                    (&["idle"], Sel::Loop(t as u32 + s.idle_off))
                }
            }
            Phase::Windup => (&["windup", "throw", "attack", "idle"], Sel::Fixed(progress("windup", s.pt, s.len))),
            Phase::Throw => (&["throw", "attack", "windup", "idle"], Sel::Once(s.pt)),
            Phase::Flee => (&["flee", "panic", "run", "move", "idle"], Sel::Loop(s.pt * 2)),
            Phase::Hurt => (&["hurt", "idle"], Sel::Once(s.pt)),
        };
        if let Some((a, flip)) = kit::find(sh, names, dir) {
            let i = match sel {
                Sel::Loop(k) => a.frame_at(k, true),
                Sel::Once(k) => a.frame_at(k, false),
                Sel::Fixed(i) => i.min(a.frames - 1),
            };
            return (super::variation::cell(SHEET, sh.cell(a, i), s.appearance), sh.anchor_of(a), flip);
        }
    }
    let c = if matches!(s.phase, Phase::Windup) { fallback_cell(true) } else { fallback_cell(false) };
    (c, (c.w / 2, c.h - 1), dir == Dir::Left)
}

enum Sel {
    Loop(u32),
    Once(u32),
    Fixed(usize),
}

/// Draw the directional pose and its restrained combat cues.
pub fn draw(b: &Body, s: &State, f: &mut Frame) {
    let (cell, anchor, flip) = current_cell(b, s, f.t);
    let zk = (b.z / 8.0).min(1.0);
    f.shadow(f.sx(b.x), f.sy(b.y), 5 - (zk * 2.0) as i32, 2, 0.42 - 0.12 * zk);

    let mut ox = 0.0;
    if s.phase == Phase::Hurt && s.pt < 8 {
        ox = if s.pt % 2 == 0 { 1.0 } else { -1.0 };
    }
    let o = DrawOpts { actor: true, foot_depth: 3, foot_half_w: b.r, ground_y: Some(b.y.floor()), elevation: b.z, flip, flash: b.flash, ..Default::default() };
    f.sprite_w(cell, anchor, b.x + ox, b.y - b.z, &o);

    if s.phase == Phase::Windup {
        let (hx, hy, hz) = hand(b, s);
        let (sx, sy) = (f.sx(hx), f.sy(hy - hz));
        // the code fallback has no seed drawn in its hands: draw one
        if sheet(SHEET).is_none() {
            draw_held_seed(f, sx, sy);
        }
        // Aim lock is a short matte seam on the held husk, with no halo or sparkle.
        let lock = s.len - LOCK_BEFORE;
        if (lock..lock + 5).contains(&s.pt) {
            f.put(sx, sy, DANGER);
            f.put(sx, sy + 1, rgb(0xdfdac1));
        }
    }
}

/// Draw a held seed only for the sheetless fallback creature.
fn draw_held_seed(f: &mut Frame, sx: i32, sy: i32) {
    if let Some(sh) = sheet(SEED_SHEET)
        && let Some(a) = ["idle", "spin", "fly"].into_iter().find_map(|n| sh.anim(n))
    {
        f.sprite(sh.cell(a, 0), sh.anchor_of(a), sx, sy, &DrawOpts::default());
        return;
    }
    let pal = [rgb(SEED_COLORS[3]), rgb(SEED_COLORS[2]), rgb(SEED_COLORS[1])];
    f.put(sx, sy - 1, pal[0]);
    f.put(sx - 1, sy, pal[1]);
    f.put(sx, sy, pal[0]);
    f.put(sx + 1, sy, pal[1]);
    f.put(sx, sy + 1, pal[2]);
}

/// Code-drawn stand-in used only if the sheet is missing (hand-placed, the imp's palette).
fn fallback_cell(arm_up: bool) -> &'static Cell {
    static A: OnceLock<Cell> = OnceLock::new();
    static B: OnceLock<Cell> = OnceLock::new();
    let pal = [
        ('1', rgb(BRAMBLE[0])),
        ('2', rgb(BRAMBLE[1])),
        ('3', rgb(BRAMBLE[2])),
        ('4', rgb(BRAMBLE[3])),
        ('t', rgb(0x655f78)),
        ('u', rgb(PAINT)),
        ('v', rgb(0xbf9da9)),
        ('M', rgb(MASK[0])),
        ('N', rgb(MASK[1])),
        ('O', rgb(MASK[2])),
        ('Q', rgb(MASK[3])),
        ('Y', EYE),
        ('Z', EYE),
    ];
    let build = |up: bool| {
        let mut rows = vec![
            "....v...v....",
            "...tu..tu....",
            "....4334.....",
            "...2443443...",
            "..23MNOOQQ...",
            "..2MNOZOOZ...",
            "..2MNOYOOY...",
            "..23MNOuOO...",
            "...2MMNNN....",
            "...23443.....",
            "..234443u....",
            ".u23444324...",
            "..v2343..4...",
            "...2333......",
            "...2..3......",
            "..22..33.....",
        ];
        if up {
            rows[9] = "...234434...";
            rows[10] = "..234443.4..";
            rows[11] = ".u234443....";
            rows[12] = "..v2343.....";
        }
        kit::outlined_emit(&rows, &pal, &[], OUTLINE)
    };
    if arm_up { A.get_or_init(|| build(true)) } else { B.get_or_init(|| build(false)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throw releases exactly on the authored release frame, after the full windup.
    #[test]
    fn seed_follows_release_pose() {
        let zone = crate::world::Zone::new(crate::world::ZoneId::Bloomwild, 30, 20);
        let (mut b, mut s) = spawn(120.0, 120.0);
        let mut fx = crate::fx::Fx::new(7);
        let mut hits = Vec::new();
        let mut projectiles = Vec::new();
        let mut ctx = EnemyCtx { player: (200.0, 120.0), player_alive: true, zone: &zone, fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 100 };
        s.started = true;
        s.target = ctx.player;
        s.aim = (1.0, 0.0);
        b.face = s.aim;
        s.set(Phase::Windup, WINDUP);
        s.pt = WINDUP - 1;
        update(&mut b, &mut s, &mut ctx);
        assert_eq!(s.phase, Phase::Throw);
        assert!(ctx.projectiles.is_empty());
        let (release_tick, duration) = throw_timing(&b);
        for _ in 1..release_tick {
            update(&mut b, &mut s, &mut ctx);
            assert!(ctx.projectiles.is_empty());
        }
        update(&mut b, &mut s, &mut ctx);
        assert_eq!(ctx.projectiles.len(), 1);
        let p = &ctx.projectiles[0];
        assert!(p.vx > 0.0);
        let release_y = sheet(SHEET).and_then(|sh| sh.dir_anim("attack", Dir::Right)).and_then(|(a, _)| a.point("release_offset", 1)).map(|(_, y)| y as f32).unwrap_or(-18.0);
        assert!((p.y - p.z - (b.y + release_y)).abs() < 1.0);
        for _ in release_tick + 1..duration {
            update(&mut b, &mut s, &mut ctx);
        }
        assert_eq!(ctx.projectiles.len(), 1, "one seed per throw");
    }

    /// Killing an imp removes its lobbed hazards without touching another imp's seed.
    #[test]
    fn death_cancels_its_marked_volley() {
        let zone = crate::world::Zone::new(crate::world::ZoneId::Bloomwild, 30, 20);
        let (mut b, mut s) = spawn(120.0, 120.0);
        let mut fx = crate::fx::Fx::new(7);
        let mut hits = Vec::new();
        let mut projectiles = vec![Projectile::new(220.0, 120.0, -2.0, 0.0, &mut fx)];
        let other = projectiles[0].id;
        let mut ctx = EnemyCtx { player: (200.0, 120.0), player_alive: true, zone: &zone, fx: &mut fx, hits: &mut hits, projectiles: &mut projectiles, t: 100 };
        s.volley = true;
        s.target = ctx.player;
        s.aim = (1.0, 0.0);
        release(&mut b, &mut s, &mut ctx, 80.0);
        assert_eq!(s.marks.len(), 3);
        assert_eq!(ctx.projectiles.len(), 4);
        on_death(&b, &s, &mut ctx);
        assert_eq!(ctx.projectiles.len(), 1);
        assert_eq!(ctx.projectiles[0].id, other);
        assert!(ctx.fx.particles.list.len() > 20, "death must produce sprite debris");
    }
}
