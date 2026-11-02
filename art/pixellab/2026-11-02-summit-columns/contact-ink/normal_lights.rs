use glintblade::{assets::write_png, gfx::{self, DrawOpts, Frame, Grade, Rgb, W, H, post, sheet}};
fn main() {
    let out=std::env::args().nth(1).unwrap();std::fs::create_dir_all(&out).unwrap();
    for (label,dx,dy,z) in [("top",0.0,0.0,1000.0),("front",0.0,1000.0,20.0),
        ("left",-1000.0,0.0,20.0),("right",1000.0,0.0,20.0)] {
        for (index,(name,anim)) in [("summit_pillars","pillar_0"),("summit_pillars","pillar_1"),
            ("summit_pillars","pillar_2"),("summit_stones","monolith_0"),("summit_stones","monolith_1")].into_iter().enumerate() {
            let mut f=Frame::new();f.begin(Rgb::gray(0.025),Rgb::BLACK);f.surface_lighting=true;
            let s=sheet(name).unwrap();let a=s.anim(anim).unwrap();let mut cell=s.cell(a,0).clone();
            for p in &mut cell.px {if p.a {p.c=Rgb::WHITE;}}
            f.sprite_w(&cell,s.anchor_of(a),192.0,160.0,&DrawOpts::default());
            f.add_light_3d_w(192.0+dx,160.0+dy,z,4000.0,Rgb::WHITE,0.8);
            gfx::surface::apply(&mut f);
            let grade=Grade{bloom:0.0,light_steps:0.0,..Default::default()};
            post::compose(&mut f,&grade);post::finish(&mut f,&grade);
            let rgba:Vec<_>=f.out.iter().flat_map(|p|p.iter().copied()).collect();
            write_png(&std::path::Path::new(&out).join(format!("{index}_{label}.png")),W as u32,H as u32,&rgba).unwrap();
        }
    }
}
