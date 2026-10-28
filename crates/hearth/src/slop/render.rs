//! Draws a game frame into a 24-bit framebuffer and encodes it for a truecolor
//! terminal, two pixels per cell using the upper half block.

use std::fmt::Write as _;

use super::game::{Game, Phase, RUNNER_X};
use super::sprites::{self, Sprite};

/// Pixels between the bottom of the canvas and the ground line.
pub const GROUND_DEPTH: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    fn mix(self, other: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
        Rgb(
            lerp(self.0, other.0),
            lerp(self.1, other.1),
            lerp(self.2, other.2),
        )
    }
}

pub struct Frame {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Rgb>,
}

impl Frame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Rgb(0, 0, 0); width * height],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Rgb {
        self.pixels[y * self.width + x]
    }

    fn set(&mut self, x: i32, y: i32, color: Rgb) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }

    /// Draws a sprite with its top-left corner at screen (x, y), clipping at edges.
    pub fn sprite(&mut self, sprite: Sprite, x: i32, y: i32) {
        for (dy, row) in sprite.iter().enumerate() {
            for (dx, key) in row.bytes().enumerate() {
                if let Some(color) = sprites::color(key) {
                    self.set(x + dx as i32, y + dy as i32, color);
                }
            }
        }
    }

    /// Encodes the frame as ANSI truecolor text. Only cells that differ from
    /// `previous` are sent, and colors are only re-sent when they change.
    pub fn to_ansi(&self, previous: Option<&Frame>) -> String {
        let previous = previous.filter(|p| p.width == self.width && p.height == self.height);
        let mut out = String::with_capacity(self.width * self.height * 4);
        let mut colors: Option<(Rgb, Rgb)> = None;
        for row in 0..self.height.div_ceil(2) {
            let mut cursor_here = false;
            for x in 0..self.width {
                let cell = self.cell(x, row);
                if previous.is_some_and(|p| p.cell(x, row) == cell) {
                    cursor_here = false;
                    continue;
                }
                if !cursor_here {
                    let _ = write!(out, "\x1b[{};{}H", row + 1, x + 1);
                    cursor_here = true;
                }
                if colors != Some(cell) {
                    let (top, bottom) = cell;
                    let _ = write!(
                        out,
                        "\x1b[38;2;{};{};{};48;2;{};{};{}m",
                        top.0, top.1, top.2, bottom.0, bottom.1, bottom.2
                    );
                    colors = Some(cell);
                }
                out.push('▀');
            }
        }
        out.push_str("\x1b[0m");
        out
    }

    /// The (top, bottom) pixel pair shown by one terminal cell.
    fn cell(&self, x: usize, row: usize) -> (Rgb, Rgb) {
        let top = self.get(x, row * 2);
        let bottom = if row * 2 + 1 < self.height {
            self.get(x, row * 2 + 1)
        } else {
            top
        };
        (top, bottom)
    }
}

/// Screen row of the ground line for a canvas of this height.
pub fn ground_line(height: usize) -> i32 {
    (height - GROUND_DEPTH) as i32
}

/// Converts a world position (feet height above ground) to a sprite's top row.
fn top_row(height: usize, sprite: Sprite, world_y: f32) -> i32 {
    ground_line(height) - world_y.round() as i32 - sprites::height(sprite) as i32
}

pub fn draw(game: &Game, width: usize, height: usize) -> Frame {
    let mut frame = Frame::new(width, height);
    sky(&mut frame, game);
    skyline(&mut frame, game.distance * 0.15, Rgb(58, 52, 92), 22, 41);
    skyline(&mut frame, game.distance * 0.4, Rgb(86, 66, 110), 13, 97);
    ground(&mut frame, game.distance);
    if matches!(game.phase, Phase::Over { .. }) {
        // Desaturate the world, but not the sprites, while the runner is slopped.
        for pixel in &mut frame.pixels {
            let grey = ((u16::from(pixel.0) + u16::from(pixel.1) + u16::from(pixel.2)) / 3) as u8;
            *pixel = pixel.mix(Rgb(grey, grey, grey), 0.5);
        }
    }

    let tick = game.frame();
    for spark in &game.sparks {
        let sprite = if (game.ticks / 8).is_multiple_of(2) {
            sprites::SPARK_A
        } else {
            sprites::SPARK_B
        };
        frame.sprite(
            sprite,
            spark.x.round() as i32,
            top_row(height, sprite, spark.y),
        );
    }
    for obstacle in &game.obstacles {
        let sprite = obstacle.kind.sprite(tick);
        frame.sprite(
            sprite,
            obstacle.x.round() as i32,
            top_row(height, sprite, obstacle.y()),
        );
    }
    let runner = game.runner_sprite();
    frame.sprite(
        runner,
        RUNNER_X as i32,
        top_row(height, runner, game.runner_y),
    );
    frame
}

/// A vertical gradient that drifts from afternoon to sunset to night as the score climbs.
fn sky(frame: &mut Frame, game: &Game) {
    const SKIES: [(Rgb, Rgb); 3] = [
        (Rgb(92, 158, 226), Rgb(250, 214, 180)),
        (Rgb(122, 78, 140), Rgb(255, 150, 96)),
        (Rgb(18, 20, 48), Rgb(70, 56, 112)),
    ];
    let cycle = (game.score() as f32 / 400.0) % 3.0;
    let (from, to) = (cycle as usize, (cycle as usize + 1) % 3);
    let t = cycle.fract();
    let top = SKIES[from].0.mix(SKIES[to].0, t);
    let bottom = SKIES[from].1.mix(SKIES[to].1, t);
    let horizon = ground_line(frame.height).max(1) as f32;
    for y in 0..frame.height {
        let color = top.mix(bottom, y as f32 / horizon);
        for x in 0..frame.width {
            frame.set(x as i32, y as i32, color);
        }
    }
}

/// Server-rack silhouettes scrolling at `offset`, up to `max_height` tall.
fn skyline(frame: &mut Frame, offset: f32, color: Rgb, max_height: i32, salt: u32) {
    let ground = ground_line(frame.height);
    let lit = color.mix(Rgb(255, 210, 130), 0.55);
    for x in 0..frame.width as i32 {
        let world = x + offset as i32;
        let block = world.div_euclid(9) as u32;
        let hash = block
            .wrapping_mul(2_654_435_761)
            .wrapping_add(salt)
            .rotate_left(13);
        let tall = 4 + (hash % max_height as u32) as i32;
        if world.rem_euclid(9) == 8 {
            continue;
        }
        for y in (ground - tall).max(0)..ground {
            let window =
                world.rem_euclid(3) == 1 && (ground - y) % 3 == 1 && (hash >> (y % 16)) & 1 == 1;
            frame.set(x, y, if window { lit } else { color });
        }
    }
}

fn ground(frame: &mut Frame, distance: f32) {
    let line = ground_line(frame.height);
    let offset = distance as i32;
    for x in 0..frame.width as i32 {
        let world = (x + offset) as u32;
        let hash = world.wrapping_mul(2_246_822_519).rotate_left(7);
        frame.set(x, line, Rgb(238, 196, 140));
        frame.set(x, line + 1, Rgb(176, 120, 82));
        for y in line + 2..frame.height as i32 {
            let depth = (y - line) as f32 / GROUND_DEPTH as f32;
            let base = Rgb(140, 88, 64).mix(Rgb(86, 52, 44), depth);
            let pebble = (hash >> (y as u32 % 24)) & 15 == 0;
            frame.set(x, y, if pebble { Rgb(206, 160, 116) } else { base });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slop::game::{Game, Input, SlopKind};

    const W: usize = 96;
    const H: usize = 46;

    fn quiet() -> Game {
        let mut game = Game::new(W, 3);
        game.place(SlopKind::Hand, 70.0);
        game
    }

    #[test]
    fn runner_stands_on_the_ground_line() {
        let game = quiet();
        let frame = draw(&game, W, H);
        let sprite = game.runner_sprite();
        let feet = ground_line(H) - 1;
        let row = sprite[sprites::height(sprite) - 1];
        let (dx, key) = row
            .bytes()
            .enumerate()
            .find(|&(_, k)| k != b'.')
            .expect("feet pixels");
        assert_eq!(
            frame.get(RUNNER_X as usize + dx, feet as usize),
            sprites::color(key).expect("color")
        );
    }

    #[test]
    fn jumping_moves_the_runner_up_the_screen() {
        let mut game = quiet();
        let standing = draw(&game, W, H);
        game.input(Input::Jump);
        for _ in 0..10 {
            game.tick();
        }
        let jumping = draw(&game, W, H);
        let head = |frame: &Frame| {
            (0..H).find(|&y| {
                frame.get(RUNNER_X as usize + 6, y) == sprites::color(b'k').expect("outline")
            })
        };
        assert!(head(&jumping) < head(&standing));
    }

    #[test]
    fn obstacles_are_drawn_where_the_game_says() {
        let game = quiet();
        let frame = draw(&game, W, H);
        let skin = sprites::color(b'f').expect("skin");
        assert!((70..84).any(|x| (0..H).any(|y| frame.get(x, y) == skin)));
    }

    #[test]
    fn sprites_clip_at_screen_edges_without_panicking() {
        let mut frame = Frame::new(8, 8);
        frame.sprite(sprites::TOWER, -6, -6);
        frame.sprite(sprites::TOWER, 5, 5);
    }

    #[test]
    fn ansi_encoding_uses_half_blocks_and_truecolor() {
        let mut frame = Frame::new(3, 4);
        frame.set(0, 0, Rgb(1, 2, 3));
        frame.set(0, 1, Rgb(4, 5, 6));
        let ansi = frame.to_ansi(None);
        assert_eq!(ansi.matches('▀').count(), 6, "3 columns x 2 cell rows");
        assert!(ansi.starts_with("\x1b[1;1H\x1b[38;2;1;2;3;48;2;4;5;6m▀"));
        assert!(ansi.ends_with("\x1b[0m"));
        // Repeated colors are not re-sent.
        assert_eq!(ansi.matches("38;2;0;0;0;48;2;0;0;0m").count(), 1);
    }

    #[test]
    fn ansi_diff_sends_only_changed_cells() {
        let before = Frame::new(4, 4);
        let mut after = Frame::new(4, 4);
        after.set(2, 3, Rgb(9, 9, 9));
        let diff = after.to_ansi(Some(&before));
        assert_eq!(diff.matches('▀').count(), 1);
        assert!(diff.starts_with("\x1b[2;3H"), "{diff:?}");
        assert_eq!(after.to_ansi(Some(&after)), "\x1b[0m");
        // A resized previous frame forces a full redraw.
        assert_eq!(
            after.to_ansi(Some(&Frame::new(2, 2))).matches('▀').count(),
            8
        );
    }

    #[test]
    fn world_greys_out_on_game_over() {
        let mut game = Game::new(W, 3);
        game.place(SlopKind::Puddle, RUNNER_X + 4.0);
        for _ in 0..30 {
            game.tick();
        }
        assert!(matches!(game.phase, Phase::Over { .. }));
        let sky = draw(&game, W, H).get(W - 1, 0);
        let spread = sky.0.max(sky.1).max(sky.2) - sky.0.min(sky.1).min(sky.2);
        assert!(spread < 80, "sky should be desaturated, got {sky:?}");
    }
}
