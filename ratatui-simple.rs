use ratatui::{
    prelude::*,
    Terminal,
    widgets::{Block, Borders, List, ListItem},
    crossterm::{
        event::{self, KeyCode},
        execute,
        terminal::{self},
    },
};
use std::io;
use std::time::{Duration};
use std::thread;

// Simple counter app
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let mut terminal = Terminal::new()?;

    // App state
    let mut counter = 0u32;

    // Main loop
    loop {
        terminal.draw(|f| {
            let block = Block::default()
                .border(Block::default().borders(Borders::ALL))
                .title("Ratatui Simple App");

            let paragraph = Paragraph::new(format!("Counter: {}", counter))
                .style(Style::default().fg(Color::Yellow));

            f.render_widget(block, f.area());
            f.render_widget(paragraph, f.area());

            Ok(())
        })?;

        // Handle input
        if event::read()? {
            if let Event::Key(key) = event::read()? {
                match key.kind {
                    KeyEventKind::Press => match key.code {
                        KeyCode::Char('c') | KeyCode::Esc => break,
                        KeyCode::Up => counter = counter.saturating_sub(1),
                        KeyCode::Down => counter = counter + 1,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

// Helper functions
fn enable_raw_mode() -> io::Result<()> {
    terminal::enable_raw_mode()
}

fn disable_raw_mode() -> io::Result<()> {
    terminal::disable_raw_mode()
}