//! Asset scenery in authored groups, with grounded native feet and separated crown shade.

use super::{Map, mat, pal, tc};
use crate::util::hash2;
use crate::world::{Prop, Zone};

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
    bake_base_contact(z, mask, &p);
    z.props.push(p);
}

/// Bed a real native foot into its material. The narrow, broken contact follows
/// opaque support columns; it cannot become an oval decal hanging over the sky.
fn bake_base_contact(z: &mut Zone, mask: &[u8], p: &Prop) {
    if !matches!(p.sheet, "bloomwild_trees" | "bloomwild_sapling" | "bloomwild_ruins" | "bloomwild_arch" | "bloomwild_aqueduct" | "round13_landmarks_garden") {
        return;
    }
    let Some(s) = p.sheet() else { return };
    let Some(a) = s.anim(&p.anim) else { return };
    let Some([left, _, right, foot]) = p.collision_bounds() else { return };
    let cell = s.cell(a, 0);
    let (ax, ay) = s.anchor_of(a);
    let base_x = p.x.floor() as i32;
    let base_y = p.y.floor() as i32;
    for wx in left.floor() as i32..right.ceil() as i32 {
        let sx = if p.flip { cell.w - 1 - (wx - base_x + ax) } else { wx - base_x + ax };
        let Some(bottom) = (0..cell.h).rev().find(|&y| cell.at(sx, y).a) else { continue };
        let wy = base_y + bottom - ay;
        if wy < foot as i32 - 6 { continue; }
        for d in 1..=2 {
            let y = wy + d;
            if wx < 0 || y < 0 || wx >= z.pw || y >= z.ph { continue; }
            let i = (y * z.pw + wx) as usize;
            if z.ground[i].r < 0.0 { continue; }
            let h = hash2(wx, y, 864);
            match mask[i] {
                mat::GRASS if d == 1 || h > 0.66 => {
                    // Damp root/stone contact, then a few warm soil grains and
                    // moss patches. Existing lawn color still shows through.
                    let shade = if d == 1 { pal::TEAL[0] } else { pal::EARTH[1] };
                    z.ground[i] = z.ground[i].lerp(shade, if d == 1 { 0.48 } else { 0.22 });
                }
                mat::SLAB | mat::PATH if d == 1 && h > 0.28 => {
                    z.ground[i] = z.ground[i].lerp(pal::EARTH[1], 0.25);
                }
                _ => {}
            }
        }
    }
}

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
                place(z, mask, "bloomwild_ruins", anim, x, y, 7, 38);
            }
            'q' => place(
                z,
                mask,
                "bloomwild_ruins",
                if hash2(tx, ty, 3) > 0.5 {
                    "pillar_broken_short"
                } else {
                    "pillar_broken_tall"
                },
                x,
                y,
                7,
                18,
            ),
            'r' => place(z, mask, "bloomwild_props", "boulder_large", x, y, 11, 10),
            'w' | 'W' => {
                // Adjacent map marks describe a wall course, not overlapping full sprites.
                if (tx + ty) % 2 == 0 {
                    place(
                        z,
                        mask,
                        "bloomwild_ruins",
                        if ch == 'W' { "wall_tall" } else { "wall_low" },
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
    opening(z, mask, &mut canopies);
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
fn opening(z: &mut Zone, mask: &[u8], canopies: &mut Vec<Prop>) {
    // The landing camera sees y=588..804: keep the tall fin's crown inside that view.
    // Leave daylight between the fins even with the artist's wider upper masses.
    // A shorter surviving pier and low channel make a single broken waterworks mass.
    let aqueduct = crate::gfx::sheet("bloomwild_aqueduct").is_some_and(|s| s.has("left_pier") && s.has("right_pier") && s.has("canopy"));
    if crate::gfx::sheet("round13_landmarks_garden").is_some_and(|s| s.has("body")) {
        // One native distributor and open conduits form a single landmark.
        // Exact separated foot spans leave the main arch passable; native
        // foreground reveal keeps Glint readable behind the elevated trough.
        place(z, mask, "round13_landmarks_garden", "body", 335.0, 710.0, 0, 0);
    } else if aqueduct {
        // Native source anchor is (64,83); pier anchors follow each actual foot rather than
        // blocking the arch's empty center with one broad rectangular collision footprint.
        let body = Prop::new("bloomwild_aqueduct", "body", 335.0, 710.0);
        bake_prop_shadow(z, mask, &body);
        for (anim,x) in [("left_pier",299.0),("right_pier",370.0)] {
            let pier = Prop::new("bloomwild_aqueduct", anim, x, 710.0);
            bake_base_contact(z, mask, &pier);
            z.props.push(pier);
        }
        let mut canopy = Prop::new("bloomwild_aqueduct", "canopy", 335.0, 710.0);
        canopy.canopy = true;
        canopies.push(canopy);
    } else {
        place(z, mask, "bloomwild_waterworks", "fin", 299.0, 710.0, 6, 109);
        place(z, mask, "bloomwild_waterworks", "fin", 335.0, 720.0, 6, 109);
        place(z, mask, "bloomwild_ruins", "pillar", 362.0, 715.0, 7, 44);
        place(z, mask, "bloomwild_ruins", "wall_low", 331.0, 716.0, 15, 20);
        place(z, mask, "bloomwild_ruins", "drum", 277.0, 725.0, 12, 8);
    }

    // The exposed trunk stays left of the path; the crown points toward the ruin.
    place(z, mask, "bloomwild_trees", "a_trunk", 110.0, 678.0, 6, 70);
    let mut crown = Prop::new("bloomwild_trees", "a_canopy", 110.0, 678.0);
    crown.canopy = true;
    canopies.push(crown);
    place(
        z,
        mask,
        "bloomwild_ruins",
        "pillar_broken_short",
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
