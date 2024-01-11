use app::App;

use clap::{arg, command, Parser};
use color_eyre::eyre::Result;
use eyre::eyre;

use utils::{initialize_logging, initialize_panic_handler, GIT_COMMIT_HASH};

#[cfg(feature = "cri")]
use runtime::cri;
use runtime::docker;

mod action;
mod app;
mod components;
mod runtime;
mod tui;
mod utils;

const DEFAULT_TICK_RATE: f64 = 4.0;
const DEFAULT_FRAME_RATE: f64 = 30.0;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    docker: Option<String>,

    #[cfg(feature = "cri")]
    #[arg(short, long)]
    cri: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging()?;

    initialize_panic_handler()?;

    #[cfg(feature = "cri")]
    let config = {
        let Args { docker, cri } = Args::parse();
        match (docker, cri) {
            (Some(docker), None) => Some(runtime::ConnectionConfig::Docker(
                docker::ConnectionConfig::socket(docker),
            )),
            (None, Some(cri)) => Some(runtime::ConnectionConfig::Cri(
                cri::ConnectionConfig::socket(cri),
            )),
            (None, None) => None,
            (Some(_), Some(_)) => {
                return Err(eyre!("You should specify --docker or --cri but not both"))?;
            }
        }
    };

    #[cfg(not(feature = "cri"))]
    let config = {
        let Args { docker } = Args::parse();
        docker.map(|d| {
            Some(runtime::ConnectionConfig::Docker(
                docker::ConnectionConfig::socket(d),
            ))
        })
    };

    runtime::init(config).await?;

    // create app and run it
    let mut app = App::new(GIT_COMMIT_HASH, DEFAULT_TICK_RATE, DEFAULT_FRAME_RATE);
    if let Err(e) = app.run().await {
        eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
        Err(e)
    } else {
        Ok(())
    }
}
