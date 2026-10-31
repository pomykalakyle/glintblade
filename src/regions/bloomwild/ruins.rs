//! Asset scenery in authored groups, with upper-left cast shadows baked into the floor.

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
    // Project the real leaf/stone silhouette down-right into a compressed hard cast shadow.
    // Repeated source pixels write one material color, so projection cannot darken a pixel twice.
    if !p.breakable && let Some(s) = p.sheet() {
        let shadow_anim = anim.strip_suffix("_trunk").map(|v| format!("{v}_canopy"));
        for a in [
            s.anim(anim),
            shadow_anim.as_deref().and_then(|name| s.anim(name)),
        ]
        .into_iter()
        .flatten()
        {
            let cell = s.cell(a, 0);
            let (ax, ay) = s.anchor_of(a);
            for py in 0..cell.h {
                for px in 0..cell.w {
                    if !cell.at(px, py).a {
                        continue;
                    }
                    let altitude = (ay - py).max(0);
                    let xx = x as i32 + px - ax + altitude / 3;
                    let yy = y as i32 + altitude / 4;
                    if xx < 0 || yy < 0 || xx >= z.pw || yy >= z.ph {
                        continue;
                    }
                    let i = (yy * z.pw + xx) as usize;
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
    z.props.push(p);
}

/// Build the broken architecture and tree groves from explicit map landmarks.
pub fn build(z: &mut Zone, m: &Map, mask: &[u8]) -> Vec<Prop> {
    let mut canopies = Vec::new();
    let mut tree_index = 0;
    for &(ch, tx, ty) in &m.marks {
        let (x, y) = tc(tx, ty);
        match ch {
            'T' | 'Y' => {
                let variant = if ch == 'T' {
                    if tree_index % 3 == 0 { "b" } else { "a" }
                } else if tree_index % 2 == 0 {
                    "c"
                } else {
                    "d"
                };
                place(
                    z,
                    mask,
                    "bloomwild_trees",
                    &format!("{variant}_trunk"),
                    x,
                    y,
                    19,
                    45,
                );
                let mut canopy = Prop::new("bloomwild_trees", &format!("{variant}_canopy"), x, y);
                canopy.canopy = true;
                canopies.push(canopy);
                tree_index += 1;
            }
            'p' => place(z, mask, "bloomwild_ruins", "pillar", x, y, 7, 38),
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
    for dx in [-17.0, 17.0] {
        place(z, mask, "bloomwild_arch", "arch_post", x + dx, y, 6, 40);
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
    // Small shrubs cluster at architecture and roots, instead of peppering every grass tile.
    for (tx, ty, anim) in [
        (13, 40, "bush_b"),
        (16, 40, "flowers_a"),
        (21, 39, "bush_a"),
        (35, 39, "flowers_b"),
        (19, 32, "bush_b"),
        (39, 28, "bush_bloom"),
        (38, 26, "bush_b"),
        (61, 24, "bush_a"),
        (74, 32, "bush_b"),
        (78, 35, "flowers_b"),
        (59, 6, "bush_bloom"),
        (61, 6, "flowers_a"),
        (64, 14, "bush_b"),
        (11, 16, "bush_a"),
        (14, 18, "flowers_a"),
        (18, 17, "bush_b"),
        (32, 6, "flowers_b"),
        (74, 13, "bush_a"),
    ] {
        let (x, y) = tc(tx, ty);
        place(z, mask, "bloomwild_props", anim, x, y, 8, 6);
    }
    // A toppled drum explains the broken colonnade rather than adding another intact pillar.
    place(z, mask, "bloomwild_ruins", "drum", 1103.0, 505.0, 12, 8);
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
    place(z, mask, "bloomwild_waterworks", "fin", 299.0, 710.0, 6, 109);
    place(z, mask, "bloomwild_waterworks", "fin", 335.0, 720.0, 6, 109);
    place(z, mask, "bloomwild_ruins", "pillar", 362.0, 715.0, 7, 44);
    place(z, mask, "bloomwild_ruins", "wall_low", 331.0, 716.0, 15, 20);
    place(z, mask, "bloomwild_ruins", "drum", 277.0, 725.0, 12, 8);

    // The exposed trunk stays left of the path; the crown points toward the ruin.
    place(z, mask, "bloomwild_trees", "a_trunk", 100.0, 676.0, 6, 70);
    let mut crown = Prop::new("bloomwild_trees", "a_canopy", 100.0, 676.0);
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
    for (x, y, anim) in [(114.0, 681.0, "flowers_a"), (377.0, 715.0, "bush_b")] {
        place(z, mask, "bloomwild_props", anim, x, y, 8, 6);
    }
}

/// Fade a canopy only when its painted leaves actually cover Glint, rather than its empty cell.
pub fn draw_canopies(canopies: &[Prop], f: &mut crate::gfx::Frame, player: (f32, f32), t: u64) {
    for p in canopies {
        let Some((x, y, w, h)) = p.bounds() else {
            continue;
        };
        if !f.visible(x, y, w, h, 4.0) {
            continue;
        }
        let Some(s) = p.sheet() else {
            continue;
        };
        let Some(anim) = s.anim(&p.anim) else {
            continue;
        };
        let cell = s.cell(anim, anim.frame_at(t as u32 + p.t0, true));
        let under = [(-3.0, -12.0), (3.0, -12.0), (0.0, -8.0)]
            .iter()
            .any(|&(dx, dy)| {
                cell.at((player.0 + dx - x) as i32, (player.1 + dy - y) as i32)
                    .a
            });
        p.draw(f, t, if under { 0.35 } else { 1.0 });
    }
}
