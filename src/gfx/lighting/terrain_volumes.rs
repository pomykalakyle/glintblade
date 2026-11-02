//! Exact finite rays through painter-authored terrain masses. Native height
//! sheets remain sheets; collision grids never manufacture shadow geometry.

use crate::world::TerrainVolume;

const EPS: f64 = 0.000_01;

/// Receiver coordinates use projected Y; direction uses world XYZ. `end` is
/// infinity for sunlight and one for the segment to a finite point emitter.
pub(super) fn blocked(volume: TerrainVolume, receiver: [f64; 3], direction: [f64; 3],
    end: f64, own_terrain: bool) -> bool {
    let origin = [receiver[0], receiver[1] + receiver[2], receiver[2]];
    let min = volume.min.map(f64::from);
    let max = volume.max.map(f64::from);
    // Painted front/cap pixels belong to their own mass. Their normal supplies
    // facing; a ray must still test every other mass beside that surface.
    if own_terrain && (0..3).all(|axis|
        origin[axis] >= min[axis] - EPS && origin[axis] <= max[axis] + EPS) { return false; }
    let mut near = EPS;
    let mut far = end - EPS;
    for axis in 0..3 {
        let d = direction[axis];
        if d.abs() < 1.0e-12 {
            if origin[axis] < min[axis] || origin[axis] >= max[axis] { return false; }
        } else {
            let a = (min[axis] - origin[axis]) / d;
            let b = (max[axis] - origin[axis]) / d;
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near >= far { return false; }
        }
    }
    near < far
}

pub(super) fn any(volumes: &[TerrainVolume], receiver: [f64; 3], direction: [f64; 3],
    end: f64, own_terrain: bool) -> bool {
    volumes.iter().any(|&volume| blocked(volume, receiver, direction, end, own_terrain))
}
