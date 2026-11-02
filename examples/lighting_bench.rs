//! Stable native rendering cases, with real moving actors. No GPU/window.
use glintblade::{
    camera::Camera,
    fx::Fx,
    gfx::{Frame, post},
    input::{Input, Raw},
    play::{Play, PlayOpts},
    world::ZoneId,
};
use std::time::Instant;
fn main() {
    let mut result = Vec::new();
    for (id, name, x, y) in [
        (ZoneId::Bloomwild, "bloomwild", 456., 536.),
        (ZoneId::Glasswater, "glasswater", 550., 102.),
        (ZoneId::Cinder, "cinder", 621., 510.),
        (ZoneId::Summit, "summit", 720., 520.),
    ] {
        let mut p = Play::new(
            id,
            &PlayOpts {
                god: true,
                peaceful: false,
            },
        );
        p.player.body.x = x;
        p.player.body.y = y;
        p.cam = Camera::new(x, y);
        let mut fx = Fx::new(7);
        let mut f = Frame::new();
        let mut input = Input::default();
        for _ in 0..90 {
            input.step(Raw::default());
            p.update(&input, &mut fx);
            p.render(&mut f, &fx);
            fx.sounds.clear();
        }
        let mut ms = Vec::new();
        for _ in 0..240 {
            input.step(Raw::default());
            p.update(&input, &mut fx);
            let began = Instant::now();
            p.render(&mut f, &fx);
            post::finish(&mut f, &p.zone.grade);
            ms.push(began.elapsed().as_secs_f64() * 1000.);
            fx.sounds.clear();
        }
        ms.sort_by(f64::total_cmp);
        let mean = ms.iter().sum::<f64>() / ms.len() as f64;
        result.push(serde_json::json!({"region":name,"frames":ms.len(),"mean_ms":mean,"median_ms":ms[ms.len()/2],"p95_ms":ms[ms.len()*95/100],"max_ms":ms[ms.len()-1]}));
    }
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
