use super::*;

fn real_water() -> Water {
    let (zone, _) = super::super::build();
    let map = super::super::paint::classify(&zone);
    let fields = super::super::paint::fields(&map);
    Water::new(&map, &fields, &zone.ground, &[])
}

#[test]
fn simulation_nodes_and_links_follow_the_actual_receiving_pixels() {
    let water = real_water();
    for &i in &water.active {
        let (x, y) = (i as i32 % water.gw * CELL + CELL / 2,
            i as i32 / water.gw * CELL + CELL / 2);
        assert!(water.is_water(x, y), "dry simulation node at {x},{y}");
        for (offset, dx, dy, bit) in [(-1, -1, 0, 1), (1, 1, 0, 2),
            (-water.gw, 0, -1, 4), (water.gw, 0, 1, 8)] {
            if water.links[i] & bit != 0 {
                assert!(water.wet[(i as i32 + offset) as usize]);
                assert!(water.is_water(x + dx, y + dy), "wave crossed a dry bank");
            }
        }
    }
}

#[test]
fn a_one_pixel_dry_bank_does_not_transmit_waves() {
    use super::super::paint::{FLOOR, PixMap};
    let (w, h) = (32, 32);
    let mut cls = vec![SHALLOW; (w*h) as usize];
    for y in 0..h { cls[(y*w + 16) as usize] = FLOOR; }
    let map = PixMap { w, h, cls, depth: vec![0; (w*h) as usize],
        span: vec![0; (w*h) as usize], ledge: vec![0; (w*h) as usize] };
    let fields = super::super::paint::fields(&map);
    let mut water = Water::new(&map, &fields, &vec![WATER[1]; (w*h) as usize], &[]);
    water.poke(15.0, 15.0, 0.9, 1.0);
    for _ in 0..60 { water.step((0, 0)); }
    assert!(water.cur.iter().any(|v| v.abs() > 0.001));
    for &i in &water.active {
        if i as i32 % water.gw * CELL + CELL / 2 > 16 {
            assert_eq!(water.cur[i], 0.0, "wave energy crossed the dry divider");
        }
    }
}

#[test]
fn every_water_pixel_retains_a_live_calm_normal_without_touching_dry_land() {
    let mut water = real_water();
    water.poke(375.0, 420.0, 3.0, 0.4);
    for _ in 0..12 { water.step((0, 0)); }
    let mut frame = Frame::new();
    frame.surface_lighting = true;
    frame.cam_x = 284;
    frame.cam_y = 310;
    water.draw(&mut frame, 12);
    let mut tilted = 0;
    for y in 0..H {
        for x in 0..W {
            let s = frame.surfaces[y*W+x];
            if water.is_water(x as i32 + frame.cam_x, y as i32 + frame.cam_y) {
                assert_eq!((s.kind, s.owner), (1, 0));
                assert_eq!(s.height, 0.0);
                assert!(s.normal[2] >= 0.94 && s.normal[2] <= 1.0);
                tilted += usize::from(s.normal[0].abs() + s.normal[1].abs() > 0.002);
            } else {
                assert_eq!(s.kind, 0, "water normal escaped its native mask");
            }
        }
    }
    assert!(tilted > 20, "a twelve-tick wake has only {tilted} live normals");
}

#[test]
fn disturbances_remain_visible_after_six_and_twelve_ticks_in_every_pool() {
    let mut water = real_water();
    for (x, y) in [(130,390), (530,82), (366,340), (1053,374), (1278,665)] {
        assert!(water.is_water(x, y));
        water.cur.fill(0.0);
        water.prev.fill(0.0);
        let (cam_x, cam_y) = (x - W as i32 / 2, y - H as i32 / 2);
        let draw = |water: &Water, t| {
            let mut frame = Frame::new();
            frame.surface_lighting = true;
            frame.cam_x = cam_x;
            frame.cam_y = cam_y;
            water.draw(&mut frame, t);
            frame
        };
        let quiet: Vec<_> = [6,12].into_iter().map(|t| draw(&water,t)).collect();
        water.poke(x as f32, y as f32, 3.0, 0.4);
        for tick in 1..=12 {
            water.step((0,0));
            if let Some(index) = [6,12].iter().position(|&t| t == tick) {
                let disturbed = draw(&water, tick);
                let visible = disturbed.albedo.iter().zip(&quiet[index].albedo).filter(|(a,b)| {
                    (a.r-b.r).abs().max((a.g-b.g).abs()).max((a.b-b.b).abs()) * 255.0 >= 3.0
                }).count();
                assert!(visible >= 4, "pool {x},{y} at {tick} ticks: only {visible} visible wake pixels");
            }
        }
    }
}
