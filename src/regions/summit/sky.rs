//! A sparse, motionless starfield behind the Summit's stonework.

use super::pal::K0;
use crate::gfx::{Frame, H, Rgb, W, rgb};
use crate::util::hash2;

/// Fixed world-space stars keep captures and stitched overviews continuous.
pub struct Sky {
    stars: Vec<(i32, i32, Rgb)>,
}

impl Sky {
    /// Place a few restrained stars in a fixed, repeatable night sky.
    pub fn new(pw: i32, ph: i32) -> Self {
        let mut stars = Vec::new();
        // About fifteen stars per screen, with large areas of uninterrupted dark.
        for gy in 0..(ph / 64 + 1) {
            for gx in 0..(pw / 64 + 1) {
                if hash2(gx, gy, 401) > 0.72 {
                    continue;
                }
                let x = gx * 64 + 8 + (hash2(gx, gy, 402) * 47.0) as i32;
                let y = gy * 64 + 8 + (hash2(gx, gy, 403) * 47.0) as i32;
                let c = if hash2(gx, gy, 404) < 0.14 { rgb(0x9eafb9) } else { rgb(0x4d5f76) };
                stars.push((x, y, c));
            }
        }
        Self { stars }
    }

    /// Paint a flat dark sky and still stars, with no bloom or point lights.
    pub fn draw(&self, f: &mut Frame) {
        f.albedo.fill(K0);
        for &(x, y, c) in &self.stars {
            f.put(x - f.cam_x, y - f.cam_y, c);
        }
    }

    /// Leave unobscured sky pixels unlit by the arena's nearby braziers.
    pub fn light(&self, f: &mut Frame, holes: &[bool], pw: i32, ph: i32) {
        let is_sky = |c: Rgb| c == K0 || c == rgb(0x9eafb9) || c == rgb(0x4d5f76);
        for sy in 0..H as i32 {
            let wy = sy + f.cam_y;
            for sx in 0..W as i32 {
                let wx = sx + f.cam_x;
                let hole = wx < 0 || wy < 0 || wx >= pw || wy >= ph || holes[(wy * pw + wx) as usize];
                let i = sy as usize * W + sx as usize;
                if hole && is_sky(f.albedo[i]) {
                    f.light[i] = Rgb::WHITE;
                }
            }
        }
    }
}
