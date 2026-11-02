//! Native PixelLab materials clipped to the authored cave masks. Water and collision geometry
//! keep their own source of truth; generated pixels only replace receiving dry stone surfaces.

use super::art::ROCK;
use super::paint::{FACE, FLOOR, Fields, PixMap, VOID, WALL_TOP};
use crate::gfx::{rgb, sheet};
use crate::util::hash2;
use crate::world::Zone;

/// Whether the new floor is present. The old painter remains a useful complete fallback.
pub fn available() -> bool {
    sheet("glasswater_materials").is_some()
}

pub fn paint(z: &mut Zone, m: &PixMap, fl: &Fields) {
    let floor = sheet("glasswater_materials");
    let floor_anim = floor.and_then(|s| s.anim("floor"));
    let wall = sheet("glasswater_walls");
    let wall_anim = wall.and_then(|s| s.anim("body"));
    let floor_macro = sheet("glasswater_floor_macro");
    let macro_anim = floor_macro.and_then(|s| s.anim("body"));
    let top = sheet("glasswater_walltop");
    let top_anim = top.and_then(|s| s.anim("body"));
    let floor_count = floor_anim.map(|a| a.frames.max(1)).unwrap_or(1);
    for y in 0..m.h {
        for x in 0..m.w {
            let i = (y * m.w + x) as usize;
            match m.cls[i] {
                FLOOR => {
                    let (Some(s), Some(floor_anim)) = (floor, floor_anim) else { continue };
                    // Keep physically shaped receiving banks and contact shade. All source
                    // modules share a quiet border so unrelated neighbours cannot leave seams.
                    if fl.d_water[i] < 7 || fl.d_rock[i] < 4 || fl.d_void[i] < 3 { continue }
                    let (tx, ty) = (x.div_euclid(32), y.div_euclid(32));
                    let v = if floor_count <= 1 || hash2(tx, ty, 3300) < 0.60 { 0 }
                        else { 1 + (hash2(tx, ty, 3301) * (floor_count - 1) as f32) as usize % (floor_count - 1) };
                    let floor_cell = s.cell(floor_anim, v);
                    let p = floor_cell.at(x.rem_euclid(32), y.rem_euclid(32));
                    if !p.a { continue }
                    let old = z.ground[i];
                    // Retain receiving shadows but not the old painted rectangular slab lines.
                    let gain = (old.luma() / ROCK[2].luma().max(0.01)).clamp(0.58, 1.08);
                    let mut color = p.c;
                    z.gset_surface(x, y, floor_cell, x.rem_euclid(32), y.rem_euclid(32), false, 0.0);
                    if let (Some(ms), Some(ma)) = (floor_macro, macro_anim) {
                        let cell = ms.cell(ma, 0);
                        let q = cell.at(x.rem_euclid(cell.w), y.rem_euclid(cell.h));
                        if q.a && p.c.key() == rgb(0x4c6070).key() && (fl.d_rock[i] < 24 || fl.d_void[i] < 18) {
                            color = q.c;
                            z.gset_surface(x, y, cell, x.rem_euclid(cell.w), y.rem_euclid(cell.h), false, 0.0);
                        }
                    }
                    z.ground[i] = color.scale(gain);
                }
                WALL_TOP => {
                    let (Some(ts), Some(ta)) = (top, top_anim) else { continue };
                    // The actual cliff silhouette owns the broad cap. A hard
                    // 4..20px atlas window made pale rectangular ribbons that
                    // ended independently of the rock mass. Bed the native
                    // facets into that cap, losing their contrast gradually
                    // toward its quiet interior and its exposed lip.
                    let mix = cap_material_mix(fl.d_open[i]);
                    if mix <= 0.0 { continue }
                    let cell = ts.cell(ta, 0);
                    let p = cell.at(x.rem_euclid(cell.w), y.rem_euclid(cell.h));
                    if p.a {
                        let base = z.ground[i];
                        let surface = z.ground_surfaces[i];
                        let gain = (base.luma() / ROCK[2].luma()).clamp(0.72, 1.20);
                        z.ground[i] = base.lerp(p.c.scale(gain), mix);
                        z.gset_surface(x, y, cell, x.rem_euclid(cell.w), y.rem_euclid(cell.h), false, f32::from(m.span[i]));
                        // Relief belongs to the same embedded material as its
                        // colour; the quiet cap remains a horizontal mass.
                        let native = z.ground_surfaces[i];
                        let normal = std::array::from_fn(|axis| {
                            (f32::from(surface.normal[axis]) * (1.0 - mix)
                                + f32::from(native.normal[axis]) * mix) / 127.0
                        });
                        z.gset_plane_surface(x, y, normal, f32::from(m.span[i]));
                    }
                }
                FACE => {
                    let (Some(ws), Some(wa)) = (wall, wall_anim) else { continue };
                    let cell = ws.cell(wa, (x.div_euclid(128) as usize) % wa.frames.max(1));
                    let sx = x.rem_euclid(cell.w);
                    // The native art has a 24px top and a 40px face. Crop the actual face
                    // height rather than squeezing it to fit every variable-height wall.
                    let sy = if m.depth[i] >= m.span[i].saturating_sub(2) { 62 + i32::from(m.depth[i] + 1 == m.span[i]) }
                        else { 24 + i32::from(m.depth[i]).min(37) };
                    let p = cell.at(sx, sy);
                    if p.a {
                        z.ground[i] = p.c;
                        z.gset_surface(x, y, cell, sx, sy, true, i32::from(m.span[i].saturating_sub(m.depth[i])) as f32);
                    }
                }
                VOID if m.depth[i] < m.span[i] && m.ledge[i] != 2 => {
                    let (Some(ws), Some(wa)) = (wall, wall_anim) else { continue };
                    let cell = ws.cell(wa, (x.div_euclid(128) as usize) % wa.frames.max(1));
                    let p = cell.at(x.rem_euclid(cell.w), 24 + i32::from(m.depth[i]).min(39));
                    if p.a {
                        let gain = if m.depth[i] >= m.span[i].saturating_sub(6) { 0.50 } else { 0.78 };
                        z.ground[i] = p.c.scale(gain);
                        // The ledge hangs below floor level, unlike a raised
                        // wall face. Its painted depth is not positive height.
                        z.gset_surface(x, y, cell, x.rem_euclid(cell.w), 24 + i32::from(m.depth[i]).min(39), true, 0.0);
                    }
                }
                _ => {}
            }
        }
    }
}

fn cap_material_mix(distance: u8) -> f32 {
    let lip = f32::from(distance.saturating_sub(3).min(4)) / 4.0;
    let interior = 1.0 - f32::from(distance.saturating_sub(12).min(16)) / 16.0;
    lip * interior * 0.5
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::paint::{DEEP, SHALLOW};

    /// Material art must not erase the simulation's receiving water or fill abyss holes.
    #[test]
    fn generated_materials_preserve_water_and_abyss() {
        let (mut z, _) = super::super::build();
        let m = super::super::paint::classify(&z);
        let fl = super::super::paint::fields(&m);
        let before = z.ground.clone();
        paint(&mut z, &m, &fl);
        let mut protected = 0;
        for i in 0..m.cls.len() {
            if matches!(m.cls[i], SHALLOW | DEEP) || m.cls[i] == VOID && (m.depth[i] >= m.span[i] || m.ledge[i] == 2) {
                protected += 1;
                assert_eq!(z.ground[i], before[i], "material art overwrote protected pixel {i}");
            }
        }
        assert!(protected > 100_000);
    }
}
