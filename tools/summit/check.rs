//! Focused headless checks for the Summit's route, composition and hook cost.

use glintblade::fx::Fx;
use glintblade::gfx::{Frame, H, W};
use glintblade::regions::summit;
use glintblade::world::{HOLE, RegionCtx, TILE, Tile};
use std::collections::VecDeque;
use std::hint::black_box;
use std::time::Instant;

/// Check the actual zone data and region hooks without opening a game window.
fn main() {
    let (mut zone, mut region) = summit::build();
    assert!(zone.boss);
    assert_eq!(zone.boss_pos, Some((720.0, 352.0)));
    assert!(zone.enemies.is_empty() && zone.shard.is_none() && zone.gate.is_none());
    assert_eq!(zone.grade.exposure, 1.0);
    assert_eq!(zone.grade.contrast, 1.0);
    assert_eq!(zone.grade.saturation, 1.0);
    assert!(zone.ground_emit.iter().all(|c| c.max_c() == 0.0));
    assert!(zone.ground.iter().any(|c| *c == HOLE));
    assert!(!zone.blocked(zone.player_start.0, zone.player_start.1, 5.0));

    // All walkable tile centers are reachable from the arrival island.
    let start = ((zone.player_start.0 as i32 / TILE), (zone.player_start.1 as i32 / TILE));
    let mut visited = vec![false; zone.tiles.len()];
    let mut queue = VecDeque::from([start]);
    while let Some((x, y)) = queue.pop_front() {
        if x < 0 || y < 0 || x >= zone.tw || y >= zone.th || zone.tile(x, y) != Tile::Floor {
            continue;
        }
        let i = (y * zone.tw + x) as usize;
        if visited[i] {
            continue;
        }
        visited[i] = true;
        queue.extend([(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]);
    }
    let floor_count = zone.tiles.iter().filter(|t| **t == Tile::Floor).count();
    assert_eq!(visited.iter().filter(|v| **v).count(), floor_count);

    // Use the real feet-box collision, including artist-authored prop footprints.
    let (mut x, mut y) = zone.player_start;
    let mut ticks = 0;
    for (tx, ty) in [(720.0f32, 782.0f32), (720.0, 500.0), (720.0, 380.0)] {
        while (x - tx).abs() > 0.1 || (y - ty).abs() > 0.1 {
            let dx = (tx - x).clamp(-1.45, 1.45);
            let dy = (ty - y).clamp(-1.45, 1.45);
            let (nx, ny, blocked) = zone.move_box(x, y, 5.0, dx, dy);
            assert!(!blocked, "approach blocked at {x},{y}");
            (x, y) = (nx, ny);
            ticks += 1;
            assert!(ticks < 900);
        }
    }

    let mut frame = Frame::new();
    let mut fx = Fx::new(1);
    for (cx, cy) in [(268, 674), (528, 532), (528, 332), (768, 292)] {
        frame.cam_x = cx;
        frame.cam_y = cy;
        frame.begin(zone.bg, zone.ambient);
        region.draw_backdrop(&zone, &mut frame, 0);
        let still = frame.albedo.clone();
        region.draw_backdrop(&zone, &mut frame, 3600);
        assert_eq!(frame.albedo, still, "starfield changed with time");
        let mut times = Vec::new();
        for t in 0..120 {
            let ctx = RegionCtx { t, player: (720.0, 500.0), movers: &[], cam: (cx, cy) };
            frame.begin(zone.bg, zone.ambient);
            let began = Instant::now();
            region.update(&mut zone, &ctx, &mut fx);
            region.draw_backdrop(&zone, &mut frame, t);
            zone.draw_ground(&mut frame);
            region.draw_ground(&zone, &mut frame, t);
            let mut items = Vec::new();
            region.sorted_items(&zone, &frame, &mut items);
            for (_, id) in items {
                region.draw_item(&zone, &mut frame, id, t);
            }
            region.draw_overlay(&zone, &mut frame, t);
            region.light(&zone, &mut frame, t);
            black_box(&frame);
            times.push(began.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        println!("camera {cx},{cy}: Summit update + draw + ground blit p50 {:.3} ms, p95 {:.3} ms", times[60], times[114]);
        assert_eq!(frame.albedo.len(), W * H);
    }
    println!(
        "PASS: {floor_count} connected floor tiles; arrival to Warden approach {:.1}s walking; still sky; neutral grade; unlit ground",
        ticks as f32 / 60.0
    );
}
