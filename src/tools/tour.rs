//! Headless visual-QA pilot. Uses ordinary movement and sword inputs, never warps or damage.
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::game::{Game, Scene};
use crate::input::Raw;
use crate::world::{Zone, ZoneId};

const STEP: i32 = 4;

#[derive(Default)]
pub(super) struct Tour {
    zone: Option<ZoneId>,
    path: Vec<(f32, f32)>,
    goal: Option<(f32, f32)>,
    refresh: u32,
    waypoint: usize,
    pub visited: Vec<String>,
}

impl Tour {
    pub fn input(&mut self, game: &Game, tick: u32) -> Raw {
        let Scene::Play(p) = &game.scene else { return Raw::default() };
        if self.zone != Some(p.id) {
            self.zone = Some(p.id);
            self.path.clear();
            self.goal = None;
            self.waypoint = 0;
            self.visited.push(format!("{:?}", p.id));
        }
        if p.arrive_t < 300 { return Raw::default(); }
        if p.id == ZoneId::Summit && p.boss_awake {
            return super::warden_input(game, tick, 0);
        }
        let here = (p.player.body.x, p.player.body.y);
        let points: &[(f32, f32)] = match p.id {
            ZoneId::Bloomwild => &[(152., 616.), (584., 584.), (1112., 552.), (1128., 304.), (920., 264.), (672., 264.)],
            ZoneId::Glasswater => &[],
            ZoneId::Cinder => &[(280., 520.), (600., 512.), (920., 552.), (1144., 360.)],
            ZoneId::Summit => &[(600., 776.), (720., 748.), (720., 588.), (720., 496.)],
        };
        while self.waypoint < points.len() && distance(here, points[self.waypoint]) < 9. {
            self.waypoint += 1;
            self.path.clear();
        }
        let objective = if self.waypoint < points.len() { Some(points[self.waypoint]) }
            else if !p.shard_taken { p.zone.shard.or(p.zone.boss_pos) }
            else { p.zone.gate };
        let Some(mut target) = objective else { return Raw::default() };
        // Fight nearby live enemies with real taps; do not chase across water or abandon the tour.
        let enemy = p.enemies.iter().filter(|e| e.body.alive)
            .filter(|e| distance(here, (e.body.x, e.body.y)) < 76.)
            .min_by(|a, b| distance(here, (a.body.x, a.body.y)).total_cmp(&distance(here, (b.body.x, b.body.y))));
        let mut attack = false;
        if let Some(e) = enemy {
            let there = (e.body.x, e.body.y);
            if distance(here, there) < 30. {
                return Raw { mv: crate::util::norm(there.0 - here.0, there.1 - here.1), attack: tick % 8 == 0, ..Raw::default() };
            }
            if !p.zone.blocked(there.0, there.1, 4.) { target = there; }
            attack = distance(here, there) < 42. && tick % 8 == 0;
        }
        if self.path.is_empty() || tick >= self.refresh || self.goal.is_none_or(|old| distance(old, target) > 12.) {
            self.path = path(&p.zone, here, target);
            self.goal = Some(target);
            self.refresh = tick + 45;
        }
        while self.path.last().is_some_and(|&point| distance(here, point) < 3.) { self.path.pop(); }
        // Skip intermediate grid points only when the full feet box can follow the segment.
        while self.path.len() > 1 {
            let next = self.path[self.path.len() - 2];
            if distance(here, next) > 18. || !clear(&p.zone, here, next) { break; }
            self.path.pop();
        }
        let next = self.path.last().copied().unwrap_or(target);
        Raw { mv: if distance(here, next) > 2. { crate::util::norm(next.0 - here.0, next.1 - here.1) } else { (0., 0.) }, attack, ..Raw::default() }
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 { (a.0 - b.0).hypot(a.1 - b.1) }

fn clear(z: &Zone, a: (f32, f32), b: (f32, f32)) -> bool {
    let n = distance(a, b).ceil().max(1.) as i32;
    (1..=n).all(|i| { let t = i as f32 / n as f32; !z.blocked(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, 4.) })
}

fn path(z: &Zone, from: (f32, f32), to: (f32, f32)) -> Vec<(f32, f32)> {
    let (w, h) = (z.pw / STEP, z.ph / STEP);
    let nearest = |point: (f32, f32)| {
        let (x, y) = ((point.0 / STEP as f32).round() as i32, (point.1 / STEP as f32).round() as i32);
        let mut candidates = Vec::new();
        for dy in -3..=3 { for dx in -3..=3 {
            let (xx, yy) = (x + dx, y + dy);
            if xx >= 0 && yy >= 0 && xx < w && yy < h && !z.blocked((xx * STEP) as f32, (yy * STEP) as f32, 4.) {
                candidates.push((distance(point, ((xx * STEP) as f32, (yy * STEP) as f32)), xx, yy));
            }
        }}
        candidates.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, x, y)| y * w + x)
    };
    let (Some(start), Some(end)) = (nearest(from), nearest(to)) else { return Vec::new() };
    let heuristic = |i: i32| (i % w - end % w).abs() + (i / w - end / w).abs();
    let mut costs = vec![i32::MAX; (w * h) as usize];
    let mut parent = vec![-1; costs.len()];
    let mut heap = BinaryHeap::new();
    costs[start as usize] = 0;
    heap.push(Reverse((heuristic(start), 0, start)));
    while let Some(Reverse((_, cost, i))) = heap.pop() {
        if i == end {
            let mut result = vec![to];
            let mut at = end;
            while at != start {
                result.push((((at % w) * STEP) as f32, ((at / w) * STEP) as f32));
                at = parent[at as usize];
            }
            return result;
        }
        if cost != costs[i as usize] { continue; }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let (x, y) = (i % w + dx, i / w + dy);
            if x < 0 || y < 0 || x >= w || y >= h { continue; }
            let j = y * w + x;
            if cost + 1 >= costs[j as usize] || z.blocked((x * STEP) as f32, (y * STEP) as f32, 4.) { continue; }
            costs[j as usize] = cost + 1;
            parent[j as usize] = i;
            heap.push(Reverse((cost + 1 + heuristic(j), cost + 1, j)));
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::Play;
    #[test]
    fn navigation_path_crosses_actual_bloomwild_bridge_and_stairs() {
        let p = Play::new(ZoneId::Bloomwild, &crate::play::PlayOpts { god: true, peaceful: true });
        let points = path(&p.zone, p.zone.player_start, p.zone.shard.unwrap());
        assert!(points.len() > 200);
        assert!(points.iter().all(|&(x, y)| !p.zone.blocked(x, y, 4.)));
        for pair in points.windows(2).skip(1) { assert!(clear(&p.zone, pair[0], pair[1])); }
        assert!(points.iter().any(|&(x, y)| x > 1060. && y > 300. && y < 360.), "uses the eastern stair");
    }

    #[test]
    fn whole_game_tour_collects_all_shards_and_reaches_real_victory() {
        use crate::game::Options;
        let mut game = Game::new(Options { start: Some(ZoneId::Bloomwild), god: true, ..Options::default() });
        let mut tour = Tour::default();
        let mut input = crate::input::Input::default();
        let mut shards = std::collections::HashSet::new();
        for tick in 1..=36_000 {
            input.step(tour.input(&game, tick));
            game.tick(&input);
            game.fx.sounds.clear();
            game.fx.lights.clear();
            if let Scene::Play(p) = &game.scene && p.shard_taken { shards.insert(format!("{:?}", p.id)); }
            if let Scene::Victory { t, .. } = &game.scene && *t >= crate::ui::VICTORY_PROMPT + 30 {
                assert_eq!(shards.len(), 3, "all actual pickups and gate transitions must happen");
                assert_eq!(tour.visited, ["Bloomwild", "Glasswater", "Cinder", "Summit"]);
                println!("whole-game real-input tour reached victory in {tick} ticks");
                return;
            }
        }
        panic!("whole-game tour stuck in {:?} at {:?}", tour.zone, tour.goal);
    }
}
