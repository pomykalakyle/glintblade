//! Ambient life in Glasswater Hollow, kept to the region's signature: drips falling from the
//! unseen ceiling into the pools on a slow beat (each one rings the water), and the abyss
//! under the rift.

use super::art::{ROCK, WATER};
use super::water::Water;
use crate::fx::{Fx, Particle};
use crate::gfx::{Frame, H, W};
use crate::util::Rng;
use crate::world::{RegionCtx, Zone};

struct Drip {
    x: f32,
    y: f32,
    z: f32,
    vz: f32,
}

/// A spot on the ceiling that lets a drop go every `period` ticks.
pub struct DripPoint {
    pub x: f32,
    pub y: f32,
    pub period: u64,
    pub offset: u64,
}

pub struct Life {
    rng: Rng,
    drips: Vec<Drip>,
    pub drip_points: Vec<DripPoint>,
}

impl Life {
    pub fn new() -> Life {
        Life { rng: Rng::new(77), drips: Vec::new(), drip_points: Vec::new() }
    }

    pub fn update(&mut self, ctx: &RegionCtx, fx: &mut Fx, water: &mut Water) {
        let (cx, cy) = (ctx.cam.0 as f32, ctx.cam.1 as f32);
        let on_screen = |x: f32, y: f32, m: f32| x > cx - m && x < cx + W as f32 + m && y > cy - m && y < cy + H as f32 + m;
        for p in &self.drip_points {
            if ctx.t % p.period == p.offset % p.period && on_screen(p.x, p.y, 40.0) {
                self.drips.push(Drip { x: p.x, y: p.y, z: 110.0, vz: -1.0 });
            }
        }
        let mut landed = Vec::new();
        self.drips.retain_mut(|d| {
            d.vz -= 0.16;
            d.z += d.vz;
            if d.z <= 0.0 {
                landed.push((d.x, d.y));
                return false;
            }
            true
        });
        let player = ctx.player;
        for (x, y) in landed {
            let wet = water.is_water(x as i32, y as i32);
            if wet {
                water.poke(x, y, 2.5, 0.85);
            }
            // a tiny crown of droplets
            for k in 0..(if wet { 4 } else { 3 }) {
                let a = k as f32 * 1.57 + self.rng.range(0.0, 0.8);
                fx.particles.add(Particle::new(x, y).vel(a.cos() * 0.3, a.sin() * 0.15, self.rng.range(0.9, 1.5)).color(WATER[4]).fade_to(WATER[3]).life(self.rng.range(12.0, 18.0)).grav(0.15));
            }
            let d2 = (x - player.0).powi(2) + (y - player.1).powi(2);
            if d2 < 220.0 * 220.0 {
                let vol = 0.2 + 0.45 * (1.0 - d2.sqrt() / 220.0);
                let i = 1 + (self.rng.next_u32() % 3);
                fx.sound_vol(&format!("drip_{i}"), vol);
            }
        }
    }

    pub fn draw_overlay(&self, f: &mut Frame) {
        for d in &self.drips {
            let sx = f.sx(d.x);
            let sy = f.sy(d.y - d.z);
            f.put(sx, sy, WATER[5]);
            f.put(sx, sy - 1, WATER[4]);
        }
    }
}

/// Unlit depth below the rift; the bridge and its visible crystals carry the composition.
pub fn draw_abyss(zone: &Zone, f: &mut Frame) {
    let base = ROCK[0].scale(0.35);
    for sy in 0..H as i32 {
        let wy = sy + f.cam_y;
        for sx in 0..W as i32 {
            let wx = sx + f.cam_x;
            let inside = wx >= 0 && wy >= 0 && wx < zone.pw && wy < zone.ph;
            if !inside || zone.ground[(wy * zone.pw + wx) as usize].r < 0.0 {
                f.albedo[sy as usize * W + sx as usize] = base;
            }
        }
    }
}
