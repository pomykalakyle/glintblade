//! The abandoned cistern retains its native stone feet. Authored contact ink
//! joins the cave material without an artificial contact apron.

use crate::gfx::rgb;
use crate::world::{Prop, Tile, Zone};

const SHEET: &str = "round13_landmarks_distributor";
const ANCHOR: (f32, f32) = (1304.0, 252.0);
const PATCH: [i32; 4] = [1230, 225, 1375, 267];

pub(super) fn place(zone: &mut Zone) {
    apron(zone);
    zone.props.push(Prop::new(SHEET, "body", ANCHOR.0, ANCHOR.1));
}

fn apron(zone: &mut Zone) {
    let Some(sheet) = crate::gfx::sheet(SHEET) else { return };
    let Some(anim) = sheet.anim("body") else { return };
    let cell = sheet.cell(anim, 0);
    let (ax, ay) = sheet.anchor_of(anim);
    let (ox, oy) = (ANCHOR.0 as i32 - ax, ANCHOR.1 as i32 - ay);
    for y in PATCH[1]..PATCH[3] {
        for x in PATCH[0]..PATCH[2] {
            if zone.tile_at(x as f32 + 0.5, y as f32 + 0.5) != Tile::Floor { continue; }
            let i = (y * zone.pw + x) as usize;
            let source = zone.ground_surfaces[i];
            if zone.ground[i].r < 0.0 || source.height != 0 || source.upright { continue; }
            // Retain the real floor under native stone and its recessed soles.
            if cell.at(x-ox,y-oy).a { continue; }
            // A shallow dry runnel exits the inspection opening and turns
            // southwest. It never underlines both foundations with a rail.
            let drain = if (236..260).contains(&y) { Some((1304,1)) }
                else if (260..267).contains(&y) { Some((1304-(y-259)*2,if y<264 {1}else{0})) }
                else { None };
            if let Some((cx,half)) = drain {
                if (x-cx).abs() <= half {
                    zone.ground[i] = zone.ground[i].lerp(rgb(0x222d3d),0.26);
                }
            }
        }
    }
    zone.invalidate_lighting();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Player, player::PlayerCtx};
    use crate::fx::Fx;
    use crate::gfx::{DrawOpts, Frame, sheet};
    use crate::input::{Input, Raw};
    use crate::world::ZoneId;

    fn walk(zone: &Zone, start: (f32, f32), mv: (f32, f32), dash: bool) -> Player {
        let mut player = Player::new(start.0, start.1);
        let mut input = Input::default();
        let mut fx = Fx::new(71);
        for t in 0..100 {
            input.step(Raw { mv, dash: dash && t == 0, ..Raw::default() });
            player.update(&input, &mut PlayerCtx { zone, fx: &mut fx, hits: &mut Vec::new() });
            fx.tick();
        }
        player
    }

    #[test]
    fn native_feet_match_collision_and_rest_on_dry_zero_height_receivers() {
        let (zone, _) = crate::regions::build(ZoneId::Glasswater);
        let prop = zone.props.iter().find(|p| p.sheet == SHEET).unwrap();
        assert_eq!((prop.x, prop.y, prop.sort_y()), (1304.0, 252.0, 249.0));
        let s = prop.sheet().unwrap();
        let a = s.anim("body").unwrap();
        let cell = s.cell(a, 0);
        assert_eq!(s.anchor_of(a), (79, 103));
        let mut supports = 0;
        for y in 92..100 {
            for x in 0..cell.w {
                let (wx, wy) = (1225 + x, 149 + y);
                let opaque = cell.at(x, y).a;
                assert_eq!(prop.blocks(wx as f32 + 0.5, wy as f32 + 0.5, 0.01, 0.01), opaque);
                if !opaque { continue; }
                assert_eq!(zone.tile_at(wx as f32 + 0.5, wy as f32 + 0.5), Tile::Floor);
                let surface = zone.ground_surfaces[(wy * zone.pw + wx) as usize];
                assert_eq!(surface.height, 0);
                assert!(!surface.upright);
                supports += 1;
            }
        }
        assert!(supports > 450);
        assert!(!prop.blocks(1265.0, 239.0, 0.01, 0.01), "projected shaft face is above the floor, not a second footing");
        assert!(!prop.blocks(1304.0, 245.0, 3.0, 2.0), "inspection aperture stays open");
        assert!(!prop.breakable && prop.light.is_none());
    }

    #[test]
    fn real_player_walk_and_dash_stop_at_feet_and_cross_the_inspection_opening() {
        let (zone, _) = crate::regions::build(ZoneId::Glasswater);
        for dash in [false, true] {
            for x in [1274.0, 1335.0] {
                let south = walk(&zone, (x, 285.0), (0.0, -1.0), dash);
                assert!(south.body.y >= 245.0 && south.body.y < 252.0, "south {x}: {}", south.body.y);
                let north = walk(&zone, (x, 211.0), (0.0, 1.0), dash);
                assert!(north.body.y > 234.0 && north.body.y <= 242.0, "north {x}: {}", north.body.y);
            }
            let west = walk(&zone, (1220.0, 243.0), (1.0, 0.0), dash);
            assert!(west.body.x < 1258.0 && west.body.x > 1248.0);
            let east = walk(&zone, (1400.0, 243.0), (-1.0, 0.0), dash);
            assert!(east.body.x > 1356.0 && east.body.x < 1370.0);
            let passage = walk(&zone, (1304.0, 285.0), (0.0, -1.0), dash);
            assert!(passage.body.y < 222.0, "open arch should be traversable: {}", passage.body.y);
        }
    }

    #[test]
    fn hero_reveal_retains_complete_physical_caster_and_registered_normals() {
        let prop = Prop::new(SHEET, "body", 180.0, 175.0);
        let hero = sheet("hero").unwrap();
        let pose = hero.anim("idle_down").unwrap();
        let focus = crate::gfx::sprite::SpriteFocus { cell: hero.cell(pose, 0), anchor: hero.anchor_of(pose),
            flip: false, x: 151.0, y: 147.0, ground_y: 147.0 };
        let draw = |reveal| {
            let mut f = Frame::new();
            f.begin(crate::gfx::Rgb::BLACK, crate::gfx::Rgb::WHITE);
            f.surface_lighting = true;
            prop.draw_revealed(&mut f, 90, 1.0, reveal);
            f
        };
        let opaque = draw(None);
        let revealed = draw(Some(focus));
        assert_ne!(opaque.albedo, revealed.albedo);
        assert_eq!(opaque.surface_casters.len(), 1);
        assert_eq!(revealed.surface_casters.len(), 1);
        let (a, b) = (&opaque.surface_casters[0], &revealed.surface_casters[0]);
        assert_eq!((a.x, a.y, a.base, a.opacity), (b.x, b.y, b.base, b.opacity));
        assert_eq!(a.opacity, 1.0);
        assert!(std::sync::Arc::ptr_eq(&a.pixels, &b.pixels));
        let mut control = Frame::new();
        control.surface_lighting = true;
        let s = prop.sheet().unwrap();
        let a = s.anim("body").unwrap();
        control.sprite_w(s.cell(a, 0), s.anchor_of(a), prop.x, prop.y, &DrawOpts::default());
        for (a, b) in control.surfaces.iter().zip(&opaque.surfaces) {
            assert_eq!((a.normal, a.height, a.owner, a.kind), (b.normal, b.height, b.owner, b.kind));
        }
    }
}
