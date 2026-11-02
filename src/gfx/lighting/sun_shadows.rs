//! Parallel sunlight through current native sprite volumes.
//!
//! Direction is receiver -> sun in world X/right, Y/down, Z/up. Screen Y=Y-Z.
//! The alpha sheet is extruded only through its actual per-column foot interval;
//! neither a collision box nor an arbitrary maximum cast length replaces it.
//! Returned visibility belongs to sunlight alone, leaving fill and emitters intact.

use crate::gfx::frame::SurfaceCaster;
use crate::gfx::{Frame, H, W};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

const EPS: f64 = 0.000_01;

/// A slow, slight sun arc. One cycle is 36 seconds at the game's 60 Hz clock.
/// Shading and shadow callers must use this same vector for the same frame.
pub(super) fn direction(t: u64) -> [f32; 3] {
    let phase = (t % 2160) as f64 * std::f64::consts::TAU / 2160.0;
    let azimuth = (-0.32_f64).atan2(-0.56) + phase.sin() * 0.20;
    let elevation = 0.76_f64.atan2(0.56_f64.hypot(0.32)) + (phase * 2.0).sin() * 0.045;
    let horizontal = elevation.cos();
    [
        (horizontal * azimuth.cos()) as f32,
        (horizontal * azimuth.sin()) as f32,
        elevation.sin() as f32,
    ]
}

fn clip(origin: f64, delta: f64, lo: f64, hi: f64, interval: &mut [f64; 2]) -> bool {
    if delta.abs() < 1.0e-12 {
        return origin >= lo && origin <= hi && interval[0] < interval[1];
    }
    let a = (lo - origin) / delta;
    let b = (hi - origin) / delta;
    interval[0] = interval[0].max(a.min(b));
    interval[1] = interval[1].min(a.max(b));
    interval[0] < interval[1]
}

fn crossing(cell: i32, origin: f64, delta: f64, step: i32) -> f64 {
    if step == 0 { f64::INFINITY }
    else { ((cell + i32::from(step > 0)) as f64 - origin) / delta }
}

/// Choose the cell entered after t, including exact negative-direction boundaries.
fn cell_after(origin: f64, delta: f64, step: i32, t: f64) -> i32 {
    let mut cell = (origin + delta * t).floor() as i32;
    if step == 0 { return cell; }
    while crossing(cell, origin, delta, step) <= t { cell += step; }
    while crossing(cell - step, origin, delta, step) > t { cell -= step; }
    cell
}

struct Native<'a> {
    source: &'a SurfaceCaster,
    x0: f64,
    y0: f64,
    lo: [f64; 3],
    hi: [f64; 3],
    image_y: [f64; 2],
    volume: bool,
    visible: u8,
    alpha: Option<Arc<AlphaBounds>>,
}

struct AlphaBounds {
    pixels: Arc<[crate::gfx::surface::NormalPixel]>,
    w: i32,
    h: i32,
    bounds: [i32; 4],
    block_columns: usize,
    occupied: Vec<u8>,
}

thread_local! {
    /// Retain the immutable Arc with its key, so allocator pointer reuse can
    /// never confuse a later animation cell with an earlier alpha silhouette.
    static ALPHA_BOUNDS: RefCell<VecDeque<Arc<AlphaBounds>>> = RefCell::new(VecDeque::new());
}

fn alpha_bounds(source: &SurfaceCaster) -> Arc<AlphaBounds> {
    ALPHA_BOUNDS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(i) = cache.iter().position(|entry| entry.w == source.w && entry.h == source.h
            && Arc::ptr_eq(&entry.pixels, &source.pixels)) {
            let entry = cache.remove(i).unwrap();
            cache.push_back(entry.clone());
            return entry;
        }
        let mut bounds = [source.w, -1, source.h, -1];
        let block_columns = ((source.w + BLOCK - 1) / BLOCK) as usize;
        let block_rows = ((source.h + BLOCK - 1) / BLOCK) as usize;
        let mut occupied = vec![0; block_columns * block_rows];
        for y in 0..source.h { for x in 0..source.w {
            if source.pixels[(y * source.w + x) as usize].a {
                bounds[0] = bounds[0].min(x); bounds[1] = bounds[1].max(x);
                bounds[2] = bounds[2].min(y); bounds[3] = bounds[3].max(y);
                occupied[(y / BLOCK) as usize * block_columns + (x / BLOCK) as usize] = 1;
            }
        } }
        let entry = Arc::new(AlphaBounds { pixels: source.pixels.clone(), w: source.w, h: source.h,
            bounds, block_columns, occupied });
        cache.push_back(entry.clone());
        if cache.len() > 128 { cache.pop_front(); }
        entry
    })
}

/// Reject distant atlas cells before scanning their alpha. Padding only expands
/// this bound; the exact Native constructed afterward determines every hit.
fn cell_reaches_frame(source: &SurfaceCaster, direction: [f64; 3], max_height: f64, f: &Frame) -> bool {
    if source.w <= 0 || source.h <= 0 || source.opacity <= 0.0 { return false; }
    let x0 = f64::from(source.x.floor()) - f64::from(if source.flip {
        source.w - 1 - source.anchor.0
    } else { source.anchor.0 });
    let y0 = f64::from(source.y.floor()) - f64::from(source.anchor.1);
    let volume = source.foot.len() == source.w as usize
        && source.foot.iter().all(|s| s[0].is_finite() && s[1].is_finite() && s[0] < s[1]);
    let (y_lo, y_hi) = if volume {
        source.foot.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), s| {
            (lo.min(f64::from(s[0])), hi.max(f64::from(s[1])))
        })
    } else { (f64::from(source.base) - 0.5, f64::from(source.base) + 0.5) };
    let padded = Native {
        source, x0, y0,
        lo: [x0, y_lo, if volume { f64::from(source.min_height) }
            else { f64::from(source.base) - y0 - f64::from(source.h) }],
        hi: [x0 + f64::from(source.w), y_hi, y_hi - y0],
        image_y: [y0, y0 + f64::from(source.h)], volume, visible: 0, alpha: None,
    };
    padded.shadow_bounds(direction, [0.0, max_height], f).is_some()
}

impl<'a> Native<'a> {
    fn new(source: &'a SurfaceCaster) -> Option<Self> {
        if source.w <= 0 || source.h <= 0 || source.opacity <= 0.0
            || source.pixels.len() != (source.w * source.h) as usize {
            return None;
        }
        let alpha = alpha_bounds(source);
        let [mut left, mut right, top, bottom] = alpha.bounds;
        if right < left { return None; }
        if source.flip { (left, right) = (source.w - 1 - right, source.w - 1 - left); }
        let x0 = f64::from(source.x.floor()) - f64::from(if source.flip {
            source.w - 1 - source.anchor.0
        } else { source.anchor.0 });
        let y0 = f64::from(source.y.floor()) - f64::from(source.anchor.1);
        let volume = source.foot.len() == source.w as usize
            && source.foot.iter().all(|s| s[0].is_finite() && s[1].is_finite() && s[0] < s[1]);
        let (y_lo, y_hi) = if volume {
            source.foot.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), s| {
                (lo.min(f64::from(s[0])), hi.max(f64::from(s[1])))
            })
        } else { (f64::from(source.base) - 0.5, f64::from(source.base) + 0.5) };
        let z_lo = if volume { f64::from(source.min_height) }
            else { f64::from(source.base) - y0 - f64::from(bottom + 1) };
        let z_hi = y_hi - y0 - f64::from(top);
        if z_lo > z_hi { return None; }
        Some(Self {
            source, x0, y0,
            lo: [x0 + f64::from(left), y_lo, z_lo],
            hi: [x0 + f64::from(right + 1), y_hi, z_hi],
            image_y: [y0 + f64::from(top), y0 + f64::from(bottom + 1)],
            volume,
            visible: ((1.0 - source.opacity.clamp(0.0, 1.0)) * 255.0).round() as u8,
            alpha: Some(alpha),
        })
    }

    #[cfg(test)]
    fn source_x(&self, x: i32) -> i32 {
        if self.source.flip { self.source.w - 1 - x } else { x }
    }

    /// Conservative receiving screen rectangle, including raised receivers.
    /// The cast reach follows real caster height divided by the sun's Z slope.
    fn shadow_bounds(&self, direction: [f64; 3], receiver_height: [f64; 2], f: &Frame) -> Option<[i32; 4]> {
        let slope = [direction[0] / direction[2], direction[1] / direction[2]];
        let mut bounds = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for x in [self.lo[0], self.hi[0]] {
            for y in [self.lo[1], self.hi[1]] {
                for z in [self.lo[2], self.hi[2]] {
                    for rz in receiver_height {
                        let px = x - slope[0] * (z - rz) - f64::from(f.cam_x);
                        let py = y - slope[1] * (z - rz) - rz - f64::from(f.cam_y);
                        bounds[0] = bounds[0].min(px); bounds[2] = bounds[2].max(px);
                        bounds[1] = bounds[1].min(py); bounds[3] = bounds[3].max(py);
                    }
                }
            }
        }
        let rect = [
            (bounds[0].floor() as i32).max(0), (bounds[1].floor() as i32).max(0),
            (bounds[2].ceil() as i32).min(W as i32 - 1),
            (bounds[3].ceil() as i32).min(H as i32 - 1),
        ];
        (rect[0] <= rect[2] && rect[1] <= rect[3]).then_some(rect)
    }

    #[cfg(test)]
    fn blocks_reference(&self, receiver: [f64; 3], direction: [f64; 3]) -> bool {
        let mut interval = [EPS, f64::INFINITY];
        for axis in 0..3 {
            if !clip(receiver[axis], direction[axis], self.lo[axis], self.hi[axis], &mut interval) {
                return false;
            }
        }
        if self.volume && !clip(receiver[1] - receiver[2], direction[1] - direction[2],
            self.image_y[0], self.image_y[1], &mut interval) {
            return false;
        }
        self.blocks_in_interval_reference(receiver, direction, interval)
    }

    #[cfg(test)]
    fn blocks_ground_reference(&self, receiver: [f64; 3], direction: [f64; 3], mut interval: [f64; 2]) -> bool {
        if !clip(receiver[0], direction[0], self.lo[0], self.hi[0], &mut interval) {
            return false;
        }
        self.blocks_in_interval_reference(receiver, direction, interval)
    }

    fn blocks(&self, receiver: [f64; 3], direction: [f64; 3]) -> bool {
        let mut interval = [EPS, f64::INFINITY];
        for axis in 0..3 {
            if !clip(receiver[axis], direction[axis], self.lo[axis], self.hi[axis], &mut interval) {
                return false;
            }
        }
        if self.volume && !clip(receiver[1] - receiver[2], direction[1] - direction[2],
            self.image_y[0], self.image_y[1], &mut interval) {
            return false;
        }
        self.blocks_in_interval(receiver, direction, interval)
    }

    /// Every ground receiver in one screen row has exactly the same Y, Z and
    /// parallel-ray direction. Intersect those slabs once, retaining the exact
    /// interval rather than repeating three divisions per receiving pixel.
    fn ground_interval(&self, world_y: f64, direction: [f64; 3]) -> Option<[f64; 2]> {
        let mut interval = [EPS, f64::INFINITY];
        if !clip(world_y, direction[1], self.lo[1], self.hi[1], &mut interval)
            || !clip(0.0, direction[2], self.lo[2], self.hi[2], &mut interval)
            || self.volume && !clip(world_y, direction[1] - direction[2],
                self.image_y[0], self.image_y[1], &mut interval) {
            return None;
        }
        Some(interval)
    }

    /// A row's exact Y/Z interval also bounds the receiver X coordinates that
    /// can ever enter this caster. Round outward so edge ties still reach the
    /// original alpha/foot DDA; this bound never decides whether a ray blocks.
    fn ground_span(&self, direction_x: f64, interval: [f64; 2], cam_x: i32,
        bounds: [i32; 2]) -> Option<[i32; 2]> {
        let offset = [-direction_x * interval[0], -direction_x * interval[1]];
        let lo = self.lo[0] + offset[0].min(offset[1]) - f64::from(cam_x) - 0.5;
        let hi = self.hi[0] + offset[0].max(offset[1]) - f64::from(cam_x) - 0.5;
        let x0 = bounds[0].max(lo.floor() as i32);
        let x1 = bounds[1].min(hi.ceil() as i32);
        (x0 <= x1).then_some([x0, x1])
    }

    fn blocks_ground(&self, receiver: [f64; 3], direction: [f64; 3], mut interval: [f64; 2]) -> bool {
        if !clip(receiver[0], direction[0], self.lo[0], self.hi[0], &mut interval) {
            return false;
        }
        self.blocks_in_interval(receiver, direction, interval)
    }

    /// Both the full three-axis reference ray and the prepared ground-row ray
    /// enter the same native silhouette/finite-foot traversal here.
    fn blocks_in_interval(&self, receiver: [f64; 3], direction: [f64; 3], interval: [f64; 2]) -> bool {
        let display_x = receiver[0] - self.x0;
        // Traverse in original atlas UVs. Reflection reverses both continuous
        // origin and ray direction, preserving cell_after's exact edge rules.
        // Unlike flipping 8px display tiles, this also handles widths % 8 != 0.
        let origin = [if self.source.flip { f64::from(self.source.w) - display_x } else { display_x },
            if self.volume { receiver[1] - receiver[2] - self.y0 }
            else { f64::from(self.source.base) - receiver[2] - self.y0 }];
        let delta = [if self.source.flip { -direction[0] } else { direction[0] },
            if self.volume { direction[1] - direction[2] } else { -direction[2] }];
        let step = delta.map(|v| if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 });
        let alpha = self.alpha.as_ref().expect("native ray has a retained alpha silhouette");
        let mut t = interval[0];
        let end = interval[1];
        let mut x = cell_after(origin[0], delta[0], step[0], t);
        // A perfectly parallel ray retains the displayed pixel's half-open
        // ownership. Reflection reverses that ownership at an exact UV edge.
        if self.source.flip && step[0] == 0 && origin[0].fract() == 0.0 { x -= 1; }
        let mut y = cell_after(origin[1], delta[1], step[1], t);
        while t < end {
            let inside = x >= 0 && y >= 0 && x < self.source.w && y < self.source.h;
            let empty = inside && alpha.occupied[(y / BLOCK) as usize * alpha.block_columns + (x / BLOCK) as usize] == 0;
            let size = if empty { BLOCK } else { 1 };
            let next_axis = |cell: i32, origin: f64, delta: f64, step: i32| {
                if step == 0 { f64::INFINITY }
                else {
                    let edge = if step > 0 { (cell.div_euclid(size) + 1) * size }
                        else { cell.div_euclid(size) * size };
                    (f64::from(edge) - origin) / delta
                }
            };
            let tx = next_axis(x, origin[0], delta[0], step[0]);
            let ty = next_axis(y, origin[1], delta[1], step[1]);
            let next = tx.min(ty).min(end);
            debug_assert!(next > t, "sun alpha DDA must advance");
            if inside && !empty && self.source.pixels[(y * self.source.w + x) as usize].a {
                if !self.volume { return true; }
                let span = self.source.foot[x as usize];
                let mut cell = [t, next];
                if clip(receiver[1], direction[1], f64::from(span[0]), f64::from(span[1]), &mut cell) {
                    return true;
                }
            }
            if next >= end { break; }
            let boundary_x = if step[0] > 0 { (x.div_euclid(size) + 1) * size }
                else { x.div_euclid(size) * size - 1 };
            let boundary_y = if step[1] > 0 { (y.div_euclid(size) + 1) * size }
                else { y.div_euclid(size) * size - 1 };
            x = if tx <= next { boundary_x } else if size == 1 || step[0] == 0 { x }
                else { cell_after(origin[0], delta[0], step[0], next) };
            y = if ty <= next { boundary_y } else if size == 1 { y }
                else { cell_after(origin[1], delta[1], step[1], next) };
            t = next;
        }
        false
    }

    #[cfg(test)]
    fn blocks_in_interval_reference(&self, receiver: [f64; 3], direction: [f64; 3], interval: [f64; 2]) -> bool {
        let origin = [receiver[0] - self.x0, if self.volume {
            receiver[1] - receiver[2] - self.y0
        } else { f64::from(self.source.base) - receiver[2] - self.y0 }];
        let delta = [direction[0], if self.volume { direction[1] - direction[2] } else { -direction[2] }];
        let step = delta.map(|v| if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 });
        let mut t = interval[0];
        let end = interval[1];
        let mut x = cell_after(origin[0], delta[0], step[0], t);
        let mut y = cell_after(origin[1], delta[1], step[1], t);
        while t < end {
            let tx = crossing(x, origin[0], delta[0], step[0]);
            let ty = crossing(y, origin[1], delta[1], step[1]);
            let next = tx.min(ty).min(end);
            debug_assert!(next > t, "sun native DDA must advance");
            if x >= 0 && y >= 0 && x < self.source.w && y < self.source.h {
                let source_x = self.source_x(x);
                if self.source.pixels[(y * self.source.w + source_x) as usize].a {
                    if !self.volume { return true; }
                    let span = self.source.foot[source_x as usize];
                    let mut cell = [t, next];
                    if clip(receiver[1], direction[1], f64::from(span[0]), f64::from(span[1]), &mut cell) {
                        return true;
                    }
                }
            }
            if next >= end { break; }
            if tx <= next { x += step[0]; }
            if ty <= next { y += step[1]; }
            t = next;
        }
        false
    }
}

struct Receiver {
    index: usize,
    point: [f64; 3],
    owner: u32,
}

struct ReceiverRow { y: i32, first: usize, end: usize }

struct RaisedGroup {
    receivers: Vec<Receiver>,
    rows: Vec<ReceiverRow>,
    height: [f64; 2],
    bounds: [i32; 4],
}

impl RaisedGroup {
    fn new() -> Self {
        Self { receivers: Vec::new(), rows: Vec::new(), height: [f64::INFINITY, f64::NEG_INFINITY],
            bounds: [W as i32, H as i32, -1, -1] }
    }

    fn push(&mut self, receiver: Receiver) {
        let y = (receiver.index / W) as i32;
        let x = (receiver.index % W) as i32;
        self.height[0] = self.height[0].min(receiver.point[2]);
        self.height[1] = self.height[1].max(receiver.point[2]);
        self.bounds[0] = self.bounds[0].min(x); self.bounds[2] = self.bounds[2].max(x);
        self.bounds[1] = self.bounds[1].min(y); self.bounds[3] = self.bounds[3].max(y);
        let index = self.receivers.len();
        self.receivers.push(receiver);
        if let Some(row) = self.rows.last_mut() && row.y == y { row.end = index + 1; }
        else { self.rows.push(ReceiverRow { y, first: index, end: index + 1 }); }
    }
}

/// Sun visibility on visible receiving surfaces, 255 clear / 0 opaque.
/// Every current caster is eligible, including a caster outside the viewport.
pub(super) fn visibility(f: &Frame, direction: [f32; 3]) -> Vec<u8> {
    let mut result = vec![255; W * H];
    if direction.iter().any(|v| !v.is_finite()) || direction[2] <= 0.0 { return result; }
    let direction = direction.map(f64::from);
    let mut max_height = 0.0_f64;
    let mut raised = BTreeMap::<i32, RaisedGroup>::new();
    for (index, surface) in f.surfaces.iter().enumerate() {
        if surface.kind == 0 || surface.height == 0.0 { continue; }
        let height = f64::from(surface.height);
        max_height = max_height.max(height);
        let point = [
            f64::from((index % W) as i32 + f.cam_x) + 0.5,
            f64::from((index / W) as i32 + f.cam_y) + 0.5 + height,
            height,
        ];
        raised.entry((height / 8.0).floor() as i32).or_insert_with(RaisedGroup::new)
            .push(Receiver { index, point, owner: surface.owner });
    }
    for source in &f.surface_casters {
        if !cell_reaches_frame(source, direction, max_height, f) { continue; }
        let Some(caster) = Native::new(source) else { continue };
        if let Some([x0, y0, x1, y1]) = caster.shadow_bounds(direction, [0.0, 0.0], f) {
            for y in y0..=y1 {
                let world_y = f64::from(y + f.cam_y) + 0.5;
                let Some(interval) = caster.ground_interval(world_y, direction) else { continue };
                let Some([lo, hi]) = caster.ground_span(direction[0], interval, f.cam_x, [x0, x1]) else { continue };
                for x in lo..=hi {
                    let i = y as usize * W + x as usize;
                    let surface = f.surfaces[i];
                    if surface.kind == 0 || surface.height != 0.0 || result[i] <= caster.visible
                        || source.owner != 0 && surface.owner == source.owner { continue; }
                    let receiver = [f64::from(x + f.cam_x) + 0.5, world_y, 0.0];
                    if caster.blocks_ground(receiver, direction, interval) { result[i] = caster.visible; }
                }
            }
        }
        for group in raised.values() {
            // Receiver rays rise toward the sun; a whole receiving band above
            // this caster cannot meet its finite native height interval.
            if group.height[0] >= caster.hi[2] { continue; }
            let Some(rect) = caster.shadow_bounds(direction, group.height, f) else { continue };
            let [x0, y0, x1, y1] = [rect[0].max(group.bounds[0]), rect[1].max(group.bounds[1]),
                rect[2].min(group.bounds[2]), rect[3].min(group.bounds[3])];
            if x0 > x1 || y0 > y1 { continue; }
            let first = group.rows.partition_point(|row| row.y < y0);
            let end = group.rows.partition_point(|row| row.y <= y1);
            for row in &group.rows[first..end] {
                let receivers = &group.receivers[row.first..row.end];
                let first = receivers.partition_point(|r| r.index % W < x0 as usize);
                let end = receivers.partition_point(|r| r.index % W <= x1 as usize);
                for receiver in &receivers[first..end] {
                    if result[receiver.index] <= caster.visible
                        || source.owner != 0 && receiver.owner == source.owner { continue; }
                    if caster.blocks(receiver.point, direction) { result[receiver.index] = caster.visible; }
                }
            }
        }
    }
    result
}

const BLOCK: i32 = 8;

#[derive(Clone, Copy)]
struct Texel { x: i32, y: i32, height: u8 }

struct TexelRow { y: i32, first: usize, end: usize, max_height: u8 }

#[derive(Clone, Copy)]
struct BlockPlane { cap: u8, front: i32 }

/// Conservative height bounds and fully verified own-sheet invariants over
/// aligned world blocks. These only prove a ray interval clear; mixed or
/// potentially intersecting blocks still use the original native pixel DDA.
struct TerrainLevel {
    size: i32,
    columns: usize,
    maxima: Vec<u8>,
    planes: Vec<BlockPlane>,
}

#[derive(Clone, Copy, Default)]
struct Projection { x: [f64; 2], y: [f64; 2], interval: [f64; 2] }

/// The exact painted raised terrain, retained once with the region's field.
/// Height-zero floor and collision-only wall tiles never become blockers.
pub(super) struct SunTerrain {
    width: i32,
    height: i32,
    columns: usize,
    heights: Vec<u8>,
    blocks: Vec<u8>,
    occupied_prefix: Vec<u32>,
    block_planes: Vec<BlockPlane>,
    coarse: Vec<TerrainLevel>,
    texels: Vec<Texel>,
    texel_rows: Vec<TexelRow>,
    max_height: u8,
    volumes: Vec<crate::world::TerrainVolume>,
}

impl SunTerrain {
    pub(super) fn new(zone: &crate::world::Zone) -> Self {
        let columns = ((zone.pw + BLOCK - 1) / BLOCK) as usize;
        let rows = ((zone.ph + BLOCK - 1) / BLOCK) as usize;
        let mut heights = vec![0; (zone.pw * zone.ph) as usize];
        let mut blocks = vec![0_u8; columns * rows];
        let mut texels = Vec::new();
        let mut texel_rows = Vec::new();
        let mut max_height = 0;
        for y in 0..zone.ph {
            let first = texels.len();
            let mut row_height = 0;
            for x in 0..zone.pw {
                let i = (y * zone.pw + x) as usize;
                let height = zone.ground_surfaces[i].height;
                if height == 0 || zone.ground[i].r < 0.0 { continue; }
                heights[i] = height;
                let b = (y / BLOCK) as usize * columns + (x / BLOCK) as usize;
                blocks[b] = blocks[b].max(height);
                max_height = max_height.max(height);
                row_height = row_height.max(height);
                texels.push(Texel { x, y, height });
            }
            if texels.len() != first {
                texel_rows.push(TexelRow { y, first, end: texels.len(), max_height: row_height });
            }
        }
        let mut occupied_prefix = vec![0; (columns + 1) * (rows + 1)];
        for y in 0..rows {
            let mut row = 0;
            for x in 0..columns {
                row += u32::from(blocks[y * columns + x] != 0);
                occupied_prefix[(y + 1) * (columns + 1) + x + 1] = occupied_prefix[y * (columns + 1) + x + 1] + row;
            }
        }
        let mut block_planes = vec![BlockPlane { cap: 0, front: -1 }; blocks.len()];
        for by in 0..rows { for bx in 0..columns {
            let b = by * columns + bx;
            if blocks[b] == 0 { continue; }
            let (x0, y0) = (bx as i32 * BLOCK, by as i32 * BLOCK);
            let first = heights[(y0 * zone.pw + x0) as usize];
            if first == 0 { continue; }
            let mut plane = BlockPlane { cap: first, front: y0 + i32::from(first) };
            for y in y0..(y0 + BLOCK).min(zone.ph) { for x in x0..(x0 + BLOCK).min(zone.pw) {
                let h = heights[(y * zone.pw + x) as usize];
                if h != plane.cap { plane.cap = 0; }
                if h == 0 || y + i32::from(h) != plane.front { plane.front = -1; }
            } }
            block_planes[b] = plane;
        } }
        let coarse = [32, 128].into_iter().map(|size| {
            let level_columns = ((zone.pw + size - 1) / size) as usize;
            let level_rows = ((zone.ph + size - 1) / size) as usize;
            let mut maxima = vec![0; level_columns * level_rows];
            let mut planes = vec![BlockPlane { cap: 0, front: -1 }; maxima.len()];
            for ly in 0..level_rows { for lx in 0..level_columns {
                let i = ly * level_columns + lx;
                let x0 = lx as i32 * size / BLOCK;
                let y0 = ly as i32 * size / BLOCK;
                let mut plane = block_planes[y0 as usize * columns + x0 as usize];
                for y in y0..(y0 + size / BLOCK).min(rows as i32) {
                    for x in x0..(x0 + size / BLOCK).min(columns as i32) {
                        let b = y as usize * columns + x as usize;
                        maxima[i] = maxima[i].max(blocks[b]);
                        if block_planes[b].cap != plane.cap { plane.cap = 0; }
                        if block_planes[b].front != plane.front { plane.front = -1; }
                    }
                }
                planes[i] = plane;
            } }
            TerrainLevel { size, columns: level_columns, maxima, planes }
        }).collect();
        Self { width: zone.pw, height: zone.ph, columns, heights, blocks, occupied_prefix,
            block_planes, coarse, texels, texel_rows, max_height, volumes: zone.terrain_volumes.clone() }
    }

    fn at(&self, x: i32, y: i32) -> u8 {
        self.heights[(y * self.width + x) as usize]
    }

    /// Empty projected ray rectangles cannot contain any native terrain sheet.
    /// A positive occupancy result is only a candidate, never a shadow itself.
    fn occupied_rect(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let (x0, y0) = (x0.max(0), y0.max(0));
        let (x1, y1) = (x1.min(self.width - 1), y1.min(self.height - 1));
        if x0 > x1 || y0 > y1 { return false; }
        let (x0, y0) = ((x0 / BLOCK) as usize, (y0 / BLOCK) as usize);
        let (x1, y1) = ((x1 / BLOCK) as usize + 1, (y1 / BLOCK) as usize + 1);
        let stride = self.columns + 1;
        self.occupied_prefix[y1 * stride + x1] + self.occupied_prefix[y0 * stride + x0]
            != self.occupied_prefix[y0 * stride + x1] + self.occupied_prefix[y1 * stride + x0]
    }

    /// Exact finite intersection with one painted 1px XY sheet at height +/- .5.
    /// The first two ray components are projected world X,Y; third is world Z.
    #[cfg(test)]
    fn texel_blocks(texel: Texel, receiver: [f64; 3], ray: [f64; 3]) -> bool {
        let mut interval = [EPS, f64::INFINITY];
        let pixel = |origin: f64, delta: f64, lo: f64, hi: f64, interval: &mut [f64; 2]| {
            // A parallel ray on a pixel's right/bottom boundary belongs to the
            // adjacent pixel, matching floor()/DDA native sample ownership.
            if delta.abs() < 1.0e-12 { origin >= lo && origin < hi && interval[0] < interval[1] }
            else { clip(origin, delta, lo, hi, interval) }
        };
        pixel(receiver[0], ray[0], f64::from(texel.x), f64::from(texel.x + 1), &mut interval)
            && pixel(receiver[1], ray[1], f64::from(texel.y), f64::from(texel.y + 1), &mut interval)
            && clip(receiver[2], ray[2], f64::from(texel.height) - 0.5,
                f64::from(texel.height) + 0.5, &mut interval)
    }

    /// Parallel projected-ray DDA for sparse raised receivers. Empty 8px blocks
    /// are skipped, and traversal ends naturally above the tallest native sheet.
    #[cfg(test)]
    fn blocked(&self, receiver: [f64; 3], own_terrain: bool, direction: [f64; 3]) -> bool {
        super::terrain_volumes::any(&self.volumes, receiver, direction, f64::INFINITY, own_terrain)
            || self.blocked_hierarchy(receiver, own_terrain, direction)
    }

    /// Turning off the block proof retains the original pixel traversal for
    /// regression comparisons. Both paths use the exact same sheet geometry.
    #[cfg(test)]
    fn blocked_inner<const BLOCK_PROOF: bool>(&self, receiver: [f64; 3], own_terrain: bool, direction: [f64; 3]) -> bool {
        if super::terrain_volumes::any(&self.volumes, receiver, direction, f64::INFINITY, own_terrain) { return true; }
        let ray = [direction[0], direction[1] - direction[2], direction[2]];
        let mut interval = [EPS, (f64::from(self.max_height) + 0.5 - receiver[2]) / ray[2]];
        if interval[1] <= interval[0] { return false; }
        if BLOCK_PROOF {
            let end = [receiver[0] + ray[0] * interval[1], receiver[1] + ray[1] * interval[1]];
            if !self.occupied_rect(
                receiver[0].min(end[0]).floor() as i32, receiver[1].min(end[1]).floor() as i32,
                receiver[0].max(end[0]).floor() as i32, receiver[1].max(end[1]).floor() as i32) { return false; }
        }
        if !clip(receiver[0], ray[0], 0.0, f64::from(self.width), &mut interval)
            || !clip(receiver[1], ray[1], 0.0, f64::from(self.height), &mut interval) { return false; }
        let step = [ray[0], ray[1]].map(|d| if d > 0.0 { 1 } else if d < 0.0 { -1 } else { 0 });
        let mut t = interval[0];
        let end = interval[1];
        let mut x = cell_after(receiver[0], ray[0], step[0], t).clamp(0, self.width - 1);
        let mut y = cell_after(receiver[1], ray[1], step[1], t).clamp(0, self.height - 1);
        let mut own_plane = own_terrain && receiver[2] > 0.0;
        while t < end && x >= 0 && y >= 0 && x < self.width && y < self.height {
            let b = (y / BLOCK) as usize * self.columns + (x / BLOCK) as usize;
            let block_height = self.blocks[b];
            let plane = self.block_planes[b];
            // Every native pixel in a proven block satisfies the SAME original
            // own-sheet invariant. Skip the block without losing continuity;
            // holes, steps, mixed planes and actors keep the pixel traversal.
            let skip_own = BLOCK_PROOF && own_plane && (plane.cap > 0
                && (f64::from(plane.cap) - receiver[2]).abs() <= 0.51
                || plane.front >= 0 && (f64::from(plane.front) + 0.5 - receiver[1] - receiver[2]).abs() <= 0.51);
            // A ray rises monotonically toward the sun. Once above all native
            // sheets in a block it cannot intersect that block later. Preserve
            // own-plane continuity until its normal native traversal ends.
            let occupied = !skip_own && block_height > 0 && (own_plane
                || receiver[2] + ray[2] * t <= f64::from(block_height) + 0.5);
            let size = if occupied { 1 } else { BLOCK };
            let next_axis = |cell: i32, origin: f64, delta: f64, step: i32| {
                if step == 0 { f64::INFINITY }
                else {
                    let edge = if step > 0 { (cell.div_euclid(size) + 1) * size }
                        else { cell.div_euclid(size) * size };
                    (f64::from(edge) - origin) / delta
                }
            };
            let tx = next_axis(x, receiver[0], ray[0], step[0]);
            let ty = next_axis(y, receiver[1], ray[1], step[1]);
            let next = tx.min(ty).min(end);
            debug_assert!(next > t, "sun terrain DDA must advance");
            if occupied {
                let h = f64::from(self.at(x, y));
                let same = own_plane && h > 0.0 && ((h - receiver[2]).abs() <= 0.51
                    || (f64::from(y) + 0.5 + h - receiver[1] - receiver[2]).abs() <= 0.51);
                if !same {
                    own_plane = false;
                    if h > 0.0 {
                        let near = (h - 0.5 - receiver[2]) / ray[2];
                        let far = (h + 0.5 - receiver[2]) / ray[2];
                        if t.max(near) < next.min(far) { return true; }
                    }
                }
            } else if !skip_own { own_plane = false; }
            if next >= end { break; }
            let boundary_x = if step[0] > 0 { (x.div_euclid(size) + 1) * size }
                else { x.div_euclid(size) * size - 1 };
            let boundary_y = if step[1] > 0 { (y.div_euclid(size) + 1) * size }
                else { y.div_euclid(size) * size - 1 };
            x = if tx <= next { boundary_x } else if size == 1 { x }
                else { cell_after(receiver[0], ray[0], step[0], next) };
            y = if ty <= next { boundary_y } else if size == 1 { y }
                else { cell_after(receiver[1], ray[1], step[1], next) };
            t = next;
        }
        false
    }

    /// Same finite sheet ray as the retained 8px reference, with larger
    /// analytically clear intervals. No ray length, height or hit is sampled.
    fn blocked_hierarchy(&self, receiver: [f64; 3], own_terrain: bool, direction: [f64; 3]) -> bool {
        let ray = [direction[0], direction[1] - direction[2], direction[2]];
        let mut interval = [EPS, (f64::from(self.max_height) + 0.5 - receiver[2]) / ray[2]];
        if interval[1] <= interval[0] { return false; }
        {
            let end = [receiver[0] + ray[0] * interval[1], receiver[1] + ray[1] * interval[1]];
            if !self.occupied_rect(
                receiver[0].min(end[0]).floor() as i32, receiver[1].min(end[1]).floor() as i32,
                receiver[0].max(end[0]).floor() as i32, receiver[1].max(end[1]).floor() as i32) { return false; }
        }
        if !clip(receiver[0], ray[0], 0.0, f64::from(self.width), &mut interval)
            || !clip(receiver[1], ray[1], 0.0, f64::from(self.height), &mut interval) { return false; }
        let step = [ray[0], ray[1]].map(|d| if d > 0.0 { 1 } else if d < 0.0 { -1 } else { 0 });
        let mut t = interval[0];
        let end = interval[1];
        let mut x = cell_after(receiver[0], ray[0], step[0], t).clamp(0, self.width - 1);
        let mut y = cell_after(receiver[1], ray[1], step[1], t).clamp(0, self.height - 1);
        let mut own_plane = own_terrain && receiver[2] > 0.0;
        while t < end && x >= 0 && y >= 0 && x < self.width && y < self.height {
            let b = (y / BLOCK) as usize * self.columns + (x / BLOCK) as usize;
            let block_height = self.blocks[b];
            let plane = self.block_planes[b];
            // Every native pixel in a proven block satisfies the SAME original
            // own-sheet invariant. Skip the block without losing continuity;
            // holes, steps, mixed planes and actors keep the pixel traversal.
            let mut skip_own = own_plane && (plane.cap > 0
                && (f64::from(plane.cap) - receiver[2]).abs() <= 0.51
                || plane.front >= 0 && (f64::from(plane.front) + 0.5 - receiver[1] - receiver[2]).abs() <= 0.51);
            // A ray rises monotonically toward the sun. Once above all native
            // sheets in a block it cannot intersect that block later. Preserve
            // own-plane continuity until its normal native traversal ends.
            let occupied = !skip_own && block_height > 0 && (own_plane
                || receiver[2] + ray[2] * t <= f64::from(block_height) + 0.5);
            let mut size = if occupied { 1 } else { BLOCK };
            let next_axis = |cell: i32, origin: f64, delta: f64, step: i32, size: i32| {
                if step == 0 { f64::INFINITY }
                else {
                    let edge = if step > 0 { (cell.div_euclid(size) + 1) * size }
                        else { cell.div_euclid(size) * size };
                    (f64::from(edge) - origin) / delta
                }
            };
            let mut tx = next_axis(x, receiver[0], ray[0], step[0], size);
            let mut ty = next_axis(y, receiver[1], ray[1], step[1], size);
            let mut next = tx.min(ty).min(end);
            if !occupied {
                for level in &self.coarse {
                    let b = (y / level.size) as usize * level.columns + (x / level.size) as usize;
                    let maximum = level.maxima[b];
                    let plane = level.planes[b];
                    let same_plane = own_plane && (plane.cap > 0
                        && (f64::from(plane.cap) - receiver[2]).abs() <= 0.51
                        || plane.front >= 0 && (f64::from(plane.front) + 0.5 - receiver[1] - receiver[2]).abs() <= 0.51);
                    // Own-sheet continuity can only cross a nonempty coarse
                    // block when EVERY native pixel proves the same original
                    // cap/front invariant. Otherwise retain the 8px/pixel path.
                    if maximum != 0 && !same_plane && (own_plane
                        || receiver[2] + ray[2] * t <= f64::from(maximum) + 0.5) { break; }
                    let bx = next_axis(x, receiver[0], ray[0], step[0], level.size);
                    let by = next_axis(y, receiver[1], ray[1], step[1], level.size);
                    let until = bx.min(by).min(end);
                    // The complete segment must stay strictly above EVERY
                    // retained sheet. A bound never substitutes for geometry.
                    if maximum == 0 || same_plane || !own_plane
                        && (receiver[2] + ray[2] * t).min(receiver[2] + ray[2] * until) > f64::from(maximum) + 0.5 {
                        size = level.size;
                        tx = bx;
                        ty = by;
                        next = until;
                        skip_own = same_plane;
                    } else { break; }
                }
            }
            debug_assert!(next > t, "sun terrain DDA must advance");
            if occupied {
                let h = f64::from(self.at(x, y));
                let same = own_plane && h > 0.0 && ((h - receiver[2]).abs() <= 0.51
                    || (f64::from(y) + 0.5 + h - receiver[1] - receiver[2]).abs() <= 0.51);
                if !same {
                    own_plane = false;
                    if h > 0.0 {
                        let near = (h - 0.5 - receiver[2]) / ray[2];
                        let far = (h + 0.5 - receiver[2]) / ray[2];
                        if t.max(near) < next.min(far) { return true; }
                    }
                }
            } else if !skip_own { own_plane = false; }
            if next >= end { break; }
            let boundary_x = if step[0] > 0 { (x.div_euclid(size) + 1) * size }
                else { x.div_euclid(size) * size - 1 };
            let boundary_y = if step[1] > 0 { (y.div_euclid(size) + 1) * size }
                else { y.div_euclid(size) * size - 1 };
            x = if tx <= next { boundary_x } else if size == 1 { x }
                else { cell_after(receiver[0], ray[0], step[0], next) };
            y = if ty <= next { boundary_y } else if size == 1 { y }
                else { cell_after(receiver[1], ray[1], step[1], next) };
            t = next;
        }
        false
    }

    /// Add terrain visibility to the native caster mask. Height-zero receiving
    /// pixels use exact forward projection, avoiding a ray walk per floor pixel.
    /// Raised actors/scenery use the same physical ray through sparse terrain.
    pub(super) fn occlude(&self, f: &Frame, direction: [f32; 3], result: &mut [u8]) {
        if self.max_height == 0 && self.volumes.is_empty() || direction[2] <= 0.0
            || direction.iter().any(|v| !v.is_finite()) { return; }
        assert_eq!(result.len(), W * H);
        let direction = direction.map(f64::from);
        self.occlude_volumes(f, direction, result);
        let slope = [direction[0] / direction[2], direction[1] / direction[2]];
        let ray = [direction[0], direction[1] - direction[2], direction[2]];
        let mut projection = [Projection::default(); 256];
        for h in 1..=usize::from(self.max_height) {
            let z = [h as f64 - 0.5, h as f64 + 0.5];
            let x = [-slope[0] * z[0], -slope[0] * z[1]];
            let y = [(1.0 - slope[1]) * z[0], (1.0 - slope[1]) * z[1]];
            projection[h] = Projection { x: [x[0].min(x[1]), x[0].max(x[1])],
                y: [y[0].min(y[1]), y[0].max(y[1])],
                interval: [EPS.max(z[0] / ray[2]), z[1] / ray[2]] };
        }
        // Texels retain the painter's increasing Y order. Limit the scan to all
        // source rows that can physically project into this camera's floor.
        let vertical_reach = (1.0 - slope[1]) * (f64::from(self.max_height) + 0.5);
        let source_y0 = f64::from(f.cam_y) - vertical_reach.max(0.0) - 1.0;
        let source_y1 = f64::from(f.cam_y) + H as f64 - vertical_reach.min(0.0);
        let first = self.texel_rows.partition_point(|p| f64::from(p.y) < source_y0);
        let end = self.texel_rows.partition_point(|p| f64::from(p.y) <= source_y1);
        for row in &self.texel_rows[first..end] {
            // A source row can only project horizontally by height * slope.
            // The extra pixel encloses the whole native source sheet. This
            // rejects off-camera X ranges, never a possible receiving ray.
            let reach = -slope[0] * (f64::from(row.max_height) + 0.5);
            let source_x0 = f64::from(f.cam_x) - reach.max(0.0) - 1.0;
            let source_x1 = f64::from(f.cam_x) + W as f64 - reach.min(0.0);
            let texels = &self.texels[row.first..row.end];
            let first = texels.partition_point(|p| f64::from(p.x) < source_x0);
            let end = texels.partition_point(|p| f64::from(p.x) <= source_x1);
            for &texel in &texels[first..end] {
            let projected = projection[usize::from(texel.height)];
            let lo_x = f64::from(texel.x - f.cam_x) + projected.x[0];
            let hi_x = f64::from(texel.x + 1 - f.cam_x) + projected.x[1];
            let lo_y = f64::from(texel.y - f.cam_y) + projected.y[0];
            let hi_y = f64::from(texel.y + 1 - f.cam_y) + projected.y[1];
            let x0 = (lo_x.floor() as i32).max(0);
            let x1 = (hi_x.ceil() as i32).min(W as i32 - 1);
            let y0 = (lo_y.floor() as i32).max(0);
            let y1 = (hi_y.ceil() as i32).min(H as i32 - 1);
            if x0 > x1 || y0 > y1 { continue; }
            for y in y0..=y1 {
                let world_y = f64::from(y + f.cam_y) + 0.5;
                let mut row_interval = projected.interval;
                let (lo, hi) = (f64::from(texel.y), f64::from(texel.y + 1));
                // Parallel pixel ownership is half-open, exactly as in the
                // original texel_blocks slab oracle.
                if ray[1].abs() < 1.0e-12 {
                    if world_y < lo || world_y >= hi { continue; }
                } else if !clip(world_y, ray[1], lo, hi, &mut row_interval) { continue; }
                for x in x0..=x1 {
                    let i = y as usize * W + x as usize;
                    if result[i] == 0 || f.surfaces[i].kind == 0 || f.surfaces[i].height > EPS as f32 { continue; }
                    let world_x = f64::from(x + f.cam_x) + 0.5;
                    let (lo, hi) = (f64::from(texel.x), f64::from(texel.x + 1));
                    let mut interval = row_interval;
                    let blocked = if ray[0].abs() < 1.0e-12 { world_x >= lo && world_x < hi }
                        else { clip(world_x, ray[0], lo, hi, &mut interval) };
                    if blocked { result[i] = 0; }
                }
            }
            }
        }
        for (i, surface) in f.surfaces.iter().enumerate() {
            if result[i] == 0 || surface.kind == 0 || surface.height <= EPS as f32 { continue; }
            let x = (i % W) as i32 + f.cam_x;
            let y = (i / W) as i32 + f.cam_y;
            let own_terrain = surface.owner == 0 && x >= 0 && y >= 0 && x < self.width && y < self.height
                && (f32::from(self.at(x, y)) - surface.height).abs() < 0.001;
            if self.blocked_hierarchy([f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(surface.height)], own_terrain, direction) {
                result[i] = 0;
            }
        }
    }

    fn occlude_volumes(&self, f: &Frame, direction: [f64; 3], result: &mut [u8]) {
        let slope = [direction[0] / direction[2], direction[1] / direction[2]];
        for &volume in &self.volumes {
            let top = f64::from(volume.max[2]);
            let dx = -slope[0] * top;
            // Enclose projections onto every nonnegative receiving height,
            // including raised stair treads. These are only candidate bounds;
            // every candidate still intersects the exact finite solid below.
            let dy = [-slope[1] * top, -top];
            let x0 = ((f64::from(volume.min[0]) + dx.min(0.0)).floor() as i32 - f.cam_x).max(0);
            let x1 = ((f64::from(volume.max[0]) + dx.max(0.0)).ceil() as i32 - f.cam_x).min(W as i32 - 1);
            let y0 = ((f64::from(volume.min[1]) + dy[0].min(dy[1]).min(0.0)).floor() as i32 - f.cam_y).max(0);
            let y1 = ((f64::from(volume.max[1]) + dy[0].max(dy[1]).max(0.0)).ceil() as i32 - f.cam_y).min(H as i32 - 1);
            for y in y0..=y1 { for x in x0..=x1 {
                let i = y as usize * W + x as usize;
                let surface = f.surfaces[i];
                if result[i] == 0 || surface.kind == 0 || f64::from(surface.height) > top { continue; }
                let (wx, wy) = (x + f.cam_x, y + f.cam_y);
                let own = surface.owner == 0 && wx >= 0 && wy >= 0 && wx < self.width && wy < self.height
                    && (f32::from(self.at(wx, wy)) - surface.height).abs() < 0.001;
                let receiver = [f64::from(wx) + 0.5, f64::from(wy) + 0.5, f64::from(surface.height)];
                if super::terrain_volumes::blocked(volume, receiver, direction, f64::INFINITY, own) { result[i] = 0; }
            } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::frame::Surface;
    use crate::gfx::surface::NormalPixel;
    use std::sync::Arc;

    fn caster(rows: &[&str]) -> SurfaceCaster {
        let w = rows[0].len() as i32;
        SurfaceCaster {
            pixels: Arc::from(rows.iter().flat_map(|row| row.bytes().map(|p| NormalPixel {
                n: [0, 127, 0], a: p != b'.', emit: false,
            })).collect::<Vec<_>>()),
            w, h: rows.len() as i32, anchor: (0, 0), x: 8.0, y: 2.0, base: 8.0,
            flip: false, owner: 7, opacity: 1.0,
            foot: Arc::from(vec![[5.0, 8.0]; w as usize]), min_height: 0.0,
            reflectivity: 0.0,
        }
    }

    /// The original complete-height rectangle implementation is retained only
    /// as an output oracle for the sparse receiving-height optimization.
    fn visibility_reference(f: &Frame, direction: [f32; 3]) -> Vec<u8> {
        let mut result = vec![255; W * H];
        let direction = direction.map(f64::from);
        let max_height = f.surfaces.iter().filter(|s| s.kind != 0)
            .fold(0.0_f64, |h, s| h.max(f64::from(s.height)));
        for source in &f.surface_casters {
            if !cell_reaches_frame(source, direction, max_height, f) { continue; }
            let Some(caster) = Native::new(source) else { continue };
            let Some([x0, y0, x1, y1]) = caster.shadow_bounds(direction, [0.0, max_height], f) else { continue };
            for y in y0..=y1 { for x in x0..=x1 {
                let i = y as usize * W + x as usize;
                let surface = f.surfaces[i];
                if surface.kind == 0 || result[i] <= caster.visible
                    || source.owner != 0 && surface.owner == source.owner { continue; }
                let height = f64::from(surface.height);
                let receiver = [f64::from(x + f.cam_x) + 0.5,
                    f64::from(y + f.cam_y) + 0.5 + height, height];
                if caster.blocks_reference(receiver, direction) { result[i] = caster.visible; }
            } }
        }
        result
    }

    /// Previous terrain raster path, without row/X culling, per-height offset
    /// reuse, or own-plane block proofs. Keep exact per-source slab tests and
    /// the original pixel DDA as an independent optimization-output oracle.
    fn terrain_visibility_reference(terrain: &SunTerrain, f: &Frame, direction: [f32; 3], mut result: Vec<u8>) -> Vec<u8> {
        let direction = direction.map(f64::from);
        terrain_volumes_reference(terrain, f, direction, &mut result);
        let slope = [direction[0] / direction[2], direction[1] / direction[2]];
        let ray = [direction[0], direction[1] - direction[2], direction[2]];
        let vertical_reach = (1.0 - slope[1]) * (f64::from(terrain.max_height) + 0.5);
        let source_y0 = f64::from(f.cam_y) - vertical_reach.max(0.0) - 1.0;
        let source_y1 = f64::from(f.cam_y) + H as f64 - vertical_reach.min(0.0);
        let first = terrain.texels.partition_point(|p| f64::from(p.y) < source_y0);
        let end = terrain.texels.partition_point(|p| f64::from(p.y) <= source_y1);
        for &texel in &terrain.texels[first..end] {
            let z = [f64::from(texel.height) - 0.5, f64::from(texel.height) + 0.5];
            let offset_x = [-slope[0] * z[0], -slope[0] * z[1]];
            let offset_y = [(1.0 - slope[1]) * z[0], (1.0 - slope[1]) * z[1]];
            let x0 = ((f64::from(texel.x - f.cam_x) + offset_x[0].min(offset_x[1])).floor() as i32).max(0);
            let x1 = ((f64::from(texel.x + 1 - f.cam_x) + offset_x[0].max(offset_x[1])).ceil() as i32).min(W as i32 - 1);
            let y0 = ((f64::from(texel.y - f.cam_y) + offset_y[0].min(offset_y[1])).floor() as i32).max(0);
            let y1 = ((f64::from(texel.y + 1 - f.cam_y) + offset_y[0].max(offset_y[1])).ceil() as i32).min(H as i32 - 1);
            if x0 > x1 || y0 > y1 { continue; }
            for y in y0..=y1 { for x in x0..=x1 {
                let i = y as usize * W + x as usize;
                if result[i] == 0 || f.surfaces[i].kind == 0 || f.surfaces[i].height > EPS as f32 { continue; }
                let receiver = [f64::from(x + f.cam_x) + 0.5, f64::from(y + f.cam_y) + 0.5, 0.0];
                if SunTerrain::texel_blocks(texel, receiver, ray) { result[i] = 0; }
            } }
        }
        for (i, surface) in f.surfaces.iter().enumerate() {
            if result[i] == 0 || surface.kind == 0 || surface.height <= EPS as f32 { continue; }
            let x = (i % W) as i32 + f.cam_x;
            let y = (i / W) as i32 + f.cam_y;
            let own = surface.owner == 0 && x >= 0 && y >= 0 && x < terrain.width && y < terrain.height
                && (f32::from(terrain.at(x, y)) - surface.height).abs() < 0.001;
            if terrain.blocked_inner::<false>([f64::from(x) + 0.5, f64::from(y) + 0.5,
                f64::from(surface.height)], own, direction) { result[i] = 0; }
        }
        result
    }

    // Immediate pre-row-preparation paths: sparse raised receivers, native
    // bounds, terrain row/X culling and 8px terrain proofs match production.
    // Only the ground ray clipping/reuse differs, isolating its timing impact.
    fn native_before_ground_rows(f: &Frame, direction: [f32; 3]) -> Vec<u8> {
        let mut result = vec![255; W * H];
        if direction.iter().any(|v| !v.is_finite()) || direction[2] <= 0.0 { return result; }
        let direction = direction.map(f64::from);
        let mut max_height = 0.0_f64;
        let mut raised = BTreeMap::<i32, RaisedGroup>::new();
        for (index, surface) in f.surfaces.iter().enumerate() {
            if surface.kind == 0 || surface.height == 0.0 { continue; }
            let height = f64::from(surface.height);
            max_height = max_height.max(height);
            let point = [
                f64::from((index % W) as i32 + f.cam_x) + 0.5,
                f64::from((index / W) as i32 + f.cam_y) + 0.5 + height,
                height,
            ];
            raised.entry((height / 8.0).floor() as i32).or_insert_with(RaisedGroup::new)
                .push(Receiver { index, point, owner: surface.owner });
        }
        for source in &f.surface_casters {
            if !cell_reaches_frame(source, direction, max_height, f) { continue; }
            let Some(caster) = Native::new(source) else { continue };
            if let Some([x0, y0, x1, y1]) = caster.shadow_bounds(direction, [0.0, 0.0], f) {
                for y in y0..=y1 { for x in x0..=x1 {
                    let i = y as usize * W + x as usize;
                    let surface = f.surfaces[i];
                    if surface.kind == 0 || surface.height != 0.0 || result[i] <= caster.visible
                        || source.owner != 0 && surface.owner == source.owner { continue; }
                    let receiver = [f64::from(x + f.cam_x) + 0.5, f64::from(y + f.cam_y) + 0.5, 0.0];
                    if caster.blocks(receiver, direction) { result[i] = caster.visible; }
                } }
            }
            for group in raised.values() {
                // Receiver rays rise toward the sun; a whole receiving band above
                // this caster cannot meet its finite native height interval.
                if group.height[0] >= caster.hi[2] { continue; }
                let Some(rect) = caster.shadow_bounds(direction, group.height, f) else { continue };
                let [x0, y0, x1, y1] = [rect[0].max(group.bounds[0]), rect[1].max(group.bounds[1]),
                    rect[2].min(group.bounds[2]), rect[3].min(group.bounds[3])];
                if x0 > x1 || y0 > y1 { continue; }
                let first = group.rows.partition_point(|row| row.y < y0);
                let end = group.rows.partition_point(|row| row.y <= y1);
                for row in &group.rows[first..end] {
                    let receivers = &group.receivers[row.first..row.end];
                    let first = receivers.partition_point(|r| r.index % W < x0 as usize);
                    let end = receivers.partition_point(|r| r.index % W <= x1 as usize);
                    for receiver in &receivers[first..end] {
                        if result[receiver.index] <= caster.visible
                            || source.owner != 0 && receiver.owner == source.owner { continue; }
                        if caster.blocks(receiver.point, direction) { result[receiver.index] = caster.visible; }
                    }
                }
            }
        }
        result
    }

    // Immediate pre-alpha-tile path: identical prepared ground rows, sparse
    // raised receiver culls and bounds, with the retained original pixel DDA.
    fn native_before_alpha_blocks(f: &Frame, direction: [f32; 3]) -> Vec<u8> {
        let mut result = vec![255; W * H];
        if direction.iter().any(|v| !v.is_finite()) || direction[2] <= 0.0 { return result; }
        let direction = direction.map(f64::from);
        let mut max_height = 0.0_f64;
        let mut raised = BTreeMap::<i32, RaisedGroup>::new();
        for (index, surface) in f.surfaces.iter().enumerate() {
            if surface.kind == 0 || surface.height == 0.0 { continue; }
            let height = f64::from(surface.height);
            max_height = max_height.max(height);
            let point = [
                f64::from((index % W) as i32 + f.cam_x) + 0.5,
                f64::from((index / W) as i32 + f.cam_y) + 0.5 + height,
                height,
            ];
            raised.entry((height / 8.0).floor() as i32).or_insert_with(RaisedGroup::new)
                .push(Receiver { index, point, owner: surface.owner });
        }
        for source in &f.surface_casters {
            if !cell_reaches_frame(source, direction, max_height, f) { continue; }
            let Some(caster) = Native::new(source) else { continue };
            if let Some([x0, y0, x1, y1]) = caster.shadow_bounds(direction, [0.0, 0.0], f) {
                for y in y0..=y1 {
                    let world_y = f64::from(y + f.cam_y) + 0.5;
                    let Some(interval) = caster.ground_interval(world_y, direction) else { continue };
                    let Some([lo, hi]) = caster.ground_span(direction[0], interval, f.cam_x, [x0, x1]) else { continue };
                    for x in lo..=hi {
                        let i = y as usize * W + x as usize;
                        let surface = f.surfaces[i];
                        if surface.kind == 0 || surface.height != 0.0 || result[i] <= caster.visible
                            || source.owner != 0 && surface.owner == source.owner { continue; }
                        let receiver = [f64::from(x + f.cam_x) + 0.5, world_y, 0.0];
                        if caster.blocks_ground_reference(receiver, direction, interval) { result[i] = caster.visible; }
                    }
                }
            }
            for group in raised.values() {
                // Receiver rays rise toward the sun; a whole receiving band above
                // this caster cannot meet its finite native height interval.
                if group.height[0] >= caster.hi[2] { continue; }
                let Some(rect) = caster.shadow_bounds(direction, group.height, f) else { continue };
                let [x0, y0, x1, y1] = [rect[0].max(group.bounds[0]), rect[1].max(group.bounds[1]),
                    rect[2].min(group.bounds[2]), rect[3].min(group.bounds[3])];
                if x0 > x1 || y0 > y1 { continue; }
                let first = group.rows.partition_point(|row| row.y < y0);
                let end = group.rows.partition_point(|row| row.y <= y1);
                for row in &group.rows[first..end] {
                    let receivers = &group.receivers[row.first..row.end];
                    let first = receivers.partition_point(|r| r.index % W < x0 as usize);
                    let end = receivers.partition_point(|r| r.index % W <= x1 as usize);
                    for receiver in &receivers[first..end] {
                        if result[receiver.index] <= caster.visible
                            || source.owner != 0 && receiver.owner == source.owner { continue; }
                        if caster.blocks_reference(receiver.point, direction) { result[receiver.index] = caster.visible; }
                    }
                }
            }
        }
        result
    }

    fn terrain_before_ground_rows(terrain: &SunTerrain, f: &Frame, direction: [f32; 3], result: &mut [u8]) {
            if terrain.max_height == 0 || direction[2] <= 0.0
                || direction.iter().any(|v| !v.is_finite()) { return; }
            assert_eq!(result.len(), W * H);
            let direction = direction.map(f64::from);
            terrain_volumes_reference(terrain, f, direction, result);
            let slope = [direction[0] / direction[2], direction[1] / direction[2]];
            let ray = [direction[0], direction[1] - direction[2], direction[2]];
            let mut projection = [Projection::default(); 256];
            for h in 1..=usize::from(terrain.max_height) {
                let z = [h as f64 - 0.5, h as f64 + 0.5];
                let x = [-slope[0] * z[0], -slope[0] * z[1]];
                let y = [(1.0 - slope[1]) * z[0], (1.0 - slope[1]) * z[1]];
                projection[h] = Projection { x: [x[0].min(x[1]), x[0].max(x[1])],
                    y: [y[0].min(y[1]), y[0].max(y[1])],
                    interval: [EPS.max(z[0] / ray[2]), z[1] / ray[2]] };
            }
            // Texels retain the painter's increasing Y order. Limit the scan to all
            // source rows that can physically project into this camera's floor.
            let vertical_reach = (1.0 - slope[1]) * (f64::from(terrain.max_height) + 0.5);
            let source_y0 = f64::from(f.cam_y) - vertical_reach.max(0.0) - 1.0;
            let source_y1 = f64::from(f.cam_y) + H as f64 - vertical_reach.min(0.0);
            let first = terrain.texel_rows.partition_point(|p| f64::from(p.y) < source_y0);
            let end = terrain.texel_rows.partition_point(|p| f64::from(p.y) <= source_y1);
            for row in &terrain.texel_rows[first..end] {
                // A source row can only project horizontally by height * slope.
                // The extra pixel encloses the whole native source sheet. This
                // rejects off-camera X ranges, never a possible receiving ray.
                let reach = -slope[0] * (f64::from(row.max_height) + 0.5);
                let source_x0 = f64::from(f.cam_x) - reach.max(0.0) - 1.0;
                let source_x1 = f64::from(f.cam_x) + W as f64 - reach.min(0.0);
                let texels = &terrain.texels[row.first..row.end];
                let first = texels.partition_point(|p| f64::from(p.x) < source_x0);
                let end = texels.partition_point(|p| f64::from(p.x) <= source_x1);
                for &texel in &texels[first..end] {
                let projected = projection[usize::from(texel.height)];
                let lo_x = f64::from(texel.x - f.cam_x) + projected.x[0];
                let hi_x = f64::from(texel.x + 1 - f.cam_x) + projected.x[1];
                let lo_y = f64::from(texel.y - f.cam_y) + projected.y[0];
                let hi_y = f64::from(texel.y + 1 - f.cam_y) + projected.y[1];
                let x0 = (lo_x.floor() as i32).max(0);
                let x1 = (hi_x.ceil() as i32).min(W as i32 - 1);
                let y0 = (lo_y.floor() as i32).max(0);
                let y1 = (hi_y.ceil() as i32).min(H as i32 - 1);
                if x0 > x1 || y0 > y1 { continue; }
                for y in y0..=y1 { for x in x0..=x1 {
                    let i = y as usize * W + x as usize;
                    if result[i] == 0 || f.surfaces[i].kind == 0 || f.surfaces[i].height > EPS as f32 { continue; }
                    let receiver = [f64::from(x + f.cam_x) + 0.5, f64::from(y + f.cam_y) + 0.5, 0.0];
                    if SunTerrain::texel_blocks(texel, receiver, ray) { result[i] = 0; }
                } }
                }
            }
            for (i, surface) in f.surfaces.iter().enumerate() {
                if result[i] == 0 || surface.kind == 0 || surface.height <= EPS as f32 { continue; }
                let x = (i % W) as i32 + f.cam_x;
                let y = (i / W) as i32 + f.cam_y;
                let own_terrain = surface.owner == 0 && x >= 0 && y >= 0 && x < terrain.width && y < terrain.height
                    && (f32::from(terrain.at(x, y)) - surface.height).abs() < 0.001;
                if terrain.blocked([f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(surface.height)], own_terrain, direction) {
                    result[i] = 0;
                }
            }
        }

    /// Unbounded receiver traversal checks the optimized volume projection
    /// candidates against the same physical solids without reusing its bounds.
    fn terrain_volumes_reference(terrain: &SunTerrain, f: &Frame, direction: [f64; 3], result: &mut [u8]) {
        for (i, surface) in f.surfaces.iter().enumerate() {
            if result[i] == 0 || surface.kind == 0 { continue; }
            let x = (i % W) as i32 + f.cam_x;
            let y = (i / W) as i32 + f.cam_y;
            let own = surface.owner == 0 && x >= 0 && y >= 0 && x < terrain.width && y < terrain.height
                && (f32::from(terrain.at(x, y)) - surface.height).abs() < 0.001;
            let receiver = [f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(surface.height)];
            if super::super::terrain_volumes::any(&terrain.volumes, receiver, direction, f64::INFINITY, own) { result[i] = 0; }
        }
    }

    /// Exactly the raised-receiver pass of occlude, isolated so the paired
    /// hierarchy timing does not include unchanged forward-projected ground.
    fn raised_terrain_mask(terrain: &SunTerrain, f: &Frame, direction: [f32; 3],
        initial: &[u8], hierarchy: bool) -> Vec<u8> {
        let mut result = initial.to_vec();
        let direction = direction.map(f64::from);
        for (i, surface) in f.surfaces.iter().enumerate() {
            if result[i] == 0 || surface.kind == 0 || surface.height <= EPS as f32 { continue; }
            let x = (i % W) as i32 + f.cam_x;
            let y = (i / W) as i32 + f.cam_y;
            let own_terrain = surface.owner == 0 && x >= 0 && y >= 0 && x < terrain.width && y < terrain.height
                && (f32::from(terrain.at(x, y)) - surface.height).abs() < 0.001;
            let receiver = [f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(surface.height)];
            let blocked = if hierarchy { terrain.blocked(receiver, own_terrain, direction) }
                else { terrain.blocked_inner::<true>(receiver, own_terrain, direction) };
            if blocked { result[i] = 0; }
        }
        result
    }

    /// Benchmark actual painted world/prop/actor receiver masks, not synthetic
    /// boxes or two differently culled shadow algorithms. Zone geometry and
    /// the nine render snapshots are prepared before any measured work.
    #[test]
    #[ignore = "manual paired native/terrain Sun row timing; run alone with --nocapture"]
    fn prepared_sun_ground_rows_paired_real_scene_timing() {
        use crate::fx::Fx;
        use crate::input::Input;
        use crate::play::{Play, PlayOpts};
        use crate::world::ZoneId;
        use std::hint::black_box;
        use std::time::Instant;

        fn measure<T>(run: impl FnOnce() -> T) -> (T, f64) {
            let start = Instant::now();
            let output = black_box(run());
            (output, start.elapsed().as_secs_f64() * 1000.0)
        }
        fn stats(before: &[f64], after: &[f64]) -> serde_json::Value {
            fn one(samples: &[f64]) -> serde_json::Value {
                let mut sorted = samples.to_vec();
                sorted.sort_by(f64::total_cmp);
                serde_json::json!({"mean_ms": samples.iter().sum::<f64>() / samples.len() as f64,
                    "median_ms": sorted[sorted.len()/2], "p95_ms": sorted[sorted.len()*95/100]})
            }
            serde_json::json!({"before_rows": one(before), "prepared_rows": one(after),
                "prepared_over_before": after.iter().sum::<f64>() / before.iter().sum::<f64>()})
        }

        let mut play = Play::new(ZoneId::Bloomwild, &PlayOpts { god: true, peaceful: false });
        let mut fx = Fx::new(7);
        for _ in 0..90 {
            play.update(&Input::default(), &mut fx);
            fx.sounds.clear();
            fx.lights.clear();
            fx.tick();
        }
        let terrain = SunTerrain::new(&play.zone);
        assert!(!terrain.texels.is_empty());
        let mut cases = Vec::new();
        for (name, x, y) in [("entry", 258.0, 640.0), ("courtyard", 456.0, 536.0),
            ("temple", 1056.0, 464.0)] {
            play.player.respawn(x, y);
            play.cam.snap(x, y);
            for tick in [90, 630, 1170] {
                play.t = tick;
                let mut frame = Frame::new();
                play.render(&mut frame, &fx);
                assert!(frame.normal_sun && frame.surface_lighting);
                assert!(!frame.surface_casters.is_empty());
                cases.push((name, tick, frame));
            }
        }

        let mut report = Vec::new();
        for (name, tick, frame) in &cases {
            let sun = direction(*tick);
            let base = visibility(frame, sun);
            let mut native_before = Vec::new();
            let mut native_after = Vec::new();
            let mut native_pixel_before = Vec::new();
            let mut native_alpha_after = Vec::new();
            let mut terrain_before = Vec::new();
            let mut terrain_after = Vec::new();
            let mut eight_pixel_before = Vec::new();
            let mut coarse_after = Vec::new();
            // Alternate AB/BA within each component. Include the same mask
            // allocation/clone cost on both sides; exact comparisons happen
            // outside the timed sections. Warm both paths before sampling.
            for sample in 0..36 {
                let (before, after) = if sample % 2 == 0 {
                    (measure(|| native_before_ground_rows(frame, sun)), measure(|| visibility(frame, sun)))
                } else {
                    let after = measure(|| visibility(frame, sun));
                    (measure(|| native_before_ground_rows(frame, sun)), after)
                };
                assert_eq!(before.0, after.0, "native {name}, tick {tick}, pair {sample}");
                if sample >= 4 {
                    native_before.push(before.1);
                    native_after.push(after.1);
                }
                let (before, after) = if sample % 2 == 0 {
                    (measure(|| native_before_alpha_blocks(frame, sun)), measure(|| visibility(frame, sun)))
                } else {
                    let after = measure(|| visibility(frame, sun));
                    (measure(|| native_before_alpha_blocks(frame, sun)), after)
                };
                assert_eq!(before.0, after.0, "native alpha {name}, tick {tick}, pair {sample}");
                if sample >= 4 {
                    native_pixel_before.push(before.1);
                    native_alpha_after.push(after.1);
                }
                let run_before = || {
                    let mut result = base.clone();
                    terrain_before_ground_rows(&terrain, frame, sun, &mut result);
                    result
                };
                let run_after = || {
                    let mut result = base.clone();
                    terrain.occlude(frame, sun, &mut result);
                    result
                };
                let (before, after) = if sample % 2 == 0 {
                    (measure(run_before), measure(run_after))
                } else {
                    let after = measure(run_after);
                    (measure(run_before), after)
                };
                assert_eq!(before.0, after.0, "terrain {name}, tick {tick}, pair {sample}");
                if sample >= 4 {
                    terrain_before.push(before.1);
                    terrain_after.push(after.1);
                }
                let run_before = || raised_terrain_mask(&terrain, frame, sun, &base, false);
                let run_after = || raised_terrain_mask(&terrain, frame, sun, &base, true);
                let (before, after) = if sample % 2 == 0 {
                    (measure(run_before), measure(run_after))
                } else {
                    let after = measure(run_after);
                    (measure(run_before), after)
                };
                assert_eq!(before.0, after.0, "terrain hierarchy {name}, tick {tick}, pair {sample}");
                if sample >= 4 {
                    eight_pixel_before.push(before.1);
                    coarse_after.push(after.1);
                }
            }
            report.push(serde_json::json!({"scene": name, "tick": tick,
                "camera": [frame.cam_x, frame.cam_y], "pairs": native_before.len(),
                "native_casters": frame.surface_casters.len(),
                "raised_receivers": frame.surfaces.iter().filter(|s| s.kind != 0 && s.height > 0.0).count(),
                "native": stats(&native_before, &native_after), "terrain": stats(&terrain_before, &terrain_after),
                "native_alpha_blocks": stats(&native_pixel_before, &native_alpha_after),
                "raised_terrain_hierarchy": stats(&eight_pixel_before, &coarse_after)}));
        }
        println!("SUN_ROW_REUSE_TIMING {}", serde_json::to_string(&report).unwrap());
    }

    #[test]
    fn coarse_terrain_blocks_match_original_pixel_and_eight_pixel_rays() {
        use crate::world::{Zone, ZoneId};
        let mut zone = Zone::new(ZoneId::Bloomwild, 32, 24);
        for y in 0..128 { for x in 128..384 {
            zone.gset_plane_surface(x, y, [0.0, 0.0, 1.0], 32.0);
        } }
        for y in 128..256 { for x in 0..384 {
            zone.gset_plane_surface(x, y, [0.0, 1.0, 0.0], (260 - y) as f32);
        } }
        zone.gset_plane_surface(63, 173, [0.0, 0.0, 1.0], 0.0);
        for y in (262..380).step_by(3) { for x in (8..500).step_by(5) {
            zone.gset_plane_surface(x, y, [0.0, 1.0, 0.0], ((x * 3 + y * 5) % 110 + 1) as f32);
        } }
        let terrain = SunTerrain::new(&zone);
        for level in &terrain.coarse {
            assert!(level.planes.iter().any(|p| p.cap == 32), "must exercise {}px complete cap", level.size);
            assert!(level.planes.iter().any(|p| p.front == 260), "must exercise {}px complete front", level.size);
            let hole = (173 / level.size) as usize * level.columns + (63 / level.size) as usize;
            assert_eq!(level.planes[hole].cap, 0);
            assert_eq!(level.planes[hole].front, -1, "one transparent pixel prevents a coarse own-plane skip");
        }
        let directions = [direction(0).map(f64::from), direction(540).map(f64::from), direction(1620).map(f64::from),
            [0.7, -0.3, 0.8], [-0.7, 1.3, 0.8], [0.0, -0.5, 0.9], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
        let boundaries = [-4, 0, 7, 8, 31, 32, 63, 64, 127, 128, 255, 256, 383, 384, 511, 515];
        let mut state = 0xa317_6419_u32;
        let mut next = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            state
        };
        let mut shadows = 0;
        let mut own_rays = 0;
        for sample in 0..95_000 {
            let x = if sample % 4 == 0 { boundaries[next() as usize % boundaries.len()] }
                else { (next() % 524) as i32 - 6 };
            let y = if sample % 4 == 1 { boundaries[next() as usize % boundaries.len()] }
                else { (next() % 396) as i32 - 6 };
            let native_height = if x >= 0 && y >= 0 && x < terrain.width && y < terrain.height {
                f64::from(terrain.at(x, y))
            } else { 0.0 };
            let own = next() % 3 == 0 && native_height > 0.0;
            let height = if own { native_height }
                else { [0.0, 5.125, 31.5, 32.0, 32.5, 80.0, 132.5, 160.0][next() as usize % 8] };
            let offset = if sample % 5 == 0 { 0.0 } else { 0.5 };
            let receiver = [f64::from(x) + offset, f64::from(y) + offset, height];
            let direction = directions[next() as usize % directions.len()];
            let original = terrain.blocked_inner::<false>(receiver, own, direction);
            let eight_pixel = terrain.blocked_inner::<true>(receiver, own, direction);
            let coarse = terrain.blocked(receiver, own, direction);
            assert_eq!(eight_pixel, original, "8px receiver={receiver:?}, own={own}, direction={direction:?}");
            assert_eq!(coarse, original, "coarse receiver={receiver:?}, own={own}, direction={direction:?}");
            shadows += usize::from(original);
            own_rays += usize::from(own);
        }
        assert!(shadows > 1000 && own_rays > 1000, "exercise both real blockers and own-plane prefixes");
    }

    #[test]
    fn terrain_row_culls_and_plane_block_proofs_preserve_full_original_masks() {
        use crate::world::{Zone, ZoneId};
        let mut zone = Zone::new(ZoneId::Bloomwild, 32, 20);
        // Complete block-aligned caps/front sheets, interrupted edges, holes,
        // and an unrelated sparse height field exercise every proof outcome.
        for y in 16..48 { for x in 24..136 {
            zone.gset_plane_surface(x, y, [0.0, 0.0, 1.0], 14.0);
        } }
        for y in 64..112 { for x in 40..400 {
            zone.gset_plane_surface(x, y, [0.0, 1.0, 0.0], (144 - y) as f32);
        } }
        zone.gset_plane_surface(81, 82, [0.0, 0.0, 1.0], 0.0);
        for y in (138..290).step_by(9) { for x in (8..500).step_by(13) {
            zone.gset_plane_surface(x, y, [0.0, 1.0, 0.0], ((x + y * 3) % 25 + 1) as f32);
        } }
        let terrain = SunTerrain::new(&zone);
        assert!(terrain.block_planes.iter().any(|p| p.cap == 14));
        assert!(terrain.block_planes.iter().any(|p| p.front == 144));
        let hole = (82 / BLOCK) as usize * terrain.columns + (81 / BLOCK) as usize;
        assert_eq!(terrain.block_planes[hole].cap, 0);
        assert_eq!(terrain.block_planes[hole].front, -1, "a single empty native pixel prevents skipping its sheet");
        let directions = [direction(0), direction(540), direction(1620),
            [0.7, 1.3, 0.8], [0.7, -0.3, 0.8], [-0.7, 0.8, 0.8],
            [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
        let mut shadowed = 0;
        for (cam_x, cam_y) in [(0, 0), (-23, -17), (225, 91), (411, -34)] {
            let mut frame = Frame::new();
            frame.cam_x = cam_x;
            frame.cam_y = cam_y;
            for y in 0..H { for x in 0..W {
                let (wx, wy) = (x as i32 + cam_x, y as i32 + cam_y);
                if wx < 0 || wy < 0 || wx >= terrain.width || wy >= terrain.height { continue; }
                let actor = (wx + wy * 3) % 19 == 0;
                let height = if actor { 5.125 + (wx % 23) as f32 } else { f32::from(terrain.at(wx, wy)) };
                frame.surfaces[y * W + x] = Surface {
                    normal: if x % 3 == 0 { [0.0, 0.0, 1.0] } else { [0.6, -0.2, 0.77] },
                    height, owner: if actor { 17 } else { 0 },
                    kind: if (wx + wy) % 17 == 0 { 0 } else if actor { 3 } else if height > 0.0 { 2 } else { 1 },
                };
            } }
            let mut source = caster(&["###", "#.#", "###", "###", ".#.", "###"]);
            source.x = 139.25; source.y = 55.5; source.base = 108.0;
            source.foot = Arc::from(vec![[106.0, 109.0]; 3]);
            source.opacity = 0.7;
            frame.surface_casters.push(source);
            for d in directions {
                let mut initial = visibility_reference(&frame, d);
                for (i, value) in initial.iter_mut().enumerate() {
                    if i % 43 == 0 { *value = 0; }
                    else if i % 37 == 0 { *value = (*value).min(77); }
                }
                let expected = terrain_visibility_reference(&terrain, &frame, d, initial.clone());
                let mut actual = initial;
                terrain.occlude(&frame, d, &mut actual);
                assert_eq!(actual, expected, "camera=({cam_x},{cam_y}), direction={d:?}");
                shadowed += actual.iter().filter(|&&v| v == 0).count();
            }
        }
        assert!(shadowed > 1000, "compare actual occluded pixels, not only empty masks");
    }

    #[test]
    fn sparse_height_receivers_match_original_mask_for_pose_flip_hover_and_camera() {
        let mut f = Frame::new();
        for y in 0..72 { for x in 0..112 {
            let height = [0.0, 0.125, 7.875, 8.0, 8.125, 15.999, 20.25, 37.75][(x / 8 + y / 8) % 8];
            f.surfaces[y * W + x] = Surface { normal: [0.0, 0.0, 1.0], height,
                owner: if (x + y) % 6 == 0 { 7 } else { 0 }, kind: if height == 0.0 { 1 } else { 3 } };
        } }
        for (i, (x, y, base)) in [(8.25, 2.5, 22.0), (28.75, 16.125, 30.0),
            (-10.5, -10.25, 20.0), (95.125, -24.5, 10.0)].into_iter().enumerate() {
            let mut source = caster(&["#####"]);
            source.x = x; source.y = y; source.base = base;
            source.h = 36;
            source.pixels = Arc::from((0..180).map(|p| NormalPixel {
                n: [0, 127, 0], a: p % 5 == 0 || p % 5 == 4 || p / 5 % 5 == i,
                emit: false,
            }).collect::<Vec<_>>());
            source.foot = Arc::from((0..5).map(|p| [base - 3.0, base + p as f32 * 0.375]).collect::<Vec<_>>());
            source.flip = i % 2 == 1;
            source.min_height = if i == 2 { 4.3 } else { 0.0 };
            source.opacity = if i == 1 { 0.5 } else { 1.0 };
            f.surface_casters.push(source);
        }
        let mut changed = 0;
        for (cx, cy) in [(0, 0), (-7, 3), (14, 5)] {
            f.cam_x = cx; f.cam_y = cy;
            for tick in [0, 540, 1100, 1640] {
                let actual = visibility(&f, direction(tick));
                let expected = visibility_reference(&f, direction(tick));
                assert_eq!(actual, expected, "camera=({cx},{cy}), tick={tick}");
                changed += actual.iter().filter(|&&v| v < 255).count();
            }
        }
        assert!(changed > 100, "compare actual native shadow pixels across the height bands");
    }

    #[test]
    fn native_alpha_tiles_match_original_pixel_rays_for_mirrors_depth_and_partial_tiles() {
        let directions = [direction(0).map(f64::from), direction(540).map(f64::from), direction(1620).map(f64::from),
            [0.7, -0.3, 0.8], [-0.7, 1.3, 0.8], [0.0, -0.5, 0.9], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
        let mut state = 0x19b7_326d_u32;
        let mut next = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            state
        };
        let mut rays = 0;
        let mut hits = 0;
        for width in [5, 9, 17, 31, 64] { for flip in [false, true] { for volume in [false, true] {
            let mut source = caster(&["#"]);
            source.w = width;
            source.h = 49;
            source.x = 20.25;
            source.y = -20.75;
            source.base = 40.0;
            source.flip = flip;
            source.min_height = if flip { 3.125 } else { 0.0 };
            source.pixels = Arc::from((0..source.h).flat_map(|y| (0..width).map(move |x| NormalPixel {
                n: [0, 127, 0], a: (x % 19 < 3 && y % 23 < 5) || (x < width / 4 && y < 24)
                    || (y >= 46 && x >= width / 3 && x < width / 2), emit: false,
            })).collect::<Vec<_>>());
            source.foot = if volume {
                Arc::from((0..width).map(|x| [34.0 + (x % 3) as f32 * 0.5,
                    40.0 + (x % 5) as f32 * 0.375]).collect::<Vec<_>>())
            } else { Arc::from(Vec::new()) };
            let native = Native::new(&source).unwrap();
            if width >= 17 {
                assert!(native.alpha.as_ref().unwrap().occupied.contains(&0), "must traverse genuinely empty alpha tiles");
            }
            for sample in 0..4750 {
                let x = (next() % 180) as i32 - 36;
                let y = (next() % 160) as i32 - 36;
                let offset = if sample % 5 == 0 { 0.0 } else { 0.5 };
                let height = [0.0, 0.5, 3.125, 7.875, 16.0, 32.5, 48.0, 64.0][next() as usize % 8];
                let receiver = [f64::from(x) + offset, f64::from(y) + offset, height];
                let direction = directions[sample % directions.len()];
                let expected = native.blocks_reference(receiver, direction);
                assert_eq!(native.blocks(receiver, direction), expected,
                    "width={width}, flip={flip}, volume={volume}, receiver={receiver:?}, direction={direction:?}");
                rays += 1;
                hits += usize::from(expected);
            }
            // Parallel rays exactly on displayed integer X edges retain the
            // displayed half-open ownership after reflected UV/tile skips.
            for x in 0..width { for y in -8..64 {
                let receiver = [native.x0 + f64::from(x), f64::from(y), 0.0];
                let direction = [0.0, -0.5, 0.9];
                assert_eq!(native.blocks(receiver, direction), native.blocks_reference(receiver, direction),
                    "parallel X ownership width={width}, flip={flip}, volume={volume}, receiver={receiver:?}");
            } }
        } } }
        assert_eq!(rays, 95_000);
        assert!(hits > 100 && hits < rays - 1000, "compare both real hits and transparent gaps");
    }

    #[test]
    fn prepared_native_ground_rows_match_original_rays_and_contain_every_hit() {
        let mut hits = 0;
        let directions = [
            [-0.7, -0.3, 0.8], [0.7, -0.3, 0.8], [-0.7, 1.3, 0.8],
            [0.0, -0.5, 0.9], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
            [1.0e-13, -1.0e-13, 1.0], [0.0, 0.0, 1.0],
        ];
        for volume in [false, true] { for flip in [false, true] {
            let mut source = caster(&["##...", "#.#..", "####.", "#..##", "##.#.", "#####"]);
            source.x = 6.25;
            source.y = -1.125;
            source.base = 6.5;
            source.flip = flip;
            source.min_height = if flip { 1.125 } else { 0.0 };
            source.foot = if volume {
                Arc::from((0..5).map(|x| [4.125 + f32::from(x as u8) * 0.125,
                    6.5 + f32::from(x as u8) * 0.25]).collect::<Vec<_>>())
            } else { Arc::from(Vec::new()) };
            let native = Native::new(&source).unwrap();
            for direction in directions { for y in -12..30 {
                let world_y = f64::from(y) + 0.5;
                let prepared = native.ground_interval(world_y, direction);
                let span = prepared.and_then(|interval| native.ground_span(direction[0], interval, -7, [-30, 40]));
                for x in -30..=40 {
                    let receiver = [f64::from(x - 7) + 0.5, world_y, 0.0];
                    let expected = native.blocks_reference(receiver, direction);
                    let actual = prepared.is_some_and(|interval| native.blocks_ground(receiver, direction, interval));
                    assert_eq!(actual, expected,
                        "volume={volume}, flip={flip}, receiver={receiver:?}, direction={direction:?}");
                    if expected {
                        hits += 1;
                        let [lo, hi] = span.expect("the row's native inverse projection retains every full-ray hit");
                        assert!((lo..=hi).contains(&x), "span=({lo},{hi}), x={x}, direction={direction:?}");
                    }
                }
            } }
        } }
        assert!(hits > 100, "the oracle must cover genuine native volume and alpha-sheet hits");
    }

    #[test]
    fn sun_direction_is_slow_normalized_periodic_and_above_the_horizon() {
        let first = direction(0);
        assert_eq!(first, direction(2160));
        let mut max_step = 0.0_f32;
        for t in 0..2160 {
            let a = direction(t); let b = direction(t + 1);
            assert!((a.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 0.000_01);
            assert!(a[2] > 0.70);
            max_step = max_step.max(a.iter().zip(b).map(|(a,b)| (a-b).abs()).sum());
        }
        assert!(max_step < 0.002);
        assert!((first[0] - direction(540)[0]).abs() > 0.04);
    }

    #[test]
    fn native_depth_retains_foot_rows_and_transparent_alpha_holes() {
        let source = caster(&["###", "#.#", "#.#", "#.#", "###", "###"]);
        let c = Native::new(&source).unwrap();
        for y in [5.5, 6.5, 7.5] {
            assert!(c.blocks([12.5, y, 0.0], [-1.0, 0.0, 1.0]));
        }
        assert!(!c.blocks([12.5, 8.5, 0.0], [-1.0, 0.0, 1.0]));
        assert!(!c.blocks([9.5, 3.5, 0.0], [0.0, 1.0, 1.0]));
    }

    #[test]
    fn shadow_bounds_preserve_all_hits_at_ground_and_raised_heights() {
        let source = caster(&["###", "#.#", "###", "###", "###", "###"]);
        let c = Native::new(&source).unwrap();
        let f = Frame::new();
        let mut hits = 0;
        for d in [[-0.7, -0.3, 0.8], [0.7, 0.2, 0.8], [0.0, -0.5, 0.9]] {
            let rect = c.shadow_bounds(d, [0.0, 4.0], &f).unwrap();
            for y in 0..24 { for x in 0..24 { for h in [0.0, 1.0, 4.0] {
                if c.blocks([x as f64 + 0.5, y as f64 + 0.5 + h, h], d) {
                    hits += 1;
                    assert!((rect[0]..=rect[2]).contains(&x) && (rect[1]..=rect[3]).contains(&y));
                }
            } } }
        }
        assert!(hits > 40);
    }

    #[test]
    fn offscreen_native_caster_shadows_visible_pixels_and_owner_is_exempt() {
        let mut f = Frame::new();
        f.cam_x = 12;
        let mut source = caster(&["###", "###", "###", "###", "###", "###"]);
        source.opacity = 0.7;
        f.surface_casters.push(source);
        let i = 6 * W;
        f.surfaces[i] = Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 };
        assert_eq!(visibility(&f, [-1.0, 0.0, 1.0])[i], 77);
        f.surfaces[i].owner = 7;
        assert_eq!(visibility(&f, [-1.0, 0.0, 1.0])[i], 255);
    }

    #[test]
    fn mirrored_alpha_and_per_column_foot_depth_reflect_the_same_volume() {
        let mut source = caster(&["##.", "#..", "###", "#..", "##.", "###"]);
        source.foot = Arc::from(vec![[5.0, 6.0], [5.0, 7.0], [5.0, 8.0]]);
        let mut flipped = source.clone();
        flipped.flip = true;
        flipped.x += 2.0;
        let a = Native::new(&source).unwrap();
        let b = Native::new(&flipped).unwrap();
        for y in 0..20 { for x in 0..20 {
            let receiver = [x as f64 + 0.5, y as f64 + 0.5, 0.0];
            let reflected = [19.0 - receiver[0], receiver[1], receiver[2]];
            assert_eq!(a.blocks(receiver, [-0.7, -0.2, 0.8]), b.blocks(reflected, [0.7, -0.2, 0.8]));
        } }
    }

    #[test]
    fn tall_caster_reaches_the_view_from_far_offscreen_without_a_length_cap() {
        let mut f = Frame::new();
        let mut source = caster(&["#"]);
        source.h = 100;
        source.pixels = Arc::from(vec![NormalPixel { n: [0, 127, 0], a: true, emit: false }; 100]);
        source.x = -60.0; source.y = -99.0;
        source.foot = Arc::from(vec![[0.0, 2.0]]);
        f.surface_casters.push(source);
        let i = W + 20;
        f.surfaces[i] = Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 };
        assert_eq!(visibility(&f, [-1.0, 0.0, 1.0])[i], 0);
    }

    #[test]
    fn opening_stair_shadows_stay_shallow_through_the_sun_cycle() {
        let (zone, _) = crate::regions::build(crate::world::ZoneId::Bloomwild);
        let terrain = SunTerrain::new(&zone);
        let mut frame = Frame::new();
        frame.cam_x = 220;
        frame.cam_y = 660;
        for (i, surface) in frame.surfaces.iter_mut().enumerate() {
            let x = (i % W) as i32 + frame.cam_x;
            let y = (i / W) as i32 + frame.cam_y;
            if x >= zone.pw || y >= zone.ph { continue; }
            let j = (y * zone.pw + x) as usize;
            if zone.ground[j].r < 0.0 { continue; }
            let native = zone.ground_surfaces[j];
            *surface = Surface { normal: native.normal.map(|n| f32::from(n) / 127.0),
                height: f32::from(native.height), owner: 0, kind: 1 };
        }
        for tick in (0..2160).step_by(270) {
            let sun = direction(tick);
            let mut visible = vec![255; W * H];
            terrain.occlude(&frame, sun, &mut visible);
            let mut shadows = 0;
            for (i, &visibility) in visible.iter().enumerate() {
                if visibility != 0 { continue; }
                shadows += 1;
                let x = (i % W) as i32 + frame.cam_x;
                let y = (i / W) as i32 + frame.cam_y;
                assert!((282..382).contains(&x) && (694..732).contains(&y),
                    "hanging cliff must not cast an above-ground shadow at {x},{y}, tick {tick}");
            }
            assert!(shadows > 0, "real stair nosings still cast short shadows at tick {tick}");
            assert!(terrain.blocked([1000.5, 356.5, 0.0], false, sun.map(f64::from)),
                "the actual raised retaining wall still casts shade at tick {tick}");
        }
    }

    #[test]
    fn retaining_wall_side_casts_onto_the_actual_east_stair_and_keeps_the_passage_open() {
        let (zone, _) = crate::regions::build(crate::world::ZoneId::Bloomwild);
        let terrain = SunTerrain::new(&zone);
        assert_eq!(terrain.volumes.len(), 2, "the two real stair jambs retain masonry depth");
        let receiver = |x, y| [f64::from(x) + 0.5, f64::from(y) + 0.5,
            f64::from(zone.ground_surfaces[(y * zone.pw + x) as usize].height)];
        for tick in (0..2160).step_by(135) {
            let sun = direction(tick).map(f64::from);
            assert!(terrain.blocked(receiver(1112, 331), true, sun),
                "west cliff side must cast onto the adjacent ascending stair, tick {tick}");
            assert!(!terrain.blocked(receiver(1144, 331), true, sun),
                "the full open stair must not become a dark rectangle, tick {tick}");
            assert!(terrain.blocked(receiver(1096, 356), true, sun),
                "cliff-front ground shadow still joins the stair-side shadow, tick {tick}");
            for (cam_x, cam_y) in [(1000, 260), (1007, 271)] {
                let mut frame = Frame::new();
                frame.cam_x = cam_x;
                frame.cam_y = cam_y;
                for (i, surface) in frame.surfaces.iter_mut().enumerate() {
                    let x = (i % W) as i32 + cam_x;
                    let y = (i / W) as i32 + cam_y;
                    if x >= zone.pw || y >= zone.ph { continue; }
                    let j = (y * zone.pw + x) as usize;
                    if zone.ground[j].r < 0.0 { continue; }
                    *surface = zone.ground_surfaces[j].surface();
                }
                let mut actual = vec![255; W * H];
                terrain.occlude(&frame, sun.map(|v| v as f32), &mut actual);
                let at = |x, y| (y - cam_y) as usize * W + (x - cam_x) as usize;
                assert_eq!(actual[at(1112, 331)], 0, "the rendered stair receives side shade");
                assert_eq!(actual[at(1144, 331)], 255, "the open stair still receives sun");
                let mut expected = vec![255; W * H];
                terrain.occlude_volumes(&frame, sun, &mut expected);
                let mut oracle = vec![255; W * H];
                terrain_volumes_reference(&terrain, &frame, sun, &mut oracle);
                assert_eq!(expected, oracle, "bounded volume projection, camera {cam_x},{cam_y}, tick {tick}");
                assert!(actual.iter().zip(&frame.surfaces).all(|(&v, s)| s.kind != 0 || v == 255),
                    "terrain shadows cannot paint into sky");
            }
        }
    }

    #[test]
    fn terrain_uses_native_height_sheets_and_camera_pans_preserve_shadow() {
        use crate::world::{Tile, Zone, ZoneId};
        let mut zone = Zone::new(ZoneId::Bloomwild, 4, 4);
        zone.set_tile(1, 1, Tile::Wall);
        let direction = [-0.5, -0.25, 1.0];
        let receiver = [24.5, 30.5, 0.0];
        assert!(!SunTerrain::new(&zone).blocked(receiver, false, direction));
        zone.gset_plane_surface(20, 20, [0.0, 1.0, 0.0], 8.0);
        let terrain = SunTerrain::new(&zone);
        assert!(terrain.blocked(receiver, false, direction));
        assert!(!terrain.blocked([24.5, 30.5, 3.0], false, direction));
        let mut frame = Frame::new();
        frame.surfaces.fill(Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 });
        let mut first = vec![255; W * H];
        terrain.occlude(&frame, direction.map(|v| v as f32), &mut first);
        assert_eq!(first[30 * W + 24], 0);
        frame.cam_x = 7; frame.cam_y = 3;
        let mut panned = vec![255; W * H];
        terrain.occlude(&frame, direction.map(|v| v as f32), &mut panned);
        assert_eq!(panned[27 * W + 17], first[30 * W + 24]);
        zone.gset_plane_surface(20, 20, [0.0, 1.0, 0.0], 12.0);
        assert!(!SunTerrain::new(&zone).blocked(receiver, false, direction), "painted sheet is not promoted to solid column below its height");
    }

    #[test]
    fn terrain_own_contiguous_front_plane_is_exempt_but_actor_is_not() {
        use crate::world::{Zone, ZoneId};
        let mut zone = Zone::new(ZoneId::Bloomwild, 4, 4);
        for y in 12..30 { zone.gset_plane_surface(20, y, [0.0, 1.0, 0.0], (32 - y) as f32); }
        let terrain = SunTerrain::new(&zone);
        let receiver = [20.5, 20.5, 12.0];
        assert!(!terrain.blocked(receiver, true, [0.0, -0.4, 1.0]));
        assert!(terrain.blocked(receiver, false, [0.0, -0.4, 1.0]));
    }

    #[test]
    fn terrain_parallel_dda_matches_independent_all_sheet_slab_oracle() {
        use crate::world::{Zone, ZoneId};
        let mut zone = Zone::new(ZoneId::Bloomwild, 4, 4);
        for y in (8..52).step_by(7) { for x in (6..52).step_by(5) {
            zone.gset_plane_surface(x, y, [0.0, 1.0, 0.0], ((x * 3 + y * 5) % 17 + 1) as f32);
        } }
        let terrain = SunTerrain::new(&zone);
        let mut rays = 0;
        for direction in [
            [-0.7, -0.3, 0.8], [0.7, -0.3, 0.8], [-0.7, 1.3, 0.8],
            [0.0, -0.5, 0.9], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
        ] {
            let ray = [direction[0], direction[1] - direction[2], direction[2]];
            for offset in [0.0, 0.5] { for y in (0..56).step_by(7) { for x in (0..56).step_by(7) {
                for height in [0.0, 3.125, 11.5] {
                    let receiver = [f64::from(x) + offset, f64::from(y) + offset, height];
                    let oracle = terrain.texels.iter().any(|&texel| SunTerrain::texel_blocks(texel, receiver, ray));
                    assert_eq!(terrain.blocked(receiver, false, direction), oracle,
                        "receiver={receiver:?}, direction={direction:?}");
                    rays += 1;
                }
            } } }
        }
        assert_eq!(rays, 2304);
    }
}
