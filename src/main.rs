use app::App;

use color_eyre::eyre::Result;
use utils::{initialize_logging, initialize_panic_handler, GIT_COMMIT_HASH};

mod action;
mod app;
mod components;
mod runtime;
mod tui;
mod utils;

const DEFAULT_TICK_RATE: f64 = 4.0;
const DEFAULT_FRAME_RATE: f64 = 30.0;

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging()?;

    initialize_panic_handler()?;

    runtime::init().await?;

    // create app and run it
    let mut app = App::new(GIT_COMMIT_HASH, DEFAULT_TICK_RATE, DEFAULT_FRAME_RATE);
    if let Err(e) = app.run().await {
        eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
        Err(e)
    } else {
        Ok(())
    }
}
