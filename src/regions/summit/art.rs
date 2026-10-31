//! Silent native-grid fallbacks for the Summit's flat slotted waterworks.
//! Live Summit sheets supply the normal art; only the small burner emits light.

use super::{ItemKind, pal::*};
use crate::gfx::{Frame, Rgb};

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
fn fin(f: &mut Frame, x: i32, y: i32, h: i32, v: u8) {
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
pub fn draw_brazier(f: &mut Frame, sx: i32, sy: i32, t: u64, seed: u32) {
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
pub fn draw_banner(f: &mut Frame, sx: i32, sy: i32, _t: u64, _seed: u32, flip: bool) {
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
pub fn draw_post(f: &mut Frame, sx: i32, sy: i32, _seed: u32) {
    f.rect(sx - 3, sy - 12, 6, 12, F1);
    f.rect(sx - 4, sy - 14, 8, 3, F3);
    f.rect(sx - 5, sy - 1, 10, 2, F1);
}

/// Existing perimeter anchors carry tall fins with distinct broken heights.
pub fn draw_pillar(f: &mut Frame, sx: i32, sy: i32, v: u8) {
    fin(f, sx, sy, [140, 101, 127][v as usize % 3], v);
}

/// Smaller slotted remnants frame the arrival without changing its footpath.
pub fn draw_monolith(f: &mut Frame, sx: i32, sy: i32, v: u8) {
    fin(f, sx, sy, [92, 66][v as usize % 2], v);
}
