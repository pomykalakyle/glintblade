//! Bloomwild Ruins: white-stone ruins overgrown on a sky island in clear daylight.
//!
//! Everything here is drawn in code on the native pixel grid, except props the region's props
//! artist delivers as sprite sheets (placed by name, skipped if missing).
//! - `map`: the hand-laid tile map and the route through it.
//! - `paint`: the pre-rendered ground (grass, flagstones, slabs, water beds, cliffs, the island's
//!   south face).
//! - `ruins`: walls, pillars, the arch, boulders and blossom trees, placed from artist sheets.
//! - `grass`: cuttable, swaying grass tufts.
//! - `sky`: the sea of clouds below the island (parallax backdrop).
//! - `life`: water currents and waterfalls, without additional atmosphere.

mod grass;
mod life;
mod map;
mod normals;
mod paint;
mod pal;
mod ruins;
mod sky;

use crate::combat::{Hit, Team};
use crate::entities::EnemyKind;
use crate::fx::Fx;
use crate::gfx::{Frame, Grade, Rgb};
use crate::world::{Region, RegionCtx, TILE, Tile, Zone, ZoneId};

/// What a map tile is, before it becomes collision + pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum K {
    Void,
    Grass,
    Tall,
    Path,
    Slab,
    Altar,
    Cliff,
    Niche,
    Deep,
    Shallow,
    Bridge,
    Stairs,
    Falls,
}

/// Pixel materials (what `paint` put at each ground pixel).
pub mod mat {
    pub const VOID: u8 = 0;
    pub const GRASS: u8 = 1;
    pub const PATH: u8 = 2;
    pub const SLAB: u8 = 3;
    pub const CLIFF: u8 = 4;
    pub const EDGE: u8 = 5;
    pub const DEEP: u8 = 6;
    pub const SHALLOW: u8 = 7;
    pub const BRIDGE: u8 = 8;
    pub const STAIRS: u8 = 9;
    pub const FALLS: u8 = 10;
    pub const BIGFALLS: u8 = 11;
    pub const DIRT: u8 = 12;
    pub const BANK: u8 = 13;
    pub fn water(m: u8) -> bool {
        matches!(m, DEEP | SHALLOW | FALLS | BIGFALLS)
    }
}

pub struct Map {
    pub tw: i32,
    pub th: i32,
    pub k: Vec<K>,
    /// 1 = upper terrace (above the cliff band), 0 = lower ground.
    pub level: Vec<u8>,
    /// Object letters from the map: (char, tile x, tile y).
    pub marks: Vec<(char, i32, i32)>,
}

impl Map {
    fn parse() -> Map {
        let th = map::MAP.len() as i32;
        let tw = map::MAP.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
        let mut k = vec![K::Void; (tw * th) as usize];
        let mut marks = Vec::new();
        for (y, row) in map::MAP.iter().enumerate() {
            assert_eq!(row.len() as i32, tw, "bloomwild map row {y} has the wrong width");
            for (x, ch) in row.chars().enumerate() {
                let kind = match ch {
                    '.' => K::Grass,
                    ';' => K::Tall,
                    '=' => K::Path,
                    'o' => K::Slab,
                    'A' => K::Altar,
                    '#' => K::Cliff,
                    'n' => K::Niche,
                    '~' => K::Deep,
                    '-' => K::Shallow,
                    'b' => K::Bridge,
                    's' => K::Stairs,
                    'F' => K::Falls,
                    ' ' => K::Void,
                    _ => {
                        marks.push((ch, x as i32, y as i32));
                        K::Grass
                    }
                };
                k[y * tw as usize + x] = kind;
            }
        }
        let mut m = Map {
            tw,
            th,
            k,
            level: vec![0; (tw * th) as usize],
            marks,
        };
        // Object tiles sit on whatever floor surrounds them.
        for &(_, x, y) in &m.marks.clone() {
            let mut counts = [0; 4];
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                match m.at(x + dx, y + dy) {
                    K::Slab | K::Altar => counts[1] += 1,
                    K::Path => counts[2] += 1,
                    K::Tall => counts[3] += 1,
                    _ => counts[0] += 1,
                }
            }
            let best = (0..4).max_by_key(|&i| counts[i]).unwrap_or(0);
            m.k[(y * tw + x) as usize] = [K::Grass, K::Slab, K::Path, K::Tall][best];
        }
        for x in 0..tw {
            let top = (0..th).find(|&y| matches!(m.at(x, y), K::Cliff | K::Niche | K::Stairs | K::Falls));
            if let Some(top) = top {
                for y in 0..top {
                    m.level[(y * tw + x) as usize] = 1;
                }
            }
        }
        m
    }

    pub fn at(&self, tx: i32, ty: i32) -> K {
        if tx < 0 || ty < 0 || tx >= self.tw || ty >= self.th {
            K::Void
        } else {
            self.k[(ty * self.tw + tx) as usize]
        }
    }
    pub fn level(&self, tx: i32, ty: i32) -> u8 {
        if tx < 0 || ty < 0 || tx >= self.tw || ty >= self.th {
            0
        } else {
            self.level[(ty * self.tw + tx) as usize]
        }
    }
    pub fn marks_of(&self, c: char) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.marks.iter().filter(move |m| m.0 == c).map(|m| (m.1, m.2))
    }
}

fn tile_of(k: K) -> Tile {
    match k {
        K::Void => Tile::Void,
        K::Cliff | K::Niche | K::Falls => Tile::Wall,
        K::Deep => Tile::Deep,
        K::Shallow => Tile::Shallow,
        _ => Tile::Floor,
    }
}

/// World pixel center of a tile.
pub fn tc(tx: i32, ty: i32) -> (f32, f32) {
    ((tx * TILE + TILE / 2) as f32, (ty * TILE + TILE / 2) as f32)
}

pub struct Bloom {
    pub map: Map,
    pub mat: Vec<u8>,
    pub sky: sky::Sky,
    pub grass: grass::Grass,
    pub life: life::Life,
    pub canopies: Vec<crate::world::Prop>,
    pub player: (f32, f32),
    pub t: u64,
}

pub fn build() -> (Zone, Box<dyn Region>) {
    let map = Map::parse();
    let mut z = Zone::new(ZoneId::Bloomwild, map.tw, map.th);
    z.title = "BLOOMWILD RUINS".to_string();
    z.music = "bloomwild".to_string();
    z.bg = pal::CLOUD[0];
    z.ambient = Rgb::WHITE;
    z.grade = Grade {
        light_steps: 32.0,
        vignette: 0.07,
        bloom: 0.3,
        bloom_threshold: 1.1,
        ..Grade::default()
    };
    for ty in 0..map.th {
        for tx in 0..map.tw {
            z.set_tile(tx, ty, tile_of(map.at(tx, ty)));
        }
    }

    let mut mat = paint::paint(&mut z, &map);
    z.set_surface_tiles(paint::physical_surface(&z, &mat));
    normals::apply(&mut z, &map, &mat);
    let canopies = ruins::build(&mut z, &map, &mat);
    let grass = grass::Grass::build(&map, &mat, &z);
    let life = life::Life::build(&map, &mut mat, &mut z);
    let sky = sky::Sky::build();

    z.player_start = tc(9, 43);
    z.shard = Some((paint::ALTAR.0, paint::ALTAR.1));
    z.gate = Some(((25 * TILE + 8) as f32, (3 * TILE + 13) as f32));

    // Enemies, in groups along the route.
    let spawns: &[(EnemyKind, i32, i32)] = &[
        // meadow courtyard: sproutlings hiding in the tall grass
        (EnemyKind::Sproutling, 24, 40),
        (EnemyKind::Sproutling, 30, 42),
        // The native broken pillar's plinth reaches farther west than the old box.
        (EnemyKind::Sproutling, 32, 38),
        // inside the broken courtyard walls
        (EnemyKind::ThornImp, 28, 28),
        // east temple ruins
        (EnemyKind::ThornImp, 64, 28),
        (EnemyKind::ThornImp, 71, 29),
        (EnemyKind::Sproutling, 62, 33),
        // blossom grove on the upper terrace
        (EnemyKind::ThornImp, 58, 10),
        (EnemyKind::Sproutling, 55, 12),
        // Leave the full visible root spread of the grove tree clear.
        (EnemyKind::Sproutling, 60, 13),
        // plaza guard around the altar
        (EnemyKind::ThornImp, 20, 9),
        (EnemyKind::ThornImp, 30, 12),
        (EnemyKind::Sproutling, 25, 15),
    ];
    for &(k, tx, ty) in spawns {
        let (x, y) = tc(tx, ty);
        z.enemies.push((k, x, y));
    }

    let region = Bloom {
        map,
        mat,
        sky,
        grass,
        life,
        canopies,
        player: z.player_start,
        t: 0,
    };
    (z, Box::new(region))
}

/// The title looks across a broad headland toward the rest of the archipelago.
/// Land continues beyond the right/bottom of the view instead of ending as a tiny pedestal.
pub fn title_garden() -> (Zone, Box<dyn Region>) {
    let mut map = Map::parse();
    map.k.fill(K::Void);
    map.level.fill(0);
    map.marks.clear();
    for y in 5..map.th {
        let left = match y {
            5 => 23, 6 => 20, 7 => 16, 8 => 13, 9 => 10,
            10 => 11, 11 => 12, 12 => 13, _ => 14,
        };
        for x in left..map.tw {
            map.k[(y * map.tw + x) as usize] = K::Grass;
        }
    }
    // A weathered route leads into the headland, rather than terminating under the ruin.
    for x in 14..map.tw { map.k[(9 * map.tw + x) as usize] = K::Path; }
    let mut z = Zone::new(ZoneId::Bloomwild, map.tw, map.th);
    z.bg = pal::CLOUD[0];
    z.ambient = Rgb::WHITE;
    z.grade = Grade { vignette: 0.05, bloom: 0.3, bloom_threshold: 1.1, light_steps: 32.0, ..Grade::default() };
    for y in 0..map.th {
        for x in 0..map.tw { z.set_tile(x, y, tile_of(map.at(x, y))); }
    }
    let mut mat = paint::paint(&mut z, &map);
    let canopies = ruins::title_garden(&mut z, &mat);
    z.set_surface_tiles(paint::physical_surface(&z, &mat));
    normals::apply(&mut z, &map, &mat);
    let grass = grass::Grass::build(&map, &mat, &z);
    let life = life::Life::build(&map, &mut mat, &mut z);
    z.player_start = (222.0, 147.0);
    let region = Bloom { map, mat, sky: sky::Sky::title(), grass, life, canopies,
        player: z.player_start, t: 0 };
    (z, Box::new(region))
}

impl Region for Bloom {
    fn update(&mut self, zone: &mut Zone, ctx: &RegionCtx, fx: &mut Fx) {
        self.t = ctx.t;
        self.player = ctx.player;
        let _ = fx;
        self.grass.update(ctx);
        self.life.update(zone, &self.mat, ctx);
    }

    fn slash(&mut self, zone: &mut Zone, hit: &Hit, fx: &mut Fx) {
        if hit.team != Team::Player {
            return;
        }
        self.grass.slash(hit, fx);
        self.life.slash(zone, &self.mat, hit);
    }

    fn draw_backdrop(&self, zone: &Zone, f: &mut Frame, t: u64) {
        self.sky.draw(zone, f, t);
    }

    fn draw_ground(&self, zone: &Zone, f: &mut Frame, t: u64) {
        self.life.draw_water(zone, &self.mat, f, t);
        self.grass.draw_stubs(f);
    }

    fn sorted_items(&self, _zone: &Zone, f: &Frame, out: &mut Vec<(f32, u32)>) {
        self.grass.items(f, out);
    }

    fn draw_item(&self, _zone: &Zone, f: &mut Frame, id: u32, t: u64) {
        self.grass.draw(f, id, t);
    }

    fn draw_item_revealed(&self, _zone: &Zone, f: &mut Frame, id: u32, t: u64, reveal: Option<crate::gfx::sprite::SpriteFocus>) {
        self.grass.draw_revealed(f, id, t, reveal);
    }

    /// Draw the separated tree crowns and arch span with accurate player occlusion.
    fn draw_overlay(&self, zone: &Zone, f: &mut Frame, t: u64) {
        self.life.draw_foot_contact(zone, &self.mat, f, self.player, t);
        ruins::draw_canopies(&self.canopies, f, Some(self.player), t, None);
    }

    fn draw_overlay_revealed(&self, zone: &Zone, f: &mut Frame, t: u64, focus: Option<(f32, f32)>, reveal: Option<crate::gfx::sprite::SpriteFocus>) {
        self.life.draw_foot_contact(zone, &self.mat, f, self.player, t);
        ruins::draw_canopies(&self.canopies, f, focus, t, reveal);
    }

    /// Keep magical point lights restrained against full daylight rather than bleaching the stone.
    fn light(&self, _zone: &Zone, f: &mut Frame, _t: u64) {
        for l in &mut f.light {
            l.r = l.r.min(1.12);
            l.g = l.g.min(1.12);
            l.b = l.b.min(1.12);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Shape;
    use std::collections::VecDeque;

    #[test]
    fn title_headland_continues_into_the_world_and_keeps_glint_on_clear_ground() {
        let (z, _) = title_garden();
        assert_eq!(z.tile_at(1200.0, 300.0), Tile::Floor);
        assert_eq!(z.tile_at(1200.0, 780.0), Tile::Floor);
        assert_eq!(z.tile_at(380.0, 150.0), Tile::Floor);
        assert_eq!(z.tile_at(260.0, 210.0), Tile::Floor);
        assert!(!z.blocked(z.player_start.0, z.player_start.1, 4.0));
        assert_eq!(z.tile_at(140.0, 145.0), Tile::Void);
        for p in &z.props {
            let Some([x0, y0, x1, y1]) = p.collision_bounds() else { continue; };
            for y in y0.floor() as i32..y1.ceil() as i32 {
                for x in x0.floor() as i32..x1.ceil() as i32 {
                    if p.blocks(x as f32 + 0.5, y as f32 + 0.5, 0.01, 0.01) {
                        assert_eq!(z.tile_at(x as f32, y as f32), Tile::Floor,
                            "title scenery must place its entire native foot on the headland");
                    }
                }
            }
        }
    }

    /// Scenery's baked shadows cannot turn physical sky cuts back into visible meadow.
    #[test]
    fn completed_scene_preserves_eroded_sky_cuts() {
        let map = Map::parse();
        let mut terrain = Zone::new(ZoneId::Bloomwild,map.tw,map.th);
        let mask = paint::paint(&mut terrain,&map);
        let (z, _) = build();
        let mut cuts = 0;
        for y in 0..z.ph {
            for x in 0..z.pw {
                let i = (y*z.pw+x) as usize;
                // A rounded top may expose attached vertical rock: EDGE is correctly
                // visible but unwalkable. True sky, including designated eroded turf,
                // must stay transparent after all scenery/contact shadow placement.
                if mask[i] == mat::VOID || mask[i] == mat::GRASS && terrain.ground[i].r < 0.0 {
                    cuts += 1;
                    assert!(z.gget(x, y).r < 0.0, "scenery repainted solid-looking ground over sky at {x},{y}");
                }
            }
        }
        assert!(cuts > 3000, "exercise the actual rounded and eroded perimeter");
    }

    /// Keep every deep-water hazard visibly water even at rounded basin corners.
    #[test]
    fn deep_water_art_covers_the_collision_mask() {
        let map = Map::parse();
        let mut z = Zone::new(ZoneId::Bloomwild, map.tw, map.th);
        let mask = paint::paint(&mut z, &map);
        for y in 0..z.ph {
            for x in 0..z.pw {
                if map.at(x / TILE, y / TILE) == K::Deep {
                    assert_eq!(
                        mask[(y * z.pw + x) as usize],
                        mat::DEEP,
                        "deep hazard painted as another material at {x},{y}"
                    );
                }
            }
        }
    }

    /// Check the actual player footprint can reach both crossings, the shard and the gate.
    #[test]
    fn route_reaches_landmarks_with_solid_props() {
        let (z, _) = build();
        let step = 4;
        let (w, h) = (z.pw / step, z.ph / step);
        let mut seen = vec![false; (w * h) as usize];
        let sx = z.player_start.0 as i32 / step;
        let sy = z.player_start.1 as i32 / step;
        let mut todo = VecDeque::from([(sx, sy)]);
        seen[(sy * w + sx) as usize] = true;
        while let Some((x, y)) = todo.pop_front() {
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let i = (ny * w + nx) as usize;
                if seen[i] || z.blocked((nx * step) as f32, (ny * step) as f32, 5.0) {
                    continue;
                }
                seen[i] = true;
                todo.push_back((nx, ny));
            }
        }
        for (name, (x, y)) in [
            ("lower bridge", tc(52, 36)),
            ("east stairs", tc(70, 20)),
            ("upper bridge", tc(44, 16)),
            ("shard", z.shard.unwrap()),
            ("gate", z.gate.unwrap()),
        ] {
            assert!(seen[((y as i32 / step) * w + x as i32 / step) as usize], "unreachable {name} at {x},{y}");
        }
        let blocked: Vec<_> = z.enemies.iter().filter(|&&(_, x, y)| z.blocked(x, y, 5.0)).collect();
        assert!(blocked.is_empty(), "enemies spawn in solid scenery: {blocked:?}");
    }

    /// A sword cuts rooted grass once into lit, settling pixels while enemy hits leave it intact.
    #[test]
    fn grass_cut_persists_and_only_reacts_to_player_hits() {
        let (mut z, mut r) = build();
        let shape = Shape::Circle { x: 480.0, y: 650.0, r: 42.0 };
        let mut fx = Fx::new(12);
        r.slash(&mut z, &Hit::new(shape, Team::Enemy, 1, 1), &mut fx);
        assert!(fx.particles.list.is_empty());
        r.slash(&mut z, &Hit::new(shape, Team::Player, 1, 2), &mut fx);
        let count = fx.particles.list.len();
        assert!(count > 30, "the meadow must contain cuttable blades");
        assert!(fx.particles.list.iter().all(|p| p.settle && p.glow == 0.0));
        r.slash(&mut z, &Hit::new(shape, Team::Player, 1, 3), &mut fx);
        assert_eq!(count, fx.particles.list.len(), "cut blades must not respawn on the next slash");
    }

    /// Render the actual native pipeline repeatedly to check animation, determinism and hook cost.
    #[test]
    fn native_renderer_is_deterministic_and_animated() {
        use crate::view::render_world;
        let (mut z, mut region) = build();
        let mut f = Frame::new();
        f.cam_x = 570;
        f.cam_y = 244;
        let mut fx = Fx::new(7);
        let movers = [(660.0, 344.0, 1.0)];
        let started = std::time::Instant::now();
        let mut first = Vec::new();
        for t in 0..120 {
            let ctx = RegionCtx {
                t,
                player: (660.0, 344.0),
                movers: &movers,
                cam: (f.cam_x, f.cam_y),
            };
            region.update(&mut z, &ctx, &mut fx);
            render_world(&z, region.as_ref(), &fx, &mut f, t, &[], &mut |_, _| {}, None);
            if t == 0 {
                first = f.out.clone();
            }
        }
        println!(
            "Bloomwild native update + render: {:.2} ms/frame",
            started.elapsed().as_secs_f64() * 1000.0 / 120.0
        );
        assert_ne!(first, f.out, "the waterfall must animate");
        let last = f.out.clone();
        render_world(&z, region.as_ref(), &fx, &mut f, 119, &[], &mut |_, _| {}, None);
        assert_eq!(last, f.out, "rendering must be stable at a fixed simulation tick");
    }
}
