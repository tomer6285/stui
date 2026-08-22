pub mod app;
pub mod models;
pub mod steam;
pub mod ui;

use std::error::Error;
use std::io::{self, stdout};
use std::panic;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;

#[derive(Parser, Debug)]
#[command(name = "stui", about = "A TUI Steam Game Launcher")]
struct Args {
    #[arg(short = 'q', long = "quit", help = "Quit after launching a game")]
    quit_after_launch: bool,
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn set_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    set_panic_hook();

    let games = steam::sync_and_load_games();
    let state = steam::load_state();

    let last_selected = if state.last_selected_id.is_empty() {
        None
    } else {
        Some(state.last_selected_id.as_str())
    };

    let mut app = App::new(games, args.quit_after_launch, last_selected);
    let mut terminal = setup_terminal()?;

    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key_event(key);
                }
            }
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}
