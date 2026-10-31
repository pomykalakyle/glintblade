//! The effects bus: anything in the game can push particles, lights, sounds, screen shake,
//! hit-stop and flashes through `Fx`. The scene owns one `Fx` and passes `&mut Fx` around.

pub mod particles;

use crate::gfx::Rgb;
use crate::util::Rng;
pub use particles::{Debris, Particle, Particles};

/// A point light in WORLD coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: Rgb,
    pub intensity: f32,
}

#[derive(Clone, Debug)]
pub struct SoundCmd {
    /// File stem under assets/sfx (e.g. "slash_1").
    pub name: String,
    pub volume: f32,
}

pub struct Fx {
    pub rng: Rng,
    pub particles: Particles,
    /// Dynamic lights for the current frame. Cleared after each render.
    pub lights: Vec<Light>,
    pub sounds: Vec<SoundCmd>,
    /// Music track the game wants (file stem under assets/music). The frontend crossfades to it.
    pub music: Option<String>,
    /// Screen shake trauma 0..1 (decays). Shake amount grows with trauma squared.
    pub trauma: f32,
    /// Hit-stop: while > 0, gameplay freezes (rendering continues).
    pub hitstop: u32,
    /// Full-screen white flash 0..1 (decays).
    pub flash: f32,
    /// Ticks since start.
    pub t: u64,
    next_hit: u32,
}

impl Fx {
    pub fn new(seed: u64) -> Fx {
        Fx {
            rng: Rng::new(seed),
            particles: Particles::default(),
            lights: Vec::new(),
            sounds: Vec::new(),
            music: None,
            trauma: 0.0,
            hitstop: 0,
            flash: 0.0,
            t: 0,
            next_hit: 1,
        }
    }

    pub fn sound(&mut self, name: &str) {
        self.sound_vol(name, 1.0);
    }
    pub fn sound_vol(&mut self, name: &str, volume: f32) {
        self.sounds.push(SoundCmd { name: name.to_string(), volume });
    }
    /// Play one of `{base}_1` .. `{base}_{n}` at random. Falls back to `base` if the variants
    /// don't exist (the frontend tries both).
    pub fn sound_var(&mut self, base: &str, n: u32) {
        let i = self.rng.irange(1, n as i32 + 1);
        self.sounds.push(SoundCmd { name: format!("{base}_{i}"), volume: 1.0 });
    }

    pub fn music(&mut self, track: &str) {
        self.music = Some(track.to_string());
    }

    /// Add screen shake (0.2 small hit, 0.5 heavy, 1.0 huge).
    pub fn shake(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    /// Freeze gameplay for `ticks` (hit-stop). Doesn't stack; takes the max.
    pub fn freeze(&mut self, ticks: u32) {
        self.hitstop = self.hitstop.max(ticks);
    }

    pub fn flash(&mut self, amount: f32) {
        self.flash = self.flash.max(amount);
    }

    /// Add a light for this frame (world coords).
    pub fn light(&mut self, x: f32, y: f32, radius: f32, color: Rgb, intensity: f32) {
        self.lights.push(Light { x, y, radius, color, intensity });
    }

    /// A fresh id for one attack swing / projectile, so each target is hit at most once by it.
    pub fn hit_id(&mut self) -> u32 {
        self.next_hit = self.next_hit.wrapping_add(1).max(1);
        self.next_hit
    }

    /// Per-tick decay of shake/flash. Called by the scene.
    pub fn tick(&mut self) {
        self.t += 1;
        self.trauma = (self.trauma - 0.025).max(0.0);
        self.flash = (self.flash - 0.08).max(0.0);
    }
}
