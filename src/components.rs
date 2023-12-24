use crossterm::event;
use ratatui::{layout::Rect, Frame};

use color_eyre::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

use crate::components::container_exec::ContainerExec;
use crate::components::container_inspect::ContainerDetails;
use crate::components::containers::Containers;
use crate::components::image_inspect::ImageInspect;
use crate::components::images::Images;
use crate::components::network_inspect::NetworkInspect;
use crate::components::networks::Networks;
use crate::components::volume_inspect::VolumeInspect;
use crate::components::volumes::Volumes;
use crate::tui;

pub mod container_exec;
pub mod container_inspect;
pub mod containers;
pub mod image_inspect;
pub mod images;
pub mod network_inspect;
pub mod networks;
pub mod volume_inspect;
pub mod volumes;

#[derive(Clone, Debug)]
pub(crate) enum ComponentInit {
    Containers,
    ContainerExec(String, String, Option<String>),
    ContainerInspect(String, String, String),
    Images,
    ImageInspect(String, String, String),
    Networks,
    NetworkInspect(String, String, String),
    Volumes,
    VolumeInspect(String, String),
}

impl ComponentInit {
    pub fn get_component(self) -> Box<dyn Component> {
        match self {
            ComponentInit::Containers => Box::new(Containers::new()),
            ComponentInit::ContainerInspect(id, name, details) => {
                Box::new(ContainerDetails::new(id, name, details))
            }
            ComponentInit::ContainerExec(id, cname, cmd) => {
                Box::new(ContainerExec::new(id, cname, cmd))
            }
            ComponentInit::Images => Box::new(Images::new()),
            ComponentInit::ImageInspect(id, name, details) => {
                Box::new(ImageInspect::new(id, name, details))
            }
            ComponentInit::Networks => Box::new(Networks::new()),
            ComponentInit::NetworkInspect(id, name, details) => {
                Box::new(NetworkInspect::new(id, name, details))
            }
            ComponentInit::Volumes => Box::new(Volumes::new()),
            ComponentInit::VolumeInspect(id, details) => Box::new(VolumeInspect::new(id, details)),
        }
    }
}

pub(crate) trait Component {
    fn get_name(&self) -> &'static str;

    fn register_action_handler(&mut self, _action_tx: UnboundedSender<Action>) {}

    fn update(&mut self, action: Action) -> Result<()>;

    /// Render the component on the screen. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `f` - A frame used for rendering.
    /// * `area` - The area in which the component should be drawn.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect);

    fn setup(&mut self, _t: &mut tui::Tui) -> Result<()> {
        Ok(())
    }
    fn teardown(&mut self, _t: &mut tui::Tui) -> Result<()> {
        Ok(())
    }

    fn handle_input(&mut self, _kevent: event::KeyEvent) -> Result<()> {
        Ok(())
    }

    fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        None
    }
}
