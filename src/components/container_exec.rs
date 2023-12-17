use std::io::{stdin, Read, Write};

use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use bollard::Docker;
use color_eyre::Result;

use futures::executor::block_on;
use futures::StreamExt;
use ratatui::prelude::*;

use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot::channel;
use tokio::task::spawn;

use crate::action::Action;
use crate::components::containers::Containers;
use crate::components::Component;
use crate::utils::clear_terminal;

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
                    while rx.try_recv().is_err() {
                        let mut buf: [u8; 1] = [0];
                        stdin()
                            .read_exact(&mut buf)
                            .expect("Unable to read from stdin");
                        input.write(&buf).await.ok();
                    }
                });

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

                clear_terminal();

                let mut stdout = std::io::stdout();
                // pipe docker exec output into stdout
                while let Some(Ok(output)) = output.next().await {
                    stdout
                        .write_all(output.into_bytes().as_ref())
                        .expect("Unable to write_all");
                    stdout.flush().expect("Unable to flush");
                }

                tx.send("Close").expect("Unable to send close command");
                handle.await.expect("Error waiting for thread termination");

                clear_terminal();
            }
        });
        Ok(Some(Action::Screen(Box::new(Containers::new()))))
    }

    fn draw(&mut self, _f: &mut Frame<'_>, _area: Rect) {}
}
