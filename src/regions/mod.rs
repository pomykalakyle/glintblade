//! One file per region. Each exposes `pub fn build() -> (Zone, Box<dyn Region>)`: it lays out
//! the tile grid, paints the pre-rendered ground, places props, lights and enemy spawns, and
//! returns the region's live behavior (grass, water, lava, weather...).

pub mod bloomwild;
pub mod cinder;
pub mod glasswater;
pub mod summit;

use crate::gfx::{Rgb, rgb};
use crate::util::{bayer4, fbm};
use crate::world::{Region, TILE, Tile, Zone, ZoneId};

pub fn build(id: ZoneId) -> (Zone, Box<dyn Region>) {
    let (zone, region) = match id {
        ZoneId::Bloomwild => bloomwild::build(),
        ZoneId::Glasswater => glasswater::build(),
        ZoneId::Cinder => cinder::build(),
        ZoneId::Summit => summit::build(),
    };
    crate::gfx::lighting::prepare(&zone, region.as_ref());
    (zone, region)
}

/// A plain walled rectangle with a noisy two-tone floor. Used by the placeholder regions.
pub fn placeholder(id: ZoneId, title: &str, floor: (u32, u32), wall: u32, ambient: Rgb, music: &str) -> Zone {
    let (tw, th) = (60, 34);
    let mut z = Zone::new(id, tw, th);
    z.title = title.to_string();
    z.music = music.to_string();
    z.ambient = ambient;
    z.bg = rgb(0x05040c);
    for ty in 0..th {
        for tx in 0..tw {
            let edge = tx == 0 || ty == 0 || tx == tw - 1 || ty == th - 1;
            let rock = fbm(tx as f32 * 0.25, ty as f32 * 0.25, 3, 7) > 0.68;
            z.set_tile(tx, ty, if edge || rock { Tile::Wall } else { Tile::Floor });
        }
    }
    let (a, b) = (rgb(floor.0), rgb(floor.1));
    let w = rgb(wall);
    for y in 0..z.ph {
        for x in 0..z.pw {
            let t = z.tile(x / TILE, y / TILE);
            let c = if t == Tile::Wall {
                w.scale(0.7 + 0.3 * ((y % TILE) as f32 / TILE as f32))
            } else {
                let n = fbm(x as f32 * 0.05, y as f32 * 0.05, 3, 1);
                if n > bayer4(x, y) * 0.3 + 0.45 { a } else { b }
            };
            z.gset(x, y, c);
        }
    }
    z.player_start = (5.0 * TILE as f32, (th / 2) as f32 * TILE as f32);
    z.gate = Some(((tw - 4) as f32 * TILE as f32, (th / 2) as f32 * TILE as f32));
    z.shard = Some(((tw - 8) as f32 * TILE as f32, (th / 2) as f32 * TILE as f32));
    z
}
