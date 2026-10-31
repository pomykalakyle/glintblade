//! Draws a zone and everything in it into a Frame, in the right order:
//! backdrop -> ground -> region ground fx -> debris -> y-sorted (props, region items, characters)
//! -> canopies -> airborne particles -> region overlay -> lights -> compose (grade).
//! The caller draws UI afterwards and then calls `post::finish`.

use crate::fx::Fx;
use crate::gfx::{Frame, post};
use crate::world::{Region, Zone};

#[derive(Clone, Copy)]
enum Item {
    Prop(usize),
    Region(u32),
    Ext(u32),
}

/// `ext` are the caller's y-sorted things (characters, projectiles): (sort_y, id); `draw_ext`
/// draws one by id. `focus` is the player position (canopies above it turn see-through).
#[allow(clippy::too_many_arguments)]
pub fn render_world(
    zone: &Zone,
    region: &dyn Region,
    fx: &Fx,
    f: &mut Frame,
    t: u64,
    ext: &[(f32, u32)],
    draw_ext: &mut dyn FnMut(&mut Frame, u32),
    focus: Option<(f32, f32)>,
) {
    f.t = t;
    f.begin(zone.bg, zone.ambient);
    region.draw_backdrop(zone, f, t);
    zone.draw_ground(f);
    region.draw_ground(zone, f, t);
    fx.particles.draw_debris(f);

    let mut items: Vec<(f32, Item)> = Vec::with_capacity(256);
    for (i, p) in zone.props.iter().enumerate() {
        if !p.alive || p.canopy {
            continue;
        }
        if let Some((x, y, w, h)) = p.bounds()
            && !f.visible(x, y, w, h, 8.0)
        {
            continue;
        }
        items.push((p.y, Item::Prop(i)));
    }
    let mut ritems = Vec::new();
    region.sorted_items(zone, f, &mut ritems);
    items.extend(ritems.into_iter().map(|(y, id)| (y, Item::Region(id))));
    items.extend(ext.iter().map(|&(y, id)| (y, Item::Ext(id))));
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, it) in items {
        match it {
            Item::Prop(i) => zone.props[i].draw(f, t, 1.0),
            Item::Region(id) => region.draw_item(zone, f, id, t),
            Item::Ext(id) => draw_ext(f, id),
        }
    }

    // Canopies: see-through when the focus point is underneath.
    for p in zone.props.iter().filter(|p| p.canopy && p.alive) {
        let Some((x, y, w, h)) = p.bounds() else { continue };
        if !f.visible(x, y, w, h, 8.0) {
            continue;
        }
        let behind = focus.map(|(fx_, fy)| fx_ > x + 2.0 && fx_ < x + w - 2.0 && fy - 8.0 > y && fy - 8.0 < y + h).unwrap_or(false);
        p.draw(f, t, if behind { 0.35 } else { 1.0 });
    }

    fx.particles.draw(f);
    region.draw_overlay(zone, f, t);

    for l in &zone.lights {
        f.add_light_w(l.x, l.y, l.radius, l.color, l.intensity);
    }
    for p in zone.props.iter().filter(|p| p.alive) {
        if let Some((c, r, i, ox, oy)) = p.light {
            f.add_light_w(p.x + ox, p.y + oy, r, c, i);
        }
    }
    for l in &fx.lights {
        f.add_light_w(l.x, l.y, l.radius, l.color, l.intensity);
    }
    region.light(zone, f, t);
    crate::gfx::lighting_hook::apply(zone, region, f, t);
    post::compose(f, &zone.grade);
}
