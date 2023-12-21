use std::io;

use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use color_eyre::eyre::Result;
use utils::{initialize_logging, initialize_panic_handler, GIT_COMMIT_HASH};

mod action;
mod app;
mod components;
mod tui;
mod utils;

fn setup_terminal() -> Result<ratatui::Terminal<CrosstermBackend<std::io::Stderr>>> {
    enable_raw_mode()?;

    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stderr);
    let mut t = Terminal::new(backend)?;

    t.clear().expect("Unable to clear terminal");

    Ok(t)
}

fn teardown(mut terminal: ratatui::Terminal<CrosstermBackend<std::io::Stderr>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

static PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

async fn run() -> Result<()> {
    let mut terminal = setup_terminal()?;

    let version: String = format!("{}@{}", PKG_VERSION, GIT_COMMIT_HASH);
    // create app and run it
    let mut app = App::new(&version);
    let res = app.run_app(&mut terminal);

    teardown(terminal)?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging()?;

    initialize_panic_handler()?;

    if let Err(e) = run().await {
        eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
        Err(e)
    } else {
        Ok(())
    }
}
