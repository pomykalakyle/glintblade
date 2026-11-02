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
fn main(){
 let mut z=Zone::new(ZoneId::Glasswater,32,24);z.ground.fill(rgb(0x71818f));z.ambient=Rgb::new(0.4,0.45,0.55);
 z.props.push(Prop::new("cinder_markers","pillar",160.8,160.2));let points=[(100.0,100.0,0.8)];
 let a=staged(&z,&NoRegion,(0,0),&points,false,None);let b=staged(&z,&NoRegion,(0,0),&points,true,None);
 let d=diff(&a,&b);println!("native_conservation changes={} max_removal={:.7}",d.0,d.1);assert!(d.0>20);
 for (old,new)in a.light.iter().zip(&b.light){assert!(new.r<=old.r+1e-5&&new.g<=old.g+1e-5&&new.b<=old.b+1e-5,"invented light");}
 let pixel=a.light.iter().zip(&b.light).position(|(p,q)|(p.r-q.r).abs()>1e-5).unwrap();let xy=((pixel%W)as i32,(pixel/W)as i32);
 let aa=staged(&z,&NoRegion,(0,0),&points,false,Some(xy));let bb=staged(&z,&NoRegion,(0,0),&points,true,Some(xy));assert_eq!(aa.light[pixel],bb.light[pixel]);println!("actor_protection at={xy:?} pass");
 let wall_xy=((pixel%W)as i32,(pixel/W)as i32);for tile in [glintblade::world::Tile::Wall,glintblade::world::Tile::Deep,glintblade::world::Tile::Lava,glintblade::world::Tile::Void]{z.set_tile(wall_xy.0/16,wall_xy.1/16,tile);let off=staged(&z,&NoRegion,(0,0),&points,false,None);let on=staged(&z,&NoRegion,(0,0),&points,true,None);assert_eq!(off.light[pixel],on.light[pixel],"flat ground shadow reached excluded receiver {tile:?}");println!("receiver_exclusion {tile:?} pass");}z.set_tile(wall_xy.0/16,wall_xy.1/16,glintblade::world::Tile::Floor);
 let low=[(100.0,100.0,0.01)];let low_off=staged(&z,&NoRegion,(0,0),&low,false,None);let low_on=staged(&z,&NoRegion,(0,0),&low,true,None);let dl=diff(&low_off,&low_on);println!("dim_source changes={} max_removal={:.8}",dl.0,dl.1);assert!(dl.0>0,"dim source was abruptly filtered");
 let cell=z.props[0].sheet().unwrap();let anim=cell.anim(&z.props[0].anim).unwrap();let art=cell.cell(anim,0);let bottom=(0..art.h).rev().find(|&y|(0..art.w).any(|x|art.at(x,y).a)).unwrap();let ground=z.props[0].y.floor()-cell.anchor_of(anim).1 as f32+bottom as f32;
 let mut energies=Vec::new();for d in [9.9,10.1]{let point=[(z.props[0].x+d,ground,0.8)];let off=staged(&z,&NoRegion,(0,0),&point,false,None);let on=staged(&z,&NoRegion,(0,0),&point,true,None);let energy: f32=off.light.iter().zip(&on.light).map(|(a,b)|(a.r-b.r).max(0.0)+(a.g-b.g).max(0.0)+(a.b-b.b).max(0.0)).sum();println!("near_source distance={d} shadow_energy={energy:.7}");energies.push(energy);}
 assert!(energies[0]>0.0&&energies[1]>0.0,"near source drops entire caster");let jump=(energies[1]-energies[0]).abs()/energies[0].max(energies[1]);assert!(jump<0.20,"near source hard pop: relative shadow-energy jump={jump}");
 let many:Vec<_>=(0..9).map(|n|(70.0+n as f32*11.0,85.0+n as f32*4.0,0.10+n as f32*0.025)).collect();let many_off=staged(&z,&NoRegion,(0,0),&many,false,None);let many_on=staged(&z,&NoRegion,(0,0),&many,true,None);let mut sum=vec![Rgb::BLACK;W*H];for point in &many{let off=staged(&z,&NoRegion,(0,0),&[*point],false,None);let on=staged(&z,&NoRegion,(0,0),&[*point],true,None);for i in 0..W*H{sum[i]+=off.light[i]-on.light[i];}}let mut max_error=0.0f32;for i in 0..W*H{let actual=many_off.light[i]-many_on.light[i];let d=actual-sum[i];max_error=max_error.max(d.r.abs().max(d.g.abs()).max(d.b.abs()));}assert!(max_error<0.00002,"emitter budget/contribution conservation mismatch: {max_error}");let mut reversed=many.clone();reversed.reverse();let reverse=staged(&z,&NoRegion,(0,0),&reversed,true,None);let order=diff(&many_on,&reverse);assert!(order.1<0.00002);println!("all9_emitters_composable max_error={max_error:.8} reverse_order_max_delta={:.8}",order.1);
 let shifted=staged(&z,&NoRegion,(17,11),&points,true,None);let stable=aligned(&b,&shifted);println!("native_camera_overlap differing={} max_delta={:.8}",stable.0,stable.1);assert_eq!(stable.0,0);
 z.props[0].alive=false;let aa=staged(&z,&NoRegion,(0,0),&points,false,None);let bb=staged(&z,&NoRegion,(0,0),&points,true,None);assert_eq!(aa.light,bb.light);println!("dead_prop_shadow_removed pass");
 for id in [ZoneId::Bloomwild,ZoneId::Glasswater,ZoneId::Cinder,ZoneId::Summit]{
  let (z,r)=regions::build(id);let native=z.props.iter().find(|p|p.alive&&!p.canopy&&p.sheet().and_then(|s|s.anim(&p.anim)).is_some() && (id!=ZoneId::Summit || p.sheet=="summit_pillars"));
  let focus=match native{Some(p)=>(p.x,p.y),None=>if id==ZoneId::Glasswater{(532.8,64.0)}else{z.player_start}};
  let camera=((focus.0 as i32-180).max(0),(focus.1 as i32-150).max(0));let points=[(focus.0-60.0,focus.1-50.0,0.8)];
  let a=staged(&z,r.as_ref(),camera,&points,false,None);let b=staged(&z,r.as_ref(),camera,&points,true,None);let d=diff(&a,&b);
  let c=staged(&z,r.as_ref(),(camera.0+17,camera.1+11),&points,true,None);let stable=aligned(&b,&c);assert_eq!(stable.0,0,"camera-dependent real-scene occlusion");
  println!("real {id:?} focus={focus:?} affected={} max_removal={:.6} camera_max_delta={:.8}",d.0,d.1,stable.1);if id!=ZoneId::Bloomwild {assert!(d.0>10,"real scenery does not cast live point shadow");}

 }
 let mut sky=Zone::new(ZoneId::Bloomwild,32,24);sky.ground.fill(glintblade::world::HOLE);sky.ambient=Rgb::new(0.4,0.45,0.55);let sky_off=staged(&sky,&NoRegion,(0,0),&points,false,None);let sky_on=staged(&sky,&NoRegion,(0,0),&points,true,None);assert_eq!(sky_off.light,sky_on.light);assert!(sky_on.light.iter().all(|&l|l==sky.ambient));println!("sky_point_spill_excluded pass");
 let mut sky=Zone::new(ZoneId::Bloomwild,32,24);sky.ground.fill(rgb(0x71818f));for y in 0..16{for x in 0..16{std::hint::black_box(staged(&sky,&NoRegion,(32+x,32+y),&[],true,None));}}println!("weather_all256_camera_remainders no_panic");
 let mut f=b;assert!(!f.point_lights.is_empty());f.begin(Rgb::BLACK,Rgb::WHITE);assert!(f.point_lights.is_empty());println!("frame_emitter_reset pass");
}
