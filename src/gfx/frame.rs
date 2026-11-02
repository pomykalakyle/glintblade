//! The software framebuffer. Everything in the game is drawn pixel by pixel into these
//! buffers at the native 384x216 resolution, then lit, graded and bloomed in `post`.
//!
//! Buffers (all W*H, row-major):
//! - `albedo`: the base color of each pixel (what you'd see under neutral white light).
//! - `light`: how much light reaches each pixel. Filled with the zone's ambient color, then
//!   point lights are added. Final lit color = albedo * light + emissive.
//! - `emissive`: self-lit color added after lighting; also feeds the bloom glow.
//! - `out`: the final graded RGBA8 image (UI is drawn straight into it after grading).
//! - `glow`: extra bloom input for UI (written after grading).
//! - `bloom`: legacy half-resolution RGBA8 texture; `finish` leaves it black.
//! - `shade`: how much of the AMBIENT light reaches each pixel (1 = all). Cast shadows write it
//!   with min(), so overlapping shadows don't stack; point lights still light shaded ground.
//! - `water`: reflectivity per pixel (0 = none). While any water is marked, every sprite drawn
//!   with `sprite()` also draws its rippling reflection into the marked pixels below its feet.
//! - `look`: extra per-frame look settings (see `Look`), reset by `begin`.
//!
//! Opt-in light and shadow helpers live in `light.rs`; masked sprite reflections in `water.rs`.
//!
//! Drawing functions take SCREEN coordinates. Convert world coords with `sx(wx)`/`sy(wy)`
//! (or the `*_w` helpers), using the camera offset `cam_x`/`cam_y`.

use crate::util::{bayer4, clamp01};
use std::ops::{Add, AddAssign, Mul, Sub};

pub const W: usize = 384;
pub const H: usize = 216;
pub const WI: i32 = W as i32;
pub const HI: i32 = H as i32;
pub const BW: usize = W / 2;
pub const BH: usize = H / 2;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Color from 0xRRGGBB.
pub const fn rgb(hex: u32) -> Rgb {
    Rgb { r: ((hex >> 16) & 0xff) as f32 / 255.0, g: ((hex >> 8) & 0xff) as f32 / 255.0, b: (hex & 0xff) as f32 / 255.0 }
}

impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0.0, g: 0.0, b: 0.0 };
    pub const WHITE: Rgb = Rgb { r: 1.0, g: 1.0, b: 1.0 };
    pub const fn new(r: f32, g: f32, b: f32) -> Rgb {
        Rgb { r, g, b }
    }
    pub const fn gray(v: f32) -> Rgb {
        Rgb { r: v, g: v, b: v }
    }
    /// Parse "#rrggbb" or "rrggbb". Bad input gives magenta so it's easy to spot.
    pub fn hex(s: &str) -> Rgb {
        let s = s.trim().trim_start_matches('#');
        u32::from_str_radix(s, 16).map(rgb).unwrap_or(rgb(0xff00ff))
    }
    pub fn from_u8(c: [u8; 3]) -> Rgb {
        Rgb::new(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0)
    }
    pub fn lerp(self, o: Rgb, t: f32) -> Rgb {
        Rgb::new(self.r + (o.r - self.r) * t, self.g + (o.g - self.g) * t, self.b + (o.b - self.b) * t)
    }
    pub fn scale(self, k: f32) -> Rgb {
        Rgb::new(self.r * k, self.g * k, self.b * k)
    }
    pub fn luma(self) -> f32 {
        self.r * 0.299 + self.g * 0.587 + self.b * 0.114
    }
    pub fn max_c(self) -> f32 {
        self.r.max(self.g).max(self.b)
    }
    pub fn clamp01(self) -> Rgb {
        Rgb::new(clamp01(self.r), clamp01(self.g), clamp01(self.b))
    }
    pub fn to_u8(self) -> [u8; 3] {
        [(clamp01(self.r) * 255.0 + 0.5) as u8, (clamp01(self.g) * 255.0 + 0.5) as u8, (clamp01(self.b) * 255.0 + 0.5) as u8]
    }
    pub fn key(self) -> u32 {
        let c = self.to_u8();
        ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
    }
}

impl Add for Rgb {
    type Output = Rgb;
    fn add(self, o: Rgb) -> Rgb {
        Rgb::new(self.r + o.r, self.g + o.g, self.b + o.b)
    }
}
impl Sub for Rgb {
    type Output = Rgb;
    fn sub(self, o: Rgb) -> Rgb {
        Rgb::new(self.r - o.r, self.g - o.g, self.b - o.b)
    }
}
impl AddAssign for Rgb {
    fn add_assign(&mut self, o: Rgb) {
        self.r += o.r;
        self.g += o.g;
        self.b += o.b;
    }
}
impl Mul<f32> for Rgb {
    type Output = Rgb;
    fn mul(self, k: f32) -> Rgb {
        self.scale(k)
    }
}
impl Mul<Rgb> for Rgb {
    type Output = Rgb;
    fn mul(self, o: Rgb) -> Rgb {
        Rgb::new(self.r * o.r, self.g * o.g, self.b * o.b)
    }
}

/// Extra look settings on top of the zone's `Grade`. `begin` resets them to the defaults below;
/// a region that wants its own sets them every frame from any hook before `compose` (e.g.
/// `Region::light`, which runs right before it). Defaults are tuned to look good everywhere.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    /// Width of the ordered-dither transition between light steps (`Grade::light_steps`), 0..1.
    /// 0 = hard bands, 1 = dither across the whole step. Flat ambient is never dithered.
    pub light_dither: f32,
    /// What ambient light becomes in full shade (cast shadows, `shade_*`), as a multiplier.
    /// Bluish by default: shadows shift toward blue/violet, like hand-shaded pixel art.
    pub shade_tint: Rgb,
    /// Multiplier the vignette pulls the frame edges toward (cool, deep indigo by default).
    pub vignette_tint: Rgb,
    /// Point lights also add this much of their own color on top of the multiply, so colored
    /// light still reads on dark materials (0 = pure albedo * light).
    pub light_scatter: f32,
    /// Color multiplier for drop shadows at full strength (`shadow`, `drop_shadow`).
    pub shadow_tint: Rgb,
    /// Reflections in marked water: color multiplier, opacity and ripple size (px).
    pub water_tint: Rgb,
    pub water_alpha: f32,
    pub water_wave: f32,
    /// Maximum native rows below the waterline; regions may shorten actor reflections.
    pub water_depth_max: i32,
    /// Lit pixels brighter than white bleed toward white (hue kept) instead of clipping per
    /// channel: 1 = full bleed, 0 = plain per-channel clip.
    pub hot_white: f32,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            light_dither: 0.5,
            shade_tint: Rgb::new(0.50, 0.50, 0.66),
            vignette_tint: Rgb::new(0.22, 0.16, 0.42),
            light_scatter: 0.06,
            shadow_tint: Rgb::new(0.06, 0.05, 0.18),
            water_tint: Rgb::new(0.62, 0.78, 0.92),
            water_alpha: 0.5,
            water_wave: 1.0,
            water_depth_max: i32::MAX,
            hot_white: 1.0,
        }
    }
}

/// Something that casts a shadow this frame (characters, projectiles): feet position, body
/// half-width and height in pixels. Filled by the scene before rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct Caster {
    pub x: f32,
    pub y: f32,
    pub half_w: f32,
    pub height: f32,
}

/// A real point emitter drawn this frame, retained for local ground-light occlusion.
#[derive(Clone, Copy, Debug)]
pub struct PointLight {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: Rgb,
    pub intensity: f32,
    /// Height above ground; x/y remain the projected world position.
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Surface {
    pub normal: [f32; 3],
    pub height: f32,
    pub owner: u32,
    /// 0 unlit/sky, 1 ground, 2 scenery/terrain cap, 3 actor, 4 ink, 5 upright terrain.
    pub kind: u8,
}

#[derive(Clone, Debug)]
pub struct SurfaceCaster {
    pub pixels: std::sync::Arc<[super::surface::NormalPixel]>,
    pub w: i32, pub h: i32, pub anchor: (i32, i32),
    pub x: f32, pub y: f32, pub base: f32,
    pub flip: bool, pub owner: u32, pub opacity: f32,
    /// Native foot-depth intervals, indexed by unmirrored source X.
    pub foot: std::sync::Arc<[[f32; 2]]>,
    pub min_height: f32,
    /// Opt-in, albedo-tinted reflection strength; zero retains the matte response.
    pub reflectivity: f32,
}

pub struct Frame {
    pub albedo: Vec<Rgb>,
    pub light: Vec<Rgb>,
    pub emissive: Vec<Rgb>,
    pub hdr: Vec<Rgb>,
    pub out: Vec<[u8; 4]>,
    pub glow: Vec<Rgb>,
    pub bloom: Vec<[u8; 4]>,
    /// World position of the screen's top-left pixel.
    pub cam_x: i32,
    pub cam_y: i32,
    /// 0 = normal, 1 = fully black. Applied to everything at the very end.
    pub fade: f32,
    /// 0 = normal, 1 = fully white flash. Applied at the very end.
    pub flash: f32,
    /// Ticks since the game started (for animated effects).
    pub t: u64,
    /// Shadow casters for this frame (see `Caster`); the scene fills it before rendering.
    pub casters: Vec<Caster>,
    pub point_lights: Vec<PointLight>,
    pub surfaces: Vec<Surface>,
    pub surface_casters: Vec<SurfaceCaster>,
    /// Retain nearby native scene art for one-bounce crystal reflections.
    pub(crate) reflection_sources: Vec<super::reflection::Source>,
    pub(crate) capture_reflection_sources: bool,
    pub(crate) reflection_lights: Vec<PointLight>,
    pub surface_lighting: bool,
    /// Only the garden has a directional daylight key.
    pub normal_sun: bool,
    /// Visible actor coverage, stamped by body sprites and cleared by opaque foreground pixels.
    pub actor_pixels: Vec<bool>,
    /// Ambient visibility per pixel (1 = fully lit). See the module docs.
    pub shade: Vec<f32>,
    /// Water reflectivity per pixel (0..255). See `mark_water`.
    pub water: Vec<u8>,
    /// True once anything marked water this frame (sprites skip reflection work otherwise).
    pub water_any: bool,
    /// The ambient color `begin` filled `light` with.
    pub ambient: Rgb,
    pub look: Look,
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            albedo: vec![Rgb::BLACK; W * H],
            light: vec![Rgb::WHITE; W * H],
            emissive: vec![Rgb::BLACK; W * H],
            hdr: vec![Rgb::BLACK; W * H],
            out: vec![[0, 0, 0, 255]; W * H],
            glow: vec![Rgb::BLACK; W * H],
            bloom: vec![[0, 0, 0, 255]; BW * BH],
            cam_x: 0,
            cam_y: 0,
            fade: 0.0,
            flash: 0.0,
            t: 0,
            casters: Vec::new(),
            point_lights: Vec::new(),
            surfaces: vec![Surface::default(); W * H],
            surface_casters: Vec::new(),
            reflection_sources: Vec::new(),
            capture_reflection_sources: false,
            reflection_lights: Vec::new(),
            surface_lighting: false,
            normal_sun: false,
            actor_pixels: vec![false; W * H],
            shade: vec![1.0; W * H],
            water: vec![0; W * H],
            water_any: false,
            ambient: Rgb::WHITE,
            look: Look::default(),
        }
    }

    /// Reset per-frame buffers. Albedo is cleared to `bg`, light to `ambient`.
    pub fn begin(&mut self, bg: Rgb, ambient: Rgb) {
        self.albedo.fill(bg);
        self.light.fill(ambient);
        self.emissive.fill(Rgb::BLACK);
        self.point_lights.clear();
        self.surfaces.fill(Surface::default());
        self.surface_casters.clear();
        self.reflection_sources.clear();
        self.capture_reflection_sources = false;
        self.reflection_lights.clear();
        self.surface_lighting = false;
        self.normal_sun = false;
        self.actor_pixels.fill(false);
        self.glow.fill(Rgb::BLACK);
        self.shade.fill(1.0);
        if self.water_any {
            self.water.fill(0);
            self.water_any = false;
        }
        self.ambient = ambient;
        self.look = Look::default();
    }

    #[inline]
    pub fn idx(x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && x < WI && y < HI { Some(y as usize * W + x as usize) } else { None }
    }

    /// World x -> screen x.
    #[inline]
    pub fn sx(&self, wx: f32) -> i32 {
        wx.floor() as i32 - self.cam_x
    }
    /// World y -> screen y.
    #[inline]
    pub fn sy(&self, wy: f32) -> i32 {
        wy.floor() as i32 - self.cam_y
    }

    /// Is a world-space box (x, y, w, h) at least partly on screen (with margin)?
    pub fn visible(&self, wx: f32, wy: f32, w: f32, h: f32, margin: f32) -> bool {
        let sx = wx - self.cam_x as f32;
        let sy = wy - self.cam_y as f32;
        sx + w >= -margin && sy + h >= -margin && sx <= W as f32 + margin && sy <= H as f32 + margin
    }

    // ---------------------------------------------------------------- albedo

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if let Some(i) = Self::idx(x, y) {
            self.albedo[i] = c;
            self.actor_pixels[i] = false;
            self.surfaces[i] = Surface::default();
        }
    }
    /// Mark an actual visible body pixel after its opaque color has been written.
    #[inline]
    pub fn mark_actor(&mut self, x: i32, y: i32) {
        if let Some(i) = Self::idx(x, y) { self.actor_pixels[i] = true; }
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Rgb {
        Self::idx(x, y).map(|i| self.albedo[i]).unwrap_or(Rgb::BLACK)
    }
    /// Blend `c` over the albedo with opacity `a` (0..1).
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if let Some(i) = Self::idx(x, y) {
            self.albedo[i] = self.albedo[i].lerp(c, a);
            if a >= 1.0 { self.actor_pixels[i] = false; self.surfaces[i] = Surface::default(); }
        }
    }
    /// Draw with ordered-dither transparency: the pixel is drawn where `a` beats the Bayer threshold.
    #[inline]
    pub fn put_dither(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if a >= 1.0 || (a > 0.0 && a > bayer4(x + self.cam_x, y + self.cam_y)) {
            self.put(x, y, c);
        }
    }
    /// Multiply albedo by k (shadows, darkening).
    #[inline]
    pub fn darken(&mut self, x: i32, y: i32, k: f32) {
        if let Some(i) = Self::idx(x, y) {
            self.albedo[i] = self.albedo[i].scale(k);
        }
    }

    // ---------------------------------------------------------------- emissive

    /// A self-lit pixel: shows exactly `c` regardless of lighting, and glows (bloom).
    #[inline]
    pub fn put_glow(&mut self, x: i32, y: i32, c: Rgb) {
        if let Some(i) = Self::idx(x, y) {
            self.albedo[i] = Rgb::BLACK;
            self.actor_pixels[i] = false;
            self.surfaces[i] = Surface::default();
            self.emissive[i] = c;
        }
    }
    /// Add light-emitting color on top of whatever is there (halos, sparks, additive glows).
    #[inline]
    pub fn add_emit(&mut self, x: i32, y: i32, c: Rgb) {
        if let Some(i) = Self::idx(x, y) {
            self.emissive[i] += c;
        }
    }

    // ---------------------------------------------------------------- light

    /// Add a round point light (composed into stepped bands) at screen (x, y). `radius` in pixels, `color` scaled by `intensity`.
    pub fn add_light(&mut self, x: f32, y: f32, radius: f32, color: Rgb, intensity: f32) {
        if radius <= 0.5 || intensity <= 0.0 {
            return;
        }
        self.retain_reflection_light(x, y, radius, color, intensity);
        let c = color.scale(intensity);
        let x0 = ((x - radius).floor() as i32).max(0);
        let x1 = ((x + radius).ceil() as i32).min(WI - 1);
        let projected_radius = if self.surface_lighting { radius * std::f32::consts::SQRT_2 } else { radius };
        let y0 = ((y - projected_radius).floor() as i32).max(0);
        let y1 = ((y + projected_radius).ceil() as i32).min(HI - 1);
        if x0 > x1 || y0 > y1 {
            return;
        }
        self.point_lights.push(PointLight { x: x+self.cam_x as f32, y: y+self.cam_y as f32,
            radius, color, intensity, height: 18.0 });
        if self.surface_lighting { return; }
        let inv_r2 = 1.0 / (radius * radius);
        for py in y0..=y1 {
            let dy = py as f32 + 0.5 - y;
            let row = py as usize * W;
            for px in x0..=x1 {
                let dx = px as f32 + 0.5 - x;
                let d2 = (dx * dx + dy * dy) * inv_r2;
                if d2 < 1.0 {
                    let f = 1.0 - d2;
                    let f = f * f;
                    let l = &mut self.light[row + px as usize];
                    l.r += c.r * f;
                    l.g += c.g * f;
                    l.b += c.b * f;
                }
            }
        }
    }

    /// Point light in world coordinates.
    pub fn add_light_w(&mut self, wx: f32, wy: f32, radius: f32, color: Rgb, intensity: f32) {
        let x = wx - self.cam_x as f32;
        let y = wy - self.cam_y as f32;
        let ry = if self.surface_lighting { radius * std::f32::consts::SQRT_2 } else { radius };
        if x + radius < 0.0 || y + ry < 0.0 || x - radius > W as f32 || y - ry > H as f32 {
            self.retain_reflection_light(x, y, radius, color, intensity);
            return;
        }
        self.add_light(x, y, radius, color, intensity);
    }

    /// A point source at (world X, ground Y, height); projection stays Y-height.
    pub fn add_light_3d_w(&mut self, wx: f32, ground_y: f32, height: f32, radius: f32, color: Rgb, intensity: f32) {
        let before = self.point_lights.len();
        let reflected_before = self.reflection_lights.len();
        self.add_light_w(wx, ground_y - height, radius, color, intensity);
        if self.point_lights.len() > before { self.point_lights.last_mut().unwrap().height = height; }
        if self.reflection_lights.len() > reflected_before { self.reflection_lights.last_mut().unwrap().height = height; }
    }

    fn retain_reflection_light(&mut self, x: f32, y: f32, radius: f32, color: Rgb, intensity: f32) {
        if self.capture_reflection_sources && radius > 0.5 && intensity > 0.0
            && self.visible(x + self.cam_x as f32 - radius, y + self.cam_y as f32 - radius,
                radius * 2.0, radius * 2.0, 160.0)
        {
            self.reflection_lights.push(PointLight { x: x + self.cam_x as f32, y: y + self.cam_y as f32,
                radius, color, intensity, height: 18.0 });
        }
    }

    /// Multiply the light at a pixel (cloud shadows, darkness pockets).
    #[inline]
    pub fn scale_light(&mut self, x: i32, y: i32, k: f32) {
        if let Some(i) = Self::idx(x, y) {
            self.light[i] = self.light[i].scale(k);
        }
    }

    // ---------------------------------------------------------------- shapes (albedo)

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        for py in y.max(0)..(y + h).min(HI) {
            for px in x.max(0)..(x + w).min(WI) {
                self.albedo[py as usize * W + px as usize] = c;
                self.actor_pixels[py as usize * W + px as usize] = false;
                self.surfaces[py as usize * W + px as usize] = Surface::default();
            }
        }
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, c: Rgb) {
        let r2 = r * r + r;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r2 {
                    self.put(cx + dx, cy + dy, c);
                }
            }
        }
    }

    pub fn circle_glow(&mut self, cx: i32, cy: i32, r: i32, c: Rgb) {
        let r2 = r * r + r;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r2 {
                    self.put_glow(cx + dx, cy + dy, c);
                }
            }
        }
    }

    /// Bresenham line calling `f(x, y)` for every pixel.
    pub fn line_with(x0: i32, y0: i32, x1: i32, y1: i32, mut f: impl FnMut(i32, i32)) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            f(x, y);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
        Self::line_with(x0, y0, x1, y1, |x, y| self.put(x, y, c));
    }

    pub fn line_glow(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
        let mut pts = Vec::new();
        Self::line_with(x0, y0, x1, y1, |x, y| pts.push((x, y)));
        for (x, y) in pts {
            self.put_glow(x, y, c);
        }
    }

    /// Crisp drop shadow under a character: min-combines visibility in an ellipse (screen coords of center).
    /// Drawn as a clean pixel ellipse (a solid core and a lighter one-pixel rim, no dither
    /// speckle), tinted toward `look.shadow_tint` so shadows read cool instead of gray.
    pub fn shadow(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, strength: f32) {
        if rx <= 0 || ry <= 0 || strength <= 0.0 {
            return;
        }
        let ax = rx as f32 + 0.5;
        let ay = ry as f32 + 0.5;
        let inside = |dx: i32, dy: i32| -> bool {
            let fx = dx as f32 / ax;
            let fy = dy as f32 / ay;
            fx * fx + fy * fy <= 1.0
        };
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                if !inside(dx, dy) { continue; }
                let edge = !inside(dx - 1, dy) || !inside(dx + 1, dy) || !inside(dx, dy - 1) || !inside(dx, dy + 1);
                self.shade_px(cx + dx, cy + dy, 1.0 - strength * if edge { 0.55 } else { 1.0 });
            }
        }
    }

    /// Albedo multiplier for a drop shadow of `strength` (0..1), tinted by `look.shadow_tint`.
    pub fn shadow_mul(&self, strength: f32) -> Rgb {
        let k = clamp01(strength);
        Rgb::WHITE.lerp(self.look.shadow_tint, k)
    }

    /// Drop shadow for something `height` px above the ground: it shrinks and fades as the
    /// caster rises, so jumps and floating things read clearly. Screen coords of the ground point.
    pub fn drop_shadow(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, strength: f32, height: f32) {
        let k = 1.0 / (1.0 + height.max(0.0) / 20.0);
        let rx2 = ((rx as f32) * (0.55 + 0.45 * k)).round().max(1.0) as i32;
        let ry2 = ((ry as f32) * (0.6 + 0.4 * k)).round().max(1.0) as i32;
        self.shadow(cx, cy, rx2, ry2, strength * (0.35 + 0.65 * k));
    }

    // ---------------------------------------------------------------- UI (drawn into `out` after grading)

    #[inline]
    pub fn ui_put(&mut self, x: i32, y: i32, c: Rgb) {
        if let Some(i) = Self::idx(x, y) {
            let u = c.to_u8();
            self.out[i] = [u[0], u[1], u[2], 255];
        }
    }
    #[inline]
    pub fn ui_blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if let Some(i) = Self::idx(x, y) {
            let o = self.out[i];
            let base = Rgb::from_u8([o[0], o[1], o[2]]);
            let u = base.lerp(c, clamp01(a)).to_u8();
            self.out[i] = [u[0], u[1], u[2], 255];
        }
    }
    /// UI pixel that also glows (feeds bloom).
    #[inline]
    pub fn ui_glow(&mut self, x: i32, y: i32, c: Rgb, glow: f32) {
        self.ui_put(x, y, c);
        if let Some(i) = Self::idx(x, y) {
            self.glow[i] += c.scale(glow);
        }
    }
    pub fn ui_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb, a: f32) {
        for py in y..y + h {
            for px in x..x + w {
                self.ui_blend(px, py, c, a);
            }
        }
    }
}
