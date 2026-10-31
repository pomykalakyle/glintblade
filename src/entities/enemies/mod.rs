//! Enemies. Each kind lives in its own file with the same small interface:
//!
//! ```ignore
//! pub struct State { ... }                                   // kind-specific state
//! pub fn spawn(x: f32, y: f32) -> (Body, State)
//! pub fn update(b: &mut Body, s: &mut State, ctx: &mut EnemyCtx)
//! pub fn draw(b: &Body, s: &State, f: &mut Frame)
//! pub fn on_hit(b: &mut Body, s: &mut State, hit: &Hit, ctx: &mut EnemyCtx)  // after damage
//! pub fn on_death(b: &Body, s: &State, ctx: &mut EnemyCtx)                    // burst, drops
//! ```
//!
//! This file only dispatches and does what every enemy shares (hit sparks, sounds, death).

pub mod basic;
pub mod crystal_beetle;
pub mod ember_hound;
pub mod spitting_totem;
pub mod sproutling;
pub mod thorn_imp;
pub mod wisp;

use super::{Body, Projectile};
use crate::combat::Hit;
use crate::fx::Fx;
use crate::gfx::{Frame, rgb};
use crate::world::Zone;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Sproutling,
    ThornImp,
    CrystalBeetle,
    Wisp,
    EmberHound,
    SpittingTotem,
}

impl EnemyKind {
    pub fn parse(s: &str) -> Option<EnemyKind> {
        Some(match s {
            "sproutling" => EnemyKind::Sproutling,
            "thorn_imp" => EnemyKind::ThornImp,
            "crystal_beetle" => EnemyKind::CrystalBeetle,
            "wisp" => EnemyKind::Wisp,
            "ember_hound" => EnemyKind::EmberHound,
            "spitting_totem" => EnemyKind::SpittingTotem,
            _ => return None,
        })
    }
}

pub enum Brain {
    Sproutling(sproutling::State),
    ThornImp(thorn_imp::State),
    CrystalBeetle(crystal_beetle::State),
    Wisp(wisp::State),
    EmberHound(ember_hound::State),
    SpittingTotem(spitting_totem::State),
}

/// What an enemy can see and do during its update.
pub struct EnemyCtx<'a> {
    /// Player feet position.
    pub player: (f32, f32),
    pub player_alive: bool,
    pub zone: &'a Zone,
    pub fx: &'a mut Fx,
    /// Push hits (contact damage, lunges, shockwaves) here.
    pub hits: &'a mut Vec<Hit>,
    /// Push projectiles here.
    pub projectiles: &'a mut Vec<Projectile>,
    pub t: u64,
}

pub struct Enemy {
    pub kind: EnemyKind,
    pub body: Body,
    pub brain: Brain,
    /// Has it noticed the player yet? (Enemies idle until the player comes near.)
    pub awake: bool,
}

impl Enemy {
    pub fn spawn(kind: EnemyKind, x: f32, y: f32) -> Enemy {
        let (body, brain) = match kind {
            EnemyKind::Sproutling => {
                let (b, s) = sproutling::spawn(x, y);
                (b, Brain::Sproutling(s))
            }
            EnemyKind::ThornImp => {
                let (b, s) = thorn_imp::spawn(x, y);
                (b, Brain::ThornImp(s))
            }
            EnemyKind::CrystalBeetle => {
                let (b, s) = crystal_beetle::spawn(x, y);
                (b, Brain::CrystalBeetle(s))
            }
            EnemyKind::Wisp => {
                let (b, s) = wisp::spawn(x, y);
                (b, Brain::Wisp(s))
            }
            EnemyKind::EmberHound => {
                let (b, s) = ember_hound::spawn(x, y);
                (b, Brain::EmberHound(s))
            }
            EnemyKind::SpittingTotem => {
                let (b, s) = spitting_totem::spawn(x, y);
                (b, Brain::SpittingTotem(s))
            }
        };
        Enemy { kind, body, brain, awake: false }
    }

    pub fn update(&mut self, ctx: &mut EnemyCtx) {
        if !self.body.alive {
            return;
        }
        if !self.awake {
            let d = self.body.dist_to(ctx.player.0, ctx.player.1);
            if d < 150.0 && ctx.zone.line_clear(self.body.x, self.body.y - 4.0, ctx.player.0, ctx.player.1 - 4.0) || self.body.hp < self.body.max_hp {
                self.awake = true;
            } else {
                self.body.integrate(ctx.zone);
                return;
            }
        }
        let b = &mut self.body;
        match &mut self.brain {
            Brain::Sproutling(s) => sproutling::update(b, s, ctx),
            Brain::ThornImp(s) => thorn_imp::update(b, s, ctx),
            Brain::CrystalBeetle(s) => crystal_beetle::update(b, s, ctx),
            Brain::Wisp(s) => wisp::update(b, s, ctx),
            Brain::EmberHound(s) => ember_hound::update(b, s, ctx),
            Brain::SpittingTotem(s) => spitting_totem::update(b, s, ctx),
        }
    }

    /// Apply a player hit. Returns true if it landed.
    pub fn on_hit(&mut self, hit: &Hit, ctx: &mut EnemyCtx) -> bool {
        if !self.body.alive || !self.body.take(hit) {
            return false;
        }
        self.awake = true;
        let fx = &mut *ctx.fx;
        fx.sound_var("hit", 3);
        fx.freeze(if hit.heavy { 6 } else { 3 });
        fx.shake(if hit.heavy { 0.28 } else { 0.14 });
        let (dx, dy) = hit.push_dir(self.body.x, self.body.y);
        fx.particles.sparks(&mut fx.rng, self.body.x, self.body.y, 6.0, dx, dy, if hit.heavy { 14 } else { 8 }, rgb(0x9ff8ff), 2.6);
        let b = &mut self.body;
        match &mut self.brain {
            Brain::Sproutling(s) => sproutling::on_hit(b, s, hit, ctx),
            Brain::ThornImp(s) => thorn_imp::on_hit(b, s, hit, ctx),
            Brain::CrystalBeetle(s) => crystal_beetle::on_hit(b, s, hit, ctx),
            Brain::Wisp(s) => wisp::on_hit(b, s, hit, ctx),
            Brain::EmberHound(s) => ember_hound::on_hit(b, s, hit, ctx),
            Brain::SpittingTotem(s) => spitting_totem::on_hit(b, s, hit, ctx),
        }
        if self.body.hp <= 0 {
            self.body.alive = false;
            ctx.fx.sound("enemy_die");
            ctx.fx.freeze(5);
            let b = &self.body;
            match &self.brain {
                Brain::Sproutling(s) => sproutling::on_death(b, s, ctx),
                Brain::ThornImp(s) => thorn_imp::on_death(b, s, ctx),
                Brain::CrystalBeetle(s) => crystal_beetle::on_death(b, s, ctx),
                Brain::Wisp(s) => wisp::on_death(b, s, ctx),
                Brain::EmberHound(s) => ember_hound::on_death(b, s, ctx),
                Brain::SpittingTotem(s) => spitting_totem::on_death(b, s, ctx),
            }
        }
        true
    }

    pub fn draw(&self, f: &mut Frame) {
        if !self.body.alive {
            return;
        }
        let b = &self.body;
        match &self.brain {
            Brain::Sproutling(s) => sproutling::draw(b, s, f),
            Brain::ThornImp(s) => thorn_imp::draw(b, s, f),
            Brain::CrystalBeetle(s) => crystal_beetle::draw(b, s, f),
            Brain::Wisp(s) => wisp::draw(b, s, f),
            Brain::EmberHound(s) => ember_hound::draw(b, s, f),
            Brain::SpittingTotem(s) => spitting_totem::draw(b, s, f),
        }
    }
}
