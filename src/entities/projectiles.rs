//! Projectiles (seeds, wisp bolts, ember globs...). Generic: enemies spawn them with a look
//! (sheet/anim or a glowing dot) and the engine moves them, hits the player, and pops them
//! on walls. Player slashes destroy enemy projectiles.

use crate::combat::{Hit, Shape, Team};
use crate::fx::{Fx, Particle};
use crate::gfx::{DrawOpts, Frame, Rgb, sheet};
use crate::world::Zone;

#[derive(Clone, Debug)]
pub struct Projectile {
    pub x: f32,
    pub y: f32,
    /// Height above ground (drawn at y - z). Lobbed shots use vz/grav.
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    /// Gravity on z per tick (0 = flies level). Lobbed shots land and pop when z hits 0.
    pub grav: f32,
    pub r: f32,
    pub team: Team,
    pub dmg: i32,
    /// Ticks left.
    pub life: u32,
    pub sheet: &'static str,
    pub anim: &'static str,
    /// Main color: trail, light, pop burst, and the fallback dot.
    pub color: Rgb,
    /// Radius of the light it casts (0 = none).
    pub light: f32,
    /// Leave a trail of glowing pixels.
    pub trail: bool,
    /// Turn toward the player by this many radians per tick (0 = straight).
    pub homing: f32,
    pub id: u32,
    pub alive: bool,
    pub t: u32,
}

impl Projectile {
    /// A level-flying enemy shot from (x, y) with velocity (vx, vy).
    pub fn new(x: f32, y: f32, vx: f32, vy: f32, fx: &mut Fx) -> Projectile {
        Projectile {
            x,
            y,
            z: 6.0,
            vx,
            vy,
            vz: 0.0,
            grav: 0.0,
            r: 3.0,
            team: Team::Enemy,
            dmg: 1,
            life: 240,
            sheet: "",
            anim: "",
            color: Rgb::new(1.0, 0.6, 0.3),
            light: 24.0,
            trail: true,
            homing: 0.0,
            id: fx.hit_id(),
            alive: true,
            t: 0,
        }
    }
    pub fn look(mut self, sheet: &'static str, anim: &'static str) -> Self {
        self.sheet = sheet;
        self.anim = anim;
        self
    }
    pub fn color(mut self, c: Rgb) -> Self {
        self.color = c;
        self
    }
    pub fn z(mut self, z: f32, vz: f32, grav: f32) -> Self {
        self.z = z;
        self.vz = vz;
        self.grav = grav;
        self
    }

    fn pop(&mut self, fx: &mut Fx) {
        self.alive = false;
        if self.light == 0.0 && !self.trail {
            fx.particles.chips(&mut fx.rng, self.x, self.y, self.z, -self.vx, -self.vy, 8, self.color, 1.8);
        } else {
            fx.particles.sparks(&mut fx.rng, self.x, self.y, self.z, -self.vx, -self.vy, 8, self.color, 1.8);
        }
    }

    /// Move one tick; returns the hit this projectile deals this tick (if any).
    pub fn update(&mut self, zone: &Zone, fx: &mut Fx, player: (f32, f32)) -> Option<Hit> {
        if !self.alive {
            return None;
        }
        self.t += 1;
        if self.life == 0 {
            self.pop(fx);
            return None;
        }
        self.life -= 1;
        if self.homing > 0.0 {
            let want = (player.1 - self.y).atan2(player.0 - self.x);
            let cur = self.vy.atan2(self.vx);
            let sp = (self.vx * self.vx + self.vy * self.vy).sqrt();
            let d = crate::util::angle_diff(cur, want).clamp(-self.homing, self.homing);
            let a = cur + d;
            self.vx = a.cos() * sp;
            self.vy = a.sin() * sp;
        }
        self.x += self.vx;
        self.y += self.vy;
        if self.grav != 0.0 {
            self.vz -= self.grav;
            self.z += self.vz;
            if self.z <= 0.0 {
                self.z = 0.0;
                self.pop(fx);
                fx.sound_vol("pot_break", 0.4);
                return Some(Hit::new(Shape::Circle { x: self.x, y: self.y, r: self.r + 6.0 }, self.team, self.dmg, self.id).knock(2.0));
            }
        }
        if zone.tile_at(self.x, self.y) == crate::world::Tile::Wall || zone.blocked(self.x, self.y, 1.0) && self.z < 8.0 {
            self.pop(fx);
            return None;
        }
        if self.trail && self.t % 2 == 0 {
            fx.particles.add(
                Particle::new(self.x, self.y)
                    .z(self.z)
                    .color(self.color)
                    .fade_to(self.color.scale(0.3))
                    .glow(1.0)
                    .grav(0.0)
                    .life(12.0),
            );
        }
        if self.light > 0.0 {
            fx.light(self.x, self.y - self.z, self.light, self.color, 0.7);
        }
        if self.grav == 0.0 {
            Some(Hit::new(Shape::Circle { x: self.x, y: self.y, r: self.r }, self.team, self.dmg, self.id).knock(2.2).dir(self.vx, self.vy))
        } else {
            None
        }
    }

    /// Destroyed by a player slash.
    pub fn deflect(&mut self, fx: &mut Fx) {
        self.pop(fx);
        fx.sound_vol("hit_1", 0.5);
    }

    pub fn draw(&self, f: &mut Frame) {
        if !self.alive {
            return;
        }
        // shadow for lobbed shots
        if self.z > 2.0 {
            f.shadow(f.sx(self.x), f.sy(self.y), 2, 1, 0.35);
        }
        if !self.sheet.is_empty() && f.anim_w(self.sheet, self.anim, self.t, self.x, self.y - self.z, &DrawOpts::default()) {
            return;
        }
        let _ = sheet;
        let (sx, sy) = (f.sx(self.x), f.sy(self.y - self.z));
        f.circle_glow(sx, sy, 2, self.color);
        f.put_glow(sx, sy, Rgb::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Input, Raw};
    use crate::play::{Play, PlayOpts};
    use crate::world::{NoRegion, Tile, ZoneId};

    /// A bolt cut by the sword cannot deal a later hit in the same scene tick.
    #[test]
    fn slashed_bolt_cannot_damage_player_later_in_the_same_tick() {
        let mut play = Play::new(ZoneId::Bloomwild, &PlayOpts { god: false, peaceful: true });
        play.zone = Zone::new(ZoneId::Bloomwild, 40, 30);
        for y in 0..30 {
            for x in 0..40 {
                play.zone.set_tile(x, y, Tile::Floor);
            }
        }
        play.region = Box::new(NoRegion);
        play.player = crate::entities::Player::new(200.0, 200.0);
        let mut fx = Fx::new(7);
        let mut input = Input::default();
        input.step(Raw { attack: true, ..Raw::default() });
        play.update(&input, &mut fx);
        for _ in 0..3 {
            input.step(Raw::default());
            play.update(&input, &mut fx);
        }
        let hp = play.player.body.hp;
        let (x, y) = (play.player.body.x, play.player.body.y);
        play.projectiles.push(Projectile::new(x, y + 12.0, 0.0, -12.0, &mut fx));
        input.step(Raw::default());
        play.update(&input, &mut fx);
        assert!(play.projectiles.is_empty(), "the active sword must destroy the incoming bolt");
        assert!(fx.sounds.iter().any(|sound| sound.name == "hit_1"), "the sword must actually deflect the bolt");
        assert_eq!(play.player.body.hp, hp, "a destroyed bolt must not damage Glint");
    }
}
