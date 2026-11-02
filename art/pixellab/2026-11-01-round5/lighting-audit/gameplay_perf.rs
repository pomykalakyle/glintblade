extern crate glintblade;
use glintblade::{game::{Game,Options,Scene},input::{Input,Raw},world::ZoneId,gfx::Dir};
use std::time::Instant;
fn main(){
 println!("trial={}",std::env::var("GLINT_LIGHT_TRIAL").unwrap_or("default".into()));
 for (name,id,pos,ticks)in[("glass_crystal_charge",ZoneId::Glasswater,(532.0,92.0),420),("cinder_pillar_charge",ZoneId::Cinder,(518.0,477.0),420),("summit_boss_active",ZoneId::Summit,(720.0,490.0),1080)]{
  let mut g=Game::new(Options{start:Some(id),god:true,peaceful:false,seed:7});
  if let Scene::Play(p)=&mut g.scene{assert!(!p.zone.blocked(pos.0,pos.1,p.player.body.r),"fixture starts blocked");p.player.respawn(pos.0,pos.1);p.player.dir=Dir::Up;p.checkpoint=pos;p.cam.snap(pos.0,pos.1);}
  let mut input=Input::default();for _ in 0..90{input.step(Raw::default());g.tick(&input);g.render();g.fx.lights.clear();g.fx.sounds.clear();}
  let mut render=Vec::new();let mut tick_time=Vec::new();let mut points=Vec::new();let mut peak_tick=0;let mut peak_points=0;let mut attacks=0;let mut boss=false;let mut peak_particles=0;
  for n in 0..ticks{let attack=n%140<80;attacks+=usize::from(attack);input.step(Raw{attack,..Raw::default()});let start=Instant::now();g.tick(&input);tick_time.push(start.elapsed().as_secs_f64()*1000.);let start=Instant::now();g.render();render.push(start.elapsed().as_secs_f64()*1000.);std::hint::black_box(&g.frame.out);let count=g.frame.point_lights.len();points.push(count);if count>peak_points{peak_tick=n;peak_points=count;}peak_particles=peak_particles.max(g.fx.particles.list.len());if let Scene::Play(p)=&g.scene{boss|=p.boss_awake;}g.fx.lights.clear();g.fx.sounds.clear();}
  let mean=render.iter().sum::<f64>()/render.len()as f64;let mt=tick_time.iter().sum::<f64>()/tick_time.len()as f64;render.sort_by(f64::total_cmp);tick_time.sort_by(f64::total_cmp);let p95=render[(render.len()*95/100).min(render.len()-1)];let p99=render[(render.len()*99/100).min(render.len()-1)];let max=render[render.len()-1];let tmax=tick_time[tick_time.len()-1];let point_mean=points.iter().sum::<usize>()as f64/points.len()as f64;let mut histogram=std::collections::BTreeMap::new();for p in points{*histogram.entry(p).or_insert(0)+=1;}
  println!("{name}: ticks={ticks} attack_held_ticks={attacks} boss_awake={boss} render_mean={mean:.4}ms p95={p95:.4}ms p99={p99:.4}ms max={max:.4}ms update_mean={mt:.4}ms update_max={tmax:.4}ms visible_points_mean={point_mean:.2} peak={peak_points} peak_tick={peak_tick} peak_particles={peak_particles} point_histogram={histogram:?}");
  assert!(mean<6.0&&p95<8.0,"actual gameplay rendering exceeds warm budget");
 }
}
