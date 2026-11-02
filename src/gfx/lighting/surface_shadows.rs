//! Source-local visibility through the current native sprite silhouettes.
//!
//! X points right, Y down along the ground and Z upwards. Screen Y is Y-Z.
//! Native foot intervals give the silhouette real ground depth. Its volume
//! follows each source pixel along view rays (delta Y = delta Z), preserving
//! the exact projected alpha instead of collapsing a body onto one ground row.
//! Legacy/test callers without a foot profile retain the one-pixel cut sheet.
//! This is intentionally 2.5D: no collision ellipse substitutes for native art.
//! Ground rows use an inverse shadow projection; raised receiving pixels use
//! the same ray/alpha intersection at their own world-space height.
//!
//! The result contains visibility of this light only. It never changes fill,
//! emission, contacts, albedo or another light's contribution.

use crate::gfx::frame::{PointLight, SurfaceCaster};
use crate::gfx::{Frame, H, W};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

const HALF_DEPTH: f32 = 0.5;
const EPS: f32 = 0.000_01;

struct AlphaBounds {
    pixels: Arc<[crate::gfx::surface::NormalPixel]>,
    w: i32,
    h: i32,
    bounds: [i32; 4],
}

thread_local! {
    // Keep the immutable Arc alive with its identity. A later animation cannot reuse a freed
    // allocation's address and accidentally inherit an earlier pose's silhouette bounds.
    static ALPHA_BOUNDS: RefCell<VecDeque<AlphaBounds>> = RefCell::new(VecDeque::new());
}

fn alpha_bounds(source: &SurfaceCaster) -> [i32; 4] {
    ALPHA_BOUNDS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(i) = cache.iter().position(|entry| entry.w == source.w && entry.h == source.h
            && Arc::ptr_eq(&entry.pixels, &source.pixels)) {
            let entry = cache.remove(i).unwrap();
            let bounds = entry.bounds;
            cache.push_back(entry);
            return bounds;
        }
        let mut bounds = [source.w, -1, source.h, -1];
        for y in 0..source.h { for x in 0..source.w {
            if source.pixels[(y * source.w + x) as usize].a {
                bounds[0] = bounds[0].min(x); bounds[1] = bounds[1].max(x);
                bounds[2] = bounds[2].min(y); bounds[3] = bounds[3].max(y);
            }
        } }
        cache.push_back(AlphaBounds { pixels: source.pixels.clone(), w: source.w, h: source.h, bounds });
        if cache.len() > 128 { cache.pop_front(); }
        bounds
    })
}

fn foot_extent(source: &SurfaceCaster) -> Option<(f32, f32)> {
    if source.foot.len() != source.w as usize {
        return None;
    }
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for span in &*source.foot {
        if !span[0].is_finite() || !span[1].is_finite() || span[0] >= span[1] {
            return None;
        }
        lo = lo.min(span[0]);
        hi = hi.max(span[1]);
    }
    (lo < hi).then_some((lo, hi))
}

/// A receiver inside the source's influence sphere cannot have a blocking
/// segment outside that sphere. Reject distant full cells before inspecting
/// their alpha; atlas padding deliberately makes this box conservative.
fn sphere_reaches_cell(source: &SurfaceCaster, light: [f32; 3], radius2: f32) -> bool {
    if source.w <= 0 || source.h <= 0 {
        return false;
    }
    let x0 = source.x.floor()
        - if source.flip {
            (source.w - 1 - source.anchor.0) as f32
        } else {
            source.anchor.0 as f32
        };
    let y0 = source.y.floor() - source.anchor.1 as f32;
    let (y_lo, y_hi, z_lo, z_hi) = if let Some((lo, hi)) = foot_extent(source) {
        (lo, hi, source.min_height, hi - y0)
    } else {
        (
            source.base - HALF_DEPTH,
            source.base + HALF_DEPTH,
            source.base - y0 - source.h as f32,
            source.base - y0,
        )
    };
    if z_hi < z_lo {
        return false;
    }
    let lo = [x0, y_lo, z_lo];
    let hi = [x0 + source.w as f32, y_hi, z_hi];
    let mut distance2 = 0.0;
    for axis in 0..3 {
        let d = (lo[axis] - light[axis])
            .max(light[axis] - hi[axis])
            .max(0.0);
        distance2 += d * d;
    }
    distance2 <= radius2
}

struct Native<'a> {
    source: &'a SurfaceCaster,
    x0: f32,
    y0: f32,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    min_z: f32,
    max_z: f32,
    image_top: f32,
    image_bottom: f32,
    volume: bool,
    visible: u8,
}

impl<'a> Native<'a> {
    fn new(source: &'a SurfaceCaster) -> Option<Self> {
        if source.w <= 0
            || source.h <= 0
            || source.opacity <= 0.0
            || source.pixels.len() != (source.w * source.h) as usize
        {
            return None;
        }
        let [mut left, mut right, top, bottom] = alpha_bounds(source);
        if right < left {
            return None;
        }
        if source.flip { (left, right) = (source.w - 1 - right, source.w - 1 - left); }
        let x0 = source.x.floor()
            - if source.flip {
                (source.w - 1 - source.anchor.0) as f32
            } else {
                source.anchor.0 as f32
            };
        let y0 = source.y.floor() - source.anchor.1 as f32;
        let extent = foot_extent(source);
        let volume = extent.is_some();
        let (min_y, max_y) = extent.unwrap_or((source.base - HALF_DEPTH, source.base + HALF_DEPTH));
        let min_z = if volume {
            source.min_height
        } else {
            source.base - y0 - bottom as f32 - 1.0
        };
        let max_z = if volume {
            max_y - y0 - top as f32
        } else {
            source.base - y0 - top as f32
        };
        if max_z < min_z {
            return None;
        }
        Some(Self {
            source,
            x0,
            y0,
            min_x: x0 + left as f32,
            max_x: x0 + right as f32 + 1.0,
            min_y,
            max_y,
            min_z,
            max_z,
            image_top: y0 + top as f32 - if volume { 0.0 } else { HALF_DEPTH },
            image_bottom: y0 + bottom as f32 + 1.0 + if volume { 0.0 } else { HALF_DEPTH },
            volume,
            visible: ((1.0 - source.opacity.clamp(0.0, 1.0)) * 255.0).round() as u8,
        })
    }

    fn opaque(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.source.w || y >= self.source.h {
            return false;
        }
        let x = if self.source.flip {
            self.source.w - 1 - x
        } else {
            x
        };
        self.source.pixels[(y * self.source.w + x) as usize].a
    }

    /// Only actual emissive ink can identify a source's own housing. Merely
    /// putting a moving light behind ordinary opaque stone must still shadow.
    fn houses_emitter(&self, light: PointLight) -> bool {
        let ground_y = light.y + light.height;
        if ground_y < self.min_y - 3.5 || ground_y > self.max_y + 3.5 {
            return false;
        }
        // A source can only be housed by ink within three projected pixels. Inspect that
        // small native window, retaining the original exact distance checks at its edges.
        // One extra candidate pixel makes the window conservative under f32 cancellation.
        let y0 = ((light.y - self.y0 - 4.0).floor() as i32).max(0);
        let y1 = ((light.y - self.y0 + 4.0).ceil() as i32).min(self.source.h - 1);
        let x0 = ((light.x - self.x0 - 4.0).floor() as i32).max(0);
        let x1 = ((light.x - self.x0 + 4.0).ceil() as i32).min(self.source.w - 1);
        for y in y0..=y1 {
            let wy = self.y0 + y as f32;
            if (wy - light.y).abs() > 3.0 {
                continue;
            }
            for displayed_x in x0..=x1 {
                let x = if self.source.flip { self.source.w - 1 - displayed_x } else { displayed_x };
                let pixel = &self.source.pixels[(y * self.source.w + x) as usize];
                if !pixel.a || !pixel.emit {
                    continue;
                }
                let (lo, hi) = if self.volume {
                    let span = self.source.foot[x as usize];
                    (span[0], span[1])
                } else {
                    (self.source.base, self.source.base)
                };
                if ground_y < lo - 3.0
                    || ground_y > hi + 3.0
                    || light.height < lo - wy - 3.0
                    || light.height > hi - wy + 3.0
                    || light.height < self.min_z - 3.0
                {
                    continue;
                }
                let world_x = self.x0
                    + if self.source.flip {
                        (self.source.w - 1 - x) as f32
                    } else {
                        x as f32
                    };
                if (world_x - light.x).abs() <= 3.0 {
                    return true;
                }
            }
        }
        false
    }

    fn depth_interval(&self, light: [f32; 3], receiver: [f32; 3]) -> Option<(f32, f32)> {
        let mut interval = (EPS, 1.0 - EPS);
        if !clip_axis(
            light[1],
            receiver[1] - light[1],
            self.min_y,
            self.max_y,
            &mut interval,
        ) || !clip_axis(
            light[2],
            receiver[2] - light[2],
            self.min_z,
            self.max_z,
            &mut interval,
        ) {
            return None;
        }
        Some(interval)
    }

    /// Y/Z and projected-Y clipping are identical for every floor pixel on one scanline.
    /// Prepare that exact interval once; only the receiver X differs within the row.
    fn ray_interval(&self, light: [f32; 3], receiver: [f32; 3]) -> Option<(f32, f32)> {
        let Some(mut interval) = self.depth_interval(light, receiver) else {
            return None;
        };
        if self.volume
            && !clip_axis(
                light[1] - light[2],
                receiver[1] - receiver[2] - light[1] + light[2],
                self.image_top,
                self.image_bottom,
                &mut interval,
            )
        {
            return None;
        }
        Some(interval)
    }

    /// Intersect the source/receiver segment with native alpha, rather than
    /// treating the sprite's bounding rectangle as a solid obstruction.
    fn blocks(&self, light: [f32; 3], receiver: [f32; 3]) -> bool {
        let Some(interval) = self.ray_interval(light, receiver) else { return false };
        self.blocks_from_interval(light, receiver, interval)
    }

    fn blocks_from_interval(&self, light: [f32; 3], receiver: [f32; 3], mut interval: (f32, f32)) -> bool {
        if !clip_axis(light[0], receiver[0] - light[0], self.min_x, self.max_x, &mut interval) {
            return false;
        }
        let (start, end) = interval;
        let origin = [
            light[0] - self.x0,
            if self.volume {
                light[1] - light[2] - self.y0
            } else {
                self.source.base - light[2] - self.y0
            },
        ];
        let delta = [
            receiver[0] - light[0],
            if self.volume {
                receiver[1] - receiver[2] - light[1] + light[2]
            } else {
                light[2] - receiver[2]
            },
        ];
        // Move just inside the clipped segment, avoiding a spurious outside
        // pixel when a ray enters exactly through a right/bottom cell edge.
        let t = (start + EPS.min((end - start).max(0.0) * 0.25)).min(end);
        let mut x = (origin[0] + delta[0] * t).floor() as i32;
        let mut y = (origin[1] + delta[1] * t).floor() as i32;
        let step_x = if delta[0] > EPS {
            1
        } else if delta[0] < -EPS {
            -1
        } else {
            0
        };
        let step_y = if delta[1] > EPS {
            1
        } else if delta[1] < -EPS {
            -1
        } else {
            0
        };
        let next = |cell: i32, origin: f32, delta: f32, step: i32| {
            if step == 0 {
                f32::INFINITY
            } else {
                ((cell + i32::from(step > 0)) as f32 - origin) / delta
            }
        };
        let mut tx = next(x, origin[0], delta[0], step_x);
        let mut ty = next(y, origin[1], delta[1], step_y);
        let dt_x = if step_x == 0 {
            f32::INFINITY
        } else {
            delta[0].abs().recip()
        };
        let dt_y = if step_y == 0 {
            f32::INFINITY
        } else {
            delta[1].abs().recip()
        };
        let mut cell_start = start;
        loop {
            if self.opaque(x, y) {
                if !self.volume {
                    return true;
                }
                let source_x = if self.source.flip {
                    self.source.w - 1 - x
                } else {
                    x
                };
                let span = self.source.foot[source_x as usize];
                let mut cell_interval = (cell_start, tx.min(ty).min(end));
                if clip_axis(
                    light[1],
                    receiver[1] - light[1],
                    span[0],
                    span[1],
                    &mut cell_interval,
                ) {
                    return true;
                }
            }
            let next_t = tx.min(ty);
            if next_t > end + EPS || !next_t.is_finite() {
                return false;
            }
            cell_start = next_t;
            // Both axes advance together at a corner: touching a diagonal
            // opaque pixel at a zero-area corner does not fill an alpha hole.
            if tx <= next_t + EPS {
                x += step_x;
                tx += dt_x;
            }
            if ty <= next_t + EPS {
                y += step_y;
                ty += dt_y;
            }
            if x < 0 || y < 0 || x >= self.source.w || y >= self.source.h {
                return false;
            }
        }
    }

    #[cfg(test)]
    fn blocks_reference(&self, light: [f32; 3], receiver: [f32; 3]) -> bool {
        let Some(mut interval) = self.depth_interval(light, receiver) else {
            return false;
        };
        if !clip_axis(
            light[0],
            receiver[0] - light[0],
            self.min_x,
            self.max_x,
            &mut interval,
        ) {
            return false;
        }
        if self.volume
            && !clip_axis(
                light[1] - light[2],
                receiver[1] - receiver[2] - light[1] + light[2],
                self.image_top,
                self.image_bottom,
                &mut interval,
            )
        {
            return false;
        }
        let (start, end) = interval;
        let origin = [
            light[0] - self.x0,
            if self.volume {
                light[1] - light[2] - self.y0
            } else {
                self.source.base - light[2] - self.y0
            },
        ];
        let delta = [
            receiver[0] - light[0],
            if self.volume {
                receiver[1] - receiver[2] - light[1] + light[2]
            } else {
                light[2] - receiver[2]
            },
        ];
        // Move just inside the clipped segment, avoiding a spurious outside
        // pixel when a ray enters exactly through a right/bottom cell edge.
        let t = (start + EPS.min((end - start).max(0.0) * 0.25)).min(end);
        let mut x = (origin[0] + delta[0] * t).floor() as i32;
        let mut y = (origin[1] + delta[1] * t).floor() as i32;
        let step_x = if delta[0] > EPS {
            1
        } else if delta[0] < -EPS {
            -1
        } else {
            0
        };
        let step_y = if delta[1] > EPS {
            1
        } else if delta[1] < -EPS {
            -1
        } else {
            0
        };
        let next = |cell: i32, origin: f32, delta: f32, step: i32| {
            if step == 0 {
                f32::INFINITY
            } else {
                ((cell + i32::from(step > 0)) as f32 - origin) / delta
            }
        };
        let mut tx = next(x, origin[0], delta[0], step_x);
        let mut ty = next(y, origin[1], delta[1], step_y);
        let dt_x = if step_x == 0 {
            f32::INFINITY
        } else {
            delta[0].abs().recip()
        };
        let dt_y = if step_y == 0 {
            f32::INFINITY
        } else {
            delta[1].abs().recip()
        };
        let mut cell_start = start;
        loop {
            if self.opaque(x, y) {
                if !self.volume {
                    return true;
                }
                let source_x = if self.source.flip {
                    self.source.w - 1 - x
                } else {
                    x
                };
                let span = self.source.foot[source_x as usize];
                let mut cell_interval = (cell_start, tx.min(ty).min(end));
                if clip_axis(
                    light[1],
                    receiver[1] - light[1],
                    span[0],
                    span[1],
                    &mut cell_interval,
                ) {
                    return true;
                }
            }
            let next_t = tx.min(ty);
            if next_t > end + EPS || !next_t.is_finite() {
                return false;
            }
            cell_start = next_t;
            // Both axes advance together at a corner: touching a diagonal
            // opaque pixel at a zero-area corner does not fill an alpha hole.
            if tx <= next_t + EPS {
                x += step_x;
                tx += dt_x;
            }
            if ty <= next_t + EPS {
                y += step_y;
                ty += dt_y;
            }
            if x < 0 || y < 0 || x >= self.source.w || y >= self.source.h {
                return false;
            }
        }
    }

    /// Original conservative X bounds retained as an independent mask oracle.
    /// A caster above the emitter's height can project to infinity; clipping
    /// the resulting cone to the receiving frame handles that without caps.
    #[cfg(test)]
    fn ground_span(
        &self,
        light: [f32; 3],
        world_y: f32,
        clip: (i32, i32),
        cam_x: i32,
    ) -> Option<(i32, i32)> {
        let interval = self.depth_interval(light, [light[0], world_y, 0.0])?;
        self.inverse_span(light[0], interval, clip, cam_x)
    }

    /// The 2D projected shadow cone conservatively bounds every receiver
    /// height. A 3D ray projects to the same screen line, so no raised pixel
    /// outside this span can intersect the sprite's image rectangle.
    fn raised_span(
        &self,
        light: PointLight,
        projected_y: f32,
        clip: (i32, i32),
        cam_x: i32,
    ) -> Option<(i32, i32)> {
        let mut interval = (EPS, 1.0 - EPS);
        if !clip_axis(
            light.y,
            projected_y - light.y,
            self.image_top,
            self.image_bottom,
            &mut interval,
        ) {
            return None;
        }
        self.inverse_span(light.x, interval, clip, cam_x)
    }

    fn inverse_span(
        &self,
        light_x: f32,
        interval: (f32, f32),
        clip: (i32, i32),
        cam_x: i32,
    ) -> Option<(i32, i32)> {
        if interval.0 <= EPS * 2.0 {
            return Some(clip);
        }
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for x in [self.min_x, self.max_x] {
            for t in [interval.0, interval.1] {
                let projected = light_x + (x - light_x) / t;
                lo = lo.min(projected);
                hi = hi.max(projected);
            }
        }
        let lo = ((lo - cam_x as f32).floor() as i32 - 1).max(clip.0);
        let hi = ((hi - cam_x as f32).ceil() as i32 + 1).min(clip.1);
        (lo <= hi).then_some((lo, hi))
    }
}

/// Clip a segment parameter against one closed slab; reject zero-length rays
/// outside it. Keeping this analytic avoids sampling gaps in tall silhouettes.
fn clip_axis(origin: f32, delta: f32, lo: f32, hi: f32, t: &mut (f32, f32)) -> bool {
    if delta.abs() < EPS {
        return t.0 <= t.1 && origin >= lo && origin <= hi;
    }
    let (a, b) = ((lo - origin) / delta, (hi - origin) / delta);
    t.0 = t.0.max(a.min(b));
    t.1 = t.1.min(a.max(b));
    t.0 <= t.1
}

/// Visibility of one real point light: 255 fully visible, 0 fully blocked.
/// Its physical center is (x, y+height, height). A spherical world radius
/// projects to horizontal r and vertical sqrt(2)*r; a screen-radius circle
/// would incorrectly discard elevated receiving faces.
pub(super) fn visibility(f: &Frame, light: PointLight) -> Vec<u8> {
    let mut result = vec![255; W * H];
    if !light.radius.is_finite() || light.radius <= 0.0 || light.intensity <= 0.0 {
        return result;
    }
    let light_world = [light.x, light.y + light.height, light.height];
    let projected = [light.x - f.cam_x as f32, light.y - f.cam_y as f32];
    let x0 = ((projected[0] - light.radius).floor() as i32).max(0);
    let x1 = ((projected[0] + light.radius).ceil() as i32).min(W as i32 - 1);
    let projected_radius_y = light.radius * std::f32::consts::SQRT_2;
    let y0 = ((projected[1] - projected_radius_y).floor() as i32).max(0);
    let y1 = ((projected[1] + projected_radius_y).ceil() as i32).min(H as i32 - 1);
    if x0 > x1 || y0 > y1 {
        return result;
    }
    let radius2 = light.radius * light.radius;
    let mut rows = Vec::new();
    let mut ground = Vec::new();
    let mut raised = Vec::new();
    for y in y0..=y1 {
        let world_y = y as f32 + f.cam_y as f32 + 0.5;
        let ground_dy = world_y - light_world[1];
        let span2 = radius2 - ground_dy * ground_dy - light.height * light.height;
        let (lo, hi) = if span2 > 0.0 {
            let dx = span2.sqrt();
            (
                ((projected[0] - dx).floor() as i32).max(x0),
                ((projected[0] + dx).ceil() as i32).min(x1),
            )
        } else {
            (1, 0)
        };
        let ground_first = ground.len();
        let first = raised.len();
        let mut height_range = [f32::INFINITY, f32::NEG_INFINITY];
        for x in x0..=x1 {
            let i = y as usize * W + x as usize;
            let surface = &f.surfaces[i];
            if surface.kind == 0 { continue; }
            if surface.height > EPS {
                let receiver = [
                    x as f32 + f.cam_x as f32 + 0.5,
                    world_y + surface.height,
                    surface.height,
                ];
                let distance2 = (receiver[0] - light_world[0]).powi(2)
                    + (receiver[1] - light_world[1]).powi(2)
                    + (receiver[2] - light_world[2]).powi(2);
                if distance2 >= radius2 {
                    continue;
                }
                raised.push((i, receiver, surface.owner));
                height_range[0] = height_range[0].min(surface.height);
                height_range[1] = height_range[1].max(surface.height);
            } else if x >= lo && x <= hi {
                ground.push(i);
            }
        }
        if ground_first != ground.len() || first != raised.len() {
            rows.push((y, ground_first, ground.len(), first, raised.len(), height_range));
        }
    }
    for source in &f.surface_casters {
        if !sphere_reaches_cell(source, light_world, radius2) {
            continue;
        }
        let Some(caster) = Native::new(source) else {
            continue;
        };
        if caster.houses_emitter(light) {
            continue;
        }
        for &(y, ground_first, ground_end, first, end, height_range) in &rows {
            let world_y = y as f32 + f.cam_y as f32 + 0.5;
            let ground_row = &ground[ground_first..ground_end];
            if let Some((interval, (lo, hi))) = (!ground_row.is_empty())
                .then(|| {
                    let interval = caster.ray_interval(light_world, [light_world[0], world_y, 0.0])?;
                    let span = caster.inverse_span(light_world[0], interval,
                        ((ground_row[0] % W) as i32, (ground_row[ground_row.len() - 1] % W) as i32), f.cam_x)?;
                    Some((interval, span))
                })
                .flatten()
            {
                let first = ground_row.partition_point(|&i| i % W < lo as usize);
                let end = ground_row.partition_point(|&i| i % W <= hi as usize);
                for &i in &ground_row[first..end] {
                    let surface = &f.surfaces[i];
                    if source.owner != 0 && surface.owner == source.owner
                        || result[i] <= caster.visible
                    {
                        continue;
                    }
                    let x = i % W;
                    let receiver = [x as f32 + f.cam_x as f32 + 0.5, world_y, 0.0];
                    if caster.blocks_from_interval(light_world, receiver, interval) {
                        result[i] = caster.visible;
                    }
                }
            }
            if first == end
                || light.height > caster.max_z && height_range[0] > caster.max_z
                || light.height < caster.min_z && height_range[1] < caster.min_z { continue; }
            let row = &raised[first..end];
            let Some((lo, hi)) = caster.raised_span(light, world_y,
                ((row[0].0 % W) as i32, (row[row.len() - 1].0 % W) as i32), f.cam_x) else {
                continue;
            };
            let first = row.partition_point(|&(i, _, _)| (i % W) < lo as usize);
            let end = row.partition_point(|&(i, _, _)| (i % W) <= hi as usize);
            for &(i, receiver, owner) in &row[first..end] {
                if source.owner != 0 && owner == source.owner || result[i] <= caster.visible {
                    continue;
                }
                if light.height > caster.max_z && receiver[2] > caster.max_z
                    || light.height < caster.min_z && receiver[2] < caster.min_z { continue; }
                if caster.blocks(light_world, receiver) {
                    result[i] = caster.visible;
                }
            }
        }
    }
    result
}

#[cfg(test)]
fn visibility_reference(f: &Frame, light: PointLight) -> Vec<u8> {
    let mut result = vec![255; W * H];
    if !light.radius.is_finite() || light.radius <= 0.0 || light.intensity <= 0.0 {
        return result;
    }
    let light_world = [light.x, light.y + light.height, light.height];
    let projected = [light.x - f.cam_x as f32, light.y - f.cam_y as f32];
    let x0 = ((projected[0] - light.radius).floor() as i32).max(0);
    let x1 = ((projected[0] + light.radius).ceil() as i32).min(W as i32 - 1);
    let projected_radius_y = light.radius * std::f32::consts::SQRT_2;
    let y0 = ((projected[1] - projected_radius_y).floor() as i32).max(0);
    let y1 = ((projected[1] + projected_radius_y).ceil() as i32).min(H as i32 - 1);
    if x0 > x1 || y0 > y1 {
        return result;
    }
    let radius2 = light.radius * light.radius;
    let mut rows = Vec::new();
    let mut raised = Vec::new();
    for y in y0..=y1 {
        let world_y = y as f32 + f.cam_y as f32 + 0.5;
        let ground_dy = world_y - light_world[1];
        let span2 = radius2 - ground_dy * ground_dy - light.height * light.height;
        let (lo, hi) = if span2 > 0.0 {
            let dx = span2.sqrt();
            (
                ((projected[0] - dx).floor() as i32).max(x0),
                ((projected[0] + dx).ceil() as i32).min(x1),
            )
        } else {
            (1, 0)
        };
        let first = raised.len();
        for x in x0..=x1 {
            let i = y as usize * W + x as usize;
            let surface = &f.surfaces[i];
            if surface.kind != 0 && surface.height > EPS {
                let receiver = [
                    x as f32 + f.cam_x as f32 + 0.5,
                    world_y + surface.height,
                    surface.height,
                ];
                let distance2 = (receiver[0] - light_world[0]).powi(2)
                    + (receiver[1] - light_world[1]).powi(2)
                    + (receiver[2] - light_world[2]).powi(2);
                if distance2 >= radius2 {
                    continue;
                }
                raised.push((i, receiver, surface.owner));
            }
        }
        rows.push((y, lo, hi, first, raised.len()));
    }
    for source in &f.surface_casters {
        if !sphere_reaches_cell(source, light_world, radius2) {
            continue;
        }
        let Some(caster) = Native::new(source) else {
            continue;
        };
        if caster.houses_emitter(light) {
            continue;
        }
        for &(y, row_lo, row_hi, first, end) in &rows {
            let world_y = y as f32 + f.cam_y as f32 + 0.5;
            if let Some((lo, hi)) = (row_lo <= row_hi)
                .then(|| caster.ground_span(light_world, world_y, (row_lo, row_hi), f.cam_x))
                .flatten()
            {
                for x in lo..=hi {
                    let i = y as usize * W + x as usize;
                    let surface = &f.surfaces[i];
                    if surface.kind == 0
                        || surface.height > EPS
                        || source.owner != 0 && surface.owner == source.owner
                        || result[i] <= caster.visible
                    {
                        continue;
                    }
                    let receiver = [x as f32 + f.cam_x as f32 + 0.5, world_y, 0.0];
                    if caster.blocks_reference(light_world, receiver) {
                        result[i] = caster.visible;
                    }
                }
            }
            let Some((lo, hi)) = caster.raised_span(light, world_y, (x0, x1), f.cam_x) else {
                continue;
            };
            let row = &raised[first..end];
            let first = row.partition_point(|&(i, _, _)| (i % W) < lo as usize);
            let end = row.partition_point(|&(i, _, _)| (i % W) <= hi as usize);
            for &(i, receiver, owner) in &row[first..end] {
                if source.owner != 0 && owner == source.owner || result[i] <= caster.visible {
                    continue;
                }
                if caster.blocks_reference(light_world, receiver) {
                    result[i] = caster.visible;
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::Rgb;
    use crate::gfx::surface::NormalPixel;
    use std::sync::Arc;

    fn caster(rows: &[&str], flip: bool) -> SurfaceCaster {
        let w = rows[0].len() as i32;
        SurfaceCaster {
            pixels: Arc::from(
                rows.iter()
                    .flat_map(|row| {
                        row.bytes().map(|p| NormalPixel {
                            n: [0, 127, 0],
                            a: p != b'.',
                            emit: p == b'e',
                        })
                    })
                    .collect::<Vec<_>>(),
            ),
            w,
            h: rows.len() as i32,
            anchor: (0, 0),
            x: 4.0,
            y: 4.0,
            base: 8.0,
            flip,
            owner: 7,
            opacity: 1.0,
            foot: Arc::from([]),
            min_height: 0.0,
            reflectivity: 0.0,
        }
    }

    /// Compare the optimized receiver lists/height culls against the original complete row
    /// walk, including empty sky, subpixel heights, raised actors, native holes and transparency.
    #[test]
    fn sparse_point_receivers_match_original_visibility_masks() {
        let mut f = Frame::new();
        for y in 0..100 { for x in 0..164 {
            let height = [0.0, 0.000_005, 0.125, 7.875, 8.0, 16.125, 47.75, 110.0][(x / 7 + y / 8) % 8];
            f.surfaces[y * W + x] = crate::gfx::frame::Surface { normal: [0.0, 0.0, 1.0], height,
                owner: if (x + y) % 6 == 0 { 7 } else { 0 }, kind: if (x + y) % 9 == 0 { 0 } else { 2 } };
        } }
        for (i, (x, y, base)) in [(8.25, 2.5, 22.0), (28.75, 16.125, 30.0),
            (-10.5, -10.25, 20.0), (95.125, -24.5, 10.0)].into_iter().enumerate() {
            let mut source = caster(&["#####"], i % 2 == 1);
            source.x = x; source.y = y; source.base = base;
            source.h = 36;
            source.pixels = Arc::from((0..180).map(|p| NormalPixel {
                n: [0, 127, 0], a: p % 5 == 0 || p % 5 == 4 || p / 5 % 5 == i,
                emit: p % 19 == i,
            }).collect::<Vec<_>>());
            source.foot = if i == 3 { Arc::from([]) } else {
                Arc::from((0..5).map(|p| [base - 3.0, base + p as f32 * 0.375]).collect::<Vec<_>>())
            };
            source.min_height = if i == 2 { 4.3 } else { 0.0 };
            source.opacity = if i == 1 { 0.5 } else { 1.0 };
            f.surface_casters.push(source);
        }
        let mut changed = 0;
        for (cx, cy) in [(0, 0), (-7, 3), (14, 5)] {
            f.cam_x = cx; f.cam_y = cy;
            for (x, y, height, radius) in [(4.5, 0.5, 0.0, 32.0), (12.5, -9.5, 12.0, 64.0),
                (60.25, 14.0, 110.0, 152.0), (100.0, 80.0, 14.0, 85.0)] {
                let light = PointLight { x, y, height, radius, color: Rgb::WHITE, intensity: 1.0 };
                let actual = visibility(&f, light);
                assert_eq!(actual, visibility_reference(&f, light),
                    "camera=({cx},{cy}), source=({x},{y},{height}), radius={radius}");
                changed += actual.iter().filter(|&&v| v < 255).count();
            }
        }
        assert!(changed > 100, "compare actual blocked receiver pixels");
    }

    #[test]
    fn cached_alpha_bounds_follow_immutable_pose_identity_and_shape() {
        let original = caster(&["....", ".e..", "...."], false);
        assert_eq!(alpha_bounds(&original), [1, 1, 1, 1]);
        let mut changed = original.clone();
        Arc::make_mut(&mut changed.pixels)[0].a = true;
        assert!(!Arc::ptr_eq(&original.pixels, &changed.pixels));
        assert_eq!(alpha_bounds(&changed), [0, 1, 0, 1]);
        assert_eq!(alpha_bounds(&original), [1, 1, 1, 1], "cached old pose must remain immutable");
        changed.flip = true;
        let native = Native::new(&changed).unwrap();
        assert_eq!((native.min_x, native.max_x), (3.0, 5.0));
        // Reinterpreting an atlas allocation with another valid shape is a different key.
        changed.w = 3; changed.h = 4;
        assert_eq!(alpha_bounds(&changed), [0, 2, 0, 1]);
    }

    /// Windowing the emissive search cannot change the original full-cell housing rule.
    #[test]
    fn local_emitter_window_matches_full_native_search() {
        for flip in [false, true] {
            let mut source = caster(&["e...e", ".e...", ".....", "...e.", "e...e"], flip);
            source.x += 0.125; source.y -= 0.75;
            source.foot = Arc::from((0..5).map(|x| [5.0, 6.0 + x as f32 * 0.25]).collect::<Vec<_>>());
            let native = Native::new(&source).unwrap();
            for x in -8..20 { for y in -8..20 { for height in [0.0, 1.5, 3.0, 7.25, 12.0] {
                let light = PointLight { x: x as f32 + 0.125, y: y as f32 + 0.5, height,
                    radius: 100.0, color: Rgb::WHITE, intensity: 1.0 };
                let ground = light.y + height;
                let expected = ground >= native.min_y - 3.5 && ground <= native.max_y + 3.5
                    && source.pixels.iter().enumerate().any(|(i,p)| {
                        let (sx,sy) = (i as i32 % source.w, i as i32 / source.w);
                        let wy = native.y0 + sy as f32;
                        let wx = native.x0 + if flip { source.w - 1 - sx } else { sx } as f32;
                        let [lo,hi] = source.foot[sx as usize];
                        p.a && p.emit && (wy-light.y).abs() <= 3.0 && (wx-light.x).abs() <= 3.0
                            && ground >= lo - 3.0 && ground <= hi + 3.0
                            && height >= lo - wy - 3.0 && height <= hi - wy + 3.0
                            && height >= native.min_z - 3.0
                    });
                assert_eq!(native.houses_emitter(light), expected, "flip={flip}, light={light:?}");
            } } }
        }
    }

    /// Prepared scanlines must preserve the original per-pixel Y/Z/projected-Y clips, including
    /// high sources, grazing floor rays, mirrored holes and the legacy thin native sheet.
    #[test]
    fn prepared_ground_ray_intervals_match_original_pixel_rays() {
        let mut hits = 0;
        for volume in [false, true] { for flip in [false, true] {
            let mut source = caster(&["###", "#.#", "#.#", "###", ".#.", "###"], flip);
            source.y = 2.0;
            if volume { source.foot = Arc::from(vec![[5.0, 7.0], [6.0, 8.0], [5.0, 7.0]]); }
            let caster = Native::new(&source).unwrap();
            for light in [[-8.0, 8.0, 12.0], [18.0, 8.0, 12.0], [5.0, 18.0, 3.0],
                [0.0, 8.0, 6.0], [5.5, 8.0, 0.0], [5.5, 8.0, 112.0]] {
                for y in -12..30 {
                    let wy = y as f32 + 0.5;
                    let interval = caster.ray_interval(light, [light[0], wy, 0.0]);
                    for x in -20..36 {
                        let receiver = [x as f32 + 0.5, wy, 0.0];
                        let actual = interval.is_some_and(|interval| caster.blocks_from_interval(light, receiver, interval));
                        let expected = caster.blocks_reference(light, receiver);
                        assert_eq!(actual, expected, "volume={volume}, flip={flip}, light={light:?}, receiver={receiver:?}");
                        hits += usize::from(actual);
                    }
                }
            }
        } }
        assert!(hits > 100);
    }

    #[test]
    fn native_alpha_holes_and_flip_are_geometric() {
        let source = caster(&["###", "#..", "###"], false);
        let c = Native::new(&source).unwrap();
        let light = [5.5, 0.0, 5.0];
        assert!(
            !c.blocks(light, [5.5, 16.0, 0.0]),
            "ray passes native transparent interior"
        );
        assert!(
            c.blocks(light, [3.5, 16.0, 0.0]),
            "ray intersects left native bar"
        );
        let mut flipped = caster(&["###", "#..", "###"], true);
        // Reposition the reflected anchor so the visible rectangle stays fixed.
        flipped.x += 2.0;
        let c = Native::new(&flipped).unwrap();
        assert!(!c.blocks(light, [3.5, 16.0, 0.0]));
        assert!(c.blocks(light, [7.5, 16.0, 0.0]));
    }

    #[test]
    fn receiving_height_changes_visibility_on_the_same_projected_pixel() {
        let source = caster(&["###", "...", "..."], false);
        let c = Native::new(&source).unwrap();
        let light = [5.5, 0.0, 8.0];
        assert!(c.blocks(light, [5.5, 16.0, 0.0]));
        // Same projected screen y=16, but an elevated surface lies at world y=20,z=4.
        assert!(!c.blocks(light, [5.5, 20.0, 4.0]));
    }

    #[test]
    fn a_higher_light_shortens_ground_shadow() {
        let source = caster(&["###", "###", "###"], false);
        let c = Native::new(&source).unwrap();
        assert!(c.blocks([5.5, 0.0, 6.0], [5.5, 20.0, 0.0]));
        assert!(!c.blocks([5.5, 0.0, 14.0], [5.5, 20.0, 0.0]));
    }

    #[test]
    fn only_matching_emissive_geometry_excludes_the_source_housing() {
        let mut source = caster(&["###", "#e#", "###"], false);
        let light = PointLight {
            x: 5.0,
            y: 5.0,
            height: 3.0,
            radius: 50.0,
            color: Rgb::WHITE,
            intensity: 1.0,
        };
        assert!(Native::new(&source).unwrap().houses_emitter(light));
        source.pixels = Arc::from(vec![
            NormalPixel {
                n: [0, 127, 0],
                a: true,
                emit: false
            };
            9
        ]);
        assert!(!Native::new(&source).unwrap().houses_emitter(light));
        let source = caster(&["###", "#e#", "###"], false);
        let mut displaced = light;
        displaced.height += 8.0;
        assert!(!Native::new(&source).unwrap().houses_emitter(displaced));
    }

    #[test]
    fn coplanar_light_still_intersects_finite_native_pixel_depth() {
        let source = caster(&["###", "###", "###"], false);
        let c = Native::new(&source).unwrap();
        assert!(c.blocks([0.0, 8.0, 6.0], [10.0, 8.0, 0.0]));
        assert!(!c.blocks([0.0, 12.0, 6.0], [10.0, 12.0, 0.0]));
    }

    #[test]
    fn projected_scanline_bounds_preserve_every_geometric_hit() {
        let source = caster(&["###", "#.#", "###"], false);
        let c = Native::new(&source).unwrap();
        let mut hits = 0;
        for (x, y, height) in [
            (-8.0, -4.0, 12.0),
            (18.0, -4.0, 12.0),
            (5.0, 18.0, 3.0),
            (0.0, 2.0, 6.0),
        ] {
            let light = PointLight {
                x,
                y,
                height,
                radius: 100.0,
                color: Rgb::WHITE,
                intensity: 1.0,
            };
            let world = [x, y + height, height];
            for py in -12..30 {
                let projected_y = py as f32 + 0.5;
                let raised_span = c.raised_span(light, projected_y, (-20, 35), 0);
                let ground_span = c.ground_span(world, projected_y, (-20, 35), 0);
                for px in -20..36 {
                    for h in [0.0, 1.0, 3.0, 7.0] {
                        let receiver = [px as f32 + 0.5, projected_y + h, h];
                        if !c.blocks(world, receiver) {
                            continue;
                        }
                        hits += 1;
                        let (lo, hi) = raised_span.expect("projected cone cannot omit a real hit");
                        assert!((lo..=hi).contains(&px));
                        if h == 0.0 {
                            let (lo, hi) = ground_span
                                .expect("inverse ground projection cannot omit a real hit");
                            assert!((lo..=hi).contains(&px));
                        }
                    }
                }
            }
        }
        assert!(hits > 100, "audit actual shadow rays, not only empty space");
    }

    #[test]
    fn world_sphere_covers_raised_receivers_outside_a_projected_radius_circle() {
        let mut f = Frame::new();
        let i = 70 * W + 100;
        f.surfaces[i] = crate::gfx::frame::Surface {
            normal: [0.0, 1.0, 0.0],
            height: 10.0,
            owner: 3,
            kind: 2,
        };
        let mut source = caster(&["###", "###", "###"], false);
        source.x = 99.0;
        source.y = 54.0;
        source.base = 70.0;
        f.surface_casters.push(source);
        let light = PointLight {
            x: 100.5,
            y: 40.5,
            height: 20.0,
            radius: 25.0,
            color: Rgb::WHITE,
            intensity: 1.0,
        };
        assert_eq!(
            visibility(&f, light)[i],
            0,
            "screen separation 30 exceeds r=25, but physical distance sqrt(500) is inside"
        );
    }

    #[test]
    fn cell_sphere_prefilter_preserves_mirrored_anchors_height_and_pixel_depth() {
        for flip in [false, true] {
            let mut source = caster(&["###", "#.#", "###"], flip);
            source.x += 0.75;
            source.y += 0.25;
            let left = if flip { 2.0 } else { 4.0 };
            // Tangency to the full cell remains eligible, including the
            // asymmetric anchor's actual reflected world bounds.
            assert!(sphere_reaches_cell(&source, [left - 1.0, 8.0, 2.0], 1.0));
            assert!(!sphere_reaches_cell(&source, [left - 1.0, 8.0, 2.0], 0.81));
            assert!(sphere_reaches_cell(
                &source,
                [left + 0.5, 8.51, 2.0],
                0.0004
            ));
            assert!(sphere_reaches_cell(&source, [left + 0.5, 8.0, 4.1], 0.0121));
            assert!(sphere_reaches_cell(&source, [left + 0.5, 8.0, 0.9], 0.0121));
            assert!(!sphere_reaches_cell(&source, [left + 0.5, 8.0, 12.0], 1.0));
        }
    }

    #[test]
    fn grazing_light_casts_a_finite_native_footprint_instead_of_one_pixel_rail() {
        let mut source = caster(&["###", "###", "###", "###", "###", "###"], false);
        source.y = 2.0;
        source.foot = Arc::from(vec![[5.0, 8.0]; 3]);
        let c = Native::new(&source).unwrap();
        let light = [-12.0, 8.0, 12.0];
        for y in [5.5, 6.5, 7.5] {
            assert!(
                c.blocks(light, [6.5, y, 0.0]),
                "an edge-on light must retain all three physical foot-depth rows"
            );
        }
        for y in [4.5, 8.5] {
            assert!(!c.blocks(light, [6.5, y, 0.0]));
        }
    }

    #[test]
    fn finite_volume_contact_follows_the_native_rounded_foot_profile() {
        let mut source = caster(&["###", "###", "###", "###", "###", ".#."], false);
        source.y = 2.0;
        source.foot = Arc::from(vec![[5.0, 7.0], [6.0, 8.0], [5.0, 7.0]]);
        let c = Native::new(&source).unwrap();
        for (x, expected) in [(4.5, false), (5.5, true), (6.5, false)] {
            assert_eq!(
                c.blocks([x, 7.5, 12.0], [x, 7.5, 0.0]),
                expected,
                "only the middle native sole extends into this front contact row"
            );
        }
    }

    #[test]
    fn finite_volume_preserves_native_alpha_tunnels_and_asymmetric_flip() {
        let mut source = caster(&["###", "#..", "###"], false);
        source.foot = Arc::from(vec![[7.9, 8.1]; 3]);
        let light = [5.5, 0.0, 5.0];
        let c = Native::new(&source).unwrap();
        assert!(!c.blocks(light, [5.5, 16.0, 0.0]));
        assert!(c.blocks(light, [3.5, 16.0, 0.0]));
        source.flip = true;
        source.x += 2.0;
        let c = Native::new(&source).unwrap();
        assert!(!c.blocks(light, [3.5, 16.0, 0.0]));
        assert!(c.blocks(light, [7.5, 16.0, 0.0]));
    }

    #[test]
    fn finite_volume_scanline_and_sphere_bounds_preserve_alpha_ray_hits() {
        let mut source = caster(&["###", "###", "###", "###", "###", ".#."], false);
        source.y = 2.0;
        source.foot = Arc::from(vec![[5.0, 7.0], [6.0, 8.0], [5.0, 7.0]]);
        let c = Native::new(&source).unwrap();
        let mut hits = 0;
        for world in [
            [-8.0, 8.0, 12.0],
            [18.0, 8.0, 12.0],
            [5.0, 18.0, 3.0],
            [0.0, 8.0, 6.0],
        ] {
            let light = PointLight {
                x: world[0],
                y: world[1] - world[2],
                height: world[2],
                radius: 50.0,
                color: Rgb::WHITE,
                intensity: 1.0,
            };
            for py in -10..24 {
                let projected_y = py as f32 + 0.5;
                for px in -20..36 {
                    for h in [0.0, 3.0, 6.0] {
                        let receiver = [px as f32 + 0.5, projected_y + h, h];
                        if !c.blocks(world, receiver) {
                            continue;
                        }
                        hits += 1;
                        let (lo, hi) = c.raised_span(light, projected_y, (-20, 35), 0).expect(
                            "view-ray extrusion preserves the projected native alpha bound",
                        );
                        assert!((lo..=hi).contains(&px));
                        if h == 0.0 {
                            let (lo, hi) = c.ground_span(world, projected_y, (-20, 35), 0).expect(
                                "ground inverse projection contains every finite-volume hit",
                            );
                            assert!((lo..=hi).contains(&px));
                        }
                        let distance2 = (0..3)
                            .map(|i| (receiver[i] - world[i]).powi(2))
                            .sum::<f32>();
                        if distance2 < light.radius * light.radius {
                            assert!(sphere_reaches_cell(
                                &source,
                                world,
                                light.radius * light.radius
                            ));
                        }
                    }
                }
            }
        }
        assert!(hits > 10);
    }
}
