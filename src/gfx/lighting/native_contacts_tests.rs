//! Distributor contact must follow real feet instead of spanning its aperture.
use super::*;
use crate::world::{NoRegion, Prop};

fn fixture(prop: Prop) -> Zone {
    let mut zone = Zone::new(ZoneId::Glasswater,24,14);
    zone.ground.fill(rgb(0x4c6070));
    zone.ground_surfaces.fill(TerrainSurface::new([0.0,0.0,1.0],0.0));
    zone.props.push(prop);
    zone
}

#[test]
fn contact_ink_correction_does_not_bake_another_underline_or_remove_physical_casts() {
    for flip in [false,true] {
        let mut prop = Prop::new("round13_landmarks_distributor","body",192.8,180.8);
        prop.flip = flip;
        let s = prop.sheet().unwrap(); let a = s.anim("body").unwrap(); let cell = s.cell(a,0);
        assert_eq!(a.json["contact"].as_str(),Some("none"));
        let zone = fixture(prop.clone());
        let field = Field::bake(&zone,&NoRegion,geometry_key(&zone));
        assert!(field.contact.iter().all(|&v|v==0),"no uniform native/ellipse contact stroke may replace the removed ink");
        let mut f=Frame::new(); f.surface_lighting=true; prop.draw(&mut f,0,1.0);
        assert_eq!(f.surface_casters.len(),1);
        let caster=&f.surface_casters[0];
        assert_eq!(caster.opacity,1.0);
        assert!(std::sync::Arc::ptr_eq(&caster.pixels,&cell.surface));
        assert!(caster.foot.iter().any(|s|s[1]>s[0]),"native physical stone still casts");
        // The recessed arch soles and outer feet retain their true native
        // geometry. Contact presentation never edits the physical alpha.
        for y in 92..100 {for x in 71..89 {assert!(!cell.at(x,y).a);}}
    }
}

#[test]
fn props_without_native_contact_opt_in_keep_the_original_ellipse() {
    let prop=Prop::new("bloomwild_ruins","pillar_broken_tall",192.0,180.0);
    let zone=fixture(prop.clone());
    let field=Field::bake(&zone,&NoRegion,geometry_key(&zone));
    let mut expected=Field::bake(&fixture(Prop::new("missing","none",192.0,180.0)),&NoRegion,0);
    expected.contact.fill(0); expected.dark.fill(0);
    let [x0,y0,x1,y1]=prop.collision_bounds().unwrap();
    expected.contact(((x0+x1)*0.5).round() as i32,y1.round() as i32,x1-x0,y1-y0);
    for (i,&raised) in field.raised.iter().enumerate(){if raised{expected.contact[i]=0;}}
    assert_eq!(field.contact,expected.contact);
}
