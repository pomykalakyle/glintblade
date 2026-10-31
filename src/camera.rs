//! Smooth follow camera with look-ahead and trauma-based screen shake.

use crate::gfx::{H, W};
use crate::util::noise2;

pub struct Camera {
    /// Center of view in world pixels (smoothed).
    pub x: f32,
    pub y: f32,
    look_x: f32,
    look_y: f32,
}

impl Camera {
    pub fn new(x: f32, y: f32) -> Camera {
        Camera { x, y, look_x: 0.0, look_y: 0.0 }
    }

    pub fn snap(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.look_x = 0.0;
        self.look_y = 0.0;
    }

    /// Follow a target with a little look-ahead in the direction it's moving/facing.
    pub fn follow(&mut self, tx: f32, ty: f32, dir: (f32, f32), speed: f32) {
        let k = (speed / 1.5).clamp(0.0, 1.0);
        self.look_x += (dir.0 * 26.0 * k - self.look_x) * 0.04;
        self.look_y += (dir.1 * 16.0 * k - self.look_y) * 0.04;
        self.x += (tx + self.look_x - self.x) * 0.12;
        self.y += (ty - 8.0 + self.look_y - self.y) * 0.12;
    }

    /// Keep a subject's visible anchor clear of the HUD and screen edges, including shake.
    pub fn keep_visible(&mut self, x: f32, y: f32) {
        // Leave room for Glint's head above the anchor and the boss bar below it.
        self.x = self.x.clamp(x - W as f32 / 2.0 + 32.0, x + W as f32 / 2.0 - 32.0);
        self.y = self.y.clamp(y - H as f32 / 2.0 + 32.0, y + H as f32 / 2.0 - 48.0);
    }

    /// Top-left screen position for this frame (integer pixels), with shake, clamped to the zone.
    pub fn view(&self, zone_w: i32, zone_h: i32, trauma: f32, t: u64) -> (i32, i32) {
        let s = trauma * trauma;
        let tt = t as f32 * 0.35;
        let ox = (noise2(tt, 0.5, 11) * 2.0 - 1.0) * 7.0 * s;
        let oy = (noise2(0.5, tt, 23) * 2.0 - 1.0) * 5.0 * s;
        let (mut cx, mut cy) = self.view_free();
        cx = if zone_w <= W as i32 { (zone_w - W as i32) / 2 } else { cx.clamp(0, zone_w - W as i32) };
        cy = if zone_h <= H as i32 { (zone_h - H as i32) / 2 } else { cy.clamp(0, zone_h - H as i32) };
        (cx + ox.round() as i32, cy + oy.round() as i32)
    }

    /// Rounded top-left without zone bounds or shake, for the title postcard.
    pub fn view_free(&self) -> (i32, i32) {
        ((self.x - W as f32 / 2.0).round() as i32, (self.y - H as f32 / 2.0).round() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Match the original clamp-before-shake math at edges, half pixels and narrow zones.
    #[test]
    fn bounded_camera_matches_original_view() {
        for (zone_w, zone_h) in [(1, 1), (383, 215), (384, 216), (385, 217), (1344, 1048)] {
            for x in [-500.5, -0.5, 191.5, 192.0, 192.5, 712.0, 1344.5] {
                for y in [-500.5, -0.5, 70.0, 107.5, 108.0, 108.5, 1048.5] {
                    let cam = Camera::new(x, y);
                    assert_eq!(cam.view_free(), ((x - W as f32 / 2.0).round() as i32, (y - H as f32 / 2.0).round() as i32));
                    for trauma in [0.0, 0.25, 0.8, 1.0] {
                        for t in [0, 1, 180, 600, 4488] {
                            let s = trauma * trauma;
                            let tt = t as f32 * 0.35;
                            let ox = (noise2(tt, 0.5, 11) * 2.0 - 1.0) * 7.0 * s;
                            let oy = (noise2(0.5, tt, 23) * 2.0 - 1.0) * 5.0 * s;
                            let mut cx = (cam.x - W as f32 / 2.0).round() as i32;
                            let mut cy = (cam.y - H as f32 / 2.0).round() as i32;
                            cx = if zone_w <= W as i32 { (zone_w - W as i32) / 2 } else { cx.clamp(0, zone_w - W as i32) };
                            cy = if zone_h <= H as i32 { (zone_h - H as i32) / 2 } else { cy.clamp(0, zone_h - H as i32) };
                            assert_eq!(cam.view(zone_w, zone_h, trauma, t), (cx + ox.round() as i32, cy + oy.round() as i32));
                        }
                    }
                }
            }
        }
    }
}
