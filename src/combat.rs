//! Hits: every attack (a sword arc, a lunge, a projectile, a shockwave) is a `Hit` with a shape.
//! Each tick the scene collects active hits and tests them against the other team's bodies.
//! A target is damaged at most once per hit `id`, so a swing that lasts several ticks hits once.

use crate::util::angle_diff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle { x: f32, y: f32, r: f32 },
    /// A ring sector centered at (x, y): radii r0..r1, angles a0..a1 (radians, a0 < a1, +y down).
    Arc { x: f32, y: f32, r0: f32, r1: f32, a0: f32, a1: f32 },
    /// A thick line segment (beams, lunges).
    Capsule { x0: f32, y0: f32, x1: f32, y1: f32, r: f32 },
}

impl Shape {
    /// Does this shape touch a circle (cx, cy, cr)?
    pub fn touches(&self, cx: f32, cy: f32, cr: f32) -> bool {
        match *self {
            Shape::Circle { x, y, r } => (cx - x).powi(2) + (cy - y).powi(2) <= (r + cr).powi(2),
            Shape::Arc { x, y, r0, r1, a0, a1 } => {
                let dx = cx - x;
                let dy = cy - y;
                let d = (dx * dx + dy * dy).sqrt();
                if d > r1 + cr || d + cr < r0 {
                    return false;
                }
                if d < cr {
                    return true;
                }
                let a = dy.atan2(dx);
                let mid = (a0 + a1) * 0.5;
                let half = (a1 - a0) * 0.5;
                // angular slack from the target's radius
                let slack = (cr / d.max(1.0)).atan();
                angle_diff(mid, a).abs() <= half + slack
            }
            Shape::Capsule { x0, y0, x1, y1, r } => {
                let (vx, vy) = (x1 - x0, y1 - y0);
                let l2 = vx * vx + vy * vy;
                let t = if l2 > 0.0 { (((cx - x0) * vx + (cy - y0) * vy) / l2).clamp(0.0, 1.0) } else { 0.0 };
                let (px, py) = (x0 + vx * t, y0 + vy * t);
                (cx - px).powi(2) + (cy - py).powi(2) <= (r + cr).powi(2)
            }
        }
    }

    /// A representative center point (for knockback direction).
    pub fn center(&self) -> (f32, f32) {
        match *self {
            Shape::Circle { x, y, .. } => (x, y),
            Shape::Arc { x, y, .. } => (x, y),
            Shape::Capsule { x0, y0, x1, y1, .. } => ((x0 + x1) * 0.5, (y0 + y1) * 0.5),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub shape: Shape,
    pub team: Team,
    pub dmg: i32,
    /// Knockback speed in px/tick.
    pub knock: f32,
    /// Knockback direction; (0, 0) = away from the shape's center.
    pub dir: (f32, f32),
    /// Unique per swing/projectile (`Fx::hit_id`).
    pub id: u32,
    /// Heavy hits stun longer and shake more.
    pub heavy: bool,
}

impl Hit {
    pub fn new(shape: Shape, team: Team, dmg: i32, id: u32) -> Hit {
        Hit { shape, team, dmg, knock: 2.0, dir: (0.0, 0.0), id, heavy: false }
    }
    pub fn knock(mut self, k: f32) -> Hit {
        self.knock = k;
        self
    }
    pub fn dir(mut self, dx: f32, dy: f32) -> Hit {
        self.dir = (dx, dy);
        self
    }
    pub fn heavy(mut self, h: bool) -> Hit {
        self.heavy = h;
        self
    }

    /// Knockback direction for a target at (x, y).
    pub fn push_dir(&self, x: f32, y: f32) -> (f32, f32) {
        if self.dir != (0.0, 0.0) {
            return crate::util::norm(self.dir.0, self.dir.1);
        }
        let (cx, cy) = self.shape.center();
        let d = crate::util::norm(x - cx, y - cy);
        if d == (0.0, 0.0) { (0.0, 1.0) } else { d }
    }
}
