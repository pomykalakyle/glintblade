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
fn diff(a:&Frame,b:&Frame)->(usize,f32){let mut changed=0;let mut max=0.0f32;for (p,q)in a.light.iter().zip(&b.light){let d=(p.r-q.r).abs().max((p.g-q.g).abs()).max((p.b-q.b).abs());changed+=usize::from(d>1e-6);max=max.max(d);} (changed,max)}
fn aligned(a:&Frame,b:&Frame)->(usize,f32){let mut changed=0;let mut max=0f32;for y in 0..H as i32{for x in 0..W as i32{let wx=x+a.cam_x;let wy=y+a.cam_y;let Some(j)=Frame::idx(wx-b.cam_x,wy-b.cam_y)else{continue};let p=a.light[y as usize*W+x as usize];let q=b.light[j];let d=(p.r-q.r).abs().max((p.g-q.g).abs()).max((p.b-q.b).abs());changed+=usize::from(d>1e-5);max=max.max(d);}}(changed,max)}

fn main(){let mut cases=0;for anim in ["pillar_broken_tall","pillar_broken_short"] {for flip in [false,true]{for frac in [0.2,0.8]{
 let mut z=Zone::new(ZoneId::Bloomwild,32,24);z.ground.fill(rgb(0x71818f));let mut p=Prop::new("bloomwild_ruins",anim,200.+frac,150.+1.-frac);p.flip=flip;let sheet=p.sheet().unwrap();let a=sheet.anim(anim).unwrap();let cell=sheet.cell(a,0);let anchor=sheet.anchor_of(a);let bottom=(0..cell.h).rev().find(|&y|(0..cell.w).any(|x|cell.at(x,y).a)).unwrap();let base=p.y.floor()+bottom as f32+1.-anchor.1 as f32;assert_eq!(bottom,53,"selected native foot bottom changed");assert_eq!(p.sort_y(),base,"sort/contact base does not match repaired native ink");let bounds=p.collision_bounds().unwrap();assert_eq!(bounds[3],base);let x0=p.x.floor()as i32-if flip{cell.w-1-anchor.0}else{anchor.0};
 let empty=staged(&z,&NoRegion,(0,0),&[],true,None);z.props.push(p);let live=staged(&z,&NoRegion,(0,0),&[],true,None);let shifted=staged(&z,&NoRegion,(17,11),&[],true,None);assert_eq!(aligned(&live,&shifted).0,0);
 let mut projected=0;for y in 0..bottom {for x in 0..cell.w{if !cell.at(x,y).a{continue;}let h=(bottom-y)as f32;let wx=(x0 as f32+if flip{(cell.w-1-x)as f32}else{x as f32}+h*0.98).round()as i32;let wy=(base+h*0.47).round()as i32;if wy<base as i32+8{continue;}let Some(i)=Frame::idx(wx,wy)else{continue};if live.albedo[i]!=rgb(0x71818f){continue;}let d=empty.light[i]-live.light[i];if d.max_c()>0.03{projected+=1;}}}assert!(projected>10,"repaired actual alpha must cast a native daylight silhouette beyond contact shade");
 z.props[0].alive=false;let dead=staged(&z,&NoRegion,(0,0),&[],true,None);assert_eq!(empty.light,dead.light,"dead repaired column left cached native shadow");println!("{anim} flip={flip} fractional={frac} opaque_bottom={bottom} actual_first_empty_world_row={base} collision_bottom={} projected_shadow_samples={projected} camera/death PASS",bounds[3]);cases+=1;
 }}}println!("repaired_native_column_cases={cases} PASS");}
