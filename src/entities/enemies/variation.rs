//! Authored native-pixel skins share the original pose clock and ground anchor.
//! Appearance belongs to the spawn, so turning, damage, afterimages and death
//! keep the same identity without consuming any simulation randomness.

use crate::gfx::{Cell, sheet};
use std::collections::HashMap;
use std::sync::OnceLock;

const SPECIES: [&str; 6] = ["sproutling", "thorn_imp", "crystal_beetle", "wisp", "ember_hound", "spitting_totem"];

thread_local! {
    static REFERENCE_SKIN: std::cell::Cell<Option<u8>> = const { std::cell::Cell::new(None) };
    static REFERENCE_POSE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The retained cardinal art advances the historical simulation random stream.
/// Live debris still uses the complete facing and appearance shown on screen.
pub(super) fn reference_pose() -> bool {
    REFERENCE_POSE.with(|v| v.get())
}

pub(super) fn with_reference_pose<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) { REFERENCE_POSE.with(|v| v.set(self.0)); }
    }
    let _restore = Restore(REFERENCE_POSE.with(|v| v.replace(true)));
    f()
}

/// Original selector for retained random-stream receipts and gameplay outlets.
/// Visual selection uses the shared eight-way contract instead.
pub(super) fn legacy_dir(x: f32, y: f32) -> crate::gfx::Dir {
    use crate::gfx::Dir;
    if x.abs() > y.abs() * 1.05 {
        if x > 0.0 { Dir::Right } else { Dir::Left }
    } else if y < 0.0 {
        Dir::Up
    } else {
        Dir::Down
    }
}

/// Evaluate an effect with a reference skin. The guard also restores normal
/// rendering during unwinding; overrides never escape into a later actor.
pub(super) fn with_skin<T>(variant: u8, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<u8>);
    impl Drop for Restore {
        fn drop(&mut self) { REFERENCE_SKIN.with(|v| v.set(self.0)); }
    }
    let _restore = Restore(REFERENCE_SKIN.with(|v| v.replace(Some(variant))));
    f()
}

pub(super) fn appearance(x: f32, y: f32) -> u8 {
    let (x, y) = (x.floor() as i32, y.floor() as i32);
    match (x, y) {
        // Keep the opening pair and neighboring temple/ridge encounters
        // visibly distinct; the remaining field population uses a stable hash.
        (392, 648) | (1144, 472) | (661, 535) => 1,
        (544, 243) => 2,
        _ => (crate::util::hash2(x, y, 941) * 3.0) as u8 % 3,
    }
}

/// Resolve the same row and frame in a complete alternate atlas. References
/// point directly into the immutable registry: no per-frame pixel operations.
pub(super) fn cell(name: &str, source: &'static Cell, variant: u8) -> &'static Cell {
    let variant = REFERENCE_SKIN.with(|v| v.get()).unwrap_or(variant);
    if variant == 0 { return source; }
    static CELLS: OnceLock<HashMap<(usize, u8), &'static Cell>> = OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let mut result = HashMap::new();
        for name in SPECIES {
            let source_sheet = sheet(name).expect("enemy base atlas");
            for variant in 1..=2 {
                let alternate = sheet(&format!("{name}_v{variant}")).expect("complete enemy skin atlas");
                assert_eq!(source_sheet.anchor, alternate.anchor, "{name} skin anchor");
                for (r, row) in source_sheet.cells.iter().enumerate() {
                    for (c, source) in row.iter().enumerate() {
                        let target = &alternate.cells[r][c];
                        assert_eq!((source.w, source.h), (target.w, target.h), "{name} skin cell");
                        result.insert((source as *const Cell as usize, variant), target);
                    }
                }
            }
        }
        result
    });
    cells.get(&(source as *const Cell as usize, variant)).copied()
        .unwrap_or_else(|| panic!("missing {name} skin {variant} pose"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_skins_keep_pose_clocks_anchors_and_combat_cues() {
        for name in SPECIES {
            let original = sheet(name).expect(name);
            for variant in 1..=2 {
                let skin = sheet(&format!("{name}_v{variant}")).expect("skin");
                assert_eq!(original.anchor, skin.anchor);
                assert_eq!(original.anims.len(), skin.anims.len());
                let (mut changed, mut silhouettes, mut pixels) = (0, 0, 0);
                for (anim_name, a) in &original.anims {
                    let b = skin.anim(anim_name).expect("every animation has a skin");
                    assert_eq!((a.row, a.frames, a.anchor, &a.durations), (b.row, b.frames, b.anchor, &b.durations));
                    for key in ["hit_frame", "release_frame", "release_offset", "airborne", "maw"] {
                        assert_eq!(a.json.get(key), b.json.get(key), "{name} {anim_name} {key}");
                    }
                    for i in 0..a.frames {
                        let source = original.cell(a, i);
                        let target = cell(name, source, variant);
                        assert!(std::ptr::eq(source, cell(name, source, 0)));
                        assert!(std::ptr::eq(target, skin.cell(b, i)));
                        assert!(!target.is_empty(), "{name} {anim_name} {i} missing skin pose");
                        for (p, q) in source.px.iter().zip(&target.px) {
                            pixels += usize::from(p.a || q.a);
                            changed += usize::from((p.a, p.c, p.emit) != (q.a, q.c, q.emit));
                            silhouettes += usize::from(p.a != q.a);
                            // The wisp's whole flame is emissive; its alternate
                            // body is allowed to change around the shared core.
                            if !matches!(name, "wisp" | "crystal_beetle") && p.emit {
                                assert_eq!((q.a, q.c, q.emit), (p.a, p.c, p.emit), "{name} {anim_name} {i} combat emission");
                            }
                        }
                    }
                }
                assert!(changed * 10 > pixels, "{name} skin {variant} must redesign the body");
                assert!(silhouettes * 100 > pixels, "{name} skin {variant} must change actual outlines");
            }
        }
    }
}
