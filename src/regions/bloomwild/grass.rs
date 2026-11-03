//! Tufts have rooted blades, a shared breeze, local bending and persistent cut stubble.

use super::{Map, composition, mat, pal};
use crate::combat::Hit;
use crate::fx::{Fx, Particle};
use crate::gfx::{Frame, Rgb, sheet};
use crate::gfx::sprite::SpriteFocus;
use crate::util::hash2;
use crate::world::{RegionCtx, Zone};
use std::f32::consts::TAU;

const ID: u32 = 1;

struct Tuft {
    x: i32,
    y: i32,
    h: i32,
    teal: bool,
    phase: f32,
    bend: f32,
    cut: bool,
    quiet: bool,
    native_variant: usize,
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
        if anim.frames == 0 { return None; }
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

/// Keep recognizable leaf silhouettes apart and spread use across the whole atlas.
/// Assignment happens only at planting time, so cutting or bending a neighbor can
/// never change a surviving tuft's sprite. A small optional atlas may run out of
/// unoccupied variants; then choose the fewest nearby repeats before balancing.
fn native_variants(tufts: &[Tuft], frames: usize) -> Vec<usize> {
    let mut variants = Vec::with_capacity(tufts.len());
    let mut uses = vec![0; frames];
    let mut nearby = vec![0; frames];
    for (i, tuft) in tufts.iter().enumerate() {
        nearby.fill(0);
        for (other, &variant) in tufts[..i].iter().zip(&variants) {
            if (tuft.x - other.x).abs() <= 24 && (tuft.y - other.y).abs() <= 12 {
                nearby[variant] += 1;
            }
        }
        let preferred = (hash2(tuft.x, tuft.y, 537) * frames as f32) as usize % frames;
        let variant = (0..frames).min_by_key(|&v| (
            nearby[v], uses[v], (v + frames - preferred) % frames,
        )).unwrap();
        variants.push(variant);
        uses[variant] += 1;
    }
    variants
}

/// Keep the exact accepted leaf pixel and its rooted height together. Raw put()
/// clears previous surface metadata, so a grass pixel must restore its own
/// normal after replacing the ground. It remains an unowned scenery receiver.
#[allow(clippy::too_many_arguments)]
fn put_leaf(f: &mut Frame, x: i32, y: i32, color: Rgb, normal: [f32; 3], kind: u8, root_y: i32, reveal: Option<SpriteFocus>) {
    let (wx, wy) = (x + f.cam_x, y + f.cam_y);
    // Short, sparse leaves need no ground-clearing window. Thin only the ink
    // actually covering Glint, and retain half the blades over his silhouette.
    if reveal.is_some_and(|focus| root_y as f32 > focus.ground_y
        && focus.contains(wx, wy) && crate::util::bayer4(wx, wy) >= 0.5) {
        return;
    }
    if let Some(i) = Frame::idx(x, y) { f.emissive[i] = Rgb::BLACK; }
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

fn leaf_line(f: &mut Frame, from: (i32, i32), to: (i32, i32), color: Rgb, normal: [f32; 3], root_y: i32, reveal: Option<SpriteFocus>) {
    Frame::line_with(from.0, from.1, to.0, to.1, |x, y| put_leaf(f, x, y, color, normal, 2, root_y, reveal));
}

/// The native leaf fans extend sideways from their small rooted center. Reserve
/// their full moving silhouette beside stone routes, rather than clearing only
/// a tuft's center and letting its leaves draw back across the path.
fn rooted_lawn(mask: &[u8], z: &Zone, x: i32, y: i32) -> bool {
    let material = |px: i32, py: i32| {
        if px < 0 || py < 0 || px >= z.pw || py >= z.ph { return mat::VOID; }
        mask[(py * z.pw + px) as usize]
    };
    if [-4, 0, 4].iter().any(|&dx| {
        material(x + dx, y) != mat::GRASS || z.gget(x + dx, y).r < 0.0
    }) { return false; }
    for dy in -13..=1 {
        for dx in -17..=16 {
            if matches!(material(x + dx, y + dy), mat::PATH | mat::SLAB | mat::BRIDGE | mat::STAIRS) {
                return false;
            }
        }
    }
    true
}

impl Grass {
    /// Place irregular rooted clumps inside the authored meadow patches, clear of solid props.
    pub fn build(m: &Map, mask: &[u8], z: &Zone) -> Self {
        let mut tufts = Vec::new();
        let native_profile = sheet("bloomwild_materials").is_some();
        let density = sheet("bloomwild_materials").and_then(|s| s.json.get("live_grass_density"))
            .and_then(|v| v.as_f64()).unwrap_or(0.36).clamp(0.0, 1.0) as f32;
        let native = if native_profile { NativeGrass::load() } else { None };
        // Reveal the actual opaque roots, including their nonblocking gaps.
        // Planting belongs at their shoulders, never over the spread itself.
        let roots: Vec<_> = z.props.iter().filter(|p| p.sheet == "bloomwild_trees")
            .filter_map(|p| p.collision_bounds()).collect();
        for gy in 1..z.ph / 7 - 1 {
            for gx in 1..z.pw / 8 - 1 {
                let x = gx * 8 + (hash2(gx, gy, 31) * 5.0) as i32;
                let y = gy * 7 + (hash2(gx, gy, 32) * 4.0) as i32;
                // The title is a separately authored headland, not this garden's map.
                let cover = if m.marks.is_empty() { 0.0 } else { composition::cover(x, y) };
                if cover <= 0.0 || !rooted_lawn(mask, z, x, y) {
                    continue;
                }
                if z.blocked(x as f32, y as f32, 4.0) || roots.iter().any(|&[l, t, r, b]| {
                    x as f32 > l - 18.0 && (x as f32) < r + 18.0
                        && y as f32 > t - 3.0 && (y as f32) < b + 16.0
                }) {
                    continue;
                }
                // Concentrate the existing native leaves inside connected beds.
                // Formerly independent density/tile hashes scattered isolated clumps
                // across entire Tall tiles and emptied arbitrary rectangular blocks.
                let probability = if native_profile { (density * 2.4).min(0.62) } else { 0.68 };
                if hash2(gx, gy, 531) > probability * cover { continue; }
                let edge = cover < 1.0;
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
                    quiet: edge,
                    native_variant: 0,
                });
            }
        }
        if let Some(native) = &native {
            let variants = native_variants(&tufts, native.frames.len());
            for (tuft, variant) in tufts.iter_mut().zip(variants) {
                tuft.native_variant = variant;
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
        self.draw_revealed(f, id, t, None);
    }

    /// Keep foreground leaves porous only over the current hero silhouette.
    pub fn draw_revealed(&self, f: &mut Frame, id: u32, t: u64, reveal: Option<SpriteFocus>) {
        if id >> 24 != ID {
            return;
        }
        let Some(tuft) = self.tufts.get((id & 0xff_ffff) as usize) else {
            return;
        };
        let (x, y) = (tuft.x - f.cam_x, tuft.y - f.cam_y);
        if let Some(native) = &self.native {
            let (ax, ay) = native.anchor;
            let wave = (t as f32 * 0.033 + tuft.phase).sin();
            let sway = (wave * 0.7 + tuft.bend * 0.4).round() as i32;
            for leaf in &native.frames[tuft.native_variant] {
                // Root pixels never move; upper leaf clusters move by a whole native pixel.
                let shift = if leaf.y < ay - 4 { sway } else { 0 };
                put_leaf(f, x + leaf.x - ax + shift, y + leaf.y - ay,
                    leaf.color, leaf.normal, leaf.kind, tuft.y, reveal);
            }
            return;
        }
        let ramp: &[Rgb] = if tuft.teal { &pal::TEAL } else { &pal::GRASS };
        let wave = (t as f32 * 0.033 + tuft.x as f32 * 0.021 + tuft.y as f32 * 0.011).sin();
        let sway = (wave * 1.3 + tuft.bend).round() as i32;
        let base = if tuft.quiet { ramp[1] } else { ramp[0] };
        let height = if tuft.quiet { (tuft.h / 2).max(4) } else { tuft.h };
        for py in y - 1..=y {
            for px in x - 3..=x + 3 { put_leaf(f, px, py, base, [0.0, 0.0, 1.0], 2, tuft.y, reveal); }
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
            leaf_line(f, (root, y), (root + lean / 3, y - h / 2), base, normal, tuft.y, reveal);
            leaf_line(f, (root + 1, y - 1), (root + lean / 3 + 1, y - h / 2), ramp[1], normal, tuft.y, reveal);
            leaf_line(f, (root + lean / 3, y - h / 2), (top, tip), ramp[2], normal, tuft.y, reveal);
            if blade % 2 == 0 && !tuft.quiet {
                leaf_line(f, (top, tip), (top + lean.signum(), tip - 2), ramp[3], normal, tuft.y, reveal);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_native_atlases_balance_the_authored_beds_without_nearby_repeats() {
        let map = super::super::Map::parse();
        let mut zone = Zone::new(crate::world::ZoneId::Bloomwild, map.tw, map.th);
        let mask = super::super::paint::paint(&mut zone, &map);
        zone.set_surface_tiles(super::super::paint::physical_surface(&zone, &mask));
        super::super::ruins::build(&mut zone, &map, &mask);
        let grass = Grass::build(&map, &mask, &zone);
        assert!(grass.tufts.len() > 300, "exercise the full authored planting");
        let native = grass.native.as_ref().unwrap();
        assert_eq!(grass.tufts.iter().map(|t| t.native_variant).collect::<Vec<_>>(),
            native_variants(&grass.tufts, native.frames.len()));

        for frames in [12, 16, 20, 24, 32] {
            let variants = native_variants(&grass.tufts, frames);
            assert_eq!(variants, native_variants(&grass.tufts, frames),
                "rebuilding the same garden must preserve its silhouettes");
            let mut uses = vec![0; frames];
            for (i, (tuft, &variant)) in grass.tufts.iter().zip(&variants).enumerate() {
                uses[variant] += 1;
                for (other, &other_variant) in grass.tufts[..i].iter().zip(&variants) {
                    if (tuft.x - other.x).abs() <= 24 && (tuft.y - other.y).abs() <= 12 {
                        assert_ne!(variant, other_variant,
                            "{frames}-frame atlas repeated at {},{} and {},{}",
                            tuft.x, tuft.y, other.x, other.y);
                    }
                }
            }
            assert!(uses.iter().max().unwrap() - uses.iter().min().unwrap() <= 1,
                "every native source should share the planting evenly: {uses:?}");
        }
    }

    #[test]
    fn authored_beds_keep_live_leaf_fans_off_routes_and_expose_native_tree_roots() {
        let map = super::super::Map::parse();
        let mut zone = Zone::new(crate::world::ZoneId::Bloomwild, map.tw, map.th);
        let mask = super::super::paint::paint(&mut zone, &map);
        zone.set_surface_tiles(super::super::paint::physical_surface(&zone, &mask));
        super::super::ruins::build(&mut zone, &map, &mask);
        let grass = Grass::build(&map, &mask, &zone);
        let native = grass.native.as_ref().expect("approved native grass remains active");
        let mut meadow_roots = 0;
        for tuft in &grass.tufts {
            assert!(!zone.blocked(tuft.x as f32, tuft.y as f32, 4.0), "grass base in a solid native foot");
            if (tuft.x - 511).abs() < 44 && (tuft.y - 641).abs() < 30 { meadow_roots += 1; }
            for p in zone.props.iter().filter(|p| p.sheet == "bloomwild_trees") {
                let Some([l, t, r, b]) = p.collision_bounds() else { continue; };
                assert!(tuft.x as f32 <= l - 18.0 || tuft.x as f32 >= r + 18.0
                    || tuft.y as f32 <= t - 3.0 || tuft.y as f32 >= b + 16.0,
                    "foreground planting buried a native root spread");
            }
            for frame in &native.frames {
                for leaf in frame {
                    for sway in [-2, 0, 2] {
                        let x = tuft.x + leaf.x - native.anchor.0
                            + if leaf.y < native.anchor.1 - 4 { sway } else { 0 };
                        let y = tuft.y + leaf.y - native.anchor.1;
                        if x < 0 || y < 0 || x >= zone.pw || y >= zone.ph { continue; }
                        assert!(!matches!(mask[(y * zone.pw + x) as usize],
                            mat::PATH | mat::SLAB | mat::BRIDGE | mat::STAIRS),
                            "swaying native leaf crossed a quiet stone route at {x},{y}");
                    }
                }
            }
        }
        assert!(meadow_roots >= 20, "the encounter needs a visible, cuttable planting mass: {meadow_roots}");
        // Optional exact source-to-runtime placement evidence for the review tool.
        // Ordinary test runs leave the working tree untouched.
        if let Ok(path) = std::env::var("GLINT_VEGETATION_EXPORT") {
            let data = serde_json::json!({
                "native_grid": [zone.pw, zone.ph],
                "roots": grass.tufts.iter().map(|t| [t.x, t.y]).collect::<Vec<_>>(),
                "native_variants": grass.tufts.iter().map(|t| t.native_variant).collect::<Vec<_>>(),
                "native_frame_count": native.frames.len(),
                "tree_support": zone.props.iter().filter(|p| p.sheet == "bloomwild_trees")
                    .filter_map(|p| p.collision_bounds()).collect::<Vec<_>>(),
                "notes": "Exact runtime grass bases after terrain, stone-route, solid-prop and exposed-tree-root checks. Native source pixels, normals, sort keys and gameplay unchanged."
            });
            std::fs::write(path, serde_json::to_string_pretty(&data).unwrap() + "\n").unwrap();
        }
    }

    #[test]
    fn opening_combat_clearings_exclude_whole_moving_native_fans() {
        let map = super::super::Map::parse();
        let mut zone = Zone::new(crate::world::ZoneId::Bloomwild, map.tw, map.th);
        let mask = super::super::paint::paint(&mut zone, &map);
        zone.set_surface_tiles(super::super::paint::physical_surface(&zone, &mask));
        super::super::ruins::build(&mut zone, &map, &mask);
        let grass = Grass::build(&map, &mask, &zone);
        let native = grass.native.as_ref().expect("approved native grass remains active");
        let mut retained_bed_roots = 0;
        for tuft in &grass.tufts {
            if (tuft.x - 511).abs() < 44 && (tuft.y - 641).abs() < 30 {
                retained_bed_roots += 1;
            }
            for frame in &native.frames {
                for leaf in frame {
                    for sway in [-2, 0, 2] {
                        let x = tuft.x + leaf.x - native.anchor.0
                            + if leaf.y < native.anchor.1 - 4 { sway } else { 0 };
                        let y = tuft.y + leaf.y - native.anchor.1;
                        for &(name, (cx, cy, rx, ry)) in &composition::CLEARINGS[..3] {
                            let (dx, dy) = ((x - cx) as f32 / rx as f32,
                                (y - cy) as f32 / ry as f32);
                            assert!(dx * dx + dy * dy >= 1.0,
                                "native fan rooted at {},{} covers {name} at {x},{y}",
                                tuft.x, tuft.y);
                        }
                    }
                }
            }
        }
        assert!(retained_bed_roots >= 20,
            "opening clearance must retain the nearby cuttable bed: {retained_bed_roots}");
    }

    fn one_tuft(native: bool) -> Grass {
        let mut grass = Grass {
            tufts: vec![Tuft { x: 100, y: 80, h: 11, teal: false, phase: 0.3,
                bend: 1.25, cut: false, quiet: false, native_variant: 0 }],
            native: native.then(|| NativeGrass::load().expect("actual native Bloomwild grass asset")),
        };
        if let Some(native) = &grass.native {
            grass.tufts[0].native_variant = native_variants(&grass.tufts, native.frames.len())[0];
        }
        grass
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

    fn hero_frame(focus: SpriteFocus, physical: bool, camera: (i32, i32)) -> Frame {
        let mut f = Frame::new();
        f.cam_x = camera.0;
        f.cam_y = camera.1;
        f.begin(Rgb::BLACK, Rgb::WHITE);
        f.surface_lighting = physical;
        f.sprite_w(focus.cell, focus.anchor, focus.x, focus.y,
            &crate::gfx::DrawOpts { actor: true, flip: focus.flip, ..Default::default() });
        f
    }

    fn check_reveal(base: &Frame, opaque: &Frame, revealed: &Frame, focus: SpriteFocus, minimum: usize) {
        let mut hidden = 0;
        let mut recovered = 0;
        let mut context_recovered = 0;
        for i in 0..base.albedo.len() {
            if base.actor_pixels[i] && !opaque.actor_pixels[i] {
                hidden += 1;
                if revealed.actor_pixels[i] {
                    recovered += 1;
                    assert_eq!(revealed.albedo[i], base.albedo[i]);
                    assert_eq!(revealed.emissive[i], base.emissive[i]);
                    assert_eq!(revealed.surfaces[i].normal, base.surfaces[i].normal);
                    assert_eq!(revealed.surfaces[i].height, base.surfaces[i].height);
                    assert_eq!(revealed.surfaces[i].owner, base.surfaces[i].owner);
                    assert_eq!(revealed.surfaces[i].kind, base.surfaces[i].kind);
                }
            }
            let (wx, wy) = (i as i32 % crate::gfx::W as i32 + base.cam_x,
                i as i32 / crate::gfx::W as i32 + base.cam_y);
            if !focus.contains(wx, wy) {
                assert_eq!(revealed.albedo[i], opaque.albedo[i], "grass outside the actual hero ink must remain opaque");
                assert_eq!(revealed.surfaces[i].normal, opaque.surfaces[i].normal);
                assert_eq!(revealed.surfaces[i].height, opaque.surfaces[i].height);
                assert_eq!(revealed.surfaces[i].owner, opaque.surfaces[i].owner);
                assert_eq!(revealed.surfaces[i].kind, opaque.surfaces[i].kind);
            }
            if !base.actor_pixels[i] && opaque.albedo[i] != base.albedo[i]
                && revealed.albedo[i] == base.albedo[i] {
                context_recovered += 1;
            }
        }
        assert!(hidden >= minimum, "fixture must exercise actual foreground leaves: {hidden}");
        assert!(recovered * 3 >= hidden && recovered * 4 <= hidden * 3,
            "grass must reveal the hero while retaining visible blades: {recovered}/{hidden}");
        assert_eq!(context_recovered, 0, "grass must not disappear from the surrounding ground");
    }

    #[test]
    fn opaque_leaves_hide_underlying_glow_and_context_holes_preserve_it() {
        let hero = sheet("hero").unwrap();
        let anim = hero.anim("idle_down").unwrap();
        let focus = SpriteFocus { cell: hero.cell(anim, 0), anchor: hero.anchor_of(anim),
            flip: false, x: 100.0, y: 76.0, ground_y: 76.0 };
        let glow = crate::gfx::rgb(0x7edbda);
        let (mut drawn, mut skipped) = (0, 0);
        let mut frame = Frame::new();
        frame.begin(Rgb::BLACK, Rgb::WHITE);
        frame.surface_lighting = true;
        for y in 45..90 { for x in 65..135 {
            let i = y as usize * crate::gfx::W + x as usize;
            frame.put_glow(x, y, glow);
            put_leaf(&mut frame, x, y, Rgb::WHITE, [0.0, 0.0, 1.0], 2, 80, Some(focus));
            if frame.albedo[i] == Rgb::WHITE {
                drawn += 1;
                assert_eq!(frame.emissive[i], Rgb::BLACK, "opaque leaf cannot retain the hidden glow");
                assert_eq!(frame.surfaces[i].kind, 2);
            } else {
                skipped += 1;
                assert_eq!(frame.emissive[i], glow, "a context hole keeps the underlying material");
                assert_eq!(frame.surfaces[i].kind, 0);
            }
        } }
        assert!(drawn > 100 && skipped > 100);
    }

    #[test]
    fn foreground_native_and_procedural_grass_reveal_color_and_actor_surfaces_together() {
        let hero = sheet("hero").unwrap();
        let anim = hero.anim("idle_down").unwrap();
        let focus = SpriteFocus { cell: hero.cell(anim, 0), anchor: hero.anchor_of(anim),
            flip: false, x: 100.0, y: 76.0, ground_y: 76.0 };
        for native in [false, true] { for physical in [false, true] {
            let grass = one_tuft(native);
            let base = hero_frame(focus, physical, (7, 9));
            let mut opaque = hero_frame(focus, physical, (7, 9));
            let mut revealed = hero_frame(focus, physical, (7, 9));
            grass.draw(&mut opaque, ID << 24, 61);
            grass.draw_revealed(&mut revealed, ID << 24, 61, Some(focus));
            check_reveal(&base, &opaque, &revealed, focus, 12);


            for ground_y in [80.0, 80.1, 81.0, 86.0, 92.0] {
                let in_front = SpriteFocus { ground_y, ..focus };
                let mut background = hero_frame(focus, physical, (7, 9));
                grass.draw_revealed(&mut background, ID << 24, 61, Some(in_front));
                assert_eq!(background.albedo, opaque.albedo, "background grass faded at {ground_y}");
                assert_eq!(background.emissive, opaque.emissive);
                assert_eq!(background.actor_pixels, opaque.actor_pixels);
            }
            let distant = SpriteFocus { x: focus.x + 100.0, ..focus };
            let mut background = hero_frame(focus, physical, (7, 9));
            grass.draw_revealed(&mut background, ID << 24, 61, Some(distant));
            assert_eq!(background.albedo, opaque.albedo, "leaves outside the context opening keep their native ink");
        }}
    }

    #[test]
    fn legal_meadow_probe_reveals_native_grass_through_the_region_hook() {
        let (zone, region) = super::super::build();
        let hero = sheet("hero").unwrap();
        let anim = hero.anim("idle_down").unwrap();
        let focus = SpriteFocus { cell: hero.cell(anim, 0), anchor: hero.anchor_of(anim),
            flip: false, x: 500.0, y: 624.0, ground_y: 624.0 };
        assert!(!zone.blocked(focus.x, focus.ground_y, 4.0), "probe needs legal hero ground");
        let base = hero_frame(focus, true, (307, 517));
        let mut opaque = hero_frame(focus, true, (307, 517));
        let mut revealed = hero_frame(focus, true, (307, 517));
        let mut items = Vec::new();
        region.sorted_items(&zone, &opaque, &mut items);
        for (y, id) in items {
            if y <= focus.ground_y { continue; }
            region.draw_item(&zone, &mut opaque, id, 90);
            region.draw_item_revealed(&zone, &mut revealed, id, 90, Some(focus));
        }
        check_reveal(&base, &opaque, &revealed, focus, 100);
    }

    #[test]
    fn native_leaf_normals_follow_actual_pixels_sway_and_world_roots() {
        let mut grass = one_tuft(true);
        let frame_count = grass.native.as_ref().unwrap().frames.len();
        for variant in 0..frame_count {
            grass.tufts[0].native_variant = variant;
            let physical = draw(&grass, true);
            let legacy = draw(&grass, false);
            assert_eq!(physical.albedo, legacy.albedo, "lighting metadata must preserve every authored color and accepted alpha pixel");
            assert!(legacy.surfaces.iter().all(|s| s.kind == 0));
            let native = grass.native.as_ref().unwrap();
            let tuft = &grass.tufts[0];
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
