//! Device-independent input for one 60 Hz tick. The windowed frontend fills it from keyboard,
//! mouse and gamepad; headless mode fills it from a script.

#[derive(Clone, Copy, Debug, Default)]
pub struct Btn {
    pub down: bool,
    /// Went down this tick.
    pub pressed: bool,
    /// Went up this tick.
    pub released: bool,
    /// Ticks held so far (0 when up).
    pub held: u32,
}

impl Btn {
    /// Advance one tick given whether the button is down now.
    pub fn step(&mut self, down: bool) {
        self.pressed = down && !self.down;
        self.released = !down && self.down;
        self.held = if down { self.held + 1 } else { 0 };
        self.down = down;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// Movement, each axis in [-1, 1], length <= 1. +y is down.
    pub mv: (f32, f32),
    /// Attack (tap = combo, hold = charge, release = spin slash).
    pub attack: Btn,
    /// Dash.
    pub dash: Btn,
    /// Confirm in menus.
    pub confirm: Btn,
    /// Pause / back.
    pub pause: Btn,
}

/// Raw device state sampled by a frontend for one tick.
#[derive(Clone, Copy, Debug, Default)]
pub struct Raw {
    pub mv: (f32, f32),
    pub attack: bool,
    pub dash: bool,
    pub confirm: bool,
    pub pause: bool,
}

impl Input {
    pub fn step(&mut self, raw: Raw) {
        let (mut x, mut y) = raw.mv;
        let l = (x * x + y * y).sqrt();
        if l > 1.0 {
            x /= l;
            y /= l;
        }
        self.mv = (x, y);
        self.attack.step(raw.attack);
        self.dash.step(raw.dash);
        self.confirm.step(raw.confirm);
        self.pause.step(raw.pause);
    }
    pub fn any_pressed(&self) -> bool {
        self.attack.pressed || self.dash.pressed || self.confirm.pressed
    }
}

/// Which keyboard layout the keys follow. Default: QWERTY (WASD move, J slash, K dash).
/// Colemak-DH is for keyboards that send Colemak-DH letters (a firmware layout). It keeps the
/// same physical keys as QWERTY: those keys send W/A/R/S to move, N to slash and E to dash.
/// Switch with `--colemak` or Tab on the title or pause screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyLayout {
    Qwerty,
    ColemakDh,
}

static LAYOUT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn layout() -> KeyLayout {
    if LAYOUT.load(std::sync::atomic::Ordering::Relaxed) == 1 { KeyLayout::ColemakDh } else { KeyLayout::Qwerty }
}

pub fn set_layout(l: KeyLayout) {
    LAYOUT.store(if l == KeyLayout::ColemakDh { 1 } else { 0 }, std::sync::atomic::Ordering::Relaxed);
}

/// Expose the game's actual layout to the browser controls. Tab changes this same state.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn glint_key_layout() -> u32 {
    match layout() {
        KeyLayout::Qwerty => 0,
        KeyLayout::ColemakDh => 1,
    }
}

/// Browser layout selection uses the same state as the native title/pause menu.
/// Ignore unknown values so a malformed page message cannot select another mapping.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn glint_set_key_layout(value: u32) {
    match value {
        0 => set_layout(KeyLayout::Qwerty),
        1 => set_layout(KeyLayout::ColemakDh),
        _ => {},
    }
}

/// The browser rejects an older cached engine instead of silently using old controls.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn glint_controls_revision() -> u32 {
    13
}

/// Key labels for the current layout: (move, slash, dash).
pub fn key_labels() -> (&'static str, &'static str, &'static str) {
    match layout() {
        KeyLayout::Qwerty => ("W A S D", "J", "K"),
        // What the same physical keys type on a Colemak-DH keyboard.
        KeyLayout::ColemakDh => ("W A R S", "N", "E"),
    }
}
