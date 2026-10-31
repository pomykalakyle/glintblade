//! Hand placement of Glasswater Hollow's crystals, mushrooms, rocks, drips and enemies.
//! Coordinates are in tiles (fractions allowed) and name the base of each sprite.
//!
//! Composition, area by area:
//! - Start cave: one iris cluster on the pool's far bank lights the first view; quiet otherwise.
//! - Lake chamber: the giant iris crystal on the island is the region's first landmark, mirrored
//!   in the deep water; small iris clusters frame the shore; the balcony looks into the rift.
//! - Gallery: a tall violet crystal in the north alcove reflected in the channel.
//! - Bridge: a crystal at each end, like lamp posts over the dark.
//! - Grove: quiet shelf fungi and a broken water distributor.
//! - Heart: the shard island in the pool, a huge violet crystal on the rift bank, two iris
//!   crystals flanking the gate.

use super::art::{self, VIOLET};
use super::life::{DripPoint, Life};
use super::paint::PixMap;
use super::{Decor, Kind, blocker};
use crate::entities::EnemyKind;
use crate::gfx::{Rgb, rgb};
use crate::world::{TILE, Tile, Zone};

const T: f32 = TILE as f32;
const VIOLET_LIGHT: Rgb = rgb(0xcdc3db);

struct P<'a> {
    z: &'a mut Zone,
    d: &'a mut Vec<Decor>,
}

impl P<'_> {
    fn check(&self, tx: f32, ty: f32, what: &str) {
        let t = self.z.tile_at(tx * T, ty * T - 1.0);
        if t != Tile::Floor && t != Tile::Shallow && std::env::var("GW_DEBUG").is_ok() {
            eprintln!("glasswater: {what} at ({tx}, {ty}) stands on {t:?}");
        }
    }

    fn crystal(&mut self, tx: f32, ty: f32, size: i32, _violet: bool, seed: u32, flip: bool) -> usize {
        self.check(tx, ty, "crystal");
        let ramp = &VIOLET;
        let (cell, anchor) = art::crystal_cluster(seed, ramp, size);
        let (x, y) = (tx * T, ty * T);
        let mut d = Decor::new(cell, anchor, x, y, Kind::Crystal);
        d.flip = flip;
        let radius = 18.0 + size as f32 * 0.55;
        d.light = Some((VIOLET_LIGHT, radius, 0.42, size as f32 * 0.45));
        let fw = (size as f32 * 0.5).clamp(8.0, 26.0);
        d.prop = Some(blocker(self.z, x, y, fw, 7.0));
        self.d.push(d);
        self.d.len() - 1
    }

    fn sprig(&mut self, tx: f32, ty: f32, size: i32, _violet: bool, seed: u32) {
        self.check(tx, ty, "sprig");
        let ramp = &VIOLET;
        let (cell, anchor) = art::crystal_sprig(seed, ramp, size);
        let mut d = Decor::new(cell, anchor, tx * T, ty * T, Kind::Sprig);
        d.light = Some((VIOLET_LIGHT, 10.0 + size as f32 * 0.4, 0.10, size as f32 * 0.5));
        self.d.push(d);
    }

    fn mushroom(&mut self, tx: f32, ty: f32, cap: i32, stem: i32, lean: i32, seed: u32) {
        self.check(tx, ty, "mushroom");
        let (cell, anchor) = art::mushroom(seed, cap, stem, lean);
        let (x, y) = (tx * T, ty * T);
        let mut d = Decor::new(cell, anchor, x, y, Kind::Mushroom);
        // Fungi are quiet secondary forms, not another source of colored pools.
        if stem >= 12 {
            d.prop = Some(blocker(self.z, x, y, 6.0, 4.0));
        }
        self.d.push(d);
    }

    fn patch(&mut self, tx: f32, ty: f32, n: i32, seed: u32) {
        self.check(tx, ty, "patch");
        let (cell, anchor) = art::mushroom_patch(seed, n);
        let d = Decor::new(cell, anchor, tx * T, ty * T, Kind::Mushroom);

        self.d.push(d);
    }

    fn spire(&mut self, tx: f32, ty: f32, size: i32, seed: u32) {
        self.check(tx, ty, "spire");
        let (cell, anchor) = if matches!(seed, 39 | 173 | 99) { art::sluice_fin(seed, if seed == 173 { 112 } else { 96 }) } else { art::stalagmite(seed, size) };
        let (x, y) = (tx * T, ty * T);
        let mut d = Decor::new(cell, anchor, x, y, Kind::Rock);
        d.prop = Some(blocker(self.z, x, y, (size as f32 * 0.35).clamp(6.0, 14.0), 5.0));
        self.d.push(d);
    }

    fn boulder(&mut self, tx: f32, ty: f32, rx: i32, ry: i32, seed: u32) {
        self.check(tx, ty, "boulder");
        let (cell, anchor) = art::boulder(seed, rx, ry);
        let (x, y) = (tx * T, ty * T);
        let mut d = Decor::new(cell, anchor, x, y, Kind::Rock);
        d.prop = Some(blocker(self.z, x, y, rx as f32 * 2.0 - 2.0, (ry as f32).max(4.0)));
        self.d.push(d);
    }

    fn enemy(&mut self, k: EnemyKind, tx: f32, ty: f32) {
        self.z.enemies.push((k, tx * T, ty * T));
    }
}

pub fn populate(z: &mut Zone, decor: &mut Vec<Decor>, _m: &PixMap) {
    let mut p = P { z, d: decor };
    use EnemyKind::{CrystalBeetle as Beetle, Wisp};

    // --- Start cave
    p.crystal(10.7, 22.7, 26, false, 11, false);
    p.sprig(4.3, 21.7, 9, true, 12);
    p.sprig(12.2, 29.2, 7, false, 13);
    p.patch(3.0, 29.7, 3, 14);
    p.spire(1.8, 22.6, 18, 15);

    // --- Passage into the lake chamber
    p.sprig(15.4, 25.7, 10, true, 21);

    // --- Lake chamber
    let island = p.crystal(32.5, 25.3, 52, false, 31, false);
    if let Some(i) = p.d[island].prop.take() {
        p.z.props[i].alive = false;
    }
    p.d[island].mirror = Some(27.1 * T);
    p.crystal(22.2, 19.7, 34, true, 32, true);
    p.crystal(43.0, 25.9, 30, false, 33, false);
    p.crystal(38.6, 34.8, 26, true, 34, false);
    p.sprig(26.2, 35.6, 8, false, 37);
    p.sprig(19.6, 27.7, 7, true, 38);
    p.spire(24.0, 14.6, 22, 39);
    p.spire(39.6, 14.6, 16, 40);
    p.boulder(36.6, 16.4, 6, 5, 41);
    // the balcony over the rift
    p.crystal(48.2, 30.4, 22, false, 42, true);
    p.enemy(Beetle, 27.0, 15.6);
    p.enemy(Beetle, 34.0, 15.2);
    p.enemy(Wisp, 38.0, 22.0);

    // --- Gallery
    p.crystal(33.3, 4.0, 44, true, 50, false);
    p.crystal(23.6, 3.8, 24, false, 52, true);
    p.sprig(41.6, 3.6, 9, false, 53);
    p.sprig(44.4, 4.3, 6, false, 54);
    p.crystal(52.4, 8.7, 20, false, 57, false);
    p.enemy(Beetle, 27.0, 7.6);
    p.enemy(Wisp, 38.0, 3.6);
    p.enemy(Wisp, 45.0, 6.0);

    // --- Ledge path and bridge
    p.spire(45.5, 12.5, 14, 61);
    p.crystal(46.2, 14.9, 18, false, 62, false);
    p.crystal(59.6, 14.9, 18, false, 63, true);

    // --- Grove
    p.crystal(65.8, 10.8, 38, false, 71, false);
    p.mushroom(64.6, 10.6, 6, 10, -2, 171);
    p.crystal(79.6, 19.6, 46, false, 72, false);
    p.spire(85.4, 10.2, 30, 173);
    p.patch(66.4, 11.4, 4, 75);
    p.patch(81.6, 20.4, 3, 76);
    p.crystal(88.3, 14.6, 30, true, 78, true);
    p.crystal(63.8, 18.8, 22, false, 79, false);
    p.enemy(Beetle, 72.0, 12.0);
    p.enemy(Beetle, 81.0, 14.0);
    p.enemy(Wisp, 67.0, 16.0);

    // --- Path south
    p.spire(70.5, 28.6, 20, 81);
    p.spire(74.4, 32.4, 15, 82);
    p.sprig(71.2, 35.5, 8, true, 83);

    // --- Heart
    p.crystal(79.1, 37.6, 30, false, 91, false);
    p.crystal(85.0, 37.8, 30, false, 92, true);
    p.crystal(60.6, 41.6, 50, true, 93, false);
    p.sprig(71.6, 44.8, 9, false, 94);
    p.sprig(74.0, 45.4, 7, false, 95);
    p.crystal(87.5, 44.8, 34, false, 96, true);
    p.spire(86.2, 47.6, 18, 99);
    p.boulder(59.6, 45.6, 5, 4, 100);
    p.enemy(Beetle, 64.0, 44.0);
    p.enemy(Beetle, 84.0, 46.0);
    p.enemy(Wisp, 69.0, 41.0);
    p.enemy(Wisp, 78.0, 44.0);
}

/// Drip spots over the pools: each lets a drop go on its own slow beat.
pub fn drips(life: &mut Life) {
    for &(tx, ty, period, offset) in &[(7.6, 23.6, 170u64, 0u64), (28.5, 21.5, 150, 40), (36.4, 30.5, 210, 120), (24.0, 26.0, 190, 77), (38.5, 5.6, 230, 10), (65.5, 21.5, 200, 60), (69.0, 42.5, 160, 30), (76.5, 46.5, 200, 140)] {
        life.drip_points.push(DripPoint { x: tx * T, y: ty * T, period, offset });
    }
}
