use std::io::Write;

use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use bollard::Docker;
use color_eyre::Result;

use crossterm::cursor::{self, MoveTo};
use crossterm::terminal::{Clear, ClearType};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;

use futures::executor::block_on;
use futures::StreamExt;
use tokio::io::{stdin, AsyncReadExt, AsyncWriteExt};
use tokio::select;
use tokio::sync::oneshot::channel;
use tokio::task::spawn;

use crate::action::Action;
use crate::components::containers::Containers;
use crate::components::Component;

const DEFAULT_CMD: &str = "/bin/bash";

pub struct ContainerExec {
    cid: String,
    command: String,
}

impl ContainerExec {
    pub fn new_with_default(cid: String) -> Self {
        ContainerExec::new(cid, DEFAULT_CMD.to_string())
    }

    pub fn new(cid: String, command: String) -> Self {
        log::debug!("{}>{}", cid, command);
        ContainerExec { cid, command }
    }
}

impl Component for ContainerExec {
    fn get_name(&self) -> &'static str {
        "ContainerExec"
    }

    fn update(&mut self, _action: Option<Action>) -> Result<Option<Action>> {
        block_on(async {
            let docker = Docker::connect_with_socket_defaults().unwrap();

            let tty_size = crossterm::terminal::size().expect("Unable to get tty size");
            let mut stdout = std::io::stdout();

            let exec = docker
                .create_exec(
                    &self.cid,
                    CreateExecOptions {
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        attach_stdin: Some(true),
                        tty: Some(true),
                        cmd: Some(vec![self.command.to_string()]),
                        ..Default::default()
                    },
                )
                .await
                .expect("Unable to create exec")
                .id;

            if let StartExecResults::Attached {
                mut output,
                mut input,
            } = docker
                .start_exec(&exec, None)
                .await
                .expect("Unable to start container exec")
            {
                let (tx, mut rx) = channel();

                // pipe stdin into the docker exec stream input
                let handle = spawn(async move {
                    let mut buf: [u8; 1] = [0];
                    let mut should_stop = false;
                    let mut stdin = stdin();
                    while !should_stop {
                        select!(
                            _ = &mut rx => { should_stop = true; },
                            _ = stdin.read(&mut buf) => { input.write(&buf).await.ok(); }
                        );
                    }
                });

                stdout.execute(MoveTo(0, 0)).expect("Unable to move cursor");
                stdout
                    .execute(Clear(ClearType::All))
                    .expect("Unable to clear screen");
                stdout.execute(cursor::Show).expect("Unable to show cursor");

                docker
                    .resize_exec(
                        &exec,
                        ResizeExecOptions {
                            height: tty_size.1,
                            width: tty_size.0,
                        },
                    )
                    .await
                    .expect("Unable to resize exec");

                // pipe docker exec output into stdout
                while let Some(Ok(output)) = output.next().await {
                    stdout
                        .write_all(output.into_bytes().as_ref())
                        .expect("Unable to write_all");
                    stdout.flush().expect("Unable to flush");
                }

                log::debug!("Closing terminal");
                tx.send(0).expect("Unable to send close command");
                handle.await.expect("Error waiting for thread termination");
            }
        });
        Ok(Some(Action::Screen(Box::new(Containers::new()), true)))
    }

    fn draw(&mut self, _f: &mut Frame<'_>, _area: Rect) {}
}
