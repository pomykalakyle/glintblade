//! Connected molten currents carry sparse angular crust plates past fixed banks and source-bound embers.

use super::art::*;
use super::field::{Grid, edt};
use super::paint::Painted;
use crate::gfx::{Frame, H, Rgb, W};
use crate::gfx::frame::PointLight;
use crate::util::hash2;

const PERIOD: i32 = 1024;
const WIDTH: i32 = 256;
/// World-fixed area-source quadrature: camera movement never reselects emitters.
const LIGHT_CELL: i32 = 80;
const LIGHT_RADIUS: f32 = 80.0;
const LIGHT_HEIGHT: f32 = 4.0;
const HEAT_POWER: [f32; 6] = [0.0, 0.12, 0.42, 0.78, 1.05, 1.30];
const RADIANT_GAIN: f32 = 3.0;

#[derive(Clone, Copy)]
struct Flow {
    u: i16,
    v: i16,
    edge: u8,
    fall: bool,
    span: u8,
    foot: u8,
}

#[derive(Clone, Copy)]
struct RadiantSample {
    x: i32,
    y: i32,
    flow: Flow,
    weight: f32,
}

struct RadiantPatch {
    x: i32,
    y: i32,
    samples: Vec<RadiantSample>,
}

pub struct Lava {
    pixels: Grid<Option<Flow>>,
    bank: Grid<u8>,
    texture: Texture,
    sources: Vec<(i32, i32, u32)>,
    radiant_patches: Vec<RadiantPatch>,
}

/// Approximate the exposed molten area with one stable patch per world cell.
/// Down-void falls cannot light the island from imaginary hovering sources;
/// only their real upper twelve rows participate with the connected river lip.
fn radiant_patches(pixels: &Grid<Option<Flow>>) -> Vec<RadiantPatch> {
    let mut patches = Vec::new();
    for y0 in (0..pixels.h).step_by(LIGHT_CELL as usize) {
        for x0 in (0..pixels.w).step_by(LIGHT_CELL as usize) {
            let (cx, cy) = (x0 + LIGHT_CELL / 2, y0 + LIGHT_CELL / 2);
            let mut best = None;
            let mut best_score = i32::MAX;
            for y in y0..(y0 + LIGHT_CELL).min(pixels.h) {
                for x in x0..(x0 + LIGHT_CELL).min(pixels.w) {
                    let Some(q) = pixels.at(x, y) else { continue };
                    if q.fall && q.u > 12 { continue; }
                    let edge_penalty = (4 - q.edge.min(4) as i32) * 48;
                    let score = (x - cx).pow(2) + (y - cy).pow(2) + edge_penalty;
                    if score < best_score {
                        best_score = score;
                        best = Some((x, y));
                    }
                }
            }
            let Some((x, y)) = best else { continue };
            let mut samples = Vec::new();
            for oy in -3i32..=3 {
                for ox in -3i32..=3 {
                    let (sx, sy) = (x + ox * 2, y + oy * 2);
                    // Adjacent world cells never claim the same emitting ink.
                    if sx < x0 || sx >= (x0 + LIGHT_CELL).min(pixels.w)
                        || sy < y0 || sy >= (y0 + LIGHT_CELL).min(pixels.h) {
                        continue;
                    }
                    let Some(flow) = pixels.at(sx, sy) else { continue };
                    if flow.fall && flow.u > 12 { continue; }
                    samples.push(RadiantSample { x: sx, y: sy, flow,
                        weight: ((4 - ox.abs()) * (4 - oy.abs())) as f32 });
                }
            }
            patches.push(RadiantPatch { x, y, samples });
        }
    }
    patches
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

/// The foundry spill flows west into the southbound river. Its incoming basis
/// turns continuously into that river at the actual mouth, instead of resetting
/// both native texture axes at the old arbitrary x400 boundary.
fn surface_coords(x: i32, y: i32, line: &[(f32, f32, f32)]) -> (i16, i16) {
    let main = river_coords(x as f32, y as f32, line);
    if x <= 368 || !(628..684).contains(&y) {
        return main;
    }
    let mouth = river_coords(380.0, 656.0, line);
    let turn = ((x - 368) as f32 / 72.0).clamp(0.0, 1.0);
    let spread = ((28 - (y - 656).abs()) as f32 / 12.0).clamp(0.0, 1.0);
    let k = turn * turn * (3.0 - 2.0 * turn) * spread;
    let branch = (mouth.0 as i32 - (x - 380), mouth.1 as i32 - (y - 656));
    (
        (main.0 as f32 + (branch.0 as f32 - main.0 as f32) * k).round() as i16,
        (main.1 as f32 + (branch.1 as f32 - main.1 as f32) * k).round() as i16,
    )
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

/// Native flow texture repeats by complete source cells, including across both world-wrap axes.
struct Texture {
    pixels: Vec<u8>,
    period: i32,
    width: i32,
}

impl Texture {
    fn native(cell: &crate::gfx::Cell) -> Self {
        let period = (PERIOD / cell.w).max(1) * cell.w;
        let width = (WIDTH / cell.h).max(1) * cell.h;
        let mut pixels = vec![3; (period * width) as usize];
        for v in 0..width {
            for u in 0..period {
                let p = cell.at(u.rem_euclid(cell.w), v.rem_euclid(cell.h));
                // Exact native palette-to-heat mapping; no resizing or interpolated pixels.
                let level = LAVA_R
                    .iter()
                    .enumerate()
                    .min_by(|(_, x), (_, y)| {
                        let distance = |c: &Rgb| {
                            (c.r - p.c.r).powi(2) + (c.g - p.c.g).powi(2) + (c.b - p.c.b).powi(2)
                        };
                        distance(x).total_cmp(&distance(y))
                    })
                    .map_or(3, |(i, _)| i as u8);
                pixels[(v * period + u) as usize] = level;
            }
        }
        Self {
            pixels,
            period,
            width,
        }
    }

    fn sample(&self, u: i32, v: i32) -> u8 {
        self.pixels[(v.rem_euclid(self.width) * self.period + u.rem_euclid(self.period)) as usize]
    }
}

/// Bake a native material on the continuous river sheet; authored masks own the fixed banks.
fn river_texture() -> Texture {
    if let Some(s) = crate::gfx::sheet("cinder_lava_surface")
        && let Some(a) = s.anim("body")
    {
        return Texture::native(s.cell(a, 0));
    }
    // A quiet complete fallback remains useful when reviewing an incomplete asset pack.
    // Native molten art is the default; no rejected diagnostic branches ship.
    let mut texture = vec![3; (PERIOD * WIDTH) as usize];
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
        // Cooled rafts are narrow fragments floating in liquid, rather than huge flat
        // polygon islands spanning the river. Two planes keep their thickness legible.
        let length = length * 3 / 4;
        let width = width * 2 / 3;
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
                        || (y > shear
                            && (!in_polygon(x as f32 + 0.5, y as f32 + 1.5, &outline)
                                || !in_polygon(x as f32 + 1.5, y as f32 + 1.5, &outline))));
                let level = if hot_edge {
                    3
                } else if y < 0 && x < length * 2 / 3 {
                    1
                } else {
                    0
                };
                let (tx, ty) = ((u + x).rem_euclid(PERIOD), (v + y).rem_euclid(WIDTH));
                texture[(ty * PERIOD + tx) as usize] = level;
            }
        }
    }
    Texture {
        pixels: texture,
        period: PERIOD,
        width: WIDTH,
    }
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
                } else {
                    surface_coords(x, y, &line)
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
        let radiant_patches = radiant_patches(&pixels);
        Self {
            pixels,
            bank,
            texture: river_texture(),
            sources,
            radiant_patches,
        }
    }

    /// The same palette/phase sample drives visible molten ink and its light.
    /// Cold level-zero crust contributes no radiant energy.
    fn heat(&self, q: &Flow, drift: i32, t: u64) -> usize {
        if q.fall {
            fall_heat(q, t)
        } else {
            let heat = self.texture.sample(q.u as i32 - drift, q.v as i32) as usize;
            if q.edge < 2 { 1 } else if q.edge < 5 { heat.min(3) } else { heat }
        }
    }

    /// Area-weighted current heat moves downstream with the actual crust. A
    /// representative source remains on real emitting ink nearest the hot
    /// centroid, so its direction changes with local flow instead of a global
    /// pulse. Neighbouring cells keep independent native texture phases.
    fn radiant_light(&self, patch: &RadiantPatch, t: u64) -> Option<PointLight> {
        let drift = (t / 6 % self.texture.period as u64) as i32;
        let (mut energy, mut source_energy, mut area, mut cx, mut cy) =
            (0.0, 0.0, 0.0, 0.0, 0.0);
        let mut color = Rgb::BLACK;
        for p in &patch.samples {
            let level = self.heat(&p.flow, drift, t);
            let power = HEAT_POWER[level] * p.weight;
            // Hot exposed channels radiate much more than their cooling crust.
            // A convex heat response preserves orange L3, reduces L1/L2, and
            // makes the actual L4/L5 tongues visibly light nearby stone as
            // they pass. Keep the native color and source centroid unchanged.
            energy += power * HEAT_POWER[level] / HEAT_POWER[3];
            source_energy += power;
            area += p.weight;
            cx += (p.x as f32 + 0.5) * power;
            cy += (p.y as f32 + 0.5) * power;
            color += LAVA_R[level].scale(power);
        }
        if source_energy <= 0.0 || area <= 0.0 { return None; }
        cx /= source_energy;
        cy /= source_energy;
        let mut position = None;
        let mut nearest = f32::MAX;
        for p in &patch.samples {
            if self.heat(&p.flow, drift, t) == 0 { continue; }
            let (x, y) = (p.x as f32 + 0.5, p.y as f32 + 0.5);
            let d2 = (x - cx).powi(2) + (y - cy).powi(2);
            if d2 < nearest {
                nearest = d2;
                position = Some((x, y));
            }
        }
        let (x, y) = position?;
        Some(PointLight { x, y, height: LIGHT_HEIGHT, radius: LIGHT_RADIUS,
            color: color.scale(1.0 / source_energy), intensity: energy / area * RADIANT_GAIN })
    }

    /// Move cooled rafts downstream in integer pixel steps; shores remain dark and crusted.
    pub fn draw(&self, f: &mut Frame, t: u64) {
        let drift = (t / 6 % self.texture.period as u64) as i32;
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
                let l = self.heat(q, drift, t);
                let c = LAVA_R[l];
                let i = sy as usize * W + sx as usize;
                // Cooled stone is opaque material, never a dark emitter with a checker halo of its own.
                if l == 0 {
                    let u = q.u as i32 - drift;
                    let v = q.v as i32;
                    let hot_top = self.texture.sample(u - 1, v - 1) >= 2;
                    f.albedo[i] = if !q.fall && hot_top {
                        L0.lerp(B2, 0.42)
                    } else {
                        L0
                    };
                    f.emissive[i] = Rgb::BLACK;
                } else if l == 1 {
                    // This is cooler red liquid, distinct from the opaque level0
                    // basalt. The old .22gain collapsed both into nearly black.
                    f.albedo[i] = Rgb::BLACK;
                    f.emissive[i] = c.scale(0.78);
                } else {
                    f.albedo[i] = Rgb::BLACK;
                    f.emissive[i] = c.scale(match l {
                        5 => 1.0,
                        4 => 0.98,
                        _ => 0.94,
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
        if f.surface_lighting {
            for patch in &self.radiant_patches {
                // Include the complete patch and 3D sphere projection, before
                // evaluating its samples. No camera-ranked source budget.
                let rx = LIGHT_RADIUS + 7.0;
                let ry = LIGHT_RADIUS * std::f32::consts::SQRT_2 + 7.0;
                if !f.visible(patch.x as f32 - rx, patch.y as f32 - ry, rx * 2.0, ry * 2.0, 0.0) {
                    continue;
                }
                if let Some(light) = self.radiant_light(patch, t) {
                    // The low thermal volume is projected onto its actual
                    // molten pixel; its finite height approximates an area
                    // emitter without lifting light dots into empty space.
                    f.add_light_3d_w(light.x, light.y + light.height, light.height,
                        light.radius, light.color, light.intensity);
                }
            }
            return;
        }
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

    fn production_lava() -> (Painted, Lava) {
        let (tw, th) = map_size();
        let mut zone = Zone::new(ZoneId::Cinder, tw, th);
        let painted = paint(&mut zone);
        let lava = Lava::new(&painted);
        (painted, lava)
    }

    #[test]
    fn physical_lava_sources_are_world_anchored_actual_molten_pixels() {
        let (painted, lava) = production_lava();
        assert!(!lava.radiant_patches.is_empty());
        for t in [0, 90, 180, 360] {
            for patch in &lava.radiant_patches {
                let Some(light) = lava.radiant_light(patch, t) else { continue };
                let (x, y) = (light.x.floor() as i32, light.y.floor() as i32);
                assert!(painted.lava_draw.at(x, y));
                let q = lava.pixels.at(x, y).unwrap();
                let drift = (t / 6 % lava.texture.period as u64) as i32;
                assert!(lava.heat(&q, drift, t) > 0, "source emitted from cold crust at {x},{y} tick{t}");
                assert!(!q.fall || q.u <= 12, "source floated in a down-void fall");
                assert_eq!((light.height, light.radius), (LIGHT_HEIGHT, LIGHT_RADIUS));
            }
        }
        let mut original = Frame::new();
        original.surface_lighting = true;
        original.cam_x = 178;
        original.cam_y = 347;
        lava.light(&mut original, 90);
        let mut shifted = Frame::new();
        shifted.surface_lighting = true;
        shifted.cam_x = original.cam_x + 7;
        shifted.cam_y = original.cam_y + 3;
        lava.light(&mut shifted, 90);
        let mut common = 0;
        for a in &original.point_lights {
            if let Some(b) = shifted.point_lights.iter().find(|b| b.x == a.x && b.y == a.y) {
                common += 1;
                assert_eq!((a.height, a.radius, a.color, a.intensity), (b.height, b.radius, b.color, b.intensity));
            }
        }
        assert!(common >= 2, "camera overlap lost world-space emitters");
        assert!(original.light.iter().all(|&l| l == Rgb::WHITE), "normal mode retained the unnormaled bank overlay");
    }

    #[test]
    fn local_lava_heat_changes_independently_with_native_crust() {
        let (_, lava) = production_lava();
        let (mut brighter, mut darker, mut moved) = (false, false, false);
        let mut changes = std::collections::BTreeSet::new();
        for patch in &lava.radiant_patches {
            for (ta, tb) in [(0, 90), (90, 180), (180, 360)] {
                let a = lava.radiant_light(patch, ta);
                let b = lava.radiant_light(patch, tb);
                let delta = b.map_or(0.0, |l| l.intensity) - a.map_or(0.0, |l| l.intensity);
                brighter |= delta > 0.03;
                darker |= delta < -0.03;
                changes.insert((delta * 1000.0).round() as i32);
                if let (Some(a), Some(b)) = (a, b) { moved |= a.x != b.x || a.y != b.y; }
            }
        }
        assert!(brighter && darker && changes.len() >= 4, "all lava lights changed as a uniform pulse");
        assert!(moved, "hot-centroid direction never followed the advecting ink");
    }

    /// The real current at the foundry spill must change received light, rather
    /// than merely animating bright lava ink against an almost static bank.
    #[test]
    fn hot_native_currents_increase_real_bank_light_variation() {
        let (tw, th) = map_size();
        let mut zone = Zone::new(ZoneId::Cinder, tw, th);
        let painted = paint(&mut zone);
        let lava = Lava::new(&painted);
        // The ash beside this spill has a real planar receiving surface, while
        // the surviving paving also retains native UV normals. Read the actual
        // production terrain pass; raw atlas metadata alone omits ash entirely.
        let mut terrain = Frame::new();
        terrain.surface_lighting = true;
        terrain.cam_x = 224;
        terrain.cam_y = 504;
        crate::gfx::lighting::ground_surface(&zone, &crate::world::NoRegion, &mut terrain);
        let receiver_at = |x: i32, y: i32| {
            let (sx, sy) = (x - terrain.cam_x, y - terrain.cam_y);
            Frame::idx(sx, sy).map(|i| terrain.surfaces[i])
        };
        let old_intensity = |patch: &RadiantPatch, t: u64| {
            let drift = (t / 6 % lava.texture.period as u64) as i32;
            let (mut energy, mut area) = (0.0, 0.0);
            for p in &patch.samples {
                energy += HEAT_POWER[lava.heat(&p.flow, drift, t)] * p.weight;
                area += p.weight;
            }
            energy / area * 1.6
        };
        let receive = |light: PointLight, x: i32, y: i32| {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::BLACK);
            f.surface_lighting = true;
            f.cam_x = x - 192;
            f.cam_y = y - 108;
            let i = 108 * W + 192;
            f.albedo[i] = zone.ground[(y * zone.pw + x) as usize];
            f.surfaces[i] = receiver_at(x, y).unwrap();
            f.add_light_3d_w(light.x, light.y + light.height, light.height,
                light.radius, light.color, light.intensity);
            crate::gfx::surface::apply(&mut f);
            f.light[i].luma()
        };
        let (mut varied_patches, mut bank_change) = (0, None);
        let mut diagnostics = Vec::new();
        // This is the actual six-second gameplay sample, in world coordinates.
        for patch in &lava.radiant_patches {
            if !(240..=600).contains(&patch.x) || !(540..=720).contains(&patch.y) {
                continue;
            }
            let samples: Vec<_> = (301..=660).step_by(6).filter_map(|t| {
                lava.radiant_light(patch, t).map(|light| (t, light, old_intensity(patch, t)))
            }).collect();
            let Some(low) = samples.iter().min_by(|a, b| a.1.intensity.total_cmp(&b.1.intensity)) else { continue };
            let high = samples.iter().max_by(|a, b| a.1.intensity.total_cmp(&b.1.intensity)).unwrap();
            let old_range = samples.iter().map(|s| s.2).fold(f32::NEG_INFINITY, f32::max)
                - samples.iter().map(|s| s.2).fold(f32::INFINITY, f32::min);
            let new_range = high.1.intensity - low.1.intensity;
            diagnostics.push(format!("patch {},{}: intensity range old{old_range:.4}/new{new_range:.4}; low{} high{}", patch.x, patch.y, low.0, high.0));
            if old_range < 0.03 || new_range < old_range * 1.5 { continue; }
            varied_patches += 1;
            if bank_change.is_some() { continue; }
            let (cx, cy) = (high.1.x.floor() as i32, high.1.y.floor() as i32);
            let mut receivers = Vec::new();
            for dy in (-32i32..=32).step_by(2) {
                for dx in (-32i32..=32).step_by(2) {
                    if !(144..=1024).contains(&(dx * dx + dy * dy)) { continue; }
                    let (x, y) = (cx + dx, cy + dy);
                    if !painted.kind.inside(x, y) || !walkable_kind(painted.kind.at(x, y)) {
                        continue;
                    }
                    let Some(surface) = receiver_at(x, y) else { continue };
                    if surface.kind != 1 || surface.height != 0.0 || surface.normal[2] < 0.63 { continue; }
                    receivers.push((dx * dx + dy * dy, x, y));
                }
            }
            receivers.sort_unstable();
            diagnostics.push(format!("  real floor candidates {}", receivers.len()));
            for (_, x, y) in receivers.into_iter().take(8) {
                let old_low = PointLight { intensity: low.2, ..low.1 };
                let old_high = PointLight { intensity: high.2, ..high.1 };
                let old_delta = (receive(old_high, x, y) - receive(old_low, x, y)).abs();
                let new_delta = (receive(high.1, x, y) - receive(low.1, x, y)).abs();
                diagnostics.push(format!("  bank {x},{y}: received variation old{old_delta:.5}/new{new_delta:.5}; normal{:?}", receiver_at(x, y).unwrap().normal));
                if new_delta > 0.025 && new_delta > old_delta * 1.5 {
                    bank_change = Some((x, y, low.0, high.0, old_delta, new_delta));
                    break;
                }
            }
        }
        assert!(varied_patches >= 2, "native current lost its stronger independent heat changes: {varied_patches}\n{}", diagnostics.join("\n"));
        assert!(bank_change.is_some(), "hot native tongues failed to increase light variation on any actual spill-bank normal\n{}", diagnostics.join("\n"));
        println!("Native molten patches with stronger variation: {varied_patches}; real bank {bank_change:?}");
    }

    #[test]
    fn molten_sources_reach_real_bank_and_obey_receiving_normals() {
        use crate::gfx::frame::Surface;
        let (painted, lava) = production_lava();
        let mut sample = None;
        'source: for patch in &lava.radiant_patches {
            let Some(light) = lava.radiant_light(patch, 90) else { continue };
            if light.intensity < 0.5 { continue; }
            for dy in -28i32..=28 {
                for dx in -28i32..=28 {
                    if !(144..=784).contains(&(dx * dx + dy * dy)) { continue; }
                    let (x, y) = (light.x.floor() as i32 + dx, light.y.floor() as i32 + dy);
                    if painted.kind.inside(x, y) && walkable_kind(painted.kind.at(x, y)) {
                        sample = Some((light, x, y));
                        break 'source;
                    }
                }
            }
        }
        let (light, x, y) = sample.expect("a live molten source beside actual walkable bank");
        let dx = light.x - (x as f32 + 0.5);
        let dy = light.y + light.height - (y as f32 + 0.5);
        let length = dx.hypot(dy);
        let facing = [dx / length, dy / length, 0.0];
        let shade = |normal: [f32; 3]| {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::BLACK);
            f.surface_lighting = true;
            f.cam_x = x - 192;
            f.cam_y = y - 108;
            let i = 108 * W + 192;
            f.albedo[i] = Rgb::WHITE;
            f.surfaces[i] = Surface { normal, height: 0.0, owner: 0, kind: 1 };
            f.add_light_3d_w(light.x, light.y + light.height, light.height, light.radius, light.color, light.intensity);
            crate::gfx::surface::apply(&mut f);
            f.light[i].luma()
        };
        let lit_face = shade(facing);
        let dark_face = shade([-facing[0], -facing[1], 0.0]);
        assert!(shade([0.0, 0.0, 1.0]) > 0.02, "live molten light did not reach its real bank");
        assert!(lit_face > dark_face * 3.0, "lava illumination ignored receiving normals");
    }

    #[test]
    fn foundry_spill_turns_into_river_without_a_texture_reset() {
        let line = river_line();
        for y in 636..676 {
            for x in 368..444 {
                let a = surface_coords(x, y, &line);
                let b = surface_coords(x + 1, y, &line);
                assert!(
                    (a.0 - b.0).abs() <= 4 && (a.1 - b.1).abs() <= 4,
                    "flow coordinates jumped at {x},{y}: {a:?} to {b:?}"
                );
            }
        }
        let west = surface_coords(500, 656, &line);
        let east = surface_coords(600, 656, &line);
        assert_eq!(
            west.0 - east.0,
            100,
            "incoming foundry liquid must move west"
        );
        assert_eq!(
            west.1, east.1,
            "straight lined canal must share one crosscurrent coordinate"
        );
    }

    /// A cropped native127px material must stay in phase when flow crosses the baked
    /// sheet's horizontal/vertical wrap. The old1024x256 bake reset by8px/2px there.
    #[test]
    fn native_material_repeats_without_wrap_phase_jumps() {
        let mut cell = crate::gfx::Cell {
            w: 127,
            h: 127,
            surface: Default::default(),
            px: Vec::new(),
        };
        for y in 0..127 {
            for x in 0..127 {
                cell.px.push(crate::gfx::Px {
                    c: LAVA_R[((x + y * 2) % 6) as usize],
                    a: true,
                    emit: false,
                });
            }
        }
        let texture = Texture::native(&cell);
        assert_eq!((texture.period, texture.width), (1016, 254));
        for v in -130..texture.width + 130 {
            for u in [
                -130,
                -1,
                0,
                1,
                126,
                127,
                texture.period - 128,
                texture.period - 1,
                texture.period,
                texture.period + 1,
            ] {
                assert_eq!(
                    texture.sample(u, v),
                    texture.sample(u + 127, v),
                    "downstream phase reset at {u},{v}"
                );
                assert_eq!(
                    texture.sample(u, v),
                    texture.sample(u, v + 127),
                    "cross-current phase reset at {u},{v}"
                );
            }
        }
    }

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
                        first.emissive[i] != Rgb::BLACK
                            || [L0, L0.lerp(B2, 0.42)].contains(&first.albedo[i]),
                        molten
                    );
                    assert_eq!(next.albedo[i] != Rgb::WHITE, molten);
                    assert_eq!(
                        next.emissive[i] != Rgb::BLACK
                            || [L0, L0.lerp(B2, 0.42)].contains(&next.albedo[i]),
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
