//! The Warden: a giant stone guardian with a cracked prism heart, assembled from part sheets
//! (`warden_body`, `warden_head`, `warden_fist`, `warden_heart`, `warden_crystal`, placed by
//! `warden_layout.json`). Submodules: `art` (sheets, layout, blitting), `fist` (the two floating
//! fists), `hazards` (rings, beams, shards, pylons, ground telegraphs), `death` (the finale).
//!
//! The fight, for a first-time player:
//! - Wake: Glint walks up, the statue's eyes ignite, it rises and roars (a harmless gust ring).
//! - Phase 1, Stone Fists: aimed fist slams (the fist hovers over a closing target ring, locks
//!   with a tremble and a flash, drops) and a double slam that sends a shockwave ring across the
//!   arena (dash through it). A slammed fist stays stuck in the ground for a while: the crystal
//!   on its back is the weak point. Break both crystals and the Warden drops to its knees, its
//!   chest splits open and the prism heart floats out in front of it. Hit the heart.
//! - Phase 2, Prism: three crystal pylons crash down and shield the heart; the shoulder crystals
//!   fire sweeping prism beams (aim line and sweep chevrons first), shards rain from the sky
//!   (each with a ground marker). Break the pylons to bring the heart out again.
//! - Phase 3, Cracked: faster slam chains, two shockwave rings, the summit crumbles (falling
//!   rocks), twin crossing beams. After the twin beams the heart overheats and drops out.
//! - Death: cracks spread, light pours out in rays, the fists crumble to rubble, the body
//!   bursts into thousands of glowing pixels, and a last sliver of the prism rises into the sky.
//!
//! Every attack deals 1 damage. If Glint falls, the Warden rests and heals back to the start of
//! its current phase, so progress is kept per phase.
//!
//! Interface the scene uses:
//! - `Boss::new(x, y)`, `update(&mut self, ctx)`, `draw(&self, f)`, `sort_y()`
//! - `hurtboxes()` -> circles (x, y, r, part index) in draw space; `on_hit(hit, part, ctx)`
//! - `hp_frac()` for the HUD bar, `finished()` once the death sequence is over (-> victory)
//! - `solid()`: the feet as an ellipse the scene can push Glint out of (optional)

mod art;
mod death;
mod fist;
mod hazards;
mod kneel;
mod material;
mod pose;

use super::Body;
use super::enemies::EnemyCtx;
use crate::combat::Hit;
use crate::fx::{Fx, Particle};
use crate::gfx::{Frame, Rgb};
use crate::util::{ease_in_out, ease_out_cubic};
use art::{Look, Spr, blit, layout, spr, spr_frame};
use fist::{Fist, St, TEMPO};
use hazards::{Beam, F4, F5, P1, P2, PW, Pylon, Ring, Shard, ShardKind, rubble};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

pub const MAX_HP: i32 = 60;
/// Each phase is this much of the bar.
const PHASE_HP: i32 = 20;
/// How far the body sinks when it kneels (px).
fn kneel_depth() -> f32 { pose::rig().kneel_depth as f32 }
const INTRO: u32 = 172;
const TRANSITION: u32 = 156;
const ROUSE: u32 = 80;
/// Where the heart floats when it drops out: ground offset in front of the feet and height.
const HEART_OUT: (f32, f32) = (24.0, 14.0);

const PART_CRYSTAL: usize = 0;
const PART_PYLON: usize = 10;
const PART_HEART: usize = 20;
const PART_FIST: usize = 30;
const PART_BODY: usize = 40;

const HEART_LIGHT: Rgb = PW;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Dormant,
    Intro(u32),
    Fight,
    /// Knelt, heart out: the punish window.
    Stagger(u32),
    /// Between phases (roar, new powers).
    Transition(u32),
    /// Glint fell: wait, healed to the start of the phase.
    Rest(u32),
    /// Glint is back: a short roar, then fight.
    Rouse(u32),
    Dying(u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Plan {
    Slam,
    Double,
    Beam,
    Rain,
    Chain,
    Twin,
}

const PLANS: [&[Plan]; 3] = [
    &[Plan::Slam, Plan::Slam, Plan::Double],
    &[Plan::Beam, Plan::Slam, Plan::Rain, Plan::Slam, Plan::Double],
    &[Plan::Chain, Plan::Rain, Plan::Double, Plan::Twin],
];

#[derive(Clone, Copy, Debug)]
enum Attack {
    Slam {
        side: usize,
    },
    Double {
        landed: bool,
        rings_left: u32,
        ring_t: u32,
    },
    Swat {
        side: usize,
    },
    Beam {
        t: u32,
        twin: bool,
    },
    Rain {
        side: usize,
        t: u32,
    },
    Chain {
        left: u32,
        side: usize,
    },
    Summon {
        t: u32,
    },
}

#[derive(Clone, Copy, Debug, Default)]
struct Heart {
    /// 0 = in the chest socket, 1 = floating out in front.
    out: f32,
    flash: f32,
    /// Recoil offset (px), springs back.
    wob: (f32, f32),
    /// Hits taken this phase (crack level).
    hits: u32,
}

pub struct Boss {
    pub body: Body,
    pub t: u32,
    /// Ticks into the death sequence (0 while alive).
    pub dying: u32,
    hp: i32,
    phase: usize,
    mode: Mode,
    fists: [Fist; 2],
    attack: Option<Attack>,
    plan_i: usize,
    cool: u32,
    rings: Vec<Ring>,
    beams: Vec<Beam>,
    shards: Vec<Shard>,
    pylons: Vec<Pylon>,
    heart: Heart,
    player: (f32, f32),
    /// Ticks Glint has been hugging the Warden's front (it swats).
    hug: u32,
    rock_t: u32,
    recent: [u32; 16],
    recent_i: usize,
    // --- look
    body_dy: f32,
    recover_from: f32,
    tremble: i32,
    head_dy: f32,
    eyes: f32,
    flash: f32,
    shoulder: [f32; 2],
    /// Body crack pixels (cell coords, growth order) and how many are lit.
    cracks: Vec<(i32, i32, u8)>,
    crack_lit: f32,
    /// The phase's pylons have been summoned at least once (so "all broken" means something).
    pylons_up: bool,
    /// Heart hits drained this phase's bar: leave the stagger.
    chunk_done: bool,
    shard_y: f32,
}

impl Boss {
    /// Create the dormant guardian and seed its branching stone cracks.
    pub fn new(x: f32, y: f32) -> Boss {
        let mut body = Body::new(x, y, 24.0, MAX_HP);
        body.face = (0.0, 1.0);
        let l = layout();
        // crack paths through the body, seeded around the heart socket and the shoulder crystals
        let b = spr(art::BODY, &["idle", "intact"], 0, true);
        let socket = (b.anchor.0 + l.heart.0, b.anchor.1 + l.heart.1);
        let mut seeds = Vec::new();
        for i in 0..9 {
            let a = i as f32 / 9.0 * TAU + 0.3;
            seeds.push((
                socket.0 + (a.cos() * 10.0) as i32,
                socket.1 + (a.sin() * 12.0) as i32,
            ));
        }
        for c in l.crystals {
            seeds.push((b.anchor.0 + c.0, b.anchor.1 + c.1 + 6));
        }
        let cracks = art::crack_paths(b.cell, &seeds, socket, 0x9e11, 1100);
        Boss {
            body,
            t: 0,
            dying: 0,
            hp: MAX_HP,
            phase: 0,
            mode: Mode::Dormant,
            fists: [
                Fist::new(0, x + l.fists[0].0 as f32 - 9.0, y + 8.0),
                Fist::new(1, x + l.fists[1].0 as f32 + 9.0, y + 8.0),
            ],
            attack: None,
            plan_i: 0,
            cool: 0,
            rings: Vec::new(),
            beams: Vec::new(),
            shards: Vec::new(),
            pylons: Vec::new(),
            heart: Heart::default(),
            player: (x, y + 120.0),
            hug: 0,
            rock_t: 0,
            recent: [0; 16],
            recent_i: 0,
            body_dy: 2.0,
            recover_from: 0.0,
            tremble: 0,
            head_dy: 3.0,
            eyes: 0.0,
            flash: 0.0,
            shoulder: [0.15, 0.15],
            cracks,
            crack_lit: 0.0,
            pylons_up: false,
            chunk_done: false,
            shard_y: 0.0,
        }
    }

    /// Return the health threshold that ends the current phase.
    fn floor_hp(&self) -> i32 {
        MAX_HP - PHASE_HP * (self.phase as i32 + 1)
    }
    /// Return the health restored by the current phase checkpoint.
    fn top_hp(&self) -> i32 {
        MAX_HP - PHASE_HP * self.phase as i32
    }

    /// Return the world anchor beneath the guardian’s feet.
    fn base(&self) -> (f32, f32) {
        (self.body.x, self.body.y)
    }

    /// World draw position of a shoulder crystal's base.
    fn shoulder_pos(&self, i: usize) -> (f32, f32) {
        let l = layout();
        (
            self.body.x + l.crystals[i].0 as f32 + self.tremble as f32,
            self.body.y + l.crystals[i].1 as f32 + self.body_dy.round(),
        )
    }

    /// Where beams leave a shoulder crystal (its glowing middle).
    fn beam_src(&self, i: usize) -> (f32, f32) {
        let (x, y) = self.shoulder_pos(i);
        let s = spr(art::CRYSTAL, &["intact", "idle"], 0, true);
        (x, y - (s.anchor.1 as f32 * 0.55).round())
    }

    /// World draw position of the heart (socket or out in front), with recoil.
    fn heart_pos(&self) -> (f32, f32) {
        let l = layout();
        let (bx, by) = self.base();
        let sock = (
            bx + l.heart.0 as f32 + self.tremble as f32,
            by + l.heart.1 as f32 + self.body_dy.round(),
        );
        let bob = ((self.t as f32) * 0.07).sin() * 1.5;
        let out = (bx, by + HEART_OUT.0 - HEART_OUT.1 + bob);
        let k = ease_in_out(self.heart.out);
        let arc = (k * PI).sin() * 12.0;
        (
            sock.0 + (out.0 - sock.0) * k + self.heart.wob.0,
            sock.1 + (out.1 - sock.1) * k - arc + self.heart.wob.1,
        )
    }

    /// Place a floating fist at its authored rest offset.
    fn rest_pos(&self, side: usize) -> (f32, f32, f32) {
        let l = layout();
        let (bx, by) = self.base();
        let (ox, oy) = l.fists[side];
        (
            bx + ox as f32,
            by + 2.0,
            (-oy as f32 + 2.0 - self.body_dy).max(2.0),
        )
    }

    /// A walkable ground spot near (x, y) (searches outward a little).
    fn ground_near(ctx: &EnemyCtx, x: f32, y: f32) -> Option<(f32, f32)> {
        if !ctx.zone.blocked(x, y, 4.0) {
            return Some((x, y));
        }
        for r in [8.0, 16.0, 24.0, 32.0] {
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                let (nx, ny) = (x + a.cos() * r, y + a.sin() * r);
                if !ctx.zone.blocked(nx, ny, 4.0) {
                    return Some((nx, ny));
                }
            }
        }
        None
    }

    // ---------------------------------------------------------------- update

    /// Advance the fight, part poses and telegraphed hazards by one tick.
    pub fn update(&mut self, ctx: &mut EnemyCtx) {
        self.t += 1;
        self.player = ctx.player;
        self.flash = (self.flash - 0.08).max(0.0);
        self.heart.flash = (self.heart.flash - 0.1).max(0.0);
        self.heart.wob.0 *= 0.75;
        self.heart.wob.1 *= 0.75;
        self.tremble = 0;
        let tempo = TEMPO[self.phase];

        // Glint fell mid-fight: rest and heal back to the start of this phase.
        if !ctx.player_alive && matches!(self.mode, Mode::Fight | Mode::Stagger(_)) {
            self.enter_rest(ctx);
        }

        match self.mode {
            Mode::Dormant => {
                self.mode = Mode::Intro(0);
            }
            Mode::Intro(t) => self.intro(t, ctx),
            Mode::Fight => self.fight(ctx),
            Mode::Stagger(t) => self.stagger(t, ctx),
            Mode::Transition(t) => self.transition(t, ctx),
            Mode::Rest(t) => {
                self.settle();
                self.eyes += (0.45 - self.eyes) * 0.05;
                let near = self.body.dist_to(ctx.player.0, ctx.player.1) < 150.0;
                if ctx.player_alive && near && t > 60 {
                    self.mode = Mode::Rouse(0);
                } else {
                    self.mode = Mode::Rest(t + 1);
                }
            }
            Mode::Rouse(t) => self.rouse(t, ctx),
            Mode::Dying(t) => {
                self.dying = t + 1;
                self.die(t, ctx);
                self.mode = Mode::Dying(t + 1);
            }
        }

        // cracks catch up to their target
        let target = match self.mode {
            Mode::Dying(_) => self.cracks.len() as f32,
            _ => [0.0, 130.0, 360.0][self.phase],
        };
        let speed = if matches!(self.mode, Mode::Dying(_)) {
            7.0
        } else {
            4.0
        };
        if self.crack_lit < target {
            self.crack_lit = (self.crack_lit + speed).min(target);
        }
        // shoulder crystals: dim in phase 1, lit after
        let sh = match self.mode {
            Mode::Dormant => 0.15,
            Mode::Intro(t) if t < 40 => 0.15,
            _ => {
                if self.phase == 0 {
                    0.45
                } else {
                    1.0
                }
            }
        };
        for s in self.shoulder.iter_mut() {
            *s += (sh - *s) * 0.06;
        }

        // fists
        let dying_burst = matches!(self.mode, Mode::Dying(t) if t >= death::FIST_L_CRUMBLE);
        for i in 0..2 {
            if dying_burst
                && ((i == 0 && self.dying > death::FIST_L_CRUMBLE)
                    || (i == 1 && self.dying > death::FIST_R_CRUMBLE))
            {
                continue;
            }
            let rest = self.rest_pos(i);
            let mut c = fist::Ctx {
                base: self.base(),
                rest,
                player: ctx.player,
                tempo,
                fx: &mut *ctx.fx,
                hits: &mut *ctx.hits,
                zone: ctx.zone,
            };
            self.fists[i].update(&mut c);
        }

        // hazards
        self.rings.retain_mut(|r| r.update(ctx));
        let srcs = [self.beam_src(0), self.beam_src(1)];
        self.beams.retain_mut(|b| {
            let s = srcs[b.side];
            b.update(ctx, s.0, s.1)
        });
        let mut landed = Vec::new();
        self.shards.retain_mut(|s| {
            let alive = s.update(ctx);
            if s.landed && s.kind == ShardKind::Pylon {
                landed.push((s.x, s.y));
            }
            alive
        });
        for (x, y) in landed {
            self.pylons.push(Pylon::new(x, y));
            self.pylons_up = true;
        }
        for p in self.pylons.iter_mut() {
            p.update();
        }

        self.body.hp = self.hp;
        self.body.flash = self.flash;
    }

    /// Body and heart drift back to standing.
    fn settle(&mut self) {
        self.body_dy = (self.body_dy - 1.2).max(0.0);
        self.heart.out = (self.heart.out - 0.08).max(0.0);
        self.head_dy += (0.0 - self.head_dy) * 0.15;
    }

    /// Cancel attacks and restore the current phase after Glint falls.
    fn enter_rest(&mut self, _ctx: &mut EnemyCtx) {
        self.mode = Mode::Rest(0);
        self.attack = None;
        self.beams.clear();
        self.rings.clear();
        self.shards.clear();
        self.hug = 0;
        self.rock_t = 0;
        self.hp = self.top_hp();
        self.heart.hits = 0;
        self.chunk_done = false;
        for f in self.fists.iter_mut() {
            f.go_home();
            if self.phase == 0 {
                f.regrow();
            }
        }
        if self.phase == 1 {
            self.pylons.clear();
            self.shards.retain(|s| s.kind != ShardKind::Pylon);
            self.pylons_up = false;
        }
    }

    /// Wake the stone guardian with a rise, roar and harmless gusts.
    fn intro(&mut self, t: u32, ctx: &mut EnemyCtx) {
        let fx = &mut *ctx.fx;
        // eyes ignite with a stutter, the heart flares
        self.eyes = if t < 14 {
            if matches!(t, 0..=2 | 6..=7 | 10..=13) {
                1.0
            } else {
                0.2
            }
        } else {
            1.0
        };
        if t == 0 {
            self.flash = 0.4;
            self.heart.flash = 1.0;
        }
        if t == 4 {
            for f in self.fists.iter_mut() {
                f.go_home();
            }
        }
        // rises out of the statue pose, head thrown back to roar
        self.body_dy = 2.0 * (1.0 - ease_in_out(t as f32 / 26.0));
        self.head_dy = if t < 22 {
            3.0 - 3.0 * ease_out_cubic(t as f32 / 22.0)
        } else if t < 128 {
            -3.0
        } else {
            -3.0 + 3.0 * ease_in_out((t - 128) as f32 / 30.0)
        };
        if (24..120).contains(&t) {
            self.tremble = if (t / 2) % 2 == 0 { 1 } else { -1 };
        }
        if t == 24 {
            let (bx, by) = self.base();
            let id = fx.hit_id();
            self.rings
                .push(Ring::new(bx, by + 4.0, 20.0, 3.4, 240.0, id, true));
            fx.shake(0.55);
            fx.flash(0.12);
            fx.freeze(3);
        }
        if t == 48 {
            let (bx, by) = self.base();
            let id = fx.hit_id();
            self.rings
                .push(Ring::new(bx, by + 4.0, 20.0, 3.0, 200.0, id, true));
            fx.shake(0.3);
        }
        // stone dust shaken off the shoulders
        if t < 70 && t % 2 == 0 {
            let b = self.body_sprite();
            let (bx, by) = self.base();
            death::shed(fx, b, bx + self.tremble as f32, by + self.body_dy.round(), by, 2);
        }
        if t + 1 >= INTRO {
            self.mode = Mode::Fight;
            self.cool = 30;
        } else {
            self.mode = Mode::Intro(t + 1);
        }
    }

    /// Resume the current phase with a short, harmless warning roar.
    fn rouse(&mut self, t: u32, ctx: &mut EnemyCtx) {
        let fx = &mut *ctx.fx;
        self.settle();
        self.eyes += (1.0 - self.eyes) * 0.2;
        self.head_dy = if (8..60).contains(&t) { -3.0 } else { 0.0 };
        if t == 8 {
            fx.sound("boss_roar");
            fx.shake(0.4);
            let (bx, by) = self.base();
            let id = fx.hit_id();
            self.rings
                .push(Ring::new(bx, by + 4.0, 20.0, 3.2, 200.0, id, true));
        }
        if (8..50).contains(&t) {
            self.tremble = if (t / 2) % 2 == 0 { 1 } else { -1 };
        }
        if t + 1 >= ROUSE {
            self.mode = Mode::Fight;
            self.cool = 20;
            self.plan_i = 0;
            if self.phase == 1 && self.pylons.iter().all(|p| !p.alive) {
                self.attack = Some(Attack::Summon { t: 0 });
            }
        } else {
            self.mode = Mode::Rouse(t + 1);
        }
    }

    /// Advance the attack pattern and detect broken shield openings.
    fn fight(&mut self, ctx: &mut EnemyCtx) {
        self.settle();
        self.eyes = 1.0;
        let (bx, by) = self.base();
        let (px, py) = ctx.player;
        // hugging the front: it will swat
        if (px - bx).abs() < 66.0 && py > by - 10.0 && py < by + 54.0 {
            self.hug += 1;
        } else {
            self.hug = self.hug.saturating_sub(2);
        }

        // openings
        if self.phase == 0 && self.fists.iter().all(|f| f.crystal_hp <= 0) {
            self.start_stagger(ctx);
            return;
        }
        if self.phase == 1
            && self.pylons_up
            && self.pylons.iter().all(|p| !p.alive)
            && !self.shards.iter().any(|s| s.kind == ShardKind::Pylon)
        {
            self.start_stagger(ctx);
            return;
        }

        // the summit crumbles in phase 3: rocks fall around Glint
        if self.phase == 2 {
            self.rock_t += 1;
            if self.rock_t >= 110 {
                self.rock_t = 0;
                let fx = &mut *ctx.fx;
                let (ox, oy) = (fx.rng.signed() * 16.0, fx.rng.signed() * 12.0);
                let (rx, ry) = (bx + fx.rng.signed() * 150.0, by + fx.rng.range(30.0, 150.0));
                let id1 = fx.hit_id();
                let id2 = fx.hit_id();
                if let Some((x, y)) = Self::ground_near(ctx, px + ox, py + oy) {
                    self.shards
                        .push(Shard::new(x, y, 0, 56, ShardKind::Rock, id1));
                }
                if let Some((x, y)) = Self::ground_near(ctx, rx, ry) {
                    self.shards
                        .push(Shard::new(x, y, 20, 56, ShardKind::Rock, id2));
                }
            }
        }

        match self.attack {
            None => {
                if self.cool > 0 {
                    self.cool -= 1;
                    return;
                }
                self.choose(ctx);
            }
            Some(a) => self.step(a, ctx),
        }
    }

    /// Choose the next attack from the current phase’s readable sequence.
    fn choose(&mut self, ctx: &mut EnemyCtx) {
        let (bx, _) = self.base();
        let px = ctx.player.0;
        let near_side = if px < bx { 0 } else { 1 };
        // hugging: swat with the fist on Glint's side
        if self.hug > 100 && self.fists[near_side].free() && self.fists[near_side].st == St::Rest {
            self.hug = 0;
            self.fists[near_side].swat(ctx.fx);
            self.attack = Some(Attack::Swat { side: near_side });
            return;
        }
        let plan = PLANS[self.phase];
        let p = plan[self.plan_i % plan.len()];
        let started = match p {
            Plan::Slam => match self.free_fist(near_side) {
                Some(side) => {
                    self.fists[side].slam(ctx.player, ctx.fx);
                    self.attack = Some(Attack::Slam { side });
                    true
                }
                None => false,
            },
            Plan::Chain => match self.free_fist(near_side) {
                Some(side) => {
                    self.fists[side].slam(ctx.player, ctx.fx);
                    self.attack = Some(Attack::Chain { left: 2, side });
                    true
                }
                None => false,
            },
            Plan::Double => {
                if self.fists.iter().all(|f| f.free()) {
                    for f in self.fists.iter_mut() {
                        f.raise(ctx.fx);
                    }
                    self.attack = Some(Attack::Double {
                        landed: false,
                        rings_left: TEMPO[self.phase].rings,
                        ring_t: 0,
                    });
                    true
                } else {
                    false
                }
            }
            Plan::Rain => match self.free_fist(1 - near_side) {
                Some(side) => {
                    self.fists[side].sky();
                    ctx.fx.sound_vol("boss_telegraph", 0.7);
                    self.attack = Some(Attack::Rain { side, t: 0 });
                    true
                }
                None => false,
            },
            Plan::Beam => {
                self.attack = Some(Attack::Beam { t: 0, twin: false });
                true
            }
            Plan::Twin => {
                self.attack = Some(Attack::Beam { t: 0, twin: true });
                true
            }
        };
        if started {
            self.plan_i += 1;
        } else {
            self.cool = 2;
        }
    }

    /// Find an available fist, favoring an intact weak point in phase one.
    fn free_fist(&self, prefer: usize) -> Option<usize> {
        let prefer = if self.phase == 0 && self.fists[prefer].crystal_hp <= 0 {
            1 - prefer
        } else {
            prefer
        };
        if self.fists[prefer].free() {
            Some(prefer)
        } else if self.fists[1 - prefer].free() {
            Some(1 - prefer)
        } else {
            None
        }
    }

    /// Advance the selected attack until its recovery or opening.
    fn step(&mut self, a: Attack, ctx: &mut EnemyCtx) {
        let tempo = TEMPO[self.phase];
        let (bx, by) = self.base();
        let done = match a {
            Attack::Slam { side } => {
                !matches!(self.fists[side].st, St::Seek(_) | St::Lock(_) | St::Drop(_))
            }
            Attack::Swat { side } => !matches!(
                self.fists[side].st,
                St::SwatWind(_) | St::SwatHold(_) | St::SwatSweep(_)
            ),
            Attack::Chain { left, side } => {
                let busy = matches!(self.fists[side].st, St::Seek(_) | St::Lock(_) | St::Drop(_));
                if busy {
                    false
                } else if left == 0 {
                    true
                } else {
                    let other = 1 - side;
                    if self.fists[other].free() {
                        self.fists[other].slam(ctx.player, ctx.fx);
                        self.attack = Some(Attack::Chain {
                            left: left - 1,
                            side: other,
                        });
                    }
                    false
                }
            }
            Attack::Double {
                mut landed,
                mut rings_left,
                mut ring_t,
            } => {
                let fx = &mut *ctx.fx;
                if !landed && self.fists.iter().all(|f| f.impact || f.stuck()) {
                    landed = true;
                    rings_left = rings_left.saturating_sub(1);
                    ring_t = 0;
                    let id = fx.hit_id();
                    self.rings.push(Ring::new(
                        bx,
                        by + 30.0,
                        12.0,
                        tempo.ring_speed,
                        210.0,
                        id,
                        false,
                    ));
                    fx.sound("boss_shockwave");
                    fx.shake(0.7);
                    fx.freeze(6);
                    fx.flash(0.08);
                } else if landed && rings_left > 0 {
                    ring_t += 1;
                    if ring_t == 30 {
                        rings_left -= 1;
                        ring_t = 0;
                        let id = fx.hit_id();
                        self.rings.push(Ring::new(
                            bx,
                            by + 30.0,
                            12.0,
                            tempo.ring_speed,
                            210.0,
                            id,
                            false,
                        ));
                        fx.sound_vol("boss_shockwave", 0.8);
                        fx.shake(0.4);
                        for f in &self.fists {
                            rubble(fx, f.x, f.y, 10, 1.4);
                        }
                    }
                }
                self.attack = Some(Attack::Double {
                    landed,
                    rings_left,
                    ring_t,
                });
                landed && rings_left == 0
            }
            Attack::Beam { t, twin } => {
                if t == 0 {
                    self.fire_beams(twin, ctx);
                }
                self.attack = Some(Attack::Beam { t: t + 1, twin });
                if t > 2 && self.beams.is_empty() {
                    if twin {
                        // the heart overheats after the twin beams: the opening of phase 3
                        self.attack = None;
                        self.start_stagger(ctx);
                        return;
                    }
                    true
                } else {
                    false
                }
            }
            Attack::Rain { side, t } => {
                if t == 26 {
                    self.rain(ctx);
                }
                self.attack = Some(Attack::Rain { side, t: t + 1 });
                let _ = side;
                t >= 64
            }
            Attack::Summon { t } => {
                if t == 0 {
                    self.summon(ctx);
                }
                self.attack = Some(Attack::Summon { t: t + 1 });
                t > 90
            }
        };
        if done {
            self.attack = None;
            self.cool = tempo.gap;
        }
    }

    /// Charge a single sweep or a pair of crossing prism beams.
    fn fire_beams(&mut self, twin: bool, ctx: &mut EnemyCtx) {
        let fx = &mut *ctx.fx;
        fx.sound("beam_charge");
        let (px, py) = ctx.player;
        let p3 = self.phase >= 2;
        let (lo, hi) = (PI * 0.1, PI * 0.9);
        if twin {
            // both shoulders, sweeping inward so the beams cross
            for side in 0..2 {
                let (a0, a1) = if side == 0 {
                    (PI * 0.86, PI * 0.30)
                } else {
                    (PI * 0.14, PI * 0.70)
                };
                let id = fx.hit_id();
                let (ox, oy) = self.beam_src(side);
                self.beams.push(Beam {
                    ox,
                    oy,
                    a: a0,
                    a0,
                    a1,
                    t: 0,
                    charge: 84,
                    sweep: 104,
                    back: false,
                    id,
                    side,
                });
            }
        } else {
            let side = if px < self.body.x { 0 } else { 1 };
            let (ox, oy) = self.beam_src(side);
            let ap = (py - 8.0 - oy).atan2(px - ox);
            let ap = hazards::clamp_down(ap, lo, hi);
            let spread = if p3 { 0.95 } else { 0.8 };
            // left shoulder sweeps rightward (angle decreasing), right shoulder leftward
            let dir = if side == 0 { -1.0 } else { 1.0 };
            let a0 = (ap - dir * spread).clamp(lo, hi);
            let a1 = (ap + dir * spread).clamp(lo, hi);
            let id = fx.hit_id();
            self.beams.push(Beam {
                ox,
                oy,
                a: a0,
                a0,
                a1,
                t: 0,
                charge: if p3 { 70 } else { 86 },
                sweep: if p3 { 88 } else { 104 },
                back: p3,
                id,
                side,
            });
        }
    }

    /// Mark falling shards around Glint and across the arena.
    fn rain(&mut self, ctx: &mut EnemyCtx) {
        let (bx, by) = self.base();
        let (px, py) = ctx.player;
        let p3 = self.phase >= 2;
        let n = if p3 { 9 } else { 7 };
        let warn = if p3 { 46 } else { 54 };
        ctx.fx.sound_vol("charge_ready", 0.6);
        ctx.fx.flash(0.06);
        for i in 0..n {
            let fx = &mut *ctx.fx;
            let (x, y, delay) = if i == 0 {
                (px, py, 0)
            } else if i == 1 {
                (px + fx.rng.signed() * 20.0, py + fx.rng.signed() * 14.0, 26)
            } else {
                let a = fx.rng.range(0.05, 0.95) * PI;
                let r = fx.rng.range(40.0, 170.0);
                (
                    bx + a.cos() * r * 1.2,
                    by + 20.0 + a.sin() * r * 0.8,
                    6 + (i as u32) * 7,
                )
            };
            let id = fx.hit_id();
            if let Some((x, y)) = Self::ground_near(ctx, x, y) {
                self.shards
                    .push(Shard::new(x, y, delay, warn, ShardKind::Crystal, id));
            }
        }
    }

    /// Drop three breakable crystals that visibly shield the heart.
    fn summon(&mut self, ctx: &mut EnemyCtx) {
        let (bx, by) = self.base();
        ctx.fx.sound_vol("boss_telegraph", 0.9);
        self.pylons.retain(|p| p.alive);
        let spots = [
            (bx - 108.0, by + 62.0),
            (bx + 108.0, by + 62.0),
            (bx, by + 128.0),
        ];
        for (i, (x, y)) in spots.into_iter().enumerate() {
            let id = ctx.fx.hit_id();
            if let Some((x, y)) = Self::ground_near(ctx, x, y) {
                self.shards
                    .push(Shard::new(x, y, i as u32 * 12, 52, ShardKind::Pylon, id));
            }
        }
    }

    /// Clear hostile hazards and begin the heart’s punish window.
    fn start_stagger(&mut self, ctx: &mut EnemyCtx) {
        self.mode = Mode::Stagger(0);
        self.attack = None;
        self.beams.clear();
        self.rings.clear();
        self.shards.clear();
        self.hug = 0;
        self.chunk_done = false;
        for f in self.fists.iter_mut() {
            f.limp();
        }
        let fx = &mut *ctx.fx;
        fx.freeze(5);
        fx.shake(0.3);
        fx.flash(0.08);
        fx.sound("boss_crack");
    }

    /// Kneel, expose the heart, then recover or advance the phase.
    fn stagger(&mut self, t: u32, ctx: &mut EnemyCtx) {
        let tempo = TEMPO[self.phase];
        let (bx, by) = self.base();
        let window_end = 38 + tempo.window;
        let fx = &mut *ctx.fx;
        self.eyes = 0.5 + 0.2 * ((t as f32) * 0.2).sin();
        if t < window_end + 14 {
            self.body_dy = kneel::drop(t, kneel_depth(), pose::rig().fold_max as f32);
            self.head_dy = 2.0 * self.body_dy / kneel_depth();
        }
        if t == kneel::CONTACT {
            fx.sound_vol("boss_slam", 0.9);
            fx.shake(0.45);
            fx.freeze(4);
            // Load arrives at the planted supports, not the empty central arch.
            for (x, y) in self.support_contacts() {
                rubble(fx, x, y, 12, 1.2);
                fx.particles.dust(&mut fx.rng, x, y, 10, F4);
            }
        }
        if t < 17 {
            self.heart.out = 0.0;
        } else if (17..38).contains(&t) {
            // Tick 17 holds the closed socket before the opening ramp. Skipping it here
            // would fall into the full-out window, then snap back at tick 18.
            self.heart.out = ease_out_cubic((t - 17) as f32 / 20.0);
            if t == 18 {
                fx.sound("crystal_shatter");
                fx.flash(0.1);
            }
            if t == 37 {
                fx.sound_vol("charge_ready", 0.9);
            }
        } else if t < window_end {
            self.heart.out = 1.0;
            // the last second: it starts to pull itself together (heart trembles)
            if t + 50 > window_end && (t / 2) % 2 == 0 {
                self.heart.wob.0 += if (t / 4) % 2 == 0 { 0.8 } else { -0.8 };
            }
            if t + 50 == window_end {
                fx.sound_vol("boss_telegraph", 0.6);
            }
        } else if t < window_end + 14 {
            let k = (t - window_end + 1) as f32 / 14.0;
            self.heart.out = 1.0 - k * k;
        } else if t < window_end + 34 {
            self.heart.out = 0.0;
            if t == window_end + 14 {
                fx.sound_vol("boss_crack", 0.5);
            }
            self.body_dy = kneel::rise(kneel_depth(), t - window_end - 14);
            self.head_dy = 2.0 * self.body_dy / kneel_depth();
        } else {
            // window missed: back to the fight, weak points restored
            self.body_dy = 0.0;
            for f in &mut self.fists {
                f.go_home();
            }
            self.mode = Mode::Fight;
            self.cool = 40;
            self.heart.hits = 0;
            match self.phase {
                0 => {
                    for f in self.fists.iter_mut() {
                        f.regrow();
                    }
                }
                1 => {
                    self.attack = Some(Attack::Summon { t: 0 });
                    self.pylons_up = false;
                }
                _ => {}
            }
            return;
        }
        // light pours out of the open chest while the heart is out
        if self.heart.out > 0.2 && t % 3 == 0 {
            let l = layout();
            let (sx, sy) = (bx + l.heart.0 as f32, by + l.heart.1 as f32 + self.body_dy);
            let a = fx.rng.range(PI * 1.1, PI * 1.9);
            let c = *fx.rng.pick(&[P2, PW, P1, P2]);
            fx.particles.add(
                Particle::new(sx, by + 2.0)
                    .z(by + 2.0 - sy)
                    .vel(a.cos() * 0.6, 0.0, -a.sin() * 0.9)
                    .color(PW)
                    .fade_to(c)
                    .glow(1.0)
                    .grav(-0.01)
                    .drag(0.97)
                    .life(fx.rng.range(24.0, 44.0)),
            );
        }
        if self.chunk_done {
            self.chunk_done = false;
            if self.hp <= 0 {
                self.start_dying(ctx);
            } else {
                self.recover_from = self.body_dy;
                self.mode = Mode::Transition(0);
                self.attack = None;
            }
            return;
        }
        self.mode = Mode::Stagger(t + 1);
    }

    /// Retract the heart, roar and introduce the next phase’s shields.
    fn transition(&mut self, t: u32, ctx: &mut EnemyCtx) {
        let fx = &mut *ctx.fx;
        let (bx, by) = self.base();
        // the heart is yanked back in, the Warden rises
        self.heart.out = (self.heart.out - 0.12).max(0.0);
        if t > 6 {
            self.body_dy = kneel::rise(self.recover_from, t - 7);
        }
        if t == 0 {
            fx.sound("boss_crack");
            fx.freeze(8);
            fx.flash(0.3);
            fx.shake(0.5);
        }
        if t == 26 {
            for f in &mut self.fists {
                f.go_home();
            }
            // next phase: the roar
            self.phase = (self.phase + 1).min(2);
            self.heart.hits = 0;
            self.plan_i = 0;
            fx.sound("boss_roar");
            fx.shake(0.75);
            fx.flash(0.25);
            fx.freeze(4);
            let id = fx.hit_id();
            self.rings
                .push(Ring::new(bx, by + 4.0, 20.0, 3.6, 260.0, id, true));
            self.heart.flash = 1.0;
            if self.phase == 2 {
                // the chest plate bursts: stone chips fly from the socket
                let l = layout();
                let (sx, sy) = (bx + l.heart.0 as f32, by + l.heart.1 as f32);
                for _ in 0..70 {
                    let (dx, dy) = fx.rng.dir();
                    let sp = fx.rng.range(0.8, 2.6);
                    let c = *fx
                        .rng
                        .pick(&[hazards::S2, hazards::S3, hazards::S4, PW, P2]);
                    let glow = if c == PW || c == P2 { 1.0 } else { 0.0 };
                    fx.particles.add(
                        Particle::new(sx + dx * 6.0, by + 6.0)
                            .z(by + 6.0 - sy - dy * 6.0)
                            .vel(dx * sp, fx.rng.range(0.3, 1.4), -dy * sp + 0.8)
                            .color(c)
                            .glow(glow)
                            .grav(0.15)
                            .drag(0.96)
                            .bounce(0.3)
                            .life(fx.rng.range(140.0, 260.0))
                            .settle(true),
                    );
                }
            }
        }
        if (26..110).contains(&t) {
            self.head_dy = -3.0;
            self.tremble = if (t / 2) % 2 == 0 { 1 } else { -1 };
        } else if t >= 110 {
            self.head_dy += (0.0 - self.head_dy) * 0.12;
        }
        if t == 70 && self.phase == 1 {
            self.pylons_up = false;
            self.summon(ctx);
        }
        if t + 1 >= TRANSITION {
            self.mode = Mode::Fight;
            self.cool = 30;
        } else {
            self.mode = Mode::Transition(t + 1);
        }
    }

    /// Cancel combat and begin the guardian’s staged destruction.
    fn start_dying(&mut self, ctx: &mut EnemyCtx) {
        self.mode = Mode::Dying(0);
        self.dying = 0;
        self.attack = None;
        self.beams.clear();
        self.rings.clear();
        self.shards.clear();
        for f in self.fists.iter_mut() {
            f.throes();
        }
        for p in self.pylons.iter_mut() {
            p.alive = false;
        }
        let fx = &mut *ctx.fx;
        fx.freeze(26);
        fx.flash(0.7);
        fx.shake(0.8);
        fx.sound("boss_crack");
        fx.sound("boss_hit");
    }

    /// Crack the stone, crumble the fists and release the prism burst.
    fn die(&mut self, t: u32, ctx: &mut EnemyCtx) {
        let (bx, by) = self.base();
        let fx = &mut *ctx.fx;
        self.heart.out = (self.heart.out - 0.09).max(0.0);
        self.body_dy = (self.body_dy - 0.8).max(0.0);
        self.eyes = if t < death::BURST { 1.0 } else { 0.0 };
        self.head_dy = if t < 30 {
            -4.0 * ease_out_cubic(t as f32 / 30.0)
        } else {
            -4.0
        };
        let k = (t as f32 / death::BURST as f32).min(1.0);
        if t < death::BURST {
            // Keep the core seeded while the intact body approaches its burst.
            self.shard_y = self.heart_pos().1;
            self.tremble = if (t / 2) % 2 == 0 { 1 } else { -1 } * if k > 0.6 { 2 } else { 1 };
            self.flash = ((t as f32 - 150.0) / 56.0).clamp(0.0, 1.0) * 0.85;
            fx.shake(0.02 + 0.05 * k);
            if death::crack_sound_at(t) {
                fx.sound_vol("boss_crack", 0.5 + 0.5 * k);
            }
            // stone flakes off
            if t % 2 == 0 {
                let b = self.body_sprite();
                death::shed(fx, b, bx + self.tremble as f32, by + self.body_dy.round(), by, 1 + (k * 4.0) as usize);
            }
        }
        if t == death::FIST_L_CRUMBLE || t == death::FIST_R_CRUMBLE {
            let i = if t == death::FIST_L_CRUMBLE { 0 } else { 1 };
            let f = &self.fists[i];
            let s = spr(art::FIST, &["idle", "clenched"], self.t, true);
            let (hx, hy) = (f.x, f.y - f.z - 14.0);
            death::collapse(
                fx,
                s,
                f.x,
                f.y - f.z,
                i == 0,
                f.y,
                (hx, hy + 6.0),
                1.3,
                false,
            );
            fx.sound("crystal_shatter_2");
            fx.sound_vol("boss_slam", 0.6);
            fx.shake(0.35);
            fx.particles.dust(&mut fx.rng, f.x, f.y, 10, F4);
        }
        if t == death::BURST {
            fx.freeze(8);
            fx.flash(1.0);
            fx.shake(1.0);
            fx.sound("boss_die");
            fx.sound("boss_shockwave");
            fx.sound("crystal_shatter");
            let heart = self.heart_pos();
            let b = self.body_sprite();
            death::collapse(fx, b, bx, by, false, by, heart, 2.3, true);
            // burst the head frame exactly where it was drawn
            let (hd, hx, hy) = self.head_part();
            death::collapse(fx, hd, hx, hy, false, by, heart, 2.0, true);
            let hs = spr(art::HEART, &["idle"], self.t, true);
            death::collapse(fx, hs, heart.0, heart.1, false, by, heart, 2.8, true);
            for i in 0..2 {
                let (x, y) = self.shoulder_pos(i);
                let cs = spr(art::CRYSTAL, &["intact", "idle"], self.t, true);
                death::collapse(fx, cs, x, y, i == 0, by, heart, 2.4, true);
            }
            let id = fx.hit_id();
            self.rings
                .push(Ring::new(bx, by + 4.0, 16.0, 4.2, 320.0, id, true));
            self.shard_y = heart.1;
        }
        if t > death::BURST {
            // the last sliver of the prism hovers, then climbs into the sky
            let s = t - death::BURST;
            if t >= death::RISE {
                let r = (t - death::RISE) as f32;
                self.shard_y -= 0.2 + r * r * 0.004;
                if t % 2 == 0 {
                    let x = bx + fx.rng.signed() * 1.5;
                    fx.particles.add(
                        Particle::new(x, by)
                            .z(by - self.shard_y)
                            .vel(0.0, 0.0, -0.3)
                            .color(PW)
                            .fade_to(P2)
                            .glow(1.0)
                            .grav(0.0)
                            .drag(0.9)
                            .life(22.0),
                    );
                }
            } else {
                self.shard_y -= 0.15 * (1.0 - s as f32 / 60.0).max(0.0);
            }
            if t == death::RISE {
                fx.sound("shard_pickup");
            }
        }
    }

    // ---------------------------------------------------------------- hits

    /// Vulnerable circles (x, y, r, part) in world draw space. Weak points first, then armor
    /// (stone answers with a clink so it's clear where not to hit).
    pub fn hurtboxes(&self) -> Vec<(f32, f32, f32, usize)> {
        let mut v = Vec::new();
        match self.mode {
            Mode::Fight | Mode::Stagger(_) => {}
            _ => return v,
        }
        for (i, f) in self.fists.iter().enumerate() {
            if self.mode == Mode::Fight && f.low() && f.crystal_alive() {
                let (cx, cy) = self.fist_crystal_center(i);
                v.push((cx, cy, 10.0, PART_CRYSTAL + i));
            }
        }
        for (i, p) in self.pylons.iter().enumerate() {
            if p.alive {
                let (x, y, r) = p.hurtbox();
                v.push((x, y, r, PART_PYLON + i));
            }
        }
        if let Mode::Stagger(t) = self.mode
            && t >= 30
            && self.heart.out > 0.85
        {
            let (hx, hy) = self.heart_pos();
            v.push((hx, hy, 11.0, PART_HEART));
        }
        for (i, f) in self.fists.iter().enumerate() {
            if f.z < 40.0 {
                v.push((f.x, f.y - f.z - 13.0, 13.0, PART_FIST + i));
            }
        }
        let (bx, by) = self.base();
        v.push((bx, by - 22.0, 24.0, PART_BODY));
        v
    }

    /// Return the reachable weak point on the back of a grounded fist.
    fn fist_crystal_center(&self, i: usize) -> (f32, f32) {
        let f = &self.fists[i];
        let (_, _, back) = fist_pose(f);
        (f.x + f.jit as f32, f.y - f.z.max(0.0) - 26.0 + back as f32)
    }

    /// Apply one sword hit to a weak point or answer armor with a clink.
    pub fn on_hit(&mut self, hit: &Hit, part: usize, ctx: &mut EnemyCtx) -> bool {
        if !matches!(self.mode, Mode::Fight | Mode::Stagger(_))
            || self.hp <= 0
            || self.recent.contains(&hit.id)
        {
            return false;
        }
        self.recent[self.recent_i] = hit.id;
        self.recent_i = (self.recent_i + 1) % self.recent.len();
        let (hx, hy) = hit.shape.center();
        let heavy = hit.heavy;
        match part {
            p if (PART_CRYSTAL..PART_CRYSTAL + 2).contains(&p) => {
                let i = p - PART_CRYSTAL;
                let (cx, cy) = self.fist_crystal_center(i);
                let fx = &mut *ctx.fx;
                let f = &mut self.fists[i];
                f.crystal_hp -= hit.dmg;
                f.crystal_flash = 1.0;
                f.flash = 0.4;
                fx.sound_var("hit", 3);
                fx.sound_vol("crystal_shatter_2", 0.3);
                fx.freeze(if heavy { 7 } else { 4 });
                fx.shake(if heavy { 0.3 } else { 0.16 });
                let (dx, dy) = crate::util::norm(cx - hx, cy - hy);
                fx.particles.sparks(
                    &mut fx.rng,
                    cx,
                    cy + 20.0,
                    20.0,
                    dx,
                    dy,
                    if heavy { 16 } else { 10 },
                    P2,
                    2.6,
                );
                if f.crystal_hp <= 0 {
                    f.crystal_hp = 0;
                    fx.sound("crystal_shatter");
                    fx.freeze(9);
                    fx.shake(0.4);
                    fx.flash(0.12);
                    let s = spr(art::CRYSTAL, &["intact", "idle"], self.t, true);
                    let (fx_, fy_) = (f.x, f.y - f.z.max(0.0) - 22.0);
                    death::burst(fx, s, fx_, fy_, i == 0, f.y, (fx_, fy_ - 4.0), 1.6, true, 1);
                    fx.particles
                        .sparks(&mut fx.rng, cx, cy + 20.0, 20.0, 0.0, -1.0, 20, PW, 3.0);
                    let floor = self.floor_hp();
                    self.hp = (self.hp - 2).max(floor + 1);
                    self.flash = 0.3;
                }
                true
            }
            p if (PART_PYLON..PART_PYLON + 8).contains(&p) => {
                let i = p - PART_PYLON;
                let fx = &mut *ctx.fx;
                let Some(py) = self.pylons.get_mut(i) else {
                    return false;
                };
                py.hp -= hit.dmg;
                py.flash = 1.0;
                py.wob = if hx < py.x { 2.0 } else { -2.0 };
                fx.sound_var("hit", 3);
                fx.sound_vol("crystal_shatter_2", 0.35);
                fx.freeze(if heavy { 7 } else { 4 });
                fx.shake(if heavy { 0.3 } else { 0.16 });
                let (x, y, _) = py.hurtbox();
                let (dx, dy) = crate::util::norm(x - hx, y - hy);
                fx.particles
                    .sparks(&mut fx.rng, x, py.y, py.y - y, dx, dy, 12, P2, 2.6);
                if py.hp <= 0 {
                    py.alive = false;
                    fx.sound("crystal_shatter");
                    fx.freeze(9);
                    fx.shake(0.45);
                    fx.flash(0.1);
                    pylon_burst(fx, py);
                    let floor = self.floor_hp();
                    self.hp = (self.hp - 2).max(floor + 1);
                    self.flash = 0.3;
                    self.heart.flash = 0.8;
                }
                true
            }
            PART_HEART => {
                let fx = &mut *ctx.fx;
                let floor = self.floor_hp();
                self.hp = (self.hp - hit.dmg).max(floor);
                self.heart.flash = 1.0;
                self.heart.hits += 1;
                let (dx, dy) = hit.push_dir(self.heart_pos().0, self.heart_pos().1);
                self.heart.wob = (dx * 4.0, dy * 3.0);
                fx.sound("boss_hit");
                fx.sound_var("hit", 3);
                fx.freeze(if heavy { 9 } else { 5 });
                fx.shake(if heavy { 0.45 } else { 0.25 });
                let (x, y) = self.heart_pos();
                let gy = self.body.y + HEART_OUT.0;
                let c = *fx.rng.pick(&[P2, P1, P2]);
                fx.particles.sparks(
                    &mut fx.rng,
                    x,
                    gy,
                    gy - y,
                    dx,
                    dy,
                    if heavy { 22 } else { 14 },
                    c,
                    3.0,
                );
                // prism chips break off and settle as glowing glass
                for _ in 0..(if heavy { 14 } else { 8 }) {
                    let a = dy.atan2(dx) + fx.rng.signed() * 1.0;
                    let sp = fx.rng.range(0.8, 2.4);
                    let c = *fx.rng.pick(&[P2, P1, PW, P2]);
                    fx.particles.add(
                        Particle::new(x, gy)
                            .z(gy - y)
                            .vel(a.cos() * sp, a.sin() * sp * 0.6, fx.rng.range(0.5, 1.8))
                            .color(c)
                            .fade_to(P1)
                            .glow(1.0)
                            .grav(0.14)
                            .drag(0.95)
                            .bounce(0.3)
                            .life(fx.rng.range(120.0, 240.0))
                            .settle(true),
                    );
                }
                if self.hp <= floor {
                    self.chunk_done = true;
                }
                true
            }
            p if (PART_FIST..PART_FIST + 2).contains(&p) => {
                let i = p - PART_FIST;
                let f = &mut self.fists[i];
                f.flash = 0.35;
                let (x, y) = (f.x, f.y - f.z - 13.0);
                clink(ctx.fx, x, y, f.y, hx, hy);
                false
            }
            _ => {
                let (bx, by) = self.base();
                self.flash = self.flash.max(0.2);
                clink(ctx.fx, bx, by - 20.0, by, hx, hy);
                false
            }
        }
    }

    /// Return the remaining health fraction for the existing boss HUD.
    pub fn hp_frac(&self) -> f32 {
        (self.hp.max(0) as f32 / MAX_HP as f32).clamp(0.0, 1.0)
    }

    /// Test fixture: set the Warden's health and its body mirror, as `tick` would.
    #[cfg(test)]
    pub(crate) fn set_hp_for_test(&mut self, hp: i32) {
        self.hp = hp;
        self.body.hp = hp;
    }

    /// Report completion after the last prism sliver rises away.
    pub fn finished(&self) -> bool {
        matches!(self.mode, Mode::Dying(t) if t >= death::DONE)
    }

    /// The feet as an ellipse (center x, center y, radius x, radius y) the scene can keep Glint
    /// out of. None while it's gone.
    pub fn solid(&self) -> Option<(f32, f32, f32, f32)> {
        match self.mode {
            Mode::Dying(t) if t >= death::BURST => None,
            _ => Some((self.body.x, self.body.y - 3.0, 30.0, 8.0)),
        }
    }

    /// Draw depth. The boss draws as one item, so pick the depth that's right where it matters:
    /// a fist or pylon next to Glint sorts against her, otherwise the body does.
    pub fn sort_y(&self) -> f32 {
        let (px, py) = self.player;
        let (bx, by) = self.base();
        let (width, height) = layout().body_size;
        let over_body = (px - bx).abs() < width as f32 * 0.5
            && py > by - height as f32 && py < by + 16.0;
        if !over_body {
            for f in &self.fists {
                if (f.x - px).abs() < 24.0 && (f.y - py).abs() < 30.0 && (f.y > py || f.z > 16.0) {
                    return py + 0.5;
                }
            }
            for p in self.pylons.iter().filter(|p| p.alive) {
                if (p.x - px).abs() < 12.0 && p.y > py && p.y - py < 30.0 {
                    return py + 0.5;
                }
            }
        }
        by
    }

    // ---------------------------------------------------------------- drawing

    /// Active prism emitters in native draw coordinates, not the feet or the giant's crown.
    pub fn attack_sources(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        self.beams
            .iter()
            .filter(|b| b.charging() || b.firing())
            .map(|b| (b.ox, b.oy))
    }

    /// A persistent native indicator is required when emitter + hero cannot fit together.
    /// This UI layer is drawn after world composition, clear of health and the boss bar.
    pub fn draw_attack_cues(&self, f: &mut Frame) {
        for b in self.beams.iter().filter(|b| b.charging() || b.firing()) {
            let Some((sx, sy, dx, dy)) = attack_cue((b.ox - f.cam_x as f32, b.oy - f.cam_y as f32))
            else {
                continue;
            };
            let c = if b.charging() {
                P1.lerp(PW, b.t as f32 / b.charge as f32)
            } else {
                PW
            };
            // A small cut-crystal with an outward arrow, rather than a fake on-screen emitter.
            for y in -4i32..=4 {
                for x in -4i32..=4 {
                    let d = x.abs() + y.abs();
                    if d <= 4 {
                        f.ui_put(
                            sx + x,
                            sy + y,
                            if d == 4 {
                                hazards::F0
                            } else if d >= 2 {
                                P1
                            } else {
                                c
                            },
                        );
                    }
                }
            }
            for k in 0..3 {
                let (ax, ay) = (sx + dx * (7 + k), sy + dy * (7 + k));
                f.ui_put(ax, ay, c);
                if k < 2 {
                    f.ui_put(ax + dy * (2 - k), ay - dx * (2 - k), c);
                    f.ui_put(ax - dy * (2 - k), ay + dx * (2 - k), c);
                }
            }
            // Fill toward the source while charging, then hold solid throughout the sweep.
            let n = if b.charging() {
                (5.0 * b.t as f32 / b.charge as f32).ceil() as i32
            } else {
                5
            };
            for k in 0..5 {
                f.ui_put(sx - 2 + k, sy + 6, if k < n { c } else { hazards::F0 });
            }
        }
    }

    /// Highest drawn assembly or surviving core pixel, excluding hostile hazards.
    pub fn framing_top(&self) -> Option<f32> {
        if let Mode::Dying(t) = self.mode
            && t > death::BURST
        {
            return (t < death::DONE).then(|| {
                let s = spr("shard", &["spin"], self.t, true);
                self.shard_y + 8.0 + art::top(s) as f32
            });
        }
        let (_, by) = self.base();
        let body = self.body_sprite();
        let mut top = by + self.body_dy.round() + art::top(body) as f32;
        let (head, _, hy) = self.head_part();
        top = top.min(hy + self.breath() + art::top(head) as f32);
        let crystal = spr(art::CRYSTAL, &["intact", "idle"], self.t, true);
        for i in 0..2 {
            top = top.min(self.shoulder_pos(i).1 + art::top(crystal) as f32);
        }
        for fi in &self.fists {
            if self.dying > crumble_t(fi.side) {
                continue;
            }
            let y = fi.y - fi.z.max(0.0);
            let (s, lift, back) = fist_pose(fi);
            top = top.min(y + (lift + art::top(s)) as f32);
            if fi.crystal_hp > 0 {
                top = top.min(y - 22.0 + (back + art::top(crystal)) as f32);
            }
        }
        Some(top)
    }

    /// Assemble the artist’s parts and render active attacks on the native grid.
    pub fn draw(&self, f: &mut Frame) {
        self.draw_revealed(f, None);
    }

    /// Open nearby stone around Glint while keeping weak points and attack cues intact.
    pub fn draw_revealed(&self, f: &mut Frame, reveal: Option<crate::gfx::sprite::SpriteFocus>) {
        let l = layout();
        let (bx, by) = self.base();
        let dying_t = if let Mode::Dying(t) = self.mode {
            Some(t)
        } else {
            None
        };
        // `update` stores the next death tick after processing its effects. Retain the body
        // until BURST has actually run, so its swap to the shard shares the flash/collapse.
        let gone = dying_t.map(|t| t > death::BURST).unwrap_or(false);
        let kneel = self.body_dy.round();
        let trem = self.tremble as f32;

        // lasting light from the boss (added here so it survives hit-stop frames)
        self.lights(f);

        if gone {
            let t = dying_t.unwrap_or(0);
            let s = t - death::BURST;
            if s < 24 {
                let (hx, hy) = (bx, self.shard_y);
                death::rays(f, hx, hy, 150 + s * 2, 1.0 - s as f32 / 24.0);
            }
            if t < death::DONE {
                f.shadow(
                    f.sx(bx),
                    f.sy(by + HEART_OUT.0),
                    3,
                    1,
                    0.3 * (1.0 - (by - self.shard_y) / 200.0).max(0.0),
                );
                death::core_shard(f, bx, self.shard_y, self.t);
            }
            for p in &self.shards {
                p.draw(f);
            }
            return;
        }

        // shadows
        let sh_k = if self.mode == Mode::Dormant {
            0.5
        } else {
            0.55
        };
        let foot_width = (l.body_size.0 as f32 * (38.0 / 120.0)).round() as i32;
        f.shadow(f.sx(bx), f.sy(by + 1.0), foot_width, 8, sh_k);
        for fi in &self.fists {
            if dying_t.map(|t| t > crumble_t(fi.side)).unwrap_or(false) {
                continue;
            }
            let k = (1.0 - fi.z / 150.0).clamp(0.2, 1.0);
            let rx = (6.0 + 8.0 * k).round() as i32;
            f.shadow(f.sx(fi.x), f.sy(fi.y), rx, (rx / 3).max(2), 0.5 * k);
        }
        let heart_out = self.heart.out > 0.02;
        if self.heart.out > 0.3 {
            f.shadow(f.sx(bx), f.sy(by + HEART_OUT.0), 5, 2, 0.4 * self.heart.out);
        }

        // pylons behind the body
        for p in self.pylons.iter().filter(|p| p.alive && p.y < by) {
            self.draw_pylon(f, p);
        }
        // fists behind the body
        for fi in self.fists.iter().filter(|fi| fi.y < by - 4.0) {
            self.draw_fist(f, fi, dying_t, reveal);
        }

        let flash = self.flash.max(self.body.flash);
        let clip = if kneel > 0.5 { f.sy(by) } else { i32::MAX };
        // body
        let body = self.body_sprite();
        let bl = Look {
            flash,
            energy_flash: if dying_t.is_some() { flash } else { 0.0 },
            clip_y: clip,
            ground_y: Some(by + 1.0),
            reveal,
            ..Default::default()
        };
        let (sx, sy) = (f.sx(bx + trem), f.sy(by + kneel));
        blit(f, body, sx, sy, &bl);
        let lit = self.crack_lit as usize;
        if lit > 0 {
            let (core, edge) = if dying_t.is_some() {
                (PW, P2)
            } else {
                (PW, P2)
            };
            let fold = if kneel > 0.0 {
                spr_frame(art::BODY, &[self.body_fold_anim()], 0)
                    .map(|source| (source, kneel.clamp(0.0, pose::rig().fold_max as f32) as i32))
            } else {
                None
            };
            let transported;
            let cracks = if material::uses_slate_body() && pose::step(self.heart.out) > 0 {
                transported = self.cracks.iter().take(lit).filter_map(|&(x, y, w)|
                    pose::transport(x, y, self.heart.out).map(|(x, y)| (x, y, w)))
                    .collect::<Vec<_>>();
                transported.as_slice()
            } else {
                &self.cracks[..lit.min(self.cracks.len())]
            };
            art::draw_cracks(f, body, sx, sy, &bl, cracks, cracks.len(), edge, core, fold);
        }
        // the chest socket: light pours out when the heart is out
        if heart_out {
            let (sxw, syw) = (bx + l.heart.0 as f32 + trem, by + l.heart.1 as f32 + kneel);
            socket_glow(f, sxw, syw, self.heart.out, self.t);
        }
        // shoulder crystals
        for i in 0..2 {
            let (x, y) = self.shoulder_pos(i);
            let charging = self.beams.iter().any(|b| b.side == i && b.charging());
            let firing = self.beams.iter().any(|b| b.side == i && !b.charging());
            let s = spr(
                art::CRYSTAL,
                &["intact", "idle"],
                self.t + i as u32 * 17,
                true,
            );
            let glow = if firing { 1.0 } else { self.shoulder[i] };
            let cl = Look {
                flip: i == 1,
                emit: glow,
                flash: if charging {
                    0.3 + 0.3 * ((self.t / 3) % 2) as f32
                } else {
                    flash
                },
                clip_y: clip,
                ground_y: Some(by + 1.0),
                ..Default::default()
            };
            blit(f, s, f.sx(x), f.sy(y), &cl);
        }
        // head over the collar, per warden_layout.json draw_order (a roar's lift bares the neck socket)
        let (head, hx, hy) = self.head_part();
        let hl = Look {
            flash,
            energy_flash: if dying_t.is_some() { flash } else { 0.0 },
            emit: self.eyes,
            clip_y: clip,
            ground_y: Some(by + 1.0),
            reveal,
            ..Default::default()
        };
        blit(f, head, f.sx(hx + trem), f.sy(hy + self.breath()), &hl);
        // heart in the socket
        if !heart_out {
            self.draw_heart(f);
        }

        // fists in front
        for fi in self.fists.iter().filter(|fi| fi.y >= by - 4.0) {
            self.draw_fist(f, fi, dying_t, reveal);
        }
        // pylons in front, and their tethers to the heart
        for p in self.pylons.iter().filter(|p| p.alive && p.y >= by) {
            self.draw_pylon(f, p);
        }
        let hp = self.heart_pos();
        for (i, p) in self.pylons.iter().enumerate().filter(|(_, p)| p.alive) {
            let top = p.top();
            hazards::tether(f, (top.0, top.1), hp, self.t, i as u32 + 3, P2, PW);
        }
        if self.pylons.iter().any(|p| p.alive) && !heart_out {
            barrier(f, hp, self.t);
        }

        // the heart out in front, tethered to the chest
        if heart_out {
            let (sxw, syw) = (bx + l.heart.0 as f32 + trem, by + l.heart.1 as f32 + kneel);
            for k in 0..3u32 {
                let c = [P2, P1, PW][k as usize];
                hazards::tether(f, (sxw, syw), hp, self.t + k * 7, 40 + k, c, PW);
            }
            self.draw_heart(f);
        }

        // death throes: light rays from the heart
        if let Some(t) = dying_t {
            death::rays(f, hp.0, hp.1, t, (t as f32 / death::BURST as f32).min(1.0));
        }

        // airborne: beams and falling shards over everything
        for b in &self.beams {
            b.draw_at_height(f,(by+1.0-b.oy).max(0.0));
        }
        for s in &self.shards {
            s.draw(f);
        }
    }

    /// Return a restrained one-pixel breathing offset.
    fn breath(&self) -> f32 {
        if pose::rig().integrated_head || matches!(self.mode, Mode::Dormant | Mode::Dying(_)) {
            return 0.0;
        }
        if ((self.t as f32) * 0.035).sin() > 0.4 {
            -1.0
        } else {
            0.0
        }
    }

    /// Actual sole ink on the current ground line, shared by load-transfer effects.
    fn support_contacts(&self) -> Vec<(f32, f32)> {
        let body = self.body_sprite();
        let row = body.anchor.1 - self.body_dy.round() as i32;
        let mut contacts = Vec::new();
        for (left, right) in [(0, body.anchor.0), (body.anchor.0, body.cell.w)] {
            // The native left sole ends one pixel above the right sole. Each
            // foot has its own lowest ink row on the common ground plane.
            let xs: Vec<_> = (row-3..=row).rev().find_map(|y| {
                let xs:Vec<_>=(left..right).filter(|&x|body.cell.at(x,y).a).collect();
                (!xs.is_empty()).then_some(xs)
            }).unwrap_or_default();
            if let (Some(&first), Some(&last)) = (xs.first(), xs.last()) {
                contacts.push((
                    self.body.x + (first + last) as f32 * 0.5 - body.anchor.0 as f32,
                    self.body.y,
                ));
            }
        }
        contacts
    }

    /// One continuous masonry assembly through every attack. Native pier folds
    /// keep the feet planted; the same two sluice leaves retract into their
    /// jambs as the heart emerges. Independently generated brace/opening shapes
    /// must not replace the guardian's architecture on a single combat tick.
    fn body_sprite(&self) -> Spr {
        if !material::uses_slate_body() {
            return self.legacy_body_sprite();
        }
        let fold = self.body_dy.round().clamp(0.0, pose::rig().fold_max as f32) as usize;
        let source = spr_frame(art::BODY, &[self.body_fold_anim()], fold)
            .unwrap_or_else(|| spr(art::BODY, &self.body_anims(), self.t, true));
        pose::sluice(source, self.phase >= 2, self.heart.out)
    }

    /// Artist overrides that supply a different masonry family retain their
    /// original pose keys as well as their matching part material.
    fn legacy_body_sprite(&self) -> Spr {
        let fold = self.body_dy.round().clamp(0.0, pose::rig().fold_max as f32) as usize;
        if fold > 0
            && let Some(s) = spr_frame(art::BODY, &[self.body_fold_anim()], fold)
        {
            return s;
        }
        let bracing = matches!(self.mode, Mode::Fight)
            && (self.fists.iter().any(|f| matches!(f.st,
                St::Lock(_) | St::Drop(_) | St::Overhead(_) | St::Slam2(_)
                | St::SwatHold(_) | St::SwatSweep(_)))
                || self.beams.iter().any(|b| b.charging())
                || matches!(self.attack, Some(Attack::Summon { t: 12..=72 })
                    | Some(Attack::Rain { t: 18..=38, .. })));
        let pose = match (self.phase >= 2, self.heart.out > 0.12, bracing) {
            (true, true, _) => Some("damaged_opening"),
            (false, true, _) => Some("opening"),
            (true, false, true) => Some("damaged_brace"),
            (false, false, true) => Some("brace"),
            _ => None,
        };
        pose.and_then(|p| spr_frame(art::BODY, &[p], 0))
            .unwrap_or_else(|| spr(art::BODY, &self.body_anims(), self.t, true))
    }

    /// Shared pose identity for the native torso and its fracture provenance.
    fn body_fold_anim(&self) -> &'static str {
        if !material::uses_slate_body() && self.heart.out > 0.12 {
            return if self.phase >= 2 { "damaged_opening_fold" } else { "opening_fold" };
        }
        if self.phase >= 2 { "damaged_fold" } else { "awake_fold" }
    }

    /// Select intact, awakened or damaged stone for this phase.
    fn body_anims(&self) -> Vec<&'static str> {
        match self.phase {
            0 if matches!(self.mode, Mode::Dormant | Mode::Intro(0..=23)) => vec!["idle", "intact"],
            0 | 1 => vec!["awake", "idle"],
            _ => vec!["damaged", "awake", "idle"],
        }
    }

    /// Select the head’s roaring or resting pose.
    fn head_anims(&self) -> Vec<&'static str> {
        let roaring = self.head_dy < -1.5;
        if roaring {
            vec!["roar", "idle"]
        } else {
            vec!["idle"]
        }
    }

    /// The head frame and its world anchor (no tremble or breath). Roar frames bake in their own
    /// lift, so the runtime `head_dy` only adds whatever lift goes beyond the sheet's peak.
    fn head_part(&self) -> (Spr, f32, f32) {
        static ROAR_PEAK: OnceLock<i32> = OnceLock::new();
        let l = layout();
        let (bx, by) = self.base();
        let head_t = match self.mode {
            Mode::Intro(t) => t.saturating_sub(22),
            Mode::Transition(t) => t.saturating_sub(26),
            Mode::Rouse(t) => t.saturating_sub(8),
            _ => self.t,
        };
        let roaring = self.head_dy < -1.5;
        let head = spr(art::HEAD, &self.head_anims(), head_t, !roaring);
        let dy = if pose::rig().integrated_head {
            // The brow, cheeks and collar are one stone construction. Roaring
            // changes its recessed iris, never lifts a helmet off the torso.
            0.0
        } else if roaring {
            let peak = *ROAR_PEAK.get_or_init(|| {
                art::peak_lift(art::HEAD, "roar", spr(art::HEAD, &["idle"], 0, true))
            });
            (self.head_dy.round() + peak as f32).min(0.0)
        } else {
            self.head_dy.round()
        };
        (
            head,
            bx + l.neck.0 as f32,
            by + l.neck.1 as f32 + self.body_dy.round() + dy,
        )
    }

    fn heart_sprite(&self) -> Spr {
        let anim = if matches!(self.mode, Mode::Dying(t) if t > 150) {
            "shattered"
        } else if self.phase >= 2 || self.heart.hits >= 8 {
            "badly_cracked"
        } else if self.phase >= 1 || self.heart.hits >= 3 {
            "cracked"
        } else {
            "whole"
        };
        spr(art::HEART, &[anim, "whole", "idle", "pulse"], self.t, true)
    }

    /// Animate the heart’s actual damage state and hit recoil.
    fn draw_heart(&self, f: &mut Frame) {
        let (x, y) = self.heart_pos();
        let s = self.heart_sprite();
        let dim = match self.mode {
            Mode::Dormant => 0.25,
            Mode::Intro(t) if t < 6 => 0.25,
            _ => 1.0,
        };
        let pulse = if self.phase == 0 && self.heart.out < 0.1 {
            0.55 + 0.25 * ((self.t as f32) * 0.08).sin()
        } else {
            1.0
        };
        let l = Look {
            emit: (dim * pulse).min(1.0),
            flash: self.heart.flash,
            ground_y: Some(self.base().1 + HEART_OUT.0 * ease_in_out(self.heart.out)),
            ..Default::default()
        };
        blit(f, s, f.sx(x), f.sy(y), &l);
    }

    /// Draw a floating hand with contact-synchronized slam poses and its weak point.
    fn draw_fist(&self, f: &mut Frame, fi: &Fist, dying_t: Option<u32>, reveal: Option<crate::gfx::sprite::SpriteFocus>) {
        if dying_t.map(|t| t > crumble_t(fi.side)).unwrap_or(false) {
            return;
        }
        let flip = fi.side == 0;
        let (x, y) = (fi.x + fi.jit as f32, fi.y - fi.z.max(0.0));
        let (s, lift, back) = fist_pose(fi);
        // smear: falling fists leave two crisp afterimages
        if matches!(fi.st, St::Drop(_) | St::Slam2(_) | St::SwatSweep(_)) {
            for (k, &(tx, ty, tz)) in fi.trail.iter().enumerate().skip(1) {
                let c = [hazards::S3, hazards::S2][(k - 1).min(1)];
                let l = Look {
                    flip,
                    silhouette: Some(c.scale(0.9)),
                    alpha: 0.5 - 0.15 * k as f32,
                    reveal,
                    ..Default::default()
                };
                blit(f, s, f.sx(tx), f.sy(ty - tz.max(0.0)) + lift, &l);
            }
        }
        let dim = matches!(fi.st, St::Dormant | St::Limp(_));
        let throe = dying_t
            .map(|t| ((t as f32 - 60.0) / 50.0).clamp(0.0, 0.8))
            .unwrap_or(0.0);
        let l = Look {
            flip,
            flash: fi.flash.max(throe),
            emit: if dim { 0.3 } else { 1.0 },
            ground_y: Some(fi.y + 2.0),
            reveal,
            ..Default::default()
        };
        blit(f, s, f.sx(x), f.sy(y) + lift, &l);
        // the weak-point crystal on the back of the fist (follows the back through squash and stretch)
        if fi.crystal_hp > 0 {
            let cs = spr(art::CRYSTAL, &["intact", "idle"], fi.t, true);
            let (cx, cy) = (x, y - 22.0 + back as f32);
            let grow = fi.crystal_grow.min(fist::GROW) as f32 / fist::GROW as f32;
            let cs = if fi.crystal_hp < fist::FIST_CRYSTAL_HP {
                spr(art::CRYSTAL, &["cracked", "intact"], fi.t, true)
            } else {
                cs
            };
            if grow < 1.0 {
                // grows back from its base: draw only the bottom part, glowing white
                let (_, oy) = art::origin(cs.cell, cs.anchor, 0, 0, false);
                let shown = (cs.cell.h as f32 * grow).round() as i32;
                let clip_top = f.sy(cy) + oy + cs.cell.h - shown;
                let l = Look {
                    flip,
                    silhouette: Some(PW),
                    ..Default::default()
                };
                blit_from(f, cs, f.sx(cx), f.sy(cy), &l, clip_top);
            } else {
                let emit = if dim { 0.35 } else { 1.0 };
                let l = Look {
                    flip,
                    emit,
                    flash: fi.crystal_flash,
                    ground_y: Some(fi.y + 2.0),
                    ..Default::default()
                };
                blit(f, cs, f.sx(cx), f.sy(cy), &l);
            }
        }
    }

    /// Draw a breakable cluster of three authored prism crystals.
    fn draw_pylon(&self, f: &mut Frame, p: &Pylon) {
        // a cluster of three crystals from the crystal sheet, the middle one tallest
        let anim = if p.hp < hazards::PYLON_HP {
            "cracked"
        } else {
            "intact"
        };
        let s = spr(art::CRYSTAL, &[anim, "intact", "idle"], p.t, true);
        let wob = p.wob.round();
        let (x, y) = (p.x + wob, p.y);
        let l = Look {
            flash: p.flash,
            ground_y: Some(p.y + 2.0),
            ..Default::default()
        };
        let side = Look {
            flip: true,
            flash: p.flash,
            ground_y: Some(p.y + 2.0),
            ..Default::default()
        };
        blit(f, s, f.sx(x - 6.0), f.sy(y + 1.0), &side);
        blit(f, s, f.sx(x + 6.0), f.sy(y + 1.0), &l);
        blit(f, s, f.sx(x), f.sy(y - 4.0), &l);
        // base rubble ring
        let (sx, sy) = (f.sx(p.x), f.sy(p.y));
        for dx in -9..=9 {
            if (dx * 7) % 3 == 0 {
                f.put(sx + dx, sy + 1, hazards::F2);
            }
        }
    }

    /// Cast motivated light from the heart and active weak points.
    fn lights(&self, f: &mut Frame) {
        if f.surface_lighting {
            self.surface_lights(f);
            return;
        }
        let (bx, by) = self.base();
        let gone = matches!(self.mode, Mode::Dying(t) if t > death::BURST);
        if gone {
            if let Mode::Dying(t) = self.mode {
                let s = (t - death::BURST) as f32;
                let k = (1.0 - s / 90.0).max(0.0);
                if t < death::DONE {
                    f.add_light_3d_w(
                        bx,
                        by,
                        by - self.shard_y,
                        14.0 + 14.0 * k,
                        PW,
                        0.3 + 0.25 * k,
                    );
                }
            }
            return;
        }
        let (hx, hy) = self.heart_pos();
        let (r, i) = match self.mode {
            Mode::Dormant => (16.0, 0.12),
            Mode::Stagger(_) => (32.0, 0.4),
            Mode::Dying(t) => (28.0 + t as f32 * 0.08, 0.3 + t as f32 * 0.001),
            _ => [(28.0, 0.25), (28.0, 0.3), (30.0, 0.35)][self.phase],
        };
        // The heart moves forward from its socket while retaining its real airborne height.
        let heart_ground_y = by + HEART_OUT.0 * ease_in_out(self.heart.out);
        f.add_light_3d_w(
            hx,
            heart_ground_y,
            heart_ground_y - hy,
            r,
            HEART_LIGHT,
            i + self.heart.flash * 0.15,
        );
        // Retain the old authored bevel catch in the comparison renderer.
        // Normal-lit geometry receives the real heart source instead.
        if !f.surface_lighting {
            let body = self.body_sprite();
            let (_, by) = self.base();
            let (ox, oy) = (
                bx + self.tremble as f32 - body.anchor.0 as f32,
                by + self.body_dy.round() - body.anchor.1 as f32,
            );
            for dy in -7i32..=7 {
                for dx in -16i32..=16 {
                    let distance = dx.abs() + dy.abs();
                    if dx.abs() < 12 || distance > 20 {
                        continue;
                    }
                    let (wx, wy) = (hx.round() as i32 + dx, hy.round() as i32 + dy);
                    if self.body_dy > 0.5 && wy >= by as i32 {
                        continue;
                    }
                    let (tx, ty) = (wx - ox.round() as i32, wy - oy.round() as i32);
                    let p = body.cell.at(tx, ty);
                    let toward = body.cell.at(tx - dx.signum() * 2, ty - dy.signum() * 2);
                    // Existing lighter bevels facing a recess receive light; ink and flat slabs do not.
                    if !p.a
                        || p.emit
                        || !toward.a
                        || p.c.luma() < 0.28
                        || p.c.luma() - toward.c.luma() < 0.045
                    {
                        continue;
                    }
                    if let Some(j) = Frame::idx(f.sx(wx as f32), f.sy(wy as f32)) {
                        let step = if distance <= 17 { 0.34 } else { 0.18 };
                        f.light[j] += HEART_LIGHT * (step * (i / 0.3).min(1.0));
                    }
                }
            }
        }
        for fi in &self.fists {
            if fi.crystal_alive() && self.dying <= crumble_t(fi.side) {
                let (x, y) = self.fist_crystal_center(fi.side);
                f.add_light_3d_w(x, fi.y, fi.y - y, 12.0, PW, 0.18 + fi.crystal_flash * 0.2);
            }
        }
        for p in self.pylons.iter().filter(|p| p.alive) {
            f.add_light_3d_w(p.x, p.y, 10.0, 16.0, PW, 0.22 + p.flash * 0.15);
        }
        for r in &self.rings {
            r.light(f);
        }
    }

    /// The heart and moving crystals are real finite-height emitters. Their radius must reach
    /// past their own altitude: the former 28px socket source sat 60px above the floor.
    fn surface_lights(&self, f: &mut Frame) {
        let (bx, by) = self.base();
        if let Mode::Dying(t) = self.mode {
            if t > death::BURST {
                if t < death::DONE {
                    let k = (1.0 - (t - death::BURST) as f32 / 90.0).max(0.0);
                    let ground = by + HEART_OUT.0;
                    let height = (ground - self.shard_y).max(0.0);
                    f.add_light_3d_w(bx, self.shard_y + height, height, 60.0 + 24.0 * k, PW, 0.45 + 0.35 * k);
                }
                return;
            }
        }
        let awake = match self.mode {
            Mode::Dormant => 0.0,
            Mode::Intro(t) => (t.saturating_sub(6) as f32 / 70.0).min(1.0),
            _ => 1.0,
        };
        let out = ease_in_out(self.heart.out);
        let (hx, hy) = self.heart_pos();
        let front = self.front_light_plane();
        let radius = 56.0 + awake * (28.0 + 5.0 * self.phase as f32) + 16.0 * out;
        let intensity = 0.16 + awake * (0.62 + 0.08 * self.phase as f32) + 0.35 * out + 0.1 * self.heart.flash;
        native_part_light(f, self.heart_sprite(), hx, hy, false,
            (by + HEART_OUT.0 * out).max(front), 0, false, radius, HEART_LIGHT, intensity);

        // The open socket remains visibly luminous after the heart leaves it. Follow its
        // real kneeling/recoil placement rather than leaving the old light at the closed chest.
        if self.heart.out > 0.02 {
            let socket = layout().heart;
            let x = bx + socket.0 as f32 + self.tremble as f32;
            let y = by + socket.1 as f32 + self.body_dy.round();
            let height = (front - y).max(0.0);
            let radius = if pose::rig().integrated_head { 76.0f32.max(height + 24.0) } else { 76.0 };
            f.add_light_3d_w(x, front, height, radius, HEART_LIGHT, 0.45 * self.heart.out);
        }
        for fi in &self.fists {
            if fi.crystal_hp <= 0 || self.dying > crumble_t(fi.side) { continue; }
            let (_, _, back) = fist_pose(fi);
            let crystal = spr(art::CRYSTAL,
                &[if fi.crystal_hp < fist::FIST_CRYSTAL_HP { "cracked" } else { "intact" }, "intact", "idle"], fi.t, true);
            let grow = fi.crystal_grow.min(fist::GROW) as f32 / fist::GROW as f32;
            let shown = (crystal.cell.h as f32 * grow).round() as i32;
            let dim = if matches!(fi.st, St::Dormant | St::Limp(_)) { 0.3 } else { 1.0 };
            native_part_light(f, crystal, fi.x + fi.jit as f32,
                fi.y - fi.z.max(0.0) - 22.0 + back as f32, fi.side == 0,
                fi.y + 2.0, crystal.cell.h - shown, grow < 1.0,
                64.0, PW, (0.86 + fi.crystal_flash * 0.12) * dim * grow * awake);
        }
        for p in self.pylons.iter().filter(|p| p.alive) {
            let crystal = spr(art::CRYSTAL,
                &[if p.hp < hazards::PYLON_HP { "cracked" } else { "intact" }, "intact", "idle"], p.t, true);
            // One source on the tall central crystal, shared by its two lower companions.
            native_part_light(f, crystal, p.x + p.wob.round(), p.y - 4.0, false,
                p.y + 2.0, 0, false, 60.0, PW, 0.74 + p.flash * 0.1);
        }
        for ring in &self.rings { ring.light(f); }
    }

    /// The socket is mounted on the torso's front, rather than embedded inside its opaque
    /// finite-depth volume. Match the blitter's kneeling cut and native foot extrusion. Moving
    /// Y and Z together leaves the source on exactly the same projected heart/socket ink.
    fn front_light_plane(&self) -> f32 {
        let body = self.body_sprite();
        let sy = (self.body.y + self.body_dy.round()).floor() as i32;
        let top = sy - body.anchor.1;
        let last = if self.body_dy.round() > 0.5 {
            (self.body.y.floor() as i32 - top).min(body.cell.h - 1)
        } else { body.cell.h - 1 };
        let bottom = (0..=last).rev().find(|&y| (0..body.cell.w).any(|x| body.cell.at(x, y).a));
        let native_front = bottom.map_or(self.body.y + 1.0, |y| (top + y + 1) as f32);
        // art::blit lifts a clipped body's native foot to its explicit by+1 ground plane.
        native_front.max(self.body.y + 1.0) + 0.25
    }
}

/// Use a current native luminous pixel, including mirrored/cropped regrowing crystals. The
/// medoid of the emitted patch stays inside the real art rather than floating between parts.
#[allow(clippy::too_many_arguments)]
fn native_part_light(f: &mut Frame, s: Spr, x: f32, y: f32, flip: bool,
    ground_y: f32, min_row: i32, silhouette: bool, radius: f32, color: Rgb, intensity: f32) {
    let emits = |i: usize| {
        let p = s.cell.px[i];
        p.a && (p.emit || silhouette) && i as i32 / s.cell.w >= min_row
    };
    let (mut sum_x, mut sum_y, mut count) = (0i64, 0i64, 0i64);
    for i in 0..s.cell.px.len() {
        if emits(i) {
            sum_x += (i as i32 % s.cell.w) as i64;
            sum_y += (i as i32 / s.cell.w) as i64;
            count += 1;
        }
    }
    if count == 0 { return; }
    let Some(i) = (0..s.cell.px.len()).filter(|&i| emits(i)).min_by_key(|&i| {
        let dx = (i as i32 % s.cell.w) as i64 * count - sum_x;
        let dy = (i as i32 / s.cell.w) as i64 * count - sum_y;
        (dx * dx + dy * dy, i)
    }) else { return };
    let (px, py) = (i as i32 % s.cell.w, i as i32 / s.cell.w);
    let (ox, oy) = art::origin(s.cell, s.anchor, x.floor() as i32, y.floor() as i32, flip);
    let wx = (ox + if flip { s.cell.w - 1 - px } else { px }) as f32 + 0.5;
    let wy = (oy + py) as f32 + 0.5;
    let height = (ground_y - wy).max(0.0);
    let radius = if pose::rig().integrated_head { radius.max(height + 28.0) } else { radius };
    f.add_light_3d_w(wx, wy + height, height, radius, color, intensity);
}

/// Clamp only the cue; leave the real beam/source in world coordinates at native scale.
/// The arrow points toward the actual offscreen emitter. No cue while its flare is readable.
fn attack_cue((x, y): (f32, f32)) -> Option<(i32, i32, i32, i32)> {
    let (left, right, top, bottom) = (
        16.0,
        crate::gfx::W as f32 - 16.0,
        28.0,
        crate::gfx::H as f32 - 38.0,
    );
    if (left..=right).contains(&x) && (top..=bottom).contains(&y) {
        return None;
    }
    let sx = x.clamp(left, right).round() as i32;
    let sy = y.clamp(top, bottom).round() as i32;
    let (dx, dy) = if y < top {
        (0, -1)
    } else if y > bottom {
        (0, 1)
    } else if x < left {
        (-1, 0)
    } else {
        (1, 0)
    };
    Some((sx, sy, dx, dy))
}

/// The `warden_fist` slam frame for a fist state: 0 rise, 1 held wind-up (the lock telegraph),
/// 2 falling, 3 impact from the contact tick for the sheet's impact duration, 4 recover while
/// embedded. None while the fist plays its hover loops.
fn slam_frame(st: St) -> Option<usize> {
    let rise = art::frame_ticks(art::FIST, "slam", 0, 8);
    let impact = art::frame_ticks(art::FIST, "slam", 3, 12);
    match st {
        St::Seek(t) | St::Raise(t) if t < rise => Some(0),
        St::Lock(_) | St::Overhead(_) => Some(1),
        St::Drop(_) | St::Slam2(_) => Some(2),
        St::Stuck(t, _) if t < impact => Some(3),
        St::Stuck(..) => Some(4),
        _ => None,
    }
}

/// A fist's frame, the sheet's baked lift to cancel (+ = draw lower, since runtime height already
/// lifts the fist), and how far its back moved from rest (+ = lower) for the weak-point crystal.
fn fist_pose(fi: &Fist) -> (Spr, i32, i32) {
    let pose: &[&str] = match fi.st {
        St::Limp(_) | St::Dormant => &["open", "clenched"],
        St::Sky(_) => &["open_lit", "open", "clenched"],
        _ => &["clenched", "idle"],
    };
    let Some(s) = slam_frame(fi.st).and_then(|i| spr_frame(art::FIST, &["slam"], i)) else {
        return (spr(art::FIST, pose, fi.t, true), 0, 0);
    };
    let rest = spr_frame(art::FIST, &["slam"], 4).unwrap_or(s);
    let lift = art::baked_lift(s, rest);
    (s, lift, art::solid_top(s) + lift - art::solid_top(rest))
}

/// Return the death tick when this fist turns to rubble.
fn crumble_t(side: usize) -> u32 {
    if side == 0 {
        death::FIST_L_CRUMBLE
    } else {
        death::FIST_R_CRUMBLE
    }
}

/// Like `blit` but skips rows above screen row `top` (crystal regrowth).
fn blit_from(f: &mut Frame, s: Spr, sx: i32, sy: i32, l: &Look, top: i32) {
    let (ox, oy) = art::origin(s.cell, s.anchor, sx, sy, l.flip);
    for y in 0..s.cell.h {
        let py = oy + y;
        if py < top {
            continue;
        }
        for x in 0..s.cell.w {
            let p = s.cell.at(x, y);
            if !p.a {
                continue;
            }
            let px = ox + if l.flip { s.cell.w - 1 - x } else { x };
            let c = l.silhouette.unwrap_or(p.c);
            f.put_glow(px, py, c);
        }
    }
}

/// Stone answers a slash with a clink: a few white-hot sparks off the surface, a deflect ting,
/// a 2-tick freeze. Tells the player "not here".
fn clink(fx: &mut Fx, x: f32, y: f32, ground: f32, hx: f32, hy: f32) {
    let (dx, dy) = crate::util::norm(hx - x, hy - y);
    fx.sound_vol("deflect", 0.55);
    fx.freeze(2);
    let (cx, cy) = (x + dx * 10.0, y + dy * 10.0);
    fx.particles.sparks(
        &mut fx.rng,
        cx,
        ground,
        (ground - cy).max(0.0),
        dx,
        dy,
        5,
        F5,
        2.2,
    );
}

/// Shatter a pylon into its constituent glowing sprite pixels.
fn pylon_burst(fx: &mut Fx, p: &Pylon) {
    let s = spr(art::CRYSTAL, &["intact", "idle"], 0, true);
    for (ox, oy) in [(-6.0, 1.0), (6.0, 1.0), (0.0, -4.0)] {
        death::burst(
            fx,
            s,
            p.x + ox,
            p.y + oy,
            ox < 0.0,
            p.y,
            (p.x, p.y - 10.0),
            1.8,
            true,
            1,
        );
    }
    fx.particles
        .sparks(&mut fx.rng, p.x, p.y, 10.0, 0.0, -1.0, 24, PW, 3.2);
}

/// The open chest: a bright core with short crisp rays of light leaking out.
fn socket_glow(f: &mut Frame, x: f32, y: f32, k: f32, t: u32) {
    let (sx, sy) = (f.sx(x), f.sy(y));
    let r = (2.0 + k * 3.0).round() as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx.abs() + dy.abs() <= r {
                f.put_glow(
                    sx + dx,
                    sy + dy,
                    if dx.abs() + dy.abs() < r - 1 { PW } else { P2 },
                );
            }
        }
    }
    let n = 7;
    for i in 0..n {
        let a = -PI * 0.5 + (i as f32 - 3.0) * 0.42 + ((t as f32) * 0.03 + i as f32).sin() * 0.05;
        let len = (10.0 + 14.0 * ((i * 37 % 11) as f32 / 11.0)) * k;
        let c = death::spectrum(a * 2.0);
        let mut d = r as f32 + 1.0;
        while d < len {
            let (px, py) = (
                sx + (a.cos() * d).round() as i32,
                sy + (a.sin() * d).round() as i32,
            );
            f.put_glow(px, py, if d < len * 0.5 { PW } else { c });
            d += 1.0;
        }
    }
}

/// The prism barrier around the heart while pylons stand: a crisp diamond outline that
/// shimmers (dashes crawl around it).
fn barrier(f: &mut Frame, (x, y): (f32, f32), t: u32) {
    let (sx, sy) = (f.sx(x), f.sy(y));
    let r = 15;
    let mut i = 0;
    for (ax, ay, bx, by) in [(0, -r, r, 0), (r, 0, 0, r), (0, r, -r, 0), (-r, 0, 0, -r)] {
        Frame::line_with(sx + ax, sy + ay, sx + bx, sy + by, |px, py| {
            i += 1;
            let phase = (i + t as i32 / 2) % 8;
            if phase < 5 {
                f.put_glow(
                    px,
                    py,
                    if phase == 0 {
                        PW
                    } else if phase < 3 {
                        P2
                    } else {
                        P1
                    },
                );
            }
        });
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod cohesion_audit;

#[cfg(test)]
mod kneel_tests;

#[cfg(test)]
mod torso_pose_tests {
    use super::*;

    /// A completed kneel must retain whole planted toes, not slice off a
    /// translated standing sprite. Fixed upper joints keep their native ink.
    #[test]
    fn native_torso_folds_keep_feet_and_upper_joints() {
        for phase in [0, 2] {
            for opening in [false, true] {
                let mut b = Boss::new(192.0, 150.0);
                b.phase = phase;
                b.mode = Mode::Stagger(45);
                b.heart.out = if opening { 1.0 } else { 0.0 };
                b.body_dy = 0.0;
                let standing = b.body_sprite();
                for depth in 0..=pose::rig().fold_max {
                    b.body_dy = depth as f32;
                    let folded = b.body_sprite();
                    assert_eq!(folded.anchor, standing.anchor);
                    assert_eq!(art::bottom(folded) + depth, 0, "phase {phase}, opening {opening}, fold {depth}");
                    for y in 0..pose::rig().fixed_upper_rows {
                        for x in 0..standing.cell.w {
                            let (a, p) = (standing.cell.at(x, y), folded.cell.at(x, y));
                            assert_eq!((a.a, a.c), (p.a, p.c));
                        }
                    }
                }
            }
        }
    }

    /// A fist can brace without replacing the supporting masonry. The chest
    /// opens inside the same shell and renderer time cannot change its pose.
    #[test]
    fn native_torso_keys_follow_attack_and_heart_states() {
        let mut b = Boss::new(192.0, 150.0);
        b.mode = Mode::Fight;
        b.body_dy = 0.0;
        b.fists[0].st = St::Lock(4);
        let standing = spr_frame(art::BODY, &["awake_fold"], 0).unwrap();
        assert!(std::ptr::eq(b.body_sprite().cell, standing.cell));
        b.mode = Mode::Stagger(45);
        b.body_dy = kneel_depth();
        b.heart.out = 1.0;
        let open = b.body_sprite();
        assert!(!std::ptr::eq(open.cell, standing.cell));
        b.t += 137;
        assert!(std::ptr::eq(b.body_sprite().cell, open.cell));
    }

    /// Existing growing fracture paths travel with native pier pixels, while a
    /// moving pier hides cracks on the apron beneath it rather than repainting
    /// the fracture onto the foreground joint.
    #[test]
    fn native_torso_fractures_follow_the_articulated_piers() {
        let source = spr_frame(art::BODY, &["awake_fold"], 0).unwrap();
        let rig = pose::rig();
        let depth=rig.fold_max;
        let folded = spr_frame(art::BODY, &["awake_fold"], depth as usize).unwrap();
        for side in [false, true] {
            let y = if rig.articulated_legs {rig.sole_top+2} else {rig.pier_top+10};
            let range = if side { rig.center..source.cell.w } else { 0..rig.center };
            let pixels: Vec<_> = range.filter(|&x| source.cell.at(x, y).a).collect();
            let x = pixels[pixels.len() / 2];
            let spread=if rig.articulated_legs {0} else {depth/8};
            let (dx, dy) = (x + if side { spread } else { -spread }, y-depth);
            assert_eq!(art::fold_crack(source, depth, x, y), Some((dx, dy)));
            let (a, b) = (source.cell.at(x, y), folded.cell.at(dx, dy));
            assert_eq!((a.a, a.c), (b.a, b.c));
            assert_eq!(art::fold_crack(source, depth, dx, dy), None,
                "foreground pier must cover the stationary apron fracture");
        }
        let socket = (source.anchor.0 + layout().heart.0, source.anchor.1 + layout().heart.1);
        assert_eq!(art::fold_crack(source, depth, socket.0, socket.1), Some(socket));
    }

    #[test]
    fn integrated_head_stays_seated_through_breath_and_roar() {
        if !pose::rig().integrated_head { return; }
        let mut b = Boss::new(192.0, 200.0);
        b.mode = Mode::Fight;
        b.body_dy = 0.0;
        b.head_dy = 0.0;
        let (_, x, y) = b.head_part();
        for t in [0, 8, 22, 40, 80, 130] {
            b.t = t;
            for lift in [3.0, 0.0, -2.0, -4.0, -8.0] {
                b.head_dy = lift;
                let (_, hx, hy) = b.head_part();
                assert_eq!((hx, hy, b.breath()), (x, y, 0.0));
            }
        }
        b.body_dy = kneel_depth();
        assert_eq!(b.head_part().2, y + kneel_depth());
        b.body_dy = 0.0;
        let height = b.body.y - b.framing_top().unwrap();
        assert!((160.0..=180.0).contains(&height), "native guardian height: {height}");
    }
}

#[cfg(test)]
mod surface_light_tests {
    use super::*;

    fn light_frame(b: &Boss) -> Frame {
        let mut f = Frame::new();
        f.begin(Rgb::BLACK, Rgb::WHITE);
        f.surface_lighting = true;
        b.lights(&mut f);
        f
    }

    /// Fist light follows the real moving crystal, then disappears with the broken weak point.
    #[test]
    fn physical_warden_fist_lights_move_and_break() {
        let mut b = Boss::new(192.0, 150.0);
        b.mode = Mode::Fight;
        for fist in &mut b.fists { fist.st = St::Rest; }
        let before = light_frame(&b);
        assert_eq!(before.point_lights.len(), 3);
        let old = before.point_lights[1];
        b.fists[0].x += 12.0;
        b.fists[0].z += 20.0;
        let moving = light_frame(&b);
        let new = moving.point_lights[1];
        assert_eq!(new.x - old.x, 12.0);
        assert_eq!(new.y - old.y, -20.0);
        assert_eq!(new.height - old.height, 20.0);
        assert!(new.radius > new.height, "a lowered fist source must reach the floor");
        b.fists[0].crystal_hp = 0;
        assert_eq!(light_frame(&b).point_lights.len(), 2);
        b.fists[1].crystal_hp = 0;
        assert_eq!(light_frame(&b).point_lights.len(), 1);
    }

    /// Opening the chest carries the source from the high socket down to the vulnerable heart;
    /// the visible open socket remains a second source, and no boss light survives its finale.
    #[test]
    fn physical_warden_heart_follows_opening_and_finale() {
        let mut b = Boss::new(192.0, 150.0);
        b.mode = Mode::Fight;
        for fist in &mut b.fists { fist.crystal_hp = 0; }
        let closed = light_frame(&b).point_lights[0];
        assert!(closed.radius > closed.height, "socket light must reach past its altitude");
        b.mode = Mode::Stagger(45);
        b.heart.out = 1.0;
        b.body_dy = kneel_depth();
        let opening = light_frame(&b);
        assert_eq!(opening.point_lights.len(), 2);
        let open = opening.point_lights[0];
        assert!(open.y > closed.y + 40.0);
        assert!(open.height < closed.height - 30.0);
        assert!(open.intensity > closed.intensity);
        assert!(opening.light.iter().all(|c| *c == Rgb::WHITE));
        b.mode = Mode::Dying(death::DONE);
        assert!(light_frame(&b).point_lights.is_empty());
    }

    /// Exercise the kneel/open handoff through real updates. The hold tick cannot briefly
    /// throw the heart and its light fully out before the following tick starts the ramp.
    #[test]
    fn physical_warden_opening_ramp_has_no_early_heart_or_light_reversal() {
        let zone = crate::world::Zone::new(crate::world::ZoneId::Summit, 60, 40);
        for phase in 0..3 {
            let mut b = Boss::new(192.0, 150.0);
            b.phase = phase;
            b.mode = Mode::Stagger(16);
            b.t = 16;
            b.body_dy = kneel_depth();
            b.head_dy = 2.0;
            for fist in &mut b.fists {
                fist.crystal_hp = 0;
                fist.st = St::Limp(16);
            }
            let mut fx = Fx::new(7);
            let (mut hits, mut projectiles) = (Vec::new(), Vec::new());
            let (mut previous_out, mut previous_source_y) = (0.0, None);
            let mut closed_source_y = None;
            let mut closed_socket_y = None;
            let mut first_heart_hurtbox = None;
            for tick in 16..=38 {
                fx.sounds.clear();
                let mut prior_rng = fx.rng.clone();
                let next_random = prior_rng.next_u32();
                b.update(&mut EnemyCtx { player: (192.0, 180.0), player_alive: true,
                    zone: &zone, fx: &mut fx, hits: &mut hits,
                    projectiles: &mut projectiles, t: tick as u64 });
                assert!(b.heart.out >= previous_out,
                    "heart opening reversed at phase {phase} tick {tick}");
                previous_out = b.heart.out;
                let source = light_frame(&b).point_lights[0];
                if tick <= 17 {
                    assert_eq!(b.heart.out, 0.0, "the kneel hold must keep the heart in its socket");
                    let socket_y=source.y-b.body_dy.round();
                    if let Some(y) = closed_socket_y {
                        assert_eq!(socket_y, y, "the heart light left its closed socket during body settlement");
                    } else {
                        closed_socket_y=Some(socket_y);
                        closed_source_y=Some(source.y);
                    }
                }
                if let Some(y) = previous_source_y {
                    // The authored pulse moves its emitting facet by up to two native pixels;
                    // the former false full-out pose reversed by several dozen pixels.
                    assert!(source.y >= y - 3.0,
                        "heart light jumped backward at phase {phase} tick {tick}");
                }
                previous_source_y = Some(source.y);
                if tick == 17 {
                    assert!(fx.sounds.is_empty(), "the hold must add no attack sound");
                    assert_eq!(fx.rng.clone().next_u32(), next_random,
                        "the corrected hold must not consume effect RNG");
                }
                let vulnerable = b.hurtboxes().iter().any(|part| part.3 == PART_HEART);
                if vulnerable && first_heart_hurtbox.is_none() { first_heart_hurtbox = Some(tick + 1); }
                if tick < 29 { assert!(!vulnerable, "the heart became vulnerable before its intended window"); }
            }
            assert_eq!(b.heart.out, 1.0);
            assert_eq!(first_heart_hurtbox, Some(30));
            assert!(previous_source_y.unwrap() > closed_source_y.unwrap() + 40.0,
                "the completed opening must carry the actual light down toward the floor");
        }
    }

    /// Advance the real death clock across the burst. The body and its native shadow must
    /// survive until the same update that emits the flash, collapse particles and burst sound.
    #[test]
    fn physical_warden_body_and_source_swap_on_actual_burst_effects() {
        let mut b = Boss::new(192.0, 150.0);
        b.mode = Mode::Dying(death::BURST - 2);
        b.hp = 0;
        for fist in &mut b.fists { fist.crystal_hp = 0; }
        let zone = crate::world::Zone::new(crate::world::ZoneId::Summit, 60, 40);
        let mut fx = Fx::new(7);
        let (mut hits, mut projectiles) = (Vec::new(), Vec::new());
        let mut burst_seen = false;
        for _ in 0..3 {
            fx.sounds.clear();
            let previous_particles = fx.particles.list.len();
            b.update(&mut EnemyCtx { player: (192.0, 180.0), player_alive: true,
                zone: &zone, fx: &mut fx, hits: &mut hits,
                projectiles: &mut projectiles, t: 0 });
            let burst_now = fx.sounds.iter().any(|sound| sound.name == "boss_die");
            burst_seen |= burst_now;
            if burst_now {
                assert_eq!(fx.flash, 1.0);
                assert_eq!(fx.hitstop, 8);
                assert!(fx.particles.list.len() > previous_particles + 100,
                    "the body swap must coincide with its actual collapse particles");
            }
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::WHITE);
            f.surface_lighting = true;
            let body = b.body_sprite();
            b.draw(&mut f);
            // A kneeling cut deliberately clones the native normal/alpha array. Identify the
            // body by its actual raster placement, then verify its entire visible alpha mask;
            // Arc identity would incorrectly reject that legitimate clipped body caster.
            let body_x = (b.body.x + b.tremble as f32).floor();
            let body_y = (b.body.y + b.body_dy.round()).floor();
            let body_caster = f.surface_casters.iter().find(|caster|
                caster.w == body.cell.w && caster.h == body.cell.h
                    && caster.anchor == body.anchor && caster.x == body_x && caster.y == body_y);
            assert_eq!(body_caster.is_some(), !burst_seen,
                "native body shadow disappeared before its burst effects");
            if let Some(caster) = body_caster {
                let clip = if b.body_dy.round() > 0.5 { b.body.y.floor() as i32 } else { i32::MAX };
                for y in 0..body.cell.h { for x in 0..body.cell.w {
                    let visible = body.cell.at(x, y).a && body_y as i32 - body.anchor.1 + y <= clip;
                    assert_eq!(caster.pixels[(y * body.cell.w + x) as usize].a, visible,
                        "body caster must retain exactly the currently drawn native alpha");
                } }
                assert!(f.surfaces.iter().filter(|surface| surface.owner == caster.owner).count() > 100,
                    "the pre-burst body must still be visibly drawn, not just a retained caster");
            }
            assert_eq!(f.point_lights.len(), 1);
            let light = f.point_lights[0];
            if burst_seen {
                assert_eq!((light.x, light.y), (b.body.x, b.shard_y),
                    "the surviving shard source must replace the heart on the burst frame");
            } else {
                assert_ne!((light.x, light.y), (b.body.x, b.shard_y),
                    "the visible heart must retain its native source before the burst");
            }
        }
        assert!(burst_seen, "fixture never advanced through the real burst update");
    }

    /// One light follows the central pylon's actual recoil; dead pylons stop illuminating.
    #[test]
    fn physical_warden_pylon_light_follows_recoil_and_removal() {
        let mut b = Boss::new(192.0, 150.0);
        b.mode = Mode::Fight;
        for fist in &mut b.fists { fist.crystal_hp = 0; }
        b.pylons.push(Pylon::new(240.0, 185.0));
        let before = light_frame(&b).point_lights[1];
        b.pylons[0].wob = -3.0;
        let recoil = light_frame(&b).point_lights[1];
        assert_eq!(recoil.x - before.x, -3.0);
        assert_eq!(recoil.y, before.y);
        assert!(recoil.radius > recoil.height);
        b.pylons[0].alive = false;
        assert_eq!(light_frame(&b).point_lights.len(), 1);
    }

    /// Exercise real native torso occlusion: an embedded source was blocked everywhere on
    /// the floor even though it existed and its receiving body owner was exempted correctly.
    #[test]
    fn physical_warden_closed_heart_reaches_front_floor_but_still_casts_body_shadow() {
        let mut b = Boss::new(192.0, 150.0);
        b.mode = Mode::Fight;
        b.body_dy = 2.0;
        for fist in &mut b.fists { fist.crystal_hp = 0; }
        let mut f = Frame::new();
        f.begin(Rgb::gray(0.5), Rgb::gray(0.2));
        f.surface_lighting = true;
        f.surfaces.fill(crate::gfx::frame::Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 });
        b.draw(&mut f);
        let light = f.point_lights[0];
        let front = 174 * crate::gfx::W + 192;
        assert_eq!(f.surfaces[front].height, 0.0, "front probe must be visible floor");
        assert_eq!(crate::gfx::lighting::point_visibility(&f, light)[front], 255,
            "the real front-facing heart must reach the floor");
        // A ground probe under the rear projection deliberately replaces its receiver only;
        // every actual native body caster remains. The correction cannot disable body shadows.
        let rear = 128 * crate::gfx::W + 192;
        f.surfaces[rear] = crate::gfx::frame::Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 };
        assert_eq!(crate::gfx::lighting::point_visibility(&f, light)[rear], 0);
        f.point_lights.truncate(1);
        crate::gfx::surface::apply(&mut f);
        assert!(f.light[front].r > 0.23, "closed heart source must visibly illuminate actual floor");
    }

    /// Run manually on a quiet CPU. This includes the real Summit terrain, native Warden,
    /// player, live hazard updates/particles, camera, HUD and final grade during firing frames.
    #[test]
    #[ignore = "manual full-Play Warden firing timing; run alone with --nocapture"]
    fn physical_warden_full_play_firing_timing() {
        use crate::fx::Fx;
        use crate::input::{Input, Raw};
        use crate::play::{Play, PlayOpts};
        use crate::world::ZoneId;
        use std::time::Instant;
        let mut results = Vec::new();
        for (phase, twin) in [(1, false), (2, true)] {
            let mut play = Play::new(ZoneId::Summit, &PlayOpts { god: true, peaceful: false });
            let (bx, by) = play.zone.boss_pos.unwrap();
            play.player.respawn(bx, by + 72.0);
            play.boss_awake = true;
            play.arrive_t = 10_000;
            let boss = play.boss.as_mut().unwrap();
            boss.phase = phase;
            boss.hp = boss.top_hp();
            boss.body.hp = boss.hp;
            boss.mode = Mode::Fight;
            boss.body_dy = 0.0;
            boss.eyes = 1.0;
            for side in 0..2 {
                let (x, y, z) = boss.rest_pos(side);
                boss.fists[side].x = x;
                boss.fists[side].y = y;
                boss.fists[side].z = z;
                boss.fists[side].st = St::Rest;
            }
            boss.attack = Some(if phase == 1 { Attack::Summon { t: 0 } } else { Attack::Beam { t: 0, twin } });
            let mut fx = Fx::new(7);
            let mut input = Input::default();
            // Keep phase two's actual shield placement and landing lifecycle, including the
            // real Summit collision search, rather than measuring invented pylon positions.
            if phase == 1 {
                for _ in 0..100 {
                    input.step(Raw::default());
                    play.update(&input, &mut fx);
                    fx.sounds.clear();
                    fx.lights.clear();
                    fx.tick();
                }
                let boss = play.boss.as_mut().unwrap();
                assert_eq!(boss.pylons.iter().filter(|p| p.alive).count(), 3);
                boss.attack = Some(Attack::Beam { t: 0, twin });
            }
            let mut frame = Frame::new();
            let hud = crate::ui::Hud::new();
            let mut times = Vec::new();
            let (mut max_beams, mut max_points, mut max_casters) = (0, 0, 0);
            for tick in 0..400 {
                input.step(Raw::default());
                play.update(&input, &mut fx);
                let active = play.boss.as_ref().unwrap().beams.iter().filter(|b| b.firing()).count();
                play.cam.snap(bx, by - 22.0);
                let start = Instant::now();
                play.render(&mut frame, &fx);
                crate::ui::hud(&mut frame, &play, &hud);
                crate::gfx::post::finish(&mut frame, &play.zone.grade);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                if active > 0 {
                    times.push(elapsed);
                    max_beams = max_beams.max(active);
                    max_points = max_points.max(frame.point_lights.len());
                    max_casters = max_casters.max(frame.surface_casters.len());
                }
                fx.sounds.clear();
                fx.lights.clear();
                fx.tick();
                if tick > 120 && !times.is_empty() && play.boss.as_ref().unwrap().beams.is_empty() { break; }
            }
            assert!(times.len() >= 80, "did not measure a full real beam sweep");
            assert_eq!(max_beams, if twin { 2 } else { 1 });
            times.sort_by(f64::total_cmp);
            results.push(serde_json::json!({"phase":phase + 1,"twin":twin,"frames":times.len(),
                "mean_ms":times.iter().sum::<f64>() / times.len() as f64,
                "median_ms":times[times.len()/2],"p95_ms":times[times.len()*95/100],
                "max_ms":times[times.len()-1],"active_beams":max_beams,
                "max_point_lights":max_points,"max_native_casters":max_casters}));
        }
        println!("WARDEN_RENDER {}", serde_json::to_string(&results).unwrap());
    }
}
