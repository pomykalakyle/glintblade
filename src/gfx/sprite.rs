//! Sprite sheets (see assets/SPRITE_FORMAT.md) and sprite drawing.
//!
//! All sheets in assets/sprites are loaded once, on first use, into a global registry:
//! `sheet("hero")` returns `Option<&'static Sheet>`.

use super::frame::{Frame, Rgb, rgb};
use crate::util::bayer4;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

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
}

impl Cell {
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
        Cell { w, h, px }
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
        let anchor = v.get("anchor").and_then(|a| Some((a.get(0)?.as_f64()? as i32, a.get(1)?.as_f64()? as i32))).unwrap_or((cw / 2, ch - 1));
        let emissive: HashSet<u32> =
            v.get("emissive").and_then(|e| e.as_array()).map(|a| a.iter().filter_map(|s| s.as_str()).map(|s| Rgb::hex(s).key()).collect()).unwrap_or_default();
        let rows = (h as i32 / ch.max(1)) as usize;
        let cols = (w as i32 / cw.max(1)) as usize;
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
                row.push(Cell { w: cw, h: ch, px });
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
    map
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

/// How to draw a sprite.
#[derive(Clone, Copy, Debug)]
pub struct DrawOpts {
    pub flip: bool,
    /// Stamp visible body coverage for lighting; scenery and afterimages leave this false.
    pub actor: bool,
    /// 0..1 material hit flash, preserving the silhouette edge and authored emission mask.
    pub flash: f32,
    /// Blend toward a color by an amount.
    pub tint: Option<(Rgb, f32)>,
    /// Dithered opacity 0..1.
    pub alpha: f32,
    /// Draw everything unlit (self-lit), e.g. for UI-like sprites or ghosts.
    pub unlit: bool,
    /// Multiplier for emissive pixels' brightness.
    pub emit_gain: f32,
    /// Draw every opaque pixel as one solid color (afterimages, silhouettes).
    pub silhouette: Option<Rgb>,
    /// If set with silhouette, the silhouette glows (emissive) instead of being lit.
    pub silhouette_glow: bool,
}

impl Default for DrawOpts {
    fn default() -> Self {
        DrawOpts { flip: false, actor: false, flash: 0.0, tint: None, alpha: 1.0, unlit: false, emit_gain: 1.0, silhouette: None, silhouette_glow: false }
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
                if o.alpha < 1.0 && o.alpha <= bayer4(px + self.cam_x, py + self.cam_y) {
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
