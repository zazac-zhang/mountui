pub mod app;
pub mod bookmark;
pub mod config;
pub mod mount;
pub mod ui;

use std::io;

use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, Terminal};

use app::App;

fn main() -> io::Result<()> {
    // Create tokio runtime for async mount operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let _guard = rt.enter();

    let mut app = App::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<app::AsyncResult>();

    while app.running {
        terminal.draw(|frame| {
            ui::render(frame, app);
        })?;

        // Poll for async results (non-blocking)
        while let Ok(result) = rx.try_recv() {
            app.handle_async_result(result);
        }

        // Poll for terminal events
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key_event(key, &tx);
                }
                Event::Resize(_, _) => {
                    // Terminal handles resize automatically via draw
                }
                _ => {}
            }
        }
    }
    Ok(())
}
