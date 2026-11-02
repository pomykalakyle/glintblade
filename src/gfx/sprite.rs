//! Sprite sheets (see assets/SPRITE_FORMAT.md) and sprite drawing.
//!
//! All sheets in assets/sprites are loaded once, on first use, into a global registry:
//! `sheet("hero")` returns `Option<&'static Sheet>`. Native builds may overlay a candidate
//! sprite directory with `GLINT_SPRITES=/path/to/sprites`; restart to load edited candidates.

use super::frame::{Frame, Rgb, rgb};
use crate::util::bayer4;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Default, Debug)]
pub struct Px {
    pub c: Rgb,
    pub a: bool,
    pub emit: bool,
}

/// One frame of pixels.
#[derive(Clone, Debug, Default)]
pub struct Cell {
    pub w: i32,
    pub h: i32,
    pub px: Vec<Px>,
    /// Linear signed vector data, on exactly the albedo grid.
    pub surface: Arc<[super::surface::NormalPixel]>,
}

impl Cell {
    pub fn new(w: i32, h: i32, px: Vec<Px>) -> Self {
        let surface = super::surface::fallback(w, h, &px);
        Self { w, h, px, surface }
    }
    /// Synchronize masks after splitting an atlas or changing emission, without changing normals.
    pub fn refresh_surface_mask(&mut self) {
        let data = Arc::make_mut(&mut self.surface);
        for (n, p) in data.iter_mut().zip(&self.px) { n.a = p.a; n.emit = p.emit; }
    }

    #[inline]
    pub fn at(&self, x: i32, y: i32) -> Px {
        if x < 0 || y < 0 || x >= self.w || y >= self.h { Px::default() } else { self.px[(y * self.w + x) as usize] }
    }
    pub fn is_empty(&self) -> bool {
        !self.px.iter().any(|p| p.a)
    }
    /// Flash material interiors while preserving the authored silhouette perimeter.
    #[inline]
    pub(crate) fn flash_color(&self, x: i32, y: i32, color: Rgb, flash: f32) -> Rgb {
        if flash <= 0.0 || [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
            .iter().any(|&(nx, ny)| !self.at(nx, ny).a)
        {
            color
        } else {
            color.lerp(rgb(0xedf9ef), flash.clamp(0.0, 1.0))
        }
    }
    /// Build a cell from rows of characters and a palette (char -> color). '.' and ' ' are transparent.
    /// Colors listed in `emissive` glow.
    pub fn from_ascii(rows: &[&str], pal: &[(char, Rgb)], emissive: &[char]) -> Cell {
        let h = rows.len() as i32;
        let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
        let mut px = vec![Px::default(); (w * h) as usize];
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if let Some((_, c)) = pal.iter().find(|(k, _)| *k == ch) {
                    px[y * w as usize + x] = Px { c: *c, a: true, emit: emissive.contains(&ch) };
                }
            }
        }
        Cell::new(w, h, px)
    }
}

#[derive(Clone, Debug)]
pub struct Anim {
    pub name: String,
    pub row: usize,
    pub frames: usize,
    pub fps: f32,
    /// Per-frame duration in 60 Hz ticks.
    pub durations: Vec<u32>,
    pub anchor: Option<(i32, i32)>,
    /// The anim's whole JSON object, for extra fields (footprint, light, scarf_anchor, ...).
    pub json: Value,
}

impl Anim {
    pub fn total_ticks(&self) -> u32 {
        self.durations.iter().sum::<u32>().max(1)
    }
    /// Frame index after `ticks` ticks of playback.
    pub fn frame_at(&self, ticks: u32, looping: bool) -> usize {
        let total = self.total_ticks();
        let mut t = if looping { ticks % total } else { ticks.min(total - 1) };
        for (i, d) in self.durations.iter().enumerate() {
            if t < *d {
                return i;
            }
            t -= d;
        }
        self.frames.saturating_sub(1)
    }
    pub fn finished(&self, ticks: u32) -> bool {
        ticks >= self.total_ticks()
    }
    /// Read an [x, y] pair (or per-frame list of pairs) from the anim JSON.
    pub fn point(&self, key: &str, frame: usize) -> Option<(i32, i32)> {
        let v = self.json.get(key)?;
        let pair = |p: &Value| -> Option<(i32, i32)> { Some((p.get(0)?.as_f64()? as i32, p.get(1)?.as_f64()? as i32)) };
        if let Some(arr) = v.as_array() {
            if arr.first().map(|f| f.is_array()).unwrap_or(false) {
                return arr.get(frame.min(arr.len().saturating_sub(1))).and_then(pair);
            }
            return pair(v);
        }
        None
    }
}

#[derive(Debug)]
pub struct Sheet {
    pub name: String,
    pub cell_w: i32,
    pub cell_h: i32,
    pub anchor: (i32, i32),
    /// cells[row][col]
    pub cells: Vec<Vec<Cell>>,
    pub anims: HashMap<String, Anim>,
    pub json: Value,
}

static EMPTY: OnceLock<Cell> = OnceLock::new();

impl Sheet {
    pub fn parse(name: &str, png: &[u8], json: &str) -> Option<Sheet> {
        let (w, h, rgba) = crate::assets::decode_png(png)?;
        let v: Value = serde_json::from_str(json).ok()?;
        let cell = v.get("cell")?;
        let cw = cell.get(0)?.as_i64()? as i32;
        let ch = cell.get(1)?.as_i64()? as i32;
        if cw <= 0 || ch <= 0 || w % cw as u32 != 0 || h % ch as u32 != 0 {
            return None;
        }
        let anchor = v.get("anchor").and_then(|a| Some((a.get(0)?.as_f64()? as i32, a.get(1)?.as_f64()? as i32))).unwrap_or((cw / 2, ch - 1));
        let emissive: HashSet<u32> =
            v.get("emissive").and_then(|e| e.as_array()).map(|a| a.iter().filter_map(|s| s.as_str()).map(|s| Rgb::hex(s).key()).collect()).unwrap_or_default();
        let rows = (h as i32 / ch.max(1)) as usize;
        let cols = (w as i32 / cw.max(1)) as usize;
        let normal = crate::assets::get(&format!("normals/{name}.png"))
            .and_then(crate::assets::decode_png)
            .filter(|(nw, nh, _)| *nw == w && *nh == h);
        let mut cells = Vec::with_capacity(rows);
        for r in 0..rows {
            let mut row = Vec::with_capacity(cols);
            for c in 0..cols {
                let mut px = vec![Px::default(); (cw * ch) as usize];
                for y in 0..ch {
                    for x in 0..cw {
                        let sx = c as i32 * cw + x;
                        let sy = r as i32 * ch + y;
                        let o = ((sy * w as i32 + sx) * 4) as usize;
                        if rgba[o + 3] >= 128 {
                            let col = Rgb::from_u8([rgba[o], rgba[o + 1], rgba[o + 2]]);
                            px[(y * cw + x) as usize] = Px { c: col, a: true, emit: emissive.contains(&col.key()) };
                        }
                    }
                }
                let mut frame = Cell::new(cw, ch, px);
                if let Some((_, _, ref normals)) = normal {
                    let mut data = Vec::with_capacity((cw * ch) as usize);
                    for y in 0..ch { for x in 0..cw {
                        let o = (((r as i32 * ch + y) * w as i32 + c as i32 * cw + x) * 4) as usize;
                        let p = frame.at(x, y);
                        data.push(super::surface::NormalPixel::decode([normals[o], normals[o+1], normals[o+2]], p.a, p.emit));
                    }}
                    frame.surface = data.into();
                }
                row.push(frame);
            }
            cells.push(row);
        }
        let mut anims = HashMap::new();
        if let Some(obj) = v.get("anims").and_then(|a| a.as_object()) {
            for (k, a) in obj {
                let row = a.get("row").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                let frames = a.get("frames").and_then(|x| x.as_u64()).unwrap_or(1).max(1) as usize;
                let fps = a.get("fps").and_then(|x| x.as_f64()).unwrap_or(8.0).max(0.1) as f32;
                let durations: Vec<u32> = match a.get("durations").and_then(|d| d.as_array()) {
                    Some(d) if !d.is_empty() => (0..frames).map(|i| d.get(i).and_then(|x| x.as_u64()).unwrap_or(6).max(1) as u32).collect(),
                    _ => vec![((60.0 / fps).round() as u32).max(1); frames],
                };
                let anchor = a.get("anchor").and_then(|p| Some((p.get(0)?.as_f64()? as i32, p.get(1)?.as_f64()? as i32)));
                anims.insert(k.clone(), Anim { name: k.clone(), row, frames, fps, durations, anchor, json: a.clone() });
            }
        }
        Some(Sheet { name: name.to_string(), cell_w: cw, cell_h: ch, anchor, cells, anims, json: v })
    }

    pub fn anim(&self, name: &str) -> Option<&Anim> {
        self.anims.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.anims.contains_key(name)
    }

    /// The cell for frame `i` of `anim` (an empty cell if anything is missing).
    pub fn cell(&self, anim: &Anim, i: usize) -> &Cell {
        self.cells.get(anim.row).and_then(|r| r.get(i.min(anim.frames.saturating_sub(1)))).unwrap_or_else(|| EMPTY.get_or_init(Cell::default))
    }

    pub fn anchor_of(&self, anim: &Anim) -> (i32, i32) {
        anim.anchor.unwrap_or(self.anchor)
    }

    /// Resolve a directional anim: tries `{base}_{dir}`; left uses `_right` mirrored; falls back to
    /// `{base}_right`, `{base}_down`, then `{base}`. Returns the anim and whether to flip.
    pub fn dir_anim(&self, base: &str, dir: Dir) -> Option<(&Anim, bool)> {
        // A dedicated `_left` anim (re-shaded for upper-left light) beats a mirrored `_right`.
        if dir == Dir::Left
            && let Some(a) = self.anims.get(&format!("{base}_left"))
        {
            return Some((a, false));
        }
        let (suffix, flip) = match dir {
            Dir::Down => ("down", false),
            Dir::Up => ("up", false),
            Dir::Right => ("right", false),
            Dir::Left => ("right", true),
        };
        if let Some(a) = self.anims.get(&format!("{base}_{suffix}")) {
            return Some((a, flip));
        }
        if let Some(a) = self.anims.get(&format!("{base}_right")) {
            return Some((a, dir == Dir::Left));
        }
        if let Some(a) = self.anims.get(&format!("{base}_down")) {
            return Some((a, false));
        }
        self.anims.get(base).map(|a| (a, dir == Dir::Left))
    }
}

/// Four-way facing used to pick directional anims.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
    Left,
    Right,
}

impl Dir {
    pub fn from_vec(x: f32, y: f32) -> Dir {
        if x.abs() > y.abs() * 1.05 {
            if x > 0.0 { Dir::Right } else { Dir::Left }
        } else if y < 0.0 {
            Dir::Up
        } else {
            Dir::Down
        }
    }
    pub fn vec(self) -> (f32, f32) {
        match self {
            Dir::Down => (0.0, 1.0),
            Dir::Up => (0.0, -1.0),
            Dir::Left => (-1.0, 0.0),
            Dir::Right => (1.0, 0.0),
        }
    }
}

static SHEETS: OnceLock<HashMap<String, Sheet>> = OnceLock::new();

fn load_all() -> HashMap<String, Sheet> {
    let mut map = HashMap::new();
    let pngs: Vec<&str> = crate::assets::list("sprites/").filter(|n| n.ends_with(".png")).collect();
    for p in pngs {
        let base = &p["sprites/".len()..p.len() - 4];
        let Some(json) = crate::assets::get_str(&format!("sprites/{base}.json")) else { continue };
        let png = crate::assets::get(p).unwrap();
        match Sheet::parse(base, png, json) {
            Some(s) => {
                map.insert(base.to_string(), s);
            }
            None => eprintln!("warning: could not parse sprite sheet {base}"),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dir) = std::env::var_os("GLINT_SPRITES").filter(|s| !s.is_empty()) {
        load_overrides(std::path::Path::new(&dir), &mut map);
    }
    prepare_aqueduct(&mut map);
    map
}

/// Separate a generated aqueduct into two grounded piers and the overhead arch on the same
/// native grid. This preserves the original source PNG while giving the game proper collision,
/// y sorting and canopy fading instead of treating the entire arch as one solid billboard.
fn prepare_aqueduct(map: &mut HashMap<String, Sheet>) {
    let Some(s) = map.get_mut("bloomwild_aqueduct") else { return };
    if (s.cell_w, s.cell_h) != (128, 96) { return; }
    let Some(body) = s.anim("body").cloned() else { return };
    let original = s.cell(&body, 0).clone();
    for (name, xmin, xmax, ymin, ymax, anchor, footprint) in [
        ("left_pier", 0, 64, 42, 96, (28, 83), Some((26, 10))),
        ("right_pier", 64, 128, 42, 96, (99, 83), Some((24, 10))),
        ("canopy", 0, 128, 0, 42, (64, 83), None),
    ] {
        let mut cell = original.clone();
        for y in 0..cell.h {
            for x in 0..cell.w {
                if x < xmin || x >= xmax || y < ymin || y >= ymax {
                    cell.px[(y * cell.w + x) as usize] = Px::default();
                }
            }
        }
        let mut anim = body.clone();
        anim.name = name.into();
        anim.row = s.cells.len();
        anim.frames = 1;
        anim.durations = vec![60];
        anim.anchor = Some(anchor);
        anim.json["footprint"] = match footprint {
            Some((w, h)) => serde_json::json!([w, h]),
            None => serde_json::json!([0, 0]),
        };
        if footprint.is_some() {
            // Pier feet are wider and at different heights in the native drawing. Keep
            // the overhead gap open, but stop Glint at each real opaque support pixel.
            let bottom = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).map_or(0, |y| y + 1);
            let mut rects = Vec::new();
            for y in (bottom - 10).max(0)..bottom {
                let mut x = 0;
                while x < cell.w {
                    if !cell.at(x, y).a { x += 1; continue; }
                    let left = x;
                    while x < cell.w && cell.at(x, y).a { x += 1; }
                    rects.push([left, y, x, y + 1]);
                }
            }
            anim.json["collision_rects"] = serde_json::json!(rects);
        }
        if name == "canopy" { anim.json["layer"] = serde_json::json!("canopy"); }
        cell.refresh_surface_mask();
        s.cells.push(vec![cell]);
        s.anims.insert(name.into(), anim);
    }
}

/// Try candidate atlases without changing embedded assets or rebuilding the game. A PNG-only
/// replacement keeps its embedded manifest (including anchors and collision footprints); new
/// sheets need both files. Invalid candidates leave the original sheet available.
#[cfg(not(target_arch = "wasm32"))]
fn load_overrides(dir: &std::path::Path, map: &mut HashMap<String, Sheet>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("warning: could not read sprite overrides {}: {e}", dir.display());
            return;
        }
    };
    let mut names = std::collections::BTreeSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if matches!(path.extension().and_then(|e| e.to_str()), Some("png" | "json"))
            && let Some(name) = path.file_stem().and_then(|s| s.to_str())
        {
            names.insert(name.to_owned());
        }
    }
    for name in names {
        let png_path = dir.join(format!("{name}.png"));
        let json_path = dir.join(format!("{name}.json"));
        let candidate = (|| -> Result<Sheet, String> {
            let png = if png_path.exists() {
                std::fs::read(&png_path).map_err(|e| e.to_string())?
            } else {
                crate::assets::get(&format!("sprites/{name}.png"))
                    .ok_or("new sheets need a PNG and JSON manifest")?.to_vec()
            };
            let json = if json_path.exists() {
                std::fs::read_to_string(&json_path).map_err(|e| e.to_string())?
            } else {
                map.get(&name).ok_or("new sheets need a PNG and JSON manifest")?.json.to_string()
            };
            let s = Sheet::parse(&name, &png, &json).ok_or("invalid PNG or manifest")?;
            // Keep silent cropping, empty cells and missing animation frames out of reviews.
            let (w, h, _) = crate::assets::decode_png(&png).ok_or("invalid PNG")?;
            if s.cell_w <= 0 || s.cell_h <= 0
                || w % s.cell_w as u32 != 0 || h % s.cell_h as u32 != 0
                || s.cells.is_empty() || s.cells[0].is_empty()
                || s.anims.values().any(|a| a.row >= s.cells.len() || a.frames > s.cells[0].len())
            {
                return Err("atlas dimensions do not match its cells and animation frames".into());
            }
            Ok(s)
        })();
        match candidate {
            Ok(sheet) => {
                eprintln!("sprite override: {name} ({}x{} cells)", sheet.cell_w, sheet.cell_h);
                map.insert(name, sheet);
            }
            Err(e) => eprintln!("warning: skipped sprite override {name}: {e}"),
        }
    }
}

/// Look up a loaded sprite sheet by base name (e.g. "hero", "bloomwild_props").
pub fn sheet(name: &str) -> Option<&'static Sheet> {
    SHEETS.get_or_init(load_all).get(name)
}

/// Names of all loaded sheets.
pub fn sheet_names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = SHEETS.get_or_init(load_all).keys().map(|s| s.as_str()).collect();
    v.sort();
    v
}

/// The current hero pose and ground position in world pixels. Foreground scenery opens
/// a stable patch of space around the body and its footing, including airborne poses.
#[derive(Clone, Copy, Debug)]
pub struct SpriteFocus {
    pub cell: &'static Cell,
    pub anchor: (i32, i32),
    pub flip: bool,
    pub x: f32,
    pub y: f32,
    pub ground_y: f32,
}

impl SpriteFocus {
    /// A clear center exposes the nearby floor; three short ordered-dither steps join
    /// it to intact scenery. Position owns the contour, so idle/sword frames cannot
    /// make it breathe. In the air a vertical capsule keeps the landing ground visible.
    pub fn scenery_alpha(&self, wx: i32, wy: i32) -> f32 {
        let dx = (wx - self.x.floor() as i32).abs();
        let body_center = self.y.floor() as i32 - 19;
        let ground_center = self.ground_y.floor() as i32 - 19;
        let top = body_center.min(ground_center);
        let bottom = body_center.max(ground_center);
        let dy = if wy < top { top - wy } else { (wy - bottom).max(0) };
        if dx >= 32 || dy >= 40 { return 1.0; }
        // Rounded shoulders clear slender upper branches and crystal tips along
        // with their stems. Integer fourth-power distances keep the contour on
        // the native grid, with the same restrained horizontal reach.
        let nx = dx * dx * 1024 / (32 * 32);
        let ny = dy * dy * 1024 / (40 * 40);
        let distance = nx * nx + ny * ny;
        const EDGE: i32 = 1024 * 1024;
        if distance < EDGE * 49 / 100 { 0.0 }
        else if distance < EDGE * 67 / 100 { 0.25 }
        else if distance < EDGE * 85 / 100 { 0.5 }
        else if distance < EDGE { 0.75 }
        else { 1.0 }
    }

    /// The whole context opening, including its stepped transition.
    pub fn covers(&self, wx: i32, wy: i32) -> bool {
        self.scenery_alpha(wx, wy) < 1.0
    }

    /// Procedural scenery shares exactly the native sprite's world-space dither.
    pub fn allows_scenery(&self, wx: i32, wy: i32) -> bool {
        self.scenery_alpha(wx, wy) > bayer4(wx, wy)
    }

    /// Restore a prop over twelve ground pixels after passing its foot, rather
    /// than switching the entire opening at the sort boundary. Distant scenery
    /// behind Glint stays solid; overhead canopies omit this depth fade.
    pub fn scenery_alpha_at_depth(&self, wx: i32, wy: i32, sort_y: f32) -> f32 {
        let passed = ((self.ground_y - sort_y) / 12.0).clamp(0.0, 1.0);
        let restored = passed * passed * (3.0 - 2.0 * passed);
        let alpha = self.scenery_alpha(wx, wy);
        alpha + (1.0 - alpha) * restored
    }

    pub fn allows_scenery_at_depth(&self, wx: i32, wy: i32, sort_y: f32) -> bool {
        self.scenery_alpha_at_depth(wx, wy, sort_y) > bayer4(wx, wy)
    }
}

/// How to draw a sprite.
#[derive(Clone, Copy, Debug)]
pub struct DrawOpts {
    pub flip: bool,
    /// Stamp visible body coverage for lighting; scenery and afterimages leave this false.
    pub actor: bool,
    /// True airborne offset, separate from the image anchor.
    pub elevation: f32,
    /// Explicit ground plane for split canopies or attached parts.
    pub ground_y: Option<f32>,
    /// Low foliage uses the top-facing native normal frame rather than an upright facade.
    pub floor_normals: bool,
    /// Native sole selection stays within the actual body, excluding weapons.
    pub foot_half_w: f32,
    pub foot_depth: i32,
    pub foot_full_width: bool,
    /// 0..1 material hit flash, preserving the silhouette edge and authored emission mask.
    pub flash: f32,
    /// Blend toward a color by an amount.
    pub tint: Option<(Rgb, f32)>,
    /// Dithered opacity 0..1.
    pub alpha: f32,
    /// Physical shadow opacity; override only presentation fades such as foreground reveal.
    /// None keeps genuinely translucent material coupled to its visual alpha.
    pub shadow_alpha: Option<f32>,
    /// Draw everything unlit (self-lit), e.g. for UI-like sprites or ghosts.
    pub unlit: bool,
    /// Multiplier for emissive pixels' brightness.
    pub emit_gain: f32,
    /// Draw every opaque pixel as one solid color (afterimages, silhouettes).
    pub silhouette: Option<Rgb>,
    /// If set with silhouette, the silhouette glows (emissive) instead of being lit.
    pub silhouette_glow: bool,
    /// Reveal space around this hero pose and its footing through foreground scenery.
    pub reveal: Option<SpriteFocus>,
    /// Sort depth of ordinary scenery; split overhead crowns/spans leave it unset.
    pub reveal_depth: Option<f32>,
}

impl Default for DrawOpts {
    fn default() -> Self {
        DrawOpts { flip: false, actor: false, elevation: 0.0, ground_y: None, floor_normals: false, foot_half_w: 6.0, foot_depth: 5, foot_full_width: false, flash: 0.0, tint: None, alpha: 1.0, shadow_alpha: None, unlit: false, emit_gain: 1.0, silhouette: None, silhouette_glow: false, reveal: None, reveal_depth: None }
    }
}

impl DrawOpts {
    pub fn flip(flip: bool) -> DrawOpts {
        DrawOpts { flip, ..Default::default() }
    }
}

impl Frame {
    /// Draw a cell so that its `anchor` pixel lands on screen position (sx, sy).
    pub fn sprite(&mut self, cell: &Cell, anchor: (i32, i32), sx: i32, sy: i32, o: &DrawOpts) {
        if self.water_any {
            self.reflect_sprite(cell, anchor, sx, sy, o);
        }
        let ordinary_base = super::surface::base(cell, anchor, sy + self.cam_y, 0.0);
        // A changing weapon/tail must never move an actor's true ground plane.
        let base = o.ground_y.unwrap_or_else(|| if o.actor {
            (sy + self.cam_y) as f32 + o.elevation
        } else { ordinary_base + o.elevation });
        let owner = if self.surface_lighting && !o.unlit && o.silhouette.is_none() {
            super::surface::register(self, cell, anchor, sx, sy, o.flip, base - ordinary_base, o.shadow_alpha.unwrap_or(o.alpha))
        } else { 0 };
        if owner!=0 && o.actor {
            let caster=self.surface_casters.last_mut().unwrap();
            caster.foot=super::surface::foot_profile_with_depth(cell,anchor,sy+self.cam_y,o.elevation,if o.foot_full_width {None}else{Some(o.foot_half_w)},o.foot_depth);
            caster.min_height=o.elevation.max(0.0);
        }
        let ox = sx - if o.flip { cell.w - 1 - anchor.0 } else { anchor.0 };
        let oy = sy - anchor.1;
        if ox > super::frame::WI || oy > super::frame::HI || ox + cell.w < 0 || oy + cell.h < 0 {
            return;
        }
        for y in 0..cell.h {
            let py = oy + y;
            if !(0..super::frame::HI).contains(&py) {
                continue;
            }
            for x in 0..cell.w {
                let p = cell.px[(y * cell.w + x) as usize];
                if !p.a {
                    continue;
                }
                let px = ox + if o.flip { cell.w - 1 - x } else { x };
                if !(0..super::frame::WI).contains(&px) {
                    continue;
                }
                let alpha = o.reveal.map_or(o.alpha, |focus| o.alpha.min(o.reveal_depth.map_or_else(
                    || focus.scenery_alpha(px + self.cam_x, py + self.cam_y),
                    |depth| focus.scenery_alpha_at_depth(px + self.cam_x, py + self.cam_y, depth))));
                if alpha < 1.0 && alpha <= bayer4(px + self.cam_x, py + self.cam_y) {
                    continue;
                }
                self.emissive[py as usize * super::frame::W + px as usize] = Rgb::BLACK;
                if let Some(s) = o.silhouette {
                    if o.silhouette_glow {
                        self.put_glow(px, py, s)
                    } else {
                        self.put(px, py, s)
                    }
                    continue;
                }
                let mut c = p.c;
                if let Some((t, k)) = o.tint {
                    c = c.lerp(t, k);
                }
                c = cell.flash_color(x, y, c, o.flash);
                if p.emit {
                    self.put_glow(px, py, c.scale(o.emit_gain));
                } else if o.unlit {
                    self.put_glow(px, py, c);
                } else {
                    self.put(px, py, c);
                }
                if o.actor { self.mark_actor(px, py); }
                if self.surface_lighting && !o.unlit && !p.emit {
                    super::surface::stamp(self, cell, x, y, px, py, owner, o.flip, base, o.actor);
                    if o.floor_normals {
                        let mut normal = cell.surface[(y * cell.w + x) as usize].floor();
                        if o.flip { normal[0] = -normal[0]; }
                        self.surfaces[py as usize * super::frame::W + px as usize].normal = normal;
                    }
                }
            }
        }
    }

    /// Draw a cell anchored at a world position.
    pub fn sprite_w(&mut self, cell: &Cell, anchor: (i32, i32), wx: f32, wy: f32, o: &DrawOpts) {
        let (sx, sy) = (self.sx(wx), self.sy(wy));
        self.sprite(cell, anchor, sx, sy, o);
    }

    /// Draw frame `ticks` of a named anim from a sheet at a world position (looping).
    /// Returns false if the sheet or anim doesn't exist.
    pub fn anim_w(&mut self, sheet: &str, anim: &str, ticks: u32, wx: f32, wy: f32, o: &DrawOpts) -> bool {
        let Some(s) = self::sheet(sheet) else { return false };
        let Some(a) = s.anim(anim) else { return false };
        let cell = s.cell(a, a.frame_at(ticks, true));
        self.sprite_w(cell, s.anchor_of(a), wx, wy, o);
        true
    }
}

#[cfg(test)]
mod shadow_opacity_tests {
    use super::*;
    use crate::gfx::frame::{PointLight, Surface};
    use crate::gfx::W;

    /// A visibility aid retains the same dithered ink as translucent material,
    /// but only actual translucency lets the source through its cast shadow.
    #[test]
    fn presentation_alpha_preserves_physical_shadow_without_changing_dither() {
        let cell = Cell::from_ascii(&["aaaaaaaa"; 8], &[('a', rgb(0x708080))], &[]);
        let draw = |alpha, shadow_alpha, silhouette| {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::WHITE);
            f.surface_lighting = true;
            f.surfaces.fill(Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind: 1 });
            f.sprite(&cell, (3, 7), 100, 100,
                &DrawOpts { alpha, shadow_alpha, silhouette, ..Default::default() });
            f
        };
        let opaque = draw(1.0, None, None);
        let translucent = draw(0.35, None, None);
        let revealed = draw(0.35, Some(1.0), None);
        assert_eq!(revealed.albedo, translucent.albedo, "physical opacity cannot alter the visible dither");
        let visible = revealed.albedo.iter().filter(|&&c| c != Rgb::BLACK).count();
        assert!(visible > 0 && visible < 64, "the presentation fade must still reveal the ground");
        let light = PointLight { x: 86.5, y: 88.5, height: 12.0, radius: 60.0, color: Rgb::WHITE, intensity: 1.0 };
        let probe = 100 * W + 112;
        let visibility = |frame: &Frame| crate::gfx::lighting::point_visibility(frame, light)[probe];
        assert_eq!(visibility(&opaque), 0, "probe must lie in the native volume's cast shadow");
        assert!(visibility(&translucent) > 0, "genuinely translucent material must retain its partial cast");
        assert_eq!(visibility(&revealed), visibility(&opaque), "a foreground reveal must not brighten the cast shadow");
        assert!(draw(0.35, Some(1.0), Some(Rgb::WHITE)).surface_casters.is_empty(),
            "a physical-opacity override must not turn an afterimage into a caster");
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod override_tests {
    use super::*;

    struct Candidate(std::path::PathBuf);

    impl Candidate {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("glintblade-sprites-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Candidate {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn png_only_candidate_retains_anchors_and_collision_contract() {
        let dir = Candidate::new();
        let json = r#"{"cell":[2,2],"anchor":[1,1],"anims":{"idle":{"row":0,"frames":1,"footprint":[3,2]}}}"#;
        let path = dir.0.join("tree.png");
        crate::assets::write_png(&path, 2, 2, &[0xff; 16]).unwrap();
        let original = Sheet::parse("tree", &std::fs::read(&path).unwrap(), json).unwrap();
        let mut map = HashMap::from([("tree".to_owned(), original)]);
        crate::assets::write_png(&path, 2, 2, &[12, 34, 56, 255].repeat(4)).unwrap();
        load_overrides(&dir.0, &mut map);
        let s = &map["tree"];
        assert_eq!(s.anchor, (1, 1));
        assert_eq!(s.anim("idle").unwrap().json["footprint"], serde_json::json!([3, 2]));
        assert_eq!(s.cells[0][0].at(0, 0).c.key(), 0x0c2238);
    }

    #[test]
    fn incomplete_animation_candidate_keeps_original_sheet() {
        let dir = Candidate::new();
        let png = dir.0.join("tree.png");
        let json = r#"{"cell":[2,2],"anims":{"idle":{"row":0,"frames":1}}}"#;
        crate::assets::write_png(&png, 2, 2, &[0xff; 16]).unwrap();
        let original = Sheet::parse("tree", &std::fs::read(&png).unwrap(), json).unwrap();
        let mut map = HashMap::from([("tree".to_owned(), original)]);
        std::fs::write(dir.0.join("tree.json"), json.replace("\"frames\":1", "\"frames\":2")).unwrap();
        load_overrides(&dir.0, &mut map);
        assert_eq!(map["tree"].anim("idle").unwrap().frames, 1);
    }

    #[test]
    fn aqueduct_split_preserves_pixels_and_keeps_arch_open() {
        let dir = Candidate::new();
        let path = dir.0.join("arch.png");
        crate::assets::write_png(&path, 128, 96, &[0xff; 128 * 96 * 4]).unwrap();
        let json = r#"{"cell":[128,96],"anchor":[64,83],"anims":{"body":{"row":0,"frames":1}}}"#;
        let original = Sheet::parse("bloomwild_aqueduct", &std::fs::read(path).unwrap(), json).unwrap();
        let mut map = HashMap::from([("bloomwild_aqueduct".into(), original)]);
        prepare_aqueduct(&mut map);
        let s = &map["bloomwild_aqueduct"];
        let parts = ["left_pier", "right_pier", "canopy"];
        for y in 0..96 {
            for x in 0..128 {
                let coverage = parts.iter().filter(|name| s.cell(s.anim(name).unwrap(), 0).at(x, y).a).count();
                assert_eq!(coverage, 1, "every source pixel belongs to exactly one layer");
            }
        }
        let left = s.anim("left_pier").unwrap();
        let right = s.anim("right_pier").unwrap();
        assert_eq!(left.json["footprint"], serde_json::json!([26, 10]));
        assert_eq!(right.json["footprint"], serde_json::json!([24, 10]));
        // At world anchor335, the actual foot centers leave a44px passage between colliders.
        let left_edge = 335 - 64 + s.anchor_of(left).0 + 13;
        let right_edge = 335 - 64 + s.anchor_of(right).0 - 12;
        assert!(right_edge - left_edge >= 40);
        assert_eq!(s.anim("canopy").unwrap().json["layer"], "canopy");
    }
}
