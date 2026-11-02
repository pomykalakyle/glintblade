//! The windowed frontend: macroquad window, keyboard + gamepad input, fixed 60 Hz ticks,
//! texture upload with a bloom composite shader, and audio (sfx + crossfading music).

use crate::game::{Game, Options};
use crate::gfx::{BH, BW, H, W};
use crate::input::{Input, Raw};
use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound};
use macroquad::prelude::*;
use std::collections::HashMap;

pub fn conf() -> Conf {
    Conf {
        window_title: "GLINTBLADE".to_string(),
        window_width: 1536,
        window_height: 864,
        high_dpi: true,
        window_resizable: true,
        sample_count: 1,
        ..Default::default()
    }
}

const VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying highp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
}
"#;

const FS: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
uniform sampler2D Texture;
uniform sampler2D Bloom;
void main() {
    vec3 base = texture2D(Texture, uv).rgb;
    vec3 glow = texture2D(Bloom, uv).rgb;
    // screen-style add keeps highlights from clipping harshly
    vec3 c = base + glow * (1.0 - base * 0.5);
    gl_FragColor = vec4(c, 1.0);
}
"#;

fn bytes(v: &[[u8; 4]]) -> &[u8] {
    // [u8; 4] has no padding, so this view is exact.
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

struct Music {
    cache: HashMap<String, Sound>,
    cur: Option<(String, Sound, f32)>,
    old: Vec<(Sound, f32)>,
}

impl Music {
    async fn update(&mut self, want: Option<&str>, volume: f32) {
        if let Some(w) = want
            && self.cur.as_ref().map(|c| c.0.as_str()) != Some(w)
        {
            if let Some((_, s, v)) = self.cur.take() {
                self.old.push((s, v));
            }
            let path = format!("music/{w}.ogg");
            if !self.cache.contains_key(w)
                && let Some(b) = crate::assets::get(&path)
                && let Ok(s) = load_sound_from_bytes(b).await
            {
                self.cache.insert(w.to_string(), s);
            }
            if let Some(s) = self.cache.get(w) {
                play_sound(s, PlaySoundParams { looped: w != "victory", volume: 0.0 });
                self.cur = Some((w.to_string(), s.clone(), 0.0));
            }
        }
        if let Some((_, s, v)) = self.cur.as_mut() {
            *v = (*v + 0.008).min(volume);
            set_sound_volume(s, *v);
        }
        for (s, v) in self.old.iter_mut() {
            *v -= 0.008;
            set_sound_volume(s, v.max(0.0));
            if *v <= 0.0 {
                stop_sound(s);
            }
        }
        self.old.retain(|(_, v)| *v > 0.0);
    }
}

/// Press edges survive rendered frames that do not advance the 60 Hz simulation.
/// Held state stays separate so a short tap never becomes a charge during catch-up ticks.
#[derive(Clone, Copy, Debug, Default)]
struct ActionPresses {
    attack: bool,
    dash: bool,
    confirm: bool,
    pause: bool,
}

impl ActionPresses {
    fn merge(&mut self, pressed: Self) {
        self.attack |= pressed.attack;
        self.dash |= pressed.dash;
        self.confirm |= pressed.confirm;
        self.pause |= pressed.pause;
    }

    fn for_tick(&mut self, mut held: Raw) -> Raw {
        let pressed = std::mem::take(self);
        held.attack |= pressed.attack;
        held.dash |= pressed.dash;
        held.confirm |= pressed.confirm;
        held.pause |= pressed.pause;
        held
    }
}

// The gamepad block that updates these is compiled out of the browser build.
#[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
fn sample_input(gil: &mut Option<Gamepads>, pad: &mut PadState) -> (Raw, ActionPresses) {
    let mut mx = 0.0f32;
    let mut my = 0.0f32;
    // Default QWERTY: WASD move, J slash, K dash. Colemak-DH mode is for keyboards that
    // send Colemak-DH letters (a firmware layout): it keeps the same physical keys, which
    // send W A R S for WASD, N for J and E for K.
    let colemak = crate::input::layout() == crate::input::KeyLayout::ColemakDh;
    let (kl, kd, ku, kr, ka, kdash) = if colemak {
        (KeyCode::A, KeyCode::R, KeyCode::W, KeyCode::S, KeyCode::N, KeyCode::E)
    } else {
        (KeyCode::A, KeyCode::S, KeyCode::W, KeyCode::D, KeyCode::J, KeyCode::K)
    };
    if is_key_down(KeyCode::Left) || is_key_down(kl) {
        mx -= 1.0;
    }
    if is_key_down(KeyCode::Right) || is_key_down(kr) {
        mx += 1.0;
    }
    if is_key_down(KeyCode::Up) || is_key_down(ku) {
        my -= 1.0;
    }
    if is_key_down(KeyCode::Down) || is_key_down(kd) {
        my += 1.0;
    }
    // Only the selected layout supplies the primary action letters. X/Z and C
    // remain aliases for both layouts and for the browser's touch buttons.
    let attack_keys = [ka, KeyCode::X, KeyCode::Z];
    let dash_keys = [kdash, KeyCode::Space, KeyCode::LeftShift, KeyCode::C];
    let confirm_keys = [KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space];
    // The physical P key sends ";" on Colemak-DH.
    let kpause = if colemak { KeyCode::Semicolon } else { KeyCode::P };
    let pause_keys = [KeyCode::Escape, kpause];
    let mut attack = attack_keys.into_iter().any(is_key_down) || is_mouse_button_down(MouseButton::Left);
    let mut dash = dash_keys.into_iter().any(is_key_down) || is_mouse_button_down(MouseButton::Right);
    let mut confirm = confirm_keys.into_iter().any(is_key_down);
    let mut pause = pause_keys.into_iter().any(is_key_down);
    // Macroquad retains a press even when keydown and keyup arrive between
    // renders. is_key_down alone would silently drop these short taps.
    let pressed = ActionPresses {
        attack: attack_keys.into_iter().any(is_key_pressed) || is_mouse_button_pressed(MouseButton::Left),
        dash: dash_keys.into_iter().any(is_key_pressed) || is_mouse_button_pressed(MouseButton::Right),
        confirm: confirm_keys.into_iter().any(is_key_pressed),
        pause: pause_keys.into_iter().any(is_key_pressed),
    };

    #[cfg(target_arch = "wasm32")]
    let _ = gil;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(g) = gil.as_mut() {
        while let Some(ev) = g.next_event() {
            let _ = ev;
        }
        for (_, gp) in g.gamepads() {
            use gilrs::{Axis, Button};
            let ax = gp.value(Axis::LeftStickX);
            let ay = -gp.value(Axis::LeftStickY);
            if ax.abs() > 0.25 {
                mx += ax;
            }
            if ay.abs() > 0.25 {
                my += ay;
            }
            if gp.is_pressed(Button::DPadLeft) {
                mx -= 1.0;
            }
            if gp.is_pressed(Button::DPadRight) {
                mx += 1.0;
            }
            if gp.is_pressed(Button::DPadUp) {
                my -= 1.0;
            }
            if gp.is_pressed(Button::DPadDown) {
                my += 1.0;
            }
            attack |= gp.is_pressed(Button::South) || gp.is_pressed(Button::West);
            dash |= gp.is_pressed(Button::East) || gp.is_pressed(Button::RightTrigger) || gp.is_pressed(Button::RightTrigger2);
            confirm |= gp.is_pressed(Button::South) || gp.is_pressed(Button::Start);
            pause |= gp.is_pressed(Button::Start);
        }
    }
    let _ = pad;
    (Raw { mv: (mx.clamp(-1.0, 1.0), my.clamp(-1.0, 1.0)), attack, dash, confirm, pause }, pressed)
}

#[derive(Default)]
struct PadState;

#[cfg(not(target_arch = "wasm32"))]
type Gamepads = gilrs::Gilrs;
// The browser build has no gamepad support.
#[cfg(target_arch = "wasm32")]
struct Gamepads;

fn open_gamepads() -> Option<Gamepads> {
    #[cfg(not(target_arch = "wasm32"))]
    return gilrs::Gilrs::new().ok();
    #[cfg(target_arch = "wasm32")]
    return None;
}

pub async fn run(opts: Options) {
    // A new run always starts with WASD unless Colemak was explicitly requested.
    crate::input::set_layout(if std::env::args().any(|a| a == "--colemak") || std::env::var("GLINT_KEYS").is_ok_and(|v| v.eq_ignore_ascii_case("colemak")) {
        crate::input::KeyLayout::ColemakDh
    } else {
        crate::input::KeyLayout::Qwerty
    });
    // Sound is on by default; `--mute` (or GLINT_MUTE=1) silences all audio, e.g. for test runs.
    let mute = std::env::args().any(|a| a == "--mute") || std::env::var("GLINT_MUTE").is_ok_and(|v| !v.is_empty() && v != "0");
    let mut game = Game::new(opts);
    let base = Texture2D::from_rgba8(W as u16, H as u16, &vec![0u8; W * H * 4]);
    base.set_filter(FilterMode::Nearest);
    let bloom = Texture2D::from_rgba8(BW as u16, BH as u16, &vec![0u8; BW * BH * 4]);
    bloom.set_filter(FilterMode::Linear);
    let material = load_material(
        ShaderSource::Glsl { vertex: VS, fragment: FS },
        MaterialParams { textures: vec!["Bloom".to_string()], ..Default::default() },
    )
    .expect("shader");

    let mut sfx: HashMap<String, Sound> = HashMap::new();
    for name in crate::assets::list("sfx/").filter(|n| n.ends_with(".wav")).collect::<Vec<_>>() {
        if let Some(b) = crate::assets::get(name)
            && let Ok(s) = load_sound_from_bytes(b).await
        {
            let stem = name.trim_start_matches("sfx/").trim_end_matches(".wav").to_string();
            sfx.insert(stem, s);
        }
    }
    let mut last_played: HashMap<String, f64> = HashMap::new();
    let mut music = Music { cache: HashMap::new(), cur: None, old: Vec::new() };
    let mut gil = open_gamepads();
    let mut pad = PadState;
    let mut input = Input::default();
    let mut pending_presses = ActionPresses::default();
    let mut pending_layout = crate::input::layout();
    let mut acc = 0.0f64;
    let mut last = get_time();
    let mut fullscreen = false;

    loop {
        if is_key_pressed(KeyCode::Tab) && game.layout_switch_allowed() {
            use crate::input::{KeyLayout, layout, set_layout};
            set_layout(if layout() == KeyLayout::Qwerty { KeyLayout::ColemakDh } else { KeyLayout::Qwerty });
        }
        if crate::input::layout() != pending_layout {
            // A browser selection can change the layout between frames too.
            // Do not deliver an unconsumed press from the previous mapping.
            pending_presses = ActionPresses::default();
            pending_layout = crate::input::layout();
        }
        // The physical F key sends T on Colemak-DH.
        let kfull = if crate::input::layout() == crate::input::KeyLayout::ColemakDh { KeyCode::T } else { KeyCode::F };
        if is_key_pressed(kfull) || (is_key_down(KeyCode::LeftSuper) && is_key_pressed(KeyCode::Enter)) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if is_key_down(KeyCode::LeftSuper) && is_key_pressed(KeyCode::Q) {
            break;
        }
        let now = get_time();
        acc += (now - last).min(0.25);
        last = now;
        let (raw, pressed) = sample_input(&mut gil, &mut pad);
        pending_presses.merge(pressed);
        let mut n = 0;
        while acc >= 1.0 / 60.0 && n < 4 {
            input.step(pending_presses.for_tick(raw));
            game.tick(&input);
            acc -= 1.0 / 60.0;
            n += 1;
        }
        if n == 4 {
            acc = 0.0;
        }
        game.render();
        base.update_from_bytes(W as u32, H as u32, bytes(&game.frame.out));
        bloom.update_from_bytes(BW as u32, BH as u32, bytes(&game.frame.bloom));

        clear_background(Color::from_rgba(4, 3, 10, 255));
        let (sw, sh) = (screen_width(), screen_height());
        let scale = (sw / W as f32).min(sh / H as f32);
        // Browser cards rarely match an integer multiple of the native picture.
        // Fit their full width/height; nearest filtering keeps the pixels sharp.
        #[cfg(not(target_arch = "wasm32"))]
        let scale = if scale >= 1.0 { scale.floor() } else { scale };
        let (dw, dh) = (W as f32 * scale, H as f32 * scale);
        let (dx, dy) = (((sw - dw) / 2.0).floor(), ((sh - dh) / 2.0).floor());
        material.set_texture("Bloom", bloom.clone());
        gl_use_material(&material);
        draw_texture_ex(&base, dx, dy, WHITE, DrawTextureParams { dest_size: Some(vec2(dw, dh)), ..Default::default() });
        gl_use_default_material();

        // audio
        if mute {
            game.fx.sounds.clear();
        }
        for cmd in game.fx.sounds.drain(..) {
            let mut name = cmd.name.clone();
            if !sfx.contains_key(&name) {
                // "slash_2" -> "slash", "slash" -> "slash_1"
                if let Some((stem, tail)) = name.rsplit_once('_')
                    && tail.chars().all(|c| c.is_ascii_digit())
                    && sfx.contains_key(stem)
                {
                    name = stem.to_string();
                } else if sfx.contains_key(&format!("{name}_1")) {
                    name = format!("{name}_1");
                } else {
                    continue;
                }
            }
            let t = get_time();
            if let Some(prev) = last_played.get(&name)
                && t - prev < 0.035
            {
                continue;
            }
            last_played.insert(name.clone(), t);
            if let Some(s) = sfx.get(&name) {
                play_sound(s, PlaySoundParams { looped: false, volume: (cmd.volume * 0.8).clamp(0.0, 1.0) });
            }
        }
        let want = if mute { None } else { game.fx.music.clone() };
        // An empty request fades to silence; an unavailable named cue keeps the current track.
        if want.as_deref().map(|w| w.is_empty() || crate::assets::get(&format!("music/{w}.ogg")).is_some()).unwrap_or(false) {
            music.update(want.as_deref(), 0.55).await;
        } else {
            music.update(None, 0.55).await;
        }

        next_frame().await
    }
}

#[cfg(test)]
mod tests {
    use super::ActionPresses;
    use crate::input::{Input, Raw};

    #[test]
    fn short_action_presses_survive_renders_without_simulation_ticks() {
        let mut pending = ActionPresses::default();
        // The key has already been released when the frontend samples it.
        pending.merge(ActionPresses { attack: true, dash: true, confirm: true, pause: true });
        // A second rendered frame clears Macroquad's press flags, but neither
        // rendered frame has advanced the simulation yet.
        pending.merge(ActionPresses::default());
        let mut input = Input::default();
        input.step(pending.for_tick(Raw::default()));
        assert!(input.attack.pressed && input.dash.pressed && input.confirm.pressed && input.pause.pressed);
        assert_eq!(input.attack.held, 1);
    }

    #[test]
    fn a_tap_is_consumed_once_and_released_on_the_next_catch_up_tick() {
        let mut pending = ActionPresses { attack: true, dash: true, ..Default::default() };
        let mut input = Input::default();
        input.step(pending.for_tick(Raw::default()));
        assert!(input.attack.pressed && input.dash.pressed);
        input.step(pending.for_tick(Raw::default()));
        assert!(input.attack.released && input.dash.released);
        assert!(!input.attack.down && !input.dash.down);
        for _ in 0..2 {
            input.step(pending.for_tick(Raw::default()));
            assert!(!input.attack.pressed && !input.dash.pressed);
            assert!(!input.attack.released && !input.dash.released);
        }
    }

    #[test]
    fn held_actions_keep_charge_and_release_semantics() {
        let mut pending = ActionPresses { attack: true, ..Default::default() };
        let mut input = Input::default();
        let held = Raw { attack: true, ..Default::default() };
        input.step(pending.for_tick(held));
        assert!(input.attack.pressed);
        for expected_held in 2..=40 {
            input.step(pending.for_tick(held));
            assert!(input.attack.down);
            assert!(!input.attack.pressed);
            assert_eq!(input.attack.held, expected_held);
        }
        input.step(pending.for_tick(Raw::default()));
        assert!(input.attack.released);
        assert_eq!(input.attack.held, 0);
    }

    #[test]
    fn pending_actions_never_change_or_retain_movement() {
        let mut pending = ActionPresses { attack: true, ..Default::default() };
        let first = pending.for_tick(Raw { mv: (-0.4, 0.7), ..Default::default() });
        assert_eq!(first.mv, (-0.4, 0.7));
        assert!(first.attack);
        let second = pending.for_tick(Raw { mv: (0.2, -0.8), dash: true, ..Default::default() });
        assert_eq!(second.mv, (0.2, -0.8));
        assert!(!second.attack);
        assert!(second.dash);
        assert_eq!(pending.for_tick(Raw::default()).mv, (0.0, 0.0));
    }
}
