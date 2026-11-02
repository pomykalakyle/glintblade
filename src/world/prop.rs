//! Static or animated scenery drawn from sprite sheets: trees, pillars, crystals, urns...
//! Props are y-sorted with characters. Breakable props burst into pixels when slashed.
//! `Prop::new` reads "footprint", "breakable", "light" and "layer" from the anim's JSON
//! (falling back to the sheet's top level), so artists control them from the manifest.

use crate::fx::Fx;
use crate::gfx::{Cell, DrawOpts, Frame, Rgb, Sheet, sheet};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Small coherent moss pads grow on the stone's actual lowest three opaque
/// rows. Retaining every source alpha bit keeps physical support and the
/// silhouette exact; drawing the resulting cell preserves reveal/dither rules.
fn mossy_pillar(source: &'static Cell, variation: usize) -> &'static Cell {
    static CELLS: OnceLock<HashMap<usize, [Cell; 3]>> = OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let mut result = HashMap::new();
        if let Some(s) = sheet("bloomwild_ruins") {
            for a in s.anims.values().filter(|a| a.name.starts_with("pillar")) {
                for frame in 0..a.frames {
                    let c = s.cell(a, frame);
                    result.entry(c as *const Cell as usize).or_insert_with(|| std::array::from_fn(|v| pillar_foot(c, v)));
                }
            }
        }
        result
    });
    cells.get(&(source as *const Cell as usize)).map_or(source, |v| &v[variation % 3])
}

fn pillar_foot(source: &Cell, variation: usize) -> Cell {
    let mut cell = source.clone();
    let Some(bottom) = (0..source.h).rev().find(|&y| (0..source.w).any(|x| source.at(x,y).a)) else { return cell };
    let xs: Vec<_> = (0..source.w).filter(|&x| (bottom-2..=bottom).any(|y| source.at(x,y).a)).collect();
    let (Some(&left), Some(&right)) = (xs.first(),xs.last()) else { return cell };
    // Two irregular cushions, leaving native bare stone between them. Their
    // placement is coherent per pillar rather than grain scattered over a foot.
    let centers = [left + 3 + variation as i32, right - 3 - variation as i32];
    for y in (bottom-2).max(0)..=bottom {
        for x in left..=right {
            let p = source.at(x,y);
            if !p.a || p.emit { continue; }
            let d = centers.iter().map(|c| (x-c).abs()).min().unwrap();
            if d > if y == bottom-2 { 0 } else { 1 } { continue; }
            let moss = if y == bottom { crate::gfx::rgb(0x829b8e) } else { crate::gfx::rgb(0x647c77) };
            cell.px[(y * cell.w + x) as usize].c = p.c.lerp(moss, 0.58);
        }
    }
    cell
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub sheet: &'static str,
    pub anim: String,
    pub x: f32,
    pub y: f32,
    pub flip: bool,
    /// Animation time offset so identical props don't animate in lockstep.
    pub t0: u32,
    /// Solid collision box (w, h) whose bottom edge sits at the anchor y.
    pub footprint: Option<(f32, f32)>,
    /// Native structural base rectangles relative to the unflipped sprite anchor.
    /// A footprint enables collision; these spans refine it without adding invisible props.
    collision_rects: Vec<[f32; 4]>,
    collision_extent: Option<[f32; 4]>,
    pub breakable: bool,
    pub hp: i32,
    pub alive: bool,
    /// (color, radius, intensity, offset x, offset y)
    pub light: Option<(Rgb, f32, f32, f32, f32)>,
    /// Drawn above characters; fades to see-through when the player is behind it.
    pub canopy: bool,
    /// Hit flash timer.
    pub flash: f32,
    /// Small shake when hit (ticks).
    pub wobble: f32,
    /// What it drops when broken: number of healing motes.
    pub heal_drop: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Player, player::PlayerCtx};
    use crate::input::{Input, Raw};
    use crate::world::{Zone, ZoneId};

    #[test]
    fn low_native_foliage_receives_sun_without_rotating_stone_or_tree_normals() {
        use crate::gfx::{H, W};
        for (name, anim, low) in [
            ("bloomwild_props", "bush_a", true),
            ("bloomwild_props", "bush_b", true),
            ("bloomwild_props", "bush_bloom", true),
            ("bloomwild_props", "flowers_a", true),
            ("bloomwild_props", "flowers_b", true),
            ("bloomwild_props", "boulder_small", false),
            ("bloomwild_trees", "a_canopy", false),
        ] {
            let p = Prop::new(name, anim, 160.0, 180.0);
            let sheet = p.sheet().unwrap();
            let a = sheet.anim(anim).unwrap();
            let cell = sheet.cell(a, a.frame_at(p.t0, true));
            let anchor = sheet.anchor_of(a);
            let mut f = Frame::new();
            f.surface_lighting = true;
            p.draw(&mut f, 0, 1.0);
            let mut tested = 0;
            for y in 0..cell.h { for x in 0..cell.w {
                let px = 160 + x - anchor.0;
                let py = 180 + y - anchor.1;
                if px < 0 || py < 0 || px >= W as i32 || py >= H as i32 { continue; }
                let source = cell.at(x, y);
                if !source.a || source.emit { continue; }
                let i = py as usize * W + px as usize;
                let normal = cell.surface[(y * cell.w + x) as usize];
                assert_eq!(f.surfaces[i].normal, if low { normal.floor() } else { normal.upright(false) }, "{name}/{anim}");
                assert_eq!(f.albedo[i], source.c);
                tested += 1;
            } }
            assert!(tested > 10);
        }
    }

    #[test]
    fn canopy_reveal_fades_native_ink_without_changing_physical_opacity() {
        let mut canopy = Prop::new("bloomwild_trees", "a_canopy", 120.0, 180.0);
        canopy.canopy = true;
        let draw = |alpha| {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::WHITE);
            f.surface_lighting = true;
            canopy.draw(&mut f, 0, alpha);
            f
        };
        let opaque = draw(1.0);
        let revealed = draw(0.35);
        assert_eq!(opaque.surface_casters.len(), 1);
        assert_eq!(revealed.surface_casters.len(), 1);
        assert_eq!(revealed.surface_casters[0].opacity, opaque.surface_casters[0].opacity);
        assert_eq!(revealed.surface_casters[0].opacity, 1.0);
        let covered = |f: &Frame| f.albedo.iter().filter(|&&c| c != Rgb::BLACK).count();
        assert!(covered(&revealed) > 0 && covered(&revealed) < covered(&opaque));
    }

    #[test]
    fn pillar_moss_preserves_support_and_only_touches_the_actual_lowest_three_rows() {
        let s = sheet("bloomwild_ruins").unwrap();
        for a in s.anims.values().filter(|a| a.name.starts_with("pillar")) {
            let source = s.cell(a,0);
            let bottom = (0..source.h).rev().find(|&y| (0..source.w).any(|x| source.at(x,y).a)).unwrap();
            for variation in 0..3 {
                let cell = mossy_pillar(source,variation);
                for y in 0..source.h {
                    for x in 0..source.w {
                        let (a,b) = (source.at(x,y),cell.at(x,y));
                        assert_eq!((a.a,a.emit),(b.a,b.emit));
                        if y < bottom-2 { assert_eq!((a.c.r,a.c.g,a.c.b),(b.c.r,b.c.g,b.c.b)); }
                    }
                }
            }
        }
    }

    fn walk(zone: &Zone, start: (f32, f32), direction: (f32, f32), dash: bool) -> Player {
        let mut player = Player::new(start.0, start.1);
        let mut input = Input::default();
        let mut fx = Fx::new(7);
        for t in 0..90 {
            input.step(Raw { mv: direction, dash: dash && t == 0, ..Raw::default() });
            player.update(&input, &mut PlayerCtx { zone, fx: &mut fx, hits: &mut Vec::new() });
            fx.tick();
        }
        player
    }

    /// These are movement inputs through Player::update, not a metadata round-trip.
    /// Every native tree/ruin foot must stop Glint from all four directions and when mirrored.
    #[test]
    fn native_tree_and_ruin_supports_stop_real_player_from_all_sides() {
        let cases = [
            ("bloomwild_trees", "a_trunk", 12), ("bloomwild_trees", "b_trunk", 12),
            ("bloomwild_trees", "c_trunk", 12), ("bloomwild_trees", "d_trunk", 12),
            ("bloomwild_trees", "e_trunk", 8),
            ("bloomwild_trees", "f_trunk", 12), ("bloomwild_trees", "g_trunk", 12),
            ("bloomwild_ruins", "pillar", 10), ("bloomwild_ruins", "pillar_b", 10),
            ("bloomwild_ruins", "pillar_c", 10), ("bloomwild_ruins", "pillar_broken_tall", 10),
            ("bloomwild_ruins", "pillar_broken_short", 10), ("bloomwild_ruins", "drum", 8),
            ("bloomwild_ruins", "statue", 8), ("bloomwild_ruins", "wall_low", 8),
            ("bloomwild_ruins", "wall_tall", 8),
            ("bloomwild_arch", "arch_post_left", 8), ("bloomwild_arch", "arch_post_right", 8),
            ("bloomwild_aqueduct", "left_pier", 10), ("bloomwild_aqueduct", "right_pier", 10),
            ("bloomwild_props", "bush_a", 5), ("bloomwild_props", "bush_b", 5),
            ("bloomwild_props", "bush_bloom", 5), ("bloomwild_props", "urn_large", 5),
            ("bloomwild_props", "urn_small", 4), ("bloomwild_props", "boulder_large", 6),
            ("bloomwild_props", "boulder_small", 4),
            ("cinder_markers", "pillar", 8), ("cinder_markers", "pillar_broken_tall", 8),
            ("cinder_markers", "pillar_lean", 8), ("cinder_markers", "pillar_slotted", 8),
            ("cinder_foundry", "press", 10), ("cinder_foundry", "broken_press", 10),
            ("cinder_pillars", "pillar_broken_short", 8),
            ("cinder_props", "urn_tall", 4), ("cinder_props", "urn_squat", 4),
            ("cinder_props", "lava_rock", 4), ("cinder_props", "shard_small", 3),
        ];
        for (name, anim, depth) in cases {
            for flip in [false, true] {
                let prop = Prop::new(name, anim, 160.0, 160.0).flipped(flip);
                assert!(!prop.collision_rects.is_empty(), "missing native support: {name}/{anim}");
                let sheet = prop.sheet().unwrap();
                let a = sheet.anim(anim).unwrap();
                let cell = sheet.cell(a, 0);
                let (ax, ay) = sheet.anchor_of(a);
                let bottom = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).unwrap() + 1;
                let mut painted = Vec::new();
                for y in (bottom - depth).max(0)..bottom {
                    for x in 0..cell.w {
                        let wx = 160.0 + if flip { (ax - x) as f32 } else { (x - ax) as f32 };
                        let wy = 160.0 + (y - ay) as f32;
                        let expected = cell.at(x, y).a;
                        assert_eq!(prop.blocks(wx + 0.5, wy + 0.5, 0.01, 0.01), expected,
                            "support must match source pixels: {name}/{anim}, flip={flip}, source={x},{y}");
                        if expected { painted.push((wx, wy)); }
                    }
                }
                // Choose an actual support pixel near the center, even if the root center is a gap.
                let bounds = prop.collision_bounds().unwrap();
                let center = ((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5);
                let target = *painted.iter().min_by(|a, b| {
                    let dist = |v: &(f32, f32)| (v.0 - center.0).abs() + (v.1 - center.1).abs();
                    dist(a).total_cmp(&dist(b))
                }).unwrap();
                let mut zone = Zone::new(ZoneId::Bloomwild, 20, 20);
                zone.props.push(prop);
                for direction in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    let start = (target.0 - direction.0 * 70.0, target.1 - direction.1 * 40.0);
                    for dash in [false, true] {
                        let player = walk(&zone, start, direction, dash);
                        let (x, y, r) = (player.body.x, player.body.y, player.body.r);
                        assert!(!painted.iter().any(|&(px, py)| x + r > px && x - r < px + 1.0
                            && y + r * 0.6 > py && y - r * 0.6 < py + 1.0),
                            "player feet entered visible support: {name}/{anim}, flip={flip}, direction={direction:?}, dash={dash}, ended={x},{y}");
                        assert!((x - target.0) * direction.0 + (y - target.1) * direction.1 < 0.0,
                            "walked through {name}/{anim}");
                        assert!((x - start.0).abs() + (y - start.1).abs() > 5.0, "approach started blocked");
                    }
                }
            }
        }
    }

    #[test]
    fn old_tree_box_missed_root_and_blocked_empty_ground() {
        let p = Prop::new("bloomwild_trees", "a_trunk", 100.0, 100.0);
        // Native root x20,y88 lies twenty pixels left of the old centered 11x5 box.
        assert!(p.blocks(80.5, 98.5, 0.01, 0.01));
        // The old box covered x43,y89, where the selected native source is transparent.
        assert!(!p.blocks(103.5, 99.5, 0.01, 0.01));
        let pier = Prop::new("bloomwild_arch", "arch_post_right", 100.0, 100.0);
        assert_eq!(pier.sort_y(), 92.0, "shorter native pier sorts at its visible foot, not old gutter");
    }

    #[test]
    fn broken_column_feet_finish_inside_the_native_cell() {
        let s = sheet("bloomwild_ruins").unwrap();
        for name in ["pillar_broken_tall", "pillar_broken_short"] {
            let anim = s.anim(name).unwrap();
            let cell = s.cell(anim, 0);
            assert!((cell.h - 2..cell.h).all(|y| (0..cell.w).all(|x| !cell.at(x, y).a)),
                "{name}: a completed footing needs clear space below its silhouette; the old shaft hit row55");
            let bottom = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).unwrap();
            let p = Prop::new("bloomwild_ruins", name, 100.8, 100.2);
            assert_eq!(p.sort_y(), 100.0 + (bottom + 1 - s.anchor_of(anim).1) as f32);
        }
    }

    #[test]
    fn dynamic_decor_support_matches_native_pixels_and_honors_disable_and_death() {
        let s = sheet("bloomwild_trees").unwrap();
        let a = s.anim("a_trunk").unwrap();
        let mut p = Prop::new("glasswater_blocker", "none", 100.0, 100.0);
        p.set_native_support(s.cell(a, 0), s.anchor_of(a), 12);
        assert!(p.blocks(80.5, 98.5, 0.01, 0.01));
        p.flip = true;
        assert!(p.blocks(120.5, 98.5, 0.01, 0.01));
        assert!(!p.blocks(80.5, 98.5, 0.01, 0.01));
        p.alive = false;
        assert!(!p.blocks(120.5, 98.5, 0.01, 0.01));
        p.alive = true;
        p.footprint = None;
        assert!(!p.blocks(120.5, 98.5, 0.01, 0.01));
    }

    #[test]
    fn fractional_native_supports_match_the_rendered_pixels_in_both_flips() {
        let color = crate::gfx::rgb(0xc8d7c4);
        let cell = Cell::from_ascii(&[" aa ", "a  a", " aa "], &[('a', color)], &[]);
        for (x, y) in [(100.2, 100.8), (100.8, 100.2)] {
            for flip in [false, true] {
                let mut prop = Prop::new("glasswater_blocker", "none", x, y).flipped(flip);
                prop.set_native_support(&cell, (1, 2), 2);
                let mut frame = Frame::new();
                frame.begin(Rgb::BLACK, Rgb::WHITE);
                frame.sprite_w(&cell, (1, 2), x, y, &DrawOpts::flip(flip));
                for py in 99..103 {
                    for px in 95..108 {
                        assert_eq!(prop.blocks(px as f32 + 0.5, py as f32 + 0.5, 0.01, 0.01),
                            frame.albedo[py * crate::gfx::W + px] == color,
                            "fractional native collision diverged from raster at {px},{py}; anchor={x},{y}, flip={flip}");
                    }
                }
                assert_eq!(prop.sort_y(), 101.0);
            }
        }
    }
}

fn field<'a>(anim: Option<&'a Value>, sheet: &'a Value, key: &str) -> Option<&'a Value> {
    anim.and_then(|a| a.get(key)).or_else(|| sheet.get(key))
}

impl Prop {
    /// A prop from `sheet`/`anim` anchored at world (x, y). Unknown sheets still create a prop
    /// (it just won't draw), so builders don't crash while art is in progress.
    pub fn new(sheet_name: &'static str, anim: &str, x: f32, y: f32) -> Prop {
        let mut p = Prop {
            sheet: sheet_name,
            anim: anim.to_string(),
            x,
            y,
            flip: false,
            t0: ((x * 7.0 + y * 13.0) as i64).unsigned_abs() as u32 % 997,
            footprint: None,
            collision_rects: Vec::new(),
            collision_extent: None,
            breakable: false,
            hp: 1,
            alive: true,
            light: None,
            canopy: false,
            flash: 0.0,
            wobble: 0.0,
            heal_drop: 0,
        };
        if let Some(s) = sheet(sheet_name) {
            let a = s.anim(anim).map(|a| &a.json);
            if let Some(fp) = field(a, &s.json, "footprint").and_then(|v| v.as_array()) {
                let w = fp.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let h = fp.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                if w > 0.0 && h > 0.0 {
                    p.footprint = Some((w, h));
                }
            }
            if let Some(rects) = field(a, &s.json, "collision_rects").and_then(Value::as_array) {
                let (ax, ay) = s.anim(anim).map(|a| s.anchor_of(a)).unwrap_or(s.anchor);
                for rect in rects {
                    let Some(r) = rect.as_array().filter(|r| r.len() == 4) else { continue };
                    let values: Option<Vec<f32>> = r.iter().map(|v| v.as_f64().map(|v| v as f32)).collect();
                    let Some(v) = values else { continue };
                    if v.iter().all(|v| v.is_finite()) && v[2] > v[0] && v[3] > v[1] {
                        p.collision_rects.push([v[0] - ax as f32, v[1] - ay as f32,
                            v[2] - ax as f32, v[3] - ay as f32]);
                    }
                }
            }
            p.breakable = field(a, &s.json, "breakable").and_then(|v| v.as_bool()).unwrap_or(false);
            if let Some(l) = field(a, &s.json, "light") {
                let color = l.get("color").and_then(|c| c.as_str()).map(Rgb::hex).unwrap_or(Rgb::WHITE);
                let radius = l.get("radius").and_then(|v| v.as_f64()).unwrap_or(32.0) as f32;
                let intensity = l.get("intensity").and_then(|v| v.as_f64()).unwrap_or(0.8) as f32;
                let off = l.get("offset").and_then(|v| v.as_array());
                let ox = off.and_then(|o| o.first()).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let oy = off.and_then(|o| o.get(1)).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                p.light = Some((color, radius, intensity, ox, oy));
            }
            p.canopy = field(a, &s.json, "layer").and_then(|v| v.as_str()) == Some("canopy");
        }
        p.refresh_collision_extent();
        p
    }

    pub fn flipped(mut self, f: bool) -> Prop {
        self.flip = f;
        self
    }

    pub fn sheet(&self) -> Option<&'static Sheet> {
        sheet(self.sheet)
    }

    fn world_support(&self, r: &[f32; 4]) -> [f32; 4] {
        // Mirroring keeps the anchor pixel fixed, just like Frame::sprite. Native spans
        // contain whole pixels, so the final pixel at x1-1 becomes the first flipped one.
        let (x0, x1) = if self.flip { (1.0 - r[2], 1.0 - r[0]) } else { (r[0], r[2]) };
        // Frame::sprite_w floors its world anchor before rasterizing native pixels.
        let (x, y) = (self.x.floor(), self.y.floor());
        [x + x0, y + r[1], x + x1, y + r[3]]
    }

    pub fn collision_rects(&self) -> &[[f32; 4]] { &self.collision_rects }

    fn refresh_collision_extent(&mut self) {
        self.collision_extent = None;
        for v in &self.collision_rects {
            let bounds = self.collision_extent.get_or_insert(*v);
            bounds[0] = bounds[0].min(v[0]); bounds[1] = bounds[1].min(v[1]);
            bounds[2] = bounds[2].max(v[2]); bounds[3] = bounds[3].max(v[3]);
        }
    }

    /// Give code-drawn decor its actual source-cell support without drawing a second sprite.
    /// `depth` selects only the structural ground base, leaving the tall facade passable behind.
    pub fn set_native_support(&mut self, cell: &Cell, anchor: (i32, i32), depth: i32) {
        self.collision_rects.clear();
        let bottom = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).map_or(0, |y| y + 1);
        for y in (bottom - depth.max(0)).max(0)..bottom {
            let mut x = 0;
            while x < cell.w {
                if !cell.at(x, y).a { x += 1; continue; }
                let left = x;
                while x < cell.w && cell.at(x, y).a { x += 1; }
                self.collision_rects.push([(left - anchor.0) as f32, (y - anchor.1) as f32,
                    (x - anchor.0) as f32, (y + 1 - anchor.1) as f32]);
            }
        }
        self.refresh_collision_extent();
        self.footprint = self.collision_extent.map(|r| (r[2] - r[0], r[3] - r[1]));
    }

    /// Depth follows the visible foot, even when an old atlas anchor has a staging gutter.
    pub fn sort_y(&self) -> f32 {
        if self.collision_extent.is_some() { self.collision_bounds().map_or(self.y, |r| r[3]) } else { self.y }
    }

    /// World-space base bounds, also used to put contact shading under the actual support.
    pub fn collision_bounds(&self) -> Option<[f32; 4]> {
        let (w, h) = self.footprint?;
        Some(self.collision_extent.as_ref().map(|r| self.world_support(r))
            .unwrap_or([self.x - w * 0.5, self.y - h, self.x + w * 0.5, self.y]))
    }

    /// Intersect the real ground support, preserving gaps and off-center native roots.
    pub fn blocks(&self, x: f32, y: f32, hw: f32, hh: f32) -> bool {
        if !self.alive { return false; }
        let Some(bounds) = self.collision_bounds() else { return false };
        let overlaps = |r: [f32; 4]| x + hw > r[0] && x - hw < r[2] && y + hh > r[1] && y - hh < r[3];
        if !overlaps(bounds) { return false; }
        self.collision_rects.is_empty() || self.collision_rects.iter().any(|r| overlaps(self.world_support(r)))
    }

    /// Height of the sprite above its anchor (for hit tests and canopy fading).
    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let s = self.sheet()?;
        let a = s.anim(&self.anim)?;
        let (ax, ay) = s.anchor_of(a);
        let x0 = if self.flip { self.x - (s.cell_w - 1 - ax) as f32 } else { self.x - ax as f32 };
        Some((x0, self.y - ay as f32, s.cell_w as f32, s.cell_h as f32))
    }

    pub fn draw(&self, f: &mut Frame, t: u64, alpha: f32) {
        self.draw_revealed(f, t, alpha, None);
    }

    /// Open nearby context while preserving the rest of the scenery and its physical cover.
    pub fn draw_revealed(&self, f: &mut Frame, t: u64, alpha: f32, reveal: Option<crate::gfx::sprite::SpriteFocus>) {
        if !self.alive {
            return;
        }
        let Some(s) = self.sheet() else { return };
        let Some(a) = s.anim(&self.anim) else { return };
        let cell = s.cell(a, a.frame_at(t as u32 + self.t0, true));
        let cell = if self.sheet == "bloomwild_ruins" && self.anim.starts_with("pillar") {
            mossy_pillar(cell, ((self.x.floor() as i32 + self.y.floor() as i32).unsigned_abs() / 7) as usize)
        } else { cell };
        let wob = if self.wobble > 0.0 { ((self.wobble * 1.7).sin() * self.wobble * 0.25).round() } else { 0.0 };
        // Foreground reveal makes room for Glint without making its physical cover transparent.
        let floor_normals = self.sheet == "bloomwild_growth_low"
            || self.sheet == "bloomwild_props" && matches!(self.anim.as_str(),
                "bush_a" | "bush_b" | "bush_bloom" | "flowers_a" | "flowers_b");
        let o = DrawOpts { floor_normals, ground_y: self.canopy.then_some(self.y.floor()), flip: self.flip, flash: self.flash, alpha, shadow_alpha: (self.canopy || reveal.is_some()).then_some(1.0), reveal, reveal_depth: (!self.canopy).then_some(self.sort_y()), ..Default::default() };
        f.sprite_w(cell, s.anchor_of(a), self.x + wob, self.y, &o);
    }

    /// Damage a breakable prop. Returns true if it broke (and burst into pixels).
    pub fn hit(&mut self, dmg: i32, fx: &mut Fx, t: u64) -> bool {
        if !self.breakable || !self.alive {
            return false;
        }
        self.hp -= dmg;
        self.flash = 0.8;
        self.wobble = 8.0;
        if self.hp > 0 {
            return false;
        }
        self.alive = false;
        if let Some(s) = self.sheet()
            && let Some(a) = s.anim(&self.anim)
        {
            let cell = s.cell(a, a.frame_at(t as u32 + self.t0, true));
            let rng = &mut fx.rng;
            fx.particles.burst_cell(rng, cell, s.anchor_of(a), self.flip, self.x, self.y, 1.6, 420.0);
        }
        fx.shake(0.15);
        true
    }

    pub fn tick(&mut self) {
        self.flash = (self.flash - 0.12).max(0.0);
        self.wobble = (self.wobble - 1.0).max(0.0);
    }
}
