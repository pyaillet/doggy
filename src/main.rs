use std::io::{self, Stdout};

use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};
use ratatui::prelude::*;

use color_eyre::eyre::Result;

mod action;
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

static GIT_HASH: &str = env!("GIT_HASH");
static PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

async fn run() -> Result<()> {
    let mut terminal = setup_terminal()?;

    let version: String = format!("{}@{}", PKG_VERSION, GIT_HASH);
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
    // env_logger::init();
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("log/output.log")?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Debug),
        )?;

    log4rs::init_config(config)?;

    if let Err(e) = run().await {
        eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
        Err(e)
    } else {
        Ok(())
    }
}
