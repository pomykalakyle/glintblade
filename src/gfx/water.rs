//! Opt-in, pixel-grid reflections. Mark water during `Region::draw_ground`, before sprites.
//! `sprite` then mirrors opaque source pixels below their anchor, clipped to the water mask.
//! Tune `frame.look.water_*` after `begin`; call `reflect_sprite` directly for a different
//! waterline (for example, a raised crystal on a bank). No blur or interpolated sprite pixels.

use super::{Cell, DrawOpts, Frame, HI, Rgb, W, WI};
use crate::util::{bayer4, clamp01};

impl Frame {
    /// Mark one screen pixel as water; zero clears it and 1 is fully reflective.
    pub fn mark_water(&mut self, x: i32, y: i32, reflectivity: f32) {
        if let Some(i) = Self::idx(x, y) {
            let r = (clamp01(reflectivity) * 255.0 + 0.5) as u8;
            self.water[i] = r;
            self.water_any |= r > 0;
        }
    }

    /// Mark a screen rectangle as reflective water, clipped to the framebuffer.
    pub fn water_rect(&mut self, x: i32, y: i32, w: i32, h: i32, reflectivity: f32) {
        let r = (clamp01(reflectivity) * 255.0 + 0.5) as u8;
        for py in y.max(0)..(y + h).min(HI) {
            for px in x.max(0)..(x + w).min(WI) {
                self.water[py as usize * W + px as usize] = r;
                self.water_any |= r > 0;
            }
        }
    }

    /// Draw a sprite's mirrored reflection below screen `sy`, restricted to marked water.
    pub fn reflect_sprite(&mut self, cell: &Cell, anchor: (i32, i32), sx: i32, sy: i32, o: &DrawOpts) {
        if !self.water_any || self.look.water_alpha <= 0.0 || o.alpha <= 0.0 || cell.h <= 0 {
            return;
        }
        let ox = sx - if o.flip { cell.w - 1 - anchor.0 } else { anchor.0 };
        let margin = self.look.water_wave.abs().ceil() as i32 + 1;
        if ox - margin >= WI || ox + cell.w + margin < 0 || sy >= HI || sy + anchor.1 + 1 < 0 {
            return;
        }
        for y in 0..cell.h.min(anchor.1 + 1) {
            let depth = anchor.1 - y;
            if depth >= self.look.water_depth_max { continue; }
            let py = sy + 1 + depth;
            if !(0..HI).contains(&py) {
                continue;
            }
            // Whole-row integer offsets keep the silhouette crisp. Long horizontal gaps
            // in the final quarter break the reflection into a few calm water ribbons.
            let wy = py + self.cam_y;
            let phase = wy as f32 * 0.22 + self.t as f32 * 0.045;
            let shift = (phase.sin() * self.look.water_wave).round() as i32;
            let reach = (depth as f32 / cell.h as f32).clamp(0.0, 1.0);
            if reach > 0.6 && (wy + (self.t / 10) as i32).rem_euclid(6) == 0 {
                continue;
            }
            let opacity = self.look.water_alpha * o.alpha * if reach > 0.6 { 0.5 } else { 1.0 };
            for x in 0..cell.w {
                let p = cell.at(x, y);
                if !p.a {
                    continue;
                }
                let px = ox + if o.flip { cell.w - 1 - x } else { x } + shift;
                let Some(i) = Self::idx(px, py) else { continue };
                let a = opacity * self.water[i] as f32 / 255.0;
                if a <= bayer4(px + self.cam_x, wy) {
                    continue;
                }
                let mut c = o.silhouette.unwrap_or(p.c);
                if o.silhouette.is_none() {
                    if let Some((t, k)) = o.tint {
                        c = c.lerp(t, k);
                    }
                    c = cell.flash_color(x, y, c, o.flash);
                }
                c = c * self.look.water_tint;
                self.emissive[i] = Rgb::BLACK;
                if (p.emit && o.silhouette.is_none()) || o.unlit || (o.silhouette.is_some() && o.silhouette_glow) {
                    self.put_glow(px, py, c.scale(o.emit_gain * 0.65));
                } else {
                    self.put(px, py, c);
                }
            }
        }
    }
}
