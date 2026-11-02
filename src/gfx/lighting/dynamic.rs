//! Slow world-space weather and occlusion of actual live point emitters.
//! No light is invented here; native source alpha removes only the emitter's own
//! ground contribution. Emission, actor ink, sky and cached geometry remain separate.

use super::*;
use crate::gfx::frame::PointLight;
use crate::util::noise2;

pub(super) fn strength() -> f32 {
    #[cfg(not(target_arch="wasm32"))]
    {
        static GAIN: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
        *GAIN.get_or_init(|| match std::env::var("GLINT_LIGHT_TRIAL").as_deref() {
            Ok("off")=>0.0, Ok("balanced")=>1.0, Ok("dramatic")=>1.35, _=>0.65,
        })
    }
    #[cfg(target_arch="wasm32")]
    { 0.65 }
}

const GRID: i32 = 16;
pub(super) struct Weather {
    clear: Rgb,
    cloudy: Rgb,
    x: i32,
    y: i32,
    cols: usize,
    density: Vec<f32>,
}

impl Weather {
    pub(super) fn new(id: ZoneId, cam_x: i32, cam_y: i32, t: u64, gain: f32) -> Option<Self> {
        // Night trials made broad blue cutouts on the Summit floor. Keep its
        // restrained masonry and real local emitters; drifting weather belongs
        // to the sunlit garden, where a passing cloud has a clear visual cause.
        if id != ZoneId::Bloomwild { return None; }
        let (x,y) = (cam_x.div_euclid(GRID)*GRID,cam_y.div_euclid(GRID)*GRID);
        let (cols,rows) = (W.div_ceil(GRID as usize)+2,H.div_ceil(GRID as usize)+2);
        let drift = t as f32*0.000075;
        let mut density = Vec::with_capacity(cols*rows);
        for gy in 0..rows { for gx in 0..cols {
            let (wx,wy) = ((x+gx as i32*GRID) as f32,(y+gy as i32*GRID) as f32);
            let n = noise2(wx/270.0-drift,wy/210.0+drift*0.35,1751)*0.8
                +noise2(wx/105.0-drift*0.4,wy/155.0,1753)*0.2;
            let a = ((n-0.29)/0.40).clamp(0.0,1.0);
            density.push(a*a*(3.0-2.0*a));
        }}
        // Palette choice and gain are uniform for the frame. Weather is affine in
        // cloud density; keep that work out of the native-pixel lighting loop.
        let gain = gain.clamp(0.0,1.5);
        let sun=Rgb::new(1.04,1.025,0.99);
        let (clear,cloudy) = (Rgb::WHITE.lerp(sun,gain),
            (Rgb::new(0.78,0.86,0.98)-sun)*(0.72*gain));
        Some(Self { clear,cloudy,x,y,cols,density })
    }

    #[cfg(test)]
    fn tint(&self,wx:i32,wy:i32)->Rgb {
        let (gx,gy) = ((wx-self.x)/GRID,(wy-self.y)/GRID);
        let (fx,fy) = (((wx-self.x)%GRID) as f32/GRID as f32,((wy-self.y)%GRID) as f32/GRID as f32);
        let i = gy as usize*self.cols+gx as usize;
        let top = self.density[i]*(1.0-fx)+self.density[i+1]*fx;
        let bot = self.density[i+self.cols]*(1.0-fx)+self.density[i+self.cols+1]*fx;
        let cloud = top*(1.0-fy)+bot*fy;
        self.clear + self.cloudy*cloud
    }

    /// Fill one scanline from canonical world-grid segments. Even the skipped prefix
    /// advances from the cell's true left edge, so changing a camera remainder cannot
    /// change rounding or accumulate a different tint at the same world pixel.
    pub(super) fn row(&self,wx:i32,wy:i32,out:&mut [Rgb;W]) {
        let gy=((wy-self.y)/GRID)as usize;
        let fy=((wy-self.y)%GRID)as f32/GRID as f32;
        let mut gx=((wx-self.x)/GRID)as usize;
        let mut skip=(wx-self.x)%GRID;
        let mut written=0;
        while written<W {
            let i=gy*self.cols+gx;
            let left=self.density[i]*(1.0-fy)+self.density[i+self.cols]*fy;
            let right=self.density[i+1]*(1.0-fy)+self.density[i+self.cols+1]*fy;
            let mut value=self.clear+self.cloudy*left;
            let step=self.cloudy*((right-left)/GRID as f32);
            for _ in 0..skip { value+=step; }
            let count=((GRID-skip)as usize).min(W-written);
            for pixel in &mut out[written..written+count] { *pixel=value; value+=step; }
            written+=count;
            gx+=1;
            skip=0;
        }
    }
}

/// A borrowed native silhouette, with the true opaque bottom independent of atlas anchors.
struct NativeCaster<'a> {
    source: crate::world::LightOccluder<'a>,
    bottom: i32,
    base: f32,
    x0: i32,
    center: f32,
}

fn native_casters<'a>(zone: &'a Zone, region: &'a dyn Region, t: u64) -> Vec<NativeCaster<'a>> {
    let mut sources = Vec::new();
    for p in zone.props.iter().filter(|p| p.alive && !p.canopy) {
        // Collision-only companions have no visible ink. Native sprites cast even when their
        // solidity lives in separate row-span props (the Summit's structural bases).
        let Some(sheet) = p.sheet() else { continue };
        let Some(anim) = sheet.anim(&p.anim) else { continue };
        let cell = sheet.cell(anim, anim.frame_at(t as u32 + p.t0, true));
        let wobble = if p.wobble > 0.0 { ((p.wobble * 1.7).sin() * p.wobble * 0.25).round() } else { 0.0 };
        let own_light = p.light.map(|(color, radius, intensity, ox, oy)| PointLight {
            x: p.x + ox, y: p.y + oy, radius, color, intensity, height: -oy,
        });
        sources.push(crate::world::LightOccluder { cell, anchor: sheet.anchor_of(anim),
            x: p.x + wobble, y: p.y, flip: p.flip, own_light });
    }
    region.light_occluders(t, &mut sources);
    sources.into_iter().filter_map(|source| {
        let cell = source.cell;
        let bottom = (0..cell.h).rev().find(|&y| (0..cell.w).any(|x| cell.at(x,y).a))?;
        let x0 = source.x.floor() as i32 - if source.flip { cell.w - 1 - source.anchor.0 } else { source.anchor.0 };
        let base = source.y.floor() + (bottom + 1 - source.anchor.1) as f32;
        let center = x0 as f32 + cell.w as f32 * 0.5;
        Some(NativeCaster { source, bottom, base, x0, center })
    }).collect()
}

fn same_emitter(a: PointLight, b: PointLight, f: &Frame) -> bool {
    // add_light_w stores its screen-to-world result. Canonicalize the authored emitter
    // through that same f32 subtraction/addition; no radius or position tolerance can
    // accidentally suppress a neighbouring (or coincident but differently coloured) source.
    a.x == (b.x - f.cam_x as f32) + f.cam_x as f32
        && a.y == (b.y - f.cam_y as f32) + f.cam_y as f32
        && a.radius == b.radius && a.color == b.color && a.intensity == b.intensity
}

/// Subtract each emitter's own attenuated term only on physical receiving tops.
/// There is no camera-ranked emitter budget or brightness/radius selection: dim lights,
/// changing rings and panning cameras retain the same world-space shadow overlap.
pub(super) fn occlude_points(zone: &Zone, region: &dyn Region, field: &Field, f: &mut Frame, t: u64, gain: f32) {
    if gain <= 0.0 || zone.id == ZoneId::Bloomwild || f.point_lights.is_empty() { return; }
    let casters = native_casters(zone, region, t);
    // Region-owned native art can extend beyond the static procedural plane proxy.
    // Ground shadows must never paint onto those visible crystal/stone surfaces.
    let mut native_ink = vec![false; W * H];
    for caster in &casters {
        let source = &caster.source;
        let cell = source.cell;
        let oy = source.y.floor() as i32 - source.anchor.1;
        let left = (f.cam_x - caster.x0).max(0);
        let right = (f.cam_x + W as i32 - caster.x0).min(cell.w);
        let top = (f.cam_y - oy).max(0);
        let bottom = (f.cam_y + H as i32 - oy).min(cell.h);
        for y in top..bottom { for x in left..right {
            if cell.at(if source.flip { cell.w - 1 - x } else { x }, y).a {
                let i = (oy + y - f.cam_y) as usize * W + (caster.x0 + x - f.cam_x) as usize;
                native_ink[i] = true;
            }
        }}
    }
    let mut mask = vec![0.0f32; W * H];
    for point in f.point_lights.clone() {
        mask.fill(0.0);
        let mut projected = false;
        for caster in &casters {
            let source = &caster.source;
            let cell = source.cell;
            if source.own_light.is_some_and(|own| same_emitter(point, own, f)) { continue; }
            let (dx, dy) = (caster.center - point.x, caster.base - point.y);
            let distance = dx.hypot(dy);
            // Pure geometric rejection includes every possible projected source pixel.
            if distance > point.radius + cell.w as f32 + cell.h as f32 { continue; }
            let height = caster.bottom as f32;
            if caster.x0 as f32 - height >= (f.cam_x + W as i32) as f32
                || (caster.x0 + cell.w) as f32 + height < f.cam_x as f32
                || caster.base - height * 0.70 >= (f.cam_y + H as i32) as f32
                || caster.base + height * 0.70 < f.cam_y as f32 { continue; }
            // Direction and amplitude tend continuously to zero at a coincident source.
            // At ordinary distances the shadow follows the source, capped at native height.
            let denominator = distance.max(64.0);
            let dir = (dx / denominator, dy / denominator * 0.70);
            let near = distance / (distance * distance + 64.0).sqrt();
            for y in 0..=caster.bottom {
                let altitude = (caster.bottom - y) as f32;
                for x in 0..cell.w {
                    if !cell.at(x,y).a { continue; }
                    let wx = caster.x0 + if source.flip { cell.w - 1 - x } else { x };
                    for sub in 0..2 {
                        let h = altitude + sub as f32 * 0.5;
                        let sx = (wx as f32 + h * dir.0).round() as i32 - f.cam_x;
                        let sy = (caster.base + h * dir.1).round() as i32 - f.cam_y;
                        if let Some(i) = Frame::idx(sx,sy) {
                            mask[i] = mask[i].max(near);
                            projected = true;
                        }
                    }
                }
            }
        }
        if !projected { continue; }
        let (px, py) = (point.x - f.cam_x as f32, point.y - f.cam_y as f32);
        let x0 = ((px - point.radius).floor() as i32).max(0);
        let x1 = ((px + point.radius).ceil() as i32).min(W as i32 - 1);
        let y0 = ((py - point.radius).floor() as i32).max(0);
        let y1 = ((py + point.radius).ceil() as i32).min(H as i32 - 1);
        let inv_r2 = 1.0 / (point.radius * point.radius);
        let color = point.color.scale(point.intensity);
        for y in y0..=y1 { for x in x0..=x1 {
            let i = y as usize * W + x as usize;
            if mask[i] == 0.0 || f.actor_pixels[i] || native_ink[i] { continue; }
            let (wx, wy) = (x + f.cam_x, y + f.cam_y);
            let Some(j) = field.index(wx, wy) else { continue };
            if !field.receiving[j] || field.raised[j]
                || !matches!(zone.tile_at(wx as f32, wy as f32), Tile::Floor | Tile::Shallow) { continue; }
            // Same operation order, f32 screen coordinates and half-pixel sampling as add_light.
            let (dx, dy) = (x as f32 + 0.5 - px, y as f32 + 0.5 - py);
            let d2 = (dx * dx + dy * dy) * inv_r2;
            if d2 >= 1.0 { continue; }
            let radial = 1.0 - d2;
            let contribution = color * (radial * radial);
            let remove = contribution * ((0.86 * gain).clamp(0.0, 1.0) * mask[i]);
            let old = f.light[i];
            let light = old - remove;
            f.light[i] = Rgb::new(light.r.max(f.ambient.r.min(old.r)),
                light.g.max(f.ambient.g.min(old.g)), light.b.max(f.ambient.b.min(old.b)));
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_owned_native_surfaces_never_receive_flat_ground_shadows() {
        let (z, region) = crate::regions::build(ZoneId::Glasswater);
        let field = Field::bake(&z, region.as_ref(), geometry_key(&z));
        let casters = native_casters(&z, region.as_ref(), 900);
        assert!(casters.len() > 30, "audit the actual cave scenery, including noncolliding cells");
        for (dx, dy) in [(-60.0, -50.0), (-60.0, 50.0), (60.0, 50.0)] {
            let mut f = Frame::new();
            f.cam_x = 352;
            f.begin(Rgb::BLACK, z.ambient);
            f.add_light_w(532.8 + dx, 64.0 + dy, 145.0, rgb(0x81d1e0), 0.8);
            let original = f.light.clone();
            occlude_points(&z, region.as_ref(), &field, &mut f, 900, 1.0);
            let mut checked = 0;
            for caster in &casters {
                let s = &caster.source;
                let oy = s.y.floor() as i32 - s.anchor.1;
                for y in 0..s.cell.h { for x in 0..s.cell.w {
                    if !s.cell.at(x, y).a { continue; }
                    let wx = caster.x0 + if s.flip { s.cell.w - 1 - x } else { x };
                    if let Some(i) = Frame::idx(wx - f.cam_x, oy + y - f.cam_y) {
                        assert_eq!(f.light[i], original[i], "flat ground shadow altered native cave ink");
                        checked += 1;
                    }
                }}
            }
            assert!(checked > 500);
        }
    }

    #[test]
    fn weather_is_world_anchored_slow_and_neutral_at_zero_gain() {
        assert!(Weather::new(ZoneId::Summit,73,121,900,1.0).is_none(),
            "night masonry retains local-source lighting without a cloud decal");
        for id in [ZoneId::Bloomwild] {
            let a=Weather::new(id,73,121,900,1.0).unwrap();
            let b=Weather::new(id,109,139,900,1.0).unwrap();
            let next=Weather::new(id,73,121,901,1.0).unwrap();
            for (x,y) in [(150,150),(242,230),(360,312)] {
                let (p,q,n)=(a.tint(x,y),b.tint(x,y),next.tint(x,y));
                assert_eq!(Weather::new(id,73,121,900,0.0).unwrap().tint(x,y),Rgb::WHITE);
                assert!((p-q).max_c().abs()<0.00001 && (q-p).max_c().abs()<0.00001);
                assert!((p-n).max_c().abs()<0.001 && (n-p).max_c().abs()<0.001);
            }
        }
    }

    #[test]
    fn weather_covers_every_native_corner_for_all_camera_grid_remainders() {
        for cx in -16..16 { for cy in -16..16 {
            let weather = Weather::new(ZoneId::Bloomwild,cx,cy,900,1.0).unwrap();
            for (sx,sy) in [(0,0),(W as i32-1,0),(0,H as i32-1),(W as i32-1,H as i32-1)] {
                let tint = weather.tint(cx+sx,cy+sy);
                assert!(tint.r.is_finite() && tint.g.is_finite() && tint.b.is_finite());
            }
        }}
    }

    #[test]
    fn weather_scanlines_match_bilinear_tints_and_keep_camera_overlap_exact() {
        let mut a=[Rgb::WHITE;W]; let mut b=[Rgb::WHITE;W];
        for id in [ZoneId::Bloomwild] { for gain in [0.0,0.65,1.0,1.35] {
            for remainder in 0..16 {
                let (cx,cy)=(-32+remainder,73+remainder);
                let weather=Weather::new(id,cx,cy,900,gain).unwrap();
                let shifted=Weather::new(id,cx+17,cy+11,900,gain).unwrap();
                for y in [0,11,71,143,H as i32-1] {
                    let wy=cy+y;
                    weather.row(cx,wy,&mut a);
                    if y>=11 { shifted.row(cx+17,wy,&mut b); }
                    for x in 0..W {
                        let expected=weather.tint(cx+x as i32,wy);
                        assert!((a[x].r-expected.r).abs()<0.000003 && (a[x].g-expected.g).abs()<0.000003 && (a[x].b-expected.b).abs()<0.000003,
                            "scanline must preserve the continuous native palette");
                        if x>=17 && y>=11 { assert_eq!(a[x],b[x-17],"world-grid interpolation must remain exact across shifted cameras"); }
                    }
                }
            }
        }}
    }

    fn lit_shadow(z:&Zone,region:&dyn Region,field:&Field,cam:(i32,i32),point:PointLight)->(Frame,Vec<Rgb>) {
        let mut f=Frame::new(); f.cam_x=cam.0; f.cam_y=cam.1;
        f.begin(z.bg,z.ambient);
        f.add_light_w(point.x,point.y,point.radius,point.color,point.intensity);
        let original=f.light.clone();
        occlude_points(z,region,field,&mut f,900,1.0);
        (f,original)
    }

    #[test]
    fn cached_native_casts_and_planes_ignore_atlas_anchor_staging_gutters() {
        let z=Zone::new(ZoneId::Bloomwild,24,20);
        for (name,anim) in [("bloomwild_ruins","pillar_broken_tall"),
            ("bloomwild_arch","arch_post_right"),("summit_pillars","pillar_0"),("summit_stones","monolith_0")] {
            for flip in [false,true] {
                let mut p=crate::world::Prop::new(name,anim,160.8,160.8); p.flip=flip;
                let sheet=p.sheet().unwrap(); let anim=sheet.anim(anim).unwrap();
                let cell=sheet.cell(anim,0); let anchor=sheet.anchor_of(anim);
                let bottom=(0..cell.h).rev().find(|&y|(0..cell.w).any(|x|cell.at(x,y).a)).unwrap();
                let mut canonical=p.clone(); canonical.y=p.y.floor()+(bottom+1-anchor.1)as f32;
                let mut original=Field::bake(&z,&crate::world::NoRegion,geometry_key(&z));
                let mut normalized=Field::bake(&z,&crate::world::NoRegion,geometry_key(&z));
                let mut a=Planes::new(Rect::zone(&z)); let mut b=Planes::new(Rect::zone(&z));
                original.sprite(&z,&mut a,cell,anchor,&p);
                normalized.sprite(&z,&mut b,cell,(anchor.0,bottom+1),&canonical);
                assert_eq!(original.raised,normalized.raised,"staging anchor must preserve visible ink: {name}, flip={flip}");
                assert_eq!(original.dark,normalized.dark,"same native foot must cast at the same ground height: {name}, flip={flip}");
                for y in 0..z.ph { for x in 0..z.pw {
                    let (a,b)=(a.at(x,y),b.at(x,y));
                    assert_eq!((a.h,a.nx,a.ny,a.nz,a.edge),(b.h,b.nx,b.ny,b.nz,b.edge),
                        "material height must follow actual ink, not an atlas gutter: {name}, flip={flip}");
                }}
            }
        }
    }

    #[test]
    fn actual_glasswater_cells_and_noncolliding_summit_sprites_cast_on_real_tops() {
        for id in [ZoneId::Glasswater,ZoneId::Summit] {
            let (z,region)=crate::regions::build(id);
            let casters=native_casters(&z,region.as_ref(),900);
            assert!(casters.len()>=28,"{id:?} actual authored native cells must be visible casters");
            if id==ZoneId::Summit {
                assert!(z.props.iter().any(|p|p.alive && p.footprint.is_none() && p.sheet=="summit_pillars"));
            }
            let target=if id==ZoneId::Glasswater {
                casters.iter().min_by(|a,b| {
                    let distance=|c:&NativeCaster| (c.source.x-532.8).hypot(c.source.y-64.0);
                    distance(a).total_cmp(&distance(b))
                }).unwrap()
            } else {
                let p=z.props.iter().find(|p|p.alive && p.sheet=="summit_pillars").unwrap();
                casters.iter().find(|c|c.source.x==p.x && c.source.y==p.y).unwrap()
            };
            if id==ZoneId::Glasswater {
                let mut authored = Vec::new();
                region.light_occluders(900, &mut authored);
                assert!(authored.iter().any(|source| std::ptr::eq(source.cell, target.source.cell)
                    && source.x == target.source.x && source.y == target.source.y
                    && source.anchor == target.source.anchor),
                    "the measured caster must be an actual region-owned cell, not the new landmark prop");
            }
            let cam=((target.center as i32-180).max(0),(target.base as i32-140).max(0));
            let point=PointLight { x:target.center-60.0,y:target.base-50.0,radius:145.0,color:rgb(0x81d1e0),intensity:0.8, height:18.0 };
            let cache=z.lighting.borrow(); let field=cache.as_ref().unwrap();
            let (f,old)=lit_shadow(&z,region.as_ref(),field,cam,point);
            let changed:Vec<_>=f.light.iter().zip(&old).enumerate().filter(|(_, (a,b))|a!=b).map(|(i,_)|i).collect();
            assert!(changed.len()>5,"{id:?} actual native scene must cast measurable shadows");
            for &i in &changed {
                let (wx,wy)=(cam.0+(i%W)as i32,cam.1+(i/W)as i32);
                assert!(matches!(z.tile_at(wx as f32,wy as f32),Tile::Floor|Tile::Shallow),"no shadow on a vertical cliff or empty air");
            }
            let (shifted,_)=lit_shadow(&z,region.as_ref(),field,(cam.0+17,cam.1+11),point);
            for y in 11..H { for x in 17..W {
                let a=f.light[y*W+x]; let b=shifted.light[(y-11)*W+x-17];
                assert!((a.r-b.r).abs()<0.00001 && (a.g-b.g).abs()<0.00001 && (a.b-b.b).abs()<0.00001,
                    "{id:?} camera pan cannot change world-space shadow overlap");
            }}
        }
    }

    #[test]
    fn dim_sources_near_supports_and_exact_emitter_identity_remain_continuous() {
        let mut z=Zone::new(ZoneId::Glasswater,24,20);
        z.ground.fill(rgb(0x71818f)); z.ambient=Rgb::new(0.4,0.45,0.55);
        z.props.push(crate::world::Prop::new("cinder_markers","pillar",160.8,160.2));
        let field=Field::bake(&z,&crate::world::NoRegion,geometry_key(&z));
        let casters=native_casters(&z,&crate::world::NoRegion,900); let c=&casters[0];
        let point=PointLight{x:c.center-60.0,y:c.base-50.0,radius:145.0,color:rgb(0x81d1e0),intensity:0.01,height:18.0};
        let (dim,old)=lit_shadow(&z,&crate::world::NoRegion,&field,(0,0),point);
        assert!(dim.light.iter().zip(&old).any(|(a,b)|a!=b),"dim emitters must retain their own shadows");
        let energy=|distance:f32| {
            let point=PointLight{x:c.center-distance,y:c.base,radius:145.0,intensity:0.8,..point};
            let (f,old)=lit_shadow(&z,&crate::world::NoRegion,&field,(0,0),point);
            old.iter().zip(&f.light).map(|(a,b)|(a.r-b.r)+(a.g-b.g)+(a.b-b.b)).sum::<f32>()
        };
        let (near_a,near_b,coincident)=(energy(9.9),energy(10.1),energy(0.0));
        assert!(near_a>0.0 && near_b>0.0 && coincident==0.0,"a near source fades continuously to coincidence");
        assert!((near_a-near_b).abs()/near_a.max(near_b)<0.20,"no former ten-pixel shadow pop");
        let mut f=Frame::new(); f.cam_x=17; f.cam_y=11;
        let own=PointLight{x:160.8,y:137.2,radius:28.0,color:rgb(0x81d1e0),intensity:0.8,height:18.0};
        f.add_light_w(own.x,own.y,own.radius,own.color,own.intensity);
        assert!(same_emitter(f.point_lights[0],own,&f));
        assert!(!same_emitter(f.point_lights[0],PointLight{intensity:0.7,..own},&f));
        assert!(!same_emitter(f.point_lights[0],PointLight{x:own.x+0.01,..own},&f));
    }

    #[test]
    fn moving_sources_cast_native_ground_shadows_without_changing_actors_or_emission() {
        let mut z=Zone::new(ZoneId::Glasswater,24,20);
        z.ground.fill(rgb(0x71818f));
        z.ambient=Rgb::new(0.4,0.45,0.55);
        z.props.push(crate::world::Prop::new("bloomwild_ruins","pillar",120.0,120.0));
        let field=Field::bake(&z,&crate::world::NoRegion,geometry_key(&z));
        let render=|z:&Zone,x:f32,actor:Option<usize>| {
            let mut f=Frame::new(); f.begin(Rgb::BLACK,z.ambient);
            f.albedo.fill(rgb(0x71818f)); f.emissive.fill(rgb(0x06090c));
            f.add_light_w(x,80.0,130.0,rgb(0x81d1e0),0.8);
            if let Some(i)=actor {f.actor_pixels[i]=true;}
            let original=f.light.clone(); let emission=f.emissive.clone(); let art=f.albedo.clone();
            occlude_points(z,&crate::world::NoRegion,&field,&mut f,0,1.0);
            assert_eq!(f.emissive,emission); assert_eq!(f.albedo,art);
            for (a,b) in f.light.iter().zip(&original) {
                assert!(a.r<=b.r && a.g<=b.g && a.b<=b.b,"occlusion cannot invent light");
                assert!(a.r>=z.ambient.r && a.g>=z.ambient.g && a.b>=z.ambient.b);
            }
            (f,original)
        };
        let (left,before)=render(&z,70.0,None);
        let changed:Vec<_>=left.light.iter().zip(&before).enumerate().filter(|(_, (a,b))|a!=b).map(|(i,_)|i).collect();
        assert!(changed.len()>20,"actual native pillar must cast a measurable local shadow");
        let (actor,actor_before)=render(&z,70.0,Some(changed[0]));
        assert_eq!(actor.light[changed[0]],actor_before[changed[0]]);
        let (right,right_before)=render(&z,170.0,None);
        let right_changed:Vec<_>=right.light.iter().zip(&right_before).enumerate().filter(|(_, (a,b))|a!=b).map(|(i,_)|i).collect();
        let mean=|pixels:&[usize]|pixels.iter().map(|i|(i%W) as f32).sum::<f32>()/pixels.len() as f32;
        assert!(!right_changed.is_empty() && mean(&changed)>mean(&right_changed)+5.0,
            "shadow must move to the opposite side when the real source crosses the pillar");
        z.props[0].alive=false;
        let (dead,dead_before)=render(&z,70.0,None);
        assert_eq!(dead.light,dead_before,"broken scenery cannot leave a live-light shadow");
        let mut f=left; f.begin(Rgb::BLACK,z.ambient);
        assert!(f.point_lights.is_empty(),"emitters must not leak into the next frame");
    }
}
