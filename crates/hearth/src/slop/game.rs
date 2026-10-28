//! Slop Runner game state. Pure and deterministic: the terminal loop feeds it
//! inputs and fixed ticks, and tests drive it the same way.

use super::sprites::{self, Sprite};

/// Simulation steps per second.
pub const TICKS_PER_SECOND: u32 = 60;
/// Screen x of the runner's left edge.
pub const RUNNER_X: f32 = 10.0;

const GRAVITY: f32 = 0.26;
const JUMP_VELOCITY: f32 = 3.4;
const FAST_FALL: f32 = 0.5;
const DUCK_TICKS: u32 = 18;
const START_SPEED: f32 = 1.8;
const MAX_SPEED: f32 = 4.4;
const ACCELERATION: f32 = 0.0006;
const SPARK_POINTS: u32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Jump,
    Duck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlopKind {
    Puddle,
    Hand,
    Tower,
    Dashes,
}

impl SlopKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Puddle => "a puddle of lukewarm filler content",
            Self::Hand => "a six-fingered hand",
            Self::Tower => "a tower of listicles",
            Self::Dashes => "an em-dash swarm",
        }
    }

    pub fn sprite(self, frame: u32) -> Sprite {
        let even = frame.is_multiple_of(2);
        match self {
            Self::Puddle if even => sprites::PUDDLE_A,
            Self::Puddle => sprites::PUDDLE_B,
            Self::Hand => sprites::HAND,
            Self::Tower => sprites::TOWER,
            Self::Dashes if even => sprites::DASHES_A,
            Self::Dashes => sprites::DASHES_B,
        }
    }

    /// Height of the sprite's bottom edge above the ground.
    fn altitude(self) -> f32 {
        match self {
            Self::Dashes => 12.0,
            _ => 0.0,
        }
    }
}

/// An axis-aligned box in world space: x grows right, y grows up from the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn overlaps(self, other: Rect) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }

    fn of_sprite(sprite: Sprite, x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            w: sprites::width(sprite) as f32,
            h: sprites::height(sprite) as f32,
        }
    }

    /// Shrinks the box so near misses do not count as hits.
    fn inset(self, by: f32) -> Self {
        Self {
            x: self.x + by,
            y: self.y + by,
            w: self.w - 2.0 * by,
            h: self.h - 2.0 * by,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Obstacle {
    pub kind: SlopKind,
    pub x: f32,
}

impl Obstacle {
    pub fn y(&self) -> f32 {
        self.kind.altitude()
    }

    pub fn hitbox(&self, frame: u32) -> Rect {
        Rect::of_sprite(self.kind.sprite(frame), self.x, self.y()).inset(2.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spark {
    pub x: f32,
    pub y: f32,
}

impl Spark {
    fn hitbox(&self) -> Rect {
        Rect::of_sprite(sprites::SPARK_A, self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Running,
    Over { by: SlopKind },
}

/// Small deterministic PRNG (xorshift64*), so runs are reproducible in tests.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// Uniform in `lo..hi`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub width: f32,
    pub phase: Phase,
    /// Height of the runner's feet above the ground.
    pub runner_y: f32,
    pub runner_vy: f32,
    pub duck_ticks: u32,
    pub speed: f32,
    pub distance: f32,
    pub sparks_collected: u32,
    pub ticks: u32,
    pub obstacles: Vec<Obstacle>,
    pub sparks: Vec<Spark>,
    pub high_score: u32,
    next_spawn: f32,
    rng: Rng,
}

impl Game {
    pub fn new(width: usize, seed: u64) -> Self {
        Self {
            width: width as f32,
            phase: Phase::Running,
            runner_y: 0.0,
            runner_vy: 0.0,
            duck_ticks: 0,
            speed: START_SPEED,
            distance: 0.0,
            sparks_collected: 0,
            ticks: 0,
            obstacles: Vec::new(),
            sparks: Vec::new(),
            high_score: 0,
            // A short grace run before the first slop arrives.
            next_spawn: width as f32 * 0.6,
            rng: Rng::new(seed),
        }
    }

    /// Starts a new run, keeping the high score and screen width.
    pub fn restart(&mut self) {
        let seed = u64::from(self.rng.next_u32()) << 32 | u64::from(self.rng.next_u32());
        let high_score = self.high_score;
        *self = Self::new(self.width as usize, seed);
        self.high_score = high_score;
    }

    pub fn score(&self) -> u32 {
        (self.distance / 10.0) as u32 + self.sparks_collected * SPARK_POINTS
    }

    pub fn on_ground(&self) -> bool {
        self.runner_y <= 0.0
    }

    pub fn ducking(&self) -> bool {
        self.duck_ticks > 0 && self.on_ground()
    }

    pub fn runner_sprite(&self) -> Sprite {
        match self.phase {
            Phase::Over { .. } => sprites::SLOPPED,
            Phase::Running if self.ducking() => sprites::DUCK,
            Phase::Running if !self.on_ground() => sprites::JUMP,
            Phase::Running if ((self.distance / 9.0) as u32).is_multiple_of(2) => sprites::RUN_A,
            Phase::Running => sprites::RUN_B,
        }
    }

    pub fn runner_hitbox(&self) -> Rect {
        Rect::of_sprite(self.runner_sprite(), RUNNER_X, self.runner_y).inset(2.0)
    }

    pub fn input(&mut self, input: Input) {
        match (self.phase, input) {
            (Phase::Over { .. }, Input::Jump) => self.restart(),
            (Phase::Over { .. }, Input::Duck) => {}
            (Phase::Running, Input::Jump) if self.on_ground() => {
                self.duck_ticks = 0;
                self.runner_vy = JUMP_VELOCITY;
            }
            (Phase::Running, Input::Jump) => {}
            (Phase::Running, Input::Duck) => {
                // Terminals rarely report key releases, so a duck lasts a moment
                // and key repeat keeps it going while the key is held.
                self.duck_ticks = DUCK_TICKS;
                if !self.on_ground() {
                    self.runner_vy -= FAST_FALL * 4.0;
                }
            }
        }
    }

    /// Advances the simulation by one tick.
    pub fn tick(&mut self) {
        if self.phase != Phase::Running {
            return;
        }
        self.ticks += 1;
        self.speed = (self.speed + ACCELERATION).min(MAX_SPEED);
        self.distance += self.speed;
        self.duck_ticks = self.duck_ticks.saturating_sub(1);

        if !self.on_ground() || self.runner_vy > 0.0 {
            self.runner_y += self.runner_vy;
            self.runner_vy -= GRAVITY;
            if self.duck_ticks > 0 {
                self.runner_vy -= FAST_FALL;
            }
            if self.runner_y <= 0.0 {
                self.runner_y = 0.0;
                self.runner_vy = 0.0;
            }
        }

        for obstacle in &mut self.obstacles {
            obstacle.x -= self.speed;
        }
        for spark in &mut self.sparks {
            spark.x -= self.speed;
        }
        self.obstacles
            .retain(|o| o.x + sprites::width(o.kind.sprite(0)) as f32 > 0.0);
        self.sparks
            .retain(|s| s.x + sprites::width(sprites::SPARK_A) as f32 > 0.0);

        self.next_spawn -= self.speed;
        if self.next_spawn <= 0.0 {
            self.spawn();
        }

        let runner = self.runner_hitbox();
        let before = self.sparks.len();
        self.sparks.retain(|s| !s.hitbox().overlaps(runner));
        self.sparks_collected += (before - self.sparks.len()) as u32;

        let frame = self.frame();
        if let Some(hit) = self
            .obstacles
            .iter()
            .find(|o| o.hitbox(frame).overlaps(runner))
        {
            self.phase = Phase::Over { by: hit.kind };
            self.high_score = self.high_score.max(self.score());
        }
    }

    /// Animation frame counter shared by sprites that wobble.
    pub fn frame(&self) -> u32 {
        self.ticks / 12
    }

    fn spawn(&mut self) {
        // Flying swarms only show up once the runner has warmed up.
        let kinds: &[SlopKind] = if self.distance < 600.0 {
            &[SlopKind::Puddle, SlopKind::Hand, SlopKind::Tower]
        } else {
            &[
                SlopKind::Puddle,
                SlopKind::Hand,
                SlopKind::Tower,
                SlopKind::Dashes,
            ]
        };
        let kind = kinds[self.rng.next_u32() as usize % kinds.len()];
        self.obstacles.push(Obstacle {
            kind,
            x: self.width,
        });

        if self.rng.next_u32().is_multiple_of(3) {
            let y = if kind == SlopKind::Dashes {
                2.0
            } else {
                self.rng.range(20.0, 26.0)
            };
            self.sparks.push(Spark {
                x: self.width + 2.0,
                y,
            });
        }

        // Gaps scale with speed so every jump stays possible.
        let min_gap = 70.0 + self.speed * 22.0;
        self.next_spawn = self.rng.range(min_gap, min_gap * 1.9);
    }

    #[cfg(test)]
    pub fn place(&mut self, kind: SlopKind, x: f32) {
        self.obstacles.push(Obstacle { kind, x });
        self.next_spawn = f32::MAX;
    }

    #[cfg(test)]
    pub fn place_spark(&mut self, x: f32, y: f32) {
        self.sparks.push(Spark { x, y });
        self.next_spawn = f32::MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet_game() -> Game {
        let mut game = Game::new(160, 7);
        game.next_spawn = f32::MAX;
        game
    }

    fn run(game: &mut Game, ticks: u32) {
        for _ in 0..ticks {
            game.tick();
        }
    }

    #[test]
    fn jump_rises_and_lands_back_on_the_ground() {
        let mut game = quiet_game();
        game.input(Input::Jump);
        game.tick();
        assert!(game.runner_y > 0.0);
        let mut peak: f32 = 0.0;
        for _ in 0..60 {
            game.tick();
            peak = peak.max(game.runner_y);
        }
        assert!((20.0..24.0).contains(&peak), "peak {peak}");
        assert!(game.on_ground());
        assert_eq!(game.runner_vy, 0.0);
    }

    #[test]
    fn no_double_jump_in_the_air() {
        let mut game = quiet_game();
        game.input(Input::Jump);
        run(&mut game, 5);
        let vy = game.runner_vy;
        game.input(Input::Jump);
        assert_eq!(game.runner_vy, vy);
    }

    #[test]
    fn standing_still_into_slop_ends_the_run() {
        let mut game = quiet_game();
        game.place(SlopKind::Hand, RUNNER_X + 30.0);
        run(&mut game, 40);
        assert_eq!(game.phase, Phase::Over { by: SlopKind::Hand });
        assert!(game.high_score > 0);
    }

    #[test]
    fn a_timed_jump_clears_the_tallest_ground_slop() {
        let mut game = quiet_game();
        game.place(SlopKind::Tower, RUNNER_X + 40.0);
        run(&mut game, 10);
        game.input(Input::Jump);
        run(&mut game, 80);
        assert_eq!(game.phase, Phase::Running);
    }

    #[test]
    fn ducking_passes_under_the_em_dash_swarm_but_standing_does_not() {
        let mut standing = quiet_game();
        standing.place(SlopKind::Dashes, RUNNER_X + 30.0);
        run(&mut standing, 60);
        assert_eq!(
            standing.phase,
            Phase::Over {
                by: SlopKind::Dashes
            }
        );

        let mut ducking = quiet_game();
        ducking.place(SlopKind::Dashes, RUNNER_X + 30.0);
        for _ in 0..60 {
            ducking.input(Input::Duck);
            ducking.tick();
        }
        assert_eq!(ducking.phase, Phase::Running);
        assert!(ducking.obstacles.is_empty() || ducking.obstacles[0].x < RUNNER_X);
    }

    #[test]
    fn sparks_add_points_once() {
        let mut game = quiet_game();
        game.place_spark(RUNNER_X + 6.0, 4.0);
        run(&mut game, 10);
        assert_eq!(game.sparks_collected, 1);
        assert!(game.sparks.is_empty());
        assert!(game.score() >= SPARK_POINTS);
    }

    #[test]
    fn game_over_freezes_until_jump_restarts_and_keeps_high_score() {
        let mut game = quiet_game();
        run(&mut game, 200);
        game.place(SlopKind::Puddle, RUNNER_X + 10.0);
        run(&mut game, 20);
        let Phase::Over { .. } = game.phase else {
            panic!("expected game over")
        };
        let score = game.score();
        run(&mut game, 50);
        assert_eq!(game.score(), score, "score must not change after game over");

        game.input(Input::Duck);
        assert!(
            matches!(game.phase, Phase::Over { .. }),
            "duck must not restart"
        );
        game.input(Input::Jump);
        assert_eq!(game.phase, Phase::Running);
        assert_eq!(game.score(), 0);
        assert_eq!(game.high_score, score);
    }

    #[test]
    fn speed_ramps_up_but_is_capped() {
        let mut game = quiet_game();
        run(&mut game, 100);
        assert!(game.speed > START_SPEED);
        game.speed = MAX_SPEED;
        game.tick();
        assert_eq!(game.speed, MAX_SPEED);
    }

    #[test]
    fn spawned_slop_is_always_jumpable_or_duckable() {
        // A bot that jumps ground slop and ducks swarms should survive any seed:
        // this proves the spawner never builds an impossible wall.
        for seed in 1..=20 {
            let mut game = Game::new(160, seed);
            for _ in 0..TICKS_PER_SECOND * 90 {
                bot(&mut game);
                game.tick();
                if let Phase::Over { by } = game.phase {
                    panic!(
                        "seed {seed}: bot died to {by:?} at distance {}",
                        game.distance
                    );
                }
            }
            assert!(game.score() > 300, "seed {seed}: score {}", game.score());
        }
    }

    fn bot(game: &mut Game) {
        let ahead = game
            .obstacles
            .iter()
            .filter(|o| o.x + 12.0 > RUNNER_X)
            .min_by(|a, b| a.x.total_cmp(&b.x));
        let Some(next) = ahead else { return };
        let gap = next.x - (RUNNER_X + 12.0);
        match next.kind {
            SlopKind::Dashes if gap < 30.0 => game.input(Input::Duck),
            SlopKind::Dashes => {}
            _ if gap < game.speed * 7.0 => game.input(Input::Jump),
            _ => {}
        }
    }

    #[test]
    fn rng_is_deterministic_per_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let mut c = Rng::new(43);
        let xs: Vec<_> = (0..8).map(|_| a.next_u32()).collect();
        let ys: Vec<_> = (0..8).map(|_| b.next_u32()).collect();
        let zs: Vec<_> = (0..8).map(|_| c.next_u32()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs, zs);
    }

    #[test]
    fn every_slop_has_a_name_for_the_game_over_screen() {
        for kind in [
            SlopKind::Puddle,
            SlopKind::Hand,
            SlopKind::Tower,
            SlopKind::Dashes,
        ] {
            assert!(!kind.name().is_empty());
        }
    }
}
