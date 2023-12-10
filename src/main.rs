use std::io::{self, Stdout};

use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use color_eyre::eyre::Result;

mod app;
mod components;
mod utils;

type DoggyTerminal = Terminal<CrosstermBackend<Stdout>>;

fn setup_terminal() -> Result<DoggyTerminal> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let t = Terminal::new(backend)?;
    Ok(t)
}

fn teardown(mut terminal: DoggyTerminal) -> Result<()> {
    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // create app and run it
    let mut app = App::new();
    let res = app.run_app(&mut terminal);

    teardown(terminal)?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    if let Err(e) = run().await {
        eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
        Err(e)
    } else {
        Ok(())
    }
}
