use color_eyre::Result;

use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::{containers::Containers, Component};
use crate::runtime::container_exec;
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
        container_exec(&self.cid, &self.command).await?;

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
            tx.send(Action::Screen(Component::Containers(Containers::new(
                Default::default(),
            ))))?;
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
