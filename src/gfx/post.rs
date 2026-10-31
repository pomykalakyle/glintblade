//! Lighting composite, color grading and bloom.
//!
//! Order each frame: world drawing -> `compose` (albedo * light + emissive, graded into `out`)
//! -> UI drawn into `out` -> `finish` (native glow, then fade/flash).
//!
//! Guarantees region builders can rely on:
//! - Under neutral light (ambient white, no point lights) and a default `Grade`, every pixel
//!   comes out exactly as authored (bit-exact RGB), except where the vignette darkens the edges.
//! - Flat ambient light is never dithered. Only the light ADDED or REMOVED on top of the
//!   ambient (point lights, shafts, shadows) is stepped into `Grade::light_steps` bands, with
//!   ordered dithering confined to the band edges (`Look::light_dither`). Steps keep the light's
//!   hue, so colored lights mix cleanly.
//! - Over-bright lit pixels bleed toward white with their hue kept, instead of clipping each
//!   channel separately (which shifts orange to yellow, cyan to green...).
//! - Glow is composited on the native grid in crisp, dithered bands around visible emitters.
//!   The legacy half-resolution GPU bloom texture stays black.

use super::frame::{BH, BW, Frame, H, Rgb, W};
use crate::util::{bayer4, clamp01};
use std::sync::OnceLock;

/// Per-zone look. Neutral defaults leave palette colors exactly as authored under white light.
#[derive(Clone, Copy, Debug)]
pub struct Grade {
    /// Multiplies the lit color before grading.
    pub exposure: f32,
    /// 1 = unchanged; >1 more contrast around mid gray.
    pub contrast: f32,
    /// 1 = unchanged; 0 = grayscale; >1 more saturated.
    pub saturation: f32,
    /// Tint added to shadows (e.g. a blue-violet lift).
    pub lift: Rgb,
    /// Multiplier for highlights (e.g. warm gold).
    pub gain: Rgb,
    /// Vignette strength 0..1.
    pub vignette: f32,
    /// Bloom strength (0 disables).
    pub bloom: f32,
    /// Added-light highlights above this bloom; ambient-only colors never do (emissive always does).
    pub bloom_threshold: f32,
    /// Quantize light into this many steps with ordered dithering (0 = smooth light).
    /// A few steps gives a hand-shaded, pixel-art feel to lighting.
    pub light_steps: f32,
}

impl Default for Grade {
    /// Choose neutral grading with restrained pixel lighting.
    fn default() -> Self {
        Grade { exposure: 1.0, contrast: 1.0, saturation: 1.0, lift: Rgb::BLACK, gain: Rgb::WHITE, vignette: 0.25, bloom: 0.9, bloom_threshold: 0.85, light_steps: 10.0 }
    }
}

/// Vignette weight per pixel (0 in the middle, rising toward the corners), computed once.
fn vignette_table() -> &'static [f32] {
    static T: OnceLock<Vec<f32>> = OnceLock::new();
    T.get_or_init(|| {
        let cx = W as f32 * 0.5;
        let cy = H as f32 * 0.5;
        let inv = 1.0 / (cx * cx + cy * cy).sqrt();
        let mut v = vec![0.0; W * H];
        for y in 0..H {
            for x in 0..W {
                let dx = x as f32 + 0.5 - cx;
                let dy = (y as f32 + 0.5 - cy) * 1.25;
                let r = ((dx * dx + dy * dy).sqrt() * inv).min(1.0);
                let k = ((r - 0.42) / 0.58).clamp(0.0, 1.0);
                // smoothstep-shaped, so the edge of the clear middle has no visible ring
                v[y * W + x] = k * k * (3.0 - 2.0 * k) * 0.8 + k * k * 0.2;
            }
        }
        v
    })
}

/// Fit an over-bright color into 0..1. Below 1 it's untouched (palette colors stay exact).
/// Above, the luma is kept and saturation drops just enough to fit, so hot light goes white
/// with its hue intact. `hot` blends between this and a plain per-channel clip.
#[inline]
fn fit(c: Rgb, hot: f32) -> Rgb {
    let m = c.max_c();
    if m <= 1.0 {
        return c;
    }
    let l = c.luma().max(0.0);
    let fitted = if l >= 1.0 {
        Rgb::WHITE
    } else {
        let k = (1.0 - l) / (m - l);
        Rgb::new(l + (c.r - l) * k, l + (c.g - l) * k, l + (c.b - l) * k)
    };
    if hot >= 1.0 { fitted } else { c.clamp01().lerp(fitted, hot) }
}

/// Round to RGB8 with enough tolerance to keep a constant light band one exact color.
#[inline]
fn palette_u8(c: Rgb) -> [u8; 3] {
    let q = |v: f32| (v * 255.0 + 0.5001).floor().clamp(0.0, 255.0) as u8;
    [q(c.r), q(c.g), q(c.b)]
}

/// Step the light at one pixel. `base` is the flat light (ambient after shade); only the
/// difference from it is quantized, by its strongest channel, so its hue is kept.
#[inline]
fn step_light(l: Rgb, base: Rgb, steps: f32, lo: f32, inv_w: f32, dither: f32) -> Rgb {
    let d = l - base;
    let hi = d.max_c();
    let low = d.r.min(d.g).min(d.b);
    let mag = if hi >= -low { hi } else { -low };
    if mag <= 1e-4 {
        return l;
    }
    let lv = mag * steps;
    let fl = lv.floor();
    let fr = lv - fl;
    let up = if fr <= lo {
        0.0
    } else if inv_w == 0.0 || fr >= 1.0 - lo || (fr - lo) * inv_w > dither {
        1.0
    } else {
        0.0
    };
    let k = (fl + up) / lv;
    base + d * k
}

/// albedo * light + emissive, then grade into `frame.out`. Keeps the unclipped result in `frame.hdr`.
pub fn compose(frame: &mut Frame, g: &Grade) {
    let look = frame.look;
    let steps = g.light_steps.max(0.0);
    let amb = frame.ambient;
    // ambient lost in full shade
    let shade_loss = Rgb::new(amb.r * (1.0 - look.shade_tint.r), amb.g * (1.0 - look.shade_tint.g), amb.b * (1.0 - look.shade_tint.b));
    let w = clamp01(look.light_dither);
    let lo = 0.5 - 0.5 * w;
    let inv_w = if w > 0.0 { 1.0 / w } else { 0.0 };
    let scatter = look.light_scatter.max(0.0);
    let vtab = vignette_table();
    let vloss = Rgb::WHITE - look.vignette_tint;
    let grade_neutral = g.contrast == 1.0 && g.saturation == 1.0 && g.lift == Rgb::BLACK && g.gain == Rgb::WHITE;
    let (cam_x, cam_y) = (frame.cam_x, frame.cam_y);
    for y in 0..H {
        let wy = y as i32 + cam_y;
        for x in 0..W {
            let i = y * W + x;
            let raw_shade = 1.0 - frame.shade[i].clamp(0.0, 1.0);
            let s = if steps > 0.0 { 1.0 - stepped(raw_shade, steps, look.light_dither, bayer4(x as i32 + cam_x, wy)) } else { 1.0 - raw_shade };
            let mut base = amb;
            let mut l = frame.light[i];
            if s < 1.0 {
                let k = 1.0 - s.max(0.0);
                base = base - shade_loss * k;
                l = l - shade_loss * k;
            }
            if steps > 0.0 {
                l = step_light(l, base, steps, lo, inv_w, bayer4(x as i32 + cam_x, wy));
            }
            l = Rgb::new(l.r.max(0.0), l.g.max(0.0), l.b.max(0.0));
            let a = frame.albedo[i];
            let e = frame.emissive[i];
            let mut c = Rgb::new(a.r * l.r + e.r, a.g * l.g + e.g, a.b * l.b + e.b);
            if scatter > 0.0 {
                // a little of the added light's own color, more on bright materials
                let add = l - base;
                let k = scatter * (0.35 + 0.65 * a.luma());
                c += Rgb::new(add.r.max(0.0), add.g.max(0.0), add.b.max(0.0)) * k;
            }
            if g.exposure != 1.0 {
                c = c.scale(g.exposure);
            }
            frame.hdr[i] = c;
            if !grade_neutral {
                // contrast
                if g.contrast != 1.0 {
                    c = Rgb::new((c.r - 0.5) * g.contrast + 0.5, (c.g - 0.5) * g.contrast + 0.5, (c.b - 0.5) * g.contrast + 0.5);
                }
                // lift / gain (split toning)
                let lum = clamp01(c.luma());
                c = c * Rgb::WHITE.lerp(g.gain, lum) + g.lift.scale((1.0 - lum).powi(2));
                // saturation
                if g.saturation != 1.0 {
                    let l2 = c.luma();
                    c = Rgb::gray(l2).lerp(c, g.saturation);
                }
            }
            // vignette, pulling toward a cool tint rather than gray
            let raw_v = vtab[i] * g.vignette;
            let v = stepped(raw_v, 16.0, 0.35, bayer4(x as i32, y as i32));
            if v > 0.0 {
                c = c * Rgb::new(1.0 - v * vloss.r, 1.0 - v * vloss.g, 1.0 - v * vloss.b);
            }
            let c = fit(c, look.hot_white);
            let u = palette_u8(c);
            frame.out[i] = [u[0], u[1], u[2], 255];
        }
    }
}

/// A separable, running-sum influence filter on the native grid, with transparent borders.
fn spread(src: &[Rgb], radius: usize) -> Vec<Rgb> {
    let mut tmp = vec![Rgb::BLACK; W * H];
    let mut out = vec![Rgb::BLACK; W * H];
    let weight = 1.0 / (radius * 2 + 1) as f32;
    for y in 0..H {
        let row = y * W;
        let mut sum = Rgb::BLACK;
        for x in 0..radius {
            sum += src[row + x];
        }
        for x in 0..W {
            if x + radius < W {
                sum += src[row + x + radius];
            }
            if x > radius {
                sum = sum - src[row + x - radius - 1];
            }
            tmp[row + x] = sum * weight;
        }
    }
    for x in 0..W {
        let mut sum = Rgb::BLACK;
        for y in 0..radius {
            sum += tmp[y * W + x];
        }
        for y in 0..H {
            if y + radius < H {
                sum += tmp[(y + radius) * W + x];
            }
            if y > radius {
                sum = sum - tmp[(y - radius - 1) * W + x];
            }
            out[y * W + x] = sum * weight;
        }
    }
    out
}

/// Quantize a nonnegative intensity, dithering only a narrow strip between solid bands.
#[inline]
pub(super) fn stepped(v: f32, steps: f32, width: f32, threshold: f32) -> f32 {
    let value = v.max(0.0) * steps;
    let lo = value.floor();
    let fraction = value - lo;
    let edge = 0.5 + (threshold - 0.5) * width;
    (lo + if fraction > edge { 1.0 } else { 0.0 }) / steps
}

/// Add stepped halos on the native grid, then fade/flash the entire image, including UI.
pub fn finish(frame: &mut Frame, g: &Grade) {
    frame.bloom.fill([0, 0, 0, 255]);
    if g.bloom > 0.0 && frame.fade < 1.0 && frame.flash < 1.0 {
        glow_native(frame, g);
    }
    let keep = 1.0 - clamp01(frame.fade);
    let flash = clamp01(frame.flash);
    if keep < 1.0 || flash > 0.0 {
        for p in &mut frame.out {
            for c in &mut p[..3] {
                let v = *c as f32 * keep;
                *c = (v + (255.0 - v) * flash + 0.5).min(255.0) as u8;
            }
        }
    }
}

/// Gather light from emissive pixels and illuminated highlights, then draw native-pixel bands.
fn glow_native(frame: &mut Frame, g: &Grade) {
    let mut src = vec![Rgb::BLACK; W * H];
    let mut any = false;
    for (i, source) in src.iter_mut().enumerate() {
        let e = frame.emissive[i].scale(g.exposure);
        let lit = frame.hdr[i] - e;
        let a = frame.albedo[i] * frame.ambient * g.exposure;
        // White stone under neutral light is not a source. Only actual added light
        // above the authored ambient value can contribute to the bright pass.
        let excess = Rgb::new((lit.r - a.r.max(g.bloom_threshold)).max(0.0), (lit.g - a.g.max(g.bloom_threshold)).max(0.0), (lit.b - a.b.max(g.bloom_threshold)).max(0.0));
        let c = e + frame.glow[i] + excess.scale(0.35);
        let m = c.max_c();
        *source = if m > 1.0 { c.scale((1.0 + (m - 1.0) * 0.2) / m) } else { c };
        any |= m > 0.0;
    }
    if !any {
        return;
    }
    // No downsampling: odd camera offsets cannot change the source footprints. These
    // filters produce influence only; the final screen receives quantized bands.
    let tight = spread(&spread(&src, 1), 1);
    let mid = spread(&spread(&src, 3), 3);
    const LEVELS: [f32; 8] = [0.0, 0.025, 0.055, 0.10, 0.17, 0.27, 0.42, 0.65];
    for y in 0..H {
        for x in 0..W {
            let c = (tight[y * W + x] * 0.7 + mid[y * W + x] * 0.6) * g.bloom;
            let magnitude = c.max_c();
            if magnitude < LEVELS[1] * 0.35 {
                continue;
            }
            let v = magnitude.min(LEVELS[LEVELS.len() - 1]);
            let mut k = 0;
            while k + 1 < LEVELS.len() && LEVELS[k + 1] <= v {
                k += 1;
            }
            let mut level = LEVELS[k];
            if k + 1 < LEVELS.len() {
                let fraction = (v - level) / (LEVELS[k + 1] - level);
                let edge = 0.5 + (bayer4(x as i32 + frame.cam_x, y as i32 + frame.cam_y) - 0.5) * 0.5;
                if fraction > edge {
                    level = LEVELS[k + 1];
                }
            }
            if level <= 0.0 {
                continue;
            }
            let i = y * W + x;
            let o = frame.out[i];
            let base = Rgb::from_u8([o[0], o[1], o[2]]);
            // Keep luminous cores and bright authored surfaces distinct from their halos.
            let attenuation = 1.0 - 0.7 * base.max_c().powi(2);
            let glow = c.scale(level / magnitude * attenuation);
            let result = palette_u8(fit(base + glow * (Rgb::WHITE - base * 0.5), frame.look.hot_white));
            frame.out[i] = [result[0], result[1], result[2], 255];
        }
    }
}

/// Software version of the GPU composite (used by headless screenshots): nearest-neighbor
/// upscale of `out` plus a bilinear upscale of `bloom`, combined like the window's shader
/// (`base + glow * (1 - base * 0.5)`).
pub fn composite_scaled(frame: &Frame, scale: usize) -> Vec<u8> {
    let (ow, oh) = (W * scale, H * scale);
    let mut img = vec![0u8; ow * oh * 4];
    if frame.bloom.iter().all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0) {
        for y in 0..oh {
            for x in 0..ow {
                let o = (y * ow + x) * 4;
                img[o..o + 4].copy_from_slice(&frame.out[(y / scale) * W + x / scale]);
            }
        }
        return img;
    }
    for y in 0..oh {
        // bloom sample coords (bloom is half res -> each bloom texel covers 2*scale output pixels)
        let by = ((y as f32 + 0.5) / (2 * scale) as f32 - 0.5).clamp(0.0, (BH - 1) as f32);
        let by0 = by.floor() as usize;
        let by1 = (by0 + 1).min(BH - 1);
        let ty = by - by0 as f32;
        for x in 0..ow {
            let bx = ((x as f32 + 0.5) / (2 * scale) as f32 - 0.5).clamp(0.0, (BW - 1) as f32);
            let bx0 = bx.floor() as usize;
            let bx1 = (bx0 + 1).min(BW - 1);
            let tx = bx - bx0 as f32;
            let s = |i: usize, c: usize| frame.bloom[i][c] as f32 / 255.0;
            let base = frame.out[(y / scale) * W + x / scale];
            let o = (y * ow + x) * 4;
            for c in 0..3 {
                let a = s(by0 * BW + bx0, c) * (1.0 - tx) + s(by0 * BW + bx1, c) * tx;
                let b = s(by1 * BW + bx0, c) * (1.0 - tx) + s(by1 * BW + bx1, c) * tx;
                let glow = a * (1.0 - ty) + b * ty;
                let bv = base[c] as f32 / 255.0;
                let v = bv + glow * (1.0 - bv * 0.5);
                img[o + c] = (v * 255.0 + 0.5).min(255.0) as u8;
            }
            img[o + 3] = 255;
        }
    }
    img
}
