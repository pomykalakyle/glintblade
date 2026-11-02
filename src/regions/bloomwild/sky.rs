//! A low-contrast cloud sea: distant billows and one cool shadow under the island.

use super::pal;
use crate::gfx::{Frame, H, Rgb, W};
use crate::util::hash2;
use crate::world::Zone;

struct Cloud {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

pub struct Sky {
    clouds: Vec<Cloud>,
    panorama: bool,
}

impl Sky {
    /// Lay out distant banks with unequal, overlapping billows.
    pub fn build() -> Self {
        let clouds = (0..32)
            .map(|i| Cloud {
                x: (hash2(i, 0, 98) * 1500.0) as i32,
                y: (hash2(i, 1, 98) * 1000.0) as i32,
                w: 100 + (hash2(i, 2, 98) * 140.0) as i32,
                h: 14 + (hash2(i, 3, 98) * 14.0) as i32,
            })
            .collect();
        Self { clouds, panorama: false }
    }

    pub fn title() -> Self {
        Self { panorama: true, ..Self::build() }
    }

    /// Draw broad cloud volumes and a continuous world-anchored shelf below the falls.
    pub fn draw(&self, _z: &Zone, f: &mut Frame, t: u64) {
        f.rect(0, 0, W as i32, H as i32, pal::CLOUD[0]);
        for c in &self.clouds {
            let x = (c.x - f.cam_x / 5 + (t / 240) as i32).rem_euclid(1500) - 200;
            let y = (c.y - f.cam_y / 7).rem_euclid(1000) - 150;
            if x > W as i32 || x + c.w < 0 || y > H as i32 || y + c.h * 2 < 0 {
                continue;
            }
            cloud(f, x, y, c.w, c.h, pal::CLOUD[0], pal::CLOUD[1]);
        }
        if self.panorama { title_vistas(f); }
        for sx in 0..W as i32 {
            let x = sx + f.cam_x;
            let crest = shelf_top(x);
            for sy in (crest - f.cam_y).max(0)..H as i32 {
                let y = sy + f.cam_y;
                // The island shadow is one connected, down-right band; its lower edge
                // follows billows rather than the bottoms of the rock columns.
                let depth = y - crest;
                let shadow_end = 17 + billow(x + 43, 79, 8);
                let c = if depth < 3 {
                    pal::CLOUD[1]
                } else if depth < shadow_end {
                    pal::CLOUD[0]
                } else {
                    pal::CLOUD[1]
                };
                f.put(sx, sy, c);
            }
        }
    }
}

/// Broad receding shelves establish the journey's scale without competing with the title.
/// Their stone disappears into the cloud bank; they are distant land, never walkable decals.
fn title_vistas(f: &mut Frame) {
    let drift = f.cam_x / 6;
    let far = pal::CLOUD[0].lerp(pal::TEAL[2], 0.23);
    let near = pal::CLOUD[0].lerp(pal::TEAL[1], 0.32);
    // Two continuous tablelands extend beyond the frame. Small ruins and a far
    // falling stream give a scale reference without putting detail behind the logo.
    distant_land(f, &[(-90,125),(-20,122),(29,125),(73,117),(111,119),
        (148,110),(182,113),(217,105),(259,108),(293,98),(338,101),
        (381,93),(450,96)], drift, far, 18);
    distant_land(f, &[(-70,151),(-8,144),(31,146),(67,137),(99,140),
        (129,130),(160,133),(190,127),(224,132),(271,125),(330,133),
        (430,129)], drift/2, near, 27);
    let x = 121 - drift/2;
    // A broken aqueduct follows the shelf rather than standing as another column.
    for (dx, h) in [(0, 9), (11, 11), (24, 8), (36, 5)] {
        f.rect(x+dx,132-h,2,h+3,near);
    }
    f.rect(x,121,25,2,near);
    f.rect(x+24,123,13,2,near);
    for (dx, y, h) in [(0,139,16),(1,140,21)] {
        f.rect(76-drift/2+dx,y,1,h,near.lerp(pal::CLOUD[1],0.45));
    }
    cloud(f,-63,161,330,17,pal::CLOUD[0],pal::CLOUD[1]);
}

fn distant_land(f: &mut Frame, crest: &[(i32,i32)], drift: i32, color: Rgb, depth: i32) {
    let face = color.lerp(pal::TEAL[1],0.12);
    let light = color.lerp(pal::CLOUD[1],0.23);
    for edge in crest.windows(2) {
        let (a,b) = (edge[0],edge[1]);
        for wx in a.0..b.0 {
            let x = wx-drift;
            if !(0..W as i32).contains(&x) { continue; }
            let y = a.1+(wx-a.0)*(b.1-a.1)/(b.0-a.0);
            let ledge = 3 + ((wx+17).div_euclid(29).rem_euclid(3));
            let root = depth+billow(wx+41,87,9)-billow(wx+11,47,6);
            for dy in 0..root {
                let c = if dy < 2 { light } else if dy < ledge { color }
                    else if (wx+dy/5).rem_euclid(61)<18 { face } else { color };
                // The lower rock dissolves into a single cloud layer, leaving
                // connected escarpments instead of separate floating teeth.
                f.put(x,y+dy,if dy > root-5 { c.lerp(pal::CLOUD[0],0.45) } else { c });
            }
        }
    }
}

/// Return an asymmetric stepped billow with broad shoulders and a narrow crown.
fn billow(x: i32, width: i32, height: i32) -> i32 {
    let u = x.rem_euclid(width);
    let d = (u - width * 2 / 5).abs();
    let radius = width * 3 / 5;
    height * (radius * radius - d * d).max(0) / (radius * radius)
}

/// Keep the cloud shelf independent of every cliff-column bottom.
pub(super) fn shelf_top(x: i32) -> i32 {
    794 - billow(x + 31, 113, 9) - billow(x + 7, 29, 4)
}

/// Join overlapping lobe profiles into a volume with curved upper and lower boundaries.
fn cloud(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, shade: Rgb, top: Rgb) {
    for dx in 0..w {
        let sx = x + dx;
        if sx < 0 || sx >= W as i32 {
            continue;
        }
        let taper = (dx.min(w - 1 - dx) * h / (w / 5)).min(h);
        let upper = h - taper - billow(dx + w / 9, w / 3, h / 3);
        let lower = h + taper / 2 + billow(dx + 17, w / 2, h / 4);
        for dy in upper.max(0)..lower {
            let sy = y + dy;
            if sy >= 0 && sy < H as i32 {
                f.put(sx, sy, if dy < h + taper / 4 { top } else { shade });
            }
        }
    }
}
