//! Slop Runner: a hidden endless runner (`hearth --slop`). Dodge the slop.

mod game;
mod render;
mod sprites;

use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};

use game::{Game, Input, Phase, TICKS_PER_SECOND};
use render::Frame;

const MIN_COLS: u16 = 64;
const MIN_ROWS: u16 = 24;
const MAX_COLS: u16 = 220;
const MAX_CANVAS_ROWS: u16 = 32;

/// Leaves raw mode and the alternate screen however the game exits.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Layout {
    cols: u16,
    canvas_rows: u16,
}

impl Layout {
    /// `None` when the terminal is too small to play.
    fn fit(cols: u16, rows: u16) -> Option<Self> {
        (cols >= MIN_COLS && rows >= MIN_ROWS).then(|| Self {
            cols: cols.min(MAX_COLS),
            canvas_rows: (rows - 1).min(MAX_CANVAS_ROWS),
        })
    }

    fn width(self) -> usize {
        usize::from(self.cols)
    }

    fn height(self) -> usize {
        usize::from(self.canvas_rows) * 2
    }
}

pub fn run() -> Result<(), String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("hearth --slop needs an interactive terminal.".into());
    }
    terminal::enable_raw_mode().map_err(|e| format!("Could not enter raw mode: {e}"))?;
    let _guard = TerminalGuard;
    let mut out = io::stdout();
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)
        .map_err(|e| format!("Could not set up the terminal: {e}"))?;
    play(&mut out).map_err(|e| format!("Slop Runner stopped: {e}"))
}

fn play(out: &mut impl Write) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let mut layout = Layout::fit(cols, rows);
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64);
    let mut game = Game::new(layout.map_or(usize::from(MIN_COLS), Layout::width), seed);
    let mut paused = false;
    let mut previous: Option<Frame> = None;
    let tick = Duration::from_secs(1) / TICKS_PER_SECOND;
    let mut next_tick = Instant::now();

    loop {
        if event::poll(next_tick.saturating_duration_since(Instant::now()))? {
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
                    layout = Layout::fit(cols, rows);
                    if let Some(fit) = layout {
                        game.width = fit.width() as f32;
                    }
                    previous = None;
                    write!(out, "\x1b[0m\x1b[2J")?;
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
            write!(
                out,
                "\x1b[0m\x1b[2J\x1b[1;1HSlop Runner needs at least {MIN_COLS}x{MIN_ROWS}. Resize the terminal or press q to quit."
            )?;
            out.flush()?;
            continue;
        };
        if !paused {
            game.tick();
        }
        let frame = render::draw(&game, fit.width(), fit.height());
        let mut text = frame.to_ansi(previous.as_ref());
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
    fn layout_rejects_small_terminals_and_caps_large_ones() {
        assert!(Layout::fit(MIN_COLS - 1, MIN_ROWS).is_none());
        assert!(Layout::fit(MIN_COLS, MIN_ROWS - 1).is_none());
        let small = Layout::fit(80, 24).expect("80x24 fits");
        assert_eq!((small.width(), small.height()), (80, 46));
        let huge = Layout::fit(500, 100).expect("big fits");
        assert_eq!(
            (huge.width(), huge.height()),
            (usize::from(MAX_COLS), usize::from(MAX_CANVAS_ROWS) * 2)
        );
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
