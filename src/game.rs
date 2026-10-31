//! Scene flow: title (with a short intro) -> zones in order -> victory -> back to the title,
//! with fades between them.
//! Both frontends (window and headless) drive a `Game` the same way:
//! `tick(&input)` at 60 Hz, `render()` once per displayed frame, then read `frame.out`/`frame.bloom`
//! and drain `fx.sounds` / read `fx.music`.

use crate::fx::Fx;
use crate::gfx::{Frame, Grade, post};
use crate::input::Input;
use crate::play::{Play, PlayOpts};
use crate::ui;
use crate::world::ZoneId;

pub struct Options {
    /// Skip the title and start in this zone.
    pub start: Option<ZoneId>,
    pub god: bool,
    pub peaceful: bool,
    pub seed: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options { start: None, god: false, peaceful: false, seed: 7 }
    }
}

pub enum Scene {
    /// The title screen plays over a live view of Bloomwild. `intro` counts ticks since start
    /// was pressed (two typed lines, then Glint dashes off and the game fades in).
    Title {
        t: u64,
        view: Box<Play>,
        intro: Option<u64>,
    },
    Play(Box<Play>),
    Victory {
        t: u64,
        view: Box<Play>,
    },
}

enum Next {
    Title,
    Zone(ZoneId),
    Victory,
}

pub struct Game {
    pub scene: Scene,
    pub fx: Fx,
    pub frame: Frame,
    pub t: u64,
    pub paused: bool,
    opts: Options,
    /// Fade transition: (ticks into it, what comes next). Fades out 40 ticks, swaps, fades in.
    transition: Option<(u32, Option<Next>)>,
    hud: ui::Hud,
    /// Ticks since pausing (pause screen animation).
    pause_t: u32,
    /// Ticks played in zones and falls across the whole run, for the victory screen.
    run_ticks: u64,
    falls: u32,
    /// Prism shards carried out of earlier zones (the HUD counter adds the current zone's).
    shards: u32,
}

const FADE: u32 = 40;
/// Title camera: the wordmark and intro panel sit on open sky, the tree and ruin stand at the right edge.
const TITLE_CENTER: (f32, f32) = (18.0, 48.0);

/// Snap a 0..1 fade to even steps, like a palette fade, instead of a smooth blend.
fn stepped(x: f32) -> f32 {
    (x * 6.0).round() / 6.0
}

/// Build a peaceful, living Bloomwild backdrop without a second controllable hero.
fn title_scene() -> Scene {
    let mut view = Box::new(Play::new(ZoneId::Bloomwild, &PlayOpts { god: true, peaceful: true }));
    view.arrive_t = 10_000;
    view.hide_player = true;
    view.free_camera = true;
    view.cam.snap(TITLE_CENTER.0, TITLE_CENTER.1);
    // Keep the title's T/B gap quiet while retaining the urn in the playable meadow.
    view.zone.props.retain(|p| !(p.anim == "urn_small" && (p.x - 184.0).abs() < 1.0 && (p.y - 648.0).abs() < 1.0));
    view.zone.invalidate_lighting();
    Scene::Title { t: 0, view, intro: None }
}

impl Game {
    pub fn new(opts: Options) -> Game {
        let fx = Fx::new(opts.seed);
        let po = PlayOpts { god: opts.god, peaceful: opts.peaceful };
        let (scene, transition) = match opts.start {
            Some(z) => (Scene::Play(Box::new(Play::new(z, &po))), None),
            // the title fades in from black
            None => (title_scene(), Some((FADE, None))),
        };
        Game { scene, fx, frame: Frame::new(), t: 0, paused: false, opts, transition, hud: ui::Hud::new(), pause_t: 0, run_ticks: 0, falls: 0, shards: 0 }
    }

    fn play_opts(&self) -> PlayOpts {
        PlayOpts { god: self.opts.god, peaceful: self.opts.peaceful }
    }

    fn start(&mut self, next: Next) {
        if self.transition.is_none() {
            self.transition = Some((0, Some(next)));
        }
    }

    /// Tab may switch the key layout on the title screen or while paused.
    pub fn layout_switch_allowed(&self) -> bool {
        self.paused || matches!(self.scene, Scene::Title { .. })
    }

    pub fn tick(&mut self, input: &Input) {
        self.t += 1;
        // transitions
        let po = self.play_opts();
        if let Some((n, next)) = self.transition.as_mut() {
            *n += 1;
            if *n == FADE {
                match next.take() {
                    Some(Next::Zone(z)) => {
                        let hp = if let Scene::Play(p) = &self.scene {
                            self.falls += p.deaths;
                            self.shards += p.shard_taken as u32;
                            Some(p.player.body.hp)
                        } else {
                            // a new run from the title
                            self.run_ticks = 0;
                            self.falls = 0;
                            self.shards = 0;
                            None
                        };
                        let mut play = Box::new(Play::new(z, &po));
                        if let Some(hp) = hp {
                            play.player.body.hp = hp.max(3);
                        }
                        self.fx.particles.clear();
                        self.hud = ui::Hud::new();
                        self.fx.music(&play.zone.music);
                        self.scene = Scene::Play(play);
                    }
                    Some(Next::Victory) => {
                        if let Scene::Play(mut p) = std::mem::replace(&mut self.scene, title_scene()) {
                            self.falls += p.deaths;
                            // Compose Glint beside the credits while the transition is fully black.
                            p.cam.x = p.player.body.x + 132.0;
                            self.scene = Scene::Victory { t: 0, view: p };
                        }
                    }
                    Some(Next::Title) => {
                        self.fx.particles.clear();
                        self.scene = title_scene();
                    }
                    None => {}
                }
            }
            if *n >= FADE * 2 {
                self.transition = None;
            }
        }
        let was_paused = self.paused;
        if (input.pause.pressed || (self.paused && input.confirm.pressed)) && matches!(self.scene, Scene::Play(_)) && self.transition.is_none() {
            self.paused = !self.paused;
            self.pause_t = 0;
            self.fx.sound(if self.paused { "ui_select" } else { "ui_back" });
        }
        if self.paused {
            self.pause_t += 1;
            return;
        }
        if was_paused {
            // Consume the resume button so the same gamepad press cannot also slash.
            self.fx.tick();
            return;
        }
        let transitioning = self.transition.is_some();
        let mut next = None;
        match &mut self.scene {
            Scene::Title { t, view, intro } => {
                *t += 1;
                self.fx.music("title");
                // A slow return path eases at both ends instead of snapping direction.
                // North-rim postcard: the word on open cloud, the basin and its trees below.
                let span = 8.0;
                let drift = (1.0 - (*t as f32 * 0.0007).cos()) * 0.5;
                view.cam.x = TITLE_CENTER.0 + span * drift;
                view.cam.y = TITLE_CENTER.1;
                view.update_ambient(&mut self.fx);
                let pressed = input.confirm.pressed || input.attack.pressed;
                match intro {
                    None => {
                        if pressed && *t > 30 && self.transition.is_none() {
                            self.fx.sound("ui_select");
                            *intro = Some(0);
                        }
                    }
                    Some(i) => {
                        *i += 1;
                        // a second press skips the lines and goes straight to Glint's dash
                        if pressed && *i > 12 && *i + 12 < ui::intro_dash() {
                            *i = ui::intro_dash() - 12;
                        }
                        if ui::intro_blip(*i) {
                            self.fx.sound_var("text_blip", 3);
                        }
                        if *i == ui::intro_dash() {
                            self.fx.sound("dash");
                        }
                        if *i == ui::intro_end() {
                            next = Some(Next::Zone(ZoneId::Bloomwild));
                        }
                    }
                }
            }
            Scene::Play(p) => {
                if transitioning {
                    // Keep the environment alive while protecting an unseen player.
                    p.update_ambient(&mut self.fx);
                } else {
                    p.update(input, &mut self.fx);
                    self.run_ticks += 1;
                }
                self.hud.update(p, self.shards + p.shard_taken as u32, &mut self.fx);
                if !transitioning {
                    if p.victory {
                        next = Some(Next::Victory);
                    } else if p.exit {
                        if let Some(z) = p.id.next() {
                            next = Some(Next::Zone(z));
                        }
                    }
                }
            }
            Scene::Victory { t, view } => {
                *t += 1;
                view.update_ambient(&mut self.fx);
                self.fx.music("victory");
                if *t == ui::VICTORY_MEET {
                    self.fx.sound("shard_pickup");
                    self.fx.flash(0.3);
                }
                if *t >= ui::VICTORY_PROMPT && (input.confirm.pressed || input.attack.pressed) && self.transition.is_none() {
                    self.fx.sound("ui_select");
                    next = Some(Next::Title);
                }
            }
        }
        if let Some(n) = next {
            self.start(n);
        }
        self.fx.tick();
    }

    pub fn render(&mut self) {
        let f = &mut self.frame;
        let grade: Grade;
        match &mut self.scene {
            Scene::Title { view, .. } | Scene::Victory { view, .. } => {
                view.add_lights(&mut self.fx);
                view.render(f, &self.fx);
                grade = view.zone.grade;
            }
            Scene::Play(p) => {
                p.add_lights(&mut self.fx);
                p.render(f, &self.fx);
                grade = p.zone.grade;
            }
        }
        // Finish the world first: crystal light must never shine through opaque UI.
        f.fade = 0.0;
        f.flash = 0.0;
        post::finish(f, &grade);
        match &self.scene {
            Scene::Title { t, intro, .. } => ui::title(f, *t, *intro),
            Scene::Play(p) if self.paused => ui::paused(f, self.pause_t, p),
            Scene::Play(p) => ui::hud(f, p, &self.hud),
            Scene::Victory { t, .. } => ui::victory(f, *t, self.run_ticks, self.falls),
        }
        f.fade = match self.transition {
            Some((n, _)) if n < FADE => stepped(n as f32 / FADE as f32),
            Some((n, _)) => stepped(1.0 - (n - FADE) as f32 / FADE as f32),
            None => 0.0,
        };
        f.flash = self.fx.flash;
        // Fade the finished world and its crisp overlay together, after all glow.
        let keep = 1.0 - f.fade;
        let flash = f.flash.clamp(0.0, 1.0);
        if keep < 1.0 || flash > 0.0 {
            for p in &mut f.out {
                for c in p.iter_mut().take(3) {
                    let v = *c as f32 * keep;
                    *c = (v + (255.0 - v) * flash) as u8;
                }
            }
        }
        self.fx.lights.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Advance the real game loop without input or queued audio.
    fn wait(game: &mut Game, ticks: u32) {
        for _ in 0..ticks {
            game.tick(&Input::default());
            game.fx.sounds.clear();
        }
    }

    /// Start at the postcard before the first tick and retain its full eight-pixel drift.
    #[test]
    fn title_camera_is_snapped_and_unbounded() {
        let mut game = Game::new(Options::default());
        for tick in [0, 1, 180, 600, 4488] {
            let ticks = tick - game.t as u32;
            wait(&mut game, ticks);
            let Scene::Title { view, .. } = &game.scene else { panic!("expected title") };
            let drift = (1.0 - (tick as f32 * 0.0007).cos()) * 0.5;
            assert!(view.free_camera);
            assert_eq!((view.cam.x, view.cam.y), (TITLE_CENTER.0 + 8.0 * drift, TITLE_CENTER.1));
            let expected = view.cam.view_free();
            assert_eq!(expected.1, -60);
            game.render();
            assert_eq!((game.frame.cam_x, game.frame.cam_y), expected);
        }
    }

    /// The supplied title script reaches a fully visible, controllable Bloomwild.
    #[test]
    fn title_start_completes_within_capture_script() {
        let mut game = Game::new(Options::default());
        wait(&mut game, 120);
        let mut input = Input::default();
        input.confirm.step(true);
        game.tick(&input);
        wait(&mut game, 200);
        assert!(matches!(&game.scene, Scene::Play(p) if p.id == ZoneId::Bloomwild));
        assert!(game.transition.is_none(), "intro ends at {}, transition at {:?}", ui::intro_end(), game.transition.as_ref().map(|v| v.0));
        game.render();
        assert_eq!(game.frame.fade, 0.0);
        assert!(!game.paused);
        assert!(matches!(&game.scene, Scene::Play(p) if !p.free_camera));
    }

    /// Pausing freezes gameplay and Enter resumes without advancing a hidden simulation.
    #[test]
    fn pause_freezes_player_and_run_clock() {
        let mut game = Game::new(Options { start: Some(ZoneId::Bloomwild), ..Options::default() });
        wait(&mut game, 5);
        let before = game.run_ticks;
        let arrive = match &game.scene {
            Scene::Play(p) => p.arrive_t,
            _ => unreachable!(),
        };
        let mut input = Input::default();
        input.pause.step(true);
        game.tick(&input);
        wait(&mut game, 90);
        assert_eq!(game.run_ticks, before);
        assert!(matches!(&game.scene, Scene::Play(p) if p.arrive_t == arrive));
        game.render();
        let mut input = Input::default();
        input.confirm.step(true);
        game.tick(&input);
        assert!(!game.paused);
        assert_eq!(game.run_ticks, before);
        wait(&mut game, 1);
        assert_eq!(game.run_ticks, before + 1);
    }

    /// Gates preserve run progress through all regions; victory wins over a stale exit flag.
    #[test]
    fn region_sequence_and_victory_return_preserve_then_reset_run() {
        let mut game = Game::new(Options { start: Some(ZoneId::Bloomwild), ..Options::default() });
        for (count, destination) in [ZoneId::Glasswater, ZoneId::Cinder, ZoneId::Summit].into_iter().enumerate() {
            if let Scene::Play(p) = &mut game.scene {
                p.shard_taken = true;
                p.deaths = 1;
                p.player.body.hp = 5;
                // Gate detection belongs to Play; this exercises the flow after it fires.
                p.exit = true;
            }
            game.tick(&Input::default());
            let before = game.run_ticks;
            wait(&mut game, FADE);
            assert!(matches!(&game.scene, Scene::Play(p) if p.id == destination && p.arrive_t == 0 && p.player.body.hp == 5));
            assert_eq!(game.run_ticks, before);
            assert_eq!(game.shards, count as u32 + 1);
            assert_eq!(game.falls, count as u32 + 1);
            game.render();
            assert!(game.frame.out.iter().all(|p| p[..3] == [0, 0, 0]));
            wait(&mut game, FADE);
            assert!(game.transition.is_none());
        }
        if let Scene::Play(p) = &mut game.scene {
            p.victory = true;
            p.exit = true;
        }
        game.tick(&Input::default());
        wait(&mut game, FADE * 2 + ui::VICTORY_PROMPT as u32);
        assert!(matches!(game.scene, Scene::Victory { .. }));
        if let Scene::Victory { view, .. } = &game.scene {
            assert!(!view.free_camera, "victory retains the bounded gameplay camera");
            assert_eq!(view.cam.x, view.player.body.x + 132.0);
        }
        game.render();
        let mut input = Input::default();
        input.confirm.step(true);
        game.tick(&input);
        wait(&mut game, FADE * 2);
        assert!(matches!(game.scene, Scene::Title { intro: None, .. }));
        game.tick(&input);
        wait(&mut game, 200);
        assert!(matches!(&game.scene, Scene::Play(p) if p.id == ZoneId::Bloomwild));
        assert_eq!(game.shards, 0);
        assert_eq!(game.falls, 0);
    }
}
