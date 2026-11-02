use glintblade::{regions,entities::Enemy,world::{ZoneId,Tile}};
fn hash(h:&mut u64,v:u8){*h^=v as u64;*h=h.wrapping_mul(1099511628211);}
fn main(){
for id in [ZoneId::Bloomwild,ZoneId::Glasswater,ZoneId::Cinder,ZoneId::Summit]{
 let(z,_)=regions::build(id);
 let(mut h,mut wet,mut types,mut count)=(1469598103934665603u64,1469598103934665603u64,[0u64;6],0u64);
 for y in 0..z.ph{for x in 0..z.pw{let t=z.tile_at(x as f32+0.5,y as f32+0.5);hash(&mut h,t as u8);hash(&mut wet,u8::from(matches!(t,Tile::Deep|Tile::Shallow)));types[t as usize]+=1;count+=1;}}
 println!("ZONE {:?} native={} surface={:016x} wet={:016x} types={:?} props={} enemies={} start={:?} shard={:?} gate={:?} boss={:?}",id,count,h,wet,types,z.props.len(),z.enemies.len(),z.player_start,z.shard,z.gate,z.boss_pos);
 for(i,p)in z.props.iter().enumerate(){
  let(mut h,mut tested,mut opaque,mut support)=(1469598103934665603u64,0,0,[0u64;6]);
  let mut rect_hash=1469598103934665603u64;for r in p.collision_rects(){for v in r{for b in v.to_bits().to_le_bytes(){hash(&mut rect_hash,b);}}}
  if let Some(b)=p.collision_bounds(){for y in b[1].floor()as i32-1..b[3].ceil()as i32+1{for x in b[0].floor()as i32-1..b[2].ceil()as i32+1{
   let blocks=p.blocks(x as f32+0.5,y as f32+0.5,0.01,0.01);hash(&mut h,u8::from(blocks));tested+=1;if blocks{opaque+=1;support[z.tile_at(x as f32+0.5,y as f32+0.5)as usize]+=1;}
  }}}
  println!("PROP {} {} {} pos={:?} flip={} footprint={:?} bounds={:?} sort={} breakable={} hp={} alive={} heal={} collision={:016x} rects={} rect_hash={:016x} tested={} opaque={} support={:?}",i,p.sheet,p.anim,(p.x,p.y),p.flip,p.footprint,p.collision_bounds(),p.sort_y(),p.breakable,p.hp,p.alive,p.heal_drop,h,p.collision_rects().len(),rect_hash,tested,opaque,support);
 }
 for(k,x,y)in &z.enemies{let e=Enemy::spawn(*k,*x,*y);println!("ENEMY {:?} pos={:?} r={} hp={} max_hp={} blocked={}",k,(*x,*y),e.body.r,e.body.hp,e.body.max_hp,z.blocked(*x,*y,e.body.r));}
 // Test movement's native tile/prop integration at deterministic world locations.
 let(mut motion,mut probes)=(1469598103934665603u64,0usize);
 for y in (8..z.ph-8).step_by(29){for x in(8..z.pw-8).step_by(31){for r in[4.0,5.0,6.0]{for (dx,dy)in[(15.0,0.0),(-15.0,0.0),(0.0,15.0),(0.0,-15.0),(8.0,8.0),(-8.0,-8.0)]{
  let(a,b,c)=z.move_box(x as f32+0.25,y as f32+0.75,r,dx,dy);for v in a.to_bits().to_le_bytes().into_iter().chain(b.to_bits().to_le_bytes()).chain([u8::from(c)]){hash(&mut motion,v);}probes+=1;
 }}}}
 println!("MOVEMENT probes={} hash={:016x}",probes,motion);
}}
