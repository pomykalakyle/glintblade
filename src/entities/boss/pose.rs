//! Native sliding sluice leaves within one persistent stone shell. This is a
//! visual pose only; the heart, attack clocks, anchors and collision stay in Boss.

use super::art::{self, Spr};
use crate::gfx::{Cell, Sheet, sheet};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Native joints belong to the loaded sculpture, including preview overrides.
/// All rectangles and the pier's bottom are exclusive, like the exporter's crops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rig {
    pub(super) gate: [i32; 4],
    pub(super) pier_top: i32,
    pub(super) pier_bottom: i32,
    pub(super) center: i32,
    pub(super) apron: [i32; 4],
    pub(super) fixed_upper_rows: i32,
    pub(super) integrated_head: bool,
    pub(super) articulated_legs: bool,
    pub(super) joint_top: i32,
    pub(super) sole_top: i32,
    pub(super) kneel_depth: i32,
    pub(super) fold_max: i32,
}

impl Default for Rig {
    fn default() -> Self {
        Self {
            gate: [40, 36, 80, 65],
            pier_top: 81,
            pier_bottom: 110,
            center: 60,
            apron: [49, 66, 70, 81],
            fixed_upper_rows: 30,
            integrated_head: false,
            articulated_legs: false,
            joint_top: 81,
            sole_top: 81,
            kneel_depth: 16,
            fold_max: 16,
        }
    }
}

impl Rig {
    /// An unmarked original sheet retains its old joints. Other families must
    /// explicitly describe valid native joints before the runtime can pose them.
    pub(super) fn for_sheet(s: &Sheet) -> Option<Self> {
        Self::from_json(&s.json, s.cell_w, s.cell_h)
    }

    fn from_json(json: &Value, w: i32, h: i32) -> Option<Self> {
        let Some(v) = json.get("sculpture") else {
            return ((w, h) == (120, 112)).then(Self::default);
        };
        if v.get("schema")?.as_u64()? != 2 {
            return None;
        }
        let int = |name: &str| i32::try_from(v.get(name)?.as_i64()?).ok();
        let rect = |name: &str| -> Option<[i32; 4]> {
            let a = v.get(name)?.as_array()?;
            if a.len() != 4 {
                return None;
            }
            Some([
                i32::try_from(a[0].as_i64()?).ok()?,
                i32::try_from(a[1].as_i64()?).ok()?,
                i32::try_from(a[2].as_i64()?).ok()?,
                i32::try_from(a[3].as_i64()?).ok()?,
            ])
        };
        let articulated_legs = v
            .get("articulated_legs")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let rig = Self {
            gate: rect("gate")?,
            pier_top: int("pier_top")?,
            pier_bottom: int("pier_bottom")?,
            center: int("center")?,
            apron: rect("apron")?,
            fixed_upper_rows: int("fixed_upper_rows")?,
            integrated_head: v.get("integrated_head")?.as_bool()?,
            articulated_legs,
            joint_top: if articulated_legs {
                int("joint_top")?
            } else {
                int("pier_top")?
            },
            sole_top: if articulated_legs {
                int("sole_top")?
            } else {
                int("pier_top")?
            },
            kneel_depth: if articulated_legs {
                int("kneel_depth")?
            } else {
                16
            },
            fold_max: if articulated_legs {
                int("fold_max")?
            } else {
                16
            },
        };
        rig.fits(w, h).then_some(rig)
    }

    fn fits(self, w: i32, h: i32) -> bool {
        let rect_fits = |[x0, y0, x1, y1]: [i32; 4]| {
            0 <= x0 && x0 < x1 && x1 <= w && 0 <= y0 && y0 < y1 && y1 <= h
        };
        rect_fits(self.gate)
            && rect_fits(self.apron)
            && self.gate[0] < self.center
            && self.center < self.gate[2]
            && self.center - self.gate[0] == self.gate[2] - self.center
            && 0 <= self.fixed_upper_rows
            && self.fixed_upper_rows <= self.gate[1]
            && self.gate[3] <= self.pier_top
            && self.apron[3] == self.pier_top
            && self.apron[0] < self.center
            && self.center < self.apron[2]
            && self.pier_top < self.pier_bottom
            && self.pier_bottom <= h
            && 0 < self.kneel_depth
            && self.kneel_depth <= self.fold_max
            && self.fold_max < self.joint_top
            && self.gate[3] <= self.joint_top
            && self.joint_top <= self.sole_top
            && self.sole_top < self.pier_bottom
    }

    fn travel(self) -> i32 {
        self.center - self.gate[0]
    }

    /// Inverse of the exporter's rigid pier fold. The center gap is never
    /// allowed to borrow pixels from the opposite foreground pier.
    pub(super) fn pier_source(self, x: i32, y: i32, depth: i32) -> Option<(i32, i32)> {
        let source_y = y + depth;
        let top = if self.articulated_legs {
            self.sole_top
        } else {
            self.pier_top
        };
        if !(top..self.pier_bottom).contains(&source_y) {
            return None;
        }
        let spread = if self.articulated_legs { 0 } else { depth / 8 };
        let source_x = if x < self.center - spread {
            x + spread
        } else if x >= self.center + spread {
            x - spread
        } else {
            return None;
        };
        Some((source_x, source_y))
    }
}

fn loaded_rig() -> &'static Option<Rig> {
    static RIG: OnceLock<Option<Rig>> = OnceLock::new();
    RIG.get_or_init(|| sheet(art::BODY).and_then(Rig::for_sheet))
}

pub(super) fn native_family() -> bool {
    loaded_rig().is_some()
}

pub(super) fn rig() -> &'static Rig {
    static LEGACY: OnceLock<Rig> = OnceLock::new();
    loaded_rig()
        .as_ref()
        .unwrap_or_else(|| LEGACY.get_or_init(Rig::default))
}

pub fn step(out: f32) -> i32 {
    (out.clamp(0.0, 1.0) * rig().travel() as f32).round() as i32
}

/// Native source provenance for fractures on the retracting leaves. Fixed
/// jambs, lintel, buttresses and foot joints never move with the gate.
pub fn transport(x: i32, y: i32, out: f32) -> Option<(i32, i32)> {
    let r = rig();
    let [left, top, right, bottom] = r.gate;
    if !(left..right).contains(&x) || !(top..bottom).contains(&y) {
        return Some((x, y));
    }
    let dx = if x < r.center { -step(out) } else { step(out) };
    let dest = x + dx;
    (left..right).contains(&dest).then_some((dest, y))
}

/// Cache only encountered native poses. Existing folded cells remain the
/// source of all fixed geometry, alpha, normals and ground contact.
pub fn sluice(source: Spr, damaged: bool, out: f32) -> Spr {
    let travel = step(out);
    let depth = (-art::bottom(source)).clamp(0, rig().fold_max);
    if (travel == 0 && depth == 0) || !super::material::uses_slate_body() {
        return source;
    }
    let Some(chamber) = art::spr_frame(
        art::BODY,
        &[if damaged {
            "damaged_opening"
        } else {
            "opening"
        }],
        0,
    ) else {
        return source;
    };
    let r = rig();
    if !r.fits(source.cell.w, source.cell.h)
        || (source.cell.w, source.cell.h) != (chamber.cell.w, chamber.cell.h)
    {
        return source;
    }
    let standing = art::spr_frame(
        art::BODY,
        &[if damaged {
            "damaged_fold"
        } else {
            "awake_fold"
        }],
        0,
    )
    .unwrap_or(source);
    if (standing.cell.w, standing.cell.h) != (source.cell.w, source.cell.h) {
        return source;
    }
    static POSES: OnceLock<Mutex<HashMap<(usize, i32), &'static Cell>>> = OnceLock::new();
    let mut poses = POSES.get_or_init(Default::default).lock().unwrap();
    let cell = *poses
        .entry((source.cell as *const Cell as usize, travel))
        .or_insert_with(|| {
            let mut cell = source.cell.clone();
            let normals = std::sync::Arc::make_mut(&mut cell.surface);
            let [left, top, right, bottom] = r.gate;
            for y in top..bottom {
                for x in left..right {
                    let i = (y * cell.w + x) as usize;
                    // Retreat behind the jambs, never over their faces. All pixels
                    // come from registered native art, with matching native normals.
                    cell.px[i] = chamber.cell.px[i];
                    normals[i] = chamber.cell.surface[i];
                    let source_x = if x < r.center { x + travel } else { x - travel };
                    let same_leaf = if x < r.center {
                        source_x < r.center
                    } else {
                        source_x >= r.center
                    };
                    if same_leaf {
                        let j = (y * source.cell.w + source_x) as usize;
                        cell.px[i] = source.cell.px[j];
                        normals[i] = source.cell.surface[j];
                    }
                    // A chamber is a dark recessed wall, not a cut through the
                    // creature. Its source contour must stay identical at all steps.
                    cell.px[i].a = source.cell.px[i].a;
                    normals[i].a = cell.px[i].a;
                    normals[i].emit = cell.px[i].emit;
                }
            }
            // The central apron telescopes behind the torso as the knees rise.
            // Retaining its hanging tab produces an isolated bright T and a third
            // apparent foot. Tuck one native row per depth, without erasing either
            // foreground pier where it overlaps the apron.
            if depth > 0 && !r.articulated_legs {
                let [left, top, right, bottom] = r.apron;
                for y in (bottom - depth).max(top)..bottom {
                    for x in left..right {
                        let covered = r
                            .pier_source(x, y, depth)
                            .is_some_and(|(x, y)| standing.cell.at(x, y).a);
                        if !covered {
                            let i = (y * cell.w + x) as usize;
                            cell.px[i] = Default::default();
                            normals[i].a = false;
                            normals[i].emit = false;
                        }
                    }
                }
            }
            Box::leak(Box::new(cell))
        });
    Spr { cell, ..source }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn towering_manifest() -> Value {
        serde_json::json!({"sculpture": {
            "schema": 2, "gate": [52,67,92,96],
            "pier_top": 129, "pier_bottom": 158, "center": 72,
            "apron": [61,114,82,129], "fixed_upper_rows": 46,
            "integrated_head": true
        }})
    }

    #[test]
    fn sculpture_joints_follow_the_loaded_native_family_and_reject_bad_bounds() {
        assert_eq!(Rig::from_json(&Value::Null, 120, 112), Some(Rig::default()));
        assert_eq!(Rig::from_json(&Value::Null, 144, 160), None);
        let current = towering_manifest();
        let r = Rig::from_json(&current, 144, 160).unwrap();
        assert_eq!(r.gate, [52, 67, 92, 96]);
        assert_eq!(r.apron, [61, 114, 82, 129]);
        assert_eq!((r.pier_top, r.pier_bottom, r.center), (129, 158, 72));
        assert_eq!(r.fixed_upper_rows, 46);
        assert!(r.integrated_head);
        assert_eq!(Rig::from_json(&current, 120, 112), None);
        for (field, value) in [
            ("schema", serde_json::json!(3)),
            ("gate", serde_json::json!([-1, 67, 92, 96])),
            ("gate", serde_json::json!([52, 67, 145, 96])),
            ("gate", serde_json::json!([52, 96, 92, 67])),
            ("gate", serde_json::json!([52, 67, 92])),
            ("gate", serde_json::json!([52, 67, 92, 96, 0])),
            ("center", serde_json::json!(71)),
            ("pier_top", serde_json::json!(158)),
            ("pier_bottom", serde_json::json!(161)),
            ("apron", serde_json::json!([61, 114, 145, 129])),
            ("fixed_upper_rows", serde_json::json!(68)),
            ("integrated_head", Value::Null),
        ] {
            let mut invalid = current.clone();
            invalid["sculpture"][field] = value;
            assert_eq!(Rig::from_json(&invalid, 144, 160), None, "{field}");
        }
    }

    #[test]
    fn opening_keeps_complete_masonry_and_registered_feet() {
        let r = rig();
        let [left, top, right, bottom] = r.gate;
        let [apron_left, apron_top, apron_right, apron_bottom] = r.apron;
        for damaged in [false, true] {
            for depth in 0..=r.fold_max as usize {
                let base = art::spr_frame(
                    art::BODY,
                    &[if damaged {
                        "damaged_fold"
                    } else {
                        "awake_fold"
                    }],
                    depth,
                )
                .unwrap();
                for travel in 0..=r.travel() {
                    let s = sluice(base, damaged, travel as f32 / r.travel() as f32);
                    assert_eq!(s.anchor, base.anchor);
                    assert_eq!(art::bottom(s) + depth as i32, 0);
                    for y in 0..base.cell.h {
                        for x in 0..base.cell.w {
                            let i = (y * base.cell.w + x) as usize;
                            let apron = !r.articulated_legs
                                && depth > 0
                                && (apron_left..apron_right).contains(&x)
                                && ((apron_bottom - depth as i32).max(apron_top)..apron_bottom)
                                    .contains(&y);
                            if !apron {
                                assert_eq!(s.cell.px[i].a, base.cell.px[i].a);
                            } else {
                                assert!(!s.cell.px[i].a || base.cell.px[i].a);
                            }
                            assert_eq!(s.cell.px[i].emit, base.cell.px[i].emit);
                            assert_eq!(s.cell.surface[i].a, s.cell.px[i].a);
                            if !apron
                                && (!(left..right).contains(&x) || !(top..bottom).contains(&y))
                            {
                                assert_eq!(s.cell.px[i].c, base.cell.px[i].c);
                                assert_eq!(s.cell.surface[i].n, base.cell.surface[i].n);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn folded_apron_tucks_without_cutting_either_foreground_pier() {
        let r = rig();
        if r.articulated_legs {
            for damaged in [false, true] {
                let key = if damaged {
                    "damaged_fold"
                } else {
                    "awake_fold"
                };
                let standing = art::spr_frame(art::BODY, &[key], 0).unwrap();
                for depth in 0..=r.fold_max {
                    let base = art::spr_frame(art::BODY, &[key], depth as usize).unwrap();
                    for out in [0.0, 0.5, 1.0] {
                        let posed = sluice(base, damaged, out);
                        for y in r.sole_top..r.pier_bottom {
                            for x in 0..standing.cell.w {
                                let a = standing.cell.at(x, y);
                                let b = posed.cell.at(x, y - depth);
                                assert_eq!(
                                    (a.a, a.c),
                                    (b.a, b.c),
                                    "fixed sole {x},{y} depth{depth}"
                                );
                                let ai = (y * standing.cell.w + x) as usize;
                                let bi = ((y - depth) * posed.cell.w + x) as usize;
                                let (a, b) = (standing.cell.surface[ai], posed.cell.surface[bi]);
                                assert_eq!((a.n, a.a, a.emit), (b.n, b.a, b.emit));
                            }
                        }
                    }
                }
            }
            return;
        }
        let [left, top, right, bottom] = r.apron;
        for damaged in [false, true] {
            let standing = art::spr_frame(
                art::BODY,
                &[if damaged {
                    "damaged_fold"
                } else {
                    "awake_fold"
                }],
                0,
            )
            .unwrap();
            let base = art::spr_frame(
                art::BODY,
                &[if damaged {
                    "damaged_fold"
                } else {
                    "awake_fold"
                }],
                16,
            )
            .unwrap();
            let folded = sluice(base, damaged, 0.0);
            let mut removed = 0;
            for y in (bottom - 16).max(top)..bottom {
                for x in left..right {
                    let covered = r
                        .pier_source(x, y, 16)
                        .is_some_and(|(x, y)| standing.cell.at(x, y).a);
                    if !covered {
                        removed += usize::from(base.cell.at(x, y).a);
                        assert!(!folded.cell.at(x, y).a, "no hanging apron at ({x},{y})");
                    }
                }
            }
            assert!(removed > 0, "the native apron actually tucks");
            let mut piers = [0, 0];
            for y in r.pier_top..r.pier_bottom {
                for x in 0..standing.cell.w {
                    let p = standing.cell.at(x, y);
                    if !p.a {
                        continue;
                    }
                    let dx = x + if x < r.center { -2 } else { 2 };
                    if !(0..folded.cell.w).contains(&dx) {
                        continue;
                    }
                    let dy = y - 16;
                    let index = (dy * folded.cell.w + dx) as usize;
                    let original = (y * standing.cell.w + x) as usize;
                    assert!(
                        folded.cell.px[index].a,
                        "complete native pier at ({dx},{dy})"
                    );
                    assert_eq!(folded.cell.px[index].c, p.c);
                    assert_eq!(
                        folded.cell.surface[index].n,
                        standing.cell.surface[original].n
                    );
                    piers[usize::from(x >= r.center)] += 1;
                }
            }
            assert!(piers.into_iter().all(|n| n > 0));
            for y in 0..r.fixed_upper_rows {
                for x in 0..standing.cell.w {
                    let (a, b) = (standing.cell.at(x, y), folded.cell.at(x, y));
                    assert_eq!((a.a, a.c), (b.a, b.c), "fixed upper joint at ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn cracked_leaves_travel_into_their_jambs_in_both_directions() {
        let r = rig();
        let [left, top, right, bottom] = r.gate;
        let y = (top + bottom) / 2;
        let travel = step(0.25);
        assert_eq!(transport(left + 10, y, 0.25), Some((left + 10 - travel, y)));
        assert_eq!(
            transport(right - 11, y, 0.25),
            Some((right - 11 + travel, y))
        );
        assert_eq!(transport(left, y, 0.25), None);
        assert_eq!(transport(right - 1, y, 0.25), None);
        assert_eq!(transport(left - 1, y, 1.0), Some((left - 1, y)));
        assert_eq!(transport(r.center, bottom, 1.0), Some((r.center, bottom)));
    }
}
