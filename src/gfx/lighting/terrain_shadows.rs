//! Source-local occlusion by the actual painted native terrain height sheets.
//! No collision tile is promoted into a blocker. A ray projects to a straight
//! line in (X,Y-Z); native pixel traversal checks its Z interval against the
//! retained source height. Empty blocks are skipped analytically, without a
//! ray-length/step cap. Static light/terrain reception is cached in world space.

use super::{Field, Frame, H, W};
use crate::gfx::frame::PointLight;
use crate::world::Zone;
use std::cell::RefCell;

const BLOCK: i32 = 8;
const EPS: f32 = 0.000_01;
// A Cinder viewport can contain more than forty recurring lava emitters. An
// ordered working set larger than either LRU would otherwise miss forever.
const CACHE_SIZE: usize = 64;
const RECENT_SIZE: usize = 128;
const CACHE_BYTES: usize = 8 * 1024 * 1024;
const CACHE_BLOCK: i32 = 32;
const UNKNOWN: u8 = 128;

pub(super) struct Geometry {
    cols: usize,
    blocks: Vec<u8>,
    occupied_prefix: Vec<u32>,
    coarse: Vec<HeightLevel>,
    cache: RefCell<Vec<Entry>>,
    recent: RefCell<Vec<[u32; 3]>>,
    #[cfg(test)]
    stats: RefCell<CacheStats>,
}

#[cfg(test)]
#[derive(Clone, Copy, Default)]
struct CacheStats {
    calls: usize,
    hits: usize,
    promotions: usize,
    first_seen: usize,
    native_computations: usize,
    native_hits: usize,
    live_computations: usize,
}

/// Upper terrain-height bounds over increasingly large aligned world blocks.
/// They are only a proof that a ray segment is empty, never replacement geometry.
struct HeightLevel {
    size: i32,
    cols: usize,
    maxima: Vec<u8>,
}

struct Entry {
    key: [u32; 3],
    x0: i32,
    y0: i32,
    w: usize,
    h: usize,
    mask: Vec<u8>,
    #[cfg(test)]
    evaluated: usize,
}

impl Geometry {
    pub(super) fn new(zone: &Zone) -> Self {
        let cols = ((zone.pw + BLOCK - 1) / BLOCK) as usize;
        let rows = ((zone.ph + BLOCK - 1) / BLOCK) as usize;
        let mut blocks = vec![0; cols * rows];
        for y in 0..zone.ph {
            for x in 0..zone.pw {
                let i = (y * zone.pw + x) as usize;
                let height = zone.ground_surfaces[i].height;
                if zone.ground[i].r >= 0.0 && height > 0 {
                    let b = (y / BLOCK) as usize * cols + (x / BLOCK) as usize;
                    blocks[b] = blocks[b].max(height);
                }
            }
        }
        let mut occupied_prefix = vec![0; (cols + 1) * (rows + 1)];
        for y in 0..rows {
            let mut row = 0;
            for x in 0..cols {
                row += u32::from(blocks[y * cols + x] != 0);
                occupied_prefix[(y + 1) * (cols + 1) + x + 1] = occupied_prefix[y * (cols + 1) + x + 1] + row;
            }
        }
        let coarse = [32, 128].into_iter().map(|size| {
            let level_cols = ((zone.pw + size - 1) / size) as usize;
            let level_rows = ((zone.ph + size - 1) / size) as usize;
            let mut maxima = vec![0; level_cols * level_rows];
            for by in 0..rows {
                for bx in 0..cols {
                    let i = (by as i32 * BLOCK / size) as usize * level_cols + (bx as i32 * BLOCK / size) as usize;
                    maxima[i] = maxima[i].max(blocks[by * cols + bx]);
                }
            }
            HeightLevel { size, cols: level_cols, maxima }
        }).collect();
        Self { cols, blocks, occupied_prefix, coarse, cache: RefCell::new(Vec::new()), recent: RefCell::new(Vec::new()),
            #[cfg(test)] stats: RefCell::new(CacheStats::default()) }
    }

    #[cfg(test)]
    fn occupied(&self, x: i32, y: i32) -> bool {
        self.blocks[(y / BLOCK) as usize * self.cols + (x / BLOCK) as usize] != 0
    }

    /// A source/receiver segment can only touch blocks in its projected AABB.
    /// The integral occupancy table proves empty rectangles without visiting
    /// their blocks; a positive result still uses the original exact DDA.
    fn occupied_rect(&self, zone: &Zone, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let x0 = x0.max(0);
        let y0 = y0.max(0);
        let x1 = x1.min(zone.pw - 1);
        let y1 = y1.min(zone.ph - 1);
        if x0 > x1 || y0 > y1 { return false; }
        let x0 = (x0 / BLOCK) as usize;
        let y0 = (y0 / BLOCK) as usize;
        let x1 = (x1 / BLOCK) as usize + 1;
        let y1 = (y1 / BLOCK) as usize + 1;
        let stride = self.cols + 1;
        self.occupied_prefix[y1 * stride + x1] + self.occupied_prefix[y0 * stride + x0]
            != self.occupied_prefix[y0 * stride + x1] + self.occupied_prefix[y1 * stride + x0]
    }

    fn any(&self, zone: &Zone, light: PointLight) -> bool {
        let ry = light.radius * std::f32::consts::SQRT_2;
        let x0 = ((light.x - light.radius).floor() as i32).max(0);
        let x1 = ((light.x + light.radius).ceil() as i32).min(zone.pw - 1);
        let y0 = ((light.y - ry).floor() as i32).max(0);
        let y1 = ((light.y + ry).ceil() as i32).min(zone.ph - 1);
        self.occupied_rect(zone, x0, y0, x1, y1)
    }
}

fn height(zone: &Zone, x: i32, y: i32) -> f32 {
    let i = (y * zone.pw + x) as usize;
    if zone.ground[i].r < 0.0 { 0.0 } else { zone.ground_surfaces[i].height as f32 }
}

fn same_front_plane(ry: f32, rh: f32, y: i32, h: f32) -> bool {
    // Constant-height caps and upright front sheets each have a distinct
    // invariant. Requiring the original invariant prevents drift over stairs.
    h > 0.0 && ((h - rh).abs() <= 0.51 || (y as f32 + 0.5 + h - ry - rh).abs() <= 0.51)
}

fn clip(origin: f64, delta: f64, limit: f64, interval: &mut (f64, f64)) -> bool {
    if delta == 0.0 { return origin >= 0.0 && origin < limit; }
    let a = -origin / delta;
    let b = (limit - origin) / delta;
    interval.0 = interval.0.max(a.min(b));
    interval.1 = interval.1.min(a.max(b));
    interval.0 < interval.1
}

fn crossing(cell: i32, origin: f64, delta: f64, step: i32, size: i32) -> f64 {
    if step == 0 { return f64::INFINITY; }
    let boundary = if step > 0 { (cell.div_euclid(size) + 1) * size } else { cell.div_euclid(size) * size };
    (boundary as f64 - origin) / delta
}

/// Pixel entered immediately after t. In particular a left/up ray at an exact
/// integer boundary belongs to the lower cell. Compare the same event times as
/// traversal rather than relying on rounded origin + delta*t to select a side.
fn cell_after(origin: f64, delta: f64, step: i32, t: f64) -> i32 {
    let mut cell = (origin + delta * t).floor() as i32;
    if step == 0 { return cell; }
    while crossing(cell, origin, delta, step, 1) <= t { cell += step; }
    while crossing(cell - step, origin, delta, step, 1) > t { cell -= step; }
    cell
}

/// Receiver -> source traversal. Only a terrain receiver's own contiguous
/// cap/front sheet is exempt; actors and separate sprite surfaces are not.
fn blocked(zone: &Zone, geometry: &Geometry, receiver: [f32; 3], own_terrain: bool, light: PointLight) -> bool {
    if !geometry.occupied_rect(zone,
        receiver[0].min(light.x).floor() as i32, receiver[1].min(light.y).floor() as i32,
        receiver[0].max(light.x).floor() as i32, receiver[1].max(light.y).floor() as i32) { return false; }
    let [rx, ry, rh] = receiver.map(f64::from);
    let dx = f64::from(light.x) - rx;
    let dy = f64::from(light.y) - ry;
    let dz = f64::from(light.height) - rh;
    let mut interval = (f64::from(EPS), 1.0 - f64::from(EPS));
    if !clip(rx, dx, zone.pw as f64, &mut interval) || !clip(ry, dy, zone.ph as f64, &mut interval) { return false; }
    let mut t = interval.0;
    let end = interval.1;
    let step_x = if dx > 0.0 { 1 } else if dx < 0.0 { -1 } else { 0 };
    let step_y = if dy > 0.0 { 1 } else if dy < 0.0 { -1 } else { 0 };
    let mut x = cell_after(rx, dx, step_x, t).clamp(0, zone.pw - 1);
    let mut y = cell_after(ry, dy, step_y, t).clamp(0, zone.ph - 1);
    let mut own_plane = own_terrain && rh > 0.0;
    loop {
        if x < 0 || y < 0 || x >= zone.pw || y >= zone.ph || t >= end { return false; }
        let block_height = geometry.blocks[(y / BLOCK) as usize * geometry.cols + (x / BLOCK) as usize];
        let occupied = block_height != 0;
        let mut size = if occupied { 1 } else { BLOCK };
        let mut tx = crossing(x, rx, dx, step_x, size);
        let mut ty = crossing(y, ry, dy, step_y, size);
        let mut next = tx.min(ty).min(end);
        let mut height_skip = false;
        if occupied && !own_plane && rh + dz * t > f64::from(block_height) + 0.5 {
            let bx = crossing(x, rx, dx, step_x, BLOCK);
            let by = crossing(y, ry, dy, step_y, BLOCK);
            let until = bx.min(by).min(end);
            // Every actual sheet in this block lies at or below its retained
            // maximum. A segment strictly above max+.5 cannot cross a sheet.
            // Do not use this for an own-plane prefix: the original per-pixel
            // traversal must first establish where that contiguous sheet ends.
            if (rh + dz * t).min(rh + dz * until) > f64::from(block_height) + 0.5 {
                size = BLOCK;
                tx = bx;
                ty = by;
                next = until;
                height_skip = true;
            }
        }
        if size == BLOCK {
            // Once an 8px block is provably clear, try wider blocks. A coarse
            // block with any possible intersection still falls back to the
            // exact fine traversal; no ray length or height is approximated.
            for level in &geometry.coarse {
                let maximum = level.maxima[(y / level.size) as usize * level.cols + (x / level.size) as usize];
                if maximum != 0 && (own_plane || rh + dz * t <= f64::from(maximum) + 0.5) { break; }
                let bx = crossing(x, rx, dx, step_x, level.size);
                let by = crossing(y, ry, dy, step_y, level.size);
                let until = bx.min(by).min(end);
                if maximum == 0 || (!own_plane && (rh + dz * t).min(rh + dz * until) > f64::from(maximum) + 0.5) {
                    size = level.size;
                    tx = bx;
                    ty = by;
                    next = until;
                    height_skip = maximum != 0;
                } else {
                    break;
                }
            }
        }
        #[cfg(test)]
        assert!(next > t, "terrain DDA must advance: receiver={receiver:?} light={light:?} cell=({x},{y}) size={size} t={t:?} next={next:?} tx={tx:?} ty={ty:?} end={end:?}");
        if occupied && !height_skip {
            let h = height(zone, x, y);
            let same = own_plane && same_front_plane(receiver[1], receiver[2], y, h);
            if !same {
                own_plane = false;
                if h > 0.0 {
                    let za = rh + dz * t;
                    let zb = rh + dz * next;
                    if za.min(zb) <= f64::from(h) + 0.5 && za.max(zb) >= f64::from(h) - 0.5 { return true; }
                }
            }
        } else {
            own_plane = false;
        }
        if next >= end { return false; }
        let boundary_x = if step_x > 0 { (x.div_euclid(size) + 1) * size } else { x.div_euclid(size) * size - 1 };
        let boundary_y = if step_y > 0 { (y.div_euclid(size) + 1) * size } else { y.div_euclid(size) * size - 1 };
        x = if tx <= next { boundary_x } else if size == 1 { x } else { cell_after(rx, dx, step_x, next) };
        y = if ty <= next { boundary_y } else if size == 1 { y } else { cell_after(ry, dy, step_y, next) };
        t = next;
    }
}

#[cfg(test)]
fn blocked_reference(zone: &Zone, geometry: &Geometry, receiver: [f32; 3], own_terrain: bool, light: PointLight) -> bool {
    if !geometry.occupied_rect(zone,
        receiver[0].min(light.x).floor() as i32, receiver[1].min(light.y).floor() as i32,
        receiver[0].max(light.x).floor() as i32, receiver[1].max(light.y).floor() as i32) { return false; }
    let [rx, ry, rh] = receiver.map(f64::from);
    let dx = f64::from(light.x) - rx;
    let dy = f64::from(light.y) - ry;
    let dz = f64::from(light.height) - rh;
    let mut interval = (f64::from(EPS), 1.0 - f64::from(EPS));
    if !clip(rx, dx, zone.pw as f64, &mut interval) || !clip(ry, dy, zone.ph as f64, &mut interval) { return false; }
    let mut t = interval.0;
    let end = interval.1;
    let step_x = if dx > 0.0 { 1 } else if dx < 0.0 { -1 } else { 0 };
    let step_y = if dy > 0.0 { 1 } else if dy < 0.0 { -1 } else { 0 };
    let mut x = cell_after(rx, dx, step_x, t).clamp(0, zone.pw - 1);
    let mut y = cell_after(ry, dy, step_y, t).clamp(0, zone.ph - 1);
    let mut own_plane = own_terrain && rh > 0.0;
    loop {
        if x < 0 || y < 0 || x >= zone.pw || y >= zone.ph || t >= end { return false; }
        let occupied = geometry.occupied(x, y);
        let size = if occupied { 1 } else { BLOCK };
        let tx = crossing(x, rx, dx, step_x, size);
        let ty = crossing(y, ry, dy, step_y, size);
        let next = tx.min(ty).min(end);
        #[cfg(test)]
        assert!(next > t, "terrain DDA must advance: receiver={receiver:?} light={light:?} cell=({x},{y}) size={size} t={t:?} next={next:?} tx={tx:?} ty={ty:?} end={end:?}");
        if occupied {
            let h = height(zone, x, y);
            let same = own_plane && same_front_plane(receiver[1], receiver[2], y, h);
            if !same {
                own_plane = false;
                if h > 0.0 {
                    let za = rh + dz * t;
                    let zb = rh + dz * next;
                    if za.min(zb) <= f64::from(h) + 0.5 && za.max(zb) >= f64::from(h) - 0.5 { return true; }
                }
            }
        } else {
            own_plane = false;
        }
        if next >= end { return false; }
        let boundary_x = if step_x > 0 { (x.div_euclid(size) + 1) * size } else { x.div_euclid(size) * size - 1 };
        let boundary_y = if step_y > 0 { (y.div_euclid(size) + 1) * size } else { y.div_euclid(size) * size - 1 };
        x = if tx <= next { boundary_x } else if size == 1 { x } else { cell_after(rx, dx, step_x, next) };
        y = if ty <= next { boundary_y } else if size == 1 { y } else { cell_after(ry, dy, step_y, next) };
        t = next;
    }
}

impl Entry {
    fn new(light: PointLight) -> Self {
        Self { key: key(light), x0: 0, y0: 0, w: 0, h: 0, mask: Vec::new(), #[cfg(test)] evaluated: 0 }
    }

    /// Allocate only visible world bounds. Unseen cells remain UNKNOWN, even
    /// inside this rectangle. Chunk-aligned growth amortizes slow camera pans.
    fn include(&mut self, zone: &Zone, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let mut x0 = (x0.max(0).div_euclid(CACHE_BLOCK) * CACHE_BLOCK).min(zone.pw);
        let mut y0 = (y0.max(0).div_euclid(CACHE_BLOCK) * CACHE_BLOCK).min(zone.ph);
        let mut x1 = (((x1 + 1).max(0) + CACHE_BLOCK - 1).div_euclid(CACHE_BLOCK) * CACHE_BLOCK).min(zone.pw);
        let mut y1 = (((y1 + 1).max(0) + CACHE_BLOCK - 1).div_euclid(CACHE_BLOCK) * CACHE_BLOCK).min(zone.ph);
        if x0 >= x1 || y0 >= y1 { return true; }
        if self.w != 0 {
            if x0 >= self.x0 && y0 >= self.y0 && x1 <= self.x0 + self.w as i32 && y1 <= self.y0 + self.h as i32 { return true; }
            x0 = x0.min(self.x0);
            y0 = y0.min(self.y0);
            x1 = x1.max(self.x0 + self.w as i32);
            y1 = y1.max(self.y0 + self.h as i32);
        }
        let w = (x1 - x0) as usize;
        let h = (y1 - y0) as usize;
        // Oversize masks fall back to the same exact uncached rays. Bound even
        // a future enormous-radius source before attempting its allocation.
        let Some(len) = w.checked_mul(h).filter(|&len| len <= CACHE_BYTES) else { return false };
        let mut mask = vec![UNKNOWN; len];
        for y in 0..self.h {
            let to = (y + (self.y0 - y0) as usize) * w + (self.x0 - x0) as usize;
            mask[to..to + self.w].copy_from_slice(&self.mask[y * self.w..(y + 1) * self.w]);
        }
        self.x0 = x0;
        self.y0 = y0;
        self.w = w;
        self.h = h;
        self.mask = mask;
        true
    }

    fn at(&mut self, x: i32, y: i32, compute: impl FnOnce() -> bool) -> bool {
        let i = (y - self.y0) as usize * self.w + (x - self.x0) as usize;
        if self.mask[i] == UNKNOWN {
            self.mask[i] = if compute() { 0 } else { 255 };
            #[cfg(test)] { self.evaluated += 1; }
        }
        self.mask[i] == 0
    }
}

fn retain_entry(cache: &mut Vec<Entry>, entry: Entry) {
    cache.push(entry);
    let mut bytes = cache.iter().map(|entry| entry.mask.len()).sum::<usize>();
    while cache.len() > CACHE_SIZE || bytes > CACHE_BYTES {
        bytes -= cache.remove(0).mask.len();
    }
}

fn key(light: PointLight) -> [u32; 3] {
    // Radius determines which receivers are reached, not whether their exact
    // source ray is blocked. Reuse valid rays when a source's radius flickers.
    [light.x.to_bits(), light.y.to_bits(), light.height.to_bits()]
}
fn reached([x, y, height]: [f32; 3], light: PointLight) -> bool {
    let dx = light.x - x;
    let dy = light.y + light.height - y - height;
    let dz = light.height - height;
    dx * dx + dy * dy + dz * dz < light.radius * light.radius
}

/// Combine into the existing per-source sprite visibility buffer. Color and
/// intensity affect radiance, never terrain visibility or its cache identity.
pub(super) fn occlude(zone: &Zone, field: &Field, f: &Frame, light: PointLight, result: &mut [u8]) {
    if light.radius <= 0.0 || light.intensity <= 0.0 || !field.terrain_shadow.any(zone, light) { return; }
    let x0 = ((light.x - f.cam_x as f32 - light.radius).floor() as i32).max(0);
    let x1 = ((light.x - f.cam_x as f32 + light.radius).ceil() as i32).min(W as i32 - 1);
    let ry = light.radius * std::f32::consts::SQRT_2;
    let y0 = ((light.y - f.cam_y as f32 - ry).floor() as i32).max(0);
    let y1 = ((light.y - f.cam_y as f32 + ry).ceil() as i32).min(H as i32 - 1);
    if x0 > x1 || y0 > y1 { return; }
    #[cfg(test)]
    let mut stats = CacheStats { calls: 1, ..CacheStats::default() };
    let mut cache = field.terrain_shadow.cache.borrow_mut();
    let wanted = key(light);
    let found = cache.iter().position(|entry| entry.key == wanted);
    let mut entry = if let Some(i) = found {
        #[cfg(test)] { stats.hits = 1; }
        Some(cache.remove(i))
    } else {
        let mut recent = field.terrain_shadow.recent.borrow_mut();
        if let Some(i) = recent.iter().position(|&k| k == wanted) {
            #[cfg(test)] { stats.promotions = 1; }
            recent.remove(i);
            Some(Entry::new(light))
        } else {
            #[cfg(test)] { stats.first_seen = 1; }
            // Continuously moving sources never allocate a world mask and
            // never evict stable light caches. Their visible rays remain exact.
            recent.push(wanted);
            if recent.len() > RECENT_SIZE { recent.remove(0); }
            None
        }
    };
    if entry.as_mut().is_some_and(|entry|
        !entry.include(zone, x0 + f.cam_x, y0 + f.cam_y, x1 + f.cam_x, y1 + f.cam_y)) {
        entry = None;
    }
    for y in y0..=y1 {
        for x in x0..=x1 {
            let i = y as usize * W + x as usize;
            let surface = f.surfaces[i];
            if result[i] == 0 || surface.kind == 0 { continue; }
            let wx = x + f.cam_x;
            let wy = y + f.cam_y;
            let receiver = [wx as f32 + 0.5, wy as f32 + 0.5, surface.height];
            if !reached(receiver, light) { continue; }
            let native_height = field.index(wx, wy).map(|j| field.terrain[j].height as f32);
            let shadow = if surface.owner == 0 && native_height == Some(surface.height) {
                let compute = || {
                    // The original native-terrain cache treats unpainted
                    // ground as absent even if a caller supplied a receiver.
                    let j = (wy * zone.pw + wx) as usize;
                    zone.ground[j].r >= 0.0 && blocked(zone, &field.terrain_shadow, receiver, true, light)
                };
                if let Some(entry) = entry.as_mut() {
                    #[cfg(test)] let before = entry.evaluated;
                    let shadow = entry.at(wx, wy, compute);
                    #[cfg(test)] {
                        stats.native_computations += entry.evaluated - before;
                        stats.native_hits += usize::from(entry.evaluated == before);
                    }
                    shadow
                } else {
                    #[cfg(test)] { stats.native_computations += 1; }
                    compute()
                }
            } else {
                #[cfg(test)] { stats.live_computations += 1; }
                blocked(zone, &field.terrain_shadow, receiver, surface.owner == 0, light)
            };
            if shadow { result[i] = 0; }
        }
    }
    if let Some(entry) = entry {
        retain_entry(&mut cache, entry);
    }
    #[cfg(test)] {
        let mut total = field.terrain_shadow.stats.borrow_mut();
        total.calls += stats.calls;
        total.hits += stats.hits;
        total.promotions += stats.promotions;
        total.first_seen += stats.first_seen;
        total.native_computations += stats.native_computations;
        total.native_hits += stats.native_hits;
        total.live_computations += stats.live_computations;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::Rgb;
    use crate::world::{Tile, ZoneId};

    fn light(x: f32, y: f32, height: f32) -> PointLight {
        PointLight { x, y, height, radius: 80.0, color: Rgb::WHITE, intensity: 1.0 }
    }

    #[test]
    fn exact_native_height_blocks_without_promoting_collision_tiles() {
        let mut zone = Zone::new(ZoneId::Glasswater, 4, 4);
        zone.set_tile(1, 1, Tile::Wall);
        let clear = Geometry::new(&zone);
        let source = light(10.5, 20.5, 16.0);
        assert!(!blocked(&zone, &clear, [30.5, 20.5, 0.0], false, source));
        zone.gset_plane_surface(20, 20, [0.0, 1.0, 0.0], 8.0);
        let geometry = Geometry::new(&zone);
        assert!(blocked(&zone, &geometry, [30.5, 20.5, 0.0], false, source));
        assert!(!blocked(&zone, &geometry, [30.5, 21.5, 0.0], false, light(10.5, 21.5, 16.0)), "adjacent transparent native row stays open");
        zone.gset_plane_surface(20, 20, [0.0, 1.0, 0.0], 12.0);
        let geometry = Geometry::new(&zone);
        assert!(!blocked(&zone, &geometry, [30.5, 20.5, 0.0], false, source), "ray must cross actual native height, not a column from zero");
    }

    #[test]
    fn own_front_plane_is_exempt_but_actor_and_separate_sheet_are_not() {
        let mut zone = Zone::new(ZoneId::Glasswater, 4, 4);
        for y in 12..30 { zone.gset_plane_surface(20, y, [0.0, 1.0, 0.0], (32 - y) as f32); }
        let geometry = Geometry::new(&zone);
        let receiver = [20.5, 20.5, 12.0];
        let source = light(20.5, 10.5, 22.0);
        assert!(!blocked(&zone, &geometry, receiver, true, source));
        assert!(blocked(&zone, &geometry, receiver, false, source), "actors cannot claim a terrain face as their own");
    }

    #[test]
    fn negative_boundary_entry_and_mirrored_rays_always_advance() {
        assert_eq!(cell_after(36.5, -26.0, -1, 0.75), 16);
        assert_eq!(cell_after(27.5, 26.0, 1, 0.75), 47);
        // Entering an occupied block through Y also crosses an internal pixel
        // boundary in X. The former floor-only reconstruction selected cell17
        // twice at t=.75 and never advanced. Transpose and reflect the geometry
        // to cover both negative axes and both directions of block traversal.
        for flip_x in [false, true] {
            for flip_y in [false, true] {
                for transpose in [false, true] {
                    let mut zone = Zone::new(ZoneId::Glasswater, 4, 4);
                    let p = |x: f32, y: f32| {
                        let (x, y) = if transpose { (y, x) } else { (x, y) };
                        [if flip_x { 64.0 - x } else { x }, if flip_y { 64.0 - y } else { y }]
                    };
                    let h = p(20.5, 20.5);
                    zone.gset_plane_surface(h[0].floor() as i32, h[1].floor() as i32, [0.0, 1.0, 0.0], 8.0);
                    let geometry = Geometry::new(&zone);
                    let r = p(36.5, 2.5);
                    let l = p(10.5, 20.5);
                    assert!(!blocked(&zone, &geometry, [r[0], r[1], 0.0], false, light(l[0], l[1], 16.0)));
                }
            }
        }
    }

    #[test]
    fn cached_terrain_visibility_is_world_anchored_but_raised_actors_are_live() {
        let mut zone = Zone::new(ZoneId::Glasswater, 4, 4);
        zone.gset_plane_surface(20, 20, [0.0, 1.0, 0.0], 8.0);
        let field = Field::bake(&zone, &crate::world::NoRegion, super::super::geometry_key(&zone));
        let mut frame = Frame::new();
        frame.surfaces.fill(crate::gfx::frame::Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 });
        let source = light(10.5, 20.5, 16.0);
        let mut first = vec![255; W * H];
        occlude(&zone, &field, &frame, source, &mut first);
        assert_eq!(first[20 * W + 30], 0);
        frame.cam_x = 7;
        frame.cam_y = 3;
        let mut recolored = source;
        recolored.color = Rgb::new(1.0, 0.2, 0.1);
        recolored.intensity = 0.23;
        let mut panned = vec![255; W * H];
        occlude(&zone, &field, &frame, recolored, &mut panned);
        let i = 17 * W + 23;
        assert_eq!(panned[i], first[20 * W + 30]);
        assert_eq!(field.terrain_shadow.cache.borrow().len(), 1, "color/intensity do not change geometric visibility");
        frame.surfaces[i] = crate::gfx::frame::Surface { normal: [0.0, 1.0, 0.0], height: 10.0, owner: 9, kind: 3 };
        let mut actor = vec![255; W * H];
        occlude(&zone, &field, &frame, source, &mut actor);
        assert_eq!(actor[i], 255, "raised actor is ray-tested at its current height, never given the cached floor shadow");
    }

    #[test]
    fn lazy_cache_only_evaluates_visible_receivers_and_reuses_flickering_radius() {
        let mut zone = Zone::new(ZoneId::Glasswater, 4, 4);
        zone.gset_plane_surface(20, 20, [0.0, 1.0, 0.0], 8.0);
        let field = Field::bake(&zone, &crate::world::NoRegion, super::super::geometry_key(&zone));
        let mut frame = Frame::new();
        let i = 20 * W + 30;
        frame.surfaces[i] = crate::gfx::frame::Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 };
        let source = light(10.5, 20.5, 16.0);
        for attempt in 0..3 {
            let mut result = vec![255; W * H];
            occlude(&zone, &field, &frame, source, &mut result);
            assert_eq!(result[i], 0);
            let cache = field.terrain_shadow.cache.borrow();
            if attempt == 0 {
                assert!(cache.is_empty(), "a one-frame moving source must not allocate a world mask");
            } else {
                assert_eq!(cache[0].evaluated, 1, "only the one actual visible receiver is ray-tested, including on a warm-cache repeat");
                assert_eq!(cache[0].mask.iter().filter(|&&v| v != UNKNOWN).count(), 1);
            }
        }
        let mut flicker = source;
        flicker.radius = 103.625;
        let mut result = vec![255; W * H];
        occlude(&zone, &field, &frame, flicker, &mut result);
        assert_eq!(result[i], 0);
        {
            let cache = field.terrain_shadow.cache.borrow();
            assert_eq!(cache.len(), 1, "changing radius does not change an already evaluated ray");
            assert_eq!(cache[0].evaluated, 1);
        }
        for n in 1..=RECENT_SIZE + 4 {
            let mut moving = source;
            moving.x += n as f32 / 17.0;
            occlude(&zone, &field, &frame, moving, &mut vec![255; W * H]);
        }
        let cache = field.terrain_shadow.cache.borrow();
        assert_eq!(cache.len(), 1, "never-repeated moving positions cannot evict a useful stable source");
        assert_eq!(cache[0].key, key(source));
        assert!(field.terrain_shadow.recent.borrow().len() <= RECENT_SIZE);
    }

    #[test]
    fn recurring_forty_seven_light_working_set_promotes_and_reuses_every_native_ray() {
        let mut zone = Zone::new(ZoneId::Cinder, 4, 4);
        zone.gset_plane_surface(20, 20, [0.0, 1.0, 0.0], 8.0);
        let field = Field::bake(&zone, &crate::world::NoRegion, super::super::geometry_key(&zone));
        let mut frame = Frame::new();
        let i = 20 * W + 30;
        frame.surfaces[i] = crate::gfx::frame::Surface {
            normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1,
        };
        let sources: Vec<_> = (0..47).map(|n| light(10.5 + n as f32 / 97.0, 20.5, 16.0)).collect();
        let receiver = [30.5, 20.5, 0.0];
        let expected: Vec<_> = sources.iter().map(|&source|
            if blocked_reference(&zone, &field.terrain_shadow, receiver, true, source) { 0 } else { 255 }).collect();
        assert!(expected.contains(&0), "exercise actual native wall occlusion");
        for frame_number in 0..6 {
            for (&source, &expected) in sources.iter().zip(&expected) {
                let mut result = vec![255; W * H];
                occlude(&zone, &field, &frame, source, &mut result);
                assert_eq!(result[i], expected);
            }
            let cache = field.terrain_shadow.cache.borrow();
            if frame_number == 0 {
                assert!(cache.is_empty(), "first sightings still avoid allocating moving-source masks");
                assert_eq!(field.terrain_shadow.recent.borrow().len(), sources.len());
            } else {
                assert_eq!(cache.len(), sources.len(), "the whole ordered viewport working set must survive");
                assert_eq!(cache.iter().map(|entry| entry.evaluated).sum::<usize>(), sources.len(),
                    "each source evaluates its native receiver exactly once after promotion");
                for &source in &sources {
                    assert!(cache.iter().any(|entry| entry.key == key(source)));
                }
            }
        }
        let stats = field.terrain_shadow.stats.borrow();
        assert_eq!(stats.promotions, sources.len());
        assert_eq!(stats.hits, sources.len() * 4);
        assert_eq!(stats.native_computations, sources.len() * 2,
            "only first sighting and first cached sighting compute rays");
        assert_eq!(stats.native_hits, sources.len() * 4);
    }

    #[test]
    fn stable_cache_has_an_explicit_byte_budget_in_addition_to_its_entry_limit() {
        let mut cache = Vec::new();
        let bytes = CACHE_BYTES / 2 + 1;
        for n in 0..3 {
            let mut entry = Entry::new(light(n as f32, 0.0, 1.0));
            entry.mask = vec![UNKNOWN; bytes];
            retain_entry(&mut cache, entry);
            assert!(cache.iter().map(|entry| entry.mask.len()).sum::<usize>() <= CACHE_BYTES);
            assert_eq!(cache.len(), 1, "oldest oversize working-set entry is evicted before exceeding the retained budget");
            assert_eq!(cache[0].key, key(light(n as f32, 0.0, 1.0)));
        }
    }

    /// Measure real, moving lava sources and the native/live receiver mix, with
    /// every gameplay light retained. Run alone so CPU contention cannot bias it.
    #[test]
    #[ignore = "explicit isolated release-mode Cinder terrain-cache diagnostic"]
    fn actual_cinder_terrain_cache_working_set_diagnostic() {
        use crate::fx::Fx;
        use crate::input::Input;
        use crate::play::{Play, PlayOpts};
        use std::hint::black_box;
        use std::time::Instant;
        let mut cases = Vec::new();
        for (name, (x, y)) in [("foundry", (626.0, 449.0)), ("caldera", (1144.0, 280.0))] {
            let mut play = Play::new(ZoneId::Cinder, &PlayOpts { god: true, peaceful: false });
            play.player.respawn(x, y);
            play.cam.snap(x, y);
            let mut fx = Fx::new(7);
            let mut frame = Frame::new();
            let hud = crate::ui::Hud::new();
            let mut samples = Vec::new();
            let mut totals = CacheStats::default();
            let (mut peak_entries, mut peak_bytes, mut peak_probation, mut peak_points) = (0, 0, 0, 0);
            let mut snapshots = Vec::new();
            for tick in 1..=180 {
                play.update(&Input::default(), &mut fx);
                let start = Instant::now();
                play.render(&mut frame, &fx);
                crate::ui::hud(&mut frame, &play, &hud);
                crate::gfx::post::finish(&mut frame, &play.zone.grade);
                black_box(&frame.out);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let field = play.zone.lighting.borrow();
                let geometry = &field.as_ref().unwrap().terrain_shadow;
                let stats = std::mem::take(&mut *geometry.stats.borrow_mut());
                let cache = geometry.cache.borrow();
                let bytes = cache.iter().map(|entry| entry.mask.len()).sum::<usize>();
                peak_entries = peak_entries.max(cache.len());
                peak_bytes = peak_bytes.max(bytes);
                peak_probation = peak_probation.max(geometry.recent.borrow().len());
                peak_points = peak_points.max(frame.point_lights.len());
                if tick > 60 {
                    samples.push(elapsed);
                    totals.calls += stats.calls;
                    totals.hits += stats.hits;
                    totals.promotions += stats.promotions;
                    totals.first_seen += stats.first_seen;
                    totals.native_computations += stats.native_computations;
                    totals.native_hits += stats.native_hits;
                    totals.live_computations += stats.live_computations;
                }
                if tick % 30 == 0 {
                    snapshots.push(serde_json::json!({"tick": tick, "points": frame.point_lights.len(),
                        "cache_entries": cache.len(), "cache_bytes": bytes,
                        "calls": stats.calls, "hits": stats.hits, "promotions": stats.promotions,
                        "first_seen": stats.first_seen, "native_computations": stats.native_computations,
                        "native_hits": stats.native_hits, "live_computations": stats.live_computations}));
                }
                drop(cache);
                drop(field);
                fx.sounds.clear();
                fx.lights.clear();
                fx.tick();
            }
            samples.sort_by(f64::total_cmp);
            cases.push(serde_json::json!({"case": name,
                "retained_mask_budget_bytes": CACHE_BYTES, "entry_limit": CACHE_SIZE,
                "probation_limit": RECENT_SIZE, "peak_entries": peak_entries,
                "peak_mask_bytes": peak_bytes, "peak_probation": peak_probation,
                "peak_points": peak_points, "samples": samples.len(),
                "mean_ms": samples.iter().sum::<f64>() / samples.len() as f64,
                "median_ms": samples[samples.len() / 2], "p95_ms": samples[samples.len() * 95 / 100],
                "calls": totals.calls, "cache_hits": totals.hits, "promotions": totals.promotions,
                "first_seen": totals.first_seen, "native_computations": totals.native_computations,
                "native_hits": totals.native_hits, "live_computations": totals.live_computations,
                "snapshots": snapshots}));
        }
        println!("{}", serde_json::to_string_pretty(&cases).unwrap());
    }

    #[test]
    fn lazy_cache_matches_uncached_native_rays_across_camera_radius_and_actor_changes() {
        let mut zone = Zone::new(ZoneId::Glasswater, 8, 8);
        for y in 20..100 {
            zone.gset_plane_surface(48, y, [0.0, 1.0, 0.0], (11 + y % 9) as f32);
        }
        for x in 24..96 {
            zone.gset_plane_surface(x, 64, [0.0, 1.0, 0.0], (4 + x % 7) as f32);
        }
        let field = Field::bake(&zone, &crate::world::NoRegion, super::super::geometry_key(&zone));
        for (cam_x, cam_y) in [(-45, -21), (13, 9), (49, 31), (-16, 41), (0, 0)] {
            for radius in [35.25, 88.125, 117.5] {
                let mut source = light(34.125, 42.875, 23.5);
                source.radius = radius;
                let mut frame = Frame::new();
                frame.cam_x = cam_x;
                frame.cam_y = cam_y;
                let mut seed = vec![255; W * H];
                for y in 0..H {
                    for x in 0..W {
                        let i = y * W + x;
                        let (wx, wy) = (x as i32 + cam_x, y as i32 + cam_y);
                        let Some(j) = field.index(wx, wy) else { continue; };
                        if (wx + wy * 3) % 13 == 0 { continue; }
                        let actor = (wx * 3 + wy) % 19 == 0;
                        frame.surfaces[i] = crate::gfx::frame::Surface {
                            normal: [0.0, 0.0, 1.0],
                            height: if actor { 7.75 + (wx % 4) as f32 } else { field.terrain[j].height as f32 },
                            owner: if actor { 6 } else { 0 }, kind: if actor { 3 } else { 1 },
                        };
                        if (wx + wy) % 17 == 0 { seed[i] = 0; }
                    }
                }
                // This oracle visits every screen receiver directly, with no
                // mask or camera/radius cache. It mirrors the previous eager
                // cache's absent-native-ground rule and live actor behavior.
                let mut expected = seed.clone();
                for y in 0..H {
                    for x in 0..W {
                        let i = y * W + x;
                        let surface = frame.surfaces[i];
                        if expected[i] == 0 || surface.kind == 0 { continue; }
                        let (wx, wy) = (x as i32 + cam_x, y as i32 + cam_y);
                        let receiver = [wx as f32 + 0.5, wy as f32 + 0.5, surface.height];
                        if !reached(receiver, source) { continue; }
                        let native = field.index(wx, wy).map(|j| (j, field.terrain[j].height as f32));
                        let absent_native = surface.owner == 0 && native.is_some_and(|(j, h)| h == surface.height && zone.ground[j].r < 0.0);
                        if !absent_native && blocked_reference(&zone, &field.terrain_shadow, receiver, surface.owner == 0, source) { expected[i] = 0; }
                    }
                }
                let mut actual = seed;
                occlude(&zone, &field, &frame, source, &mut actual);
                assert_eq!(actual, expected, "camera=({cam_x},{cam_y}) radius={radius}");
            }
        }
    }

    #[test]
    fn height_bound_skips_match_original_native_dda_including_own_plane_prefixes() {
        let mut zone = Zone::new(ZoneId::Glasswater, 8, 8);
        for y in 0..zone.ph {
            for x in 0..zone.pw {
                let h = if (x / 11 + y / 13) % 5 == 0 { 0 }
                    else if x < 48 { 12 }
                    else if y < 72 { 96 - y }
                    else { 1 + (x * 17 + y * 23) % 63 };
                if h > 0 { zone.gset_plane_surface(x, y, [0.0, 1.0, 0.0], h as f32); }
            }
        }
        let geometry = Geometry::new(&zone);
        let mut state = 0x8c44_7a59u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for case in 0..12_000 {
            let x = next() as i32 & 127;
            let y = next() as i32 & 127;
            let own = case % 3 == 0;
            let receiver_height = if own { height(&zone, x, y) } else { (next() % 641) as f32 * 0.25 };
            let receiver = [x as f32 + 0.5, y as f32 + 0.5, receiver_height];
            let source = light((next() % 609) as f32 * 0.25 - 12.0,
                (next() % 609) as f32 * 0.25 - 12.0, (next() % 641) as f32 * 0.25);
            assert_eq!(blocked(&zone, &geometry, receiver, own, source),
                blocked_reference(&zone, &geometry, receiver, own, source),
                "case={case} receiver={receiver:?} own={own} source={source:?}");
        }
    }

    #[test]
    fn authored_summit_high_beam_full_masks_match_original_dda_with_native_actors() {
        let (zone, region) = crate::regions::build(ZoneId::Summit);
        let (bx, by) = zone.boss_pos.unwrap();
        let field = Field::bake(&zone, region.as_ref(), super::super::geometry_key(&zone));
        let player = crate::entities::player::Player::new(bx + 44.375, by + 72.625);
        let boss = crate::entities::boss::Boss::new(bx, by);
        let mut saw_raised_native_actor = false;
        let mut saw_shadow = false;
        for (cam_x, cam_y) in [(528, 244), (461, 221), (637, 308)] {
            let mut frame = Frame::new();
            frame.cam_x = cam_x;
            frame.cam_y = cam_y;
            frame.begin(zone.bg, zone.ambient);
            frame.surface_lighting = true;
            zone.draw_ground(&mut frame);
            super::super::ground_surface(&zone, region.as_ref(), &mut frame);
            player.draw(&mut frame);
            boss.draw(&mut frame);
            saw_raised_native_actor |= frame.surfaces.iter().any(|s| s.owner != 0 && s.height > 16.0);
            // Shoulder-height beam sources include the real 112px altitude and
            // 152px radius, source motion, offscreen approaches, and lower
            // heart/fist-height lighting. All actors retain their native poses.
            for (x, y, h) in [(bx - 37.5, by - 105.5, 112.0), (bx + 37.5, by - 105.5, 112.0),
                (bx + 108.125, by - 35.875, 112.0), (bx - 104.875, by + 81.125, 112.0),
                (bx + 0.5, by + 1.25, 36.0), (bx - 52.25, by + 42.75, 20.0)] {
                let mut source = light(x, y, h);
                source.radius = 152.0;
                let mut seed = vec![255; W * H];
                for i in (0..W * H).step_by(41) { seed[i] = 0; }
                let mut expected = seed.clone();
                for y in 0..H {
                    for x in 0..W {
                        let i = y * W + x;
                        let surface = frame.surfaces[i];
                        if expected[i] == 0 || surface.kind == 0 { continue; }
                        let (wx, wy) = (x as i32 + cam_x, y as i32 + cam_y);
                        let receiver = [wx as f32 + 0.5, wy as f32 + 0.5, surface.height];
                        if !reached(receiver, source) { continue; }
                        let native = field.index(wx, wy).map(|j| (j, field.terrain[j].height as f32));
                        let absent_native = surface.owner == 0 && native.is_some_and(|(j, h)| h == surface.height && zone.ground[j].r < 0.0);
                        if !absent_native && blocked_reference(&zone, &field.terrain_shadow, receiver, surface.owner == 0, source) {
                            expected[i] = 0;
                            saw_shadow = true;
                        }
                    }
                }
                // Repeat once to exercise the exact static-world mask as well
                // as the first-seen source path, while actor rays stay live.
                for _ in 0..2 {
                    let mut actual = seed.clone();
                    occlude(&zone, &field, &frame, source, &mut actual);
                    assert_eq!(actual, expected, "camera=({cam_x},{cam_y}) source={source:?}");
                }
            }
        }
        assert!(saw_raised_native_actor, "the comparison must include current native raised actor reception");
        assert!(saw_shadow, "the comparison must exercise actual terrain intersections, not an empty mask");
    }

    #[test]
    fn integral_empty_rectangle_proof_matches_block_occupancy() {
        let mut zone = Zone::new(ZoneId::Glasswater, 4, 4);
        for (x, y) in [(0, 0), (23, 15), (31, 40), (63, 63)] {
            zone.gset_plane_surface(x, y, [0.0, 1.0, 0.0], 8.0);
        }
        let geometry = Geometry::new(&zone);
        let boundaries = [-9, -1, 0, 7, 8, 15, 16, 31, 32, 55, 63, 64, 71];
        for &x0 in &boundaries {
            for &x1 in &boundaries {
                for &y0 in &boundaries {
                    for &y1 in &boundaries {
                        let mut expected = false;
                        for by in 0..8 {
                            for bx in 0..8 {
                                if geometry.blocks[by * geometry.cols + bx] != 0
                                    && x0.max(0) <= x1.min(63) && y0.max(0) <= y1.min(63)
                                    && bx as i32 >= x0.max(0) / BLOCK && bx as i32 <= x1.min(63) / BLOCK
                                    && by as i32 >= y0.max(0) / BLOCK && by as i32 <= y1.min(63) / BLOCK { expected = true; }
                            }
                        }
                        assert_eq!(geometry.occupied_rect(&zone, x0, y0, x1, y1), expected, "rect=({x0},{y0})..({x1},{y1})");
                    }
                }
            }
        }
    }

    #[test]
    fn actual_glasswater_crystal_is_occluded_across_its_native_wall() {
        let (zone, region) = crate::regions::build(ZoneId::Glasswater);
        assert_eq!(zone.tile_at(617.6, 556.8), Tile::Floor);
        assert_eq!(zone.tile_at(618.5, 589.5), Tile::Floor);
        let mut frame = Frame::new();
        frame.cam_x = 480;
        frame.cam_y = 480;
        frame.begin(zone.bg, zone.ambient);
        frame.surface_lighting = true;
        zone.draw_ground(&mut frame);
        super::super::ground_surface(&zone, region.as_ref(), &mut frame);
        region.light(&zone, &mut frame, 90);
        let source = *frame.point_lights.iter().find(|p| (p.x - 617.6).abs() < 20.0 && (p.y + p.height - 556.0).abs() < 0.1).expect("authored south-lake crystal source, including native tip offset and raster-aligned sprite position");
        // A visible floor pixel beyond the neighboring painted wall; the
        // independent slab probe below proves its exact native-height crossing.
        let receiver = [618.5, 589.5, 0.0];
        let cache = zone.lighting.borrow();
        let field = cache.as_ref().unwrap();
        assert!(reached(receiver, source));
        // Independent cell/slab intersection establishes that this is an
        // actual painted wall crossing, not merely a coarse collision wall or
        // a positive result produced by the traversal being tested.
        let mut crossings = Vec::new();
        let dx = f64::from(source.x) - f64::from(receiver[0]);
        let dy = f64::from(source.y) - f64::from(receiver[1]);
        let dz = f64::from(source.height) - f64::from(receiver[2]);
        for y in source.y.min(receiver[1]).floor() as i32..=source.y.max(receiver[1]).ceil() as i32 {
            for x in source.x.min(receiver[0]).floor() as i32..=source.x.max(receiver[0]).ceil() as i32 {
                if x < 0 || y < 0 || x >= zone.pw || y >= zone.ph { continue; }
                let h = height(&zone, x, y);
                if h <= 0.0 { continue; }
                let mut interval = (f64::from(EPS), 1.0 - f64::from(EPS));
                if clip(f64::from(receiver[0]) - x as f64, dx, 1.0, &mut interval)
                    && clip(f64::from(receiver[1]) - y as f64, dy, 1.0, &mut interval)
                    && clip(f64::from(receiver[2] - h) + 0.5, dz, 1.0, &mut interval) {
                    crossings.push((x, y, h, interval));
                }
            }
        }
        assert!(!crossings.is_empty(), "native height sheet must intersect the independently clipped source ray: source={source:?} receiver={receiver:?}");
        eprintln!("Glass terrain source={source:?}, receiver={receiver:?}, independent native crossings={crossings:?}");
        assert!(blocked(&zone, &field.terrain_shadow, receiver, true, source), "actual native wall must stop its neighboring crystal light: source={source:?} receiver={receiver:?}");
        assert!(field.terrain_shadow.blocks.iter().any(|&h| h > 0));
    }
}
