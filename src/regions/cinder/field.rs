//! Per-pixel scalar fields used while painting the ground once at load: masks, blurs that
//! round blocky tile shapes into organic ones, and distance transforms.

#[derive(Clone)]
pub struct Grid<T> {
    pub w: i32,
    pub h: i32,
    pub v: Vec<T>,
}

impl<T: Copy> Grid<T> {
    /// Allocate a filled scalar field.
    pub fn new(w: i32, h: i32, fill: T) -> Grid<T> {
        Grid {
            w,
            h,
            v: vec![fill; (w * h) as usize],
        }
    }
    /// Check whether a pixel belongs to this field.
    #[inline]
    pub fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }
    /// Value at (x, y), clamped to the edge.
    #[inline]
    pub fn at(&self, x: i32, y: i32) -> T {
        let x = x.clamp(0, self.w - 1);
        let y = y.clamp(0, self.h - 1);
        self.v[(y * self.w + x) as usize]
    }
    /// Set a pixel when it lies inside the field.
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, val: T) {
        if self.inside(x, y) {
            self.v[(y * self.w + x) as usize] = val;
        }
    }
}

impl Grid<bool> {
    /// Convert a boolean mask to a unit scalar field.
    pub fn to_f32(&self) -> Grid<f32> {
        Grid {
            w: self.w,
            h: self.h,
            v: self.v.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect(),
        }
    }
}

/// Box blur of radius r (window 2r+1), edges clamped. Two passes give a soft, round kernel.
pub fn box_blur(src: &Grid<f32>, r: i32) -> Grid<f32> {
    if r <= 0 {
        return src.clone();
    }
    let (w, h) = (src.w, src.h);
    let n = (2 * r + 1) as f32;
    let mut tmp = Grid::new(w, h, 0.0f32);
    for y in 0..h {
        let row = (y * w) as usize;
        let mut acc = 0.0;
        for k in -r..=r {
            acc += src.v[row + k.clamp(0, w - 1) as usize];
        }
        for x in 0..w {
            tmp.v[row + x as usize] = acc / n;
            acc += src.v[row + (x + r + 1).clamp(0, w - 1) as usize]
                - src.v[row + (x - r).clamp(0, w - 1) as usize];
        }
    }
    let mut out = Grid::new(w, h, 0.0f32);
    for x in 0..w {
        let mut acc = 0.0;
        for k in -r..=r {
            acc += tmp.v[(k.clamp(0, h - 1) * w + x) as usize];
        }
        for y in 0..h {
            out.v[(y * w + x) as usize] = acc / n;
            acc += tmp.v[((y + r + 1).clamp(0, h - 1) * w + x) as usize]
                - tmp.v[((y - r).clamp(0, h - 1) * w + x) as usize];
        }
    }
    out
}

/// Soft field: two box passes of radius r/2 each (close to a Gaussian).
pub fn soften(src: &Grid<f32>, r: i32) -> Grid<f32> {
    let a = box_blur(src, (r / 2).max(1));
    box_blur(&a, (r - r / 2).max(1))
}

/// Compute squared distance along one axis using the parabola lower envelope.
fn edt_1d(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    // Felzenszwalb & Huttenlocher, lower envelope of parabolas.
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let qf = q as f64;
        let mut s;
        loop {
            let p = v[k];
            let pf = p as f64;
            s = ((f[q] + qf * qf) - (f[p] + pf * pf)) / (2.0 * qf - 2.0 * pf);
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                break;
            }
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dd = q as f64 - v[k] as f64;
        d[q] = dd * dd + f[v[k]];
    }
}

/// Euclidean distance from each pixel to the nearest `true` pixel (0 on true pixels).
pub fn edt(mask: &Grid<bool>) -> Grid<f32> {
    let (w, h) = (mask.w as usize, mask.h as usize);
    const INF: f64 = 1e20;
    let mut g: Vec<f64> = mask.v.iter().map(|&b| if b { 0.0 } else { INF }).collect();
    let m = w.max(h);
    let mut f = vec![0.0f64; m];
    let mut d = vec![0.0f64; m];
    let mut v = vec![0usize; m];
    let mut z = vec![0.0f64; m + 1];
    for x in 0..w {
        for y in 0..h {
            f[y] = g[y * w + x];
        }
        edt_1d(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            g[y * w + x] = d[y];
        }
    }
    let mut out = Grid::new(mask.w, mask.h, 0.0f32);
    for y in 0..h {
        f[..w].copy_from_slice(&g[y * w..y * w + w]);
        edt_1d(&f[..w], &mut d[..w], &mut v, &mut z);
        for x in 0..w {
            out.v[y * w + x] = (d[x].sqrt() as f32).min(1e6);
        }
    }
    out
}
