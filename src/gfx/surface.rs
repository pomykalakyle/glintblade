//! Per-visible-pixel normal lighting. Normal PNGs are vector data, never color/gamma data.
//! X right, Y south on the ground, Z up; projection is screen_y = Y-Z.
use super::frame::{Frame, H, Surface, SurfaceCaster, W};
use super::sprite::{Cell, Px};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default)]
pub struct NormalPixel {
    /// OpenGL image tangent: red right, green up, blue towards the viewer.
    pub n: [i8; 3],
    pub a: bool,
    pub emit: bool,
}
impl NormalPixel {
    pub fn decode(rgb: [u8; 3], a: bool, emit: bool) -> Self {
        let v = rgb.map(|c| c as f32 / 127.5 - 1.0);
        let l = (v.iter().map(|v| v * v).sum::<f32>()).sqrt().max(0.001);
        Self {
            n: v.map(|v| (v / l * 127.0).round() as i8),
            a,
            emit,
        }
    }
    pub fn upright(self, flip: bool) -> [f32; 3] {
        let k = 1.0 / 127.0;
        [
            self.n[0] as f32 * k * if flip { -1.0 } else { 1.0 },
            self.n[2] as f32 * k,
            self.n[1] as f32 * k,
        ]
    }
    pub fn floor(self) -> [f32; 3] {
        let k = 1.0 / 127.0;
        [
            self.n[0] as f32 * k,
            -self.n[1] as f32 * k,
            self.n[2] as f32 * k,
        ]
    }
}
/// Quiet geometric fallback for procedural sprites: broad volume from alpha only.
/// Albedo highlights/cracks are deliberately not interpreted as geometry.
pub fn fallback(w: i32, h: i32, px: &[Px]) -> Arc<[NormalPixel]> {
    if w <= 0 || h <= 0 {
        return Arc::from([]);
    }
    let mut result = Vec::with_capacity(px.len());
    let top = px.iter().position(|p| p.a).map_or(0, |i| i as i32 / w);
    for y in 0..h {
        let row = &px[(y * w) as usize..((y + 1) * w) as usize];
        let lo = row.iter().position(|p| p.a).unwrap_or(0) as f32;
        let hi = row.iter().rposition(|p| p.a).unwrap_or(0) as f32;
        for x in 0..w {
            let p = px[(y * w + x) as usize];
            let nx =
                ((x as f32 - (lo + hi) * 0.5) / ((hi - lo + 1.0) * 0.5)).clamp(-1.0, 1.0) * 0.45;
            let ny = (1.0 - (y - top) as f32 / 4.0).clamp(0.0, 1.0) * 0.40;
            let nz = (1.0 - nx * nx - ny * ny).sqrt();
            result.push(NormalPixel {
                n: [(nx * 127.0) as i8, (ny * 127.0) as i8, (nz * 127.0) as i8],
                a: p.a,
                emit: p.emit,
            });
        }
    }
    result.into()
}

pub fn base(cell: &Cell, anchor: (i32, i32), sy_world: i32, elevation: f32) -> f32 {
    let bottom = cell
        .px
        .iter()
        .rposition(|p| p.a)
        .map_or(anchor.1, |i| i as i32 / cell.w + 1);
    sy_world as f32 + (bottom - anchor.1) as f32 + elevation
}
/// Ground-depth profile from actual native foot pixels. The same projected alpha
/// bounds the finite volume above it, preserving overhangs and transparent gaps.
/// Actor selection uses the body center so a low sword/tail cannot become a sole.
pub fn foot_profile(
    cell: &Cell,
    anchor: (i32, i32),
    sy_world: i32,
    airborne: f32,
    actor_half_w: Option<f32>,
) -> Arc<[[f32; 2]]> {
    foot_profile_with_depth(
        cell,
        anchor,
        sy_world,
        airborne,
        actor_half_w,
        if actor_half_w.is_some() { 5 } else { 0 },
    )
}

pub fn foot_profile_with_depth(
    cell: &Cell,
    anchor: (i32, i32),
    sy_world: i32,
    airborne: f32,
    actor_half_w: Option<f32>,
    depth: i32,
) -> Arc<[[f32; 2]]> {
    if cell.w <= 0 || cell.h <= 0 {
        return Arc::from([]);
    }
    let (left, right) = actor_half_w.map_or((0, cell.w), |r| {
        (
            (anchor.0 - r.ceil() as i32).max(0),
            (anchor.0 + r.ceil() as i32 + 1).min(cell.w),
        )
    });
    let upper = if depth > 0 {
        (anchor.1 + 8).min(cell.h - 1)
    } else {
        cell.h - 1
    };
    let lower = if depth > 0 { (anchor.1 - 14).max(0) } else { 0 };
    let bottom = (lower..=upper).rev().find(|&y| {
        (left..right).any(|x| {
            let p = cell.at(x, y);
            p.a && !p.emit
        })
    });
    let bottom = bottom
        .or_else(|| cell.px.iter().rposition(|p| p.a).map(|i| i as i32 / cell.w))
        .unwrap_or(anchor.1);
    let depth = if depth > 0 {
        depth
    } else {
        ((right - left) as f32 * 0.22).round().clamp(4.0, 10.0) as i32
    };
    let top = (bottom - depth + 1).max(0);
    let offset = (sy_world - anchor.1) as f32 + airborne;
    let mut spans = vec![[f32::NAN; 2]; cell.w as usize];
    for x in left..right {
        let lo = (top..=bottom).find(|&y| {
            let p = cell.at(x, y);
            p.a && !p.emit
        });
        let hi = (top..=bottom).rev().find(|&y| {
            let p = cell.at(x, y);
            p.a && !p.emit
        });
        if let (Some(lo), Some(hi)) = (lo, hi) {
            spans[x as usize] = [offset + lo as f32, offset + hi as f32 + 1.0];
        }
    }
    let known: Vec<_> = spans
        .iter()
        .enumerate()
        .filter(|(_, s)| s[0].is_finite())
        .map(|(x, s)| (x, *s))
        .collect();
    for (x, s) in spans.iter_mut().enumerate() {
        if !s[0].is_finite() {
            *s = known.iter().min_by_key(|(i, _)| i.abs_diff(x)).map_or(
                [offset + bottom as f32 - 2.0, offset + bottom as f32 + 1.0],
                |(_, s)| *s,
            );
        }
    }
    spans.into()
}

/// Gather before viewport rejection: off-screen silhouettes can block an on-screen source.
#[allow(clippy::too_many_arguments)]
pub fn register(
    f: &mut Frame,
    cell: &Cell,
    anchor: (i32, i32),
    sx: i32,
    sy: i32,
    flip: bool,
    elevation: f32,
    opacity: f32,
) -> u32 {
    if !f.surface_lighting || cell.surface.is_empty() || opacity <= 0.0 {
        return 0;
    }
    let owner = f.surface_casters.len() as u32 + 1;
    f.surface_casters.push(SurfaceCaster {
        pixels: Arc::clone(&cell.surface),
        w: cell.w,
        h: cell.h,
        anchor,
        x: (sx + f.cam_x) as f32,
        y: (sy + f.cam_y) as f32,
        base: base(cell, anchor, sy + f.cam_y, elevation),
        flip,
        owner,
        opacity,
        foot: foot_profile(cell, anchor, sy + f.cam_y, elevation.max(0.0), None),
        min_height: elevation.max(0.0),
        reflectivity: 0.0,
    });
    owner
}
/// Center a receiving point within native pixel cells. Fractional hover offsets
/// can make a one-pixel interval a ULP narrower than1; center/half-width keeps
/// its two bounds equal rather than reversing them through separate +/-0.5.
fn foot_center(base: f32, span: [f32; 2]) -> f32 {
    let center = span[0] + (span[1] - span[0]) * 0.5;
    let half = ((span[1] - span[0] - 1.0) * 0.5).max(0.0);
    base.clamp(center - half, center + half)
}

/// Stamp only the pixels the color renderer actually accepted (pose, flip, reveal and alpha).
#[allow(clippy::too_many_arguments)]
pub fn stamp(
    f: &mut Frame,
    cell: &Cell,
    x: i32,
    y: i32,
    px: i32,
    py: i32,
    owner: u32,
    flip: bool,
    base: f32,
    actor: bool,
) {
    let Some(i) = Frame::idx(px, py) else { return };
    let Some(n) = cell.surface.get((y * cell.w + x) as usize) else {
        return;
    };
    let p = cell.at(x, y);
    let ink = p.c.luma() < 0.105
        && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
            .iter()
            .any(|&(x, y)| !cell.at(x, y).a);
    let caster = owner
        .checked_sub(1)
        .and_then(|id| f.surface_casters.get(id as usize));
    let volume_base = caster
        .and_then(|c| c.foot.get(x as usize))
        .map_or(base, |span| foot_center(base, *span));
    let min_height = caster.map_or(0.0, |c| c.min_height);
    f.surfaces[i] = Surface {
        normal: n.upright(flip),
        height: (volume_base - (py + f.cam_y) as f32 - 0.5).max(min_height),
        owner,
        kind: if p.emit {
            0
        } else if ink {
            4
        } else if actor {
            3
        } else {
            2
        },
    };
}

/// Surface-aware, source-local direct illumination. Ambient/contact shading remains separate.
pub fn apply(f: &mut Frame) {
    apply_inner(None, f);
}

pub fn apply_world(zone: &crate::world::Zone, f: &mut Frame) {
    apply_inner(Some(zone), f);
}

fn apply_inner(zone: Option<&crate::world::Zone>, f: &mut Frame) {
    super::ambient::apply(zone, f);
    // Split daylight into cool fill and a real moving directional key. The same
    // direction drives native alpha/terrain visibility and per-pixel incidence.
    // Emissive pixels and sky never acquire a sunlight proxy.
    if f.normal_sun {
        let direction = super::lighting::sun_direction(f.t);
        let visibility = super::lighting::sun_visibility(zone, f, direction);
        for (i, (s, light)) in f.surfaces.iter().zip(&mut f.light).enumerate() {
            if s.kind == 0 {
                continue;
            }
            let dot = (direction[0] * s.normal[0]
                + direction[1] * s.normal[1]
                + direction[2] * s.normal[2])
                .max(0.0);
            let response = if s.kind == 4 {
                0.02
            } else {
                // Daylight fill carries the exposure; the moving key supplies
                // warm facing and grounded casts without a day/night jump.
                0.02 + 0.44 * dot
            };
            *light +=
                super::Rgb::new(1.0, 0.98, 0.94).scale(response * visibility[i] as f32 / 255.0);
        }
    }
    #[cfg(test)]
    if REFERENCE_POINT_PASS.with(std::cell::Cell::get) {
        apply_points_rectangle(zone, f);
        return;
    }
    apply_points(zone, f);
}

/// Geometry and material eligibility are shared by every source in a frame. Most
/// ground rows contain long, constant-height spans; build them once rather than
/// rediscovering the same empty pixels and height for every overlapping light.
#[derive(Clone, Copy)]
struct HeightRun {
    start: u16,
    end: u16,
    height: f32,
}

struct HeightRuns {
    rows: [usize; H + 1],
    runs: Vec<HeightRun>,
}

impl HeightRuns {
    fn new(f: &Frame) -> Self {
        let mut result = Self {
            rows: [0; H + 1],
            runs: Vec::with_capacity(H * 4),
        };
        for y in 0..H {
            result.rows[y] = result.runs.len();
            let mut x = 0;
            while x < W {
                let i = y * W + x;
                if !Self::eligible(f, i) {
                    x += 1;
                    continue;
                }
                let height = f.surfaces[i].height;
                let start = x;
                x += 1;
                while x < W
                    && f.surfaces[y * W + x].height.to_bits() == height.to_bits()
                    && Self::eligible(f, y * W + x)
                {
                    x += 1;
                }
                result.runs.push(HeightRun {
                    start: start as u16,
                    end: x as u16,
                    height,
                });
            }
        }
        result.rows[H] = result.runs.len();
        result
    }

    #[inline]
    fn eligible(f: &Frame, i: usize) -> bool {
        f.surfaces[i].kind != 0
            && !(f.albedo[i] == super::Rgb::BLACK && f.emissive[i] != super::Rgb::BLACK)
    }
}

/// Evaluate exactly the old left-associated distance expression. In particular,
/// do not combine dy²+dz²: that changes float rounding at light-radius boundaries.
#[inline]
fn distance_squared(lx: f32, x: usize, dy2: f32, dz2: f32) -> f32 {
    let dx = lx - x as f32 - 0.5;
    dx * dx + dy2 + dz2
}

/// Trim a constant-height row using the original falloff predicate, not a rounded
/// square root radius. Binary searches preserve even the last ULP at the sphere
/// edge. Short spans cost less to shade directly.
#[inline]
fn reached_run(
    lx: f32,
    dy2: f32,
    dz2: f32,
    inv_r2: f32,
    start: usize,
    end: usize,
) -> (usize, usize) {
    if end - start < 8
        || !lx.is_finite()
        || !dy2.is_finite()
        || !dz2.is_finite()
        || !inv_r2.is_finite()
        || inv_r2 <= 0.0
    {
        return (start, end);
    }
    let reaches = |x| 1.0 - distance_squared(lx, x, dy2, dz2) * inv_r2 > 0.0;
    let left_center = (lx - 0.5).floor().clamp(start as f32, (end - 1) as f32) as usize;
    let center = if left_center + 1 < end
        && distance_squared(lx, left_center + 1, dy2, dz2)
            < distance_squared(lx, left_center, dy2, dz2)
    {
        left_center + 1
    } else {
        left_center
    };
    if !reaches(center) {
        return (start, start);
    }
    let mut first = start;
    if !reaches(start) {
        let mut last = center;
        while first < last {
            let mid = first + (last - first) / 2;
            if reaches(mid) {
                last = mid;
            } else {
                first = mid + 1;
            }
        }
    }
    let mut last = end;
    if !reaches(end - 1) {
        let mut lo = center + 1;
        while lo < last {
            let mid = lo + (last - lo) / 2;
            if reaches(mid) {
                lo = mid + 1;
            } else {
                last = mid;
            }
        }
    }
    (first, last)
}

fn apply_points(zone: Option<&crate::world::Zone>, f: &mut Frame) {
    // Grouping scans the entire viewport once. A few local emitters cost less
    // to visit directly; enough overlapping sources amortize that shared work.
    // Both paths retain the same source order and exact per-pixel arithmetic.
    if f.point_lights.len() <= 6 {
        apply_points_rectangle(zone, f);
        return;
    }
    let runs = HeightRuns::new(f);
    let soften = zone.is_some_and(|z| z.id != crate::world::ZoneId::Bloomwild);
    for li in 0..f.point_lights.len() {
        let light = f.point_lights[li];
        let visibility = if let Some(zone) = zone {
            super::lighting::point_visibility_world(zone, f, light)
        } else {
            super::lighting::point_visibility(f, light)
        };
        let lx = light.x - f.cam_x as f32;
        let ly = light.y - f.cam_y as f32;
        let r = light.radius;
        let x0 = (lx - r).floor().max(0.0) as usize;
        let x1 = (lx + r).ceil().min(W as f32 - 1.0).max(0.0) as usize;
        let y0 = (ly - r * std::f32::consts::SQRT_2).floor().max(0.0) as usize;
        let y1 = (ly + r * std::f32::consts::SQRT_2)
            .ceil()
            .min(H as f32 - 1.0)
            .max(0.0) as usize;
        let inv_r2 = 1.0 / (r * r);
        for y in y0..=y1 {
            let row_dy = ly + light.height - y as f32 - 0.5;
            for run in &runs.runs[runs.rows[y]..runs.rows[y + 1]] {
                let start = (run.start as usize).max(x0);
                let end = (run.end as usize).min(x1 + 1);
                if start >= end {
                    continue;
                }
                let dy = row_dy - run.height;
                let dz = light.height - run.height;
                let dy2 = dy * dy;
                let dz2 = dz * dz;
                let (start, end) = reached_run(lx, dy2, dz2, inv_r2, start, end);
                for x in start..end {
                    let i = y * W + x;
                    if visibility[i] == 0 {
                        continue;
                    }
                    let s = f.surfaces[i];
                    let dx = lx - x as f32 - 0.5;
                    let d2 = dx * dx + dy2 + dz2;
                    let a = 1.0 - d2 * inv_r2;
                    if a <= 0.0 {
                        continue;
                    }
                    let inv_d = 1.0 / d2.sqrt().max(1.0);
                    let mut response = point_response(s, [dx, dy, dz], inv_d, soften);
                    response += point_reflection(f, s, [dx, dy, dz]);
                    let strength =
                        light.intensity * a * a * response * visibility[i] as f32 / 255.0;
                    f.light[i] += light.color.scale(strength);
                }
            }
        }
    }
}

/// Terrain receives direct light only on its exposed side. In the three dark
/// regions, mix microfacet response with the top plane to keep sword/lava light
/// calm while the independent ambient pass supplies continuous surface detail.
fn point_response(surface: Surface, direction: [f32; 3], inverse_distance: f32, soften: bool) -> f32 {
    let terrain = surface.owner == 0 && matches!(surface.kind, 1 | 2 | 5);
    let top = terrain && surface.kind != 5;
    if top && direction[2] < 0.0 { return 0.0; }
    let mut dot = surface.normal.iter().zip(direction).map(|(n, d)| n * d).sum::<f32>();
    if top && soften {
        dot = direction[2] + 0.55 * (dot - direction[2]);
    }
    let diffuse = (dot * inverse_distance).clamp(0.0, 1.0);
    if terrain {
        1.25 * diffuse
    } else if surface.kind == 4 {
        0.16
    } else {
        0.12 + 1.25 * diffuse
    }
}

/// Broad, quiet Blinn reflection for explicitly opted-in scenery. The fixed
/// orthographic view points south/up (screen_y = Y-Z), so camera scrolling cannot
/// make a static face sparkle. Source motion, normals and visibility supply all
/// variation; the existing light pass tints this with albedo and source color.
fn point_reflection(f: &Frame, surface: Surface, direction: [f32; 3]) -> f32 {
    if surface.kind != 2 || surface.owner == 0 { return 0.0; }
    let Some(caster) = f.surface_casters.get(surface.owner as usize - 1) else { return 0.0 };
    if caster.reflectivity <= 0.0 { return 0.0; }
    // Diffuse clamps distance to one pixel; a reflection still needs a unit
    // direction beside an emissive crest, including subpixel source distances.
    let distance = direction.iter().map(|d| d * d).sum::<f32>().sqrt();
    if distance <= 0.001 { return 0.0; }
    let light = direction.map(|d| d / distance);
    let facing = surface.normal.iter().zip(light).map(|(n, l)| n * l).sum::<f32>().clamp(0.0, 1.0);
    if facing <= 0.0 { return 0.0; }
    let view = std::f32::consts::FRAC_1_SQRT_2;
    let half = [light[0], light[1] + view, light[2] + view];
    let length = half.iter().map(|v| v * v).sum::<f32>().sqrt().max(0.001);
    let dot = (surface.normal.iter().zip(half).map(|(n, h)| n * h).sum::<f32>() / length).clamp(0.0, 1.0);
    caster.reflectivity * facing * dot.powi(16)
}

#[cfg(test)]
thread_local! {
    static REFERENCE_POINT_PASS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Original rectangle scan serves small light sets and remains the independent
/// exact-output oracle for grouping and sphere-edge interval searches in tests.
fn apply_points_rectangle(zone: Option<&crate::world::Zone>, f: &mut Frame) {
    let lights = f.point_lights.clone();
    let soften = zone.is_some_and(|z| z.id != crate::world::ZoneId::Bloomwild);
    for light in lights {
        let visibility = if let Some(zone) = zone {
            super::lighting::point_visibility_world(zone, f, light)
        } else {
            super::lighting::point_visibility(f, light)
        };
        let lx = light.x - f.cam_x as f32;
        let ly = light.y - f.cam_y as f32;
        let r = light.radius;
        let x0 = (lx - r).floor().max(0.0) as usize;
        let x1 = (lx + r).ceil().min(W as f32 - 1.0).max(0.0) as usize;
        let y0 = (ly - r * std::f32::consts::SQRT_2).floor().max(0.0) as usize;
        let y1 = (ly + r * std::f32::consts::SQRT_2)
            .ceil()
            .min(H as f32 - 1.0)
            .max(0.0) as usize;
        let inv_r2 = 1.0 / (r * r);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = y * W + x;
                let s = f.surfaces[i];
                if s.kind == 0
                    || visibility[i] == 0
                    || (f.albedo[i] == super::Rgb::BLACK && f.emissive[i] != super::Rgb::BLACK)
                {
                    continue;
                }
                let dx = lx - x as f32 - 0.5;
                let dy = ly + light.height - y as f32 - 0.5 - s.height;
                let dz = light.height - s.height;
                let d2 = dx * dx + dy * dy + dz * dz;
                let a = 1.0 - d2 * inv_r2;
                if a <= 0.0 {
                    continue;
                }
                let inv_d = 1.0 / d2.sqrt().max(1.0);
                let mut response = point_response(s, [dx, dy, dz], inv_d, soften);
                response += point_reflection(f, s, [dx, dy, dz]);
                let strength = light.intensity * a * a * response * visibility[i] as f32 / 255.0;
                f.light[i] += light.color.scale(strength);
            }
        }
    }
}

#[cfg(test)]
#[path = "surface_reflection_tests.rs"]
mod reflection_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::Rgb;

    #[test]
    fn terrain_point_sources_cannot_light_high_caps_or_rear_faces() {
        use crate::gfx::frame::PointLight;
        let index = 100 * W + 80;
        for count in [1, 7] {
            for (normal, height, kind, source, lit) in [
                ([0.0, 0.0, 1.0], 30.0, 2, [80.5, 100.5, 12.0], false),
                ([0.0, -0.3, 0.9539392], 30.0, 2, [80.5, 100.5, 29.0], false),
                ([0.0, 1.0, 0.0], 15.0, 5, [80.5, 140.5, 12.0], true),
                ([0.0, 1.0, 0.0], 15.0, 5, [80.5, 80.5, 12.0], false),
                ([-0.126, 0.740, 0.661], 17.0, 5, [80.5, 158.5, 12.0], true),
            ] {
                let mut frame = Frame::new();
                frame.begin(Rgb::BLACK, Rgb::BLACK);
                frame.surfaces[index] = Surface { normal, height, owner: 0, kind };
                frame.point_lights.push(PointLight { x: source[0], y: source[1] - source[2],
                    height: source[2], radius: 100.0, color: Rgb::WHITE, intensity: 1.0 });
                for _ in 1..count {
                    frame.point_lights.push(PointLight { x: -1000.0, y: -1000.0, height: 12.0,
                        radius: 2.0, color: Rgb::WHITE, intensity: 1.0 });
                }
                apply_points(None, &mut frame);
                if lit {
                    assert!(frame.light[index].r > 0.1, "a facing vertical cliff still receives low light");
                } else {
                    assert_eq!(frame.light[index], Rgb::BLACK,
                        "{count}-source pass leaked onto cap/rear face: {normal:?} from {source:?}");
                }
            }
        }
    }

    #[test]
    fn calmer_local_relief_preserves_flat_terrain_and_character_light() {
        let direction = [12.0, 10.0, 8.0];
        let inverse = 1.0 / 308.0_f32.sqrt();
        let terrain = |x| Surface { normal: [x, 0.0, (1.0_f32 - x*x).sqrt()],
            height: 0.0, owner: 0, kind: 1 };
        let original = point_response(terrain(0.25), direction, inverse, false)
            - point_response(terrain(-0.25), direction, inverse, false);
        let softened = point_response(terrain(0.25), direction, inverse, true)
            - point_response(terrain(-0.25), direction, inverse, true);
        assert!((softened / original - 0.55).abs() < 0.00001);
        assert_eq!(point_response(terrain(0.0), direction, inverse, false),
            point_response(terrain(0.0), direction, inverse, true));
        let actor = Surface { owner: 1, kind: 3, ..terrain(0.25) };
        assert_eq!(point_response(actor, direction, inverse, false), point_response(actor, direction, inverse, true));
    }
    /// The reference switch is thread-local so these complete scene renders can
    /// run alongside unrelated tests, and is reset even when an assertion panics.
    fn with_reference_points<T>(draw: impl FnOnce() -> T) -> T {
        struct Restore(bool);
        impl Drop for Restore {
            fn drop(&mut self) {
                REFERENCE_POINT_PASS.with(|flag| flag.set(self.0));
            }
        }
        let restore = Restore(REFERENCE_POINT_PASS.with(|flag| flag.replace(true)));
        let result = draw();
        drop(restore);
        result
    }

    fn assert_light_bits(a: &Frame, b: &Frame, label: &str) {
        assert_eq!(a.albedo, b.albedo, "{label}: albedo changed");
        assert_eq!(a.emissive, b.emissive, "{label}: emission changed");
        for (i, (a, b)) in a.light.iter().zip(&b.light).enumerate() {
            assert_eq!(
                [a.r.to_bits(), a.g.to_bits(), a.b.to_bits()],
                [b.r.to_bits(), b.g.to_bits(), b.b.to_bits()],
                "{label}: light at ({},{})",
                i % W,
                i / W,
            );
        }
    }

    #[test]
    fn reached_height_runs_preserve_every_float_boundary() {
        for lx in [
            -91.25, -0.001, 0.5, 17.5, 103.49999, 192.625, 383.5, 492.125,
        ] {
            for dy in [-160.0_f32, -71.001, -0.125, 0.0, 18.25, 93.0] {
                for dz in [-90.0_f32, 0.0, 4.25, 112.0] {
                    for r in [1.0_f32, 34.0, 64.0, 152.0, 212.0] {
                        for r in [
                            f32::from_bits(r.to_bits() - 1),
                            r,
                            f32::from_bits(r.to_bits() + 1),
                        ] {
                            let inv_r2 = 1.0 / (r * r);
                            for (start, end) in [(0, W), (19, 321), (93, 99), (192, 201), (377, W)]
                            {
                                let (lo, hi) =
                                    reached_run(lx, dy * dy, dz * dz, inv_r2, start, end);
                                for x in start..end {
                                    let a =
                                        1.0 - distance_squared(lx, x, dy * dy, dz * dz) * inv_r2;
                                    assert!(
                                        a <= 0.0 || (lo..hi).contains(&x),
                                        "reached pixel omitted: x={x}, range={start}..{end}, lx={lx}, dy={dy}, dz={dz}, r={r}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn point_row_runs_match_reference_light_bits_and_rgba() {
        use crate::gfx::frame::PointLight;
        fn fixture(camera: (i32, i32)) -> Frame {
            let mut f = Frame::new();
            f.begin(Rgb::new(0.31, 0.42, 0.53), Rgb::new(0.19, 0.23, 0.29));
            f.surface_lighting = true;
            (f.cam_x, f.cam_y) = camera;
            for y in 0..H {
                for x in 0..W {
                    let i = y * W + x;
                    let height = match (x / 53 + y / 31) % 5 {
                        0 => 0.0,
                        1 => 12.0,
                        2 => 31.375,
                        3 => ((x * 7 + y * 3) % 19) as f32 * 0.125,
                        _ => (83 - y.min(83)) as f32,
                    };
                    f.surfaces[i] = Surface {
                        normal: if x % 3 == 0 {
                            [-0.6, 0.0, 0.8]
                        } else {
                            [0.0, 0.6, 0.8]
                        },
                        height,
                        owner: 0,
                        kind: ((x / 7 + y / 5) % 5) as u8,
                    };
                    if x % 17 == 0 {
                        f.albedo[i] = Rgb::BLACK;
                        f.emissive[i] = Rgb::new(0.4, 0.7, 1.0);
                    }
                }
            }
            for (x, y, height, radius) in [
                (-30.25, 93.125, 18.0, 64.0),
                (17.5, 0.5, 0.0, 34.0),
                (190.375, 77.625, 112.0, 152.0),
                (383.5, 215.5, 48.125, 80.0),
                (240.0, 120.0, 0.0, 1.0),
                (103.49999, 93.125, 4.25, 64.00001),
                (440.0, 217.0, 80.0, 212.0),
            ] {
                f.point_lights.push(PointLight {
                    x: x + camera.0 as f32,
                    y: y + camera.1 as f32,
                    height,
                    radius,
                    color: Rgb::new(0.31, 0.67, 0.89),
                    intensity: 1.13,
                });
            }
            let cell = Cell::from_ascii(
                &["..xx..", ".xxxx.", "xx..xx", "xx..xx"],
                &[('x', Rgb::WHITE)],
                &[],
            );
            for (x, y, flip, opacity) in [
                (88, 95, false, 1.0),
                (195, 139, true, 0.35),
                (381, 211, false, 0.65),
            ] {
                f.sprite(
                    &cell,
                    (3, 4),
                    x,
                    y,
                    &super::super::sprite::DrawOpts {
                        flip,
                        alpha: opacity,
                        reflectivity: 0.22,
                        ..Default::default()
                    },
                );
            }
            f
        }
        for camera in [(0, 0), (-41, 93), (1307, 761)] {
            // Exercise both paths, especially the 6/7-source routing boundary.
            for count in [0, 1, 6, 7, 8, 12] {
                let mut actual = fixture(camera);
                let mut expected = fixture(camera);
                for frame in [&mut actual, &mut expected] {
                    let original = frame.point_lights.clone();
                    while frame.point_lights.len() < count {
                        frame.point_lights.extend_from_slice(&original);
                    }
                    frame.point_lights.truncate(count);
                }
                apply_inner(None, &mut actual);
                with_reference_points(|| apply_inner(None, &mut expected));
                assert_light_bits(
                    &actual,
                    &expected,
                    &format!("synthetic varied height/mask/caster frame, {count} sources"),
                );
                let grade = super::super::post::Grade::default();
                super::super::post::compose(&mut actual, &grade);
                super::super::post::compose(&mut expected, &grade);
                super::super::post::finish(&mut actual, &grade);
                super::super::post::finish(&mut expected, &grade);
                assert_eq!(
                    actual.out, expected.out,
                    "synthetic final RGBA, camera {camera:?}, {count} sources"
                );
            }
        }
    }

    #[test]
    fn point_row_runs_match_complete_real_region_render_bits_and_rgba() {
        use crate::fx::Fx;
        use crate::input::Input;
        use crate::play::{Play, PlayOpts};
        use crate::world::ZoneId;
        for id in [
            ZoneId::Bloomwild,
            ZoneId::Glasswater,
            ZoneId::Cinder,
            ZoneId::Summit,
        ] {
            let mut play = Play::new(
                id,
                &PlayOpts {
                    god: true,
                    peaceful: false,
                },
            );
            let mut fx = Fx::new(7);
            if let Some((x, y)) = play.zone.boss_pos {
                play.player.body.x = x;
                play.player.body.y = y + 95.0;
                play.cam.snap(x, y + 50.0);
            }
            for target in [1, 240] {
                while play.t < target {
                    play.update(&Input::default(), &mut fx);
                    fx.sounds.clear();
                    fx.lights.clear();
                }
                let mut actual = Frame::new();
                let mut expected = Frame::new();
                play.render(&mut actual, &fx);
                with_reference_points(|| play.render(&mut expected, &fx));
                assert!(actual.surface_lighting && !actual.point_lights.is_empty());
                assert_light_bits(&actual, &expected, &format!("{id:?}, tick {target}"));
                super::super::post::finish(&mut actual, &play.zone.grade);
                super::super::post::finish(&mut expected, &play.zone.grade);
                assert_eq!(
                    actual.out, expected.out,
                    "{id:?}, tick {target}: final world RGBA"
                );
            }
        }
    }

    /// Paired, alternating-order comparison within one process. This deliberately
    /// measures the complete live render rather than comparing different builds
    /// at different CPU clocks. Each A/B pair has identical gameplay, camera,
    /// particles, sources and native geometry; only the point-shading loop differs.
    #[test]
    #[ignore = "explicit isolated release-mode paired point-shading timing"]
    fn paired_full_play_point_shading_timing() {
        use crate::fx::Fx;
        use crate::input::Input;
        use crate::play::{Play, PlayOpts};
        use crate::world::ZoneId;
        use std::hint::black_box;
        use std::time::Instant;
        fn render(play: &mut Play, fx: &Fx, frame: &mut Frame, hud: &crate::ui::Hud) -> f64 {
            let start = Instant::now();
            play.render(frame, fx);
            crate::ui::hud(frame, play, hud);
            super::super::post::finish(frame, &play.zone.grade);
            black_box(&frame.out);
            start.elapsed().as_secs_f64() * 1000.0
        }
        fn summary(mut samples: Vec<f64>) -> serde_json::Value {
            samples.sort_by(f64::total_cmp);
            serde_json::json!({
                "samples": samples.len(),
                "mean_ms": samples.iter().sum::<f64>() / samples.len() as f64,
                "median_ms": samples[samples.len() / 2],
                "p95_ms": samples[samples.len() * 95 / 100],
                "max_ms": samples[samples.len() - 1],
            })
        }
        let mut results = Vec::new();
        for (name, id, position) in [
            ("bloom_start", ZoneId::Bloomwild, None),
            ("bloom_courtyard", ZoneId::Bloomwild, Some((1056.0, 464.0))),
            ("glass_gallery", ZoneId::Glasswater, Some((672.0, 588.0))),
            ("cinder_foundry", ZoneId::Cinder, Some((626.0, 449.0))),
            ("summit_arena", ZoneId::Summit, None),
        ] {
            let mut play = Play::new(
                id,
                &PlayOpts {
                    god: true,
                    peaceful: false,
                },
            );
            let position = if id == ZoneId::Summit {
                play.zone.boss_pos.map(|(x, y)| (x, y + 72.0))
            } else {
                position
            };
            if let Some((x, y)) = position {
                play.player.respawn(x, y);
                play.cam.snap(x, y);
            }
            let mut fx = Fx::new(7);
            let input = Input::default();
            let hud = crate::ui::Hud::new();
            let mut actual = Frame::new();
            let mut reference = Frame::new();
            // Advance actual scene life and warm both independent frame buffers,
            // shared art caches, visibility cache probation and post-processing.
            for tick in 0..90 {
                play.update(&input, &mut fx);
                if tick >= 80 {
                    if tick % 2 == 0 {
                        render(&mut play, &fx, &mut actual, &hud);
                        with_reference_points(|| render(&mut play, &fx, &mut reference, &hud));
                    } else {
                        with_reference_points(|| render(&mut play, &fx, &mut reference, &hud));
                        render(&mut play, &fx, &mut actual, &hud);
                    }
                }
                fx.sounds.clear();
                fx.lights.clear();
                fx.tick();
            }
            let mut optimized_times = Vec::with_capacity(60);
            let mut reference_times = Vec::with_capacity(60);
            let mut batches = Vec::new();
            let (mut min_points, mut max_points, mut max_runs) = (usize::MAX, 0, 0);
            for batch in 0..6 {
                let mut batch_actual = Vec::with_capacity(10);
                let mut batch_reference = Vec::with_capacity(10);
                for sample in 0..10 {
                    play.update(&input, &mut fx);
                    let (a, b) = if (batch * 10 + sample) % 2 == 0 {
                        let a = render(&mut play, &fx, &mut actual, &hud);
                        let b =
                            with_reference_points(|| render(&mut play, &fx, &mut reference, &hud));
                        (a, b)
                    } else {
                        let b =
                            with_reference_points(|| render(&mut play, &fx, &mut reference, &hud));
                        let a = render(&mut play, &fx, &mut actual, &hud);
                        (a, b)
                    };
                    // Outside the timed regions: exact output, not a perceptual
                    // approximation, is required of either performance choice.
                    assert_light_bits(&actual, &reference, name);
                    assert_eq!(actual.out, reference.out, "paired final RGBA: {name}");
                    min_points = min_points.min(actual.point_lights.len());
                    max_points = max_points.max(actual.point_lights.len());
                    max_runs = max_runs.max(HeightRuns::new(&actual).runs.len());
                    optimized_times.push(a);
                    reference_times.push(b);
                    batch_actual.push(a);
                    batch_reference.push(b);
                    fx.sounds.clear();
                    fx.lights.clear();
                    fx.tick();
                }
                batches.push(serde_json::json!({"batch": batch,
                    "adaptive": summary(batch_actual), "original_rectangle": summary(batch_reference)}));
            }
            results.push(serde_json::json!({"case": name,
                "min_points": min_points, "max_points": max_points, "max_height_runs": max_runs,
                "adaptive": summary(optimized_times), "original_rectangle": summary(reference_times),
                "batches": batches}));
        }
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
    }

    #[test]
    fn conventions_and_mirror() {
        let n = NormalPixel::decode([218, 180, 199], true, false);
        let a = n.upright(false);
        let b = n.upright(true);
        assert!(a[0] > 0.0 && a[1] > 0.0 && a[2] > 0.0);
        assert_eq!(a[0], -b[0]);
        assert_eq!(a[1..], b[1..]);
        assert_eq!(n.floor(), [a[0], -a[2], a[1]]);
    }
    #[test]
    fn lighting_exchanges_faces_without_changing_albedo_or_emission() {
        let mut f = Frame::new();
        f.begin(Rgb::gray(0.5), Rgb::gray(0.2));
        f.surface_lighting = true;
        for (x, n) in [(100, [-0.8, 0.0, 0.6]), (102, [0.8, 0.0, 0.6])] {
            f.surfaces[100 * W + x] = Surface {
                normal: n,
                height: 12.0,
                owner: 1,
                kind: 2,
            };
        }
        let albedo = f.albedo.clone();
        let emit = f.emissive.clone();
        f.add_light_3d_w(70.0, 112.5, 12.0, 100.0, Rgb::WHITE, 1.0);
        apply(&mut f);
        assert!(f.light[100 * W + 100].r > f.light[100 * W + 102].r + 0.3);
        f.light.fill(Rgb::gray(0.2));
        f.point_lights.clear();
        f.add_light_3d_w(132.0, 112.5, 12.0, 100.0, Rgb::WHITE, 1.0);
        apply(&mut f);
        assert!(f.light[100 * W + 102].r > f.light[100 * W + 100].r + 0.3);
        assert_eq!(albedo, f.albedo);
        assert_eq!(emit, f.emissive);
    }
    #[test]
    fn actor_ground_plane_does_not_follow_weapon_or_tail_pixels() {
        let mut a = Cell::from_ascii(&["xx..", "xx..", "....", "...x"], &[('x', Rgb::WHITE)], &[]);
        let mut f = Frame::new();
        f.surface_lighting = true;
        let opts = super::super::sprite::DrawOpts {
            actor: true,
            elevation: 7.0,
            ..Default::default()
        };
        f.sprite(&a, (1, 2), 100, 90, &opts);
        let ground = f.surface_casters[0].base;
        a.px[15] = Px::default();
        a.refresh_surface_mask();
        f.sprite(&a, (1, 2), 100, 90, &opts);
        assert_eq!(ground, 97.0);
        assert_eq!(f.surface_casters[1].base, 97.0);
    }
    #[test]
    fn visible_owner_tracks_alpha_flip_foreground_and_glow() {
        let c = Cell::from_ascii(&["x.x", "xxx"], &[('x', Rgb::WHITE)], &[]);
        let mut f = Frame::new();
        f.surface_lighting = true;
        f.sprite(
            &c,
            (0, 1),
            10,
            20,
            &super::super::sprite::DrawOpts::default(),
        );
        let first = f.surfaces[19 * W + 10].owner;
        f.sprite(
            &c,
            (0, 1),
            10,
            20,
            &super::super::sprite::DrawOpts {
                flip: true,
                ..Default::default()
            },
        );
        assert_ne!(first, f.surfaces[19 * W + 10].owner);
        assert_eq!(f.surfaces[19 * W + 11].kind, 0);
        f.put_glow(10, 19, Rgb::WHITE);
        assert_eq!(f.surfaces[19 * W + 10].kind, 0);
        f.put(10, 20, Rgb::BLACK);
        assert_eq!(f.surfaces[20 * W + 10].kind, 0);
    }
    #[test]
    fn explicit_canopy_base_preserves_height_above_its_shared_root() {
        let c = Cell::from_ascii(&["xx", "xx"], &[('x', Rgb::WHITE)], &[]);
        let mut f = Frame::new();
        f.surface_lighting = true;
        f.sprite(
            &c,
            (0, 10),
            50,
            90,
            &super::super::sprite::DrawOpts {
                ground_y: Some(90.0),
                ..Default::default()
            },
        );
        assert_eq!(f.surface_casters[0].base, 90.0);
        assert!(f.surfaces[80 * W + 50].height >= 8.0);
    }
    #[test]
    fn sole_profile_ignores_low_emissive_weapon_and_follows_native_foot_shape() {
        let c = Cell::from_ascii(
            &["..xx...", "..xx...", ".xxxx..", "..xx...", "......s"],
            &[('x', Rgb::WHITE), ('s', Rgb::WHITE)],
            &['s'],
        );
        let f = foot_profile(&c, (3, 3), 100, 0.0, Some(2.0));
        assert!(f.iter().all(|s| s[0].is_finite() && s[1] > s[0]));
        assert!(f[6][1] <= 101.0);
        assert_eq!(f[3][1], 101.0);
        assert!(f[1][1] < f[2][1]);
    }
    #[test]
    fn one_pixel_feet_remain_valid_at_fractional_hover_heights() {
        for lo in [127.21363_f32, 960.793, 0.001, 2000.6666] {
            let span = [lo, lo + 1.0];
            for base in [lo - 100.0, lo + 0.5, lo + 100.0] {
                let p = foot_center(base, span);
                assert!(p.is_finite() && p >= span[0] && p <= span[1]);
            }
        }
    }
    #[test]
    fn bright_night_ambient_cannot_create_a_directional_sun() {
        let mut f = Frame::new();
        f.begin(Rgb::gray(0.5), Rgb::gray(0.91));
        f.surface_lighting = true;
        let i = 100 * W + 100;
        f.surfaces[i] = Surface {
            normal: [-0.8, 0.0, 0.6],
            height: 20.0,
            owner: 1,
            kind: 2,
        };
        let light = f.light[i];
        apply(&mut f);
        assert_eq!(f.light[i], light);
        f.normal_sun = true;
        apply(&mut f);
        assert!(f.light[i].r > light.r);
    }

    #[test]
    fn shaded_ground_keeps_material_relief_without_a_false_direct_sun() {
        use crate::world::{NoRegion, TerrainVolume, Zone, ZoneId};
        let mut zone = Zone::new(ZoneId::Bloomwild, 32, 20);
        zone.ground.fill(Rgb::WHITE);
        zone.terrain_volumes.push(TerrainVolume {
            min: [60.0, 80.0, 0.0], max: [100.0, 110.0, 40.0],
        });
        super::super::lighting::prepare(&zone, &NoRegion);
        let mut frame = Frame::new();
        frame.begin(Rgb::BLACK, Rgb::gray(0.5));
        frame.surface_lighting = true;
        let indices = [114 * W + 110, 114 * W + 112, 114 * W + 114];
        for (index, normal) in indices.into_iter().zip([
            [-0.25, 0.0, 0.9375_f32.sqrt()],
            [0.25, 0.0, 0.9375_f32.sqrt()],
            [0.0, 0.0, 1.0],
        ]) {
            frame.surfaces[index] = Surface { normal, height: 0.0, owner: 0, kind: 1 };
        }
        apply_world(&zone, &mut frame);
        assert!(indices.into_iter().all(|i| frame.light[i] == Rgb::gray(0.5)),
            "night fill must not acquire an invented daylight direction");
        frame.normal_sun = true;
        let direction = super::super::lighting::sun_direction(frame.t);
        let visibility = super::super::lighting::sun_visibility(Some(&zone), &frame, direction);
        assert!(indices.into_iter().all(|i| visibility[i] == 0), "fixture must be in actual geometric shade");
        apply_world(&zone, &mut frame);
        assert!((frame.light[indices[0]].r - frame.light[indices[1]].r) > 0.04,
            "opposing turf facets must remain distinguishable under skylight in full cast shade");
        assert_eq!(frame.light[indices[2]], Rgb::gray(0.5), "flat ground retains its previous fill exposure");
    }
}
