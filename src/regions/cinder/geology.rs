//! Cooled volcanic crust and settled ash share one retained material relief.
//! Irregular, partly buried plates have calm tops and narrow broken flanks;
//! pigment never invents geometry that the receiving normals do not describe.

use super::art::{RA, RB};
use super::field::Grid;
use super::paint::{K_ASH, K_GRASS, K_PATH, P};
use crate::gfx::terrain_detail::{self, Material};
use crate::util::{hash2, noise2};

pub(super) struct Crust {
    relief: Grid<f32>,
    exposed: Grid<f32>,
    edge: Grid<f32>,
}

pub(super) fn dry(kind: u8) -> bool {
    matches!(kind, K_ASH | K_GRASS | K_PATH)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Nearest two unequal crust nuclei in a gently sheared flow coordinate system.
/// Their shared bisector is a break, rather than an outline around a placed rock.
fn plate(x: f32, y: f32) -> (f32, f32, f32, f32, f32) {
    let u = x * 0.82
        + y * 0.34
        + (noise2(x / 13.0, y / 9.0, 0xc231) - 0.5) * 3.2
        + (noise2(x / 95.0, y / 71.0, 0xc211) - 0.5) * 9.0;
    let v = y * 1.85 - x * 0.11
        + (noise2(x / 11.0, y / 8.0, 0xc232) - 0.5) * 3.0
        + (noise2(x / 79.0, y / 83.0, 0xc212) - 0.5) * 6.0;
    let (gx, gy) = ((u / 35.0).floor() as i32, (v / 35.0).floor() as i32);
    let mut first = (f32::MAX, 0.0, 0.0, 0, 0);
    let mut second = first;
    for cy in gy - 2..=gy + 2 {
        for cx in gx - 2..=gx + 2 {
            if hash2(cx, cy, 0xc218) < 0.23 {
                continue;
            }
            let sx = (cx as f32 + 0.15 + hash2(cx, cy, 0xc213) * 0.70) * 35.0;
            let sy = (cy as f32 + 0.15 + hash2(cx, cy, 0xc214) * 0.70) * 35.0;
            let d = (u - sx).powi(2) + (v - sy).powi(2);
            let site = (d, sx, sy, cx, cy);
            if d < first.0 {
                second = first;
                first = site;
            } else if d < second.0 {
                second = site;
            }
        }
    }
    let separation = (first.1 - second.1).hypot(first.2 - second.2);
    let edge = (second.0 - first.0) / (2.0 * separation.max(1.0));
    let id = hash2(first.3, first.4, 0xc215);
    let tilt = (u - first.1) * (hash2(first.3, first.4, 0xc216) - 0.5) * 0.055
        + (v - first.2) * (hash2(first.3, first.4, 0xc217) - 0.5) * 0.035;
    (edge, id, tilt, u - first.1, v - first.2)
}

/// Paint natural substrate before roads/masonry/props take their own exact masks.
/// Keep this relief separate: road wear must not cut it off at a map-kind boundary.
pub(super) fn paint(
    px: &mut Grid<P>,
    kind: &Grid<u8>,
    lava_dist: &Grid<f32>,
    rock_dist: &Grid<f32>,
) -> Crust {
    let mut crust = Crust {
        relief: Grid::new(px.w, px.h, 0.0),
        exposed: Grid::new(px.w, px.h, 0.0),
        edge: Grid::new(px.w, px.h, 100.0),
    };
    for y in 0..px.h {
        for x in 0..px.w {
            let i = (y * px.w + x) as usize;
            if !dry(kind.v[i]) {
                continue;
            }
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let (edge, id, tilt, _, _) = plate(fx, fy);
            // Broad burial varies gradually; it does not cut a hard color polygon.
            // Near the live bank, less ash survives on the exposed cooled crust.
            let burial = noise2(fx / 135.0, fy / 91.0, 0xc220) * 0.72
                + noise2(fx / 51.0, fy / 39.0, 0xc221) * 0.28;
            let bank = (1.0 - lava_dist.v[i] / 58.0).clamp(0.0, 1.0);
            let exposed = (1.0 - smooth(0.31, 0.64, burial) * 0.94 + bank * 0.17).clamp(0.0, 1.0);
            // The broad top is almost planar. Steep normals belong only to its
            // 1–3px chipped edge, never a rounded cushion across the plate.
            let stress = smooth(0.27, 0.71, noise2(fx / 25.0, fy / 21.0, 0xc223));
            let rise = (edge / (1.0 + id * 1.2)).clamp(0.0, 1.0);
            let height = (0.65 + id * 0.4 + tilt).max(0.25) * (1.0 - (1.0 - rise) * stress);
            // Fractured, angular clinker lives at a much smaller scale than a
            // whole flow raft. One connected field drives its value clusters and slopes.
            let clink = noise2(fx / 7.5 + fy / 31.0, fy / 4.8 - fx / 43.0, 0xc222);
            let clink2 = noise2(fx / 3.9 - fy / 23.0, fy / 3.0, 0xc224);
            let grain = clink * 0.76 + clink2 * 0.24;
            let grains = ((clink - 0.47).abs() * 1.0 + (clink2 - 0.5) * 0.23) * exposed;
            let mut folded = 0.0;
            let mut flow_cover: f32 = 0.0;
            let mut fold_ink = None;
            for (cx, cy, rx, ry, bend, phase, axis) in [
                (132.0, 748.0, 98.0, 43.0, 14.0, 0.0, 0.13),
                (845.0, 335.0, 119.0, 37.0, 17.0, 0.0, -0.08),
                (910.0, 347.0, 76.0, 30.0, -13.0, 4.0, 0.22),
                (264.0, 654.0, 93.0, 37.0, -14.0, 2.0, -0.17),
                (1035.0, 584.0, 109.0, 47.0, 15.0, 4.0, 0.06),
                (1062.0, 462.0, 61.0, 47.0, 15.0, 4.0, 0.21),
                (1141.0, 355.0, 79.0, 40.0, -12.0, 3.0, -0.14),
            ] {
                let u = (fx - cx) / rx;
                let v = fy - cy - (fx - cx) * axis - u * u * bend;
                let fade = (1.0 - u * u - (v / ry).powi(2)).max(0.0);
                if fade <= 0.0 {
                    continue;
                }
                let drift = noise2(fx / 28.0, fy / 21.0, 0xc225) * 3.0;
                let period = 11.0
                    + hash2(cx as i32, cy as i32, 0xc227) * 5.0
                    + u * 1.7
                    + (noise2(fx / 67.0, fy / 41.0, 0xc228) - 0.5) * 2.5;
                let cycle = (v + drift + phase) / period;
                let band = cycle.floor() as i32;
                let q = cycle.rem_euclid(1.0) * period;
                let strength = if hash2(cx as i32, band, 0xc229) < 0.30 {
                    0.12
                } else {
                    0.75 + hash2(cy as i32, band, 0xc230) * 0.45
                };
                let broken = 0.20 + smooth(0.26, 0.65, noise2(fx / 41.0, fy / 17.0, 0xc226)) * 0.80;
                let strength = strength * broken;
                let distance = q.min(period - q);
                let ridge = (1.0 - (q / period * std::f32::consts::TAU).cos()) * 0.5;
                // Broad pressure lobes are gently convex beneath their cooling
                // wrinkles. They carry form light, not a flat colored silhouette.
                flow_cover = flow_cover.max(fade);
                folded += fade * fade * 4.2 + (ridge * strength - 0.35) * 1.65 * fade;
                if fade > 0.27
                    && strength > 0.62
                    && distance < 0.8
                    && noise2(fx / 18.0, fy / 12.0, 0xc226) > 0.35
                {
                    fold_ink = Some(P { r: RB, l: 1 });
                } else if fade > 0.48 && strength > 0.62 && (1.0..1.8).contains(&distance) {
                    fold_ink = Some(P { r: RB, l: 2 });
                }
            }
            crust.relief.v[i] = height * exposed * (1.0 - flow_cover * 0.72) + grains + folded;
            crust.exposed.v[i] = exposed;
            crust.edge.v[i] = edge;
            // Two adjacent basalt/ash steps describe the weathered plate tops.
            // Only exposed openings get dark cores: many joints are filled or
            // welded, avoiding a wall-to-wall cobblestone outline network.
            let mut ink = if exposed > 0.44 {
                P {
                    r: if id > 0.36 { RB } else { RA },
                    l: 2,
                }
            } else {
                P { r: RA, l: 2 }
            };
            if exposed > 0.72 && grain < 0.19 {
                ink = P { r: RB, l: 1 };
            } else if exposed > 0.80 && grain > 0.81 {
                ink = P { r: RB, l: 3 };
            }
            if flow_cover < 0.25 && exposed > 0.72 && edge < 0.43 && id > 0.40 && stress > 0.83 {
                ink = P { r: RA, l: 1 };
            } else if flow_cover < 0.25 && exposed > 0.58 && edge < 1.05 && stress > 0.65 {
                ink = P { r: RB, l: 1 };
            } else if flow_cover < 0.25
                && exposed > 0.82
                && edge < 1.8
                && id > 0.90
                && stress > 0.84
            {
                // A few fresh, broken facets, not a bright outline around every face.
                ink = P { r: RB, l: 3 };
            } else if exposed < 0.30 && edge < 3.4 {
                // Ash settles in the low joint; it is a cover, not a raised stripe.
                ink = P { r: RA, l: 2 };
            }
            if let Some(fold) = fold_ink {
                ink = fold;
            }
            // Retain the original narrow contact soot and molten-bank crust.
            if (lava_dist.v[i] < 7.0 || rock_dist.v[i] < 3.0) && px.v[i].r == RA {
                ink.l = ink.l.min(px.v[i].l);
            }
            px.v[i] = ink;
        }
    }
    crust
}

impl Crust {
    /// Exposed lava skin keeps its own coherent facets/folds. Loose ash receives
    /// subdued fine grains, instead of turf/earth noise under the same rock color.
    pub(super) fn finish_normal(&self, x: i32, y: i32, base: [f32; 3]) -> [f32; 3] {
        let grain = terrain_detail::normal(Material::Ash, x, y, base);
        let weight = 0.10 + (1.0 - self.exposed.at(x, y)) * 0.65;
        std::array::from_fn(|i| base[i] * (1.0 - weight) + grain[i] * weight)
    }

    /// Traffic wears the same crust. Dark joints continue through the ash road;
    /// a quiet center is only one ash step above its shoulder, without a bright strip.
    pub(super) fn path(&self, x: i32, y: i32, center: bool) -> P {
        let edge = self.edge.at(x, y);
        let exposed = self.exposed.at(x, y) * 0.65;
        let stress = noise2(x as f32 / 25.0, y as f32 / 21.0, 0xc223);
        if edge < 0.55 && exposed > 0.54 && stress > 0.68 {
            P { r: RA, l: 1 }
        } else if edge < 1.5 && exposed > 0.45 && stress > 0.60 {
            P { r: RA, l: 2 }
        } else {
            P {
                r: RA,
                l: if center { 3 } else { 2 },
            }
        }
    }

    /// A plate continues across ash/grass/path tags; only actual substrate masks
    /// stop the derivative. Architectural and lava geometry remains independent.
    pub(super) fn normal(&self, kind: &Grid<u8>, x: i32, y: i32) -> [f32; 3] {
        if !dry(kind.at(x, y)) {
            return [0.0, 0.0, 1.0];
        }
        let center = self.relief.at(x, y);
        let sample = |nx, ny| {
            if kind.inside(nx, ny) && dry(kind.at(nx, ny)) {
                self.relief.at(nx, ny)
            } else {
                center
            }
        };
        [
            -(sample(x + 1, y) - sample(x - 1, y)) * 0.5,
            -(sample(x, y + 1) - sample(x, y - 1)) * 0.5,
            1.0,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::super::paint::{K_BANK, K_DAIS, K_DECK, K_LAVA, K_PAVE, K_ROCKTOP};
    use super::*;

    #[test]
    fn crust_changes_only_natural_substrate_and_is_deterministic() {
        let mut ink = Grid::new(257, 129, P { r: RA, l: 2 });
        let mut kind = Grid::new(ink.w, ink.h, K_ASH);
        let materials = [
            K_ASH, K_GRASS, K_PATH, K_PAVE, K_LAVA, K_BANK, K_ROCKTOP, K_DECK, K_DAIS,
        ];
        for (i, k) in kind.v.iter_mut().enumerate() {
            *k = materials[i % materials.len()];
        }
        let before = ink.clone();
        let far = Grid::new(ink.w, ink.h, 100.0);
        let first = paint(&mut ink, &kind, &far, &far);
        let mut again = before.clone();
        let second = paint(&mut again, &kind, &far, &far);
        assert_eq!(ink.v, again.v);
        assert_eq!(first.relief.v, second.relief.v);
        for (i, &k) in kind.v.iter().enumerate() {
            if !dry(k) {
                assert_eq!(ink.v[i], before.v[i]);
                assert_eq!(first.relief.v[i], 0.0);
            }
            assert!(first.relief.v[i].is_finite());
        }
        assert!(ink.v.iter().zip(&before.v).filter(|(a, b)| a != b).count() > 1000);
    }

    #[test]
    fn buried_road_tags_do_not_create_a_new_geometric_seam() {
        let mut ink = Grid::new(257, 129, P { r: RA, l: 2 });
        let kind = Grid::new(ink.w, ink.h, K_ASH);
        let far = Grid::new(ink.w, ink.h, 100.0);
        let crust = paint(&mut ink, &kind, &far, &far);
        let mut tagged = kind.clone();
        for y in 0..tagged.h {
            for x in 0..tagged.w {
                tagged.set(x, y, if x < 128 { K_GRASS } else { K_PATH });
            }
        }
        for y in 1..kind.h - 1 {
            for x in 1..kind.w - 1 {
                assert_eq!(
                    crust.normal(&kind, x, y),
                    crust.normal(&tagged, x, y),
                    "map tags bent the same cooled sheet at {x},{y}"
                );
            }
        }
        // A neighboring lava pixel must not pull a dry receiving normal toward
        // an unrelated field; the real molten bank owns its own geometry.
        tagged.set(128, 64, K_LAVA);
        assert_eq!(crust.normal(&tagged, 128, 64), [0.0, 0.0, 1.0]);
        let center = crust.relief.at(127, 64);
        assert_eq!(
            crust.normal(&tagged, 127, 64)[0],
            -(center - crust.relief.at(126, 64)) * 0.5
        );
    }
}
