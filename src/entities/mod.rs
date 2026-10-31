//! Characters: the player, enemies, projectiles and the Warden boss share `Body`.

pub mod boss;
pub mod enemies;
pub mod player;
pub mod projectiles;

use crate::combat::Hit;
use crate::world::Zone;
pub use enemies::{Enemy, EnemyCtx, EnemyKind};
pub use player::Player;
pub use projectiles::Projectile;

/// Physical state shared by every character.
#[derive(Clone, Debug)]
pub struct Body {
    pub x: f32,
    pub y: f32,
    /// Height above ground (hops, knock-ups). Drawn at y - z.
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    /// Knockback velocity (decays separately from movement).
    pub kx: f32,
    pub ky: f32,
    /// Collision half-width of the feet box; also the hurt radius.
    pub r: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    /// Hit flash 0..1.
    pub flash: f32,
    /// Ticks of hit-stun left (no actions).
    pub stun: u32,
    /// Ticks of invulnerability left.
    pub iframes: u32,
    /// Facing direction (unit vector).
    pub face: (f32, f32),
    recent_hits: [u32; 6],
}

impl Body {
    pub fn new(x: f32, y: f32, r: f32, hp: i32) -> Body {
        Body {
            x,
            y,
            z: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            kx: 0.0,
            ky: 0.0,
            r,
            hp,
            max_hp: hp,
            alive: true,
            flash: 0.0,
            stun: 0,
            iframes: 0,
            face: (0.0, 1.0),
            recent_hits: [0; 6],
        }
    }

    /// Apply a hit if it's new and we're vulnerable. Returns true if it landed.
    /// Applies damage, flash, knockback and stun; death is left to the caller (`hp <= 0`).
    pub fn take(&mut self, hit: &Hit) -> bool {
        if !self.alive || self.iframes > 0 || self.recent_hits.contains(&hit.id) {
            return false;
        }
        self.recent_hits.rotate_right(1);
        self.recent_hits[0] = hit.id;
        self.hp -= hit.dmg;
        self.flash = 1.0;
        let (dx, dy) = hit.push_dir(self.x, self.y);
        self.kx = dx * hit.knock;
        self.ky = dy * hit.knock;
        self.stun = if hit.heavy { 22 } else { 12 };
        true
    }

    /// Does the hit's shape touch this body?
    pub fn touched_by(&self, hit: &Hit) -> bool {
        hit.shape.touches(self.x, self.y - self.r * 0.5, self.r)
    }

    /// Move by velocity + knockback with wall sliding; decay knockback, flash and timers.
    pub fn integrate(&mut self, zone: &Zone) -> bool {
        let (nx, ny, hit) = zone.move_box(self.x, self.y, self.r, self.vx + self.kx, self.vy + self.ky);
        self.x = nx;
        self.y = ny;
        self.kx *= 0.82;
        self.ky *= 0.82;
        if self.kx.abs() < 0.02 {
            self.kx = 0.0;
        }
        if self.ky.abs() < 0.02 {
            self.ky = 0.0;
        }
        if self.z > 0.0 || self.vz != 0.0 {
            self.vz -= 0.25;
            self.z += self.vz;
            if self.z <= 0.0 {
                self.z = 0.0;
                self.vz = 0.0;
            }
        }
        self.flash = (self.flash - 0.12).max(0.0);
        self.stun = self.stun.saturating_sub(1);
        self.iframes = self.iframes.saturating_sub(1);
        hit
    }

    pub fn dist_to(&self, x: f32, y: f32) -> f32 {
        ((self.x - x).powi(2) + (self.y - y).powi(2)).sqrt()
    }

    pub fn dir_to(&self, x: f32, y: f32) -> (f32, f32) {
        crate::util::norm(x - self.x, y - self.y)
    }
}
