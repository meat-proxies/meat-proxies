//! Slop Runner: a hidden endless runner (`hearth --slop`). Dodge the slop.

mod art;
mod game;
mod graphics;
mod raster;
mod render;
mod sprites;

use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};

use art::ArtSet;
use game::{Game, Input, Phase, TICKS_PER_SECOND};
use graphics::Graphics;
use render::Frame;

const MIN_COLS: u16 = 64;
const MIN_ROWS: u16 = 24;
/// Widest and tallest playfield, in world units.
const MAX_WORLD_W: usize = 220;
const MAX_WORLD_H: usize = 64;
/// World height the detailed art needs to fit a full jump.
const MIN_WORLD_H: usize = 46;
/// Largest image, in pixels, sent per frame before dropping to a lower scale.
const IMAGE_PIXEL_BUDGET: usize = 60_000;

/// Leaves raw mode and the alternate screen however the game exits.
struct TerminalGuard {
    kitty: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = io::stdout();
        if self.kitty {
            let _ = out.write_all(graphics::KITTY_CLEAR.as_bytes());
        }
        let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    graphics: Graphics,
    /// Art pixels per world unit.
    scale: usize,
    /// World size in units (one unit is one pixel of the small art).
    world_w: usize,
    world_h: usize,
    /// Terminal cells the playfield covers.
    cols: u16,
    canvas_rows: u16,
}

impl Layout {
    /// `None` when the terminal is too small to play.
    fn fit(cols: u16, rows: u16, graphics: Graphics) -> Option<Self> {
        if cols < MIN_COLS || rows < MIN_ROWS {
            return None;
        }
        let rows_free = usize::from(rows - 1);
        let cols = usize::from(cols);
        let layout = match graphics {
            Graphics::Kitty | Graphics::Iterm => {
                // Images are scaled to the cells by the terminal, so the
                // detailed art fits any size. Each cell is one unit wide and
                // two tall.
                let (world_w, world_h) = (cols.min(MAX_WORLD_W), (rows_free * 2).min(MAX_WORLD_H));
                let scale = if world_w * world_h * 9 <= IMAGE_PIXEL_BUDGET {
                    3
                } else {
                    2
                };
                Self {
                    graphics,
                    scale,
                    world_w,
                    world_h,
                    cols: world_w as u16,
                    canvas_rows: (world_h / 2) as u16,
                }
            }
            Graphics::Blocks if cols / 2 >= usize::from(MIN_COLS) && rows_free >= MIN_WORLD_H => {
                // Detailed art drawn with half blocks: two cells per unit across,
                // one cell (two half-block pixels) per unit down.
                let (world_w, world_h) = ((cols / 2).min(MAX_WORLD_W), rows_free.min(MAX_WORLD_H));
                Self {
                    graphics,
                    scale: 2,
                    world_w,
                    world_h,
                    cols: (world_w * 2) as u16,
                    canvas_rows: world_h as u16,
                }
            }
            Graphics::Blocks => {
                let (world_w, world_h) = (cols.min(MAX_WORLD_W), (rows_free * 2).min(MAX_WORLD_H));
                Self {
                    graphics,
                    scale: 1,
                    world_w,
                    world_h,
                    cols: world_w as u16,
                    canvas_rows: (world_h / 2) as u16,
                }
            }
        };
        Some(layout)
    }

    /// Frame size in pixels.
    fn frame_size(self) -> (usize, usize) {
        (self.world_w * self.scale, self.world_h * self.scale)
    }

    fn width(self) -> usize {
        usize::from(self.cols)
    }
}

/// Art sets baked so far, indexed by scale.
#[derive(Default)]
struct ArtCache(Vec<Option<ArtSet>>);

impl ArtCache {
    fn get(&mut self, scale: usize) -> &ArtSet {
        if self.0.len() <= scale {
            self.0.resize_with(scale + 1, || None);
        }
        self.0[scale].get_or_insert_with(|| ArtSet::new(scale))
    }
}

pub fn run() -> Result<(), String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("hearth --slop needs an interactive terminal.".into());
    }
    let graphics = Graphics::detect(|name| std::env::var(name).ok())?;
    terminal::enable_raw_mode().map_err(|e| format!("Could not enter raw mode: {e}"))?;
    let _guard = TerminalGuard {
        kitty: graphics == Graphics::Kitty,
    };
    let mut out = io::stdout();
    execute!(
        out,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(terminal::ClearType::All)
    )
    .map_err(|e| format!("Could not set up the terminal: {e}"))?;
    play(&mut out, graphics).map_err(|e| format!("Slop Runner stopped: {e}"))
}

fn play(out: &mut impl Write, graphics: Graphics) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let mut layout = Layout::fit(cols, rows, graphics);
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64);
    let mut game = Game::new(layout.map_or(usize::from(MIN_COLS), |l| l.world_w), seed);
    let mut art = ArtCache::default();
    let mut paused = false;
    let mut previous: Option<Frame> = None;
    let mut image_id = 1;
    // Set when a key or resize may have changed what is on screen.
    let mut redraw = true;
    let mut frames: u64 = 0;
    let mut was_over = false;
    let tick = Duration::from_secs(1) / TICKS_PER_SECOND;
    let mut next_tick = Instant::now();

    loop {
        if event::poll(next_tick.saturating_duration_since(Instant::now()))? {
            redraw = true;
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('p') => paused = !paused,
                    KeyCode::Char(' ' | 'w' | 'k') | KeyCode::Up | KeyCode::Enter if !paused => {
                        game.input(Input::Jump)
                    }
                    KeyCode::Char('s' | 'j') | KeyCode::Down if !paused => game.input(Input::Duck),
                    _ => {}
                },
                Event::Resize(cols, rows) => {
                    layout = Layout::fit(cols, rows, graphics);
                    if let Some(fit) = layout {
                        game.width = fit.world_w as f32;
                    }
                    previous = None;
                    write!(out, "\x1b[0m\x1b[2J")?;
                    if graphics == Graphics::Kitty {
                        out.write_all(graphics::KITTY_CLEAR.as_bytes())?;
                    }
                }
                _ => {}
            }
            continue;
        }

        let now = Instant::now();
        if now.duration_since(next_tick) > tick * 15 {
            // Fell far behind (suspended, slow terminal): skip ahead rather than fast-forward.
            next_tick = now;
        }
        next_tick += tick;

        let Some(fit) = layout else {
            if redraw {
                write!(
                    out,
                    "\x1b[0m\x1b[2J\x1b[1;1HSlop Runner needs at least {MIN_COLS}x{MIN_ROWS}. Resize the terminal or press q to quit."
                )?;
                out.flush()?;
                redraw = false;
            }
            continue;
        };
        if !paused {
            game.tick();
        }
        frames += 1;
        let over = matches!(game.phase, Phase::Over { .. });
        // Show the moment of getting slopped, and the restart, right away.
        redraw |= over != was_over;
        was_over = over;
        let still = paused || over;
        // Still scenes only redraw after input; images go out at 30 fps.
        if (still || (graphics != Graphics::Blocks && frames % 2 == 1)) && !redraw {
            continue;
        }
        redraw = false;
        let (w, h) = fit.frame_size();
        let frame = render::draw(&game, art.get(fit.scale), w, h);
        let mut text = match fit.graphics {
            Graphics::Blocks => frame.to_ansi(previous.as_ref()),
            Graphics::Kitty => {
                image_id = 3 - image_id;
                graphics::kitty(&frame, fit.cols, fit.canvas_rows, image_id)
            }
            Graphics::Iterm => graphics::iterm(&frame, fit.cols, fit.canvas_rows),
        };
        text.push_str(&hud(&game, paused, fit));
        out.write_all(text.as_bytes())?;
        out.flush()?;
        previous = Some(frame);
    }
}

fn hud(game: &Game, paused: bool, layout: Layout) -> String {
    format!(
        "\x1b[{};1H\x1b[0m\x1b[38;2;250;214;180;48;2;43;26;22m{:<width$}\x1b[0m",
        layout.canvas_rows + 1,
        status_line(game, paused, layout.width()),
        width = layout.width()
    )
}

/// The status bar text, truncated to `width` characters.
fn status_line(game: &Game, paused: bool, width: usize) -> String {
    let best = game.high_score.max(game.score());
    let line = match game.phase {
        Phase::Over { by } => format!(
            " Slopped by {}!  score {:05}  best {best:05}  [space] run again  [q] quit",
            by.name(),
            game.score()
        ),
        _ => {
            let keys = if paused {
                "PAUSED  [p] resume  [q] quit"
            } else {
                "[space] jump  [down] duck  [p] pause  [q] quit"
            };
            format!(
                " SLOP RUNNER  score {:05}  best {best:05}  sparks {}  {keys}",
                game.score(),
                game.sparks_collected
            )
        }
    };
    line.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_rejects_small_terminals() {
        for graphics in [Graphics::Blocks, Graphics::Kitty, Graphics::Iterm] {
            assert!(Layout::fit(MIN_COLS - 1, MIN_ROWS, graphics).is_none());
            assert!(Layout::fit(MIN_COLS, MIN_ROWS - 1, graphics).is_none());
        }
    }

    #[test]
    fn blocks_use_detailed_art_only_when_it_fits() {
        let small = Layout::fit(80, 24, Graphics::Blocks).expect("80x24 fits");
        assert_eq!(
            (
                small.scale,
                small.world_w,
                small.world_h,
                small.frame_size()
            ),
            (1, 80, 46, (80, 46))
        );
        let big = Layout::fit(160, 50, Graphics::Blocks).expect("160x50 fits");
        assert_eq!(
            (big.scale, big.world_w, big.world_h, big.frame_size()),
            (2, 80, 49, (160, 98))
        );
        // One row short of a full jump falls back to the small art.
        assert_eq!(
            Layout::fit(160, 46, Graphics::Blocks).map(|l| l.scale),
            Some(1)
        );
        let huge = Layout::fit(1000, 300, Graphics::Blocks).expect("huge fits");
        assert_eq!((huge.world_w, huge.world_h), (MAX_WORLD_W, MAX_WORLD_H));
    }

    #[test]
    fn images_get_detailed_art_at_any_size_within_budget() {
        let small = Layout::fit(80, 24, Graphics::Kitty).expect("fits");
        assert_eq!(
            (small.scale, small.world_w, small.world_h, small.canvas_rows),
            (3, 80, 46, 23)
        );
        assert_eq!(small.frame_size(), (240, 138));
        let big = Layout::fit(220, 60, Graphics::Iterm).expect("fits");
        assert_eq!(big.scale, 2);
        let (w, h) = big.frame_size();
        assert!(w * h <= IMAGE_PIXEL_BUDGET);
    }

    #[test]
    fn art_cache_bakes_each_scale_once() {
        let mut cache = ArtCache::default();
        assert_eq!(cache.get(2).scale, 2);
        assert_eq!(cache.get(1).scale, 1);
        let first: *const ArtSet = cache.get(2);
        assert!(std::ptr::eq(first, cache.get(2)));
    }

    #[test]
    fn status_line_fits_the_width_and_names_the_slop_on_game_over() {
        let mut game = Game::new(64, 1);
        assert!(status_line(&game, false, 64).starts_with(" SLOP RUNNER"));
        assert!(status_line(&game, true, 200).contains("PAUSED"));
        game.place(game::SlopKind::Puddle, game::RUNNER_X + 4.0);
        for _ in 0..30 {
            game.tick();
        }
        let over = status_line(&game, false, 64);
        assert!(over.contains("lukewarm filler content!"), "{over}");
        assert_eq!(over.chars().count(), 64);
    }
}
