//! Small shared helpers: deterministic RNG, hashing noise, ordered dithering, vector math.

/// Deterministic PCG-style RNG. Every system that needs randomness takes one of these,
/// so headless runs are reproducible.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x1234567))
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = self.0;
        let xorshifted = (((x >> 18) ^ x) >> 27) as u32;
        let rot = (x >> 59) as u32;
        xorshifted.rotate_right(rot)
    }
    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [a, b).
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
    /// Uniform integer in [a, b).
    pub fn irange(&mut self, a: i32, b: i32) -> i32 {
        if b <= a {
            return a;
        }
        a + (self.next_u32() % (b - a) as u32) as i32
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
    /// Uniform in [-1, 1).
    pub fn signed(&mut self) -> f32 {
        self.f32() * 2.0 - 1.0
    }
    /// Random unit vector.
    pub fn dir(&mut self) -> (f32, f32) {
        let a = self.f32() * std::f32::consts::TAU;
        (a.cos(), a.sin())
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_u32() as usize % items.len()]
    }
}

/// Integer hash -> [0, 1). Stable across runs; use for per-pixel/per-tile variation.
pub fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6b343) ^ (y as u32).wrapping_mul(0xd8163841) ^ seed.wrapping_mul(0xcb1ab31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

pub fn hash1(x: i32, seed: u32) -> f32 {
    hash2(x, 0x2f1, seed)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in [0, 1), smooth, period-free.
pub fn noise2(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = smooth(x - xi as f32);
    let fy = smooth(y - yi as f32);
    let a = hash2(xi, yi, seed);
    let b = hash2(xi + 1, yi, seed);
    let c = hash2(xi, yi + 1, seed);
    let d = hash2(xi + 1, yi + 1, seed);
    let ab = a + (b - a) * fx;
    let cd = c + (d - c) * fx;
    ab + (cd - ab) * fy
}

/// Fractal value noise in roughly [0, 1).
pub fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for i in 0..octaves {
        sum += noise2(x * freq, y * freq, seed.wrapping_add(i * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Ordered-dither threshold in (0, 1) for pixel (x, y), 4x4 Bayer.
#[inline]
pub fn bayer4(x: i32, y: i32) -> f32 {
    (BAYER4[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0
}

/// Ordered-dither threshold in (0, 1) for pixel (x, y), 8x8 Bayer.
#[inline]
pub fn bayer8(x: i32, y: i32) -> f32 {
    (BAYER8[(y & 7) as usize][(x & 7) as usize] as f32 + 0.5) / 64.0
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

pub fn len(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

pub fn norm(x: f32, y: f32) -> (f32, f32) {
    let l = len(x, y);
    if l < 1e-6 { (0.0, 0.0) } else { (x / l, y / l) }
}

/// Frame-rate independent approach: move `cur` toward `target` by fraction `k` per tick.
pub fn approach(cur: f32, target: f32, k: f32) -> f32 {
    cur + (target - cur) * k
}

/// Signed smallest difference between two angles (radians), in [-PI, PI].
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}
