//! Zones: the tile grid used for collision, a pre-rendered ground image, props, static lights,
//! spawns and the per-zone look. Region-specific life (grass, water, lava, weather) lives in a
//! `Region` implementation next to each zone's builder in `src/regions/`.

pub mod prop;

use crate::combat::Hit;
use crate::entities::EnemyKind;
use crate::fx::{Fx, Light};
use crate::gfx::{Frame, Grade, H, Rgb, W};
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
    /// Pre-rendered ground albedo, pw*ph. Region builders paint this once at load.
    pub ground: Vec<Rgb>,
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
            ground: vec![Rgb::BLACK; (pw * ph) as usize],
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
            self.invalidate_lighting();
        }
    }
    pub fn tile_at(&self, wx: f32, wy: f32) -> Tile {
        self.tile((wx / TILE as f32).floor() as i32, (wy / TILE as f32).floor() as i32)
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
            self.ground[(y * self.pw + x) as usize] = c;
            self.invalidate_lighting();
        }
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
        let x0 = ((x - hw) / TILE as f32).floor() as i32;
        let x1 = ((x + hw - 0.01) / TILE as f32).floor() as i32;
        let y0 = ((y - hh) / TILE as f32).floor() as i32;
        let y1 = ((y + hh - 0.01) / TILE as f32).floor() as i32;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.tile(tx, ty).solid() {
                    return true;
                }
            }
        }
        for p in &self.props {
            if let Some((fw, fh)) = p.footprint
                && p.alive
                && (x - p.x).abs() < hw + fw * 0.5
                && (y - (p.y - fh * 0.5)).abs() < hh + fh * 0.5
            {
                return true;
            }
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
    /// After characters and canopies: falling petals, embers, fog, god rays.
    fn draw_overlay(&self, _zone: &Zone, _f: &mut Frame, _t: u64) {}
    /// After ambient and point lights are in `f.light`: cloud shadows, light shafts, darkness.
    fn light(&self, _zone: &Zone, _f: &mut Frame, _t: u64) {}
    /// Background seen beyond the map edge / in void tiles, drawn before the ground
    /// (sea of clouds, starfield). Default: nothing.
    fn draw_backdrop(&self, _zone: &Zone, _f: &mut Frame, _t: u64) {}
}

pub struct NoRegion;
impl Region for NoRegion {}
