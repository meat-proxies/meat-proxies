use ratatui::{
    prelude::*,
    Terminal,
    crossterm::{
        event::{self, KeyCode},
        execute,
        terminal::{self},
    },
    widgets::{Block, Borders, List, ListItem},
};
use std::io;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let mut terminal = Terminal::new()?;

    // Create our application state
    let mut app = App::new();

    // Run the terminal loop
    let result = run_app(&mut terminal, &mut app)?;

    // Restore terminal state
    disable_raw_mode()?;
    execute!(
        LeaveAlternateScreen,
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    println!(); // Print a new line after the prompt

    Ok(result)
}

struct App {
    selected: usize,
    items: Vec<String>,
}

impl App {
    fn new() -> Self {
        Self {
            selected: 0,
            items: vec!["Home".to_string(), "Settings".to_string(), "About".to_string()],
        }
    }

    fn next(&mut self) {
        self.selected = (self.selected + 1) % self.items.len();
    }

    fn previous(&mut self) {
        self.selected = (self.selected + self.items.len() - 1) % self.items.len();
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .split_rules(
                    "vertical", // direction
                    [100],       // ratio or size
                )[0]; // placeholder

            let items: Vec<_> = app.items.iter().map(|i| {
                let selected = if app.selected == 0 { ">> " } else { "" };
                ListItem::new(format!("{}{}", selected, i))
            }).collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Simple TUI"))
                .highlight_style(Style::default().fg(Color::Yellow));

            f.render_widget(list, chunks[0]);
            Ok(())
        })?;

        // Handle input
        if crossterm::event::read()? {
            match event::Event::Key(key_event) if key_event.kind == event::KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('c') | KeyCode::Esc => return Ok(()),
                    KeyCode::Up => app.previous(),
                    KeyCode::Down => app.next(),
                    _ => {}
                }
            }
        }

        // Simple delay to avoid busy loop
        thread::sleep(Duration::from_millis(100));
    }
}

// Helper to enable raw mode and capture input
fn enable_raw_mode() -> io::Result<()> {
    terminal::enable_raw_mode()
}