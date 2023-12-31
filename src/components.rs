use crossterm::event::{self, KeyEvent};
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

use self::container_logs::ContainerLogs;

pub mod container_exec;
pub mod container_inspect;
pub mod container_logs;
pub mod containers;
pub mod image_inspect;
pub mod images;
pub mod network_inspect;
pub mod networks;
pub mod volume_inspect;
pub mod volumes;

#[derive(Clone, Debug)]
pub(crate) enum Component {
    Containers(Containers),
    ContainerExec(ContainerExec),
    ContainerInspect(ContainerDetails),
    ContainerLogs(ContainerLogs),
    Images(Images),
    ImageInspect(ImageInspect),
    Networks(Networks),
    NetworkInspect(NetworkInspect),
    Volumes(Volumes),
    VolumeInspect(VolumeInspect),
}

macro_rules! component_delegate {
    ($self:ident.$func:ident$args:tt, [$($member:tt),+]) => {
        match $self {
            $(Component::$member(c) => c.$func$args,)+
        }
    };
    ($self:ident.$func:ident$args:tt, [$($member:tt),+], $def:expr) => {
        match $self {
            $(Component::$member(c) => c.$func$args,)+
            _ => $def
        }
    };
    ($self:ident.$func:ident$args:tt.await, [$($member:tt),+]) => {
        match $self {
            $(Component::$member(c) => c.$func$args.await,)+
        }
    };
    ($self:ident.$func:ident$args:tt.await, [$($member:tt),+], $def:expr) => {
        match $self {
            $(Component::$member(c) => c.$func$args.await,)+
            _ => $def
        }
    };
}

impl Component {
    pub(crate) fn get_name(&self) -> &'static str {
        component_delegate!(
            self.get_name(),
            [
                Containers,
                ContainerExec,
                ContainerInspect,
                ContainerLogs,
                Images,
                ImageInspect,
                Networks,
                NetworkInspect,
                Volumes,
                VolumeInspect
            ]
        )
    }

    pub(crate) fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        component_delegate!(
            self.register_action_handler(action_tx),
            [
                Containers,
                ContainerExec,
                ContainerInspect,
                ContainerLogs,
                Images,
                ImageInspect,
                Networks,
                NetworkInspect,
                Volumes,
                VolumeInspect
            ]
        )
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        component_delegate!(
            self.update(action).await,
            [
                Containers,
                ContainerExec,
                ContainerInspect,
                ContainerLogs,
                Images,
                ImageInspect,
                Networks,
                NetworkInspect,
                Volumes,
                VolumeInspect
            ]
        )
    }

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
    pub(crate) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        component_delegate!(
            self.draw(f, area),
            [
                Containers,
                ContainerInspect,
                ContainerLogs,
                Images,
                ImageInspect,
                Networks,
                NetworkInspect,
                Volumes,
                VolumeInspect
            ],
            {}
        )
    }

    pub(crate) fn setup(&mut self, t: &mut tui::Tui) -> Result<()> {
        component_delegate!(self.setup(t), [ContainerExec], Ok(()))
    }
    pub(crate) fn teardown(&mut self, t: &mut tui::Tui) -> Result<()> {
        component_delegate!(self.teardown(t), [ContainerExec], Ok(()))
    }

    pub(crate) fn handle_input(
        &mut self,
        kevent: event::KeyEvent,
    ) -> Result<Option<event::KeyEvent>> {
        component_delegate!(self.handle_input(kevent), [Containers], Ok(Some(kevent)))
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        component_delegate!(
            self.get_bindings(),
            [Containers, ContainerLogs, Images, Networks, Volumes],
            None
        )
    }

    pub(crate) fn get_action(&self, k: &KeyEvent) -> Option<Action> {
        component_delegate!(
            self.get_action(k),
            [Containers, ContainerLogs, Images],
            None
        )
    }

    pub(crate) fn has_filter(&self) -> bool {
        component_delegate!(
            self.has_filter(),
            [Containers, Images, Networks, Volumes],
            false
        )
    }
}
