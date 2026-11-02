//! Opt-in, pixel-grid reflections. Mark water during `Region::draw_ground`, before sprites.
//! `sprite` then mirrors opaque source pixels below their anchor, clipped to the water mask.
//! Tune `frame.look.water_*` after `begin`; call `reflect_sprite` directly for a different
//! waterline (for example, a raised crystal on a bank). No blur or interpolated sprite pixels.

use super::{Cell, DrawOpts, Frame, HI, Rgb, W, WI};
use crate::util::{bayer4, clamp01};

impl Frame {
    /// Mark one screen pixel as water; zero clears it and 1 is fully reflective.
    pub fn mark_water(&mut self, x: i32, y: i32, reflectivity: f32) {
        if let Some(i) = Self::idx(x, y) {
            let r = (clamp01(reflectivity) * 255.0 + 0.5) as u8;
            self.water[i] = r;
            self.water_any |= r > 0;
        }
    }

    /// Mark a screen rectangle as reflective water, clipped to the framebuffer.
    pub fn water_rect(&mut self, x: i32, y: i32, w: i32, h: i32, reflectivity: f32) {
        let r = (clamp01(reflectivity) * 255.0 + 0.5) as u8;
        for py in y.max(0)..(y + h).min(HI) {
            for px in x.max(0)..(x + w).min(WI) {
                self.water[py as usize * W + px as usize] = r;
                self.water_any |= r > 0;
            }
        }
    }

    /// Mirror the projected sprite around its physical ground plane onto marked water.
    pub fn reflect_sprite(&mut self, cell: &Cell, anchor: (i32, i32), sx: i32, sy: i32, o: &DrawOpts) {
        if !self.water_any || self.look.water_alpha <= 0.0 || o.alpha <= 0.0 || cell.h <= 0 {
            return;
        }
        let ox = sx - if o.flip { cell.w - 1 - anchor.0 } else { anchor.0 };
        // `sy` already includes airborne projection. Mirror the projected image around
        // its physical water plane, so lifting a sprite moves its reflection down.
        let ground = o.ground_y.map(|y| y - self.cam_y as f32)
            .unwrap_or(sy as f32 + o.elevation);
        let mirror_y = (2.0 * ground - sy as f32).round() as i32;
        let margin = self.look.water_wave.abs().ceil() as i32 + 1;
        let source_end = cell.h.min(anchor.1 + 1);
        // Reject distant sprites before inspecting their native alpha bounds.
        if ox - margin >= WI || ox + cell.w + margin < 0 || mirror_y >= HI
            || mirror_y + anchor.1 + 1 < 0 || source_end <= 0 {
            return;
        }
        // The native atlas is padded. Its transparent rows must not postpone the fade,
        // and the cap must shorten the fade rather than simply chop off a solid image.
        let Some(top) = (0..source_end).find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)) else { return };
        let span = (anchor.1 - top + 1).min(self.look.water_depth_max);
        if span <= 0 || mirror_y + span < 0 { return; }
        for y in top..source_end {
            let depth = anchor.1 - y;
            if depth >= span { continue; }
            let py = mirror_y + 1 + depth;
            if !(0..HI).contains(&py) { continue; }
            let wy = py + self.cam_y;
            let phase = wy as f32 * 0.22 + self.t as f32 * 0.045;
            let shift = (phase.sin() * self.look.water_wave).round() as i32;
            let reach = (depth as f32 + 0.5) / span as f32;
            let tail = ((reach - 0.35) / 0.65).clamp(0.0, 1.0);
            let fade = 1.0 - tail * tail * (3.0 - 2.0 * tail);
            // Restrained horizontal ribbons read as water, while retaining the native
            // silhouette instead of deleting a complete head row at some phases.
            let ribbon = if reach > 0.6 && (wy + (self.t / 10) as i32).rem_euclid(6) == 0 { 0.45 } else { 1.0 };
            let opacity = self.look.water_alpha * fade * ribbon;
            for x in 0..cell.w {
                let p = cell.at(x, y);
                if !p.a { continue; }
                let px = ox + if o.flip { cell.w - 1 - x } else { x } + shift;
                let Some(i) = Self::idx(px, py) else { continue };
                let a = (opacity * self.water[i] as f32 / 255.0).clamp(0.0, 1.0);
                if a <= 0.0 || o.alpha <= bayer4(px + self.cam_x, wy) || self.actor_pixels[i]
                    || self.surfaces[i].owner != 0
                    || if self.surface_lighting { self.surfaces[i].kind != 1 }
                        else { !matches!(self.surfaces[i].kind, 0 | 1) } {
                    continue;
                }
                let mut c = o.silhouette.unwrap_or(p.c);
                if o.silhouette.is_none() {
                    if let Some((t, k)) = o.tint { c = c.lerp(t, k); }
                    c = cell.flash_color(x, y, c, o.flash);
                }
                c = c * self.look.water_tint;
                // A reflected image is transmitted through water. Blend both channels
                // into that receiver; opaque replacement made the short old mirror look
                // like another chunk of the actor attached directly to its boots.
                let glow = (p.emit && o.silhouette.is_none()) || o.unlit
                    || (o.silhouette.is_some() && o.silhouette_glow);
                self.albedo[i] = self.albedo[i].lerp(if glow { Rgb::BLACK } else { c }, a);
                self.emissive[i] = self.emissive[i].lerp(if glow { c.scale(o.emit_gain * 0.65) } else { Rgb::BLACK }, a);
                // Direct channel blending deliberately retains terrain geometry/ownership.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::frame::Surface;

    #[test]
    fn reflected_actors_and_emission_preserve_the_water_receiver() {
        for glow in [false, true] {
            let emits = if glow { vec!['a'] } else { Vec::new() };
            let cell = Cell::from_ascii(&["a", "a"], &[('a', Rgb::WHITE)], &emits);
            let mut f = Frame::new();
            f.surface_lighting = true;
            f.look.water_alpha = 1.0;
            f.look.water_wave = 0.0;
            f.look.water_depth_max = 8;
            f.look.water_tint = Rgb::WHITE;
            f.water_rect(15, 20, 12, 8, 1.0);
            let receiver = Surface { normal: [0.1, 0.0, 0.995], height: 0.0, owner: 0, kind: 1 };
            f.surfaces.fill(receiver);
            f.reflect_sprite(&cell, (0, 1), 20, 20, &DrawOpts::default());
            let i = 21 * W + 20;
            assert!(f.albedo[i].luma() > 0.0 || f.emissive[i].luma() > 0.0);
            assert_eq!(f.surfaces[i].normal, receiver.normal);
            assert_eq!((f.surfaces[i].kind, f.surfaces[i].height), (1, 0.0));
            assert!(!f.actor_pixels[i], "a reflection is still receiving water");
        }
    }

    #[test]
    fn a_reflection_cannot_paint_over_a_visible_actor_or_prop() {
        let cell = Cell::from_ascii(&["a", "a"], &[('a', Rgb::WHITE)], &[]);
        for actor in [false, true] { for lighting in [false, true] {
            let mut f = Frame::new();
            f.surface_lighting = lighting;
            f.look.water_alpha = 1.0;
            f.look.water_wave = 0.0;
            f.look.water_depth_max = 8;
            f.water_rect(15, 20, 12, 8, 1.0);
            let i = 21 * W + 20;
            f.actor_pixels[i] = actor;
            f.surfaces[i] = Surface { normal: [0.0, 1.0, 0.0], height: 8.0, owner: 3,
                kind: if actor { 3 } else { 2 } };
            f.albedo[i] = Rgb::new(0.8, 0.1, 0.1);
            let old = f.albedo[i];
            f.reflect_sprite(&cell, (0, 1), 20, 20, &DrawOpts::default());
            assert_eq!(f.albedo[i], old);
            assert_eq!(f.surfaces[i].owner, 3);
        }}
    }

    #[test]
    fn a_stale_water_mask_cannot_reflect_over_foreground_but_real_water_keeps_emission() {
        let cell = Cell::from_ascii(&["a"; 8], &[('a', Rgb::WHITE)], &[]);
        for kind in [0, 1, 2, 4, 5] {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::WHITE);
            f.surface_lighting = true;
            f.look.water_wave = 0.0;
            f.look.water_alpha = 0.5;
            f.water_rect(100, 101, 1, 8, 1.0);
            let i = 101 * W + 100;
            let old_albedo = Rgb::new(0.2, 0.3, 0.4);
            let old_emission = Rgb::new(0.3, 0.2, 0.1);
            f.albedo[i] = old_albedo;
            f.emissive[i] = old_emission;
            f.surfaces[i] = Surface { normal: [0.0, 0.0, 1.0], height: 0.0, owner: 0, kind };
            f.reflect_sprite(&cell, (0, 7), 100, 100, &DrawOpts::default());
            if kind == 1 {
                assert_ne!(f.albedo[i], old_albedo, "positive control: receiving water reflects");
                assert!(f.emissive[i].luma() > 0.0, "water emission is attenuated, never discarded");
            } else {
                assert_eq!(f.albedo[i], old_albedo, "stale wet foreground kind{kind}");
                assert_eq!(f.emissive[i], old_emission);
            }
            assert_eq!(f.surfaces[i].kind, kind);
        }
    }

    #[test]
    fn native_hero_reflection_keeps_head_and_does_not_change_the_live_body() {
        let sheet = crate::gfx::sheet("hero").unwrap();
        let anim = sheet.anim("idle_down").unwrap();
        let cell = sheet.cell(anim, 0);
        let anchor = sheet.anchor_of(anim);
        assert_eq!((cell.w, cell.h, anchor), (48, 48, (24, 33)), "exercise the actual padded cave actor");
        let mut reflected = Frame::new();
        reflected.begin(Rgb::BLACK, Rgb::WHITE);
        reflected.look.water_wave = 0.0;
        reflected.look.water_alpha = 1.0;
        reflected.look.water_depth_max = 24;
        reflected.look.water_tint = Rgb::WHITE;
        reflected.water_rect(0, 0, WI, HI, 1.0);
        reflected.reflect_sprite(cell, anchor, 100, 100, &DrawOpts::default());
        let top = (0..cell.h).find(|&y| (0..cell.w).any(|x| cell.at(x, y).a)).unwrap();
        let mut head = 0;
        for y in top..top + 7 {
            for x in 0..cell.w {
                let p = cell.at(x, y);
                if !p.a { continue; }
                let i = (101 + anchor.1 - y) as usize * W + (100 - anchor.0 + x) as usize;
                if reflected.albedo[i].max_c() + reflected.emissive[i].max_c() > 0.001 { head += 1; }
            }
        }
        assert!(head > 8, "the old eight-row crop cannot reflect the native head ({head})");
        // Actual animation/weapon masks, rather than an artificial all-actor buffer.
        for anim in sheet.anims.values() {
            for frame in 0..anim.frames {
                let cell = sheet.cell(anim, frame);
                let anchor = sheet.anchor_of(anim);
                for flip in [false, true] {
                    let mut wet = Frame::new();
                    let mut dry = Frame::new();
                    for f in [&mut wet, &mut dry] {
                        f.begin(Rgb::new(0.1, 0.2, 0.3), Rgb::WHITE);
                        f.look.water_wave = 0.6;
                        f.look.water_alpha = 1.0;
                        f.look.water_depth_max = 24;
                    }
                    wet.water_rect(0, 0, WI, HI, 1.0);
                    let opts = DrawOpts { actor: true, flip, ..Default::default() };
                    wet.sprite(cell, anchor, 100, 100, &opts);
                    dry.sprite(cell, anchor, 100, 100, &opts);
                    for y in 0..cell.h { for x in 0..cell.w {
                        if !cell.at(x, y).a { continue; }
                        let px = 100 - if flip { cell.w - 1 - anchor.0 } else { anchor.0 }
                            + if flip { cell.w - 1 - x } else { x };
                        let py = 100 - anchor.1 + y;
                        let i = py as usize * W + px as usize;
                        assert_eq!(wet.albedo[i], dry.albedo[i], "{} frame{frame} flip{flip} at{x},{y}", anim.name);
                        assert_eq!(wet.emissive[i], dry.emissive[i]);
                        assert!(wet.actor_pixels[i]);
                    }}
                }
            }
        }
    }

    #[test]
    fn capped_reflections_fade_into_the_receiver_and_ignore_atlas_padding() {
        let compact = Cell::from_ascii(&["a"; 12], &[('a', Rgb::WHITE)], &['a']);
        let mut padded_rows = vec!["."; 48];
        padded_rows[20..32].fill("a");
        let padded = Cell::from_ascii(&padded_rows, &[('a', Rgb::WHITE)], &['a']);
        for cap in [4, 24] {
            let mut frames = [Frame::new(), Frame::new()];
            for (f, (cell, anchor)) in frames.iter_mut().zip([(&compact, (0, 11)), (&padded, (0, 31))]) {
                f.begin(Rgb::new(0.1, 0.2, 0.3), Rgb::WHITE);
                f.look.water_wave = 0.0;
                f.look.water_alpha = 1.0;
                f.look.water_depth_max = cap;
                f.look.water_tint = Rgb::WHITE;
                // A narrow receiving channel also leaves positive dry-bank controls.
                f.water_rect(100, 101, 1, 12, 1.0);
                f.reflect_sprite(cell, anchor, 100, 100, &DrawOpts::default());
            }
            assert_eq!(frames[0].albedo, frames[1].albedo, "padding cannot change fade");
            assert_eq!(frames[0].emissive, frames[1].emissive);
            let f = &frames[0];
            let end = cap.min(12) as usize;
            assert!(f.emissive[101 * W + 100].max_c() > 0.4);
            assert!(f.emissive[(100 + end) * W + 100].max_c() < 0.1, "fade must run before a regional cap");
            for y in 101..115 { for x in 99..102 {
                let i = y * W + x;
                if x != 100 || y > 100 + end {
                    assert_eq!(f.albedo[i], Rgb::new(0.1, 0.2, 0.3));
                    assert_eq!(f.emissive[i], Rgb::BLACK);
                }
            }}
        }
    }

    #[test]
    fn reflections_mirror_the_projected_sprite_around_its_ground_plane() {
        let cell = Cell::from_ascii(&["a", "a", "a"], &[('a', Rgb::WHITE)], &[]);
        for z in [0, 8, 16] { for explicit in [false, true] {
            let mut f = Frame::new();
            f.begin(Rgb::BLACK, Rgb::WHITE);
            f.cam_y = 40;
            f.look.water_wave = 0.0;
            f.look.water_alpha = 1.0;
            f.look.water_tint = Rgb::WHITE;
            f.water_rect(0, 0, WI, HI, 1.0);
            let ground_screen = 100;
            let projected = ground_screen - z;
            f.reflect_sprite(&cell, (0, 2), 100, projected, &DrawOpts {
                actor: true, elevation: z as f32,
                ground_y: explicit.then_some((ground_screen + f.cam_y) as f32),
                ..Default::default()
            });
            let rows: Vec<_> = (0..HI).filter(|&y| f.albedo[y as usize * W + 100] != Rgb::BLACK).collect();
            assert_eq!(rows, vec![101 + z, 102 + z, 103 + z], "z{z} explicit{explicit}");
        }}
    }

}
