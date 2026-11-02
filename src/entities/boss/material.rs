//! One slate material for the articulated Weir Keeper.
//!
//! The torso acquired new native art while the moving parts retained the earlier
//! stone ramp and generated, rounded normals. Adapt those original stone pixels
//! at the art boundary, leaving every prism color, cue mask and pose pixel intact.
//! Plane locations follow tools/warden/{head,fist,crystal}.py, including their
//! authored lifts; albedo highlights are not interpreted as surface curvature.

use super::art::{BODY, CRYSTAL, FIST, HEAD, HEART};
use crate::gfx::surface::NormalPixel;
use crate::gfx::{Cell, rgb, sheet};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

const SLATE: [u32; 13] = [
    0x1d232e, 0x242b3a, 0x303848, 0x3a4254, 0x4a5263, 0x585f6f, 0x6c7281, 0x7a808d, 0x8a8f9b,
    0x979ba5, 0xa2a5ad, 0xb3b5bb, 0xccccce,
];

/// Exact authored stone keys only. In particular dull iris glass is not stone.
fn stone(key: u32) -> Option<u32> {
    Some(match key {
        0x1f2838 => SLATE[0],  // outline and cut seams
        0x2b3548 => SLATE[1],  // recessed wall under the head's lintel
        0x333b4c => SLATE[3],  // shadowed returns
        0x555d70 => SLATE[6],  // broad stone fronts
        0x7f8797 => SLATE[8],  // authored lit fronts and roof edges
        0xb0b7bb => SLATE[11], // narrow bevel highlights
        0xd9dacc => SLATE[12],
        _ => return None,
    })
}

fn slate_cell(cell: &Cell) -> bool {
    let mut seen = [false; SLATE.len()];
    for p in cell.px.iter().filter(|p| p.a) {
        if p.emit {
            return false;
        }
        let Some(index) = SLATE.iter().position(|c| *c == p.c.key()) else {
            return false;
        };
        seen[index] = true;
    }
    seen.into_iter().all(|present| present)
}

/// Check loaded pixels, including GLINT_SPRITES overrides. A legacy body keeps
/// its matching legacy parts; a filename or stale manifest is insufficient.
pub(super) fn uses_slate_body() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        sheet(BODY).is_some_and(|s| {
            super::pose::native_family()
                && s.anim("awake")
                    .or_else(|| s.anim("idle"))
                    .is_some_and(|a| slate_cell(s.cell(a, 0)))
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Plane {
    Front,
    Top,
    Left,
    Right,
}

impl Plane {
    fn normal(self, original: NormalPixel) -> NormalPixel {
        // Native tangent X,Z,Y, exactly as the torso's semantic world normals.
        let n = match self {
            Self::Front => [0, 0, 127],
            Self::Top => [0, 127, 0],
            Self::Left => [-102, 0, 76],
            Self::Right => [102, 0, 76],
        };
        NormalPixel { n, ..original }
    }
}

#[derive(Clone, Copy)]
struct Hand {
    top: i32,
    bottom: i32,
    half: i32,
    fingers: i32,
    cuff: i32,
    open: bool,
}

impl Hand {
    fn plane(self, x: i32, y: i32) -> Plane {
        let cuff_y = self.top + 1;
        let weight_y = cuff_y + self.cuff;
        let weight_end = if self.open {
            cuff_y + 14
        } else {
            self.bottom - 1 - self.fingers
        };
        if (cuff_y..weight_y).contains(&y) && (15..=25).contains(&x) {
            return if y == cuff_y {
                Plane::Top
            } else if x >= 23 && y < weight_y - 1 {
                Plane::Right
            } else if x == 15 {
                Plane::Left
            } else {
                Plane::Front
            };
        }
        // Thumb owns these pixels in front of the counterweight. Its highlight
        // band remains albedo; only the explicit roof and edge are new planes.
        let thumb_y = weight_y + if self.open { 3 } else { 2 };
        let thumb_end = if self.open {
            thumb_y + 4
        } else {
            weight_end + 4
        };
        let thumb_left = if self.open { 3 } else { 20 - self.half - 5 };
        if (thumb_y..=thumb_end).contains(&y) {
            let hook = if !self.open && y >= thumb_end - 2 {
                2
            } else {
                0
            };
            let cut = i32::from(!self.open && (y == thumb_y || y == thumb_end));
            let left = thumb_left + hook + cut;
            let right = thumb_left + hook + 6 - cut;
            if (left..=right).contains(&x) {
                return if y == thumb_y {
                    Plane::Top
                } else if x == left {
                    Plane::Left
                } else {
                    Plane::Front
                };
            }
        }
        if (weight_y..=weight_end).contains(&y) {
            let cut = match y - weight_y {
                0 => 2,
                1 => 1,
                _ => 0,
            };
            let left = 20 - self.half + cut;
            let right = 20 + self.half - cut;
            if (left..=right).contains(&x) {
                return if y <= weight_y + 1 {
                    Plane::Top
                } else if x >= right - 3 && y < weight_end {
                    Plane::Right
                } else if x == left {
                    Plane::Left
                } else {
                    Plane::Front
                };
            }
        }
        let finger_y = weight_end + if self.open { 1 } else { 2 };
        if (finger_y..self.bottom).contains(&y) {
            let ranges = if self.open {
                [(3, 17), (23, 37)]
            } else {
                [(20 - self.half, 19), (21, 20 + self.half)]
            };
            for (left, right) in ranges {
                if (left..=right).contains(&x) {
                    return if y == finger_y {
                        Plane::Top
                    } else if x >= right - 2 {
                        Plane::Right
                    } else if x == left {
                        Plane::Left
                    } else {
                        Plane::Front
                    };
                }
            }
        }
        Plane::Front
    }
}

#[derive(Clone, Copy)]
enum Geometry {
    Head { dx: i32, dy: i32, brow: i32 },
    Hand(Hand),
    Sill,
    Gem,
}

impl Geometry {
    fn for_frame(name: &str, anim: &str, frame: usize) -> Option<Self> {
        Some(match name {
            HEAD => {
                let (dy, dx, brow) = match anim {
                    "idle" | "flare" => (0, 0, 0),
                    "roar" => *[
                        (1, 0, 0),
                        (-2, 0, 0),
                        (-3, 0, 1),
                        (-3, -1, 1),
                        (-3, 1, 1),
                        (-1, 0, 0),
                    ]
                    .get(frame)?,
                    _ => return None,
                };
                Self::Head { dx, dy, brow }
            }
            FIST => {
                let mut h = Hand {
                    top: 27,
                    bottom: 53,
                    half: 11,
                    fingers: 9,
                    cuff: 5,
                    open: false,
                };
                match anim {
                    "clenched" | "open" | "open_lit" => {
                        let bob = *[0, -1, -1, 0].get(frame)?;
                        h.open = anim != "clenched";
                        h.top += bob - i32::from(h.open);
                        h.bottom += bob;
                    }
                    "slam" => match frame {
                        0 => {
                            h.top = 18;
                            h.bottom = 44;
                        }
                        1 => {
                            h.top = 1;
                            h.bottom = 30;
                        }
                        2 => {
                            h.top = 19;
                            h.bottom = 50;
                            h.half = 10;
                        }
                        3 => {
                            h.top = 33;
                            h.half = 13;
                            h.fingers = 7;
                            h.cuff = 3;
                        }
                        4 => (),
                        _ => return None,
                    },
                    _ => return None,
                }
                Self::Hand(h)
            }
            CRYSTAL => Self::Sill,
            HEART => Self::Gem,
            _ => return None,
        })
    }

    fn plane(self, x: i32, y: i32) -> Plane {
        match self {
            Self::Head { dx, dy, brow } => {
                let (x, y) = (x - dx, y - dy);
                let roof = 13 - brow;
                if (roof..=roof + 8).contains(&y) {
                    if y <= roof + 1 {
                        Plane::Top
                    } else if x >= 36 && y < roof + 8 {
                        Plane::Right
                    } else if x == 3 {
                        Plane::Left
                    } else {
                        Plane::Front
                    }
                } else if (22..=32).contains(&y) && x >= 33 {
                    Plane::Right
                } else if (24..=27).contains(&y) && x == 32 {
                    Plane::Left // inner right jamb faces back into the socket
                } else if (31..=40).contains(&y) {
                    let right = [32, 31, 31, 30, 30, 29, 29, 28, 28, 27][(y - 31) as usize];
                    if x >= right - 5 {
                        Plane::Right
                    } else {
                        Plane::Front
                    }
                } else {
                    Plane::Front
                }
            }
            Self::Hand(h) => h.plane(x, y),
            Self::Sill => {
                if y == 17 && (3..=15).contains(&x) {
                    Plane::Top
                } else if (18..=21).contains(&y) && (13..=15).contains(&x) {
                    Plane::Right
                } else if (18..=20).contains(&y) && x == 3 {
                    Plane::Left
                } else {
                    Plane::Front
                }
            }
            Self::Gem => Plane::Front,
        }
    }
}

fn adapt(source: &Cell, geometry: Geometry) -> Cell {
    let mut result = source.clone();
    let surface = Arc::make_mut(&mut result.surface);
    for (i, (pixel, normal)) in result.px.iter_mut().zip(surface).enumerate() {
        if pixel.a
            && !pixel.emit
            && let Some(color) = stone(pixel.c.key())
        {
            pixel.c = rgb(color);
            *normal = geometry
                .plane(i as i32 % source.w, i as i32 / source.w)
                .normal(*normal);
        }
    }
    result
}

/// The art, shadow caster, destruction and debris paths all receive the same
/// immutable Cell. The cache has one entry per real animation cell, never per
/// tick or pose lookup, and the global sheets own the pointer keys permanently.
pub(super) fn part(name: &str, source: &'static Cell) -> &'static Cell {
    if !matches!(name, HEAD | FIST | CRYSTAL | HEART) || !uses_slate_body() {
        return source;
    }
    static CELLS: OnceLock<HashMap<usize, Cell>> = OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let mut cells = HashMap::new();
        for (name, size) in [
            (HEAD, (44, 44)),
            (FIST, (40, 56)),
            (CRYSTAL, (20, 24)),
            (HEART, (24, 32)),
        ] {
            let Some(s) = sheet(name).filter(|s| (s.cell_w, s.cell_h) == size) else {
                continue;
            };
            for a in s.anims.values() {
                for i in 0..a.frames {
                    let source = s.cell(a, i);
                    if !source.is_empty()
                        && let Some(geometry) = Geometry::for_frame(name, &a.name, i)
                    {
                        cells
                            .entry(source as *const Cell as usize)
                            .or_insert_with(|| adapt(source, geometry));
                    }
                }
            }
        }
        cells
    });
    cells
        .get(&(source as *const Cell as usize))
        .unwrap_or(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_parts_keep_every_contour_prism_and_cue_pixel() {
        assert!(
            uses_slate_body(),
            "regression requires the current native slate body"
        );
        let mut stone_pixels = 0;
        let mut prism_pixels = 0;
        for name in [HEAD, FIST, CRYSTAL, HEART] {
            let s = sheet(name).unwrap();
            for a in s.anims.values() {
                for i in 0..a.frames {
                    let original = s.cell(a, i);
                    let processed = part(name, original);
                    assert!(
                        std::ptr::eq(processed, part(name, original)),
                        "stable cache"
                    );
                    assert_eq!((processed.w, processed.h), (original.w, original.h));
                    for (index, (before, after)) in
                        original.px.iter().zip(&processed.px).enumerate()
                    {
                        assert_eq!(
                            (before.a, before.emit),
                            (after.a, after.emit),
                            "{name}/{}:{i}:{index}",
                            a.name
                        );
                        let bn = original.surface[index];
                        let an = processed.surface[index];
                        assert_eq!((bn.a, bn.emit), (an.a, an.emit));
                        if before.a && !before.emit && stone(before.c.key()).is_some() {
                            stone_pixels += 1;
                            assert!(SLATE.contains(&after.c.key()));
                            assert_eq!(
                                an.n[1],
                                if an.n == [0, 127, 0] { 127 } else { 0 },
                                "stone fronts do not bend up or down"
                            );
                        } else if before.a && !before.emit && SLATE.contains(&before.c.key()) {
                            // New native parts already carry authored slate and
                            // planar normals; the legacy adapter must be inert.
                            stone_pixels += 1;
                            assert_eq!(after.c, before.c);
                            assert_eq!(an.n, bn.n);
                        } else {
                            assert_eq!(
                                after.c, before.c,
                                "all prism shades survive, including dull glass"
                            );
                            assert_eq!(an.n, bn.n);
                            prism_pixels += usize::from(before.a);
                        }
                    }
                }
            }
        }
        assert!(stone_pixels > 10_000 && prism_pixels > 2_000);
    }

    fn normal(name: &str, anim: &str, frame: usize, x: i32, y: i32) -> [i8; 3] {
        let s = sheet(name).unwrap();
        let source = s.cell(s.anim(anim).unwrap(), frame);
        assert!(source.at(x, y).a && stone(source.at(x, y).c.key()).is_some());
        part(name, source).surface[(y * source.w + x) as usize].n
    }

    #[test]
    fn native_faces_share_torso_planes_through_roar_and_slam() {
        assert_eq!(normal(FIST, "clenched", 0, 17, 37), [0, 0, 127]);
        assert_eq!(normal(FIST, "slam", 1, 17, 11), [0, 0, 127]);
        assert_eq!(normal(FIST, "clenched", 0, 18, 28), [0, 127, 0]);
        assert_eq!(normal(FIST, "slam", 1, 18, 2), [0, 127, 0]);
        assert_eq!(normal(FIST, "clenched", 0, 7, 35), [0, 127, 0]);
        assert_eq!(normal(FIST, "clenched", 0, 11, 35), [0, 0, 127]);
        assert_eq!(normal(FIST, "slam", 3, 9, 39), [0, 0, 127]);
        assert_eq!(normal(CRYSTAL, "intact", 0, 7, 17), [0, 127, 0]);
        assert_eq!(normal(CRYSTAL, "intact", 0, 7, 19), [0, 0, 127]);
        assert_eq!(normal(CRYSTAL, "intact", 0, 14, 19), [102, 0, 76]);
    }

    #[test]
    fn legacy_head_adapter_keeps_its_original_native_planes() {
        // A pure legacy-sized cell exercises the old geometry independently
        // of whichever native head family is currently installed.
        let source = Cell::new(
            44,
            44,
            vec![
                crate::gfx::Px {
                    c: rgb(0x555d70),
                    a: true,
                    emit: false,
                };
                44 * 44
            ],
        );
        let idle = adapt(&source, Geometry::for_frame(HEAD, "idle", 1).unwrap());
        let roar = adapt(&source, Geometry::for_frame(HEAD, "roar", 3).unwrap());
        let n = |cell: &Cell, x, y| cell.surface[(y * cell.w + x) as usize].n;
        assert_eq!(n(&idle, 20, 17), [0, 0, 127]);
        assert_eq!(n(&idle, 20, 13), [0, 127, 0]);
        assert_eq!(n(&idle, 38, 17), [102, 0, 76]);
        assert_eq!(n(&roar, 19, 13), [0, 0, 127]);
        assert_eq!(n(&roar, 19, 9), [0, 127, 0]);
    }

    #[test]
    fn authored_integrated_head_keeps_native_material_and_normals() {
        let head = sheet(HEAD).unwrap();
        if !super::super::pose::rig().integrated_head {
            assert_eq!((head.cell_w, head.cell_h), (44, 44));
            return;
        }
        assert_ne!((head.cell_w, head.cell_h), (44, 44));
        let mut native_stone = 0;
        for anim in head.anims.values() {
            for frame in 0..anim.frames {
                let source = head.cell(anim, frame);
                assert!(std::ptr::eq(part(HEAD, source), source));
                for (p, n) in source.px.iter().zip(source.surface.iter()) {
                    assert_eq!((p.a, p.emit), (n.a, n.emit));
                    if p.a && !p.emit && SLATE.contains(&p.c.key()) {
                        native_stone += 1;
                    }
                    assert!(
                        !p.a || p.emit || stone(p.c.key()).is_none(),
                        "integrated head uses its authored slate family"
                    );
                }
            }
        }
        assert!(native_stone > 0);
    }

    #[test]
    fn compatibility_uses_pixels_and_leaves_legacy_or_partial_palettes_alone() {
        let current = Cell::new(
            13,
            1,
            SLATE
                .iter()
                .map(|c| crate::gfx::Px {
                    c: rgb(*c),
                    a: true,
                    emit: false,
                })
                .collect(),
        );
        assert!(slate_cell(&current));
        let mut legacy = current.clone();
        legacy.px[0].c = rgb(0x1f2838);
        assert!(!slate_cell(&legacy));
        let partial = Cell::from_ascii(&["a"], &[('a', rgb(SLATE[0]))], &[]);
        assert!(!slate_cell(&partial));
    }
}
