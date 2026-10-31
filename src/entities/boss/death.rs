//! The Warden's death: it cracks all over, light pours out of the cracks and the heart in rays,
//! the fists crumble to rubble, and then stone pixels fall and settle while iris light escapes
//! from the authored emissive material. A last
//! sliver of the prism rises out of the light and climbs into the sky.

use super::art::{Spr, origin};
use super::hazards::{P0, P1, P2, PW, S2, S3};
use crate::fx::{Fx, Particle};
use crate::gfx::{Frame, Rgb};
use std::f32::consts::TAU;

/// Timeline (ticks of `Dying`).
pub const FIST_L_CRUMBLE: u32 = 112;
pub const FIST_R_CRUMBLE: u32 = 134;
pub const BURST: u32 = 206;
pub const RISE: u32 = 262;
pub const DONE: u32 = 336;

/// Cracks on the body get louder and faster toward the burst.
pub fn crack_sound_at(t: u32) -> bool {
    matches!(t, 14 | 62 | 100 | 128 | 148 | 164 | 176 | 186 | 194 | 200)
}

const SPECTRUM: [Rgb; 6] = [P1, P2, PW, P2, P1, P2];

/// Return an iris facet at an angle around the heart.
pub fn spectrum(a: f32) -> Rgb {
    let u = (a / TAU).rem_euclid(1.0) * SPECTRUM.len() as f32;
    let i = u.floor() as usize % SPECTRUM.len();
    SPECTRUM[i]
}

/// Burst a part into stone and iris fragments flying out from the heart in world draw space.
/// `ground` is the world y the part stands on; `glow` releases its authored emissive pixels.
/// Ordinary stone remains unlit in both burst modes.
#[allow(clippy::too_many_arguments)]
pub fn burst(
    fx: &mut Fx,
    s: Spr,
    wx: f32,
    wy: f32,
    flip: bool,
    ground: f32,
    heart: (f32, f32),
    power: f32,
    glow: bool,
    stride: usize,
) {
    let cell = s.cell;
    let (ox, oy) = origin(cell, s.anchor, wx.floor() as i32, wy.floor() as i32, flip);
    let mut n = 0usize;
    for y in 0..cell.h {
        for x in 0..cell.w {
            let p = cell.at(x, y);
            if !p.a {
                continue;
            }
            n += 1;
            if stride > 1 && n % stride != 0 {
                continue;
            }
            let px = (ox + if flip { cell.w - 1 - x } else { x }) as f32;
            let py = (oy + y) as f32;
            let z = (ground - py).max(0.0);
            let (dx, dy) = (px - heart.0, py - heart.1);
            let l = (dx * dx + dy * dy).sqrt().max(1.0);
            let sp = power * fx.rng.range(0.35, 1.25) * (1.15 - (l / 140.0).min(0.6));
            let (ux, uy) = (dx / l, dy / l);
            let mut part = Particle::new(px, ground + fx.rng.range(-2.5, 2.5))
                .z(z)
                .vel(
                    ux * sp + fx.rng.signed() * 0.25,
                    fx.rng.signed() * power * 0.55,
                    -uy * sp + fx.rng.range(0.0, 0.8),
                );
            if glow {
                let a = dy.atan2(dx);
                let lit = p.c.luma();
                let hot = spectrum(a + fx.rng.signed() * 0.25);
                // Stone stays stone through the breakup; only authored emitters release light.
                let c0 = if p.emit { hot } else { p.c };
                let c1 = if p.emit {
                    if lit < 0.08 { P0 } else { P1 }
                } else {
                    S2
                };
                let emission = if p.emit { 0.65 } else { 0.0 };
                if fx.rng.chance(0.18) {
                    // embers: drift up into the night and fade
                    part = part
                        .color(c0)
                        .fade_to(c1)
                        .glow(emission)
                        .grav(if p.emit { -0.012 } else { 0.045 })
                        .drag(0.965)
                        .life(fx.rng.range(90.0, 200.0))
                        .fade(true);
                } else {
                    part = part
                        .color(c0)
                        .fade_to(c1)
                        .glow(emission)
                        .grav(0.045)
                        .drag(0.93)
                        .bounce(0.3)
                        .life(fx.rng.range(170.0, 320.0))
                        .settle(true);
                }
            } else {
                part = part
                    .color(p.c)
                    .grav(0.18)
                    .drag(0.95)
                    .bounce(0.3)
                    .life(fx.rng.range(200.0, 360.0))
                    .settle(true);
                if fx.rng.chance(0.1) {
                    part = part.color(if p.emit { P2 } else { S3 }).glow(if p.emit {
                        0.35
                    } else {
                        0.0
                    });
                }
            }
            fx.particles.add(part);
        }
    }
}

/// Light rays from the heart: crisp lines that grow, rotate slowly and flicker in one by one.
pub fn rays(f: &mut Frame, hx: f32, hy: f32, t: u32, strength: f32) {
    if strength <= 0.0 {
        return;
    }
    let n = 6;
    for i in 0..n {
        // rays appear one by one as the cracking spreads
        let born = (i as u32 * 13) % 150;
        if t < born {
            continue;
        }
        let age = (t - born) as f32;
        let a = i as f32 * (TAU / n as f32)
            + 0.37 * i as f32
            + (t as f32 * 0.004)
            + ((i as f32 * 1.7 + t as f32 * 0.05).sin() * 0.03);
        let len = ((age * 2.6).min(60.0 + 210.0 * strength) * if i % 3 == 0 { 1.0 } else { 0.7 })
            .max(0.0);
        let (dx, dy) = (a.cos(), a.sin());
        let c = spectrum(a);
        let flicker = (crate::util::hash2(i, (t / 3) as i32, 77) > 0.12) as i32 as f32;
        if flicker == 0.0 {
            continue;
        }
        let (sx, sy) = (hx - f.cam_x as f32, hy - f.cam_y as f32);
        let mut d = 6.0;
        while d < len {
            let (x, y) = ((sx + dx * d).floor() as i32, (sy + dy * d).floor() as i32);
            // core near the heart, thin colored tail
            let k = d / len.max(1.0);
            let col = if k < 0.35 {
                PW
            } else if k < 0.7 {
                PW.lerp(c, (k - 0.35) / 0.35)
            } else {
                c
            };
            f.put_glow(x, y, col);
            if k < 0.25 && i % 2 == 0 {
                // the strongest rays are two pixels wide near the heart
                let (ox, oy) = if dx.abs() > dy.abs() { (0, 1) } else { (1, 0) };
                f.put_glow(x + ox, y + oy, PW);
            }
            d += 1.0;
        }
    }
}

/// Stone flakes shed from a part during the throes (the body starts coming apart).
pub fn shed(fx: &mut Fx, s: Spr, wx: f32, wy: f32, ground: f32, n: usize) {
    let cell = s.cell;
    let (ox, oy) = origin(cell, s.anchor, wx.floor() as i32, wy.floor() as i32, false);
    for _ in 0..n {
        let x = fx.rng.irange(0, cell.w);
        let y = fx.rng.irange(0, cell.h);
        let p = cell.at(x, y);
        if !p.a || p.emit {
            continue;
        }
        let (px, py) = ((ox + x) as f32, (oy + y) as f32);
        fx.particles.add(
            Particle::new(px, ground + fx.rng.range(-2.0, 2.0))
                .z((ground - py).max(0.0))
                .vel(
                    fx.rng.signed() * 0.5,
                    fx.rng.signed() * 0.3,
                    fx.rng.range(0.0, 0.8),
                )
                .color(if fx.rng.chance(0.5) { p.c } else { S2 })
                .grav(0.2)
                .drag(0.97)
                .bounce(0.3)
                .life(fx.rng.range(24.0, 40.0))
                .settle(false),
        );
    }
}

/// Collapse into short-lived grains and a few coherent stone patches, preserving burst RNG draws.
#[allow(clippy::too_many_arguments)]
pub fn collapse(
    fx: &mut Fx, s: Spr, wx: f32, wy: f32, flip: bool, ground: f32,
    heart: (f32, f32), power: f32, glow: bool,
) {
    let start = fx.particles.list.len();
    burst(fx, s, wx, wy, flip, ground, heart, power, glow, 1);
    let mut indices = vec![None; (s.cell.w * s.cell.h) as usize];
    let mut next = start;
    for y in 0..s.cell.h {
        for x in 0..s.cell.w {
            if !s.cell.at(x, y).a { continue; }
            if next < fx.particles.list.len() {
                indices[(y * s.cell.w + x) as usize] = Some(next);
                let p = &mut fx.particles.list[next];
                p.life = (p.life * 0.16).clamp(24.0, 38.0);
                p.max_life = p.life;
                p.settle = false;
                next += 1;
            }
        }
    }
    // Native source patches share one trajectory, including their landing and dark residue.
    for cy in (6..s.cell.h - 5).step_by(12) {
        for cx in (7..s.cell.w - 6).step_by(13) {
            if crate::util::hash2(cx, cy, 91) > 0.36 { continue; }
            let center = s.cell.at(cx, cy);
            if !center.a || center.emit || center.c.luma() < 0.06 { continue; }
            let Some(root) = indices[(cy * s.cell.w + cx) as usize] else { continue; };
            let origin = fx.particles.list[root];
            let size = (crate::util::hash2(cx, cy, 92) * 3.0) as usize;
            let (rx, ry) = [(3i32, 2i32), (5, 3), (6, 5)][size.min(2)];
            for dy in -ry..=ry {
                for dx in -rx..=rx {
                    // Chipped masonry planes in three sizes, rather than identical pebble lozenges.
                    if (dx == rx && dy < -ry / 2) || (dx < -rx + 2 && dy == ry) { continue; }
                    let (x, y) = (cx + dx, cy + dy);
                    let source = s.cell.at(x, y);
                    if !source.a || source.emit { continue; }
                    let Some(i) = indices[(y * s.cell.w + x) as usize] else { continue; };
                    let p = &mut fx.particles.list[i];
                    p.x = origin.x + (if flip { -dx } else { dx }) as f32;
                    p.y = origin.y + dy as f32;
                    p.z = origin.z;
                    p.vx = origin.vx;
                    p.vy = origin.vy;
                    p.vz = origin.vz;
                    p.grav = 0.18;
                    p.drag = 0.95;
                    p.bounce = 0.2;
                    p.life = 150.0;
                    p.max_life = p.life;
                    p.c0 = source.c;
                    p.c1 = S2;
                    p.glow = 0.0;
                    p.settle = true;
                }
            }
        }
    }
}

/// The last sliver of the prism: a tall diamond of light (drawn in code; the level's `shard`
/// sheet is used instead when it exists).
pub fn core_shard(f: &mut Frame, x: f32, y: f32, t: u32) {
    if f.anim_w(
        "shard",
        "spin",
        t,
        x,
        y + 8.0,
        &crate::gfx::DrawOpts::default(),
    ) {
        return;
    }
    let (sx, sy) = (f.sx(x), f.sy(y));
    let w = ((t as f32 * 0.12).sin().abs() * 3.0).round() as i32;
    for dy in -6i32..=6 {
        let half = (6 - dy.abs()) * w / 6;
        for dx in -half..=half {
            let c = if dx == 0 {
                PW
            } else if dx < 0 {
                P2
            } else {
                P1
            };
            f.put_glow(sx + dx, sy + dy, c);
        }
    }
    f.put_glow(sx, sy - 7, PW);
    f.put_glow(sx, sy + 7, PW);
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::boss::art;
    use crate::world::{Zone, ZoneId};

    /// The real body keeps burst RNG and initial mass while grains clear before the core rises.
    #[test]
    fn collapse_keeps_rng_and_clears_fine_grains() {
        let s = art::spr(art::BODY, &["idle"], 0, true);
        let mut old = Fx::new(7);
        let mut new = Fx::new(7);
        burst(&mut old, s, 300.0, 300.0, false, 300.0, (300.0, 240.0), 2.3, true, 1);
        collapse(&mut new, s, 300.0, 300.0, false, 300.0, (300.0, 240.0), 2.3, true);
        assert_eq!(old.rng.next_u32(), new.rng.next_u32());
        assert_eq!(old.particles.list.len(), new.particles.list.len());
        assert!(new.particles.list.iter().any(|p| p.settle && p.max_life == 150.0));
        assert!(new.particles.list.iter().all(|p| p.settle || p.life <= 38.0));
        let zone = Zone::new(ZoneId::Summit, 60, 40);
        for _ in 0..40 { new.particles.update(&zone); }
        assert!(new.particles.list.iter().all(|p| p.settle));
        for _ in 40..90 { new.particles.update(&zone); }
        assert!(!new.particles.debris.is_empty(), "coherent material must leave a quiet remnant");
        assert_eq!((FIST_L_CRUMBLE, FIST_R_CRUMBLE, BURST, RISE, DONE), (112, 134, 206, 262, 336));
    }
}
