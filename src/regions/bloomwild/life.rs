//! Water stays within the hazard bed, breaks over stone lips and falls toward the cloud sea.

use super::{Map, mat, pal};
use crate::gfx::{Frame, H, Rgb, W, rgb};
use crate::util::hash2;
use crate::world::{RegionCtx, Zone};

// Clear mineral water keeps a blue-green body beneath the warm chalk and moss.
// These are surface colours, not emitters: terrace shadows still cross the river.
const RIVER: [Rgb; 5] = [
    rgb(0x3b6473),
    rgb(0x568894),
    rgb(0x85b6b7),
    rgb(0xb9d6cb),
    rgb(0xe4ecda),
];

pub struct Life {
    wakes: Vec<Wake>,
    outlet: bool,
}

struct Wake {
    x: i32,
    y: i32,
    born: u64,
}

/// Shape a continuous spill into unequal hanging strands, then lose it in the cloud shelf.
fn south_span(y: i32) -> (i32, i32) {
    let dy = y - 736;
    let left = 816 + if dy < 12 { 0 } else { (dy / 17).min(4) };
    let right = 864 - if dy < 23 { 0 } else { (dy / 13).min(6) };
    (left, right)
}

impl Life {
    /// Continue the existing three-tile outlet over the island face without changing collision.
    pub fn build(m: &Map, mask: &mut [u8], z: &mut Zone) -> Self {
        let outlet = z.pw > 866 && z.ph >= 823 && mat::water(mask[(735 * z.pw + 840) as usize]);
        if !outlet {
            return Self {
                wakes: Vec::new(),
                outlet,
            };
        }
        // The distant cloud shelf overlaps the lowest backing rock, between the hanging strands.
        for y in 782..z.ph {
            for x in 814..866 {
                if m.at(x / 16, y / 16) == super::K::Void {
                    if y >= 790 {
                        mask[(y * z.pw + x) as usize] = mat::VOID;
                    }
                    z.gset(x, y, crate::world::HOLE);
                }
            }
        }
        for y in 46 * 16..z.ph.min(820) {
            let (left, right) = south_span(y);
            for x in left..right {
                let tx = x / 16;
                let bottom = (0..m.th)
                    .rev()
                    .find(|&y| m.at(tx, y) != super::K::Void)
                    .unwrap_or(45);
                if y < (bottom + 1) * 16 {
                    continue;
                }
                // Below the rock, gaps expose distant clouds rather than a rectangular water wall.
                let split = y > 787 && ((830..833).contains(&x) || (845..848).contains(&x));
                let end = 813 - ((x - 816) / 7 % 3) * 4;
                if split || y > end {
                    continue;
                }
                mask[(y * z.pw + x) as usize] = mat::BIGFALLS;
                // The outlet is an eroded opening through the decorative rock skirt.
                // An empty backing lets separated ribbons reveal the actual cloud sea.
                z.gset(x, y, crate::world::HOLE);
            }
        }
        Self {
            wakes: Vec::new(),
            outlet,
        }
    }

    /// Feet disturb the same shallow surface that movement uses. This bounded pool
    /// does not emit ambient particles or leave rings behind on bridges and dry banks.
    pub fn update(&mut self, z: &Zone, mask: &[u8], ctx: &RegionCtx) {
        self.wakes
            .retain(|wake| ctx.t.saturating_sub(wake.born) < 30);
        if !ctx.t.is_multiple_of(8) {
            return;
        }
        for &(x, y, speed) in ctx.movers {
            let (x, y) = (x.floor() as i32, y.floor() as i32);
            if speed < 0.15 || x < 0 || y < 0 || x >= z.pw || y >= z.ph {
                continue;
            }
            if mask[(y * z.pw + x) as usize] != mat::SHALLOW {
                continue;
            }
            if self.wakes.len() == 48 {
                self.wakes.remove(0);
            }
            self.wakes.push(Wake { x, y, born: ctx.t });
        }
    }

    /// The near side of the surface sits in front of the hero's soles while wading.
    /// This stays at the foot contact, so dry bridge decks and the full body silhouette
    /// remain clear; the larger wake rings are drawn underneath the actor.
    pub fn draw_foot_contact(
        &self,
        z: &Zone,
        mask: &[u8],
        f: &mut Frame,
        player: (f32, f32),
        t: u64,
    ) {
        let (x, y) = (player.0.floor() as i32, player.1.floor() as i32);
        if x < 4
            || y < 1
            || x + 4 >= z.pw
            || y + 1 >= z.ph
            || mask[(y * z.pw + x) as usize] != mat::SHALLOW
        {
            return;
        }
        for dx in -4_i32..=4 {
            let yy = y + i32::from(dx.abs() > 2);
            if mask[(yy * z.pw + x + dx) as usize] != mat::SHALLOW {
                continue;
            }
            let c = if (dx + (t / 9) as i32).rem_euclid(5) == 0 {
                RIVER[2]
            } else {
                RIVER[1]
            };
            f.put(x + dx - f.cam_x, yy - f.cam_y, c);
        }
    }

    /// Animate coherent currents, source-bound impact rings and downward ribbons.
    pub fn draw_water(&self, z: &Zone, mask: &[u8], f: &mut Frame, t: u64) {
        for sy in 0..H as i32 {
            let y = sy + f.cam_y;
            if y < 0 || y >= z.ph {
                continue;
            }
            for sx in 0..W as i32 {
                let x = sx + f.cam_x;
                if x < 0 || x >= z.pw {
                    continue;
                }
                let g = mask[(y * z.pw + x) as usize];
                if !mat::water(g) {
                    continue;
                }
                let c = if matches!(g, mat::FALLS | mat::BIGFALLS) {
                    falling(x, y, t, g == mat::BIGFALLS)
                } else {
                    Some(surface(z, mask, x, y, t, g))
                };
                if let Some(c) = c {
                    f.put(sx, sy, c);
                }
            }
        }
        // Only the fall's terminal spray occupies the air, merging into the same
        // cloud shelf, with no independent mist layer or persistent particles.
        for y in 779..if self.outlet { 823 } else { 779 } {
            let depth = y - 779;
            let radius = 20 + depth / 4;
            for x in 840 - radius..=840 + radius {
                if !f.visible(x as f32, y as f32, 1.0, 1.0, 0.0) {
                    continue;
                }
                let g = mask[(y * z.pw + x) as usize];
                if !matches!(g, mat::VOID | mat::EDGE | mat::BIGFALLS) {
                    continue;
                }
                let drift = ((t / 3) as i32 + depth * 2).rem_euclid(31);
                let strand = (x - 817 + depth / 6).rem_euclid(13);
                let fade = (823 - y).min(16) as f32 / 16.0;
                let coverage = fade * (1.0 - (x - 840).abs() as f32 / radius as f32) * 0.55;
                if strand < 4 && drift < 10 && crate::util::bayer4(x, y) < coverage {
                    let c = if y < super::sky::shelf_top(x) {
                        RIVER[2]
                    } else {
                        pal::CLOUD[1]
                    };
                    f.put(x - f.cam_x, y - f.cam_y, c);
                }
            }
        }
        for wake in &self.wakes {
            let age = t.saturating_sub(wake.born) as i32;
            if age >= 30 {
                continue;
            }
            let rx = 2 + age / 3;
            let ry = 1 + age / 8;
            for dy in -ry..=ry {
                for dx in -rx..=rx {
                    let (x, y) = (wake.x + dx, wake.y + dy);
                    if x < 0 || y < 0 || x >= z.pw || y >= z.ph {
                        continue;
                    }
                    if !matches!(mask[(y * z.pw + x) as usize], mat::SHALLOW | mat::DEEP) {
                        continue;
                    }
                    let ellipse = dx * dx * ry * ry + dy * dy * rx * rx;
                    let outer = rx * rx * ry * ry;
                    let inner = (rx - 1) * (rx - 1) * ry * ry;
                    if ellipse > outer || ellipse < inner || (dx > 0 && dy < 0) {
                        continue;
                    }
                    if age > 16 && (dx + dy).rem_euclid(3) == 0 {
                        continue;
                    }
                    let color = if dy <= 0 { RIVER[2] } else { RIVER[1] };
                    f.put(x - f.cam_x, y - f.cam_y, color);
                }
            }
        }
        // A shallow arched underside hangs below each deck. Its feet stay on the
        // dry banks; only the suspended ring overlaps water. The opening remains
        // visibly water, including its bridge shadow and moving downstream current.
        for (bx, by) in [(656, 256), (784, 576)] {
            for x in bx..bx + 112 {
                for dy in 0..6 {
                    let y = by + 32 + dy;
                    if !mat::water(mask[(y * z.pw + x) as usize]) {
                        continue;
                    }
                    let from_mid = (x - (bx + 56)).abs();
                    let depth = 2 + ((from_mid - 6).max(0) / 4).min(4);
                    if dy >= depth {
                        continue;
                    }
                    let c = if dy == 0 {
                        pal::STONE[2]
                    } else if dy == depth - 1 {
                        pal::STONE[0]
                    } else {
                        pal::STONE[1]
                    };
                    f.put(x - f.cam_x, y - f.cam_y, c);
                }
            }
        }
    }
}

/// Cool connected water holds the volume; narrow unequal strands carry the light.
fn falling(x: i32, y: i32, t: u64, south: bool) -> Option<Rgb> {
    let (origin, left) = if south { (736, 816) } else { (320, 688) };
    let dy = y - origin;
    let lx = x - left;
    let curl = 2 + (lx - 23).abs() / 13;
    if dy < curl + 3 {
        return Some(if dy < curl {
            RIVER[2]
        } else if dy < curl + 1 {
            RIVER[4]
        } else {
            RIVER[2]
        });
    }
    let depth = dy - curl - 3;
    if south && depth > 34 {
        // The long spill keeps unequal tapered volumes, with genuine sky between
        // their ends. The short terrace fall remains one connected body.
        let inside =
            [(8, 10, 75), (25, 11, 82), (41, 7, 69)]
                .into_iter()
                .any(|(center, width, end)| {
                    let width = (width - (depth - 27).max(0) / 7).max(2);
                    (lx - center).abs() <= width
                        && crate::util::bayer4(x, y) < ((end - dy) as f32 / 18.0).clamp(0.0, 1.0)
                });
        if !inside {
            return None;
        }
    }
    let edge = 2 + depth / if south { 25 } else { 18 };
    let mut color = if lx < edge || lx > 47 - edge {
        RIVER[1]
    } else {
        RIVER[1].lerp(RIVER[2], 0.66)
    };
    let travel = depth * 112 / (depth + 112);
    // Unequal 1–3px runs descend at different phases. Their shallow bends are
    // fixed in the fall, so movement reads downward rather than sideways sway.
    for (i, (center, half, lean)) in [
        (5, 0, 1),
        (12, 1, -1),
        (21, 0, 1),
        (28, 1, -1),
        (37, 1, 1),
        (44, 0, -1),
    ]
    .into_iter()
    .enumerate()
    {
        let center = center
            + lean * depth / (22 + i as i32 * 4)
            + ((depth + i as i32 * 5) / 11).rem_euclid(2);
        let dx = lx - center;
        let phase = (travel * 2 - t as i32 + [0, 19, 37, 8, 49, 27][i]).rem_euclid(67);
        if dx >= -3 && dx < -half - 1 {
            color = RIVER[1].lerp(RIVER[2], 0.30);
        }
        if dx.abs() > half {
            continue;
        }
        color = if (8..12).contains(&phase) && dx == 0 && depth > 2 {
            RIVER[4]
        } else if phase < 34 {
            RIVER[3]
        } else {
            RIVER[2]
        };
    }
    Some(color)
}

/// Keep deep water dark, show bank undercuts and attach foam only to the terrace impact.
fn surface(z: &Zone, mask: &[u8], x: i32, y: i32, t: u64, g: u8) -> crate::gfx::Rgb {
    let wet = |xx: i32, yy: i32| {
        xx >= 0 && yy >= 0 && xx < z.pw && yy < z.ph && mat::water(mask[(yy * z.pw + xx) as usize])
    };
    // Depth classification comes from the same native mask used by movement. Surface
    // shading may vary within a shelf, but never depicts shallow ground as deep water.
    let bank = !wet(x - 2, y) || !wet(x + 2, y) || !wet(x, y - 2);
    let mut c = if bank || g == mat::DEEP {
        RIVER[0]
    } else {
        RIVER[1]
    };
    // Unequal submerged shelves break up the shallow bank strip.
    // These remain blue water throughout; no stone or grass enters the hazard footprint.
    if !bank {
        let bend = ((y / 23) % 4 - 2).abs() * 2;
        let reach = if x < 840 { 7 + bend } else { 4 + bend };
        let shore = (3..=reach).any(|d| !wet(x - d, y) || !wet(x + d, y));
        if shore && g == mat::SHALLOW {
            c = if (768..912).contains(&x) && (608..736).contains(&y) {
                let west = (3..=reach).any(|d| !wet(x - d, y));
                if !west && (y + x / 19 * 5).rem_euclid(47) < 29 {
                    RIVER[1].lerp(RIVER[2], 0.45)
                } else {
                    RIVER[1]
                }
            } else {
                RIVER[1].lerp(RIVER[2], 0.45)
            };
        }
    }
    // A few broad, low-contrast sky catches establish a horizontal water plane.
    // Each pool has its own composition; the narrow river keeps only one small
    // reflected interval. All remain dark blue water inside the physical deep bed.
    if g == mat::DEEP && !bank {
        for (top, bottom, left, right, slope) in [
            (132, 145, 648, 761, 2),
            (175, 181, 680, 786, -2),
            (376, 385, 650, 762, 2),
            (658, 667, 811, 859, -1),
        ] {
            if (top..bottom).contains(&y) {
                let d = y - top;
                let trim = if d < 2 {
                    12 - d * 5
                } else if y + 2 >= bottom {
                    (y + 3 - bottom) * 5
                } else {
                    0
                };
                if x >= left + d * slope + trim
                    && x < right + d * slope - trim
                    && wet(x - 4, y)
                    && wet(x + 4, y)
                {
                    c = c.lerp(RIVER[1], 0.18);
                }
            }
        }
    }
    // Horizontal glints stretch across a southward current. Unequal runs share a
    // slow downstream phase, with no per-frame noise or independent sparkling dots.
    let drift = y - (t / 9) as i32;
    let row = drift.div_euclid(53);
    let offset = (hash2(row, 0, 418) * 53.0) as i32;
    let px = (x + offset).rem_euclid(89);
    let py = drift.rem_euclid(53);
    let length = 12 + (hash2(row, 1, 418) * 15.0) as i32;
    if !bank && ((py == 0 && (8..8 + length).contains(&px)) || (py == 2 && (5..12).contains(&px))) {
        c = if g == mat::DEEP { RIVER[1] } else { RIVER[2] };
    }
    // The curtain actually strikes this basin. A low pale impact pad meets the
    // falling face, then two incomplete elliptical crests spread into deeper water.
    if (656..772).contains(&x) && (352..396).contains(&y) {
        let dx = x - 712;
        let dy = y - 352;
        let radius = ((dx * dx + dy * dy * 12) as f32).sqrt() as i32;
        let wave = (radius - (t / 6) as i32).rem_euclid(41);
        if dy > 5 && wave < 2 && radius < 74 && (dx < -16 || dx > 9) {
            c = RIVER[2];
        }
        let foot = 6 - dx.abs() / 5;
        if dx.abs() < 28 && dy < foot {
            let broken = (dx + (t / 7) as i32).rem_euclid(23);
            c = if dy < 2 || broken < 15 {
                RIVER[4]
            } else {
                RIVER[3]
            };
        } else if (3..9).contains(&dy) && (dx + (t / 11) as i32).rem_euclid(31) < 7 && dx.abs() < 34
        {
            c = RIVER[3];
        }
    }
    // At the island edge water rolls over a visible bright comb, without foam on dry banks.
    if (816..864).contains(&x) && (730..736).contains(&y) {
        let dy = y - 730;
        let turn = 2 + (x - 840).abs() / 10;
        c = if dy >= turn && dy < turn + 2 {
            RIVER[4]
        } else if dy >= turn + 2 {
            RIVER[2]
        } else if (x + dy / 2).rem_euclid(15) < 9 {
            RIVER[3]
        } else {
            RIVER[1]
        };
    }
    // Retain the narrow under-deck contact. Longer directional casts belong to
    // the moving sun, rather than a second fixed stripe painted into the water.
    let legacy = crate::gfx::lighting::legacy_mode();
    for (bx, by) in [(656, 256), (784, 576)] {
        let dy = y - (by + 32);
        if (bx..bx + 112).contains(&x) && (0..if legacy { 15 } else { 3 }).contains(&dy) {
            c = c.scale(if legacy { 0.70 + dy as f32 * 0.02 } else { 0.82 + dy as f32 * 0.06 });
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_fall_remains_a_connected_curtain_and_only_long_outlet_separates() {
        for t in [0, 30, 60, 90, 120] {
            for y in 320..352 {
                for x in 688..736 {
                    assert!(
                        falling(x, y, t, false).is_some(),
                        "hole in upper curtain at {x},{y}, t={t}"
                    );
                }
            }
            assert!(
                (816..864).any(|x| falling(x, 793, t, true).is_none()),
                "long fall must expose air between its terminal ribbons"
            );
            assert!(
                (816..864).any(|x| falling(x, 790, t, true).is_some()),
                "outlet cannot disappear at one horizontal cutoff"
            );
        }
    }

    #[test]
    fn clear_water_changes_never_paint_dry_banks_or_bridge_decks() {
        let m = Map::parse();
        let mut z = Zone::new(crate::world::ZoneId::Bloomwild, m.tw, m.th);
        let mut mask = crate::regions::bloomwild::paint::paint(&mut z, &m);
        let life = Life::build(&m, &mut mask, &mut z);
        assert!(life.outlet, "real stream must feed the southern fall");
        for (cx, cy) in [(520, 248), (624, 488), (620, 600)] {
            let mut f = Frame::new();
            f.begin(Rgb::WHITE, Rgb::WHITE);
            f.cam_x = cx;
            f.cam_y = cy;
            life.draw_water(&z, &mask, &mut f, 90);
            for sy in 0..H {
                for sx in 0..W {
                    let (x, y) = (cx + sx as i32, cy + sy as i32);
                    if y >= 779 {
                        continue;
                    } // Authored terminal mist is deliberately airborne.
                    let g = mask[(y * z.pw + x) as usize];
                    if !mat::water(g) {
                        assert_eq!(
                            f.albedo[sy * W + sx],
                            Rgb::WHITE,
                            "water escaped its physical bed at {x},{y}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn wading_wakes_are_bounded_and_never_spawn_on_dry_bridge_or_deep_water() {
        let mut z = Zone::new(crate::world::ZoneId::Bloomwild, 2, 2);
        let m = Map {
            tw: 2,
            th: 2,
            k: vec![super::super::K::Grass; 4],
            level: vec![0; 4],
            marks: vec![],
        };
        let mut mask = vec![mat::GRASS; (z.pw * z.ph) as usize];
        mask[10 * z.pw as usize + 10] = mat::SHALLOW;
        mask[10 * z.pw as usize + 12] = mat::DEEP;
        mask[10 * z.pw as usize + 14] = mat::BRIDGE;
        let mut life = Life::build(&m, &mut mask, &mut z);
        let movers = [
            (10.0, 10.0, 1.0),
            (12.0, 10.0, 1.0),
            (14.0, 10.0, 1.0),
            (16.0, 10.0, 1.0),
        ];
        for t in 0..512 {
            life.update(
                &z,
                &mask,
                &RegionCtx {
                    t,
                    player: (10.0, 10.0),
                    movers: &movers,
                    cam: (0, 0),
                },
            );
        }
        assert!(!life.wakes.is_empty());
        assert!(life.wakes.len() <= 4);
        assert!(life.wakes.iter().all(|w| w.x == 10 && w.y == 10));
        life.update(
            &z,
            &mask,
            &RegionCtx {
                t: 600,
                player: (10.0, 10.0),
                movers: &[],
                cam: (0, 0),
            },
        );
        assert!(
            life.wakes.is_empty(),
            "old foot contacts decay when the water is left alone"
        );
    }

    /// Water keeps its exact native coordinates and depth while the dry coast
    /// deliberately gains rounded corners. Retain the meadow continuity check too.
    #[test]
    fn signature_material_footprint_and_meadow_continuity() {
        let m = Map::parse();
        let mut z = Zone::new(crate::world::ZoneId::Bloomwild, m.tw, m.th);
        let mut mask = crate::regions::bloomwild::paint::paint(&mut z, &m);
        Life::build(&m, &mut mask, &mut z);
        let hash = mask
            .iter()
            .enumerate()
            .filter(|(_, v)| mat::water(**v))
            .fold(0xcbf29ce484222325u64, |h, (i, &v)| {
                (i as u32)
                    .to_le_bytes()
                    .into_iter()
                    .chain([v])
                    .fold(h, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
            });
        assert_eq!(
            hash, 0xe0476554b34f36ed,
            "preserve every wet pixel's exact coordinate and depth classification"
        );
        for y in [640, 680, 720] {
            assert_eq!(z.gget(431, y), z.gget(432, y), "unmotivated seam at y={y}");
        }
    }

    /// Time live movement, attacks, actors and the complete native render at four Bloomwild views.
    #[test]
    fn signature_live_game_stays_inside_eight_millisecond_budget() {
        use crate::game::{Game, Options, Scene};
        use crate::input::{Input, Raw};
        use crate::world::ZoneId;
        for (name, pos) in [
            ("opening", (152.0, 696.0)),
            ("falls", (780.0, 700.0)),
            ("edge_ruin", (330.0, 700.0)),
            ("terrace", (712.0, 350.0)),
        ] {
            let mut game = Game::new(Options {
                start: Some(ZoneId::Bloomwild),
                god: true,
                ..Options::default()
            });
            if let Scene::Play(p) = &mut game.scene {
                p.player.respawn(pos.0, pos.1);
                p.checkpoint = pos;
                p.cam.snap(pos.0, pos.1);
            }
            let mut input = Input::default();
            game.render(); // Exclude the one-time full-zone lighting bake.
            let mut times = Vec::new();
            for t in 0..240 {
                input.step(Raw {
                    mv: if t < 40 {
                        (1.0, 0.0)
                    } else if t < 80 {
                        (-1.0, 0.0)
                    } else {
                        (0.0, 0.0)
                    },
                    attack: t % 60 < 3 || (120..180).contains(&t),
                    ..Raw::default()
                });
                let started = std::time::Instant::now();
                game.tick(&input);
                game.fx.sounds.clear();
                game.render();
                times.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            let mean = times.iter().sum::<f64>() / times.len() as f64;
            let p95 = times[times.len() * 95 / 100];
            println!(
                "Bloomwild live {name}: mean {mean:.3} ms; p95 {p95:.3} ms; max {:.3} ms",
                times[times.len() - 1]
            );
            assert!(
                mean <= 8.0 && p95 <= 8.0,
                "{name}: mean {mean:.3}, p95 {p95:.3} ms"
            );
        }
    }
}
