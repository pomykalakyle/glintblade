//! Default pixel-grid lighting: A silhouettes/contact, C mineral planes, B lava sources only.
//! Static geometry is baked in world space into a single Zone-owned field, never a global cache.
//! Ground/emission/tile setters invalidate it; bulk buffer edits must call invalidate_lighting.
//! Sprite breakage refreshes bounded world-space patches; other geometry still invalidates the bake.

mod cinder_floor;
mod dynamic;
mod patch;
mod surface_shadows;
mod terrain_shadows;
mod terrain_volumes;
mod sun_shadows;
use patch::{Planes, Rect, incremental, procedural_bounds, prop_bounds};

use super::{Caster, Cell, Frame, H, Rgb, W, rgb};
use crate::world::{Region, TILE, Tile, Zone, ZoneId};
use std::hash::{Hash, Hasher};

const PROCEDURAL_HEIGHT: i32 = 192;

/// One native diagnostic selects the former baked lighting. Browser/default
/// builds always use the live surface renderer and never bake its casts into art.
pub(crate) fn legacy_mode() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static LEGACY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *LEGACY.get_or_init(|| std::env::var("GLINT_LIGHTING").is_ok_and(|v| v == "legacy"))
    }
    #[cfg(target_arch = "wasm32")]
    { false }
}

/// Reserve inferred box planes for upright stone masses with an explicit material identity.
fn mineral_sprite(p: &crate::world::Prop, cell: &Cell) -> bool {
    let selected = match p.sheet {
        "bloomwild_ruins" => p.anim.starts_with("pillar") || p.anim == "statue",
        "bloomwild_arch" => p.anim == "arch_post",
        "bloomwild_waterworks" | "cinder_pillars" | "cinder_statue" | "cinder_ruins"
        | "summit_pillars" | "summit_stones" | "glasswater_crystals_big" => true,
        "bloomwild_props" | "cinder_props" | "cinder_props_m" => p.anim.starts_with("rock"),
        _ => false,
    };
    if !selected { return false; }
    let (mut left, mut right, mut top, mut bottom) = (cell.w, -1, cell.h, -1);
    for y in 0..cell.h {
        for x in 0..cell.w {
            if cell.at(x, y).a {
                left = left.min(x); right = right.max(x);
                top = top.min(y); bottom = bottom.max(y);
            }
        }
    }
    bottom >= top && bottom - top >= right - left
}

/// A collision-only companion must not recover a second copy of an existing sprite's planes.
fn sprite_companion(z: &Zone, p: &crate::world::Prop) -> bool {
    p.sheet == "__summit_blocker" || p.sheet().and_then(|s| s.anim(&p.anim)).is_none()
        && z.props.iter().any(|other| other.alive
            && (other.x - p.x).abs() <= 2.0 && (other.y - p.y).abs() <= 2.0
            && other.sheet().and_then(|s| s.anim(&other.anim)).is_some())
}

#[derive(Clone, Copy)]
struct Source {
    x: f32,
    y: f32,
    z: f32,
    radius: f32,
    color: Rgb,
}

#[derive(Clone, Copy, Default)]
struct Plane {
    h: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    edge: f32,
}

/// Compact retained terrain geometry: signed normal, altitude and macro orientation.
/// Zero normal denotes absent metadata. Five bytes per world pixel preserves an
/// authored upright face independently of its small material-normal facets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TerrainSurface {
    pub normal: [i8; 3],
    pub height: u8,
    pub upright: bool,
}

impl TerrainSurface {
    pub(crate) fn new(normal: [f32; 3], height: f32) -> Self {
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if !length.is_finite() || length < 0.001 { return Self::default(); }
        Self {
            normal: normal.map(|n| (n / length * 127.0).round().clamp(-127.0, 127.0) as i8),
            height: height.round().clamp(0.0, 255.0) as u8,
            upright: normal[2] / length <= 0.5,
        }
    }

    pub(crate) fn present(self) -> bool { self.normal != [0; 3] }

    fn surface(self) -> super::frame::Surface {
        super::frame::Surface {
            normal: self.normal.map(|n| n as f32 / 127.0),
            height: self.height as f32,
            owner: 0,
            kind: if self.upright { 5 } else if self.height == 0 { 1 } else { 2 },
        }
    }
}

pub(crate) struct Field {
    settings: crate::world::LightingSettings,
    key: u64,
    clip: Rect,
    alive: Vec<bool>,
    #[cfg(test)]
    patched_pixels: usize,
    w: i32,
    h: i32,
    light: Vec<Rgb>,
    dark: Vec<u8>,
    /// Source-independent contact/AO, retained separately from legacy fixed casts.
    contact: Vec<u8>,
    /// Cold matte form multiplier; physical lighting supplies all point-source facing live.
    cold_form: Vec<u8>,
    raised: Vec<bool>,
    receiving: Vec<bool>,
    terrain: Vec<TerrainSurface>,
    terrain_shadow: terrain_shadows::Geometry,
    sun_terrain: Option<sun_shadows::SunTerrain>,
    art: Vec<Rgb>,
    sources: Vec<Source>,
    lava: Vec<(i32, i32, u16)>,
    court: Option<cinder_floor::Floor>,
}

/// Fingerprint metadata, not megabytes of ground pixels; Zone setters track buffer contents.
fn geometry_key(z: &Zone) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    (z.id, z.pw, z.ph, z.tw, z.th, z.lighting_revision).hash(&mut hash);
    z.terrain_volumes.len().hash(&mut hash);
    for volume in &z.terrain_volumes {
        for component in volume.min.into_iter().chain(volume.max) { component.to_bits().hash(&mut hash); }
    }
    for c in [z.lighting_settings.fill, z.lighting_settings.shadow_tint] {
        for v in [c.r, c.g, c.b] { v.to_bits().hash(&mut hash); }
    }
    (
        z.ground.as_ptr(),
        z.ground.len(),
        z.ground_emit.as_ptr(),
        z.ground_emit.len(),
    )
        .hash(&mut hash);
    for p in &z.props {
        (
            p.sheet,
            &p.anim,
            p.alive || incremental(p),
            p.breakable,
            p.flip,
            p.canopy,
            p.t0,
        )
            .hash(&mut hash);
        for v in [
            p.x,
            p.y,
            p.footprint.map_or(0.0, |v| v.0),
            p.footprint.map_or(0.0, |v| v.1),
        ] {
            v.to_bits().hash(&mut hash);
        }
        p.footprint.is_some().hash(&mut hash);
        for r in p.collision_rects() {
            for v in r { v.to_bits().hash(&mut hash); }
        }
        if let Some((c, r, i, x, y)) = p.light {
            for v in [c.r, c.g, c.b, r, i, x, y] {
                v.to_bits().hash(&mut hash);
            }
        }
        p.light.is_some().hash(&mut hash);
    }
    for l in &z.lights {
        for v in [
            l.x,
            l.y,
            l.radius,
            l.color.r,
            l.color.g,
            l.color.b,
            l.intensity,
        ] {
            v.to_bits().hash(&mut hash);
        }
    }
    hash.finish()
}

/// Draw authored scenery at a canonical tick for alpha and procedural relief recovery.
/// Page from the requested crop's origin: a small patch straddling old screen-page
/// seams needs one reconstruction, rather than up to four complete frames. All
/// material/reflection sampling stays in world coordinates.
fn scenery(z: &Zone, region: &dyn Region, rect: Rect) -> (Vec<Rgb>, Vec<Rgb>, Vec<u8>) {
    let n = rect.len();
    let mut art = vec![Rgb::BLACK; n];
    let mut emit = vec![Rgb::BLACK; n];
    let mut contact = vec![0; n];
    if n == 0 { return (art, emit, contact); }
    let row_width = (rect.x1 - rect.x0) as usize;
    let mut f = Frame::new();
    for cy in (rect.y0..rect.y1).step_by(H) {
        for cx in (rect.x0..rect.x1).step_by(W) {
            f.cam_x = cx;
            f.cam_y = cy;
            f.begin(z.bg, z.ambient);
            z.draw_ground(&mut f);
            region.draw_ground(z, &mut f, 0);
            let mut items = Vec::new();
            region.sorted_items(z, &f, &mut items);
            let mut ordered: Vec<_> = items
                .into_iter()
                .map(|(y, id)| (y, false, id as usize))
                .collect();
            ordered.extend(
                z.props
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.alive && !p.canopy)
                    .map(|(i, p)| (p.sort_y(), true, i)),
            );
            ordered.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
            for (_, prop, i) in ordered {
                if prop {
                    z.props[i].draw(&mut f, 0, 1.0);
                } else {
                    region.draw_item(z, &mut f, i as u32, 0);
                }
            }
            for p in z.props.iter().filter(|p| p.alive && p.canopy) {
                p.draw(&mut f, 0, 1.0);
            }
            // Copy only reconstructed rows/columns. Iterating whole canonical pages
            // and testing rect.index per pixel made a 12k crop scan up to 330k pixels.
            let rows = (rect.y1.min(z.ph) - cy).clamp(0, H as i32) as usize;
            let cols = (rect.x1.min(z.pw) - cx).clamp(0, W as i32) as usize;
            for y in 0..rows {
                let src = y * W;
                let dst = ((cy - rect.y0) as usize + y) * row_width + (cx - rect.x0) as usize;
                art[dst..dst + cols].copy_from_slice(&f.albedo[src..src + cols]);
                emit[dst..dst + cols].copy_from_slice(&f.emissive[src..src + cols]);
                for (out, shade) in contact[dst..dst + cols].iter_mut().zip(&f.shade[src..src + cols]) {
                    *out = ((1.0 - shade) * 255.0).round() as u8;
                }
            }
        }
    }
    (art, emit, contact)
}

/// Infer C's cap and broad side planes from a real opaque row, independent of texture gradients.
fn plane(x: i32, y: i32, left: i32, right: i32, top: i32, base: i32) -> Plane {
    let u = (x - left) as f32 / (right - left + 1).max(1) as f32;
    let (nx, ny, nz) = if y - top < ((base - top) / 7).clamp(2, 7) {
        (-0.25, -0.35, 0.90)
    } else if u < 0.38 {
        (-0.80, 0.05, 0.60)
    } else if u > 0.80 {
        (0.80, 0.10, 0.60)
    } else {
        (0.10, 0.65, 0.75)
    };
    Plane {
        h: (base - y).max(1) as f32,
        nx,
        ny,
        nz,
        edge: if x - left < 2 {
            -1.0
        } else if right - x < 2 {
            1.0
        } else {
            0.0
        },
    }
}

impl Field {
    /// Safely index a world pixel, including casts extending past zone boundaries.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            Some((y * self.w + x) as usize)
        } else {
            None
        }
    }

    /// Min-combine visibility as max darkness, retaining exact native-pixel projection gaps.
    fn write(&mut self, x: i32, y: i32, amount: u8) {
        if let Some(i) = self.paint_index(x, y) {
            self.dark[i] = self.dark[i].max(amount);
        }
    }

    /// Anchor scenery with A's attached two-band contact, without an extra actor ellipse.
    fn contact(&mut self, x: i32, y: i32, width: f32, depth: f32) {
        let rx = (width * 0.5).ceil().max(2.0) as i32 + 2;
        let ry = (depth * 0.5).ceil().clamp(2.0, 7.0) as i32;
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                let q = (dx as f32 / rx as f32).powi(2) + (dy as f32 / ry as f32).powi(2);
                if q <= 1.0 {
                    let amount = if q < 0.5 { 89 } else { 46 };
                    self.write(x + dx, y - 1 + dy, amount);
                    if let Some(i) = self.paint_index(x + dx, y - 1 + dy) {
                        self.contact[i] = self.contact[i].max(amount);
                    }
                }
            }
        }
    }

    /// Bake B's chamfer distance and nearest actual molten pixel, with no crystal source skirts.
    fn lava_distance(&mut self, z: &Zone) {
        if z.id != ZoneId::Cinder {
            return;
        }
        self.lava = vec![(-1, -1, u16::MAX); self.art.len()];
        for y in 0..self.h {
            for x in 0..self.w {
                let i = (y * self.w + x) as usize;
                let e = z.ground_emit[i];
                if z.tile_at(x as f32, y as f32) == Tile::Lava || e.r > 0.18 && e.r > e.b * 1.3 {
                    self.lava[i] = (x, y, 0);
                }
            }
        }
        for reverse in [false, true] {
            for yy in 0..self.h {
                let y = if reverse { self.h - 1 - yy } else { yy };
                for xx in 0..self.w {
                    let x = if reverse { self.w - 1 - xx } else { xx };
                    let i = (y * self.w + x) as usize;
                    let s = if reverse { 1 } else { -1 };
                    for (dx, dy) in [(s, 0), (0, s), (s, s), (-s, s)] {
                        if let Some(j) = self.index(x + dx, y + dy) {
                            let a = self.lava[j];
                            let d = a.2.saturating_add(if dx != 0 && dy != 0 { 4 } else { 3 });
                            if d < self.lava[i].2 {
                                self.lava[i] = (a.0, a.1, d);
                            }
                        }
                    }
                }
            }
        }
        for v in &mut self.lava {
            v.2 /= 3;
        }
    }

    /// Pick the dominant nearby physical source using only world coordinates.
    fn source(&self, x: f32, y: f32) -> Option<Source> {
        let mut best = None;
        let mut distance = 1.0;
        for &s in &self.sources {
            let d = ((x - s.x).powi(2) + (y - s.y).powi(2)) / s.radius.powi(2);
            if d < distance {
                best = Some(s);
                distance = d;
            }
        }
        if let Some(i) = self.index(x.round() as i32, y.round() as i32)
            && let Some(&(lx, ly, d)) = self.lava.get(i)
            && d < 96
            && (d as f32 / 96.0).powi(2) < distance
        {
            best = Some(Source {
                x: lx as f32,
                y: ly as f32,
                z: 3.0,
                radius: 96.0,
                color: rgb(0xefb68a),
            });
        }
        best
    }

    /// Project away from daylight or an exposed local emitter; unlit rests cast no invented sun shadow.
    fn direction(&self, id: ZoneId, x: f32, y: f32, heart: Option<Source>) -> Option<(f32, f32)> {
        if id == ZoneId::Bloomwild {
            return Some((0.55, 0.27));
        }
        let local = self.source(x, y);
        let source = heart
            .filter(|s| {
                let d = (s.x - x).hypot(s.y - y);
                d > 1.0
                    && d < s.radius
                    && local.is_none_or(|l| d / s.radius < (l.x - x).hypot(l.y - y) / l.radius)
            })
            .or(local)?;
        let (dx, dy) = (x - source.x, y - source.y);
        let d = dx.hypot(dy).max(1.0);
        let length = (d / 35.0).clamp(0.55, 1.30);
        Some((dx / d * length, dy / d * length * 0.65))
    }

    /// Project A's exact sprite alpha, including separate tree crowns, and cache C's solid planes.
    fn sprite(
        &mut self,
        z: &Zone,
        planes: &mut Planes,
        cell: &Cell,
        anchor: (i32, i32),
        p: &crate::world::Prop,
    ) {
        let ox = p.x.floor() as i32
            - if p.flip {
                cell.w - 1 - anchor.0
            } else {
                anchor.0
            };
        let oy = p.y.floor() as i32 - anchor.1;
        let Some(bottom) = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)) else { return };
        let top = (0..=bottom).find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).unwrap();
        // The first empty row below native ink is the receiving ground, even when
        // the atlas stages the sprite above its anchor or extends a base below it.
        let base = oy + bottom + 1;
        let dir = self.direction(z.id, p.x.floor(), base as f32, None);
        // Trees retain their own authored foliage ramp; planes are reserved for mineral masses.
        let mineral = mineral_sprite(p, cell);
        for y in 0..cell.h {
            let xs: Vec<_> = (0..cell.w).filter(|&x| cell.at(x, y).a).collect();
            let (Some(&left), Some(&right)) = (xs.first(), xs.last()) else {
                continue;
            };
            for x in xs {
                let wx = ox + if p.flip { cell.w - 1 - x } else { x };
                let wy = oy + y;
                if let Some(i) = self.paint_index(wx, wy) {
                    self.raised[i] = true;
                    if mineral {
                        let (l, r) = if p.flip {
                            (cell.w - 1 - right, cell.w - 1 - left)
                        } else {
                            (left, right)
                        };
                        planes.set(wx, wy, plane(wx - ox, y, l, r, top, bottom + 1));
                    }
                }
                if let Some((dx, dy)) = dir {
                    let height = bottom - y;
                    if height < 0 {
                        continue;
                    }
                    let steps = (dx.abs().max(dy.abs()) * 2.0).ceil().max(2.0) as i32;
                    for sub in 0..steps {
                        let h = height as f32 + sub as f32 / steps as f32;
                        self.write(
                            (wx as f32 + h * dx).round() as i32,
                            (base as f32 + h * dy).round() as i32,
                            186,
                        );
                    }
                }
            }
        }
    }

    /// Recover private procedural stone/crystal silhouettes once from the complete scenery bake.
    fn procedural(&mut self, z: &Zone, planes: &mut Planes, p: &crate::world::Prop) {
        let Some((width, _)) = p.footprint else {
            return;
        };
        if width > 80.0 {
            return;
        }
        let (cx, base) = (p.x.round() as i32, p.y.round() as i32);
        let radius = (width * 1.15 + 9.0).min(42.0) as i32;
        let mut rows = Vec::new();
        let mut top = base;
        let mut gaps = 0;
        for y in ((base - PROCEDURAL_HEIGHT).max(0)..base).rev() {
            let xs: Vec<_> = ((cx - radius).max(0)..(cx + radius + 1).min(self.w))
                .filter(|&x| {
                    let i = (y * self.w + x) as usize;
                    let a = self.art[i];
                    let g = z.ground[i];
                    (a.r - g.r).abs() + (a.g - g.g).abs() + (a.b - g.b).abs() > 0.08
                        && !matches!(
                            z.tile_at(x as f32, y as f32),
                            Tile::Shallow | Tile::Deep | Tile::Lava
                        )
                })
                .collect();
            if let (Some(&l), Some(&r)) = (xs.first(), xs.last())
                && l <= cx + width as i32 / 2
                && r >= cx - width as i32 / 2
            {
                top = y;
                rows.push((y, l, r, xs));
                gaps = 0;
                continue;
            }
            gaps += 1;
            if gaps > 3 && !rows.is_empty() || gaps > 12 && rows.is_empty() {
                break;
            }
        }
        let dir = self.direction(z.id, p.x, p.y, None);
        for (y, l, r, xs) in rows {
            for x in xs {
                if let Some(i) = self.paint_index(x, y) {
                    self.raised[i] = true;
                    planes.set(x, y, plane(x, y, l, r, top, base));
                }
                if let Some((dx, dy)) = dir {
                    for sub in 0..3 {
                        let h = (base - y) as f32 + sub as f32 / 3.0;
                        self.write(
                            (x as f32 + h * dx).round() as i32,
                            (p.y + h * dy).round() as i32,
                            170,
                        );
                    }
                }
            }
        }
    }

    /// Bake all static contact, form planes, cast shadows, source rims and lava-bank illumination once.
    fn bake(z: &Zone, region: &dyn Region, key: u64) -> Self {
        let clip = Rect::zone(z);
        let (art, emit, dark) = scenery(z, region, clip);
        let n = art.len();
        let mut f = Self {
            settings: z.lighting_settings,
            key,
            clip,
            alive: z.props.iter().map(|p| p.alive).collect(),
            #[cfg(test)]
            patched_pixels: 0,
            w: z.pw,
            h: z.ph,
            light: vec![Rgb::WHITE; n],
            contact: dark.clone(),
            cold_form: vec![255; n],
            dark,
            raised: vec![false; n],
            receiving: vec![false; n],
            terrain: vec![TerrainSurface::default(); n],
            terrain_shadow: terrain_shadows::Geometry::new(z),
            sun_terrain: (z.id == ZoneId::Bloomwild).then(|| sun_shadows::SunTerrain::new(z)),
            art,
            sources: Vec::new(),
            lava: Vec::new(),
            court: cinder_floor::Floor::bake(z),
        };
        for l in &z.lights {
            f.sources.push(Source {
                x: l.x,
                y: l.y,
                z: 30.0,
                radius: l.radius,
                color: l.color,
            });
        }
        for p in z.props.iter().filter(|p| p.alive) {
            if let Some((color, radius, _, ox, oy)) = p.light {
                f.sources.push(Source {
                    x: p.x + ox,
                    y: p.y,
                    z: (-oy).max(20.0),
                    radius: radius.max(56.0),
                    color,
                });
            } else if z.id == ZoneId::Glasswater
                && p.sheet == "glasswater_blocker"
                && let Some((width, 7.0)) = p.footprint
                && (8.0..=26.0).contains(&width)
            {
                let mut count = 0;
                let mut crest = 0.0_f32;
                for y in (p.y as i32 - 64).max(0)..(p.y as i32).min(z.ph) {
                    for x in (p.x as i32 - 18).max(0)..(p.x as i32 + 19).min(z.pw) {
                        if emit[(y * z.pw + x) as usize].max_c() > 0.28 {
                            count += 1;
                            crest = crest.max(p.y - y as f32);
                        }
                    }
                }
                if count >= 5 {
                    f.sources.push(Source {
                        x: p.x,
                        y: p.y,
                        // Follow the visible cap instead of placing tall crystals' key inside their root.
                        z: crest.max(28.0),
                        radius: 104.0,
                        color: rgb(0xcdc3db),
                    });
                }
            }
        }
        f.lava_distance(z);
        f.geometry(z);
        f
    }

    /// Reconstruct only the clipped terrain and prop contributions, retaining max-combined overlaps.
    fn geometry(&mut self, z: &Zone) {
        let f = self;
        let mut planes = Planes::new(f.clip);
        let walls = f.clip.expand(24).clamp(z);
        // These regions paint curved, pixel-defined faces that deliberately differ from collision tiles.
        let authored_terrain = matches!(z.id, ZoneId::Bloomwild | ZoneId::Glasswater | ZoneId::Summit);
        for y in walls.y0..walls.y1 {
            for x in walls.x0..walls.x1 {
                let i = (y * z.pw + x) as usize;
                let inside = f.clip.index(x, y).is_some();
                let tile = z.tile_at(x as f32, y as f32);
                if inside {
                    f.receiving[i] =
                        (matches!(tile, Tile::Floor | Tile::Shallow)
                            || (authored_terrain && !matches!(tile, Tile::Deep | Tile::Lava)))
                            && z.ground[i].r >= 0.0;
                }
                if tile == Tile::Wall && !authored_terrain {
                    let (tx, ty) = (x.div_euclid(TILE), y.div_euclid(TILE));
                    let (mut left, mut right, mut front) = (96, 96, 96);
                    for step in 1..=4 {
                        if z.tile(tx - step, ty) != Tile::Wall {
                            left = left.min(x - (tx - step + 1) * TILE);
                        }
                        if z.tile(tx + step, ty) != Tile::Wall {
                            right = right.min((tx + step) * TILE - x - 1);
                        }
                        if z.tile(tx, ty + step) != Tile::Wall {
                            front = front.min((ty + step) * TILE - y - 1);
                        }
                    }
                    let (nx, ny, nz) = if left < 9 && left < right && left < front {
                        (-0.85, 0.0, 0.45)
                    } else if right < 9 && right < front {
                        (0.85, 0.0, 0.45)
                    } else if front < 13 {
                        (0.0, 0.85, 0.45)
                    } else {
                        (0.0, 0.0, 1.0)
                    };
                    planes.set(
                        x,
                        y,
                        Plane {
                            h: 18.0,
                            nx,
                            ny,
                            nz,
                            ..Plane::default()
                        },
                    );
                    if inside {
                        f.raised[i] = true;
                    }
                    // The exposed front wall silhouette also casts off its actual connected edge.
                    if z.tile(tx, ty + 1) != Tile::Wall
                        && y.rem_euclid(TILE) == TILE - 1
                        && let Some((dx, dy)) = f.direction(z.id, x as f32, y as f32, None)
                    {
                        for height in 1..=18 {
                            f.write(
                                x + (height as f32 * dx).round() as i32,
                                y + (height as f32 * dy).round() as i32,
                                160,
                            );
                        }
                    }
                } else if inside && !authored_terrain && tile == Tile::Void && z.ground[i].r >= 0.0 {
                    planes.set(
                        x,
                        y,
                        Plane {
                            h: 20.0,
                            ny: 0.9,
                            nz: 0.4,
                            ..Plane::default()
                        },
                    );
                    f.raised[i] = true;
                } else if inside && f.receiving[i] && !authored_terrain {
                    let distances = [
                        (z.tile(x / TILE, y / TILE - 1), y.rem_euclid(TILE)),
                        (z.tile(x / TILE - 1, y / TILE), x.rem_euclid(TILE)),
                        (
                            z.tile(x / TILE + 1, y / TILE),
                            TILE - 1 - x.rem_euclid(TILE),
                        ),
                    ];
                    let d = distances
                        .iter()
                        .filter(|(t, _)| *t == Tile::Wall)
                        .map(|(_, d)| *d)
                        .min()
                        .unwrap_or(16);
                    let contact = match d {
                        0 => 82,
                        1 => 56,
                        2..=3 => 26,
                        _ => 0,
                    };
                    f.dark[i] = f.dark[i].max(contact);
                    f.contact[i] = f.contact[i].max(contact);
                }
            }
        }
        // Retain receiving terrain before y-sorted prop planes overwrite the scratch.
        // A revealed tree/column must expose ground geometry, never its cached body plane.
        for y in f.clip.y0..f.clip.y1 {
            for x in f.clip.x0..f.clip.x1 {
                let i = (y * f.w + x) as usize;
                let native = z.ground_surfaces[i];
                let p = planes.at(x, y);
                f.terrain[i] = if z.ground[i].r < 0.0 {
                    TerrainSurface::default()
                } else if native.present() {
                    native
                } else {
                    let base = [p.nx, p.ny, p.nz];
                    let normal = if !authored_terrain
                        && matches!(z.tile_at(x as f32, y as f32), Tile::Wall | Tile::Void) {
                        crate::gfx::terrain_detail::normal(crate::gfx::terrain_detail::Material::Rock, x, y, base)
                    } else { base };
                    TerrainSurface::new(normal, p.h)
                };
            }
        }
        let mut props: Vec<_> = z.props.iter().filter(|p| p.alive).collect();
        props.sort_by(|a, b| a.canopy.cmp(&b.canopy).then(a.sort_y().total_cmp(&b.sort_y())));
        for p in props {
            // Offset row-span physics proxies have no visible silhouette or contact shade.
            // Their authored source sprite already supplies both, even far from this box.
            if p.sheet == "__summit_blocker" { continue; }
            let bounds = if p.sheet().and_then(|s| s.anim(&p.anim)).is_some() {
                prop_bounds(p)
            } else {
                procedural_bounds(p)
            };
            if !bounds.intersects(f.clip) {
                continue;
            }
            if let Some([x0, y0, x1, y1]) = p.collision_bounds() {
                let (w, d) = (x1 - x0, y1 - y0);
                // The painted dais has its own stone faces; its collision proxy is not an oval prop.
                if !(z.id == ZoneId::Summit && p.sheet == "__summit_blocker" && w > 80.0) {
                    f.contact(((x0 + x1) * 0.5).round() as i32, y1.round() as i32, w, d);
                }
            }
            if let Some(s) = p.sheet()
                && let Some(a) = s.anim(&p.anim)
            {
                f.sprite(z, &mut planes, s.cell(a, 0), s.anchor_of(a), p);
            } else if !sprite_companion(z, p) {
                f.procedural(z, &mut planes, p);
            }
        }
        for y in f.clip.y0..f.clip.y1 {
            for x in f.clip.x0..f.clip.x1 {
                let i = (y * f.w + x) as usize;
                let plane = planes.at(x, y);
                f.light[i] = f.form(z.id, plane, x as f32, y as f32);
                f.cold_form[i] = (Self::cold_form(plane) * 255.0).round() as u8;
                if f.raised[i] || !f.receiving[i] {
                    f.dark[i] = 0;
                    f.contact[i] = 0;
                }
            }
        }
    }

    /// Light C's broad planes with the upper-left key or nearby motivated source, plus B's lava banks.
    fn form(&self, id: ZoneId, p: Plane, x: f32, y: f32) -> Rgb {
        let mut light = if id == ZoneId::Bloomwild {
            let key = (((-0.56 * p.nx - 0.32 * p.ny + 0.76 * p.nz).max(0.0) * 4.0).round() / 4.0).max(0.25);
            self.settings.fill + rgb(0xffefd3) * (key * 0.68)
        } else {
            let fill = self.settings.fill;
            let form = Self::cold_form(p);
            fill * form
        };
        if id != ZoneId::Bloomwild
            && p.h > 0.0
            && let Some(s) = self.source(x, y + p.h)
        {
            let (dx, dy, dz) = (s.x - x, s.y - y - p.h, s.z - p.h);
            let d = (dx * dx + dy * dy + dz * dz).sqrt().max(1.0);
            let weight = (1.0 - d / s.radius).max(0.0);
            let incidence =
                (((p.nx * dx + p.ny * dy + p.nz * dz) / d).max(0.0) * 3.0).round() / 3.0;
            light += s.color * (incidence * weight * 0.65);
            if p.edge * dx > 0.0 && weight > 0.45 {
                light += s.color * (0.20 * weight);
            }
        }
        if id == ZoneId::Cinder
            && let Some(i) = self.index(x.round() as i32, y.round() as i32)
        {
            let d = self.lava[i].2;
            let heat = match d {
                0..=7 => 0.52,
                8..=19 => 0.38,
                20..=37 => 0.24,
                38..=61 => 0.12,
                _ => 0.0,
            };
            light += Rgb::new(1.0, 0.64, 0.40) * heat;
        }
        light
    }

    fn cold_form(p: Plane) -> f32 {
        if p.h > 0.0 {
            (0.72 + p.nz * 0.20 - p.nx * 0.08 - p.ny * 0.12).clamp(0.56, 1.0)
        } else {
            1.0
        }
    }

    /// No cached point-source direction or rim enters the physical base light.
    /// Cinder's broad molten-bank reception remains a motivated area-emission fill.
    fn cold_light(&self, id: ZoneId, i: usize, actor: bool) -> Rgb {
        let form = if actor { 1.0 } else { self.cold_form[i] as f32 / 255.0 };
        // Open daylight receives broad sky and ground bounce even when the sun
        // is hidden. Trade some direct-key energy for this diffuse contribution
        // so moving cast shadows stay in the same daytime exposure as the sky.
        let fill = if id == ZoneId::Bloomwild {
            self.settings.fill + Rgb::new(0.26, 0.26, 0.24)
        } else {
            self.settings.fill
        };
        let mut light = fill * form;
        if id == ZoneId::Cinder {
            let heat = match self.lava[i].2 {
                0..=7 => 0.52,
                8..=19 => 0.38,
                20..=37 => 0.24,
                38..=61 => 0.12,
                _ => 0.0,
            };
            // The exposed moving current supplies directional, occluded light;
            // retain only a little broad area bounce as the cold fill.
            light += Rgb::new(1.0, 0.64, 0.40) * (heat * 0.30);
        }
        light
    }
}

/// Finalize at zone load; refresh sprite breakage locally and rebuild other geometry on first use.
pub(crate) fn prepare(zone: &Zone, region: &dyn Region) {
    let key = geometry_key(zone);
    let mut cache = zone.lighting.borrow_mut();
    if cache.as_ref().is_none_or(|v| v.key != key) {
        *cache = Some(Field::bake(zone, region, key));
    } else if let Some(field) = cache.as_mut() {
        field.refresh_breakage(zone, region);
    }
}

/// Stamp the visible terrain after region ground effects and before sorted objects.
/// Native texture normals keep their painter's exact UV; geometry-only fallbacks
/// supply planar floors and authored cliff faces without inferring bumps from RGB.
pub fn ground_surface(zone: &Zone, region: &dyn Region, f: &mut Frame) {
    if !f.surface_lighting { return; }
    prepare(zone, region);
    let cache = zone.lighting.borrow();
    let field = cache.as_ref().unwrap();
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            // Region ground cells already stamped their current frame metadata.
            if f.surfaces[i].kind != 0 { continue; }
            let Some(j) = field.index(x as i32 + f.cam_x, y as i32 + f.cam_y) else { continue };
            let surface = field.terrain[j];
            if surface.present() { f.surfaces[i] = surface.surface(); }
        }
    }
}

/// Diagnostic counts: opaque terrain with retained normals (native source UVs,
/// authored geometry or completed planes), raised pixels and distinct normals.
/// No lighting or geometry is changed.
pub fn terrain_normal_coverage(zone: &Zone) -> (usize, usize, usize) {
    let mut pixels = 0;
    let mut raised = 0;
    let mut normals = std::collections::HashSet::new();
    for (art, surface) in zone.ground.iter().zip(&zone.ground_surfaces) {
        if art.r < 0.0 || !surface.present() { continue; }
        pixels += 1;
        raised += usize::from(surface.height > 0);
        normals.insert(surface.normal);
    }
    (pixels, raised, normals.len())
}

/// Per-source geometric visibility for the normal-aware direct-light pass.
pub(crate) fn sun_direction(t: u64) -> [f32; 3] { sun_shadows::direction(t) }

pub(crate) fn sun_visibility(zone: Option<&Zone>, f: &Frame, direction: [f32; 3]) -> Vec<u8> {
    let mut visible = sun_shadows::visibility(f, direction);
    if let Some(zone) = zone {
        let cached = zone.lighting.borrow();
        if let Some(terrain) = cached.as_ref().and_then(|field| field.sun_terrain.as_ref()) {
            terrain.occlude(f, direction, &mut visible);
        }
    }
    visible
}

pub(crate) fn point_visibility(f: &Frame, light: super::frame::PointLight) -> Vec<u8> {
    surface_shadows::visibility(f, light)
}

/// Geometric local-light visibility including exact native terrain height sheets.
pub(crate) fn point_visibility_world(zone: &Zone, f: &Frame, light: super::frame::PointLight) -> Vec<u8> {
    let mut visibility = surface_shadows::visibility(f, light);
    if let Some(field) = zone.lighting.borrow().as_ref() {
        terrain_shadows::occlude(zone, field, f, light, &mut visibility);
    }
    visibility
}

/// Bake on first use; later frames only sample static light and project moving body casters.
pub fn apply(zone: &Zone, region: &dyn Region, f: &mut Frame, t: u64) {
    prepare(zone, region);
    let cache = zone.lighting.borrow();
    let field = cache.as_ref().unwrap();
    let strength = dynamic::strength();
    if !f.surface_lighting { dynamic::occlude_points(zone,region,field,f,t,strength); }
    let weather = if !f.surface_lighting && strength>0.0 { dynamic::Weather::new(zone.id,f.cam_x,f.cam_y,t,strength) } else { None };
    let mut dynamic = vec![0u8; W * H];
    let heart = if zone.id == ZoneId::Summit {
        f.casters.iter().find(|c| c.height > 80.0).map(|c| Source {
            x: c.x,
            y: c.y - c.height * 0.32,
            z: 40.0,
            radius: 155.0,
            color: rgb(0xe2d8e9),
        })
    } else {
        None
    };
    let physical = f.surface_lighting;
    if !physical {
        for &c in &f.casters {
            let source = if c.height > 80.0 { None } else { heart };
            if let Some(dir) = field.direction(zone.id, c.x, c.y, source) {
                character_shadow(f, &mut dynamic, c, dir);
            }
        }
    }
    let ambient = f.ambient;
    if zone.id != ZoneId::Glasswater { f.look.light_dither = 0.0; }
    f.look.light_scatter = 0.018;
    let mut weather_row=[Rgb::WHITE;W];
    for y in 0..H {
        if let Some(weather)=&weather { weather.row(f.cam_x,f.cam_y+y as i32,&mut weather_row); }
        for x in 0..W {
            let i = y * W + x;
            let (wx, wy) = (x as i32 + f.cam_x, y as i32 + f.cam_y);
            let Some(j) = field.index(wx, wy) else {
                if !f.actor_pixels[i] { f.light[i] = region.backdrop_light(ambient); }
                continue;
            };
            if zone.ground[j].r < 0.0 && !field.raised[j] && !f.actor_pixels[i] {
                f.light[i] = region.backdrop_light(ambient);
                continue;
            }
            let actor = f.actor_pixels[i];
            let raw = f.light[i] - ambient;
            let live = Rgb::new(raw.r.max(0.0), raw.g.max(0.0), raw.b.max(0.0));
            let raised = field.raised[j] || actor || (f.surface_lighting && f.surfaces[i].height > 0.5);
            let cached_dark = if physical { field.contact[j] } else { field.dark[j] };
            let darkness = if !raised && field.receiving[j] {
                (cached_dark.max(dynamic[i]) as f32 / 255.0).max(1.0 - f.shade[i])
            } else {
                0.0
            };
            // A single visibility loss, never AO multiplied over a cast shadow or C's form plane.
            let tint = Rgb::WHITE.lerp(field.settings.shadow_tint, darkness);
            let cold = if physical { field.cold_light(zone.id, j, actor) } else { field.light[j] };
            let mut base = cold * tint;
            if !physical && !raised && field.receiving[j]
                && let Some((warm, mul)) = field.court.as_ref().and_then(|court| court.at(wx, wy)) {
                // Darken only cold fill; cast/contact shadows still occlude the shaped fire reception.
                base = (cold - field.settings.fill * (1.0 - mul)) * tint
                    + warm * (1.0 - darkness * 1.6).max(0.0);
            }
            let mut light = base
                + live
                    * (if zone.id == ZoneId::Glasswater {
                        1.15
                    } else {
                        1.0
                    });
            if physical && zone.id != ZoneId::Bloomwild { light = light.scale(0.82); }
            if actor {
                let floor = if physical { Rgb::new(0.42, 0.46, 0.57) } else { Rgb::new(0.78, 0.81, 0.90) };
                light = Rgb::new(light.r.max(floor.r), light.g.max(floor.g), light.b.max(floor.b));
                if !f.surface_lighting && zone.id == ZoneId::Cinder { light += actor_rim(field, f, x, y); }
            }
            if weather.is_some() {
                let original = light;
                light = light * weather_row[x];
                // Weather modulates materials, while silhouettes stay readable in a dark arena.
                if actor { light = Rgb::new(light.r.max(original.r.min(0.78)),light.g.max(original.g.min(0.81)),light.b.max(original.b.min(0.90))); }
            }
            f.light[i] = light;
            f.shade[i] = 1.0; // Compose must not apply the existing contact a second time.
        }
    }
}

/// Add a crisp actor edge facing a nearby Cinder emitter without inventing another glow source.
fn actor_rim(field: &Field, frame: &Frame, x: usize, y: usize) -> Rgb {
    let (wx, wy) = (x as f32 + frame.cam_x as f32, y as f32 + frame.cam_y as f32);
    let Some(caster) = frame.casters.iter().min_by(|a, b| {
        let da = (wx - a.x).powi(2) + (wy - a.y + a.height * 0.5).powi(2);
        let db = (wx - b.x).powi(2) + (wy - b.y + b.height * 0.5).powi(2);
        da.total_cmp(&db)
    }) else { return Rgb::BLACK };
    let Some(source) = field.source(caster.x, caster.y) else { return Rgb::BLACK };
    let (dx, dy) = (source.x - caster.x, source.y - caster.y);
    let distance = dx.hypot(dy);
    let i = y * W + x;
    if x == 0 || x + 1 == W || y == 0 || y + 1 == H
        || ![i - 1, i + 1, i - W, i + W].iter().all(|&p| frame.actor_pixels[p])
    { return Rgb::BLACK; }
    // Keep the outer ink contour dark; the lit material sits one native pixel inside it.
    let facing = (dx < 0.0 && x > 1 && !frame.actor_pixels[i - 2])
        || (dx > 0.0 && x + 2 < W && !frame.actor_pixels[i + 2])
        || (dy < 0.0 && y > 1 && !frame.actor_pixels[i - W * 2])
        || (dy > 0.0 && y + 2 < H && !frame.actor_pixels[i + W * 2]);
    if facing { source.color * (0.55 * (1.0 - distance / source.radius).clamp(0.0, 1.0)) }
    else { Rgb::BLACK }
}

/// Cast A's tapered body proxy on the grid; existing sprite contacts supply the small foot shadow.
fn character_shadow(f: &Frame, mask: &mut [u8], c: Caster, dir: (f32, f32)) {
    for h in 0..c.height.ceil() as i32 {
        let phase = h as f32 / c.height.max(1.0);
        let section = (1.0 - ((phase - 0.48) / 0.54).powi(2)).max(0.08).sqrt();
        let width = (c.half_w * section).ceil() as i32;
        for xx in -width..=width {
            for sub in 0..3 {
                let height = h as f32 + sub as f32 / 3.0;
                let x = (c.x + xx as f32 + height * dir.0).round() as i32 - f.cam_x;
                let y = (c.y + height * dir.1).round() as i32 - f.cam_y;
                if let Some(i) = Frame::idx(x, y) {
                    mask[i] = mask[i].max(186);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod retained_surface_tests {
    use super::*;
    use crate::gfx::surface::NormalPixel;
    use crate::world::{NoRegion, Prop};

    #[test]
    fn moving_day_shadows_preserve_daylight_exposure_and_materials() {
        let mut zone = Zone::new(ZoneId::Bloomwild, 8, 8);
        zone.ground.fill(rgb(0x929b90));
        let column = Cell::from_ascii(&["xxxxxxxxxxxx"; 48], &[('x', rgb(0xc2b59b))], &[]);
        let mut darkest_shade = f32::INFINITY;
        let mut dimmest_sun = f32::INFINITY;
        let mut brightest_sun = 0.0_f32;
        let mut masks = Vec::new();
        for t in (0..2160).step_by(270) {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, zone.ambient);
            f.surface_lighting = true;
            f.normal_sun = true;
            f.t = t;
            zone.draw_ground(&mut f);
            ground_surface(&zone, &NoRegion, &mut f);
            f.sprite(&column, (6, 48), 56, 72, &Default::default());
            // An actor facing away from the key still shares the scene's day fill.
            let actor = 112 * W + 112;
            f.actor_pixels[actor] = true;
            f.surfaces[actor] = super::super::frame::Surface {
                normal: [1.0, 0.0, 0.0], height: 12.0, owner: 0, kind: 3,
            };
            f.put_glow(110, 110, rgb(0xffa870));
            let art = f.albedo.clone();
            let emission = f.emissive.clone();
            let visibility = sun_visibility(Some(&zone), &f, sun_direction(t));
            apply(&zone, &NoRegion, &mut f, t);
            super::super::surface::apply_world(&zone, &mut f);
            let mut shadowed = 0;
            let mut sunny = 0;
            for y in 4..124 { for x in 4..124 {
                let i = y * W + x;
                if f.surfaces[i].kind != 1 { continue; }
                match visibility[i] {
                    0 => {
                        shadowed += 1;
                        darkest_shade = darkest_shade.min(f.light[i].luma());
                    }
                    255 => {
                        sunny += 1;
                        dimmest_sun = dimmest_sun.min(f.light[i].luma());
                        brightest_sun = brightest_sun.max(f.light[i].luma());
                    }
                    _ => {}
                }
            } }
            assert!(shadowed > 40 && sunny > 1000, "exercise actual geometric cast shade and open ground at tick {t}");
            assert!(f.light[actor].luma() > 0.58, "back-facing actor remains in daylight at tick {t}");
            assert_eq!(f.albedo, art, "day fill must preserve authored materials");
            assert_eq!(f.emissive, emission, "day fill must preserve self-lit pixels");
            masks.push(visibility);
        }
        assert!(darkest_shade > brightest_sun * 0.58, "cast shade must remain daylight: shade={darkest_shade}, sun={brightest_sun}");
        assert!(darkest_shade < dimmest_sun * 0.78, "moving casts must remain visible");
        assert!((0.90..1.10).contains(&dimmest_sun) && brightest_sun < 1.10, "retain the lit material exposure");
        assert!(brightest_sun - dimmest_sun < 0.07, "sun arc must not create an exposure cycle");
        assert!(masks.iter().skip(1).any(|mask| mask != &masks[0]), "the sun and its geometric shadows must keep moving");
    }

    #[test]
    fn native_terrain_uv_retains_vectors_without_changing_art_or_physics() {
        assert_eq!(std::mem::size_of::<TerrainSurface>(), 5);
        let mut cell = Cell::from_ascii(&["ab"], &[( 'a', rgb(0x112233)), ('b', rgb(0xeeddcc))], &[]);
        cell.surface = std::sync::Arc::from([
            NormalPixel { n: [0, 0, 127], a: true, emit: false },
            NormalPixel { n: [64, 32, 104], a: true, emit: false },
        ]);
        let mut zone = Zone::new(ZoneId::Bloomwild, 4, 4);
        let art = zone.ground.clone();
        let tiles = zone.tiles.clone();
        zone.gset_surface(11, 13, &cell, 1, 0, false, 0.0);
        let floor = zone.ground_surfaces[(13 * zone.pw + 11) as usize];
        assert!(floor.normal[0] > 0 && floor.normal[1] < 0 && floor.normal[2] > 0);
        zone.gset_surface(12, 13, &cell, 1, 0, true, 17.0);
        let face = zone.ground_surfaces[(13 * zone.pw + 12) as usize];
        assert_eq!(face.height, 17);
        assert_eq!(face.normal, [floor.normal[0], floor.normal[2], -floor.normal[1]]);
        zone.gset_surface(14, 13, &cell, 0, 0, true, 20.0);
        zone.gorient_surface(14, 13, [1.0, 0.0]);
        assert_eq!(zone.ground_surfaces[(13 * zone.pw + 14) as usize].normal, [127, 0, 0]);
        assert_eq!(zone.ground, art);
        assert_eq!(zone.tiles, tiles);
    }

    #[test]
    fn revealed_ground_never_inherits_a_cached_prop_plane() {
        let mut zone = Zone::new(ZoneId::Bloomwild, 8, 8);
        zone.ground.fill(rgb(0x929b90));
        zone.props.push(Prop::new("bloomwild_ruins", "pillar_b", 64.0, 80.0));
        let field = Field::bake(&zone, &NoRegion, geometry_key(&zone));
        assert!(field.raised.iter().any(|&v| v), "fixture must include actual raised native art");
        assert!(field.terrain.iter().all(|p| p.normal == [0, 0, 127] && p.height == 0));
        let mut f = Frame::new();
        f.begin(Rgb::BLACK, zone.ambient);
        f.surface_lighting = true;
        f.surfaces[0] = super::super::frame::Surface { normal: [1.0, 0.0, 0.0], height: 9.0, owner: 42, kind: 2 };
        ground_surface(&zone, &NoRegion, &mut f);
        assert_eq!(f.surfaces[0].owner, 42, "actual ground-hook native drawing wins over the terrain fallback");
        assert_eq!(f.surfaces[80 * W + 64].normal, [0.0, 0.0, 1.0]);
        assert_eq!(f.surfaces[80 * W + 64].height, 0.0);
        assert_eq!(f.surfaces[127 * W + 127].kind, 1);
        assert_eq!(f.surfaces[128 * W + 128].kind, 0, "map-edge backdrop has no terrain normal");
    }

    #[test]
    fn physical_night_fill_excludes_cached_point_facing_and_proxy_casts() {
        let make = |source_x| {
            let mut zone = Zone::new(ZoneId::Glasswater, 8, 8);
            zone.ground.fill(rgb(0x929b90));
            zone.props.push(Prop::new("bloomwild_ruins", "pillar_b", 64.0, 80.0));
            zone.lights.push(crate::fx::Light {
                x: source_x, y: 40.0, radius: 128.0, color: rgb(0xffbc78), intensity: 1.0,
            });
            zone
        };
        let left = make(8.0);
        let right = make(120.0);
        let sample = |zone: &Zone, physical: bool, proxy: bool| {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, zone.ambient);
            f.surface_lighting = physical;
            if proxy { f.casters.push(Caster { x: 64.0, y: 110.0, half_w: 5.0, height: 20.0 }); }
            apply(zone, &NoRegion, &mut f, 90);
            f.light
        };
        let physical = sample(&left, true, false);
        assert_eq!(physical, sample(&right, true, false), "moving cached source metadata cannot leave frozen facing or cast shadows in physical fill");
        assert_eq!(physical, sample(&left, true, true), "physical night casts belong to native silhouette visibility, not the old body ellipse");
        assert_ne!(sample(&left, false, false), sample(&right, false, false), "fixture must exercise the legacy source-facing/static-cast terms");
        let field = left.lighting.borrow();
        let field = field.as_ref().unwrap();
        assert!(field.contact.iter().any(|&a| a > 0), "attached AO remains present");
        assert!(field.dark.iter().zip(&field.contact).any(|(cast, contact)| cast > contact), "fixture separates cast darkness from contact darkness");
    }
}
