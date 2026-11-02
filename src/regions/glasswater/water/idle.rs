//! Quiet open-water catches for large shallow basins. The main lake keeps its
//! authored composition; these few drifting strokes give the heart pool the
//! same sense of a surface without filling it with foam or sparkle.

const MIN_BASIN: usize = 20_000;
const MARKS: usize = 8;

pub(super) struct Surface {
    width: i32,
    height: i32,
    /// Only the swept footprint of a catch pays for phase evaluation. Zero is
    /// dry/quiet; other entries index a single mark, already clipped to its basin.
    receiving: Vec<u16>,
    marks: Vec<Mark>,
}

struct Mark {
    x: i32,
    y: i32,
    length: i32,
    rows: i32,
    travel: i32,
    period: u64,
    phase: u64,
}

impl Surface {
    pub(super) fn new(pw: i32, ph: i32, bed: &[u8], dland: &[u8]) -> Self {
        let mut surface = Self {
            width: pw.max(0),
            height: ph.max(0),
            receiving: Vec::new(),
            marks: Vec::new(),
        };
        if pw <= 0 || ph <= 0 {
            return surface;
        }
        let n = pw as usize * ph as usize;
        if bed.len() != n || dland.len() != n {
            return surface;
        }
        surface.receiving.resize(n, 0);
        let mut labels = vec![0u32; n];
        let mut label = 0u32;
        for seed in 0..n {
            if bed[seed] == 255 || labels[seed] != 0 {
                continue;
            }
            label += 1;
            labels[seed] = label;
            let mut pixels = vec![seed];
            let mut cursor = 0;
            let (mut x0, mut y0, mut x1, mut y1) = (pw, ph, 0, 0);
            let mut deep = false;
            while cursor < pixels.len() {
                let i = pixels[cursor];
                cursor += 1;
                let (x, y) = (i as i32 % pw, i as i32 / pw);
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
                deep |= bed[i] >= 8;
                for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                    if nx < 0 || nx >= pw || ny < 0 || ny >= ph {
                        continue;
                    }
                    let j = (ny * pw + nx) as usize;
                    if bed[j] != 255 && labels[j] == 0 {
                        labels[j] = label;
                        pixels.push(j);
                    }
                }
            }
            // The gallery, start pool and narrow grove channel already have
            // ample shoreline/crystal movement. Deep water has authored marks.
            if pixels.len() < MIN_BASIN || deep || y1 - y0 < 48 {
                continue;
            }
            for k in 0..MARKS {
                let seed = mix((x0 as u32).wrapping_mul(31) ^ y0 as u32 ^ k as u32 * 977);
                let y =
                    y0 + (y1 - y0) * (k as i32 + 1) / (MARKS as i32 + 1) + (seed % 7) as i32 - 3;
                // Select a real interior run, rather than assuming the basin's
                // bounding rectangle is all water. Islands can split this row.
                let mut runs = Vec::new();
                let mut start = None;
                for x in x0..=x1 + 1 {
                    let receives = x <= x1 && y >= 0 && y < ph && {
                        let i = (y * pw + x) as usize;
                        labels[i] == label && dland[i] >= 6
                    };
                    if receives && start.is_none() {
                        start = Some(x);
                    } else if !receives && let Some(lo) = start.take() {
                        if x - lo >= 28 {
                            runs.push((lo, x));
                        }
                    }
                }
                let Some(&(lo, hi)) = runs.iter().max_by_key(|&&(lo, hi)| hi - lo) else {
                    continue;
                };
                let length = (28 + (mix(seed) % 31) as i32).min(hi - lo - 4);
                let travel = 18 + (mix(seed ^ 67) % 13) as i32;
                let margin = (hi - lo - length).max(0);
                let x = lo
                    + margin / 2
                    + ((mix(seed ^ 91) % 11) as i32 - 5).clamp(-margin / 2, margin / 2);
                let mark = Mark {
                    x,
                    y,
                    length,
                    rows: if k % 3 == 0 { 1 } else { 2 },
                    travel,
                    period: 440 + u64::from(mix(seed ^ 117) % 241),
                    phase: u64::from(mix(seed ^ 139) % 680),
                };
                let index = surface.marks.len() + 1;
                // The horizontal breathing can extend either end by two pixels;
                // slow vertical drift only reaches the immediately adjacent row.
                for yy in (y - 1).max(0)..(y + mark.rows + 1).min(ph) {
                    for xx in (x - travel - 2).max(0)..(x + length + travel + 2).min(pw) {
                        let i = (yy * pw + xx) as usize;
                        if labels[i] == label && dland[i] >= 6 {
                            surface.receiving[i] = index as u16;
                        }
                    }
                }
                surface.marks.push(mark);
            }
        }
        surface
    }

    /// World-anchored native-pixel highlight strength. Individual marks breathe
    /// on different long cycles; no trigonometry, noise or global pulse per pixel.
    #[inline]
    pub(super) fn accent(&self, wx: i32, wy: i32, t: u64) -> f32 {
        if wx < 0 || wy < 0 || wx >= self.width || wy >= self.height {
            return 0.0;
        }
        let Some(&index) = self.receiving.get((wy * self.width + wx) as usize) else {
            return 0.0;
        };
        if index == 0 {
            return 0.0;
        }
        let mark = &self.marks[index as usize - 1];
        let clock = t.wrapping_add(mark.phase) % mark.period;
        let end = mark.period - 64;
        if clock >= end {
            return 0.0;
        }
        let life =
            smooth((clock as f32 / 44.0).min(1.0)) * smooth(((end - clock) as f32 / 66.0).min(1.0));
        let drift = triangle(clock, mark.period);
        let x = mark.x + ((drift * 2.0 - 1.0) * mark.travel as f32).round() as i32;
        let row = (triangle(t.wrapping_add(mark.phase * 3), mark.period + 173) * 2.0 - 1.0).round()
            as i32;
        let y = mark.y + row;
        if wy < y || wy >= y + mark.rows {
            return 0.0;
        }
        let breathe =
            (triangle(t.wrapping_add(mark.phase * 5), mark.period + 97) * 2.0).round() as i32;
        let left = x - breathe;
        let right = x + mark.length + breathe;
        if wx < left || wx >= right {
            return 0.0;
        }
        let tip = ((wx - left + 1).min(right - wx) as f32 * 0.25).min(1.0);
        let row_gain = if wy == y { 1.0 } else { 0.65 };
        life * tip * row_gain
    }
}

#[inline]
fn smooth(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

#[inline]
fn triangle(t: u64, period: u64) -> f32 {
    smooth(1.0 - ((t % period) as f32 * (2.0 / period as f32) - 1.0).abs())
}

fn mix(mut seed: u32) -> u32 {
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(0x7feb_352d);
    seed ^= seed >> 15;
    seed = seed.wrapping_mul(0x846c_a68b);
    seed ^ (seed >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basin(width: i32, height: i32) -> (Vec<u8>, Vec<u8>) {
        let n = (width * height) as usize;
        (vec![1; n], vec![20; n])
    }

    #[test]
    fn preserves_small_pools_and_the_authored_deep_lake() {
        let (bed, distance) = basin(120, 120);
        assert!(Surface::new(120, 120, &bed, &distance).marks.is_empty());
        let (mut bed, distance) = basin(200, 150);
        bed[100] = 8;
        assert!(Surface::new(200, 150, &bed, &distance).marks.is_empty());
        bed[100] = 1;
        assert!(!Surface::new(200, 150, &bed, &distance).marks.is_empty());
    }

    #[test]
    fn catches_stay_on_the_interior_of_the_actual_receiving_mask() {
        let (mut bed, mut distance) = basin(240, 160);
        // A dry island and quiet shallow margin must never become a rectangle
        // of highlights merely because they lie inside the basin's bounds.
        for y in 45..110 {
            for x in 80..160 {
                bed[y * 240 + x] = 255;
            }
        }
        for y in 0..160 {
            for x in 0..240 {
                if x < 6 || y < 6 || x >= 234 || y >= 154 {
                    distance[y * 240 + x] = 2;
                }
            }
        }
        let surface = Surface::new(240, 160, &bed, &distance);
        assert!(!surface.marks.is_empty());
        for t in (0..840).step_by(13) {
            for y in 0..160 {
                for x in 0..240 {
                    let i = (y * 240 + x) as usize;
                    let a = surface.accent(x, y, t);
                    assert!((0.0..=1.0).contains(&a));
                    if bed[i] == 255 || distance[i] < 6 {
                        assert_eq!(a, 0.0);
                    }
                }
            }
        }
        assert_eq!(surface.accent(-1, 80, 100), 0.0);
        assert_eq!(surface.accent(240, 80, 100), 0.0);
    }

    #[test]
    fn sparse_world_marks_move_without_a_camera_or_a_global_flash() {
        let (bed, distance) = basin(240, 160);
        let surface = Surface::new(240, 160, &bed, &distance);
        let mut moved = vec![false; bed.len()];
        let mut peak = 0;
        for t in (0..840).step_by(8) {
            let mut active = 0;
            for y in 0..160 {
                for x in 0..240 {
                    if surface.accent(x, y, t) > 0.12 {
                        active += 1;
                        moved[(y * 240 + x) as usize] = true;
                    }
                }
            }
            peak = peak.max(active);
        }
        let touched = moved.iter().filter(|&&v| v).count();
        assert!(peak * 100 < bed.len() * 3, "too busy: {peak}");
        assert!(touched > 1_000, "too still: {touched}");
        assert!(touched * 100 < bed.len() * 12, "too widespread: {touched}");
        // Rasterize two camera windows and compare their overlap, including
        // visible marks. Camera movement must not slide a stroke over its bank.
        let raster = |cx, cy| {
            (0..120)
                .flat_map(|y| (0..160).map(move |x| (x, y)))
                .map(|(x, y)| surface.accent(x + cx, y + cy, 500))
                .collect::<Vec<_>>()
        };
        let a = raster(0, 0);
        let b = raster(40, 20);
        let mut visible = 0;
        for y in 20..120 {
            for x in 40..160 {
                let value = a[y * 160 + x];
                assert_eq!(value, b[(y - 20) * 160 + x - 40]);
                visible += usize::from(value > 0.12);
            }
        }
        assert!(
            visible > 40,
            "exercise the visible surface catches: {visible}"
        );
    }
}
