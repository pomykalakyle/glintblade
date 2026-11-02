//! Glasswater Hollow: slate waterworks with pale iris crystals and quiet shelf fungi, with
//! shallow water that ripples when anything wades through it.
//!
//! Layout (tiles, 92x52; the map lives in `glasswater/map.txt`):
//! start cave (west) -> the lake chamber -> north into the gallery with its channel ->
//! the rift ledge and the bridge -> the mushroom grove (north-east) -> south to the heart,
//! where the prism shard sits on an island in a glowing pool and the gate waits in an alcove.

mod art;
mod life;
mod materials;
mod normals;
mod paint;
mod place;
mod water;

use crate::combat::{Hit, Shape, Team};
use crate::fx::Fx;
use crate::gfx::{Cell, DrawOpts, Frame, Rgb, rgb};
use crate::world::{LightOccluder, Prop, Region, RegionCtx, TILE, Tile, Zone, ZoneId};
use crate::gfx::frame::PointLight;
use std::f32::consts::TAU;

const MAP: &str = include_str!("glasswater/map.txt");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Big crystal cluster: rings and flashes when slashed.
    Crystal,
    /// Small crystal sprig: shatters into pixels when slashed.
    Sprig,
    Mushroom,
    Rock,
}

/// A code-drawn sprite placed in the world, y-sorted with characters.
pub struct Decor {
    pub cell: Cell,
    pub anchor: (i32, i32),
    pub x: f32,
    pub y: f32,
    /// Height of the supporting cap above the cave floor. X/Y keep the
    /// authored projected pose; lighting and native caster geometry use this Z.
    pub elevation: f32,
    pub flip: bool,
    pub kind: Kind,
    /// (color, radius, intensity, height of the light above the base)
    pub light: Option<(Rgb, f32, f32, f32)>,
    /// Native crest offset: light belongs to the crystal tip, not an arbitrary base-height circle.
    pub light_offset_x: f32,
    pub phase: f32,
    pub alive: bool,
    pub flash: f32,
    pub wobble: f32,
    /// Ring brightness after a slash (decays).
    pub ring: f32,
    /// Index of its collision prop in `zone.props`, if any.
    pub prop: Option<usize>,
    /// Waterline to mirror around, when it isn't the sprite's base (a crystal on an island).
    pub mirror: Option<f32>,
}

impl Decor {
    pub fn new(cell: Cell, anchor: (i32, i32), x: f32, y: f32, kind: Kind) -> Decor {
        Decor { cell, anchor, x, y, elevation: 0.0, flip: false, kind, light: None, light_offset_x: 0.0, phase: (x * 0.37 + y * 0.11) % TAU, alive: true, flash: 0.0, wobble: 0.0, ring: 0.0, prop: None, mirror: None }
    }
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let ax = if self.flip { self.cell.w - 1 - self.anchor.0 } else { self.anchor.0 };
        (self.x - ax as f32, self.y - self.anchor.1 as f32, self.cell.w as f32, self.cell.h as f32)
    }
    fn wobble_x(&self) -> f32 {
        if self.wobble > 0.0 { ((self.wobble * 1.7).sin() * self.wobble * 0.2).round() } else { 0.0 }
    }
    fn point_light(&self, t: u64) -> Option<PointLight> {
        self.light.map(|(color, radius, intensity, height)| {
            let breathe = match self.kind {
                Kind::Mushroom => 0.0,
                _ => 0.94 + 0.06 * (t as f32 * 0.013 + self.phase).sin(),
            };
            PointLight { x: (self.x + self.wobble_x()).floor() + self.light_offset_x,
                y: self.y.floor() - height, height: height + self.elevation,
                radius: radius * (1.0 + self.ring * 0.25),
                color, intensity: intensity * breathe + self.ring * 0.6 }
        })
    }
    fn emit_gain(&self, t: u64) -> f32 {
        let tf = t as f32;
        match self.kind {
            Kind::Mushroom => 0.45,
            Kind::Crystal | Kind::Sprig => 0.76 + 0.04 * (tf * 0.013 + self.phase).sin() + self.ring * 0.8,
            Kind::Rock => 1.0,
        }
    }
}

pub fn build() -> (Zone, Box<dyn Region>) {
    let rows: Vec<&str> = MAP.lines().collect();
    let th = rows.len() as i32;
    let tw = rows.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
    let mut z = Zone::new(ZoneId::Glasswater, tw, th);
    z.title = "GLASSWATER HOLLOW".to_string();
    z.music = "glasswater".to_string();
    z.bg = rgb(0x222d3d);
    for (ty, row) in rows.iter().enumerate() {
        for tx in 0..tw {
            let c = row.as_bytes().get(tx as usize).copied().unwrap_or(b'#');
            let t = match c {
                b'.' => Tile::Floor,
                b'~' => Tile::Shallow,
                b'=' => Tile::Deep,
                b' ' => Tile::Void,
                _ => Tile::Wall,
            };
            z.set_tile(tx, ty as i32, t);
        }
    }
    let m = paint::classify(&z);
    // Movement follows the same native shore used by water/reflections, including the
    // straight causeway. Tile corners must not create invisible walls on a curved bank.
    z.set_surface_tiles(m.cls.iter().map(|&c| match c {
        paint::FLOOR => Tile::Floor,
        paint::SHALLOW => Tile::Shallow,
        paint::DEEP => Tile::Deep,
        paint::VOID => Tile::Void,
        _ => Tile::Wall,
    }).collect());
    let fl = paint::fields(&m);
    paint::paint(&mut z, &m, &fl);
    paint::moss(&mut z, &m, &fl);
    paint::details(&mut z, &m, &fl);
    materials::paint(&mut z, &m, &fl);
    paint::bridge(&mut z, &m);

    // Low, cool slate ambient leaves the crystals and blade to carve out the cave. The grade
    // stays near neutral; the palette carries the mood.
    z.ambient = rgb(0x9caeb9).scale(0.84);
    // Slate bounce keeps the receiving banks legible; the iris sources still supply the key.
    z.lighting_settings.fill = Rgb::new(0.51, 0.57, 0.67);
    z.lighting_settings.shadow_tint = Rgb::new(0.44, 0.51, 0.69);
    z.grade.vignette = 0.10;
    z.grade.bloom = 0.24;
    // Fine value steps retain crisp native surfaces without turning the crystal
    // falloff into a few enormous concentric disks on the quiet cave floor.
    z.grade.light_steps = 32.0;
    if std::env::var("GW_FLAT").is_ok() {
        z.ambient = Rgb::WHITE;
        z.grade = Default::default();
        z.grade.vignette = 0.0;
    }

    let t = TILE as f32;
    z.player_start = (2.0 * t, 26.6 * t);
    z.shard = Some((72.6 * t, 44.9 * t));
    z.gate = Some((81.5 * t, 37.95 * t));

    let mut decor = Vec::new();
    place::populate(&mut z, &mut decor, &m);
    paint::lit_banks(&mut z, &m, &fl, &decor);
    paint::walkable_surfaces(&mut z, &m, &fl);
    normals::apply(&mut z, &m, &fl);
    let water = water::Water::new(&m, &fl, &z.ground, &decor);
    let mut life = life::Life::new();
    place::drips(&mut life);
    (z, Box::new(Glasswater { decor, water, life, player: (0.0, 0.0), t: 0, water_movers: Vec::new(), water_hit_ids: Vec::new() }))
}

/// An invisible prop that only blocks movement (for code-drawn crystals and rocks).
pub fn blocker(z: &mut Zone, decor: &Decor, depth: i32) -> usize {
    let mut p = Prop::new("glasswater_blocker", "none", decor.x, decor.y);
    p.set_native_support(&decor.cell, decor.anchor, depth);
    p.flip = decor.flip;
    z.props.push(p);
    z.props.len() - 1
}

#[derive(Clone, Copy)]
struct WaterMover {
    position: (f32, f32),
    travel: f32,
    wet: bool,
}

pub struct Glasswater {
    decor: Vec<Decor>,
    water: water::Water,
    life: life::Life,
    player: (f32, f32),
    t: u64,
    water_movers: Vec<WaterMover>,
    water_hit_ids: Vec<u32>,
}

impl Glasswater {
    /// Contact spacing follows displacement, so pushing against a bank does not make waves.
    fn wade(&mut self, zone: &Zone, movers: &[(f32, f32, f32)]) {
        for (i, &(x, y, _speed)) in movers.iter().enumerate() {
            let wet = zone.tile_at(x, y) == Tile::Shallow
                && self.water.is_water(x.floor() as i32, y.floor() as i32);
            let previous = self.water_movers.get(i).copied();
            let mut track = previous.unwrap_or(WaterMover { position: (x, y), travel: 0.0, wet });
            if let Some(previous) = previous {
                let distance = (x - previous.position.0).hypot(y - previous.position.1);
                // Discontinuous positions can be a respawn or an enemy removed from the list.
                if distance > 12.0 || !wet {
                    track.travel = 0.0;
                } else if distance > 0.05 {
                    track.travel += distance;
                    if !previous.wet || track.travel >= 5.0 {
                        let strength = 0.27 + distance.min(4.0) * 0.07;
                        let radius = 2.8 + distance.min(4.0) * 0.10;
                        self.water.poke(x, y, radius, strength);
                        track.travel %= 5.0;
                    }
                }
            }
            track.position = (x, y);
            track.wet = wet;
            if let Some(slot) = self.water_movers.get_mut(i) { *slot = track; }
            else { self.water_movers.push(track); }
        }
        self.water_movers.truncate(movers.len());
    }

    /// Project the combat shape (five pixels above the feet) onto the receiving water.
    /// Sampling is bounded, clipped to actual wet pixels, and happens once per swing.
    fn slash_water(&mut self, hit: &Hit) {
        if self.water_hit_ids.contains(&hit.id) { return; }
        let (x0, y0, x1, y1) = match hit.shape {
            Shape::Circle { x, y, r } => (x - r, y - r, x + r, y + r),
            Shape::Arc { x, y, r1, .. } => (x - r1, y - r1, x + r1, y + r1),
            Shape::Capsule { x0, y0, x1, y1, r } =>
                (x0.min(x1) - r, y0.min(y1) - r, x0.max(x1) + r, y0.max(y1) + r),
        };
        let nx = ((x1 - x0) / 5.0).ceil().clamp(1.0, 12.0) as usize;
        let ny = ((y1 - y0) / 5.0).ceil().clamp(1.0, 12.0) as usize;
        let mut contacts = Vec::new();
        for gy in 0..ny {
            let y = y0 + (gy as f32 + 0.5) * (y1 - y0) / ny as f32;
            for gx in 0..nx {
                let x = x0 + (gx as f32 + 0.5) * (x1 - x0) / nx as f32;
                if hit.shape.touches(x, y, 0.0)
                    && self.water.is_water(x.floor() as i32, (y + 5.0).floor() as i32) {
                    contacts.push((x, y + 5.0));
                }
            }
        }
        let count = contacts.len().min(12);
        if count == 0 { return; }
        for i in 0..count {
            let (x, y) = contacts[i * contacts.len() / count];
            self.water.poke(x, y, if hit.heavy { 3.4 } else { 2.8 },
                if hit.heavy { 0.42 } else { 0.30 });
        }
        self.water_hit_ids.push(hit.id);
        if self.water_hit_ids.len() > 32 { self.water_hit_ids.remove(0); }
    }
}

impl Region for Glasswater {
    fn update(&mut self, zone: &mut Zone, ctx: &RegionCtx, fx: &mut Fx) {
        self.t = ctx.t;
        self.player = ctx.player;
        self.wade(zone, ctx.movers);
        self.life.update(ctx, fx, &mut self.water);
        self.water.step(ctx.cam);
        for d in self.decor.iter_mut() {
            d.flash = (d.flash - 0.1).max(0.0);
            d.wobble = (d.wobble - 1.0).max(0.0);
            d.ring *= 0.94;
        }
    }

    fn slash(&mut self, zone: &mut Zone, hit: &Hit, fx: &mut Fx) {
        if hit.team != Team::Player {
            return;
        }
        self.slash_water(hit);
        for d in self.decor.iter_mut() {
            if !d.alive || !matches!(d.kind, Kind::Crystal | Kind::Sprig) {
                continue;
            }
            let (_, by, _, bh) = d.bounds();
            let r = (d.cell.w as f32 * 0.35).max(5.0);
            // test the base and the middle of the sprite
            let cy = (by + bh * 0.6).min(d.y - 2.0);
            if !(hit.shape.touches(d.x, d.y - 2.0, r) || hit.shape.touches(d.x, cy, r)) {
                continue;
            }
            if d.flash > 0.3 {
                continue;
            }
            let color = d.light.map(|l| l.0).unwrap_or(rgb(0xcdc3db));
            match d.kind {
                Kind::Sprig => {
                    d.alive = false;
                    let rng = &mut fx.rng;
                    fx.particles.burst_cell(rng, &d.cell, d.anchor, d.flip, d.x, d.y, 1.5, 480.0);
                    let rng = &mut fx.rng;
                    fx.particles.sparks(rng, d.x, d.y, 4.0, 0.0, -1.0, 6, color, 1.6);
                    fx.sound_var("crystal_shatter", 2);
                    fx.shake(0.12);
                    if let Some(i) = d.prop {
                        zone.props[i].alive = false;
                    }
                }
                _ => {
                    d.flash = 0.7;
                    d.wobble = 6.0;
                    d.ring = 1.0;
                    let (hx, hy) = hit.shape.center();
                    let rng = &mut fx.rng;
                    fx.particles.sparks(rng, d.x, d.y, 10.0, d.x - hx, d.y - hy, 8, color, 2.0);
                    fx.sound("deflect");
                    fx.shake(0.08);
                }
            }
        }
    }

    fn draw_ground(&self, _zone: &Zone, f: &mut Frame, t: u64) {
        f.look.light_dither = 0.25;
        f.look.water_alpha = 1.0;
        f.look.water_depth_max = 8;
        f.look.water_tint = Rgb::new(0.48, 0.62, 0.72);
        f.look.water_wave = 0.6;
        self.water.draw(f, t);
        for d in &self.decor {
            if !d.alive || d.kind == Kind::Rock && d.cell.h < 10 {
                continue;
            }
            let (x, y, w, h) = d.bounds();
            // the reflection hangs below the base, about as tall as the sprite
            let ry = d.mirror.unwrap_or(d.y);
            if !f.visible(x, ry, w, h + 4.0 + (ry - d.y) * 2.0, 4.0) {
                continue;
            }
            self.water.reflect(f, &d.cell, d.anchor, d.x, d.y, d.flip, d.wobble_x(), d.emit_gain(t), t);
            let _ = y;
        }
    }

    fn sorted_items(&self, zone: &Zone, f: &Frame, out: &mut Vec<(f32, u32)>) {
        for (i, d) in self.decor.iter().enumerate() {
            if !d.alive {
                continue;
            }
            let (x, y, w, h) = d.bounds();
            if f.surface_lighting || f.visible(x, y, w, h, 8.0) {
                let sort_y = d.prop.and_then(|id| zone.props.get(id)).map_or(d.y, Prop::sort_y);
                out.push((sort_y, i as u32));
            }
        }
    }

    fn draw_item(&self, _zone: &Zone, f: &mut Frame, id: u32, t: u64) {
        let Some(d) = self.decor.get(id as usize) else {
            return;
        };
        let wob = d.wobble_x();
        let o = DrawOpts { flip: d.flip, elevation: d.elevation, flash: d.flash, emit_gain: d.emit_gain(t), ..Default::default() };
        let reflecting = f.water_any;
        f.water_any = false; // Already reflected once on the actual receiving bank above.
        f.sprite_w(&d.cell, d.anchor, d.x + wob, d.y, &o);
        f.water_any = reflecting;
    }

    fn light_occluders<'a>(&'a self, t: u64, out: &mut Vec<LightOccluder<'a>>) {
        for d in self.decor.iter().filter(|d| d.alive) {
            out.push(LightOccluder { cell: &d.cell, anchor: d.anchor, x: d.x + d.wobble_x(),
                y: d.y, flip: d.flip, own_light: d.point_light(t) });
        }
    }

    fn draw_overlay(&self, _zone: &Zone, f: &mut Frame, _t: u64) {
        self.life.draw_overlay(f);
    }

    fn light(&self, _zone: &Zone, f: &mut Frame, t: u64) {
        for d in self.decor.iter().filter(|d| d.alive) {
            if let Some(p) = d.point_light(t) {
                let (radius, intensity) = if f.surface_lighting && d.kind == Kind::Crystal {
                    // These are the cave's practical lights. A lit side and a
                    // shadowed side remain visible as Glint walks around them.
                    // Tall landmark crests need a little more power to keep
                    // their floor reception readable across the real height.
                    let landmark_gain = if p.height - d.elevation > 40.0 { 1.22 } else { 1.0 };
                    (p.radius * 1.12, p.intensity * 2.15 * landmark_gain)
                } else if f.surface_lighting && d.kind == Kind::Sprig {
                    // Breaking a small crystal extinguishes a visible local
                    // source, not merely a handful of glowing sprite pixels.
                    (p.radius.max(30.0), p.intensity * 4.0)
                } else { (p.radius, p.intensity) };
                let (x, ground_y) = (p.x, d.y.floor() + d.elevation);
                f.add_light_3d_w(x, ground_y, p.height, radius, p.color, intensity);
            }
        }
    }

    fn draw_backdrop(&self, zone: &Zone, f: &mut Frame, _t: u64) {
        life::draw_abyss(zone, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn interaction_fixture() -> (Zone, Glasswater) {
        let mut zone = Zone::new(ZoneId::Glasswater, 16, 16);
        zone.tiles.fill(Tile::Shallow);
        for y in 0..zone.th { for x in 9..zone.tw { zone.set_tile(x, y, Tile::Wall); } }
        let m = paint::classify(&zone);
        zone.set_surface_tiles(m.cls.iter().map(|&c| match c {
            paint::SHALLOW => Tile::Shallow,
            paint::DEEP => Tile::Deep,
            paint::FLOOR => Tile::Floor,
            _ => Tile::Wall,
        }).collect());
        let fields = paint::fields(&m);
        zone.ground.fill(art::WATER[1]);
        let water = water::Water::new(&m, &fields, &zone.ground, &[]);
        (zone, Glasswater { decor: Vec::new(), water, life: life::Life::new(),
            player: (0.0, 0.0), t: 0, water_movers: Vec::new(), water_hit_ids: Vec::new() })
    }

    fn interaction_slopes(region: &Glasswater) -> Vec<(f32, f32)> {
        (32..140).flat_map(|y| (32..140).map(move |x| region.water.slope(x, y))).collect()
    }

    /// Real walking produces a wake; continuing to press into the wall adds no phantom energy.
    #[test]
    fn water_contacts_follow_actual_player_motion_and_stop_at_collision() {
        use crate::entities::{Player, player::PlayerCtx};
        let (mut zone, mut moving) = interaction_fixture();
        let (mut quiet_zone, mut quiet) = interaction_fixture();
        let mut player = Player::new(80.0, 80.0);
        let mut player_fx = Fx::new(5);
        let mut fx = Fx::new(6);
        let mut quiet_fx = Fx::new(6);
        let input = crate::input::Input { mv: (1.0, 0.0), ..Default::default() };
        let mut peak = 0.0f32;
        for t in 1..=80 {
            player.update(&input, &mut PlayerCtx { zone: &zone, fx: &mut player_fx, hits: &mut Vec::new() });
            let movers = [(player.body.x, player.body.y, player.body.vx.hypot(player.body.vy))];
            let ctx = RegionCtx { t, player: (player.body.x, player.body.y), movers: &movers, cam: (0, 0) };
            moving.update(&mut zone, &ctx, &mut fx);
            quiet.update(&mut quiet_zone, &ctx, &mut quiet_fx);
            for (x, y) in interaction_slopes(&moving) { peak = peak.max(x.abs() + y.abs()); }
        }
        assert!(peak > 0.03, "walking failed to produce a readable water slope: {peak}");
        let stopped = (player.body.x, player.body.y);
        for t in 81..=140 {
            player.update(&input, &mut PlayerCtx { zone: &zone, fx: &mut player_fx, hits: &mut Vec::new() });
            let movers = [(player.body.x, player.body.y, player.body.vx.hypot(player.body.vy))];
            moving.update(&mut zone, &RegionCtx { t, player: stopped, movers: &movers, cam: (0, 0) }, &mut fx);
            quiet.update(&mut quiet_zone, &RegionCtx { t, player: stopped, movers: &[], cam: (0, 0) }, &mut quiet_fx);
            assert_eq!((player.body.x, player.body.y), stopped, "player had not reached the bank");
            assert!(player.body.vx > 1.0, "test must retain an attempted movement velocity");
            assert_eq!(interaction_slopes(&moving), interaction_slopes(&quiet),
                "collision-blocked motion injected a phantom wake");
        }
    }

    /// A sword stirs actual receiving water once per hit; air, dry banks and enemies do not.
    #[test]
    fn sword_water_contacts_are_wet_clipped_and_once_per_hit() {
        let (mut zone, mut region) = interaction_fixture();
        let mut fx = Fx::new(7);
        let hit = Hit::new(Shape::Circle { x: 80.0, y: 75.0, r: 12.0 }, Team::Player, 1, 91);
        region.slash(&mut zone, &hit, &mut fx);
        let first = interaction_slopes(&region);
        assert!(first.iter().any(|&(x, y)| x.abs() + y.abs() > 0.03), "sword did not disturb the water");
        for _ in 0..12 { region.slash(&mut zone, &hit, &mut fx); }
        assert_eq!(interaction_slopes(&region), first, "active sword frames stacked the same wave");
        region.slash(&mut zone,
            &Hit::new(Shape::Circle { x: 80.0, y: 75.0, r: 12.0 }, Team::Enemy, 1, 92), &mut fx);
        region.slash(&mut zone,
            &Hit::new(Shape::Circle { x: 220.0, y: 175.0, r: 12.0 }, Team::Player, 1, 93), &mut fx);
        assert_eq!(interaction_slopes(&region), first, "non-water contact injected a wave");
    }

    /// A shattered native sprig disappears from the live light and shadow
    /// submissions in the same frame, including its optional collision proxy.
    #[test]
    fn shattering_sprig_extinguishes_surrounding_light_and_occluder() {
        use crate::combat::Shape;
        let mut zone = Zone::new(ZoneId::Glasswater, 12, 12);
        zone.tiles.fill(Tile::Floor);
        let m = paint::classify(&zone);
        let fields = paint::fields(&m);
        let (cell, anchor) = art::crystal_sprig(12, &art::VIOLET, 9);
        let mut d = Decor::new(cell, anchor, 80.0, 100.0, Kind::Sprig);
        d.light = Some((rgb(0xcdc3db), 14.0, 0.10, 6.0));
        let prop = blocker(&mut zone, &d, 4);
        d.prop = Some(prop);
        let water = water::Water::new(&m, &fields, &zone.ground, std::slice::from_ref(&d));
        let mut region = Glasswater { decor: vec![d], water, life: life::Life::new(), player: (0.0, 0.0), t: 0, water_movers: Vec::new(), water_hit_ids: Vec::new() };
        let mut f = Frame::new();
        f.surface_lighting = true;
        region.light(&zone, &mut f, 0);
        assert_eq!(f.point_lights.len(), 1);
        assert!(f.point_lights[0].radius >= 30.0 && f.point_lights[0].intensity > 0.3);
        let mut fx = Fx::new(8);
        region.slash(&mut zone, &Hit::new(Shape::Circle { x: 80.0, y: 98.0, r: 8.0 }, Team::Player, 1, 1), &mut fx);
        assert!(!region.decor[0].alive && !zone.props[prop].alive);
        f.begin(Rgb::BLACK, Rgb::WHITE);
        f.surface_lighting = true;
        region.light(&zone, &mut f, 0);
        assert!(f.point_lights.is_empty(), "the sprig left its surrounding light behind");
        let mut casters = Vec::new();
        region.light_occluders(0, &mut casters);
        assert!(casters.is_empty(), "the broken sprig left a shadow housing behind");
    }

    /// The rift crossing must read as masonry and play as the same straight deck. Use
    /// real input across both landings, then walk into its visible north edge.
    #[test]
    fn causeway_input_crossing_and_visible_edge_match() {
        let mut play = crate::play::Play::new(ZoneId::Glasswater, &crate::play::PlayOpts { god: true, peaceful: true });
        let mut fx = Fx::new(7);
        let mut input = crate::input::Input { mv: (1.0, 0.0), ..Default::default() };
        play.player.body.x = 738.0;
        play.player.body.y = 256.0;
        for _ in 0..155 {
            play.update(&input, &mut fx);
            fx.sounds.clear();
            fx.lights.clear();
        }
        assert!(play.player.body.x > 957.0, "causeway landing blocked at {}", play.player.body.x);
        assert!((play.player.body.y - 256.0).abs() < 0.1);
        play.player.body.x = 832.0;
        play.player.body.y = 256.0;
        play.player.body.vx = 0.0;
        play.player.body.vy = 0.0;
        input.mv = (0.0, -1.0);
        for _ in 0..50 { play.update(&input, &mut fx); }
        assert!((242.3..244.4).contains(&play.player.body.y), "feet did not stop at the visible deck edge: {}", play.player.body.y);
        assert_eq!(play.zone.tile_at(832.0, 239.0), Tile::Void);
        assert_eq!(play.zone.tile_at(832.0, 240.0), Tile::Floor);
    }

    /// Verify the real player collision box can reach both objectives and every combat group.
    #[test]
    fn objectives_and_encounters_are_reachable() {
        let (z, _) = build();
        let step = 4;
        let (w, h) = (z.pw / step, z.ph / step);
        let mut seen = vec![false; (w * h) as usize];
        let start = (z.player_start.0 as i32 / step, z.player_start.1 as i32 / step);
        let mut q = VecDeque::from([start]);
        seen[(start.1 * w + start.0) as usize] = true;
        while let Some((x, y)) = q.pop_front() {
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let i = (ny * w + nx) as usize;
                if !seen[i] && !z.blocked((nx * step) as f32, (ny * step) as f32, 4.0) {
                    seen[i] = true;
                    q.push_back((nx, ny));
                }
            }
        }
        let reachable = |x: f32, y: f32| {
            let (gx, gy) = (x as i32 / step, y as i32 / step);
            (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    let (nx, ny) = (gx + dx, gy + dy);
                    nx >= 0 && ny >= 0 && nx < w && ny < h && seen[(ny * w + nx) as usize]
                })
            })
        };
        assert!(!z.blocked(z.player_start.0, z.player_start.1, 4.0));
        for (name, pos) in [("shard", z.shard), ("gate", z.gate)] {
            let (x, y) = pos.expect("objective");
            assert!(reachable(x, y), "{name} is unreachable at {x},{y}");
        }
        assert!((8..=14).contains(&z.enemies.len()));
        for &(kind, x, y) in &z.enemies {
            assert!(!z.blocked(x, y, 4.0), "{kind:?} spawns blocked at {x},{y}");
            assert!(reachable(x, y), "{kind:?} is unreachable at {x},{y}");
        }
    }
}
