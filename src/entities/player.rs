//! Glint, the player: movement, the 3-hit combo, dash, hold-to-charge spin slash, hurt, death and
//! respawn. The body comes from the `hero` sprite sheet (with a code-drawn fallback while it's
//! being drawn). Everything else is drawn here in code, pixel by pixel, with hard edges and the
//! hero / FX palette ramps: the trailing scarf (a small 3D verlet chain), the slash smears
//! (designed crescents), dash afterimages, the charge ring and glints.
//!
//! Timing is in 60 Hz ticks. Every swing has an anticipation (windup), a short active window
//! with a lunge, and a recovery that can be cancelled: into the next combo hit, a dash, a charge,
//! or (late in the recovery) plain movement. Presses are buffered so early inputs and mashing work.
//!
//! The player's lights are added in `draw` (not through `Fx`), so they stay on during hit-stop,
//! when the scene skips `update`.

use super::Body;
use crate::combat::{Hit, Shape, Team};
use crate::fx::{Fx, Particle};
use crate::gfx::{Anim, Cell, Dir, DrawOpts, Frame, Rgb, rgb, sheet};
use crate::input::Input;
use crate::util::norm;
use crate::world::{Tile, Zone, ZoneId};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

pub const MAX_HP: i32 = 6;

// ------------------------------------------------------------------------------------ tuning

const RUN_SPEED: f32 = 1.5;
/// Fraction of the gap to the target velocity closed per tick when speeding up / slowing down.
const ACCEL: f32 = 0.34;
const DECEL: f32 = 0.5;
const WATER_SLOW: f32 = 0.74;

const ATTACK_BUFFER: u32 = 12;
const DASH_BUFFER: u32 = 10;
/// After a swing fully recovers, a press within this many ticks continues the combo.
const COMBO_WINDOW: u32 = 16;
/// After the third hit the chain resets; a short beat before the next combo can start.
const COMBO_REST: u32 = 5;

/// Dash speed per tick (px): a hard burst that eases out. Sum is the dash distance (~40 px).
const DASH_SPEED: [f32; 11] = [5.2, 5.0, 4.8, 4.5, 4.2, 3.8, 3.4, 2.9, 2.3, 1.7, 1.1];
const DASH_TICKS: u32 = DASH_SPEED.len() as u32;
/// I-frames last the whole dash plus this many ticks (generous on purpose).
const DASH_IFRAMES_EXTRA: u32 = 6;
const DASH_COOLDOWN: u32 = 8;

const CHARGE_START: u32 = 16;
const CHARGE_FULL: u32 = 50;
const CHARGE_SPEED: f32 = 0.55;

const SPIN_WINDUP: u32 = 3;
const SPIN_ACTIVE: u32 = 16;
const SPIN_TOTAL: u32 = 30;
const SPIN_RADIUS: f32 = 30.0;

const HURT_TICKS: u32 = 16;
/// Hurt can be dashed out of after this many ticks.
const HURT_DASH_AT: u32 = 7;
const HURT_IFRAMES: u32 = 80;
const RESPAWN_HEIGHT: f32 = 70.0;
const RESPAWN_IFRAMES: u32 = 110;
/// Ticks between the body bursting into pixels and the respawn.
const DEATH_LINGER: u32 = 64;

const GHOST_LIFE: u32 = 13;
const DASH_GHOST_LIFE: u32 = 8;
const GHOST_SPACING: f32 = 16.0;

// ------------------------------------------------------------------------------------ palette

// The Long Quiet ramps in assets/concept/palettes_v2.json; cyan stays with Glint.
pub const BLADE_CORE: Rgb = rgb(0xbaf5ec);
pub const BLADE_MID: Rgb = rgb(0x63d7e0);
pub const BLADE_EDGE: Rgb = rgb(0x429aa9);
const WHITE: Rgb = rgb(0xedf9ef);
const FX_BLUE: Rgb = rgb(0x429aa9);
const FX_DEEP: Rgb = rgb(0x416974);
const OUTLINE: Rgb = rgb(0x262b40);
// Keep four shading slots for the ribbon's existing twist; v2 has three cloth swatches.
const SCARF: [Rgb; 4] = [rgb(0x416974), rgb(0x416974), rgb(0x79a6ae), rgb(0xb4d1ce)];
const DANGER: [Rgb; 3] = [rgb(0x724b59), rgb(0xbd7a85), rgb(0xe6b4ac)];
const HEAL: Rgb = rgb(0xd7f3eb);

/// One combo swing's timing, movement, hitbox and smear.
struct Swing {
    /// Anticipation ticks before the strike.
    windup: u32,
    /// Ticks the hitbox is out (starting at `windup`).
    active: u32,
    /// Total length; the recovery is what's left after the active window.
    total: u32,
    /// From this tick a buffered press chains into the next hit.
    chain_at: u32,
    /// From this tick, holding a direction cancels the recovery into running.
    free_at: u32,
    /// Forward speed at the strike (px/tick), decays quickly.
    lunge: f32,
    reach: f32,
    /// Hitbox half-angle (radians).
    half: f32,
    dmg: i32,
    knock: f32,
    heavy: bool,
    /// Smear outer radius, max thickness and swept angle.
    r: f32,
    width: f32,
    sweep: f32,
    /// +1 swings the same way as the facing's forehand, -1 is the backhand.
    hand: f32,
}

const SWINGS: [Swing; 3] = [
    Swing { windup: 3, active: 4, total: 21, chain_at: 8, free_at: 13, lunge: 2.4, reach: 24.0, half: 1.25, dmg: 1, knock: 2.6, heavy: false, r: 21.0, width: 6.0, sweep: 2.75, hand: 1.0 },
    Swing { windup: 2, active: 4, total: 21, chain_at: 7, free_at: 12, lunge: 2.4, reach: 24.0, half: 1.25, dmg: 1, knock: 2.6, heavy: false, r: 21.0, width: 6.0, sweep: 2.75, hand: -1.0 },
    Swing { windup: 8, active: 5, total: 32, chain_at: 99, free_at: 22, lunge: 3.8, reach: 29.0, half: 1.8, dmg: 2, knock: 4.6, heavy: true, r: 26.0, width: 8.0, sweep: 4.0, hand: 1.0 },
];

/// Smear keyframes from the strike on: (tail, head, thickness, age). `tail`/`head` are positions
/// along the swing (0 = start, 1 = end); age 0 is fresh (white rim), 3 is the broken last wisp.
const SMEAR_KEYS: [(f32, f32, f32, u8); 7] = [
    (0.0, 0.72, 0.75, 0),
    (0.0, 1.0, 1.0, 0),
    (0.16, 1.0, 0.9, 1),
    (0.38, 1.0, 0.7, 1),
    (0.58, 1.0, 0.48, 2),
    (0.72, 1.0, 0.32, 2),
    (0.82, 1.0, 0.22, 3),
];

/// Visual-only widening of the combo smear body relative to `Swing::width`, so the strike
/// reads as one broad mass whose inner edge reaches back toward the blade.
const SMEAR_BODY: f32 = 1.45;
const SMEAR_SQUASH: f32 = 0.74;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Normal,
    /// Combo stage 1..=3, ticks into the swing.
    Attack { stage: u8, t: u32 },
    Dash { t: u32 },
    /// Holding attack after the charge threshold.
    Charge { t: u32 },
    Spin { t: u32 },
    Hurt { t: u32 },
    Dead { t: u32 },
    /// Dropping back in from the sky after a fall.
    Respawn { t: u32 },
}

pub struct PlayerCtx<'a> {
    pub zone: &'a Zone,
    pub fx: &'a mut Fx,
    pub hits: &'a mut Vec<Hit>,
}

/// One point of the scarf chain, in world space with height `z` (drawn at y - z).
#[derive(Clone, Copy, Debug)]
struct Knot {
    x: f32,
    y: f32,
    z: f32,
    px: f32,
    py: f32,
    pz: f32,
}

/// A dash afterimage: a frozen copy of the sprite cell.
struct Ghost {
    x: f32,
    y: f32,
    cell: &'static Cell,
    anchor: (i32, i32),
    flip: bool,
    age: u32,
    dash: bool,
}

/// The sprite frame being shown.
struct Pose {
    cell: &'static Cell,
    anchor: (i32, i32),
    flip: bool,
    anim: Option<&'static Anim>,
    frame: usize,
}

const SCARF_KNOTS: usize = 8;
const SCARF_SEG: f32 = 1.45;

pub struct Player {
    pub body: Body,
    pub state: State,
    /// Four-way facing for the sprite (the attack aim in `body.face` is eight-way).
    pub dir: Dir,
    /// Ticks in the current animation.
    pub anim_t: u32,
    pub dash_cd: u32,
    pub dash_dir: (f32, f32),
    /// Set when the death sequence finishes: the scene respawns the player.
    pub wants_respawn: bool,
    t: u32,
    attack_buf: u32,
    dash_buf: u32,
    /// Ticks since the last swing ended (for continuing the chain from Normal).
    combo_gap: u32,
    last_stage: u8,
    swing_id: u32,
    /// Aim angle of the current swing and which way it sweeps (+1 clockwise on screen).
    swing_angle: f32,
    swing_hand: f32,
    /// The current swing hit something (from hit-stop detection).
    connected: bool,
    spin_dir: f32,
    ghosts: Vec<Ghost>,
    scarf: Vec<Knot>,
    /// Scarf came loose (death): it falls and settles instead of following the neck.
    scarf_free: bool,
    moving: bool,
    /// Ticks since the run started (footstep timing).
    step_frame: usize,
    /// fx.t at our last update; a jump means the game was frozen in between (hit-stop).
    last_fx_t: u64,
    swung_last: bool,
    hurt_last: bool,
    /// Post-hurt invulnerability blinks; dash i-frames don't.
    blink: bool,
    /// The body burst into pixels (death); nothing but the fallen scarf is drawn.
    burst: bool,
    heal_t: u32,
    /// Ticks since the charge became full (glint and pulse timing), 0 = not full.
    full_t: u32,
    /// Ticks since the respawn landing (dust ring), 0 = not landed yet.
    land_t: u32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Player {
        let mut p = Player {
            body: Body::new(x, y, 4.0, MAX_HP),
            state: State::Normal,
            dir: Dir::Down,
            anim_t: 0,
            dash_cd: 0,
            dash_dir: (0.0, 1.0),
            wants_respawn: false,
            t: 0,
            attack_buf: 0,
            dash_buf: 0,
            combo_gap: 99,
            last_stage: 0,
            swing_id: 0,
            swing_angle: PI * 0.5,
            swing_hand: 1.0,
            connected: false,
            spin_dir: 1.0,
            ghosts: Vec::new(),
            scarf: Vec::new(),
            scarf_free: false,
            moving: false,
            step_frame: 0,
            last_fx_t: 0,
            swung_last: false,
            hurt_last: false,
            blink: false,
            burst: false,
            heal_t: 0,
            full_t: 0,
            land_t: 0,
        };
        p.reset_scarf();
        p
    }

    pub fn alive(&self) -> bool {
        !matches!(self.state, State::Dead { .. })
    }

    /// Back at the checkpoint after a fall: Glint drops in from the sky.
    pub fn respawn(&mut self, x: f32, y: f32) {
        let hp = self.body.max_hp;
        self.body = Body::new(x, y, 4.0, hp);
        self.body.face = (0.0, 1.0);
        self.body.z = RESPAWN_HEIGHT;
        self.body.vz = -1.0;
        self.body.iframes = RESPAWN_IFRAMES;
        self.dir = Dir::Down;
        self.set_state(State::Respawn { t: 0 });
        self.wants_respawn = false;
        self.burst = false;
        self.blink = false;
        self.scarf_free = false;
        self.ghosts.clear();
        self.land_t = 0;
        self.full_t = 0;
        self.reset_scarf();
    }

    pub fn heal(&mut self, n: i32) {
        self.body.hp = (self.body.hp + n).min(self.body.max_hp);
        self.heal_t = 24;
    }

    /// Preserve button edges while the scene is in hit-stop, before its early return.
    pub fn buffer_input(&mut self, input: &Input) {
        if input.attack.pressed {
            self.attack_buf = ATTACK_BUFFER;
        }
        if input.dash.pressed {
            self.dash_buf = DASH_BUFFER;
        }
    }

    fn set_state(&mut self, s: State) {
        self.state = s;
        self.anim_t = 0;
    }

    /// Face along a vector: eight-way aim, four-way sprite with hysteresis on diagonals, so
    /// running diagonally doesn't flicker between sprites.
    fn face_to(&mut self, x: f32, y: f32) {
        if x == 0.0 && y == 0.0 {
            return;
        }
        self.body.face = norm(x, y);
        self.dir = pick_dir(self.dir, x, y);
    }

    // ---------------------------------------------------------------------------- update

    pub fn update(&mut self, input: &Input, ctx: &mut PlayerCtx) {
        let zone = ctx.zone;
        let fx = &mut *ctx.fx;

        // Did our swing connect? A landed hit freezes the game (hit-stop), so a jump in fx.t
        // right after a tick in which we swung, without us getting hurt, means it landed.
        let elapsed = fx.t.saturating_sub(self.last_fx_t).max(1);
        let froze = self.t > 0 && elapsed > 1;
        if froze && self.swung_last && !self.hurt_last {
            self.on_connect(fx);
        }
        self.last_fx_t = fx.t;
        self.swung_last = false;
        self.hurt_last = false;

        self.t += 1;
        self.anim_t += 1;
        self.dash_cd = self.dash_cd.saturating_sub(1);
        self.attack_buf = self.attack_buf.saturating_sub(1);
        self.dash_buf = self.dash_buf.saturating_sub(1);
        self.combo_gap = self.combo_gap.saturating_add(1);
        self.heal_t = self.heal_t.saturating_sub(1);
        if self.land_t > 0 {
            self.land_t += 1;
        }
        self.buffer_input(input);
        // Also recover held edges when a scene doesn't call buffer_input during hit-stop.
        if froze && input.attack.down && input.attack.held as u64 <= elapsed {
            self.attack_buf = ATTACK_BUFFER;
        }
        if froze && input.dash.down && input.dash.held as u64 <= elapsed {
            self.dash_buf = DASH_BUFFER;
        }
        let mv = input.mv;
        let has_mv = mv != (0.0, 0.0);
        let was_moving = self.moving;
        self.moving = false;
        // Movement target this tick and how quickly velocity approaches it.
        let mut target = (0.0, 0.0);
        let mut rate = DECEL;
        if !matches!(self.state, State::Charge { .. }) {
            self.full_t = 0;
        }

        match self.state {
            State::Dead { t } => {
                self.update_dead(t, zone, fx);
                return;
            }
            State::Respawn { t } => {
                self.state = State::Respawn { t: t + 1 };
                if t % 2 == 0 && self.body.z > 4.0 {
                    self.add_ghost();
                }
                if self.body.z <= 0.0 && self.land_t == 0 && t > 0 {
                    self.land_t = 1;
                    fx.shake(0.3);
                    fx.sound_vol("dash_2", 0.7);
                    fx.sound_var(step_sound(zone, self.body.x, self.body.y), 3);
                    let c = dust_color(zone, self.body.x, self.body.y);
                    for i in 0..14 {
                        let a = i as f32 / 14.0 * TAU + fx.rng.signed() * 0.2;
                        let s = fx.rng.range(0.9, 1.6);
                        fx.particles.add(Particle::new(self.body.x + a.cos() * 3.0, self.body.y + a.sin() * 1.5).z(1.0).vel(a.cos() * s, a.sin() * s * 0.5, fx.rng.range(0.2, 0.6)).color(c).life(fx.rng.range(16.0, 26.0)).grav(0.04).drag(0.86));
                    }
                }
                if self.land_t >= 10 {
                    self.set_state(State::Normal);
                }
            }
            State::Hurt { t } => {
                if t >= HURT_DASH_AT && self.try_dash(mv, zone, fx) {
                } else if t + 1 >= HURT_TICKS {
                    self.set_state(State::Normal);
                } else {
                    self.state = State::Hurt { t: t + 1 };
                }
            }
            State::Dash { t } => {
                // a touch of steering, mostly committed
                if has_mv {
                    let d = norm(mv.0, mv.1);
                    self.dash_dir = norm(self.dash_dir.0 + d.0 * 0.07, self.dash_dir.1 + d.1 * 0.07);
                }
                let s = DASH_SPEED[(t as usize).min(DASH_SPEED.len() - 1)];
                target = (self.dash_dir.0 * s, self.dash_dir.1 * s);
                rate = 1.0;
                // Sample travel distance, not time: the easing tail must not pile up echoes.
                let spaced = self.ghosts.last().is_none_or(|g| (self.body.x - g.x).hypot(self.body.y - self.body.z - g.y) >= GHOST_SPACING);
                if spaced && t < DASH_TICKS - 3 {
                    self.add_ghost();
                }
                if t % 3 == 1 && t < DASH_TICKS - 3 {
                    let c = dust_color(zone, self.body.x, self.body.y);
                    fx.particles.add(Particle::new(self.body.x - self.dash_dir.0 * 3.0 + fx.rng.signed(), self.body.y + fx.rng.signed() * 0.5).z(0.5).vel(-self.dash_dir.0 * 0.4, -self.dash_dir.1 * 0.2, 0.25).color(c).life(fx.rng.range(10.0, 16.0)).grav(0.03).drag(0.85));
                }
                if t + 1 >= DASH_TICKS {
                    self.dash_cd = DASH_COOLDOWN;
                    let c = dust_color(zone, self.body.x, self.body.y);
                    fx.particles.dust(&mut fx.rng, self.body.x + self.dash_dir.0 * 2.0, self.body.y, 3, c);
                    if self.attack_buf > 0 {
                        // dash into a slash: the strike carries the dash's momentum
                        self.attack_buf = 0;
                        self.start_attack(1, mv, fx);
                    } else {
                        self.set_state(State::Normal);
                    }
                } else {
                    self.state = State::Dash { t: t + 1 };
                }
            }
            State::Attack { stage, t } => {
                let sw = &SWINGS[(stage - 1) as usize];
                let strike = sw.windup;
                // Re-aim during the anticipation, so a press just before turning still goes
                // where the player is pointing.
                if t < strike && has_mv {
                    self.face_to(mv.0, mv.1);
                    self.swing_angle = self.body.face.1.atan2(self.body.face.0);
                    self.swing_hand = sw.hand * hand_for(self.dir);
                }
                let (fxd, fyd) = self.body.face;
                if t < strike {
                    // anticipation: settle, the big third hit draws back a little
                    let back = if stage == 3 { -0.35 } else { 0.0 };
                    target = (fxd * back, fyd * back);
                    rate = 0.4;
                    if stage == 3 && t + 1 == strike {
                        fx.sound_vol("charge_ready", 0.25);
                    }
                } else if t == strike {
                    // the strike: lunge, swoosh, hitbox
                    self.body.vx = fxd * sw.lunge + self.body.vx * 0.3;
                    self.body.vy = fyd * sw.lunge + self.body.vy * 0.3;
                    target = (self.body.vx, self.body.vy);
                    rate = 0.0;
                    fx.sound_vol(["slash_1", "slash_2", "slash_3"][(stage - 1) as usize], if stage == 3 { 1.0 } else { 0.85 });
                    if stage == 3 {
                        let c = dust_color(zone, self.body.x, self.body.y);
                        fx.particles.dust(&mut fx.rng, self.body.x - fxd * 3.0, self.body.y, 4, c);
                    }
                } else {
                    // friction after the lunge; a little drift if the stick is held
                    let drift = if has_mv { 0.25 } else { 0.0 };
                    target = (mv.0 * drift, mv.1 * drift);
                    rate = if self.connected { 0.35 } else { 0.22 };
                }
                if t >= strike && t < strike + sw.active {
                    let (cx, cy) = (self.body.x, self.body.y - 5.0);
                    let a = self.swing_angle;
                    let hit = Hit::new(Shape::Arc { x: cx, y: cy, r0: 0.0, r1: sw.reach, a0: a - sw.half, a1: a + sw.half }, Team::Player, sw.dmg, self.swing_id)
                        .knock(sw.knock)
                        .dir(fxd, fyd)
                        .heavy(sw.heavy);
                    ctx.hits.push(hit);
                    self.swung_last = true;
                }
                let chain_at = if self.connected { sw.chain_at.saturating_sub(2) } else { sw.chain_at };
                let committed = t >= strike && t < strike + 2;
                if !committed && self.try_dash(mv, zone, fx) {
                    self.connected = false;
                } else if t >= chain_at && self.attack_buf > 0 && stage < 3 {
                    self.attack_buf = 0;
                    self.start_attack(stage + 1, mv, fx);
                } else if t >= chain_at && input.attack.down && input.attack.held >= CHARGE_START {
                    self.combo_gap = 0;
                    self.set_state(State::Charge { t: CHARGE_START });
                    fx.sound_vol("charge", 0.7);
                } else if t >= sw.free_at && has_mv && !input.attack.down {
                    self.end_swing(stage);
                    self.set_state(State::Normal);
                } else if t + 1 >= sw.total {
                    self.end_swing(stage);
                    self.set_state(State::Normal);
                } else {
                    self.state = State::Attack { stage, t: t + 1 };
                }
            }
            State::Charge { t } => {
                // Strafe: keep the aim, move slowly.
                target = (mv.0 * CHARGE_SPEED, mv.1 * CHARGE_SPEED);
                rate = ACCEL;
                if has_mv {
                    self.moving = true;
                }
                if t == CHARGE_FULL {
                    self.full_t = 1;
                    fx.sound("charge_ready");
                    let (bx, by, bz) = self.blade_tip();
                    fx.particles.sparks(&mut fx.rng, bx, by, bz, 0.0, -1.0, 6, BLADE_MID, 1.4);
                } else if self.full_t > 0 {
                    self.full_t += 1;
                    if self.full_t % 60 == 0 {
                        fx.sound_vol("charge_loop", 0.3);
                    }
                }
                if t < CHARGE_FULL && t % 4 == 0 {
                    // motes drawn in toward the blade
                    let (bx, by, bz) = self.blade_tip();
                    let a = fx.rng.f32() * TAU;
                    let r = fx.rng.range(14.0, 20.0);
                    let (px, py) = (bx + a.cos() * r, by + a.sin() * r * 0.5);
                    let life = 14.0;
                    fx.particles.add(
                        Particle::new(px, py)
                            .z(bz + a.sin() * 4.0)
                            .vel((bx - px) / life * 1.6, (by - py) / life * 1.6, -a.sin() * 4.0 / life)
                            .color(BLADE_CORE)
                            .fade_to(FX_BLUE)
                            .glow(1.0)
                            .grav(0.0)
                            .drag(0.94)
                            .life(life),
                    );
                }
                if !self.try_dash(mv, zone, fx) {
                    if !input.attack.down {
                        if t >= CHARGE_FULL {
                            self.start_spin(fx);
                        } else {
                            self.set_state(State::Normal);
                        }
                    } else {
                        self.state = State::Charge { t: t + 1 };
                    }
                }
            }
            State::Spin { t } => {
                if t < SPIN_WINDUP {
                    rate = 0.5;
                } else {
                    // a short glide along the aim while spinning
                    let k = if t < SPIN_WINDUP + SPIN_ACTIVE { 0.7 } else { 0.0 };
                    target = (self.body.face.0 * k, self.body.face.1 * k);
                    rate = 0.25;
                }
                if t == SPIN_WINDUP {
                    fx.sound("spin");
                    fx.shake(0.32);
                    let c = dust_color(zone, self.body.x, self.body.y);
                    for i in 0..16 {
                        let a = i as f32 / 16.0 * TAU;
                        let s = fx.rng.range(1.2, 2.0);
                        fx.particles.add(Particle::new(self.body.x + a.cos() * 6.0, self.body.y + a.sin() * 3.0).z(0.5).vel(a.cos() * s, a.sin() * s * 0.5, fx.rng.range(0.3, 0.9)).color(c).life(fx.rng.range(14.0, 24.0)).grav(0.06).drag(0.88));
                    }
                }
                if (SPIN_WINDUP..SPIN_WINDUP + SPIN_ACTIVE).contains(&t) {
                    let hit = Hit::new(Shape::Circle { x: self.body.x, y: self.body.y - 5.0, r: SPIN_RADIUS }, Team::Player, 3, self.swing_id).knock(5.0).heavy(true);
                    ctx.hits.push(hit);
                    self.swung_last = true;
                }
                if t >= SPIN_WINDUP + SPIN_ACTIVE + 4 && self.try_dash(mv, zone, fx) {
                } else if t + 1 >= SPIN_TOTAL || (t >= SPIN_TOTAL - 8 && has_mv) {
                    self.combo_gap = 99;
                    self.last_stage = 0;
                    self.set_state(State::Normal);
                } else {
                    self.state = State::Spin { t: t + 1 };
                }
            }
            State::Normal => {
                if has_mv {
                    target = (mv.0 * RUN_SPEED, mv.1 * RUN_SPEED);
                    // Turning hard kicks up a little dust.
                    let (vx, vy) = (self.body.vx, self.body.vy);
                    let sp = (vx * vx + vy * vy).sqrt();
                    if sp > 1.0 && (vx * mv.0 + vy * mv.1) / sp < -0.3 && self.t % 4 == 0 {
                        let c = dust_color(zone, self.body.x, self.body.y);
                        fx.particles.dust(&mut fx.rng, self.body.x, self.body.y, 2, c);
                    }
                    self.face_to(mv.0, mv.1);
                    self.moving = true;
                }
                rate = if has_mv { ACCEL } else { DECEL };
                if self.try_dash(mv, zone, fx) {
                } else if input.attack.down && input.attack.held >= CHARGE_START {
                    self.set_state(State::Charge { t: CHARGE_START });
                    fx.sound_vol("charge", 0.7);
                } else if self.attack_buf > 0 && self.combo_gap >= COMBO_REST {
                    self.attack_buf = 0;
                    let stage = if self.combo_gap <= COMBO_WINDOW && self.last_stage > 0 && self.last_stage < 3 { self.last_stage + 1 } else { 1 };
                    self.start_attack(stage, mv, fx);
                }
            }
        }

        // A movement cancel takes the new aim on this tick. A one-tick turn followed by
        // attack must not swing in the previous direction during recovery.
        if matches!(self.state, State::Normal) && has_mv {
            self.face_to(mv.0, mv.1);
            self.moving = true;
            target = (mv.0 * RUN_SPEED, mv.1 * RUN_SPEED);
            rate = ACCEL;
        }
        if matches!(self.state, State::Normal) && self.moving != was_moving {
            self.anim_t = 0;
            self.step_frame = 0;
        }

        // Start the burst on the input tick, including a dash cancel out of hurt or a swing.
        // Advancing to tick 1 here avoids replaying the kick-off speed on the following tick.
        if matches!(self.state, State::Dash { t: 0 }) {
            target = (self.dash_dir.0 * DASH_SPEED[0], self.dash_dir.1 * DASH_SPEED[0]);
            rate = 1.0;
            self.state = State::Dash { t: 1 };
        }

        // Velocity approaches the target (the dash and lunges set it directly).
        self.body.vx += (target.0 - self.body.vx) * rate;
        self.body.vy += (target.1 - self.body.vy) * rate;
        let in_water = zone.tile_at(self.body.x, self.body.y) == Tile::Shallow;
        let (vx, vy) = (self.body.vx, self.body.vy);
        if in_water && !matches!(self.state, State::Dash { .. }) {
            self.body.vx *= WATER_SLOW;
            self.body.vy *= WATER_SLOW;
        }
        self.body.integrate(zone);
        // keep our intended velocity (the water slow is a per-tick cap, not momentum loss)
        self.body.vx = vx;
        self.body.vy = vy;

        self.footsteps(zone, fx);
        self.update_scarf();
        for g in self.ghosts.iter_mut() {
            g.age += 1;
        }
        self.ghosts.retain(|g| g.age < if g.dash { DASH_GHOST_LIFE } else { GHOST_LIFE });
        if self.heal_t > 0 && self.heal_t % 4 == 0 {
            let a = fx.rng.f32() * TAU;
            fx.particles.add(Particle::new(self.body.x + a.cos() * 5.0, self.body.y + a.sin() * 2.0).z(fx.rng.range(2.0, 12.0)).vel(0.0, 0.0, 0.35).color(WHITE).fade_to(HEAL).glow(1.0).grav(0.0).life(18.0));
        }
    }

    fn start_attack(&mut self, stage: u8, mv: (f32, f32), fx: &mut Fx) {
        if mv != (0.0, 0.0) {
            self.face_to(mv.0, mv.1);
        }
        let sw = &SWINGS[(stage - 1) as usize];
        self.set_state(State::Attack { stage, t: 0 });
        self.swing_id = fx.hit_id();
        self.swing_angle = self.body.face.1.atan2(self.body.face.0);
        self.swing_hand = sw.hand * hand_for(self.dir);
        self.last_stage = stage;
        self.connected = false;
    }

    fn end_swing(&mut self, stage: u8) {
        if stage >= 3 {
            self.last_stage = 0;
            self.combo_gap = 0;
        } else {
            self.combo_gap = 0;
        }
        self.connected = false;
    }

    fn start_spin(&mut self, fx: &mut Fx) {
        self.set_state(State::Spin { t: 0 });
        self.swing_id = fx.hit_id();
        self.swing_angle = self.body.face.1.atan2(self.body.face.0);
        // The directionless spin is authored clockwise for every starting facing.
        self.spin_dir = 1.0;
        self.full_t = 0;
        self.body.iframes = self.body.iframes.max(SPIN_WINDUP + SPIN_ACTIVE + 3);
    }

    /// Dash if one is buffered and off cooldown. Returns true if the dash started.
    fn try_dash(&mut self, mv: (f32, f32), zone: &Zone, fx: &mut Fx) -> bool {
        if self.dash_buf == 0 || self.dash_cd > 0 {
            return false;
        }
        self.dash_buf = 0;
        let d = if mv != (0.0, 0.0) { norm(mv.0, mv.1) } else { self.body.face };
        self.dash_dir = d;
        self.face_to(d.0, d.1);
        self.body.iframes = self.body.iframes.max(DASH_TICKS + DASH_IFRAMES_EXTRA);
        self.blink = false;
        if let State::Attack { stage, .. } = self.state {
            self.end_swing(stage);
        }
        self.set_state(State::Dash { t: 0 });
        self.body.kx = 0.0;
        self.body.ky = 0.0;
        self.add_ghost();
        fx.sound_var("dash", 2);
        // kick-off puff behind the feet
        for _ in 0..5 {
            let s = fx.rng.range(0.4, 1.1);
            let a = (-d.1).atan2(-d.0) + fx.rng.signed() * 0.8;
            fx.particles.add(Particle::new(self.body.x - d.0 * 2.0, self.body.y - d.1).z(fx.rng.range(0.0, 1.5)).vel(a.cos() * s, a.sin() * s * 0.6, fx.rng.range(0.1, 0.4)).color(dust_color(zone, self.body.x, self.body.y)).life(fx.rng.range(12.0, 20.0)).grav(0.02).drag(0.86));
        }
        true
    }

    /// One of our swings landed (seen as hit-stop): stop the lunge, bounce back a little, and
    /// open the chain window early so a rhythm of hits feels rewarding.
    fn on_connect(&mut self, fx: &mut Fx) {
        if self.connected {
            return;
        }
        self.connected = true;
        let (fxd, fyd) = self.body.face;
        let k = match self.state {
            State::Attack { stage: 3, .. } => 0.9,
            State::Spin { .. } => 0.0,
            _ => 1.4,
        };
        self.body.vx = -fxd * k;
        self.body.vy = -fyd * k;
        let (bx, by, bz) = self.blade_tip();
        fx.particles.sparks(&mut fx.rng, bx, by, bz, fxd, fyd, 3, BLADE_MID, 1.6);
    }

    fn update_dead(&mut self, t: u32, zone: &Zone, fx: &mut Fx) {
        self.state = State::Dead { t: t + 1 };
        self.body.vx = 0.0;
        self.body.vy = 0.0;
        let burst_at = self.death_burst_at();
        if !self.burst && t + 1 >= burst_at {
            if let Some(p) = self.pose() {
                fx.particles.burst_cell(&mut fx.rng, p.cell, p.anchor, p.flip, self.body.x, self.body.y, 1.15, 520.0);
            }
            self.burst = true;
            self.scarf_free = true;
            // the loose scarf gets a little upward flick before it drifts down
            for (i, k) in self.scarf.iter_mut().enumerate() {
                k.pz = k.z - 0.6 - i as f32 * 0.03;
            }
            fx.shake(0.35);
            fx.sound_vol("crystal_shatter", 0.6);
            for i in 0..18 {
                let a = i as f32 / 18.0 * TAU;
                let s = fx.rng.range(0.8, 2.2);
                fx.particles.add(Particle::new(self.body.x, self.body.y).z(8.0).vel(a.cos() * s, a.sin() * s * 0.6, fx.rng.range(0.4, 1.8)).color(WHITE).fade_to(FX_BLUE).glow(1.0).life(fx.rng.range(20.0, 40.0)).grav(0.04).drag(0.92).streak(true));
            }
        }
        if t > burst_at + DEATH_LINGER {
            self.wants_respawn = true;
        }
        self.body.integrate(zone);
        self.update_scarf();
        for g in self.ghosts.iter_mut() {
            g.age += 1;
        }
        self.ghosts.retain(|g| g.age < if g.dash { DASH_GHOST_LIFE } else { GHOST_LIFE });
    }

    /// Ticks into death when the body bursts: after the death anim if the sheet has one.
    fn death_burst_at(&self) -> u32 {
        sheet("hero").and_then(|s| s.dir_anim("death", self.dir)).map(|(a, _)| a.total_ticks() + 8).unwrap_or(30).max(12)
    }

    /// Called by the scene when an enemy hit touches us. Returns true if it landed.
    pub fn on_hit(&mut self, hit: &Hit, fx: &mut Fx) -> bool {
        if !self.alive() || matches!(self.state, State::Respawn { .. }) || !self.body.take(hit) {
            return false;
        }
        self.hurt_last = true;
        self.body.iframes = HURT_IFRAMES;
        self.blink = true;
        self.connected = false;
        let (kx, ky) = norm(self.body.kx, self.body.ky);
        // face the danger: the hurt pose recoils away from it
        if kx != 0.0 || ky != 0.0 {
            self.face_to(-kx, -ky);
        }
        fx.sound("player_hurt");
        for i in 0..12 {
            let a = ky.atan2(kx) + fx.rng.signed() * 1.0;
            let s = fx.rng.range(1.0, 2.8);
            let c = DANGER[i % 3];
            fx.particles.add(Particle::new(self.body.x, self.body.y).z(7.0).vel(a.cos() * s, a.sin() * s * 0.7, fx.rng.range(0.3, 1.6)).color(WHITE).fade_to(c).glow(1.0).life(fx.rng.range(10.0, 20.0)).grav(0.1).drag(0.88).streak(true));
        }
        if self.body.hp <= 0 {
            self.body.hp = 0;
            self.body.kx *= 0.5;
            self.body.ky *= 0.5;
            self.set_state(State::Dead { t: 0 });
            fx.sound("player_die");
            fx.shake(0.65);
            fx.freeze(16);
            fx.flash(0.25);
        } else {
            self.set_state(State::Hurt { t: 0 });
            fx.shake(0.45);
            fx.freeze(7);
        }
        true
    }

    // ---------------------------------------------------------------------------- footsteps

    fn footsteps(&mut self, zone: &Zone, fx: &mut Fx) {
        if !(matches!(self.state, State::Normal) && self.moving) {
            return;
        }
        // Step on the run cycle's contact frames (first and middle frame), or every 14 ticks.
        let (frame, frames) = match sheet("hero").and_then(|s| s.dir_anim("run", self.dir)) {
            Some((a, _)) => (a.frame_at(self.anim_t, true), a.frames.max(2)),
            None => (((self.anim_t / 7) % 4) as usize, 4),
        };
        let contact = frame == 0 || frame == frames / 2;
        let changed = frame != self.step_frame;
        self.step_frame = frame;
        if !(contact && (changed || self.anim_t == 1)) {
            return;
        }
        let (x, y) = (self.body.x, self.body.y);
        let water = zone.tile_at(x, y) == Tile::Shallow;
        fx.sound_var(step_sound(zone, x, y), 3);
        let side = if frame == 0 { 1.0 } else { -1.0 };
        let (px, py) = (-self.body.face.1 * side * 1.5, self.body.face.0 * side * 0.8);
        if water {
            let splash = match zone.id {
                ZoneId::Bloomwild => rgb(0xe3e6d4),
                ZoneId::Glasswater => rgb(0xc2d7cf),
                _ => rgb(0xc1c8c5),
            };
            for _ in 0..4 {
                let a = fx.rng.f32() * TAU;
                fx.particles.add(Particle::new(x + px, y + py).z(0.5).vel(a.cos() * 0.5, a.sin() * 0.3, fx.rng.range(0.8, 1.4)).color(splash).life(fx.rng.range(12.0, 20.0)).grav(0.14).drag(0.95));
            }
        } else {
            let c = dust_color(zone, x, y);
            let n = 1 + fx.rng.irange(0, 2) as usize;
            for _ in 0..n {
                fx.particles.add(
                    Particle::new(x + px - self.body.face.0 * 2.0, y + py)
                        .z(0.5)
                        .vel(-self.body.face.0 * 0.35 + fx.rng.signed() * 0.2, -self.body.face.1 * 0.15 + fx.rng.signed() * 0.1, fx.rng.range(0.15, 0.4))
                        .color(c)
                        .life(fx.rng.range(12.0, 20.0))
                        .grav(0.03)
                        .drag(0.86),
                );
            }
        }
    }

    // ---------------------------------------------------------------------------- scarf

    /// Where the scarf leaves the body, in world space with height.
    fn neck(&self) -> (f32, f32, f32) {
        let (dx, dy) = self.pose().and_then(|p| pose_point(&p, "scarf_anchor")).unwrap_or(match self.dir {
            Dir::Down => (0.0, -10.0),
            Dir::Up => (0.0, -10.0),
            Dir::Right => (-2.0, -10.0),
            Dir::Left => (2.0, -10.0),
        });
        // A screen offset can't tell height from depth; treat it as height, and tuck the root
        // a pixel behind the body so the tail starts at the nape.
        let back = match self.dir {
            Dir::Down => -1.0,
            Dir::Up => 1.0,
            _ => 0.0,
        };
        (self.body.x + dx, self.body.y + back, (-dy).max(2.0) + self.body.z)
    }

    fn reset_scarf(&mut self) {
        let (x, y, z) = self.neck();
        let side = if self.body.face.0 > 0.2 { -1.0 } else { 1.0 };
        self.scarf = (0..SCARF_KNOTS)
            .map(|i| {
                let k = i as f32;
                let (kx, ky, kz) = (x + side * k * 0.9, y, (z - k * 1.0).max(0.0));
                Knot { x: kx, y: ky, z: kz, px: kx, py: ky, pz: kz }
            })
            .collect();
    }

    /// A light chain in 3D: drag toward the air (a breeze plus whatever the body does), a little
    /// gravity, a travelling flutter, the ground below, then follow-the-leader length constraints.
    fn update_scarf(&mut self) {
        let n = self.scarf.len();
        let t = self.t as f32;
        let speed = (self.body.vx.powi(2) + self.body.vy.powi(2)).sqrt();
        let pace = (speed / RUN_SPEED).clamp(0.0, 1.6);
        // A breeze with slow gusts. Facing sideways it streams the scarf out behind; facing
        // down or up it carries it off to one side, where it peeks out past the body. (Drifting
        // it "behind" in depth would project it straight up on screen and read as an antenna.)
        let gust = 0.8 + 0.2 * (t * 0.011).sin() * (t * 0.029 + 1.3).sin();
        let breeze = match self.dir {
            Dir::Right => -0.3,
            Dir::Left => 0.3,
            _ => 0.26,
        };
        let air = (breeze * gust, 0.0, 0.05);
        let spin = matches!(self.state, State::Spin { t } if (SPIN_WINDUP..SPIN_WINDUP + SPIN_ACTIVE + 4).contains(&t));
        let whip = self.scarf_whip();
        let (ax, ay, az) = self.neck();
        if !self.scarf_free {
            let r = self.scarf[0];
            if (r.x - ax).abs() + (r.y - ay).abs() > 16.0 {
                // teleported (respawn, a tool warp): don't whip the chain across the map
                self.reset_scarf();
            }
        }
        let first = if self.scarf_free { 0 } else { 1 };
        for i in first..n {
            let k = i as f32 / (n - 1) as f32;
            let p = self.scarf[i];
            let (mut vx, mut vy, mut vz) = (p.x - p.px, p.y - p.py, p.z - p.pz);
            let settle = (1.0 - pace).max(0.0);
            let drag = if self.scarf_free { 0.12 } else { 0.1 + 0.1 * settle };
            vx += (air.0 - vx) * drag;
            vy += (air.1 - vy) * drag;
            vz += (air.2 - vz) * drag;
            vz -= if self.scarf_free { 0.02 } else { 0.022 };
            if !self.scarf_free {
                // Screen y is y - z, so a scarf drooping back in depth projects as a flat line.
                // A weak spring toward the neck's depth keeps a resting scarf hanging in the
                // visible plane; running still trails it in depth through inertia.
                vy += (ay - p.y) * 0.016;
                // Ease back to a short hanging ribbon after a turn, rather than letting
                // inertia fold the tip into a loop across the cloak.
                let side = if self.dir == Dir::Right { -1.0 } else { 1.0 };
                let rest_x = ax + side * i as f32 * 0.7;
                let rest_z = (az - i as f32 * 1.2).max(0.0);
                vx += (rest_x - p.x) * 0.025 * settle;
                vz += (rest_z - p.z) * 0.025 * settle;
            }
            // flutter: a wave running from the neck to the tip, stronger when moving
            let prev = if i > 0 { self.scarf[i - 1] } else { p };
            let (sx, sy) = norm(p.x - prev.x, p.y - prev.y);
            let (sx, sy) = if sx == 0.0 && sy == 0.0 { (1.0, 0.0) } else { (sx, sy) };
            let wave = (t * 0.27 - i as f32 * 0.7).sin() * (0.006 + 0.07 * pace) * k;
            vx += -sy * wave;
            vy += sx * wave;
            vz += (t * 0.21 - i as f32 * 0.9).cos() * (0.004 + 0.03 * pace) * k;
            if spin {
                // swirl around the body during the spin
                let (rx, ry) = (p.x - self.body.x, p.y - self.body.y);
                let r = (rx * rx + ry * ry).sqrt().max(1.0);
                vx += -ry / r * 0.22 * self.spin_dir * k;
                vy += rx / r * 0.22 * self.spin_dir * k;
            }
            if whip != 0.0 {
                // combo follow-through: the cloth is swung round with the torso, then lags
                let (rx, ry) = (p.x - self.body.x, p.y - self.body.y);
                let r = (rx * rx + ry * ry).sqrt().max(1.0);
                vx += -ry / r * whip * k;
                vy += rx / r * whip * k;
                vz += whip.abs() * 0.9 * k;
            }
            let mut q = Knot { x: p.x + vx, y: p.y + vy, z: p.z + vz, px: p.x, py: p.y, pz: p.z };
            if !self.scarf_free && q.z > az + 2.0 {
                // cloth this light shouldn't climb over the head: a soft ceiling just above
                // the neck stops stop-and-flip moments from reading as a hook or antenna
                let over = q.z - az - 2.0;
                q.z -= over * 0.5;
                q.pz = q.z - (q.z - q.pz) * 0.5;
            }
            if q.z < 0.0 {
                q.z = 0.0;
                q.pz = 0.0;
                // ground friction
                q.px = q.x - vx * 0.6;
                q.py = q.y - vy * 0.6;
            }
            self.scarf[i] = q;
        }
        if !self.scarf_free {
            self.scarf[0] = Knot { x: ax, y: ay, z: az, px: ax, py: ay, pz: az };
        }
        for _ in 0..3 {
            for i in 1..n {
                let a = self.scarf[i - 1];
                let b = self.scarf[i];
                let (dx, dy, dz) = (b.x - a.x, b.y - a.y, b.z - a.z);
                let d = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
                let k = (d - SCARF_SEG) / d;
                if self.scarf_free {
                    let h = k * 0.5;
                    self.scarf[i - 1].x += dx * h;
                    self.scarf[i - 1].y += dy * h;
                    self.scarf[i - 1].z = (self.scarf[i - 1].z + dz * h).max(0.0);
                    self.scarf[i].x -= dx * h;
                    self.scarf[i].y -= dy * h;
                    self.scarf[i].z = (self.scarf[i].z - dz * h).max(0.0);
                } else {
                    self.scarf[i].x -= dx * k;
                    self.scarf[i].y -= dy * k;
                    self.scarf[i].z = (self.scarf[i].z - dz * k).max(0.0);
                }
            }
            // a little bending stiffness keeps the curve smooth (clean pixel lines, no kinks)
            for i in 1..n - 1 {
                let (a, b) = (self.scarf[i - 1], self.scarf[i + 1]);
                let m = &mut self.scarf[i];
                m.x += ((a.x + b.x) * 0.5 - m.x) * 0.12;
                m.y += ((a.y + b.y) * 0.5 - m.y) * 0.12;
                m.z += ((a.z + b.z) * 0.5 - m.z) * 0.12;
            }
        }
    }

    /// Signed swirl applied to the scarf during a combo swing (same sense as the smear): a
    /// small pull against the swing on the last windup ticks, then a kick with it while the
    /// hitbox is out. Purely visual; zero outside attacks.
    fn scarf_whip(&self) -> f32 {
        let State::Attack { stage, t } = self.state else { return 0.0 };
        let sw = &SWINGS[(stage - 1) as usize];
        let t = t.saturating_sub(1);
        let k = if t < sw.windup {
            if t + 2 >= sw.windup { -0.05 } else { 0.0 }
        } else if t < sw.windup + sw.active {
            if sw.heavy { 0.3 } else { 0.22 }
        } else {
            0.0
        };
        k * self.swing_hand
    }

    // ---------------------------------------------------------------------------- sprite

    fn add_ghost(&mut self) {
        if let Some(p) = self.pose() {
            self.ghosts.push(Ghost { x: self.body.x, y: self.body.y - self.body.z, cell: p.cell, anchor: p.anchor, flip: p.flip, age: 0, dash: matches!(self.state, State::Dash { .. }) });
        }
    }

    /// The anim to show and the ticks to sample it at.
    fn anim_key(&self) -> (&'static str, u32, bool) {
        match self.state {
            State::Normal => (if self.moving { "run" } else { "idle" }, self.anim_t, true),
            State::Attack { stage, t } => {
                let (name, ticks) = self.attack_pose_ticks(stage, t);
                (name, ticks, false)
            }
            State::Dash { t } => ("dash", t, false),
            State::Charge { .. } => (if self.moving { "charge_walk" } else { "charge" }, self.anim_t, true),
            State::Spin { t } => ("spin", t.saturating_sub(1 + SPIN_WINDUP), false),
            State::Hurt { t } => ("hurt", t, false),
            State::Dead { t } => ("death", t, false),
            State::Respawn { t } => {
                if self.land_t > 0 {
                    ("land", self.land_t, false)
                } else {
                    ("fall", t, true)
                }
            }
        }
    }

    /// Map a combo's combat tick to the authored anticipation and strike durations.
    fn attack_pose_ticks(&self, stage: u8, t: u32) -> (&'static str, u32) {
        let name = ["attack1", "attack2", "attack3"][(stage - 1) as usize];
        let t = t.saturating_sub(1);
        let windup = SWINGS[(stage - 1) as usize].windup;
        let anticipations = if stage == 3 { 2 } else { 1 };
        let first = sheet("hero")
            .and_then(|s| s.dir_anim(name, self.dir))
            .map(|(a, _)| a.durations.iter().take(anticipations).sum::<u32>())
            .unwrap_or(1);
        let ticks = if t < windup {
            (t * first / windup.max(1)).min(first.saturating_sub(1))
        } else {
            first + (t - windup)
        };
        (name, ticks)
    }

    /// The current sprite frame (sheet, or the code-drawn fallback).
    fn pose(&self) -> Option<Pose> {
        let (name, ticks, looping) = self.anim_key();
        self.pose_at(name, ticks, looping)
    }

    /// Resolve an authored pose without changing the player's animation or combat state.
    fn pose_at(&self, name: &str, ticks: u32, looping: bool) -> Option<Pose> {
        if let Some(s) = sheet("hero") {
            for n in fallbacks(name) {
                if let Some((a, flip)) = s.dir_anim(n, self.dir) {
                    // Mirroring a directionless rotation reverses its authored turn and
                    // disconnects the blade (and its metadata) from the clockwise smear.
                    let flip = flip && a.name != "spin";
                    let f = a.frame_at(ticks, looping);
                    return Some(Pose { cell: s.cell(a, f), anchor: s.anchor_of(a), flip, anim: Some(a), frame: f });
                }
            }
        }
        let c = fallback_cell(self.dir);
        Some(Pose { cell: c, anchor: (c.w / 2, c.h - 1), flip: self.dir == Dir::Left, anim: None, frame: 0 })
    }

    /// The sprite cell for the current frame: (cell, anchor, flip).
    pub fn current_cell(&self) -> Option<(&'static Cell, (i32, i32), bool)> {
        self.pose().map(|p| (p.cell, p.anchor, p.flip))
    }

    /// World position (x, ground y, height) of the blade tip.
    fn blade_tip(&self) -> (f32, f32, f32) {
        let (dx, dy) = self.pose().and_then(|p| pose_point(&p, "blade_tip")).unwrap_or(match self.dir {
            Dir::Down => (-6.0, -3.0),
            Dir::Up => (6.0, -4.0),
            Dir::Right => (8.0, -3.0),
            Dir::Left => (-8.0, -3.0),
        });
        // Keep the projection exact even when the drawn blade extends below the feet.
        (self.body.x + dx, self.body.y + dy.max(0.0), (-dy).max(0.0) + self.body.z)
    }

    /// An actual glowing pixel near the upper end of the live blade. Most idle tips drag
    /// below the feet: using the tip as a height-zero source cannot illuminate the floor.
    /// Keep this emitter on the visible blade instead of raising an invisible body lantern.
    fn blade_light_position(&self) -> Option<(f32, f32, f32)> {
        let p = self.pose()?;
        let (px, py) = if let Some(a) = p.anim {
            type BladeSources = std::collections::HashMap<(usize, usize), Option<(i32, i32)>>;
            static SOURCES: OnceLock<BladeSources> = OnceLock::new();
            *SOURCES.get_or_init(|| {
                let mut sources = BladeSources::new();
                if let Some(s) = sheet("hero") {
                    for a in s.anims.values() {
                        for frame in 0..a.frames {
                            let source = a.point("blade_tip", frame)
                                .and_then(|tip| upper_blade_pixel(s.cell(a, frame), tip));
                            sources.insert((a.row, frame), source);
                        }
                    }
                }
                sources
            }).get(&(a.row, p.frame))?
        } else {
            let (tx, gy, gz) = self.blade_tip();
            let dx = (tx - self.body.x) as i32;
            let dy = (gy - gz - (self.body.y - self.body.z)) as i32;
            upper_blade_pixel(p.cell, (p.anchor.0 + if p.flip { -dx } else { dx }, p.anchor.1 + dy))
        }?;
        // Match the sprite raster, including fractional motion, elevation and mirroring.
        let x = self.body.x.floor() + if p.flip { p.anchor.0 - px } else { px - p.anchor.0 } as f32 + 0.5;
        let projected_y = (self.body.y - self.body.z).floor() + (py - p.anchor.1) as f32 + 0.5;
        let height = (self.body.y.floor() - projected_y).max(self.body.z.max(0.0));
        Some((x, projected_y + height, height))
    }

    // ---------------------------------------------------------------------------- draw

    pub fn draw(&self, f: &mut Frame) {
        self.draw_lights(f);
        let dead_burst = matches!(self.state, State::Dead { .. }) && self.burst;
        if dead_burst {
            self.draw_scarf(f);
            return;
        }
        let z = self.body.z;
        let zk = (1.0 - z / (RESPAWN_HEIGHT + 10.0)).clamp(0.2, 1.0);
        f.shadow(f.sx(self.body.x), f.sy(self.body.y), (5.0 * zk).round().max(2.0) as i32, if zk > 0.6 { 2 } else { 1 }, 0.45 * zk);
        self.draw_ground_fx(f);
        // Recent echoes sit below the live sprite's value; older ones lose coverage quickly.
        // Insertion order is oldest first, so the nearest echo owns any overlap.
        for g in &self.ghosts {
            let (c, alpha) = match (g.dash, g.age) {
                (true, 0..=1) => (FX_BLUE.scale(0.65), 1.0),
                (true, 2..=4) => (FX_DEEP.scale(0.7), 1.0),
                (true, _) => (FX_DEEP.scale(0.5), 0.5),
                (false, 0..=3) => (FX_BLUE, 1.0),
                (false, 4..=8) => (FX_DEEP, 1.0),
                (false, _) => (FX_DEEP, 0.5),
            };
            let o = DrawOpts { flip: g.flip, alpha, silhouette: Some(c), silhouette_glow: true, ..Default::default() };
            f.sprite_w(g.cell, g.anchor, g.x, g.y, &o);
        }
        let attack_dir = pick_dir(self.dir, self.swing_angle.cos(), self.swing_angle.sin());
        let smear_behind = matches!(self.state, State::Attack { .. }) && attack_dir == Dir::Up;
        if smear_behind {
            self.draw_slash(f);
        }
        let scarf_front = self.dir == Dir::Up;
        if !scarf_front {
            self.draw_scarf(f);
        }
        let blink = self.blink && self.body.iframes > 0 && !matches!(self.state, State::Hurt { .. } | State::Dead { .. }) && (self.t / 4) % 2 == 0;
        if let Some(p) = self.pose() {
            let tint = if self.heal_t > 0 { Some((HEAL, self.heal_t as f32 / 24.0 * 0.4)) } else { None };
            let flash = if matches!(self.state, State::Dead { t } if t < 6) { 1.0 } else { self.body.flash };
            let o = DrawOpts { actor: true, foot_depth: 4, foot_half_w: self.body.r, ground_y: Some(self.body.y.floor()), elevation: self.body.z, flip: p.flip, flash, alpha: if blink { 0.5 } else { 1.0 }, shadow_alpha: blink.then_some(1.0), tint, ..Default::default() };
            f.sprite_w(p.cell, p.anchor, self.body.x, self.body.y - z, &o);
        }
        if scarf_front {
            self.draw_scarf(f);
        }
        if !smear_behind {
            self.draw_slash(f);
        }
        self.draw_glints(f);
    }

    /// Surface lighting follows the blade in every region; the legacy renderer retains its
    /// original daylight gate and compact additive pools.
    fn draw_lights(&self, f: &mut Frame) {
        if f.surface_lighting {
            self.draw_surface_lights(f);
            return;
        }
        // Read the current frame, including headless views that never update the player.
        // Emissive blade/eye pixels are drawn separately and remain bright in every zone.
        let dark = (1.0 - f.ambient.luma() * 1.4).clamp(0.0, 1.0);
        if dark <= 0.0 {
            return;
        }
        let (bx, by, bz) = self.blade_tip();
        let x = self.body.x;
        if matches!(self.state, State::Dead { .. }) {
            if self.burst {
                let t = match self.state {
                    State::Dead { t } => t.saturating_sub(self.death_burst_at()),
                    _ => 0,
                };
                let k = (1.0 - t as f32 / 50.0).max(0.0);
                f.add_light_3d_w(x, self.body.y, self.body.z + 6.0, 18.0 * k + 4.0, BLADE_MID, 0.35 * k * dark);
            }
            return;
        }
        // A blade-sized reflection, rather than a lantern-sized pool around the hero.
        let r = 14.0 + 4.0 * dark;
        let i = 0.26 * dark;
        f.add_light_3d_w(bx, by, bz, r, BLADE_MID, i);
        match self.state {
            State::Attack { stage, t } => {
                let t = t.saturating_sub(1);
                let sw = &SWINGS[(stage - 1) as usize];
                if t >= sw.windup && t < sw.windup + sw.active + 4 {
                    // Keep the flash within the cutting edge's immediate footprint.
                    let k = 1.0 - (t - sw.windup) as f32 / (sw.active + 4) as f32;
                    let (fxd, fyd) = self.body.face;
                    f.add_light_3d_w(x + fxd * 12.0, self.body.y + fyd * 8.0, self.body.z + 6.0, 12.0 + sw.r * 0.35, BLADE_MID, 0.45 * dark * k);
                }
            }
            State::Charge { t } => {
                let p = ((t - CHARGE_START) as f32 / (CHARGE_FULL - CHARGE_START) as f32).min(1.0);
                let pulse = if self.full_t > 0 { 0.04 * ((self.full_t as f32) * 0.2).sin() } else { 0.0 };
                f.add_light_3d_w(bx, by, bz, 14.0 + 8.0 * p, BLADE_MID, (0.3 + 0.15 * p + pulse) * dark);
            }
            State::Spin { t } if t > SPIN_WINDUP => {
                let t = t - 1;
                let k = 1.0 - ((t - SPIN_WINDUP) as f32 / (SPIN_TOTAL - SPIN_WINDUP) as f32);
                f.add_light_3d_w(x, self.body.y, self.body.z + 6.0, 24.0, BLADE_MID, 0.5 * k * dark);
            }
            State::Respawn { .. } => {
                f.add_light_3d_w(x, self.body.y, self.body.z + 6.0, 18.0, BLADE_MID, 0.3 * dark);
            }
            _ => {}
        }
    }

    /// One moving source, rather than stacked circular pools. Its authored pose changes both
    /// the direction of reflected light and the shadows of nearby enemies and scenery.
    fn draw_surface_lights(&self, f: &mut Frame) {
        if matches!(self.state, State::Dead { .. }) {
            return;
        }
        let Some((x, y, height)) = self.blade_light_position() else { return };
        let (radius, intensity) = match self.state {
            State::Attack { stage, t } => {
                let sw = &SWINGS[(stage - 1) as usize];
                let tick = t.saturating_sub(1);
                let k = if tick >= sw.windup {
                    (1.0 - (tick - sw.windup) as f32 / (sw.active + 4) as f32).max(0.0)
                } else { 0.0 };
                (48.0 + (if sw.heavy { 16.0 } else { 12.0 }) * k,
                 0.9 + (if sw.heavy { 0.6 } else { 0.4 }) * k)
            }
            State::Charge { t } => {
                let p = (t.saturating_sub(CHARGE_START) as f32 / (CHARGE_FULL - CHARGE_START) as f32).min(1.0);
                let pulse = if self.full_t > 0 { 0.035 * (self.full_t as f32 * 0.2).sin() } else { 0.0 };
                (48.0 + 16.0 * p, 0.9 + 0.45 * p + pulse)
            }
            State::Spin { t } if t > SPIN_WINDUP => {
                let k = (1.0 - (t - SPIN_WINDUP) as f32 / (SPIN_TOTAL - SPIN_WINDUP) as f32).max(0.0);
                (48.0 + 16.0 * k, 0.9 + 0.45 * k)
            }
            _ => (48.0, 0.9),
        };
        f.add_light_3d_w(x, y, height, radius, BLADE_MID, intensity);
    }

    /// The scarf ribbon: rasterized around the chain's projected centerline, shaded by which face
    /// shows (a twist wave travelling down it) and which edge faces the light, with an outline.
    fn draw_scarf(&self, f: &mut Frame) {
        let n = self.scarf.len();
        if n < 2 {
            return;
        }
        // If the player was moved without an update (zone viewer), carry the scarf along.
        let (ox, oy) = if self.scarf_free {
            (0.0, 0.0)
        } else {
            let (nx, ny, nz) = self.neck();
            let r = self.scarf[0];
            let (dx, dy) = (nx - r.x, (ny - nz) - (r.y - r.z));
            if dx.abs() + dy.abs() > 6.0 { (dx, dy) } else { (0.0, 0.0) }
        };
        // Project with depth compressed relative to the neck: a steeper camera for the cloth
        // alone, so a scarf streaming straight back while running at the camera tucks behind
        // the head instead of standing up above it.
        const DEPTH_K: f32 = 0.5;
        let ry = self.scarf[0].y;
        let pts: Vec<(f32, f32)> = self.scarf.iter().map(|k| (k.x + ox - f.cam_x as f32, ry + (k.y - ry) * DEPTH_K - k.z + oy - f.cam_y as f32)).collect();
        let mut cum = vec![0.0f32; n];
        for i in 1..n {
            cum[i] = cum[i - 1] + ((pts[i].0 - pts[i - 1].0).powi(2) + (pts[i].1 - pts[i - 1].1).powi(2)).sqrt();
        }
        let total = cum[n - 1].max(0.01);
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in &pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let (gx, gy) = (x0.floor() as i32 - 3, y0.floor() as i32 - 3);
        let (gw, gh) = ((x1.ceil() as i32 + 3 - gx + 1).max(1), (y1.ceil() as i32 + 3 - gy + 1).max(1));
        if gw > 80 || gh > 80 {
            return;
        }
        // 0 empty, 1 outline, 2.. palette index + 2
        let mut grid = vec![0u8; (gw * gh) as usize];
        let t = self.t as f32;
        let pace = ((self.body.vx.powi(2) + self.body.vy.powi(2)).sqrt() / RUN_SPEED).clamp(0.0, 1.5);
        let twist_speed = 0.07 + 0.09 * pace;
        // front face shows unless the twist wave flips it
        let front = |s: f32| (s * 8.5 - t * twist_speed).sin() > -0.45;
        let width = |s: f32| if s < 0.6 { 2.0 } else { 1.0 };
        let color_at = |s: f32, lit: bool| -> u8 {
            let fr = front(s);
            let idx = match (fr, lit) {
                (true, true) => {
                    if s < 0.3 {
                        3
                    } else {
                        2
                    }
                }
                (true, false) => 1,
                (false, true) => 1,
                (false, false) => 0,
            };
            idx + 2
        };
        for py in 0..gh {
            for px in 0..gw {
                let (qx, qy) = ((gx + px) as f32 + 0.5, (gy + py) as f32 + 0.5);
                // closest point on the polyline
                let mut best = (f32::MAX, 0.0f32, 0.0f32, 0.0f32);
                for i in 1..n {
                    let (ax, ay) = pts[i - 1];
                    let (bx, by) = pts[i];
                    let (vx, vy) = (bx - ax, by - ay);
                    let l2 = vx * vx + vy * vy;
                    let u = if l2 > 1e-6 { (((qx - ax) * vx + (qy - ay) * vy) / l2).clamp(0.0, 1.0) } else { 0.0 };
                    let (cx, cy) = (ax + vx * u, ay + vy * u);
                    let d2 = (qx - cx).powi(2) + (qy - cy).powi(2);
                    if d2 < best.0 {
                        let s = (cum[i - 1] + (cum[i] - cum[i - 1]) * u) / total;
                        // which side of the ribbon: toward the light (upper left) or away
                        let side = (qx - cx) * -0.7 + (qy - cy) * -0.7;
                        best = (d2, s, side, 0.0);
                    }
                }
                let (d2, s, side, _) = best;
                let w = width(s);
                if d2.sqrt() <= w * 0.5 {
                    let lit = w < 1.5 || side > 0.0;
                    grid[(py * gw + px) as usize] = color_at(s, lit);
                }
            }
        }
        // the centerline guarantees an unbroken 1-pixel tail
        for i in 1..n {
            let (ax, ay) = (pts[i - 1].0.floor() as i32 - gx, pts[i - 1].1.floor() as i32 - gy);
            let (bx, by) = (pts[i].0.floor() as i32 - gx, pts[i].1.floor() as i32 - gy);
            let s = cum[i] / total;
            let c = color_at(s, true);
            Frame::line_with(ax, ay, bx, by, |x, y| {
                if x >= 0 && y >= 0 && x < gw && y < gh && grid[(y * gw + x) as usize] == 0 {
                    grid[(y * gw + x) as usize] = c;
                }
            });
        }
        // outline: empty pixels touching the ribbon (4-neighbors), except right at the neck
        let root = (pts[0].0 - gx as f32, pts[0].1 - gy as f32);
        let mut outline = Vec::new();
        for py in 0..gh {
            for px in 0..gw {
                if grid[(py * gw + px) as usize] != 0 {
                    continue;
                }
                let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                    let (x, y) = (px + dx, py + dy);
                    x >= 0 && y >= 0 && x < gw && y < gh && grid[(y * gw + x) as usize] >= 2
                });
                let by_root = (px as f32 + 0.5 - root.0).powi(2) + (py as f32 + 0.5 - root.1).powi(2) < 6.5;
                if near && !by_root {
                    outline.push((px, py));
                }
            }
        }
        for (px, py) in outline {
            grid[(py * gw + px) as usize] = 1;
        }
        for py in 0..gh {
            for px in 0..gw {
                let v = grid[(py * gw + px) as usize];
                if v == 0 {
                    continue;
                }
                let c = if v == 1 { OUTLINE } else { SCARF[(v - 2) as usize] };
                f.put(gx + px, gy + py, c);
                f.mark_actor(gx + px, gy + py);
            }
        }
    }

    fn draw_slash(&self, f: &mut Frame) {
        let (cx, cy) = (self.body.x + self.body.face.0 * 2.0, self.body.y - 6.0 - self.body.z);
        match self.state {
            State::Attack { stage, t } => {
                let t = t.saturating_sub(1);
                let sw = &SWINGS[(stage - 1) as usize];
                if t < sw.windup {
                    return;
                }
                let i = (t - sw.windup) as usize;
                let Some(&(tail, head, wk, age)) = SMEAR_KEYS.get(i) else { return };
                let sweep = sw.sweep * self.swing_hand;
                let (radius, angle) = self.combo_smear_head(stage, t + 1);
                let span = sweep * (head - tail);
                // One dominant swept mass per strike; it loses area through the keys instead of
                // being echoed by a second inner crescent.
                draw_crescent(f, cx, cy, radius, angle - span, span, (sw.width * SMEAR_BODY * wk).max(1.0), age);
            }
            State::Spin { t } if t > SPIN_WINDUP => {
                let k = t - 1 - SPIN_WINDUP;
                let fade_ticks = 7;
                if k >= SPIN_ACTIVE + fade_ticks {
                    return;
                }
                // The directionless spin sheet starts facing down and turns by an eighth
                // every two ticks. Keep it unmirrored, even when the charge faced left.
                let turns = TAU;
                let prog = (k as f32 / SPIN_ACTIVE as f32).min(1.0);
                let head = PI * 0.5 + self.spin_dir * prog * turns;
                let mut len = (prog * turns).clamp(PI * 0.25, PI * 1.35);
                let mut w = 9.0;
                let mut age = 0;
                if k >= SPIN_ACTIVE {
                    let d = (k - SPIN_ACTIVE) as f32 / fade_ticks as f32;
                    len *= 1.0 - d * 0.8;
                    w *= 1.0 - d * 0.75;
                    age = 1 + (d * 2.5) as u8;
                }
                draw_crescent(f, cx, cy, SPIN_RADIUS - 2.0, head - self.spin_dir * len, self.spin_dir * len, w.max(1.0), age.min(3));
            }
            _ => {}
        }
    }

    /// Follow the live blade during a strike, then hold its last active tip as the trail fades.
    fn combo_smear_head(&self, stage: u8, t: u32) -> (f32, f32) {
        let sw = &SWINGS[(stage - 1) as usize];
        let (name, ticks) = self.attack_pose_ticks(stage, t.min(sw.windup + sw.active));
        let tip = self
            .pose_at(name, ticks, false)
            .and_then(|p| pose_point(&p, "blade_tip"));
        if let Some((dx, dy)) = tip {
            let dx = dx - self.body.face.0 * 2.0;
            let dy = (dy + 6.0) / SMEAR_SQUASH;
            (dx.hypot(dy), dy.atan2(dx))
        } else {
            (sw.r, self.swing_angle + sw.sweep * self.swing_hand * 0.5)
        }
    }

    /// Ground-level effects drawn under the body: the charge ring and the spin shockwave.
    fn draw_ground_fx(&self, f: &mut Frame) {
        let (gx, gy) = (self.body.x, self.body.y);
        match self.state {
            State::Charge { t } => {
                let p = ((t - CHARGE_START) as f32 / (CHARGE_FULL - CHARGE_START) as f32).min(1.0);
                let ease = 1.0 - (1.0 - p).powi(2);
                let rx = 21.0 - 10.0 * ease;
                let (c, dashes) = if self.full_t > 0 {
                    (if self.full_t < 6 { WHITE } else { BLADE_MID }, 0)
                } else {
                    (if p > 0.6 { BLADE_EDGE } else { FX_BLUE }, 12)
                };
                draw_ring(f, gx, gy, rx, rx * 0.5, c, dashes, self.t as f32 * 0.03, true);
                if self.full_t > 0 && self.full_t < 10 {
                    let r2 = rx + self.full_t as f32 * 1.6;
                    draw_ring(f, gx, gy, r2, r2 * 0.5, if self.full_t < 5 { BLADE_MID } else { FX_BLUE }, 16, 0.0, true);
                }
            }
            State::Spin { t } if t > SPIN_WINDUP => {
                let k = (t - 1 - SPIN_WINDUP) as f32;
                if k < 12.0 {
                    let r = 10.0 + (1.0 - (1.0 - k / 12.0).powi(2)) * (SPIN_RADIUS - 10.0);
                    let c = if k < 3.0 {
                        WHITE
                    } else if k < 7.0 {
                        BLADE_MID
                    } else {
                        FX_BLUE
                    };
                    draw_ring(f, gx, gy, r, r * 0.5, c, if k < 6.0 { 0 } else { 20 }, k * 0.02, false);
                }
            }
            State::Respawn { .. } if self.land_t > 0 && self.land_t < 12 => {
                let k = self.land_t as f32;
                let r = 6.0 + k * 2.2;
                draw_ring(f, gx, gy, r, r * 0.5, if k < 4.0 { WHITE } else { BLADE_MID }, if k < 5.0 { 0 } else { 14 }, 0.0, false);
            }
            _ => {}
        }
    }

    /// Glints: the gleam that telegraphs the heavy third hit, and the charged blade's star.
    fn draw_glints(&self, f: &mut Frame) {
        let (bx, by, bz) = self.blade_tip();
        let (sx, sy) = (f.sx(bx), f.sy(by - bz));
        match self.state {
            State::Attack { stage: 3, t } => {
                let t = t.saturating_sub(1);
                let w = SWINGS[2].windup;
                if t + 4 >= w && t < w {
                    let size = [1, 2, 3, 2][(t + 4 - w).min(3) as usize];
                    draw_star(f, sx, sy, size);
                }
            }
            State::Charge { .. } if self.full_t > 0 => {
                let ft = self.full_t;
                let size = if ft < 3 {
                    3
                } else if ft < 7 {
                    2
                } else if ft % 40 < 6 {
                    [1, 2, 2, 1, 1, 1][(ft % 40) as usize]
                } else {
                    1
                };
                draw_star(f, sx, sy, size);
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------------------------------ helpers

/// Four-way sprite direction from a vector, keeping the current one on near-diagonals.
fn pick_dir(cur: Dir, x: f32, y: f32) -> Dir {
    let (ax, ay) = (x.abs(), y.abs());
    let h = if x > 0.0 { Dir::Right } else { Dir::Left };
    let v = if y > 0.0 { Dir::Down } else { Dir::Up };
    if ax > ay * 1.3 {
        return h;
    }
    if ay > ax * 1.3 {
        return v;
    }
    if cur == h || cur == v { cur } else { h }
}

/// Which way the forehand sweeps for a facing (+1 = clockwise on screen), matching the hero
/// sheet: attack1 and attack3 sweep clockwise (facing down: from screen right, through the
/// front, to the left), attack2 is the backhand. Left is the mirrored right.
fn hand_for(dir: Dir) -> f32 {
    match dir {
        Dir::Left => -1.0,
        _ => 1.0,
    }
}

fn fallbacks(name: &str) -> &'static [&'static str] {
    match name {
        "run" => &["run", "idle"],
        "attack1" => &["attack1", "attack2", "attack3", "idle"],
        "attack2" => &["attack2", "attack1", "attack3", "idle"],
        "attack3" => &["attack3", "attack1", "idle"],
        "dash" => &["dash", "run", "idle"],
        "charge" => &["charge", "attack1", "idle"],
        "charge_walk" => &["charge_walk", "charge", "run", "idle"],
        "spin" => &["spin", "attack3", "attack1", "idle"],
        "hurt" => &["hurt", "idle"],
        "death" => &["death", "hurt", "idle"],
        "fall" => &["fall", "hurt", "dash", "idle"],
        "land" => &["land", "charge", "idle"],
        _ => &["idle"],
    }
}

/// A named point from the anim JSON as an offset from the anchor (flip-aware).
fn pose_point(p: &Pose, key: &str) -> Option<(f32, f32)> {
    let (px, py) = p.anim?.point(key, p.frame)?;
    let dx = if p.flip { p.anchor.0 - px } else { px - p.anchor.0 };
    Some((dx as f32, (py - p.anchor.1) as f32))
}

/// The blade's connected emissive component excludes disconnected eye pixels. Cache callers
/// resolve this once per native pose; the per-frame source then needs no image scan/allocation.
fn upper_blade_pixel(cell: &Cell, tip: (i32, i32)) -> Option<(i32, i32)> {
    let seed = cell.px.iter().enumerate().filter(|(_, p)| p.a && p.emit)
        .min_by_key(|(i, _)| {
            let (x, y) = (*i as i32 % cell.w, *i as i32 / cell.w);
            ((x - tip.0).pow(2) + (y - tip.1).pow(2), y, x)
        }).map(|(i, _)| i)?;
    let mut visited = vec![false; cell.px.len()];
    let mut component = vec![seed];
    visited[seed] = true;
    let mut head = 0;
    while head < component.len() {
        let i = component[head];
        head += 1;
        let (x, y) = (i as i32 % cell.w, i as i32 / cell.w);
        for dy in -1..=1 { for dx in -1..=1 {
            let (nx, ny) = (x + dx, y + dy);
            if !(0..cell.w).contains(&nx) || !(0..cell.h).contains(&ny) { continue; }
            let next = (ny * cell.w + nx) as usize;
            let p = cell.px[next];
            if !visited[next] && p.a && p.emit {
                visited[next] = true;
                component.push(next);
            }
        }}
    }
    component.into_iter().map(|i| (i as i32 % cell.w, i as i32 / cell.w))
        .min_by_key(|&(x, y)| (y, (x - tip.0).abs(), x))
}

fn step_sound(zone: &Zone, x: f32, y: f32) -> &'static str {
    match (zone.tile_at(x, y), zone.id) {
        (Tile::Shallow, _) => "step_water",
        (_, ZoneId::Bloomwild) => "step_grass",
        _ => "step_stone",
    }
}

/// Dust takes the color of the ground under the feet, a step lighter.
fn dust_color(zone: &Zone, x: f32, y: f32) -> Rgb {
    let g = zone.gget(x.floor() as i32, y.floor() as i32 + 1);
    if g.r < 0.0 {
        return rgb(0x94a1ae);
    }
    g.lerp(rgb(0xd9dacc), 0.2)
}

/// Is a pixel center inside the (squashed) disk of radius r around (cx, cy)?
#[inline]
fn in_disk(wx: f32, wy: f32, cx: f32, cy: f32, r: f32, squash: f32) -> bool {
    let dx = wx - cx;
    let dy = (wy - cy) / squash;
    dx * dx + dy * dy <= r * r
}

/// A slash smear: a crescent whose outer edge is a clean 1-pixel arc of radius `r` (in a 3/4
/// view ellipse), from angle `a0` sweeping `sweep` radians (the sign is the direction). One
/// broad leading mass sits just behind the head (where the blade is), with a long thin taper to
/// the tail and a short taper into the head. Colors run in a few bands from the rim inward,
/// closed by a dark matte inner contour; `age` 0 is fresh (white rim), 1-2 cool down, 3 breaks
/// the band into dashes as it dissipates.
#[allow(clippy::too_many_arguments)]
pub fn draw_crescent(f: &mut Frame, cx: f32, cy: f32, r: f32, a0: f32, sweep: f32, w: f32, age: u8) {
    if sweep.abs() < 0.02 || r < 2.0 {
        return;
    }
    const SQUASH: f32 = SMEAR_SQUASH;
    let dirn = sweep.signum();
    let span = sweep.abs().min(TAU);
    let x0 = f.sx(cx - r - 1.0);
    let x1 = f.sx(cx + r + 1.0);
    let y0 = f.sy(cy - r * SQUASH - 1.0);
    let y1 = f.sy(cy + r * SQUASH + 1.0);
    let arc_len = span * r;
    for sy in y0..=y1 {
        for sx in x0..=x1 {
            let wx = (sx + f.cam_x) as f32 + 0.5;
            let wy = (sy + f.cam_y) as f32 + 0.5;
            if !in_disk(wx, wy, cx, cy, r, SQUASH) {
                continue;
            }
            let dx = wx - cx;
            let dy = (wy - cy) / SQUASH;
            let d = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            let delta = ((a - a0) * dirn).rem_euclid(TAU);
            if delta > span {
                continue;
            }
            let u = delta / span; // 0 tail .. 1 head
            // thickness profile: a hairline tail that swells late (ease-in), a full body over
            // the last third, and a short taper into the head so the leading end is pointed
            let mut prof = (u / 0.68).min(1.0).powf(1.7);
            if u > 0.86 {
                prof *= 1.0 - ((u - 0.86) / 0.14).powf(1.5) * 0.7;
            }
            let thick = (w * prof).max(1.0);
            let depth = r - d;
            if depth >= thick {
                continue;
            }
            // the rim: inside pixels with a 4-neighbor outside the disk (an 8-connected line)
            let rim = !in_disk(wx + 1.0, wy, cx, cy, r, SQUASH) || !in_disk(wx - 1.0, wy, cx, cy, r, SQUASH) || !in_disk(wx, wy + 1.0, cx, cy, r, SQUASH) || !in_disk(wx, wy - 1.0, cx, cy, r, SQUASH);
            if age >= 3 {
                // broken into dashes along the arc
                let s = (u * arc_len / 4.0).floor() as i32;
                if s % 2 == 1 {
                    continue;
                }
            }
            let band = depth / thick; // 0 at the rim .. 1 at the inner edge
            // the innermost pixel row of a thick band is a dark matte contour, so the crescent's
            // shape reads against bright ground and stays separate from the hero
            if thick >= 3.0 && depth >= thick - 1.0 {
                f.put(sx, sy, FX_DEEP);
                continue;
            }
            let c = match age {
                0 => {
                    if rim {
                        if u > 0.35 { WHITE } else { BLADE_CORE }
                    } else if band < 0.3 && u > 0.4 {
                        BLADE_CORE
                    } else if u > 0.22 {
                        BLADE_MID
                    } else {
                        BLADE_EDGE
                    }
                }
                1 => {
                    if rim {
                        BLADE_CORE
                    } else if band < 0.4 && u > 0.3 {
                        BLADE_MID
                    } else {
                        BLADE_EDGE
                    }
                }
                2 => {
                    if rim || band < 0.3 {
                        BLADE_MID
                    } else {
                        FX_BLUE
                    }
                }
                _ => {
                    if rim {
                        BLADE_EDGE
                    } else {
                        FX_DEEP
                    }
                }
            };
            f.put_glow(sx, sy, c);
        }
    }
}

/// Kept for other modules: a slash arc between radii r0..r1 from `a0` sweeping `sweep`.
#[allow(clippy::too_many_arguments)]
pub fn draw_arc(f: &mut Frame, cx: f32, cy: f32, r0: f32, r1: f32, a0: f32, sweep: f32, fade: f32, heavy: bool) {
    if fade <= 0.0 {
        return;
    }
    let age = if fade > 0.8 {
        0
    } else if fade > 0.5 {
        1
    } else if fade > 0.25 {
        2
    } else {
        3
    };
    let w = ((r1 - r0) * if heavy { 0.4 } else { 0.32 }).max(1.0);
    draw_crescent(f, cx, cy, r1, a0, sweep, w, age);
}

/// A 1-pixel ellipse outline on the ground (clean 8-connected edge), optionally dashed.
#[allow(clippy::too_many_arguments)]
fn draw_ring(f: &mut Frame, cx: f32, cy: f32, rx: f32, ry: f32, c: Rgb, dashes: u32, phase: f32, open: bool) {
    if rx < 1.0 || ry < 0.5 {
        return;
    }
    let inside = |x: f32, y: f32| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0;
    let (x0, x1) = (f.sx(cx - rx - 1.0), f.sx(cx + rx + 1.0));
    let (y0, y1) = (f.sy(cy - ry - 1.0), f.sy(cy + ry + 1.0));
    for sy in y0..=y1 {
        for sx in x0..=x1 {
            let wx = (sx + f.cam_x) as f32 + 0.5;
            let wy = (sy + f.cam_y) as f32 + 0.5;
            if !inside(wx, wy) {
                continue;
            }
            if inside(wx + 1.0, wy) && inside(wx - 1.0, wy) && inside(wx, wy + 1.0) && inside(wx, wy - 1.0) {
                continue;
            }
            let angle = ((wy - cy) / ry).atan2((wx - cx) / rx) / TAU + 0.5;
            if open && (angle + phase * 0.2).rem_euclid(1.0) < 0.36 {
                continue;
            }
            if dashes > 0 {
                let a = angle + phase;
                if ((a * dashes as f32 * 2.0).floor() as i32).rem_euclid(2) == 1 {
                    continue;
                }
            }
            f.put_glow(sx, sy, c);
        }
    }
}

/// A four-point glint star at a screen pixel: size 1..3 (arm length).
fn draw_star(f: &mut Frame, sx: i32, sy: i32, size: i32) {
    f.put_glow(sx, sy, WHITE);
    let ramp = [BLADE_CORE, BLADE_MID, BLADE_EDGE];
    for i in 1..=size {
        let c = ramp[((i - 1) + (3 - size)).clamp(0, 2) as usize];
        f.put_glow(sx + i, sy, c);
        f.put_glow(sx - i, sy, c);
        f.put_glow(sx, sy + i, c);
        f.put_glow(sx, sy - i, c);
    }
}

/// A simple code-drawn Glint, used until the hero sprite sheet exists.
fn fallback_cell(dir: Dir) -> &'static Cell {
    static CELLS: OnceLock<[Cell; 3]> = OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let pal = [
            ('o', rgb(0x0a0a24)),
            ('h', rgb(0x2c3a9a)),
            ('H', rgb(0x4058cc)),
            ('k', rgb(0x05051a)),
            ('e', rgb(0x8ff8ff)),
            ('s', rgb(0x62c4f6)),
            ('b', rgb(0x3a2232)),
            ('g', rgb(0xd8fbff)),
        ];
        let down = Cell::from_ascii(
            &[
                "....oooo....",
                "...oHHHho...",
                "..oHhhhhho..",
                "..ohkkkkho..",
                "..ohkekeho..",
                "..ohkkkkho..",
                "..osssssso..",
                ".ohhsshhhho.",
                "gohhhshhhho.",
                "gohhhshhhho.",
                "..ohhhhhho..",
                "..obbo.obo..",
                "...oo...oo..",
            ],
            &pal,
            &['e', 'g'],
        );
        let up = Cell::from_ascii(
            &[
                "....oooo....",
                "...oHHHho...",
                "..oHhhhhho..",
                "..ohhhhhho..",
                "..ohhhhhho..",
                "..ohhhhhho..",
                "..osssssso..",
                ".ohhhhhhhho.",
                ".ohhhhhhhhog",
                ".ohhhhhhhhog",
                "..ohhhhhho..",
                "..obbo.obo..",
                "...oo...oo..",
            ],
            &pal,
            &['e', 'g'],
        );
        let right = Cell::from_ascii(
            &[
                "....oooo....",
                "...oHHHHo...",
                "..oHhhhhho..",
                "..ohhhkkko..",
                "..ohhhkkeo..",
                "..ohhhkkko..",
                "..osssssso..",
                ".ohhhhhhho..",
                ".ohhhhhhhogg",
                ".ohhhhhhho..",
                "..ohhhhho...",
                "..obo.obo...",
                "...o...oo...",
            ],
            &pal,
            &['e', 'g'],
        );
        [down, up, right]
    });
    match dir {
        Dir::Down => &cells[0],
        Dir::Up => &cells[1],
        _ => &cells[2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Raw;

    /// Advance the real player and effects for one tick in a collision-enabled zone.
    fn tick(p: &mut Player, zone: &Zone, fx: &mut Fx, input: &mut Input, raw: Raw) -> Vec<Hit> {
        input.step(raw);
        let mut hits = Vec::new();
        p.update(input, &mut PlayerCtx { zone, fx, hits: &mut hits });
        fx.tick();
        hits
    }

    /// A dash moves immediately, covers its tuned distance and protects its entire burst.
    #[test]
    fn dash_is_immediate_and_invulnerable() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        let mut p = Player::new(100.0, 100.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        let raw = Raw { mv: (1.0, 0.0), dash: true, ..Default::default() };
        tick(&mut p, &zone, &mut fx, &mut input, raw);
        assert!((p.body.x - 105.2).abs() < 0.001);
        assert_eq!(p.ghosts.len(), 1);
        let hit = Hit::new(Shape::Circle { x: p.body.x, y: p.body.y, r: 8.0 }, Team::Enemy, 1, fx.hit_id());
        assert!(!p.on_hit(&hit, &mut fx));
        for _ in 1..DASH_TICKS {
            tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
            assert!(!p.on_hit(&hit, &mut fx));
        }
        assert!((p.body.x - 100.0 - DASH_SPEED.iter().sum::<f32>()).abs() < 0.01);
        assert_eq!(p.state, State::Normal);
        assert!(p.body.iframes >= DASH_IFRAMES_EXTRA);
    }

    /// The fast burst must stop at a thin wall instead of tunneling through it.
    #[test]
    fn dash_respects_collision() {
        let mut zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        for y in 0..20 {
            zone.set_tile(7, y, Tile::Wall);
        }
        let mut p = Player::new(100.0, 100.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        for t in 0..DASH_TICKS {
            tick(&mut p, &zone, &mut fx, &mut input, Raw { mv: (1.0, 0.0), dash: t == 0, ..Default::default() });
            assert!(p.body.x + p.body.r <= 112.01);
        }
        assert!(p.body.x > 100.0);
    }

    /// Echoes keep their distance in every facing and expire soon after the dash ends.
    #[test]
    fn dash_echoes_leave_a_short_separated_tail() {
        let zone = Zone::new(ZoneId::Bloomwild, 40, 40);
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            let mut p = Player::new(300.0, 300.0);
            let mut fx = Fx::new(1);
            let mut input = Input::default();
            for t in 0..DASH_TICKS + DASH_GHOST_LIFE {
                tick(&mut p, &zone, &mut fx, &mut input, Raw { mv: dir.vec(), dash: t == 0, ..Default::default() });
                assert!(p.ghosts.len() <= 2);
                for pair in p.ghosts.windows(2) {
                    assert!((pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y) >= GHOST_SPACING);
                    assert!(pair[0].age > pair[1].age);
                }
            }
            assert!(p.ghosts.is_empty());
        }
        // A blocked dash must not accumulate overlapping samples at a wall.
        let mut zone = zone;
        for y in 0..zone.th {
            zone.set_tile(20, y, Tile::Wall);
        }
        let mut p = Player::new(315.0, 300.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        for t in 0..DASH_TICKS {
            tick(&mut p, &zone, &mut fx, &mut input, Raw { mv: (1.0, 0.0), dash: t == 0, ..Default::default() });
            assert!(p.ghosts.len() <= 1);
        }
    }

    /// Early taps chain three unique swings, with one hit id per active window.
    #[test]
    fn buffered_combo_has_three_weighted_hits() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        let mut p = Player::new(100.0, 100.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        let mut swings: Vec<(u32, i32, usize)> = Vec::new();
        for (raw, n) in crate::tools::parse_script("A1 W8 A1 W8 A1 W50") {
            for _ in 0..n {
                for hit in tick(&mut p, &zone, &mut fx, &mut input, raw) {
                    if let Some(s) = swings.last_mut().filter(|s| s.0 == hit.id) {
                        s.2 += 1;
                    } else {
                        swings.push((hit.id, hit.dmg, 1));
                    }
                }
            }
        }
        assert_eq!(swings.iter().map(|s| (s.1, s.2)).collect::<Vec<_>>(), [(1, 4), (1, 4), (2, 5)]);
        assert!(swings.windows(2).all(|w| w[0].0 != w[1].0));
        assert_eq!(p.state, State::Normal);
    }

    /// A brief movement cancel followed by attack must use the new facing immediately.
    #[test]
    fn recovery_cancel_keeps_the_new_aim() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        let mut p = Player::new(100.0, 100.0);
        p.face_to(1.0, 0.0);
        p.state = State::Attack { stage: 3, t: SWINGS[2].free_at };
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        tick(&mut p, &zone, &mut fx, &mut input, Raw { mv: (0.0, 1.0), ..Default::default() });
        assert_eq!(p.dir, Dir::Down);
        assert!(p.body.vy > 0.0);
        // The five-tick rest can buffer the next attack while preserving the turn.
        tick(&mut p, &zone, &mut fx, &mut input, Raw { attack: true, ..Default::default() });
        for _ in 0..COMBO_REST {
            tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
        }
        assert!(matches!(p.state, State::Attack { stage: 1, .. }));
        assert_eq!(p.body.face, (0.0, 1.0));
    }

    /// Holding builds one charge and release produces one protected spin, then recovery.
    #[test]
    fn charge_releases_a_single_protected_spin() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        let mut p = Player::new(100.0, 100.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        for _ in 0..60 {
            tick(&mut p, &zone, &mut fx, &mut input, Raw { attack: true, ..Default::default() });
        }
        assert!(matches!(p.state, State::Charge { t } if t >= CHARGE_FULL));
        assert!(p.full_t > 0);
        tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
        assert_eq!(p.state, State::Spin { t: 0 });
        let mut spin_hits = Vec::new();
        for _ in 0..SPIN_TOTAL + 5 {
            let hits = tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
            if !hits.is_empty() {
                assert!(p.body.iframes > 0);
                spin_hits.extend(hits);
            }
        }
        assert_eq!(spin_hits.len(), SPIN_ACTIVE as usize);
        assert!(spin_hits.iter().all(|h| h.dmg == 3 && h.id == spin_hits[0].id));
        assert_eq!(p.state, State::Normal);
    }

    /// A held press begun during scene hit-stop survives until player updates resume.
    #[test]
    fn recovers_a_press_held_through_hitstop() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        let mut p = Player::new(100.0, 100.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
        for _ in 0..5 {
            input.step(Raw { attack: true, ..Default::default() });
            fx.tick();
        }
        tick(&mut p, &zone, &mut fx, &mut input, Raw { attack: true, ..Default::default() });
        assert!(matches!(p.state, State::Attack { stage: 1, .. }));
    }

    /// The scene can preserve even a tap that starts and ends entirely during hit-stop.
    #[test]
    fn scene_can_buffer_a_short_tap_during_hitstop() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        let mut p = Player::new(100.0, 100.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
        for t in 0..6 {
            input.step(Raw { dash: t == 1, ..Default::default() });
            p.buffer_input(&input);
            fx.tick();
        }
        tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
        assert!(matches!(p.state, State::Dash { .. }));
    }

    /// Hurt grants grace, a buffered dash escapes recoil, and death returns to a fresh body.
    #[test]
    fn hurt_dash_escape_and_respawn() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        let mut p = Player::new(100.0, 100.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        let hit = Hit::new(Shape::Circle { x: 95.0, y: 100.0, r: 8.0 }, Team::Enemy, 1, fx.hit_id()).knock(3.0);
        assert!(p.on_hit(&hit, &mut fx));
        assert!(!p.on_hit(&hit, &mut fx));
        for t in 0..=HURT_DASH_AT {
            tick(&mut p, &zone, &mut fx, &mut input, Raw { mv: (1.0, 0.0), dash: t == 1, ..Default::default() });
        }
        assert!(matches!(p.state, State::Dash { .. }));
        assert_eq!((p.body.kx, p.body.ky), (0.0, 0.0));
        p.body.iframes = 0;
        let lethal = Hit::new(hit.shape, Team::Enemy, MAX_HP, fx.hit_id());
        assert!(p.on_hit(&lethal, &mut fx));
        assert!(!p.alive());
        for _ in 0..p.death_burst_at() + DEATH_LINGER + 2 {
            tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
        }
        assert!(p.wants_respawn);
        assert!(p.burst && p.scarf_free);
        p.respawn(120.0, 120.0);
        for _ in 0..60 {
            tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
        }
        assert_eq!(p.state, State::Normal);
        assert_eq!(p.body.hp, MAX_HP);
        assert!(!p.wants_respawn && !p.burst && !p.scarf_free);
    }

    /// Cloth stays continuous and finite through running, direction reversals and dashes.
    #[test]
    fn scarf_stays_bounded_through_turns() {
        let zone = Zone::new(ZoneId::Bloomwild, 40, 40);
        let mut p = Player::new(300.0, 300.0);
        let mut fx = Fx::new(1);
        let mut input = Input::default();
        for (raw, n) in crate::tools::parse_script("R30 RX1 R10 L30 LX1 L10 U30 UX1 D30 DX1 W120") {
            for _ in 0..n {
                tick(&mut p, &zone, &mut fx, &mut input, raw);
                for k in &p.scarf {
                    assert!(k.x.is_finite() && k.y.is_finite() && k.z.is_finite() && k.z >= 0.0);
                }
                for w in p.scarf.windows(2) {
                    let len = ((w[0].x - w[1].x).powi(2) + (w[0].y - w[1].y).powi(2) + (w[0].z - w[1].z).powi(2)).sqrt();
                    assert!(len < SCARF_SEG * 1.75, "stretched scarf: {len}");
                }
            }
        }
    }

    /// The combo follow-through kick moves the scarf but keeps it continuous and finite.
    #[test]
    fn scarf_follows_combos_and_stays_bounded() {
        let zone = Zone::new(ZoneId::Bloomwild, 40, 40);
        for dir in ["R", "L", "D", "U"] {
            let mut p = Player::new(300.0, 300.0);
            let mut fx = Fx::new(1);
            let mut input = Input::default();
            let mut whipped = false;
            for (raw, n) in crate::tools::parse_script(&format!("{dir}6 W30 A1 W9 A1 W9 A1 W60")) {
                for _ in 0..n {
                    tick(&mut p, &zone, &mut fx, &mut input, raw);
                    whipped |= p.scarf_whip() != 0.0;
                    for k in &p.scarf {
                        assert!(k.x.is_finite() && k.y.is_finite() && k.z.is_finite() && k.z >= 0.0);
                    }
                    for w in p.scarf.windows(2) {
                        let len = ((w[0].x - w[1].x).powi(2) + (w[0].y - w[1].y).powi(2) + (w[0].z - w[1].z).powi(2)).sqrt();
                        assert!(len < SCARF_SEG * 1.75, "{dir} stretched scarf: {len}");
                    }
                }
            }
            assert!(whipped, "{dir} combo never kicked the scarf");
            assert_eq!(p.scarf_whip(), 0.0);
        }
    }

    /// Every combo's smear turns with the authored blade, including mirrored left poses.
    #[test]
    fn combo_smears_follow_blade_handedness() {
        let mut fx = Fx::new(1);
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            for stage in 1..=3 {
                let mut p = Player::new(100.0, 100.0);
                let (x, y) = dir.vec();
                p.face_to(x, y);
                p.start_attack(stage, (0.0, 0.0), &mut fx);
                let sw = &SWINGS[(stage - 1) as usize];
                let first = if stage == 3 { 2 } else { 1 };
                let mut angles = Vec::new();
                for t in 0..sw.total {
                    p.state = State::Attack { stage, t };
                    let pose = p.pose().unwrap();
                    if pose.frame == first + angles.len() && angles.len() < 2 {
                        let (bx, by, bz) = p.blade_tip();
                        angles.push((by - bz - (p.body.y - 6.0)).atan2(bx - (p.body.x + x * 2.0)));
                    }
                }
                assert_eq!(angles.len(), 2);
                let delta = (angles[1] - angles[0] + PI).rem_euclid(TAU) - PI;
                assert_eq!(delta.signum(), p.swing_hand, "{dir:?} combo {stage}");
            }
        }
    }

    /// Every active combo tick meets its held blade tip, and recovery only erases the old trail.
    #[test]
    fn combo_smear_heads_follow_held_strike_frames() {
        let mut fx = Fx::new(1);
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            for stage in 1..=3 {
                let mut p = Player::new(100.0, 100.0);
                let (x, y) = dir.vec();
                p.face_to(x, y);
                p.start_attack(stage, (0.0, 0.0), &mut fx);
                let sw = &SWINGS[(stage - 1) as usize];
                let (cx, cy) = (p.body.x + x * 2.0, p.body.y - 6.0);
                for t in sw.windup + 1..=sw.windup + sw.active {
                    p.state = State::Attack { stage, t };
                    let (r, a) = p.combo_smear_head(stage, t);
                    let endpoint = (cx + r * a.cos(), cy + r * SMEAR_SQUASH * a.sin());
                    let (bx, by, bz) = p.blade_tip();
                    assert!(
                        (endpoint.0 - bx).hypot(endpoint.1 - (by - bz)) < 0.001,
                        "{dir:?} stage {stage} tick {t}"
                    );
                    let mut frame = Frame::new();
                    frame.begin(Rgb::BLACK, Rgb::WHITE);
                    p.draw_slash(&mut frame);
                    let nearest = frame
                        .emissive
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.max_c() > 0.1)
                        .map(|(i, _)| {
                            ((i % crate::gfx::W) as f32 + 0.5 - bx)
                                .hypot((i / crate::gfx::W) as f32 + 0.5 - (by - bz))
                        })
                        .fold(f32::INFINITY, f32::min);
                    assert!(
                        nearest < 2.0,
                        "raster missed {dir:?} stage {stage} tick {t}: {nearest}"
                    );
                }
                let last = p.combo_smear_head(stage, sw.windup + sw.active);
                for t in sw.windup + sw.active + 1..sw.windup + SMEAR_KEYS.len() as u32 + 1 {
                    p.state = State::Attack { stage, t };
                    assert_eq!(
                        p.combo_smear_head(stage, t),
                        last,
                        "recovery dragged the trail"
                    );
                }
            }
        }
    }

    /// The single spin sheet and its trail retain the same clockwise turn from every facing.
    #[test]
    fn spin_stays_unmirrored_in_every_facing() {
        let mut fx = Fx::new(1);
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            let mut p = Player::new(100.0, 100.0);
            let (x, y) = dir.vec();
            p.face_to(x, y);
            p.start_spin(&mut fx);
            assert_eq!(p.spin_dir, 1.0);
            for t in 0..SPIN_TOTAL {
                p.state = State::Spin { t };
                let pose = p.pose().unwrap();
                assert!(!pose.flip);
                let (dx, dy) = pose_point(&pose, "blade_tip").unwrap();
                let (bx, by, bz) = p.blade_tip();
                assert_eq!((bx - p.body.x, by - bz - p.body.y), (dx, dy));
            }
        }
    }

    /// A lethal hit displays every authored death frame before releasing the pixel burst.
    #[test]
    fn death_finishes_every_pose_before_bursting() {
        let zone = Zone::new(ZoneId::Bloomwild, 20, 20);
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            let mut p = Player::new(100.0, 100.0);
            let (x, y) = dir.vec();
            p.face_to(x, y);
            let mut fx = Fx::new(1);
            let mut input = Input::default();
            let hit = Hit::new(Shape::Circle { x: 100.0, y: 100.0, r: 8.0 }, Team::Enemy, MAX_HP, fx.hit_id());
            assert!(p.on_hit(&hit, &mut fx));
            let anim = p.pose().unwrap().anim.unwrap();
            let mut seen = vec![0; anim.frames];
            for _ in 0..p.death_burst_at() {
                assert!(!p.burst);
                seen[p.pose().unwrap().frame] += 1;
                tick(&mut p, &zone, &mut fx, &mut input, Raw::default());
            }
            assert!(seen.iter().zip(&anim.durations).all(|(shown, duration)| shown >= duration));
            assert!(p.burst && p.scarf_free);
        }
    }

    /// The short ribbon settles without folding back on itself after each facing's dash.
    #[test]
    fn scarf_settles_in_every_facing() {
        let zone = Zone::new(ZoneId::Bloomwild, 40, 40);
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            let mut p = Player::new(300.0, 300.0);
            let mut fx = Fx::new(1);
            let mut input = Input::default();
            for t in 0..180 {
                let raw = if t < 30 { Raw { mv: dir.vec(), dash: t == 20, ..Default::default() } } else { Raw::default() };
                tick(&mut p, &zone, &mut fx, &mut input, raw);
                if t > 140 {
                    let side = if dir == Dir::Right { -1.0 } else { 1.0 };
                    for w in p.scarf.windows(2) {
                        assert!((w[1].x - w[0].x) * side > -0.1, "{dir:?} scarf folded back");
                        assert!(w[1].y * 0.5 - w[1].z > w[0].y * 0.5 - w[0].z, "{dir:?} scarf curled upward");
                    }
                }
            }
        }
    }

    /// Daylight actions add no floor light, while the authored blade and eyes stay emissive.
    #[test]
    fn daylight_has_no_hero_floor_light() {
        let mut p = Player::new(192.0, 108.0);
        let mut f = Frame::new();
        for ambient in [Rgb::WHITE, Rgb::new(0.91, 0.90, 0.94)] {
            for state in [State::Normal, State::Attack { stage: 1, t: 5 }, State::Attack { stage: 3, t: 10 }, State::Charge { t: CHARGE_FULL }, State::Spin { t: 5 }, State::Respawn { t: 1 }, State::Dead { t: 40 }] {
                p.state = state;
                p.burst = matches!(state, State::Dead { .. });
                f.begin(Rgb::BLACK, ambient);
                // An existing regional light must survive the hero's daylight gate exactly.
                f.add_light_w(210.0, 100.0, 30.0, DANGER[1], 0.4);
                let regional = f.light.clone();
                p.draw_lights(&mut f);
                assert_eq!(f.light, regional, "daylight floor pool in {state:?}");
            }
        }
        p.state = State::Normal;
        p.burst = false;
        f.begin(Rgb::BLACK, Rgb::WHITE);
        p.draw(&mut f);
        assert!(f.emissive.iter().any(|c| c.max_c() > 0.5), "hero lost emissive pixels");
    }

    /// Unupdated headless heroes use the cave's current ambient and keep every light compact.
    #[test]
    fn cave_hero_light_is_local_and_uses_current_frame() {
        let mut p = Player::new(192.0, 108.0);
        let mut f = Frame::new();
        let ambient = rgb(0x9caeb9).scale(0.72);
        for state in [State::Normal, State::Attack { stage: 1, t: 5 }, State::Attack { stage: 3, t: 10 }, State::Charge { t: CHARGE_FULL }, State::Spin { t: 5 }, State::Respawn { t: 1 }, State::Dead { t: 40 }] {
            p.state = state;
            p.burst = matches!(state, State::Dead { .. });
            f.begin(Rgb::BLACK, ambient);
            p.draw_lights(&mut f);
            let mut affected = 0;
            for (i, c) in f.light.iter().enumerate() {
                let added = *c - ambient;
                if added.max_c() > 0.0001 {
                    affected += 1;
                    let dx = (i % crate::gfx::W) as f32 + 0.5 - p.body.x;
                    let dy = (i / crate::gfx::W) as f32 + 0.5 - p.body.y;
                    assert!(dx.hypot(dy) < 42.0, "light escaped the blade/action in {state:?}");
                    assert!(added.max_c() < 0.27, "light overpowered the cave in {state:?}");
                }
            }
            assert!(affected > 0 && affected < 2300, "unexpected light area {affected} in {state:?}");
            assert!(f.emissive.iter().all(|c| *c == Rgb::BLACK));
            // Render again immediately in daylight without an update or cached zone state.
            f.begin(Rgb::BLACK, Rgb::WHITE);
            p.draw_lights(&mut f);
            assert!(f.light.iter().all(|c| *c == Rgb::WHITE));
        }
    }

    /// Blade lights stay on the sprite's tip, including below-feet poses and mirrored art.
    #[test]
    fn blade_light_matches_live_sprite_anchors() {
        let mut p = Player::new(100.0, 100.0);
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            p.dir = dir;
            let pose = p.pose().expect("hero pose");
            let (dx, dy) = pose_point(&pose, "blade_tip").expect("hero blade_tip metadata");
            let (x, y, z) = p.blade_tip();
            assert!((x - (p.body.x + dx)).abs() < 0.001);
            assert!((y - z - (p.body.y + dy - p.body.z)).abs() < 0.001);
        }
    }

    /// The real blade lights remain visible in bright regions, with one local source instead
    /// of ambient-gated or stacked pools. The old renderer still uses its daylight gate above.
    #[test]
    fn physical_blade_light_survives_bright_ambient() {
        let mut p = Player::new(192.0, 108.0);
        let mut f = Frame::new();
        for ambient in [Rgb::WHITE, Rgb::new(0.91, 0.90, 0.94), rgb(0x9caeb9).scale(0.72)] {
            for state in [State::Normal, State::Attack { stage: 1, t: 5 }, State::Attack { stage: 3, t: 10 }, State::Charge { t: CHARGE_FULL }, State::Spin { t: 5 }, State::Respawn { t: 1 }] {
                p.state = state;
                f.begin(Rgb::BLACK, ambient);
                f.surface_lighting = true;
                p.draw_lights(&mut f);
                assert_eq!(f.point_lights.len(), 1, "missing or stacked blade sources in {state:?}");
                let light = f.point_lights[0];
                assert!((48.0..=64.0).contains(&light.radius));
                assert!((0.9..=1.5).contains(&light.intensity));
                assert_eq!(light.color, BLADE_MID);
                assert!(f.light.iter().all(|c| *c == ambient), "physical source wrote a radial pool");
            }
        }
    }

    /// Moving/turning/striking relocates the light to an actual emissive blade pixel, including
    /// sprite raster snapping and airborne poses. No fixed body or phantom overhead source.
    #[test]
    fn physical_blade_light_stays_on_live_emissive_art() {
        let mut p = Player::new(192.35, 108.7);
        p.body.z = 7.25;
        let mut sources = std::collections::HashSet::new();
        for dir in [Dir::Down, Dir::Up, Dir::Right, Dir::Left] {
            p.dir = dir;
            for moving in [false, true] {
                p.moving = moving;
                for t in 0..80 {
                    p.anim_t = t;
                    for state in [State::Normal, State::Attack { stage: 1, t }, State::Attack { stage: 2, t }, State::Attack { stage: 3, t }, State::Charge { t: CHARGE_FULL }, State::Spin { t }] {
                        p.state = state;
                        let (x, y, z) = p.blade_light_position().expect("live blade has emissive pixels");
                        let pose = p.pose().unwrap();
                        let dx = x.floor() as i32 - p.body.x.floor() as i32;
                        let px = pose.anchor.0 + if pose.flip { -dx } else { dx };
                        let py = (y - z).floor() as i32 - (p.body.y - p.body.z).floor() as i32 + pose.anchor.1;
                        let pixel = pose.cell.at(px, py);
                        assert!(pixel.a && pixel.emit, "source left the live blade in {dir:?} {state:?}");
                        assert!(z >= p.body.z, "source fell below the airborne blade");
                        sources.insert((x.to_bits(), (y - z).to_bits()));
                    }
                }
            }
        }
        assert!(sources.len() > 50, "the emitter did not follow authored blade motion");
    }

    #[test]
    fn blade_emitter_excludes_disconnected_eyes() {
        let cell = Cell::from_ascii(
            &["..e.e..", ".......", ".....b.", "....bb.", "...b..."],
            &[('e', BLADE_CORE), ('b', BLADE_MID)], &['e', 'b'],
        );
        assert_eq!(upper_blade_pixel(&cell, (3, 4)), Some((5, 2)));
        assert_eq!(upper_blade_pixel(&Cell::default(), (0, 0)), None);
    }
}
