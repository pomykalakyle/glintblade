//! Bounded reconstruction after sprite props change alive state, with unchanged world-space sources.
use super::*;
use crate::world::Prop;

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    /// Cover every pixel owned by the actual zone.
    pub fn zone(z: &Zone) -> Self {
        Self {
            x0: 0,
            y0: 0,
            x1: z.pw,
            y1: z.ph,
        }
    }

    /// Count pixels in this half-open world rectangle.
    pub fn len(self) -> usize {
        ((self.x1 - self.x0).max(0) * (self.y1 - self.y0).max(0)) as usize
    }

    /// Index patch-local storage without changing the world anchor.
    pub fn index(self, x: i32, y: i32) -> Option<usize> {
        (x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1)
            .then(|| ((y - self.y0) * (self.x1 - self.x0) + x - self.x0) as usize)
    }

    /// Include both sets of affected pixels.
    pub fn union(self, b: Self) -> Self {
        Self {
            x0: self.x0.min(b.x0),
            y0: self.y0.min(b.y0),
            x1: self.x1.max(b.x1),
            y1: self.y1.max(b.y1),
        }
    }

    /// Pad for a bounded projection or reflected silhouette.
    pub fn expand(self, n: i32) -> Self {
        Self {
            x0: self.x0 - n,
            y0: self.y0 - n,
            x1: self.x1 + n,
            y1: self.y1 + n,
        }
    }

    /// Limit storage and iteration to actual zone pixels.
    pub fn clamp(self, z: &Zone) -> Self {
        Self {
            x0: self.x0.clamp(0, z.pw),
            y0: self.y0.clamp(0, z.ph),
            x1: self.x1.clamp(0, z.pw),
            y1: self.y1.clamp(0, z.ph),
        }
    }

    /// Detect possible dependencies without relying on the active camera.
    pub fn intersects(self, b: Self) -> bool {
        self.x0 < b.x1 && self.x1 > b.x0 && self.y0 < b.y1 && self.y1 > b.y0
    }
}

pub(super) struct Planes {
    rect: Rect,
    data: Vec<Plane>,
}

impl Planes {
    /// Allocate form scratch proportional to the affected rectangle, never the zone on breakage.
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            data: vec![
                Plane {
                    nz: 1.0,
                    ..Plane::default()
                };
                rect.len()
            ],
        }
    }

    /// Replace a plane only inside the reconstructed patch.
    pub fn set(&mut self, x: i32, y: i32, p: Plane) {
        if let Some(i) = self.rect.index(x, y) {
            self.data[i] = p;
        }
    }

    /// Retrieve a plane for a pixel inside the patch.
    pub fn at(&self, x: i32, y: i32) -> Plane {
        self.data[self.rect.index(x, y).unwrap()]
    }
}

/// Admit ordinary sprite breakables only; source or procedural changes retain full invalidation.
pub(super) fn incremental(p: &Prop) -> bool {
    p.breakable
        && p.light.is_none()
        && p.sheet != "glasswater_blocker"
        && p.sheet().and_then(|s| s.anim(&p.anim)).is_some()
}

/// Bound a sprite, its reflection, attached contact and every possible key-light projection.
pub(super) fn prop_bounds(p: &Prop) -> Rect {
    let (x, y, w, h) = p.bounds().unwrap();
    let height = (p.y - y).max(0.0).ceil() as i32 + 1;
    let reach = (height as f32 * 1.3).ceil() as i32 + 3;
    let (fw, fd) = p.footprint.unwrap_or((0.0, 0.0));
    Rect {
        x0: (x.floor() as i32 - reach).min(p.x.round() as i32 - (fw * 0.5).ceil() as i32 - 3),
        y0: (y.floor() as i32)
            .min(p.y.round() as i32 - (fd * 0.5).ceil().clamp(2.0, 7.0) as i32 - 1),
        x1: (x.ceil() as i32 + w as i32 + reach + 1)
            .max(p.x.round() as i32 + (fw * 0.5).ceil() as i32 + 4),
        y1: (y.ceil() as i32 + h as i32 + 1).max(p.y.round() as i32 + height + reach + 2),
    }
}

/// Cover the fixed procedural scan and its longest possible projection and contact.
pub(super) fn procedural_bounds(p: &Prop) -> Rect {
    let (w, d) = p.footprint.unwrap_or((0.0, 0.0));
    let rx = (w * 0.5).ceil() as i32 + 3;
    let ry = (d * 0.5).ceil().clamp(2.0, 7.0) as i32 + 2;
    let x = p.x.round() as i32;
    let y = p.y.round() as i32;
    let reach = (PROCEDURAL_HEIGHT as f32 * 1.3).ceil() as i32;
    Rect {
        x0: x - (reach + 45).max(rx),
        y0: y - PROCEDURAL_HEIGHT.max(ry),
        x1: x + (reach + 46).max(rx),
        y1: y + ((reach as f32 * 0.65).ceil() as i32 + 4).max(ry),
    }
}

impl Field {
    /// Clip writes while keeping source and procedural reads in the complete world-space field.
    pub(super) fn paint_index(&self, x: i32, y: i32) -> Option<usize> {
        self.clip.index(x, y).and_then(|_| self.index(x, y))
    }

    /// Rebuild affected scenery and overlapping contributions after source-free sprite breakage.
    pub(super) fn refresh_breakage(&mut self, z: &Zone, region: &dyn Region) {
        let mut dirty: Option<Rect> = None;
        for (i, p) in z.props.iter().enumerate() {
            if self.alive[i] != p.alive {
                let rect = prop_bounds(p);
                dirty = Some(dirty.map_or(rect, |r| r.union(rect)));
                self.alive[i] = p.alive;
            }
        }
        let Some(mut rect) = dirty else { return };
        // Procedural recovery reads canonical art above its anchor. Rebuild dependent masses too.
        let changed_art = rect;
        for p in &z.props {
            if p.alive
                && p.sheet().and_then(|s| s.anim(&p.anim)).is_none()
                && !sprite_companion(z, p)
                && let Some((width, _)) = p.footprint
                && width <= 80.0
            {
                let radius = (width * 1.15 + 9.0).min(42.0) as i32;
                let scan = Rect {
                    x0: p.x.round() as i32 - radius,
                    x1: p.x.round() as i32 + radius + 1,
                    y0: p.y.round() as i32 - PROCEDURAL_HEIGHT,
                    y1: p.y.round() as i32,
                };
                if scan.intersects(changed_art) {
                    rect = rect.union(procedural_bounds(p));
                }
            }
        }
        self.clip = rect.clamp(z);
        let (art, _, contact) = scenery(z, region, self.clip);
        for y in self.clip.y0..self.clip.y1 {
            for x in self.clip.x0..self.clip.x1 {
                let i = self.index(x, y).unwrap();
                let j = self.clip.index(x, y).unwrap();
                self.art[i] = art[j];
                self.dark[i] = contact[j];
                self.raised[i] = false;
            }
        }
        self.geometry(z);
        #[cfg(test)]
        {
            self.patched_pixels += self.clip.len();
        }
        self.clip = Rect::zone(z);
    }
}
