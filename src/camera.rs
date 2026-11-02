//! Smooth follow camera with look-ahead and trauma-based screen shake.

use crate::gfx::{H, W};
use crate::util::noise2;

pub struct Camera {
    /// Center of view in world pixels (smoothed).
    pub x: f32,
    pub y: f32,
    look_x: f32,
    look_y: f32,
    /// Optional native view interval for a tightly fitting assembly/hero/weapon composition.
    /// Applied after shake, which loses amplitude at the boundary rather than cropping art.
    view_y_limits: Option<(i32, i32)>,
}

impl Camera {
    pub fn new(x: f32, y: f32) -> Camera {
        Camera { x, y, look_x: 0.0, look_y: 0.0, view_y_limits: None }
    }

    pub fn snap(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.look_x = 0.0;
        self.look_y = 0.0;
        self.view_y_limits = None;
    }

    /// Follow a target with a little look-ahead in the direction it's moving/facing.
    pub fn follow(&mut self, tx: f32, ty: f32, dir: (f32, f32), speed: f32) {
        self.view_y_limits = None;
        let k = (speed / 1.5).clamp(0.0, 1.0);
        self.look_x += (dir.0 * 26.0 * k - self.look_x) * 0.04;
        self.look_y += (dir.1 * 16.0 * k - self.look_y) * 0.04;
        self.x += (tx + self.look_x - self.x) * 0.12;
        self.y += (ty - 8.0 + self.look_y - self.y) * 0.12;
    }

    /// Keep a subject's visible anchor clear of the HUD and screen edges, including shake.
    pub fn keep_visible(&mut self, x: f32, y: f32) {
        // Leave room for Glint's head above the anchor and the boss bar below it.
        self.x = self.x.clamp(x - W as f32 / 2.0 + 32.0, x + W as f32 / 2.0 - 32.0);
        self.y = self.y.clamp(y - H as f32 / 2.0 + 32.0, y + H as f32 / 2.0 - 48.0);
    }

    /// Use the same weapon-safe view during approach, idle, charge, fire and retreat.
    /// Far follow centers Glint, so this lower bound naturally stops affecting the view.
    pub fn prepare_weapon_margin(&mut self, player_y: f32) {
        self.y = self.y.max(player_y - H as f32 / 2.0 + 38.0);
        // Keep the authored blade above row196 even when shake points toward the HUD.
        self.view_y_limits = Some((player_y.floor() as i32 - H as i32 + 38,
            player_y.floor() as i32 - 48));
    }

    /// During a beam warning/sweep, prioritize its real emitter and Glint over the whole giant.
    /// Native art never zooms. If both cannot fit, favor the source as far as Glint's safe
    /// rectangle allows; the boss draws a directional edge cue for every remaining emitter.
    pub fn frame_attack_sources(&mut self, player: (f32, f32), sources: impl IntoIterator<Item = (f32, f32)>) {
        let (px, py) = player;
        // The downward sword extends below the feet; its edge must clear the boss bar too.
        let (mut x0, mut x1, mut y0, mut y1) = (px - 8.0, px + 8.0, py - 18.0, py + 16.0);
        let mut any = false;
        for (x, y) in sources {
            any = true;
            x0 = x0.min(x - 10.0); x1 = x1.max(x + 10.0);
            y0 = y0.min(y - 10.0); y1 = y1.max(y + 10.0);
        }
        if !any { return; }
        let (hw, hh) = (W as f32 / 2.0, H as f32 / 2.0);
        let left = x1 - (W as f32 - 16.0) + hw;
        let right = x0 - 16.0 + hw;
        let top = y1 - (H as f32 - 22.0) + hh;
        let bottom = y0 - 24.0 + hh;
        // At most six pixels per tick avoids snapping when an emitter starts charging.
        let target_x = if left <= right { self.x.clamp(left, right) } else { (x0 + x1) * 0.5 };
        let target_y = if top <= bottom { self.y.clamp(top, bottom) } else { bottom };
        self.x += (target_x - self.x).clamp(-6.0, 6.0);
        self.y += (target_y - self.y).clamp(-6.0, 6.0);
        self.keep_visible(px, py);
        self.y = self.y.max(py - hh + 38.0);
        if let Some((lo, hi)) = self.view_y_limits { self.y = self.y.clamp(lo as f32 + hh, hi as f32 + hh); }
    }

    /// Retain a fitting crown/core and Glint's weapon together in the usable native view.
    pub fn keep_fitting_assembly(&mut self, top: f32, player_y: f32) {
        let min_view = player_y.floor() as i32 + 16 - (H as i32 - 22);
        let max_view = (top.floor() as i32 - 10).min(player_y.floor() as i32 - 48);
        if min_view <= max_view {
            self.view_y_limits = Some((min_view, max_view));
            let hh = H as f32 / 2.0;
            self.y = self.y.clamp(min_view as f32 + hh, max_view as f32 + hh);
        }
    }

    /// Top-left screen position for this frame (integer pixels), with shake, clamped to the zone.
    pub fn view(&self, zone_w: i32, zone_h: i32, trauma: f32, t: u64) -> (i32, i32) {
        let s = trauma * trauma;
        let tt = t as f32 * 0.35;
        let ox = (noise2(tt, 0.5, 11) * 2.0 - 1.0) * 7.0 * s;
        let oy = (noise2(0.5, tt, 23) * 2.0 - 1.0) * 5.0 * s;
        let (mut cx, mut cy) = self.view_free();
        cx = if zone_w <= W as i32 { (zone_w - W as i32) / 2 } else { cx.clamp(0, zone_w - W as i32) };
        cy = if zone_h <= H as i32 { (zone_h - H as i32) / 2 } else { cy.clamp(0, zone_h - H as i32) };
        let mut vy = cy + oy.round() as i32;
        if let Some((lo, hi)) = self.view_y_limits { vy = vy.clamp(lo, hi); }
        (cx + ox.round() as i32, vy)
    }

    /// Rounded top-left without zone bounds or shake, for the title postcard.
    pub fn view_free(&self) -> (i32, i32) {
        ((self.x - W as f32 / 2.0).round() as i32, (self.y - H as f32 / 2.0).round() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Actual emitter coordinates, rather than the giant's crown, own the active-attack view.
    #[test]
    fn attack_camera_fits_sources_when_possible_and_keeps_hero_when_not() {
        for sources in [vec![(448.0, 240.0)], vec![(448.0, 240.0), (512.0, 240.0)]] {
            for py in [300.0, 355.0, 452.0] {
                let mut cam = Camera::new(480.0, py - 8.0);
                for _ in 0..90 { cam.frame_attack_sources((480.0, py), sources.iter().copied()); }
                let (cx, cy) = cam.view(960, 640, 0.0, 0);
                let hero_y = py - cy as f32;
                assert!((48.0..=178.0).contains(&hero_y), "hero and downward sword must remain above the boss HUD: {hero_y}");
                assert!(hero_y + 16.0 <= 194.0);
                if py <= 355.0 {
                    for (x, y) in &sources {
                        assert!((24.0..=360.0).contains(&(x - cx as f32)));
                        assert!((32.0..=174.0).contains(&(y - cy as f32)), "fitting source clipped: {y} - {cy}");
                    }
                } else {
                    assert!(sources[0].1 - (cy as f32) < 24.0, "native source cannot fit; it requires an edge cue");
                }
            }
        }
    }

    /// A full 168px crown-to-feet span still fits with the blade above the HUD at row196.
    /// Shake must be clipped at the composition boundary, not violate one of its subjects.
    #[test]
    fn tightly_fitting_crown_hero_and_weapon_survive_shake() {
        for gap in [151.0, 160.0, 168.0] {
            let (top, py) = (234.0, 234.0 + gap);
            let mut cam = Camera::new(720.0, py - 8.0);
            cam.frame_attack_sources((720.0, py), [(688.0, 270.0), (752.0, 270.0)]);
            cam.keep_fitting_assembly(top, py);
            for trauma in [0.0, 0.4, 0.8, 1.0] { for t in 0..180 {
                let (_, cy) = cam.view(1440, 960, trauma, t);
                assert!(top - cy as f32 >= 10.0, "crown cropped at gap {gap}, trauma {trauma}");
                assert!(py + 16.0 - cy as f32 <= 194.0, "weapon enters HUD at gap {gap}, trauma {trauma}");
            }}
            // Retreat/follow must release the tight interval on its very next update.
            cam.follow(720.0, 700.0, (0.0, 1.0), 1.5);
            assert!(cam.view_y_limits.is_none());
            cam.snap(720.0, 700.0);
            assert!(cam.view_y_limits.is_none());
        }
    }

    /// Already visible subjects do not move when an emitter begins charging.
    #[test]
    fn prepared_assembly_does_not_snap_at_beam_onset() {
        for gap in [151.0, 160.0, 168.0] {
            let (top, py) = (234.0, 234.0 + gap);
            let mut cam = Camera::new(720.0, py - 70.0);
            cam.prepare_weapon_margin(py);
            cam.keep_fitting_assembly(top, py);
            let before = cam.view(1440, 960, 0.0, 2522);
            cam.frame_attack_sources((720.0, py), [(688.0, 270.0), (752.0, 270.0)]);
            assert_eq!(cam.view(1440, 960, 0.0, 2523), before, "charging changed an already usable native view");
        }
    }

    /// Partial approach/retreat distances must use exactly the same idle and charge reserve.
    #[test]
    fn distant_charge_onset_preserves_prepared_weapon_view() {
        for py in [400.0, 572.0, 632.0] {
            let mut cam = Camera::new(720.0, py - 70.0);
            cam.prepare_weapon_margin(py);
            let before = cam.view(1440, 960, 0.0, 0);
            cam.frame_attack_sources((720.0, py), [(688.0, 240.0)]);
            assert_eq!(cam.view(1440, 960, 0.0, 0), before);
            assert!(py + 16.0 - before.1 as f32 <= 194.0);
        }
    }

    /// Match the original clamp-before-shake math at edges, half pixels and narrow zones.
    #[test]
    fn bounded_camera_matches_original_view() {
        for (zone_w, zone_h) in [(1, 1), (383, 215), (384, 216), (385, 217), (1344, 1048)] {
            for x in [-500.5, -0.5, 191.5, 192.0, 192.5, 712.0, 1344.5] {
                for y in [-500.5, -0.5, 70.0, 107.5, 108.0, 108.5, 1048.5] {
                    let cam = Camera::new(x, y);
                    assert_eq!(cam.view_free(), ((x - W as f32 / 2.0).round() as i32, (y - H as f32 / 2.0).round() as i32));
                    for trauma in [0.0, 0.25, 0.8, 1.0] {
                        for t in [0, 1, 180, 600, 4488] {
                            let s = trauma * trauma;
                            let tt = t as f32 * 0.35;
                            let ox = (noise2(tt, 0.5, 11) * 2.0 - 1.0) * 7.0 * s;
                            let oy = (noise2(0.5, tt, 23) * 2.0 - 1.0) * 5.0 * s;
                            let mut cx = (cam.x - W as f32 / 2.0).round() as i32;
                            let mut cy = (cam.y - H as f32 / 2.0).round() as i32;
                            cx = if zone_w <= W as i32 { (zone_w - W as i32) / 2 } else { cx.clamp(0, zone_w - W as i32) };
                            cy = if zone_h <= H as i32 { (zone_h - H as i32) / 2 } else { cy.clamp(0, zone_h - H as i32) };
                            assert_eq!(cam.view(zone_w, zone_h, trauma, t), (cx + ox.round() as i32, cy + oy.round() as i32));
                        }
                    }
                }
            }
        }
    }
}
