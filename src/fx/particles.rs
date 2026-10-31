//! Noita-style pixel particles. Every particle is one (or a 2x2) pixel with a height `z`
//! above the ground: it flies, falls under gravity, bounces, and when it comes to rest it can
//! "settle" into ground debris that lingers and slowly fades. Sprites can be burst into
//! particles pixel by pixel (`burst_cell`), with small source-shaped fragments among the grains.

use crate::gfx::{Cell, Frame, Rgb};
use crate::util::{Rng, bayer4};
use crate::world::Zone;

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    /// Height above the ground (drawn at y - z).
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    /// Color at birth and at death (linear blend over life).
    pub c0: Rgb,
    pub c1: Rgb,
    /// 0 = lit like the world, 1 = fully self-lit (glows).
    pub glow: f32,
    pub life: f32,
    pub max_life: f32,
    /// Downward acceleration on z per tick (0 = floats).
    pub grav: f32,
    /// Velocity multiplier per tick (air drag), e.g. 0.96.
    pub drag: f32,
    /// Fraction of vz kept on bounce.
    pub bounce: f32,
    /// 1 = single pixel, 2 = 2x2.
    pub size: u8,
    /// On landing (slow), turn into ground debris instead of vanishing.
    pub settle: bool,
    /// Draw a tiny shadow on the ground while airborne.
    pub shadow: bool,
    /// Radius of a light this particle casts (0 = none).
    pub light: f32,
    /// Fade out with dithering over the last part of life.
    pub fade: bool,
    /// Draw a short streak along velocity (sparks).
    pub streak: bool,
    /// Source-pixel offset from a shared fragment trajectory, on the native screen grid.
    fragment_offset: (i32, i32),
}

impl Particle {
    /// A lit, falling pixel at a world position with sensible defaults. Chain the builders.
    pub fn new(x: f32, y: f32) -> Particle {
        Particle {
            x,
            y,
            z: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            c0: Rgb::WHITE,
            c1: Rgb::WHITE,
            glow: 0.0,
            life: 60.0,
            max_life: 60.0,
            grav: 0.18,
            drag: 0.97,
            bounce: 0.35,
            size: 1,
            settle: false,
            shadow: false,
            light: 0.0,
            fade: true,
            streak: false,
            fragment_offset: (0, 0),
        }
    }
    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }
    pub fn vel(mut self, vx: f32, vy: f32, vz: f32) -> Self {
        self.vx = vx;
        self.vy = vy;
        self.vz = vz;
        self
    }
    pub fn color(mut self, c: Rgb) -> Self {
        self.c0 = c;
        self.c1 = c;
        self
    }
    pub fn fade_to(mut self, c: Rgb) -> Self {
        self.c1 = c;
        self
    }
    pub fn glow(mut self, g: f32) -> Self {
        self.glow = g;
        self
    }
    pub fn life(mut self, ticks: f32) -> Self {
        self.life = ticks;
        self.max_life = ticks;
        self
    }
    pub fn grav(mut self, g: f32) -> Self {
        self.grav = g;
        self
    }
    pub fn drag(mut self, d: f32) -> Self {
        self.drag = d;
        self
    }
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b;
        self
    }
    pub fn size(mut self, s: u8) -> Self {
        self.size = s;
        self
    }
    pub fn settle(mut self, s: bool) -> Self {
        self.settle = s;
        self
    }
    pub fn shadow(mut self, s: bool) -> Self {
        self.shadow = s;
        self
    }
    pub fn light(mut self, r: f32) -> Self {
        self.light = r;
        self
    }
    pub fn fade(mut self, f: bool) -> Self {
        self.fade = f;
        self
    }
    pub fn streak(mut self, s: bool) -> Self {
        self.streak = s;
        self
    }
}

/// A settled pixel lying on the ground.
#[derive(Clone, Copy, Debug)]
pub struct Debris {
    pub x: i32,
    pub y: i32,
    pub c: Rgb,
    pub glow: f32,
    pub life: u32,
    pub max_life: u32,
}

#[derive(Default)]
pub struct Particles {
    pub list: Vec<Particle>,
    pub debris: Vec<Debris>,
}

const MAX_PARTICLES: usize = 12_000;
const MAX_DEBRIS: usize = 16_000;

/// Pick up to eight connected, notched 3x3 pieces directly from the source silhouette.
fn fragment_groups(cell: &Cell) -> Vec<Vec<(i32, i32)>> {
    let mut groups = Vec::new();
    for by in (0..cell.h).step_by(3) {
        for bx in (0..cell.w).step_by(3) {
            let mut remaining = Vec::new();
            for dy in 0..3 {
                for dx in 0..3 {
                    // Opposite missing corners give a diagonal chip rather than a square.
                    if (dx, dy) != (0, 0) && (dx, dy) != (2, 2) && cell.at(bx + dx, by + dy).a {
                        remaining.push((bx + dx, by + dy));
                    }
                }
            }
            while let Some(seed) = remaining.pop() {
                let mut group = vec![seed];
                let mut next = 0;
                while next < group.len() {
                    let (x, y) = group[next];
                    let mut i = 0;
                    while i < remaining.len() {
                        let (nx, ny) = remaining[i];
                        if (nx - x).abs() + (ny - y).abs() == 1 {
                            group.push(remaining.swap_remove(i));
                        } else {
                            i += 1;
                        }
                    }
                    next += 1;
                }
                if group.len() >= 3 {
                    groups.push(group);
                }
            }
        }
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.len()));
    groups.truncate(8);
    groups
}

impl Particles {
    pub fn add(&mut self, p: Particle) {
        if self.list.len() < MAX_PARTICLES {
            self.list.push(p);
        }
    }

    pub fn clear(&mut self) {
        self.list.clear();
        self.debris.clear();
    }

    /// Burst a sprite cell into particles, pixel by pixel. The cell is drawn as if anchored at
    /// world (wx, wy) with `anchor`; each pixel starts at its height above the anchor line and
    /// flies outward with speed ~`power`; a few connected source clusters settle as debris.
    #[allow(clippy::too_many_arguments)]
    pub fn burst_cell(&mut self, rng: &mut Rng, cell: &Cell, anchor: (i32, i32), flip: bool, wx: f32, wy: f32, power: f32, debris_life: f32) {
        let mut indices = vec![None; cell.px.len()];
        for y in 0..cell.h {
            for x in 0..cell.w {
                let p = cell.at(x, y);
                if !p.a {
                    continue;
                }
                let lx = if flip { cell.w - 1 - x } else { x };
                let px = wx + (lx - if flip { cell.w - 1 - anchor.0 } else { anchor.0 }) as f32;
                let h = (anchor.1 - y) as f32; // height above ground
                let (dx, dy) = (px - wx, -h);
                let l = (dx * dx + dy * dy).sqrt().max(1.0);
                let sp = power * rng.range(0.4, 1.2);
                let mut part = Particle::new(px, wy + (-h).max(0.0))
                    .z(h.max(0.0))
                    .vel(dx / l * sp + rng.signed() * power * 0.5, rng.signed() * power * 0.7, rng.range(0.5, 2.2) * power * 0.8)
                    .color(p.c)
                    .glow(if p.emit { 1.0 } else { 0.0 })
                    .life(debris_life * rng.range(0.7, 1.3))
                    .grav(rng.range(0.12, 0.23))
                    .drag(rng.range(0.94, 0.985))
                    .bounce(rng.range(0.22, 0.5))
                    .settle(true)
                    .shadow(false);
                // Keep the same RNG draws per source pixel: this visual change must not
                // perturb later enemies' random decisions on the shared effects stream.
                let grain_life = part.life / debris_life.max(1.0) * debris_life.min(42.0);
                if self.list.len() < MAX_PARTICLES {
                    indices[(y * cell.w + x) as usize] = Some(self.list.len());
                }
                part.life = grain_life;
                part.max_life = grain_life;
                part.settle = false;
                self.add(part);
            }
        }
        for group in fragment_groups(cell) {
            // The lowest source pixel is the common ground contact for this little shard.
            let &(rx, ry) = group.iter().max_by_key(|(_, y)| y).unwrap();
            let Some(root) = indices[(ry * cell.w + rx) as usize] else { continue };
            let origin = self.list[root];
            let life = origin.life / debris_life.min(42.0).max(1.0) * debris_life.min(132.0);
            for (x, y) in group {
                let Some(index) = indices[(y * cell.w + x) as usize] else { continue };
                let p = &mut self.list[index];
                p.fragment_offset = (if flip { rx - x } else { x - rx }, y - ry);
                p.x = origin.x;
                p.y = origin.y;
                p.z = origin.z;
                p.vx = origin.vx;
                p.vy = origin.vy;
                p.vz = origin.vz;
                p.grav = origin.grav;
                p.drag = origin.drag;
                p.bounce = origin.bounce;
                p.life = life;
                p.max_life = life;
                p.settle = true;
            }
        }
    }

    /// A spray of glowing sparks along direction (dx, dy).
    #[allow(clippy::too_many_arguments)]
    pub fn sparks(&mut self, rng: &mut Rng, x: f32, y: f32, z: f32, dx: f32, dy: f32, n: usize, color: Rgb, speed: f32) {
        for _ in 0..n {
            let a = dy.atan2(dx) + rng.signed() * 0.9;
            let s = speed * rng.range(0.4, 1.3);
            self.add(
                Particle::new(x, y)
                    .z(z)
                    .vel(a.cos() * s, a.sin() * s, rng.range(0.2, 1.6))
                    .color(Rgb::WHITE)
                    .fade_to(color)
                    .glow(1.0)
                    .life(rng.range(10.0, 22.0))
                    .grav(rng.range(0.08, 0.18))
                    .drag(rng.range(0.87, 0.97))
                    .bounce(rng.range(0.2, 0.45))
                    .streak(true),
            );
        }
    }

    /// Scatter matte material chips with the same trajectories and RNG draws as sparks.
    #[allow(clippy::too_many_arguments)]
    pub fn chips(&mut self, rng: &mut Rng, x: f32, y: f32, z: f32, dx: f32, dy: f32, n: usize, color: Rgb, speed: f32) {
        let start = self.list.len();
        self.sparks(rng, x, y, z, dx, dy, n, color, speed);
        for particle in &mut self.list[start..] {
            particle.c0 = color;
            particle.c1 = color;
            particle.glow = 0.0;
            particle.streak = false;
        }
    }

    /// Soft dust puff at a world position (footsteps, landings, dashes).
    pub fn dust(&mut self, rng: &mut Rng, x: f32, y: f32, n: usize, color: Rgb) {
        for _ in 0..n {
            self.add(
                Particle::new(x + rng.signed() * 3.0, y + rng.signed() * 1.5)
                    .z(rng.range(0.0, 2.0))
                    .vel(rng.signed() * 0.5, rng.signed() * 0.25, rng.range(0.1, 0.5))
                    .color(color)
                    .life(rng.range(14.0, 26.0))
                    .grav(0.01)
                    .drag(0.9),
            );
        }
    }

    pub fn update(&mut self, zone: &Zone) {
        let mut landed: Vec<Debris> = Vec::new();
        self.list.retain_mut(|p| {
            p.life -= 1.0;
            if p.life <= 0.0 {
                return false;
            }
            p.vx *= p.drag;
            p.vy *= p.drag;
            // Short substeps stop fast shards tunnelling through a tile. Resolve each
            // axis separately, so a glancing hit still slides along the wall.
            let steps = ((p.vx.abs().max(p.vy.abs()) / 3.0).ceil() as usize).clamp(1, 32);
            for _ in 0..steps {
                let nx = p.x + p.vx / steps as f32;
                if p.z < 10.0 && zone.solid_tile_at(nx, p.y) {
                    p.vx *= -0.4;
                } else {
                    p.x = nx;
                }
                let ny = p.y + p.vy / steps as f32;
                if p.z < 10.0 && zone.solid_tile_at(p.x, ny) {
                    p.vy *= -0.4;
                } else {
                    p.y = ny;
                }
            }
            if p.grav != 0.0 {
                p.vz -= p.grav;
                p.z += p.vz;
                if p.z <= 0.0 {
                    p.z = 0.0;
                    if p.vz < -0.8 {
                        p.vz = -p.vz * p.bounce;
                        p.vx *= 0.6;
                        p.vy *= 0.6;
                    } else {
                        p.vz = 0.0;
                        p.vx *= 0.75;
                        p.vy *= 0.75;
                        if p.settle && p.vx.abs() + p.vy.abs() < 0.15 {
                            let x = p.x.floor() as i32 + p.fragment_offset.0;
                            let y = p.y.floor() as i32 + p.fragment_offset.1;
                            if !zone.liquid_at(x as f32, y as f32) {
                                let life = p.life.max(60.0) as u32;
                                let age = (1.0 - p.life / p.max_life.max(1.0)).clamp(0.0, 1.0);
                                landed.push(Debris { x, y, c: p.c0.lerp(p.c1, age), glow: p.glow, life, max_life: life });
                            }
                            return false;
                        }
                    }
                }
            } else {
                p.z = (p.z + p.vz).max(0.0);
                p.vz *= p.drag;
            }
            true
        });
        if !landed.is_empty() {
            let room = MAX_DEBRIS.saturating_sub(self.debris.len());
            if landed.len() > room {
                let drop = (landed.len() - room).min(self.debris.len());
                self.debris.drain(0..drop);
            }
            self.debris.extend(landed);
            if self.debris.len() > MAX_DEBRIS {
                self.debris.drain(0..self.debris.len() - MAX_DEBRIS);
            }
        }
        self.debris.retain_mut(|d| {
            d.life = d.life.saturating_sub(1);
            d.life > 0
        });
    }

    /// Ground debris: draw after the ground, before characters.
    pub fn draw_debris(&self, f: &mut Frame) {
        for d in &self.debris {
            let sx = d.x - f.cam_x;
            let sy = d.y - f.cam_y;
            if !(0..crate::gfx::WI).contains(&sx) || !(0..crate::gfx::HI).contains(&sy) {
                continue;
            }
            let k = d.life as f32 / d.max_life.max(1) as f32;
            let a = (k * 4.0).min(1.0);
            if a < 1.0 && a <= bayer4(d.x, d.y) {
                continue;
            }
            if d.glow > 0.0 {
                // glowing debris dims as it ages
                let heat = ((k.clamp(0.0, 1.0) * 4.0).ceil() / 4.0).max(0.25);
                let c = d.c.scale(0.35 + 0.65 * heat);
                f.put(sx, sy, c.scale(0.5));
                f.add_emit(sx, sy, c.scale(d.glow * (0.3 + 0.7 * heat)));
            } else {
                f.put(sx, sy, d.c);
            }
        }
    }

    /// Airborne particles: draw after characters.
    pub fn draw(&self, f: &mut Frame) {
        for p in &self.list {
            let k = (1.0 - p.life / p.max_life.max(1.0)).clamp(0.0, 1.0);
            if p.fade && k > 0.7 {
                let a = 1.0 - (k - 0.7) / 0.3;
                if a <= bayer4(p.x.floor() as i32, p.y.floor() as i32) {
                    continue;
                }
            }
            let c = p.c0.lerp(p.c1, k);
            let heat = ((1.0 - k) * 4.0).ceil() / 4.0;
            let sx = f.sx(p.x) + p.fragment_offset.0;
            let sy = f.sy(p.y - p.z) + p.fragment_offset.1;
            if p.shadow && p.z > 1.0 {
                f.darken(f.sx(p.x), f.sy(p.y), 0.7);
            }
            let n = p.size as i32;
            for oy in 0..n {
                for ox in 0..n {
                    if p.glow >= 1.0 {
                        f.put_glow(sx + ox, sy + oy, c.scale(0.4 + 0.6 * heat));
                    } else {
                        f.put(sx + ox, sy + oy, c);
                        if p.glow > 0.0 {
                            f.add_emit(sx + ox, sy + oy, c.scale(p.glow * (0.4 + 0.6 * heat)));
                        }
                    }
                }
            }
            if p.streak && p.vx.abs() + (p.vy - p.vz).abs() > 0.8 {
                let tx = sx - (p.vx * 1.5).clamp(-6.0, 6.0).round() as i32;
                let ty = sy - ((p.vy - p.vz) * 1.5).clamp(-6.0, 6.0).round() as i32;
                let cc = c.scale(0.4 * heat);
                Frame::line_with(sx, sy, tx, ty, |x, y| {
                    if (x, y) != (sx, sy) {
                        f.add_emit(x, y, cc);
                    }
                });
            }
            if p.light > 0.0 {
                f.add_light_w(p.x, p.y - p.z, p.light, c, 0.5 * (1.0 - k));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{DrawOpts, H, W, post, rgb};
    use crate::world::{Tile, ZoneId};

    /// Bursting a sprite starts with exactly its silhouette, including a flipped anchor.
    #[test]
    fn burst_preserves_initial_pixels_and_settles() {
        let cell = Cell::from_ascii(&[".aa..", "abbaa", "..a.a"], &[('a', rgb(0x78b9cc)), ('b', rgb(0xc5e4d8))], &['b']);
        for flip in [false, true] {
            let mut particles = Particles::default();
            particles.burst_cell(&mut Rng::new(27), &cell, (1, 1), flip, 80.0, 80.0, 2.0, 180.0);
            let mut sprite = Frame::new();
            sprite.sprite(&cell, (1, 1), 80, 80, &DrawOpts::flip(flip));
            let mut burst = Frame::new();
            particles.draw(&mut burst);
            assert_eq!(sprite.albedo, burst.albedo);
            assert_eq!(sprite.emissive, burst.emissive);
            let zone = Zone::new(ZoneId::Bloomwild, 30, 20);
            for _ in 0..100 {
                particles.update(&zone);
            }
            assert!(particles.list.is_empty());
            assert!(!particles.debris.is_empty());
            for _ in 0..300 {
                particles.update(&zone);
            }
            assert!(particles.debris.is_empty());
        }
    }

    /// Source-shaped fragments hold together in flight while loose grains retire first.
    #[test]
    fn burst_clusters_keep_source_pixels_and_clear_the_grains() {
        let cell = Cell::from_ascii(&["aaaaaaaaa"; 9], &[('a', rgb(0x78b9cc))], &[]);
        let zone = Zone::new(ZoneId::Bloomwild, 30, 20);
        for flip in [false, true] {
            let mut rng = Rng::new(27);
            let mut particles = Particles::default();
            particles.burst_cell(&mut rng, &cell, (2, 12), flip, 80.0, 80.0, 2.0, 420.0);
            assert_eq!(particles.list.len(), 81);
            let mut original_rng = Rng::new(27);
            // The existing burst consumes eight draws per visible pixel, including grains.
            for _ in 0..81 * 8 {
                original_rng.next_u32();
            }
            assert_eq!(rng.next_u32(), original_rng.next_u32());
            let groups = fragment_groups(&cell);
            assert_eq!(groups.len(), 8);
            assert!(particles.list.iter().any(|p| !p.settle));
            for _ in 0..12 {
                particles.update(&zone);
            }
            for group in &groups {
                let root = particles.list[(group[0].1 * cell.w + group[0].0) as usize];
                for &(x, y) in group {
                    let p = particles.list[(y * cell.w + x) as usize];
                    assert_eq!((p.x, p.y, p.z), (root.x, root.y, root.z));
                    assert_eq!(p.life, root.life);
                    assert_eq!(p.fragment_offset.0 - root.fragment_offset.0, (x - group[0].0) * if flip { -1 } else { 1 });
                    assert_eq!(p.fragment_offset.1 - root.fragment_offset.1, y - group[0].1);
                }
            }
            for _ in 12..60 {
                particles.update(&zone);
            }
            assert!(particles.list.iter().all(|p| p.settle), "loose grains outlived the spray");
            assert!(!particles.debris.is_empty(), "material pieces should leave a brief residue");
            for _ in 60..240 {
                particles.update(&zone);
            }
            assert!(particles.list.is_empty() && particles.debris.is_empty());
        }
    }

    /// Fast fragments bounce off a one-tile wall, and liquid never retains settled debris.
    #[test]
    fn shards_do_not_tunnel_or_settle_in_liquid() {
        let mut zone = Zone::new(ZoneId::Glasswater, 30, 20);
        for y in 0..zone.th {
            zone.set_tile(5, y, Tile::Wall);
        }
        let mut particles = Particles::default();
        particles.add(Particle::new(70.0, 80.0).vel(40.0, 2.0, 0.0).grav(0.0).drag(1.0));
        particles.update(&zone);
        assert!(particles.list[0].x < 80.0);
        assert!(particles.list[0].vx < 0.0);
        assert!(particles.list[0].y > 80.0);
        particles.clear();
        zone.set_tile(3, 3, Tile::Shallow);
        particles.add(Particle::new(50.0, 50.0).settle(true));
        particles.update(&zone);
        assert!(particles.list.is_empty());
        assert!(particles.debris.is_empty());
    }

    /// Export four native-pixel burst stages when explicitly requested for visual inspection.
    #[test]
    fn particle_study() {
        let Some(dest) = std::env::var_os("GLINT_PARTICLE_STUDY") else { return };
        let cell = Cell::from_ascii(
            &["....aa....", "...abba...", "..abbcca..", ".abbbccca.", "abbbbccccd", "abbbccccdd", ".abccccdd.", "..acccdd..", "...add...."],
            &[('a', rgb(0x326b8c)), ('b', rgb(0x58a9b5)), ('c', rgb(0x97d2c8)), ('d', rgb(0xdbede3))],
            &['c', 'd'],
        );
        let zone = Zone::new(ZoneId::Glasswater, 30, 20);
        let mut rng = Rng::new(27);
        let mut particles = Particles::default();
        particles.burst_cell(&mut rng, &cell, (4, 8), false, 192.0, 115.0, 3.0, 240.0);
        particles.sparks(&mut rng, 192.0, 115.0, 5.0, 1.0, -0.2, 28, rgb(0x57b6cc), 3.0);
        let mut f = Frame::new();
        let mut img = vec![0u8; W * 4 * H * 4];
        for tick in 0..=100 {
            if let Some(panel) = [0, 12, 36, 100].iter().position(|t| *t == tick) {
                f.begin(rgb(0x272838), Rgb::WHITE);
                f.rect(0, 116, W as i32, 2, rgb(0x3e465a));
                particles.draw_debris(&mut f);
                particles.draw(&mut f);
                let g = crate::gfx::Grade { vignette: 0.0, ..Default::default() };
                post::compose(&mut f, &g);
                post::finish(&mut f, &g);
                for y in 0..H {
                    for x in 0..W {
                        let i = (y * W * 4 + panel * W + x) * 4;
                        img[i..i + 4].copy_from_slice(&f.out[y * W + x]);
                    }
                }
            }
            particles.update(&zone);
        }
        crate::assets::write_png(std::path::Path::new(&dest), (W * 4) as u32, H as u32, &img).unwrap();
    }
}
