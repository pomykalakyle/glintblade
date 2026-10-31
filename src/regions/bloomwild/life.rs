//! Water stays within the hazard bed, breaks over stone lips and falls toward the cloud sea.

use super::{Map, mat, pal};
use crate::gfx::{Frame, H, W};
use crate::util::hash2;
use crate::world::Zone;

pub struct Life;

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
        Self
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
        for y in 779..823 {
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
                        pal::WATER[2]
                    } else {
                        pal::CLOUD[1]
                    };
                    f.put(x - f.cam_x, y - f.cam_y, c);
                }
            }
        }
        // Bridge fronts remain in front of the moving water, with their original deck bounds.
        for (x, y, w) in [
            (41 * 16, 18 * 16 - 3, 7 * 16),
            (49 * 16, 38 * 16 - 3, 7 * 16),
        ] {
            f.rect(x - f.cam_x, y - f.cam_y, w, 3, pal::STONE[1]);
            f.rect(x - f.cam_x, y - f.cam_y - 1, w, 1, pal::STONE[5]);
        }
    }
}

/// Turn the lip into unequal ribbons that accelerate, narrow and dissolve at cloud depth.
fn falling(x: i32, y: i32, t: u64, south: bool) -> Option<crate::gfx::Rgb> {
    let (origin, left) = if south { (736, 816) } else { (320, 688) };
    let dy = y - origin;
    let lx = x - left;
    // The convex crest turns through a chalk highlight into a cool falling face.
    let curl = 2 + (lx - 23).abs() / 9;
    if dy < curl + 3 {
        return Some(if dy < curl - 1 {
            pal::WATER[3]
        } else if dy < curl + 1 {
            pal::WATER[4]
        } else {
            pal::WATER[2]
        });
    }
    let length = if south { 83 } else { 34 };
    let mut color = None;
    for (i, (center, half, lean)) in [(6, 6, 1), (18, 7, -1), (31, 8, 1), (43, 5, -1)]
        .into_iter()
        .enumerate()
    {
        let i = i as i32;
        let bend = lean * (dy / (19 + i * 3));
        let center = center + bend;
        let width = (half - dy / (15 + i * 4)).max(1);
        let dx = lx - center;
        if dx.abs() > width {
            continue;
        }
        // Mapping distance into a compressed phase makes each moving segment
        // visibly stretch downwards, with different timing for each ribbon.
        let travel = dy * 96 / (dy + 96);
        let phase = (travel - (t as i32 * (2 + i % 2)) + i * 19 + dx * 2).rem_euclid(59 + i * 7);
        let end = length - [3, 12, 0, 8][i as usize];
        let fade = if south {
            ((end - dy) as f32 / 23.0).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if crate::util::bayer4(x, y) >= fade {
            continue;
        }
        let bright_width = (width * 2 / 3 - phase / 12).max(1);
        let c = if dx == -width {
            pal::WATER[1]
        } else if dx < 0 && dx >= -bright_width && phase < 32 {
            pal::WATER[4]
        } else if phase < 44 && dx < width - 1 {
            pal::WATER[3]
        } else {
            pal::WATER[2]
        };
        // Each ribbon meets haze at a different depth; ordered coverage avoids
        // a horizontal value cut where solid water becomes terminal spray.
        let haze = ((dy - 42 - i * 4) as f32 / 25.0).clamp(0.0, 1.0);
        color = Some(if south && crate::util::bayer4(x + 1, y) < haze {
            if y < super::sky::shelf_top(x) {
                pal::WATER[2]
            } else {
                pal::CLOUD[1]
            }
        } else {
            c
        });
    }
    // The upper curtain stays connected; only its lower portion separates.
    color.or_else(|| {
        if dy < 21 || !south {
            Some(pal::WATER[2])
        } else {
            None
        }
    })
}

/// Keep deep water dark, show bank undercuts and attach foam only to the terrace impact.
fn surface(z: &Zone, mask: &[u8], x: i32, y: i32, t: u64, mut g: u8) -> crate::gfx::Rgb {
    let wet = |xx: i32, yy: i32| {
        xx >= 0 && yy >= 0 && xx < z.pw && yy < z.ph && mat::water(mask[(yy * z.pw + xx) as usize])
    };
    // Keep the walkable shelves bright, but let their inner boundaries vary by up to four pixels.
    // Only the rendered color changes; the shallow/deep hazard mask remains exact.
    if g == mat::SHALLOW && (768..912).contains(&x) && (608..736).contains(&y) {
        let west_run = match (y - 608).rem_euclid(89) { 0..=16 => 0, 17..=27 => 2, 28..=51 => 4, 52..=60 => 1, 61..=78 => 3, _ => 0 };
        let east_run = match (y - 608).rem_euclid(101) { 0..=20 => 0, 21..=34 => 3, 35..=55 => 1, 56..=82 => 4, _ => 2 };
        let deep = |xx: i32| mask[(y * z.pw + xx) as usize] == mat::DEEP;
        if (1..=west_run).any(|d| deep(x + d)) || (1..=east_run).any(|d| deep(x - d)) { g = mat::DEEP; }
    }
    let bank = !wet(x - 2, y) || !wet(x + 2, y) || !wet(x, y - 2);
    let mut c = if bank || g == mat::DEEP {
        pal::WATER[0]
    } else {
        pal::WATER[1]
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
                if !west && (y + x / 19 * 5).rem_euclid(47) < 29 { pal::WATER[2] }
                else { pal::WATER[1] }
            } else { pal::WATER[2] };
        }
    }
    // A few long, staggered currents follow the river, leaving most of its surface unmarked.
    let drift = y - (t / 8) as i32;
    let row = drift.div_euclid(37);
    let offset = (hash2(row, 0, 418) * 37.0) as i32;
    let px = (x + offset).rem_euclid(67);
    let py = drift.rem_euclid(37);
    if !bank && ((py == 0 && (8..24).contains(&px)) || (py == 1 && (5..9).contains(&px))) {
        c = if g == mat::DEEP {
            pal::WATER[1]
        } else {
            pal::WATER[2]
        };
    }
    // The terrace waterfall strikes at 712,355; broken elliptical crests spread downstream.
    if (664..764).contains(&x) && (352..400).contains(&y) {
        let dx = x - 712;
        let dy = y - 355;
        let radius = ((dx * dx + dy * dy * 9) as f32).sqrt() as i32;
        let wave = (radius - (t / 4) as i32).rem_euclid(27);
        if dy > 1 && wave < 2 && radius < 68 && !(-7..=10).contains(&dx) {
            c = pal::WATER[2];
        }
        let crest = (dx + (t / 5) as i32).rem_euclid(19);
        if dx.abs() < 24 && (0..8 - dx.abs() / 4).contains(&dy) {
            c = if crest < 12 {
                pal::WATER[4]
            } else {
                pal::WATER[2]
            };
        }
    }
    // At the island edge water rolls over a visible bright comb, without foam on dry banks.
    if (816..864).contains(&x) && (730..736).contains(&y) {
        let dy = y - 730;
        let turn = 2 + (x - 840).abs() / 10;
        c = if dy >= turn && dy < turn + 2 {
            pal::WATER[4]
        } else if dy >= turn + 2 {
            pal::WATER[2]
        } else if (x + dy / 2).rem_euclid(15) < 9 {
            pal::WATER[3]
        } else {
            pal::WATER[1]
        };
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Retain every pre-pass material byte while removing the arbitrary open-meadow value seam.
    #[test]
    fn signature_material_footprint_and_meadow_continuity() {
        let m = Map::parse();
        let mut z = Zone::new(crate::world::ZoneId::Bloomwild, m.tw, m.th);
        let mut mask = crate::regions::bloomwild::paint::paint(&mut z, &m);
        Life::build(&m, &mut mask, &mut z);
        // Recorded from an exact byte-for-byte comparison with the frozen pre-pass painters.
        let hash = mask.iter().fold(0xcbf29ce484222325u64, |h, &v| {
            (h ^ v as u64).wrapping_mul(0x100000001b3)
        });
        assert_eq!(
            hash, 0x52f2b0b1e1085a34,
            "preserve all 1,118,208 material bytes"
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
