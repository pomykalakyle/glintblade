//! Tufts have rooted blades, a shared breeze, local bending and persistent cut stubble.

use super::{K, Map, mat, pal};
use crate::combat::Hit;
use crate::fx::{Fx, Particle};
use crate::gfx::{Frame, Rgb};
use crate::util::hash2;
use crate::world::{RegionCtx, Zone};
use std::f32::consts::TAU;

const ID: u32 = 1;

/// Root three small grass beds at the landing tree, broken pier and distributor base.
fn opening_bed(x: i32, y: i32) -> bool {
    [(93, 694, 21, 13), (244, 704, 23, 15), (326, 730, 31, 8)]
        .iter()
        .any(|&(cx, cy, rx, ry)| {
            let (dx, dy) = ((x - cx) as f32 / rx as f32, (y - cy) as f32 / ry as f32);
            dx * dx + dy * dy < 1.0
        })
}

struct Tuft {
    x: i32,
    y: i32,
    h: i32,
    teal: bool,
    phase: f32,
    bend: f32,
    cut: bool,
    quiet: bool,
}

/// Keep a connected, trampled clearing around the existing meadow encounter.
fn quiet_clearing(x: i32, y: i32) -> bool {
    [(400, 652, 38, 26), (410, 693, 42, 30), (461, 684, 36, 28)]
        .iter()
        .any(|&(cx, cy, rx, ry)| {
            let (dx, dy) = ((x - cx) as f32 / rx as f32, (y - cy) as f32 / ry as f32);
            dx * dx + dy * dy < 1.0
        })
}

pub struct Grass {
    tufts: Vec<Tuft>,
}

impl Grass {
    /// Place irregular rooted clumps inside the authored meadow patches, clear of solid props.
    pub fn build(m: &Map, mask: &[u8], z: &Zone) -> Self {
        let mut tufts = Vec::new();
        for gy in 1..z.ph / 7 - 1 {
            for gx in 1..z.pw / 8 - 1 {
                let x = gx * 8 + (hash2(gx, gy, 31) * 5.0) as i32;
                let y = gy * 7 + (hash2(gx, gy, 32) * 4.0) as i32;
                let meadow = m.at(x / 16, y / 16) == K::Tall;
                if (!meadow && !opening_bed(x, y)) || mask[(y * z.pw + x) as usize] != mat::GRASS {
                    continue;
                }
                if hash2(gx, gy, 22) < 0.18 || z.blocked(x as f32, y as f32, 4.0) {
                    continue;
                }
                // Open broad gaps between rooted groups instead of an even field of thin marks.
                if meadow && hash2(gx / 4, gy / 3, 419) < 0.24 {
                    continue;
                }
                let edge = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                    if meadow {
                        m.at(x / 16 + dx, y / 16 + dy) != K::Tall
                    } else {
                        !opening_bed(x + dx * 8, y + dy * 7)
                    }
                });
                if edge && hash2(gx, gy, 36) < 0.55 {
                    continue;
                }
                let teal = hash2(gx / 9, gy / 7, 35) > 0.55;
                tufts.push(Tuft {
                    x,
                    y,
                    h: if edge { 5 } else { 8 } + (hash2(gx, gy, 33) * 5.0) as i32,
                    teal,
                    phase: hash2(gx, gy, 34) * TAU,
                    bend: 0.0,
                    cut: false,
                    quiet: quiet_clearing(x, y),
                });
            }
        }
        Self { tufts }
    }

    /// Let nearby moving bodies press blades aside and let them spring back gradually.
    pub fn update(&mut self, ctx: &RegionCtx) {
        for tuft in &mut self.tufts {
            if tuft.cut {
                continue;
            }
            tuft.bend *= 0.88;
            for &(x, y, speed) in ctx.movers {
                if speed < 0.1
                    || (x - tuft.x as f32).abs() > 18.0
                    || (y - tuft.y as f32).abs() > 12.0
                {
                    continue;
                }
                let dx = tuft.x as f32 - x;
                let dy = tuft.y as f32 - y;
                if dx * dx + dy * dy * 1.8 < 220.0 {
                    tuft.bend = if dx >= 0.0 { 3.5 } else { -3.5 };
                }
            }
        }
    }

    /// Sever touched tufts once and throw physical blade pixels along the sword's direction.
    pub fn slash(&mut self, hit: &Hit, fx: &mut Fx) {
        let mut cut = 0;
        for tuft in &mut self.tufts {
            if tuft.cut || !hit.shape.touches(tuft.x as f32, tuft.y as f32, 3.0) {
                continue;
            }
            tuft.cut = true;
            cut += 1;
            let (dx, dy) = hit.push_dir(tuft.x as f32, tuft.y as f32);
            let ramp = if tuft.teal {
                &pal::TEAL[..]
            } else {
                &pal::GRASS[..]
            };
            for blade in 0..5 {
                for segment in 0..3 {
                    let speed = fx.rng.range(0.7, 2.4);
                    fx.particles.add(
                        Particle::new(tuft.x as f32 + (blade - 2) as f32, tuft.y as f32)
                            .z((3 + segment * 3) as f32)
                            .vel(
                                dx * speed + fx.rng.signed() * 0.7,
                                dy * speed + fx.rng.signed() * 0.6,
                                fx.rng.range(0.6, 2.0),
                            )
                            .color(ramp[2 + segment as usize % 2])
                            .life(fx.rng.range(160.0, 250.0))
                            .grav(0.13)
                            .drag(0.94)
                            .settle(true),
                    );
                }
            }
        }
        if cut > 0 {
            fx.sound_vol("grass_cut", 0.35);
        }
    }

    /// Draw a short rooted remainder wherever the player has cut the meadow.
    pub fn draw_stubs(&self, f: &mut Frame) {
        for tuft in &self.tufts {
            if !tuft.cut || !f.visible(tuft.x as f32, tuft.y as f32, 1.0, 1.0, 4.0) {
                continue;
            }
            let (x, y) = (tuft.x - f.cam_x, tuft.y - f.cam_y);
            f.rect(x - 2, y, 5, 1, pal::GRASS[1]);
            for dx in [-2, 0, 2] {
                f.put(x + dx, y - 1, pal::GRASS[3]);
            }
        }
    }

    /// Submit only visible uncut tufts for depth sorting with characters.
    pub fn items(&self, f: &Frame, out: &mut Vec<(f32, u32)>) {
        for (i, tuft) in self.tufts.iter().enumerate() {
            if !tuft.cut && f.visible(tuft.x as f32 - 5.0, tuft.y as f32 - 16.0, 10.0, 18.0, 2.0) {
                out.push((tuft.y as f32, (ID << 24) | i as u32));
            }
        }
    }

    /// Draw five tapered blades with a common low-amplitude wind wave.
    pub fn draw(&self, f: &mut Frame, id: u32, t: u64) {
        if id >> 24 != ID {
            return;
        }
        let Some(tuft) = self.tufts.get((id & 0xff_ffff) as usize) else {
            return;
        };
        let (x, y) = (tuft.x - f.cam_x, tuft.y - f.cam_y);
        let ramp: &[Rgb] = if tuft.teal { &pal::TEAL } else { &pal::GRASS };
        let wave = (t as f32 * 0.033 + tuft.x as f32 * 0.021 + tuft.y as f32 * 0.011).sin();
        let sway = (wave * 1.3 + tuft.bend).round() as i32;
        let base = if tuft.quiet { ramp[1] } else { ramp[0] };
        let height = if tuft.quiet { (tuft.h / 2).max(4) } else { tuft.h };
        f.rect(x - 3, y - 1, 7, 2, base);
        for blade in 0i32..5 {
            if tuft.quiet && (blade == 0 || blade == 4) {
                continue;
            }
            let root = x + (blade - 2) * 2;
            let h = height - (blade - 2).abs() * 2;
            let lean = (blade - 2) + sway;
            let top = root + lean;
            let tip = y - h + (tuft.phase.sin() * 1.0).round() as i32;
            f.line(root, y, root + lean / 3, y - h / 2, base);
            f.line(root + 1, y - 1, root + lean / 3 + 1, y - h / 2, ramp[1]);
            f.line(root + lean / 3, y - h / 2, top, tip, ramp[2]);
            if blade % 2 == 0 && !tuft.quiet {
                f.line(top, tip, top + lean.signum(), tip - 2, ramp[3]);
            }
        }
    }
}
