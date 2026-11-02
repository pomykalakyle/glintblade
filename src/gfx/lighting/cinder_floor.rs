//! Baked, slab-shaped reception from Cinder's visible brazier, with quiet court margins.

use super::{Rect, Rgb, Zone, ZoneId, rgb};
use crate::world::Tile;

const EDGES: &[&[(f32, f32)]] = &[
    &[(456.0, 396.0), (540.0, 396.0), (540.0, 432.0), (508.0, 448.0), (496.0, 472.0), (496.0, 520.0), (512.0, 552.0), (520.0, 632.0), (456.0, 632.0)],
    &[(520.0, 584.0), (600.0, 592.0), (664.0, 584.0), (736.0, 580.0), (784.0, 580.0), (784.0, 632.0), (520.0, 632.0)],
    &[(732.0, 396.0), (784.0, 396.0), (784.0, 476.0), (752.0, 476.0), (744.0, 524.0), (784.0, 524.0), (784.0, 580.0), (736.0, 580.0), (728.0, 516.0), (724.0, 468.0)],
];
const WARM: &[(&[(f32, f32)], f32)] = &[
    (&[(612.0, 436.0), (690.0, 436.0), (694.0, 478.0), (660.0, 510.0), (604.0, 522.0), (574.0, 500.0), (590.0, 462.0)], 0.16),
    (&[(628.0, 440.0), (676.0, 440.0), (684.0, 470.0), (640.0, 482.0), (616.0, 470.0)], 0.4),
];

pub(super) struct Floor {
    rect: Rect,
    values: Vec<(Rgb, f32)>,
}

/// Test an authored plane at a world point using the usual half-open polygon rule.
fn inside(points: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut hit = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (xi, yi) = points[i];
        let (xj, yj) = points[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi { hit = !hit; }
        j = i;
    }
    hit
}

impl Floor {
    /// Bake only the authored court and its actual obsidian paving around the visible flame.
    pub fn bake(z: &Zone) -> Option<Self> {
        if z.id != ZoneId::Cinder { return None; }
        let source = z.props.iter().find(|p| p.alive && p.sheet == "cinder_brazier")?;
        let (color, _, _, _, _) = source.light?;
        let rect = Rect { x0: 456, y0: 396, x1: 784, y1: 632 }.clamp(z);
        let mut values = vec![(Rgb::BLACK, 1.0); rect.len()];
        for y in rect.y0..rect.y1 {
            for x in rect.x0..rect.x1 {
                let material = z.gget(x, y);
                // Receive on the actual dry court, including its worn copper
                // bed. An exact RGB whitelist dropped the light whenever the
                // artist weathered a paving color. Upright/actor surfaces are
                // excluded by the caller; water, lava and void stay unlit here.
                if material.r < 0.0 || z.tile_at(x as f32 + 0.5, y as f32 + 0.5) != Tile::Floor { continue; }
                let row = y.div_euclid(24);
                let off = if row.rem_euclid(2) == 0 { 0 } else { 16 };
                let cx = (x + off).div_euclid(32) * 32 - off + 16;
                let cy = row * 24 + 12;
                let mut mul = if EDGES.iter().any(|p| inside(p, x as f32 + 0.5, y as f32 + 0.5)) { 0.80 } else { 1.0 };
                let mut k = 0.0;
                for &(points, warmth) in WARM {
                    if inside(points, cx as f32, cy as f32) { k = warmth; mul = 1.0; }
                }
                if material == rgb(0x3f414f) { k *= 0.15; }
                values[rect.index(x, y).unwrap()] = (color.scale(k), mul);
            }
        }
        Some(Self { rect, values })
    }

    /// Sample cached reception without re-running any world geometry during a frame.
    pub fn at(&self, x: i32, y: i32) -> Option<(Rgb, f32)> {
        let v = self.values[self.rect.index(x, y)?];
        (v.0 != Rgb::BLACK || v.1 != 1.0).then_some(v)
    }
}
