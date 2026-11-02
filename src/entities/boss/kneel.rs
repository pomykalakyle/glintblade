//! Shared load, contact and recovery beats for the vulnerable kneel.

pub(super) const CONTACT: u32 = 15;
// Fists also advance on the stagger-trigger update. An airborne hand therefore
// reaches the floor three stagger updates before the body's load arrives.
pub(super) const HAND_DROP_TICKS: u32 = 14;
pub(super) const RISE_TICKS: u32 = 20;

/// The first few frames show the joints giving way; gravity then takes over.
/// Contact reaches the actual lowest pose on this same update. A small recoil
/// settles before the vulnerable heart reaches its playable destination.
pub(super) fn drop(t: u32, depth: f32, max_fold: f32) -> f32 {
    let impact = (depth + 2.0).min(max_fold);
    match t {
        0..=3 => [0.0, 0.25, 0.75, 1.5][t as usize],
        4..=CONTACT => {
            let k = (t - 3) as f32 / (CONTACT - 3) as f32;
            1.5 + (impact - 1.5) * k * k
        }
        16 => depth + (impact - depth) * 0.5,
        17 => depth,
        18 | 19 => (depth - 1.0).max(0.0),
        _ => depth,
    }
}

/// One supported extension curve for both a missed window and a phase clear.
pub(super) fn rise(from: f32, tick: u32) -> f32 {
    from * (1.0 - crate::util::ease_in_out(((tick + 1) as f32 / RISE_TICKS as f32).min(1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contact_and_settle_have_weight_without_exceeding_authored_poses() {
        for (depth, max) in [(16.0, 16.0), (24.0, 26.0)] {
            let values: Vec<_> = (0..40).map(|t| drop(t, depth, max)).collect();
            assert!(values[..=CONTACT as usize].windows(2).all(|v| v[1] >= v[0]));
            assert_eq!(values[CONTACT as usize], max);
            assert!(values.iter().all(|&v| v >= 0.0 && v <= max));
            assert_eq!(&values[20..], vec![depth; 20].as_slice());
            assert!(values[15] - values[14] > values[8] - values[7]);
        }
    }

    #[test]
    fn both_recovery_paths_can_use_the_same_complete_extension() {
        for from in [0.0, 16.0, 24.0, 26.0] {
            let mut prior = from;
            for t in 0..RISE_TICKS {
                let next = rise(from, t);
                assert!(next >= 0.0 && next <= prior);
                prior = next;
            }
            assert_eq!(prior, 0.0);
        }
    }
}
