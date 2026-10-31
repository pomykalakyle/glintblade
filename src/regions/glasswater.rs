//! Glasswater Hollow: slate waterworks with pale iris crystals and quiet shelf fungi, with
//! shallow water that ripples when anything wades through it.
//!
//! Layout (tiles, 92x52; the map lives in `glasswater/map.txt`):
//! start cave (west) -> the lake chamber -> north into the gallery with its channel ->
//! the rift ledge and the bridge -> the mushroom grove (north-east) -> south to the heart,
//! where the prism shard sits on an island in a glowing pool and the gate waits in an alcove.

mod art;
mod life;
mod paint;
mod place;
mod water;

use crate::combat::{Hit, Team};
use crate::fx::Fx;
use crate::gfx::{Cell, DrawOpts, Frame, Rgb, rgb};
use crate::world::{Prop, Region, RegionCtx, TILE, Tile, Zone, ZoneId};
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
    pub flip: bool,
    pub kind: Kind,
    /// (color, radius, intensity, height of the light above the base)
    pub light: Option<(Rgb, f32, f32, f32)>,
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
        Decor { cell, anchor, x, y, flip: false, kind, light: None, phase: (x * 0.37 + y * 0.11) % TAU, alive: true, flash: 0.0, wobble: 0.0, ring: 0.0, prop: None, mirror: None }
    }
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let ax = if self.flip { self.cell.w - 1 - self.anchor.0 } else { self.anchor.0 };
        (self.x - ax as f32, self.y - self.anchor.1 as f32, self.cell.w as f32, self.cell.h as f32)
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
    let fl = paint::fields(&m);
    paint::paint(&mut z, &m, &fl);
    paint::moss(&mut z, &m, &fl);
    paint::details(&mut z, &m, &fl);

    // Low, cool slate ambient leaves the crystals and blade to carve out the cave. The grade
    // stays near neutral; the palette carries the mood.
    z.ambient = rgb(0x9caeb9).scale(0.72);
    // Slate bounce keeps the receiving banks legible; the iris sources still supply the key.
    z.lighting_settings.fill = Rgb::new(0.47, 0.53, 0.63);
    z.lighting_settings.shadow_tint = Rgb::new(0.44, 0.51, 0.69);
    z.grade.vignette = 0.10;
    z.grade.bloom = 0.24;
    z.grade.light_steps = 8.0;
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
    let water = water::Water::new(&m, &fl, &z.ground, &decor);
    let mut life = life::Life::new();
    place::drips(&mut life);
    (z, Box::new(Glasswater { decor, water, life, player: (0.0, 0.0), t: 0 }))
}

/// An invisible prop that only blocks movement (for code-drawn crystals and rocks).
pub fn blocker(z: &mut Zone, x: f32, y: f32, w: f32, h: f32) -> usize {
    let mut p = Prop::new("glasswater_blocker", "none", x, y);
    p.footprint = Some((w, h));
    z.props.push(p);
    z.props.len() - 1
}

pub struct Glasswater {
    decor: Vec<Decor>,
    water: water::Water,
    life: life::Life,
    player: (f32, f32),
    t: u64,
}

impl Region for Glasswater {
    fn update(&mut self, zone: &mut Zone, ctx: &RegionCtx, fx: &mut Fx) {
        self.t = ctx.t;
        self.player = ctx.player;
        // wading: everything moving through water pushes rings out from its feet
        for &(x, y, speed) in ctx.movers {
            if zone.tile_at(x, y) == Tile::Shallow && speed > 0.15 && self.water.is_water(x as i32, y as i32) {
                if ctx.t % 3 == 0 {
                    self.water.poke(x, y, 3.0, 0.55 + speed.min(3.0) * 0.25);
                }
            }
        }
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
        f.look.light_dither = 0.12;
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
            self.water.reflect(f, &d.cell, d.anchor, d.x, d.y, d.flip, d.emit_gain(t), t);
            let _ = y;
        }
    }

    fn sorted_items(&self, _zone: &Zone, f: &Frame, out: &mut Vec<(f32, u32)>) {
        for (i, d) in self.decor.iter().enumerate() {
            if !d.alive {
                continue;
            }
            let (x, y, w, h) = d.bounds();
            if f.visible(x, y, w, h, 8.0) {
                out.push((d.y, i as u32));
            }
        }
    }

    fn draw_item(&self, _zone: &Zone, f: &mut Frame, id: u32, t: u64) {
        let Some(d) = self.decor.get(id as usize) else {
            return;
        };
        let wob = if d.wobble > 0.0 { ((d.wobble * 1.7).sin() * d.wobble * 0.2).round() } else { 0.0 };
        let o = DrawOpts { flip: d.flip, flash: d.flash, emit_gain: d.emit_gain(t), ..Default::default() };
        f.sprite_w(&d.cell, d.anchor, d.x + wob, d.y, &o);
    }

    fn draw_overlay(&self, _zone: &Zone, f: &mut Frame, _t: u64) {
        self.life.draw_overlay(f);
    }

    fn light(&self, _zone: &Zone, f: &mut Frame, t: u64) {
        let tf = t as f32;
        for d in &self.decor {
            if !d.alive {
                continue;
            }
            if let Some((c, r, i, hgt)) = d.light {
                let breathe = match d.kind {
                    Kind::Mushroom => 0.0,
                    _ => 0.94 + 0.06 * (tf * 0.013 + d.phase).sin(),
                };
                f.add_light_w(d.x, d.y - hgt, r * (1.0 + d.ring * 0.25), c, i * breathe + d.ring * 0.6);
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
