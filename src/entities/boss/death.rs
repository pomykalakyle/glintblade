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
    burst_with(
        fx,
        s,
        wx,
        wy,
        flip,
        ground,
        heart,
        power,
        glow,
        stride,
        |fx, _, part| fx.particles.add(part),
    );
}

/// Construct particles in native source order, allowing collapse to fill already accepted slots.
#[allow(clippy::too_many_arguments)]
fn burst_with(
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
    mut emit: impl FnMut(&mut Fx, usize, Particle),
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
            emit(fx, (y * cell.w + x) as usize, part);
        }
    }
}

// One towering part must leave room for the head, crystals, and existing fist fragments.
// Actual free space is measured through Particles::add; its global cap stays owned by FX.
const COLLAPSE_PARTICLES: usize = 6_000;

struct CollapsePatch {
    center: (i32, i32),
    pixels: Vec<(i32, i32)>,
}

fn collapse_patches(s: Spr) -> Vec<CollapsePatch> {
    let mut patches = Vec::new();
    for cy in (6..s.cell.h - 5).step_by(12) {
        for cx in (7..s.cell.w - 6).step_by(13) {
            if crate::util::hash2(cx, cy, 91) > 0.36 {
                continue;
            }
            let center = s.cell.at(cx, cy);
            if !center.a || center.emit || center.c.luma() < 0.06 {
                continue;
            }
            let size = (crate::util::hash2(cx, cy, 92) * 3.0) as usize;
            let (rx, ry) = [(3i32, 2i32), (5, 3), (6, 5)][size.min(2)];
            let mut pixels = Vec::new();
            for dy in -ry..=ry {
                for dx in -rx..=rx {
                    // Chipped masonry planes in three sizes, rather than identical pebble lozenges.
                    if (dx == rx && dy < -ry / 2) || (dx < -rx + 2 && dy == ry) {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    let source = s.cell.at(x, y);
                    if source.a && !source.emit {
                        pixels.push((x, y));
                    }
                }
            }
            patches.push(CollapsePatch {
                center: (cx, cy),
                pixels,
            });
        }
    }
    patches
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
    fx: &mut Fx,
    s: Spr,
    wx: f32,
    wy: f32,
    flip: bool,
    ground: f32,
    heart: (f32, f32),
    power: f32,
    glow: bool,
) {
    let start = fx.particles.list.len();
    let opaque: Vec<_> = s
        .cell
        .px
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.a.then_some(i))
        .collect();
    // Reserve through the real admission path, including when other effects nearly fill it.
    // No placeholder survives: the accepted slots are filled from exact native source pixels.
    for _ in 0..opaque.len().min(COLLAPSE_PARTICLES) {
        let before = fx.particles.list.len();
        fx.particles.add(Particle::new(0.0, 0.0));
        if fx.particles.list.len() == before {
            break;
        }
    }
    let budget = fx.particles.list.len() - start;
    let mut selected = vec![false; (s.cell.w * s.cell.h) as usize];
    let mut patches = collapse_patches(s);
    if budget == opaque.len() {
        for &i in &opaque {
            selected[i] = true;
        }
    } else {
        // Keep complete native chunks before thinning grains. A fixed spatial hash spreads
        // the chunks over the part instead of spending their allowance on its first rows.
        patches.sort_by_key(|p| crate::util::hash2(p.center.0, p.center.1, 93).to_bits());
        let mut chunk_budget = budget / 3;
        patches.retain(|patch| {
            if patch.pixels.len() > chunk_budget {
                return false;
            }
            chunk_budget -= patch.pixels.len();
            for &(x, y) in &patch.pixels {
                selected[(y * s.cell.w + x) as usize] = true;
            }
            true
        });
        let chosen = selected.iter().filter(|&&p| p).count();
        let remaining = budget - chosen;
        let candidates = opaque.len() - chosen;
        // Integer error diffusion spreads the remaining grains over the entire source,
        // rather than filling the allowance from the part's first rows alone.
        let mut accumulator = candidates / 2;
        for &i in &opaque {
            if selected[i] {
                continue;
            }
            accumulator += remaining;
            if accumulator >= candidates {
                accumulator -= candidates;
                selected[i] = true;
            }
        }
    }
    let mut indices = vec![None; (s.cell.w * s.cell.h) as usize];
    let mut next = start;
    for &i in &opaque {
        if selected[i] {
            indices[i] = Some(next);
            next += 1;
        }
    }
    debug_assert_eq!(next, start + budget);
    burst_with(
        fx,
        s,
        wx,
        wy,
        flip,
        ground,
        heart,
        power,
        glow,
        1,
        |fx, source, mut p| {
            if let Some(i) = indices[source] {
                p.life = (p.life * 0.16).clamp(24.0, 38.0);
                p.max_life = p.life;
                p.settle = false;
                fx.particles.list[i] = p;
            }
        },
    );
    // Native source patches share one trajectory, including their landing and dark residue.
    for patch in patches {
        let (cx, cy) = patch.center;
        let Some(root) = indices[(cy * s.cell.w + cx) as usize] else {
            continue;
        };
        let origin = fx.particles.list[root];
        for (x, y) in patch.pixels {
            let (dx, dy) = (x - cx, y - cy);
            let source = s.cell.at(x, y);
            let Some(i) = indices[(y * s.cell.w + x) as usize] else {
                continue;
            };
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
    use crate::gfx::{Cell, Px};
    use crate::world::{Zone, ZoneId};
    use std::collections::{HashMap, HashSet};

    fn native_fixture(w: i32, h: i32) -> Spr {
        let px = (0..h)
            .flat_map(|y| {
                (0..w).map(move |x| Px {
                    c: Rgb::new(
                        (x + 1) as f32 / (w + 1) as f32,
                        (y + 1) as f32 / (h + 1) as f32,
                        0.4,
                    ),
                    a: x >= 2 && x < w - 2 && y >= 2 && y < h - 2,
                    emit: x % 31 == 15 && y % 13 < 4,
                })
            })
            .collect();
        Spr {
            cell: Box::leak(Box::new(Cell::new(w, h, px))),
            anchor: (w / 2, h - 1),
        }
    }

    fn fill_particles(fx: &mut Fx) -> usize {
        for _ in 0..50_000 {
            let before = fx.particles.list.len();
            fx.particles.add(Particle::new(-999.0, -999.0));
            if before == fx.particles.list.len() {
                return before;
            }
        }
        panic!("particle admission must have a finite cap");
    }

    fn stone_source(p: &Particle, s: Spr) -> (i32, i32) {
        let x = (p.c0.r * (s.cell.w + 1) as f32).round() as i32 - 1;
        let y = (p.c0.g * (s.cell.h + 1) as f32).round() as i32 - 1;
        (x, y)
    }

    /// Parts that fit retain the full burst and RNG; grains clear before the core rises.
    #[test]
    fn collapse_keeps_rng_and_clears_fine_grains() {
        let s = native_fixture(64, 56);
        let mut old = Fx::new(7);
        let mut new = Fx::new(7);
        burst(
            &mut old,
            s,
            300.0,
            300.0,
            false,
            300.0,
            (300.0, 240.0),
            2.3,
            true,
            1,
        );
        collapse(
            &mut new,
            s,
            300.0,
            300.0,
            false,
            300.0,
            (300.0, 240.0),
            2.3,
            true,
        );
        assert_eq!(old.rng.next_u32(), new.rng.next_u32());
        assert_eq!(old.particles.list.len(), new.particles.list.len());
        assert!(
            new.particles
                .list
                .iter()
                .any(|p| p.settle && p.max_life == 150.0)
        );
        assert!(
            new.particles
                .list
                .iter()
                .all(|p| p.settle || p.life <= 38.0)
        );
        let zone = Zone::new(ZoneId::Summit, 60, 40);
        for _ in 0..40 {
            new.particles.update(&zone);
        }
        assert!(new.particles.list.iter().all(|p| p.settle));
        for _ in 40..90 {
            new.particles.update(&zone);
        }
        assert!(
            !new.particles.debris.is_empty(),
            "coherent material must leave a quiet remnant"
        );
        assert_eq!(
            (FIST_L_CRUMBLE, FIST_R_CRUMBLE, BURST, RISE, DONE),
            (112, 134, 206, 262, 336)
        );
    }

    #[test]
    fn towering_collapse_covers_native_silhouette_and_keeps_connected_masonry() {
        let s = native_fixture(144, 160);
        let opaque = s.cell.px.iter().filter(|p| p.a).count();
        assert!(opaque > 12_000);
        for flip in [false, true] {
            let mut fx = Fx::new(19);
            let mut old = Fx::new(19);
            collapse(
                &mut fx,
                s,
                300.0,
                400.0,
                flip,
                400.0,
                (300.0, 280.0),
                2.3,
                true,
            );
            burst(
                &mut old,
                s,
                300.0,
                400.0,
                flip,
                400.0,
                (300.0, 280.0),
                2.3,
                true,
                1,
            );
            assert_eq!(fx.rng.next_u32(), old.rng.next_u32());
            assert_eq!(fx.particles.list.len(), COLLAPSE_PARTICLES);
            let (ox, oy) = origin(s.cell, s.anchor, 300, 400, flip);
            let mut quadrants = [0; 4];
            let mut sources = HashSet::new();
            let mut chunks: HashMap<_, Vec<_>> = HashMap::new();
            let mut emitted = 0;
            for p in &fx.particles.list {
                let (x, y) = if p.glow > 0.0 {
                    emitted += 1;
                    assert!(!p.settle);
                    assert_eq!(p.glow, 0.65);
                    let lx = p.x as i32 - ox;
                    let x = if flip { s.cell.w - 1 - lx } else { lx };
                    let y = (400.0 - p.z) as i32 - oy;
                    assert!(s.cell.at(x, y).emit);
                    assert!(SPECTRUM.contains(&p.c0));
                    (x, y)
                } else {
                    let (x, y) = stone_source(p, s);
                    let source = s.cell.at(x, y);
                    assert!(source.a && !source.emit);
                    assert_eq!(p.c0, source.c);
                    assert_eq!(p.x, (ox + if flip { s.cell.w - 1 - x } else { x }) as f32);
                    assert!((p.y - p.z - (oy + y) as f32).abs() <= 2.5);
                    if p.settle {
                        assert_eq!(p.life, 150.0);
                        chunks
                            .entry((
                                p.vx.to_bits(),
                                p.vy.to_bits(),
                                p.vz.to_bits(),
                                p.z.to_bits(),
                            ))
                            .or_default()
                            .push((x, y));
                    }
                    (x, y)
                };
                assert!(
                    sources.insert((x, y)),
                    "one accepted particle per selected native pixel"
                );
                quadrants[(y >= s.cell.h / 2) as usize * 2 + (x >= s.cell.w / 2) as usize] += 1;
                assert_eq!(p.size, 1);
                assert_eq!(p.life, p.max_life);
                assert!(p.settle || (24.0..=38.0).contains(&p.life));
                assert!(
                    [p.x, p.y, p.z, p.vx, p.vy, p.vz]
                        .iter()
                        .all(|v| v.is_finite())
                );
            }
            assert!(emitted > 0, "authored emitters survive source sampling");
            assert!(
                quadrants.iter().all(|&n| n > 500),
                "whole-body coverage: {quadrants:?}"
            );
            assert!(sources.iter().any(|&(_, y)| y < 5));
            assert!(sources.iter().any(|&(_, y)| y >= s.cell.h - 5));
            assert!(sources.iter().any(|&(x, _)| x < 5));
            assert!(sources.iter().any(|&(x, _)| x >= s.cell.w - 5));
            assert!(
                chunks.values().any(|group| {
                    if group.len() < 9 {
                        return false;
                    }
                    let mut remaining: HashSet<_> = group.iter().copied().collect();
                    let mut connected = vec![group[0]];
                    remaining.remove(&group[0]);
                    let mut i = 0;
                    while i < connected.len() {
                        let (x, y) = connected[i];
                        for neighbour in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                            if remaining.remove(&neighbour) {
                                connected.push(neighbour);
                            }
                        }
                        i += 1;
                    }
                    remaining.is_empty()
                }),
                "sampling retains a connected native masonry patch"
            );
        }
    }

    #[test]
    fn collapse_respects_actual_remaining_capacity_and_existing_particles() {
        let s = native_fixture(144, 160);
        for free in [0, 1, 3, 47] {
            let mut fx = Fx::new(23);
            let capacity = fill_particles(&mut fx);
            fx.particles.list.truncate(capacity - free);
            let start = fx.particles.list.len();
            collapse(
                &mut fx,
                s,
                300.0,
                400.0,
                false,
                400.0,
                (300.0, 280.0),
                2.3,
                true,
            );
            assert_eq!(fx.particles.list.len(), capacity);
            assert!(
                fx.particles.list[..start]
                    .iter()
                    .all(|p| p.x == -999.0 && p.y == -999.0)
            );
            assert!(
                fx.particles.list[start..]
                    .iter()
                    .all(|p| p.x > 0.0 && p.max_life >= 24.0)
            );
            if free >= 3 {
                assert!(fx.particles.list[start..].iter().any(|p| p.z > 100.0));
                assert!(fx.particles.list[start..].iter().any(|p| p.z < 60.0));
            }
        }
    }

    #[test]
    fn towering_body_leaves_capacity_for_authored_head_and_crystals() {
        let mut fx = Fx::new(29);
        let capacity = fill_particles(&mut fx);
        fx.particles.clear();
        let body = native_fixture(144, 160);
        collapse(
            &mut fx,
            body,
            300.0,
            400.0,
            false,
            400.0,
            (300.0, 280.0),
            2.3,
            true,
        );
        assert_eq!(fx.particles.list.len(), COLLAPSE_PARTICLES);
        for name in [art::HEAD, art::CRYSTAL, art::CRYSTAL] {
            let s = art::spr(name, &["idle"], 0, true);
            let expected = s
                .cell
                .px
                .iter()
                .filter(|p| p.a)
                .count()
                .min(COLLAPSE_PARTICLES);
            let before = fx.particles.list.len();
            collapse(
                &mut fx,
                s,
                300.0,
                260.0,
                false,
                400.0,
                (300.0, 280.0),
                2.3,
                true,
            );
            assert_eq!(
                fx.particles.list.len() - before,
                expected,
                "room for {name}"
            );
        }
        assert!(fx.particles.list.len() <= capacity);
    }
}
