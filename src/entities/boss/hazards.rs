//! The Warden's hazards: shockwave rings, prism beams, falling shards and rocks, and the
//! crystal pylons of phase 2. Plus the ground telegraph drawing they share.
//!
//! Fairness rules used everywhere here:
//! - Every hazard shows exactly where it hurts before it hurts (a ground marker, an aim line).
//! - Ground hazards hurt where their ground marking is; the beam hurts where it visibly covers
//!   Glint's sprite. Collision is computed here against the player and handed to the scene as a
//!   tiny hit circle on the player, so the drawn shape and the hit shape are the same shape.
//! - Each hazard hits at most once (one hit id).

use crate::combat::{Hit, Shape, Team};
use crate::entities::enemies::EnemyCtx;
use crate::fx::{Debris, Fx, Particle};
use crate::gfx::{Frame, Rgb, rgb};
use crate::util::{angle_diff, bayer4, hash2};
use std::f32::consts::{PI, TAU};

// Prism warnings retain the dark/mid/locked contrast steps in the world's iris ramp.
pub const D0: Rgb = warning_tone(P0, rgb(0x724b59));
pub const D1: Rgb = D0;
pub const D2: Rgb = warning_tone(P1, rgb(0xbd7a85));
pub const D3: Rgb = warning_tone(PW, rgb(0xe6b4ac));
pub const D4: Rgb = D3;
// One iris prism family, shared by the heart and its attacks.
pub const P0: Rgb = rgb(0x564d6e);
pub const P1: Rgb = rgb(0x8d7fa8);
pub const P2: Rgb = rgb(0xb7add0);
pub const PW: Rgb = rgb(0xe2d8e9);
// v2 summit floor: dust, rubble and ground scars are unlit stone.
pub const F0: Rgb = rgb(0x1f2838);
pub const F1: Rgb = rgb(0x2b3548);
pub const F2: Rgb = rgb(0x46526a);
pub const F3: Rgb = rgb(0x64738a);
pub const F4: Rgb = rgb(0x94a1ae);
pub const F5: Rgb = rgb(0xc1c8c5);
// v2 Warden stone ramp.
pub const S2: Rgb = rgb(0x555d70);
pub const S3: Rgb = rgb(0x7f8797);
pub const S4: Rgb = rgb(0xb0b7bb);

/// Change warning hue while retaining the preceding danger ramp's exact albedo luminance.
const fn warning_tone(c: Rgb, reference: Rgb) -> Rgb {
    let k = (reference.r * 0.299 + reference.g * 0.587 + reference.b * 0.114)
        / (c.r * 0.299 + c.g * 0.587 + c.b * 0.114);
    Rgb::new(c.r * k, c.g * k, c.b * k)
}

/// Glow factor that makes a one-frame decal's emissive come out at exactly its color
/// (see `Particles::draw_debris` at life/max_life = 0.5).
const DECAL_GLOW: f32 = 2.28;

/// A ground pixel for this frame only: drawn on the ground under every character and prop
/// (through the debris layer). Push every tick while it should show.
#[inline]
pub fn decal(fx: &mut Fx, x: i32, y: i32, c: Rgb) {
    fx.particles.debris.push(Debris {
        x,
        y,
        c,
        glow: DECAL_GLOW,
        life: 2,
        max_life: 2,
    });
}

/// A dull (unlit-by-itself) ground pixel for this frame only.
#[inline]
pub fn decal_dull(fx: &mut Fx, x: i32, y: i32, c: Rgb) {
    fx.particles.debris.push(Debris {
        x,
        y,
        c,
        glow: 0.0,
        life: 2,
        max_life: 2,
    });
}

/// Seat a bright warning in a one-pixel stone fissure without moving its danger boundary.
pub fn prism_crack(fx: &mut Fx, x: i32, y: i32, c: Rgb) {
    decal_dull(fx, x, y + 1, F0);
    decal(fx, x, y, c);
}

/// A lasting ground mark (cracks, scorch) that fades out with dithering.
#[inline]
pub fn scar(fx: &mut Fx, x: i32, y: i32, c: Rgb, life: u32) {
    fx.particles.debris.push(Debris {
        x,
        y,
        c,
        glow: 0.0,
        life,
        max_life: life,
    });
}

/// Midpoint circle outline points (radius r, integer).
pub fn circle_pts(r: i32, mut f: impl FnMut(i32, i32)) {
    if r <= 0 {
        f(0, 0);
        return;
    }
    let (mut x, mut y, mut err) = (r, 0, 1 - r);
    while x >= y {
        for (a, b) in [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ] {
            f(a, b);
        }
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

/// A target marker on the ground: a crisp ring with four inward notches. `k` 0..1 is how close
/// the strike is (the notches close in); `locked` turns it white-hot then solid.
pub fn marker(fx: &mut Fx, x: f32, y: f32, r: f32, k: f32, locked: u32, t: u32) {
    let (cx, cy) = (x.round() as i32, y.round() as i32);
    let ri = r.round() as i32;
    let ring = if locked > 0 && locked < 6 {
        D4
    } else if locked > 0 {
        D2
    } else {
        D1.lerp(D2, 0.5 + 0.5 * k)
    };
    // dashed while tracking (rotating dashes read as "aiming"), solid when locked
    let mut pts = Vec::new();
    circle_pts(ri, |a, b| pts.push((a, b)));
    for (a, b) in pts {
        if locked == 0 {
            let ang = (b as f32).atan2(a as f32) + t as f32 * 0.05;
            if (ang * 8.0 / TAU).rem_euclid(1.0) < 0.38 {
                continue;
            }
        }
        prism_crack(fx, cx + a, cy + b, ring);
    }
    // Tracking notches close in; the solid locked ring carries the final cue by itself.
    if locked == 0 {
        let gap = 3 + ((1.0 - k) * (r - 6.0)).max(0.0).round() as i32;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for i in 0..3 {
                let d = gap + i;
                if d < ri {
                    prism_crack(fx, cx + dx * d, cy + dy * d, D3);
                }
            }
        }
    }

}

/// Hand the scene a hit that lands on the player (collision already decided here).
pub fn hit_player(ctx: &mut EnemyCtx, id: u32, dir: (f32, f32), knock: f32) {
    let (px, py) = ctx.player;
    ctx.hits.push(
        Hit::new(
            Shape::Circle {
                x: px,
                y: py - 2.0,
                r: 1.0,
            },
            Team::Enemy,
            1,
            id,
        )
        .knock(knock)
        .dir(dir.0, dir.1)
        .heavy(true),
    );
}

/// Dust and stone chips kicked up from the ground.
pub fn rubble(fx: &mut Fx, x: f32, y: f32, n: usize, power: f32) {
    for _ in 0..n {
        let (dx, dy) = fx.rng.dir();
        let sp = fx.rng.range(0.4, 1.0) * power;
        let c = *fx.rng.pick(&[F2, F3, F3, F4, S3]);
        fx.particles.add(
            Particle::new(x + dx * 3.0, y + dy * 2.0)
                .z(fx.rng.range(0.0, 3.0))
                .vel(dx * sp, dy * sp * 0.7, fx.rng.range(1.0, 3.0) * power * 0.6)
                .color(c)
                .life(fx.rng.range(60.0, 160.0))
                .grav(0.2)
                .drag(0.96)
                .bounce(0.35)
                .settle(true),
        );
    }
}

// ------------------------------------------------------------------ shockwave ring

#[derive(Clone, Debug)]
pub struct Ring {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub speed: f32,
    pub max_r: f32,
    pub id: u32,
    /// Roars push a harmless gust ring (white, no hit).
    pub harmless: bool,
    pub t: u32,
}

impl Ring {
    /// Create a telegraphed hazard at its world anchor.
    pub fn new(x: f32, y: f32, r: f32, speed: f32, max_r: f32, id: u32, harmless: bool) -> Ring {
        Ring {
            x,
            y,
            r,
            speed,
            max_r,
            id,
            harmless,
            t: 0,
        }
    }

    /// Returns false when done.
    pub fn update(&mut self, ctx: &mut EnemyCtx) -> bool {
        self.t += 1;
        self.r += self.speed;
        if self.r > self.max_r {
            return false;
        }
        let (px, py) = ctx.player;
        let d = ((px - self.x).powi(2) + (py - self.y).powi(2)).sqrt();
        // the band that hurts is the bright part of the drawn ring: r-4 ..= r
        if !self.harmless && ctx.player_alive && d > self.r - 4.0 - 3.0 && d < self.r + 3.0 {
            let dir = if d > 0.1 {
                ((px - self.x) / d, (py - self.y) / d)
            } else {
                (0.0, 1.0)
            };
            hit_player(ctx, self.id, dir, 3.6);
        }
        // kick up dust along the front
        let fx = &mut *ctx.fx;
        let n = if self.harmless { 3 } else { 5 };
        for _ in 0..n {
            let a = fx.rng.f32() * TAU;
            let (x, y) = (self.x + a.cos() * self.r, self.y + a.sin() * self.r);
            if ctx.zone.solid_tile_at(x, y) {
                continue;
            }
            let c = if self.harmless {
                *fx.rng.pick(&[F4, F5, F3])
            } else {
                *fx.rng.pick(&[F3, F4, S3])
            };
            fx.particles.add(
                Particle::new(x, y)
                    .z(0.5)
                    .vel(
                        a.cos() * self.speed * 0.6,
                        a.sin() * self.speed * 0.6,
                        fx.rng.range(0.6, 1.8),
                    )
                    .color(c)
                    .life(fx.rng.range(16.0, 30.0))
                    .grav(0.12)
                    .drag(0.9),
            );
        }
        self.decal(fx);
        true
    }

    /// Rasterize the expanding shockwave as a crisp ground band.
    fn decal(&self, fx: &mut Fx) {
        let k = self.r / self.max_r;
        // thins and dithers away over the last stretch
        let fade = ((1.0 - k) / 0.18).clamp(0.0, 1.0);
        let r = self.r;
        let width = if self.harmless { 3.0 } else { 6.0 };
        let r0 = (r - width).max(0.0);
        let (cx, cy) = (self.x.round() as i32, self.y.round() as i32);
        let ri = r.ceil() as i32 + 1;
        for dy in -ri..=ri {
            let fy = dy as f32;
            let outer2 = r * r - fy * fy;
            if outer2 < 0.0 {
                continue;
            }
            let xo = outer2.sqrt();
            let inner2 = r0 * r0 - fy * fy;
            let xi = if inner2 > 0.0 { inner2.sqrt() } else { 0.0 };
            let (a, b) = (xi.floor() as i32, xo.round() as i32);
            for dxa in a..=b {
                for s in [-1, 1] {
                    let dx = dxa * s;
                    if dxa == 0 && s == 1 {
                        continue;
                    }
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    let from_edge = r - d; // 0 at the leading edge
                    if !(-0.5..=width).contains(&from_edge) {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if fade < 1.0 && fade <= bayer4(x, y) {
                        continue;
                    }
                    let c = if self.harmless {
                        if from_edge < 1.0 {
                            F5
                        } else if bayer4(x, y) < 0.5 {
                            F4
                        } else {
                            continue;
                        }
                    } else if from_edge < 1.0 {
                        D4
                    } else if from_edge < 3.0 {
                        D2
                    } else if from_edge < 4.5 {
                        D1
                    } else if bayer4(x, y) < 0.5 {
                        D0
                    } else {
                        continue;
                    };
                    if self.harmless {
                        decal_dull(fx, x, y, c);
                    } else if from_edge < 1.0 {
                        prism_crack(fx, x, y, c);
                    } else {
                        decal(fx, x, y, c);
                    }
                }
            }
        }
    }

    /// Lights along the ring (called from draw).
    pub fn light(&self, f: &mut Frame) {
        if self.harmless {
            return;
        }
        let k = (1.0 - self.r / self.max_r).clamp(0.0, 1.0);
        let n = 8;
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            f.add_light_w(
                self.x + a.cos() * self.r,
                self.y + a.sin() * self.r,
                12.0,
                D2,
                0.15 * k,
            );
        }
    }
}

// ------------------------------------------------------------------ prism beam

#[derive(Clone, Debug)]
pub struct Beam {
    /// Source in world draw coordinates (height already applied).
    pub ox: f32,
    pub oy: f32,
    pub a: f32,
    pub a0: f32,
    pub a1: f32,
    pub t: u32,
    pub charge: u32,
    pub sweep: u32,
    /// Sweep there and back again.
    pub back: bool,
    pub id: u32,
    /// Which shoulder (0 left, 1 right).
    pub side: usize,
}

pub const BEAM_LEN: f32 = 520.0;
const BEAM_FADE: u32 = 14;

impl Beam {
    /// Return the charge, sweep and fade duration.
    pub fn total(&self) -> u32 {
        self.charge + self.sweep * if self.back { 2 } else { 1 } + BEAM_FADE
    }
    /// Report whether the visible beam currently deals damage.
    pub fn firing(&self) -> bool {
        self.t >= self.charge && self.t < self.charge + self.sweep * if self.back { 2 } else { 1 }
    }
    /// Report whether the beam is still telegraphing its sweep.
    pub fn charging(&self) -> bool {
        self.t < self.charge
    }
    /// Beam half-width this tick (0 = not drawn).
    pub fn width(&self) -> f32 {
        let fire_end = self.charge + self.sweep * if self.back { 2 } else { 1 };
        if self.t < self.charge {
            0.0
        } else if self.t < self.charge + 3 {
            // the first frames punch out wide
            5.0
        } else if self.t < fire_end {
            3.0 + if (self.t / 3) % 2 == 0 { 0.0 } else { 0.5 }
        } else {
            3.0 * (1.0 - (self.t - fire_end) as f32 / BEAM_FADE as f32)
        }
    }

    /// Advance the hazard and emit hits only during its visible danger window.
    pub fn update(&mut self, ctx: &mut EnemyCtx, ox: f32, oy: f32) -> bool {
        self.t += 1;
        self.ox = ox;
        self.oy = oy;
        if self.t < self.charge {
            self.a = self.a0;
            // charge: prism light gathers into the crystal
            let fx = &mut *ctx.fx;
            if self.t % 2 == 0 {
                let a = fx.rng.f32() * TAU;
                let r = fx.rng.range(18.0, 34.0);
                let (sx, sy) = (ox + a.cos() * r, oy + a.sin() * r);
                let c = *fx.rng.pick(&[P2, P1, P2, PW]);
                fx.particles.add(
                    Particle::new(sx, sy + 60.0)
                        .z(60.0)
                        .vel((ox - sx) * 0.09, (oy - sy) * 0.09, 0.0)
                        .color(c)
                        .fade_to(PW)
                        .glow(1.0)
                        .grav(0.0)
                        .drag(0.97)
                        .life(11.0),
                );
            }
            if self.t == self.charge - 1 {
                fx.sound("beam_fire");
                fx.shake(0.4);
                fx.flash(0.12);
            }
            return true;
        }
        let s = self.t - self.charge;
        let (from, to, k) = if s < self.sweep {
            (self.a0, self.a1, s as f32 / self.sweep as f32)
        } else if self.back && s < self.sweep * 2 {
            (
                self.a1,
                self.a0,
                (s - self.sweep) as f32 / self.sweep as f32,
            )
        } else {
            let end = if self.back { self.a0 } else { self.a1 };
            (end, end, 1.0)
        };
        // ease the ends so the sweep starts and stops with weight
        let e = k * k * (3.0 - 2.0 * k);
        let k2 = 0.25 * k + 0.75 * e;
        self.a = from + angle_diff(from, to) * k2;
        let fx = &mut *ctx.fx;
        if self.firing() {
            fx.shake(0.04);
            if s % 55 == 40 {
                fx.sound_vol("beam_loop", 0.7);
            }
            // sparks shed along the beam
            let (dx, dy) = (self.a.cos(), self.a.sin());
            for _ in 0..3 {
                let along = fx.rng.range(20.0, 300.0);
                let (x, y) = (ox + dx * along, oy + dy * along);
                let c = *fx.rng.pick(&[P2, P1, P2, PW]);
                let side = fx.rng.signed();
                fx.particles.add(
                    Particle::new(x, y + 10.0)
                        .z(10.0)
                        .vel(
                            -dy * side * 1.2 + fx.rng.signed() * 0.3,
                            dx * side * 1.2,
                            fx.rng.range(-0.3, 0.6),
                        )
                        .color(PW)
                        .fade_to(c)
                        .glow(1.0)
                        .grav(0.05)
                        .drag(0.88)
                        .life(fx.rng.range(8.0, 16.0)),
                );
            }
            // A brief, irregular contact chip, rather than persistent dotted sweep rings.
            if s % 24 == 0 {
                let along = 48.0 + hash2(s as i32, self.id as i32, 917) * 238.0;
                let x = (ox + dx * along).round() as i32;
                let y = (oy + dy * along).round() as i32 + 8;
                scar(fx, x, y, F0, 30);
                scar(fx, x + 1, y, F1, 30);
            }
            // where Glint is: test the beam against the sprite (feet to head)
            if ctx.player_alive {
                let (px, py) = ctx.player;
                let w = self.width();
                let mut hit = false;
                for hy in [3.0, 8.0, 13.0] {
                    let (qx, qy) = (px - ox, py - hy - oy);
                    let along = qx * dx + qy * dy;
                    let perp = (qx * -dy + qy * dx).abs();
                    if along > 0.0 && along < BEAM_LEN && perp < w + 2.5 {
                        hit = true;
                    }
                }
                if hit {
                    // knock out of the beam, sideways along the sweep direction
                    let sweep_dir = angle_diff(from, to).signum();
                    hit_player(ctx, self.id, (-dy * sweep_dir, dx * sweep_dir), 3.2);
                }
            }
        }
        self.t < self.total()
    }

    /// Draw the hazard’s crisp silhouette and motivated light.
    pub fn draw(&self, f: &mut Frame) {
        let (ox, oy) = (self.ox, self.oy);
        if self.charging() {
            // aim line: a thin dotted danger line along the start angle, crawling outward,
            // and chevrons showing which way it will sweep
            let k = self.t as f32 / self.charge as f32;
            if self.t > self.charge / 4 {
                let (dx, dy) = (self.a0.cos(), self.a0.sin());
                let blink = self.t + 10 > self.charge && (self.t / 2) % 2 == 0;
                let c = if blink { D4 } else { D2.lerp(D3, k) };
                let mut d = 14.0;
                while d < BEAM_LEN {
                    let phase = ((d - self.t as f32 * 1.5) / 6.0).rem_euclid(1.0);
                    if phase < 0.5 {
                        let (x, y) = (f.sx(ox + dx * d), f.sy(oy + dy * d));
                        f.put(x, y + 1, F0);
                        f.put_glow(x, y, c);
                    }
                    d += 1.0;
                }
                // chevrons along the sweep arc
                let dir = angle_diff(self.a0, self.a1).signum();
                let n = 4;
                for i in 0..n {
                    let u = (i as f32 + (self.t as f32 * 0.04).rem_euclid(1.0)) / n as f32;
                    let a = self.a0 + angle_diff(self.a0, self.a1) * u;
                    let rr = 78.0;
                    let (cx, cy) = (ox + a.cos() * rr, oy + a.sin() * rr);
                    let (tx, ty) = (-a.sin() * dir, a.cos() * dir); // sweep tangent
                    let (nx, ny) = (a.cos(), a.sin());
                    for j in 0..4 {
                        let j = j as f32;
                        for s in [-1.0, 1.0] {
                            let x = cx - tx * j + nx * j * s;
                            let y = cy - ty * j + ny * j * s;
                            f.put(f.sx(x), f.sy(y) + 1, F0);
                            f.put_glow(f.sx(x), f.sy(y), D2.lerp(D3, k));
                        }
                    }
                }
            }
            // gathering flare at the source
            let r = (k * 4.0).round() as i32;
            let (sx, sy) = (f.sx(ox), f.sy(oy));
            for i in -r..=r {
                f.put_glow(sx + i, sy, PW);
                f.put_glow(sx, sy + i, PW);
            }
            f.add_light_w(ox, oy, 12.0 + k * 12.0, PW, 0.18 + k * 0.18);
            return;
        }
        let w = self.width();
        if w <= 0.0 {
            return;
        }
        let (dx, dy) = (self.a.cos(), self.a.sin());
        // rasterize: every screen pixel near the line gets its color by perpendicular distance
        let (sx0, sy0) = (ox - f.cam_x as f32, oy - f.cam_y as f32);
        let (ex, ey) = (sx0 + dx * BEAM_LEN, sy0 + dy * BEAM_LEN);
        let pad = w + 3.0;
        let x0 = (sx0.min(ex) - pad).floor().max(0.0) as i32;
        let x1 = (sx0.max(ex) + pad).ceil().min(crate::gfx::WI as f32 - 1.0) as i32;
        let y0 = (sy0.min(ey) - pad).floor().max(0.0) as i32;
        let y1 = (sy0.max(ey) + pad).ceil().min(crate::gfx::HI as f32 - 1.0) as i32;
        // A hard narrow core, one iris body tone and a stepped mineral edge at the same width.
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (qx, qy) = (x as f32 + 0.5 - sx0, y as f32 + 0.5 - sy0);
                let along = qx * dx + qy * dy;
                if along < 0.0 || along > BEAM_LEN {
                    continue;
                }
                let perp = qx * -dy + qy * dx;
                let ap = perp.abs();
                if ap > w + 2.0 {
                    continue;
                }
                let c = if ap <= w.min(1.0) {
                    PW
                } else if ap <= w + 0.6 {
                    P2
                } else if bayer4(x + f.cam_x, y + f.cam_y) < 0.5 {
                    P0
                } else {
                    continue;
                };
                f.put_glow(x, y, c);
            }
        }
        // source flare: a crisp starburst
        let (sx, sy) = (f.sx(ox), f.sy(oy));
        let flare = 5 + (self.t % 4) as i32 / 2;
        for i in -flare..=flare {
            let c = if i.abs() < 3 { PW } else { P2 };
            f.put_glow(sx + i, sy, c);
            f.put_glow(sx, sy + i, c);
        }
        for i in -2..=2 {
            f.put_glow(sx + i, sy + i, PW);
            f.put_glow(sx + i, sy - i, PW);
        }
        // Small source-bound lights along the actual beam, leaving the floor calm.
        let mut d = 0.0;
        while d < 300.0 {
            f.add_light_w(ox + dx * d, oy + dy * d, 16.0, PW, 0.18);
            d += 40.0;
        }
    }
}

// ------------------------------------------------------------------ falling shards and rocks

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShardKind {
    /// A prism shard from the sky: shatters into glowing pixels.
    Crystal,
    /// A chunk of the crumbling summit (phase 3): stone rubble.
    Rock,
    /// A big crystal that stays standing as a pylon.
    Pylon,
}

#[derive(Clone, Debug)]
pub struct Shard {
    pub x: f32,
    pub y: f32,
    pub t: u32,
    /// Ticks before the marker appears.
    pub delay: u32,
    /// Marker time before impact.
    pub warn: u32,
    pub kind: ShardKind,
    pub id: u32,
    /// Landed this tick (the boss turns pylons into `Pylon`s).
    pub landed: bool,
}

const FALL: u32 = 14;
const SKY: f32 = 240.0;

impl Shard {
    /// Create a telegraphed hazard at its world anchor.
    pub fn new(x: f32, y: f32, delay: u32, warn: u32, kind: ShardKind, id: u32) -> Shard {
        Shard {
            x,
            y,
            t: 0,
            delay,
            warn,
            kind,
            id,
            landed: false,
        }
    }
    /// Return the radius shared by the landing marker and impact.
    pub fn radius(&self) -> f32 {
        match self.kind {
            ShardKind::Crystal => 9.0,
            ShardKind::Rock => 10.0,
            ShardKind::Pylon => 13.0,
        }
    }
    /// Height above the ground now (None while not falling yet).
    pub fn z(&self) -> Option<f32> {
        let s = self.t.checked_sub(self.delay)?;
        let start = self.warn.saturating_sub(FALL);
        let fs = s.checked_sub(start)?;
        let k = (fs as f32 / FALL as f32).min(1.0);
        Some(SKY * (1.0 - k * k))
    }

    /// Returns false when done.
    pub fn update(&mut self, ctx: &mut EnemyCtx) -> bool {
        self.t += 1;
        self.landed = false;
        if self.t < self.delay {
            return true;
        }
        let s = self.t - self.delay;
        let r = self.radius();
        let k = (s as f32 / self.warn as f32).min(1.0);
        let locked = if s + 10 > self.warn {
            self.warn + 10 - s
        } else {
            0
        };
        marker(ctx.fx, self.x, self.y, r, k, locked.min(20), s);
        if s == self.warn {
            self.landed = true;
            if ctx.player_alive {
                let (px, py) = ctx.player;
                let d = ((px - self.x).powi(2) + (py - self.y).powi(2)).sqrt();
                if d < r + 3.0 {
                    let dir = if d > 0.1 {
                        ((px - self.x) / d, (py - self.y) / d)
                    } else {
                        (0.0, 1.0)
                    };
                    hit_player(ctx, self.id, dir, 3.0);
                }
            }
            let fx = &mut *ctx.fx;
            match self.kind {
                ShardKind::Crystal => {
                    fx.sound_vol("crystal_shatter_2", 0.45);
                    fx.shake(0.12);
                    for _ in 0..34 {
                        let (dx, dy) = fx.rng.dir();
                        let sp = fx.rng.range(0.5, 2.2);
                        let c = *fx.rng.pick(&[P2, P1, PW, P2, P1]);
                        fx.particles.add(
                            Particle::new(self.x + dx * 2.0, self.y + dy)
                                .z(fx.rng.range(0.0, 6.0))
                                .vel(dx * sp, dy * sp * 0.7, fx.rng.range(0.8, 2.6))
                                .color(c)
                                .fade_to(P0)
                                .glow(1.0)
                                .life(fx.rng.range(120.0, 260.0))
                                .grav(0.16)
                                .drag(0.95)
                                .bounce(0.35)
                                .settle(true),
                        );
                    }
                    fx.particles
                        .sparks(&mut fx.rng, self.x, self.y, 2.0, 0.0, -1.0, 8, P2, 2.2);
                }
                ShardKind::Rock => {
                    fx.sound_vol("boss_slam", 0.35);
                    fx.shake(0.16);
                    rubble(fx, self.x, self.y, 30, 1.6);
                    fx.particles.dust(&mut fx.rng, self.x, self.y, 8, F4);
                }
                ShardKind::Pylon => {
                    fx.sound("crystal_shatter");
                    fx.sound_vol("boss_slam", 0.6);
                    fx.shake(0.35);
                    rubble(fx, self.x, self.y, 26, 1.8);
                    fx.particles.dust(&mut fx.rng, self.x, self.y, 12, F4);
                    fx.particles
                        .sparks(&mut fx.rng, self.x, self.y, 8.0, 0.0, -1.0, 16, P2, 2.6);
                }
            }
            // a cracked crater left in the ground
            crater(fx, self.x, self.y, r as i32 - 3, 240);
            return false;
        }
        true
    }

    /// Draw the hazard’s crisp silhouette and motivated light.
    pub fn draw(&self, f: &mut Frame) {
        let Some(z) = self.z() else { return };
        let (sx, sy) = (f.sx(self.x), f.sy(self.y - z));
        let h = match self.kind {
            ShardKind::Pylon => 16,
            _ => 8,
        };
        // the falling body: a crisp elongated diamond with a streak behind it
        let (core, edge) = match self.kind {
            ShardKind::Rock => (S4, S2),
            _ => (PW, P2),
        };
        for i in 1..(h * 3) {
            let c = if self.kind == ShardKind::Rock {
                F3
            } else if i < h {
                P2
            } else if i < h * 2 {
                P1
            } else {
                P0
            };
            if self.kind == ShardKind::Rock {
                if i < h && (i % 2 == 0) {
                    f.put(sx, sy - h - i, c);
                }
            } else {
                f.put_glow(sx, sy - h - i, c);
            }
        }
        for y in -h..=0 {
            let half = ((h + y) as f32 / h as f32
                * if self.kind == ShardKind::Pylon {
                    4.0
                } else {
                    2.0
                })
            .round() as i32;
            let half = half.min(if y > -2 { 1 } else { 9 });
            for x in -half..=half {
                let c = if x <= 0 { core } else { edge };
                if self.kind == ShardKind::Rock {
                    f.put(sx + x, sy + y, c);
                } else {
                    f.put_glow(sx + x, sy + y, c);
                }
            }
        }
        if self.kind != ShardKind::Rock {
            f.add_light_w(self.x, self.y - z, 12.0, PW, 0.22);
        }
        // shadow grows as it nears the ground
        let k = 1.0 - z / SKY;
        let rr = (self.radius() * 0.5 * k).round() as i32;
        if rr > 0 {
            f.shadow(f.sx(self.x), f.sy(self.y), rr, (rr / 2).max(1), 0.4 * k);
        }
    }
}

/// Cracked ground: a ragged ring of dark pixels and a few radial cracks, lasting `life` ticks.
pub fn crater(fx: &mut Fx, x: f32, y: f32, r: i32, life: u32) {
    let (cx, cy) = (x.round() as i32, y.round() as i32);
    let r = r.max(3);
    let mut pts = Vec::new();
    circle_pts(r, |a, b| pts.push((a, b)));
    for (a, b) in pts {
        if fx.rng.chance(0.7) {
            scar(fx, cx + a, cy + b, F1, life);
        }
    }
    let n = 5 + fx.rng.irange(0, 3);
    for i in 0..n {
        let ang = i as f32 / n as f32 * TAU + fx.rng.signed() * 0.4;
        let len = r as f32 + fx.rng.range(3.0, 9.0);
        let (mut px, mut py) = (0.0f32, 0.0f32);
        let (mut dx, mut dy) = (ang.cos(), ang.sin());
        let mut d = 0.0;
        while d < len {
            px += dx;
            py += dy;
            d += 1.0;
            if fx.rng.chance(0.25) {
                let a2 = dy.atan2(dx) + fx.rng.signed() * 0.6;
                dx = a2.cos();
                dy = a2.sin();
            }
            scar(
                fx,
                cx + px.round() as i32,
                cy + py.round() as i32,
                if d < r as f32 { F0 } else { F1 },
                life,
            );
        }
    }
    // a few pale chips around the rim
    for _ in 0..6 {
        let (dx, dy) = fx.rng.dir();
        let d = r as f32 + fx.rng.range(1.0, 4.0);
        scar(
            fx,
            cx + (dx * d).round() as i32,
            cy + (dy * d).round() as i32,
            F4,
            life / 2,
        );
    }
}

// ------------------------------------------------------------------ pylons (phase 2 shields)

#[derive(Clone, Debug)]
pub struct Pylon {
    pub x: f32,
    pub y: f32,
    pub hp: i32,
    pub flash: f32,
    pub alive: bool,
    pub t: u32,
    /// Knock wobble (px).
    pub wob: f32,
}

pub const PYLON_HP: i32 = 4;

impl Pylon {
    /// Create a telegraphed hazard at its world anchor.
    pub fn new(x: f32, y: f32) -> Pylon {
        Pylon {
            x,
            y,
            hp: PYLON_HP,
            flash: 0.0,
            alive: true,
            t: 0,
            wob: 0.0,
        }
    }
    /// Where its glowing heart is (world draw coords) and the hurt radius.
    pub fn hurtbox(&self) -> (f32, f32, f32) {
        (self.x, self.y - 9.0, 10.0)
    }
    /// Return the pylon tip where its shielding tether begins.
    pub fn top(&self) -> (f32, f32) {
        (self.x, self.y - 22.0)
    }
    /// Advance the hazard and emit hits only during its visible danger window.
    pub fn update(&mut self) {
        self.t += 1;
        self.flash = (self.flash - 0.12).max(0.0);
        self.wob *= -0.7;
        if self.wob.abs() < 0.3 {
            self.wob = 0.0;
        }
    }
}

/// Polyline from a to b that crackles: a few kinked segments, re-rolled every few ticks.
pub fn tether(f: &mut Frame, a: (f32, f32), b: (f32, f32), t: u32, seed: u32, c: Rgb, core: Rgb) {
    let n = 7;
    let mut prev = (f.sx(a.0), f.sy(a.1));
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let (nx, ny) = (-dy / len, dx / len);
    for i in 1..=n {
        let k = i as f32 / n as f32;
        let jit = if i == n {
            0.0
        } else {
            (crate::util::hash2(i, (t / 3) as i32, seed) - 0.5) * 6.0
        };
        let p = (a.0 + dx * k + nx * jit, a.1 + dy * k + ny * jit);
        let cur = (f.sx(p.0), f.sy(p.1));
        let mut pts = Vec::new();
        Frame::line_with(prev.0, prev.1, cur.0, cur.1, |x, y| pts.push((x, y)));
        for (j, (x, y)) in pts.into_iter().enumerate() {
            f.put_glow(x, y, if (j + i as usize) % 5 == 0 { core } else { c });
        }
        prev = cur;
    }
}

/// Angle helper: clamp an angle (radians, +y down) into [lo, hi] measured on the lower half.
pub fn clamp_down(a: f32, lo: f32, hi: f32) -> f32 {
    let mut a = a.rem_euclid(TAU);
    if a > PI * 1.5 {
        a -= TAU;
    }
    if a > PI {
        // pointing up: snap to the nearer side of the allowed range
        a = if a < PI * 1.25 { hi } else { lo };
    }
    a.clamp(lo, hi)
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Iris fissures keep the former rose ramp's luminance at every telegraph contrast step.
    #[test]
    fn prism_warnings_preserve_danger_contrast() {
        for (new, old) in [(D0, rgb(0x724b59)), (D1, rgb(0x724b59)),
            (D2, rgb(0xbd7a85)), (D3, rgb(0xe6b4ac)), (D4, rgb(0xe6b4ac))] {
            assert!((new.luma() - old.luma()).abs() < 0.000001);
        }
    }
}
