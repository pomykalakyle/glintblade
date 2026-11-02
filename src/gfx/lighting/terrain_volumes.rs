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

/// Exact solid masses, indexed once when the lighting field is built. Region
/// painters can retain small silhouette rectangles without testing every rock
/// rectangle for every receiving pixel and every local light.
pub(super) struct Index {
    volumes: Vec<TerrainVolume>,
    nodes: Vec<Node>,
}

struct Node {
    bounds: TerrainVolume,
    range: std::ops::Range<usize>,
    children: Option<[usize; 2]>,
}

impl Index {
    pub(super) fn new(volumes: &[TerrainVolume]) -> Self {
        let mut index = Self { volumes: volumes.to_vec(), nodes: Vec::new() };
        if volumes.len() > 4 { index.build(0..volumes.len()); }
        index
    }

    fn build(&mut self, range: std::ops::Range<usize>) -> usize {
        let mut bounds = self.volumes[range.start];
        for volume in &self.volumes[range.clone()] {
            for axis in 0..3 {
                bounds.min[axis] = bounds.min[axis].min(volume.min[axis]);
                bounds.max[axis] = bounds.max[axis].max(volume.max[axis]);
            }
        }
        let node = self.nodes.len();
        self.nodes.push(Node { bounds, range: range.clone(), children: None });
        if range.len() > 4 {
            let axis = (0..3).max_by(|&a, &b|
                (bounds.max[a] - bounds.min[a]).total_cmp(&(bounds.max[b] - bounds.min[b]))).unwrap();
            self.volumes[range.clone()].sort_unstable_by(|a, b|
                (a.min[axis] + a.max[axis]).total_cmp(&(b.min[axis] + b.max[axis])));
            let middle = range.start + range.len() / 2;
            let left = self.build(range.start..middle);
            let right = self.build(middle..range.end);
            self.nodes[node].children = Some([left, right]);
        }
        node
    }

    pub(super) fn any(&self, receiver: [f64; 3], direction: [f64; 3], end: f64, own: bool) -> bool {
        if self.nodes.is_empty() { return any(&self.volumes, receiver, direction, end, own); }
        self.visit(0, receiver, direction, end, own)
    }

    fn visit(&self, node: usize, receiver: [f64; 3], direction: [f64; 3], end: f64, own: bool) -> bool {
        let node = &self.nodes[node];
        // Broad bounds never inherit the self-surface exemption: a receiver
        // inside a group may still be shadowed by another mass in that group.
        if !blocked(node.bounds, receiver, direction, end, false) { return false; }
        if let Some([left, right]) = node.children {
            self.visit(left, receiver, direction, end, own)
                || self.visit(right, receiver, direction, end, own)
        } else {
            any(&self.volumes[node.range.clone()], receiver, direction, end, own)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_solids_match_finite_and_sun_rays_including_self_surfaces_and_gaps() {
        let volumes: Vec<_> = (0..64).map(|i| {
            let x = (i % 8 * 13) as f32;
            let y = (i / 8 * 17) as f32;
            TerrainVolume { min: [x, y, 0.0], max: [x + 7.0, y + 9.0, 4.0 + (i % 5) as f32 * 6.0] }
        }).collect();
        let index = Index::new(&volumes);
        let mut random = 7341u32;
        let mut next = || { random = random.wrapping_mul(1664525).wrapping_add(1013904223); random };
        for _ in 0..6000 {
            let receiver = [(next() % 140) as f64 - 10.0, (next() % 140) as f64 - 10.0,
                (next() % 40) as f64];
            let direction = [(next() % 280) as f64 - 140.0, (next() % 280) as f64 - 140.0,
                (next() % 100) as f64 - 50.0];
            for end in [1.0, f64::INFINITY] { for own in [false, true] {
                assert_eq!(index.any(receiver, direction, end, own),
                    any(&volumes, receiver, direction, end, own),
                    "receiver={receiver:?}, direction={direction:?}, end={end}, own={own}");
            }}
        }
        assert!(!index.any([9.0, -10.0, 2.0], [0.0, 200.0, 0.0], 1.0, false),
            "the open two-pixel gap remains open, even inside the aggregate bounds");
        assert!(index.any([3.0, 3.0, 4.0], [20.0, 0.0, 0.0], 1.0, true),
            "exempting one's own cap must not exempt the neighboring taller stone");
    }
}
