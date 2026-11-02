extern crate glintblade;
use glintblade::{gfx::{self, Frame,W,H,rgb},world::{Zone,ZoneId,Region},regions};
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
fn main(){let(z,r)=regions::build(ZoneId::Glasswater);let mut occluders=Vec::new();r.light_occluders(900,&mut occluders);println!("actualGlassoccluders={}",occluders.len());let mut total=0;let mut leaking=0;for (index,source)in occluders.iter().enumerate(){for shift in [(-45.0,-40.0),(-45.0,40.0),(45.0,40.0)]{let cam=((source.x as i32-180).max(0),(source.y as i32-140).max(0));let points=[(source.x+shift.0,source.y+shift.1,0.8)];let off=staged(&z,r.as_ref(),cam,&points,false,None);let on=staged(&z,r.as_ref(),cam,&points,true,None);let x0=source.x.floor()as i32-if source.flip{source.cell.w-1-source.anchor.0}else{source.anchor.0};let y0=source.y.floor()as i32-source.anchor.1;let mut checked=0;let mut wrong=0;let mut max=0.0f32;let mut examples=Vec::new();for y in 0..source.cell.h{for x in 0..source.cell.w{let px=source.cell.at(x,y);if !px.a||px.emit{continue;}let(wx,wy)=(x0+if source.flip{source.cell.w-1-x}else{x},y0+y);let Some(i)=Frame::idx(wx-cam.0,wy-cam.1)else{continue};if off.albedo[i]!=px.c{continue;}checked+=1;let(a,b)=(off.light[i],on.light[i]);let d=(a.r-b.r).abs().max((a.g-b.g).abs()).max((a.b-b.b).abs());if d>1e-5{wrong+=1;max=max.max(d);if examples.len()<4{examples.push((wx,wy,format!("{:?}",z.tile_at(wx as f32,wy as f32))));}}}}total+=checked;leaking+=wrong;if wrong>0{println!("decor{index} source_shift{shift:?} at{},{} size{}x{} visible_native_nonemit{checked} live_shadow_on_body{wrong} max{max:.7} examples{examples:?}",source.x,source.y,source.cell.w,source.cell.h);}}}println!("total_native_body_samples={total} ground_mask_leaked_samples={leaking}");assert_eq!(leaking,0,"physical ground shadows painted real native sprite surfaces");}
