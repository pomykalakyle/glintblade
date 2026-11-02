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
use crate::gfx::{Rgb, rgb, sheet};
use crate::world::{TILE, Tile, Zone};

const T: f32 = TILE as f32;
const VIOLET_LIGHT: Rgb = rgb(0xcdc3db);

/// The point source belongs to the geometric centers of actual emissive native
/// pixels. Atlas anchors address pixel origins, so centers contribute +0.5px.
fn crystal_light_crest(d: &Decor) -> Option<(f32, f32)> {
    let (mut sx, mut sy, mut count) = (0.0, 0.0, 0usize);
    for (i, _) in d.cell.px.iter().enumerate().filter(|(_, p)| p.a && p.emit) {
        sx += (i as i32 % d.cell.w) as f32;
        sy += (i as i32 / d.cell.w) as f32;
        count += 1;
    }
    if count == 0 { return None; }
    let offset = sx / count as f32 - d.anchor.0 as f32;
    Some((if d.flip { -offset } else { offset } + 0.5,
        d.anchor.1 as f32 - sy / count as f32 - 0.5))
}

struct P<'a> {
    z: &'a mut Zone,
    d: &'a mut Vec<Decor>,
}

/// A native foot wholly resting on raised horizontal caps must clear their
/// highest support altitude. A nearby wall cannot lift a prop rooted on ordinary floor.
fn support_elevation(z: &Zone, prop: &crate::world::Prop) -> f32 {
    let Some([left, top, right, bottom]) = prop.collision_bounds() else { return 0.0 };
    let mut highest = 0;
    let mut found = false;
    for y in top.floor() as i32..bottom.ceil() as i32 {
        for x in left.floor() as i32..right.ceil() as i32 {
            if !prop.blocks(x as f32 + 0.5, y as f32 + 0.5, 0.01, 0.01) { continue; }
            if x < 0 || y < 0 || x >= z.pw || y >= z.ph { return 0.0; }
            let surface = z.ground_surfaces[(y * z.pw + x) as usize];
            if z.tile_at(x as f32 + 0.5, y as f32 + 0.5) != Tile::Wall
                || !surface.present() || surface.upright || surface.height == 0 { return 0.0; }
            highest = highest.max(surface.height);
            found = true;
        }
    }
    if found { f32::from(highest) } else { 0.0 }
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
        let native = sheet(if size >= 38 { "glasswater_crystals_native" } else { "glasswater_crystals_small" }).is_some();
        d.flip = flip && !native;
        // Locate the real native emissive crest. The earlier size-based height
        // placed every source near the base and exposed identical blue disks.
        // A broad, gentler pool now falls from the actual crystal geometry.
        if let Some((offset_x, height)) = crystal_light_crest(&d) {
            d.light_offset_x = offset_x;
            let radius = 36.0 + height * 0.85 + d.cell.w as f32 * 0.15;
            let intensity = if size >= 38 { 0.30 } else { 0.23 };
            d.light = Some((VIOLET_LIGHT, radius, intensity, height));
        }
        d.prop = Some(blocker(self.z, &d, 8));
        d.elevation = support_elevation(self.z, &self.z.props[d.prop.unwrap()]);
        // Static reception follows this actual crystal's crest, independently of the
        // support shape. The live animated light remains owned by Decor::light.
        self.z.props[d.prop.unwrap()].light = d.light.map(|(color, radius, _, height)|
            (color, radius, 0.0, d.light_offset_x, -height));
        self.d.push(d);
        self.d.len() - 1
    }

    fn sprig(&mut self, tx: f32, ty: f32, size: i32, _violet: bool, seed: u32) {
        self.check(tx, ty, "sprig");
        let ramp = &VIOLET;
        let (cell, anchor) = art::crystal_sprig(seed, ramp, size);
        let mut d = Decor::new(cell, anchor, tx * T, ty * T, Kind::Sprig);
        if let Some((offset_x,height))=crystal_light_crest(&d) {
            d.light_offset_x=offset_x;
            d.light=Some((VIOLET_LIGHT,10.0+size as f32*0.4,0.10,height));
        }
        self.d.push(d);
    }

    fn mushroom(&mut self, tx: f32, ty: f32, cap: i32, stem: i32, lean: i32, seed: u32) {
        self.check(tx, ty, "mushroom");
        let (cell, anchor) = art::mushroom(seed, cap, stem, lean);
        let (x, y) = (tx * T, ty * T);
        let mut d = Decor::new(cell, anchor, x, y, Kind::Mushroom);
        // Fungi are quiet secondary forms, not another source of colored pools.
        if stem >= 12 {
            d.prop = Some(blocker(self.z, &d, 4));
        }
        self.d.push(d);
    }

    fn patch(&mut self, tx: f32, ty: f32, n: i32, seed: u32) {
        self.check(tx, ty, "patch");
        let (cell, anchor) = art::mushroom_patch(seed, n);
        let mut d = Decor::new(cell, anchor, tx * T, ty * T, Kind::Mushroom);
        if sheet("glasswater_fungi").is_some() {
            d.prop = Some(blocker(self.z, &d, 8));
        }
        self.d.push(d);
    }

    fn spire(&mut self, tx: f32, ty: f32, size: i32, seed: u32) {
        self.check(tx, ty, "spire");
        let (cell, anchor) = if matches!(seed, 39 | 173 | 99) { art::sluice_fin(seed, if seed == 173 { 112 } else { 96 }) } else { art::stalagmite(seed, size) };
        let (x, y) = (tx * T, ty * T);
        let mut d = Decor::new(cell, anchor, x, y, Kind::Rock);
        let native_sluice = matches!(seed, 39 | 173 | 99) && sheet("glasswater_waterworks").is_some();
        d.prop = Some(blocker(self.z, &d, if native_sluice { 9 } else { 5 }));
        self.d.push(d);
    }

    fn boulder(&mut self, tx: f32, ty: f32, rx: i32, ry: i32, seed: u32) {
        self.check(tx, ty, "boulder");
        let (cell, anchor) = art::boulder(seed, rx, ry);
        let (x, y) = (tx * T, ty * T);
        let mut d = Decor::new(cell, anchor, x, y, Kind::Rock);
        d.prop = Some(blocker(self.z, &d, ry.clamp(3, 6)));
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
    // Seat the full native foot on the raised cap, clear of its front face.
    let island = p.crystal(32.5, 24.925, 52, false, 31, false);
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
    p.patch(68.3, 12.4, 4, 75);
    p.patch(83.0, 21.0, 3, 76);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Check the completed native scene, rather than only each prop's anchor.
    /// A wide foot can hang into deep water while its anchor is on the bank.
    #[test]
    fn every_structural_foot_is_seated_on_actual_terrain() {
        let (z, _) = super::super::build();
        let terrain = super::super::paint::classify(&z);
        let mut tested = 0;
        for p in &z.props {
            let Some([left, top, right, bottom]) = p.collision_bounds() else { continue };
            for y in top.floor() as i32..bottom.ceil() as i32 {
                for x in left.floor() as i32..right.ceil() as i32 {
                    if !p.blocks(x as f32 + 0.5, y as f32 + 0.5, 0.01, 0.01) { continue; }
                    tested += 1;
                    assert!(matches!(z.tile_at(x as f32 + 0.5, y as f32 + 0.5),
                        Tile::Floor | Tile::Shallow | Tile::Wall),
                        "{}:{} has an unsupported structural foot at {x},{y}", p.sheet, p.anim);
                    if (p.x - 520.0).abs() < 0.1 {
                        assert_eq!(terrain.at(x, y), super::super::paint::WALL_TOP,
                            "the main-lake crystal must rest on its island cap, not its vertical face at {x},{y}");
                    }
                }
            }
        }
        assert!(tested > 1_000, "exercise the full native scenery, not an empty fixture");
    }

    /// The island's crystal keeps its native projected pose while its source,
    /// receiving pixels and entire native caster stand on the same raised cap.
    #[test]
    fn raised_crystal_attaches_its_light_and_native_caster_to_actual_cap_height() {
        let (mut z, _) = super::super::build();
        let m = super::super::paint::classify(&z);
        let fields = super::super::paint::fields(&m);
        z.props.clear();
        let mut decor = Vec::new();
        populate(&mut z, &mut decor, &m);
        let id = decor.iter().position(|d| d.mirror.is_some()).unwrap();
        let elevation = decor[id].elevation;
        assert_eq!(elevation, 30.0, "the real foot spans the island's 26px and 30px caps");
        assert!(decor.iter().enumerate().all(|(i, d)| i == id || d.elevation == 0.0),
            "a neighboring cliff must not lift ordinary floor-rooted crystals");
        let water = super::super::water::Water::new(&m, &fields, &z.ground, &decor);
        let mut region = super::super::Glasswater { decor, water, life: Life::new(),
            player: (0.0, 0.0), t: 0, water_movers: Vec::new(), water_hit_ids: Vec::new() };
        let frame = || {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::WHITE);
            f.surface_lighting = true;
            f.cam_x = 336;
            f.cam_y = 290;
            f
        };
        region.decor[id].elevation = 0.0;
        let flat_source = region.decor[id].point_light(90).unwrap();
        let mut flat = frame();
        region.draw_item(&z, &mut flat, id as u32, 90);
        region.decor[id].elevation = elevation;
        let source = region.decor[id].point_light(90).unwrap();
        let mut raised = frame();
        region.draw_item(&z, &mut raised, id as u32, 90);
        assert_eq!(raised.albedo, flat.albedo, "support altitude cannot move or repaint native pixels");
        assert_eq!(raised.emissive, flat.emissive, "the painted crest keeps its exact projected pose");
        let mut received = 0;
        for (a, b) in raised.surfaces.iter().zip(&flat.surfaces) {
            if b.owner == 0 { continue; }
            received += 1;
            assert_eq!((a.owner, a.kind, a.normal), (b.owner, b.kind, b.normal));
            assert_eq!(a.height, b.height + elevation);
        }
        assert!(received > 100);
        let (a, b) = (&raised.surface_casters[0], &flat.surface_casters[0]);
        assert_eq!(a.base, b.base + elevation);
        assert_eq!(a.min_height, elevation);
        for (a, b) in a.foot.iter().zip(&*b.foot) {
            for axis in 0..2 { assert_eq!(a[axis], b[axis] + elevation); }
        }
        let d = &region.decor[id];
        let prop = &z.props[d.prop.unwrap()];
        let [left, top, right, bottom] = prop.collision_bounds().unwrap();
        let mut feet = 0;
        for y in top.floor() as i32..bottom.ceil() as i32 {
            for x in left.floor() as i32..right.ceil() as i32 {
                if !prop.blocks(x as f32 + 0.5, y as f32 + 0.5, 0.01, 0.01) { continue; }
                let rendered = raised.surfaces[(y - raised.cam_y) as usize * W + (x - raised.cam_x) as usize];
                let cap = z.ground_surfaces[(y * z.pw + x) as usize];
                assert!(rendered.owner > 0 && rendered.height >= f32::from(cap.height),
                    "native foot embedded below its cap at {x},{y}");
                feet += 1;
            }
        }
        assert_eq!(feet, 194);
        assert_eq!((source.x, source.y), (flat_source.x, flat_source.y));
        assert_eq!(source.height, flat_source.height + elevation);
        region.light(&z, &mut raised, 90);
        let live = raised.point_lights.iter().find(|p| p.x == source.x && p.y == source.y).unwrap();
        assert_eq!(live.height, source.height);
        assert_eq!(live.y + live.height, d.y.floor() + elevation);
        let mut occluders = Vec::new();
        region.light_occluders(90, &mut occluders);
        let own = occluders[id].own_light.unwrap();
        assert_eq!((own.x, own.y, own.height), (live.x, live.y, live.height));
    }
    use crate::gfx::{Cell, DrawOpts, Frame, W, H};
    use crate::world::{Region, ZoneId};

    #[test]
    fn every_populated_crystal_lights_from_actual_native_emissive_housing() {
        // These are the actual crystal size/seed pairs in populate, including
        // the former empty-mask regressions 32,34,42,93 and96.
        let placements = [(26,11),(52,31),(34,32),(30,33),(26,34),(22,42),
            (44,50),(24,52),(20,57),(18,62),(18,63),(38,71),(46,72),
            (30,78),(22,79),(30,91),(30,92),(50,93),(34,96)];
        let mut zone = Zone::new(ZoneId::Glasswater, 24, 16);
        let mut decor = Vec::new();
        {
            let mut place = P { z: &mut zone, d: &mut decor };
            for &(size, seed) in &placements { place.crystal(12.0, 9.0, size, true, seed, true); }
        }
        let mut frame = Frame::new();
        for (d, &(_, seed)) in decor.iter().zip(&placements) {
            let point = d.point_light(0).expect("every placed crystal has a housed light");
            let mut pixels = Vec::new();
            for (i, p) in d.cell.px.iter().enumerate() {
                if !p.a || !p.emit { continue; }
                let (x, y) = ((i as i32 % d.cell.w) as f32, (i as i32 / d.cell.w) as f32);
                let dx = x - d.anchor.0 as f32;
                pixels.push((d.x + if d.flip { -dx } else { dx } + 0.5,
                    d.y - d.anchor.1 as f32 + y + 0.5,
                    d.anchor.1 as f32 - y - 0.5));
            }
            assert!(!pixels.is_empty() && pixels.len() <= 8, "seed {seed} has no small native crest");
            let count = pixels.len() as f32;
            let center = pixels.iter().fold((0.0,0.0,0.0), |sum,p|
                (sum.0+p.0/count,sum.1+p.1/count,sum.2+p.2/count));
            assert!((point.x - center.0).abs() < 0.0001, "seed {seed} source x");
            assert!((point.y - center.1).abs() < 0.0001, "seed {seed} projected source y");
            assert!((point.height - center.2).abs() < 0.0001, "seed {seed} source height");
            assert!((point.y + point.height - d.y).abs() < 0.0001);
            assert!(pixels.iter().any(|p|
                (point.x-p.0).abs() <= 3.0 && (point.y-p.1).abs() <= 3.0
                    && (point.height-p.2).abs() <= 3.0), "seed {seed} point outside actual housing");
            for (p,n) in d.cell.px.iter().zip(d.cell.surface.iter()) {
                assert_eq!((p.a,p.emit),(n.a,n.emit));
            }
            let proxy = zone.props[d.prop.unwrap()].light.unwrap();
            assert_eq!(proxy.2,0.0);
            assert!((d.x+proxy.3-point.x).abs() < 0.0001);
            assert!((d.y+proxy.4-point.y).abs() < 0.0001);

            frame.begin(Rgb::BLACK, Rgb::WHITE);
            frame.surface_lighting = true;
            frame.sprite_w(&d.cell, d.anchor, d.x, d.y,
                &DrawOpts { flip: d.flip, ..Default::default() });
            let visibility = crate::gfx::lighting::point_visibility(&frame,point);
            assert_eq!(visibility.len(),W*H);
            assert!(visibility.iter().all(|&v| v==255), "seed {seed} self-occludes its own housed source");
        }
    }

    #[test]
    fn an_empty_crystal_mask_cannot_invent_a_size_based_source() {
        let d = Decor::new(Cell::new(2,2,vec![Default::default();4]),(1,1),20.0,20.0,Kind::Crystal);
        assert!(crystal_light_crest(&d).is_none());
    }

    #[test]
    fn crest_centers_follow_the_renderers_mirrored_anchor_convention() {
        let cell = Cell::from_ascii(&["x.",".."], &[('x',Rgb::WHITE)], &['x']);
        let mut d = Decor::new(cell,(1,1),20.0,20.0,Kind::Crystal);
        for flip in [false,true] {
            d.flip = flip;
            let (offset,height) = crystal_light_crest(&d).unwrap();
            let mut frame = Frame::new();
            frame.begin(Rgb::BLACK,Rgb::WHITE);
            frame.sprite_w(&d.cell,d.anchor,d.x,d.y,&DrawOpts{flip,..Default::default()});
            let px = (d.x+offset).floor() as usize;
            let py = (d.y-height).floor() as usize;
            assert_eq!(frame.emissive[py*W+px],Rgb::WHITE);
            assert_eq!(height,0.5);
        }
    }

    #[test]
    fn live_crystal_light_tracks_fractional_mirrored_and_wobbling_raster() {
        for flip in [false, true] {
            for wobble in [0.0, 6.0] {
                let mut zone = Zone::new(ZoneId::Glasswater, 24, 16);
                zone.tiles.fill(Tile::Floor);
                let mut decor = Vec::new();
                { let mut p = P { z: &mut zone, d: &mut decor }; p.crystal(12.0, 9.0, 34, true, 32, false); }
                let d = &mut decor[0];
                d.x += 0.73;
                d.y += 0.81;
                d.flip = flip;
                d.wobble = wobble;
                let (offset, height) = crystal_light_crest(d).unwrap();
                d.light_offset_x = offset;
                d.light.as_mut().unwrap().3 = height;
                let map = super::super::paint::classify(&zone);
                let fields = super::super::paint::fields(&map);
                let water = super::super::water::Water::new(&map, &fields, &zone.ground, &decor);
                let region = super::super::Glasswater { decor, water, life: Life::new(), player: (0.0, 0.0), t: 0, water_movers: Vec::new(), water_hit_ids: Vec::new() };
                let mut frame = Frame::new();
                frame.surface_lighting = true;
                region.draw_item(&zone, &mut frame, 0, 90);
                region.light(&zone, &mut frame, 90);
                let pixels: Vec<_> = frame.emissive.iter().enumerate()
                    .filter(|(_, c)| c.r + c.g + c.b > 0.0)
                    .map(|(i, _)| ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5))
                    .collect();
                assert!(!pixels.is_empty());
                let center = pixels.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
                let light = frame.point_lights[0];
                assert!((light.x - center.0 / pixels.len() as f32).abs() < 0.0001);
                assert!((light.y - center.1 / pixels.len() as f32).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn small_sprig_source_follows_its_existing_emissive_pixels() {
        let mut zone=Zone::new(ZoneId::Glasswater,24,16);let mut decor=Vec::new();
        {let mut p=P{z:&mut zone,d:&mut decor};p.sprig(12.0,9.0,7,true,38);}
        let d=&decor[0];let lamp=d.point_light(0).expect("existing bright sprig cap");
        let points:Vec<_>=d.cell.px.iter().enumerate().filter(|(_,p)|p.emit).map(|(i,_)|(
            d.x+(i as i32%d.cell.w-d.anchor.0)as f32+0.5,
            d.y+(i as i32/d.cell.w-d.anchor.1)as f32+0.5)).collect();
        assert!(!points.is_empty());let n=points.len()as f32;
        assert!((lamp.x-points.iter().map(|p|p.0).sum::<f32>()/n).abs()<0.0001);
        assert!((lamp.y-points.iter().map(|p|p.1).sum::<f32>()/n).abs()<0.0001);
    }

}
