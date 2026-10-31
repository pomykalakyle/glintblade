//! Light helpers beyond round point lights: beams, sun shafts (god rays), cast shadows,
//! cloud shadows, shade blobs and stepped glow halos.
//!
//! Coordinates are SCREEN pixels unless the name ends in `_w` (world pixels).
//! Light-buffer helpers (`light_beam`, `sun_shafts`) must run after `begin` and before
//! `compose`: a region's `light` hook is the natural place. Shade helpers (`cast_shadow`,
//! `shade_ellipse`, `cloud_shadows`) can run any time in between, e.g. right where a prop is
//! drawn. Shade only dims the AMBIENT light (toward `look.shade_tint`), so point lights still
//! light shaded ground, and overlapping shadows don't stack.

use super::frame::{Frame, H, HI, Rgb, W, WI};
use super::post::stepped;
use super::sprite::Cell;
use crate::util::{bayer4, clamp01, fbm, noise2};

/// Default direction for `cast_shadow`: light comes from the upper left, so shadows fall to
/// the lower right. Ground offset per pixel of height (x, y).
pub const SUN_SHADOW: (f32, f32) = (0.62, 0.34);

/// Screen-wide god rays: parallel diagonal shafts of light, anchored to the world so they stay
/// put as the camera moves, slowly drifting and breathing. Use `Shafts::default()` and tweak.
#[derive(Clone, Copy, Debug)]
pub struct Shafts {
    /// Direction the light travels, radians (default: from the upper left, down and right).
    pub angle: f32,
    /// Average distance between shafts, px.
    pub spacing: f32,
    /// 0..1: how much of the screen lies inside a shaft.
    pub coverage: f32,
    pub color: Rgb,
    /// Extra light in the brightest part of a shaft.
    pub intensity: f32,
    /// Visible haze in the air (additive glow), as a fraction of the light. Keep it small.
    pub air: f32,
    /// Sideways drift, px per tick.
    pub drift: f32,
    /// Ticks (animates the drift and the slow breathing along each shaft).
    pub t: u64,
    pub seed: u32,
}

impl Default for Shafts {
    /// Configure optional shafts; regions must explicitly opt in.
    fn default() -> Self {
        Shafts { angle: 1.05, spacing: 70.0, coverage: 0.42, color: Rgb::WHITE, intensity: 0.32, air: 0.12, drift: 0.04, t: 0, seed: 11 }
    }
}

/// Smooth an influence field before its final pixel-band quantization.
#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = clamp01((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

impl Frame {
    /// Draw a visible emissive core and its point light together at a world position.
    pub fn light_source_w(&mut self, wx: f32, wy: f32, core_radius: i32, radius: f32, color: Rgb, intensity: f32) {
        let (sx, sy) = (self.sx(wx), self.sy(wy));
        if core_radius >= 0 && self.visible(wx, wy, 1.0, 1.0, core_radius as f32) {
            self.circle_glow(sx, sy, core_radius, color);
        }
        self.add_light_w(wx, wy, radius, color, intensity);
    }

    /// Min-write a shade value (1 = fully lit, 0 = full shade).
    #[inline]
    pub fn shade_px(&mut self, x: i32, y: i32, s: f32) {
        if let Some(i) = Self::idx(x, y)
            && s < self.shade[i]
        {
            self.shade[i] = s;
        }
    }

    /// A stepped beam of light from (x0, y0) to (x1, y1): `w0` wide at the start widening to
    /// `w1` at the end, brightest at the start and fading out along its length. Lights the
    /// ground; `air` (0..1) also makes the beam itself visible as a faint glow. For light
    /// through a crack, a window, a sweeping boss beam (use a big `air`), a lantern cone...
    #[allow(clippy::too_many_arguments)]
    pub fn light_beam(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, w0: f32, w1: f32, color: Rgb, intensity: f32, air: f32) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.0 || intensity <= 0.0 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let pad = w0.max(w1) * 0.5 + 1.0;
        let bx0 = ((x0.min(x1) - pad).floor() as i32).max(0);
        let bx1 = ((x0.max(x1) + pad).ceil() as i32).min(WI - 1);
        let by0 = ((y0.min(y1) - pad).floor() as i32).max(0);
        let by1 = ((y0.max(y1) + pad).ceil() as i32).min(HI - 1);
        let c = color.scale(intensity);
        for py in by0..=by1 {
            for px in bx0..=bx1 {
                let rx = px as f32 + 0.5 - x0;
                let ry = py as f32 + 0.5 - y0;
                let t = (rx * ux + ry * uy) / len;
                if !(0.0..=1.0).contains(&t) {
                    continue;
                }
                let s = (rx * -uy + ry * ux).abs();
                let hw = (w0 + (w1 - w0) * t) * 0.5;
                if s >= hw {
                    continue;
                }
                let across = 1.0 - (s / hw) * (s / hw);
                let along = (1.0 - t).powf(0.8) * smoothstep(0.0, 0.06, t);
                let k = across * along;
                let i = py as usize * W + px as usize;
                self.light[i] += c.scale(k);
                if air > 0.0 {
                    self.emissive[i] += c.scale(stepped(k * air, 12.0, 0.35, bayer4(px + self.cam_x, py + self.cam_y)));
                }
            }
        }
    }

    /// `light_beam` in world coordinates.
    #[allow(clippy::too_many_arguments)]
    pub fn light_beam_w(&mut self, wx0: f32, wy0: f32, wx1: f32, wy1: f32, w0: f32, w1: f32, color: Rgb, intensity: f32, air: f32) {
        let (cx, cy) = (self.cam_x as f32, self.cam_y as f32);
        self.light_beam(wx0 - cx, wy0 - cy, wx1 - cx, wy1 - cy, w0, w1, color, intensity, air);
    }

    /// God rays over the whole screen (see `Shafts`). Adds to the light buffer, plus a faint
    /// stepped haze in the air.
    pub fn sun_shafts(&mut self, s: &Shafts) {
        if s.intensity <= 0.0 {
            return;
        }
        let (dx, dy) = (s.angle.cos(), s.angle.sin());
        let (nx, ny) = (-dy, dx);
        let t = s.t as f32;
        // world coordinate across the shafts (u) and along them (v), for each screen corner
        let cam = (self.cam_x as f32, self.cam_y as f32);
        let corners = [(0.0, 0.0), (W as f32, 0.0), (0.0, H as f32), (W as f32, H as f32)];
        let us: Vec<f32> = corners.iter().map(|(x, y)| (x + cam.0) * nx + (y + cam.1) * ny).collect();
        let vs: Vec<f32> = corners.iter().map(|(x, y)| (x + cam.0) * dx + (y + cam.1) * dy).collect();
        let u0 = us.iter().cloned().fold(f32::MAX, f32::min).floor() as i32 - 1;
        let u1 = us.iter().cloned().fold(f32::MIN, f32::max).ceil() as i32 + 1;
        let v0 = vs.iter().cloned().fold(f32::MAX, f32::min).floor() as i32 - 1;
        let v1 = vs.iter().cloned().fold(f32::MIN, f32::max).ceil() as i32 + 1;
        let th = 1.0 - clamp01(s.coverage);
        let sp = s.spacing.max(8.0);
        // across: shafts with soft edges; along: slow breathing so each shaft fades in and out
        let across: Vec<f32> = (u0..=u1)
            .map(|u| {
                let n = noise2((u as f32 + t * s.drift) / sp, 0.37, s.seed) * 0.75 + noise2((u as f32 - t * s.drift * 0.5) / (sp * 0.37), 5.1, s.seed + 3) * 0.25;
                smoothstep(th - 0.06, th + 0.12, n)
            })
            .collect();
        let along: Vec<f32> = (v0..=v1)
            .map(|v| {
                let n = noise2(v as f32 / 140.0, t * 0.0025, s.seed + 7);
                0.45 + 0.55 * smoothstep(0.25, 0.75, n)
            })
            .collect();
        let c = s.color.scale(s.intensity);
        for y in 0..H {
            let wy = y as f32 + 0.5 + cam.1;
            for x in 0..W {
                let wx = x as f32 + 0.5 + cam.0;
                let u = (wx * nx + wy * ny).floor() as i32;
                let a = across[(u - u0) as usize];
                if a <= 0.0 {
                    continue;
                }
                let v = (wx * dx + wy * dy).floor() as i32;
                let k = a * along[(v - v0) as usize];
                let i = y * W + x;
                self.light[i] += c.scale(k);
                if s.air > 0.0 {
                    // haze in three dithered steps, anchored to the world
                    let q = ((k * 3.0 + bayer4(wx as i32, wy as i32) - 0.5).floor() / 3.0).max(0.0);
                    if q > 0.0 {
                        self.emissive[i] += c.scale(q * s.air);
                    }
                }
            }
        }
    }

    /// A sprite's shadow cast on the ground by a low sun: each pixel at height h above the
    /// anchor row lands at (x + h * dir.0, feet + h * dir.1). `dir` = `SUN_SHADOW` for the
    /// game's upper-left light. Hard-edged; goes into `shade`, so characters walking through the
    /// shadow are darkened too, while the caster's own pixels are left alone.
    #[allow(clippy::too_many_arguments)]
    pub fn cast_shadow(&mut self, cell: &Cell, anchor: (i32, i32), sx: i32, sy: i32, flip: bool, dir: (f32, f32), strength: f32) {
        if strength <= 0.0 {
            return;
        }
        let s = 1.0 - clamp01(strength);
        let ox = sx - if flip { cell.w - 1 - anchor.0 } else { anchor.0 };
        let oy = sy - anchor.1;
        let reach = cell.h as f32;
        if ox + cell.w + (reach * dir.0.abs()) as i32 + 2 < 0 || ox - (reach * dir.0.abs()) as i32 - 2 > WI {
            return;
        }
        for y in 0..cell.h {
            let h = (anchor.1 - y) as f32;
            if h < 0.0 {
                continue;
            }
            for x in 0..cell.w {
                if !cell.px[(y * cell.w + x) as usize].a {
                    continue;
                }
                let px = ox + if flip { cell.w - 1 - x } else { x };
                // fill from this height to the next so steep sun angles leave no gaps
                let steps = (dir.0.abs().max(dir.1.abs())).ceil().max(1.0) as i32;
                for k in 0..steps {
                    let hh = h + k as f32 / steps as f32;
                    let gx = (px as f32 + hh * dir.0).round() as i32;
                    let gy = (sy as f32 + hh * dir.1).round() as i32;
                    // leave the caster itself unshaded
                    let lx = gx - ox;
                    let lx = if flip { cell.w - 1 - lx } else { lx };
                    if cell.at(lx, gy - oy).a {
                        continue;
                    }
                    self.shade_px(gx, gy, s);
                }
            }
        }
    }

    /// `cast_shadow` at a world position.
    #[allow(clippy::too_many_arguments)]
    pub fn cast_shadow_w(&mut self, cell: &Cell, anchor: (i32, i32), wx: f32, wy: f32, flip: bool, dir: (f32, f32), strength: f32) {
        let (sx, sy) = (self.sx(wx), self.sy(wy));
        self.cast_shadow(cell, anchor, sx, sy, flip, dir, strength);
    }

    /// A shade blob on the ground (under a big creature, a floating island, the Warden), as a
    /// clean pixel ellipse with a lighter one-pixel rim. Dims ambient light only.
    pub fn shade_ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, strength: f32) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        let ax = rx as f32 + 0.5;
        let ay = ry as f32 + 0.5;
        let inside = |dx: i32, dy: i32| {
            let fx = dx as f32 / ax;
            let fy = dy as f32 / ay;
            fx * fx + fy * fy <= 1.0
        };
        let s = 1.0 - clamp01(strength);
        let rim = 1.0 - clamp01(strength) * 0.55;
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                if inside(dx, dy) {
                    let edge = !inside(dx - 1, dy) || !inside(dx + 1, dy) || !inside(dx, dy - 1) || !inside(dx, dy + 1);
                    self.shade_px(cx + dx, cy + dy, if edge { rim } else { s });
                }
            }
        }
    }

    /// Big soft cloud shadows drifting over the ground, anchored to the world. `scale` = rough
    /// cloud size in px (150-300), `coverage` 0..1, `strength` 0..1 (how dark), `wind` px/tick.
    /// Edges are a narrow ordered dither, so they stay crisp.
    pub fn cloud_shadows(&mut self, t: u64, scale: f32, coverage: f32, strength: f32, wind: (f32, f32)) {
        if strength <= 0.0 || coverage <= 0.0 {
            return;
        }
        const G: i32 = 8;
        let ox = self.cam_x as f32 - wind.0 * t as f32;
        let oy = self.cam_y as f32 - wind.1 * t as f32;
        // sample the noise on a coarse grid aligned to world multiples of G, then interpolate
        let gx0 = (ox / G as f32).floor() as i32;
        let gy0 = (oy / G as f32).floor() as i32;
        let gw = WI / G + 3;
        let gh = HI / G + 3;
        let inv = 1.0 / scale.max(16.0);
        let mut grid = vec![0.0f32; (gw * gh) as usize];
        for j in 0..gh {
            for i in 0..gw {
                let wx = ((gx0 + i) * G) as f32;
                let wy = ((gy0 + j) * G) as f32;
                grid[(j * gw + i) as usize] = fbm(wx * inv, wy * inv * 1.3, 3, 913);
            }
        }
        let th = 1.0 - clamp01(coverage) * 0.6 - 0.2;
        let s = 1.0 - clamp01(strength);
        let fx0 = ox - (gx0 * G) as f32;
        let fy0 = oy - (gy0 * G) as f32;
        for y in 0..H {
            let gy = (y as f32 + fy0) / G as f32;
            let j = gy as i32;
            let ty = gy - j as f32;
            for x in 0..W {
                let gxf = (x as f32 + fx0) / G as f32;
                let i = gxf as i32;
                let tx = gxf - i as f32;
                let a = grid[(j * gw + i) as usize];
                let b = grid[(j * gw + i + 1) as usize];
                let c = grid[((j + 1) * gw + i) as usize];
                let d = grid[((j + 1) * gw + i + 1) as usize];
                let n = (a + (b - a) * tx) + ((c + (d - c) * tx) - (a + (b - a) * tx)) * ty;
                let e = (n - th) / 0.025;
                if e <= 0.0 {
                    continue;
                }
                if e < 1.0 && e <= bayer4(x as i32 + (ox.floor() as i32), y as i32 + (oy.floor() as i32)) {
                    continue;
                }
                let k = y * W + x;
                if s < self.shade[k] {
                    self.shade[k] = s;
                }
            }
        }
    }

    /// A glow in the air around a light source, in a few crisp stepped rings (dithered at the
    /// ring edges), added as emissive so it shows over dark void and feeds the bloom.
    /// `r` = outer radius px, `k` = brightness at the center.
    pub fn halo(&mut self, x: f32, y: f32, r: f32, color: Rgb, k: f32) {
        if r < 1.0 || k <= 0.0 {
            return;
        }
        let x0 = ((x - r).floor() as i32).max(0);
        let x1 = ((x + r).ceil() as i32).min(WI - 1);
        let y0 = ((y - r).floor() as i32).max(0);
        let y1 = ((y + r).ceil() as i32).min(HI - 1);
        const RINGS: f32 = 4.0;
        for py in y0..=y1 {
            for px in x0..=x1 {
                let dx = px as f32 + 0.5 - x;
                let dy = py as f32 + 0.5 - y;
                let d = (dx * dx + dy * dy).sqrt() / r;
                if d >= 1.0 {
                    continue;
                }
                let p = (1.0 - d) * (1.0 - d);
                let lv = p * RINGS;
                let fl = lv.floor();
                let fr = lv - fl;
                // dither only the outer quarter of each ring step
                let up = if fr > 0.75 && (fr - 0.75) * 4.0 > bayer4(px + self.cam_x, py + self.cam_y) { 1.0 } else { 0.0 };
                let q = (fl + up) / RINGS;
                if q > 0.0 {
                    self.emissive[py as usize * W + px as usize] += color.scale(k * q);
                }
            }
        }
    }

    /// `halo` in world coordinates.
    pub fn halo_w(&mut self, wx: f32, wy: f32, r: f32, color: Rgb, k: f32) {
        let x = wx - self.cam_x as f32;
        let y = wy - self.cam_y as f32;
        if x + r < 0.0 || y + r < 0.0 || x - r > W as f32 || y - r > H as f32 {
            return;
        }
        self.halo(x, y, r, color, k);
    }

    /// Point light whose radius and brightness flicker like a flame (deterministic from `t`
    /// and `seed`). `amount` 0..1 = how much it flickers.
    #[allow(clippy::too_many_arguments)]
    pub fn add_light_flicker_w(&mut self, wx: f32, wy: f32, radius: f32, color: Rgb, intensity: f32, amount: f32, t: u64, seed: u32) {
        let amount = clamp01(amount);
        let tt = t as f32;
        let n = noise2(tt * 0.09, seed as f32 * 1.37, seed) * 0.65 + noise2(tt * 0.31, seed as f32 * 0.71, seed.wrapping_add(1)) * 0.35;
        let f = 1.0 - amount * (1.0 - n) * 0.6;
        self.add_light_w(wx, wy, radius * (0.92 + 0.08 * f), color, intensity * f);
    }
}
