//! Connected molten currents carry sparse angular crust plates past fixed banks and source-bound embers.

use super::art::*;
use super::field::{Grid, edt};
use super::paint::Painted;
use crate::gfx::{Frame, H, Rgb, W};
use crate::util::hash2;

const PERIOD: i32 = 1024;
const WIDTH: i32 = 256;

#[derive(Clone, Copy)]
struct Flow {
    u: i16,
    v: i16,
    edge: u8,
    fall: bool,
    span: u8,
    foot: u8,
}

pub struct Lava {
    pixels: Grid<Option<Flow>>,
    bank: Grid<u8>,
    texture: Vec<u8>,
    sources: Vec<(i32, i32, u32)>,
}

/// Sample a gently bending centerline so plates turn downstream without per-segment texture resets.
fn river_line() -> Vec<(f32, f32, f32)> {
    const LINE: [(f32, f32); 15] = [
        (1450.0, 70.0),
        (1300.0, 124.0),
        (1144.0, 176.0),
        (976.0, 232.0),
        (880.0, 249.0),
        (768.0, 272.0),
        (656.0, 294.0),
        (544.0, 316.0),
        (456.0, 346.0),
        (400.0, 400.0),
        (374.0, 480.0),
        (362.0, 560.0),
        (346.0, 640.0),
        (328.0, 720.0),
        (320.0, 940.0),
    ];
    let mut line = vec![(LINE[0].0, LINE[0].1, 0.0)];
    for i in 0..LINE.len() - 1 {
        let (a, b, c, d) = (
            LINE[i.saturating_sub(1)],
            LINE[i],
            LINE[i + 1],
            LINE[(i + 2).min(LINE.len() - 1)],
        );
        for step in 1..=12 {
            let t = step as f32 / 12.0;
            let spline = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b
                    + (c - a) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t
                    + (-a + 3.0 * b - 3.0 * c + d) * t * t * t)
            };
            let (x, y) = (spline(a.0, b.0, c.0, d.0), spline(a.1, b.1, c.1, d.1));
            let &(px, py, length) = line.last().unwrap();
            line.push((x, y, length + (x - px).hypot(y - py)));
        }
    }
    line
}

/// Return continuous downstream distance and signed distance across the curved river.
fn river_coords(x: f32, y: f32, line: &[(f32, f32, f32)]) -> (i16, i16) {
    let (mut best, mut along, mut across) = (f32::MAX, 0.0, 0.0);
    for pts in line.windows(2) {
        let ((ax, ay, length), (bx, by, end)) = (pts[0], pts[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let len = end - length;
        let t = (((x - ax) * dx + (y - ay) * dy) / (len * len)).clamp(0.0, 1.0);
        let (qx, qy) = (x - ax - t * dx, y - ay - t * dy);
        let d = qx * qx + qy * qy;
        if d < best {
            best = d;
            along = length + len * t;
            across = d.sqrt().copysign(qy * dx - qx * dy);
        }
    }
    (along.round() as i16, across.round() as i16)
}

/// Test a pixel against one hard-edged authored polygon, without antialiasing.
fn in_polygon(x: f32, y: f32, polygon: &[(i32, i32)]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        let (ax, ay) = (previous.0 as f32, previous.1 as f32);
        let (bx, by) = (current.0 as f32, current.1 as f32);
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

/// Bake broad connected molten lanes and a deliberately uneven flotilla of flat crust plates.
fn river_texture() -> Vec<u8> {
    let mut texture = vec![3; (PERIOD * WIDTH) as usize];
    for v in 0..WIDTH {
        for u in 0..PERIOD {
            // Long unequal tongues carry the current; no lattice or closed bright cell rims.
            let phase = u as f32 * std::f32::consts::TAU / PERIOD as f32;
            let bend = (phase.sin() * 13.0 + (phase * 3.0).sin() * 5.0).round() as i32;
            let cross = (v - bend + WIDTH / 2).rem_euclid(WIDTH) - WIDTH / 2;
            let mut level = 3;
            for (offset, shift) in [(-79, 37), (0, 0), (91, 103)] {
                let distance = (cross - offset).abs();
                let tongue = (u + shift + cross / 2).rem_euclid(256);
                // One broad warm tongue carries a fine pale filament, interrupted by cooled rafts.
                let width = 9 + ((phase * 5.0 + shift as f32).sin() * 2.0).round() as i32;
                if distance <= width && tongue < 229 {
                    level = level.max(4);
                    if distance <= 1 && (18..180).contains(&tongue) {
                        level = 5;
                    }
                }
            }
            texture[(v * PERIOD + u) as usize] = level;
        }
    }
    // Coordinates, length, width and shear are composed rather than chosen from repeating cells.
    // Gaps between the plates remain broad open liquid, especially along the narrow main river.
    const PLATES: [(i32, i32, i32, i32, i32); 39] = [
        (24, -8, 47, 32, -4),
        (105, 17, 27, 19, 5),
        (169, -6, 68, 38, 6),
        (284, 15, 39, 28, -7),
        (370, -13, 83, 37, -5),
        (503, 12, 51, 31, 7),
        (594, -14, 33, 23, -3),
        (664, 8, 74, 36, -8),
        (789, -9, 62, 34, 4),
        (895, 17, 28, 22, 3),
        (966, -8, 33, 26, -4),
        (58, 62, 73, 39, 8),
        (187, 96, 31, 25, -3),
        (256, 53, 57, 34, -7),
        (369, 104, 42, 28, 4),
        (468, 64, 95, 43, -9),
        (625, 98, 59, 34, 4),
        (742, 55, 33, 27, 5),
        (840, 100, 68, 37, -6),
        (961, 54, 41, 28, 3),
        (17, -91, 64, 33, 4),
        (144, -57, 38, 27, -5),
        (235, -105, 93, 41, 6),
        (393, -68, 63, 34, -8),
        (516, -107, 29, 24, 4),
        (594, -56, 72, 39, 6),
        (725, -103, 48, 31, -4),
        (832, -64, 83, 41, -8),
        (971, -96, 37, 27, 3),
        (98, 127, 35, 28, 6),
        (344, -128, 28, 24, -4),
        (578, 130, 44, 30, -5),
        (800, -133, 25, 23, 4),
        (72, -40, 24, 18, 3),
        (328, 35, 29, 20, -4),
        (568, -38, 18, 16, 4),
        (766, 37, 23, 18, -3),
        (214, 29, 18, 15, 3),
        (937, -35, 21, 18, -3),
    ];
    for (n, &(u, v, length, width, shear)) in PLATES.iter().enumerate() {
        let notch = if n % 3 == 0 {
            length / 2
        } else {
            length * 3 / 4
        };
        let outline = [
            (0, -width / 5),
            (length / 5, -width / 2),
            (length * 2 / 3, -width / 2 + shear),
            (length, -width / 6 + shear),
            (length - 3, width / 4 + shear),
            (notch, width / 4 + shear),
            (length * 2 / 3, width / 2),
            (length / 5, width / 2 - 2),
            (0, width / 5),
        ];
        for y in -width..=width + shear.abs() {
            for x in 0..=length {
                if !in_polygon(x as f32 + 0.5, y as f32 + 0.5, &outline) {
                    continue;
                }
                // Only selected downstream edges glow; each plate keeps a solid, unperforated body.
                let hot_edge = x > length / 2
                    && (!in_polygon(x as f32 + 1.5, y as f32 + 0.5, &outline)
                        || (y > shear && (!in_polygon(x as f32 + 0.5, y as f32 + 1.5, &outline)
                            || !in_polygon(x as f32 + 1.5, y as f32 + 1.5, &outline))));
                let level = if hot_edge {
                    4
                } else if y > width / 5 && x > length / 3 {
                    1
                } else {
                    0
                };
                let (tx, ty) = ((u + x).rem_euclid(PERIOD), (v + y).rem_euclid(WIDTH));
                texture[(ty * PERIOD + tx) as usize] = level;
            }
        }
    }
    texture
}

/// Stretch a single hot fall core downstream and fold its terminal impact into the existing mask.
fn fall_heat(q: &Flow, t: u64) -> usize {
    let depth = q.u as i32;
    let across = q.v as i32;
    let width = q.span as i32;
    let margin = (width - across.abs()) / 2;
    if margin < 2 {
        return 1;
    }
    if margin < 4 {
        return 3;
    }
    let travel = (depth - (t / 2 % 96) as i32).rem_euclid(96);
    // The bright tongue stretches and narrows as it drops, rather than sliding striped columns.
    let lean = match travel {
        0..=22 => -2,
        23..=24 => -1,
        25..=65 => 0,
        66..=67 => 1,
        _ => 2,
    };
    let distance = (across - lean).abs();
    let core = match travel {
        0..=43 => 5,
        44..=49 => 4,
        50..=55 => 3,
        _ => 2,
    };
    let mut level = if distance <= core {
        5
    } else if distance <= core + 6 {
        4
    } else {
        3
    };
    if depth < 3 && distance <= core + 4 {
        level = 5;
    }
    // A low forked splash stays within the authored foot; no extra silhouette, pool or particles.
    if q.foot < 9 {
        let phase = ((t / 6) % 4) as i32;
        let foot = q.foot as i32;
        let arc = (distance / 4 + phase).rem_euclid(7);
        level = if foot < 2 {
            1
        } else if (foot - arc - 2).abs() <= 1 {
            if distance < width / 2 { 5 } else { 4 }
        } else {
            3
        };
    }
    level
}

impl Lava {
    /// Bake flow coordinates, bank distances, and a seamless field of cooled crust rafts.
    pub fn new(p: &Painted) -> Self {
        let (w, h) = (p.kind.w, p.kind.h);
        let source_edge = edt(&Grid {
            w,
            h,
            v: p.lava_full.v.iter().map(|&v| !v).collect(),
        });
        let edge = edt(&Grid {
            w,
            h,
            v: p.lava_draw.v.iter().map(|&v| !v).collect(),
        });
        let dist = edt(&p.lava_draw);
        let mut pixels = Grid {
            w,
            h,
            v: (0..w * h).map(|_| None).collect(),
        };
        let mut bank = Grid::new(w, h, 0u8);
        let mut sources = Vec::new();
        let line = river_line();
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if dist.v[i] < 44.0 {
                    bank.v[i] = (44.0 - dist.v[i]) as u8;
                }
                if !p.lava_draw.v[i] {
                    continue;
                }
                let fall = p.fall.v[i];
                let (mut span, mut foot) = (0, 0);
                let (u, v) = if fall {
                    let (mut left, mut right, mut top, mut bottom) = (x, x, y, y);
                    while left > 0 && p.fall.at(left - 1, y) {
                        left -= 1;
                    }
                    while right + 1 < w && p.fall.at(right + 1, y) {
                        right += 1;
                    }
                    while top > 0 && p.fall.at(x, top - 1) {
                        top -= 1;
                    }
                    while bottom + 1 < h && p.fall.at(x, bottom + 1) {
                        bottom += 1;
                    }
                    span = (right - left + 1).min(255) as u8;
                    foot = (bottom - y).min(255) as u8;
                    ((y - top) as i16, (2 * x - left - right) as i16)
                } else if (636..676).contains(&y) && x > 400 && x < 735 {
                    ((735 - x) as i16, y as i16)
                } else {
                    river_coords(x as f32, y as f32, &line)
                };
                pixels.v[i] = Some(Flow {
                    u,
                    v,
                    edge: edge.v[i].min(255.0) as u8,
                    fall,
                    span,
                    foot,
                });
                if x % 27 == 0 && y % 23 == 0 && source_edge.v[i] > 4.0 && hash2(x, y, 940) > 0.35 {
                    sources.push((x, y, (hash2(x, y, 941) * 180.0) as u32));
                }
            }
        }
        Self {
            pixels,
            bank,
            texture: river_texture(),
            sources,
        }
    }

    /// Move cooled rafts downstream in integer pixel steps; shores remain dark and crusted.
    pub fn draw(&self, f: &mut Frame, t: u64) {
        let drift = (t / 6 % PERIOD as u64) as i32;
        for sy in 0..H as i32 {
            let y = sy + f.cam_y;
            if y < 0 || y >= self.pixels.h {
                continue;
            }
            for sx in 0..W as i32 {
                let x = sx + f.cam_x;
                if x < 0 || x >= self.pixels.w {
                    continue;
                }
                let Some(q) = &self.pixels.v[(y * self.pixels.w + x) as usize] else {
                    continue;
                };
                let l = if q.fall {
                    fall_heat(q, t)
                } else {
                    let u = (q.u as i32 - drift).rem_euclid(PERIOD);
                    let v = (q.v as i32).rem_euclid(WIDTH);
                    let heat = self.texture[(v * PERIOD + u) as usize] as usize;
                    // Four liquid steps: dark red bank, salmon, orange tongue and a fine pale core.
                    if q.edge < 4 {
                        1
                    } else if q.edge < 8 {
                        heat.min(3)
                    } else {
                        heat
                    }
                };
                let c = LAVA_R[l];
                let i = sy as usize * W + sx as usize;
                // Cooled stone is opaque material, never a dark emitter with a checker halo of its own.
                if l == 0 {
                    f.albedo[i] = L0;
                    f.emissive[i] = Rgb::BLACK;
                } else {
                    f.albedo[i] = Rgb::BLACK;
                    f.emissive[i] = c.scale(match l {
                        5 => 1.0,
                        4 => 0.92,
                        3 => 0.87,
                        _ => 0.72,
                    });
                }
            }
        }
    }

    /// Emit only from fixed lava sources, with a cool tail and a short rising lifetime.
    pub fn embers(&self, f: &mut Frame, t: u64) {
        for &(x, y, phase) in &self.sources {
            if !f.visible(x as f32 - 8.0, y as f32 - 44.0, 24.0, 48.0, 0.0) {
                continue;
            }
            let age = ((t + phase as u64) % 180) as i32;
            if age >= 92 {
                continue;
            }
            let sway = ((age as f32 * 0.06 + phase as f32).sin() * 3.0) as i32;
            let sx = x - f.cam_x + age / 12 + sway;
            let sy = y - f.cam_y - age / 3;
            let c = if age < 30 {
                EMBER[1]
            } else if age < 65 {
                EMBER[0]
            } else {
                L2
            };
            f.put_glow(sx, sy, c.scale(0.8));
            if age < 55 {
                f.put_glow(sx, sy + 1, L2.scale(0.7));
            }
        }
    }

    /// Add a low, stepped warm bounce within 44 pixels of actual molten pixels.
    pub fn light(&self, f: &mut Frame, t: u64) {
        f.look.light_dither = 0.0; // Broad stepped banks stay calm; no all-over hatch texture.
        let pulse = 0.92 + (t as f32 * 0.035).sin() * 0.08;
        for sy in 0..H as i32 {
            let y = sy + f.cam_y;
            if y < 0 || y >= self.bank.h {
                continue;
            }
            for sx in 0..W as i32 {
                let x = sx + f.cam_x;
                if x < 0 || x >= self.bank.w {
                    continue;
                }
                let b = self.bank.v[(y * self.bank.w + x) as usize];
                if b == 0 {
                    continue;
                }
                let a = b as f32 / 44.0;
                f.light[sy as usize * W + sx as usize] += LAVA_LIGHT.scale(a * a * 0.28 * pulse);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regions::cinder::paint::{K_FALL, K_LAVA, map_size, paint, walkable_kind};
    use crate::world::{Zone, ZoneId};

    /// Time the default renderer and lava hooks at entry, bridge, caldera and the full fall.
    #[test]
    #[ignore = "explicit release-mode Cinder render timing"]
    fn cinder_signature_render_timing() {
        use crate::game::{Game, Options, Scene};
        use std::{hint::black_box, time::Instant};
        let mut game = Game::new(Options {
            start: Some(ZoneId::Cinder),
            god: true,
            peaceful: false,
            seed: 7,
        });
        for _ in 0..90 {
            game.tick(&crate::input::Input::default());
        }
        for (name, pos) in [
            ("entry", (170.0, 720.0)),
            ("bridge", (370.0, 455.0)),
            ("caldera", (1144.0, 280.0)),
            ("fall", (330.0, 850.0)),
        ] {
            if let Scene::Play(p) = &mut game.scene {
                p.player.body.x = pos.0;
                p.player.body.y = pos.1;
                p.cam.snap(pos.0, pos.1);
            }
            for _ in 0..8 {
                game.render();
            }
            let mut timings = Vec::new();
            for _ in 0..120 {
                let start = Instant::now();
                game.render();
                black_box(&game.frame.out);
                timings.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            timings.sort_by(f64::total_cmp);
            println!(
                "Cinder {name}: default render median {:.3} ms, p95 {:.3} ms",
                timings[60], timings[114]
            );
            assert!(
                timings[114] < 8.0,
                "Cinder {name} exceeded the default renderer budget"
            );
        }
        let (tw, th) = map_size();
        let mut zone = Zone::new(ZoneId::Cinder, tw, th);
        let painted = paint(&mut zone);
        let lava = Lava::new(&painted);
        let mut frame = Frame::new();
        frame.cam_x = 952;
        frame.cam_y = 172;
        let start = Instant::now();
        for t in 0..600 {
            lava.draw(&mut frame, t);
            lava.embers(&mut frame, t);
            lava.light(&mut frame, t);
            black_box(&frame.emissive);
        }
        println!(
            "Cinder lava draw/embers/light: {:.3} ms/frame",
            start.elapsed().as_secs_f64() * 1000.0 / 600.0
        );
    }

    /// Keep molten material and opaque crust masked, world-aligned and moving in each river or fall view.
    #[test]
    fn river_motion_preserves_banks_crossings_and_camera_alignment() {
        let (tw, th) = map_size();
        let mut zone = Zone::new(ZoneId::Cinder, tw, th);
        let painted = paint(&mut zone);
        let lava = Lava::new(&painted);
        for i in 0..painted.kind.v.len() {
            if painted.lava_draw.v[i] {
                assert!(matches!(painted.kind.v[i], K_LAVA | K_FALL));
                assert!(!walkable_kind(painted.kind.v[i]));
            }
        }
        for (cx, cy) in [(178, 347), (952, 172), (428, 542), (180, 696)] {
            let mut moving_pixels = 0;
            let mut first = Frame::new();
            first.begin(Rgb::WHITE, Rgb::WHITE);
            first.cam_x = cx;
            first.cam_y = cy;
            lava.draw(&mut first, 90);
            let mut next = Frame::new();
            next.begin(Rgb::WHITE, Rgb::WHITE);
            next.cam_x = cx;
            next.cam_y = cy;
            lava.draw(&mut next, 150);
            let mut shifted = Frame::new();
            shifted.begin(Rgb::WHITE, Rgb::WHITE);
            shifted.cam_x = cx + 7;
            shifted.cam_y = cy + 3;
            lava.draw(&mut shifted, 90);
            for sy in 0..H {
                for sx in 0..W {
                    let i = sy * W + sx;
                    let world = ((cy + sy as i32) * zone.pw + cx + sx as i32) as usize;
                    let molten = painted.lava_draw.v[world];
                    assert_eq!(first.albedo[i] != Rgb::WHITE, molten);
                    assert_eq!(
                        first.emissive[i] != Rgb::BLACK || first.albedo[i] == L0,
                        molten
                    );
                    assert_eq!(next.albedo[i] != Rgb::WHITE, molten);
                    assert_eq!(
                        next.emissive[i] != Rgb::BLACK || next.albedo[i] == L0,
                        molten
                    );
                    if !molten {
                        assert_eq!(first.albedo[i], Rgb::WHITE, "overpainted bank or crossing");
                    }
                    moving_pixels += usize::from(first.emissive[i] != next.emissive[i]);
                    if sx >= 7 && sy >= 3 {
                        assert_eq!(first.emissive[i], shifted.emissive[(sy - 3) * W + sx - 7]);
                        assert_eq!(first.albedo[i], shifted.albedo[(sy - 3) * W + sx - 7]);
                    }
                }
            }
            assert!(
                moving_pixels > 0,
                "lava stopped flowing at camera {cx},{cy}"
            );
        }
    }
}
