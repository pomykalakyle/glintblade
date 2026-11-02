//! Cinder Steppes: an old obsidian foundry cut by a living lava river.

mod art;
mod field;
mod heat;
mod lava;
mod map;
mod paint;
mod textures;

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
        ("pillar", 1185.0, 232.0),
    ] {
        let markers = if anim == "pillar_broken_short" {
            "cinder_pillars"
        } else {
            "cinder_markers"
        };
        let variant = if x == 1178.0 {
            "pillar_lean"
        } else if x == 1185.0 {
            "pillar_slotted"
        } else {
            anim
        };
        z.props.push(Prop::new(markers, variant, x, y));
    }
    z.props
        .push(Prop::new("cinder_foundry", "press", 621.0, 450.0));
    z.props
        .push(Prop::new("cinder_foundry", "broken_press", 600.0, 718.0));
    // A visible keeper's brazier motivates the court's focal light, away from bridge mouths.
    let mut brazier = Prop::new("cinder_brazier", "brazier_lit", 646.0, 456.0);
    if let Some(s) = brazier.sheet() && let Some(a) = s.anim("brazier_lit") {
        brazier.set_native_support(s.cell(a, 0), s.anchor_of(a), 5);
    }
    // Retain the actual authored emitter separately from the legacy cached-light
    // placeholder. Physical normal lighting must not inherit its zero intensity.
    let brazier_source = brazier.light.map(|light| (z.props.len(),light));
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
    (z, Box::new(Cinder { lava, heat, brazier_source }))
}

struct Cinder {
    lava: lava::Lava,
    heat: heat::Heat,
    brazier_source: Option<(usize,(Rgb,f32,f32,f32,f32))>,
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
    fn light(&self, zone: &Zone, f: &mut Frame, t: u64) {
        if f.surface_lighting && let Some((id,(color,radius,intensity,ox,oy)))=self.brazier_source
            && let Some(prop)=zone.props.get(id).filter(|p|p.alive) {
            f.add_light_3d_w(prop.x+ox,prop.y,-oy,radius,color,intensity);
        }
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

    #[test]
    fn live_brazier_keeps_its_authored_3d_emitter_in_normal_mode() {
        let (mut zone,region)=build();
        let id=zone.props.iter().position(|p|p.sheet=="cinder_brazier").unwrap();
        let mut f=Frame::new();f.cam_x=454;f.cam_y=348;
        f.begin(Rgb::BLACK,zone.ambient);f.surface_lighting=true;
        region.light(&zone,&mut f,90);
        let lamp=f.point_lights.iter().find(|l|(l.x-646.0).abs()<0.01).expect("visible flame needs a live normal-aware source");
        assert_eq!((lamp.y,lamp.height,lamp.radius,lamp.intensity),(436.0,20.0,64.0,0.9));
        zone.props[id].alive=false;
        f.begin(Rgb::BLACK,zone.ambient);f.surface_lighting=true;
        region.light(&zone,&mut f,90);
        assert!(f.point_lights.iter().all(|l| (l.x-646.0).abs()>0.01 || (l.y-436.0).abs()>0.01),
            "a removed brazier cannot keep lighting the scene");
        assert!(!f.point_lights.is_empty(), "extinguishing a brazier must preserve the live lava sources");
    }

    /// The visible brazier is solid at its stone foot, and its foundry lane remains open.
    #[test]
    fn brazier_support_stops_player_without_sealing_court_lane() {
        let mut play = crate::play::Play::new(ZoneId::Cinder, &crate::play::PlayOpts { god: true, peaceful: true });
        let mut fx = crate::fx::Fx::new(7);
        play.player.body.x = 646.0;
        play.player.body.y = 488.0;
        let mut input = crate::input::Input { mv: (0.0, -1.0), ..Default::default() };
        for _ in 0..60 { play.update(&input, &mut fx); }
        let brazier = play.zone.props.iter().find(|p| p.sheet == "cinder_brazier").unwrap();
        assert_eq!(brazier.collision_bounds().unwrap()[3], 458.0, "native stone foot extends two rows below the old anchor");
        assert!((460.4..462.0).contains(&play.player.body.y), "feet did not stop at the visible brazier foot: {}", play.player.body.y);
        play.player.body.x = 622.0;
        play.player.body.y = 480.0;
        play.player.body.vx = 0.0;
        play.player.body.vy = 0.0;
        input.mv = (1.0, 0.0);
        for _ in 0..60 { play.update(&input, &mut fx); }
        assert!(play.player.body.x > 700.0, "brazier sealed court route at {}", play.player.body.x);
        assert!((play.player.body.y - 480.0).abs() < 0.1);
    }

    /// All three crossing types use ordinary player input. The basalt arch and the
    /// two masonry bridges must carry the player over blocked lava to real landings.
    #[test]
    fn bridge_input_crossings_preserve_both_landings() {
        let mut play = crate::play::Play::new(ZoneId::Cinder, &crate::play::PlayOpts { god: true, peaceful: true });
        let mut fx = crate::fx::Fx::new(7);
        for (name, start, movement, ticks, axis, target) in [
            ("basalt arch", (310.0, 488.0), (1.0, 0.0), 100, 0, 450.0),
            ("canal bridge", (560.0, 632.0), (0.0, 1.0), 65, 1, 700.0),
            ("caldera bridge", (1144.0, 332.0), (0.0, -1.0), 85, 2, 233.0),
        ] {
            play.player.body.x = start.0;
            play.player.body.y = start.1;
            play.player.body.vx = 0.0;
            play.player.body.vy = 0.0;
            let input = crate::input::Input { mv: movement, ..Default::default() };
            for _ in 0..ticks {
                play.update(&input, &mut fx);
                fx.sounds.clear();
                fx.lights.clear();
            }
            let pos = (play.player.body.x, play.player.body.y);
            assert!(match axis { 0 => pos.0 > target, 1 => pos.1 > target, _ => pos.1 < target }, "{name} blocked at {pos:?}");
            assert_eq!(play.zone.tile_at(pos.0, pos.1), crate::world::Tile::Floor, "{name} ended over lava");
        }
    }

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
