//! The central rift's lower rock, attached to the final native bank silhouette.
//! Near ledges remain real light-reactive terrain. The deeper continuation is
//! distant backdrop: it neither invents walkable floor nor casts an above-floor
//! wall. All source pixels stay on the native grid.

mod rim;

pub(super) fn paint_rim(zone: &mut Zone, map: &PixMap, fields: &super::paint::Fields) {
    rim::paint(zone, map, fields);
}

use super::art::ROCK;
use super::paint::{DEEP, PixMap, SHALLOW, VOID};
use crate::gfx::{Cell, Frame, H, Rgb, W, sheet};
use crate::world::Zone;

const X0: i32 = 704;
const X1: i32 = 1008;
const Y0: i32 = 128;
const Y1: i32 = 832;
const WIDTH: i32 = X1 - X0;
const DROP: i32 = 120;

fn crossing(x: i32, y: i32) -> bool {
    (752..944).contains(&x) && (240..310).contains(&y)
}

fn material(x: i32) -> Option<&'static Cell> {
    let s = sheet("glasswater_ravine")?;
    let a = s.anim("body")?;
    let cell = s.cell(a, usize::from(x >= 848).min(a.frames.saturating_sub(1)));
    (cell.w == 128 && cell.h == Y1 - Y0).then_some(cell)
}

/// Only replace the already-visible hanging stone beneath natural banks.
/// Raised cave walls, water, bridge masonry, holes and collision are untouched.
pub(super) fn paint_near(zone: &mut Zone, map: &PixMap) {
    for y in Y0..Y1.min(map.h) {
        for x in X0..X1.min(map.w) {
            let i = (y * map.w + x) as usize;
            if crossing(x, y) || map.cls[i] != VOID || map.ledge[i] == 2
                || map.depth[i] >= map.span[i] || zone.ground[i].r < 0.0
            { continue; }
            let Some(cell) = material(x) else { continue; };
            let (sx, sy) = (x.rem_euclid(cell.w), y - Y0);
            let p = cell.at(sx, sy);
            if !p.a { continue; }
            let d = i32::from(map.depth[i]);
            let gain = if d < 5 { 0.68 } else if d < 18 { 0.58 }
                else if d < 30 { 0.47 } else { 0.38 };
            zone.ground[i] = p.c.scale(gain);
            zone.gset_surface(x, y, cell, sx, sy, true, 0.0);
        }
    }
}

pub(super) struct Ravine {
    pixels: Vec<Rgb>,
    #[cfg(test)]
    roots: Vec<Option<(i32, i32, i32)>>,
}

impl Ravine {
    pub(super) fn new(zone: &Zone, map: &PixMap) -> Self {
        let base = ROCK[0].scale(0.35);
        let mut pixels = vec![base; (WIDTH * (Y1 - Y0)) as usize];
        let mut depths = vec![u8::MAX; pixels.len()];
        #[cfg(test)]
        let mut roots = vec![None; pixels.len()];
        let hole = |x: i32, y: i32| {
            x >= 0 && y >= 0 && x < zone.pw && y < zone.ph
                && zone.ground[(y * zone.pw + x) as usize].r < 0.0
        };
        // Seed the ACTUAL finished ledge foot, including its native notches.
        // Side returns batter inward only 1px per 8px of drop; all faces share
        // screen_y = Y-Z. The bridge is never a seed (no invented curtain/piers).
        for y in Y0..Y1.min(zone.ph) {
            for x in X0..X1.min(zone.pw) {
                let i = (y * zone.pw + x) as usize;
                if hole(x, y) || crossing(x, y) || matches!(map.cls[i], SHALLOW | DEEP)
                    || map.cls[i] == VOID && map.ledge[i] == 2
                { continue; }
                for direction in [-1, 0, 1] {
                    let exposed = if direction == 0 { hole(x, y + 1) }
                        else { hole(x + direction, y) };
                    if !exposed { continue; }
                    for d in 1..=DROP {
                        let (wx, wy) = (x + direction * d / 8, y + d);
                        if !(X0..X1).contains(&wx) || !(Y0..Y1).contains(&wy)
                            || !hole(wx, wy)
                        { continue; }
                        let j = ((wy - Y0) * WIDTH + wx - X0) as usize;
                        if d >= i32::from(depths[j]) { continue; }
                        depths[j] = d as u8;
                        #[cfg(test)]
                        { roots[j] = Some((x, y, d)); }
                        let c = material(x).map(|cell| cell.at(x.rem_euclid(cell.w), wy - Y0))
                            .filter(|p| p.a).map_or(ROCK[1], |p| p.c);
                        // Unequal broad depth intervals dissolve the lower rock
                        // into the existing abyss, without fog, glow or noise.
                        let fade = if d < 18 { 1.0 } else if d < 43 { 0.76 }
                            else if d < 74 { 0.48 } else if d < 101 { 0.22 } else { 0.06 };
                        let facing = if direction < 0 { 0.34 } else if direction > 0 { 0.43 } else { 0.38 };
                        pixels[j] = base.lerp(c.scale(facing), fade);
                    }
                }
            }
        }
        Self { pixels, #[cfg(test)] roots }
    }

    pub(super) fn draw(&self, zone: &Zone, f: &mut Frame) {
        let base = ROCK[0].scale(0.35);
        for sy in 0..H as i32 {
            let wy = sy + f.cam_y;
            for sx in 0..W as i32 {
                let wx = sx + f.cam_x;
                let inside = wx >= 0 && wy >= 0 && wx < zone.pw && wy < zone.ph;
                if inside && zone.ground[(wy * zone.pw + wx) as usize].r >= 0.0 { continue; }
                let c = if (X0..X1).contains(&wx) && (Y0..Y1).contains(&wy) {
                    self.pixels[((wy - Y0) * WIDTH + wx - X0) as usize]
                } else { base };
                f.albedo[sy as usize * W + sx as usize] = c;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{rgb, frame::Surface};

    #[test]
    fn lower_faces_attach_to_natural_native_roots_and_never_the_bridge() {
        let (zone, _) = super::super::build();
        let map = super::super::paint::classify(&zone);
        let ravine = Ravine::new(&zone, &map);
        let mut attached = 0;
        for (i, root) in ravine.roots.iter().enumerate() {
            let Some((x, y, d)) = *root else { continue; };
            assert!(!crossing(x, y), "bridge grew a false support at {x},{y}");
            let source = (y * zone.pw + x) as usize;
            assert!(zone.ground[source].r >= 0.0);
            assert!(!matches!(map.cls[source], SHALLOW | DEEP));
            let target = ((i as i32 / WIDTH + Y0) * zone.pw + i as i32 % WIDTH + X0) as usize;
            assert!(zone.ground[target].r < 0.0, "backdrop crossed a physical rim");
            assert!((1..=DROP).contains(&d));
            attached += 1;
        }
        assert!(attached > 8_000, "exercise the full central rift: {attached}");
    }

    #[test]
    fn backdrop_is_world_registered_and_changes_only_hole_albedo() {
        let (zone, _) = super::super::build();
        let map = super::super::paint::classify(&zone);
        let ravine = Ravine::new(&zone, &map);
        let scene = |cam| {
            let mut f = Frame::new();
            f.begin(rgb(0xe400b9), rgb(0x246817));
            (f.cam_x, f.cam_y) = cam;
            f.surface_lighting = true;
            f.surfaces.fill(Surface {normal: [0.0, 1.0, 0.0], height: 17.0, owner: 0, kind: 5});
            let emissive = f.emissive.clone();
            let light = f.light.clone();
            let water = f.water.clone();
            ravine.draw(&zone, &mut f);
            assert_eq!(f.emissive, emissive);
            assert_eq!(f.light, light);
            assert_eq!(f.water, water);
            assert!(f.surface_casters.is_empty());
            assert!(f.surfaces.iter().all(|s| s.normal == [0.0,1.0,0.0] && s.height == 17.0 && s.kind == 5));
            for sy in 0..H { for sx in 0..W {
                let (wx, wy) = (sx as i32 + cam.0, sy as i32 + cam.1);
                if zone.ground[(wy * zone.pw + wx) as usize].r >= 0.0 {
                    assert_eq!(f.albedo[sy*W+sx], rgb(0xe400b9), "overwrote real ground at {wx},{wy}");
                }
            }}
            f
        };
        let a = scene((648,136));
        let b = scene((665,147));
        for y in 147..352 { for x in 665..1032 {
            let ia = (y-136) * W + x-648;
            let ib = (y-147) * W + x-665;
            assert_eq!(a.albedo[ia], b.albedo[ib], "camera moved a rock pixel at {x},{y}");
        }}
    }

    #[test]
    fn near_material_keeps_native_normals_and_all_gameplay_masks() {
        let (mut zone, _) = super::super::build();
        let map = super::super::paint::classify(&zone);
        let old = zone.ground.clone();
        let old_surfaces = zone.ground_surfaces.clone();
        let collision = zone.surface_tiles.clone();
        let emit = zone.ground_emit.clone();
        paint_near(&mut zone, &map);
        let mut replaced = 0;
        for y in 0..zone.ph { for x in 0..zone.pw {
            let i = (y * zone.pw + x) as usize;
            let eligible = (X0..X1).contains(&x) && (Y0..Y1).contains(&y) && !crossing(x,y)
                && map.cls[i] == VOID && map.ledge[i] != 2 && map.depth[i] < map.span[i] && old[i].r >= 0.0;
            if !eligible {
                assert_eq!(zone.ground[i], old[i], "repaint escaped scope at {x},{y}");
                assert_eq!(zone.ground_surfaces[i], old_surfaces[i]);
                continue;
            }
            let cell = material(x).expect("native material installed");
            let normal = cell.surface[((y - Y0) * cell.w + x.rem_euclid(cell.w)) as usize].upright(false);
            let surface = zone.ground_surfaces[i];
            assert_eq!(surface.height, 0, "descending rock became a raised caster");
            assert!(surface.upright);
            assert_eq!(surface.normal, crate::gfx::lighting::TerrainSurface::new(normal, 0.0).normal);
            replaced += 1;
        }}
        assert!(replaced > 2_000, "exercise both native banks: {replaced}");
        assert_eq!(zone.surface_tiles, collision);
        assert_eq!(zone.ground_emit, emit);
    }
}
