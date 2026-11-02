//! The Summit: the Warden's arena at the top of the world.
//!
//! Layout (world px, 90x60 tiles):
//! - A small landing island in the south-west where Glint arrives.
//! - A stone bridge (aqueduct-style arches underneath) east across the void to a round terrace.
//! - A grand stair north up the arena's masonry rim wall.
//! - The arena: a broad elliptical floor of displaced, quiet stone slabs,
//!   braziers on the rim, and an empty prism socket on a dais at the north. The Warden
//!   stands in front of the dais.
//!
//! Everything beyond the islands is a sparse, still starfield. Only the braziers and
//! the Warden's heart cast light. The ground is painted once in `paint.rs`; artist sheets
//! supply the props, with hand-authored fallbacks in `art.rs` while assets are in progress.

mod art;
mod paint;
mod sky;

use crate::fx::Fx;
use crate::gfx::{Frame, Grade, Rgb, sheet};
use crate::util::hash2;
use crate::world::{Prop, Region, RegionCtx, Zone, ZoneId};

/// Zone size in tiles.
const TW: i32 = 90;
const TH: i32 = 60;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Ellipse {
    pub cx: f32,
    pub cy: f32,
    pub rx: f32,
    pub ry: f32,
}

impl Ellipse {
    /// Normalized radius: 1 on the outline.
    #[inline]
    pub fn rho(&self, x: f32, y: f32) -> f32 {
        let u = (x - self.cx) / self.rx;
        let v = (y - self.cy) / self.ry;
        (u * u + v * v).sqrt()
    }
    /// Test whether a point lies on the platform top.
    #[inline]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.rho(x, y) <= 1.0
    }
    /// World point on the outline at angle `a` (0 = east, +y down).
    pub fn at(&self, a: f32, k: f32) -> (f32, f32) {
        (self.cx + a.cos() * self.rx * k, self.cy + a.sin() * self.ry * k)
    }
}

/// The arena floor.
pub(crate) const ARENA: Ellipse = Ellipse { cx: 720.0, cy: 440.0, rx: 336.0, ry: 224.0 };
/// The round terrace at the foot of the grand stair.
pub(crate) const TERRACE: Ellipse = Ellipse { cx: 720.0, cy: 784.0, rx: 128.0, ry: 64.0 };
/// The arrival island.
pub(crate) const LANDING: Ellipse = Ellipse { cx: 228.0, cy: 776.0, rx: 108.0, ry: 72.0 };
/// The bridge deck (x0, y0, x1, y1).
pub(crate) const BRIDGE: (i32, i32, i32, i32) = (320, 752, 640, 800);
/// The grand stair (x0, y0, x1, y1): from the arena lip down to the terrace.
pub(crate) const STAIR: (i32, i32, i32, i32) = (688, 650, 752, 724);
/// The empty prism dais, just inside the north rim.
pub(crate) const DAIS: Ellipse = Ellipse { cx: 720.0, cy: 252.0, rx: 64.0, ry: 30.0 };
/// Where the Warden stands (feet).
pub(crate) const BOSS_POS: (f32, f32) = (720.0, 352.0);
/// Rim wall height (px of masonry visible under the arena's south lip).
pub(crate) const ARENA_FACE: i32 = 56;

/// The blue-gray night ramps from `palettes_v2.json`, using `regions.warden`.
#[allow(dead_code)]
pub(crate) mod pal {
    use crate::gfx::{Rgb, rgb};
    pub const F0: Rgb = rgb(0x1f2838);
    pub const F1: Rgb = rgb(0x2b3548);
    pub const F2: Rgb = rgb(0x46526a);
    pub const F2B: Rgb = rgb(0x46526a);
    pub const F3: Rgb = rgb(0x64738a);
    pub const F3B: Rgb = rgb(0x64738a);
    pub const F4: Rgb = rgb(0x94a1ae);
    pub const F5: Rgb = rgb(0xc1c8c5);
    pub const S0: Rgb = rgb(0x2b3548);
    pub const S1: Rgb = rgb(0x333b4c);
    pub const S2: Rgb = rgb(0x555d70);
    pub const S3: Rgb = rgb(0x7f8797);
    pub const S4: Rgb = rgb(0xb0b7bb);
    pub const S5: Rgb = rgb(0xd9dacc);
    pub const M0: Rgb = rgb(0x3e5258);
    pub const M1: Rgb = rgb(0x3e5258);
    pub const M2: Rgb = rgb(0x627975);
    pub const M3: Rgb = rgb(0x627975);
    pub const G0: Rgb = rgb(0x655e60);
    pub const G1: Rgb = rgb(0x948574);
    pub const G2: Rgb = rgb(0xbdad8d);
    pub const G3: Rgb = rgb(0xded0ad);
    pub const G4: Rgb = rgb(0xded0ad);
    pub const K0: Rgb = rgb(0x1f2838);
    pub const K1: Rgb = rgb(0x1f2838);
    pub const FIRE: Rgb = rgb(0xded0ad);
}

/// Things the region draws itself, y-sorted with characters.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ItemKind {
    Brazier,
    /// Banner pole; `bool` = flipped.
    Banner(bool),
    /// Short balustrade post at a bridge or terrace edge.
    Post,
    /// Broken pillar stump, `u8` = variant.
    Pillar(u8),
    /// Standing stone at the landing, `u8` = variant.
    Monolith(u8),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Item {
    pub kind: ItemKind,
    pub x: f32,
    pub y: f32,
    pub seed: u32,
    pub sprite: bool,
}

pub struct Summit {
    items: Vec<Item>,
    sky: sky::Sky,
    /// Hole mask (true = backdrop shows), pw*ph, for fast per-pixel tests.
    hole: Vec<bool>,
    pw: i32,
    ph: i32,
}

/// Build the quiet approach and a clear, two-screen-wide boss arena.
pub fn build() -> (Zone, Box<dyn Region>) {
    let mut z = Zone::new(ZoneId::Summit, TW, TH);
    z.title = "THE WARDEN'S SUMMIT".to_string();
    z.music = String::new(); // Quiet approach; the Warden cue begins on awakening.
    z.bg = pal::K1;
    // The palette supplies the night color; grading stays neutral.
    z.ambient = Rgb::gray(0.8);
    z.grade = Grade { vignette: 0.10, bloom: 0.55, bloom_threshold: 1.0, ..Grade::default() };
    z.boss = true;
    z.boss_pos = Some(BOSS_POS);
    z.shard = None;
    z.gate = None;
    z.player_start = (196.0, 782.0);

    paint::paint(&mut z);
    let items = place_items(&mut z);
    art::place_waterworks_frame(&mut z);

    let hole: Vec<bool> = z.ground.iter().map(|g| g.r < 0.0).collect();
    let region = Summit { items, sky: sky::Sky::new(z.pw, z.ph), hole, pw: z.pw, ph: z.ph };
    (z, Box::new(region))
}

/// An invisible collision block (a prop with no sprite).
fn blocker(z: &mut Zone, x: f32, y: f32, w: f32, h: f32) {
    let mut p = Prop::new("__summit_blocker", "none", x, y);
    p.footprint = Some((w, h));
    z.props.push(p);
}

/// Match collision to native visible support pixels. The rectangles are merged
/// source-row spans, rather than one oversized box across angled corners/gaps.
fn native_supports(z: &mut Zone, name: &str, anim: &str, x: f32, y: f32) -> bool {
    let Some(s) = sheet(name) else { return false };
    let Some(a) = s.anim(anim) else { return false };
    let Some(rects) = a.json.get("collision_rects").and_then(|v| v.as_array()) else { return false };
    let (ax, ay) = s.anchor_of(a);
    let mut placed = false;
    for r in rects {
        let Some(v) = r.as_array().filter(|v| v.len() == 4) else { continue };
        let values: Option<Vec<f32>> = v.iter().map(|n| n.as_f64().map(|n| n as f32)).collect();
        let Some(v) = values else { continue };
        let (x0, y0, x1, y1) = (v[0], v[1], v[2], v[3]);
        if x1 <= x0 || y1 <= y0 { continue; }
        blocker(z, x - ax as f32 + (x0 + x1) * 0.5,
            y - ay as f32 + y1, x1 - x0, y1 - y0);
        placed = true;
    }
    placed
}

/// Place architecture outside the central fighting space and use live artist sheets.
fn place_items(z: &mut Zone) -> Vec<Item> {
    let mut items = Vec::new();
    let mut push = |kind: ItemKind, x: f32, y: f32| {
        let seed = (hash2(x as i32, y as i32, 77) * 1e6) as u32;
        items.push(Item { kind, x, y, seed, sprite: false });
    };

    // Braziers around the arena rim, evenly spaced, skipping the stair (south) and dais (north).
    let n = 12;
    for i in 0..n {
        let a = (i as f32 + 0.5) / n as f32 * std::f32::consts::TAU;
        let deg = a.to_degrees();
        if (deg - 90.0).abs() < 20.0 || (deg - 270.0).abs() < 20.0 {
            continue;
        }
        let (x, y) = ARENA.at(a, 0.92);
        push(ItemKind::Brazier, x.round(), y.round());
    }
    // Two flames frame the stair, with an empty socket beyond the Warden.
    push(ItemKind::Brazier, (STAIR.0 - 13) as f32, 658.0);
    push(ItemKind::Brazier, (STAIR.2 + 12) as f32, 658.0);
    // The painted socket supplies the story beat without a second cap behind the Keeper.
    // Banners on the terrace and at the landing.
    push(ItemKind::Banner(false), 628.0, 790.0);
    push(ItemKind::Banner(true), 812.0, 790.0);
    push(ItemKind::Banner(false), 300.0, 760.0);
    // Balustrade posts along the bridge's south edge (a gap where one has fallen).
    let mut x = 352;
    while x < 620 {
        if x != 480 {
            push(ItemKind::Post, x as f32, (BRIDGE.3 - 1) as f32);
        }
        x += 32;
    }
    // Broken pillars: a ring of old columns on the arena's outer course.
    for (a_deg, v) in [(205.0f32, 0u8), (243.0, 1), (297.0, 2), (335.0, 1), (148.0, 2)] {
        let (x, y) = ARENA.at(a_deg.to_radians(), 0.80);
        push(ItemKind::Pillar(v), x.round(), y.round());
    }
    // Standing stones at the landing.
    // Both broken tops fit the actual arrival camera; the bridge's eastbound
    // route stays open below this complete, readable landing landmark.
    push(ItemKind::Monolith(0), 150.0, 764.0);
    push(ItemKind::Monolith(1), 196.0, 756.0);

    for it in &mut items {
        let (name, anim) = it.sprite();
        if sheet(name).and_then(|s| s.anim(&anim)).is_some() {
            let mut p = Prop::new(name, &anim, it.x, it.y);
            p.flip = matches!(it.kind, ItemKind::Banner(true));
            // Collision is instantiated below from native support masks (or the
            // legacy feet fallback), so the visible prop never adds a second box.
            p.footprint = None;
            z.props.push(p);
            it.sprite = true;
        }
        let native = matches!(it.kind, ItemKind::Pillar(_) | ItemKind::Monolith(_))
            && native_supports(z, name, &anim, it.x, it.y);
        let footprint = if native { None } else { match it.kind {
            ItemKind::Brazier => Some((14.0, 6.0)),
            ItemKind::Pillar(_) => Some((16.0, 7.0)),
            ItemKind::Monolith(_) => Some((14.0, 6.0)),
            ItemKind::Banner(_) => Some((4.0, 3.0)),
            _ => None,
        } };
        if let Some((w, h)) = footprint {
            blocker(z, it.x, it.y + 1.0, w, h);
        }
    }
    // The raised dais is solid.
    if !native_supports(z, "summit_dais", "dais", DAIS.cx, DAIS.cy + 14.0) {
    let dais_footprint = sheet("summit_dais").and_then(|s| s.anim("dais"))
        .and_then(|a| a.point("footprint", 0))
        .map(|(w, h)| (w as f32, h as f32)).unwrap_or((DAIS.rx * 1.6, DAIS.ry * 1.3));
    blocker(z, DAIS.cx, DAIS.cy + DAIS.ry * 0.7, dais_footprint.0, dais_footprint.1);
    }
    items
}

impl Item {
    /// Match the artist's sheet and animation names without tying geometry to sprite size.
    fn sprite(&self) -> (&'static str, String) {
        match self.kind {
            ItemKind::Brazier => ("summit_brazier", "brazier".into()),
            ItemKind::Banner(_) => ("summit_banners", "banner".into()),
            ItemKind::Post => ("summit_walls", "post".into()),
            ItemKind::Pillar(v) => ("summit_pillars", format!("pillar_{v}")),
            ItemKind::Monolith(v) => ("summit_stones", format!("monolith_{v}")),
        }
    }

    /// Vary the flames gently, without pulsing the whole arena.
    fn flicker(&self, t: u64) -> f32 {
        let phase = self.seed as f32 * 0.001;
        1.0 + 0.06 * (t as f32 * 0.21 + phase).sin() + 0.03 * (t as f32 * 0.53 + phase).sin()
    }

    /// Warm only the visible bowl facets and short stone shoulders below this flame.
    fn light_receivers(&self, zone: &Zone, f: &mut Frame, t: u64) {
        let (sx, sy) = (f.sx(self.x), f.sy(self.y));
        if !f.visible(self.x - 19.0, self.y - 25.0, 38.0, 33.0, 0.0) {
            return;
        }
        let fire = pal::FIRE * self.flicker(t);
        for dy in -24i32..=7 {
            for dx in -18i32..=18 {
                let Some(i) = Frame::idx(sx + dx, sy + dy) else { continue };
                if f.actor_pixels[i] {
                    continue;
                }
                let c = f.albedo[i];
                // The bowl's existing brass planes face the coals; preserve its dark underside.
                if (-23..=-14).contains(&dy) && dx.abs() <= 10 && (c == pal::G1 || c == pal::G2) {
                    f.light[i] += fire * if dy < -18 { 0.32 } else { 0.18 };
                }
                if c == pal::F3 && (((-16..=-5).contains(&dy) && (-5..=-3).contains(&dx)) || ((-4..=-1).contains(&dy) && dx.abs() <= 6)) {
                    f.light[i] += fire * 0.24;
                }
                let (wx, wy) = (self.x as i32 + dx, self.y as i32 + dy);
                if !(-3..=5).contains(&dy) || dx.abs() < 6 || dx.abs() > 17 - dy.max(0) {
                    continue;
                }
                let stone = zone.gget(wx, wy);
                if c != stone || !(stone == pal::F3 || stone == pal::F4 || stone == pal::S2) {
                    continue;
                }
                // Only existing raised curb planes receive these bands; the flat combat floor stays quiet.
                let k = if dx.abs() <= 11 && dy <= 2 { 0.24 } else { 0.12 };
                f.light[i] += fire * k;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// New materials and silhouettes cannot create a walkable void or narrow the
    /// bridge/stair: movement still follows the original five platform domains.
    #[test]
    fn native_environment_preserves_all_platform_tiles() {
        use crate::world::{TILE, Tile};
        let (zone, _) = build();
        for ty in 0..TH {
            for tx in 0..TW {
                let x = tx * TILE + TILE / 2;
                let y = ty * TILE + TILE / 2;
                let rect = |r: (i32, i32, i32, i32)| x >= r.0 && x < r.2 && y >= r.1 && y < r.3;
                let floor = [ARENA, TERRACE, LANDING].iter().any(|e| e.contains(x as f32 + 0.5, y as f32 + 0.5))
                    || rect(BRIDGE) || rect(STAIR);
                assert_eq!(zone.tile(tx, ty), if floor { Tile::Floor } else { Tile::Void }, "tile {tx},{ty}");
            }
        }
    }

    /// Every visible support pixel is solid, including asymmetric shoulders and
    /// the front tips that the old centered 16x7 collision boxes missed.
    #[test]
    fn native_support_masks_cover_all_visible_base_pixels_and_keep_routes_open() {
        let (zone, _) = build();
        for item in zone.props.iter().filter(|p| p.sheet == "summit_pillars" || p.sheet == "summit_stones") {
            let (name, anim) = (item.sheet, &item.anim);
            let s = sheet(name).unwrap();
            let a = s.anim(&anim).unwrap();
            let first = a.json["support_first_row"].as_i64().expect("native support row") as i32;
            let cell = s.cell(a, 0);
            let (ax, ay) = s.anchor_of(a);
            for y in first..cell.h {
                for x in 0..cell.w {
                    if cell.at(x, y).a {
                        assert!(zone.blocked(item.x - ax as f32 + x as f32 + 0.5,
                            item.y - ay as f32 + y as f32 + 0.5, 0.05), "{name}/{anim} source {x},{y}");
                    }
                }
            }
            if name == "summit_stones" {
                let first_opaque = (0..cell.h).find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).unwrap();
                assert!(item.y - ay as f32 + first_opaque as f32 >= zone.player_start.1 - 108.0,
                    "arrival landmark top cropped: {anim}");
            }
        }
        let s = sheet("summit_dais").unwrap();
        let a = s.anim("dais").unwrap();
        let (ax, ay) = s.anchor_of(a);
        let cell = s.cell(a, 0);
        for y in 0..cell.h {
            for x in 0..cell.w {
                if cell.at(x, y).a {
                    assert!(zone.blocked(DAIS.cx - ax as f32 + x as f32 + 0.5,
                        DAIS.cy + 14.0 - ay as f32 + y as f32 + 0.5, 0.05), "dais source {x},{y}");
                }
            }
        }
        for x in (196..720).step_by(2) {
            assert!(!zone.blocked(x as f32, 782.0, 4.0), "bridge route {x},782");
        }
        for y in (640..792).step_by(2) {
            assert!(!zone.blocked(720.0, y as f32, 4.0), "stair route 720,{y}");
        }
    }

    /// A rounded visible corner must not be replaced by the coarse16px navigation cell.
    #[test]
    fn curved_platform_contacts_use_the_finished_native_top() {
        use crate::world::Tile;
        let (z, _) = build();
        assert_eq!(z.tile(27, 20), Tile::Floor);
        assert_eq!(z.tile_at(435.0, 320.0), Tile::Void, "outside the visible curved top in the same coarse floor tile");
        assert_eq!(z.tile_at(440.0, 328.0), Tile::Floor);
        let (x, y, hit) = z.move_box(470.0, 320.0, 4.0, -60.0, 0.0);
        assert!(hit && (440.0..448.0).contains(&x), "native corner contact: {x},{y}");
        assert!(!z.blocked(x, y, 4.0));
        assert!(z.blocked(x - 2.0, y, 4.0));
        for py in (y - 2.4).floor() as i32..(y + 2.4).ceil() as i32 {
            for px in (x - 4.0).floor() as i32..(x + 4.0).ceil() as i32 {
                assert!(!z.tile_at(px as f32, py as f32).solid(), "feet entered a visible gap");
            }
        }
    }

    /// Reproduce the independent reviewer's ordinary R50 contact case through
    /// real Game::tick input. Glint must stop before the visible left footing.
    #[test]
    fn west_pier_contact_stops_hero_outside_visible_stone() {
        use crate::game::{Game, Options, Scene};
        use crate::input::{Input, Raw};
        let mut game = Game::new(Options { start: Some(ZoneId::Summit), peaceful: true, ..Options::default() });
        if let Scene::Play(p) = &mut game.scene { p.player.respawn(460.0, 532.0); }
        let mut input = Input::default();
        for _ in 0..50 {
            input.step(Raw { mv: (1.0, 0.0), ..Raw::default() });
            game.tick(&input);
            game.fx.sounds.clear();
        }
        let Scene::Play(p) = &game.scene else { panic!("left play scene") };
        let (x, y) = (p.player.body.x, p.player.body.y);
        assert!(x > 462.0 && x <= 469.0, "walked inside visible footing: {x},{y}");
        assert!(!p.zone.blocked(x, y, p.player.body.r));
        assert!(p.zone.blocked(x + 1.5, y, p.player.body.r));
    }

    /// Received fire stays on existing stone and bowl facets, preserving actors and albedo.
    #[test]
    fn brazier_receivers_preserve_materials_and_actor_ink() {
        let (zone, _) = build();
        let it = Item { kind: ItemKind::Brazier, x: 675.0, y: 658.0, seed: 1, sprite: true };
        let mut frame = Frame::new();
        frame.cam_x = 528;
        frame.cam_y = 564;
        frame.begin(zone.bg, zone.ambient);
        zone.draw_ground(&mut frame);
        let (sx, sy) = (frame.sx(it.x), frame.sy(it.y));
        frame.put(sx, sy - 21, pal::G2);
        frame.put(sx + 1, sy - 21, pal::G2);
        frame.mark_actor(sx + 1, sy - 21);
        let actor = Frame::idx(sx + 1, sy - 21).unwrap();
        let bowl = Frame::idx(sx, sy - 21).unwrap();
        let albedo = frame.albedo.clone();
        let emit = frame.emissive.clone();
        let light = frame.light.clone();
        it.light_receivers(&zone, &mut frame, 300);
        assert_eq!(frame.albedo, albedo);
        assert_eq!(frame.emissive, emit);
        assert_eq!(frame.light[actor], light[actor]);
        assert!(frame.light[bowl].luma() > light[bowl].luma());
        let changed: Vec<_> = frame.light.iter().zip(&light).enumerate().filter(|(_, (a, b))| a != b).collect();
        assert!(changed.len() > 4 && changed.len() < 250);
        for (i, _) in changed {
            let (dx, dy) = (i as i32 % crate::gfx::W as i32 - sx, i as i32 / crate::gfx::W as i32 - sy);
            assert!(dx.abs() <= 18 && (-24..=7).contains(&dy));
        }
    }
}

impl Region for Summit {
    fn backdrop_light(&self, _ambient: Rgb) -> Rgb {
        Rgb::WHITE
    }

    fn update(&mut self, _zone: &mut Zone, _ctx: &RegionCtx, fx: &mut Fx) {
        // Pixels that fall off the edge drop into the sky instead of resting on thin air.
        let (pw, ph) = (self.pw, self.ph);
        let hole = &self.hole;
        let is_hole = |x: f32, y: f32| {
            let (xi, yi) = (x.floor() as i32, y.floor() as i32);
            xi < 0 || yi < 0 || xi >= pw || yi >= ph || hole[(yi * pw + xi) as usize]
        };
        fx.particles.list.retain(|p| !(p.z <= 0.5 && p.grav > 0.0 && is_hole(p.x, p.y)));
        fx.particles.debris.retain(|d| !is_hole(d.x as f32, d.y as f32));
    }

    fn draw_backdrop(&self, _zone: &Zone, f: &mut Frame, _t: u64) {
        self.sky.draw(f);
    }

    fn sorted_items(&self, _zone: &Zone, f: &Frame, out: &mut Vec<(f32, u32)>) {
        for (i, it) in self.items.iter().enumerate() {
            if it.sprite {
                continue;
            }
            let (w, h) = art::item_bounds(it.kind);
            if f.visible(it.x - w * 0.5, it.y - h, w, h + 8.0, 16.0) {
                out.push((it.y, i as u32));
            }
        }
    }

    fn draw_item(&self, zone: &Zone, f: &mut Frame, id: u32, t: u64) {
        self.draw_item_revealed(zone, f, id, t, None);
    }

    fn draw_item_revealed(&self, _zone: &Zone, f: &mut Frame, id: u32, t: u64, reveal: Option<crate::gfx::sprite::SpriteFocus>) {
        let Some(it) = self.items.get(id as usize) else {
            return;
        };
        let (sx, sy) = (f.sx(it.x), f.sy(it.y));
        art::draw_item(f, sx, sy, it.kind, t, it.seed, reveal);
    }

    fn light(&self, zone: &Zone, f: &mut Frame, t: u64) {
        for it in &self.items {
            if matches!(it.kind, ItemKind::Brazier) {
                if !it.sprite {
                    f.add_light_3d_w(it.x, it.y, 23.0, 22.0, pal::FIRE, 0.3 * it.flicker(t));
                }
                if !f.surface_lighting { it.light_receivers(zone, f, t); }
            }
        }
        // The boss code owns its heart light, so it dies with the Warden.
        self.sky.light(f, &self.hole, self.pw, self.ph);
    }
}
