//! Zones: the tile grid used for collision, a pre-rendered ground image, props, static lights,
//! spawns and the per-zone look. Region-specific life (grass, water, lava, weather) lives in a
//! `Region` implementation next to each zone's builder in `src/regions/`.

pub mod prop;

use crate::combat::Hit;
use crate::entities::EnemyKind;
use crate::fx::{Fx, Light};
use crate::gfx::{Cell, Frame, Grade, H, Rgb, W};
use crate::gfx::frame::PointLight;
pub use prop::Prop;

pub const TILE: i32 = 16;

/// Paint this into `Zone::ground` to leave a hole where the region backdrop shows through
/// (sky below the island edge, the starfield around the summit).
pub const HOLE: Rgb = Rgb { r: -1.0, g: -1.0, b: -1.0 };

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoneId {
    Bloomwild,
    Glasswater,
    Cinder,
    Summit,
}

impl ZoneId {
    pub fn next(self) -> Option<ZoneId> {
        match self {
            ZoneId::Bloomwild => Some(ZoneId::Glasswater),
            ZoneId::Glasswater => Some(ZoneId::Cinder),
            ZoneId::Cinder => Some(ZoneId::Summit),
            ZoneId::Summit => None,
        }
    }
    pub fn parse(s: &str) -> Option<ZoneId> {
        match s.to_ascii_lowercase().as_str() {
            "bloomwild" | "1" => Some(ZoneId::Bloomwild),
            "glasswater" | "2" => Some(ZoneId::Glasswater),
            "cinder" | "3" => Some(ZoneId::Cinder),
            "summit" | "warden" | "4" => Some(ZoneId::Summit),
            _ => None,
        }
    }
}

/// What a tile is for movement. Visuals come from the zone's pre-rendered ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// Walkable ground.
    Floor,
    /// Walkable shallow water (slows a little, ripples).
    Shallow,
    /// Blocks movement: cliffs, rock, ruins.
    Wall,
    /// Blocks movement: deep water.
    Deep,
    /// Blocks movement: lava.
    Lava,
    /// Blocks movement: the sky beyond the island edge.
    Void,
}

impl Tile {
    pub fn solid(self) -> bool {
        !matches!(self, Tile::Floor | Tile::Shallow)
    }
    pub fn liquid(self) -> bool {
        matches!(self, Tile::Shallow | Tile::Deep | Tile::Lava)
    }
}

/// Region-owned fill and cool cast-shadow values, consumed by the static lighting bake.
#[derive(Clone, Copy, Debug)]
pub struct LightingSettings {
    pub fill: Rgb,
    pub shadow_tint: Rgb,
}

/// Authored solid terrain in world X/right, Y/down, Z/up coordinates. Painted
/// height sheets describe visible fronts; these optional masses also retain
/// their real sides where a passage cuts into a bank. Collision stays separate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerrainVolume {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl LightingSettings {
    /// Preserve the selected lighting profiles until a region deliberately tunes its values.
    pub fn for_zone(id: ZoneId) -> Self {
        let fill = match id {
            ZoneId::Bloomwild => crate::gfx::rgb(0x8590ac) * 0.68,
            ZoneId::Glasswater => Rgb::new(0.43, 0.49, 0.60),
            ZoneId::Cinder => Rgb::new(0.47, 0.47, 0.55),
            ZoneId::Summit => Rgb::new(0.61, 0.66, 0.79),
        };
        Self { fill, shadow_tint: Rgb::new(0.48, 0.55, 0.73) }
    }
}

pub struct Zone {
    /// One bake owned by this actual zone; dropping the zone frees its lighting.
    pub(crate) lighting: std::cell::RefCell<Option<crate::gfx::lighting::Field>>,
    pub(crate) lighting_revision: u64,
    pub lighting_settings: LightingSettings,
    pub id: ZoneId,
    /// Shown in the banner when you arrive (e.g. "BLOOMWILD RUINS").
    pub title: String,
    /// Size in tiles.
    pub tw: i32,
    pub th: i32,
    /// Size in pixels.
    pub pw: i32,
    pub ph: i32,
    pub tiles: Vec<Tile>,
    /// Optional native-pixel movement surface, matching rounded shores and authored decks.
    /// The coarse tile grid remains available for map structure and navigation.
    pub surface_tiles: Option<Vec<Tile>>,
    /// Pre-rendered ground albedo, pw*ph. Region builders paint this once at load.
    pub ground: Vec<Rgb>,
    /// Native source or authored geometric normals retained by terrain painters (four bytes per pixel).
    pub(crate) ground_surfaces: Vec<crate::gfx::lighting::TerrainSurface>,
    pub(crate) terrain_volumes: Vec<TerrainVolume>,
    /// Pre-rendered ground emissive (glowing moss, lava cracks...), pw*ph.
    pub ground_emit: Vec<Rgb>,
    pub props: Vec<Prop>,
    /// Static lights (world coords).
    pub lights: Vec<Light>,
    pub enemies: Vec<(EnemyKind, f32, f32)>,
    pub player_start: (f32, f32),
    /// Where the prism shard sits (picking it up opens the gate). None = gate starts open.
    pub shard: Option<(f32, f32)>,
    /// Exit gate position (anchor, world coords). Walking into the open gate leaves the zone.
    pub gate: Option<(f32, f32)>,
    /// Ambient light color (multiplies albedo). White = palette colors exactly as drawn.
    pub ambient: Rgb,
    /// Color for anything outside the map.
    pub bg: Rgb,
    pub grade: Grade,
    /// Music track stem under assets/music.
    pub music: String,
    /// Does this zone host the Warden fight?
    pub boss: bool,
    /// Where the Warden stands (feet, world coords), if this zone hosts it.
    pub boss_pos: Option<(f32, f32)>,
}

impl Zone {
    pub fn new(id: ZoneId, tw: i32, th: i32) -> Zone {
        let (pw, ph) = (tw * TILE, th * TILE);
        Zone {
            lighting: std::cell::RefCell::new(None),
            lighting_revision: 0,
            lighting_settings: LightingSettings::for_zone(id),
            id,
            title: String::new(),
            tw,
            th,
            pw,
            ph,
            tiles: vec![Tile::Floor; (tw * th) as usize],
            surface_tiles: None,
            ground: vec![Rgb::BLACK; (pw * ph) as usize],
            ground_surfaces: vec![crate::gfx::lighting::TerrainSurface::default(); (pw * ph) as usize],
            terrain_volumes: Vec::new(),
            ground_emit: vec![Rgb::BLACK; (pw * ph) as usize],
            props: Vec::new(),
            lights: Vec::new(),
            enemies: Vec::new(),
            player_start: (pw as f32 / 2.0, ph as f32 / 2.0),
            shard: None,
            gate: None,
            ambient: Rgb::WHITE,
            bg: Rgb::BLACK,
            grade: Grade::default(),
            music: String::new(),
            boss: false,
            boss_pos: None,
        }
    }

    /// Invalidate after bulk edits to public ground, emission or tile buffers; setters do this automatically.
    pub fn invalidate_lighting(&mut self) {
        self.lighting_revision = self.lighting_revision.wrapping_add(1);
        *self.lighting.get_mut() = None;
    }

    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if tx < 0 || ty < 0 || tx >= self.tw || ty >= self.th { Tile::Void } else { self.tiles[(ty * self.tw + tx) as usize] }
    }
    pub fn set_tile(&mut self, tx: i32, ty: i32, t: Tile) {
        if tx >= 0 && ty >= 0 && tx < self.tw && ty < self.th {
            self.tiles[(ty * self.tw + tx) as usize] = t;
            if let Some(surface) = &mut self.surface_tiles {
                for y in ty * TILE..(ty + 1) * TILE {
                    let start = (y * self.pw + tx * TILE) as usize;
                    surface[start..start + TILE as usize].fill(t);
                }
            }
            self.invalidate_lighting();
        }
    }
    pub fn tile_at(&self, wx: f32, wy: f32) -> Tile {
        if let Some(surface) = &self.surface_tiles {
            let (x, y) = (wx.floor() as i32, wy.floor() as i32);
            if x < 0 || y < 0 || x >= self.pw || y >= self.ph { return Tile::Void; }
            surface[(y * self.pw + x) as usize]
        } else {
            self.tile((wx / TILE as f32).floor() as i32, (wy / TILE as f32).floor() as i32)
        }
    }
    /// Install the finished painter's physical material classification on the native grid.
    pub fn set_surface_tiles(&mut self, pixels: Vec<Tile>) {
        assert_eq!(pixels.len(), (self.pw * self.ph) as usize, "surface must cover the native zone");
        self.surface_tiles = Some(pixels);
        self.invalidate_lighting();
    }
    pub fn solid_tile_at(&self, wx: f32, wy: f32) -> bool {
        self.tile_at(wx, wy).solid()
    }
    pub fn liquid_at(&self, wx: f32, wy: f32) -> bool {
        self.tile_at(wx, wy).liquid()
    }

    /// Ground pixel helpers for region builders.
    #[inline]
    pub fn gset(&mut self, x: i32, y: i32, c: Rgb) {
        if x >= 0 && y >= 0 && x < self.pw && y < self.ph {
            let i = (y * self.pw + x) as usize;
            self.ground[i] = c;
            if c.r < 0.0 { self.ground_surfaces[i] = crate::gfx::lighting::TerrainSurface::default(); }
            self.invalidate_lighting();
        }
    }
    /// Retain the normal at the exact native UV copied by a terrain painter.
    /// OpenGL source axes are right/up/toward-viewer; floor and upright art
    /// have distinct world bases. Color and collision are never changed here.
    #[allow(clippy::too_many_arguments)]
    pub fn gset_surface(&mut self, x: i32, y: i32, cell: &crate::gfx::Cell, sx: i32, sy: i32, upright: bool, height: f32) {
        if x < 0 || y < 0 || x >= self.pw || y >= self.ph || sx < 0 || sy < 0 || sx >= cell.w || sy >= cell.h { return; }
        let Some(pixel) = cell.surface.get((sy * cell.w + sx) as usize) else { return };
        if !pixel.a { return; }
        let [nx, ny, nz] = pixel.n.map(|n| n as f32 / 127.0);
        let normal = if upright { [nx, nz, ny] } else { [nx, -ny, nz] };
        let mut surface = crate::gfx::lighting::TerrainSurface::new(normal, height);
        surface.upright = upright;
        self.ground_surfaces[(y * self.pw + x) as usize] = surface;
        self.invalidate_lighting();
    }
    /// Rotate an upright material's front axis around a curved authored wall.
    /// `front` is its horizontal world-space outward normal, never an RGB gradient.
    pub fn gorient_surface(&mut self, x: i32, y: i32, front: [f32; 2]) {
        if x < 0 || y < 0 || x >= self.pw || y >= self.ph { return; }
        let i = (y * self.pw + x) as usize;
        let surface = self.ground_surfaces[i];
        if !surface.present() { return; }
        let [nx, ny, nz] = surface.normal.map(|n| n as f32 / 127.0);
        let length = front[0].hypot(front[1]).max(0.001);
        let [fx, fy] = front.map(|n| n / length);
        let mut rotated = crate::gfx::lighting::TerrainSurface::new(
            [nx * fy + ny * fx, ny * fy - nx * fx, nz], surface.height as f32);
        rotated.upright = surface.upright;
        self.ground_surfaces[i] = rotated;
        self.invalidate_lighting();
    }
    /// Retain an explicitly authored geometric face where there is no native normal atlas.
    pub fn gset_plane_surface(&mut self, x: i32, y: i32, normal: [f32; 3], height: f32) {
        if x < 0 || y < 0 || x >= self.pw || y >= self.ph { return; }
        self.ground_surfaces[(y * self.pw + x) as usize] = crate::gfx::lighting::TerrainSurface::new(normal, height);
        self.invalidate_lighting();
    }
    /// Complete painted walkable geometry without replacing exact source normals.
    /// Call after installing the painter's native movement mask. Material relief
    /// belongs to the painter; this helper never derives geometry from albedo.
    pub(crate) fn fill_walkable_surfaces(&mut self, mut sample: impl FnMut(i32, i32) -> ([f32; 3], f32)) {
        let mut changed = false;
        for y in 0..self.ph {
            for x in 0..self.pw {
                let i = (y * self.pw + x) as usize;
                if self.ground[i].r < 0.0 || self.ground_surfaces[i].present()
                    || self.tile_at(x as f32, y as f32).solid() { continue; }
                let (normal, height) = sample(x, y);
                self.ground_surfaces[i] = crate::gfx::lighting::TerrainSurface::new(normal, height);
                changed = true;
            }
        }
        if changed { self.invalidate_lighting(); }
    }
    #[inline]
    pub fn gget(&self, x: i32, y: i32) -> Rgb {
        if x >= 0 && y >= 0 && x < self.pw && y < self.ph { self.ground[(y * self.pw + x) as usize] } else { self.bg }
    }
    #[inline]
    pub fn eset(&mut self, x: i32, y: i32, c: Rgb) {
        if x >= 0 && y >= 0 && x < self.pw && y < self.ph {
            self.ground_emit[(y * self.pw + x) as usize] = c;
            self.invalidate_lighting();
        }
    }

    /// Does an entity's feet box (centered at x, y; half-width r, half-height r*0.6) hit anything solid?
    pub fn blocked(&self, x: f32, y: f32, r: f32) -> bool {
        let hw = r;
        let hh = r * 0.6;
        if let Some(surface) = &self.surface_tiles {
            let (x0, x1) = ((x - hw).floor() as i32, (x + hw).ceil() as i32);
            let (y0, y1) = ((y - hh).floor() as i32, (y + hh).ceil() as i32);
            if x0 < 0 || y0 < 0 || x1 > self.pw || y1 > self.ph { return true; }
            for py in y0..y1 {
                if surface[(py * self.pw + x0) as usize..(py * self.pw + x1) as usize].iter().any(|t| t.solid()) {
                    return true;
                }
            }
        } else {
            let x0 = ((x - hw) / TILE as f32).floor() as i32;
            let x1 = ((x + hw - 0.01) / TILE as f32).floor() as i32;
            let y0 = ((y - hh) / TILE as f32).floor() as i32;
            let y1 = ((y + hh - 0.01) / TILE as f32).floor() as i32;
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    if self.tile(tx, ty).solid() { return true; }
                }
            }
        }
        for p in &self.props {
            if p.blocks(x, y, hw, hh) { return true; }
        }
        false
    }

    /// Move a feet box by (dx, dy), sliding along walls. Returns the new position and whether
    /// anything blocked the move.
    pub fn move_box(&self, x: f32, y: f32, r: f32, dx: f32, dy: f32) -> (f32, f32, bool) {
        let mut hit = false;
        let steps = ((dx.abs().max(dy.abs())) / 2.0).ceil().max(1.0) as i32;
        let (sx, sy) = (dx / steps as f32, dy / steps as f32);
        let (mut cx, mut cy) = (x, y);
        for _ in 0..steps {
            if sx != 0.0 {
                if !self.blocked(cx + sx, cy, r) {
                    cx += sx;
                } else {
                    hit = true;
                }
            }
            if sy != 0.0 {
                if !self.blocked(cx, cy + sy, r) {
                    cy += sy;
                } else {
                    hit = true;
                }
            }
        }
        (cx, cy, hit)
    }

    /// Clear straight line between two points (for enemy sight / shots)?
    pub fn line_clear(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
        let d = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let n = (d / 6.0).ceil().max(1.0) as i32;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let t2 = self.tile_at(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            if t2 == Tile::Wall {
                return false;
            }
        }
        true
    }

    /// Blit the visible part of the pre-rendered ground into the frame. Pixels outside the map
    /// and `HOLE` pixels are left alone, so whatever the region's backdrop drew shows through.
    pub fn draw_ground(&self, f: &mut Frame) {
        for sy in 0..H as i32 {
            let wy = sy + f.cam_y;
            if wy < 0 || wy >= self.ph {
                continue;
            }
            let row = sy as usize * W;
            let grow = (wy * self.pw) as usize;
            for sx in 0..W as i32 {
                let wx = sx + f.cam_x;
                if wx < 0 || wx >= self.pw {
                    continue;
                }
                let g = self.ground[grow + wx as usize];
                if g.r < 0.0 {
                    continue;
                }
                let i = row + sx as usize;
                f.albedo[i] = g;
                let e = self.ground_emit[grow + wx as usize];
                if e.r + e.g + e.b > 0.0 {
                    f.emissive[i] += e;
                }
            }
        }
    }
}

/// What a region's hooks can see each tick.
pub struct RegionCtx<'a> {
    pub t: u64,
    pub player: (f32, f32),
    /// Positions of things that move through the world (player, enemies) with their speed.
    pub movers: &'a [(f32, f32, f32)],
    pub cam: (i32, i32),
}

/// Actual rendered native ink that may occlude a live light. Collision proxies are separate.
/// The bottom opaque source row supplies its ground base, including atlas gutters.
pub struct LightOccluder<'a> {
    pub cell: &'a Cell,
    pub anchor: (i32, i32),
    pub x: f32,
    pub y: f32,
    pub flip: bool,
    /// This exact emitter belongs to the sprite and must not shadow itself.
    pub own_light: Option<PointLight>,
}

/// Region-specific life. Every hook has a default no-op, so implement what you need.
/// Items returned by `sorted_items` are y-sorted with characters and props; the engine calls
/// `draw_item` back for each in order.
pub trait Region {
    fn update(&mut self, _zone: &mut Zone, _ctx: &RegionCtx, _fx: &mut Fx) {}
    /// A player attack happened this tick (cut grass, ring crystals...).
    fn slash(&mut self, _zone: &mut Zone, _hit: &Hit, _fx: &mut Fx) {}
    /// After the ground blit and debris, before characters: animated water, lava, decals.
    fn draw_ground(&self, _zone: &Zone, _f: &mut Frame, _t: u64) {}
    /// Y-sorted region items on screen: push (sort_y, id).
    fn sorted_items(&self, _zone: &Zone, _f: &Frame, _out: &mut Vec<(f32, u32)>) {}
    fn draw_item(&self, _zone: &Zone, _f: &mut Frame, _id: u32, _t: u64) {}
    /// Expose region-owned rendered cells; physical solidity does not decide light occlusion.
    fn light_occluders<'a>(&'a self, _t: u64, _out: &mut Vec<LightOccluder<'a>>) {}
    /// After characters and canopies: falling petals, embers, fog, god rays.
    fn draw_overlay(&self, _zone: &Zone, _f: &mut Frame, _t: u64) {}
    /// After ambient and point lights are in `f.light`: cloud shadows, light shafts, darkness.
    fn light(&self, _zone: &Zone, _f: &mut Frame, _t: u64) {}
    /// Background seen beyond the map edge / in void tiles, drawn before the ground
    /// (sea of clouds, starfield). Default: nothing.
    fn draw_backdrop(&self, _zone: &Zone, _f: &mut Frame, _t: u64) {}
    /// Exposure of actual background pixels, after nearby world lights are excluded.
    fn backdrop_light(&self, ambient: Rgb) -> Rgb { ambient }
}

pub struct NoRegion;
impl Region for NoRegion {}

#[cfg(test)]
mod surface_tests {
    use super::*;
    use crate::entities::{Player, player::PlayerCtx};
    use crate::input::{Input, Raw};

    #[test]
    fn walkable_normal_completion_obeys_native_collision_and_preserves_painted_surfaces() {
        let mut zone = Zone::new(ZoneId::Bloomwild, 2, 1);
        zone.ground.fill(Rgb::WHITE);
        zone.set_tile(1, 0, Tile::Wall);
        let mut physical = vec![Tile::Void; zone.ground.len()];
        for (x, tile) in [(1, Tile::Floor), (2, Tile::Shallow), (3, Tile::Wall),
            (4, Tile::Deep), (5, Tile::Lava), (7, Tile::Floor), (8, Tile::Floor), (17, Tile::Floor)] {
            physical[x] = tile;
        }
        zone.set_surface_tiles(physical);
        zone.gset(7, 0, HOLE);
        zone.gset_plane_surface(8, 0, [-0.4, 0.2, 1.0], 3.0);
        let source = zone.ground_surfaces[8];
        let albedo = zone.ground.clone();
        let native = zone.surface_tiles.clone();
        let coarse = zone.tiles.clone();
        let revision = zone.lighting_revision;
        let mut sampled = Vec::new();
        zone.fill_walkable_surfaces(|x, y| {
            sampled.push((x, y));
            ([0.3, -0.2, 1.0], 0.0)
        });
        assert_eq!(sampled, [(1, 0), (2, 0), (17, 0)], "precise floor and shallows win over coarse walls; holes and blocked terrain are excluded");
        assert_eq!(zone.ground_surfaces[8], source, "authored native geometry survives completion");
        for (i, surface) in zone.ground_surfaces.iter().enumerate() {
            assert_eq!(surface.present(), [1, 2, 8, 17].contains(&i));
        }
        assert_eq!(zone.ground, albedo);
        assert_eq!(zone.surface_tiles, native);
        assert_eq!(zone.tiles, coarse);
        assert_eq!(zone.lighting_revision, revision + 1, "bulk completion invalidates the bake once");
        zone.fill_walkable_surfaces(|_, _| panic!("completed geometry must not be sampled again"));
        assert_eq!(zone.lighting_revision, revision + 1);
    }

    #[test]
    fn native_surface_uses_full_half_open_feet_and_preserves_coarse_map() {
        let mut zone = Zone::new(ZoneId::Glasswater, 12, 12);
        let mut surface = vec![Tile::Floor; (zone.pw * zone.ph) as usize];
        for y in 0..zone.ph {
            for x in 100..zone.pw { surface[(y * zone.pw + x) as usize] = Tile::Deep; }
        }
        zone.set_surface_tiles(surface);
        assert_eq!(zone.tile(6, 3), Tile::Floor);
        assert_eq!(zone.tile_at(100.0, 50.0), Tile::Deep);
        assert_eq!(zone.tile_at(99.999, 50.0), Tile::Floor);
        assert_eq!(zone.tile_at(-0.01, 50.0), Tile::Void);
        assert_eq!(zone.tile_at(10.0, -0.01), Tile::Void);
        assert_eq!(zone.tile_at(zone.pw as f32, 50.0), Tile::Void);
        assert!(!zone.blocked(96.0, 50.0, 4.0), "exact contact is allowed");
        assert!(zone.blocked(96.01, 50.0, 4.0), "one fraction over the pixel shore is blocked");
        assert!(zone.blocked(3.99, 50.0, 4.0));
        zone.set_tile(7, 3, Tile::Shallow);
        assert_eq!(zone.tile_at(112.0, 50.0), Tile::Shallow, "later physical tile edits update both surfaces");
    }

    #[test]
    fn real_player_stops_at_painted_shore_inside_walkable_coarse_tile() {
        let mut zone = Zone::new(ZoneId::Glasswater, 12, 12);
        let mut surface = vec![Tile::Floor; (zone.pw * zone.ph) as usize];
        for y in 0..zone.ph {
            // A curved shore steps at native pixels, not at 16px tile boundaries.
            let edge = 113 + (y / 8) % 4;
            for x in edge..zone.pw { surface[(y * zone.pw + x) as usize] = Tile::Deep; }
        }
        zone.set_surface_tiles(surface);
        let mut player = Player::new(80.0, 100.0);
        let mut input = Input::default();
        let mut fx = Fx::new(7);
        for _ in 0..90 {
            input.step(Raw { mv: (1.0, 0.0), ..Raw::default() });
            player.update(&input, &mut PlayerCtx { zone: &zone, fx: &mut fx, hits: &mut Vec::new() });
            fx.tick();
        }
        assert_eq!(zone.tile(7, 6), Tile::Floor);
        assert!(player.body.x > 107.0 && player.body.x + player.body.r <= 113.001,
            "player should stop at the actual shore, ended {:?}", (player.body.x, player.body.y));
    }

    #[test]
    fn coarse_collision_remains_available_without_surface() {
        let mut zone = Zone::new(ZoneId::Bloomwild, 12, 12);
        zone.set_tile(7, 3, Tile::Wall);
        assert_eq!(zone.tile_at(-0.01, 50.0), Tile::Void);
        assert!(!zone.blocked(108.0, 50.0, 4.0));
        assert!(zone.blocked(108.01, 50.0, 4.0));
    }
}
