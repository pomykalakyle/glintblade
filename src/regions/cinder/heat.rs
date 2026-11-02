//! Cached fire reception on visible stone, leaving the flat court free of a radial light pool.

use super::paint::{Painted, K_ROCKFACE, K_ROCKTOP, K_WALLFACE, K_WALLTOP};
use crate::gfx::{Frame, Rgb, H, W};
use crate::world::Zone;

struct Pixel {
    x: i32,
    y: i32,
    material: Rgb,
    light: Rgb,
}

pub struct Heat {
    pixels: Vec<Pixel>,
    actor_key: Option<(f32, f32, Rgb)>,
}

impl Heat {
    /// Bake the existing flame's falloff only onto authored wall, rock and prop surfaces.
    pub fn build(z: &Zone, painted: &Painted) -> Self {
        let Some(source) = z.props.iter().find(|p| p.sheet == "cinder_brazier" && p.alive) else {
            return Self { pixels: Vec::new(), actor_key: None };
        };
        let Some((color, radius, _, ox, oy)) = source.light else {
            return Self { pixels: Vec::new(), actor_key: None };
        };
        let (sx, sy) = (source.x + ox, source.y + oy);
        let mut materials = vec![None; z.ground.len()];
        for (i, &kind) in painted.kind.v.iter().enumerate() {
            if matches!(kind, K_ROCKFACE | K_ROCKTOP | K_WALLFACE | K_WALLTOP)
                && z.ground[i].luma() > 0.10 {
                materials[i] = Some((z.ground[i], 1.0));
            }
        }
        let mut props: Vec<_> = z.props.iter().filter(|p| p.alive && p.sheet.starts_with("cinder_")).collect();
        props.sort_by(|a, b| a.y.total_cmp(&b.y));
        for prop in props {
            let Some(sheet) = prop.sheet() else { continue };
            let Some(anim) = sheet.anim(&prop.anim) else { continue };
            let Some((x0, y0, _, _)) = prop.bounds() else { continue };
            let cell = sheet.cell(anim, 0);
            for y in 0..cell.h {
                for x in 0..cell.w {
                    let p = cell.at(x, y);
                    if !p.a { continue; }
                    let wx = x0.round() as i32 + if prop.flip { cell.w - 1 - x } else { x };
                    let wy = y0.round() as i32 + y;
                    if wx < 0 || wy < 0 || wx >= z.pw || wy >= z.ph { continue; }
                    let i = (wy * z.pw + wx) as usize;
                    let facing = (wx as f32 + 0.5 - prop.x) * (sx - prop.x) >= 0.0;
                    materials[i] = (!p.emit && p.c.luma() > 0.12).then_some((p.c, if facing { 1.0 } else { 0.35 }));
                }
            }
        }
        let mut pixels = Vec::new();
        for (i, material) in materials.into_iter().enumerate() {
            let Some((material, incidence)) = material else { continue };
            let (x, y) = ((i as i32) % z.pw, (i as i32) / z.pw);
            let (dx, dy) = (x as f32 + 0.5 - sx, y as f32 + 0.5 - sy);
            let d2 = (dx * dx + dy * dy) / (radius * radius);
            if d2 >= 1.0 { continue; }
            let gain = ((1.0 - d2).powi(2) * 4.0).floor() / 4.0 * 0.95 * incidence;
            if gain == 0.0 { continue; }
            pixels.push(Pixel { x, y, material, light: color.scale(gain) });
        }
        Self { pixels, actor_key: Some((sx, sy, color)) }
    }

    /// Apply cached stone reception and the existing flame key inside opaque actor contours.
    pub fn light(&self, f: &mut Frame) {
        // The real brazier is now evaluated per normal and per-source visibility.
        if f.surface_lighting { return; }
        self.static_light(f);
        let Some((sx, sy, color)) = self.actor_key else { return };
        for y in 1..H - 1 {
            for x in 1..W - 1 {
                let i = y * W + x;
                if ![i, i - 1, i + 1, i - W, i + W].iter().all(|&j| f.actor_pixels[j])
                    || f.emissive[i] != Rgb::BLACK || f.albedo[i].luma() <= 0.10 { continue; }
                let (dx, dy) = (x as f32 + f.cam_x as f32 + 0.5 - sx, y as f32 + f.cam_y as f32 + 0.5 - sy);
                let d2 = (dx * dx + dy * dy) / (64.0 * 64.0);
                if d2 < 1.0 { f.light[i] += color.scale((1.0 - d2).powi(2) * 0.9); }
            }
        }
    }

    /// Keep the baked stone term from leaking onto overlaid actors, particles or emissive sources.
    fn static_light(&self, f: &mut Frame) {
        for p in &self.pixels {
            let (x, y) = (p.x - f.cam_x, p.y - f.cam_y);
            if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 { continue; }
            let i = y as usize * W + x as usize;
            if !f.actor_pixels[i] && f.emissive[i] == Rgb::BLACK && f.albedo[i] == p.material {
                f.light[i] += p.light;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cached real-region heat respects actor coverage, emissive sources and changed materials.
    #[test]
    fn heat_only_reaches_visible_matched_stone() {
        let (mut z, _) = super::super::build();
        let painted = super::super::paint::paint(&mut z);
        let heat = Heat::build(&z, &painted);
        let p = heat.pixels.first().expect("visible stone beside the real flame");
        let mut f = Frame::new();
        f.begin(Rgb::BLACK, Rgb::BLACK);
        f.cam_x = p.x - 192;
        f.cam_y = p.y - 108;
        let i = 108 * W + 192;
        f.albedo[i] = p.material;
        heat.static_light(&mut f);
        assert!(f.light[i].max_c() > 0.0);
        for (actor, emissive, material) in [(true, Rgb::BLACK, p.material), (false, Rgb::WHITE, p.material), (false, Rgb::BLACK, Rgb::WHITE)] {
            f.light[i] = Rgb::BLACK;
            f.actor_pixels[i] = actor;
            f.emissive[i] = emissive;
            f.albedo[i] = material;
            heat.static_light(&mut f);
            assert_eq!(f.light[i], Rgb::BLACK);
        }
    }

    /// The real flame lights actor interiors without coloring their ink or the uncovered floor.
    #[test]
    fn actor_heat_preserves_ink_and_floor() {
        let (mut z, _) = super::super::build();
        let painted = super::super::paint::paint(&mut z);
        let heat = Heat::build(&z, &painted);
        let (sx, sy, _) = heat.actor_key.unwrap();
        let mut f = Frame::new();
        f.begin(Rgb::BLACK, Rgb::BLACK);
        f.cam_x = sx as i32 - 192;
        f.cam_y = sy as i32 - 108;
        for y in 107..=109 {
            for x in 191..=193 {
                let i = y * W + x;
                f.actor_pixels[i] = true;
                f.albedo[i] = Rgb::new(0.3, 0.3, 0.3);
            }
        }
        heat.light(&mut f);
        let center = 108 * W + 192;
        assert!(f.light[center].r > f.light[center].b);
        assert_eq!(f.light.iter().filter(|c| c.max_c() > 0.0).count(), 1);
        assert!(f.emissive.iter().all(|c| *c == Rgb::BLACK));
        f.actor_pixels.fill(false);
        f.light.fill(Rgb::BLACK);
        heat.light(&mut f);
        assert!(f.light.iter().all(|c| *c == Rgb::BLACK));
    }
}
