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
        Life {
            rng: Rng::new(77),
            drips: Vec::new(),
            drip_points: Vec::new(),
        }
    }

    pub fn update(&mut self, ctx: &RegionCtx, fx: &mut Fx, water: &mut Water) {
        let (cx, cy) = (ctx.cam.0 as f32, ctx.cam.1 as f32);
        let on_screen = |x: f32, y: f32, m: f32| {
            x > cx - m && x < cx + W as f32 + m && y > cy - m && y < cy + H as f32 + m
        };
        for p in &self.drip_points {
            if ctx.t % p.period == p.offset % p.period && on_screen(p.x, p.y, 40.0) {
                self.drips.push(Drip {
                    x: p.x,
                    y: p.y,
                    z: 110.0,
                    vz: -1.0,
                });
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
                fx.particles.add(
                    Particle::new(x, y)
                        .vel(a.cos() * 0.3, a.sin() * 0.15, self.rng.range(0.9, 1.5))
                        .color(WATER[4])
                        .fade_to(WATER[3])
                        .life(self.rng.range(12.0, 18.0))
                        .grav(0.15),
                );
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

/// Distant, unlit wall planes below the rift. Their broad native facets stay well below the
/// receiving ledges in value: the bridge remains the crossing, with no invented piers.
fn abyss_plane(x: i32, y: i32) -> Option<f32> {
    const FAR: &[(i32, i32)] = &[
        (825, 183),
        (893, 169),
        (946, 184),
        (946, 224),
        (882, 233),
        (837, 215),
    ];
    const RIGHT: &[(i32, i32)] = &[
        (888, 296),
        (944, 278),
        (951, 371),
        (932, 458),
        (900, 490),
        (877, 452),
        (880, 369),
    ];
    const LEFT: &[(i32, i32)] = &[
        (750, 346),
        (778, 361),
        (789, 412),
        (776, 479),
        (743, 496),
        (734, 438),
    ];
    if (825..=946).contains(&x) && (169..=233).contains(&y) && in_abyss_plane(x, y, FAR) {
        Some(if x + y / 3 < 951 { 0.55 } else { 0.43 })
    } else if (877..=951).contains(&x) && (278..=490).contains(&y) && in_abyss_plane(x, y, RIGHT) {
        Some(if x < 891 + (y - 300) / 16 { 0.64 } else { 0.48 })
    } else if (734..=789).contains(&x) && (346..=496).contains(&y) && in_abyss_plane(x, y, LEFT) {
        Some(if x < 762 - (y - 390) / 18 { 0.49 } else { 0.59 })
    } else {
        None
    }
}

/// Half-open point-in-polygon on integer native pixels, with no antialiased boundary.
fn in_abyss_plane(x: i32, y: i32, outline: &[(i32, i32)]) -> bool {
    let (px, py) = (x * 2 + 1, y * 2 + 1);
    let mut inside = false;
    let mut previous = outline[outline.len() - 1];
    for &(bx, by) in outline {
        let (ax, ay) = previous;
        let (ax, ay, bx, by) = (ax * 2, ay * 2, bx * 2, by * 2);
        if (ay > py) != (by > py) {
            let lhs = (px - ax) * (by - ay);
            let rhs = (bx - ax) * (py - ay);
            if if by > ay { lhs < rhs } else { lhs > rhs } {
                inside = !inside;
            }
        }
        previous = (bx / 2, by / 2);
    }
    inside
}

/// Draw only behind actual missing ground, so far wall planes cannot cover a bank, bridge,
/// waterline, actor or native prop. World anchoring keeps them still while the camera moves.
pub fn draw_abyss(zone: &Zone, f: &mut Frame) {
    let base = ROCK[0].scale(0.35);
    for sy in 0..H as i32 {
        let wy = sy + f.cam_y;
        for sx in 0..W as i32 {
            let wx = sx + f.cam_x;
            let inside = wx >= 0 && wy >= 0 && wx < zone.pw && wy < zone.ph;
            if !inside || zone.ground[(wy * zone.pw + wx) as usize].r < 0.0 {
                f.albedo[sy as usize * W + sx as usize] = if inside {
                    abyss_plane(wx, wy).map_or(base, |scale| ROCK[0].scale(scale))
                } else {
                    base
                };
            }
        }
    }
}
