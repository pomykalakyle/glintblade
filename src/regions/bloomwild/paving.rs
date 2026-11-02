//! One laid limestone bond shared by the floor painter and its lighting planes.
//! Courses run through atlas, map-tile and camera boundaries without restarting.

use crate::gfx::Rgb;
use crate::util::hash2;
use super::pal;

#[derive(Clone, Copy, Debug)]
pub(super) struct Stone {
    pub id: u32,
    pub mortar: bool,
    pub normal: [f32; 3],
    pub color: Rgb,
}

#[derive(Clone, Copy)]
struct Flag { x: i32, y: i32, w: i32, h: i32, id: u32 }

/// Pack real rectangles once across an area larger than the entire island.
/// Unlike independent texture patches, every cell refers to its complete flag.
fn layout() -> &'static [Flag] {
    static PLAN: std::sync::OnceLock<Vec<Flag>> = std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        const W: usize = 128;
        const H: usize = 144;
        let empty = Flag { x: 0, y: 0, w: 0, h: 0, id: 0 };
        let mut plan = vec![empty; W * H];
        let shapes = [(4, 3), (3, 4), (3, 3), (4, 2), (2, 3), (3, 2), (2, 2)];
        for row in 0..H {
            for col in 0..W {
                if plan[row * W + col].w != 0 { continue; }
                let start = (hash2(col as i32, row as i32, 9510) * shapes.len() as f32) as usize;
                let (w, h) = (0..shapes.len()).map(|i| shapes[(start + i) % shapes.len()])
                    .chain([(2, 1), (1, 1)])
                    .enumerate()
                    .filter(|&(_, (w, h))| col + w <= W && row + h <= H
                        && (row..row + h).all(|y| (col..col + w).all(|x| plan[y * W + x].w == 0)))
                    .min_by_key(|&(rank, (w, h))| {
                        // Prefer a flag that crosses the joint above it. This
                        // prevents a stack of fillers becoming a tall panel.
                        let edge = col + w;
                        let run = if edge < W {
                            (1..=row.min(8)).take_while(|&dy|
                                plan[(row - dy) * W + edge - 1].id != plan[(row - dy) * W + edge].id).count()
                        } else { 0 };
                        let panel = if run + h > 6 { 100 } else { run * 3 };
                        (panel + usize::from(w == 1 || h == 1) * 24, rank)
                    })
                    .map(|(_, shape)| shape)
                    .unwrap();
                let flag = Flag { x: col as i32 * 16, y: row as i32 * 8,
                    w: w as i32 * 16, h: h as i32 * 8,
                    id: hash2(col as i32, row as i32, 9513).to_bits() };
                for y in row..row + h { for x in col..col + w { plan[y * W + x] = flag; }}
            }
        }
        plan
    })
}

/// Unequal, foreshortened flags in a mixed rectangular bond. Small chips widen
/// an existing seam rather than cutting a stone into islands. The large layout
/// period is beyond every view of Bloomwild, including its full-map overview.
pub(super) fn sample(x: i32, y: i32) -> Stone {
    let (xx, yy) = ((x + 128).rem_euclid(2048), (y + 128).rem_euclid(1152));
    let flag = layout()[(yy / 8 * 128 + xx / 16) as usize];
    let (lx, ly, w, h) = (xx - flag.x, yy - flag.y, flag.w, flag.h);
    let (col, row) = (flag.x / 16, flag.y / 8);
    let id = flag.id;
    let wear = hash2(col, row, 9514);
    let chip = wear > 0.72 && lx > 0 && ly > 0 && lx + ly <= 3
        || wear < 0.18 && lx == w - 1 && ly == h - 1;
    let mortar = lx == 0 || ly == 0 || chip;
    let normal = if mortar { [0.0, 0.0, 1.0] } else {
        [(i32::from(lx == w - 1) - i32::from(lx == 1)) as f32 * 0.16,
         (i32::from(ly == h - 1) - i32::from(ly == 1)) as f32 * 0.16, 1.0]
    };
    // Neighboring stones vary within one limestone ramp. Quiet interiors leave
    // the shadows and the altar legible; only attached edges carry a bevel.
    let tone = hash2(col, row, 9515);
    let base = pal::STONE[3].lerp(pal::STONE[4], 0.08 + tone * 0.24);
    let color = if mortar {
        pal::STONE[2].lerp(pal::STONE[3], 0.20).lerp(pal::EARTH[2], 0.18)
    } else if ly == 1 || lx == 1 {
        base.lerp(pal::STONE[4], 0.10)
    } else if ly == h - 1 || lx == w - 1 {
        base.lerp(pal::STONE[2], 0.10)
    } else {
        // A few short mineral traces lie within a flag, never across grout.
        let trace_y = 3 + (hash2(col, row, 9516) * (h - 6) as f32) as i32;
        let trace_x = 7 + (wear * (w - 15) as f32) as i32;
        if tone > 0.58 && ly == trace_y && (trace_x..trace_x + 4).contains(&lx) {
            base.lerp(pal::STONE[2], 0.13)
        } else { base }
    };
    Stone { id, mortar, normal, color }
}

/// The final tapered bridge aprons replace turf as well as its lighting plane.
pub(super) fn bridge_approach(x: i32, y: i32) -> bool {
    [(656, 256), (784, 576)].into_iter().any(|(bx, by)| {
        let d = if x < bx { bx - x - 1 } else { x - bx - 112 };
        let ly = y - by;
        (0..9).contains(&d) && (3 + d / 3..29 - d / 3).contains(&ly)
            && (d < 3 || hash2(x / 3, y / 4, 736) > d as f32 / 11.0)
    })
}

/// Vertical joints change bond exactly on the bridge's horizontal mortar.
pub(super) fn bridge_joint(x: i32, y: i32) -> i32 {
    let course = if y < 12 { 0 } else if y < 22 { 1 } else { 2 };
    (x + course * 17).rem_euclid(35)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laid_flags_have_connected_joints_and_low_horizontal_planes() {
        let mut vertical = 0;
        for y in -68..340 {
            for x in -128..1408 {
                let s = sample(x, y);
                assert!(s.normal[2] > 0.9);
                if sample(x - 1, y).id != s.id || sample(x, y - 1).id != s.id {
                    assert!(s.mortar, "stone boundary has an open joint at {x},{y}");
                }
                if s.mortar && !sample(x - 1, y).mortar && !sample(x + 1, y).mortar {
                    vertical += 1;
                    assert!(sample(x, y - 1).mortar || sample(x, y + 1).mortar,
                        "isolated seam at {x},{y}");
                }
            }
        }
        assert!(vertical > 10_000);
    }

    #[test]
    fn bridge_bonds_end_on_horizontal_seams_without_two_pixel_stubs() {
        for (top, bottom) in [(3, 11), (13, 21), (23, 28)] {
            for x in 0..112 {
                for y in top..=bottom {
                    assert_eq!(bridge_joint(x, y), bridge_joint(x, top),
                        "vertical joint changed inside a bridge course at {x},{y}");
                }
            }
        }
        assert_ne!(bridge_joint(0, 11), bridge_joint(0, 12));
        assert_ne!(bridge_joint(0, 21), bridge_joint(0, 22));
    }
}
