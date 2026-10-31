//! The Warden's two floating stone fists. Each fist is a small state machine; attacks only
//! start a move (`slam`, `raise`, `swat`, `sky`, `limp`) and the fist plays it out: rise with
//! weight, track, lock (anticipation lift + tremble), drop fast, sit stuck in the cracked ground
//! (its crystal exposed), pull free and float home.

use super::hazards::{self, D2, D3, F3, F4, S3, crater, marker, rubble};
use crate::combat::{Hit, Shape, Team};
use crate::fx::{Fx, Particle};
use crate::util::{ease_in_out, ease_out_cubic};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum St {
    /// Statue pose before the fight: resting on the ground.
    Dormant,
    Rest,
    /// Rising toward the hover point over the target while tracking it.
    Seek(u32),
    /// Target locked: lifts a little and trembles (the "now dodge" cue).
    Lock(u32),
    Drop(u32),
    /// Embedded in the ground; the crystal on its back is in reach.
    Stuck(u32, u32),
    Pull(u32),
    Return(u32),
    /// Double slam: overhead.
    Raise(u32),
    Overhead(u32),
    Slam2(u32),
    /// Close-range swat across the front.
    SwatWind(u32),
    SwatHold(u32),
    SwatSweep(u32),
    SwatRest(u32),
    /// Raised to the sky (shard rain).
    Sky(u32),
    /// Fallen limp beside the kneeling Warden.
    Limp(u32),
    /// Flailing upward during the death throes.
    Throes(u32),
}

/// Per-phase timing.
#[derive(Clone, Copy, Debug)]
pub struct Tempo {
    pub seek: u32,
    pub lock: u32,
    pub stuck: u32,
    pub stuck2: u32,
    pub overhead: u32,
    pub track: f32,
    pub gap: u32,
    pub ring_speed: f32,
    pub rings: u32,
    pub window: u32,
}

pub const TEMPO: [Tempo; 3] = [
    Tempo {
        seek: 52,
        lock: 18,
        stuck: 86,
        stuck2: 124,
        overhead: 24,
        track: 0.09,
        gap: 46,
        ring_speed: 2.0,
        rings: 1,
        window: 290,
    },
    Tempo {
        seek: 44,
        lock: 16,
        stuck: 70,
        stuck2: 100,
        overhead: 20,
        track: 0.12,
        gap: 34,
        ring_speed: 2.2,
        rings: 2,
        window: 260,
    },
    Tempo {
        seek: 36,
        lock: 14,
        stuck: 56,
        stuck2: 84,
        overhead: 16,
        track: 0.15,
        gap: 24,
        ring_speed: 2.45,
        rings: 2,
        window: 230,
    },
];

pub const HOVER: f32 = 62.0;
pub const FIST_CRYSTAL_HP: i32 = 5;
const DROP: u32 = 5;
const SWAT_R: f32 = 58.0;

pub struct Fist {
    /// 0 = screen-left, 1 = screen-right.
    pub side: usize,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    from: (f32, f32, f32),
    pub st: St,
    pub tx: f32,
    pub ty: f32,
    pub crystal_hp: i32,
    pub crystal_flash: f32,
    /// Ticks since the crystal started growing back (it's whole at `GROW`).
    pub crystal_grow: u32,
    pub flash: f32,
    pub hit_id: u32,
    /// Tremble offset this tick (px).
    pub jit: i32,
    /// Slammed into the ground this tick.
    pub impact: bool,
    /// Recent drawn positions for the drop smear.
    pub trail: [(f32, f32, f32); 3],
    pub t: u32,
}

pub const GROW: u32 = 48;

pub struct Ctx<'a> {
    /// Body anchor (feet).
    pub base: (f32, f32),
    /// Rest position (x, y, z).
    pub rest: (f32, f32, f32),
    pub player: (f32, f32),
    pub tempo: Tempo,
    pub fx: &'a mut Fx,
    pub hits: &'a mut Vec<Hit>,
    pub zone: &'a crate::world::Zone,
}

impl Fist {
    /// Create a dormant floating hand and its intact weak point.
    pub fn new(side: usize, x: f32, y: f32) -> Fist {
        Fist {
            side,
            x,
            y,
            z: 0.0,
            from: (x, y, 0.0),
            st: St::Dormant,
            tx: x,
            ty: y,
            crystal_hp: FIST_CRYSTAL_HP,
            crystal_flash: 0.0,
            crystal_grow: GROW,
            flash: 0.0,
            hit_id: 0,
            jit: 0,
            impact: false,
            trail: [(x, y, 0.0); 3],
            t: 0,
        }
    }

    /// Return the horizontal direction away from the body.
    pub fn sgn(&self) -> f32 {
        if self.side == 0 { -1.0 } else { 1.0 }
    }

    /// Report whether the weak point has finished regrowing.
    pub fn crystal_alive(&self) -> bool {
        self.crystal_hp > 0 && self.crystal_grow >= GROW
    }

    /// Low enough to reach (and actually holding still on the ground).
    pub fn low(&self) -> bool {
        self.z < 12.0
    }

    /// Free to take a new move.
    pub fn free(&self) -> bool {
        matches!(self.st, St::Rest | St::Return(_) | St::Pull(_))
    }

    /// Report whether the hand is embedded after a slam.
    pub fn stuck(&self) -> bool {
        matches!(self.st, St::Stuck(..))
    }

    /// Regrow a broken weak point for another opening attempt.
    pub fn regrow(&mut self) {
        if self.crystal_hp <= 0 {
            self.crystal_hp = FIST_CRYSTAL_HP;
            self.crystal_grow = 0;
        }
    }

    /// Capture the current pose before entering a new movement.
    fn enter(&mut self, st: St) {
        self.st = st;
        self.from = (self.x, self.y, self.z);
    }

    /// Begin tracking a target for an aimed slam.
    pub fn slam(&mut self, target: (f32, f32), fx: &mut Fx) {
        self.tx = target.0;
        self.ty = target.1;
        self.hit_id = fx.hit_id();
        self.enter(St::Seek(0));
    }
    /// Lift the hand for a synchronized double slam.
    pub fn raise(&mut self, fx: &mut Fx) {
        self.hit_id = fx.hit_id();
        self.enter(St::Raise(0));
    }
    /// Wind up a readable close-range sweep.
    pub fn swat(&mut self, fx: &mut Fx) {
        self.hit_id = fx.hit_id();
        self.enter(St::SwatWind(0));
    }
    /// Raise the hand to call down prism shards.
    pub fn sky(&mut self) {
        self.enter(St::Sky(0));
    }
    /// Drop the hand beside the kneeling body.
    pub fn limp(&mut self) {
        self.enter(St::Limp(0));
    }
    /// Return the hand to its floating rest pose.
    pub fn go_home(&mut self) {
        if !matches!(self.st, St::Rest | St::Return(_)) {
            self.enter(St::Return(0));
        }
    }
    /// Lift and tremble before the hand crumbles.
    pub fn throes(&mut self) {
        self.enter(St::Throes(0));
    }

    /// Interpolate from the captured pose to a target position.
    fn tween(&mut self, to: (f32, f32, f32), k: f32) {
        self.x = self.from.0 + (to.0 - self.from.0) * k;
        self.y = self.from.1 + (to.1 - self.from.1) * k;
        self.z = self.from.2 + (to.2 - self.from.2) * k;
    }

    /// Advance anticipation, contact and recovery for the current movement.
    pub fn update(&mut self, c: &mut Ctx) {
        self.t += 1;
        self.impact = false;
        self.jit = 0;
        self.flash = (self.flash - 0.1).max(0.0);
        self.crystal_flash = (self.crystal_flash - 0.12).max(0.0);
        if self.crystal_hp > 0 && self.crystal_grow < GROW {
            self.crystal_grow += 1;
            if self.crystal_grow == GROW {
                c.fx.sound_vol("charge_ready", 0.5);
                self.crystal_flash = 1.0;
            }
        }
        self.trail.rotate_right(1);
        self.trail[0] = (self.x, self.y, self.z);
        let s = self.sgn();
        let (bx, by) = c.base;
        let tempo = c.tempo;
        match self.st {
            St::Dormant => {
                self.x = bx + s * 54.0;
                self.y = by + 8.0;
                self.z = 0.0;
            }
            St::Rest => {
                let bob = ((self.t as f32) * 0.045 + self.side as f32 * PI).sin() * 1.6;
                let (rx, ry, rz) = c.rest;
                self.x += (rx - self.x) * 0.2;
                self.y += (ry - self.y) * 0.2;
                self.z += (rz + bob - self.z) * 0.2;
            }
            St::Seek(t) => {
                // the target follows Glint, a little behind (so moving always beats it)
                self.tx += (c.player.0 - self.tx) * tempo.track;
                self.ty += (c.player.1 - self.ty) * tempo.track;
                let goal = (self.tx, self.ty, HOVER);
                let rise = 26;
                if t < rise {
                    let k = ease_in_out(t as f32 / rise as f32);
                    self.tween(goal, k);
                    // lift with a little arc
                    self.z += (k * PI).sin() * 14.0;
                } else {
                    self.x += (goal.0 - self.x) * 0.35;
                    self.y += (goal.1 - self.y) * 0.35;
                    self.z += (goal.2 + ((t as f32) * 0.2).sin() - self.z) * 0.3;
                }
                let k = t as f32 / (tempo.seek + tempo.lock + DROP) as f32;
                marker(c.fx, self.tx, self.ty, 18.0, k, 0, self.t);
                if t + 1 >= tempo.seek {
                    self.enter(St::Lock(0));
                    c.fx.sound_vol("boss_telegraph", 0.8);
                } else {
                    self.st = St::Seek(t + 1);
                }
            }
            St::Lock(t) => {
                let k = ease_out_cubic(t as f32 / tempo.lock as f32);
                self.x += (self.tx - self.x) * 0.5;
                self.y += (self.ty - self.y) * 0.5;
                self.z = HOVER + k * 12.0;
                self.jit = if (t / 2) % 2 == 0 { 1 } else { -1 };
                marker(c.fx, self.tx, self.ty, 18.0, 0.8 + 0.2 * k, t + 1, self.t);
                if t + 1 >= tempo.lock {
                    self.enter(St::Drop(0));
                } else {
                    self.st = St::Lock(t + 1);
                }
            }
            St::Drop(t) => {
                let k = ((t + 1) as f32 / DROP as f32).min(1.0);
                self.x = self.tx;
                self.y = self.ty;
                self.z = self.from.2 * (1.0 - k * k);
                marker(c.fx, self.tx, self.ty, 18.0, 1.0, 20, self.t);
                if t + 1 >= DROP {
                    self.z = 0.0;
                    self.land(c, 16.0, 0.5, true);
                    self.enter(St::Stuck(0, tempo.stuck));
                    self.z = -1.0;
                } else {
                    self.st = St::Drop(t + 1);
                }
            }
            St::Stuck(t, len) => {
                self.z = -1.0;
                if t + 20 > len {
                    // struggling free: tremble and spit grit
                    self.jit = if (t / 2) % 2 == 0 { 1 } else { -1 };
                    if t % 4 == 0 {
                        let ox = c.fx.rng.signed() * 10.0;
                        rubble(c.fx, self.x + ox, self.y + 2.0, 1, 0.8);
                    }
                }
                if t + 1 >= len {
                    self.enter(St::Pull(0));
                    c.fx.sound_vol("boss_crack", 0.25);
                    rubble(c.fx, self.x, self.y, 10, 1.2);
                    c.fx.particles.dust(&mut c.fx.rng, self.x, self.y, 8, F4);
                } else {
                    self.st = St::Stuck(t + 1, len);
                }
            }
            St::Pull(t) => {
                let k = ease_out_cubic((t + 1) as f32 / 9.0);
                self.z = k * 18.0;
                if t + 1 >= 9 {
                    self.enter(St::Return(0));
                } else {
                    self.st = St::Pull(t + 1);
                }
            }
            St::Return(t) => {
                let len = 32;
                let k = ease_in_out((t + 1) as f32 / len as f32);
                self.tween(c.rest, k);
                if t + 1 >= len {
                    self.st = St::Rest;
                } else {
                    self.st = St::Return(t + 1);
                }
            }
            St::Raise(t) => {
                let len = 36;
                let goal = (bx + s * 30.0, by - 2.0, 118.0);
                let k = ease_in_out((t + 1) as f32 / len as f32);
                self.tween(goal, k);
                // markers where both fists will land
                let (lx, ly) = (bx + s * 24.0, by + 30.0);
                marker(c.fx, lx, ly, 17.0, k * 0.6, 0, self.t);
                if t + 1 >= len {
                    self.enter(St::Overhead(0));
                    if self.side == 0 {
                        c.fx.sound_vol("boss_telegraph", 0.9);
                    }
                } else {
                    self.st = St::Raise(t + 1);
                }
            }
            St::Overhead(t) => {
                self.jit = if (t / 2) % 2 == 0 { 1 } else { -1 };
                self.z = 118.0 + ((t as f32) / tempo.overhead as f32) * 8.0;
                let (lx, ly) = (bx + s * 24.0, by + 30.0);
                marker(
                    c.fx,
                    lx,
                    ly,
                    17.0,
                    0.7 + 0.3 * t as f32 / tempo.overhead as f32,
                    t + 1,
                    self.t,
                );
                if t + 1 >= tempo.overhead {
                    self.enter(St::Slam2(0));
                } else {
                    self.st = St::Overhead(t + 1);
                }
            }
            St::Slam2(t) => {
                let len = 6;
                let (lx, ly) = (bx + s * 24.0, by + 30.0);
                let k = ((t + 1) as f32 / len as f32).min(1.0);
                self.x = self.from.0 + (lx - self.from.0) * k;
                self.y = self.from.1 + (ly - self.from.1) * k;
                self.z = self.from.2 * (1.0 - k * k);
                marker(c.fx, lx, ly, 17.0, 1.0, 20, self.t);
                if t + 1 >= len {
                    self.tx = lx;
                    self.ty = ly;
                    self.z = 0.0;
                    self.land(c, 17.0, 0.0, true);
                    self.enter(St::Stuck(0, tempo.stuck2));
                    self.z = -1.0;
                } else {
                    self.st = St::Slam2(t + 1);
                }
            }
            St::SwatWind(t) => {
                let len = 24;
                let a0 = if self.side == 0 {
                    PI * 1.02
                } else {
                    -PI * 0.02
                };
                let goal = (
                    bx + a0.cos() * (SWAT_R + 16.0),
                    by + 8.0 + a0.sin() * SWAT_R * 0.8,
                    10.0,
                );
                let k = ease_out_cubic((t + 1) as f32 / len as f32);
                self.tween(goal, k);
                swat_marker(c.fx, bx, by, self.side, k * 0.7, 0, self.t);
                if t + 1 >= len {
                    self.enter(St::SwatHold(0));
                    c.fx.sound_vol("boss_telegraph", 0.8);
                } else {
                    self.st = St::SwatWind(t + 1);
                }
            }
            St::SwatHold(t) => {
                let len = 14;
                self.jit = if (t / 2) % 2 == 0 { 1 } else { -1 };
                swat_marker(c.fx, bx, by, self.side, 1.0, t + 1, self.t);
                if t + 1 >= len {
                    self.enter(St::SwatSweep(0));
                    c.fx.sound_vol("spin", 0.7);
                } else {
                    self.st = St::SwatHold(t + 1);
                }
            }
            St::SwatSweep(t) => {
                let len = 14;
                let (a0, a1) = if self.side == 0 {
                    (PI * 1.02, PI * 0.12)
                } else {
                    (-PI * 0.02, PI * 0.88)
                };
                let k = ease_in_out((t + 1) as f32 / len as f32);
                let a = a0 + (a1 - a0) * k;
                self.x = bx + a.cos() * SWAT_R;
                self.y = by + 8.0 + a.sin() * SWAT_R * 0.8;
                self.z = 8.0;
                swat_marker(c.fx, bx, by, self.side, 1.0, 20, self.t);
                c.hits.push(
                    Hit::new(
                        Shape::Circle {
                            x: self.x,
                            y: self.y,
                            r: 13.0,
                        },
                        Team::Enemy,
                        1,
                        self.hit_id,
                    )
                    .knock(4.0)
                    .heavy(true),
                );
                if t % 2 == 0 {
                    rubble(c.fx, self.x, self.y, 2, 1.0);
                }
                if t + 1 >= len {
                    self.enter(St::SwatRest(0));
                    c.fx.shake(0.2);
                } else {
                    self.st = St::SwatSweep(t + 1);
                }
            }
            St::SwatRest(t) => {
                let len = 44;
                self.z += (3.0 - self.z) * 0.3;
                if t + 1 >= len {
                    self.enter(St::Return(0));
                } else {
                    self.st = St::SwatRest(t + 1);
                }
            }
            St::Sky(t) => {
                let goal = (bx + s * 34.0, by - 4.0, 132.0);
                let k = ease_out_cubic(((t + 1) as f32 / 26.0).min(1.0));
                self.tween(goal, k);
                if t > 26 {
                    self.jit = if (t / 3) % 2 == 0 { 1 } else { 0 };
                }
                if t + 1 >= 72 {
                    self.enter(St::Return(0));
                } else {
                    self.st = St::Sky(t + 1);
                }
            }
            St::Limp(t) => {
                let len = 14;
                let goal = (bx + s * 56.0, by + 10.0, 0.0);
                let k = ((t + 1) as f32 / len as f32).min(1.0);
                self.x = self.from.0 + (goal.0 - self.from.0) * ease_out_cubic(k);
                self.y = self.from.1 + (goal.1 - self.from.1) * ease_out_cubic(k);
                self.z = self.from.2 * (1.0 - k * k);
                if t + 1 == len {
                    c.fx.sound_vol("boss_slam", 0.45);
                    c.fx.shake(0.2);
                    rubble(c.fx, self.x, self.y, 12, 1.2);
                    c.fx.particles.dust(&mut c.fx.rng, self.x, self.y, 8, F4);
                }
                self.st = St::Limp(t + 1);
            }
            St::Throes(t) => {
                let goal = (bx + s * 62.0, by - 6.0, 66.0 + (t as f32 * 0.3).sin() * 4.0);
                let k = ease_out_cubic(((t + 1) as f32 / 24.0).min(1.0));
                self.tween(goal, k);
                self.jit = if (t / 2) % 2 == 0 { 1 } else { -1 };
                self.st = St::Throes(t + 1);
            }
        }
    }

    /// The fist hits the ground: the hit, hit-stop, shake, dust, rubble, a crater.
    fn land(&mut self, c: &mut Ctx, r: f32, shake: f32, hit: bool) {
        self.impact = true;
        if hit {
            c.hits.push(
                Hit::new(
                    Shape::Circle {
                        x: self.tx,
                        y: self.ty,
                        r,
                    },
                    Team::Enemy,
                    1,
                    self.hit_id,
                )
                .knock(3.6)
                .heavy(true),
            );
        }
        let fx = &mut *c.fx;
        fx.sound("boss_slam");
        if shake > 0.0 {
            fx.shake(shake);
            fx.freeze(4);
        }
        fx.particles.dust(&mut fx.rng, self.tx, self.ty, 14, F4);
        rubble(fx, self.tx, self.ty, 22, 1.6);
        // a low ring of dust skidding outward
        for i in 0..20 {
            let a = i as f32 / 20.0 * PI * 2.0;
            fx.particles.add(
                Particle::new(self.tx + a.cos() * 8.0, self.ty + a.sin() * 6.0)
                    .z(1.0)
                    .vel(a.cos() * 1.6, a.sin() * 1.1, 0.4)
                    .color(F4)
                    .fade_to(F3)
                    .life(fx.rng.range(14.0, 24.0))
                    .grav(0.05)
                    .drag(0.88),
            );
        }
        crater(fx, self.tx, self.ty, r as i32 - 4, 300);
        let _ = c.zone;
    }
}

/// The swat's danger arc on the ground: the band the fist will sweep through.
fn swat_marker(fx: &mut Fx, bx: f32, by: f32, side: usize, k: f32, locked: u32, t: u32) {
    let (a0, a1) = if side == 0 {
        (PI * 0.12, PI * 1.02)
    } else {
        (-PI * 0.02, PI * 0.88)
    };
    let (r0, r1) = (SWAT_R - 12.0, SWAT_R + 12.0);
    let c_edge = if locked > 0 && locked < 6 {
        hazards::D4
    } else if locked > 0 {
        D2
    } else {
        hazards::D1.lerp(D2, k)
    };
    let ri = r1.ceil() as i32 + 2;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let (x, y) = (dx as f32, dy as f32 / 0.8);
            let d = (x * x + y * y).sqrt();
            if d < r0 - 0.5 || d > r1 + 0.5 {
                continue;
            }
            let a = y.atan2(x);
            let mut a = a;
            if a < a0 - 0.2 {
                a += PI * 2.0;
            }
            if a < a0 || a > a1 {
                continue;
            }
            let edge = d < r0 + 0.5 || d > r1 - 0.5 || (a - a0) < 0.03 || (a1 - a) < 0.03;
            let (px, py) = (bx.round() as i32 + dx, (by + 8.0).round() as i32 + dy);
            if edge {
                // dashes crawl along the edge while aiming
                if locked == 0 && ((a * 12.0 + t as f32 * 0.08).rem_euclid(1.0) < 0.35) {
                    continue;
                }
                hazards::prism_crack(fx, px, py, c_edge);
            } else if locked > 0 && crate::util::bayer4(px, py) < 0.19 {
                hazards::decal(fx, px, py, if locked < 6 { D3 } else { hazards::D1 });
            }
        }
    }
    let _ = S3;
}
