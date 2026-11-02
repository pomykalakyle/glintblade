//! Broken slate shoulders connect walkable floor to the descending ravine.
//! The 2D native contour owns the bevel, including lateral banks that have no
//! screen-Y ledge depth. Collision and the water/bridge boundaries stay exact.

use super::{X0, X1, Y0, Y1, WIDTH, crossing, material};
use super::super::paint::{FLOOR, Fields, PixMap, VOID};
use crate::gfx::Rgb;
use crate::world::Zone;

const REACH: i32 = 12;

#[derive(Clone, Copy)]
struct Root {
    x: i32,
    y: i32,
    outward: [f32; 2],
    width: f32,
}

struct Shoulders {
    roots: Vec<Root>,
    nearest: Vec<usize>,
    distance: Vec<u16>,
}

/// Unequal long bedding runs make wider shoulders and narrow broken noses.
/// Width follows geological blocks, rather than one uniform outlined ribbon.
fn width(x: i32, y: i32) -> f32 {
    let mut local = (x + y / 2).rem_euclid(179);
    for (span, wide) in [(31, 8), (47, 5), (23, 9), (42, 6), (36, 8)] {
        if local < span {
            return (wide - (4 - local.min(span - 1 - local)).max(0)).max(3) as f32;
        }
        local -= span;
    }
    unreachable!()
}

impl Shoulders {
    fn new(map: &PixMap, fields: &Fields) -> Self {
        let mut roots = Vec::new();
        for y in Y0..Y1.min(map.h) { for x in X0..X1.min(map.w) {
            let i = (y * map.w + x) as usize;
            if map.cls[i] != FLOOR || crossing(x, y) || fields.d_water[i] < 10
                || ![(-1,0),(1,0),(0,-1),(0,1)].iter().any(|&(dx,dy)| map.at(x+dx,y+dy) == VOID)
            { continue; }
            // Average the real contour over a few native pixels; one-pixel
            // notches cannot turn an entire geological shoulder sideways.
            let mut outward = [0.0f32; 2];
            for dy in -4i32..=4 { for dx in -4i32..=4 {
                if map.at(x+dx,y+dy) != VOID { continue; }
                let weight = (5-dx.abs()) * (5-dy.abs());
                outward[0] += (dx * weight) as f32;
                outward[1] += (dy * weight) as f32;
            }}
            let length = outward[0].hypot(outward[1]);
            if length < 0.1 { continue; }
            roots.push(Root {x,y,outward:outward.map(|v| v/length),width:width(x,y)});
        }}
        let n = (WIDTH * (Y1-Y0)) as usize;
        let mut nearest = vec![usize::MAX; n];
        let mut distance = vec![u16::MAX; n];
        for (id, root) in roots.iter().enumerate() {
            for dy in -REACH..=REACH { for dx in -REACH..=REACH {
                let (x,y) = (root.x+dx,root.y+dy);
                if !(X0..X1).contains(&x) || !(Y0..Y1).contains(&y) { continue; }
                let d2 = (dx*dx+dy*dy) as u16;
                let i = ((y-Y0)*WIDTH+x-X0) as usize;
                if d2 < distance[i] { distance[i] = d2; nearest[i] = id; }
            }}
        }
        Self {roots,nearest,distance}
    }

    fn at(&self, x: i32, y: i32) -> Option<(Root, f32)> {
        if !(X0..X1).contains(&x) || !(Y0..Y1).contains(&y) { return None; }
        let i = ((y-Y0)*WIDTH+x-X0) as usize;
        self.roots.get(self.nearest[i]).copied().map(|root| (root,(self.distance[i] as f32).sqrt()))
    }
}

fn interior(zone: &Zone, map: &PixMap, old: &[Rgb], root: Root) -> Rgb {
    for d in (3..=12).rev() {
        let x = root.x - (root.outward[0]*d as f32).round() as i32;
        let y = root.y - (root.outward[1]*d as f32).round() as i32;
        if map.at(x,y) == FLOOR && !crossing(x,y) {
            return old[(y*zone.pw+x) as usize];
        }
    }
    old[(root.y*zone.pw+root.x) as usize]
}

pub(super) fn paint(zone: &mut Zone, map: &PixMap, fields: &Fields) {
    let shoulders = Shoulders::new(map,fields);
    let old = zone.ground.clone();
    for y in Y0..Y1.min(map.h) { for x in X0..X1.min(map.w) {
        let i = (y*map.w+x) as usize;
        if crossing(x,y) || fields.d_water[i] < 7 || !matches!(map.cls[i], FLOOR|VOID)
            || map.cls[i] == VOID && map.ledge[i] == 2
        { continue; }
        let Some((root,distance)) = shoulders.at(x,y) else { continue; };
        let Some(cell) = material(x) else { continue; };
        let p = cell.at(x.rem_euclid(cell.w),y-Y0);
        if !p.a { continue; }
        let floor = interior(zone,map,&old,root);
        if map.cls[i] == FLOOR {
            if distance > root.width { continue; }
            let t = (1.0-distance/root.width).clamp(0.0,1.0);
            // Retain quiet floor colour, carrying only the largest native
            // fractures across the fold. The old single-pixel bright outline
            // gives way to an actual outward-tilting, irregular stone plane.
            let top = floor.lerp(p.c.scale(0.76),0.42);
            zone.ground[i] = old[i].lerp(top,t);
            let s = zone.ground_surfaces[i];
            let normal = [root.outward[0]*0.70*t,root.outward[1]*0.70*t,1.0];
            let normal = std::array::from_fn(|a| normal[a]*t+f32::from(s.normal[a])/127.0*(1.0-t));
            zone.gset_plane_surface(x,y,normal,0.0);
        } else {
            let span = root.width+2.0;
            if distance > span { continue; }
            // Three cut planes roll over the brink before the deep upright
            // material. These are native stone faces, never new player floor.
            let t = distance/span;
            let (slope,z,gain) = if t < 0.30 { (0.7,1.0,0.72) }
                else if t < 0.63 { (1.0,0.62,0.59) } else { (1.0,0.23,0.46) };
            let stone = p.c.scale(gain);
            zone.ground[i] = if t < 0.30 { floor.lerp(stone,0.72) } else { stone };
            zone.gset_plane_surface(x,y,[root.outward[0]*slope,root.outward[1]*slope,z],0.0);
        }
    }}
    zone.invalidate_lighting();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::lighting::TerrainSurface;

    #[test]
    fn shoulders_preserve_routes_water_bridge_and_all_other_terrain() {
        let (mut zone, _) = super::super::super::build();
        let map = super::super::super::paint::classify(&zone);
        let fields = super::super::super::paint::fields(&map);
        let albedo = zone.ground.clone();
        let surfaces = zone.ground_surfaces.clone();
        let collision = zone.surface_tiles.clone();
        let emit = zone.ground_emit.clone();
        paint(&mut zone,&map,&fields);
        let mut affected = 0;
        for y in 0..map.h { for x in 0..map.w {
            let i = (y*map.w+x) as usize;
            if zone.ground[i] == albedo[i] && zone.ground_surfaces[i] == surfaces[i] { continue; }
            affected += 1;
            assert!((X0..X1).contains(&x) && (Y0..Y1).contains(&y));
            assert!(!crossing(x,y));
            assert!(matches!(map.cls[i],FLOOR|VOID));
            assert!(fields.d_water[i] >= 7);
            assert!(map.cls[i] != VOID || map.ledge[i] != 2);
            assert_eq!(zone.ground_surfaces[i].height,0,"shoulder became an above-floor caster");
        }}
        assert!(affected > 1_000,"exercise actual rift shoulders: {affected}");
        assert_eq!(zone.surface_tiles,collision);
        assert_eq!(zone.ground_emit,emit);
    }

    #[test]
    fn real_lateral_banks_roll_from_walkable_top_to_outward_facing_stone() {
        let (mut zone, _) = super::super::super::build();
        let map = super::super::super::paint::classify(&zone);
        let fields = super::super::super::paint::fields(&map);
        let shoulders = Shoulders::new(&map,&fields);
        // Remove source normals to verify this pass owns the actual macro
        // planes, independently of albedo and optional native microrelief.
        zone.ground_surfaces.fill(TerrainSurface::default());
        paint(&mut zone,&map,&fields);
        let (mut floor,mut slope,mut front) = (0,0,0);
        let mut widths = std::collections::BTreeSet::new();
        for y in Y0..Y1 { for x in X0..X1 {
            let i = (y*map.w+x) as usize;
            let s = zone.ground_surfaces[i];
            if !s.present() { continue; }
            let Some((root,_)) = shoulders.at(x,y) else { continue; };
            if root.outward[0].abs() < 0.8 { continue; }
            let n = s.normal.map(|n| f32::from(n)/127.0);
            assert_eq!(s.height,0);
            widths.insert(root.width as i32);
            if map.cls[i] == FLOOR {
                assert!(!s.upright && n[2] > 0.6,"floor bevel stopped reading as walkable");
                floor += 1;
            } else if !s.upright {
                slope += 1;
                assert!(n[0]*root.outward[0]+n[1]*root.outward[1] > 0.45,
                    "side shoulder still uses the generic south-facing wall normal");
            } else {
                front += 1;
                // Equal lights on opposite sides must give different
                // incidence on the physical side face, independent of colour.
                assert!(n[0]*root.outward[0]+n[1]*root.outward[1] > 0.90);
                assert!(n[2] < 0.4);
            }
        }}
        assert!(floor > 250 && slope > 80 && front > 80,"missing a real plane: {floor}/{slope}/{front}");
        assert!(widths.len() >= 5,"one uniform outline width replaced the geology");
    }
}
