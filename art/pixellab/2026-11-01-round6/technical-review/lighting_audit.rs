pub use glintblade::world;
use glintblade::{regions,world::{ZoneId,Tile},entities::Enemy,gfx::{Frame,Rgb,W,H},gfx::frame::PointLight};
fn hash(h:&mut u64,v:u8){*h^=v as u64;*h=h.wrapping_mul(1099511628211);}
fn hash_rgb(h:&mut u64,p:Rgb){for v in[p.r,p.g,p.b]{for b in v.to_bits().to_le_bytes(){hash(h,b);}}}
fn same(a:PointLight,b:PointLight,f:&Frame)->bool{a.x==(b.x-f.cam_x as f32)+f.cam_x as f32&&a.y==(b.y-f.cam_y as f32)+f.cam_y as f32&&a.radius==b.radius&&a.color==b.color&&a.intensity==b.intensity}
fn mark_native(mask:&mut[bool],f:&Frame,cell:&glintblade::gfx::Cell,anchor:(i32,i32),x:f32,y:f32,flip:bool){let ox=x.floor()as i32-if flip{cell.w-1-anchor.0}else{anchor.0};let oy=y.floor()as i32-anchor.1;for cy in 0..cell.h{for cx in 0..cell.w{if cell.at(cx,cy).a{let wx=ox+if flip{cell.w-1-cx}else{cx};if let Some(i)=Frame::idx(wx-f.cam_x,oy+cy-f.cam_y){mask[i]=true;}}}}}
fn main(){
 let(z,r)=regions::build(ZoneId::Glasswater);let mut checked=0usize;let mut centroids=0usize;
 for t in[0,317,1100]{let mut casters=Vec::new();r.light_occluders(t,&mut casters);for c in &casters{if let Some(own)=c.own_light{for(dx,dy)in[(0,0),(7,3),(15,15)]{let mut f=Frame::new();f.cam_x=c.x.floor()as i32-192+dx;f.cam_y=c.y.floor()as i32-108+dy;f.begin(Rgb::BLACK,z.ambient);r.light(&z,&mut f,t);assert!(f.point_lights.iter().any(|&p|same(p,own,&f)),"self source mismatch at {},{} tick{} camera{},{}",c.x,c.y,t,dx,dy);checked+=1;}}
  if t==0&&c.cell.px.iter().any(|p|p.emit){let(mut sx,mut sy,mut n)=(0f32,0f32,0f32);for y in 0..c.cell.h{for x in 0..c.cell.w{if c.cell.at(x,y).a&&c.cell.at(x,y).emit{sx+=x as f32;sy+=y as f32;n+=1.0;}}}if n>0.0 {if let Some(own)=c.own_light{let ex=c.x+if c.flip{c.anchor.0 as f32-sx/n}else{sx/n-c.anchor.0 as f32};let ey=c.y+sy/n-c.anchor.1 as f32; // Large native crystals are the crest change under audit; small sprigs have independent authored height.
   if z.props.iter().any(|p|p.sheet=="glasswater_blocker"&&p.light.is_some()&&p.x==c.x&&p.y==c.y){assert!((own.x-ex).abs()<0.001&&(own.y-ey).abs()<0.001,"crest location {},{} expected{},{}",own.x,own.y,ex,ey);centroids+=1;}
  }}}
 }}
 println!("OWN_EMITTER checks={} exact_all_crystal_centroids={}",checked,centroids);
 court_probe::run();
 for(id,scenes)in[(ZoneId::Bloomwild,vec![(392,648),(800,360),(1100,480),(408,176)]),(ZoneId::Glasswater,vec![(432,250),(600,90),(1150,300),(1160,700)]),(ZoneId::Cinder,vec![(205,590),(646,468),(950,460),(1120,200)]),(ZoneId::Summit,vec![(196,680),(448,520),(720,352),(900,170)])]{
  let(z,r)=regions::build(id);
  for(s,(cx,cy))in scenes.iter().enumerate(){let t=300+s as u64*317;let mut f=Frame::new();f.cam_x=cx-192;f.cam_y=cy-108;f.begin(z.bg,z.ambient);r.draw_backdrop(&z,&mut f,t);z.draw_ground(&mut f);r.draw_ground(&z,&mut f,t);for p in &z.props{p.draw(&mut f,t,1.0);}let mut items=Vec::new();r.sorted_items(&z,&f,&mut items);for(_,i)in items{r.draw_item(&z,&mut f,i,t);}
   for(kind,x,y)in&z.enemies{Enemy::spawn(*kind,*x,*y).draw(&mut f);}
   for y in 90..105{for x in 180..187{f.put(x,y,glintblade::gfx::rgb(0x9fcfa6));f.mark_actor(x,y);}}
   let mut native=vec![false;W*H];for p in &z.props{if !p.alive{continue;}if let Some(sh)=p.sheet(){if let Some(a)=sh.anim(&p.anim){mark_native(&mut native,&f,sh.cell(a,a.frame_at(t as u32+p.t0,true)),sh.anchor_of(a),p.x,p.y,p.flip);}}}let mut cs=Vec::new();r.light_occluders(t,&mut cs);for c in cs{mark_native(&mut native,&f,c.cell,c.anchor,c.x,c.y,c.flip);}
   for l in&z.lights{f.add_light_w(l.x,l.y,l.radius,l.color,l.intensity);}for p in&z.props{if let Some((c,r,i,x,y))=p.light{f.add_light_w(p.x+x,p.y+y,r,c,i);}}r.light(&z,&mut f,t);
   if std::env::args().any(|a|a=="--probe-light"){f.add_light_w(*cx as f32-45.0,*cy as f32-30.0,170.0,glintblade::gfx::rgb(0x81d1e0),0.85);}
   let(albedo,emissive,water,actors,points)=(f.albedo.clone(),f.emissive.clone(),f.water.clone(),f.actor_pixels.clone(),format!("{:?}",f.point_lights));glintblade::gfx::lighting::apply(&z,r.as_ref(),&mut f,t);
   assert_eq!(f.albedo,albedo,"albedo changed");assert_eq!(f.emissive,emissive,"emission changed");assert_eq!(f.water,water,"water reflection mask changed");assert_eq!(f.actor_pixels,actors,"actor coverage changed");assert_eq!(format!("{:?}",f.point_lights),points,"point list changed");
   let(mut protect,mut complete,mut counts)=(1469598103934665603u64,1469598103934665603u64,[0usize;3]);
   for i in 0..W*H{let p=f.light[i];assert!(p.r.is_finite()&&p.g.is_finite()&&p.b.is_finite(),"nonfinite lighting");hash_rgb(&mut complete,p);let(wx,wy)=(f.cam_x+(i%W)as i32,f.cam_y+(i/W)as i32);let j=if wx>=0&&wy>=0&&wx<z.pw&&wy<z.ph{Some((wy*z.pw+wx)as usize)}else{None};let sky=j.is_none_or(|j|z.ground[j].r<0.0)&&!native[i]&&!actors[i];let wet=matches!(z.tile_at(wx as f32,wy as f32),Tile::Deep|Tile::Lava|Tile::Void)&&!native[i]&&!actors[i];
    if actors[i]||native[i]||sky||wet{hash_rgb(&mut protect,p);}counts[0]+=usize::from(actors[i]);counts[1]+=usize::from(native[i]);counts[2]+=usize::from(sky||wet);
   }
   println!("LIGHT {:?} scene={} protected={:016x} all={:016x} actor={} native={} nonreceiver={} material_pixels={}",id,s,protect,complete,counts[0],counts[1],counts[2],W*H);
  }
 }
}
mod court_probe{
 use glintblade::{gfx::{Rgb,rgb},world::{Zone,ZoneId,Tile}};
 #[derive(Clone,Copy)]pub struct Rect{x0:i32,y0:i32,x1:i32,y1:i32}
 impl Rect{fn clamp(self,z:&Zone)->Self{Self{x0:self.x0.clamp(0,z.pw),x1:self.x1.clamp(0,z.pw),y0:self.y0.clamp(0,z.ph),y1:self.y1.clamp(0,z.ph)}}fn len(self)->usize{((self.x1-self.x0)*(self.y1-self.y0))as usize}fn index(self,x:i32,y:i32)->Option<usize>{(x>=self.x0&&y>=self.y0&&x<self.x1&&y<self.y1).then(||((y-self.y0)*(self.x1-self.x0)+x-self.x0)as usize)}}
 #[path="/home/krp/Projects/glintblade/src/gfx/lighting/cinder_floor.rs"]mod actual_floor;
 pub fn run(){let(z,_)=glintblade::regions::build(ZoneId::Cinder);let floor=actual_floor::Floor::bake(&z).unwrap();let(mut count,mut wet,mut checked)=(0,0,0);for y in 0..z.ph{for x in 0..z.pw{checked+=1;if let Some((warm,mul))=floor.at(x,y){assert_eq!(z.tile_at(x as f32+0.5,y as f32+0.5),Tile::Floor);assert!(z.gget(x,y).r>=0.0);assert!(warm.r.is_finite()&&mul.is_finite());count+=1;}else if z.tile_at(x as f32+0.5,y as f32+0.5).liquid(){wet+=1;}}}assert!(count>1000);println!("COURT checked={} actual_floor_receivers={} excluded_liquid={}",checked,count,wet);}
}
