//! Title screen, HUD, region banners, the Warden's bar, pause and victory/credits.
//!
//! Everything here is drawn into `frame.out` after world lighting and glow, 1:1 on the native grid.
//! UI colors stay exactly as authored, with no bloom over text or opaque panels.
//! Text uses the artist's `font_small`, `font_tiny`, and `font_display` glyph sheets,
//! with a built-in 3x5 fallback when a sheet is missing.
//! Fades step through a few even levels instead of blending, and motion is in whole pixels.
//!
//! State: `Hud` (health pips and the Warden's bar animate from changes, so the game updates it
//! every tick) and the title's intro clock (owned by the game). Everything else is a pure
//! function of time.

use crate::entities::player::{Player, State as PState};
use crate::fx::Fx;
use crate::gfx::{Cell, Dir, Frame, HI, Px, Rgb, WI, rgb, sheet};
use crate::play::Play;
use crate::util::{bayer4, ease_out_cubic, hash1};
use crate::world::ZoneId;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::OnceLock;

// ---------------------------------------------------------------- palette (palettes_v2.json "ui")
// Style v2: quiet bone text on slate. Cyan stays with Glint and the Prism; UI ornament is ash.

pub const INK: Rgb = rgb(0x252d3c);
const PANEL0: Rgb = rgb(0x303949);
const PANEL1: Rgb = rgb(0x46505b);
const BORDER0: Rgb = rgb(0x6c7a83);
const BORDER1: Rgb = rgb(0x98a49f);
const MUTED: Rgb = rgb(0x9ba79e);
const TEXT: Rgb = rgb(0xdce0ce);
const HP: [Rgb; 4] = [rgb(0x53424d), rgb(0x89636e), rgb(0xb98087), rgb(0xd8b0ab)];
/// The thin energy line under the health marks.
const ENERGY: [Rgb; 4] = [rgb(0x426774), rgb(0x6aa1ad), rgb(0xa1cbc9), rgb(0xd1e3d8)];
/// Glint's blade (hero "blade" ramp): his dash afterimages on the title.
const CYAN: [Rgb; 4] = [rgb(0x429aa9), rgb(0x63d7e0), rgb(0xbaf5ec), rgb(0xedf9ef)];
/// The Prism (fx "prism" ramp): shards, the restored Prism and the Warden's heart.
const PRISM_RAMP: [Rgb; 4] = [rgb(0x564d6e), rgb(0x8d7fa8), rgb(0xb7add0), rgb(0xe2d8e9)];
/// Deep iris outline shared with the shard sprites.
const PRISM_INK: Rgb = rgb(0x38324a);
/// Glint's scarf (hero "scarf" ramp; the lightest step doubles as the lit edge).
const SCARF: [Rgb; 4] = [rgb(0x416974), rgb(0x416974), rgb(0x79a6ae), rgb(0xb4d1ce)];

/// `c` stepped in from ink: `a` 0..1 snaps to quarters. None when invisible.
fn step_in(c: Rgb, a: f32) -> Option<Rgb> {
    let k = (a.clamp(0.0, 1.0) * 4.0).round() / 4.0;
    if k <= 0.0 { None } else { Some(INK.lerp(c, k)) }
}

/// 0..1 over `len` ticks starting at `start`.
fn ramp(t: f32, start: f32, len: f32) -> f32 {
    ((t - start) / len).clamp(0.0, 1.0)
}

/// Darken the whole screen toward ink by `a`.
fn dim(f: &mut Frame, a: f32) {
    if a <= 0.0 {
        return;
    }
    let a = (a.clamp(0.0, 1.0) * 8.0).round() / 8.0;
    let ink = INK.to_u8();
    for p in f.out.iter_mut() {
        for (c, k) in p.iter_mut().zip(ink) {
            *c = (*c as f32 * (1.0 - a) + k as f32 * a) as u8;
        }
    }
}

// ---------------------------------------------------------------- fonts

struct Glyph {
    w: i32,
    /// One bitmask per row of the cell (bit x = column x).
    rows: Vec<u64>,
}

/// A pixel font read from a glyph sheet: `glyphs` maps each char to [x, y, w] in the PNG
/// (white ink, `cell_h` rows tall), and `font` holds the metrics.
pub struct Font {
    pub cell_h: i32,
    pub baseline: i32,
    pub cap_h: i32,
    pub tracking: i32,
    pub line_h: i32,
    glyphs: HashMap<char, Glyph>,
    kerning: HashMap<(char, char), i32>,
    caps_only: bool,
}

impl Font {
    /// Read glyph masks and spacing from the embedded artist-authored font manifest.
    fn from_sheet(name: &str) -> Option<Font> {
        let s = sheet(name)?;
        let meta = s.json.get("font")?;
        let num = |k: &str, d: i32| meta.get(k).and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(d);
        let cell_h = num("cell_h", s.cell_h);
        let mut glyphs = HashMap::new();
        for (k, v) in s.json.get("glyphs")?.as_object()? {
            let mut it = k.chars();
            let (Some(ch), None) = (it.next(), it.next()) else { continue };
            let Some(a) = v.as_array() else { continue };
            let get = |i: usize| a.get(i).and_then(|x| x.as_i64()).map(|x| x as i32);
            let (Some(x), Some(y), Some(w)) = (get(0), get(1), get(2)) else { continue };
            let (col, row) = ((x / s.cell_w) as usize, (y / s.cell_h) as usize);
            let Some(cell) = s.cells.get(row).and_then(|r| r.get(col)) else { continue };
            let (ox, oy) = (x % s.cell_w, y % s.cell_h);
            let rows = (0..cell_h).map(|gy| (0..w).fold(0u64, |m, gx| if cell.at(ox + gx, oy + gy).a { m | 1 << gx } else { m })).collect();
            glyphs.insert(ch, Glyph { w, rows });
        }
        if glyphs.is_empty() {
            return None;
        }
        let kerning = meta
            .get("kerning")
            .and_then(|k| k.as_object())
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| {
                        let mut c = k.chars();
                        Some(((c.next()?, c.next()?), v.as_i64()? as i32))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(Font {
            cell_h,
            baseline: num("baseline", cell_h - 1),
            cap_h: num("cap_height", cell_h),
            tracking: num("tracking", 1),
            line_h: num("line_height", cell_h + 2),
            glyphs,
            kerning,
            caps_only: false,
        })
    }

    /// The built-in 3x5 font, read back from `gfx::font` so the API stays the same.
    fn builtin() -> Font {
        let mut fr = Frame::new();
        let mut glyphs = HashMap::new();
        for code in 32u8..127 {
            let ch = code as char;
            for y in 0..6 {
                for x in 0..4 {
                    fr.out[y * WI as usize + x] = [0, 0, 0, 255];
                }
            }
            crate::gfx::font::text(&mut fr, 0, 0, &ch.to_string(), Rgb::WHITE, 1, 0.0);
            let rows = (0..5).map(|y| (0..3).fold(0u64, |m, x| if fr.out[y * WI as usize + x][0] > 128 { m | 1 << x } else { m })).collect();
            glyphs.insert(ch, Glyph { w: 3, rows });
        }
        Font { cell_h: 5, baseline: 4, cap_h: 5, tracking: 1, line_h: 8, glyphs, kerning: HashMap::new(), caps_only: true }
    }

    /// Resolve a glyph, using capitals when the font has no lowercase form.
    fn glyph(&self, c: char) -> Option<&Glyph> {
        if self.caps_only {
            return self.glyphs.get(&c.to_ascii_uppercase());
        }
        self.glyphs.get(&c).or_else(|| self.glyphs.get(&c.to_ascii_uppercase()))
    }

    /// Each char with its x offset.
    fn layout(&self, s: &str, extra: i32) -> Vec<(char, i32)> {
        let chars: Vec<char> = s.chars().collect();
        let mut out = Vec::with_capacity(chars.len());
        let mut x = 0;
        for (i, &c) in chars.iter().enumerate() {
            out.push((c, x));
            x += self.glyph(c).map(|g| g.w).unwrap_or(3) + self.tracking + extra;
            if let Some(&n) = chars.get(i + 1) {
                x += self.kerning.get(&(c, n)).copied().unwrap_or(0);
            }
        }
        out
    }

    /// Measure the exact pixel advance of a line, without trailing tracking.
    pub fn width(&self, s: &str, extra: i32) -> i32 {
        self.layout(s, extra).last().map(|&(c, x)| x + self.glyph(c).map(|g| g.w).unwrap_or(3)).unwrap_or(0)
    }

    /// Rows above the cap line inside a cell (cap top = cell top + this).
    pub fn cap_top(&self) -> i32 {
        (self.baseline - self.cap_h + 1).max(0)
    }

    /// Tint and draw a glyph mask at native resolution.
    fn blit(&self, f: &mut Frame, c: char, x: i32, y: i32, col: Rgb) {
        let Some(g) = self.glyph(c) else { return };
        for (gy, bits) in g.rows.iter().enumerate() {
            if *bits == 0 {
                continue;
            }
            for gx in 0..g.w {
                if bits >> gx & 1 == 1 {
                    f.ui_put(x + gx, y + gy as i32, col);
                }
            }
        }
    }
}

pub struct Fonts {
    pub small: Font,
    pub tiny: Font,
    pub display: Font,
}

/// Load all three authored fonts once, retaining a readable fallback.
pub fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| Fonts {
        small: Font::from_sheet("font_small").unwrap_or_else(Font::builtin),
        tiny: Font::from_sheet("font_tiny").unwrap_or_else(Font::builtin),
        display: Font::from_sheet("font_display").unwrap_or_else(Font::builtin),
    })
}

/// How to ink a line of text.
#[derive(Clone, Copy)]
pub struct Ink {
    pub color: Rgb,
    /// 1px drop shadow (down-right).
    pub shadow: Option<Rgb>,
    /// 1px outline all around.
    pub outline: Option<Rgb>,
    /// Extra letter spacing.
    pub spacing: i32,
}

impl Ink {
    /// Construct a text style with a one-pixel ink shadow.
    pub const fn new(color: Rgb) -> Ink {
        Ink { color, shadow: Some(INK), outline: None, spacing: 0 }
    }
    /// Add a one-pixel outline to a text style.
    const fn outline(mut self, c: Rgb) -> Ink {
        self.outline = Some(c);
        self
    }
    /// Set additional tracking in native pixels.
    const fn spacing(mut self, s: i32) -> Ink {
        self.spacing = s;
        self
    }
    /// Disable the text shadow on solid panel backgrounds.
    const fn no_shadow(mut self) -> Ink {
        self.shadow = None;
        self
    }
}

const AROUND: [(i32, i32); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];

/// Draw `s` with its cell top-left at (x, y). `color_at(i)` can recolor char i (None = skip it).
fn text_with(f: &mut Frame, font: &Font, x: i32, y: i32, s: &str, ink: &Ink, mut color_at: impl FnMut(usize, char) -> Option<(Rgb, i32)>) {
    let lay = font.layout(s, ink.spacing);
    let cols: Vec<Option<(Rgb, i32)>> = lay.iter().enumerate().map(|(i, &(c, _))| color_at(i, c)).collect();
    if let Some(o) = ink.outline {
        for (i, &(c, cx)) in lay.iter().enumerate() {
            if let Some((_, dy)) = cols[i] {
                for (dx, dy2) in AROUND {
                    font.blit(f, c, x + cx + dx, y + dy + dy2, o);
                }
            }
        }
    }
    if let Some(sh) = ink.shadow {
        for (i, &(c, cx)) in lay.iter().enumerate() {
            if let Some((_, dy)) = cols[i] {
                font.blit(f, c, x + cx + 1, y + dy + 1, sh);
            }
        }
    }
    for (i, &(c, cx)) in lay.iter().enumerate() {
        if let Some((col, dy)) = cols[i] {
            font.blit(f, c, x + cx, y + dy, col);
        }
    }
}

/// Draw one native-resolution line using the chosen font and ink.
pub fn text(f: &mut Frame, font: &Font, x: i32, y: i32, s: &str, ink: &Ink) {
    let c = ink.color;
    text_with(f, font, x, y, s, ink, |_, _| Some((c, 0)));
}

/// Centered on `cx`.
pub fn text_c(f: &mut Frame, font: &Font, cx: i32, y: i32, s: &str, ink: &Ink) {
    text(f, font, cx - font.width(s, ink.spacing) / 2, y, s, ink);
}

/// Text stepped in from ink (`a` 0..1); the shadow only shows once the text is half in.
fn text_fade(f: &mut Frame, font: &Font, cx: i32, y: i32, s: &str, ink: &Ink, a: f32) {
    let Some(c) = step_in(ink.color, a) else { return };
    let mut k = *ink;
    k.color = c;
    if a < 0.5 {
        k.shadow = None;
        k.outline = None;
    }
    text_c(f, font, cx, y, s, &k);
}

/// "Bloomwild Ruins" from "BLOOMWILD RUINS" (the display font sets lowercase as small caps).
fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut start = true;
    for c in s.chars() {
        if start {
            out.extend(c.to_uppercase())
        } else {
            out.extend(c.to_lowercase())
        }
        start = c == ' ' || c == '-';
    }
    out
}

// ---------------------------------------------------------------- small drawing helpers

/// Draw a sprite cell into the UI layer with its top-left at (x, y). `paint` maps each opaque
/// pixel (cell x, cell y, pixel) to an opaque color, or None to skip it.
fn ui_cell(f: &mut Frame, cell: &Cell, x: i32, y: i32, flip: bool, mut paint: impl FnMut(i32, i32, Px) -> Option<Rgb>) {
    for cy in 0..cell.h {
        for cx in 0..cell.w {
            let p = cell.at(cx, cy);
            if !p.a {
                continue;
            }
            let Some(c) = paint(cx, cy, p) else { continue };
            let px = x + if flip { cell.w - 1 - cx } else { cx };
            f.ui_put(px, y + cy, c);
        }
    }
}

/// A small diamond (the prism motif) centered on (x, y), radius r.
fn diamond(f: &mut Frame, x: i32, y: i32, r: i32, fill: Rgb, rim: Option<Rgb>) {
    for dy in -r..=r {
        let w = r - dy.abs();
        for dx in -w..=w {
            let edge = dx.abs() == w;
            let c = match rim {
                Some(rc) if edge => rc,
                _ => fill,
            };
            f.ui_put(x + dx, y + dy, c);
        }
    }
}

/// Width of an authored button prompt, including its transparent-free drawn bounds.
fn key_width(label: &str) -> i32 {
    let name = format!("key_{}", label.to_ascii_lowercase());
    sheet("hud_prompts").and_then(|s| s.json.get("sizes")?.get(&name)?.get(0)?.as_i64()).map(|w| w as i32).unwrap_or_else(|| fonts().tiny.width(label, 0) + 6)
}

/// Draw a manifest-sized prompt, with its actual top-left at (x, y).
fn prompt(f: &mut Frame, name: &str, x: i32, y: i32, a: f32) -> i32 {
    let Some(s) = sheet("hud_prompts") else { return 0 };
    let Some(anim) = s.anim(name) else { return 0 };
    let Some(size) = s.json.get("sizes").and_then(|v| v.get(name)) else { return 0 };
    let w = size[0].as_i64().unwrap_or(11) as i32;
    let h = size[1].as_i64().unwrap_or(11) as i32;
    ui_cell(f, s.cell(anim, 0), x, y + h - s.cell_h, false, |_, _, p| step_in(p.c, a));
    w
}

/// Draw one of the artist's keyboard prompts, with a font fallback for unknown keys.
fn keycap(f: &mut Frame, x: i32, y: i32, label: &str, a: f32) -> i32 {
    let name = format!("key_{}", label.to_ascii_lowercase());
    let w = prompt(f, &name, x, y, a);
    if w > 0 {
        return w;
    }
    let ft = &fonts().tiny;
    let w = key_width(label);
    if let Some(c) = step_in(PANEL1, a) {
        f.ui_rect(x, y, w, 11, c, 1.0);
    }
    if let Some(c) = step_in(TEXT, a) {
        text(f, ft, x + 3, y + 3 - ft.cap_top(), label, &Ink::new(c).no_shadow());
    }
    w
}

/// Stretch the artist's nine-slice panel without scaling any corner pixels.
fn panel(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, opacity: f32) {
    let opacity = (opacity.clamp(0.0, 1.0) * 4.0).round() / 4.0;
    if opacity == 0.0 {
        return;
    }
    let Some(s) = sheet("hud_panel") else { return };
    let Some(a) = s.anim("frame") else { return };
    let cell = s.cell(a, 0);
    let slices = s.json.get("slice");
    let edge = |i| slices.and_then(|v| v.get(i)).and_then(|v| v.as_i64()).unwrap_or(4) as i32;
    let (l, t, r, b) = (edge(0), edge(1), edge(2), edge(3));
    if w < l + r + 1 || h < t + b + 1 {
        return;
    }
    for yy in 0..h {
        let sy = if yy < t {
            yy
        } else if yy >= h - b {
            cell.h - h + yy
        } else {
            t
        };
        for xx in 0..w {
            let sx = if xx < l {
                xx
            } else if xx >= w - r {
                cell.w - w + xx
            } else {
                l
            };
            let p = cell.at(sx, sy);
            if p.a {
                let (px, py) = (x + xx, y + yy);
                f.ui_blend(px, py, p.c, opacity);
            }
        }
    }
}

/// The single selected menu action is marked by the artist's gently moving crystal blade.
fn cursor(f: &mut Frame, x: i32, y: i32, t: u32, a: f32) {
    let Some(s) = sheet("hud_cursor") else { return };
    let Some(anim) = s.anim("idle") else { return };
    let (ax, ay) = s.anchor_of(anim);
    ui_cell(f, s.cell(anim, anim.frame_at(t, true)), x - ax, y - ay, false, |_, _, p| step_in(p.c, a));
}

/// A row of key caps and labels: [("J", "slash"), ...]. Returns its width (draws if `draw`).
fn key_row(f: &mut Frame, x: i32, y: i32, items: &[(&str, &str)], a: f32, label: Rgb, draw: bool) -> i32 {
    let ft = &fonts().tiny;
    let mut cx = x;
    for (i, (key, what)) in items.iter().enumerate() {
        if i > 0 {
            cx += 10;
        }
        for (j, k) in key.split(' ').enumerate() {
            if j > 0 {
                cx += 2;
            }
            cx += if draw { keycap(f, cx, y, k, a) } else { key_width(k) };
        }
        cx += 4;
        if draw && let Some(c) = step_in(label, a) {
            text(f, ft, cx, y + 2 - ft.cap_top(), what, &Ink::new(c));
        }
        cx += ft.width(what, 0);
    }
    cx - x
}

/// A quiet ash hairline from x0 to x1 at y, with an ink line under it once mostly faded in.
fn rule(f: &mut Frame, x0: i32, x1: i32, y: i32, a: f32) {
    let Some(c) = step_in(BORDER0, a) else { return };
    for x in x0.min(x1)..=x0.max(x1) {
        f.ui_put(x, y, c);
        if a >= 0.5 {
            f.ui_put(x, y + 1, INK);
        }
    }
}

// ---------------------------------------------------------------- title

/// The intro after pressing start: two typed lines, then Glint dashes into the meadow.
const INTRO_1: &str = "The Prism cracked. Typical.";
const INTRO_2: &str = "Go on, Glint. Mind the drop.";
const TYPE_RATE: u64 = 1;
const INTRO_L1: u64 = 10;

/// Start the second line once the first has landed.
fn intro_l2() -> u64 {
    INTRO_L1 + INTRO_1.chars().count() as u64 * TYPE_RATE + 10
}
/// Glint crouches, then dashes away at this intro tick.
pub fn intro_dash() -> u64 {
    intro_l2() + INTRO_2.chars().count() as u64 * TYPE_RATE + 20
}
/// The game starts fading into Bloomwild at this intro tick.
pub fn intro_end() -> u64 {
    intro_dash() + 22
}
/// Should a text blip sound at intro tick `i`?
pub fn intro_blip(i: u64) -> bool {
    for (start, s) in [(INTRO_L1, INTRO_1), (intro_l2(), INTRO_2)] {
        if i >= start && (i - start) % (TYPE_RATE * 2) == 0 {
            let n = ((i - start) / TYPE_RATE) as usize;
            if let Some(c) = s.chars().nth(n) {
                return c.is_alphanumeric();
            }
        }
    }
    false
}

/// Ticks of crouch before the dash.
const CROUCH: u64 = 12;

/// Glint's sprite cell for a pose (sheet when it exists, the player's fallback otherwise).
fn glint_cell(state: PState, ticks: u32, dir: Dir) -> Option<(&'static Cell, (i32, i32), bool, (i32, i32))> {
    let mut p = Player::new(0.0, 0.0);
    p.dir = dir;
    p.state = state;
    p.anim_t = ticks;
    let (cell, anchor, flip) = p.current_cell()?;
    let base = match state {
        PState::Dash { .. } => "dash",
        PState::Charge { .. } => "charge",
        _ => "idle",
    };
    let neck = sheet("hero")
        .and_then(|s| {
            let (a, fl) = s.dir_anim(base, dir)?;
            let (ax, ay) = s.anchor_of(a);
            let (px, py) = a.point("scarf_anchor", a.frame_at(ticks, true))?;
            Some((if fl { ax - px } else { px - ax }, py - ay))
        })
        .unwrap_or((0, -10));
    Some((cell, anchor, flip, neck))
}

/// A scarf streaming behind (to the left of) a right-facing Glint, rippling in the wind.
/// `stream` 0..1 pulls it straighter (the crouch before the dash, the dash itself).
fn draw_scarf(f: &mut Frame, nx: i32, ny: i32, t: u64, stream: f32) {
    let n = 15;
    let tt = t as f32;
    let gust = 0.6 + 0.4 * (tt * 0.021).sin();
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let i = i as f32;
            let k = i / (n - 1) as f32;
            let droop = (0.16 * i + 0.016 * i * i) * (1.0 - stream * 0.8) * (1.15 - gust * 0.3);
            let wave = (1.0 + 2.6 * gust) * k * (tt * 0.2 - i * 0.55).sin() * (1.0 - stream * 0.6);
            (nx as f32 - i * 1.55 * (1.0 - 0.1 * k), ny as f32 + droop + wave)
        })
        .collect();
    for i in 0..n - 1 {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[i + 1];
        let k = i as f32 / (n - 1) as f32;
        let wide = k < 0.62;
        // facing up-left catches the light; facing down falls into shade
        let rising = y1 < y0 - 0.35;
        let (top, bot) = if k > 0.86 {
            (SCARF[1], SCARF[0])
        } else if rising && k > 0.12 {
            (SCARF[3], SCARF[2])
        } else {
            (SCARF[2], SCARF[1])
        };
        Frame::line_with(x0.round() as i32, y0.round() as i32, x1.round() as i32, y1.round() as i32, |px, py| {
            f.ui_put(px, py, top);
            if wide {
                f.ui_put(px, py + 1, bot);
            }
        });
    }
}

/// Glint at the real meadow anchor, then crouching and dashing during the intro.
fn draw_glint(f: &mut Frame, t: u64, intro: Option<u64>, (gx, feet): (i32, i32)) {
    let dash_at = intro_dash();
    let i = intro.unwrap_or(0);
    if intro.is_some() && i >= dash_at {
        // Follow the headland's clear road; the tree and ruin stand behind it.
        let k = (i - dash_at) as f32;
        let pos = |k: f32| (gx as f32 + k * 9.0 + k * k * 0.05, feet as f32);
        for back in [9.0, 6.0, 3.0] {
            let kk = k - back;
            if kk < 0.0 {
                continue;
            }
            let life = 1.0 - back / 12.0;
            if let Some((cell, (ax, ay), flip, _)) = glint_cell(PState::Dash { t: 2 }, 2, Dir::Right) {
                let (x, y) = pos(kk);
                let (ox, oy) = (x as i32 - ax, y as i32 - ay);
                let col = CYAN[1].lerp(CYAN[2], life);
                // dithered like the in-game dash afterimages
                ui_cell(f, cell, ox, oy, flip, |cx, cy, _| (life * 0.8 > bayer4(ox + cx, oy + cy)).then_some(col));
            }
        }
        if let Some((cell, (ax, ay), flip, (nx, ny))) = glint_cell(PState::Dash { t: (k as u32).min(6) }, k as u32, Dir::Right) {
            let (x, y) = pos(k);
            draw_scarf(f, x as i32 + nx, y as i32 + ny, t, 1.0);
            ui_cell(f, cell, x as i32 - ax, y as i32 - ay, flip, |_, _, p| Some(p.c));
        }
        // takeoff dust
        if k < 20.0 {
            for d in 0..8 {
                let a = PI + 0.2 + hash1(d, 41) * 1.2;
                let sp = 0.5 + hash1(d, 43) * 0.8;
                let (dx, dy) = (a.cos() * sp * k, a.sin() * sp * k * 0.5 + 0.03 * k * k);
                let c = if k < 8.0 { rgb(0xc7cfca) } else { BORDER1 };
                if (k as i32 + d) % 5 != 0 {
                    f.ui_put(gx + dx as i32, feet - 1 + dy as i32, c);
                }
            }
        }
        return;
    }
    let crouch = intro.is_some() && i + CROUCH >= dash_at;
    let (state, ticks) = if crouch { (PState::Charge { t: 0 }, (i + CROUCH - dash_at) as u32) } else { (PState::Normal, t as u32) };
    if let Some((cell, (ax, ay), flip, (nx, ny))) = glint_cell(state, ticks, Dir::Right) {
        let dy = if crouch && sheet("hero").and_then(|s| s.dir_anim("charge", Dir::Right)).is_none() { 1 } else { 0 };
        draw_scarf(f, gx + nx, feet + ny + dy, t, if crouch { 0.6 } else { 0.0 });
        ui_cell(f, cell, gx - ax, feet - ay + dy, flip, |_, _, p| Some(p.c));
    }
}

/// A logo sheet ("logo" or "logo_small") with its letters' baseline centered on (cx, base_y).
/// The style v2 logo is static cut stone: no shine sweep. `a` steps the logo in from ink.
/// Returns false if the sheet doesn't exist.
fn draw_logo_sheet(f: &mut Frame, name: &str, cx: i32, base_y: i32, a: f32) -> bool {
    let Some(s) = sheet(name) else { return false };
    let Some(anim) = s.anim("idle").or_else(|| s.anims.values().next()) else { return false };
    let cell = s.cell(anim, 0);
    let (ax, ay) = s.anchor_of(anim);
    ui_cell(f, cell, cx - ax, base_y - ay, false, |_, _, p| step_in(p.c, a));
    true
}

/// The title logo, letters' baseline at `base_y`. Until the sheet exists: the display font.
fn draw_logo(f: &mut Frame, cx: i32, base_y: i32, a: f32) {
    // Recompose the existing native wordmark into two intact words. No resampling:
    // the quiet sky column belongs to the title, the actual island belongs to Glint.
    if let Some(s) = sheet("logo") && let Some(anim) = s.anim("idle") {
        let cell = s.cell(anim, 0);
        let split = 96.min(cell.w);
        let (_, ay) = s.anchor_of(anim);
        ui_cell(f, cell, cx - split / 2, base_y - ay, false, |x, _, p| {
            (x < split).then(|| step_in(p.c, a)).flatten()
        });
        ui_cell(f, cell, cx - (cell.w - split) / 2 + 16 - split, base_y + 36 - ay, false, |x, _, p| {
            (x >= split).then(|| step_in(p.c, a)).flatten()
        });
        return;
    }
    let d = &fonts().display;
    let ink = Ink::new(TEXT).outline(INK).spacing(3);
    let Some(col) = step_in(ink.color, a) else { return };
    for (line, y) in [("GLINT", base_y), ("BLADE", base_y + 26)] {
        text_c(f, d, cx, y - d.baseline, line, &Ink { color: col, ..ink });
    }
}

/// The title overlay. `t` = ticks on the title, `intro` = ticks since start was pressed.
pub fn title(f: &mut Frame, t: u64, intro: Option<u64>, hero: (i32, i32)) {
    let tf = t as f32;
    let i = intro.unwrap_or(0);
    let leaving = intro.map(|i| 1.0 - ramp(i as f32, 0.0, 16.0)).unwrap_or(1.0);
    dim(f, if intro.is_some() { 0.3 } else { 0.12 });

    // logo: steps in once and then holds still
    let logo_a = ramp(tf, 40.0, 24.0) * leaving;
    draw_logo(f, 84, 56, logo_a);

    // A steady, quiet start action.
    let pa = ramp(tf, 88.0, 20.0) * leaving;
    if pa > 0.0 {
        let sm = &fonts().small;
        let w = sm.width("Press", 0) + 5 + key_width("ENTER") + 8 + 10;
        let x = 100 - w / 2;
        if let Some(c) = step_in(TEXT, pa) {
            text(f, sm, x, 132, "Press", &Ink::new(c).outline(INK));
        }
        let x = x + sm.width("Press", 0) + 5;
        let kw = keycap(f, x, 130, "ENTER", pa);
        prompt(f, "pad_south", x + kw + 8, 130, pa);
    }

    draw_glint(f, t, intro, hero);

    // Existing bindings settle before the start prompt appears, including a prompt-time confirm.
    let ca = ramp(tf, 66.0, 20.0) * leaving;
    if ca > 0.0 {
        let (km, ks, kd) = crate::input::key_labels();
        let other = if crate::input::layout() == crate::input::KeyLayout::Qwerty { "colemak" } else { "qwerty" };
        // Keep each hint inside the quiet sky column; the headland starts at x152.
        // Group combat together so neither layout's slash label crosses the cliff.
        let rows: [(&[(&str, &str)], i32); 3] = [
            (&[(km, "move")], 166),
            (&[(ks, "slash"), (kd, "dash")], 183),
            (&[("TAB", other)], 200),
        ];
        for (items, y) in rows {
            let width = key_row(f, 0, y, items, ca, TEXT, false);
            key_row(f, 84 - width / 2, y, items, ca, TEXT, true);
        }
    }

    // the intro lines
    if intro.is_some() && i >= INTRO_L1 {
        let fade = 1.0 - ramp(i as f32, intro_end() as f32 - 10.0, 16.0);
        panel(f, 40, 153, 272, 43, fade);
        let sm = &fonts().small;
        for (start, line, y) in [(INTRO_L1, INTRO_1, 163), (intro_l2(), INTRO_2, 178)] {
            if i < start {
                continue;
            }
            let shown = ((i - start) / TYPE_RATE + 1) as usize;
            let w = sm.width(line, 0);
            let x = WI / 2 - w / 2;
            let glint_at = line.find("Glint").map(|b| line[..b].chars().count());
            text_with(f, sm, x, y - sm.cap_top(), line, &Ink::new(TEXT), |k, _| {
                if k >= shown {
                    return None;
                }
                let hi = glint_at.map(|g| k >= g && k < g + 5).unwrap_or(false);
                let base = if hi { ENERGY[2] } else { TEXT };
                // the newest letter lands a pixel high
                let dy = if k + 1 == shown { -1 } else { 0 };
                step_in(base, fade).map(|c| (c, dy))
            });
        }
    }
}

// ---------------------------------------------------------------- HUD

#[derive(Clone, Copy, Default)]
struct HeartChange {
    /// Tick the change shows (refills are staggered, so this can be in the future).
    at: u64,
    gained: bool,
    set: bool,
}

#[derive(Default)]
struct BossBar {
    on: bool,
    /// Ticks since it appeared.
    t: u32,
    shown: f32,
    trail: f32,
    hold: u32,
    /// Ticks since the last hit.
    hit_t: u32,
    /// Ticks since its health hit zero.
    dead: Option<u32>,
}

/// HUD animation state. `update` once per game tick while playing.
pub struct Hud {
    t: u64,
    init: bool,
    hp: i32,
    max: i32,
    hearts: Vec<HeartChange>,
    hurt_t: u64,
    shards: u32,
    /// Tick the shard count last went up.
    shard_t: Option<u64>,
    boss: BossBar,
}

impl Default for Hud {
    /// Start the HUD without pending health or shard animations.
    fn default() -> Self {
        Self::new()
    }
}

const HUD_X: i32 = 8;
const HUD_Y: i32 = 7;
const BAR_W: i32 = 168;
const BAR_FILL0: u32 = 26;
const BAR_FILL1: u32 = 86;

impl Hud {
    /// Initialize health, shard, and encounter presentation state.
    pub fn new() -> Hud {
        Hud { t: 0, init: false, hp: 0, max: 0, hearts: vec![HeartChange::default(); 32], hurt_t: 0, shards: 0, shard_t: None, boss: BossBar::default() }
    }

    /// `shards` = prism shards collected this run, including the current zone's.
    pub fn update(&mut self, play: &Play, shards: u32, fx: &mut Fx) {
        self.t += 1;
        let hp = play.player.body.hp.clamp(0, 31);
        let max = play.player.body.max_hp.clamp(1, 31);
        if !self.init {
            self.init = true;
            self.hp = hp;
            self.shards = shards;
        }
        if hp < self.hp {
            for i in hp..self.hp {
                self.hearts[i as usize] = HeartChange { at: self.t, gained: false, set: true };
            }
            self.hurt_t = self.t;
        } else if hp > self.hp {
            // refills land one after another
            for (k, i) in (self.hp..hp).enumerate() {
                self.hearts[i as usize] = HeartChange { at: self.t + k as u64 * 5, gained: true, set: true };
            }
        }
        self.hp = hp;
        self.max = max;
        if shards > self.shards {
            self.shard_t = Some(self.t);
        }
        self.shards = shards;

        // the Warden's bar
        let b = &mut self.boss;
        match (&play.boss, play.boss_awake) {
            (Some(boss), true) => {
                let target = boss.hp_frac();
                if !b.on {
                    *b = BossBar { on: true, hit_t: 99, ..Default::default() };
                }
                b.t += 1;
                b.hit_t = b.hit_t.saturating_add(1);
                if b.t < BAR_FILL0 {
                    b.shown = 0.0;
                    b.trail = 0.0;
                } else if b.t < BAR_FILL1 {
                    // it fills up with a rising tick, like a wind-up
                    let k = ease_out_cubic((b.t - BAR_FILL0) as f32 / (BAR_FILL1 - BAR_FILL0) as f32);
                    b.shown = target * k;
                    b.trail = b.shown;
                    if (b.t - BAR_FILL0) % 6 == 0 {
                        fx.sound_vol("ui_move", 0.3);
                    }
                } else {
                    if target < b.shown - 1e-4 {
                        b.trail = b.trail.max(b.shown);
                        b.shown = target;
                        b.hold = 30;
                        b.hit_t = 0;
                    } else if target > b.shown {
                        b.shown = target;
                    }
                    if b.hold > 0 {
                        b.hold -= 1;
                    } else {
                        b.trail = (b.trail - 0.007).max(b.shown);
                    }
                }
                if target <= 0.0 {
                    b.dead = Some(b.dead.map_or(0, |d| d + 1));
                }
            }
            _ => b.on = false,
        }
    }
}

/// Fallback health pip until `hud_hearts` exists: a small faceted shard.
/// o = outline, a/b/c = light/mid/dark.
const PIP: [&str; 8] = ["...o...", "..oao..", ".oaabo.", "oaabbco", "oabbcco", ".obcco.", "..oco..", "...o..."];

/// Draw a readable health fallback when the heart sheet is unavailable.
fn draw_pip(f: &mut Frame, x: i32, y: i32, full: bool, flash: bool) {
    for (py, row) in PIP.iter().enumerate() {
        for (px, ch) in row.chars().enumerate() {
            let c = match ch {
                '.' => continue,
                'o' => INK,
                _ if flash => Rgb::WHITE,
                'a' if !full => BORDER0,
                _ if !full => PANEL1,
                'a' => HP[3],
                'b' => HP[2],
                _ => HP[1],
            };
            f.ui_put(x + px as i32, y + py as i32, c);
        }
    }
}

/// One heart per hit point (the UI artist's `hud_hearts`): "lose" plays on a heart that empties,
/// "gain" on one that fills. With one heart left, it thumps: a 1 px hop twice a beat.
fn draw_health(f: &mut Frame, hud: &Hud) {
    let t = hud.t;
    let since_hurt = t.saturating_sub(hud.hurt_t);
    let shake = if hud.hurt_t > 0 && since_hurt < 8 { [1, -1, 1, -1, 1, 0, -1, 0][since_hurt as usize] } else { 0 };
    let hearts = sheet("hud_hearts");
    let spacing = hearts.and_then(|s| s.json.get("spacing")?.as_i64()).map(|v| v as i32).unwrap_or(9);
    let beat = (t % 64) as i32;
    for i in 0..hud.max {
        let x = HUD_X + i * spacing + shake;
        let ch = hud.hearts[i as usize];
        let full = i < hud.hp;
        let thump = hud.hp == 1 && i == 0 && (beat < 4 || (9..13).contains(&beat));
        let y = HUD_Y - thump as i32;
        // a one-shot (lose/gain) if one is playing, else the held state
        let pending = ch.set && ch.gained && t < ch.at;
        let shot = if ch.set && t >= ch.at { Some((if ch.gained { "gain" } else { "lose" }, (t - ch.at) as u32)) } else { None };
        let held = if full && !pending { "full" } else { "empty" };
        if let Some(s) = hearts {
            let (a, k) = match shot.and_then(|(n, k)| s.anim(n).filter(|a| !a.finished(k)).map(|a| (a, k))) {
                Some(x) => x,
                None => match s.anim(held) {
                    Some(a) => (a, 0),
                    None => continue,
                },
            };
            let cell = s.cell(a, a.frame_at(k, false));
            let (ax, ay) = s.anchor_of(a);
            ui_cell(f, cell, x - ax, y - ay, false, |_, _, p| Some(p.c));
        } else {
            let flash = shot.map(|(_, k)| k < 4).unwrap_or(false);
            draw_pip(f, x, y, held == "full", flash);
        }
    }
}

/// Prism shards collected (the artist's `hud_shard` icon and the count), top right.
/// The icon pops (white flash, a little hop) when the count goes up.
fn draw_shards(f: &mut Frame, hud: &Hud) {
    let sm = &fonts().small;
    let label = hud.shards.to_string();
    let tw = sm.width(&label, 0);
    let ix = WI - 8 - 11 - tw;
    f.ui_rect(ix - 3, HUD_Y - 3, WI - 5 - (ix - 3), 15, PANEL0, 1.0);
    let iy = HUD_Y;
    let popping = hud.shard_t.map(|at| (hud.t - at) as u32);
    if let Some(s) = sheet("hud_shard") {
        let (a, k) = match (popping, s.anim("pop")) {
            (Some(k), Some(pop)) if !pop.finished(k) => (pop, k),
            _ => match s.anim("idle") {
                Some(a) => (a, 0),
                None => return,
            },
        };
        let cell = s.cell(a, a.frame_at(k, false));
        let (ax, ay) = s.anchor_of(a);
        ui_cell(f, cell, ix - ax, iy - ay, false, |_, _, p| Some(p.c));
    } else {
        diamond(f, ix + 4, iy + 5, 4, CYAN[2], Some(INK));
    }
    let hop = matches!(popping, Some(k) if k < 6) as i32;
    text(f, sm, ix + 11, iy + 2 - hop - sm.cap_top(), &label, &Ink::new(TEXT));
}

/// Short lines under each region's name when you arrive.
fn region_line(id: ZoneId) -> &'static str {
    match id {
        ZoneId::Bloomwild => "Mind the sproutlings. They bite.",
        ZoneId::Glasswater => "Bring a light. Oh wait, you are one.",
        ZoneId::Cinder => "The floor is lava. Mostly.",
        ZoneId::Summit => "Something big is waiting up here.",
    }
}

const BANNER_IN: u32 = 34;
const BANNER_OUT: u32 = 250;

/// The region's name lands like an inscription on one quiet plate, followed by its short line.
fn draw_banner(f: &mut Frame, play: &Play) {
    let a = play.arrive_t;
    if !(BANNER_IN..BANNER_OUT + 40).contains(&a) || play.zone.title.is_empty() {
        return;
    }
    let sm = &fonts().small;
    let name = title_case(&play.zone.title);
    let display = &fonts().display;
    let d = if display.width(&name, 0) <= WI - 32 { display } else { sm };
    let spacing = if d.width(&name, 1) <= WI - 92 { 1 } else { 0 };
    let ink = Ink::new(TEXT).outline(INK).spacing(spacing);
    let w = d.width(&name, ink.spacing);
    let cx = WI / 2;
    let y = 30;
    let at = (a - BANNER_IN) as f32;
    let out = 1.0 - ramp(a as f32, BANNER_OUT as f32, 32.0);
    // A quiet backing keeps lit crystals and scenery out of the letters' counters.
    let panel_w = (w.max(sm.width(region_line(play.zone.id), 0)) + 24).min(WI - 16);
    panel(f, cx - panel_w / 2, y - 8, panel_w, 43, ramp(at, 0.0, 8.0) * out);
    // letters land one at a time, each a white flash first
    let n = name.chars().count();
    let x0 = cx - w / 2;
    text_with(f, d, x0, y, &name, &ink, |k, _| {
        let land = 8.0 + k as f32 * 2.5;
        if at < land {
            return None;
        }
        let since = at - land;
        let dy = if since < 2.0 {
            -2
        } else if since < 4.0 {
            -1
        } else {
            0
        };
        let c = if since < 3.0 { Rgb::WHITE } else { TEXT };
        step_in(c, out).map(|c| (c, dy))
    });
    // the line under it
    let sub_at = 8.0 + n as f32 * 2.5 + 14.0;
    let sa = ramp(at, sub_at, 14.0) * out;
    text_fade(f, sm, cx, y + d.cell_h + 5, region_line(play.zone.id), &Ink::new(TEXT), sa);
}

/// Draw the Warden frame, delayed damage trail, and encounter name.
fn draw_boss_bar(f: &mut Frame, hud: &Hud) {
    let b = &hud.boss;
    if !b.on {
        return;
    }
    let fade = match b.dead {
        Some(d) => 1.0 - ramp(d as f32, 20.0, 24.0),
        None => 1.0,
    };
    if fade <= 0.0 {
        return;
    }
    let shake = if b.hit_t < 8 { [0, 2, -1, 1, -1, 1, 0, 0][b.hit_t as usize] } else { 0 };
    let cx = WI / 2 + shake;
    let sm = &fonts().small;
    let name = "The Warden";
    let typed = (b.t.saturating_sub(10) / 2) as usize;
    let name_ink = Ink::new(TEXT).outline(INK).spacing(1);

    // the UI artist's frame (stone bar, blade point, the Warden's cracked prism heart)
    if let Some(s) = sheet("hud_bossbar")
        && let Some(a) = s.anim("frame").or_else(|| s.anims.values().next())
    {
        let cell = s.cell(a, 0);
        let (_, ay) = s.anchor_of(a);
        let name_w = sm.width(name, name_ink.spacing);
        let left = (WI - cell.w - name_w - 10) / 2 + name_w + 10 + shake;
        let top = HI - 20 - ay;
        let fa = fade * ramp(b.t as f32, 0.0, 12.0);
        ui_cell(f, cell, left, top, false, |_, _, p| step_in(p.c, fa));
        let fr: Vec<i32> =
            s.json.get("fill_rect").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_i64()).map(|x| x as i32).collect()).unwrap_or_default();
        let (fx0, fy0, fw, fh) = if fr.len() == 4 { (fr[0], fr[1], fr[2], fr[3]) } else { (16, 7, 184, 5) };
        // fill columns: 0 health, 1 chip (recently lost), 2 white hit flash
        let cols = sheet("hud_bossbar_fill").and_then(|s| s.cells.first());
        let fill = (fw as f32 * b.shown).round() as i32;
        let trail = (fw as f32 * b.trail).round() as i32;
        for k in 0..trail.max(fill) {
            let col = if k >= fill {
                1
            } else if b.hit_t < 2 {
                2
            } else {
                0
            };
            for yy in 0..fh {
                let c = match cols.and_then(|r| r.get(col)) {
                    Some(cc) => cc.at(0, yy).c,
                    None => [PRISM_RAMP[2], TEXT, Rgb::WHITE][col],
                };
                if let Some(c) = step_in(c, fade) {
                    f.ui_put(left + fx0 + k, top + fy0 + yy, c);
                }
            }
        }
        text_with(f, sm, left - 10 - name_w, top + fy0 + fh / 2 - sm.cap_h / 2 - sm.cap_top(), name, &name_ink, |k, _| if k < typed { step_in(TEXT, fade).map(|c| (c, 0)) } else { None });
        return;
    }

    // fallback: a code-drawn bar
    let y = HI - 16;
    let name_w = sm.width(name, name_ink.spacing);
    let cx = cx + (name_w + 10) / 2;
    let grow = ease_out_cubic(ramp(b.t as f32, 0.0, 18.0));
    let half = ((BAR_W / 2) as f32 * grow).round() as i32;
    if half < 2 {
        return;
    }
    let (x0, x1) = (cx - half, cx + half);
    let Some(edge) = step_in(BORDER0, fade) else { return };
    // frame: ink outline, border, dark track
    for x in x0 - 1..=x1 + 1 {
        for yy in -2..=5 {
            let c = if yy == -2 || yy == 5 || x == x0 - 1 || x == x1 + 1 {
                INK
            } else if yy == -1 || yy == 4 || x == x0 || x == x1 {
                edge
            } else {
                PANEL0
            };
            if (x == x0 - 1 || x == x1 + 1) && (yy == -2 || yy == 5) {
                continue;
            }
            f.ui_put(x, y + yy, c);
        }
    }
    // end caps: small prism diamonds
    if let (Some(gc), Some(gd)) = (step_in(BORDER1, fade), step_in(BORDER0, fade)) {
        diamond(f, x0 - 3, y + 1, 3, gd, Some(INK));
        diamond(f, x1 + 3, y + 1, 3, gd, Some(INK));
        f.ui_put(x0 - 3, y, gc);
        f.ui_put(x1 + 3, y, gc);
    }
    // fill and damage trail
    let inner = x1 - x0 - 1;
    let fill = (inner as f32 * b.shown).round() as i32;
    let trail = (inner as f32 * b.trail).round() as i32;
    let low = b.shown < 0.3 && (hud.t / 8) % 4 == 0;
    let flash = b.hit_t < 3;
    for k in 0..trail.max(fill) {
        let x = x0 + 1 + k;
        for yy in 0..4 {
            let c = if k >= fill {
                if b.hold > 12 { TEXT } else { HP[3] }
            } else if flash && k >= fill - 2 {
                Rgb::WHITE
            } else {
                let c = match yy {
                    0 => PRISM_RAMP[3],
                    3 => PRISM_RAMP[1],
                    _ => PRISM_RAMP[2],
                };
                if low { c.lerp(PRISM_RAMP[3], 0.5) } else { c }
            };
            if let Some(c) = step_in(c, fade) {
                f.ui_put(x, y + yy, c);
            }
        }
    }
    // name, typed in
    let w = sm.width(name, 1);
    text_with(
        f,
        sm,
        cx - BAR_W / 2 - 10 - w,
        y + 2 - sm.cap_h / 2 - sm.cap_top(),
        name,
        &name_ink,
        |k, _| if k < typed { step_in(TEXT, fade).map(|c| (c, 0)) } else { None },
    );
}

/// Draw health, collected shards, arrival banners, and encounter health.
pub fn hud(f: &mut Frame, play: &Play, hud: &Hud) {
    let spacing = sheet("hud_hearts").and_then(|s| s.json.get("spacing")?.as_i64()).unwrap_or(8) as i32;
    let width = (hud.max - 1).max(0) * spacing + 5;
    f.ui_rect(HUD_X - 3, HUD_Y - 3, width + 6, 15, PANEL0, 1.0);
    draw_health(f, hud);
    let ready = play.player.dash_cd == 0 && !matches!(play.player.state, PState::Dash { .. });
    f.ui_rect(HUD_X, HUD_Y + 8, width, 1, if ready { ENERGY[1] } else { BORDER0 }, 1.0);
    draw_shards(f, hud);
    draw_banner(f, play);
    draw_boss_bar(f, hud);
}

// ---------------------------------------------------------------- pause

/// Pause over a frozen world with a compact control card and one clear resume action.
pub fn paused(f: &mut Frame, pt: u32, play: &Play) {
    dim(f, 0.625);
    panel(f, 40, 20, 304, 176, 1.0);
    let a = ramp(pt as f32, 0.0, 8.0);
    let d = &fonts().display;
    let sm = &fonts().small;
    let ft = &fonts().tiny;
    let cx = WI / 2;
    text_fade(f, d, cx, 34, "Paused", &Ink::new(TEXT).spacing(1), a);
    text_fade(f, ft, cx, 55, &play.zone.title, &Ink::new(MUTED), a);
    rule(f, cx - 45, cx - 4, 70, a);
    rule(f, cx + 45, cx + 4, 70, a);

    let rows = [
        (crate::input::key_labels().0, "pad_lstick", "move"),
        (crate::input::key_labels().1, "pad_south", "slash / hold for spin"),
        (crate::input::key_labels().2, "pad_east", "dash through danger"),
        ("ESC", "pad_menu", "pause / resume"),
    ];
    for (i, (keys, pad, what)) in rows.iter().enumerate() {
        let y = 83 + i as i32 * 18;
        let kw = keys.split(' ').map(key_width).sum::<i32>() + 2 * (keys.split(' ').count() as i32 - 1);
        let mut x = 144 - kw;
        for k in keys.split(' ') {
            x += keycap(f, x, y, k, a) + 2;
        }
        prompt(f, pad, 152, y - if *pad == "pad_lstick" { 1 } else { 0 }, a);
        if let Some(c) = step_in(MUTED, a) {
            text(f, sm, 176, y + 2, what, &Ink::new(c));
        }
    }
    let x = 151;
    cursor(f, x - 18, 174, pt, a);
    let kw = keycap(f, x, 169, "ENTER", a);
    if let Some(c) = step_in(TEXT, a) {
        text(f, sm, x + kw + 6, 171, "Resume", &Ink::new(c));
    }
}

// ---------------------------------------------------------------- victory and credits

/// From this tick on the victory screen, a press goes back to the title.
pub const VICTORY_PROMPT: u64 = 470;

/// Format a run duration as minutes and seconds.
fn clock(ticks: u64) -> String {
    let s = ticks / 60;
    format!("{}:{:02}", s / 60, s % 60)
}

/// The whole Prism, hand-placed: lit from the upper left (white and cyan facets), magenta and
/// violet on the shaded side, a iris girdle. o = deep prism violet outline.
const PRISM: [&str; 24] = [
    ".......o.......",
    "......oWo......",
    ".....owWmo.....",
    "....owwWmmo....",
    "...owwwWmmmo...",
    "..ocwwwWmmmvo..",
    ".occcwwWmmvvvo.",
    "occccwwWmvvvvvo",
    "oyWyyyyyyyyyyGo",
    "ogggggggggggGGo",
    "obbbcccWvvmVVVo",
    ".obbccWvvmVVVo.",
    ".obbbccWvmVVVo.",
    "..obbcWvmVVVo..",
    "..obbccWmVVVo..",
    "...obcWmVVVo...",
    "...obccWVVVo...",
    "....obcWVVo....",
    "....obccVVo....",
    ".....obcVo.....",
    ".....obWVo.....",
    "......obo......",
    "......oco......",
    ".......o.......",
];

/// Resolve the authored Prism facet palette.
fn prism_color(ch: char) -> Option<Rgb> {
    Some(match ch {
        'o' => PRISM_INK,
        'W' => PRISM_RAMP[3],
        'w' => PRISM_RAMP[2],
        'c' => PRISM_RAMP[2],
        'b' => PRISM_RAMP[1],
        'm' => PRISM_RAMP[1],
        'v' => PRISM_RAMP[0],
        'V' => PRISM_INK,
        'y' => PRISM_RAMP[3],
        'g' => PRISM_RAMP[1],
        'G' => PRISM_RAMP[0],
        _ => return None,
    })
}

/// Victory timeline (ticks into the victory scene).
const V_FLY: f32 = 30.0;
/// The three shards meet and the Prism is whole (the game plays a chime and a flash here).
pub const VICTORY_MEET: u64 = 95;
const V_HEAD: f32 = 122.0;
const PRISM_C: (i32, i32) = (WI / 2, 22);

/// The three shards (one per region) fly in from the edges, meet, flash, and the Prism appears.
fn draw_prism_moment(f: &mut Frame, t: u64) {
    let tf = t as f32;
    let (pcx, pcy) = PRISM_C;
    let meet = VICTORY_MEET as f32;
    if tf >= V_FLY && tf < meet + 2.0 {
        // from the left, the right and below; each on a gentle arc
        let starts = [(-24.0, 120.0, -50.0), (408.0, 96.0, -36.0), (150.0, 236.0, 60.0)];
        for (i, &(sx, sy, bend)) in starts.iter().enumerate() {
            let k = ease_out_cubic(((tf - V_FLY - i as f32 * 5.0) / (meet - V_FLY - i as f32 * 5.0)).clamp(0.0, 1.0));
            if k <= 0.0 {
                continue;
            }
            let (ex, ey) = (pcx as f32, pcy as f32 + 4.0);
            let mx = (sx + ex) / 2.0 + bend * 0.4;
            let my = (sy + ey) / 2.0 + bend;
            let at = |k: f32| {
                let u = 1.0 - k;
                (u * u * sx + 2.0 * u * k * mx + k * k * ex, u * u * sy + 2.0 * u * k * my + k * k * ey)
            };
            // a short dithered wake
            let (wx, wy) = at((k - 0.06).max(0.0));
            let (x, y) = at(k);
            Frame::line_with(wx as i32, wy as i32, x as i32, y as i32, |px, py| {
                if bayer4(px, py) < 0.5 {
                    f.ui_put(px, py, CYAN[1]);
                }
            });
            if let Some(s) = sheet("shard")
                && let Some(a) = s.anim("spin")
            {
                let cell = s.cell(a, a.frame_at(t as u32 + i as u32 * 3, true));
                let (ax, ay) = s.anchor_of(a);
                // the shard floats ~11 px over its anchor; center it on the path
                ui_cell(f, cell, x as i32 - ax, y as i32 - ay + 11, false, |_, _, p| Some(p.c));
            } else {
                diamond(f, x as i32, y as i32, 3, CYAN[2], Some(rgb(0x2a1d6a)));
            }
        }
    }
    if tf < meet {
        return;
    }
    let since = tf - meet;
    // the shards' pickup flash at the meeting point, and one dotted ring going out
    if let Some(s) = sheet("shard")
        && let Some(a) = s.anim("pickup")
        && !a.finished(since as u32)
    {
        let cell = s.cell(a, a.frame_at(since as u32, false));
        let (ax, ay) = s.anchor_of(a);
        ui_cell(f, cell, pcx - ax, pcy + 4 - ay + 11, false, |_, _, p| Some(p.c));
    }
    if since < 30.0 {
        let r = 6.0 + since * 2.2;
        let c = if since < 10.0 {
            CYAN[3]
        } else if since < 20.0 {
            CYAN[2]
        } else {
            CYAN[1]
        };
        let n = (r * 2.4) as i32;
        for k in 0..n {
            if k % 2 == 1 {
                continue;
            }
            let ang = k as f32 / n as f32 * PI * 2.0;
            f.ui_put(pcx + (ang.cos() * r).round() as i32, pcy + (ang.sin() * r * 0.8).round() as i32, c);
        }
    }
    // the Prism: a white silhouette first, then its colors
    if since >= 3.0 {
        let white = since < 7.0;
        let (w, h) = (PRISM[0].len() as i32, PRISM.len() as i32);
        let (x0, y0) = (pcx - w / 2, pcy - h / 2);
        for (yy, row) in PRISM.iter().enumerate() {
            for (xx, ch) in row.chars().enumerate() {
                let Some(mut c) = prism_color(ch) else { continue };
                if white && ch != 'o' {
                    c = Rgb::WHITE;
                }
                f.ui_put(x0 + xx as i32, y0 + yy as i32, c);
            }
        }
    }
}

/// Stage the restored Prism, run statistics, credits, and return prompt.
pub fn victory(f: &mut Frame, t: u64, run_ticks: u64, falls: u32) {
    let tf = t as f32;
    dim(f, 0.64 * ease_out_cubic(ramp(tf, 10.0, 110.0)));
    let d = &fonts().display;
    let sm = &fonts().small;
    let ft = &fonts().tiny;
    let cx = WI / 2;

    draw_prism_moment(f, t);

    // The restored Prism is followed by a letter-by-letter inscription.
    let head = "The Prism is whole again.";
    let ink = Ink::new(TEXT).spacing(0);
    let w = d.width(head, ink.spacing);
    let y = 44;
    text_with(f, d, cx - w / 2, y, head, &ink, |k, _| {
        let land = V_HEAD + k as f32 * 3.0;
        if tf < land {
            return None;
        }
        let since = tf - land;
        Some((
            if since < 3.0 { Rgb::WHITE } else { TEXT },
            if since < 2.0 {
                -2
            } else if since < 4.0 {
                -1
            } else {
                0
            },
        ))
    });
    let after = V_HEAD + head.chars().count() as f32 * 3.0;
    text_fade(f, sm, cx, y + d.cell_h + 5, "Glint has earned a dramatic exit.", &Ink::new(MUTED), ramp(tf, after + 20.0, 16.0));

    // a rule and the run's numbers
    let ry = y + d.cell_h + 27;
    let ra = ramp(tf, after + 60.0, 12.0);
    let len = (56.0 * ease_out_cubic(ramp(tf, after + 60.0, 20.0))) as i32;
    if len > 0 {
        rule(f, cx - 6, cx - 6 - len, ry, ra);
        rule(f, cx + 6, cx + 6 + len, ry, ra);
        if let Some(c) = step_in(BORDER1, ra) {
            diamond(f, cx, ry, 2, c, Some(INK));
        }
    }
    let sa = ramp(tf, after + 80.0, 12.0);
    let stats = [("TIME", clock(run_ticks)), ("FALLS", falls.to_string())];
    for (i, (label, value)) in stats.iter().enumerate() {
        let sx = cx + if i == 0 { -34 } else { 34 };
        text_fade(f, ft, sx, ry + 8, label, &Ink::new(BORDER1), sa);
        text_fade(f, sm, sx, ry + 16, value, &Ink::new(TEXT), sa);
    }

    // credits: the static cut-stone logo, then two lines
    let ca_t = after + 150.0;
    let ca = ramp(tf, ca_t, 16.0);
    let base = ry + 58;
    if !draw_logo_sheet(f, "logo_small", cx, base, ca) {
        text_fade(f, ft, cx, base - 6, "GLINTBLADE", &Ink::new(CYAN[2]).spacing(2), ca);
    }
    text_fade(f, sm, cx, base + 14, "A tiny sky adventure, made for Kyle.", &Ink::new(MUTED), ramp(tf, ca_t + 30.0, 16.0));
    text_fade(f, sm, cx, base + 27, "Thanks for playing!", &Ink::new(TEXT), ramp(tf, ca_t + 55.0, 16.0));

    // back to the title
    if t >= VICTORY_PROMPT {
        let pa = ramp(tf, VICTORY_PROMPT as f32, 20.0);
        let phase = (t / 24) % 5;
        let c = [MUTED, MUTED.lerp(TEXT, 0.5), TEXT, TEXT, MUTED.lerp(TEXT, 0.5)][phase as usize];
        let kw = ft.width("ENTER", 0) + 6;
        let label = "title";
        let w = kw + 5 + ft.width(label, 0);
        let x = cx - w / 2;
        let y = HI - 20;
        keycap(f, x, y, "ENTER", pa);
        if let Some(col) = step_in(c, pa) {
            text(f, ft, x + kw + 5, y + 2 - ft.cap_top(), label, &Ink::new(col));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::boss::{self, Boss};
    use crate::play::PlayOpts;

    /// Glyphs reproduce the artist's exact ink, including lowercase descenders and punctuation.
    #[test]
    fn glyph_sheet_masks_render_without_lost_pixels() {
        for name in ["font_small", "font_tiny", "font_display"] {
            let font = Font::from_sheet(name).expect("authored font available");
            let sheet = sheet(name).unwrap();
            let mut frame = Frame::new();
            for (&ch, glyph) in &font.glyphs {
                frame.out.fill([0, 0, 0, 255]);
                font.blit(&mut frame, ch, 0, 0, TEXT);
                let bounds = sheet.json["glyphs"][ch.to_string()].as_array().unwrap();
                let x = bounds[0].as_i64().unwrap() as i32;
                let y = bounds[1].as_i64().unwrap() as i32;
                let cell = &sheet.cells[(y / sheet.cell_h) as usize][(x / sheet.cell_w) as usize];
                for gy in 0..font.cell_h {
                    for gx in 0..glyph.w {
                        assert_eq!(
                            frame.out[(gy * WI + gx) as usize][0] > 0,
                            cell.at(x % sheet.cell_w + gx, y % sheet.cell_h + gy).a,
                            "{name} {ch:?} at {gx},{gy}"
                        );
                    }
                }
            }
        }
    }

    /// Heart loss, staggered healing, shard pickup, and boss damage trigger distinct animations.
    #[test]
    fn hud_reacts_to_health_shards_and_warden_damage() {
        let mut play = Play::new(ZoneId::Bloomwild, &PlayOpts { god: true, peaceful: true });
        let mut fx = Fx::new(7);
        let mut hud = Hud::new();
        hud.update(&play, 0, &mut fx);
        let hp = play.player.body.hp;
        play.player.body.hp -= 2;
        hud.update(&play, 0, &mut fx);
        assert!(hud.hearts[(hp - 1) as usize].set);
        assert!(!hud.hearts[(hp - 1) as usize].gained);
        play.player.body.hp += 2;
        hud.update(&play, 1, &mut fx);
        assert!(hud.hearts[(hp - 1) as usize].gained);
        assert!(hud.hearts[(hp - 1) as usize].at > hud.hearts[(hp - 2) as usize].at);
        assert_eq!(hud.shard_t, Some(hud.t));
        play.boss = Some(Boss::new(200.0, 150.0));
        play.boss_awake = true;
        for _ in 0..100 {
            hud.update(&play, 1, &mut fx);
        }
        assert_eq!(hud.boss.shown, 1.0);
        play.boss.as_mut().unwrap().set_hp_for_test(boss::MAX_HP / 2);
        hud.update(&play, 1, &mut fx);
        assert_eq!(hud.boss.shown, 0.5);
        assert_eq!(hud.boss.trail, 1.0);
        for _ in 0..40 {
            hud.update(&play, 1, &mut fx);
        }
        assert!(hud.boss.trail < 1.0 && hud.boss.trail > hud.boss.shown);
    }
}
