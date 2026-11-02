//! Silent native-grid fallbacks for the Summit's flat slotted waterworks.
//! Live Summit sheets supply the normal art; only the small burner emits light.

use super::{ItemKind, pal::*};
use crate::gfx::{Frame, Rgb};
use crate::gfx::sprite::SpriteFocus;

/// A conduit frame around the socket recalls the garden's abandoned machinery.
/// Native independent feet stand outside the fighting lane; one visible prop
/// supplies normals and shadows without an invisible shadowing proxy.
pub fn place_waterworks_frame(z: &mut crate::world::Zone) {
    if crate::gfx::sheet("round13_landmarks_summit").is_some_and(|s| s.has("body")) {
        z.props.push(crate::world::Prop::new(
            "round13_landmarks_summit", "body", 720.0, 294.0));
    }
}

/// The fallback follows the same local, world-anchored reveal as native props.
/// Ground shadows remain physical; only solid scenery pixels become porous.
struct Ink<'a> {
    frame: &'a mut Frame,
    reveal: Option<SpriteFocus>,
    depth: f32,
}

impl Ink<'_> {
    fn visible(&self, x: i32, y: i32) -> bool {
        let (wx, wy) = (x + self.frame.cam_x, y + self.frame.cam_y);
        self.reveal.is_none_or(|focus| focus.allows_scenery_at_depth(wx, wy, self.depth))
    }

    fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if self.visible(x, y) && let Some(i) = Frame::idx(x, y) {
            self.frame.emissive[i] = Rgb::BLACK;
            self.frame.put(x, y, c);
        }
    }

    fn put_glow(&mut self, x: i32, y: i32, c: Rgb) {
        if self.visible(x, y) { self.frame.put_glow(x, y, c); }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        for py in y.max(0)..(y + h).min(crate::gfx::H as i32) {
            for px in x.max(0)..(x + w).min(crate::gfx::W as i32) { self.put(px, py, c); }
        }
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
        Frame::line_with(x0, y0, x1, y1, |x, y| self.put(x, y, c));
    }

    fn shadow(&mut self, x: i32, y: i32, rx: i32, ry: i32, opacity: f32) {
        self.frame.shadow(x, y, rx, ry, opacity);
    }
}

pub fn draw_item(f: &mut Frame, sx: i32, sy: i32, kind: ItemKind, t: u64, seed: u32, reveal: Option<SpriteFocus>) {
    let depth = (sy + f.cam_y) as f32;
    let f = &mut Ink { frame: f, reveal, depth };
    match kind {
        ItemKind::Brazier => draw_brazier(f, sx, sy, t, seed),
        ItemKind::Banner(flip) => draw_banner(f, sx, sy, flip),
        ItemKind::Post => draw_post(f, sx, sy),
        ItemKind::Pillar(v) => draw_pillar(f, sx, sy, v),
        ItemKind::Monolith(v) => draw_monolith(f, sx, sy, v),
    }
}

/// Approximate dimensions above each unchanged world anchor.
pub fn item_bounds(k: ItemKind) -> (f32, f32) {
    match k {
        ItemKind::Brazier => (24.0, 36.0),
        ItemKind::Banner(_) => (32.0, 59.0),
        ItemKind::Post => (16.0, 22.0),
        ItemKind::Pillar(_) => (64.0, 155.0),
        ItemKind::Monolith(_) => (40.0, 107.0),
    }
}

/// A broad fin with a genuine open slot and an upper-left-facing stone plane.
fn fin(f: &mut Ink<'_>, x: i32, y: i32, h: i32, v: u8) {
    f.shadow(x + 3, y + 1, 15, 3, 0.35);
    let slot_bottom = y - if v == 1 { 19 } else { 31 };
    for yy in y - h + 4..y - 5 {
        for xx in x - 12..x + 12 {
            if (x - 4..=x + 2).contains(&xx) && (y - h + 16..=slot_bottom).contains(&yy) {
                continue;
            }
            let c = if xx >= x + 7 {
                F1
            } else if xx == x - 12 {
                F4
            } else {
                F3
            };
            f.put(xx, yy, c);
        }
    }
    f.line(x - 7, y - h, x + 6, y - h + 1, F4);
    f.line(x - 4, slot_bottom + 1, x + 2, slot_bottom + 1, F4);
    f.line(x - 11, y - 24, x + 6, y - 24, F2);
    f.rect(x - 16, y - 6, 32, 3, F3);
    f.rect(x - 16, y - 3, 32, 4, F1);
}

/// A small source-bound flame on a shallow stone bowl.
fn draw_brazier(f: &mut Ink<'_>, sx: i32, sy: i32, t: u64, seed: u32) {
    f.shadow(sx + 2, sy + 1, 7, 2, 0.35);
    f.rect(sx - 3, sy - 13, 5, 12, F1);
    f.rect(sx - 3, sy - 13, 1, 11, F2);
    f.rect(sx - 7, sy - 18, 14, 3, F0);
    f.line(sx - 7, sy - 18, sx + 6, sy - 18, G1);
    f.rect(sx - 6, sy - 2, 12, 3, F2);
    let lean = ((t / 8 + seed as u64) % 3) as i32 - 1;
    f.line(sx, sy - 20, sx + lean, sy - 26, G2);
    for y in sy - 21..=sy - 19 {
        for x in sx - 1..=sx + 1 {
            f.put_glow(x, y, FIRE);
        }
    }
}

/// Still torn slate cloth with a small worn seam and no gold emblem.
fn draw_banner(f: &mut Ink<'_>, sx: i32, sy: i32, flip: bool) {
    let dir = if flip { -1 } else { 1 };
    f.line(sx, sy, sx, sy - 48, F2);
    f.line(sx - dir, sy - 45, sx + 15 * dir, sy - 45, F2);
    for x in 1i32..=13 {
        let h = 21 + [0, 2, 5, 1][(x / 4) as usize];
        for y in 0..h {
            let c: Rgb = if x < 4 { F1 } else { S1 };
            f.put(sx + x * dir, sy - 44 + y, c);
        }
    }
}

/// A plain low distributor-wall post at the bridge edge.
fn draw_post(f: &mut Ink<'_>, sx: i32, sy: i32) {
    f.rect(sx - 3, sy - 12, 6, 12, F1);
    f.rect(sx - 4, sy - 14, 8, 3, F3);
    f.rect(sx - 5, sy - 1, 10, 2, F1);
}

/// Existing perimeter anchors carry tall fins with distinct broken heights.
fn draw_pillar(f: &mut Ink<'_>, sx: i32, sy: i32, v: u8) {
    fin(f, sx, sy, [140, 101, 127][v as usize % 3], v);
}

/// Smaller slotted remnants frame the arrival without changing its footpath.
fn draw_monolith(f: &mut Ink<'_>, sx: i32, sy: i32, v: u8) {
    fin(f, sx, sy, [92, 66][v as usize % 2], v);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{DrawOpts, W, sheet};

    #[test]
    fn opaque_fallback_ink_hides_glow_while_reveal_holes_preserve_it() {
        let hero = sheet("hero").unwrap();
        let idle = hero.anim("idle_down").unwrap();
        let focus = SpriteFocus { cell: hero.cell(idle, 0), anchor: hero.anchor_of(idle),
            flip: false, x: 120.0, y: 169.0, ground_y: 169.0 };
        let (underneath, glow, stone) = (Rgb::gray(0.1), Rgb::new(0.3, 0.7, 0.9), F3);
        for reveal in [None, Some(focus)] {
            let mut f = Frame::new();
            f.begin(underneath, Rgb::WHITE);
            for y in 140..176 { for x in 84..156 {
                f.put_glow(x, y, glow);
            } }
            Ink { frame: &mut f, reveal, depth: 170.0 }.rect(84, 140, 72, 36, stone);
            let (mut covered, mut open) = (0, 0);
            for y in 140..176 { for x in 84..156 {
                let i = Frame::idx(x, y).unwrap();
                if reveal.is_none_or(|focus| focus.allows_scenery_at_depth(x, y, 170.0)) {
                    assert_eq!(f.albedo[i], stone);
                    assert_eq!(f.emissive[i], Rgb::BLACK, "opaque stone must hide underlying blade glow");
                    covered += 1;
                } else {
                    assert_eq!(f.albedo[i], Rgb::BLACK);
                    assert_eq!(f.emissive[i], glow, "a reveal hole must preserve the scene beneath");
                    open += 1;
                }
            } }
            assert!(covered > 0, "the fixture must exercise accepted scenery pixels");
            if reveal.is_some() {
                assert!(open > 0, "the fixture must exercise reveal holes");
            }
        }
    }

    #[test]
    fn fallback_foreground_reveals_hero_context_and_preserves_distant_art() {
        let hero = sheet("hero").unwrap();
        let idle = hero.anim("idle_down").unwrap();
        let cell = hero.cell(idle, 0);
        let anchor = hero.anchor_of(idle);
        let kinds = [ItemKind::Brazier, ItemKind::Banner(false), ItemKind::Banner(true),
            ItemKind::Post, ItemKind::Pillar(0), ItemKind::Pillar(1), ItemKind::Pillar(2),
            ItemKind::Monolith(0), ItemKind::Monolith(1)];

        for kind in kinds { for flip in [false, true] { for elevation in [0, 6] {
            // Changing the camera phase exercises a world-anchored dither, while
            // the context area follows both the rendered pose and its ground position.
            for camera in [(0, 0), (13, 9)] {
                let focus = SpriteFocus { cell, anchor, flip, x: 120.0,
                    y: (169 - elevation) as f32, ground_y: 169.0 };
                let hero_frame = || {
                    let mut f = Frame::new();
                    (f.cam_x, f.cam_y) = camera;
                    f.begin(Rgb::gray(0.05), Rgb::WHITE);
                    let (x, y) = (f.sx(focus.x), f.sy(focus.y));
                    f.sprite(cell, anchor, x, y, &DrawOpts { actor: true, flip,
                        elevation: elevation as f32, ground_y: Some(focus.ground_y),
                        ..Default::default() });
                    f
                };
                let before = hero_frame();
                let mut opaque = hero_frame();
                let mut revealed = hero_frame();
                let (sx, sy) = (opaque.sx(120.0), opaque.sy(170.0));
                draw_item(&mut opaque, sx, sy, kind, 24, 5, None);
                draw_item(&mut revealed, sx, sy, kind, 24, 5, Some(focus));

                let (mut hidden, mut restored, mut outside_art, mut context_restored) = (0, 0, 0, 0);
                for i in 0..before.albedo.len() {
                    let (wx, wy) = (i as i32 % W as i32 + camera.0,
                        i as i32 / W as i32 + camera.1);
                    if !focus.covers(wx, wy) {
                        assert_eq!(revealed.albedo[i], opaque.albedo[i], "outside context area: {kind:?}");
                        assert_eq!(revealed.emissive[i], opaque.emissive[i]);
                        assert_eq!(revealed.actor_pixels[i], opaque.actor_pixels[i]);
                        outside_art += usize::from(opaque.albedo[i] != before.albedo[i]);
                    }
                    if before.actor_pixels[i] && !opaque.actor_pixels[i] {
                        hidden += 1;
                        if revealed.actor_pixels[i] {
                            restored += 1;
                            assert_eq!(revealed.albedo[i], before.albedo[i]);
                            assert_eq!(revealed.emissive[i], before.emissive[i], "preserve the glowing blade");
                        }
                    }
                    if !focus.allows_scenery_at_depth(wx, wy, 170.0) {
                        assert_eq!(revealed.albedo[i], before.albedo[i], "world-anchored reveal: {kind:?}");
                        assert_eq!(revealed.emissive[i], before.emissive[i]);
                    }
                    if !before.actor_pixels[i] && revealed.albedo[i] != opaque.albedo[i] {
                        context_restored += 1;
                    }
                }
                assert!(hidden > 0 && restored > 0, "actual hero must be revealed through {kind:?}, flip {flip}, elevation {elevation}");
                assert!(context_restored > 0, "surrounding space must be revealed through {kind:?}");
                if matches!(kind, ItemKind::Pillar(_) | ItemKind::Monolith(_)) {
                    assert!(outside_art > 0, "distant portions of {kind:?} must remain opaque");
                }
                assert_eq!(revealed.shade, opaque.shade, "reveal must preserve physical shadows");
            }
        } } }
    }
}
