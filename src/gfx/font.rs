//! A tiny built-in 3x5 pixel font for debug text and fallback UI.
//! Text is drawn into `frame.out` (after grading), so it is never affected by lighting.

use super::frame::{Frame, Rgb};

fn glyph(c: char) -> Option<[u8; 5]> {
    let c = c.to_ascii_uppercase();
    Some(match c {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b110],
        '.' => [0, 0, 0, 0, 0b010],
        ',' => [0, 0, 0, 0b010, 0b100],
        '!' => [0b010, 0b010, 0b010, 0, 0b010],
        '?' => [0b110, 0b001, 0b010, 0, 0b010],
        ':' => [0, 0b010, 0, 0b010, 0],
        '-' => [0, 0, 0b111, 0, 0],
        '\'' => [0b010, 0b010, 0, 0, 0],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '+' => [0, 0b010, 0b111, 0b010, 0],
        '(' => [0b001, 0b010, 0b010, 0b010, 0b001],
        ')' => [0b100, 0b010, 0b010, 0b010, 0b100],
        '%' => [0b101, 0b001, 0b010, 0b100, 0b101],
        '=' => [0, 0b111, 0, 0b111, 0],
        '>' => [0b100, 0b010, 0b001, 0b010, 0b100],
        '<' => [0b001, 0b010, 0b100, 0b010, 0b001],
        ' ' => [0, 0, 0, 0, 0],
        _ => return None,
    })
}

/// Width in pixels of `s` at `scale`.
pub fn text_width(s: &str, scale: i32) -> i32 {
    let n = s.chars().count() as i32;
    if n == 0 { 0 } else { (n * 4 - 1) * scale }
}

/// Draw text into the UI layer. `glow` > 0 also feeds bloom.
pub fn text(frame: &mut Frame, x: i32, y: i32, s: &str, color: Rgb, scale: i32, glow: f32) {
    let mut cx = x;
    for ch in s.chars() {
        if let Some(g) = glyph(ch) {
            for (row, bits) in g.iter().enumerate() {
                for col in 0..3 {
                    if bits & (0b100 >> col) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cx + col * scale + sx;
                                let py = y + row as i32 * scale + sy;
                                if glow > 0.0 { frame.ui_glow(px, py, color, glow) } else { frame.ui_put(px, py, color) }
                            }
                        }
                    }
                }
            }
        }
        cx += 4 * scale;
    }
}

/// Text with a 1px dark drop shadow, for readability over the game.
pub fn text_shadow(frame: &mut Frame, x: i32, y: i32, s: &str, color: Rgb, shadow: Rgb, scale: i32) {
    text(frame, x + scale.max(1), y + scale.max(1), s, shadow, scale, 0.0);
    text(frame, x, y, s, color, scale, 0.0);
}

/// Centered horizontally on `cx`.
pub fn text_center(frame: &mut Frame, cx: i32, y: i32, s: &str, color: Rgb, scale: i32, glow: f32) {
    text(frame, cx - text_width(s, scale) / 2, y, s, color, scale, glow);
}
