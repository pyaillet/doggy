use std::io::Write;

use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use color_eyre::Result;

use crossterm::cursor::{self, MoveTo};
use crossterm::terminal::{Clear, ClearType};
use crossterm::ExecutableCommand;

use futures::StreamExt;
use tokio::io::{stdin, AsyncReadExt, AsyncWriteExt};
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::spawn;
use tokio_util::sync::CancellationToken;

use crate::action::Action;
use crate::components::{containers::Containers, Component};
use crate::runtime::get_docker_connection;
use crate::tui;

const DEFAULT_CMD: &str = "/bin/bash";

#[derive(Clone, Debug)]
pub struct ContainerExec {
    cid: String,
    cname: String,
    command: String,
    action_tx: Option<UnboundedSender<Action>>,
    should_stop: bool,
}

impl ContainerExec {
    pub fn new(cid: String, cname: String, command: Option<String>) -> Self {
        log::debug!("{}>{:?}", cid, command);
        ContainerExec {
            cid,
            cname,
            command: command.unwrap_or(DEFAULT_CMD.to_string()),
            action_tx: None,
            should_stop: false,
        }
    }

    async fn exec(&mut self) -> Result<()> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let tty_size = crossterm::terminal::size()?;
        let mut stdout = std::io::stdout();

        let docker = get_docker_connection()?;
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
            .await?
            .id;

        if let StartExecResults::Attached {
            mut output,
            mut input,
        } = docker.start_exec(&exec, None).await?
        {
            // pipe stdin into the docker exec stream input
            let handle = spawn(async move {
                let mut buf: [u8; 1] = [0];
                let mut should_stop = false;
                let mut stdin = stdin();
                while !should_stop {
                    select!(
                        _ = _cancellation_token.cancelled() => { should_stop = true; },
                        _ = stdin.read(&mut buf) => { input.write(&buf).await.ok(); }
                    );
                }
            });

            stdout.execute(MoveTo(0, 0))?;
            stdout.execute(Clear(ClearType::All))?;
            stdout.execute(cursor::Show)?;

            docker
                .resize_exec(
                    &exec,
                    ResizeExecOptions {
                        height: tty_size.1,
                        width: tty_size.0,
                    },
                )
                .await?;

            // pipe docker exec output into stdout
            while let Some(Ok(output)) = output.next().await {
                stdout.write_all(output.into_bytes().as_ref())?;
                stdout.flush()?;
                log::debug!("========================== FLUSH");
            }

            log::debug!("Closing terminal");
            cancellation_token.cancel();
            handle.await?;
        }
        Ok(())
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "ContainerExec"
    }

    pub(crate) fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    pub(crate) fn setup(&mut self, t: &mut tui::Tui) -> Result<()> {
        t.stop()?;
        Ok(())
    }

    pub(crate) fn teardown(&mut self, t: &mut tui::Tui) -> Result<()> {
        t.clear()?;
        Ok(())
    }

    pub(crate) async fn update(&mut self, _action: Action) -> Result<()> {
        let tx = self.action_tx.clone().expect("Unable to get event sender");

        if !self.should_stop {
            let res = self.exec().await;

            self.should_stop = true;
            tx.send(Action::Resume)?;
            tx.send(Action::Screen(Component::Containers(Containers::new(None))))?;
            if let Err(e) = res {
                tx.send(Action::Error(format!(
                    "Unable to execute command \"{}\" in container \"{}\"\n{}",
                    self.command, self.cname, e
                )))?;
            }
        }
        Ok(())
    }
}
