//! Static or animated scenery drawn from sprite sheets: trees, pillars, crystals, urns...
//! Props are y-sorted with characters. Breakable props burst into pixels when slashed.
//! `Prop::new` reads "footprint", "breakable", "light" and "layer" from the anim's JSON
//! (falling back to the sheet's top level), so artists control them from the manifest.

use crate::fx::Fx;
use crate::gfx::{DrawOpts, Frame, Rgb, Sheet, sheet};
use serde_json::Value;

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
        p
    }

    pub fn flipped(mut self, f: bool) -> Prop {
        self.flip = f;
        self
    }

    pub fn sheet(&self) -> Option<&'static Sheet> {
        sheet(self.sheet)
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
        if !self.alive {
            return;
        }
        let Some(s) = self.sheet() else { return };
        let Some(a) = s.anim(&self.anim) else { return };
        let cell = s.cell(a, a.frame_at(t as u32 + self.t0, true));
        let wob = if self.wobble > 0.0 { ((self.wobble * 1.7).sin() * self.wobble * 0.25).round() } else { 0.0 };
        let o = DrawOpts { flip: self.flip, flash: self.flash, alpha, ..Default::default() };
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
