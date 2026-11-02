//! Asset scenery in authored groups, with grounded native feet and separated crown shade.

use super::{Map, mat, pal, tc};
use crate::util::hash2;
use crate::world::{Prop, Zone};

/// Static appearances are chosen before foot contact is baked. Nearby ruins
/// avoid the same broken outline, while least-used choices keep the full set visible.
#[derive(Default)]
struct RuinVariety {
    placed: Vec<(&'static str, &'static str, f32, f32)>,
}

impl RuinVariety {
    fn choose(&mut self, base: &'static str, x: f32, y: f32) -> &'static str {
        let names: &[&'static str] = match base {
            "pillar_broken_short" => &[
                "pillar_broken_short", "pillar_broken_short_b", "pillar_broken_short_c",
                "pillar_broken_short_d", "pillar_broken_short_e", "pillar_broken_short_f",
            ],
            "wall_low" => &[
                "wall_low", "wall_low_b", "wall_low_c", "wall_low_d", "wall_low_e", "wall_low_f",
            ],
            _ => return base,
        };
        let Some(sheet) = crate::gfx::sheet("bloomwild_ruins") else { return base };
        let available: Vec<_> = names.iter().copied().filter(|name| sheet.has(name)).collect();
        if available.is_empty() { return base; }
        let preferred = (hash2(x as i32, y as i32, 781) * available.len() as f32) as usize % available.len();
        let (_, selected) = available.iter().enumerate().min_by_key(|&(i, name)| {
            let mut nearby = 0;
            let mut uses = 0;
            for &(kind, prior, px, py) in &self.placed {
                if kind == base && prior == *name {
                    uses += 1;
                    if (x - px).abs() <= 192.0 && (y - py).abs() <= 108.0 { nearby += 1; }
                }
            }
            (nearby, uses, (i + available.len() - preferred) % available.len())
        }).unwrap();
        self.placed.push((base, selected, x, y));
        selected
    }
}

/// Place scenery with static ground shade; breakables receive removable renderer shadows.
fn place(
    z: &mut Zone,
    mask: &[u8],
    sheet: &'static str,
    anim: &str,
    x: f32,
    y: f32,
    _radius: i32,
    _height: i32,
) {
    let p = Prop::new(sheet, anim, x, y);
    bake_prop_shadow(z, mask, &p);
    z.props.push(p);
}

// Ground-contact ink is corrected on the authored sprite support, without
// painting a fixed teal/soil underline into the receiving landscape. Physical
// native cast shadows continue through the lighting field below.

fn bake_prop_shadow(z: &mut Zone, mask: &[u8], p: &Prop) {
    // Both ordinary placement and aqueduct openings use this helper. A baked
    // fixed cast would remain behind when the physical sun moves its silhouette.
    if !crate::gfx::lighting::legacy_mode() { return; }
    // Ordinary native scenery already casts through the cached lighting field. Only
    // overlay crowns/spans live outside that field; baking stone again doubled the
    // shade and left the temple with repeated long dark parallelograms.
    let overlay = if let Some(tree) = p.anim.strip_suffix("_trunk") {
        format!("{tree}_canopy")
    } else if p.sheet == "bloomwild_aqueduct" && p.anim == "body" {
        "canopy".to_owned()
    } else {
        return;
    };
    if !p.breakable && let Some(s) = p.sheet() {
        if let Some(a) = s.anim(&overlay) {
            let cell = s.cell(a, 0);
            let (ax, ay) = s.anchor_of(a);
            let base = s.anim(&p.anim).and_then(|body| {
                let cell = s.cell(body, 0);
                (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).a))
                    .map(|bottom| p.y.floor() + (bottom + 1 - s.anchor_of(body).1) as f32)
            }).unwrap_or(p.y);
            let source_foot = (base - p.y.floor()) as i32 + ay - 1;
            for py in 0..cell.h {
                for px in 0..cell.w {
                    if !cell.at(px, py).a {
                        continue;
                    }
                    let altitude = (source_foot - py).max(0);
                    let xx = p.x.floor() as i32 + px - ax + (altitude as f32 * 0.55).round() as i32;
                    let yy = base.floor() as i32 + (altitude as f32 * 0.27).round() as i32;
                    if xx < 0 || yy < 0 || xx >= z.pw || yy >= z.ph {
                        continue;
                    }
                    let i = (yy * z.pw + xx) as usize;
                    // The turf mask retains the original tile category at an eroded skyline.
                    // Cast shade can darken existing ground, never paint ground back over sky.
                    if z.ground[i].r < 0.0 { continue; }
                    let c = match mask[i] {
                        mat::GRASS => if super::paint::sheltered_shadow(z, xx, yy) { pal::TEAL[0] } else { pal::GRASS[1] },
                        mat::PATH => pal::EARTH[2],
                        mat::SLAB | mat::BRIDGE | mat::STAIRS => pal::STONE[2],
                        _ => continue,
                    };
                    z.ground[i] = c;
                }
            }
        }
    }
}

/// A complete tree and broken ruin rooted in the title's broad garden headland.
pub(super) fn title_garden(z: &mut Zone, mask: &[u8]) -> Vec<Prop> {
    place(z, mask, "bloomwild_trees", "f_trunk", 286.0, 132.0, 0, 0);
    place(z, mask, "bloomwild_ruins", "pillar_broken_tall", 342.0, 114.0, 0, 0);
    let mut canopy = Prop::new("bloomwild_trees", "f_canopy", 286.0, 132.0);
    canopy.canopy = true;
    vec![canopy]
}

/// Build the broken architecture and tree groves from explicit map landmarks.
pub fn build(z: &mut Zone, m: &Map, mask: &[u8]) -> Vec<Prop> {
    let mut canopies = Vec::new();
    let mut variety = RuinVariety::default();
    let mut tree_index = 0;
    for &(ch, tx, ty) in &m.marks {
        let (x, y) = match (ch, tx, ty) {
            // Native opaque roots/feet must fit the rounded finished shore,
            // including the few landmarks formerly balanced over empty sky.
            ('p', 22, 3) => (369.0, 63.0),
            ('p', 28, 3) => (449.0, 63.0),
            ('T' | 'Y', 6, 5) => (125.0, 93.0),
            ('T' | 'Y', 5, 30) => (104.0, 483.0),
            _ => tc(tx, ty),
        };
        match ch {
            'T' | 'Y' => {
                // Authored skyline groups mix upright ivory trees, broad rose
                // crowns, gnarled survivors and one young tree. A hash-only
                // selection put the same silhouette side by side too often.
                let variant = match (tx, ty) {
                    (6, 5) => "c", (60, 5) => "f", (69, 5) => "d",
                    (38, 6) => "b", (52, 7) => "g", (66, 9) => "a",
                    (73, 12) => "f", (63, 13) => "c", (56, 15) => "g",
                    (12, 17) => "g", (76, 17) => "e", (58, 23) => "b",
                    (40, 27) => "c", (5, 30) => "f", (77, 34) => "g",
                    (15, 39) => "b", (75, 41) => "a", (40, 42) => "d",
                    _ => ["a", "f", "c", "g", "b", "d"][tree_index % 6],
                };
                // The young tree has its own native crown; all mature trees
                // retain their existing atlas, density and physical roots.
                let sheet = if variant == "e" { "bloomwild_sapling" } else { "bloomwild_trees" };
                place(
                    z,
                    mask,
                    sheet,
                    &format!("{variant}_trunk"),
                    x,
                    y,
                    19,
                    45,
                );
                let mut canopy = Prop::new(sheet, &format!("{variant}_canopy"), x, y);
                canopy.canopy = true;
                canopies.push(canopy);
                tree_index += 1;
            }
            'p' => {
                let variants = ["pillar", "pillar_b", "pillar_c"];
                let candidate = match (tx, ty) {
                    (19, 5) | (66, 25) => "pillar_broken_tall",
                    (34, 13) | (66, 31) => "pillar_broken_short",
                    _ => variants[(hash2(tx, ty, 702) * 3.0) as usize % 3],
                };
                let anim = if crate::gfx::sheet("bloomwild_ruins").is_some_and(|s| s.has(candidate)) { candidate } else { "pillar" };
                let anim = variety.choose(anim, x, y);
                place(z, mask, "bloomwild_ruins", anim, x, y, 7, 38);
            }
            'q' => {
                let anim = variety.choose(if hash2(tx, ty, 3) > 0.5 {
                    "pillar_broken_short"
                } else {
                    "pillar_broken_tall"
                }, x, y);
                place(z, mask, "bloomwild_ruins", anim, x, y, 7, 18);
            }
            'r' => place(z, mask, "bloomwild_props", "boulder_large", x, y, 11, 10),
            'w' | 'W' => {
                // Adjacent map marks describe a wall course, not overlapping full sprites.
                if (tx + ty) % 2 == 0 {
                    let anim = variety.choose(if ch == 'W' { "wall_tall" } else { "wall_low" }, x, y);
                    place(
                        z,
                        mask,
                        "bloomwild_ruins",
                        anim,
                        x,
                        y,
                        15,
                        20,
                    );
                }
            }
            _ => {}
        }
    }
    // The entry arch fits the three-tile path and reads before the first fight.
    let (x, y) = tc(9, 38);
    for (dx, side) in [(-17.0, "arch_post_left"), (17.0, "arch_post_right")] {
        let anim = if crate::gfx::sheet("bloomwild_arch").is_some_and(|s| s.has(side)) { side } else { "arch_post" };
        place(z, mask, "bloomwild_arch", anim, x + dx, y, 6, 40);
    }
    let mut span = Prop::new("bloomwild_arch", "arch_span", x, y);
    span.canopy = true;
    canopies.push(span);
    opening(z, mask, &mut canopies, &mut variety);
    // A lone statue at the lower waterfall is the route's quiet overlook.
    place(z, mask, "bloomwild_ruins", "statue", 776.0, 650.0, 10, 38);
    // Urns sit beside the path and column bases, never across the route.
    for (tx, ty, anim) in [
        (11, 40, "urn_small"),
        (22, 33, "urn_large"),
        (35, 33, "urn_small"),
        (61, 30, "urn_large"),
        (73, 26, "urn_small"),
        (33, 14, "urn_large"),
        (19, 7, "urn_small"),
    ] {
        let (x, y) = tc(tx, ty);
        place(z, mask, "bloomwild_props", anim, x, y, 5, 8);
        if let Some(p) = z.props.last_mut() {
            p.heal_drop = 1;
        }
    }
    // Shrub masses finish the same eleven authored beds as the live grass. Keep
    // long path and altar fields quiet, and expose the actual tree/stone roots.
    for (tx, ty, anim) in [
        (28, 39, "bush_a"),
        (34, 41, "flowers_b"),
        (24, 28, "bush_b"),
        (31, 28, "bush_bloom"),
        (39, 27, "bush_b"),
        (61, 24, "bush_a"),
        (75, 27, "bush_b"),
        (77, 28, "flowers_b"),
        (59, 6, "bush_bloom"),
        (61, 6, "flowers_a"),
        (58, 11, "bush_b"),
        (63, 13, "flowers_b"),
        (73, 7, "bush_a"),
        (74, 8, "flowers_a"),
        (8, 13, "bush_b"),
        (11, 15, "bush_a"),
        (10, 24, "bush_b"),
    ] {
        let (x, y) = tc(tx, ty);
        place(z, mask, "bloomwild_props", anim, x, y, 8, 6);
    }
    // A toppled drum explains the broken colonnade rather than adding another intact pillar.
    place(z, mask, "bloomwild_ruins", "drum", 1102.0, 528.0, 12, 8);
    // A few fallen chips next to the broken temple edge.
    for (x, y) in [
        (1000, 506),
        (1004, 509),
        (1100, 515),
        (1205, 408),
        (1210, 412),
    ] {
        for dy in 0..2 {
            for dx in 0..3 {
                z.gset(x + dx, y + dy, pal::STONE[3]);
            }
        }
    }
    canopies
}

/// Frame the landing with a complete distributor ruin and a rooted wind-combed tree.
fn opening(z: &mut Zone, mask: &[u8], canopies: &mut Vec<Prop>, variety: &mut RuinVariety) {
    // The landing camera sees y=588..804: keep the tall fin's crown inside that view.
    // Leave daylight between the fins even with the artist's wider upper masses.
    // A shorter surviving pier and low channel make a single broken waterworks mass.
    let aqueduct = crate::gfx::sheet("bloomwild_aqueduct").is_some_and(|s| s.has("left_pier") && s.has("right_pier") && s.has("canopy"));
    if aqueduct {
        // Native source anchor is (64,83); pier anchors follow each actual foot rather than
        // blocking the arch's empty center with one broad rectangular collision footprint.
        let body = Prop::new("bloomwild_aqueduct", "body", 335.0, 710.0);
        bake_prop_shadow(z, mask, &body);
        for (anim,x) in [("left_pier",299.0),("right_pier",370.0)] {
            let pier = Prop::new("bloomwild_aqueduct", anim, x, 710.0);
            z.props.push(pier);
        }
        let mut canopy = Prop::new("bloomwild_aqueduct", "canopy", 335.0, 710.0);
        canopy.canopy = true;
        canopies.push(canopy);
    } else {
        place(z, mask, "bloomwild_waterworks", "fin", 299.0, 710.0, 6, 109);
        place(z, mask, "bloomwild_waterworks", "fin", 335.0, 720.0, 6, 109);
        place(z, mask, "bloomwild_ruins", "pillar", 362.0, 715.0, 7, 44);
        let anim = variety.choose("wall_low", 331.0, 716.0);
        place(z, mask, "bloomwild_ruins", anim, 331.0, 716.0, 15, 20);
        place(z, mask, "bloomwild_ruins", "drum", 277.0, 725.0, 12, 8);
    }

    // The exposed trunk stays left of the path; the crown points toward the ruin.
    place(z, mask, "bloomwild_trees", "a_trunk", 110.0, 678.0, 6, 70);
    let mut crown = Prop::new("bloomwild_trees", "a_canopy", 110.0, 678.0);
    crown.canopy = true;
    canopies.push(crown);
    let stump = variety.choose("pillar_broken_short", 102.0, 722.0);
    place(
        z,
        mask,
        "bloomwild_ruins",
        stump,
        102.0,
        722.0,
        7,
        18,
    );
    for (x, y, anim) in [(114.0, 681.0, "flowers_a"), (437.0, 710.0, "bush_b")] {
        place(z, mask, "bloomwild_props", anim, x, y, 8, 6);
    }
    if crate::gfx::sheet("bloomwild_growth").is_some_and(|s| s.has("body")) {
        // One larger planting anchors the waterfall ruin; low tufts keep the landing readable.
        place(z, mask, "bloomwild_growth", "body", 773.0, 667.0, 0, 0);
    }
    if crate::gfx::sheet("bloomwild_growth_low").is_some_and(|s| s.has("body")) {
        for (x, y) in [(113.0, 685.0), (390.0, 719.0), (312.0, 714.0)] {
            place(z, mask, "bloomwild_growth_low", "body", x, y, 0, 0);
        }
    }
}

/// Fade a canopy only when its painted leaves actually cover Glint, rather than its empty cell.
pub fn draw_canopies(canopies: &[Prop], f: &mut crate::gfx::Frame, player: Option<(f32, f32)>, t: u64, reveal: Option<crate::gfx::sprite::SpriteFocus>) {
    for p in canopies {
        if !p.alive { continue; }
        let Some((x, y, w, h)) = p.bounds() else {
            continue;
        };
        if !f.surface_lighting && !f.visible(x, y, w, h, 4.0) {
            continue;
        }
        if reveal.is_some() {
            p.draw_revealed(f, t, 1.0, reveal);
            continue;
        }
        let Some(s) = p.sheet() else {
            continue;
        };
        let Some(anim) = s.anim(&p.anim) else {
            continue;
        };
        let cell = s.cell(anim, anim.frame_at(t as u32 + p.t0, true));
        let under = player.is_some_and(|player| [(-3.0, -12.0), (3.0, -12.0), (0.0, -8.0)]
            .iter()
            .any(|&(dx, dy)| {
                let source_x = (player.0 + dx).floor() as i32 - x.floor() as i32;
                cell.at(if p.flip { cell.w - 1 - source_x } else { source_x },
                    (player.1 + dy).floor() as i32 - y.floor() as i32)
                    .a
            }));
        p.draw(f, t, if under { 0.35 } else { 1.0 });
    }
}

#[cfg(test)]
mod variety_tests {
    use super::*;
    use crate::gfx::{Frame, Rgb, sheet};

    #[test]
    fn static_ruin_shapes_are_distributed_before_contact_baking() {
        let (zone, _) = crate::regions::bloomwild::build();
        for base in ["pillar_broken_short", "wall_low"] {
            let props: Vec<_> = zone.props.iter().filter(|p| p.sheet == "bloomwild_ruins"
                && (p.anim == base || p.anim.starts_with(&format!("{base}_")))).collect();
            assert_eq!(props.len(), 11, "{base}: keep the authored object population");
            let mut counts = std::collections::BTreeMap::new();
            for p in &props {
                *counts.entry(&p.anim).or_insert(0) += 1;
                let s = p.sheet().unwrap();
                assert_eq!(s.anim(&p.anim).unwrap().frames, 1, "stone appearances cannot animate");
            }
            assert!(counts.len() >= 5, "{base}: use the new native shapes");
            assert!(counts.values().all(|&n| n <= 3), "{base}: too many copies of one shape: {counts:?}");
            for (i, a) in props.iter().enumerate() {
                for b in &props[i + 1..] {
                    if (a.x - b.x).abs() <= 192.0 && (a.y - b.y).abs() <= 108.0 {
                        assert_ne!(a.anim, b.anim, "nearby {base} repeat at {},{} and {},{}", a.x, a.y, b.x, b.y);
                    }
                }
            }
        }
    }

    #[test]
    fn every_appended_stone_keeps_its_support_and_upright_shading() {
        let s = sheet("bloomwild_ruins").unwrap();
        for (prefix, first_preserved_row) in [("pillar_broken_short", 40), ("wall_low", 45)] {
            let reference_anim = s.anim(prefix).unwrap();
            let reference = s.cell(reference_anim, 0);
            let original = Prop::new("bloomwild_ruins", prefix, 180.0, 170.0);
            let mut checked = 0;
            for a in s.anims.values().filter(|a| a.name.starts_with(&format!("{prefix}_"))) {
                checked += 1;
                let cell = s.cell(a, 0);
                let p = Prop::new("bloomwild_ruins", &a.name, 180.0, 170.0);
                assert_eq!(p.sort_y(), original.sort_y());
                assert_eq!(p.collision_bounds(), original.collision_bounds());
                for y in first_preserved_row..cell.h {
                    for x in 0..cell.w {
                        let (before, after) = (reference.at(x, y), cell.at(x, y));
                        assert_eq!((before.a, before.c, before.emit), (after.a, after.c, after.emit));
                        assert_eq!(reference.surface[(y * cell.w + x) as usize].n,
                            cell.surface[(y * cell.w + x) as usize].n);
                    }
                }
                let mut f = Frame::new();
                f.begin(Rgb::BLACK, Rgb::WHITE);
                f.surface_lighting = true;
                p.draw(&mut f, 0, 1.0);
                assert_eq!(f.surface_casters.len(), 1, "{} needs a native caster", a.name);
                assert_eq!(f.surface_casters[0].base, p.sort_y(), "{} detached shadow", a.name);
                let (ax, ay) = s.anchor_of(a);
                let samples = a.json.get("normal_samples").and_then(|v| v.as_array())
                    .expect("new masonry needs authored cap/front/side acceptance points");
                assert!(samples.len() >= 3, "{}: too few geometric face checks", a.name);
                for sample in samples {
                    let x = sample["xy"][0].as_i64().unwrap() as i32;
                    let y = sample["xy"][1].as_i64().unwrap() as i32;
                    assert!(cell.at(x, y).a);
                    let screen = (170 + y - ay) as usize * crate::gfx::W + (180 + x - ax) as usize;
                    let actual = f.surfaces[screen].normal;
                    let expected: [f32; 3] = std::array::from_fn(|i| sample["world_normal"][i].as_f64().unwrap() as f32);
                    let length = actual.iter().map(|n| n * n).sum::<f32>().sqrt();
                    let dot = actual.iter().zip(expected).map(|(a, b)| a * b).sum::<f32>() / length;
                    assert!(dot > 0.999, "{}/{}: wrong physical face normal {:?}", a.name, sample["plane"], actual);
                }
                for y in 0..cell.h { for x in 0..cell.w {
                    if !cell.at(x, y).a { continue; }
                    let screen = (170 + y - ay) as usize * crate::gfx::W + (180 + x - ax) as usize;
                    assert_eq!(f.surfaces[screen].normal,
                        cell.surface[(y * cell.w + x) as usize].upright(false), "{} uses wrong normal basis", a.name);
                } }
            }
            assert!(checked >= 4, "{prefix}: missing native variants");
        }
    }

    #[test]
    fn original_and_variant_feet_stamp_the_authored_shared_surface_planes() {
        let s = sheet("bloomwild_ruins").unwrap();
        let profiles = s.json.get("shared_foot_normals").and_then(|v| v.as_array())
            .expect("original masonry feet need authored surface-plane profiles");
        assert_eq!(profiles.len(), 2, "cover both shared structural feet");
        let alignment = |actual: [f32; 3], expected: [f32; 3]| {
            let length = actual.iter().map(|n| n * n).sum::<f32>().sqrt();
            actual.iter().zip(expected).map(|(a, b)| a * b).sum::<f32>() / length
        };
        let mut sample_count = 0;
        for (template, first_row, regression, minimum_shapes) in [
            ("pillar_broken_short", 40, [20, 50], 5),
            ("wall_low", 45, [34, 46], 6),
        ] {
            let profile = profiles.iter().find(|p| p["template"].as_str() == Some(template))
                .expect("every original structural foot needs its own profile");
            assert_eq!(profile["first_row"].as_i64(), Some(first_row));
            let samples = profile["samples"].as_array().expect("shared foot acceptance points");
            assert!(!samples.is_empty(), "{template}: empty shared foot profile");
            sample_count += samples.len();
            let regression_sample = samples.iter().find(|sample| {
                sample["xy"][0].as_i64() == Some(regression[0])
                    && sample["xy"][1].as_i64() == Some(regression[1])
            }).expect("retain the formerly upward-facing vertical-front regression point");
            for (axis, expected) in [0.0, 1.0, 0.0].into_iter().enumerate() {
                assert_eq!(regression_sample["world_normal"][axis].as_f64(), Some(expected),
                    "{template}: the broad vertical front must face south, not upward");
            }

            let prefix = format!("{template}_");
            let mut checked = 0;
            for a in s.anims.values().filter(|a| a.name == template || a.name.starts_with(&prefix)) {
                checked += 1;
                let cell = s.cell(a, 0);
                let (ax, ay) = s.anchor_of(a);
                for flip in [false, true] {
                    let prop = Prop::new("bloomwild_ruins", &a.name, 180.0, 170.0).flipped(flip);
                    let mut f = Frame::new();
                    f.begin(Rgb::BLACK, Rgb::WHITE);
                    f.surface_lighting = true;
                    prop.draw(&mut f, 0, 1.0);
                    for sample in samples {
                        let x = sample["xy"][0].as_i64().unwrap() as i32;
                        let y = sample["xy"][1].as_i64().unwrap() as i32;
                        assert!(y >= first_row as i32 && y < cell.h && x >= 0 && x < cell.w,
                            "{}: shared-foot sample outside the retained support", a.name);
                        assert!(cell.at(x, y).a, "{}: shared-foot sample must cover real stone", a.name);
                        let mut expected: [f32; 3] = std::array::from_fn(|i| {
                            sample["world_normal"][i].as_f64().unwrap() as f32
                        });
                        if flip { expected[0] = -expected[0]; }
                        let decoded = cell.surface[(y * cell.w + x) as usize].upright(flip);
                        assert!(alignment(decoded, expected) > 0.999,
                            "{}/{} at {x},{y}: decoded foot face {:?}, expected {:?}",
                            a.name, sample["plane"], decoded, expected);
                        let px = 180 + if flip { ax - x } else { x - ax };
                        let surface = f.surfaces[Frame::idx(px, 170 + y - ay).unwrap()];
                        assert_ne!(surface.kind, 0, "{}: Prop::draw omitted the foot surface", a.name);
                        assert_eq!(surface.normal, decoded,
                            "{}: Prop::draw changed the decoded shared-foot normal, flip={flip}", a.name);
                        assert!(alignment(surface.normal, expected) > 0.999,
                            "{}/{}: rendered shared-foot face uses the wrong world basis", a.name, sample["plane"]);
                    }
                }
            }
            assert!(checked >= minimum_shapes, "{template}: original or named variants are missing");
        }
        assert!(sample_count >= 33, "retain the authored cap, bevel and vertical-front checks");
    }

    #[test]
    fn entire_extended_atlases_load_the_exact_paired_normal_texels() {
        use crate::assets::{decode_png, get};
        use crate::gfx::surface::NormalPixel;
        for name in ["bloomwild_grass", "bloomwild_ruins"] {
            let (w, h, art) = decode_png(get(&format!("sprites/{name}.png")).unwrap()).unwrap();
            let (nw, nh, normals) = decode_png(get(&format!("normals/{name}.png")).unwrap()).unwrap();
            assert_eq!((w, h), (nw, nh), "{name}: a mismatch silently discards ALL native normals");
            assert!(art.chunks_exact(4).zip(normals.chunks_exact(4)).all(|(a, n)| a[3] == n[3]));
            let s = sheet(name).unwrap();
            for a in s.anims.values() { for frame in 0..a.frames {
                let cell = s.cell(a, frame);
                for y in 0..cell.h { for x in 0..cell.w {
                    let p = cell.at(x, y);
                    if !p.a { continue; }
                    let uv = (((a.row as i32 * cell.h + y) * w as i32 + frame as i32 * cell.w + x) * 4) as usize;
                    let expected = NormalPixel::decode([normals[uv], normals[uv + 1], normals[uv + 2]], true, p.emit);
                    assert_eq!(cell.surface[(y * cell.w + x) as usize].n, expected.n,
                        "{name}/{} frame{frame} at{x},{y}: wrong or fallback normal", a.name);
                } }
            } }
        }
    }
}
