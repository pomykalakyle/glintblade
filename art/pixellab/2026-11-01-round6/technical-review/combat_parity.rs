use glintblade::{entities::{Enemy,EnemyCtx},fx::Fx,regions,world::ZoneId,combat::{Hit,Shape,Team}};
fn hash(h:&mut u64,s:&str){for v in s.bytes(){*h^=v as u64;*h=h.wrapping_mul(1099511628211);}}
fn main(){for id in[ZoneId::Bloomwild,ZoneId::Glasswater,ZoneId::Cinder]{let(z,_)=regions::build(id);for(kind,x,y)in&z.enemies{
 let mut e=Enemy::spawn(*kind,*x,*y);e.awake=true;let mut fx=Fx::new(947);let mut ps=Vec::new();let mut hs=Vec::new();let(mut digest,mut hits,mut maxprojectiles)=(1469598103934665603u64,0usize,0usize);
 for t in 0..2400 {let a=t as f32*0.011;let player=(*x+a.cos()*66.0,*y+a.sin()*48.0);
  e.update(&mut EnemyCtx{player,player_alive:true,zone:&z,fx:&mut fx,hits:&mut hs,projectiles:&mut ps,t});
  if t==700{let hit=Hit::new(Shape::Circle{x:e.body.x,y:e.body.y,r:20.0},Team::Player,1,900001).knock(1.3);e.on_hit(&hit,&mut EnemyCtx{player,player_alive:true,zone:&z,fx:&mut fx,hits:&mut hs,projectiles:&mut ps,t});}
  for p in &mut ps {p.update(&z,&mut fx,player);}
  maxprojectiles=maxprojectiles.max(ps.len());hits+=hs.len();
  let mut copy=fx.rng.clone();hash(&mut digest,&format!("{:?} {:?} {:?} {:?} {} {} {:?}",e.body,ps,hs,copy.next_u32(),fx.hitstop,fx.trauma,fx.sounds));
  ps.retain(|p|p.alive);hs.clear();fx.sounds.clear();fx.tick();
 }
 println!("{:?} {:?} {},{} ticks=2400 digest={:016x} hits={} maxprojectiles={}",id,kind,x,y,digest,hits,maxprojectiles);
}}}
