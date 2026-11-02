//! Tufts have rooted blades, a shared breeze, local bending and persistent cut stubble.

use super::{K, Map, mat, pal};
use crate::combat::Hit;
use crate::fx::{Fx, Particle};
use crate::gfx::{Frame, Rgb, sheet};
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
    native: Option<NativeGrass>,
}

struct NativeGrass {
    anchor: (i32, i32),
    // Native cells have substantial transparent padding. Cache only their actual leaf pixels.
    frames: Vec<Vec<LeafPixel>>,
}

struct LeafPixel {
    x: i32,
    y: i32,
    color: Rgb,
    normal: [f32; 3],
    kind: u8,
}

impl NativeGrass {
    fn load() -> Option<Self> {
        let s = sheet("bloomwild_grass").filter(|s| (s.cell_w, s.cell_h) == (32, 32))?;
        let anim = s.anim("body")?;
        let frames = (0..anim.frames).map(|i| {
            let cell = s.cell(anim, i);
            let mut pixels = Vec::new();
            for y in 0..cell.h {
                for x in 0..cell.w {
                    let p = cell.at(x, y);
                    if !p.a { continue; }
                    let ink = p.c.luma() < 0.105
                        && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                            .iter().any(|&(x, y)| !cell.at(x, y).a);
                    pixels.push(LeafPixel {
                        x, y, color: p.c,
                        // These low leaf beds expose their upper surfaces.
                        // Blue is the bed's up axis, rather than the south-
                        // facing axis used for upright walls and tree trunks.
                        normal: cell.surface.get((y * cell.w + x) as usize)
                            .map_or([0.0, 0.0, 1.0], |n| n.floor()),
                        kind: if ink { 4 } else { 2 },
                    });
                }
            }
            pixels
        }).collect();
        Some(Self { anchor: s.anchor_of(anim), frames })
    }
}

/// Keep the exact accepted leaf pixel and its rooted height together. Raw put()
/// clears previous surface metadata, so a grass pixel must restore its own
/// normal after replacing the ground. It remains an unowned scenery receiver.
fn put_leaf(f: &mut Frame, x: i32, y: i32, color: Rgb, normal: [f32; 3], kind: u8, root_y: i32) {
    f.put(x, y, color);
    if f.surface_lighting && let Some(i) = Frame::idx(x, y) {
        f.surfaces[i] = crate::gfx::frame::Surface {
            normal,
            height: (root_y - y - f.cam_y).max(0) as f32,
            owner: 0,
            kind,
        };
    }
}

fn leaf_line(f: &mut Frame, from: (i32, i32), to: (i32, i32), color: Rgb, normal: [f32; 3], root_y: i32) {
    Frame::line_with(from.0, from.1, to.0, to.1, |x, y| put_leaf(f, x, y, color, normal, 2, root_y));
}

impl Grass {
    /// Place irregular rooted clumps inside the authored meadow patches, clear of solid props.
    pub fn build(m: &Map, mask: &[u8], z: &Zone) -> Self {
        let mut tufts = Vec::new();
        let native_profile = sheet("bloomwild_materials").is_some();
        let density = sheet("bloomwild_materials").and_then(|s| s.json.get("live_grass_density"))
            .and_then(|v| v.as_f64()).unwrap_or(0.36).clamp(0.0, 1.0) as f32;
        let native = if native_profile { NativeGrass::load() } else { None };
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
                if native_profile && hash2(gx, gy, 531) > density { continue; }
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
                    h: if native_profile {
                        (if edge { 4 } else { 6 }) + (hash2(gx, gy, 33) * 4.0) as i32
                    } else {
                        (if edge { 5 } else { 8 }) + (hash2(gx, gy, 33) * 5.0) as i32
                    },
                    teal,
                    phase: hash2(gx, gy, 34) * TAU,
                    bend: 0.0,
                    cut: false,
                    quiet: quiet_clearing(x, y),
                });
            }
        }
        Self { tufts, native }
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
            if !tuft.cut && f.visible(tuft.x as f32 - 16.0, tuft.y as f32 - 28.0, 32.0, 32.0, 2.0) {
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
        if let Some(native) = &self.native {
            let variant = (hash2(tuft.x, tuft.y, 537) * native.frames.len() as f32) as usize % native.frames.len();
            let (ax, ay) = native.anchor;
            let wave = (t as f32 * 0.033 + tuft.phase).sin();
            let sway = (wave * 0.7 + tuft.bend * 0.4).round() as i32;
            for leaf in &native.frames[variant] {
                // Root pixels never move; upper leaf clusters move by a whole native pixel.
                let shift = if leaf.y < ay - 4 { sway } else { 0 };
                put_leaf(f, x + leaf.x - ax + shift, y + leaf.y - ay,
                    leaf.color, leaf.normal, leaf.kind, tuft.y);
            }
            return;
        }
        let ramp: &[Rgb] = if tuft.teal { &pal::TEAL } else { &pal::GRASS };
        let wave = (t as f32 * 0.033 + tuft.x as f32 * 0.021 + tuft.y as f32 * 0.011).sin();
        let sway = (wave * 1.3 + tuft.bend).round() as i32;
        let base = if tuft.quiet { ramp[1] } else { ramp[0] };
        let height = if tuft.quiet { (tuft.h / 2).max(4) } else { tuft.h };
        for py in y - 1..=y {
            for px in x - 3..=x + 3 { put_leaf(f, px, py, base, [0.0, 0.0, 1.0], 2, tuft.y); }
        }
        for blade in 0i32..5 {
            if tuft.quiet && (blade == 0 || blade == 4) {
                continue;
            }
            let root = x + (blade - 2) * 2;
            let h = height - (blade - 2).abs() * 2;
            let lean = (blade - 2) + sway;
            let top = root + lean;
            let tip = y - h + (tuft.phase.sin() * 1.0).round() as i32;
            let nx = (blade - 2) as f32 * 0.20;
            let ny = lean as f32 * 0.05;
            let scale = (nx * nx + ny * ny + 1.0).sqrt().recip();
            let normal = [-nx * scale, -ny * scale, scale];
            leaf_line(f, (root, y), (root + lean / 3, y - h / 2), base, normal, tuft.y);
            leaf_line(f, (root + 1, y - 1), (root + lean / 3 + 1, y - h / 2), ramp[1], normal, tuft.y);
            leaf_line(f, (root + lean / 3, y - h / 2), (top, tip), ramp[2], normal, tuft.y);
            if blade % 2 == 0 && !tuft.quiet {
                leaf_line(f, (top, tip), (top + lean.signum(), tip - 2), ramp[3], normal, tuft.y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_tuft(native: bool) -> Grass {
        Grass {
            tufts: vec![Tuft { x: 100, y: 80, h: 11, teal: false, phase: 0.3,
                bend: 1.25, cut: false, quiet: false }],
            native: native.then(|| NativeGrass::load().expect("actual native Bloomwild grass asset")),
        }
    }

    fn draw(grass: &Grass, physical: bool) -> Frame {
        let mut frame = Frame::new();
        frame.cam_x = 7;
        frame.cam_y = 9;
        frame.begin(Rgb::BLACK, Rgb::new(0.36, 0.39, 0.46));
        frame.surface_lighting = physical;
        grass.draw(&mut frame, ID << 24, 61);
        frame
    }

    #[test]
    fn native_leaf_normals_follow_actual_pixels_sway_and_world_roots() {
        let grass = one_tuft(true);
        let physical = draw(&grass, true);
        let legacy = draw(&grass, false);
        assert_eq!(physical.albedo, legacy.albedo, "lighting metadata must preserve every authored color and accepted alpha pixel");
        assert!(legacy.surfaces.iter().all(|s| s.kind == 0));
        let native = grass.native.as_ref().unwrap();
        let tuft = &grass.tufts[0];
        let variant = (hash2(tuft.x, tuft.y, 537) * native.frames.len() as f32) as usize % native.frames.len();
        let (ax, ay) = native.anchor;
        let sway = ((61.0_f32 * 0.033 + tuft.phase).sin() * 0.7 + tuft.bend * 0.4).round() as i32;
        let source = sheet("bloomwild_grass").unwrap();
        let cell = source.cell(source.anim("body").unwrap(), variant);
        for leaf in &native.frames[variant] {
            let shift = if leaf.y < ay - 4 { sway } else { 0 };
            let px = tuft.x - physical.cam_x + leaf.x - ax + shift;
            let py = tuft.y - physical.cam_y + leaf.y - ay;
            let i = Frame::idx(px, py).unwrap();
            let surface = physical.surfaces[i];
            assert_eq!(physical.albedo[i], cell.at(leaf.x, leaf.y).c);
            assert_eq!(surface.normal, cell.surface[(leaf.y * cell.w + leaf.x) as usize].floor());
            assert_eq!(surface.height, (ay - leaf.y).max(0) as f32);
            assert!(matches!(surface.kind, 2 | 4), "opaque leaves must remain live sunlight receivers");
            assert_eq!(surface.owner, 0);
            assert!(!physical.actor_pixels[i]);
        }
        assert_eq!(physical.surfaces.iter().filter(|s| s.kind != 0).count(), native.frames[variant].len(),
            "transparent padding cannot gain phantom receiving pixels");
    }

    #[test]
    fn opposing_real_leaf_normals_respond_in_opposite_directions_to_the_moving_sun() {
        let grass = one_tuft(true);
        let mut first = draw(&grass, true);
        let mut later = draw(&grass, true);
        first.normal_sun = true;
        later.normal_sun = true;
        first.t = 540;
        later.t = 1620;
        crate::gfx::surface::apply(&mut first);
        crate::gfx::surface::apply(&mut later);
        let mut brighter = 0;
        let mut darker = 0;
        for (i, surface) in first.surfaces.iter().enumerate() {
            if surface.kind != 2 { continue; }
            let delta = later.light[i].luma() - first.light[i].luma();
            if delta > 0.015 { brighter += 1; }
            if delta < -0.015 { darker += 1; }
        }
        assert!(brighter >= 8 && darker >= 8,
            "a real sun arc must change opposing native leaf facets differently: brighter={brighter}, darker={darker}");
        assert_eq!(first.albedo, later.albedo, "only source incidence changes in this fixed-pose test");
    }

    #[test]
    fn procedural_blades_preserve_legacy_pixels_and_restore_their_own_raised_surfaces() {
        let grass = one_tuft(false);
        let physical = draw(&grass, true);
        let legacy = draw(&grass, false);
        assert_eq!(physical.albedo, legacy.albedo);
        assert!(legacy.surfaces.iter().all(|s| s.kind == 0));
        let mut raised = 0;
        for (i, &color) in physical.albedo.iter().enumerate() {
            if color == Rgb::BLACK { continue; }
            let surface = physical.surfaces[i];
            assert_eq!(surface.kind, 2);
            assert_eq!(surface.height, (80 - (i / crate::gfx::W) as i32 - physical.cam_y).max(0) as f32);
            assert!(surface.normal[2] > 0.85);
            if surface.height > 0.0 { raised += 1; }
        }
        assert!(raised > 20, "actual procedural blade pixels must receive at their own rooted height");
    }
}
