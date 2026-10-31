//! Cinder Steppes: an old obsidian foundry cut by a living lava river.

mod art;
mod field;
mod heat;
mod lava;
mod map;
mod paint;

use crate::entities::EnemyKind;
use crate::gfx::{Frame, Grade, H, Rgb, W};
use crate::world::{Prop, Region, Zone, ZoneId};

/// Build the highland approach, foundry court, and caldera sanctuary.
pub fn build() -> (Zone, Box<dyn Region>) {
    let (tw, th) = paint::map_size();
    let mut z = Zone::new(ZoneId::Cinder, tw, th);
    z.title = "CINDER STEPPES".into();
    z.music = "cinder".into();
    z.ambient = Rgb::new(0.91, 0.90, 0.94);
    // Cold ash fill separates combat silhouettes without warming the unlit steppe.
    z.lighting_settings.fill = Rgb::new(0.59, 0.57, 0.65);
    z.lighting_settings.shadow_tint = Rgb::new(0.60, 0.62, 0.76);
    z.bg = art::S0;
    z.grade = Grade {
        vignette: 0.0,
        bloom: 0.10,
        light_steps: 16.0,
        ..Grade::default()
    };
    let painted = paint::paint(&mut z);
    z.player_start = (136.0, 752.0);
    z.shard = Some((1144.0, 184.0));
    z.gate = Some((1144.0, 104.0));

    // An old processional road: broken markers at entry, four pillars around the foundry,
    // and a quieter pair at the shrine. Keep all combat lanes and bridge mouths open.
    for (anim, x, y) in [
        ("pillar_broken_short", 96.0, 744.0),
        ("pillar_broken_tall", 182.0, 705.0),
        ("pillar_broken_short", 280.0, 480.0),
        ("pillar", 518.0, 454.0),
        ("pillar_broken_tall", 714.0, 454.0),
        ("pillar_broken_short", 519.0, 568.0),
        ("pillar", 714.0, 568.0),
        ("pillar_broken_tall", 1178.0, 361.0),
        ("pillar_broken_short", 1098.0, 232.0),
        ("pillar", 1190.0, 232.0),
    ] {
        z.props.push(Prop::new("cinder_pillars", anim, x, y));
    }
    z.props
        .push(Prop::new("cinder_statue", "statue", 621.0, 450.0));
    z.props
        .push(Prop::new("cinder_statue", "statue", 600.0, 718.0).flipped(true));
    // A visible keeper's brazier motivates the court's focal light, away from bridge mouths.
    // Scenery only: preserve the existing collision map and every combat lane.
    let mut brazier = Prop::new("cinder_brazier", "brazier_lit", 646.0, 456.0);
    brazier.footprint = None;
    // The same flame drives cached stone reception, cast directions and actor rims.
    if let Some(light) = brazier.light.as_mut() {
        light.1 = 144.0;
        light.2 = 0.0;
    }
    z.props.push(brazier);

    // Twelve enemies in four encounters: foothills, bridge, foundry, and caldera approach.
    use EnemyKind::{EmberHound as Hound, SpittingTotem as Totem};
    for (kind, x, y) in [
        (Hound, 181.0, 612.0),
        (Hound, 247.0, 588.0),
        (Totem, 295.0, 543.0),
        (Hound, 285.0, 433.0),
        (Totem, 443.0, 462.0),
        (Hound, 575.0, 510.0),
        (Hound, 661.0, 535.0),
        (Totem, 676.0, 470.0),
        (Hound, 893.0, 570.0),
        (Hound, 1120.0, 537.0),
        (Totem, 1195.0, 399.0),
        (Hound, 1116.0, 374.0),
    ] {
        z.enemies.push((kind, x, y));
    }
    // The runtime lighting field owns cast shadows; do not bake another dark pass into albedo.
    let lava = lava::Lava::new(&painted);
    let heat = heat::Heat::build(&z, &painted);
    (z, Box::new(Cinder { lava, heat }))
}

struct Cinder {
    lava: lava::Lava,
    heat: heat::Heat,
}

impl Region for Cinder {
    /// Draw the advecting crust and molten seams only in the visible river area.
    fn draw_ground(&self, _zone: &Zone, f: &mut Frame, t: u64) {
        self.lava.draw(f, t);
    }

    /// Float sparse embers directly up from the lava surface.
    fn draw_overlay(&self, _zone: &Zone, f: &mut Frame, t: u64) {
        self.lava.embers(f, t);
    }

    /// Illuminate banks with the nearby lava, without unrelated ambient lights.
    fn light(&self, _zone: &Zone, f: &mut Frame, t: u64) {
        self.lava.light(f, t);
        self.heat.light(f);
    }

    /// Paint a still dusk sky and distant volcanic silhouettes below the hanging cliffs.
    fn draw_backdrop(&self, _zone: &Zone, f: &mut Frame, _t: u64) {
        use crate::util::bayer4;
        for sy in 0..H as i32 {
            let wy = sy + f.cam_y;
            let band = ((wy + 40) / 145).clamp(0, 5) as usize;
            let frac = (wy + 40).rem_euclid(145) as f32 / 145.0;
            let c0 = art::SKY_R[band];
            let c1 = art::SKY_R[(band + 1).min(5)];
            for sx in 0..W as i32 {
                let wx = sx + f.cam_x;
                let mut c = if frac > bayer4(wx, wy) { c1 } else { c0 };
                // The ridges live below the island, no moon, smoke, or drifting ash.
                let rx = wx.rem_euclid(470);
                let ridge = 800 - (175 - (rx - 235).abs()).max(0) / 2;
                if wy > ridge {
                    c = art::S1;
                }
                let rx = (wx + 90).rem_euclid(350);
                if wy > 874 - (140 - (rx - 175).abs()).max(0) / 3 {
                    c = art::S0;
                }
                f.put(sx, sy, c.scale(0.25));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Traverse actual feet-box collision from entry to every encounter, shard, and gate.
    #[test]
    fn processional_route_is_traversable() {
        let (z, region) = build();
        assert_eq!((z.tw, z.th), (88, 57));
        assert_eq!(z.enemies.len(), 12);
        let step = 4;
        let (w, h) = (z.pw / step, z.ph / step);
        let mut reached = vec![false; (w * h) as usize];
        let start = (
            z.player_start.0 as i32 / step,
            z.player_start.1 as i32 / step,
        );
        assert!(!z.blocked(z.player_start.0, z.player_start.1, 4.0));
        let mut queue = VecDeque::from([start]);
        reached[(start.1 * w + start.0) as usize] = true;
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let i = (ny * w + nx) as usize;
                if reached[i] || z.blocked((nx * step) as f32, (ny * step) as f32, 4.0) {
                    continue;
                }
                reached[i] = true;
                queue.push_back((nx, ny));
            }
        }
        for (name, (x, y)) in [("shard", z.shard.unwrap()), ("gate", z.gate.unwrap())] {
            assert!(!z.blocked(x, y, 4.0), "blocked {name}");
            assert!(
                reached[(y as i32 / step * w + x as i32 / step) as usize],
                "unreachable {name}"
            );
        }
        for (kind, x, y) in &z.enemies {
            assert!(!z.blocked(*x, *y, 6.0), "blocked {kind:?} at {x},{y}");
            assert!(
                reached[(*y as i32 / step * w + *x as i32 / step) as usize],
                "unreachable enemy at {x},{y}"
            );
        }
        // Exercise actual pickup/open/exit logic with real player movement along the shrine.
        let mut play = crate::play::Play::new(
            ZoneId::Cinder,
            &crate::play::PlayOpts {
                god: true,
                peaceful: true,
            },
        );
        play.player.body.x = 1144.0;
        play.player.body.y = 300.0;
        let mut fx = crate::fx::Fx::new(7);
        let mut input = crate::input::Input::default();
        for _ in 0..320 {
            input.mv = if !play.shard_taken || play.gate == crate::play::Gate::Open {
                (0.0, -1.0)
            } else {
                (0.0, 0.0)
            };
            play.update(&input, &mut fx);
            fx.sounds.clear();
            fx.lights.clear();
            if play.exit {
                break;
            }
        }
        assert!(
            play.shard_taken,
            "player could not collect the caldera shard"
        );
        assert!(
            play.exit,
            "player could not pass through the opened exit gate"
        );

        // Region rendering cost excludes the engine composite, and uses the busiest lava view.
        let mut f = Frame::new();
        f.cam_x = 950;
        f.cam_y = 130;
        let started = std::time::Instant::now();
        for t in 0..300 {
            region.draw_backdrop(&z, &mut f, t);
            region.draw_ground(&z, &mut f, t);
            region.draw_overlay(&z, &mut f, t);
            region.light(&z, &mut f, t);
        }
        eprintln!(
            "Cinder hooks: {:.3} ms/frame",
            started.elapsed().as_secs_f64() * 1000.0 / 300.0
        );
    }
}
