//! Authored planting plan, independent of terrain albedo and its native relief.
//!
//! Lobes describe connected garden beds, not a noise mask over map tiles. Dense
//! centers belong to walls, piers and groves; the spaces between them belong to
//! Glint. Grass and the native placement diagram read this same plan.

pub(super) type Oval = (i32, i32, i32, i32);

pub(super) const BEDS: &[(&str, &[Oval])] = &[
    ("landing tree", &[(91, 703, 20, 13), (81, 686, 13, 16)]),
    ("aqueduct west toe", &[(239, 716, 24, 13)]),
    ("aqueduct east toe", &[(438, 715, 28, 14)]),
    ("west colonnade", &[(128, 208, 33, 29), (137, 241, 37, 30), (169, 254, 27, 18)]),
    ("west meadow", &[(167, 386, 33, 19), (201, 402, 47, 26), (241, 416, 37, 19)]),
    ("courtyard wall", &[(379, 450, 43, 23), (426, 453, 43, 28), (479, 456, 47, 22), (520, 445, 33, 19)]),
    ("lower encounter", &[(466, 626, 42, 23), (511, 641, 48, 31), (550, 659, 37, 30), (520, 684, 38, 19)]),
    ("east wall", &[(963, 642, 34, 26), (996, 668, 42, 25), (1034, 654, 43, 25), (1013, 630, 33, 18)]),
    ("temple east", &[(1203, 415, 26, 29), (1214, 445, 27, 27), (1195, 400, 20, 20)]),
    ("grove root shoulders", &[(858, 182, 33, 26), (900, 173, 46, 29), (944, 195, 37, 28), (926, 224, 32, 20)]),
    ("northeast grove", &[(1176, 112, 31, 26), (1202, 135, 33, 29), (1193, 159, 25, 21)]),
];

/// The first three retain the established meadow clearing. Their overlaps form
/// a connected arena rather than a set of randomly missing grass tiles.
pub(super) const CLEARINGS: &[(&str, Oval)] = &[
    ("meadow entry", (400, 652, 38, 26)),
    ("meadow turn", (410, 693, 42, 30)),
    ("meadow exit", (461, 684, 36, 28)),
    ("courtyard arena", (447, 485, 67, 23)),
    ("grove passage", (877, 220, 24, 21)),
    ("altar approach", (408, 268, 142, 28)),
    ("aqueduct opening", (335, 707, 57, 18)),
];

fn ellipse(x: i32, y: i32, (cx, cy, rx, ry): Oval) -> f32 {
    let (dx, dy) = ((x - cx) as f32 / rx as f32, (y - cy) as f32 / ry as f32);
    dx * dx + dy * dy
}

pub(super) fn clearing(x: i32, y: i32) -> bool {
    CLEARINGS.iter().any(|&(_, oval)| ellipse(x, y, oval) < 1.0)
}

/// Opening combat needs the whole moving native fan outside its clearing, not
/// just the small root. Test the closest point of that fan's bounds against
/// each meadow ellipse; other beds keep their existing planting margins.
fn meadow_fan_clearance(x: i32, y: i32) -> bool {
    CLEARINGS[..3].iter().any(|&(_, oval @ (cx, cy, _, _))| {
        ellipse(cx.clamp(x - 17, x + 16), cy.clamp(y - 13, y + 1), oval) < 1.0
    })
}

/// Full cover in a bed's interior, a narrow irregular native-leaf margin at its
/// edge. The placement hash jitters individual roots only; it never prunes tiles.
pub(super) fn cover(x: i32, y: i32) -> f32 {
    if clearing(x, y) || meadow_fan_clearance(x, y) { return 0.0; }
    BEDS.iter().flat_map(|&(_, lobes)| lobes).fold(0.0_f32, |cover, &oval| {
        cover.max(((1.0 - ellipse(x, y, oval)) * 3.0).clamp(0.0, 1.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planted_beds_keep_named_focal_spaces_open() {
        for (x, y) in [(400, 652), (410, 693), (461, 684), (447, 485), (877, 220), (408, 268), (335, 707)] {
            assert_eq!(cover(x, y), 0.0, "authored clearing at {x},{y}");
        }
        // The existing legal native-grass reveal probe stays inside a real bed.
        assert_eq!(cover(500, 637), 1.0);
        assert_eq!(cover(1202, 135), 1.0);
        assert_eq!(cover(700, 470), 0.0, "long quiet route field");
        assert_eq!(cover(408, 176), 0.0, "altar is the focal space");
    }
}
