extern crate glintblade;
use glintblade::{gfx::{self, Frame,Rgb,W,H,rgb},world::{Zone,ZoneId,Prop,NoRegion,Region},regions};
fn staged(z:&Zone,r:&dyn Region,cam:(i32,i32),points:&[(f32,f32,f32)],enabled:bool,actor:Option<(i32,i32)>)->Frame {
 let mut f=Frame::new();f.cam_x=cam.0;f.cam_y=cam.1;f.begin(z.bg,z.ambient);
 z.draw_ground(&mut f);r.draw_ground(z,&mut f,900);
 let mut ordered:Vec<_>=z.props.iter().filter(|p|p.alive&&!p.canopy).collect();ordered.sort_by(|a,b|a.sort_y().total_cmp(&b.sort_y()));
 for p in ordered {p.draw(&mut f,900,1.0);}
 let mut items=Vec::new();r.sorted_items(z,&f,&mut items);for (_,id)in items{r.draw_item(z,&mut f,id,900);}
 for &(x,y,intensity)in points{f.add_light_w(x,y,145.0,rgb(0x81d1e0),intensity);}
 if !enabled{f.point_lights.clear();}
 if let Some((x,y))=actor {let i=(y-cam.1)as usize*W+(x-cam.0)as usize;f.actor_pixels[i]=true;}
 let albedo=f.albedo.clone();let emit=f.emissive.clone();gfx::lighting::apply(z,r,&mut f,900);
 assert_eq!(albedo,f.albedo,"lighting mutated albedo");assert_eq!(emit,f.emissive,"lighting mutated emission");f
}

fn main(){
 let(mut z,mut r)=regions::build(ZoneId::Glasswater);let mut nodes=Vec::new();r.light_occluders(900,&mut nodes);
 let targets:Vec<_>=nodes.iter().filter(|s|s.cell.h<=16&&s.own_light.is_some_and(|p|p.intensity<0.2)).map(|s|(s.x,s.y)).collect();drop(nodes);
 let mut removed=0usize;
 for &(x,y)in &targets{let cam=((x as i32-180).max(0),(y as i32-140).max(0));let mut before=Vec::new();r.light_occluders(900,&mut before);let previous=before.len();drop(before);let mut fx=glintblade::fx::Fx::new(9);
  r.slash(&mut z,&glintblade::combat::Hit::new(glintblade::combat::Shape::Circle{x,y:y-2.,r:1.},glintblade::combat::Team::Player,1,fx.hit_id()),&mut fx);
  for t in 1..=20{r.update(&mut z,&glintblade::world::RegionCtx{t,player:(x,y+40.),movers:&[],cam},&mut fx);}
  let mut after=Vec::new();r.light_occluders(900,&mut after);assert!(after.len()<previous,"real sprig did not break");removed+=previous-after.len();drop(after);
  let cached=staged(&z,r.as_ref(),cam,&[],true,None);z.invalidate_lighting();let fresh=staged(&z,r.as_ref(),cam,&[],true,None);
  let changed=cached.light.iter().zip(&fresh.light).filter(|(a,b)|a!=b).count();println!("sprig_at={x},{y} cache_vs_fresh_changed_light_pixels={changed}");assert_eq!(changed,0,"dead region sprig changed cached form/reception");
 }
 println!("actual_region_sprigs_destroyed={removed} cases={} cached_light_equals_fresh pass",targets.len());
}
