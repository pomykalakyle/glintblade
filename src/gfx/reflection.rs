//! One-bounce, local scene imagery in opted-in crystal faces. Rays use the same
//! X/Y/Z geometry as lighting. Native scene art survives direct-view occlusion;
//! the lit scene supplies environment colors. Reflection never emits a light.
use super::{DrawOpts, Frame, Grade, H, Rgb, W, sprite::{Cell, Px}};
use crate::util::bayer4;

const RANGE: f32 = 96.0;
const BIAS: f32 = 1.0;
const EPS: f32 = 0.0001;

pub(crate) struct Source {
    owner: u32,
    pixels: Vec<Px>,
    alpha: f32,
    actor: bool,
    origin: [f32; 2],
    depth: [f32; 2],
}

pub(crate) fn capture_source(f: &mut Frame, cell: &Cell, owner: u32, o: &DrawOpts) {
    let pixels = cell.px.iter().enumerate().map(|(i, p)| {
        let mut p = *p;
        if let Some((tint, amount)) = o.tint { p.c = p.c.lerp(tint, amount); }
        p.c = cell.flash_color(i as i32 % cell.w, i as i32 / cell.w, p.c, o.flash);
        if p.emit { p.c = p.c.scale(o.emit_gain); }
        p
    }).collect();
    let c = &f.surface_casters[owner as usize - 1];
    let origin = [c.x.floor() - (if c.flip { c.w - 1 - c.anchor.0 } else { c.anchor.0 }) as f32,
        c.y.floor() - c.anchor.1 as f32];
    let depth = [c.foot.iter().map(|s| s[0]).fold(f32::INFINITY, f32::min),
        c.foot.iter().map(|s| s[1]).fold(f32::NEG_INFINITY, f32::max)];
    f.reflection_sources.push(Source { owner, pixels, alpha: o.alpha, actor: o.actor, origin, depth });
}

#[derive(Clone, Copy)]
enum Donor { Scene(usize), Native(usize, usize, [f32; 3]) }
#[derive(Clone, Copy)]
struct Hit { distance: f32, donor: Donor }

fn clip(origin: f32, delta: f32, low: f32, high: f32, interval: &mut (f32, f32)) -> bool {
    if delta.abs() < EPS { return origin >= low && origin < high; }
    let a = (low - origin) / delta;
    let b = (high - origin) / delta;
    interval.0 = interval.0.max(a.min(b));
    interval.1 = interval.1.min(a.max(b));
    interval.0 < interval.1
}

/// Exact projected native-cell traversal. Each visitor clips the ray's interval
/// against that cell's actual depth/height; there is no distance-step tolerance.
fn walk(origin: [f32; 2], delta: [f32; 2], bounds: [f32; 4], mut interval: (f32, f32),
    mut visit: impl FnMut(i32, i32, (f32, f32)) -> Option<Hit>) -> Option<Hit>
{
    if !clip(origin[0], delta[0], bounds[0], bounds[2], &mut interval)
        || !clip(origin[1], delta[1], bounds[1], bounds[3], &mut interval) { return None; }
    let start = (interval.0 + EPS).min((interval.0 + interval.1) * 0.5);
    let mut cell = [0; 2];
    let mut step = [0; 2];
    let mut next = [f32::INFINITY; 2];
    let mut dt = [f32::INFINITY; 2];
    for a in 0..2 {
        cell[a] = (origin[a] + delta[a] * start).floor() as i32;
        if delta[a].abs() >= EPS {
            step[a] = if delta[a] > 0.0 { 1 } else { -1 };
            next[a] = ((cell[a] + i32::from(step[a] > 0)) as f32 - origin[a]) / delta[a];
            dt[a] = delta[a].abs().recip();
        }
    }
    let mut t = interval.0;
    // A 96-world-pixel ray crosses fewer than 170 native projected cells.
    for _ in 0..256 {
        let end = next[0].min(next[1]).min(interval.1);
        if end > t && let Some(hit) = visit(cell[0], cell[1], (t, end)) { return Some(hit); }
        if end >= interval.1 { return None; }
        for a in 0..2 { if next[a] <= end + EPS { cell[a] += step[a]; next[a] += dt[a]; } }
        t = end;
    }
    None
}

fn scene_hit(f: &Frame, p: [f32; 3], ray: [f32; 3], owner: u32, end: f32) -> Option<Hit> {
    walk([p[0], p[1] - p[2]], [ray[0], ray[1] - ray[2]],
        [f.cam_x as f32, f.cam_y as f32, f.cam_x as f32 + W as f32, f.cam_y as f32 + H as f32],
        (BIAS, end), |x, y, interval| {
            // Clipped boundary arithmetic can land one ULP outside the view.
            // A missing scene donor is transparent, never an unsigned index.
            let i = Frame::idx(x - f.cam_x, y - f.cam_y)?;
            let s = f.surfaces[i];
            if s.kind == 0 || s.owner == owner { return None; }
            let (axis, plane) = if s.owner != 0 || s.kind == 5 {
                (1, y as f32 + 0.5 + s.height)
            } else { (2, s.height) };
            if ray[axis].abs() < EPS { return None; }
            let distance = (plane - p[axis]) / ray[axis];
            (distance >= interval.0 && distance < interval.1 && p[2] + distance * ray[2] >= 0.0)
                .then_some(Hit { distance, donor: Donor::Scene(i) })
        })
}

fn native_hit(f: &Frame, source_id: usize, p: [f32; 3], ray: [f32; 3], end: f32) -> Option<Hit> {
    let native = &f.reflection_sources[source_id];
    let c = &f.surface_casters[native.owner as usize - 1];
    if c.foot.len() != c.w as usize { return None; }
    let [ox, oy] = native.origin;
    let mut interval = (BIAS, end);
    let [low, high] = native.depth;
    if !clip(p[1], ray[1], low, high, &mut interval)
        || !clip(p[2], ray[2], c.min_height, (high - oy).max(c.min_height + EPS), &mut interval) { return None; }
    walk([p[0] - ox, p[1] - p[2] - oy], [ray[0], ray[1] - ray[2]],
        [0.0, 0.0, c.w as f32, c.h as f32], interval, |display_x, y, mut range| {
            if display_x < 0 || display_x >= c.w || y < 0 || y >= c.h { return None; }
            let x = if c.flip { c.w - 1 - display_x } else { display_x };
            let index = (y * c.w + x) as usize;
            if !native.pixels[index].a || native.alpha <= bayer4((ox as i32) + display_x, (oy as i32) + y) { return None; }
            let span = c.foot[x as usize];
            if !clip(p[1], ray[1], span[0], span[1], &mut range) { return None; }
            let point = std::array::from_fn(|a| p[a] + ray[a] * range.0);
            Some(Hit { distance: range.0, donor: Donor::Native(source_id, index, point) })
        })
}

fn source_color(zone: Option<&crate::world::Zone>, f: &Frame, donor: Donor, g: &Grade) -> Rgb {
    let Donor::Native(id, pixel, point) = donor else {
        if let Donor::Scene(i) = donor { return f.hdr[i]; }
        unreachable!()
    };
    let native = &f.reflection_sources[id];
    let c = &f.surface_casters[native.owner as usize - 1];
    let material = native.pixels[pixel];
    let sx = point[0].floor() as i32 - f.cam_x;
    let sy = (point[1] - point[2]).floor() as i32 - f.cam_y;
    if let Some(i) = Frame::idx(sx, sy)
        && (f.surfaces[i].owner == native.owner
            || material.emit && f.actor_pixels[i] && f.emissive[i] == material.c)
    { return f.hdr[i]; }
    // Hidden/off-screen art uses the cave's fill and real source visibility.
    // Only characters get the actor readability floor.
    if material.emit { return material.c.scale(g.exposure); }
    let mut light = zone.map_or_else(|| if native.actor { Rgb::new(0.42, 0.46, 0.57) } else { f.ambient },
        |z| super::lighting::reflection_fill(z, point, native.actor));
    let normal = c.pixels[pixel].upright(c.flip);
    let lights = if f.reflection_lights.is_empty() { &f.point_lights } else { &f.reflection_lights };
    for source in lights {
        let d = [source.x - point[0], source.y + source.height - point[1], source.height - point[2]];
        let d2 = d.iter().map(|v| v * v).sum::<f32>();
        let a = (1.0 - d2 / (source.radius * source.radius)).max(0.0);
        if a == 0.0 { continue; }
        let length = d2.sqrt();
        let direction = d.map(|v| v / length.max(0.001));
        if f.reflection_sources.iter().enumerate().any(|(id, other)| other.owner != native.owner
            && native_hit(f, id, point, direction, length - 0.01).is_some()) { continue; }
        let target = [source.x, source.y + source.height, source.height];
        if zone.is_some_and(|z| super::lighting::reflection_blocked(z, point, target)) { continue; }
        let diffuse = (normal.iter().zip(d).map(|(n, v)| n * v).sum::<f32>() / d2.sqrt().max(1.0)).clamp(0.0, 1.0);
        light += source.color.scale(source.intensity * a * a * (0.12 + 1.25 * diffuse));
    }
    (material.c * light).scale(g.exposure)
}

fn reflected_ray(normal: [f32; 3]) -> Option<[f32; 3]> {
    let length = normal.iter().map(|v| v * v).sum::<f32>().sqrt();
    if length < EPS { return None; }
    let n = normal.map(|v| v / length);
    let v = std::f32::consts::FRAC_1_SQRT_2;
    let dot = (n[1] + n[2]) * v;
    if dot <= 0.0 { return None; }
    Some([2.0 * dot * n[0], 2.0 * dot * n[1] - v, 2.0 * dot * n[2] - v])
}

pub(crate) fn apply(zone: Option<&crate::world::Zone>, f: &mut Frame, g: &Grade) {
    if !f.surface_lighting || !f.surface_casters.iter().any(|c| c.reflectivity > 0.0) { return; }
    // Native audit switch compares the exact same live scene with/without this
    // pass. Browser gameplay always includes scene reflections.
    #[cfg(not(target_arch = "wasm32"))]
    {
        static DISABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *DISABLED.get_or_init(|| std::env::var("GLINT_CRYSTAL_REFLECTIONS").is_ok_and(|v| v == "off")) { return; }
    }
    let mut result = Vec::new();
    for (i, s) in f.surfaces.iter().enumerate() {
        if s.kind != 2 || s.owner == 0 { continue; }
        let Some(c) = f.surface_casters.get(s.owner as usize - 1) else { continue; };
        if c.reflectivity <= 0.0 { continue; }
        let Some(ray) = reflected_ray(s.normal) else { continue; };
        let p = [(i % W) as f32 + f.cam_x as f32 + 0.5,
            (i / W) as f32 + f.cam_y as f32 + 0.5 + s.height, s.height];
        let mut closest = None;
        for actor in 0..f.reflection_sources.len() {
            if f.reflection_sources[actor].owner == s.owner { continue; }
            if let Some(hit) = native_hit(f, actor, p, ray, closest.map_or(RANGE, |h: Hit| h.distance)) { closest = Some(hit); }
        }
        if let Some(hit) = scene_hit(f, p, ray, s.owner, closest.map_or(RANGE, |h| h.distance)) { closest = Some(hit); }
        let Some(hit) = closest else { continue; };
        let target = std::array::from_fn(|a| p[a] + ray[a] * hit.distance);
        if zone.is_some_and(|z| super::lighting::reflection_blocked(z, p, target)) { continue; }
        // Close characters retain enough contrast to read as an image. Beyond
        // arm's reach the same restrained material response fades to matte.
        let fade = (1.0 - (hit.distance - 16.0) / (RANGE - 16.0)).clamp(0.0, 1.0);
        let amount = c.reflectivity.min(0.25) * fade;
        let albedo = f.albedo[i];
        let tint = Rgb::WHITE.lerp(albedo.scale(albedo.max_c().max(0.001).recip()), 0.35);
        let color = f.hdr[i].lerp(source_color(zone, f, hit.donor, g) * tint, amount);
        result.push((i, color));
    }
    // Read all donors before writing any reflection: no self/recursive images,
    // stale previous-frame donors, source emission changes or new light energy.
    for (i, color) in result {
        f.hdr[i] = color;
        f.out[i] = super::post::reflected_pixel(color, g, f.look.hot_white);
    }
}

#[cfg(test)]
#[path = "reflection_tests.rs"]
mod tests;
